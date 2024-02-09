use std::{
    fmt::{self, Display, Formatter},
    time::{Duration, Instant},
};

use super::Engine;
use crate::{board::Move, evaluation::Eval};
use alpha_beta::alpha_beta_search;

/// For carrying out the search.
mod alpha_beta;

/// The principle variation: the current best sequence of moves for both sides.
// non-circular queue, as all the moves are enqueued exactly once before all
// the moves are dequeued exactly once (then it goes out of scope)
// 512 bytes
#[allow(clippy::missing_docs_in_private_items)]
struct Pv {
    moves: [Move; MAX_PLY],
    first_item: u8,
    first_empty: u8,
}

/// Information about a search.
struct SearchInfo {
    /// The depth to be searched.
    pub depth: u8,
    /// How long the search has been going.
    pub time: Duration,
    /// How many positions have been searched.
    pub nodes: u64,
    /// The principle variation: the optimal sequence of moves for both sides.
    pub pv: Pv,
    // ignore this for now
    //_multipv: [[Move]],
    /// The score of the position from the perspective of the side to move.
    pub score: Eval,
    /// Which move is currently being searched at root.
    pub _currmove: Move,
    /// Which move number is currently being searched at root.
    pub _currmovenumber: u8,
    /// How full the transposition table is (in permill).
    pub _hashfull: u16,
    /// How many positions have been reached on average per second.
    pub nps: u64,
}

/// The highest possible (positive) evaluation.
const INF_EVAL: Eval = Eval::MAX;
/// The maximum depth this engine supports.
const MAX_PLY: usize = u8::MAX as usize;

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

impl Iterator for Pv {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.dequeue()
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

impl Engine {
    /// Start the search. Runs to infinity if `depth == None`,
    /// otherwise runs to depth `Some(depth)`.
    // this triggers because of `elapsed_us` and `elapsed_ms`, which are
    // obviously different
    #[allow(clippy::similar_names)]
    #[inline]
    pub fn search(&self, depth: Option<u8>) {
        let time = Instant::now();
        let depth = depth.unwrap_or(u8::MAX);
        let alpha = -INF_EVAL;
        let beta = INF_EVAL;
        let mut search_info = SearchInfo::new(depth);

        let result = alpha_beta_search(&mut search_info, &self.board.clone(), -beta, -alpha, depth);

        // the initial call to `alpha_beta_search()` counts the starting
        // position, so remove that count
        search_info.nodes -= 1;
        search_info.time = time.elapsed();
        search_info.score = result;
        search_info.nps = 1_000_000 * search_info.nodes / search_info.time.as_micros() as u64;

        println!("{search_info}");
    }
}

impl Pv {
    /// Returns a new 0-initialised [`Pv`].
    const fn new() -> Self {
        Self {
            // TODO: `MaybeUninit` might be faster?
            moves: [Move::null(); MAX_PLY],
            first_item: 0,
            first_empty: 0,
        }
    }

    /// Appends the [`Move`]s from `other_pv` to `self`.
    fn append_pv(&mut self, other_pv: &mut Self) {
        // NOTE: `collect_into()` would be a more ergonomic way to do this,
        // but that's currently nightly
        for mv in other_pv {
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
    const fn new(depth: u8) -> Self {
        Self {
            depth,
            time: Duration::ZERO,
            nodes: 0,
            pv: Pv::new(),
            score: 0,
            _currmove: Move::null(),
            _currmovenumber: 1,
            _hashfull: 0,
            nps: 0,
        }
    }
}
