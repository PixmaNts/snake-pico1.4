# Snake Game Architecture

This document explains the modular, hardware-agnostic architecture of the Snake game.

## Overview

The game has been refactored to separate concerns and make it portable across different hardware platforms. The core game logic is completely independent of display technology, input methods, or platform-specific features.

## Architecture Components

### Core Game Logic (`game.rs`)
- Pure business logic for Snake game
- No dependencies on hardware or display
- Handles game state, collision detection, food spawning
- Can be used with any hardware implementation

### Abstraction Layer (`traits.rs`)
- `GameDisplay`: Abstracts different display technologies (SPI, I2C, web canvas, etc.)
- `GameInput`: Abstracts input methods (joystick, keyboard, touch, gamepad, etc.)
- `GamePlatform`: Abstracts platform operations (timing, delays)
- `GameRenderer`: High-level rendering interface combining display operations

### Game Engine (`engine.rs`)
- Hardware-agnostic game loop
- Uses trait abstractions to work with any hardware
- Handles timing, input processing, and rendering coordination
- Completely portable across platforms

### Hardware Implementations (`hardware/`)
- Platform-specific implementations of the traits
- `pico_waveshare.rs`: Raspberry Pi Pico + Waveshare ST7789 display
- `example_i2c_oled.rs`: Example for I2C OLED displays + keyboard input

## Benefits of This Architecture

### 1. Hardware Portability
Switch between different hardware configurations by simply implementing the traits:
```rust
// Raspberry Pi Pico version
let engine = GameEngine::new(pico_input, pico_platform, pico_renderer, 40, 22);

// I2C OLED version  
let engine = GameEngine::new(keyboard_input, desktop_platform, oled_renderer, 32, 16);

// Web version (hypothetical)
let engine = GameEngine::new(web_input, web_platform, canvas_renderer, 50, 30);
```

### 2. Easy Testing
Mock implementations can be created for automated testing:
```rust
struct MockDisplay { /* ... */ }
impl GameDisplay for MockDisplay { /* ... */ }

// Test with mock hardware
let engine = GameEngine::new(mock_input, mock_platform, mock_renderer, 10, 10);
```

### 3. Different Display Technologies
- **SPI displays**: ST7789, ILI9341, etc.
- **I2C displays**: SSD1306 OLED, etc.
- **Web canvas**: HTML5 canvas rendering
- **Terminal**: ASCII art rendering
- **LED matrices**: WS2812B strips, etc.

### 4. Different Input Methods
- **Analog joystick**: ADC-based directional input
- **Digital buttons**: GPIO-based controls
- **Keyboard**: WASD or arrow keys
- **Gamepad**: USB/Bluetooth controllers
- **Touch**: Capacitive touch panels
- **Accelerometer**: Tilt-based controls

### 5. Different Platforms
- **Embedded**: Embassy async on microcontrollers
- **Desktop**: Standard Rust with tokio
- **Web**: WebAssembly with web APIs
- **RTOS**: FreeRTOS or other real-time systems

## Implementation Guide

### Adding New Hardware Support

1. **Implement the traits** for your hardware:
```rust
pub struct MyDisplay { /* your display driver */ }
impl GameDisplay for MyDisplay { /* implement methods */ }

pub struct MyInput { /* your input hardware */ }  
impl GameInput for MyInput { /* implement methods */ }

pub struct MyPlatform { /* platform specifics */ }
impl GamePlatform for MyPlatform { /* implement methods */ }

pub struct MyRenderer { /* combines display ops */ }
impl GameRenderer for MyRenderer { /* implement methods */ }
```

2. **Create the engine** with your implementations:
```rust
let engine = GameEngine::new(my_input, my_platform, my_renderer, width, height);
engine.run().await?;
```

### Example: Web Version
```rust
// Hypothetical web implementation
struct WebCanvas { /* HTML5 canvas context */ }
impl GameDisplay for WebCanvas { /* draw to canvas */ }

struct WebInput { /* keyboard event listeners */ }
impl GameInput for WebInput { /* read keyboard state */ }

struct WebPlatform { /* web timing */ }
impl GamePlatform for WebPlatform { /* use setTimeout/requestAnimationFrame */ }

// Same game logic, different platform!
let engine = GameEngine::new(web_input, web_platform, web_renderer, 50, 30);
```

### Example: Terminal Version
```rust
struct TerminalDisplay { /* stdout/ncurses */ }
impl GameDisplay for TerminalDisplay { /* ASCII art rendering */ }

struct TerminalInput { /* stdin/termios */ }
impl GameInput for TerminalInput { /* read arrow keys */ }

// Play Snake in your terminal!
let engine = GameEngine::new(term_input, desktop_platform, term_renderer, 80, 24);
```

## File Structure
```
src/
├── main.rs              # Raspberry Pi Pico main application
├── game.rs              # Pure game logic
├── traits.rs            # Hardware abstraction traits
├── engine.rs            # Hardware-agnostic game engine
└── hardware/
    ├── mod.rs
    ├── pico_waveshare.rs    # Pico + ST7789 implementation
    └── example_i2c_oled.rs # Example I2C OLED implementation
```

## Summary

This architecture makes the Snake game:
- **Portable**: Works on any hardware with trait implementations
- **Testable**: Easy to mock and unit test
- **Maintainable**: Clear separation of concerns
- **Extensible**: Add new platforms without changing game logic
- **Reusable**: Game engine can be used for other simple games

The same game logic now runs on embedded systems, desktops, web browsers, or any platform you can implement the traits for!