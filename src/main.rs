//! Crab, a UCI-compatible chess engine for my A-level project written in Rust.

use crate::uci::Uci;

/// Items related to the board. Mainly [`Board`](board::Board).
mod board;
/// Definitions and enumerations.
mod defs;
/// A container for [`Engine`](engine::Engine).
mod engine;
/// A container for [`Movelist`](movelist::Movelist).
mod movelist;
/// Handles UCI input
mod uci;
/// Miscellaneous useful functions.
mod util;

fn main() {
    Uci::main_loop();
}
