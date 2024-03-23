//! Crab, a UCI-compatible chess engine written in Rust.
//!
//! Accepted commands:
//! - `f`: find magics for the bishop and rook
//! - `go` with the options `wtime`, `btime`, `winc`, `binc`, `movestogo`,
//!   `depth`, `nodes`, `movetime` and `infinite`
//! - `isready`
//! - `p`: pretty-print the current board
//! - `perft <depth>`: run perft to `<depth>`
//! - `position`
//! - `setoption`: see output of `uci` command for more detail
//! - `stop`
//! - `uci`
//! - `ucinewgame`
//! - `quit`

use std::io;

use engine::Engine;

/// For unit testing.
mod bench;
/// Items associated with [`Bitboard`](bitboard::Bitboard).
mod bitboard;
/// Items associated with [`Board`](board::Board).
mod board;
/// Definitions and enumerations.
mod defs;
/// Items associated with [`Engine`].
mod engine;
/// Error handling.
mod error;
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

fn main() -> Result<(), io::Error> {
    Engine::new().main_loop()?;
    Ok(())
}
