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
    ops::RangeInclusive,
    process::exit,
    str::FromStr,
    sync::{
        atomic::Ordering,
        mpsc::{channel, RecvError},
        Mutex,
    },
    thread::{scope, spawn},
    time::Duration,
};

use crate::{
    bench::bench,
    board::Board,
    defs::{File, PieceType, Side},
    lookups::magic::find_magics,
    movegen::Move,
    perft::perft,
    search::{
        BoardHistory, CompressedDepth, HistoryItem, Limits, PieceDest, SearchStatus, SharedState,
        Worker,
    },
    transposition_table::TranspositionTable,
};

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
    pub const THREAD_RANGE: RangeInclusive<usize> = (1..=256);
    /// The range that the hash size can take.
    // hardware limit: 48-bit pointers
    pub const HASH_RANGE: RangeInclusive<usize> = (1..=2_usize.pow(48) / (1024 * 1024));
}

impl Default for UciOptions {
    fn default() -> Self {
        Self {
            move_overhead: Duration::from_millis(16),
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

/// Repeatedly waits for a command and executes it according to the UCI
/// protocol.
///
/// Will run until [`recv()`](std::sync::mpsc::Receiver::recv) on the UCI
/// receiver returns an error or the process exits. I would make the [`Ok`]
/// type a never type, but that's experimental.
pub fn main_loop() -> Result<(), RecvError> {
    let (uci_tx, uci_rx) = channel();

    spawn(move || {
        let stdin = stdin();

        for command in stdin.lines() {
            let command = command.expect("Error while reading from stdin");
            uci_tx
                .send(command)
                .expect("It's not possible for this thread to exit later than the main thread.");
        }
    });

    let mut options = UciOptions::new();
    let mut board = Board::new();
    let mut board_history = BoardHistory::new();
    let tt = TranspositionTable::with_capacity(options.hash());
    let mut state = SharedState::new(Mutex::new(uci_rx), tt);
    let mut workers = create_workers(
        &state,
        &board_history,
        &board,
        options.threads(),
        options.move_overhead(),
    );

    loop {
        // the sender will never hang up
        let command = state.uci_rx.lock().map_err(|_| RecvError)?.recv()?;
        let mut tokens = command.split_whitespace();

        match tokens.next() {
            Some("bench") => {
                bench(tokens);
            }
            Some("f") => {
                find_magics::<{ PieceType::BISHOP.0 }>();
                find_magics::<{ PieceType::ROOK.0 }>();
            }
            Some("go") => {
                go(tokens, &board, &mut workers, &state);
            }
            Some("ispseudolegal") => {
                let is_pseudolegal = tokens
                    .next()
                    .and_then(|mv| parse_move(mv, &board))
                    .filter(|&mv| board.is_pseudolegal(mv))
                    .is_some();
                println!("{is_pseudolegal}");
            }
            Some("ispseudolegalkiller") => {
                let is_pseudolegal_killer = tokens
                    .next()
                    .and_then(|mv| parse_move(mv, &board))
                    .filter(|&mv| board.is_pseudolegal_killer(mv))
                    .is_some();
                println!("{is_pseudolegal_killer}");
            }
            Some("isquiet") => {
                let is_quiet = tokens
                    .next()
                    .and_then(|mv| parse_move(mv, &board))
                    .filter(|&mv| board.is_quiet(mv))
                    .is_some();
                println!("{is_quiet}");
            }
            Some("isready") => {
                println!("readyok");
            }
            Some("p") => {
                board.pretty_print();
            }
            Some("position") => {
                set_position(tokens, &mut board_history, &mut board);
                for worker in &mut workers {
                    worker.set_board(&board_history, &board);
                }
            }
            Some("see") => {
                let is_winning_exchange = tokens
                    .next()
                    .and_then(|mv| parse_move(mv, &board))
                    .filter(|&mv| board.is_quiet(mv))
                    .is_some();
                println!("{is_winning_exchange}");
            }
            Some("setoption") => {
                set_option(tokens, &mut options, &mut state);
                workers = create_workers(
                    &state,
                    &board_history,
                    &board,
                    options.threads(),
                    options.move_overhead(),
                );
            }
            Some("uci") => {
                UciOptions::print();
                println!("uciok");
            }
            Some("ucinewgame") => {
                board.set_startpos();
                board_history.clear();
                state.tt.clear();
                workers = create_workers(
                    &state,
                    &board_history,
                    &board,
                    options.threads(),
                    options.move_overhead(),
                );
            }
            Some("quit") => {
                break Ok(());
            }
            Some(other) => {
                println!("info string Unrecognised command \"{other}\".");
            }
            _ => (),
        }
    }
}

/// Interprets and executes the `go` command.
pub fn go<'a, 'b, T>(
    mut given_limits: T,
    board: &Board,
    workers: &mut [Worker<'a>],
    state: &SharedState,
) where
    T: Iterator<Item = &'b str>,
{
    let mut limits = Limits::default();

    while let Some(token) = given_limits.next() {
        let next = given_limits.next();

        match token {
            "wtime" if board.side_to_move() == Side::WHITE => {
                if let Some(time) = parse_time(next) {
                    limits = Limits::new_timed(time);
                }
            }
            "btime" if board.side_to_move() == Side::BLACK => {
                if let Some(time) = parse_time(next) {
                    limits = Limits::new_timed(time);
                }
            }
            "winc" if board.side_to_move() == Side::WHITE => {
                if let Some(time) = parse_time(next) {
                    limits.set_inc(time);
                }
            }
            "binc" if board.side_to_move() == Side::BLACK => {
                if let Some(time) = parse_time(next) {
                    limits.set_inc(time);
                }
            }
            "movestogo" => {
                if let Some(moves) = parse_into_nonzero_option(next) {
                    limits.set_moves_to_go(CompressedDepth(moves));
                }
            }
            "depth" => {
                if let Some(depth) = parse_into_nonzero_option(next) {
                    limits = Limits::Depth(CompressedDepth(depth));
                }
            }
            "nodes" => {
                if let Some(nodes) = parse_into_nonzero_option(next) {
                    limits = Limits::Nodes(nodes);
                }
            }
            "movetime" => {
                if let Some(movetime) = parse_time(next) {
                    limits = Limits::Movetime(movetime);
                }
            }
            "infinite" => limits = Limits::Infinite,
            "perft" => {
                if let Some(depth) = parse_into_nonzero_option(next) {
                    perft::<true, true>(board, depth);
                }
                return;
            }
            _ => (),
        }
    }

    state.nodes.store(0, Ordering::Relaxed);
    state
        .status
        .store(SearchStatus::Continue.into(), Ordering::Relaxed);

    let best_move = scope(|s| {
        let mut main_handle = None;

        for (id, worker) in workers.iter_mut().enumerate() {
            worker.set_limits(limits);
            let handle = s.spawn(move || worker.start_search());
            if id == 0 {
                main_handle = Some(handle);
            }
        }

        main_handle
            .expect("no workers were started")
            .join()
            .expect("a thread panicked during the search")
    });
    println!("bestmove {best_move}");

    if SearchStatus::from(state.status.load(Ordering::Relaxed)) == SearchStatus::Quit {
        exit(0);
    }
}

/// Sets the board to a position specified by the `position` command.
///
/// Will not change anything if the command fails to get parsed
/// successfully.
pub fn set_position<'b, T>(mut tokens: T, old_history: &mut BoardHistory, old_board: &mut Board)
where
    T: Iterator<Item = &'b str>,
{
    let mut board = Board::new();
    let mut board_history = BoardHistory::new();

    match tokens.next() {
        Some("startpos") => board.set_startpos(),
        Some("fen") => {
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

    // if there are no moves to begin with, this loop will just be skipped
    for mv in tokens {
        let Some(mv) = parse_move(mv, &board).filter(|&mv| board.is_pseudolegal(mv)) else {
            return;
        };

        let dest = mv.end();
        let piece = board.piece_on(mv.start());
        let counter_move_info = PieceDest::new(piece, dest);
        board_history.push(HistoryItem::new(board.key(), Some(counter_move_info)));

        if !board.make_move(mv) {
            return;
        }

        // we can safely discard all moves before an irreversible move
        if board.halfmoves() == 0 {
            board_history.clear();
        }
    }

    *old_board = board;
    old_history.set_to(&board_history);
}

/// Sets a UCI option from a `setoption` command.
pub fn set_option<'b, T>(mut tokens: T, options: &mut UciOptions, state: &mut SharedState)
where
    T: Iterator<Item = &'b str>,
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
                options.set_move_overhead(d);
            }
        }
        Some("Threads") => {
            if tokens.next() != Some("value") {
                return;
            }

            if let Some(t) = parse_option(tokens.next()) {
                options.set_threads(t);
            }
        }
        Some("Hash") => {
            if tokens.next() != Some("value") {
                return;
            }

            if let Some(h) = parse_option(tokens.next()) {
                options.set_hash(h);
                state.tt.resize(h);
            }
        }
        Some("Clear") => {
            if tokens.next() != Some("Hash") {
                return;
            }
            state.tt.clear();
        }
        _ => (),
    }
}

/// Creates `threads` [`Worker`]s.
fn create_workers<'a>(
    state: &'a SharedState,
    board_history: &BoardHistory,
    board: &Board,
    threads: usize,
    move_overhead: Duration,
) -> Vec<Worker<'a>> {
    (0..)
        .map(|id| {
            Worker::new(state, id)
                .with_board(board_history, board)
                .with_move_overhead(move_overhead)
        })
        .take(threads)
        .collect()
}

/// Parses a move in long algebraic notation into a [`Move`].
///
/// Returns [`None`] if the move cannot be parsed.
fn parse_move(string: &str, board: &Board) -> Option<Move> {
    let start = string.get(0..=1)?.parse().ok()?;
    let end = string.get(2..=3)?.parse().ok()?;
    let piece = board.piece_on(start);

    // if the move is fully legal, these will always be correct
    let is_promotion = string.len() == 5;
    let is_castling = PieceType::from(piece) == PieceType::KING
        && File::from(start).0.abs_diff(File::from(end).0) == 2;
    let is_en_passant = PieceType::from(piece) == PieceType::PAWN && end == board.ep_square();

    if is_castling {
        let is_white = board.side_to_move() == Side::WHITE;
        let is_kingside = File::from(end) >= File::FILE5;

        Some(Move::new_castle_any(is_white, is_kingside))
    } else if is_promotion {
        // SAFETY: `mv` has a non-zero length so `chars()` must return
        // something
        let promotion_char = unsafe { string.chars().next_back().unwrap_unchecked() };
        let piece_type = PieceType::try_from(promotion_char).ok()?;

        Some(Move::new_promo_any(start, end, piece_type))
    } else if is_en_passant {
        Some(Move::new_en_passant(start, end))
    } else {
        Some(Move::new(start, end))
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
