//! Crab, a UCI-compatible chess engine written in Rust.

use uci::Uci;

/// For unit testing.
mod bench;
/// Items associated with [`Bitboard`](bitboard::Bitboard).
mod bitboard;
/// Items associated with [`Board`](board::Board).
mod board;
/// Definitions and enumerations.
mod defs;
/// Items associated with [`Engine`](engine::Engine).
mod engine;
/// Items related to evaluation.
mod evaluation;
/// Items related to move generation.
mod movegen;
/// Perft: see <https://www.chessprogramming.org/Perft>.
mod perft;
/// The search.
mod search;
/// UCI input.
mod uci;
/// Utility.
mod util;

fn main() {
    Uci::new().main_loop();
}
