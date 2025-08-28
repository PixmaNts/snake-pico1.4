# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-12-XX

### Added
- Initial release of Snake game for Raspberry Pi Pico with Waveshare LCD 1.14"
- Classic Snake gameplay with modern embedded Rust implementation
- Embassy async framework integration for concurrent input handling
- Advanced game states: Start screen, Playing, Paused, Death animation, Blinking game over
- Visual effects:
  - White 1px border frame around game area
  - Death animation with brown fade and shrinking effect (2 seconds)
  - Blinking game over screen (12 blinks over 3 seconds)
  - Pause screen with current score and food count display
- Optimized rendering with dirty rectangle technique to eliminate flicker
- Responsive joystick controls with proper debouncing (150ms cooldown)
- Button controls: A (reset), B (start/pause)
- Score system tracking points and food consumed
- Hardware abstraction for easy porting to different displays
- Comprehensive error handling and logging with defmt
- Memory-efficient implementation using heapless collections
- High-performance SPI communication (62.5 MHz)

### Technical Features
- `#![no_std]` implementation for minimal memory footprint
- Async/await programming model with Embassy
- Static memory allocation (no heap required)
- RGB565 color format support
- 240×135 display with 90° rotation support
- 40×22 game grid with 6×6 pixel cells
- 30 FPS main loop with 3 FPS game logic updates

### Hardware Support
- Raspberry Pi Pico (RP2040 microcontroller)
- Waveshare LCD 1.14" with ST7789 controller
- Digital joystick and button inputs
- SPI display interface with hardware acceleration

### Documentation
- Comprehensive README with setup instructions
- Pin configuration documentation
- Build and flash instructions
- Troubleshooting guide
- Learning outcomes and technical details
- MIT license for open source distribution

### Development Tools
- Rust 2021 edition with embedded toolchain
- probe-rs integration for debugging and flashing
- Cargo.toml with proper metadata for publication
- Code formatting with rustfmt
- Linting with clippy