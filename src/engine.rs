use std::{
    str::FromStr,
    sync::mpsc::{channel, Sender},
    thread::{spawn, JoinHandle},
    time::Duration,
};

use crate::{
    board::Board,
    defs::Side,
    perft::perft,
    search::{iterative_deepening, Limits, SearchInfo, Stop},
    uci::UciOptions,
};

/// Master object that contains all the other major objects.
pub struct Engine {
    /// The internal board.
    ///
    /// See [`Board`].
    board: Board,
    /// A tramsmitter to the search thread to tell it to stop and a join handle
    /// to the same thread.
    search_thread_state: Option<(Sender<Stop>, JoinHandle<()>)>,
    /// The current set options.
    options: UciOptions,
}

impl Clone for Engine {
    fn clone(&self) -> Self {
        Self {
            board: self.board,
            search_thread_state: None,
            options: self.options,
        }
    }
}

impl Engine {
    /// Creates a new [`Engine`].
    ///
    /// Note that the board is completely empty, as UCI specifies that a
    /// `position` command should be given before `go`.
    pub fn new() -> Self {
        Self {
            board: Board::new(),
            search_thread_state: None,
            options: UciOptions::new(),
        }
    }

    /// Interprets and executes the `go` command.
    pub fn go(&mut self, line: &str) {
        let mut tokens = line.split_whitespace();
        let mut limits = Limits::default();

        if tokens.next() != Some("go") {
            return;
        }

        while let Some(token) = tokens.next() {
            let next = tokens.next();

            match token {
                "wtime" => {
                    if self.board().side_to_move() != Side::WHITE {
                        continue;
                    }
                    let wtime = parse_into_nonzero_option(next).map(Duration::from_millis);
                    limits.set_time(wtime);
                }
                "btime" => {
                    if self.board().side_to_move() != Side::BLACK {
                        continue;
                    }
                    let btime = parse_into_nonzero_option(next).map(Duration::from_millis);
                    limits.set_time(btime);
                }
                "winc" => {
                    if self.board().side_to_move() != Side::WHITE {
                        continue;
                    }
                    let winc = parse_into_nonzero_option(next).map(Duration::from_millis);
                    limits.set_inc(winc);
                }
                "binc" => {
                    if self.board().side_to_move() != Side::BLACK {
                        continue;
                    }
                    let binc = parse_into_nonzero_option(next).map(Duration::from_millis);
                    limits.set_inc(binc);
                }
                "movestogo" => limits.set_moves_to_go(parse_into_nonzero_option(next)),
                "depth" => limits.set_depth(parse_into_nonzero_option(next)),
                "nodes" => limits.set_nodes(parse_into_nonzero_option(next)),
                "movetime" => {
                    limits.set_movetime(parse_into_nonzero_option(next).map(Duration::from_millis));
                }
                // if depth is specified and then `infinite` is give, the
                // latter should override the former
                "infinite" => limits.set_infinite(),
                _ => (),
            }
        }

        let (control_tx, control_rx) = channel();
        let search_info = SearchInfo::new(control_rx, limits, self.options().move_overhead());
        let board = *self.board();

        self.stop_search();
        *self.search_thread_state_mut() = Some((
            control_tx,
            spawn(move || {
                iterative_deepening(search_info, board);
            }),
        ));
    }

    /// Given a `perft` command, run [`perft`] to the specified depth.
    pub fn perft<const SHOULD_PRINT: bool, const IS_TIMED: bool>(&self, line: &str) -> u64 {
        let mut tokens = line.split_whitespace();

        if tokens.next() != Some("perft") {
            return 0;
        }
        let Some(depth) = parse_into_nonzero_option(tokens.next()) else {
            return 0;
        };

        perft::<SHOULD_PRINT, IS_TIMED>(self.board(), depth)
    }

    /// Sets the board to a position specified by the `position` command.
    ///
    /// Will not change anything if the command fails to get parsed
    /// successfully.
    pub fn set_position(&mut self, line: &str) {
        if let Ok(b) = line.parse() {
            *self.board_mut() = b;
        }
    }

    /// Sets a UCI option from a `setoption` command.
    pub fn set_option(&mut self, line: &str) {
        let mut tokens = line.split_whitespace();

        if tokens.next() != Some("setoption") {
            return;
        }
        if tokens.next() != Some("name") {
            return;
        }

        let Some(token) = tokens.next() else { return };
        // more options added later, so be quiet, clippy
        #[allow(clippy::single_match)]
        match token {
            "Move" => {
                if tokens.next() != Some("Overhead") {
                    return;
                }
                if tokens.next() != Some("value") {
                    return;
                }

                if let Some(d) = parse_option(tokens.next()) {
                    self.options_mut().set_move_overhead(d);
                }
            }
            _ => (),
        }
    }

    /// Stops the search, if any.
    pub fn stop_search(&mut self) {
        // we don't particularly care if it's already stopped, we just want it
        // to stop.
        #[allow(unused_must_use)]
        if let Some((tx, handle)) = self.search_thread_state_mut().take() {
            tx.send(Stop);
            #[allow(clippy::use_debug)]
            handle
                .join()
                .map_err(|e| println!("info string Warning! Search thread panicked: {e:?}"));
        }
    }

    /// Sets the engine to its initial state. Should be called after the
    /// `ucinewgame` command.
    pub fn reset(&mut self) {
        self.stop_search();
        self.board_mut().set_startpos();
    }

    /// Returns a reference to the board.
    pub const fn board(&self) -> &Board {
        &self.board
    }

    /// Returns a mutable reference to the board.
    pub fn board_mut(&mut self) -> &mut Board {
        &mut self.board
    }

    /// Returns a mutable reference to the search thread state.
    pub fn search_thread_state_mut(&mut self) -> &mut Option<(Sender<Stop>, JoinHandle<()>)> {
        &mut self.search_thread_state
    }

    /// Returns a reference to the UCI options.
    pub const fn options(&self) -> &UciOptions {
        &self.options
    }

    /// Returns a mutable reference to the UCI options.
    pub fn options_mut(&mut self) -> &mut UciOptions {
        &mut self.options
    }
}

/// Parses an `Option<&str>` into an `Option<T>`.
///
/// If the parse fails, it will return [`None`].
fn parse_option<T: FromStr>(num: Option<&str>) -> Option<T> {
    num.and_then(|t| t.parse::<T>().ok())
}

/// Parses an `Option<&str>` into an `Option<T>`.
///
/// Returns [`None`] if the result of the parse is 0 or an `Err`.
fn parse_into_nonzero_option<T: FromStr + PartialEq<T> + From<u8>>(num: Option<&str>) -> Option<T> {
    parse_option(num).and_then(|t| if t == T::from(0) { None } else { Some(t) })
}
