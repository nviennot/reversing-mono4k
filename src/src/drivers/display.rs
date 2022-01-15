use stm32f1xx_hal::{
    prelude::*,
    gpio::*,
    gpio::gpioa::*,
    gpio::gpiob::*,
    gpio::gpioc::*,
    //pac::Peripherals,
    //rcc::{Clocks, APB1},
    delay::Delay,
};

use spi_memory::{
    prelude::*,
};

pub struct Display {
    pub reset: PC6<Output<PushPull>>,
    pub backlight: PA10<Output<PushPull>>,
}

impl Display {
    // We use Bank0 (0x60000000) to address the display.
    // The A16 wire is used to select the DATA or CMD register. Its address is
    // 0x00020000 = 1 << (16 + 1) (not sure why the +1).
    const TFT_CMD:  *mut u16 = 0x6000_0000u32 as *mut u16;
    const TFT_DATA: *mut u16 = 0x6002_0000u32 as *mut u16;
    pub const WIDTH: u16 = 320;
    pub const HEIGHT: u16 = 240;

    pub const FULL_SCREEN: Rectangle =
        Rectangle::new(
            Point::new(0,0),
            Size::new(Self::WIDTH as u32, Self::HEIGHT as u32)
        );

    pub fn write_cmd(&mut self, v: u16) {
        unsafe { Self::TFT_CMD.write_volatile(v); }
    }

    pub fn write_data(&mut self, v: u16) {
        unsafe { Self::TFT_DATA.write_volatile(v); }
    }

    pub fn init(&mut self, delay: &mut Delay) {
        delay.delay_ms(10u8);
        self.reset.set_high();
        delay.delay_ms(10u8);
        self.reset.set_low();
        delay.delay_ms(80u8);
        self.reset.set_high();
        delay.delay_ms(50u8);

        self.cmd(0xCF, &[0x00, 0xC1, 0x30]);
        self.cmd(0xED, &[0x64, 0x03, 0x12, 0x81]);
        self.cmd(0xE8, &[0x85, 0x10, 0x7A]);
        self.cmd(0xCB, &[0x39, 0x2C, 0x00, 0x34, 0x02]);
        self.cmd(0xF7, &[0x20]);
        self.cmd(0xEA, &[0x00,0x00]);
        self.cmd(0xC0, &[0x1B]);
        self.cmd(0xC1, &[0x01]);
        self.cmd(0xC5, &[0x30, 0x30]);
        self.cmd(0xC7, &[0xB7]);
        self.cmd(0x3A, &[0x55]);
        self.cmd(0x36, &[0xA8]);
        self.cmd(0xB1, &[0x00, 0x12]);
        self.cmd(0xB6, &[0x0A, 0xA2]);
        self.cmd(0x44, &[0x02]);
        self.cmd(0xF2, &[0x00]);

        // Gamma settings
        self.cmd(0x26, &[0x01]);
        self.cmd(0xE0, &[15, 42, 40, 8, 14, 8, 84, 169, 67, 10, 15, 0, 0, 0, 0]);
        self.cmd(0xE1, &[0, 21, 23, 7, 17, 6, 43, 86, 60, 5, 16, 15, 63, 63, 15]);

        // Sleep Out
        self.cmd(0x11, &[]);
        delay.delay_ms(8u8);

        // Display ON
        self.cmd(0x29, &[]);
        delay.delay_ms(1u8);

        self.fill_screen(0);

        delay.delay_ms(110u32);
        self.backlight.set_high();
    }

    pub fn write_data_as_two_u8(&mut self, v: u16) {
        self.write_data(v >> 8);
        self.write_data(v & 0xFF);
    }

    pub fn cmd(&mut self, cmd: u16, args: &[u16]) {
        self.write_cmd(cmd);
        for a in args {
            self.write_data(*a);
        }
    }

    pub fn start_drawing(&mut self, top_left: (u16, u16), bottom_right: (u16, u16)) {
        let (left, top) = top_left;
        let (right, bottom) =  bottom_right;

        self.write_cmd(0x2A);
        self.write_data_as_two_u8(left);
        self.write_data_as_two_u8(right - 1);
        self.write_cmd(0x2B);
        self.write_data_as_two_u8(top);
        self.write_data_as_two_u8(bottom - 1);
        self.write_cmd(0x2C);
    }

    pub fn start_drawing_full_screen(&mut self) {
        self.start_drawing((0,0), (Self::WIDTH, Self::HEIGHT));
    }

    pub fn fill_screen(&mut self, color: u16) {
        self.start_drawing_full_screen();
        for _ in 0..Self::WIDTH {
            for _ in 0..Self::HEIGHT {
                self.write_data(color);
            }
        }
    }

    pub fn draw_background_image(&mut self, ext_flash: &mut ExtFlash, img_index: u8, area: &Rectangle) {
        let area = area.intersection(&self.bounding_box());
        if area.is_zero_sized() {
            return;
        }

        let image_addr = 0x30000 * (img_index as u32);

        let width = area.size.width as u16;
        let left_col = area.top_left.x as u16;
        let right_col = left_col + width;

        const BYTES_PER_PIXEL: u32 = 2;

        let mut buf_ = [0u8; (BYTES_PER_PIXEL as usize)*Self::WIDTH as usize];

        for row in area.rows() {
            let buf = &mut buf_[0..(BYTES_PER_PIXEL as usize)*(width as usize)];
            let start_pixel_index = (row as u32) * (Self::WIDTH as u32) + left_col as u32;
            ext_flash.0.read(image_addr + BYTES_PER_PIXEL*start_pixel_index, buf).unwrap();

            let row = row as u16;
            self.start_drawing((left_col,  row),
                               (right_col, row+1));

            for i in 0..width {
                let i = i as usize;
                self.write_data(((buf[2*i+1] as u16) << 8) | buf[2*i] as u16);
            }
        }
    }
}


// Embedded Graphics integration

use core::convert::TryInto;
use embedded_graphics::{
    prelude::*,
    pixelcolor::{Rgb565, raw::RawU16},
    primitives::Rectangle,
};

use super::ext_flash::ExtFlash;

impl DrawTarget for Display {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            const W: i32 = Display::WIDTH as i32;
            const H: i32 = Display::HEIGHT as i32;
            if let Ok((x @ 0..=W, y @ 0..=H)) = coord.try_into() {
                // Calculate the index in the framebuffer.
                let x = x as u16;
                let y = y as u16;
                self.start_drawing((x,y), (x,y));
                self.write_data(RawU16::from(color).into_inner());
            }
        }

        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        // Clamp area to drawable part of the display target
        let drawable_area = area.intersection(&self.bounding_box());

        // Check that there are visible pixels to be drawn
        if drawable_area.size != Size::zero() {
            let start = drawable_area.top_left;
            let end = drawable_area.bottom_right().unwrap();
            self.start_drawing((start.x as u16, start.y as u16),
                               (end.x as u16,   end.y as u16));

            area.points()
                .zip(colors)
                .filter(|(pos, _color)| drawable_area.contains(*pos))
                .for_each(|(_, color)| self.write_data(RawU16::from(color).into_inner()));
        }
        Ok(())
    }
}

impl OriginDimensions for Display {
    fn size(&self) -> Size {
        Size::new(Self::WIDTH.into(), Self::HEIGHT.into())
    }
}
