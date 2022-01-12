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
    gpio::*,
    spi::{self, *}, delay::Delay,
};

use drivers::ext_flash::ExtFlash;

use spi_memory::series25::Flash;

struct Machine {
    ext_flash: ExtFlash,
}

use embedded_hal::digital::v2::OutputPin;


/*




*/



impl Machine {
    pub fn init() -> Self {
        // Initialize the device to run at 48Mhz using the 8Mhz crystal on
        // the PCB instead of the internal oscillator.
        let cp = cortex_m::Peripherals::take().unwrap();
        let dp = Peripherals::take().unwrap();
        let mut rcc = dp.RCC.constrain();
        let mut flash = dp.FLASH.constrain();

        let clocks = rcc.cfgr
            .use_hse(8.mhz())
            .sysclk(48.mhz())
            .freeze(&mut flash.acr);

        let delay = Delay::new(cp.SYST, clocks);

        let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);
        let mut gpiob = dp.GPIOB.split(&mut rcc.apb2);
        let mut gpioc = dp.GPIOC.split(&mut rcc.apb2);
        let mut gpiod = dp.GPIOD.split(&mut rcc.apb2);
        let mut gpioe = dp.GPIOE.split(&mut rcc.apb2);

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


        let mut pa10_backlight = gpioa.pa10.into_push_pull_output(&mut gpioa.crh);

        let mut pa6 = gpioa.pa6.into_push_pull_output(&mut gpioa.crl);
        let mut pc6 = gpioc.pc6.into_push_pull_output(&mut gpioc.crl);
        //pa6.set_high().unwrap(); // At some point?

        // 7 more pins

        // 20 pins for the ram controller
        gpiod.pd0.into_alternate_push_pull(&mut gpiod.crl);
        gpiod.pd1.into_alternate_push_pull(&mut gpiod.crl);
        gpiod.pd4.into_alternate_push_pull(&mut gpiod.crl);
        gpiod.pd5.into_alternate_push_pull(&mut gpiod.crl);
        gpiod.pd7.into_alternate_push_pull(&mut gpiod.crl);
        gpiod.pd8.into_alternate_push_pull(&mut gpiod.crh);
        gpiod.pd9.into_alternate_push_pull(&mut gpiod.crh);
        gpiod.pd10.into_alternate_push_pull(&mut gpiod.crh);
        gpiod.pd11.into_alternate_push_pull(&mut gpiod.crh);
        gpiod.pd14.into_alternate_push_pull(&mut gpiod.crh);
        gpiod.pd15.into_alternate_push_pull(&mut gpiod.crh);
        gpioe.pe7.into_alternate_push_pull(&mut gpioe.crl);
        gpioe.pe8.into_alternate_push_pull(&mut gpioe.crh);
        gpioe.pe9.into_alternate_push_pull(&mut gpioe.crh);
        gpioe.pe10.into_alternate_push_pull(&mut gpioe.crh);
        gpioe.pe11.into_alternate_push_pull(&mut gpioe.crh);
        gpioe.pe12.into_alternate_push_pull(&mut gpioe.crh);
        gpioe.pe13.into_alternate_push_pull(&mut gpioe.crh);
        gpioe.pe14.into_alternate_push_pull(&mut gpioe.crh);
        gpioe.pe15.into_alternate_push_pull(&mut gpioe.crh);

        unsafe {
            dp.FSMC.bcr1.write(|w| w
                .wren().set_bit()
                .mwid().bits(0b01)
                .mbken().set_bit()
            );
        }

        // systick_wait 10_000
        pc6.set_high().unwrap();
        // systick_wait 10_000
        pc6.set_low().unwrap();
        // systick_wait 80_000
        pc6.set_high().unwrap();
        // systick_wait 50_000

        // Now send tft commands


        //rcc.apb1


        pa10_backlight.set_high().unwrap();

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
