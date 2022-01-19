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
    // Set to 0 to disable the pressure detection.
    // 30% is too little. points can jump around.
    const TOUCH_PRESSURE_THRESHOLD: u32 = 35;

    pub fn new(
        cs: PC7<Input<Floating>>,
        sck: PC8<Input<Floating>>,
        miso: PC9<Input<Floating>>,
        mosi: PA8<Input<Floating>>,
        touch_active: PA9<Input<Floating>>,
        gpioa_crh: &mut Cr<CRH, 'A'>,
        gpioc_crl: &mut Cr<CRL, 'C'>,
        gpioc_crh: &mut Cr<CRH, 'C'>,
    ) -> Self {
        let cs = cs.into_push_pull_output_with_state(gpioc_crl, PinState::High);
        let sck = sck.into_push_pull_output(gpioc_crh);
        let miso = miso.into_floating_input(gpioc_crh);
        let mosi = mosi.into_push_pull_output(gpioa_crh);

        Self { cs, sck, miso, mosi, touch_active }
    }

    pub fn has_touch(&self) -> bool {
        self.touch_active.is_low()
    }

    pub fn read_x_y(&mut self, delay: &mut Delay) -> Option<(u16, u16)> {
        if !self.has_touch() {
            return None;
        }

        const NUM_SAMPLES: usize = 5;

        let mut xs = [0u16; NUM_SAMPLES];
        let mut ys = [0u16; NUM_SAMPLES];
        let mut index: usize = 0;
        let mut valid_samples: u16 = 0;

        // We're looking for a continuous stream of NUM_SAMPLES valid samples.
        // As soon as we do, we average the (x,y) values we got.
        // Samples are taken every 1ms.
        for _ in 0..10*NUM_SAMPLES {
            valid_samples <<= 1;

            if let Some((x,y)) = self.read_raw_x_y_once() {
                valid_samples |= 1;
                xs[index] = x;
                ys[index] = y;
                index += 1;
                if index == NUM_SAMPLES {
                    index = 0;
                }
            }

            if valid_samples == (1 << NUM_SAMPLES)-1  {
                let x_sum: u32 = xs.iter().map(|v| *v as u32).sum();
                let y_sum: u32 = ys.iter().map(|v| *v as u32).sum();

                let x = (x_sum / NUM_SAMPLES as u32) as u16;
                let y = (y_sum / NUM_SAMPLES as u32) as u16;

                const MAX_X: u16 = 1 << 12;
                let x = (MAX_X - x)/11 - 36;
                let y = y/15 - 15;

                return Some((x,y))
            }

            delay.delay_ms(1u8);
        }

        None
    }

    // Returns (x,y) coordinates if a touch is detected
    fn read_raw_x_y_once(&mut self) -> Option<(u16, u16)> {
        let x = self.read_cmd(0x90) >> 4; // swapping MSB -> LSB gives 0x09
        let y = self.read_cmd(0xD0) >> 4; // swapping MSB -> LSB gives 0x0B

        // Touch not detected
        if x < 10 || y < 10 {
            return None;
        }

        if Self::TOUCH_PRESSURE_THRESHOLD > 0 {
            let z = self.read_cmd(0x16);

            // z/y seems to measure pressure.
            if (100*z as u32) < (Self::TOUCH_PRESSURE_THRESHOLD * y as u32) {
                return None;
            }
        }

        Some((x,y))
    }

    fn read_cmd(&mut self, cmd: u8) -> u16 {
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
    fn exchange_data(&mut self, mut v: u8) -> u8 {
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
