#![no_std]
#![no_main]

use defmt::{info, debug};
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output, Pull, Input};
// use embassy_rp::adc::{Adc, Channel}; // Commented out for now
use embassy_rp::spi::{Config as SpiConfig, Spi};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::{Mutex as BlockingMutex};
use embassy_sync::channel::Channel;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
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
async fn main(spawner: Spawner) {
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
    
    // Spawn the input handler task
    spawner.spawn(input_handler(joy_up, joy_down, joy_left, joy_right, button_a, button_b)).unwrap();

    info!("Display initialized, starting Snake with joystick control!");

    // Clear screen 
    display.clear(Rgb565::BLACK).unwrap();
    
    // Simple Snake game loop
    use embedded_graphics::primitives::{Rectangle, PrimitiveStyle};
    use game::{Game, Direction};

// Input events for async handling
#[derive(Copy, Clone, Debug)]
pub enum InputEvent {
    DirectionChange(Direction),
    ButtonA,
    ButtonB,
}

// Global event channel for input events  
static INPUT_CHANNEL: Channel<CriticalSectionRawMutex, InputEvent, 10> = Channel::new();

// Input handler task
#[embassy_executor::task]
async fn input_handler(
    joy_up: Input<'static>,
    joy_down: Input<'static>, 
    joy_left: Input<'static>,
    joy_right: Input<'static>,
    button_a: Input<'static>,
    button_b: Input<'static>,
) {
    let sender = INPUT_CHANNEL.sender();
    let mut last_direction_time = Instant::now();
    const DIRECTION_COOLDOWN_MS: u64 = 150;
    
    loop {
        // Poll inputs with a small delay - still much better than the old approach
        Timer::after_millis(20).await;
        
        // Check which input is active and send appropriate event
        let now = Instant::now();
        if now.duration_since(last_direction_time).as_millis() > DIRECTION_COOLDOWN_MS {
            if joy_up.is_low() {
                sender.send(InputEvent::DirectionChange(Direction::Right)).await;
                last_direction_time = now;
                debug!("Direction: UP");
                Timer::after_millis(50).await; // Extra debounce for direction
            } else if joy_down.is_low() {
                sender.send(InputEvent::DirectionChange(Direction::Left)).await;
                last_direction_time = now;
                debug!("Direction: DOWN");
                Timer::after_millis(50).await;
            } else if joy_left.is_low() {
                sender.send(InputEvent::DirectionChange(Direction::Up)).await;
                last_direction_time = now;
                debug!("Direction: LEFT");
                Timer::after_millis(50).await;
            } else if joy_right.is_low() {
                sender.send(InputEvent::DirectionChange(Direction::Down)).await;
                last_direction_time = now;
                debug!("Direction: RIGHT");
                Timer::after_millis(50).await;
            }
        }
        
        if button_a.is_low() {
            sender.send(InputEvent::ButtonA).await;
            Timer::after_millis(200).await; // Longer debounce for reset button
        }
        
        if button_b.is_low() {
            sender.send(InputEvent::ButtonB).await;
            Timer::after_millis(100).await;
        }
    }
}
    
    let mut snake_game = Game::new((DISPLAY_WIDTH / CELL_SIZE) as u8, (DISPLAY_HEIGHT / CELL_SIZE) as u8);
    
    // Clear screen once at start
    display.clear(Rgb565::BLACK).unwrap();
    
    let mut frame_counter = 0u32;
    let mut previous_snake = snake_game.snake.clone();
    let mut previous_food = snake_game.food;
    
    // Get receiver for input events
    let receiver = INPUT_CHANNEL.receiver();
    
    loop {
        // Check for input events (non-blocking)
        while let Ok(event) = receiver.try_receive() {
            match event {
                InputEvent::DirectionChange(direction) => {
                    snake_game.set_direction(direction);
                }
                InputEvent::ButtonA => {
                    snake_game.reset();
                    display.clear(Rgb565::BLACK).unwrap();
                    previous_snake = snake_game.snake.clone();
                    previous_food = snake_game.food;
                }
                InputEvent::ButtonB => {
                    // Could add pause/resume functionality here
                }
            }
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


