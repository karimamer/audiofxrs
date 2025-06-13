//! AudioFX-RS - A Rust-based audio effects processor
//!
//! This crate provides a command-line interface for applying various audio effects
//! to WAV files. It features a modular design with a unified CLI and extensible
//! effect system.

mod audio_io;
mod effects;
mod cli;

use cli::run_cli;

fn main() {
    run_cli();
}
