#!/bin/bash
echo "Building hardware-agnostic Snake Game for Raspberry Pi Pico..."
cargo build --release
echo "Build complete!"
echo "Flash with: probe-rs run --chip RP2040 target/thumbv6m-none-eabi/release/snake_embedded"
echo ""
echo "Note: This game uses a modular architecture - see ARCHITECTURE.md for details"
echo "on how to port to different hardware platforms."