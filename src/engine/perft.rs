use std::time::Instant;

use super::Engine;
use crate::board::{Board, Moves};

impl Engine {
    /// Outputs and returns the number of leaf nodes `depth` moves in the
    /// future.
    ///
    /// If `IS_TIMED`, it will also output the time taken and the average NPS.
    #[inline]
    pub fn perft<const IS_TIMED: bool>(&mut self, depth: u8) -> u64 {
        if IS_TIMED {
            let time = Instant::now();
            let result = self.perft::<false>(depth);
            let elapsed_us = time.elapsed().as_micros() as u64;
            println!(
                "Time taken: {} ms; NPS: {}",
                elapsed_us / 1_000,
                1_000_000 * result / elapsed_us
            );
            return result;
        }

        println!("Result:");
        if depth == 0 {
            println!("1");
            return 1;
        }

        let mut moves = Moves::new();
        self.board.generate_moves(&mut moves);

        let mut total = 0;
        for mv in moves {
            let moves = {
                let mut copy = self.board.clone();
                if !copy.make_move(mv) {
                    continue;
                }
                Self::perft_inner(depth - 1, &copy)
            };
            total += moves;
            println!("{}: {moves}", mv.stringify());
        }
        println!("Total: {total}");
        total
    }
}

impl Engine {
    /// Counts the number of leaf nodes `depth` moves in the future. It is used
    /// because copy-make requires an additional parameter, but I don't want to
    /// have that parameter in the API.
    fn perft_inner(depth: u8, board: &Board) -> u64 {
        if depth == 0 {
            return 1;
        }

        let mut moves = Moves::new();
        board.generate_moves(&mut moves);

        let mut total = 0;
        for mv in moves {
            total += {
                let mut copy = board.clone();
                if !copy.make_move(mv) {
                    continue;
                }
                Self::perft_inner(depth - 1, &copy)
            };
        }
        total
    }
}
