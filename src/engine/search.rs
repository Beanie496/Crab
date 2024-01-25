use std::{
    fmt::{self, Display, Formatter},
    time::{Duration, Instant},
};

use super::Engine;
use crate::board::{Board, Move, Moves};

/// Information about a search.
struct SearchInfo {
    /// The depth to be searched.
    pub depth: u8,
    /// The maximum depth reached during quiessence (not implemented).
    pub seldepth: u8,
    /// How long the search has been going.
    pub time: Duration,
    /// How many positions have been searched.
    pub nodes: u64,
    // not sure how to make these two work yet - just the first move for now?
    /// The principle variation: the optimal sequence of moves for both sides.
    pub pv: Move,
    //_multipv: [[Move]],
    /// The score of the position from the perspective of the side to move.
    pub score: i16,
    /// Which move is currently being searched at root.
    pub _currmove: Move,
    /// Which move number is currently being searched at root.
    pub _currmovenumber: u8,
    /// How full the transposition table is (in permill).
    pub _hashfull: u16,
    /// How many positions have been reached on average per second.
    pub nps: u64,
}

/// Infinity, for an `i16` at least.
const INF: i16 = i16::MAX;

impl Display for SearchInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "info depth {} seldepth {} time {} nodes {} pv {} score cp {} nps {}",
            self.depth,
            self.seldepth,
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
        let mut max = -INF;
        let mut search_info = SearchInfo::new(depth);

        let mut moves = Moves::new();
        self.board.generate_moves(&mut moves);

        for mv in moves {
            let mut copy = self.board.clone();
            if !copy.make_move(mv) {
                break;
            }

            let result = -self.negamax(&mut search_info, &copy, depth - 1);
            if result > max {
                max = result;
                search_info.pv = mv;
            }
        }

        search_info.seldepth = depth;
        search_info.time = time.elapsed();
        search_info.score = max;
        search_info.nps = 1_000_000 * search_info.nodes / search_info.time.as_micros() as u64;

        println!("{search_info}");
    }

    /// Performs negamax on `board`. Returns the evaluation of after searching
    /// to the given depth.
    fn negamax(&self, search_info: &mut SearchInfo, board: &Board, depth: u8) -> i16 {
        search_info.nodes += 1;
        if depth == 0 {
            return self.evaluate_board();
        }

        let mut max = -INF;
        let mut moves = Moves::new();
        board.generate_moves(&mut moves);

        for mv in moves {
            let mut copy = board.clone();
            if !copy.make_move(mv) {
                break;
            }

            let result = -self.negamax(search_info, &copy, depth - 1);
            if result > max {
                max = result;
            }
        }

        max
    }
}

impl SearchInfo {
    /// Creates a new [`SearchInfo`] with the initial information that searches
    /// start with.
    const fn new(depth: u8) -> Self {
        Self {
            depth,
            seldepth: 0,
            time: Duration::ZERO,
            nodes: 0,
            pv: Move::null(),
            score: 0,
            _currmove: Move::null(),
            _currmovenumber: 1,
            _hashfull: 0,
            nps: 0,
        }
    }
}
