// Example implementation for a different hardware configuration:
// I2C OLED display + keyboard input (for desktop/web)
// 
// This demonstrates how the same game logic can work with
// completely different hardware interfaces

#![allow(dead_code)]

use crate::game::{GameState, Position};
use crate::traits::{Color, GameDisplay, GameInput, GamePlatform, GameRenderer, InputEvent};

// Example for SSD1306 I2C OLED display
pub struct I2COLEDDisplay {
    // display: SSD1306<...>, // Would contain actual I2C display driver
    width: u16,
    height: u16,
    cell_size: u16,
}

impl I2COLEDDisplay {
    pub fn new(width: u16, height: u16, cell_size: u16) -> Self {
        Self { width, height, cell_size }
    }
}

impl GameDisplay for I2COLEDDisplay {
    type Error = ();
    
    fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }
    
    fn clear(&mut self, _color: Color) -> Result<(), Self::Error> {
        // Clear OLED display buffer
        // self.display.clear();
        Ok(())
    }
    
    fn draw_rect(&mut self, _x: u16, _y: u16, _width: u16, _height: u16, _color: Color) -> Result<(), Self::Error> {
        // Draw rectangle on OLED
        // self.display.fill_rect(x, y, width, height, color.into())?;
        Ok(())
    }
    
    fn draw_text(&mut self, _text: &str, _x: u16, _y: u16, _color: Color) -> Result<(), Self::Error> {
        // Draw text on OLED
        // self.display.draw_text(text, x, y, font, color.into())?;
        Ok(())
    }
    
    fn update(&mut self) -> Result<(), Self::Error> {
        // Flush buffer to display
        // self.display.flush()?;
        Ok(())
    }
}

// Example keyboard input (could be used for desktop/web versions)
pub struct KeyboardInput {
    // Could interface with stdin, web input events, etc.
}

impl KeyboardInput {
    pub fn new() -> Self {
        Self {}
    }
}

impl GameInput for KeyboardInput {
    type Error = ();
    
    async fn read_input(&mut self) -> Result<InputEvent, Self::Error> {
        // Read from keyboard/stdin
        // match read_key() {
        //     'w' | 'W' => Ok(InputEvent::Direction(Direction::Up)),
        //     's' | 'S' => Ok(InputEvent::Direction(Direction::Down)),
        //     'a' | 'A' => Ok(InputEvent::Direction(Direction::Left)),
        //     'd' | 'D' => Ok(InputEvent::Direction(Direction::Right)),
        //     ' ' => Ok(InputEvent::ButtonA),
        //     _ => Ok(InputEvent::None),
        // }
        Ok(InputEvent::None)
    }
}

// Example platform for desktop/simulation
pub struct DesktopPlatform {
    // start_time: std::time::Instant,  // Would use std::time for desktop
}

impl DesktopPlatform {
    pub fn new() -> Self {
        Self {}
    }
}

impl GamePlatform for DesktopPlatform {
    async fn delay_ms(&self, _ms: u32) {
        // For desktop: std::thread::sleep(Duration::from_millis(ms));
        // For async: tokio::time::sleep(Duration::from_millis(ms)).await;
    }
    
    fn current_time_ms(&self) -> u32 {
        // self.start_time.elapsed().as_millis() as u32
        0
    }
}

// Renderer for the I2C OLED setup
pub struct I2COLEDRenderer {
    display: I2COLEDDisplay,
    cell_size: u16,
}

impl I2COLEDRenderer {
    pub fn new(display: I2COLEDDisplay, cell_size: u16) -> Self {
        Self { display, cell_size }
    }
}

impl GameRenderer for I2COLEDRenderer {
    type Error = ();
    
    fn render_game(&mut self, 
                   snake: &[Position], 
                   food: &Position, 
                   score: u16, 
                   state: GameState,
                   _grid_width: u8,
                   _grid_height: u8) -> Result<(), Self::Error> {
        
        self.display.clear(Color::BLACK)?;
        
        match state {
            GameState::Playing => {
                // Draw snake segments
                for segment in snake {
                    self.display.draw_rect(
                        segment.x as u16 * self.cell_size,
                        segment.y as u16 * self.cell_size,
                        self.cell_size,
                        self.cell_size,
                        Color::WHITE, // OLED is monochrome
                    )?;
                }
                
                // Draw food
                self.display.draw_rect(
                    food.x as u16 * self.cell_size,
                    food.y as u16 * self.cell_size,
                    self.cell_size,
                    self.cell_size,
                    Color::WHITE,
                )?;
                
                // Draw score
                let mut score_text = heapless::String::<32>::new();
                core::fmt::write(&mut score_text, format_args!("Score: {}", score)).unwrap();
                self.display.draw_text(&score_text, 0, 0, Color::WHITE)?;
            }
            GameState::GameOver => {
                self.display.draw_text("GAME OVER", 32, 16, Color::WHITE)?;
                
                let mut final_score = heapless::String::<32>::new();
                core::fmt::write(&mut final_score, format_args!("Score: {}", score)).unwrap();
                self.display.draw_text(&final_score, 32, 32, Color::WHITE)?;
                
                self.display.draw_text("Press SPACE", 32, 48, Color::WHITE)?;
            }
        }
        
        self.display.update()?;
        Ok(())
    }
}

/*
Usage example for this alternative hardware:

let oled_display = I2COLEDDisplay::new(128, 64, 4);
let keyboard_input = KeyboardInput::new();
let desktop_platform = DesktopPlatform::new();
let oled_renderer = I2COLEDRenderer::new(oled_display, 4);

let mut engine = GameEngine::new(
    keyboard_input,
    desktop_platform,
    oled_renderer,
    32, // 128/4 = 32 cells wide
    16, // 64/4 = 16 cells tall
);

engine.run().await?;
*/