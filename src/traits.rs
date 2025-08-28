use crate::game::{Direction, GameState, Position};

/// Color representation that can be implemented for different display types
#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8, 
    pub b: u8,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255 };
    pub const GREEN: Color = Color { r: 0, g: 255, b: 0 };
    pub const RED: Color = Color { r: 255, g: 0, b: 0 };
}

/// Input events from various input sources
#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    Direction(Direction),
    ButtonA,
    #[allow(dead_code)]
    ButtonB,
    None,
}

/// Abstraction for different display technologies
pub trait GameDisplay {
    type Error;
    
    /// Get display dimensions in pixels
    #[allow(dead_code)]
    fn dimensions(&self) -> (u16, u16);
    
    /// Clear the entire display
    fn clear(&mut self, color: Color) -> Result<(), Self::Error>;
    
    /// Draw a filled rectangle
    fn draw_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: Color) -> Result<(), Self::Error>;
    
    /// Draw text at specified position
    fn draw_text(&mut self, text: &str, x: u16, y: u16, color: Color) -> Result<(), Self::Error>;
    
    /// Update/flush the display (for buffered displays)
    fn update(&mut self) -> Result<(), Self::Error>;
}

/// Abstraction for different input methods
pub trait GameInput {
    type Error;
    
    /// Read the current input state
    async fn read_input(&mut self) -> Result<InputEvent, Self::Error>;
}

/// Abstraction for platform-specific operations
pub trait GamePlatform {
    /// Delay for specified milliseconds
    async fn delay_ms(&self, ms: u32);
    
    /// Get current time in milliseconds (for game timing)
    fn current_time_ms(&self) -> u32;
}

/// Complete game renderer that handles the visual aspects
pub trait GameRenderer {
    type Error;
    
    /// Render the complete game state
    fn render_game(&mut self, 
                   snake: &[Position], 
                   food: &Position, 
                   score: u16, 
                   state: GameState,
                   grid_width: u8,
                   grid_height: u8) -> Result<(), Self::Error>;
}