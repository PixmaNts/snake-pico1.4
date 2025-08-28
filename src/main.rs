#![no_std]
#![no_main]

use defmt::{info, println};
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output, Pull, Input};
// use embassy_rp::adc::{Adc, Channel}; // Commented out for now
use embassy_rp::spi::{Config as SpiConfig, Spi};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::{Mutex as BlockingMutex};
use core::cell::RefCell;
use embassy_time::{Timer, Instant};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
};
// Provides the parallel port and display interface builders
use mipidsi::interface::SpiInterface;

// Provides the Display builder
use mipidsi::{models::ST7789, options::ColorInversion, Builder};
mod game;
mod traits;
mod engine;
// mod hardware;  // Keep this commented for now

// Game modules (commented out for now)
// use engine::GameEngine;
// use hardware::pico_waveshare::{PicoWaveshareDisplay, PicoWaveshareInput, PicoWaveshareRenderer, PicoPlatform};


const DISPLAY_WIDTH: i32 = 135;
const DISPLAY_HEIGHT: i32 = 240;
const CELL_SIZE: i32 = 6;
const GRID_WIDTH: i32 = DISPLAY_WIDTH / CELL_SIZE;
const GRID_HEIGHT: i32 = DISPLAY_HEIGHT / CELL_SIZE;


type SpiBus = BlockingMutex<NoopRawMutex, RefCell<Spi<'static, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>>>;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    info!("Snake Game Starting!");

    // Configure SPI for display
    let mosi = p.PIN_11; // SDA
    let clk = p.PIN_10;  // SCL
    let cs = p.PIN_9;   // CS
    let dc = p.PIN_8;   // DC
    let rst = p.PIN_12; // RST
    let bl = p.PIN_13;  // Backlight

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
    
    // Initialize display with working config
    let mut display = Builder::new(ST7789, spi_interface)
        .display_size(DISPLAY_WIDTH as u16, DISPLAY_HEIGHT as u16)
        .display_offset(53, 40) // Waveshare LCD 1.14" offset for 90° rotation
        .invert_colors(ColorInversion::Inverted)
        .reset_pin(reset_pin)
        .init(&mut embassy_time::Delay)
        .unwrap();

    // Turn on backlight
    let mut _backlight = Output::new(bl, Level::High);

    
    // Joystik pin for pico lcd 1.4
    // gp2 -up
    // gp3 ctrl
    // gp16 - left
    // gp18 - down
    // gp20 - right
    
    // TODO: Re-enable ADC for joystick once we fix the API
    // let mut adc = Adc::new(p.ADC, irq, embassy_rp::adc::Config::default());
    // let mut joystick_x = Channel::new_pin(p.PIN_26, Pull::None);
    // let mut joystick_y = Channel::new_pin(p.PIN_27, Pull::None);

    // User bouton on pico lcd 1.4 :
    // gp15 : Bouton A
    // gp17 : Bouton B

    // Initialize buttons
    let button_a = Input::new(p.PIN_15, Pull::Up);
    let button_b = Input::new(p.PIN_17, Pull::Up);
    
    // Initialize joystick pins
    let joy_up = Input::new(p.PIN_2, Pull::Up);
    let joy_left = Input::new(p.PIN_16, Pull::Up);
    let joy_down = Input::new(p.PIN_18, Pull::Up);
    let joy_right = Input::new(p.PIN_20, Pull::Up);

    info!("Display initialized, starting Snake with joystick control!");

    // Clear screen 
    display.clear(Rgb565::BLACK).unwrap();
    
    // Simple Snake game loop
    use embedded_graphics::primitives::{Rectangle, PrimitiveStyle};
    use game::{Game, Direction};
    
    let mut snake_game = Game::new((DISPLAY_WIDTH / CELL_SIZE) as u8, (DISPLAY_HEIGHT / CELL_SIZE) as u8);
    let mut last_direction_change = Instant::now();
    
    // Clear screen once at start
    display.clear(Rgb565::BLACK).unwrap();
    
    let mut frame_counter = 0u32;
    let mut previous_snake = snake_game.snake.clone();
    let mut previous_food = snake_game.food;
    
    // Input debouncing for shitty joystick
    let mut input_counters = [0u8; 4]; // up, down, left, right
    let mut last_direction_set = Instant::now();
    const INPUT_THRESHOLD: u8 = 3; // Need 3 consecutive reads to register
    const DIRECTION_COOLDOWN_MS: u64 = 150; // Minimum time between direction changes
    
    loop {
        // DEBOUNCED INPUT READING - fixes shitty joystick!
        
        // Read current joystick state
        let current_inputs = [
            joy_up.is_low(),    // 0
            joy_down.is_low(),  // 1
            joy_left.is_low(),  // 2
            joy_right.is_low()  // 3
        ];
        
        // Update counters based on current state
        for i in 0..4 {
            if current_inputs[i] {
                input_counters[i] = (input_counters[i] + 1).min(INPUT_THRESHOLD + 1);
            } else {
                input_counters[i] = 0;
            }
        }
        
        // Only change direction if:
        // 1. Input has been stable for THRESHOLD frames
        // 2. Enough time has passed since last direction change (cooldown)
        if Instant::now().duration_since(last_direction_set).as_millis() > DIRECTION_COOLDOWN_MS {
            if input_counters[0] >= INPUT_THRESHOLD { // up
                snake_game.set_direction(Direction::Up);
                last_direction_set = Instant::now();
                println!("Direction: UP");
            } else if input_counters[1] >= INPUT_THRESHOLD { // down
                snake_game.set_direction(Direction::Down);
                last_direction_set = Instant::now();
                println!("Direction: DOWN");
            } else if input_counters[2] >= INPUT_THRESHOLD { // left
                snake_game.set_direction(Direction::Left);
                last_direction_set = Instant::now();
                println!("Direction: LEFT");
            } else if input_counters[3] >= INPUT_THRESHOLD { // right
                snake_game.set_direction(Direction::Right);
                last_direction_set = Instant::now();
                println!("Direction: RIGHT");
            }
        }
        
        // Button A to reset game (check every few frames to avoid spam)
        if frame_counter % 10 == 0 && button_a.is_low() {
            snake_game.reset();
            // Full clear only on reset
            display.clear(Rgb565::BLACK).unwrap();
            previous_snake = snake_game.snake.clone();
            previous_food = snake_game.food;
            Timer::after_millis(200).await; // Debounce
        }
        
        // Only update game logic every 10 frames (slower game speed)
        if frame_counter % 10 == 0 {
            snake_game.update();
            
            // DIRTY RECTANGLE RENDERING - NO MORE FLICKER!
            
            // 1. Erase old snake positions (draw black rectangles)
            for old_segment in &previous_snake {
                let mut found = false;
                // Check if this position is still occupied by snake
                for new_segment in &snake_game.snake {
                    if old_segment.x == new_segment.x && old_segment.y == new_segment.y {
                        found = true;
                        break;
                    }
                }
                // If not occupied anymore, erase it
                if !found {
                    Rectangle::new(
                        Point::new((old_segment.x as i32) * CELL_SIZE, (old_segment.y as i32) * CELL_SIZE),
                        Size::new(CELL_SIZE as u32, CELL_SIZE as u32)
                    )
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(&mut display).unwrap();
                }
            }
            
            // 2. Erase old food position if it moved
            if previous_food.x != snake_game.food.x || previous_food.y != snake_game.food.y {
                Rectangle::new(
                    Point::new((previous_food.x as i32) * CELL_SIZE, (previous_food.y as i32) * CELL_SIZE),
                    Size::new(CELL_SIZE as u32, CELL_SIZE as u32)
                )
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(&mut display).unwrap();
            }
            
            // 3. Draw new snake positions
            for new_segment in &snake_game.snake {
                Rectangle::new(
                    Point::new((new_segment.x as i32) * CELL_SIZE, (new_segment.y as i32) * CELL_SIZE),
                    Size::new(CELL_SIZE as u32, CELL_SIZE as u32)
                )
                .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
                .draw(&mut display).unwrap();
            }
            
            // 4. Draw food
            Rectangle::new(
                Point::new((snake_game.food.x as i32) * CELL_SIZE, (snake_game.food.y as i32) * CELL_SIZE),
                Size::new(CELL_SIZE as u32, CELL_SIZE as u32)
            )
            .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
            .draw(&mut display).unwrap();
            
            // Update previous state for next frame
            previous_snake = snake_game.snake.clone();
            previous_food = snake_game.food;
        }
        
        frame_counter = frame_counter.wrapping_add(1);
        Timer::after_millis(50).await; // Much faster loop, but only updates game occasionally
    }
}


