use std::{ops::RangeInclusive, process::exit, sync::mpsc::RecvError, time::Duration};

use crate::{defs::PieceType, engine::Engine, movegen::magic::find_magics};

/// The UCI options this engine supports.
#[derive(Clone, Copy)]
pub struct UciOptions {
    /// The overhead of sending a move from the engine to the GUI.
    move_overhead: Duration,
}

/// The name of the author of this engine.
const ID_AUTHOR: &str = "Beanie";
/// The name of this engine.
const ID_NAME: &str = "Crab";
/// The version of this engine.
const ID_VERSION: &str = env!("CARGO_PKG_VERSION");

#[allow(clippy::missing_docs_in_private_items)]
impl UciOptions {
    /// The range that the move overhead can take, in milliseconds.
    pub const MOVE_OVERHEAD_RANGE: RangeInclusive<u64> = (0..=1000);
}

impl Default for UciOptions {
    fn default() -> Self {
        Self {
            move_overhead: Duration::from_millis(1),
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

        println!("id name {ID_NAME} {ID_VERSION}");
        println!("id author {ID_AUTHOR}");
        println!(
            "option name Move Overhead type spin default {} min {} max {}",
            defaults.move_overhead().as_millis(),
            move_overhead_range.start(),
            move_overhead_range.end(),
        );
    }

    /// Sets the move overhead, in milliseconds, clamped in the range
    /// [`MOVE_OVERHEAD_RANGE`](Self::MOVE_OVERHEAD_RANGE).
    pub fn set_move_overhead(&mut self, duration: u64) {
        self.move_overhead = Duration::from_millis(duration.clamp(
            *Self::MOVE_OVERHEAD_RANGE.start(),
            *Self::MOVE_OVERHEAD_RANGE.end(),
        ));
    }

    /// Returns the move overhead.
    pub const fn move_overhead(&self) -> Duration {
        self.move_overhead
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
            let command = self.uci_rx().recv()?;
            self.handle_command(&command);
        }
    }

    /// Interprets the command given by `line`.
    fn handle_command(&mut self, line: &str) {
        let Some(command) = line.split_whitespace().next() else {
            return;
        };

        match command {
            "f" => {
                find_magics::<{ PieceType::BISHOP.0 }>();
                find_magics::<{ PieceType::ROOK.0 }>();
            }
            "go" => {
                self.go(line);
            }
            "isready" => {
                println!("readyok");
            }
            "p" => {
                self.board().pretty_print();
            }
            "perft" => {
                self.perft::<true, true>(line);
            }
            "position" => {
                self.set_position(line);
            }
            "setoption" => {
                self.set_option(line);
            }
            "uci" => {
                UciOptions::print();
                println!("uciok");
            }
            "ucinewgame" => {
                self.reset();
            }
            "quit" => {
                exit(0);
            }
            other => {
                println!("info string Unrecognised command \"{other}\".");
            }
        }
    }
}
