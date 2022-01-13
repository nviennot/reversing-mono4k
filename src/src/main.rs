#![no_std]
#![no_main]
#![allow(unused_imports)]

mod drivers;

#[macro_use]
mod macros {
    macro_rules! debug {
        ($($arg:expr),*) => ( cortex_m_semihosting::hprintln!($($arg),*).unwrap(); );
    }
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

use drivers::ext_flash::ExtFlash;

use spi_memory::series25::Flash;

struct Machine {
    ext_flash: ExtFlash,
}

use embedded_hal::digital::v2::OutputPin;


use drivers::clock;

impl Machine {
    pub fn init() -> Self {
        // Initialize the device to run at 48Mhz using the 8Mhz crystal on
        // the PCB instead of the internal oscillator.
        let cp = cortex_m::Peripherals::take().unwrap();
        let dp = Peripherals::take().unwrap();
        //let mut rcc = dp.RCC.constrain();
        let mut flash = dp.FLASH.constrain();

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

        //--------------------------
        //  External flash
        //--------------------------

        let mut ext_flash = {
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
                    clocks.pclk1(), // Run as fast as we can (60Mhz). The flash chip can go up to 133Mhz.
                    clocks,
                )
            };

            // Initialize the spi-memory library
            ExtFlash(Flash::init(spi, cs).unwrap())
        };


        //--------------------------
        //  TFT display
        //--------------------------

        let _tft_display = {

            unsafe {
                (*pac::RCC::ptr()).ahbenr.modify(|_,w| w.bits(1 << 8));
            }

            let mut backlight = gpioa.pa10.into_push_pull_output(&mut gpioa.crh);

            let mut notsure = gpioa.pa6.into_push_pull_output(&mut gpioa.crl);
            let mut reset = gpioc.pc6.into_push_pull_output(&mut gpioc.crl);

            {
                // PD4: EXMC_NOE: Output Enable
                gpiod.pd4.into_alternate_push_pull(&mut gpiod.crl);

                // PD5: EXMC_NWE: Write enable
                gpiod.pd5.into_alternate_push_pull(&mut gpiod.crl);

                // PD7: EXMC_NCE0: Chip select
                gpiod.pd7.into_alternate_push_pull(&mut gpiod.crl);

                // A16: Selects the Command or Data register
                // Via setting (1 << 16) in the address.
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
                        // Data latency 0 => 2 clk
                        .datlat().bits(0)
                        // syn clock divide ratio: 1
                        .clkdiv().bits(0)

                        // Bus latency
                        .busturn().bits(0)
                        // Data setup time
                        .datast().bits(1)
                        // Address setup time
                        .addset().bits(0)

                        /* Theirs
                        // Bus latency
                        .busturn().bits(3)
                        // Data setup time
                        .datast().bits(29)
                        // Address setup time
                        .addhld().bits(15)
                        // Address setup time
                        .addset().bits(15)
                        */
                    );
                }
            }

            backlight.set_high();

            delay.delay_ms(10u8);
            reset.set_high();
            delay.delay_ms(10u8);
            reset.set_low();
            delay.delay_ms(80u8);
            reset.set_high();
            delay.delay_ms(50u8);

            // Now send tft commands

            const TFT_REG: *mut u16 = 0x6000_0000u32 as *mut u16;
            const TFT_DATA: *mut u16 = 0x6002_0000u32 as *mut u16;
                fn cmd(cmd: u16, args: &[u16]) {
                    unsafe {
                        *TFT_REG = cmd;
                        for a in args {
                            *TFT_DATA = *a;
                        }
                    }
                }

                fn tft_command(cmd: u16) {
                    unsafe { TFT_REG.write_volatile(cmd); }
                }

                fn tft_args(cmd: u16) {
                    unsafe { TFT_DATA.write_volatile(cmd); }
                }

                tft_command(0xCF);
                tft_args(0);
                tft_args(0xC1);
                tft_args(0x30);
                delay.delay_us(80u8);
                tft_command(0xED);
                tft_args(0x64);
                tft_args(3);
                tft_args(0x12);
                tft_args(0x81);
                delay.delay_us(80u8);
                tft_command(0xE8);
                tft_args(0x85);
                tft_args(0x10);
                tft_args(0x7A);
                delay.delay_us(80u8);
                tft_command(0xCB);
                tft_args(57);
                tft_args(44);
                tft_args(0);
                tft_args(52);
                tft_args(2);
                delay.delay_us(80u8);
                tft_command(0xF7);
                tft_args(32);
                delay.delay_us(80u8);
                tft_command(0xEA);
                tft_args(0);
                tft_args(0);
                delay.delay_us(80u8);
                tft_command(0xC0);
                tft_args(27);
                delay.delay_us(80u8);
                tft_command(0xC1);
                tft_args(1);
                delay.delay_us(80u8);
                tft_command(0xC5);
                tft_args(48);
                tft_args(48);
                delay.delay_us(80u8);
                tft_command(0xC7);
                tft_args(0xB7);
                delay.delay_us(80u8);
                tft_command(0x3A);
                tft_args(0x55);
                delay.delay_us(80u8);
                tft_command(0x36);
                tft_args(0xA8);
                delay.delay_us(80u8);
                tft_command(0xB1);
                tft_args(0);
                tft_args(0x12);
                delay.delay_us(80u8);
                tft_command(0xB6);
                tft_args(10);
                tft_args(162);
                delay.delay_us(80u8);
                tft_command(0x44);
                tft_args(2);
                delay.delay_us(80u8);
                tft_command(0xF2);
                tft_args(0);
                delay.delay_us(80u8);
                tft_command(0x26);
                tft_args(1);
                delay.delay_us(80u8);
                tft_command(0xE0);
                tft_args(0xF);

                /*
                delay.delay_us(80u8);
                tft_command(0xE0);
                tft_args(0xF);
                tft_args(0x2A);
                tft_args(0x28);
                tft_args(8);

                tft_args(0xE);
                tft_args(8);
                tft_args(0x54);
                tft_args(0xA9);

                tft_args(0x43);
                tft_args(0xA);
                tft_args(0xF);
                tft_args(0);

                tft_args(0);
                tft_args(0);
                tft_args(0);
                delay.delay_us(80u8);
                tft_command(0xE1);
                tft_args(0);
                tft_args(21);
                tft_args(23);
                tft_args(7);
                tft_args(17);
                tft_args(6);
                tft_args(43);
                tft_args(86);
                tft_args(60);
                tft_args(5);
                tft_args(16);
                tft_args(15);
                tft_args(63);
                tft_args(63);
                tft_args(15);
                */

                delay.delay_us(80u8);

                tft_command(0x11);
                delay.delay_ms(8u8);

                tft_command(0x29);
                delay.delay_ms(1u8);

                fn set_region() {
                    tft_command(0x2A);
                    tft_args(0);
                    tft_args(0);
                    tft_args(319 >> 8);
                    tft_args(319);
                    tft_command(0x2B);
                    tft_args(0);
                    tft_args(0);
                    tft_args(239 >> 8);
                    tft_args(239);
                    tft_command(0x2C);
                }

                set_region();

                use spi_memory::prelude::*;

                const BUFFER_SIZE: usize = 1*1024;
                let mut buf = [0u8; BUFFER_SIZE];

                loop {
                    for img_offset in 0..30 {
                        for offset in (0..320*240*2).step_by(BUFFER_SIZE) {
                            ext_flash.0.read(img_offset*0x30000 + offset as u32, &mut buf).unwrap();
                            for i in 0..BUFFER_SIZE/2 {
                                tft_args(((buf[2*i+1] as u16) << 8) | buf[2*i] as u16);
                            }
                        }

                        delay.delay_ms(1000u32);
                    }
                }


                //delay.delay_ms(100u8);
                //pa10_backlight.set_high().unwrap();
            };

            Self { ext_flash }
        }
    }


fn main() -> ! {
    let mut machine = Machine::init();
    //machine.ext_flash.dump();
    debug!("done");
    loop {}
}

// For some reason, having #[entry] on main() makes auto-completion a bit broken.
// Adding a function call fixes it.
#[entry]
fn _main() -> ! { main() }
