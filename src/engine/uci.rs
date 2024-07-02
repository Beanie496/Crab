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

use std::{ops::RangeInclusive, process::exit, sync::mpsc::RecvError, time::Duration};

use super::Engine;
use crate::{bench::bench, defs::PieceType, movegen::magic::find_magics};

/// The UCI options this engine supports.
#[derive(Clone, Copy)]
pub struct UciOptions {
    /// The overhead of sending a move from the engine to the GUI.
    move_overhead: Duration,
    /// How many threads should be used.
    threads: usize,
    /// How large the transposition table should be, in MiB.
    hash: usize,
}

/// The name of the author of this engine.
const ID_AUTHOR: &str = "Jasper Shovelton";
/// The name of this engine.
const ID_NAME: &str = "Crab";
/// The version of this engine.
const ID_VERSION: &str = env!("CARGO_PKG_VERSION");

#[allow(clippy::missing_docs_in_private_items)]
impl UciOptions {
    /// The range that the move overhead can take, in milliseconds.
    pub const MOVE_OVERHEAD_RANGE: RangeInclusive<u64> = (0..=1_000);
    /// The range that the number of threads can take.
    pub const THREAD_RANGE: RangeInclusive<usize> = (1..=255);
    /// The range that the hash size can take.
    // hardware limit: 48-bit pointers
    pub const HASH_RANGE: RangeInclusive<usize> = (1..=2_usize.pow(48) / (1024 * 1024));
}

impl Default for UciOptions {
    fn default() -> Self {
        Self {
            move_overhead: Duration::from_millis(1),
            threads: 1,
            hash: 32,
        }
    }
}

impl UciOptions {
    /// Creates new [`UciOptions`] with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Prints the identification of this engine and all the UCI options it
    /// supports.
    fn print() {
        let defaults = Self::default();
        let move_overhead_range = Self::MOVE_OVERHEAD_RANGE;
        let thread_range = Self::THREAD_RANGE;
        let hash_range = Self::HASH_RANGE;

        println!("id name {ID_NAME} {ID_VERSION}");
        println!("id author {ID_AUTHOR}");
        println!(
            "option name Move Overhead type spin default {} min {} max {}",
            defaults.move_overhead().as_millis(),
            move_overhead_range.start(),
            move_overhead_range.end(),
        );
        println!(
            "option name Threads type spin default {} min {} max {}",
            defaults.threads(),
            thread_range.start(),
            thread_range.end(),
        );
        println!(
            "option name Hash type spin default {} min {} max {}",
            defaults.hash(),
            hash_range.start(),
            hash_range.end(),
        );
        println!("option name Clear Hash type button");
    }

    /// Sets the move overhead, in milliseconds, clamped in the range
    /// [`MOVE_OVERHEAD_RANGE`](Self::MOVE_OVERHEAD_RANGE).
    pub fn set_move_overhead(&mut self, duration: u64) {
        self.move_overhead = Duration::from_millis(duration.clamp(
            *Self::MOVE_OVERHEAD_RANGE.start(),
            *Self::MOVE_OVERHEAD_RANGE.end(),
        ));
    }

    /// Sets the thread range, clamped in the range
    /// [`THREAD_RANGE`](Self::THREAD_RANGE).
    pub fn set_threads(&mut self, threads: usize) {
        self.threads = threads.clamp(*Self::THREAD_RANGE.start(), *Self::THREAD_RANGE.end());
    }

    /// Sets the hash range, clamped in the range
    /// [`HASH_RANGE`](Self::HASH_RANGE).
    pub fn set_hash(&mut self, hash: usize) {
        self.hash = hash.clamp(*Self::HASH_RANGE.start(), *Self::HASH_RANGE.end());
    }

    /// Returns the move overhead.
    pub const fn move_overhead(&self) -> Duration {
        self.move_overhead
    }

    /// Returns the number of threads.
    pub const fn threads(&self) -> usize {
        self.threads
    }

    /// Returns the hash size.
    pub const fn hash(&self) -> usize {
        self.hash
    }
}

impl Engine {
    /// Repeatedly waits for a command and executes it according to the UCI
    /// protocol.
    ///
    /// Will run until [`recv()`](std::sync::mpsc::Receiver::recv) on the UCI
    /// receiver returns an error or the process exits. I would make the [`Ok`]
    /// type a never type, but that's experimental.
    pub fn main_loop(&mut self) -> Result<(), RecvError> {
        loop {
            // the sender will never hang up
            let command = self.uci_rx().lock().map_err(|_e| RecvError)?.recv()?;
            self.handle_command(&command);
        }
    }

    /// Interprets the command given by `line`.
    fn handle_command(&mut self, command: &str) {
        let mut tokens = command.split_whitespace();

        match tokens.next() {
            Some("bench") => bench(tokens),
            Some("f") => {
                find_magics::<{ PieceType::BISHOP.0 }>();
                find_magics::<{ PieceType::ROOK.0 }>();
            }
            Some("go") => {
                self.go(tokens);
            }
            Some("isready") => {
                println!("readyok");
            }
            Some("p") => {
                self.board().pretty_print();
            }
            Some("position") => {
                self.set_position(tokens);
            }
            Some("setoption") => {
                self.set_option(tokens);
            }
            Some("uci") => {
                UciOptions::print();
                println!("uciok");
            }
            Some("ucinewgame") => {
                self.reset();
            }
            Some("quit") => {
                exit(0);
            }
            Some(other) => {
                println!("info string Unrecognised command \"{other}\".");
            }
            _ => (),
        }
    }
}
