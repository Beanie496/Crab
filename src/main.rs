//! Crab, a UCI-compatible chess engine for my A-level project written in Rust.
#![allow(clippy::new_without_default)]

use uci::Uci;

/// For unit testing.
mod bench;
/// A container for [`Bitboard`](bitboard::Bitboard).
pub mod bitboard;
/// Items related to the board. Mainly [`Board`](board::Board).
pub mod board;
/// Definitions and enumerations.
pub mod defs;
/// A container for [`Engine`](engine::Engine).
pub mod engine;
/// For evaluation.
mod evaluation;
/// Handles UCI input.
pub mod uci;

fn main() {
    Uci::new().main_loop();
}
