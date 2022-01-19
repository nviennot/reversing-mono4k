#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![allow(unused_imports, dead_code, unused_variables, unused_macros, unreachable_code)]

mod drivers;


pub mod macros {
    macro_rules! debug {
        () => {
            cortex_m_semihosting::hprintln!("").unwrap();
        };
        ($s:expr) => {
            cortex_m_semihosting::hprintln!($s).unwrap();
        };
        ($s:expr, $($tt:tt)*) => {
            {
                use core::fmt::Write;

                let mut string = arrayvec::ArrayString::<1024>::new();
                let _ = write!(&mut string, concat!($s, "\n"), $($tt)*);
                cortex_m_semihosting::hprint!(&string).unwrap();
            }
        };
    }
    pub(crate) use debug;
}

use cortex_m_rt::entry;

// pick a panicking behavior
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
// use panic_itm as _; // logs messages over ITM; requires ITM support
use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

use stm32f1xx_hal::{
    prelude::*,
    pac::{Peripherals, self, interrupt, Interrupt, TIM7},
    gpio::PinState,
    spi::{self, *},
    delay::Delay,
    rcc::Clocks,
    timer::{Timer, Tim2NoRemap, Event, CountDownTimer},
    pwm::Channel,

};

use drivers::{
    ext_flash::ExtFlash,
    display::Display,
    touch_screen::TouchScreen,
    stepper::{prelude::*, Direction, Stepper, InterruptibleStepper},
};

use spi_memory::series25::Flash;

struct Machine {
    ext_flash: ExtFlash,
    display: Display,
    touch_screen: TouchScreen,
    stepper: InterruptibleStepper,
    delay: Delay,
}

use embedded_hal::digital::v2::OutputPin;


use drivers::clock;

use macros::debug;

use core::{cell::RefCell, time::Duration, mem::MaybeUninit};
use cortex_m::interrupt::Mutex;

impl Machine {
    pub fn init() -> Self {
        // Initialize the device to run at 48Mhz using the 8Mhz crystal on
        // the PCB instead of the internal oscillator.
        let cp = cortex_m::Peripherals::take().unwrap();
        let dp = Peripherals::take().unwrap();

        let mut gpioa = dp.GPIOA.split();
        let mut gpiob = dp.GPIOB.split();
        let mut gpioc = dp.GPIOC.split();
        let mut gpiod = dp.GPIOD.split();
        let mut gpioe = dp.GPIOE.split();

        let mut afio = dp.AFIO.constrain();


        // Note, we can't use separate functions, because we are consuming (as
        // in taking ownership of) the device peripherals struct, and so we
        // cannot pass it as arguments to a function, as it would only be
        // partially valid.

        //--------------------------
        //  Clock configuration
        //--------------------------

        // Can't use the HAL. The GD32 is too different.
        let clocks = clock::setup_clock_120m_hxtal(dp.RCC);
        let mut delay = Delay::new(cp.SYST, clocks);
        //init_systick(&clocks, cp.SYST);

        //--------------------------
        //  External flash
        //--------------------------

        let ext_flash = ExtFlash::new(
            gpiob.pb12, gpiob.pb13, gpiob.pb14, gpiob.pb15,
            dp.SPI2,
            &clocks, &mut gpiob.crh
        );

        //--------------------------
        //  TFT display
        //--------------------------

        //let _notsure = gpioa.pa6.into_push_pull_output(&mut gpioa.crl);
        let mut display = Display::new(
            gpioc.pc6, gpioa.pa10,
            gpiod.pd4, gpiod.pd5, gpiod.pd7, gpiod.pd11,
            gpiod.pd14, gpiod.pd15, gpiod.pd0, gpiod.pd1, gpioe.pe7, gpioe.pe8,
            gpioe.pe9, gpioe.pe10, gpioe.pe11, gpioe.pe12, gpioe.pe13,
            gpioe.pe14, gpioe.pe15, gpiod.pd8, gpiod.pd9, gpiod.pd10,
            dp.FSMC,
            &mut gpioa.crh, &mut gpioc.crl, &mut gpiod.crl, &mut gpiod.crh, &mut gpioe.crl, &mut gpioe.crh,
        );
        display.init(&mut delay);

        //--------------------------
        //  Touch screen
        //--------------------------

        let touch_screen = TouchScreen::new(
            gpioc.pc7, gpioc.pc8, gpioc.pc9, gpioa.pa8, gpioa.pa9,
            &mut gpioa.crh, &mut gpioc.crl, &mut gpioc.crh,
        );

        //--------------------------
        //  Stepper motor (Z-axis)
        //--------------------------

        let stepper = Stepper::new(
            gpioe.pe4, gpioe.pe5, gpioe.pe6,
            gpioc.pc3, gpioc.pc0,
            gpioc.pc1, gpioc.pc2,
            gpioa.pa3,
            Timer::new(dp.TIM2, &clocks), Timer::new(dp.TIM7, &clocks),
            &mut gpioa.crl, &mut gpioc.crl, &mut gpioe.crl, &mut afio.mapr,
        ).interruptible();

        Self { ext_flash, display, touch_screen, stepper, delay }
    }
}

struct TouchGlobal {
    touch_screen: TouchScreen,
    delay: Delay,
}

static mut TOUCH: Option<TouchGlobal> = None;

fn main() -> ! {
    let start = cortex_m_rt::heap_start() as usize;
    let size = 70*1024;
    unsafe { ALLOCATOR.init(start, size) }


    let machine = Machine::init();

    let mut display = machine.display;
    let ext_flash = machine.ext_flash;
    let delay = machine.delay;
    let mut stepper = machine.stepper;
    let touch_screen = machine.touch_screen;

    unsafe {
        let delay = core::mem::transmute_copy(&delay);
        TOUCH = Some(TouchGlobal { touch_screen, delay });
    }

    //machine.touch_screen.demo();

    //display.draw_background_image(&mut ext_flash, 15, &Display::FULL_SCREEN);
    display.backlight.set_high();

    /*
    for position in [(-100.0).mm(), 100.0.mm()] {
        machine.stepper.modify(|s| { s.set_target_relative(position); });
        machine.stepper.wait_for_completion();
        delay.delay_ms(1000u32);
    }
    */

    {
        use core::fmt::Write;
        use cstr_core::CString;

        use lvgl::*;
        use lvgl::style::*;
        use lvgl::widgets::*;

        use embedded_graphics::{
            prelude::*,
            pixelcolor::{Rgb565, raw::RawU16},
            primitives::Rectangle,
        };

        let mut ui = UI::init().unwrap();
        ui.disp_drv_register(display).unwrap();
        let mut screen = ui.scr_act().unwrap();

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
            label.set_text(CString::new("Turbo Resin v0.1.0").unwrap().as_c_str()).unwrap();
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
}

// For some reason, having #[entry] on main() makes auto-completion a bit broken.
// Adding a function call fixes it.
#[entry]
fn _main() -> ! { main() }

use alloc_cortex_m::CortexMHeap;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

#[alloc_error_handler]
fn oom(_: core::alloc::Layout) -> ! {
    debug!("OOM");
    loop {}
}
