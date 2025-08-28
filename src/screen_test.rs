#![no_std]
#![no_main]

use core::cell::RefCell;
use defmt::info;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::spi::{Config as SpiConfig, Spi};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_time::Timer;
use mipidsi::options::Orientation;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};
use mipidsi::interface::SpiInterface;
use mipidsi::{
    models::ST7789,
    options::{ColorInversion, Rotation},
    Builder,
};

const DISPLAY_WIDTH: i32 = 135;
const DISPLAY_HEIGHT: i32 = 240;

type SpiBus = BlockingMutex<
    NoopRawMutex,
    RefCell<Spi<'static, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>>,
>;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    info!("Screen Test Starting!");

    // Configure SPI for display
    let mosi = p.PIN_11; // SDA
    let clk = p.PIN_10; // SCL
    let cs = p.PIN_9; // CS
    let dc = p.PIN_8; // DC
    let rst = p.PIN_12; // RST
    let bl = p.PIN_13; // Backlight

    let mut spi_config = SpiConfig::default();
    spi_config.frequency = 62_500_000; // 62.5 MHz

    // Use blocking SPI
    let spi = Spi::new_blocking_txonly(p.SPI1, clk, mosi, spi_config.clone());

    // Create shared SPI bus
    static SPI_BUS: StaticCell<SpiBus> = StaticCell::new();
    let spi_bus = SPI_BUS.init(BlockingMutex::new(RefCell::new(spi)));

    // Create SPI device with CS pin
    let spi_device = SpiDeviceWithConfig::new(spi_bus, Output::new(cs, Level::High), spi_config);

    // Buffer for mipidsi
    static mut BUFFER: [u8; 64] = [0; 64];
    let buffer = unsafe { (&raw mut BUFFER).cast::<[u8; 64]>().as_mut().unwrap() };

    // Create SPI interface
    let spi_interface = SpiInterface::new(spi_device, Output::new(dc, Level::Low), buffer);

    // Create reset pin
    let reset_pin = Output::new(rst, Level::High);

    // Initialize display
    let mut display = Builder::new(ST7789, spi_interface)
        .display_size(DISPLAY_WIDTH as u16, DISPLAY_HEIGHT as u16) // Rotated dimensions
        .display_offset(53, 40) // Waveshare LCD 1.14" offset for 90° rotation
        .invert_colors(ColorInversion::Inverted)
        .reset_pin(reset_pin)
        .init(&mut embassy_time::Delay)
        .unwrap();

    // Turn on backlight
    let mut _backlight = Output::new(bl, Level::High);

    info!("Display initialized, starting tests...");

    display.clear(Rgb565::BLACK).unwrap();
    // Test 1: Fill screen with colors
    display
        .fill_solid(
            &Rectangle::new(
                Point::new(0, 0),
                Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32),
            ),
            Rgb565::RED,
        )
        .unwrap();
    Timer::after_millis(1000).await;

    display
        .fill_solid(
            &Rectangle::new(
                Point::new(0, 0),
                Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32),
            ),
            Rgb565::GREEN,
        )
        .unwrap();
    Timer::after_millis(1000).await;

    display
        .fill_solid(
            &Rectangle::new(
                Point::new(0, 0),
                Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32),
            ),
            Rgb565::BLUE,
        )
        .unwrap();
    Timer::after_millis(1000).await;

    // Test 2: Draw shapes
    display
        .fill_solid(
            &Rectangle::new(
                Point::new(0, 0),
                Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32),
            ),
            Rgb565::BLACK,
        )
        .unwrap();

    // Draw some rectangles
    Rectangle::new(Point::new(10, 10), Size::new(50, 30))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
        .draw(&mut display)
        .unwrap();

    Rectangle::new(Point::new(70, 20), Size::new(40, 40))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
        .draw(&mut display)
        .unwrap();

    Rectangle::new(Point::new(120, 15), Size::new(30, 50))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
        .draw(&mut display)
        .unwrap();

    // Draw some circles
    Circle::new(Point::new(170, 10), 25)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW))
        .draw(&mut display)
        .unwrap();

    Circle::new(Point::new(200, 40), 20)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::MAGENTA))
        .draw(&mut display)
        .unwrap();

    Timer::after_millis(2000).await;

    // Test 3: Text display
    display
        .fill_solid(
            &Rectangle::new(
                Point::new(0, 0),
                Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32),
            ),
            Rgb565::BLACK,
        )
        .unwrap();

    let text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);

    Text::with_baseline("Screen Test", Point::new(10, 20), text_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();

    Text::with_baseline(
        "Display: 240x135",
        Point::new(10, 40),
        text_style,
        Baseline::Top,
    )
    .draw(&mut display)
    .unwrap();

    Text::with_baseline(
        "All systems OK!",
        Point::new(10, 60),
        text_style,
        Baseline::Top,
    )
    .draw(&mut display)
    .unwrap();

    Timer::after_millis(2000).await;

    // Test 4: Animated pattern
    info!("Starting animation test...");

    loop {
        for i in 0..30 {
            display
                .fill_solid(
                    &Rectangle::new(
                        Point::new(0, 0),
                        Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32),
                    ),
                    Rgb565::BLACK,
                )
                .unwrap();

            // Moving circle
            let x = (i * 8) % (DISPLAY_WIDTH - 20);
            let y = 50 + ((i * 2) % 20);

            Circle::new(Point::new(x, y), 10)
                .into_styled(PrimitiveStyle::with_fill(Rgb565::CYAN))
                .draw(&mut display)
                .unwrap();

            // Static text
            Text::with_baseline(
                "Animation Test",
                Point::new(10, 10),
                text_style,
                Baseline::Top,
            )
            .draw(&mut display)
            .unwrap();

            Timer::after_millis(100).await;
        }
    }
}
