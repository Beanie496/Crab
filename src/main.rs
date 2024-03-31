//! Crab, a UCI-compatible chess engine written in Rust.
//!
//! Accepted commands:
//! - `f`: find magics for the bishop and rook
//! - `go` with the options `wtime`, `btime`, `winc`, `binc`, `movestogo`,
//!   `depth`, `nodes`, `movetime` and `infinite`. There's also a special
//!   option `perft <depth>`, which overrides the regular search to run perft
//!   to `<depth>`.
//! - `isready`
//! - `p`: pretty-print the current board
//! - `position`
//! - `setoption`: see output of `uci` command for more detail
//! - `stop`
//! - `uci`
//! - `ucinewgame`
//! - `quit`

use std::sync::mpsc::RecvError;

use engine::Engine;

/// Unit testing.
mod bench;
/// Items associated with [`Bitboard`](bitboard::Bitboard).
mod bitboard;
/// Items associated with [`Board`](board::Board).
mod board;
/// Definitions and enumerations.
mod defs;
/// Items associated with [`Engine`].
mod engine;
/// Error variants.
mod error;
/// Items related to evaluation.
mod evaluation;
/// Items related to move generation.
mod movegen;
/// Perft: see <https://www.chessprogramming.org/Perft>.
mod perft;
/// Items related to searching.
mod search;
/// Items for handling UCI input.
mod uci;
/// Utility.
mod util;

fn main() -> Result<(), RecvError> {
    Engine::new().main_loop()?;
    Ok(())
}
