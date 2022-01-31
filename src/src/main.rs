#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(int_abs_diff)]
#![allow(unused_imports, dead_code, unused_variables, unused_macros, unreachable_code)]

mod drivers;
mod consts;
mod ui;

use cstr_core::CString;
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
        lvgl: Lvgl<AppState>,
        lvgl_ticks: lvgl::core::Ticks,
    }

    #[derive(Default)]
    pub struct AppState {
        last_touch_event: Option<TouchEvent>,
    }

    fn lvgl_init(display: Display) -> Lvgl<AppState> {
        let mut lvgl = Lvgl::<AppState>::new();
        //lvgl.register_logger(|s| rtt_target::rprint!(s));
        static mut DRAW_BUFFER: [MaybeUninit<Rgb565>; LVGL_BUFFER_LEN] =
            [MaybeUninit::<Rgb565>::uninit(); LVGL_BUFFER_LEN];

        let mut display = lvgl.register_display(unsafe { &mut DRAW_BUFFER }, display);

        display.register_input_device(|app_state| {
            if let Some(ref e) = app_state.last_touch_event {
                TouchPad::Pressed { x: e.x as i16, y: e.y as i16 }
            } else {
                TouchPad::Released
            }
        });

        construct_ui(&mut display);

        // Fill the display with something before turning it on.
        lvgl.run_tasks(&mut Default::default());
        display.backlight.set_high();

        lvgl
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_target::rtt_init_print!();
        lvgl::allocator::heap_init();

        debug!("Booting");

        let machine = Machine::new(ctx.core, ctx.device);

        let display = machine.display;
        let systick = machine.systick;
        let stepper = machine.stepper;
        let touch_screen = machine.touch_screen;

        let lvgl = lvgl_init(display);

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
            Local { lvgl, lvgl_ticks },
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

    #[idle(local = [lvgl], shared = [last_touch_event])]
    fn idle(mut ctx: idle::Context) -> ! {
        let mut app_state: AppState = Default::default();
        let lvgl = ctx.local.lvgl;
        loop {
            app_state.last_touch_event = ctx.shared.last_touch_event.lock(|e| *e);
            lvgl.run_tasks(&mut app_state);
        }
    }
}

fn construct_ui(display: &mut lvgl::core::Display<Display, app::AppState>) {
    use lvgl::widgets::*;
    use lvgl::style::*;
    use lvgl::core::Widget;

    let spacing = 12;

    let mut screen = display.screen();

    let mut btn = Btn::new(&mut screen);
    let mut label = Label::new(&mut btn);
    label.set_text(CString::new("Move up").unwrap().as_c_str());

    btn.on_event(lvgl::core::Event::Clicked, |target, event, child_target| {
        debug!("Button event: {:?}", event);
    });

    let mut speed_slider = Slider::new(&mut screen);
    speed_slider.align_to(&mut screen, Align::Center, 0, 0);
}

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
