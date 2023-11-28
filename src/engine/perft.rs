use std::time::Instant;

use super::Engine;
use crate::board::movegen::Moves;

impl Engine {
    /// Counts the number of leaf nodes `depth` moves in the future.
    ///
    /// If `IS_ROOT`, it also prints each move followed by the number of leaf
    /// nodes reached from that move, or just "1" if `depth == 0`, and
    /// prints total node count, time and NPS at the end.
    pub fn perft<const PRINT_MOVES: bool, const IS_TIMED: bool>(&mut self, depth: u8) -> u64 {
        if IS_TIMED {
            let time = Instant::now();
            let result = self.perft::<PRINT_MOVES, false>(depth);
            let elapsed_us = time.elapsed().as_micros() as u64;
            println!(
                "Time taken: {} ms; NPS: {}",
                elapsed_us / 1_000,
                1_000_000 * result / elapsed_us
            );
            return result;
        }

        if PRINT_MOVES {
            println!("Result:");
            if depth == 0 {
                println!("1");
                return 1;
            }
        }

        let mut moves = Moves::new();
        self.board.generate_moves(&mut moves);

        let mut total = 0;
        for mv in moves {
            let is_leaf = depth == 1;
            let moves = if is_leaf {
                1
            } else {
                let next_depth = depth - 1;
                let is_next_leaf = next_depth == 1;
                self.board.make_move(mv);
                let result = if is_next_leaf {
                    let mut next_moves = Moves::new();
                    self.board.generate_moves(&mut next_moves);
                    next_moves.moves() as u64
                } else {
                    self.perft::<false, false>(next_depth)
                };
                self.board.unmake_move();
                result
            };
            total += moves;
            if PRINT_MOVES {
                println!("{}: {moves}", mv.stringify());
            }
        }
        if PRINT_MOVES {
            println!("Total: {total}");
        }
        total
    }
}
