/*
 * Crab, a UCI-compatible chess engine
 * Copyright (C) 2024 Jasper Shovelton
 *
 * Crab is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Crab is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE. See the GNU General Public License for more
 * details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Crab. If not, see <https://www.gnu.org/licenses/>.
 */

//! Crab, a UCI-compatible chess engine written in Rust.
//!
//! Accepted commands:
//! - `bench`: run a benchmark
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
//!
//! This program also accepted command-line arguments, which it will process
//! and execute before running the UCI loop as normal.
//! Accepted command-line arguments:
//! - `bench`: runs a benchmark on the test positions in the root-level
//!   directory
//! - `quit`: same as UCI `quit`

use std::{env::args, process::exit, sync::mpsc::RecvError};

use bench::bench;
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
    // if there are any command-line arguments, run them and exit
    if args().len() > 1 {
        for token in args() {
            match token.as_str() {
                "bench" => bench(),
                "quit" => exit(0),
                _ => (),
            }
        }
        exit(0);
    }

    Engine::new().main_loop()?;
    Ok(())
}
