#![no_std]
#![no_main]

use defmt::{debug, info};
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use heapless::Vec;
// use embassy_rp::adc::{Adc, Channel}; // Commented out for now
use core::cell::RefCell;
use embassy_rp::spi::{Config as SpiConfig, Spi};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Instant, Timer};
use mipidsi::options::{Orientation, Rotation};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Baseline, Text},
};
// Provides the parallel port and display interface builders
use mipidsi::interface::SpiInterface;

// Provides the Display builder
use mipidsi::{models::ST7789, options::ColorInversion, Builder};
mod engine;
mod game;
mod traits;
// mod hardware;  // Keep this commented for now

// Game modules (commented out for now)
// use engine::GameEngine;
// use hardware::pico_waveshare::{PicoWaveshareDisplay, PicoWaveshareInput, PicoWaveshareRenderer, PicoPlatform};

const DISPLAY_WIDTH: i32 = 240; // Swapped due to 90° rotation
const DISPLAY_HEIGHT: i32 = 135;
const CELL_SIZE: i32 = 6;
const GRID_WIDTH: i32 = DISPLAY_WIDTH / CELL_SIZE;
const GRID_HEIGHT: i32 = DISPLAY_HEIGHT / CELL_SIZE;

type SpiBus = BlockingMutex<
    NoopRawMutex,
    RefCell<Spi<'static, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>>,
>;

#[derive(Clone, Copy, PartialEq)]
enum GameState {
    WaitingStart,
    Playing,
    Paused,
    DeathAnimation,
    BlinkingGameOver,
    GameOver,
}

// Helper function to draw white border around game area
fn draw_border<T: embedded_graphics::draw_target::DrawTarget<Color = Rgb565>>(display: &mut T) {
    use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};

    // Top border
    let _ = Rectangle::new(Point::new(0, 0), Size::new(DISPLAY_WIDTH as u32, 1))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
        .draw(display);

    // Bottom border
    let _ = Rectangle::new(
        Point::new(0, DISPLAY_HEIGHT - 1),
        Size::new(DISPLAY_WIDTH as u32, 1),
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
    .draw(display);

    // Left border
    let _ = Rectangle::new(Point::new(0, 0), Size::new(1, DISPLAY_HEIGHT as u32))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
        .draw(display);

    // Right border
    let _ = Rectangle::new(
        Point::new(DISPLAY_WIDTH - 1, 0),
        Size::new(1, DISPLAY_HEIGHT as u32),
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
    .draw(display);
}

// Helper function to show start screen
fn show_start_screen<T: embedded_graphics::draw_target::DrawTarget<Color = Rgb565>>(
    display: &mut T,
) {
    let text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);

    // Centered positions for 240x135 landscape orientation
    let _ =
        Text::with_baseline("Press B", Point::new(95, 60), text_style, Baseline::Top).draw(display);
    let _ = Text::with_baseline("to Start", Point::new(90, 75), text_style, Baseline::Top)
        .draw(display);
}

// Helper function to show pause screen with score
fn show_pause_screen<T: embedded_graphics::draw_target::DrawTarget<Color = Rgb565>>(
    display: &mut T,
    score: u16,
    food_eaten: u16,
) {
    let text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);

    // Show PAUSED at top
    let _ =
        Text::with_baseline("PAUSED", Point::new(95, 40), text_style, Baseline::Top).draw(display);

    // Show score
    let score_text = heapless::String::<32>::new();
    let mut score_text = score_text;
    use core::fmt::Write;
    write!(&mut score_text, "Score: {}", score).unwrap();
    let _ = Text::with_baseline(&score_text, Point::new(85, 60), text_style, Baseline::Top)
        .draw(display);

    // Show food eaten
    let food_text = heapless::String::<32>::new();
    let mut food_text = food_text;
    write!(&mut food_text, "Food: {}", food_eaten).unwrap();
    let _ = Text::with_baseline(&food_text, Point::new(90, 75), text_style, Baseline::Top)
        .draw(display);

    // Show resume instruction
    let _ =
        Text::with_baseline("Press B", Point::new(95, 95), text_style, Baseline::Top).draw(display);
    let _ = Text::with_baseline("to Resume", Point::new(85, 110), text_style, Baseline::Top)
        .draw(display);
}

// Helper function to show game over screen with final score
fn show_game_over_screen<T: embedded_graphics::draw_target::DrawTarget<Color = Rgb565>>(
    display: &mut T,
    score: u16,
    food_eaten: u16,
) {
    let text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);

    // Show GAME OVER at top
    let _ = Text::with_baseline("GAME OVER", Point::new(80, 35), text_style, Baseline::Top)
        .draw(display);

    // Show final score
    let score_text = heapless::String::<32>::new();
    let mut score_text = score_text;
    use core::fmt::Write;
    write!(&mut score_text, "Final Score: {}", score).unwrap();
    let _ = Text::with_baseline(&score_text, Point::new(70, 55), text_style, Baseline::Top)
        .draw(display);

    // Show food eaten
    let food_text = heapless::String::<32>::new();
    let mut food_text = food_text;
    write!(&mut food_text, "Food Eaten: {}", food_eaten).unwrap();
    let _ = Text::with_baseline(&food_text, Point::new(75, 75), text_style, Baseline::Top)
        .draw(display);

    // Show restart instruction
    let _ = Text::with_baseline("Press A", Point::new(95, 100), text_style, Baseline::Top)
        .draw(display);
    let _ = Text::with_baseline("to Restart", Point::new(85, 115), text_style, Baseline::Top)
        .draw(display);
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    info!("Snake Game Starting!");

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

    // Initialize display with working config
    let mut display = Builder::new(ST7789, spi_interface)
        .display_size(135, 240) // Physical dimensions before rotation
        .display_offset(52, 40) // Waveshare LCD 1.14" offset for 90° rotation
        .invert_colors(ColorInversion::Inverted)
        .orientation(Orientation::new().rotate(Rotation::Deg90))
        .reset_pin(reset_pin)
        .init(&mut embassy_time::Delay)
        .unwrap();

    // Turn on backlight
    let mut _backlight = Output::new(bl, Level::High);

    // Wait a bit for display to stabilize
    Timer::after_millis(100).await;

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
    spawner
        .spawn(input_handler(
            joy_up, joy_down, joy_left, joy_right, button_a, button_b,
        ))
        .unwrap();

    info!("Display initialized, starting Snake with joystick control!");

    // Clear screen and draw border
    display.clear(Rgb565::BLACK).unwrap();
    draw_border(&mut display);

    // Simple Snake game loop
    use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
    use game::{Direction, Game, Position};

    // Game state management
    let mut current_state = GameState::WaitingStart;

    // Show initial start screen
    show_start_screen(&mut display);

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
                    sender
                        .send(InputEvent::DirectionChange(Direction::Up))
                        .await;
                    last_direction_time = now;
                    debug!("Direction: UP");
                    Timer::after_millis(50).await; // Extra debounce for direction
                } else if joy_down.is_low() {
                    sender
                        .send(InputEvent::DirectionChange(Direction::Down))
                        .await;
                    last_direction_time = now;
                    debug!("Direction: DOWN");
                    Timer::after_millis(50).await;
                } else if joy_left.is_low() {
                    sender
                        .send(InputEvent::DirectionChange(Direction::Left))
                        .await;
                    last_direction_time = now;
                    debug!("Direction: LEFT");
                    Timer::after_millis(50).await;
                } else if joy_right.is_low() {
                    sender
                        .send(InputEvent::DirectionChange(Direction::Right))
                        .await;
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

    let mut snake_game = Game::new(
        (DISPLAY_WIDTH / CELL_SIZE) as u8,
        (DISPLAY_HEIGHT / CELL_SIZE) as u8,
    );

    // Clear screen once at start
    display.clear(Rgb565::BLACK).unwrap();

    let mut frame_counter = 0u32;
    let mut previous_snake = snake_game.snake.clone();
    let mut previous_food = snake_game.food;

    // Death animation variables
    let mut death_animation_frame = 0u32;
    let mut death_snake = Vec::<Position, 64>::new();
    let death_animation_duration = 60; // frames (~2 seconds at 30fps)

    // Blinking game over effect variables
    let mut blink_frame = 0u32;
    let blink_duration = 90; // frames (~3 seconds at 30fps)
    let total_blinks = 12; // Number of blinks
    let blink_interval = blink_duration / (total_blinks * 2); // frames per half-blink

    // Get receiver for input events
    let receiver = INPUT_CHANNEL.receiver();

    loop {
        // Check for input events (non-blocking)
        while let Ok(event) = receiver.try_receive() {
            match event {
                InputEvent::DirectionChange(direction) => {
                    // Only allow direction changes when playing
                    if current_state == GameState::Playing {
                        snake_game.set_direction(direction);
                    }
                }
                InputEvent::ButtonA => {
                    match current_state {
                        GameState::GameOver => {
                            // Restart game from game over screen
                            snake_game.reset();
                            display.clear(Rgb565::BLACK).unwrap();
                            draw_border(&mut display);
                            show_start_screen(&mut display);
                            current_state = GameState::WaitingStart;
                            previous_snake = snake_game.snake.clone();
                            previous_food = snake_game.food;
                            info!("Game restarted from game over");
                        }
                        GameState::Playing
                        | GameState::Paused
                        | GameState::DeathAnimation
                        | GameState::BlinkingGameOver => {
                            // Reset game to start screen (works when playing or paused)
                            snake_game.reset();
                            display.clear(Rgb565::BLACK).unwrap();
                            draw_border(&mut display);
                            show_start_screen(&mut display);
                            current_state = GameState::WaitingStart;
                            previous_snake = snake_game.snake.clone();
                            previous_food = snake_game.food;
                            info!("Game reset to start screen");
                        }
                        GameState::WaitingStart => {
                            // Do nothing when waiting for start
                        }
                    }
                }
                InputEvent::ButtonB => {
                    match current_state {
                        GameState::WaitingStart => {
                            // Start the game
                            current_state = GameState::Playing;
                            display.clear(Rgb565::BLACK).unwrap();
                            draw_border(&mut display);
                            info!("Game started!");
                        }
                        GameState::Playing => {
                            // Pause and show score
                            current_state = GameState::Paused;
                            display.clear(Rgb565::BLACK).unwrap();
                            draw_border(&mut display);
                            show_pause_screen(
                                &mut display,
                                snake_game.score,
                                snake_game.food_eaten,
                            );
                            info!(
                                "Game paused - Score: {}, Food: {}",
                                snake_game.score, snake_game.food_eaten
                            );
                        }
                        GameState::Paused => {
                            // Resume game
                            current_state = GameState::Playing;
                            display.clear(Rgb565::BLACK).unwrap();
                            draw_border(&mut display);
                            // Force full redraw of game state
                            previous_snake.clear();
                            previous_food = Position::new(255, 255); // Force food redraw
                            info!("Game resumed!");
                        }
                        GameState::GameOver
                        | GameState::DeathAnimation
                        | GameState::BlinkingGameOver => {
                            // Do nothing on B press in game over, death animation, or blinking (use A to restart)
                        }
                    }
                }
            }
        }

        // Only update game logic every 10 frames (slower game speed) and when playing
        if current_state == GameState::Playing && frame_counter % 10 == 0 {
            snake_game.update();

            // Check for game over
            if snake_game.game_over {
                current_state = GameState::DeathAnimation;
                death_animation_frame = 0;
                death_snake = snake_game.snake.clone();
                info!(
                    "Starting death animation - Final Score: {}, Food Eaten: {}",
                    snake_game.score, snake_game.food_eaten
                );
            } else {
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
                            Point::new(
                                (old_segment.x as i32) * CELL_SIZE + 1,
                                (old_segment.y as i32) * CELL_SIZE + 1,
                            ),
                            Size::new((CELL_SIZE - 1) as u32, (CELL_SIZE - 1) as u32),
                        )
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                        .draw(&mut display)
                        .unwrap();
                    }
                }

                // 2. Erase old food position if it moved
                if previous_food.x != snake_game.food.x || previous_food.y != snake_game.food.y {
                    Rectangle::new(
                        Point::new(
                            (previous_food.x as i32) * CELL_SIZE + 1,
                            (previous_food.y as i32) * CELL_SIZE + 1,
                        ),
                        Size::new((CELL_SIZE - 1) as u32, (CELL_SIZE - 1) as u32),
                    )
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(&mut display)
                    .unwrap();
                }

                // 3. Draw new snake positions
                for new_segment in &snake_game.snake {
                    Rectangle::new(
                        Point::new(
                            (new_segment.x as i32) * CELL_SIZE + 1,
                            (new_segment.y as i32) * CELL_SIZE + 1,
                        ),
                        Size::new((CELL_SIZE - 1) as u32, (CELL_SIZE - 1) as u32),
                    )
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
                    .draw(&mut display)
                    .unwrap();
                }

                // 4. Draw food
                Rectangle::new(
                    Point::new(
                        (snake_game.food.x as i32) * CELL_SIZE + 1,
                        (snake_game.food.y as i32) * CELL_SIZE + 1,
                    ),
                    Size::new((CELL_SIZE - 1) as u32, (CELL_SIZE - 1) as u32),
                )
                .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
                .draw(&mut display)
                .unwrap();

                // Update previous state for next frame
                previous_snake = snake_game.snake.clone();
                previous_food = snake_game.food;
            }
        }

        // Handle death animation
        if current_state == GameState::DeathAnimation {
            death_animation_frame += 1;

            // Calculate animation progress (0.0 to 1.0)
            let progress = death_animation_frame as f32 / death_animation_duration as f32;

            if progress >= 1.0 {
                // Animation finished, start blinking effect
                current_state = GameState::BlinkingGameOver;
                blink_frame = 0;
                display.clear(Rgb565::BLACK).unwrap();
                draw_border(&mut display);
                show_game_over_screen(&mut display, snake_game.score, snake_game.food_eaten);
            } else {
                // Animate snake shrinking and fading to brown
                let segments_to_show = ((1.0 - progress) * death_snake.len() as f32) as usize;

                // Erase old snake completely
                for segment in &previous_snake {
                    Rectangle::new(
                        Point::new(
                            (segment.x as i32) * CELL_SIZE + 1,
                            (segment.y as i32) * CELL_SIZE + 1,
                        ),
                        Size::new((CELL_SIZE - 1) as u32, (CELL_SIZE - 1) as u32),
                    )
                    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                    .draw(&mut display)
                    .unwrap();
                }

                // Draw shrinking snake with brown color fade
                for (i, segment) in death_snake.iter().enumerate() {
                    if i < segments_to_show {
                        // Fade from green to brown based on progress
                        let green_intensity = ((1.0 - progress) * 255.0) as u8;
                        let brown_intensity = (progress * 139.0) as u8; // Brown RGB component

                        // Create brownish color (approximating brown with RGB565)
                        let color = if progress < 0.5 {
                            Rgb565::new(brown_intensity / 8, green_intensity / 4, 0)
                        // Brownish-green fade
                        } else {
                            Rgb565::new(17, 9, 0) // Brown color in RGB565
                        };

                        Rectangle::new(
                            Point::new(
                                (segment.x as i32) * CELL_SIZE + 1,
                                (segment.y as i32) * CELL_SIZE + 1,
                            ),
                            Size::new((CELL_SIZE - 1) as u32, (CELL_SIZE - 1) as u32),
                        )
                        .into_styled(PrimitiveStyle::with_fill(color))
                        .draw(&mut display)
                        .unwrap();
                    }
                }

                // Update previous_snake for next frame
                previous_snake.clear();
                for (i, segment) in death_snake.iter().enumerate() {
                    if i < segments_to_show {
                        previous_snake.push(*segment).ok();
                    }
                }
            }
        }

        // Handle blinking game over effect
        if current_state == GameState::BlinkingGameOver {
            blink_frame += 1;

            if blink_frame >= blink_duration {
                // Blinking finished, stay on final game over screen
                current_state = GameState::GameOver;
            } else {
                // Calculate if screen should be visible (blinking effect)
                let blink_cycle = blink_frame / blink_interval;
                let is_visible = blink_cycle % 2 == 0;

                if is_visible {
                    // Show game over screen
                    display.clear(Rgb565::BLACK).unwrap();
                    draw_border(&mut display);
                    show_game_over_screen(&mut display, snake_game.score, snake_game.food_eaten);
                } else {
                    // Hide game over screen (just border)
                    display.clear(Rgb565::BLACK).unwrap();
                    draw_border(&mut display);
                }
            }
        }

        frame_counter = frame_counter.wrapping_add(1);
        Timer::after_millis(30).await; // Much faster loop, but only updates game occasionally
    }
}
