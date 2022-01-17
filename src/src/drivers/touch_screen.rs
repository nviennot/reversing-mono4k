use stm32f1xx_hal::{
    prelude::*,
    gpio::*,
    gpio::gpioa::*,
    gpio::gpioc::*,
    delay::Delay,
};

use spi_memory::{
    prelude::*,
    series25::Flash,
};

use crate::debug;

pub struct TouchScreen {
    pub cs: PC7<Output<PushPull>>,
    pub sck: PC8<Output<PushPull>>,
    pub miso: PC9<Input<Floating>>,
    pub mosi: PA8<Output<PushPull>>,
    pub touch_active: PA9<Input<Floating>>,
}

impl TouchScreen {
    const TOUCH_PRESSURE_THRESHOLD: u32 = 30; // 40%

    const NUM_SAMPLES_DEBOUNCE: usize = 16;
    const SAMPLE_DELAY_US: u32 = 1_000;

    pub fn has_touch(&self) -> bool {
        self.touch_active.is_low()
    }

    pub fn is_touch_stable(&mut self, delay: &mut Delay) -> bool {
        const WAIT_MILLIS: usize = 10;

        if !self.has_touch() {
            return false;
        }

        let mut missed_count = 0u8;

        for _ in 0..WAIT_MILLIS {
            delay.delay_ms(1u8);
            if !self.has_touch() {
                missed_count += 1;
                if missed_count > 2 {
                    return false;
                }
            }
        }

        true
    }

    pub fn read_x_y(&mut self, delay: &mut Delay) -> Option<(u16, u16)> {
        if !self.is_touch_stable(delay) {
            return None;
        }

        let (mut x_sum, mut y_sum) = (0u32, 0u32);

        for _ in 0..Self::NUM_SAMPLES_DEBOUNCE {
            if let Some((x,y)) = self.read_x_y_once() {
                x_sum += x as u32;
                y_sum += y as u32;
            } else {
                return None;
            }
        }

        let x = (x_sum / Self::NUM_SAMPLES_DEBOUNCE as u32) as u16;
        let y = (y_sum / Self::NUM_SAMPLES_DEBOUNCE as u32) as u16;

        const MAX_X: u16 = 1 << 12;
        let x = (MAX_X - x)/11 - 36;
        let y = y/15 - 15;

        Some((x,y))
    }

    // Returns (x,y) coordinates if a touch is detected
    pub fn read_x_y_once(&mut self) -> Option<(u16, u16)> {
        // The touch wire better be on
        if !self.has_touch() {
            return None;
        }

        let x = self.read_cmd(0x90) >> 4; // swapping MSB -> LSB gives 0x09
        let y = self.read_cmd(0xD0) >> 4; // swapping MSB -> LSB gives 0x0B
        let z = self.read_cmd(0x16);

        // Touch not detected
        if x < 10 || y < 10 {
            return None;
        }

        // z/y seems to measure pressure.
        // 40% is a reasonnable threshold.
        if (100*z as u32) < (Self::TOUCH_PRESSURE_THRESHOLD * y as u32) {
            return None;
        }

        Some((x,y))
    }

    pub fn read_cmd(&mut self, cmd: u8) -> u16 {
        self.cs.set_low();

        Self::delay(2);
        self.exchange_data(cmd);
        Self::delay(10);

        let high_bits = self.exchange_data(0) as u16;
        let low_bits = self.exchange_data(0) as u16;
        let result = (high_bits << 8) | (low_bits);

        self.cs.set_high();

        Self::delay(6);

        result
    }

    pub fn write_cmd(&mut self, cmd: u8, data: u16) {
        self.cs.set_low();

        Self::delay(2);
        self.exchange_data(cmd);
        Self::delay(10);

        self.exchange_data((data >> 8) as u8);
        self.exchange_data(data as u8);

        self.cs.set_high();

        Self::delay(6);
    }

    // Serial data, MSB first
    pub fn exchange_data(&mut self, mut v: u8) -> u8 {
        let mut out: u8 = 0;

        self.sck.set_low();
        Self::delay(1);

        for _ in 0..8 {
            out <<= 1;
            if v & 0x80 != 0 {
                self.mosi.set_high();
            } else {
                self.mosi.set_low();
            }

            Self::delay(1); // added
            self.sck.set_high();

            Self::delay(1);
            self.sck.set_low();

            Self::delay(1);
            if self.miso.is_high() {
                out |= 1;
            }
            v <<= 1;
        }

        out
    }

    fn delay(num: u32) {
        // 20 was found from the original firmware.
        // It's pretty long. 2 is actually enough.
        cortex_m::asm::delay(num*20);
    }
}
