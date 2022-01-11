#![no_std]
#![no_main]
#![allow(unused_imports)]

mod drivers;

use cortex_m_rt::entry;

// pick a panicking behavior
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
// use panic_itm as _; // logs messages over ITM; requires ITM support
use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

use stm32f1xx_hal::{
    prelude::*,
    pac::Peripherals,
};

use drivers::ext_flash::ExtFlash;

use stm32f1xx_hal::{
    prelude::*,
    gpio::*,
    spi::{self, *},
};

use spi_memory::series25::Flash;

struct Machine {
    ext_flash: ExtFlash,
}

impl Machine {
    pub fn init() -> Self {
        // Initialize the device to run at 48Mhz using the 8Mhz crystal on
        // the PCB instead of the internal oscillator.
        let dp = Peripherals::take().unwrap();
        let mut rcc = dp.RCC.constrain();
        let mut flash = dp.FLASH.constrain();
        let clocks = rcc.cfgr
            .use_hse(8.mhz())
            .sysclk(48.mhz())
            .freeze(&mut flash.acr);

        let mut gpiob = dp.GPIOB.split(&mut rcc.apb2);

        let ext_flash = {
            let cs = gpiob.pb12.into_push_pull_output_with_state(&mut gpiob.crh, State::High);

            let spi = {
                let sck = gpiob.pb13.into_alternate_push_pull(&mut gpiob.crh);
                let miso = gpiob.pb14.into_floating_input(&mut gpiob.crh);
                let mosi = gpiob.pb15.into_alternate_push_pull(&mut gpiob.crh);

                Spi::spi2(
                    dp.SPI2,
                    (sck, miso, mosi),
                    // For the SPI mode, I just picked the first option.
                    spi::Mode { polarity: spi::Polarity::IdleLow, phase: spi::Phase::CaptureOnFirstTransition },
                    clocks.pclk1(), // Run as fast as we can. The flash chip can go up to 133Mhz.
                    clocks,
                    &mut rcc.apb1,
                )
            };

            // Initialize the spi-memory library
            ExtFlash(Flash::init(spi, cs).unwrap())
        };

        Self { ext_flash }
    }
}


fn main() -> ! {
    let mut machine = Machine::init();
    machine.ext_flash.dump();
    loop {}
}

// For some reason, having #[entry] on main() makes auto-completion a bit broken.
// Adding a function call fixes it.
#[entry]
fn _main() -> ! { main() }
