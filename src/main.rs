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

// this prevents clippy complaining about 'OpenBench' not being in backticks
#![allow(clippy::doc_markdown)]

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
//! If this program is given command-line arguments, it will execute them
//! instead of the UCI look. Accepted command-line arguments:
//! - `bench`: this is the same as the regular `bench` command
//! - `genfens <N> seed <S> book <None|path/to/some_book.epd> [T]`: see
//! [`generate_fens()`] for more detail. Note that this is **one** argument
//! because that's how OpenBench will run the argument.
//! - `sample <path/to/some_book.epd>`: see module-level documentation of
//! `game_sampler`. Requires the `sample` feature.
//! - `tune <path/to/positions.fen> [learning rate]`: see module-level
//! documentation of `tune`. Requires the `tune` feature.

use std::{env::args, sync::mpsc::RecvError};

use bench::bench;
use engine::Engine;
use fen_generation::generate_fens;
#[cfg(feature = "sample")]
use game_parser::sample_from_games;
#[cfg(feature = "tune")]
use tune::tune;

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
/// Generation for openings in FEN.
mod fen_generation;
#[cfg(feature = "sample")]
mod game_parser;
/// Static lookup items.
mod lookups;
/// Items related to move generation.
mod movegen;
/// Perft: see <https://www.chessprogramming.org/Perft>.
mod perft;
/// Items related to searching.
mod search;
/// A transposition table.
mod transposition_table;
#[cfg(feature = "tune")]
mod tune;
/// Utility.
mod util;

fn main() -> Result<(), RecvError> {
    let mut args = args();
    args.next();

    if let Some(arg) = args.next() {
        if arg == "bench" {
            bench(args.map(|s| s.leak() as &str));
            return Ok(());
        }
        #[cfg(feature = "sample")]
        if arg == "sample" {
            sample_from_games(args);
            return Ok(());
        }
        #[cfg(feature = "tune")]
        if arg == "tune" {
            let openings_file = &args.next().expect("expected positions file");
            let learning_rate = args.next().and_then(|lr| lr.parse().ok()).unwrap_or(0.1);
            tune(openings_file, learning_rate);
            return Ok(());
        }
        let mut tokens = arg.split_whitespace();
        if tokens.next() == Some("genfens") {
            generate_fens(tokens);
            return Ok(());
        }
    }
    Engine::new().main_loop()
}
