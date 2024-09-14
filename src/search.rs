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
    ops::{Deref, DerefMut},
    slice::Iter,
    sync::{mpsc::Receiver, Mutex},
    time::{Duration, Instant},
};

use arrayvec::ArrayVec;

use crate::{
    board::{Board, Key},
    defs::{Piece, PieceType, Side, Square},
    evaluation::{CompressedEvaluation, Evaluation},
    movegen::{Move, Moves},
    transposition_table::TranspositionTable,
};
pub use depth::{CompressedDepth, Depth, Height};
use time::calculate_time_window;

/// For running the main alpha-beta search.
pub mod alpha_beta_search;
/// For running the aspiration loop.
pub mod aspiration;
/// Items related to [`Depth`] and [`Height`], separated for neatness.
mod depth;
/// For running the iterative deepening loop.
pub mod iterative_deepening;
/// For selecting which order moves are searched in.
mod movepick;
/// Time management.
pub mod time;

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
enum SearchStatus {
    /// Do nothing: continue the search as normal.
    Continue,
    /// Stop the search.
    Stop,
    /// Stop the search and then exit the process.
    Quit,
}

/// The history of a board, excluding the current state of the board.
///
/// Each item corresponds to a previous state of the board. Each item is the
/// previous item with a single move applied to it. Applying a move to the most
/// recent item would get the current board state.
#[allow(clippy::missing_docs_in_private_items)]
pub struct BoardHistory {
    history: ArrayVec<HistoryItem, { Depth::MAX.to_index() }>,
}

/// Information needed for indexing into counter moves.
#[derive(Clone, Copy)]
pub struct CounterMoveInfo {
    /// The piece being moved.
    piece: Piece,
    /// The destination square of that piece.
    dest: Square,
}

/// An item of the board history.
#[derive(Clone, Copy)]
pub struct HistoryItem {
    /// The key of the item.
    key: Key,
    /// Information for counter moves.
    ///
    /// It's an [`Option`] because of null moves.
    counter_move_info: Option<CounterMoveInfo>,
}

/// A struct containing various histories relating to the board.
pub struct Histories {
    /// A history of bonuses for previous quiets.
    ///
    /// Indexed by side to move, start square then end square.
    ///
    /// So called because the wasted space looks a little like a butterfly's
    /// wings.
    butterfly_history: Box<[[[CompressedEvaluation; Square::TOTAL]; Square::TOTAL]; Side::TOTAL]>,
    /// A history of bonuses for previous captures.
    ///
    /// Indexed by the piece being moved, the piece being captured and the
    /// destination square.
    capture_history: Box<[[[CompressedEvaluation; Square::TOTAL]; PieceType::TOTAL]; Piece::TOTAL]>,
    /// Killer moves.
    ///
    /// For each depth, the best move from the previous search at the same
    /// depth that originated from the same node.
    killers: [[Option<Move>; 2]; Depth::MAX.to_index() + 1],
    /// Counter moves.
    ///
    /// The previous best response to a certain piece landing on a certain
    /// square.
    counter_moves: [[Option<Move>; Square::TOTAL]; Piece::TOTAL],
    /// A stack of keys of previous board states, beginning from the initial
    /// `position fen ...` command.
    ///
    /// The first (bottom) element is the initial board and the top element is
    /// the current board.
    board_history: BoardHistory,
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
    /// How many positions have been searched.
    nodes: u64,
    /// The final PV from the initial position.
    root_pv: Pv,
    /// The status of the search: continue, stop or quit?
    status: SearchStatus,
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
}

impl Histories {
    /// The maximum value a butterfly history entry can have.
    const MAX_BUTTERFLY_HISTORY: Evaluation = Evaluation(i16::MAX as i32 / 2);
    /// The maximum value a capture history entry can have.
    const MAX_CAPTURE_HISTORY: Evaluation = Evaluation(8192);
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

impl Deref for BoardHistory {
    type Target = ArrayVec<HistoryItem, { Depth::MAX.to_index() }>;

    fn deref(&self) -> &Self::Target {
        &self.history
    }
}

impl DerefMut for BoardHistory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.history
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

impl BoardHistory {
    /// Creates a new, empty [`BoardHistory`].
    pub fn new() -> Self {
        Self {
            history: ArrayVec::new(),
        }
    }

    /// Sets the items of `self` to `other`.
    pub fn set_to(&mut self, other: &Self) {
        self.clear();

        for &item in other.iter() {
            // SAFETY: `other.len() <= self.capacity()`
            unsafe {
                self.push_unchecked(item);
            }
        }
    }
}

impl CounterMoveInfo {
    /// Creates new [`CounterMoveInfo`].
    pub const fn new(piece: Piece, dest: Square) -> Self {
        Self { piece, dest }
    }
}

impl HistoryItem {
    /// Creates a new [`HistoryItem`] with the given fields.
    pub const fn new(key: Key, counter_move_info: Option<CounterMoveInfo>) -> Self {
        Self {
            key,
            counter_move_info,
        }
    }
}

impl Histories {
    /// Creates new, empty [`Histories`].
    fn new() -> Self {
        Self {
            butterfly_history: Box::new(
                [[[CompressedEvaluation(0); Square::TOTAL]; Square::TOTAL]; Side::TOTAL],
            ),
            capture_history: Box::new(
                [[[CompressedEvaluation(0); Square::TOTAL]; PieceType::TOTAL]; Piece::TOTAL],
            ),
            killers: [[None; 2]; Depth::MAX.to_index() + 1],
            counter_moves: [[None; Square::TOTAL]; Piece::TOTAL],
            board_history: BoardHistory::new(),
        }
    }

    /// The bonus of a good move.
    fn bonus(depth: Depth) -> Evaluation {
        Evaluation::from(CompressedEvaluation(depth.0.min(8) * 100))
    }

    /// Updates a particular item of a history table.
    ///
    /// `is_bonus` is if the update should be a bonus (as opposed to a malus).
    fn update_history_value(
        value: &mut CompressedEvaluation,
        depth: Depth,
        is_bonus: bool,
        max_history: Evaluation,
    ) {
        let abs_bonus = Self::bonus(depth);
        let signed_bonus = if is_bonus { abs_bonus } else { -abs_bonus };
        // the value cannot exceed max_history, so the bonus is lerped between
        // its original value (for val == 0) and 0 (for val == max_history)
        let delta = signed_bonus - abs_bonus * Evaluation::from(*value) / max_history;
        *value += CompressedEvaluation::from(delta);
    }

    /// Returns the type of the captured piece on `board` from `mv`.
    pub fn captured_piece_type(board: &Board, mv: Move, end: Square) -> PieceType {
        let captured_type = PieceType::from(board.piece_on(end));

        if captured_type == PieceType::NONE {
            debug_assert!(
                mv.is_en_passant() || mv.is_promotion(),
                "{mv} on {board} is being scored as a capture"
            );
            // if a promotion isn't capturing anything, we can just use the
            // pawn index instead, as it's impossible to capture a pawn on the
            // final rank anyway
            PieceType::PAWN
        } else {
            captured_type
        }
    }

    /// Clears all the histories apart from the board history.
    fn clear(&mut self) {
        self.butterfly_history =
            Box::new([[[CompressedEvaluation(0); Square::TOTAL]; Square::TOTAL]; Side::TOTAL]);
        self.capture_history =
            Box::new([[[CompressedEvaluation(0); Square::TOTAL]; PieceType::TOTAL]; Piece::TOTAL]);
        self.counter_moves = [[None; Square::TOTAL]; Piece::TOTAL];
        self.killers[0] = [None; 2];
    }

    /// Updates the butterfly history with a bonus for `best_move` and a
    /// penalty for all other moves in `quiets`.
    ///
    /// `quiets` may or may not contain `best_move`.
    fn update_butterfly_history(
        &mut self,
        quiets: &Moves,
        best_move: Move,
        side: Side,
        depth: Depth,
    ) {
        let side = side.to_index();

        for mv in quiets.iter().map(|scored_move| scored_move.mv) {
            let start = mv.start().to_index();
            let end = mv.end().to_index();

            Self::update_history_value(
                &mut self.butterfly_history[side][start][end],
                depth,
                best_move == mv,
                Self::MAX_BUTTERFLY_HISTORY,
            );
        }
    }

    /// Returns the butterfly score of a move by the given side from `start` to
    /// `end`.
    pub fn get_butterfly_score(
        &self,
        side: Side,
        start: Square,
        end: Square,
    ) -> CompressedEvaluation {
        self.butterfly_history[side.to_index()][start.to_index()][end.to_index()]
    }

    /// Updates the capture history with a bonus for `best_move` and a penalty
    /// for all other moves in `captures`.
    ///
    /// `captures` may or may not contain `best_move`.
    fn update_capture_history(
        &mut self,
        board: &Board,
        captures: &Moves,
        best_move: Move,
        depth: Depth,
    ) {
        for mv in captures.iter().map(|scored_move| scored_move.mv) {
            let end = mv.end();
            let piece = board.piece_on(mv.start());
            let captured_type = Self::captured_piece_type(board, mv, end);

            Self::update_history_value(
                &mut self.capture_history[piece.to_index()][captured_type.to_index()]
                    [end.to_index()],
                depth,
                best_move == mv,
                Self::MAX_CAPTURE_HISTORY,
            );
        }
    }

    /// Returns the capture score of `piece` moving to `end` and capturing
    /// `captured_type`.
    pub fn get_capture_score(
        &self,
        piece: Piece,
        captured_type: PieceType,
        end: Square,
    ) -> CompressedEvaluation {
        self.capture_history[piece.to_index()][captured_type.to_index()][end.to_index()]
    }

    /// Replace the second killer of the current height with the given move.
    fn insert_into_killers(&mut self, height: Height, mv: Move) {
        let height = height.to_index();
        if self.killers[height][0] == Some(mv) {
            return;
        }
        self.killers[height][1] = self.killers[height][0];
        self.killers[height][0] = Some(mv);
    }

    /// Return the killers of the current height.
    const fn current_killers(&self, height: Height) -> [Option<Move>; 2] {
        self.killers[height.to_index()]
    }

    /// Clear the killers of the next height.
    fn clear_next_killers(&mut self, height: Height) {
        self.killers[height.to_index() + 1] = [None; 2];
    }

    /// Inserts `mv` into the table as given by `history_item`.
    fn insert_into_counter_moves(&mut self, history_item: HistoryItem, mv: Move) {
        if let Some(counter_move_info) = history_item.counter_move_info {
            let piece = counter_move_info.piece.to_index();
            let square = counter_move_info.dest.to_index();

            self.counter_moves[piece][square] = Some(mv);
        }
    }

    /// Gets the counter move as indexed by `history_item`.
    fn get_counter_move(&self, history_item: HistoryItem) -> Option<Move> {
        history_item.counter_move_info.and_then(|info| {
            let piece = info.piece.to_index();
            let square = info.dest.to_index();
            self.counter_moves[piece][square]
        })
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
            seldepth: Height::default(),
            nodes: 0,
            root_pv: Pv::new(),
            status: SearchStatus::Continue,
            nmp_rights: NmpRights::new(),
            histories: Histories::new(),
            can_print: true,
            limits: Limits::default(),
            allocated: Duration::MAX,
            move_overhead: Duration::ZERO,
            board: Board::new(),
            state,
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
    pub fn start_search(&mut self) {
        self.start = Instant::now();
        self.seldepth = Height::default();
        self.nodes = 0;
        self.status = SearchStatus::Continue;
        self.nmp_rights = NmpRights::new();
        self.histories.clear();
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

    /// Makes `mv` on `board` and returns whether or not the move was legal.
    pub fn make_move(&mut self, board: &mut Board, mv: Move) -> bool {
        let old_key = board.key();

        if !board.make_move(mv) {
            return false;
        }

        let dest = mv.end();
        let piece = board.piece_on(dest);
        let counter_move_info = CounterMoveInfo::new(piece, dest);
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
                if depth >= Depth::from(d) {
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
        let nps = 1_000_000 * self.nodes / time.as_micros().max(1) as u64;

        println!(
            "info depth {depth} seldepth {} {score} hashfull {} nodes {} time {} nps {nps} pv {pv}",
            self.seldepth.0,
            self.state.tt.estimate_hashfull(),
            self.nodes,
            time.as_millis(),
        );
    }
}
