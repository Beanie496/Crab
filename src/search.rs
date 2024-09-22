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
    fmt::{self, Display, Formatter, Write},
    slice::Iter,
    sync::{
        atomic::{AtomicU64, AtomicU8, Ordering},
        mpsc::Receiver,
        Mutex,
    },
    time::{Duration, Instant},
};

use arrayvec::ArrayVec;

use crate::{
    board::{Board, Key},
    defs::Side,
    evaluation::Evaluation,
    movegen::Move,
    transposition_table::TranspositionTable,
    util::BufferedAtomicU64Counter,
};
pub use aspiration::AspirationWindow;
pub use depth::{CompressedDepth, Depth, Height};
pub use history::{BoardHistory, Histories, HistoryItem, PieceDest};
use movepick::{AllMovesPicker, QuiescenceMovePicker};

/// For running the main alpha-beta search.
mod alpha_beta_search;
/// For running the aspiration loop.
mod aspiration;
/// Items related to [`Depth`] and [`Height`], separated for neatness.
mod depth;
/// Items related to history of any kind.
mod history;
/// For running the iterative deepening loop.
mod iterative_deepening;
/// For selecting which order moves are searched in.
mod movepick;
/// Time management.
mod time;

/// A marker for a type of node to allow searches with generic node types.
#[allow(clippy::missing_docs_in_private_items)]
pub trait Node {
    const IS_PV: bool;
    const IS_ROOT: bool;
}

/// A node with a zero window: is expected not to be in the final PV.
struct NonPvNode;
/// A node that could be in the final PV.
struct PvNode;
/// The node from which the search starts.
pub struct RootNode;

impl Node for NonPvNode {
    const IS_ROOT: bool = false;
    const IS_PV: bool = false;
}

impl Node for PvNode {
    const IS_ROOT: bool = false;
    const IS_PV: bool = true;
}

impl Node for RootNode {
    const IS_ROOT: bool = true;
    const IS_PV: bool = true;
}

/// The type of a search and its limits.
#[derive(Clone, Copy)]
pub enum Limits {
    /// Go under timed conditions.
    Timed {
        /// The time left.
        time: Duration,
        /// The increment.
        inc: Duration,
        /// Moves until the next time control.
        ///
        /// This is set to [`Depth::MAX`] if not given as a parameter.
        moves_to_go: CompressedDepth,
    },
    /// Go to an exact depth.
    Depth(CompressedDepth),
    /// Go to an an exact number of nodes.
    Nodes(u64),
    /// Go for an exact amount of time.
    Movetime(Duration),
    /// Go until told to stop.
    Infinite,
}

/// The current status of the search.
#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum SearchStatus {
    /// Do nothing: continue the search as normal.
    Continue,
    /// Stop the search.
    Stop,
    /// Stop the search and then exit the process.
    Quit,
}

/// Whether White, Black or both sides can do a null move within a search.
#[allow(clippy::missing_docs_in_private_items)]
struct NmpRights {
    rights: u8,
}

/// The principle variation: the current best sequence of moves for both sides.
#[derive(Clone)]
pub struct Pv {
    /// A non-circular queue of moves.
    moves: ArrayVec<Move, { Depth::MAX.to_index() }>,
}

/// The information that [`Worker`]s need to share between them.
pub struct SharedState {
    /// How many positions have been searched across all threads.
    pub nodes: AtomicU64,
    /// The status of the search.
    ///
    /// This is a [`SearchStatus`] that has been converted into a `u8`.
    pub status: AtomicU8,
    /// A receiver to receive UCI commands from.
    pub uci_rx: Mutex<Receiver<String>>,
    /// A hash table of previously-encountered positions.
    pub tt: TranspositionTable,
}

/// Performs the searching.
///
/// It retains the working information of the search, so it can be queried for
/// the final statistics of the search (nodes, time taken, etc.)
pub struct Worker<'a> {
    /// The moment the search started.
    start: Instant,
    /// The maximum depth reached.
    seldepth: Height,
    /// A buffer around `state.nodes`.
    nodes: BufferedAtomicU64Counter<'a>,
    /// The final PV from the initial position.
    root_pv: Pv,
    /// Which side (if at all) null move pruning is allowed for.
    nmp_rights: NmpRights,
    /// The histories used exlusively within the search.
    histories: Histories,
    /// If the search is allowed to print to stdout.
    can_print: bool,
    /// The limits of the search.
    limits: Limits,
    /// How much time we're allocated.
    allocated: Duration,
    /// The overhead of sending a move.
    ///
    /// See [`UciOptions`](crate::uci::UciOptions).
    move_overhead: Duration,
    /// The initial board.
    ///
    /// See [`Board`].
    board: Board,
    /// State that all threads have access to.
    state: &'a SharedState,
    /// The ID of the current thread, starting from 0 for the main thread.
    thread_id: usize,
}

impl NmpRights {
    /// The flag for Black being able to make a null move.
    const BLACK: u8 = 0b01;
    /// The flag for White being able to make a null move.
    const WHITE: u8 = 0b10;
    /// The flag for both sides being able to make a null move.
    const BOTH: u8 = Self::BLACK | Self::WHITE;
}

impl Default for Limits {
    fn default() -> Self {
        Self::Infinite
    }
}

impl Display for Pv {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut ret_str = String::with_capacity(self.len());
        for mv in self.iter() {
            write!(ret_str, "{mv} ")?;
        }
        ret_str.pop();
        write!(f, "{ret_str}")
    }
}

impl From<u8> for SearchStatus {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::Continue,
            1 => Self::Stop,
            _ => Self::Quit,
        }
    }
}

impl From<SearchStatus> for u8 {
    fn from(val: SearchStatus) -> Self {
        val as Self
    }
}

impl Limits {
    /// Sets the increment.
    ///
    /// If `self` is not [`Timed`](Self::Timed), it will be set to
    /// [`Infinite`](Self::Infinite).
    pub fn set_inc(&mut self, increment: Duration) {
        if let &mut Self::Timed { ref mut inc, .. } = self {
            *inc = increment;
        } else {
            *self = Self::Infinite;
        }
    }

    /// Sets the moves to go until the next time control.
    ///
    /// If `self` is not [`Timed`](Self::Timed), it will be set to
    /// [`Infinite`](Self::Infinite).
    pub fn set_moves_to_go(&mut self, mtg: CompressedDepth) {
        if let &mut Self::Timed {
            ref mut moves_to_go,
            ..
        } = self
        {
            *moves_to_go = mtg;
        } else {
            *self = Self::Infinite;
        }
    }

    /// Constructs a new [`Limits::Timed`] variant with the given time, no
    /// increment and the maximum moves to go.
    pub fn new_timed(time: Duration) -> Self {
        Self::Timed {
            time,
            inc: Duration::ZERO,
            moves_to_go: Depth::MAX.into(),
        }
    }
}

impl NmpRights {
    /// Creates new [`NmpRights`] with both rights being enabled.
    const fn new() -> Self {
        Self { rights: Self::BOTH }
    }

    /// Checks if `side` can make a null move.
    #[allow(clippy::assertions_on_constants)]
    fn can_make_null_move(&self, side: Side) -> bool {
        assert!(
            Self::BLACK == Side::BLACK.0 + 1,
            "this function breaks without this precondition"
        );
        assert!(
            Self::WHITE == Side::WHITE.0 + 1,
            "this function breaks without this precondition"
        );

        self.rights & (side.0 + 1) != 0
    }

    /// Adds the right of `side`.
    fn add_right(&mut self, side: Side) {
        debug_assert!(
            !self.can_make_null_move(side),
            "adding a right to a side that already has it"
        );
        self.rights ^= side.0 + 1;
    }

    /// Removes the right of `side`.
    fn remove_right(&mut self, side: Side) {
        debug_assert!(
            self.can_make_null_move(side),
            "removing a nmp right from a side that doesn't have it"
        );
        self.rights ^= side.0 + 1;
    }
}

impl Pv {
    /// Returns a new [`Pv`].
    pub fn new() -> Self {
        Self {
            moves: ArrayVec::new(),
        }
    }

    /// Appends another [`Pv`].
    pub fn append_pv(&mut self, other_pv: &Self) {
        // NOTE: `collect_into()` would be a more ergonomic way to do this,
        // but that's currently nightly
        for &mv in other_pv.iter() {
            self.enqueue(mv);
        }
    }

    /// Adds a [`Move`] to the back of the queue.
    pub fn enqueue(&mut self, mv: Move) {
        debug_assert!(self.moves.len() < self.moves.capacity(), "overflowing a PV");
        // SAFETY: we just checked it's safe to push
        unsafe { self.moves.push_unchecked(mv) };
    }

    /// Clears all moves from the queue.
    pub fn clear(&mut self) {
        self.moves.clear();
    }

    /// Returns an iterator over the moves.
    pub fn iter(&self) -> Iter<'_, Move> {
        self.moves.iter()
    }

    /// Returns the length of the queue.
    pub const fn len(&self) -> usize {
        self.moves.len()
    }
}

impl SharedState {
    /// Created new [`SharedState`].
    pub fn new(uci_rx: Mutex<Receiver<String>>, tt: TranspositionTable) -> Self {
        Self {
            nodes: AtomicU64::new(0),
            status: AtomicU8::new(SearchStatus::Continue.into()),
            uci_rx,
            tt,
        }
    }
}

impl<'a> Worker<'a> {
    /// Creates a new [`Worker`].
    ///
    /// Each field starts off zeroed.
    ///
    /// Note that printing, by default, is only enabled for the main thread.
    pub fn new(state: &'a SharedState, thread_id: usize) -> Self {
        Self {
            start: Instant::now(),
            seldepth: Height::default(),
            nodes: BufferedAtomicU64Counter::new(&state.nodes),
            root_pv: Pv::new(),
            nmp_rights: NmpRights::new(),
            histories: Histories::new(),
            can_print: thread_id == 0,
            limits: Limits::default(),
            allocated: Duration::MAX,
            move_overhead: Duration::ZERO,
            board: Board::new(),
            state,
            thread_id,
        }
    }

    /// Calls [`Self::set_board()`] on `self`.
    pub fn with_board(mut self, board_history: &BoardHistory, board: &Board) -> Self {
        self.set_board(board_history, board);
        self
    }

    /// Sets whether or not the worker should print.
    pub const fn with_printing(mut self, can_print: bool) -> Self {
        self.can_print = can_print;
        self
    }

    /// Calls [`Self::set_limits()`] on `self`.
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.set_limits(limits);
        self
    }

    /// Sets the overhead of sending the best move of the worker.
    pub const fn with_move_overhead(mut self, move_overhead: Duration) -> Self {
        self.move_overhead = move_overhead;
        self
    }

    /// Sets the limits of the worker.
    pub fn set_limits(&mut self, limits: Limits) {
        self.limits = limits;
    }

    /// Sets the board of the worker to the given board and board history.
    pub fn set_board(&mut self, board_history: &BoardHistory, board: &Board) {
        self.histories.board_history.set_to(board_history);
        self.board = *board;
    }

    /// Clears the board history and sets the board to `board`.
    pub fn reset_board(&mut self, board: &Board) {
        self.histories.board_history.clear();
        self.board = *board;
    }

    /// Starts the search.
    ///
    /// Each necessary field is reset.
    ///
    /// Returns the best move.
    pub fn start_search(&mut self) -> Move {
        self.start = Instant::now();
        self.seldepth = Height::default();
        self.nodes.clear();
        self.nmp_rights = NmpRights::new();
        self.histories.age_all();
        self.calculate_time_window();

        self.iterative_deepening()
    }

    /// Returns the number of searched nodes.
    pub fn nodes(&self) -> u64 {
        self.nodes.count()
    }

    /// Returns an iterator over the PV of the current positon.
    #[allow(dead_code)]
    pub fn root_pv(&self) -> Iter<'_, Move> {
        self.root_pv.iter()
    }

    /// Returns the time taken since the search started.
    pub fn elapsed_time(&self) -> Duration {
        self.start.elapsed()
    }

    /// Makes `mv` on `board` and returns whether or not the move was legal.
    pub fn make_move(&mut self, board: &mut Board, mv: Move) -> bool {
        let old_key = board.key();

        if !board.make_move(mv) {
            return false;
        }

        let dest = mv.end();
        let piece = board.piece_on(dest);
        let counter_move_info = PieceDest::new(piece, dest);
        self.push_board_history(HistoryItem::new(old_key, Some(counter_move_info)));
        true
    }

    /// Makes a null move on `board`.
    fn make_null_move(&mut self, board: &mut Board) {
        self.nmp_rights.remove_right(board.side_to_move());
        self.push_board_history(HistoryItem::new(board.key(), None));
        board.make_null_move();
    }

    /// Unmakes the most recent move.
    pub fn unmake_move(&mut self) {
        self.pop_board_history();
    }

    /// Unmakes a null move, assuming `board` was the original board.
    fn unmake_null_move(&mut self, board: &Board) {
        self.nmp_rights.add_right(board.side_to_move());
        self.pop_board_history();
    }

    /// Adds a history item to the stack.
    fn push_board_history(&mut self, item: HistoryItem) {
        debug_assert!(
            self.histories.board_history.len() < self.histories.board_history.capacity(),
            "stack overflow"
        );
        // SAFETY: we just checked that we can push
        unsafe { self.histories.board_history.push_unchecked(item) };
    }

    /// Pops a history item off the stack.
    fn pop_board_history(&mut self) -> Option<HistoryItem> {
        self.histories.board_history.pop()
    }

    /// Check the status of the search.
    ///
    /// This will check the UCI receiver to see if the GUI has told us to stop,
    /// then check to see if we're exceeding the limits of the search.
    fn check_status(&self) -> SearchStatus {
        let status = self.state.status.load(Ordering::Relaxed).into();
        // only check every so often and don't bother wasting more time if
        // we've already stopped
        if !self.nodes.has_empty_buffer() || status != SearchStatus::Continue {
            return status;
        }

        #[allow(clippy::unwrap_used)]
        if let Ok(token) = self.state.uci_rx.lock().unwrap().try_recv() {
            let token = token.trim();
            if token == "stop" {
                self.state
                    .status
                    .store(SearchStatus::Stop.into(), Ordering::Relaxed);
                return SearchStatus::Stop;
            }
            if token == "quit" {
                self.state
                    .status
                    .store(SearchStatus::Quit.into(), Ordering::Relaxed);
                return SearchStatus::Quit;
            }
            if token == "isready" {
                println!("readyok");
            }
        }

        // these are the only variants that can cause a search to exit early
        #[allow(clippy::wildcard_enum_match_arm)]
        match self.limits {
            Limits::Nodes(n) => {
                if self.nodes() >= n {
                    self.state
                        .status
                        .store(SearchStatus::Stop.into(), Ordering::Relaxed);
                    return SearchStatus::Stop;
                }
            }
            Limits::Movetime(m) => {
                if self.start.elapsed() >= m {
                    self.state
                        .status
                        .store(SearchStatus::Stop.into(), Ordering::Relaxed);
                    return SearchStatus::Stop;
                }
            }
            Limits::Timed { time, .. } => {
                // if we've used all of our time and are eating into move
                // overhead, stop the search
                if self.start.elapsed() >= time {
                    self.state
                        .status
                        .store(SearchStatus::Stop.into(), Ordering::Relaxed);
                    return SearchStatus::Stop;
                }
            }
            _ => (),
        };

        SearchStatus::Continue
    }

    /// Calculates if the iterative deepening loop should be exited.
    ///
    /// Assumes that this is being called at the end of the loop.
    fn should_stop(&self, depth: Depth) -> bool {
        if self.check_status() != SearchStatus::Continue {
            return true;
        }

        #[allow(clippy::wildcard_enum_match_arm)]
        match self.limits {
            Limits::Depth(d) => {
                if depth >= Depth::from(d) {
                    if self.is_main_thread() {
                        self.state
                            .status
                            .store(SearchStatus::Stop.into(), Ordering::Relaxed);
                    }
                    return true;
                }
            }
            Limits::Timed { .. } => {
                // if we do not have a realistic chance of finishing the next
                // loop, assume we won't, and stop early.
                if self.is_main_thread() && self.start.elapsed() > self.allocated.mul_f32(0.4) {
                    self.state
                        .status
                        .store(SearchStatus::Stop.into(), Ordering::Relaxed);
                    return true;
                }
            }
            _ => (),
        }

        false
    }

    /// Returns if the root node should print extra information.
    fn should_print(&self) -> bool {
        self.start.elapsed() > Duration::from_millis(3000) && self.can_print
    }

    /// Checks if the position is drawn, either because of repetition or
    /// because of the fifty-move rule.
    fn is_draw(&self, halfmoves: u8, current_key: Key) -> bool {
        // 50mr
        if halfmoves >= 100 {
            return true;
        }

        // check if any past position's key is the same as the current key
        self.histories
            .board_history
            .iter()
            // the previous position is last
            .rev()
            // it is impossible to get a repetition within the past 4
            // halfmoves, so skip the previous 3
            .skip(3)
            // stop after an irreversible position, or stop immediately for
            // halfmoves < 4
            .take(usize::from(halfmoves).saturating_sub(3))
            // skip positions with the wrong stm
            .step_by(2)
            .any(|item| item.key == current_key)
    }

    /// Prints information about a completed search iteration.
    fn print_report(&self, score: Evaluation, pv: &Pv, depth: Depth) {
        let time = self.start.elapsed();
        let nodes = self.nodes.count();
        let nps = 1_000_000 * nodes / time.as_micros().max(1) as u64;

        println!(
            "info depth {depth} seldepth {} {score} hashfull {} nodes {nodes} time {} nps {nps} pv {pv}",
            self.seldepth.0,
            self.state.tt.estimate_hashfull(),
            time.as_millis(),
        );
    }

    /// Checks if the current thread is the main one.
    const fn is_main_thread(&self) -> bool {
        self.thread_id == 0
    }
}
