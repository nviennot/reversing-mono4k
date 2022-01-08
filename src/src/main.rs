#![no_std]
#![no_main]
#![allow(unused_imports)]

// pick a panicking behavior
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
// use panic_itm as _; // logs messages over ITM; requires ITM support
use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;

use gd32f3::gd32f307;

#[entry]
fn main() -> ! {
    use cortex_m_semihosting::{hio::open, nr};
    let mut file = open("hello_world.bin\0", nr::open::RW_TRUNC_BINARY).unwrap();
    file.write_all(b"We can send binaries").unwrap();
    loop {}
}


/*
use cortex_m_semihosting::{hio::open, nr};
// For some reason, having #[entry] on main() makes auto-completion a bit broken.
// Having a function call fixes it.
#[entry]
fn _main() -> ! { main() }

fn main() -> ! {
    let peripherals = gd32f307::Peripherals::take().unwrap();
    hprintln!("Hello, world!");

    let peripherals = gd32f307::Peripherals::take().unwrap();
    hprintln!("PORTA: {:x} {:x}",
        peripherals.GPIOA.ctl0.read().bits(),
        peripherals.GPIOA.ctl1.read().bits(),
    );

    dbg!(peripherals.GPIOA.ctl0.read().bits(),
        peripherals.GPIOA.ctl1.read().bits(),
    );

    //cortex_m_semihosting::hio::open();
    //peripherals.GPIOA.

    let mut file = open("hello_world.bin\0", nr::open::RW_TRUNC_BINARY).unwrap();
    file.write_all(b"We can send binaries").unwrap();

    panic!("We are done");
}

*/
