use cortex_m::peripheral::{SYST, syst::SystClkSource};
use stm32f1xx_hal::{
    time::Hertz,
    rcc::Clocks,
};

use gd32f3::gd32f307::{
    PMU,
    RCU,
};

use stm32f1xx_hal::{
    prelude::*,
};

use cortex_m_rt::exception;

// incremented every 10us
const TICK_DURATION_US: u32 = 10;
static mut TICKS: u64 = 0;
static mut SYSTICK: Option<SYST> = None;

#[exception]
fn SysTick() {
    unsafe { TICKS += 1; }
}

pub fn get_ticks() -> u64 {
    cortex_m::interrupt::free(|_| unsafe { TICKS })
}

use core::time::Duration;
pub fn get_boot_time() -> Duration {
    Duration::from_micros(get_ticks() * 10)
}


/*
pub fn clock_cycles() -> u64 {
    let syst = unsafe { SYSTICK.as_mut().unwrap() };
    let reload_value = syst.rvr.read();
    let current_value = syst.cvr.read();

    cortex_m::interrupt::free(|_| {
        let ticks = unsafe { TICKS };
        syst.
    });
    0
}
*/

pub fn init_systick(clocks: &Clocks, mut syst: SYST) {
    const MAX_RVR: u32 = 0x00FF_FFFF;
    const NUM_US_IN_SEC: u32 = 1_000_000;
    let reload_value = TICK_DURATION_US * clocks.sysclk().0 / NUM_US_IN_SEC;
    assert!(reload_value < MAX_RVR);

    // configures the system timer to trigger a SysTick exception every second
    syst.set_clock_source(SystClkSource::Core);
    syst.set_reload(reload_value);
    syst.clear_current();
    syst.enable_counter();
    syst.enable_interrupt();

    unsafe { SYSTICK = Some(syst) };
}
