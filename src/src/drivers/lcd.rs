use nb::block;
use stm32f1xx_hal::{
    pac::SPI1,
    gpio::*,
    gpio::gpioa::*,
    gpio::gpiod::*,
    afio::MAPR,
    rcc::Clocks,
    prelude::*,
    spi::*,
    spi,
};

pub struct Lcd {
    cs: PA4<Output<PushPull>>,
    spi: Spi<
            SPI1,
            Spi1NoRemap,
            (
                PA5<Alternate<PushPull>>,
                PA6<Input<Floating>>,
                PA7<Alternate<PushPull>>
            ),
            u16,
         >,
}

impl Lcd {
    const COLS: u16 = 3840;
    const ROWS: u16 = 2400;

    pub fn new(
        reset: PD12<Input<Floating>>,
        cs: PA4<Input<Floating>>,
        sck: PA5<Input<Floating>>,
        miso: PA6<Input<Floating>>,
        mosi: PA7<Input<Floating>>,
        spi1: SPI1,
        clocks: &Clocks,
        gpioa_crl: &mut Cr<CRL, 'A'>,
        gpiod_crh: &mut Cr<CRH, 'D'>,
        mapr: &mut MAPR,
    ) -> Self {
        let reset = reset.into_push_pull_output_with_state(gpiod_crh, PinState::Low);
        let cs = cs.into_push_pull_output_with_state(gpioa_crl, PinState::High);

        let spi = {
            let sck = sck.into_alternate_push_pull(gpioa_crl);
            let miso = miso.into_floating_input(gpioa_crl);
            let mosi = mosi.into_alternate_push_pull(gpioa_crl);

            Spi::spi1(
                spi1,
                (sck, miso, mosi),
                mapr,
                spi::Mode { polarity: spi::Polarity::IdleLow, phase: spi::Phase::CaptureOnFirstTransition },
                clocks.pclk1()/2, // Run as fast as we can (60Mhz)
                *clocks,
            ).frame_size_16bit()
        };

        Self { cs, spi }
    }

    pub fn demo(&mut self) {
        self.draw_all_black();
        self.draw_waves();
        self.draw_all_black();
        self.draw_waves();
    }

    pub fn draw_all_black(&mut self) {
        self.draw(|row, col| { 0 })
    }

    pub fn draw_waves(&mut self) {
        self.draw(|row, col| {
            if row % 100 == 0 || col % 100 == 0 {
                0x0F
            } else {
                ((16*16 * row as u32 * col as u32) / (Self::ROWS as u32 * Self::COLS as u32)) as u8
            }
        })
    }

    pub fn draw(&mut self, f: impl Fn(u16, u16) -> u8) {
        self.cs.set_low();
        self.delay_150ns(10);

        self.cmd(Command::StartDraw);

        for row in 0..Self::ROWS {
            for col in 0..Self::COLS/4 {
                let color =
                    (((f(row, 4*col+0)&0x0F) as u16) << 12) |
                    (((f(row, 4*col+1)&0x0F) as u16) <<  8) |
                    (((f(row, 4*col+2)&0x0F) as u16) <<  4) |
                    (((f(row, 4*col+3)&0x0F) as u16) <<  0);
                self.spi.spi_write(&[color]).unwrap();
            }
        }

        self.delay_150ns(60);
        self.cs.set_high();
    }

    fn cmd(&mut self, cmd: Command) {
        match cmd {
            Command::GetVersion => {
                let mut version = [0_u16; 2];
                self.cmd_inner(cmd, &[], Some(&mut version));
                crate::debug!("version = {:?}", version);
                //let version = (version[1] as u32 << 16) | (version[0]);
            }
            Command::StartDraw => {
                // The FPGA seems a little buggy.
                // It won't take the command well. Apparently, we have to send it twice.
                // Otherwise, 2/3 of a second frame won't render. There's a strange bug.
                self.cmd_inner(cmd, &[], None);
                self.delay_150ns(60);
                self.cs.set_high();
                self.delay_150ns(6000); // 1ms delay
                self.cs.set_low();
                self.delay_150ns(10);
                self.cmd_inner(cmd, &[], None);
            }
            //0xFC => self.cmd_inner(0xF1, &[0; 16], None),
        }
    }

    fn cmd_inner(&mut self, cmd: Command, tx: &[u16], rx: Option<&mut [u16]>) {
        self.spi.spi_write(&[cmd as u16, 0]).unwrap();

        self.spi.spi_write(tx).unwrap();

        if let Some(rx) = rx {
            for v in rx {
                block!(self.spi.send(0)).unwrap();
                *v = block!(self.spi.read()).unwrap();
                self.delay_150ns(1);
            }
        }

        // check rx[0], it should be not nul.
    }

    pub fn delay_150ns(&self, count: usize) {
        cortex_m::asm::delay(20)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u16)]
enum Command {
    GetVersion = 0xF0,
    StartDraw = 0xFB,
}
