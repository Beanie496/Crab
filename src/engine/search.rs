use std::time::Instant;

use super::Engine;
use crate::board::{Board, Move, Moves};

/// The parameters for a search.
struct SearchParameters {
    /// The number of positions visited after the search starts.
    nodes: u64,
}

/// Infinity, for an `i16` at least.
const INF: i16 = i16::MAX;

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
        let mut best_move = Move::null();
        let mut max = -INF;
        let mut search_parameters = SearchParameters::default();

        let mut moves = Moves::new();
        self.board.generate_moves(&mut moves);

        for mv in moves {
            let mut copy = self.board.clone();
            if !copy.make_move(mv) {
                break;
            }

            let result = -self.negamax(&mut search_parameters, &copy, depth - 1);
            if result > max {
                max = result;
                best_move = mv;
            }
        }

        let elapsed_us = time.elapsed().as_micros() as u64;
        let nodes = search_parameters.nodes;
        let elapsed_ms = elapsed_us / 1_000;
        let nps = 1_000_000 * nodes / elapsed_us;

        println!(
            "info depth {depth} score {max} nodes {nodes} nps {nps} time {elapsed_ms} pv {}",
            best_move.stringify()
        );
    }

    /// Performs negamax on `board`. Returns the evaluation of after searching
    /// to the given depth.
    fn negamax(&self, search_parameters: &mut SearchParameters, board: &Board, depth: u8) -> i16 {
        search_parameters.nodes += 1;
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

            let result = -self.negamax(search_parameters, &copy, depth - 1);
            if result > max {
                max = result;
            }
        }

        max
    }
}

impl Default for SearchParameters {
    #[inline]
    fn default() -> Self {
        Self { nodes: 0 }
    }
}
