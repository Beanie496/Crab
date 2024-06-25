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
//! - `bench [TT size] [limit] [limit type]`: run a benchmark. The default
//!   options are [`TT_SIZE`](crate::bench::TT_SIZE),
//!   [`LIMIT`](crate::bench::LIMIT) and
//!   [`LIMIT_TYPE`](crate::bench::LIMIT_TYPE) respectively.
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
//! This program also accepts `bench` as a command-line argument, which it will
//! process and execute instead of running the UCI loop.

use std::{env::args, sync::mpsc::RecvError};

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
/// A transposition table.
mod transposition_table;
/// Utility.
mod util;

fn main() -> Result<(), RecvError> {
    let mut args = args();
    args.next();

    // if it's on the command line, execute the `bench` command and return.
    // Otherwise, continue as normal
    if args.next().is_some_and(|s| s == "bench") {
        // there's practically no difference between deallocating at the end of
        // `bench()` and at the end of the program
        bench(args.map(|s| s.leak() as &str));
        Ok(())
    } else {
        Engine::new().main_loop()
    }
}
