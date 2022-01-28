#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(int_abs_diff)]
#![allow(unused_imports, dead_code, unused_variables, unused_macros, unreachable_code)]

mod drivers;
mod consts;
mod ui;

use stm32f1xx_hal::pac::Interrupt;
use consts::system::*;
use drivers::{
    init::{Systick, Machine, prelude::*},
    touch_screen::{TouchScreenResult, TouchEvent},
    display::Display,
};

pub(crate) use runtime::debug;

mod runtime {
    use super::*;

    #[global_allocator]
    static ALLOCATOR: alloc_cortex_m::CortexMHeap = alloc_cortex_m::CortexMHeap::empty();

    pub fn init_heap() {
        let start = cortex_m_rt::heap_start() as usize;
        unsafe { ALLOCATOR.init(start, HEAP_SIZE) }
    }

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
            rtt_target::rprintln!($($tt)*);
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
        last_touch_event: Option<TouchEvent>,
        touch_screen: drivers::touch_screen::TouchScreen,
    }

    /* resources local to specific RTIC tasks */
    #[local]
    struct Local {
        display: drivers::display::Display,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_target::rtt_init_print!();
        runtime::init_heap();

        let machine = Machine::new(ctx.core, ctx.device);

        let mut display = machine.display;
        let systick = machine.systick;
        let stepper = machine.stepper;
        let touch_screen = machine.touch_screen;
        /*
        let ext_flash = machine.ext_flash;
        let delay = machine.delay;
        let mut stepper = machine.stepper;
        */

        //touch_task::spawn().unwrap();

        /*
        struct TouchGlobal {
            touch_screen: TouchScreen,
            delay: Delay,
        }

        static mut TOUCH: Option<TouchGlobal> = None;
        unsafe {
            let delay = core::mem::transmute_copy(&delay);
            TOUCH = Some(TouchGlobal { touch_screen, delay });
        }
        */

        display.backlight.set_high();
        let last_touch_event = None;

        debug!("Init complete");

        (
            Shared { stepper, touch_screen, last_touch_event },
            Local { display },
            init::Monotonics(systick),
        )
    }

    #[idle(local = [], shared = [stepper])]
    fn idle(_cx: idle::Context) -> ! {
        loop {}
        /*
        use drivers::stepper::prelude::*;

        use core::fmt::Write;
        use cstr_core::CString;

        use lvgl::*;
        use lvgl::style::*;
        use lvgl::widgets::*;

        let mut ui = UI::init().unwrap();
        let display = *cx.local.display;
        let touch_screen = *cx.local.touch_screen;

        ui.disp_drv_register(display).unwrap();
        let mut screen = ui.scr_act().unwrap();

        struct TouchGlobal {
            touch_screen: TouchScreen,
        }

        static mut TOUCH: Option<TouchGlobal> = None;
        unsafe {
            //let delay = core::mem::transmute_copy(&delay);
            TOUCH = Some(TouchGlobal { touch_screen });
        }


        unsafe extern "C" fn input_read_cb(drv: *mut lvgl_sys::lv_indev_drv_t, data: *mut lvgl_sys::lv_indev_data_t) -> bool {
            let t = TOUCH.as_mut().unwrap();


            if let Some((x,y)) = t.touch_screen.read_x_y(&mut t.delay) {
                (*data).point.x = x as i16;
                (*data).point.y = y as i16;
                (*data).state = lvgl_sys::LV_INDEV_STATE_PR as u8;
            } else {
                (*data).state = lvgl_sys::LV_INDEV_STATE_REL as u8;
            }

            false
        }

        let indev_drv = unsafe {
            let mut indev_drv = MaybeUninit::<lvgl_sys::lv_indev_drv_t>::uninit();
            lvgl_sys::lv_indev_drv_init(indev_drv.as_mut_ptr());
            let mut indev_drv = indev_drv.assume_init();
            indev_drv.type_ = lvgl_sys::LV_INDEV_TYPE_POINTER as u8;
            indev_drv.read_cb = Some(input_read_cb);
            lvgl_sys::lv_indev_drv_register(&mut indev_drv as *mut lvgl_sys::lv_indev_drv_t);
            indev_drv
        };

        let mut screen_style = Style::default();
        screen_style.set_bg_color(State::DEFAULT, Color::from_rgb((80, 80, 80)));
        //screen_style.set_radius(State::DEFAULT, 0);
        //screen.add_style(Part::Main, screen_style).unwrap();
        //screen_style.fl

        let spacing = 12;

        {
            let mut label = Label::new(&mut screen).unwrap();
            label.set_text(CString::new("Turbo Resin v0.1.1").unwrap().as_c_str()).unwrap();
            label.set_align(&mut screen, Align::InBottomRight, -5, -5).unwrap();
        }

        let mut btn_up = {
            let mut btn = Btn::new(&mut screen).unwrap();
            btn.set_align(&mut screen, Align::InTopMid, 0, 2*spacing).unwrap();
            let mut label = Label::new(&mut btn).unwrap();
            label.set_text(CString::new("Move Up").unwrap().as_c_str()).unwrap();
            label.set_label_align(LabelAlign::Center).unwrap();
            btn.set_checkable(true).unwrap();
            btn
        };

        let mut btn_up_active = false;
        let mut btn_down_active = false;

        let mut btn_down = {
            let mut btn = Btn::new(&mut screen).unwrap();
            btn.set_align(&mut btn_up, Align::OutBottomMid, 0, spacing).unwrap();
            let mut label = Label::new(&mut btn).unwrap();
            label.set_text(CString::new("Move Down").unwrap().as_c_str()).unwrap();
            label.set_label_align(LabelAlign::Center).unwrap();
            btn.set_checkable(true).unwrap();
            btn
        };

        btn_up.on_event(|_, event| {
            if let lvgl::Event::Clicked = event {
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
            }
        }).unwrap();

        btn_down.on_event(|_, event| {
            if let lvgl::Event::Clicked = event {
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
            }
        }).unwrap();

        let (speed_slider, mut speed_label) = {
            let mut speed_slider = Slider::new(&mut screen).unwrap();
            speed_slider.set_align(&mut btn_down, Align::OutBottomMid, 0, 2*spacing).unwrap();

            let mut speed_label = Label::new(&mut screen).unwrap();
            speed_label.set_align(&mut speed_slider, Align::OutBottomLeft, 70, spacing).unwrap();
            speed_label.set_width(320).unwrap();
            speed_label.set_text(&CString::new("Max Speed: 5 mm/s").unwrap()).unwrap();
            speed_label.set_label_align(LabelAlign::Left).unwrap();

            let value = unsafe { lvgl_sys::lv_bar_set_range(speed_slider.raw().unwrap().as_mut(), 1500, 10_000) };
            unsafe { lvgl_sys::lv_bar_set_value(speed_slider.raw().unwrap().as_mut(), 10_000, 0) };
            speed_slider.on_event(|slider, event| {
                let value = unsafe { lvgl_sys::lv_slider_get_value(slider.raw().unwrap().as_ref()) };
                let value = (value as f32)/10000.0;
                let value = value*value*value;

                let value = value * 30.0;

                stepper.modify(|s| s.set_max_speed(Some(value.mm())));
            }).unwrap();

            (speed_slider, speed_label)
        };

        let mut position_label = {
            let mut position = Label::new(&mut screen).unwrap();
            position.set_align(&mut speed_label, Align::OutBottomLeft, 0, 0).unwrap();
            position.set_text(CString::new("Position: 0 mm").unwrap().as_c_str()).unwrap();
            position
        };

        loop {
            let text = {
                let mut string = arrayvec::ArrayString::<100>::new();
                let p = stepper.access(|s| s.current_position);
                let _ = write!(&mut string, "Position: {:.2} mm", p.as_mm());
                CString::new(&*string).unwrap()
            };
            position_label.set_text(&text).unwrap();

            let text = {
                let mut string = arrayvec::ArrayString::<100>::new();
                let p = stepper.access(|s| s.max_speed);
                let _ = write!(&mut string, "Max Speed: {:.2} mm/s", p.as_mm());
                CString::new(&*string).unwrap()
            };
            speed_label.set_text(&text).unwrap();

            ui.task_handler();

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
        */
    }


    #[task(priority = 5, binds = TIM7, shared = [stepper])]
    fn stepper_interrupt(mut ctx: stepper_interrupt::Context) {
        ctx.shared.stepper.lock(|s| s.on_interrupt());
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

    #[task(priority = 2, local = [display], shared = [touch_screen, last_touch_event])]
    fn touch_screen_sampling_task(mut ctx: touch_screen_sampling_task::Context) {
        use TouchScreenResult::*;
        match ctx.shared.touch_screen.lock(|ts| ts.on_delay_expired()) {
            DelayMs(delay_ms) => {
                touch_screen_sampling_task::spawn_after((delay_ms as u64).millis()).unwrap();
            },
            Done(touch_event) => {
                draw_touch_event(&mut ctx.local.display, touch_event.as_ref());
                ctx.shared.last_touch_event.lock(|t| *t = touch_event);
                unsafe { cortex_m::peripheral::NVIC::unmask(Interrupt::EXTI9_5); }
            },
        }
    }
}

fn draw_touch_event(display: &mut Display, touch_event: Option<&TouchEvent>) {
    use embedded_graphics::{prelude::*, primitives::{Circle, PrimitiveStyle}, pixelcolor::Rgb565};

    if let Some(touch_event) = touch_event {
        Circle::new(Point::new(touch_event.x as i32, touch_event.y as i32), 3)
            .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
            .draw(display).unwrap();
    }
}
