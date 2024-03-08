//! Crab, a UCI-compatible chess engine for my A-level project written in Rust.

use uci::Uci;

/// For unit testing.
mod bench;
/// A container for [`Bitboard`](bitboard::Bitboard).
mod bitboard;
/// Items related to the board. Mainly [`Board`](board::Board).
mod board;
/// Definitions and enumerations.
mod defs;
/// A container for [`Engine`](engine::Engine).
mod engine;
/// For evaluation.
mod evaluation;
/// Handles UCI input.
mod uci;

fn main() {
    Uci::new().main_loop();
}
