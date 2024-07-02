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

use std::{
    io::stdin,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicU64},
        mpsc::{channel, Receiver},
        Mutex,
    },
    thread::{scope, spawn},
    time::{Duration, Instant},
};

use crate::{
    board::{Board, Key},
    defs::{MoveType, PieceType, Side, Square},
    evaluation::MATE_BOUND,
    movegen::{generate_moves, Move},
    perft::perft,
    search::{
        iterative_deepening, time::calculate_time_window, Depth, Limits, SearchReferences,
        SearchReport,
    },
    transposition_table::TranspositionTable,
    util::Stack,
};
use uci::UciOptions;

/// Items for handling UCI input.
pub mod uci;

/// A stack of zobrist keys.
pub type ZobristStack = Stack<Key, { Depth::MAX as usize }>;

/// Master object that contains all the other major objects.
pub struct Engine {
    /// The internal board.
    ///
    /// See [`Board`].
    board: Board,
    /// The current set options.
    options: UciOptions,
    /// A receiver to receive UCI commands from.
    uci_rx: Mutex<Receiver<String>>,
    /// A stack of zobrist hashes of previous board states, beginning from the
    /// initial `position fen ...` command.
    ///
    /// The first (bottom) element is the initial board and the top element is
    /// the current board.
    past_zobrists: ZobristStack,
    /// A hash table of previously-encountered positions.
    tt: TranspositionTable,
}

impl Engine {
    /// Creates a new [`Engine`] and spawns a thread to receive UCI input from.
    ///
    /// Note that the board is completely empty, as UCI specifies that a
    /// `position` command should be given before `go`.
    pub fn new() -> Self {
        let (tx, rx) = channel();

        spawn(move || {
            let stdin = stdin();

            for command in stdin.lines() {
                let command = command.expect("Error while reading from stdin");
                tx.send(command).expect(
                    "It's not possible for this thread to exit later than the main thread.",
                );
            }
        });

        let options = UciOptions::new();
        Self {
            board: Board::new(),
            options,
            uci_rx: Mutex::new(rx),
            past_zobrists: Stack::new(),
            tt: TranspositionTable::with_capacity(options.hash()),
        }
    }

    /// Interprets and executes the `go` command.
    pub fn go<'a, T>(&mut self, mut options: T)
    where
        T: Iterator<Item = &'a str>,
    {
        let start = Instant::now();
        let mut limits = Limits::default();

        while let Some(token) = options.next() {
            let next = options.next();

            match token {
                "wtime" if self.board().side_to_move() == Side::WHITE => {
                    limits.set_time(parse_time(next));
                }
                "btime" if self.board().side_to_move() == Side::BLACK => {
                    limits.set_time(parse_time(next));
                }
                "winc" if self.board().side_to_move() == Side::WHITE => {
                    limits.set_inc(parse_time(next));
                }
                "binc" if self.board().side_to_move() == Side::BLACK => {
                    limits.set_inc(parse_time(next));
                }
                "movestogo" => limits.set_moves_to_go(parse_into_nonzero_option(next)),
                "depth" => limits.set_depth(parse_into_nonzero_option(next)),
                "nodes" => limits.set_nodes(parse_into_nonzero_option(next)),
                "movetime" => limits.set_movetime(parse_time(next)),
                "infinite" => limits.set_infinite(),
                "perft" => {
                    if let Some(depth) = parse_into_nonzero_option(next) {
                        perft::<true, true>(self.board(), depth);
                    }
                    return;
                }
                _ => (),
            }
        }

        let allocated = calculate_time_window(limits, start, self.options().move_overhead());
        let nodes = AtomicU64::new(0);
        let should_stop = AtomicBool::new(false);

        scope(|s| {
            let mut handles = Vec::with_capacity(self.options().threads());

            for thread in 0..self.options().threads() {
                let search_refs = SearchReferences::new(
                    start,
                    &nodes,
                    &should_stop,
                    limits,
                    allocated,
                    self.uci_rx(),
                    self.past_zobrists().clone(),
                    self.tt(),
                    thread,
                );
                handles.push(s.spawn(|| iterative_deepening(search_refs, *self.board())));
            }

            let reports = handles
                .into_iter()
                .map(|handle| handle.join().expect("a thread panicked during the search"))
                .collect::<Vec<_>>();
            let best_move = best_move_of(&reports);
            println!("bestmove {best_move}");
        });
    }

    /// Sets the board to a position specified by the `position` command.
    ///
    /// Will not change anything if the command fails to get parsed
    /// successfully.
    pub fn set_position<'a, T>(&mut self, mut tokens: T)
    where
        T: Iterator<Item = &'a str>,
    {
        let mut board = Board::new();
        let mut zobrists = Stack::new();

        match tokens.next() {
            Some("startpos") => board.set_startpos(),
            Some("fen") => {
                // Creating a new `String` is annoying, but probably not too
                // expensive, considering this only happens a few tens of times
                // per game.
                let mut fen_str = String::with_capacity(128);

                // The FEN string should have exactly 6 tokens - more or fewer
                // should raise an error later or now respectively.
                for _ in 0..6 {
                    let Some(token) = tokens.next() else {
                        return;
                    };
                    fen_str.push_str(token);
                    fen_str.push(' ');
                }

                if let Ok(b) = fen_str.parse() {
                    board = b;
                } else {
                    return;
                }
            }
            _ => return,
        };

        // check if we have any moves to parse
        if let Some(token) = tokens.next() {
            if token != "moves" {
                return;
            }
        }
        zobrists.push(board.zobrist());

        // if there are no moves to begin with, this loop will just be skipped
        for mv in tokens {
            let mut moves = generate_moves::<{ MoveType::ALL }>(&board);

            let Some(start) = mv.get(0..=1) else {
                return;
            };
            let Ok(start) = Square::from_str(start) else {
                return;
            };
            let Some(end) = mv.get(2..=3) else {
                return;
            };
            let Ok(end) = Square::from_str(end) else {
                return;
            };

            // Each move should be exactly 4 characters; if it's a promotion,
            // the last char will be the promotion char.
            let Some(mv) = (if mv.len() == 5 {
                // SAFETY: `mv` has a non-zero length so `chars()` must return
                // something
                let promotion_char = unsafe { mv.chars().next_back().unwrap_unchecked() };
                let Ok(piece_type) = PieceType::try_from(promotion_char) else {
                    return;
                };
                moves.move_with_promo(start, end, piece_type)
            } else {
                moves.move_with(start, end)
            }) else {
                return;
            };

            if !board.make_move(mv) {
                return;
            }

            // we can safely discard all moves before an irreversible move
            if board.halfmoves() == 0 {
                zobrists.clear();
            }

            zobrists.push(board.zobrist());
        }

        *self.board_mut() = board;
        *self.past_zobrists_mut() = zobrists;
    }

    /// Sets a UCI option from a `setoption` command.
    pub fn set_option<'a, T>(&mut self, mut tokens: T)
    where
        T: Iterator<Item = &'a str>,
    {
        if tokens.next() != Some("name") {
            return;
        }

        match tokens.next() {
            Some("Move") => {
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
            Some("Threads") => {
                if tokens.next() != Some("value") {
                    return;
                }

                if let Some(t) = parse_option(tokens.next()) {
                    self.options_mut().set_threads(t);
                }
            }
            Some("Hash") => {
                if tokens.next() != Some("value") {
                    return;
                }

                if let Some(h) = parse_option(tokens.next()) {
                    self.options_mut().set_hash(h);
                    self.tt_mut().resize(h);
                }
            }
            Some("Clear") => {
                if tokens.next() != Some("Hash") {
                    return;
                }
                self.tt_mut().clear();
            }
            _ => (),
        }
    }

    /// Sets the state of the engine to the starting position. Should be called
    /// after the `ucinewgame` command.
    pub fn reset(&mut self) {
        self.board_mut().set_startpos();
        self.past_zobrists_mut().clear();
        let board_zobrist = self.board().zobrist();
        self.past_zobrists_mut().push(board_zobrist);
        self.tt_mut().clear();
    }

    /// Returns a reference to the board.
    pub const fn board(&self) -> &Board {
        &self.board
    }

    /// Returns a mutable reference to the board.
    pub fn board_mut(&mut self) -> &mut Board {
        &mut self.board
    }

    /// Returns a reference to the UCI options.
    pub const fn options(&self) -> &UciOptions {
        &self.options
    }

    /// Returns a mutable reference to the UCI options.
    pub fn options_mut(&mut self) -> &mut UciOptions {
        &mut self.options
    }

    /// Returns a reference to the receiver of the inputted UCI commands.
    pub const fn uci_rx(&self) -> &Mutex<Receiver<String>> {
        &self.uci_rx
    }

    /// Returns a reference to the current stack of zobrist hashes of board
    /// states.
    pub const fn past_zobrists(&self) -> &ZobristStack {
        &self.past_zobrists
    }

    /// Returns a mutable reference to the current stack of zobrist hashes of
    /// board states.
    pub fn past_zobrists_mut(&mut self) -> &mut ZobristStack {
        &mut self.past_zobrists
    }

    /// Returns a reference to the transposition table.
    pub const fn tt(&self) -> &TranspositionTable {
        &self.tt
    }

    /// Returns a mutable reference to the transposition table.
    pub fn tt_mut(&mut self) -> &mut TranspositionTable {
        &mut self.tt
    }
}

/// Given an array of [`SearchReport`]s, select the best move of the best
/// report.
fn best_move_of(search_reports: &[SearchReport]) -> Move {
    let (mut best_report, other_reports) =
        search_reports.split_first().expect("No reports to select");

    for report in other_reports {
        let best_depth = best_report.depth;
        let best_score = best_report.score;
        let depth = report.depth;
        let score = report.score;

        // if the depth is higher, the score will be more accurate (even if
        // it's lower). The exception is if the lower depth obtained a mate
        // score which the higher depth missed.
        if depth > best_depth && (best_score < MATE_BOUND || score > best_score) {
            best_report = report;
        }
        // Use the faster of the two mates (if the score is a mate) or the
        // better score if the two depths are the same.
        if (depth == best_depth || score >= MATE_BOUND) && score > best_score {
            best_report = report;
        }
    }
    best_report.best_move()
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

/// Parses an `Option<&str>` into an `Option<Duration>`, where the string is
/// some kind of length of time.
///
/// Returns `None` if `num` cannot be parsed. If `num` can be parsed but is
/// negative, it will return [`Some`] with a small amount of time to account
/// for CCRL.
fn parse_time(num: Option<&str>) -> Option<Duration> {
    parse_option::<i32>(num)
        // pre-emptive CCRL fix from Alexandria: if the GUI gives us a negative
        // time, take advantage of the 5-second grace period and assume we do
        // actually have a little time
        .map(|t| if t < 0 { 1000 } else { t })
        // SAFETY: we just made sure `t` is positive
        .map(|t| unsafe { u64::try_from(t).unwrap_unchecked()})
        .map(Duration::from_millis)
}
