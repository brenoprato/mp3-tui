# MP3 Player TUI

A terminal-based MP3 player built with Rust, using [Ratatui](https://github.com/ratatui-org/ratatui) for the user interface and [Rodio](https://github.com/RustAudio/rodio) for audio playback.

This project was created as a study exercise to learn Rust, TUI development, and audio handling.

## Features (so far)

- Browse directories and display MP3 files.
- Play, pause, stop, and resume audio.
- Basic file navigation with arrow keys.
- Simple progress bar and simulated visualizer (cava-like).
- Two UI modes: default split view and full-screen player.

## Current Status

⚠️ **Work in progress** – there are known bugs and unfinished features. The code is being improved and refactored.

## Dependencies

- [ratatui](https://crates.io/crates/ratatui) – TUI framework
- [crossterm](https://crates.io/crates/crossterm) – terminal handling
- [rodio](https://crates.io/crates/rodio) – audio playback
- [color-eyre](https://crates.io/crates/color-eyre) – error handling
- [mp3-duration](https://crates.io/crates/mp3-duration) – fallback for duration extraction

## How to Run

```bash
cargo run