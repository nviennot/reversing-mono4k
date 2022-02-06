#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(int_abs_diff)]
#![allow(unused_imports, dead_code, unused_variables, unused_macros, unreachable_code)]

mod drivers;
mod consts;
mod ui;

use alloc::format;
use cstr_core::{CStr, CString};
use drivers::stepper::Direction;
use lvgl::style::State;
use stm32f1xx_hal::pac::Interrupt;
use consts::system::*;
use consts::display::*;
use drivers::{
    init::{Systick, Machine, prelude::*},
    touch_screen::{TouchScreenResult, TouchEvent},
    display::Display as RawDisplay,
};

use embedded_graphics::pixelcolor::Rgb565;

use lvgl::core::{
    Lvgl, TouchPad, Display, InputDevice,
};


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
    use super::*;

    #[monotonic(binds = SysTick, default = true)]
    type MonotonicClock = Systick;

    /* resources shared across RTIC tasks */
    #[shared]
    struct Shared {
        stepper: drivers::stepper::Stepper,
        #[lock_free]
        touch_screen: drivers::touch_screen::TouchScreen,
        last_touch_event: Option<TouchEvent>,
    }

    /* resources local to specific RTIC tasks */
    #[local]
    struct Local {
        lvgl: Lvgl,
        lvgl_ticks: Ticks,
        lvgl_input_device: InputDevice::<TouchPad>,
        display: Display::<RawDisplay>,
        move_z_ui: Screen<MoveZ>,
    }

    fn lvgl_init(display: RawDisplay) -> (Lvgl, Display<RawDisplay>, InputDevice<TouchPad>) {
        let mut lvgl = Lvgl::new();

        // Register logger
        lvgl.register_logger(|s| rtt_target::rprint!(s));

        static mut DRAW_BUFFER: [MaybeUninit<Rgb565>; LVGL_BUFFER_LEN] =
            [MaybeUninit::<Rgb565>::uninit(); LVGL_BUFFER_LEN];

        let mut display = Display::new(&lvgl, display, unsafe { &mut DRAW_BUFFER });

        let input_device = lvgl::core::InputDevice::<TouchPad>::new(&mut display);

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

        let (mut lvgl, mut display, lvgl_input_device) = lvgl_init(display);

        let lvgl_ticks = lvgl.ticks();
        lvgl_tick_task::spawn().unwrap();

        let last_touch_event = None;

        let mut move_z_ui = MoveZ::new(&display);
        // Fill the display with something before turning it on.
        display.load_screen(&mut move_z_ui);
        lvgl.run_tasks();
        display.backlight.set_high();

        /*
        let ext_flash = machine.ext_flash;
        let delay = machine.delay;
        let mut stepper = machine.stepper;
        */

        debug!("Init complete");

        (
            Shared { stepper, touch_screen, last_touch_event },
            Local { lvgl, lvgl_ticks, lvgl_input_device, display, move_z_ui },
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
    fn touch_screen_pen_down_interrupt(ctx: touch_screen_pen_down_interrupt::Context) {
        use TouchScreenResult::*;
        match ctx.shared.touch_screen.on_pen_down_interrupt() {
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
        match ctx.shared.touch_screen.on_delay_expired() {
            DelayMs(delay_ms) => {
                touch_screen_sampling_task::spawn_after((delay_ms as u64).millis()).unwrap();
            },
            Done(touch_event) => {
                ctx.shared.last_touch_event.lock(|t| *t = touch_event);
                unsafe { cortex_m::peripheral::NVIC::unmask(Interrupt::EXTI9_5); }
            },
        }
    }

    #[idle(local = [lvgl, lvgl_input_device, display, move_z_ui], shared = [last_touch_event, stepper])]
    fn idle(mut ctx: idle::Context) -> ! {
        let lvgl = ctx.local.lvgl;
        let lvgl_input_device = ctx.local.lvgl_input_device;
        let move_z_ui = ctx.local.move_z_ui.context().as_mut().unwrap();

        loop {
            ctx.shared.last_touch_event.lock(|e| {
                *lvgl_input_device.state() = if let Some(ref e) = e {
                    TouchPad::Pressed { x: e.x as i16, y: e.y as i16 }
                } else {
                    TouchPad::Released
                };
            });

            lvgl.run_tasks();

            move_z_ui.update(&mut ctx.shared.stepper);
        }
    }
}


use lvgl::core::Screen;
use lvgl::core::Ticks;
use lvgl::widgets::*;
use lvgl::core::*;

#[derive(Debug)]
enum UserAction {
    MoveUp,
    MoveDown,
    StopRequested,
    SetSpeed(f32),
}

use drivers::stepper::prelude::*;

pub struct MoveZ {
    btn_move_up: Btn<Self>,
    btn_move_down: Btn<Self>,
    speed_slider: Slider<Self>,
    speed_label: Label<Self>,
    position_label: Label<Self>,

    user_action: Option<UserAction>,
}

impl MoveZ {
    pub fn new<D>(display: &Display<D>) -> Screen::<Self> {
        use lvgl::widgets::*;
        use lvgl::style::*;
        use lvgl::core::*;
        use lvgl::prelude::*;

        let spacing = 12;

        let mut screen = Screen::<Self>::new(display);

        let btn_move_up = Btn::new(&mut screen).apply(|obj| {
            Label::new(obj)
                .set_text(&CStr::from_bytes_with_nul(b"Move Up\0").unwrap());
            obj
            .align_to(&screen, Align::TopMid, 0, 2*spacing)
            .add_flag(Flag::CHECKABLE)
            .on_event(Event::Clicked, |context| {
                let checked = context.btn_move_up.has_state(State::CHECKED);
                context.user_action = Some(
                    if checked { UserAction::MoveUp }
                    else { UserAction::StopRequested }
                );
            });
        });

        let btn_move_down = Btn::new(&mut screen).apply(|obj| {
            Label::new(obj)
                .set_text(&CStr::from_bytes_with_nul(b"Move Down\0").unwrap());
            obj
            .align_to(&btn_move_up, Align::OutBottomMid, 0, spacing)
            .add_flag(Flag::CHECKABLE)
            .on_event(Event::Clicked, |context| {
                let checked = context.btn_move_down.has_state(State::CHECKED);
                context.user_action = Some(
                    if checked { UserAction::MoveDown }
                    else { UserAction::StopRequested }
                );
            });
        });

        let speed_slider = Slider::new(&mut screen).apply(|obj| { obj
            .align_to(&btn_move_down, Align::OutBottomMid, 0, 2*spacing)
            .set_range(1500, 10_000)
            .set_value(10_000, 0)
            .on_event(Event::ValueChanged, |context| {
                let value = unsafe { lvgl_sys::lv_slider_get_value(context.speed_slider.raw) };

                let value = (value as f32)/10000.0;
                let value = value*value*value;
                let value = value * 30.0;

                context.user_action = Some(UserAction::SetSpeed(value));
            });
        });

        let speed_label = Label::new(&mut screen).apply(|obj| { obj
            .align_to(&speed_slider, Align::OutBottomLeft, 70, spacing)
            .set_text(&CStr::from_bytes_with_nul(b"Max Speed: 5 mm/s\0").unwrap());
        });

        let position_label = Label::new(&mut screen).apply(|obj| { obj
            .align_to(&speed_label, Align::OutBottomLeft, 0, 0)
            .set_text(&CStr::from_bytes_with_nul(b"Position: 0 mm\0").unwrap());
        });

        Label::new(&mut screen).apply(|obj| { obj
            .set_text(&CStr::from_bytes_with_nul(b"Turbo Resin v0.1.1\0").unwrap())
            .align_to(&screen, Align::BottomRight, -5, -5);
        });

        let context = Self {
            btn_move_up,
            btn_move_down,
            speed_slider,
            speed_label,
            position_label,

            user_action: None,
        };

        screen.apply(|s| {
            s.context().replace(context);
        })
    }

    fn update(&mut self, stepper: &mut impl rtic::Mutex<T=drivers::stepper::Stepper>) {
        match self.user_action.take() {
            Some(UserAction::MoveUp) => {
                stepper.lock(|s| s.set_target_relative(40.0.mm()));
                self.btn_move_down.add_state(State::DISABLED);
            },
            Some(UserAction::MoveDown) => {
                stepper.lock(|s| s.set_target_relative((-40.0).mm()));
                self.btn_move_up.add_state(State::DISABLED);
            }
            Some(UserAction::StopRequested) => {
                self.btn_move_down.add_state(State::DISABLED);
                self.btn_move_up.add_state(State::DISABLED);
                stepper.lock(|s| s.controlled_stop());
            }
            Some(UserAction::SetSpeed(v)) => stepper.lock(|s| s.set_max_speed(Some(v.mm()))),
            None => {}
        }


        let (is_idle, current_position, max_speed) = stepper.lock(|s|
            (s.is_idle(), s.current_position, s.max_speed)
        );

        if is_idle {
            self.btn_move_up.clear_state(State::CHECKED | State::DISABLED);
            self.btn_move_down.clear_state(State::CHECKED | State::DISABLED);
        }

        // set_text() makes a copy of the string internally.
        self.position_label.set_text(&CStr::from_bytes_with_nul(
            format!("Position: {:.2} mm\0", current_position.as_mm()).as_bytes()
        ).unwrap());

        self.speed_label.set_text(&CStr::from_bytes_with_nul(
            format!("Max speed: {:.2} mm/s\0", max_speed.as_mm()).as_bytes()
        ).unwrap());
    }
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
