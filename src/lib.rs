#![no_std]
use core::convert::Infallible;

use embedded_graphics::{
    pixelcolor::{Gray4, raw::RawU4},
    prelude::*,
};
use embedded_hal::{delay::DelayNs, digital::OutputPin, spi::SpiDevice};

const WIDTH: u8 = 128;
const HEIGHT: u8 = 64;
const BUFFER_SIZE: usize = WIDTH as usize * HEIGHT as usize;
const SSD1322_CMD_COL_ADDRESS: u8 = 0x15;
const SSD1322_CMD_WRITE_RAM: u8 = 0x5C;
const SSD1322_CMD_ROW_ADDRESS: u8 = 0x75;
const SSD1322_CMD_MUX_RATIO: u8 = 0xCA;
const SSD1322_CMD_REMAP: u8 = 0xA0;
const SSD1322_CMD_START_LINE: u8 = 0xA1;
const SSD1322_CMD_OFFSET_LINE: u8 = 0xA2;
const SSD1322_CMD_MODE_ALL_OFF: u8 = 0xA4;
const SSD1322_CMD_MODE_ALL_ON: u8 = 0xA5;
const SSD1322_CMD_MODE_NORMAL: u8 = 0xA6;
const SSD1322_CMD_MODE_INVERT: u8 = 0xA7;
const SSD1322_CMD_DISPLAY_ON: u8 = 0xAF;
const SSD1322_CMD_DISPLAY_OFF: u8 = 0xAE;
const SSD1322_CMD_CLOCK_DIVIDER: u8 = 0xB3;

#[allow(dead_code)]
enum SSD1322Mode {
    Normal,
    AllOn,
    AllOff,
    Invert,
}

pub struct SSD1322<SPI, OUTPIN>
where
    SPI: SpiDevice,
    OUTPIN: OutputPin<Error = Infallible>,
{
    spi: SPI,
    dc: OUTPIN,
    res: OUTPIN,
    frames: [[u8; BUFFER_SIZE]; 2],
    front: usize,
}

impl<SPI, OUTPIN> SSD1322<SPI, OUTPIN>
where
    SPI: SpiDevice,
    OUTPIN: OutputPin<Error = Infallible>,
{
    pub fn new(spi: SPI, dc: OUTPIN, res: OUTPIN) -> Self {
        Self {
            spi,
            dc,
            res,
            frames: [[0; BUFFER_SIZE]; 2],
            front: 0,
        }
    }

    pub fn draw_pixel(&mut self, x: i32, y: i32, color: Gray4) {
        if (0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y) {
            let c = RawU4::from(color).into_inner();
            let c = c << 4 | c;
            let ind = (y * WIDTH as i32 + x) as usize; // contains 保证了不会溢出和越界
            self.frames[self.front ^ 1][ind] = c;
        }
    }

    pub fn init<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), SPI::Error> {
        self.res.set_high().ok();
        self.reset(delay);

        self.set_clock(9, 1)?;

        self.set_mux_ratio(HEIGHT - 1)?;

        self.set_start_row(0)?;
        self.set_offset_row(0)?;

        self.set_remap(0x16, 0x11)?;

        self.set_display_mode(SSD1322Mode::Normal)?;

        self.display_off()?;

        // Magic values for "Display enhancement A"
        self.write_cmd(0xB4)?;
        self.write_data(0xA0)?;
        self.write_data(0xFD)?;

        // Magic values for "Display enhancement B"
        self.write_cmd(0xD1)?;
        self.write_data(0x82)?;
        self.write_data(0x20)?;

        self.init_display()?;

        self.display_on(delay)
    }

    pub fn flush(&mut self) -> Result<(), SPI::Error> {
        self.set_column_range(0x1c, 0x5b)?;
        self.set_row_range(0, 0x3f)?;
        self.write_cmd(SSD1322_CMD_WRITE_RAM)?;
        self.dc.set_high().ok();
        self.front ^= 1;
        self.spi.write(&self.frames[self.front])?;
        Ok(())
    }

    fn write_cmd(&mut self, cmd: u8) -> Result<(), SPI::Error> {
        self.dc.set_low().ok();
        self.spi.write(&[cmd])?;
        Ok(())
    }

    fn write_data(&mut self, data: u8) -> Result<(), SPI::Error> {
        self.dc.set_high().ok();
        self.spi.write(&[data])?;
        Ok(())
    }

    fn delay<D: DelayNs>(delay: &mut D, mills: u32) {
        delay.delay_ms(mills);
    }

    fn reset<D: DelayNs>(&mut self, delay: &mut D) {
        self.res.set_low().ok();
        Self::delay(delay, 10);
        self.res.set_high().ok();
        Self::delay(delay, 300);
    }

    fn set_mux_ratio(&mut self, h: u8) -> Result<(), SPI::Error> {
        self.write_cmd(SSD1322_CMD_MUX_RATIO)?;
        self.write_data(h & 0x7f)
    }

    fn set_row_range(&mut self, r1: u8, r2: u8) -> Result<(), SPI::Error> {
        self.write_cmd(SSD1322_CMD_ROW_ADDRESS)?;
        self.write_data(r1 & 0x7f)?;
        self.write_data(r2 & 0x7f)
    }

    fn set_column_range(&mut self, c1: u8, c2: u8) -> Result<(), SPI::Error> {
        self.write_cmd(SSD1322_CMD_COL_ADDRESS)?;
        self.write_data(c1 & 0x7f)?;
        self.write_data(c2 & 0x7f)
    }

    fn set_clock(&mut self, freq: u8, divisor: u8) -> Result<(), SPI::Error> {
        self.write_cmd(SSD1322_CMD_CLOCK_DIVIDER)?;
        self.write_data(((freq & 0x0F) << 4) | (divisor & 0x0F))
    }

    fn set_remap(&mut self, a: u8, b: u8) -> Result<(), SPI::Error> {
        self.write_cmd(SSD1322_CMD_REMAP)?;
        self.write_data(a)?;
        self.write_data(b)
    }

    fn set_start_row(&mut self, r: u8) -> Result<(), SPI::Error> {
        self.write_cmd(SSD1322_CMD_START_LINE)?;
        self.write_data(r & 0x7F)
    }

    fn set_offset_row(&mut self, r: u8) -> Result<(), SPI::Error> {
        self.write_cmd(SSD1322_CMD_OFFSET_LINE)?;
        self.write_data(r & 0x7F)
    }

    fn set_display_mode(&mut self, mode: SSD1322Mode) -> Result<(), SPI::Error> {
        let cmd = match mode {
            SSD1322Mode::Normal => SSD1322_CMD_MODE_NORMAL,
            SSD1322Mode::AllOn => SSD1322_CMD_MODE_ALL_ON,
            SSD1322Mode::AllOff => SSD1322_CMD_MODE_ALL_OFF,
            SSD1322Mode::Invert => SSD1322_CMD_MODE_INVERT,
        };
        self.write_cmd(cmd)
    }

    fn display_on<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), SPI::Error> {
        self.write_cmd(SSD1322_CMD_DISPLAY_ON)?;
        Self::delay(delay, 200);
        Ok(())
    }

    fn display_off(&mut self) -> Result<(), SPI::Error> {
        self.write_cmd(SSD1322_CMD_DISPLAY_OFF)
    }

    fn init_display(&mut self) -> Result<(), SPI::Error> {
        self.set_column_range(0x1c, 0x5b)?;
        self.set_row_range(0, 0x3f)?;
        self.write_cmd(SSD1322_CMD_WRITE_RAM)?;
        self.dc.set_high().ok();
        self.spi.write(&self.frames[0])?;
        Ok(())
    } // 避免刚上电的花屏
}

impl<SPI, OUTPIN> OriginDimensions for SSD1322<SPI, OUTPIN>
where
    SPI: SpiDevice,
    OUTPIN: OutputPin<Error = Infallible>,
{
    fn size(&self) -> Size {
        Size {
            width: WIDTH as u32,
            height: HEIGHT as u32,
        }
    }
}

impl<SPI, OUTPIN> DrawTarget for SSD1322<SPI, OUTPIN>
where
    SPI: SpiDevice,
    OUTPIN: OutputPin<Error = Infallible>,
{
    type Color = Gray4;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for pixel in pixels {
            let Pixel(Point { x, y }, color) = pixel;
            self.draw_pixel(x, y, color);
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let c = RawU4::from(color).into_inner();
        let c = c << 4 | c;
        self.frames[self.front ^ 1].fill(c);
        Ok(())
    }
}
