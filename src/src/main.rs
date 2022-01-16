#![no_std]
#![no_main]
#![allow(unused_imports)]

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
    pac::{Peripherals, self},
    gpio::PinState,
    spi::{self, *}, delay::Delay, rcc::Clocks,
};

use drivers::{
    ext_flash::ExtFlash,
    display::Display,
    touch_screen::TouchScreen,
};

use spi_memory::series25::Flash;

struct Machine {
    ext_flash: ExtFlash,
    display: Display,
    touch_screen: TouchScreen,
    delay: Delay,
}

use embedded_hal::digital::v2::OutputPin;


use drivers::clock;

use macros::debug;

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

        debug!("delay: {:#?}", clocks);

        //--------------------------
        //  External flash
        //--------------------------

        let ext_flash = {
            let cs = gpiob.pb12.into_push_pull_output_with_state(&mut gpiob.crh, PinState::High);

            let spi = {
                let sck = gpiob.pb13.into_alternate_push_pull(&mut gpiob.crh);
                let miso = gpiob.pb14.into_floating_input(&mut gpiob.crh);
                let mosi = gpiob.pb15.into_alternate_push_pull(&mut gpiob.crh);

                Spi::spi2(
                    dp.SPI2,
                    (sck, miso, mosi),
                    // For the SPI mode, I just picked the first option.
                    spi::Mode { polarity: spi::Polarity::IdleLow, phase: spi::Phase::CaptureOnFirstTransition },
                    clocks.pclk1()/2, // Run as fast as we can (30Mhz). The flash chip can go up to 133Mhz.
                    clocks,
                )
            };

            // Initialize the spi-memory library
            ExtFlash(Flash::init(spi, cs).unwrap())
        };


        //--------------------------
        //  TFT display
        //--------------------------

        let display = {
            let reset = gpioc.pc6.into_push_pull_output(&mut gpioc.crl);
            let _notsure = gpioa.pa6.into_push_pull_output(&mut gpioa.crl);
            let backlight = gpioa.pa10.into_push_pull_output(&mut gpioa.crh);

            // This initializes the EXMC module for the TFT display
            {

                unsafe {
                    // Enables the EXMC module
                    (*pac::RCC::ptr()).ahbenr.modify(|_,w| w.bits(1 << 8));
                }
                // PD4: EXMC_NOE: Output Enable
                gpiod.pd4.into_alternate_push_pull(&mut gpiod.crl);

                // PD5: EXMC_NWE: Write enable
                gpiod.pd5.into_alternate_push_pull(&mut gpiod.crl);

                // PD7: EXMC_NE0: Chip select
                gpiod.pd7.into_alternate_push_pull(&mut gpiod.crl);

                // A16: Selects the Command or Data register
                gpiod.pd11.into_alternate_push_pull(&mut gpiod.crh);

                // PD14..15: EXMC_D0..1
                // PD0..1:   EXMC_D2..3
                // PE7..15:  EXMC_D4..12
                // PD8..10:  EXMC_D13..15
                gpiod.pd14.into_alternate_push_pull(&mut gpiod.crh);
                gpiod.pd15.into_alternate_push_pull(&mut gpiod.crh);
                gpiod.pd0.into_alternate_push_pull(&mut gpiod.crl);
                gpiod.pd1.into_alternate_push_pull(&mut gpiod.crl);
                gpioe.pe7.into_alternate_push_pull(&mut gpioe.crl);
                gpioe.pe8.into_alternate_push_pull(&mut gpioe.crh);
                gpioe.pe9.into_alternate_push_pull(&mut gpioe.crh);
                gpioe.pe10.into_alternate_push_pull(&mut gpioe.crh);
                gpioe.pe11.into_alternate_push_pull(&mut gpioe.crh);
                gpioe.pe12.into_alternate_push_pull(&mut gpioe.crh);
                gpioe.pe13.into_alternate_push_pull(&mut gpioe.crh);
                gpioe.pe14.into_alternate_push_pull(&mut gpioe.crh);
                gpioe.pe15.into_alternate_push_pull(&mut gpioe.crh);
                gpiod.pd8.into_alternate_push_pull(&mut gpiod.crh);
                gpiod.pd9.into_alternate_push_pull(&mut gpiod.crh);
                gpiod.pd10.into_alternate_push_pull(&mut gpiod.crh);

                unsafe {
                    dp.FSMC.bcr1.write(|w| w
                        // Enable NOR Bank 0
                        .mbken().set_bit()
                        // data width: 16 bits
                        .mwid().bits(1)
                        // write: enable
                        .wren().set_bit()
                    );
                    dp.FSMC.btr1.write(|w| w
                        // Access Mode A
                        .accmod().bits(0)
                        // Address setup time: not needed.
                        .addset().bits(0)
                        // Data setup and hold time.
                        // (2+1)/120MHz = 25ns. Should be plenty enough.
                        // Typically, 10ns is the minimum.
                        .datast().bits(2)
                        .datlat().bits(2)
                    );
                }
            }

            let mut display = Display { reset, backlight };
            display.init(&mut delay);
            display
        };

        let touch_screen = {
            let cs = gpioc.pc7.into_push_pull_output_with_state(&mut gpioc.crl, PinState::High);
            let clk = gpioc.pc8.into_push_pull_output(&mut gpioc.crh);
            let rx = gpioc.pc9.into_floating_input(&mut gpioc.crh);
            let tx = gpioa.pa8.into_push_pull_output(&mut gpioa.crh);
            let touch = gpioa.pa9.into_floating_input(&mut gpioa.crh);

            TouchScreen { cs, clk, rx, tx, touch }
        };

        Self { ext_flash, display, touch_screen, delay }
    }
}


fn main() -> ! {
    let mut machine = Machine::init();

    let display = &mut machine.display;
    let mut ext_flash = &mut machine.ext_flash;
    let delay = &mut machine.delay;

    display.draw_background_image(&mut ext_flash, 15, &Display::FULL_SCREEN);

    let mut count = 0;
    loop {
        use embedded_graphics::{
            mono_font::{ascii::FONT_9X18_BOLD, MonoTextStyle},
            pixelcolor::Rgb565,
            prelude::*,
            text::{Text, Alignment},
            primitives::{Circle, PrimitiveStyle, PrimitiveStyleBuilder},
        };

        let style = PrimitiveStyle::with_fill(Rgb565::MAGENTA);

        if let Some((x,y)) = machine.touch_screen.read_x_y(delay) {
            count = 0;

            let x = x as i32;
            let y = y as i32;

            Circle::new(Point::new(x, y), 10)
                .into_styled(style)
                .draw(display).unwrap();
        }

        count +=1;
        if count > 10000000 {
            count = 0;
            display.draw_background_image(&mut ext_flash, 15, &Display::FULL_SCREEN);
        }
    }



    loop {
        for img_offset in 0..30 {
            display.draw_background_image(&mut ext_flash, img_offset, &Display::FULL_SCREEN);
            delay.delay_ms(2000u32);
        }
    }

    {
        use embedded_graphics::{
            mono_font::{ascii::FONT_9X18_BOLD, MonoTextStyle},
            pixelcolor::Rgb565,
            prelude::*,
            text::{Text, Alignment},
        };

        // Create a new character style
        let style = MonoTextStyle::new(&FONT_9X18_BOLD, Rgb565::WHITE);

        // Create a text at position (20, 30) and draw it using the previously defined style
        let mut text = Text::with_alignment(
            "We are in!!",
            Point::new(100,100),
            style,
            Alignment::Left
        );

        display.draw_background_image(&mut ext_flash, 15, &Display::FULL_SCREEN);

        let mut translate_by = Point::new(1,1);

        loop {
            text.translate_mut(translate_by);
            text.draw(display).unwrap();

            let bb = text.bounding_box();

            delay.delay_ms(20u8);

            display.draw_background_image(&mut ext_flash, 15, &bb);

            let top_left = bb.top_left;
            let bottom_right = bb.bottom_right().unwrap();

            translate_by.x = match (translate_by.x > 0, top_left.x == 0, bottom_right.x as u16 == Display::WIDTH) {
                (true, _, true) => -translate_by.x,
                (false, true, _) => -translate_by.x,
                _ => translate_by.x,
            };

            translate_by.y = match (translate_by.y > 0, top_left.y == 0, bottom_right.y as u16 == Display::HEIGHT) {
                (true, _, true) => -translate_by.y,
                (false, true, _) => -translate_by.y,
                _ => translate_by.y,
            };
        }
    }

    //machine.ext_flash.dump();
}

// For some reason, having #[entry] on main() makes auto-completion a bit broken.
// Adding a function call fixes it.
#[entry]
fn _main() -> ! { main() }
