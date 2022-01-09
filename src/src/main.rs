#![no_std]
#![no_main]
#![allow(unused_imports)]

mod hio;

// pick a panicking behavior
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
// use panic_itm as _; // logs messages over ITM; requires ITM support
use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;

use embedded_hal::digital::v2::OutputPin;

use stm32f1xx_hal::{
    spi,
    prelude::*,
    pac::Peripherals,
};

use spi_memory::{
    series25::Flash,
    prelude::*,
};

use crate::hio::nr::open;

fn main() -> ! {
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

    // The example in the spi-memory library uses a CS GPIO
    // We have one, great!
    let cs = gpiob.pb12.into_push_pull_output(&mut gpiob.crh);

    // The SPI module
    let spi = {
        let sck = gpiob.pb13.into_alternate_push_pull(&mut gpiob.crh);
        let miso = gpiob.pb14.into_floating_input(&mut gpiob.crh);
        let mosi = gpiob.pb15.into_alternate_push_pull(&mut gpiob.crh);

        spi::Spi::spi2(
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
    let mut flash = Flash::init(spi, cs).unwrap();

    // And read the chip JEDEC ID. In the datasheet, the first byte should be
    // 0xEF, which corresponds to Winbond
    let id = flash.read_jedec_id().unwrap();
    hprintln!("id={:?}", id).unwrap();

    const FLASH_SIZE: u32 = 16*1024*1024; // 16MB
    const BUFFER_SIZE: usize = 32*1024; // 32KB
    let mut buf = [0; BUFFER_SIZE];

    let mut file = hio::open("ext.bin\0", open::RW_TRUNC_BINARY).unwrap();

    for addr in (0..FLASH_SIZE).step_by(BUFFER_SIZE) {
        flash.read(addr, &mut buf).unwrap();
        file.write_all(&buf).unwrap();
    }

    hprintln!("DONE").unwrap();

    loop {}
}

// For some reason, having #[entry] on main() makes auto-completion a bit broken.
// Adding a function call fixes it.
#[entry]
fn _main() -> ! { main() }
