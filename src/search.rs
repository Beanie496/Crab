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
    mem::MaybeUninit,
    sync::{mpsc::Receiver, Mutex},
    time::{Duration, Instant},
};

use arrayvec::ArrayVec;

use crate::{
    board::{Board, Key},
    defs::Side,
    evaluation::{is_mate, moves_to_mate, Eval},
    movegen::Move,
    transposition_table::TranspositionTable,
    util::{get_unchecked, insert_unchecked},
};
use time::calculate_time_window;

/// For running the main alpha-beta search.
pub mod alpha_beta_search;
/// For running the aspiration loop.
pub mod aspiration;
/// For running the iterative deepening loop.
pub mod iterative_deepening;
/// For selecting which order moves are searched in.
mod movepick;
/// Time management.
pub mod time;

/// The difference between the root or leaf node (for height or depth
/// respectively) and the current node.
pub type Depth = u8;
/// A stack of zobrist keys.
pub type ZobristKeyStack = ArrayVec<Key, { Depth::MAX as usize }>;

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
        /// This is set to [`u8::MAX`] if not given as a parameter.
        moves_to_go: u8,
    },
    /// Go to an exact depth.
    Depth(u8),
    /// Go to an an exact number of nodes.
    Nodes(u64),
    /// Go for an exact amount of time.
    Movetime(Duration),
    /// Go until told to stop.
    Infinite,
}

/// The current status of the search.
#[derive(Clone, Copy, Eq, PartialEq)]
enum SearchStatus {
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
    moves: [MaybeUninit<Move>; Depth::MAX as usize],
    /// Index of the first [`Move`].
    ///
    /// This will be equal to `first_empty` if there are no [`Move`]s.
    first_item: u8,
    /// Index of the first empty space.
    first_empty: u8,
}

struct PvIter<'a> {
    /// A slice of moves to iterate over.
    moves: &'a [MaybeUninit<Move>],
    /// The index of the next move to return.
    index: usize,
}

/// The information that [`Worker`]s need to share between them.
pub struct SharedState {
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
    seldepth: Depth,
    /// How many positions have been searched.
    nodes: u64,
    /// The final PV from the initial position.
    root_pv: Pv,
    /// The status of the search: continue, stop or quit?
    status: SearchStatus,
    /// Which side (if at all) null move pruning is allowed for.
    nmp_rights: NmpRights,
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
    /// A stack of keys of previous board states, beginning from the initial
    /// `position fen ...` command.
    ///
    /// The first (bottom) element is the initial board and the top element is
    /// the current board.
    past_keys: ZobristKeyStack,
    /// State that all threads have access to.
    state: &'a SharedState,
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

impl Iterator for Pv {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.dequeue()
    }
}

impl<'a> Iterator for PvIter<'a> {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        (self.index < self.moves.len()).then(|| {
            let mv = *get_unchecked(self.moves, self.index);
            self.index += 1;
            // SAFETY: all moves within `self.moves` are initialised
            unsafe { mv.assume_init_read() }
        })
    }
}

impl Limits {
    /// Sets the allocated time to the value in `time`.
    ///
    /// If `self` is not already [`Timed`](Self::Timed), it will be set to
    /// [`Timed`](Self::Timed) with the given time. If `time` is [`None`], the
    /// variant of `self` will be set to [`Infinite`](Self::Infinite).
    pub fn set_time(&mut self, time: Option<Duration>) {
        if let Some(t) = time {
            if let &mut Self::Timed { ref mut time, .. } = self {
                *time = t;
            } else {
                *self = Self::new_timed(t);
            }
        } else {
            self.set_infinite();
        }
    }

    /// Sets the increment to the value in `inc`.
    ///
    /// If `self` is not [`Timed`](Self::Timed), it will be set to
    /// [`Infinite`](Self::Infinite). If `inc` is [`None`], it will be ignored.
    pub fn set_inc(&mut self, inc: Option<Duration>) {
        if let Some(i) = inc {
            if let &mut Self::Timed { ref mut inc, .. } = self {
                *inc = i;
            } else {
                self.set_infinite();
            }
        }
    }

    /// Sets the moves to go to the value in `moves_to_go`.
    ///
    /// If `self` is not [`Timed`](Self::Timed), it will be set to
    /// [`Infinite`](Self::Infinite). If `moves_to_go` is [`None`], it will be
    /// ignored.
    pub fn set_moves_to_go(&mut self, moves_to_go: Option<Depth>) {
        if let Some(mtg) = moves_to_go {
            if let &mut Self::Timed {
                ref mut moves_to_go,
                ..
            } = self
            {
                *moves_to_go = mtg;
            } else {
                self.set_infinite();
            }
        }
    }

    /// Sets `self` to [`Depth(depth)`](Self::Depth).
    ///
    /// If `depth` is [`None`], `self` will be set to
    /// [`Infinite`](Self::Infinite).
    pub fn set_depth(&mut self, depth: Option<Depth>) {
        if let Some(depth) = depth {
            *self = Self::Depth(depth);
        } else {
            self.set_infinite();
        }
    }

    /// Sets `self` to [`Nodes(nodes)`](Self::Nodes).
    ///
    /// If `nodes` is [`None`], `self` will be set to
    /// [`Infinite`](Self::Infinite).
    pub fn set_nodes(&mut self, nodes: Option<u64>) {
        if let Some(nodes) = nodes {
            *self = Self::Nodes(nodes);
        } else {
            self.set_infinite();
        }
    }

    /// Sets `self` to [`Movetime(movetime)`](Self::Movetime).
    ///
    /// If `nodes` is [`None`], `self` will be set to
    /// [`Infinite`](Self::Infinite).
    pub fn set_movetime(&mut self, movetime: Option<Duration>) {
        if let Some(mt) = movetime {
            *self = Self::Movetime(mt);
        } else {
            self.set_infinite();
        }
    }

    /// Sets `self` to [`Infinite`](Self::Infinite).
    pub fn set_infinite(&mut self) {
        *self = Self::Infinite;
    }

    /// Constructs a new [`Limits::Timed`] variant with the given time, no
    /// increment and the maximum moves to go.
    const fn new_timed(time: Duration) -> Self {
        Self::Timed {
            time,
            inc: Duration::ZERO,
            moves_to_go: u8::MAX,
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
    /// Returns a new 0-initialised [`Pv`].
    pub const fn new() -> Self {
        Self {
            moves: [MaybeUninit::uninit(); Depth::MAX as usize],
            first_item: 0,
            first_empty: 0,
        }
    }

    /// Appends and consumes another [`Pv`] without clearing it afterwards.
    fn append_pv(&mut self, other_pv: &mut Self) {
        // NOTE: `collect_into()` would be a more ergonomic way to do this,
        // but that's currently nightly
        for mv in other_pv {
            self.enqueue(mv);
        }
    }

    /// Adds a [`Move`] to the back of the queue.
    fn enqueue(&mut self, mv: Move) {
        insert_unchecked(
            &mut self.moves,
            self.first_empty as usize,
            MaybeUninit::new(mv),
        );
        self.first_empty += 1;
    }

    /// Removes a [`Move`] from the front of the queue.
    fn dequeue(&mut self) -> Option<Move> {
        (self.first_item < self.first_empty).then(|| {
            let mv = *get_unchecked(&self.moves, self.first_item as usize);
            self.first_item += 1;
            // SAFETY: all moves within `self.first_item..self.first_empty` are
            // initialised
            unsafe { mv.assume_init_read() }
        })
    }

    /// Clears all moves from the queue.
    fn clear(&mut self) {
        self.first_item = 0;
        self.first_empty = 0;
    }

    /// Returns a slice to the moves of the queue.
    fn iter(&self) -> PvIter<'_> {
        PvIter::new(self)
    }

    /// Calculates the length of the queue.
    fn len(&self) -> usize {
        usize::from(self.first_empty - self.first_item)
    }
}

impl<'a> PvIter<'a> {
    /// Creates a new [`PvIter`] which contains a slice of all the moves in
    /// `pv`.
    fn new(pv: &'a Pv) -> Self {
        Self {
            moves: &pv.moves[(pv.first_item as usize)..(pv.first_empty as usize)],
            index: 0,
        }
    }
}

impl SharedState {
    /// Created new [`SharedState`].
    pub const fn new(uci_rx: Mutex<Receiver<String>>, tt: TranspositionTable) -> Self {
        Self { uci_rx, tt }
    }
}

impl<'a> Worker<'a> {
    /// Creates a new [`Worker`].
    ///
    /// Each field starts off zeroed.
    pub fn new(state: &'a SharedState) -> Self {
        Self {
            start: Instant::now(),
            seldepth: 0,
            nodes: 0,
            root_pv: Pv::new(),
            status: SearchStatus::Continue,
            nmp_rights: NmpRights::new(),
            can_print: true,
            limits: Limits::default(),
            allocated: Duration::MAX,
            move_overhead: Duration::ZERO,
            board: Board::new(),
            past_keys: ZobristKeyStack::new(),
            state,
        }
    }

    /// Calls [`Self::set_board()`] on `self`.
    pub fn with_board(mut self, past_keys: ZobristKeyStack, board: &Board) -> Self {
        self.set_board(past_keys, board);
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

    /// Sets the board of the worker to the given board and zobrist key
    /// stack.
    ///
    /// The top entry of `past_keys` is assumed to be the key of `board`.
    pub fn set_board(&mut self, past_keys: ZobristKeyStack, board: &Board) {
        self.past_keys = past_keys;
        self.board = *board;
    }

    /// Clears the key stack, sets the board to `board` and adds the key of the
    /// board to the stack.
    pub fn reset_board(&mut self, board: &Board) {
        self.past_keys.clear();
        self.past_keys.push(board.key());
        self.board = *board;
    }

    /// Starts the search.
    ///
    /// Each necessary field is reset.
    pub fn start_search(&mut self) {
        self.start = Instant::now();
        self.seldepth = 0;
        self.nodes = 0;
        self.status = SearchStatus::Continue;
        self.nmp_rights = NmpRights::new();
        self.allocated = calculate_time_window(self.start, self.limits, self.move_overhead);

        self.iterative_deepening();
    }

    /// Returns the number of searched nodes.
    pub const fn nodes(&self) -> u64 {
        self.nodes
    }

    /// Returns a copy of the PV of the current positon.
    #[allow(dead_code)]
    pub fn root_pv(&self) -> Pv {
        self.root_pv.clone()
    }

    /// Returns the time taken since the search started.
    pub fn elapsed_time(&self) -> Duration {
        self.start.elapsed()
    }

    /// Adds a zobrist key to the stack.
    pub fn push_key(&mut self, key: Key) {
        debug_assert!(
            self.past_keys.len() < self.past_keys.capacity(),
            "stack overflow"
        );
        // SAFETY: we just checked that we can push
        unsafe { self.past_keys.push_unchecked(key) };
    }

    /// Pops a zobrist key off the stack.
    pub fn pop_key(&mut self) -> Option<Key> {
        self.past_keys.pop()
    }

    /// Check the status of the search.
    ///
    /// This will check the UCI receiver to see if the GUI has told us to stop,
    /// then check to see if we're exceeding the limits of the search.
    fn check_status(&mut self) -> SearchStatus {
        // only check every 2048 nodes and don't bother wasting more time if
        // we've already stopped
        if self.nodes % 2048 != 0 || self.status != SearchStatus::Continue {
            return self.status;
        }

        #[allow(clippy::unwrap_used)]
        if let Ok(token) = self.state.uci_rx.lock().unwrap().try_recv() {
            let token = token.trim();
            if token == "stop" {
                self.status = SearchStatus::Stop;
                return self.status;
            }
            if token == "quit" {
                self.status = SearchStatus::Quit;
                return self.status;
            }
            if token == "isready" {
                println!("readyok");
            }
        }

        // these are the only variants that can cause a search to exit early
        #[allow(clippy::wildcard_enum_match_arm)]
        match self.limits {
            Limits::Nodes(n) => {
                if self.nodes >= n {
                    self.status = SearchStatus::Stop;
                }
            }
            Limits::Movetime(m) => {
                if self.start.elapsed() >= m {
                    self.status = SearchStatus::Stop;
                }
            }
            Limits::Timed { time, .. } => {
                // if we're about to pass our total amount of time (which
                // includes the move overhead), stop the search
                if self.start.elapsed() + Duration::from_millis(1) > time {
                    self.status = SearchStatus::Stop;
                }
            }
            _ => (),
        };

        self.status
    }

    /// Calculates if the iterative deepening loop should be exited.
    ///
    /// Assumes that this is being called at the end of the loop.
    fn should_stop(&mut self, depth: Depth) -> bool {
        if self.check_status() != SearchStatus::Continue {
            return true;
        }

        #[allow(clippy::wildcard_enum_match_arm)]
        match self.limits {
            Limits::Depth(d) => {
                if depth >= d {
                    self.status = SearchStatus::Stop;
                }
            }
            Limits::Timed { .. } => {
                // if we do not have a realistic chance of finishing the next
                // loop, assume we won't, and stop early.
                if self.start.elapsed() > self.allocated.mul_f32(0.4) {
                    self.status = SearchStatus::Stop;
                }
            }
            _ => (),
        }

        self.status != SearchStatus::Continue
    }

    /// Returns if the root node should print extra information.
    fn should_print(&self) -> bool {
        self.start.elapsed() > Duration::from_millis(3000) && self.can_print
    }

    /// Checks if the position is drawn, either because of repetition or
    /// because of the fifty-move rule.
    fn is_draw(&self, halfmoves: u8) -> bool {
        // 50mr
        if halfmoves >= 100 {
            return true;
        }

        // SAFETY: there is always at least 1 key
        let current_key = unsafe { self.past_keys.get_unchecked(self.past_keys.len() - 1) };

        // check if any past position's key is the same as the current key
        self.past_keys
            .iter()
            // most recent position is last
            .rev()
            // it is impossible to get a repetition within the past 4 halfmoves
            .skip(4)
            // stop after an irreversible position, or stop immediately for
            // halfmoves < 4
            .take(usize::from(halfmoves).saturating_sub(3))
            // skip positions with the wrong stm
            .step_by(2)
            .any(|key| key == current_key)
    }

    /// Prints information about a completed search iteration.
    fn print_report(&self, score: Eval, pv: &Pv, depth: Depth) {
        let time = self.start.elapsed();
        let nps = 1_000_000 * self.nodes / time.as_micros().max(1) as u64;

        #[allow(clippy::unwrap_used)]
        let score_str = if is_mate(score) {
            format!("score mate {}", moves_to_mate(score))
        } else {
            format!("score cp {score}")
        };

        println!(
            "info depth {} seldepth {} {score_str} hashfull {} nodes {} time {} nps {} pv {}",
            depth,
            self.seldepth,
            self.state.tt.estimate_hashfull(),
            self.nodes,
            time.as_millis(),
            nps,
            pv,
        );
    }
}
