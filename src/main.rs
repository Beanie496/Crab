//! Crab, a UCI-compatible chess engine for my A-level project written in Rust.

use crate::uci::Uci;

/// Module for testing.
mod bench;
/// Items related to the board. Mainly [`Board`](board::Board).
mod board;
/// Definitions and enumerations.
mod defs;
/// A container for [`Engine`](engine::Engine).
mod engine;
/// Handles UCI input
mod uci;

fn main() {
    Uci::main_loop();
}
