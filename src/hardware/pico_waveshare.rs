use crate::game::{Direction, GameState, Position};
use crate::traits::{Color, GameDisplay, GameInput, GamePlatform, GameRenderer, InputEvent};

use embassy_rp::adc::{Adc, Channel};
use embassy_rp::gpio::{Input, Output};
use embassy_time::{Duration, Instant, Timer};

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use mipidsi::Display;
use mipidsi::interface::SpiInterface;
use mipidsi::models::ST7789;
use embassy_embedded_hal::adapter::BlockingAsync;

// Type alias to simplify the complex Display type
type MipiDisplay = Display<
    SpiInterface<'static, BlockingAsync<embassy_rp::spi::Spi<'static, embassy_rp::peripherals::SPI0, embassy_rp::spi::Blocking>>, Output<'static>>,
    ST7789,
    Output<'static>
>;

// Convert our generic Color to Rgb565
impl From<Color> for Rgb565 {
    fn from(color: Color) -> Self {
        Rgb565::new(
            (color.r >> 3) as u8,
            (color.g >> 2) as u8, 
            (color.b >> 3) as u8,
        )
    }
}

pub struct PicoWaveshareDisplay {
    display: MipiDisplay,
    #[allow(dead_code)]
    cell_size: u16,
}

impl PicoWaveshareDisplay {
    pub fn new(
        display: MipiDisplay,
        cell_size: u16,
    ) -> Self {
        Self { display, cell_size }
    }
}

impl GameDisplay for PicoWaveshareDisplay {
    type Error = ();
    
    fn dimensions(&self) -> (u16, u16) {
        (240, 135)
    }
    
    fn clear(&mut self, color: Color) -> Result<(), Self::Error> {
        self.display.clear(color.into()).ok();
        Ok(())
    }
    
    fn draw_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: Color) -> Result<(), Self::Error> {
        let rect = Rectangle::new(
            Point::new(x as i32, y as i32),
            Size::new(width as u32, height as u32),
        );
        rect.into_styled(PrimitiveStyle::with_fill(color.into()))
            .draw(&mut self.display).ok();
        Ok(())
    }
    
    fn draw_text(&mut self, text: &str, x: u16, y: u16, color: Color) -> Result<(), Self::Error> {
        let text_style = MonoTextStyle::new(&FONT_6X10, color.into());
        Text::new(text, Point::new(x as i32, y as i32), text_style)
            .draw(&mut self.display).ok();
        Ok(())
    }
    
    fn update(&mut self) -> Result<(), Self::Error> {
        // ST7789 doesn't need explicit update
        Ok(())
    }
}

pub struct PicoWaveshareInput {
    adc: Adc<'static, embassy_rp::adc::Blocking>,
    joystick_x: Channel<'static>,
    joystick_y: Channel<'static>,
    button_a: Input<'static>,
    _button_b: Input<'static>,
}

impl PicoWaveshareInput {
    pub fn new(
        adc: Adc<'static, embassy_rp::adc::Blocking>,
        joystick_x: Channel<'static>,
        joystick_y: Channel<'static>,
        button_a: Input<'static>,
        button_b: Input<'static>,
    ) -> Self {
        Self {
            adc,
            joystick_x,
            joystick_y,
            button_a,
            _button_b: button_b,
        }
    }
    
    fn joystick_to_direction(x: u16, y: u16) -> Option<Direction> {
        const THRESHOLD: u16 = 1000;
        const CENTER: u16 = 2048;

        if x < CENTER - THRESHOLD {
            Some(Direction::Left)
        } else if x > CENTER + THRESHOLD {
            Some(Direction::Right)
        } else if y < CENTER - THRESHOLD {
            Some(Direction::Up)
        } else if y > CENTER + THRESHOLD {
            Some(Direction::Down)
        } else {
            None
        }
    }
}

impl GameInput for PicoWaveshareInput {
    type Error = embassy_rp::adc::Error;
    
    async fn read_input(&mut self) -> Result<InputEvent, Self::Error> {
        // Check button first (higher priority)
        if self.button_a.is_low() {
            return Ok(InputEvent::ButtonA);
        }
        
        // Read joystick  
        let x_val = self.adc.blocking_read(&mut self.joystick_x).unwrap_or(2048);
        let y_val = self.adc.blocking_read(&mut self.joystick_y).unwrap_or(2048);
        
        if let Some(direction) = Self::joystick_to_direction(x_val, y_val) {
            Ok(InputEvent::Direction(direction))
        } else {
            Ok(InputEvent::None)
        }
    }
}

pub struct PicoPlatform {
    start_time: Instant,
}

impl PicoPlatform {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
}

impl GamePlatform for PicoPlatform {
    async fn delay_ms(&self, ms: u32) {
        Timer::after(Duration::from_millis(ms as u64)).await;
    }
    
    fn current_time_ms(&self) -> u32 {
        self.start_time.elapsed().as_millis() as u32
    }
}

pub struct PicoWaveshareRenderer {
    display: PicoWaveshareDisplay,
    cell_size: u16,
}

impl PicoWaveshareRenderer {
    pub fn new(display: PicoWaveshareDisplay, cell_size: u16) -> Self {
        Self { display, cell_size }
    }
}

impl GameRenderer for PicoWaveshareRenderer {
    type Error = ();
    
    fn render_game(&mut self, 
                   snake: &[Position], 
                   food: &Position, 
                   score: u16, 
                   state: GameState,
                   _grid_width: u8,
                   _grid_height: u8) -> Result<(), Self::Error> {
        
        self.display.clear(Color::BLACK).ok();
        
        match state {
            GameState::Playing => {
                // Draw snake
                for segment in snake {
                    self.display.draw_rect(
                        segment.x as u16 * self.cell_size,
                        segment.y as u16 * self.cell_size,
                        self.cell_size,
                        self.cell_size,
                        Color::GREEN,
                    ).ok();
                }
                
                // Draw food
                self.display.draw_rect(
                    food.x as u16 * self.cell_size,
                    food.y as u16 * self.cell_size,
                    self.cell_size, 
                    self.cell_size,
                    Color::RED,
                ).ok();
                
                // Draw score
                let mut score_text = heapless::String::<32>::new(); 
                core::fmt::write(&mut score_text, format_args!("Score: {}", score)).unwrap();
                self.display.draw_text(&score_text, 5, 15, Color::WHITE).ok();
            }
            GameState::GameOver => {
                self.display.draw_text("GAME OVER", 85, 55, Color::WHITE).ok();
                
                let mut final_score = heapless::String::<32>::new();
                core::fmt::write(&mut final_score, format_args!("Final Score: {}", score)).unwrap();
                self.display.draw_text(&final_score, 75, 70, Color::WHITE).ok();
                
                self.display.draw_text("Press A to restart", 60, 90, Color::WHITE).ok();
            }
        }
        
        self.display.update().ok();
        Ok(())
    }
}