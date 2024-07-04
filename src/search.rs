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
    sync::{mpsc::Receiver, Mutex},
    time::{Duration, Instant},
};

use crate::{
    engine::ZobristStack,
    evaluation::{is_mate, moves_to_mate, Eval},
    movegen::Move,
    transposition_table::TranspositionTable,
    util::{get_unchecked, insert_unchecked},
};

/// For running the main alpha-beta search within the iterative deepening loop.
mod alpha_beta_search;
/// For running the iterative deepening loop.
pub mod iterative_deepening;
/// For selecting which order moves are searched in.
mod movepick;
/// Time management.
pub mod time;

/// The difference between the root or leaf node (for height or depth
/// respectively) and the current node.
pub type Depth = u8;

/// A marker for a type of node to allow searches with generic node types.
#[allow(clippy::missing_docs_in_private_items)]
trait Node {
    const IS_PV: bool;
    const IS_ROOT: bool;
}

/// A node with a zero window: is expected not to be in the final PV.
struct NonPvNode;
/// A node that could be in the final PV.
struct PvNode;
/// The node from which the search starts.
struct RootNode;

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

/// The principle variation: the current best sequence of moves for both sides.
#[derive(Clone)]
pub struct Pv {
    /// A non-circular queue of moves.
    moves: [Move; Depth::MAX as usize],
    /// Index of the first [`Move`].
    ///
    /// This will be equal to `first_empty` if there are no [`Move`]s.
    first_item: u8,
    /// Index of the first empty space.
    first_empty: u8,
}

/// Various items needed throughout during the search.
pub struct SearchReferences<'a> {
    /// The moment the search started.
    start: Instant,
    /// The depth being searched.
    depth: Depth,
    /// The maximum depth reached.
    seldepth: Depth,
    /// How many positions have been searched.
    nodes: u64,
    /// The status of the search: continue, stop or quit?
    status: SearchStatus,
    /// The limits of the search.
    limits: Limits,
    /// How much time we're allocated.
    allocated: Duration,
    /// A receiver for the inputted UCI commands.
    uci_rx: &'a Mutex<Receiver<String>>,
    /// A stack of zobrist hashes of previous board states, beginning from the
    /// FEN string in the initial `position fen ...` command.
    ///
    /// The first (bottom) element is the initial board and the top element is
    /// the current board.
    past_zobrists: ZobristStack,
    /// The transposition table.
    tt: &'a TranspositionTable,
}

impl Default for Limits {
    fn default() -> Self {
        Self::Infinite
    }
}

impl Display for Pv {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut ret_str = String::with_capacity(self.len());
        for mv in self.moves() {
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

impl Pv {
    /// Returns a new 0-initialised [`Pv`].
    const fn new() -> Self {
        Self {
            moves: [Move::null(); Depth::MAX as usize],
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
        insert_unchecked(&mut self.moves, self.first_empty as usize, mv);
        self.first_empty += 1;
    }

    /// Removes a [`Move`] from the front of the queue.
    fn dequeue(&mut self) -> Option<Move> {
        (self.first_item < self.first_empty).then(|| {
            let mv = *get_unchecked(&self.moves, self.first_item as usize);
            self.first_item += 1;
            mv
        })
    }

    /// Clears all moves from the queue.
    fn clear(&mut self) {
        self.first_item = 0;
        self.first_empty = 0;
    }

    /// Returns a slice to the moves of the queue.
    fn moves(&self) -> &[Move] {
        &self.moves[(self.first_item as usize)..(self.first_empty as usize)]
    }

    /// Gets the [`Move`] at the given index.
    fn get(&self, index: usize) -> Move {
        *get_unchecked(&self.moves, index)
    }

    /// Calculates the length of the queue.
    fn len(&self) -> usize {
        usize::from(self.first_empty - self.first_item)
    }
}

impl<'a> SearchReferences<'a> {
    /// Creates a new [`SearchReferences`], which includes but is not limited to the
    /// given parameters.
    pub const fn new(
        start: Instant,
        limits: Limits,
        allocated: Duration,
        uci_rx: &'a Mutex<Receiver<String>>,
        past_zobrists: ZobristStack,
        tt: &'a TranspositionTable,
    ) -> Self {
        Self {
            start,
            depth: 0,
            seldepth: 0,
            nodes: 0,
            status: SearchStatus::Continue,
            limits,
            allocated,
            uci_rx,
            past_zobrists,
            tt,
        }
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
        if let Ok(token) = self.uci_rx.lock().unwrap().try_recv() {
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
    fn should_stop(&mut self) -> bool {
        if self.check_status() != SearchStatus::Continue || self.depth == Depth::MAX {
            return true;
        }

        #[allow(clippy::wildcard_enum_match_arm)]
        match self.limits {
            Limits::Depth(d) => {
                if self.depth >= d {
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
    fn should_print(&mut self) -> bool {
        self.start.elapsed() > Duration::from_millis(3000)
    }

    /// Checks if the position is drawn, either because of repetition or
    /// because of the fifty-move rule.
    fn is_draw(&self, halfmoves: u8) -> bool {
        // 50mr
        if halfmoves >= 100 {
            return true;
        }

        let current_key = self.past_zobrists.peek();

        // check if any past position's key is the same as the current key
        self.past_zobrists
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
}

/// Prints information about a completed search iteration.
fn print_report(search_refs: &SearchReferences<'_>, time: Duration, score: Eval, pv: &Pv) {
    let mut print_str = format!(
        "info depth {} seldepth {}",
        search_refs.depth, search_refs.seldepth
    );

    #[allow(clippy::unwrap_used)]
    if is_mate(score) {
        write!(&mut print_str, " score mate {}", moves_to_mate(score))
    } else {
        write!(&mut print_str, " score cp {score}")
    }
    .unwrap();

    let nps = 1_000_000 * search_refs.nodes / time.as_micros().max(1) as u64;

    println!(
        "{print_str} hashfull {} nodes {} time {} nps {} pv {}",
        search_refs.tt.estimate_hashfull(),
        search_refs.nodes,
        time.as_millis(),
        nps,
        pv,
    );
}
