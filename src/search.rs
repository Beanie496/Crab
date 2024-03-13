use std::{
    fmt::{self, Display, Formatter},
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

use crate::{
    board::Board,
    evaluation::{Eval, INF_EVAL},
    movegen::Move,
};
use alpha_beta::alpha_beta_search;

/// For carrying out the search.
mod alpha_beta;
/// Move ordering.
mod ordering;
/// Time management.
mod time;

/// The storage type for a given depth.
pub type Depth = u8;

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

/// The principle variation: the current best sequence of moves for both sides.
// 512 bytes
#[derive(Clone, Copy)]
struct Pv {
    /// A non-circular queue of moves.
    ///
    /// All the moves are enqueued exactly once before all the moves are
    /// dequeued exactly once (then it goes out of scope)
    moves: [Move; Depth::MAX as usize],
    /// Index of the first [`Move`]. This will be equal to `first_empty` if
    /// there are no [`Move`]s.
    first_item: u8,
    /// Index of the first empty space.
    first_empty: u8,
}

/// An iterator over a [`Pv`].
#[allow(clippy::missing_docs_in_private_items)]
struct PvIter {
    pv: Pv,
}

/// Information about a search.
pub struct SearchInfo {
    /// The moment our search started. Called as early as possible.
    time_start: Instant,
    /// The depth currently being searched.
    depth: Depth,
    /// How long the search has been going.
    time: Duration,
    /// How many positions have been searched.
    nodes: u64,
    /// The principle variation: the optimal sequence of moves for both sides.
    pv: Pv,
    // ignore this for now
    //_multipv: [[Move]],
    /// The score of the position from the perspective of the side to move.
    score: Eval,
    /// Which move is currently being searched at root.
    _currmove: Move,
    /// Which move number is currently being searched at root.
    _currmovenumber: u8,
    /// How full the transposition table is (in permill).
    _hashfull: u16,
    /// How many positions have been reached on average per second.
    nps: u64,
    /// A channel to receive the 'stop' command from.
    control_rx: Receiver<Stop>,
    /// Whether or not the search has received the 'stop' command.
    has_stopped: bool,
    /// The previous PV.
    history: Pv,
    /// The limits of our search.
    limits: Limits,
}

/// Used to tell the search thread to stop.
pub struct Stop;

impl Default for Limits {
    fn default() -> Self {
        Self::Infinite
    }
}

impl Display for Pv {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut ret_str = String::with_capacity(self.len());
        for mv in 0..self.len() {
            ret_str.push_str(&self.get(mv).to_string());
            ret_str.push(' ');
        }
        ret_str.pop();
        write!(f, "{ret_str}")
    }
}

impl IntoIterator for Pv {
    type Item = Move;
    type IntoIter = PvIter;

    fn into_iter(self) -> Self::IntoIter {
        PvIter::new(self)
    }
}

impl Iterator for PvIter {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.pv.dequeue()
    }
}

impl Display for SearchInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "info depth {} time {} nodes {} pv {} score cp {} nps {}",
            self.depth,
            self.time.as_millis(),
            self.nodes,
            self.pv,
            self.score,
            self.nps,
        )
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

impl PvIter {
    /// Creates a new [`PvIter`].
    // this function will be inlined anyway
    #[allow(clippy::large_types_passed_by_value)]
    const fn new(pv: Pv) -> Self {
        Self { pv }
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

    /// Appends the [`Move`]s from `other_pv` to `self`.
    fn append_pv(&mut self, other_pv: &Self) {
        // NOTE: `collect_into()` would be a more ergonomic way to do this,
        // but that's currently nightly
        for mv in other_pv.into_iter() {
            self.enqueue(mv);
        }
    }

    /// Adds a [`Move`] to the back of `self`.
    fn enqueue(&mut self, mv: Move) {
        self.moves[self.first_empty as usize] = mv;
        self.first_empty += 1;
    }

    /// Removes a [`Move`] from the front of `self`.
    fn dequeue(&mut self) -> Option<Move> {
        (self.first_item < self.first_empty).then(|| {
            let mv = self.moves[self.first_item as usize];
            self.first_item += 1;
            mv
        })
    }

    /// Clears all moves from `self`.
    ///
    /// This only sets a couple of variables, so it's basically free.
    fn clear(&mut self) {
        self.first_item = 0;
        self.first_empty = 0;
    }

    /// Gets the [`Move`] at the given index.
    ///
    /// Useful for read-only iteration.
    const fn get(&self, index: usize) -> Move {
        self.moves[index]
    }

    /// Calculates the length of `self`.
    fn len(&self) -> usize {
        usize::from(self.first_empty - self.first_item)
    }
}

impl SearchInfo {
    /// Creates a new [`SearchInfo`] with the initial information that searches
    /// start with.
    pub fn new(control_rx: Receiver<Stop>, limits: Limits) -> Self {
        Self {
            time_start: Instant::now(),
            depth: 0,
            time: Duration::ZERO,
            nodes: 0,
            pv: Pv::new(),
            score: 0,
            _currmove: Move::null(),
            _currmovenumber: 1,
            _hashfull: 0,
            nps: 0,
            control_rx,
            has_stopped: false,
            history: Pv::new(),
            limits,
        }
    }

    /// Checks if the search was stopped or if it finished on its own.
    const fn has_stopped(&self) -> bool {
        self.has_stopped
    }

    /// If the search needs to stop.
    fn should_stop(&mut self) -> bool {
        // only check every 2048 nodes
        if self.nodes & 2047 != 0 {
            return self.has_stopped;
        }

        if self.control_rx.try_recv().is_ok() {
            self.has_stopped = true;
            return true;
        }

        // these are the only variants that can cause a search to exit early
        #[allow(clippy::wildcard_enum_match_arm)]
        match self.limits {
            Limits::Nodes(n) => {
                if self.nodes >= n {
                    self.has_stopped = true;
                }
            }
            Limits::Movetime(m) => {
                if self.time_start.elapsed() >= m {
                    self.has_stopped = true;
                }
            }
            Limits::Timed { time, .. } => {
                // if we're approaching our total time (assuming a min overhead
                // of 100 us), stop the search
                if self.time_start.elapsed() + Duration::from_micros(100) > time {
                    self.has_stopped = true;
                }
            }
            _ => (),
        };

        self.has_stopped
    }

    /// Calculates if the iterative deepening loop should be exited.
    ///
    /// Assumes that this is being called at the end of the loop.
    pub fn should_exit_loop(&mut self, duration: Duration) -> bool {
        if self.should_stop() || self.depth == Depth::MAX {
            return true;
        }

        // if any other cases would have resulted in `true`, `has_stopped`
        // would have been true already
        #[allow(clippy::wildcard_enum_match_arm)]
        match self.limits {
            Limits::Depth(d) => {
                if self.depth >= d {
                    self.has_stopped = true;
                }
            }
            Limits::Timed { .. } => {
                // if we do not have a realistic chance of finishing the next
                // loop, assume we won't, and stop early. (Assuming a branching
                // factor of >10.)
                self.has_stopped = self.time_start.elapsed() > duration.mul_f32(0.1);
            }
            _ => (),
        }

        self.has_stopped
    }
}

/// Performs iterative deepening on the given board.
pub fn iterative_deepening(board: &Board, mut search_info: SearchInfo) {
    let alpha = -INF_EVAL;
    let beta = INF_EVAL;
    let mut best_move = Move::null();
    let time_allocated = search_info.calculate_time_window();

    'iter_deep: loop {
        search_info.pv.clear();
        search_info.depth += 1;
        let depth = search_info.depth;

        let eval = alpha_beta_search(&board.clone(), &mut search_info, alpha, beta, depth);

        if search_info.has_stopped() {
            // technically `best_move` would be null if this is called before
            // even depth 1 finishes, but there isn't much we can do about that
            // currently
            break 'iter_deep;
        }

        best_move = search_info.pv.get(0);
        search_info.score = eval;
        search_info.time = search_info.time_start.elapsed();
        search_info.nps = 1_000_000 * search_info.nodes / search_info.time.as_micros() as u64;
        search_info.history = search_info.pv;

        println!("{search_info}");

        if search_info.should_exit_loop(time_allocated) {
            break 'iter_deep;
        }
    }
    println!("bestmove {best_move}");
}
