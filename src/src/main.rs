#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(int_abs_diff)]
#![allow(unused_imports, dead_code, unused_variables, unused_macros, unreachable_code)]

mod drivers;
mod consts;
mod ui;

use cstr_core::{CStr, CString};
use stm32f1xx_hal::pac::Interrupt;
use consts::system::*;
use consts::display::*;
use drivers::{
    init::{Systick, Machine, prelude::*},
    touch_screen::{TouchScreenResult, TouchEvent},
    display::Display,
};

use lvgl::core::{Lvgl, TouchPad};

pub(crate) use runtime::debug;

extern crate alloc;

use core::mem::MaybeUninit;

mod runtime {
    use super::*;

    /*
    #[global_allocator]
    static ALLOCATOR: alloc_cortex_m::CortexMHeap = alloc_cortex_m::CortexMHeap::empty();

    pub fn init_heap() {
        // Using cortex_m_rt::heap_start() is bad. It doesn't tell us if our
        // HEAP_SIZE is too large and we will fault accessing non-existing RAM
        // Instead, we'll allocate a static buffer for our heap.
        unsafe {
            static mut HEAP: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
            ALLOCATOR.init((&mut HEAP).as_ptr() as usize, HEAP_SIZE);
        }
    }
    */

    #[alloc_error_handler]
    fn oom(l: core::alloc::Layout) -> ! {
        panic!("Out of memory. Failed to allocate {} bytes", l.size());
    }

    #[inline(never)]
    #[panic_handler]
    fn panic(info: &core::panic::PanicInfo) -> ! {
        debug!("{}", info);
        loop {}
    }

    macro_rules! debug {
        ($($tt:tt)*) => {
            rtt_target::rprintln!($($tt)*)
        }
    }
    pub(crate) use debug;
}

#[rtic::app(
    device = stm32f1xx_hal::stm32, peripherals = true,
    // Picked random interrupts that we'll never use. RTIC will use this to schedule tasks.
    dispatchers=[CAN_RX1, CAN_SCE, CAN2_RX0, CAN2_RX1]
)]
mod app {

    use embedded_graphics::pixelcolor::Rgb565;

    use super::*;

    #[monotonic(binds = SysTick, default = true)]
    type MonotonicClock = Systick;

    /* resources shared across RTIC tasks */
    #[shared]
    struct Shared {
        stepper: drivers::stepper::Stepper,
        touch_screen: drivers::touch_screen::TouchScreen,
        last_touch_event: Option<TouchEvent>,
    }

    /* resources local to specific RTIC tasks */
    #[local]
    struct Local {
        lvgl: Lvgl,
        lvgl_ticks: lvgl::core::Ticks,
        lvgl_input_device: lvgl::core::InputDevice::<TouchPad>,
        display: lvgl::core::Display::<Display>,
    }

    fn lvgl_init(display: Display) -> (Lvgl, lvgl::core::Display<Display>, lvgl::core::InputDevice<TouchPad>) {
        let mut lvgl = Lvgl::new();

        // Register logger
        lvgl.register_logger(|s| rtt_target::rprint!(s));

        static mut DRAW_BUFFER: [MaybeUninit<Rgb565>; LVGL_BUFFER_LEN] =
            [MaybeUninit::<Rgb565>::uninit(); LVGL_BUFFER_LEN];

        let mut display = lvgl::core::Display::new(&lvgl, display, unsafe { &mut DRAW_BUFFER });

        let input_device = lvgl::core::InputDevice::<TouchPad>::new(&mut display);

        construct_ui(&mut display);

        // Fill the display with something before turning it on.
        lvgl.run_tasks();
        display.backlight.set_high();

        (lvgl, display, input_device)
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_target::rtt_init_print!();
        debug!("Booting");

        lvgl::core::Lvgl::new();

        let machine = Machine::new(ctx.core, ctx.device);

        let display = machine.display;
        let systick = machine.systick;
        let stepper = machine.stepper;
        let touch_screen = machine.touch_screen;

        let (lvgl, display, lvgl_input_device) = lvgl_init(display);

        let lvgl_ticks = lvgl.ticks();
        lvgl_tick_task::spawn().unwrap();

        let last_touch_event = None;

        /*
        let ext_flash = machine.ext_flash;
        let delay = machine.delay;
        let mut stepper = machine.stepper;
        */

        debug!("Init complete");

        (
            Shared { stepper, touch_screen, last_touch_event },
            Local { lvgl, lvgl_ticks, lvgl_input_device, display },
            init::Monotonics(systick),
        )
    }

    #[task(priority = 5, binds = TIM7, shared = [stepper])]
    fn stepper_interrupt(mut ctx: stepper_interrupt::Context) {
        ctx.shared.stepper.lock(|s| s.on_interrupt());
    }

    #[task(priority = 3, local = [lvgl_ticks], shared = [])]
    fn lvgl_tick_task(ctx: lvgl_tick_task::Context) {
        // Not very precise (by the time we get here, some time has passed
        // already), but good enough
        lvgl_tick_task::spawn_after(1.millis()).unwrap();
        ctx.local.lvgl_ticks.inc(1);
    }

    #[task(priority = 2, binds = EXTI9_5, shared = [touch_screen])]
    fn touch_screen_pen_down_interrupt(mut ctx: touch_screen_pen_down_interrupt::Context) {
        use TouchScreenResult::*;
        match ctx.shared.touch_screen.lock(|ts| { ts.on_pen_down_interrupt() }) {
            DelayMs(delay_ms) => {
                cortex_m::peripheral::NVIC::mask(Interrupt::EXTI9_5);
                touch_screen_sampling_task::spawn_after((delay_ms as u64).millis()).unwrap();
            }
            Done(None) => {},
            Done(Some(_)) => unreachable!(),
        }
    }

    #[task(priority = 2, local = [], shared = [touch_screen, last_touch_event])]
    fn touch_screen_sampling_task(mut ctx: touch_screen_sampling_task::Context) {
        use TouchScreenResult::*;
        match ctx.shared.touch_screen.lock(|ts| ts.on_delay_expired()) {
            DelayMs(delay_ms) => {
                touch_screen_sampling_task::spawn_after((delay_ms as u64).millis()).unwrap();
            },
            Done(touch_event) => {
                ctx.shared.last_touch_event.lock(|t| *t = touch_event);
                unsafe { cortex_m::peripheral::NVIC::unmask(Interrupt::EXTI9_5); }
            },
        }
    }

    #[idle(local = [lvgl, lvgl_input_device, display], shared = [last_touch_event])]
    fn idle(mut ctx: idle::Context) -> ! {
        let lvgl = ctx.local.lvgl;
        loop {
            let last_touch_event = ctx.shared.last_touch_event.lock(|e| *e);
            *ctx.local.lvgl_input_device.state() = if let Some(ref e) = last_touch_event {
                TouchPad::Pressed { x: e.x as i16, y: e.y as i16 }
            } else {
                TouchPad::Released
            };
            lvgl.run_tasks();
        }
    }
}


use lvgl::widgets::*;

/*
struct UIMoveZ<'p> {
    btn_move_up: Btn<'p, Self>,
}
 */

 struct UiContext {
     x: u16,
 }

fn construct_ui<D>(display: &mut lvgl::core::Display<D>) {
    use lvgl::widgets::*;
    use lvgl::style::*;
    use lvgl::core::*;
    use lvgl::prelude::*;

    let spacing = 12;

    let screen = Screen::<UiContext>::new(display);

    let mut screen = screen.nest(|p|
        Label::new(p)
            .text(&CStr::from_bytes_with_nul(b"Turbo Resin v0.1.1\0").unwrap())
            .align(p, Align::BottomRight, -5, -5)
    );

    let btn_move_up = Btn::new(&mut screen)
        .nest(|p| Label::new(p).text(&CStr::from_bytes_with_nul(b"Move Up\0").unwrap()))
        .align(&screen, Align::TopMid, 0, 2*spacing)
        .on_event(Event::Clicked, |context| {
            debug!("Button event: pressed!");
            /*
            btn_up_active = !btn_up_active;
            if btn_up_active {
                stepper.modify(|s| s.set_target_relative(100.0.mm()));
                unsafe {
                    lvgl_sys::lv_btn_set_state(btn_down.raw().unwrap().as_mut(),
                        lvgl_sys::LV_BTN_STATE_DISABLED as u8);
                }
            } else {
                stepper.modify(|s| s.controlled_stop());
            }
            */
        });

    let btn_move_down = Btn::new(&mut screen)
        .nest(|p| Label::new(p).text(&CStr::from_bytes_with_nul(b"Move Down\0").unwrap()))
        .align(&btn_move_up, Align::OutBottomMid, 0, spacing)
        .on_event(Event::Clicked, |state| {
            /*
            btn_down_active = !btn_down_active;
            if btn_down_active {
                stepper.modify(|s| s.set_target_relative((-100.0).mm()));
                unsafe {
                    lvgl_sys::lv_btn_set_state(btn_up.raw().unwrap().as_mut(),
                        lvgl_sys::LV_BTN_STATE_DISABLED as u8);
                }
            } else {
                stepper.modify(|s| s.controlled_stop());
            }
            */
            debug!("Button event: pressed!");
        });


    let speed_slider = Slider::new(&mut screen)
        .align(&btn_move_down, Align::OutBottomMid, 0, 2*spacing)
        .range(1500, 10_000)
        .value(10_000, 0)
        .on_event(Event::ValueChanged, |state| {
            /*
            let value = unsafe { lvgl_sys::lv_slider_get_value(slider.raw().unwrap().as_ref()) };
            let value = (value as f32)/10000.0;
            let value = value*value*value;

            let value = value * 30.0;

            stepper.modify(|s| s.set_max_speed(Some(value.mm())));
            */
            debug!("Slider changed");
        });

    let speed_label = Label::new(&mut screen)
        .align(&speed_slider, Align::OutBottomLeft, 70, spacing)
        //.width(320-70)
        .text(&CStr::from_bytes_with_nul(b"Max Speed: 5 mm/s\0").unwrap());

    let position_label = Label::new(&mut screen)
        .align(&speed_label, Align::OutBottomLeft, 0, 0)
        .text(&CStr::from_bytes_with_nul(b"Position: 0 mm\0").unwrap());

    let context = UiContext { x: 0 };

    screen.context().replace(context);
    display.load_screen(&mut screen);

    //UIMoveZ { btn_move_up }

    //let mut speed_slider = Slider::new(&mut screen);
    //speed_slider.align_to(&mut screen, Align::Center, 0, 0);
}

/*

fn draw() {
        if stepper.access(|s| s.is_idle()) {
            btn_down_active = false;
            btn_up_active = false;

            // turn off warning. Weird.
            if btn_down_active || btn_up_active {}

            unsafe {
                lvgl_sys::lv_btn_set_state(btn_up.raw().unwrap().as_mut(),
                    lvgl_sys::LV_BTN_STATE_RELEASED as u8);
                lvgl_sys::lv_btn_set_state(btn_down.raw().unwrap().as_mut(),
                    lvgl_sys::LV_BTN_STATE_RELEASED as u8);
            }
        }

        ui.tick_inc(Duration::from_millis(20));
    }

}

*/


/*
fn draw_touch_event(display: &mut Display, touch_event: Option<&TouchEvent>) {
    use embedded_graphics::{prelude::*, primitives::{Circle, PrimitiveStyle}, pixelcolor::Rgb565};

    if let Some(touch_event) = touch_event {
        Circle::new(Point::new(touch_event.x as i32, touch_event.y as i32), 3)
            .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
            .draw(display).unwrap();
    }
}
*/
