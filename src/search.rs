use std::{
    fmt::{self, Display, Formatter},
    process::exit,
    rc::Rc,
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

use crate::{
    board::{Board, Key},
    engine::ZobristStack,
    index_into_unchecked, index_unchecked,
    movegen::Move,
    util::Stack,
};
use main_search::search;

/// For carrying out the search.
mod main_search;
/// Time management.
mod time;

/// The storage type for a given depth.
pub type Depth = u8;

/// A marker for a type of node to allow searches with generic node types.
// Idea is taken from viridithas.
#[allow(clippy::missing_docs_in_private_items)]
trait Node {
    const IS_PV: bool;
    const IS_ROOT: bool;
}

/// A node that is not root.
struct OtherNode;
/// The node from which the search starts.
struct RootNode;

impl Node for OtherNode {
    const IS_ROOT: bool = false;
    const IS_PV: bool = false;
}

impl Node for RootNode {
    const IS_ROOT: bool = true;
    const IS_PV: bool = true;
}

/// The type of a search and its limits.
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
struct Pv {
    /// A non-circular queue of moves.
    moves: [Move; Depth::MAX as usize],
    /// Index of the first [`Move`].
    ///
    /// This will be equal to `first_empty` if there are no [`Move`]s.
    first_item: u8,
    /// Index of the first empty space.
    first_empty: u8,
}

/// Information about a search.
pub struct SearchInfo {
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
    ///
    /// It may seem odd to make this an [`Rc`] instead of a regular reference,
    /// but I'll need to make it an [`Arc`](std::sync::Arc) when I add lazy SMP
    /// and I'd like the transition to be as painless as possible.
    uci_rx: Rc<Receiver<String>>,
    /// A stack of zobrist hashes of previous board states, beginning from the
    /// FEN string in the initial `position fen ...` command.
    ///
    /// The first (bottom) element is the initial board and the top element is
    /// the current board.
    past_zobrists: ZobristStack,
}

/// The parameters of a search.
pub struct SearchParams {
    /// The moment the search started. Set as early as possible.
    start: Instant,
    /// The limits of the search.
    limits: Limits,
    /// The overhead of sending a move.
    move_overhead: Duration,
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
            ret_str.push_str(&mv.to_string());
            ret_str.push(' ');
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
        index_into_unchecked!(self.moves, self.first_empty as usize, mv);
        self.first_empty += 1;
    }

    /// Removes a [`Move`] from the front of the queue.
    fn dequeue(&mut self) -> Option<Move> {
        (self.first_item < self.first_empty).then(|| {
            let mv = index_unchecked!(self.moves, self.first_item as usize);
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
        index_unchecked!(self.moves, index)
    }

    /// Calculates the length of the queue.
    fn len(&self) -> usize {
        usize::from(self.first_empty - self.first_item)
    }
}

impl SearchInfo {
    /// Creates a new [`SearchInfo`], which includes but is not limited to the
    /// given parameters.
    pub fn new(
        search_params: SearchParams,
        time_allocated: Duration,
        uci_rx: Rc<Receiver<String>>,
        past_zobrists: Stack<Key, { Depth::MAX as usize }>,
    ) -> Self {
        Self {
            start: search_params.start,
            depth: 0,
            seldepth: 0,
            nodes: 0,
            status: SearchStatus::Continue,
            limits: search_params.limits,
            allocated: time_allocated,
            uci_rx,
            past_zobrists,
        }
    }

    /// Check the status of the search.
    ///
    /// This will check the UCI receiver to see if the GUI has told us to stop,
    /// then check to see if we're exceeding the limits of the search.
    fn check_status(&mut self) -> SearchStatus {
        // only check every 2048 nodes, and don't bother wasting more time if
        // we've already stopped
        if self.nodes & 2047 != 0 || self.status != SearchStatus::Continue {
            return self.status;
        }

        if let Ok(token) = self.uci_rx.try_recv() {
            let token = token.trim();
            if token == "stop" {
                self.status = SearchStatus::Stop;
                return self.status;
            }
            if token == "quit" {
                self.status = SearchStatus::Quit;
                return self.status;
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
            // start at the most recent position
            .rev()
            // skip very recent positions
            .skip(4)
            // stop after an irreversible position, or stop immediately for
            // halfmoves < 4
            .take(usize::from(halfmoves).saturating_sub(3))
            // skip positions with the wrong stm
            .step_by(2)
            .any(|key| key == current_key)
    }
}

impl SearchParams {
    /// Creates a new [`SearchInfo`] with the initial information that searches
    /// start with.
    pub fn new(limits: Limits, move_overhead: Duration) -> Self {
        Self {
            start: Instant::now(),
            limits,
            move_overhead,
        }
    }
}

/// Performs iterative deepening on the given board.
pub fn iterative_deepening(
    search_params: SearchParams,
    board: &Board,
    uci_rx: Rc<Receiver<String>>,
    past_zobrists: Stack<Key, { Depth::MAX as usize }>,
) {
    let time_allocated = search_params.calculate_time_window();
    let mut search_info = SearchInfo::new(search_params, time_allocated, uci_rx, past_zobrists);
    let mut pv = Pv::new();
    let mut best_move;
    let mut depth = 1;

    'iter_deep: loop {
        search_info.depth = depth;
        search_info.seldepth = 0;
        search_info.status = SearchStatus::Continue;

        let eval = search::<RootNode>(&mut search_info, &mut pv, board, depth);

        // the root search guarantees that there will always be 1 valid move in
        // the PV
        best_move = pv.get(0);

        if search_info.check_status() != SearchStatus::Continue {
            break 'iter_deep;
        }

        let time = search_info.start.elapsed();
        let nps = 1_000_000 * search_info.nodes / time.as_micros() as u64;

        println!(
            "info depth {} seldepth {} score cp {} nodes {} time {} nps {} pv {}",
            depth,
            search_info.seldepth,
            eval,
            search_info.nodes,
            time.as_millis(),
            nps,
            pv,
        );

        if search_info.should_stop() {
            break 'iter_deep;
        }

        pv.clear();
        depth += 1;
    }

    println!("bestmove {best_move}");

    if search_info.check_status() == SearchStatus::Quit {
        exit(0);
    }
}
