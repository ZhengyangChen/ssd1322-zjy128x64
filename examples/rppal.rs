use std::{
    error::Error,
    time::{Duration, Instant},
};

use embedded_graphics::{
    Drawable,
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::Gray4,
    prelude::{GrayColor, Point, Primitive},
    primitives::{Circle, PrimitiveStyle},
    text::{Baseline, Text},
};
use rppal::{
    gpio::Gpio,
    hal::Delay,
    spi::{Bus, Mode, SlaveSelect, Spi},
};
use ssd1322_zjy128x64::SSD1322;

fn main() -> Result<(), Box<dyn Error>> {
    let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 8_000_000, Mode::Mode0)?;
    let gpios = Gpio::new()?;
    let dc = gpios.get(22)?.into_output();
    let res = gpios.get(27)?.into_output();
    let mut display = SSD1322::new(spi, dc, res);
    let mut delay = Delay::new();
    display.init(&mut delay)?;

    let mut pos = 0_i32;
    let mut current = 0_u128;
    let mut fps = 0_i32;
    let mut counter = 0_i32;
    let start = Instant::now();
    let style = MonoTextStyle::new(&FONT_10X20, Gray4::WHITE);
    loop {
        let now = start.elapsed().as_millis();
        if now - current < 1000 {
            counter += 1;
        } else {
            current = now;
            fps = counter;
            counter = 0;
        }
        let c = (pos / 2 % 32) as u8;
        let c = if c > 15 { 31 - c } else { c };
        let color = Gray4::new(c);
        display.clear(Gray4::new(15 - c))?;
        Circle::with_center(Point::new(pos % 128, 32), 60)
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(&mut display)?;
        Text::with_baseline(&fps.to_string(), Point::zero(), style, Baseline::Top)
            .draw(&mut display)?;

        display.flush()?;
        pos += 1;
        let used_millis = (start.elapsed().as_millis() - now) as u64;
        if used_millis < 33 {
            std::thread::sleep(Duration::from_millis(33 - used_millis));
        }
    }
}
