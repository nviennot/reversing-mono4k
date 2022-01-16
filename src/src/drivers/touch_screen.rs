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
    pub clk: PC8<Output<PushPull>>,
    pub rx: PC9<Input<Floating>>,
    pub tx: PA8<Output<PushPull>>,
    pub touch: PA9<Input<Floating>>,
}

impl TouchScreen {
    pub fn has_touch(&self) -> bool {
        self.touch.is_low()
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

        const NUM_SAMPLES: usize = 16;
        let (mut x_sum, mut y_sum) = (0u32, 0u32);

        for _ in 0..NUM_SAMPLES {
            if let Some((x,y)) = self.read_x_y_once() {
                x_sum += x as u32;
                y_sum += y as u32;
            } else {
                return None;
            }
        }

        let x = (x_sum / NUM_SAMPLES as u32) as u16;
        let y = (y_sum / NUM_SAMPLES as u32) as u16;

        const MAX_X: u16 = 1 << 12;
        let x = (MAX_X - x)/11 - 36;
        let y = y/15 - 15;

        Some((x,y))
    }

    pub fn read_x_y_once(&mut self) -> Option<(u16, u16)> {
        let x = self.read_cmd(0x90) >> 4;
        let y = self.read_cmd(0xD0) >> 4;
        if x < 10 || y < 10 {
            None
        } else {
            Some((x,y))
        }
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

    // Serial data, MSB first
    pub fn exchange_data(&mut self, mut v: u8) -> u8 {
        let mut out: u8 = 0;

        self.clk.set_low();
        Self::delay(1);

        for _ in 0..8 {
            out <<= 1;
            if v & 0x80 != 0 {
                self.tx.set_high();
            } else {
                self.tx.set_low();
            }

            Self::delay(1); // added
            self.clk.set_high();

            Self::delay(1);
            self.clk.set_low();

            Self::delay(1);
            if self.rx.is_high() {
                out |= 1;
            }
            v <<= 1;
        }

        out
    }

    fn delay(num: u32) {
        // 20 was found from the original firmware.
        // It's pretty long. *2 is actually enough.
        cortex_m::asm::delay(num*20);
    }
}
