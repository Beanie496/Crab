use std::time::Instant;

use super::Engine;
use crate::{movelist::Movelist, util::stringify_move};

impl Engine {
    /// Counts the number of leaf nodes `depth` moves in the future.
    ///
    /// If `IS_ROOT`, it also prints each move followed by the number of leaf
    /// nodes reached from that move, or just "1" if `depth == 0`, and
    /// prints total node count, time and NPS at the end.
    pub fn perft<const IS_ROOT: bool>(&mut self, depth: u8) -> u64 {
        if IS_ROOT {
            println!("Result:");
            if depth == 0 {
                println!("1");
                return 1;
            }
        }

        let time = Instant::now();
        let mut ml = Movelist::new();
        self.board.generate_moves(&mut ml);

        let mut total = 0;
        for mv in ml {
            let is_leaf = depth == 1;
            let moves = if IS_ROOT && is_leaf {
                1
            } else {
                let next_depth = depth - 1;
                let is_next_leaf = next_depth == 1;
                self.board.make_move(mv);
                let result = if is_next_leaf {
                    let mut next_ml = Movelist::new();
                    self.board.generate_moves(&mut next_ml);
                    next_ml.moves() as u64
                } else {
                    self.perft::<false>(next_depth)
                };
                self.board.unmake_move();
                result
            };
            total += moves;
            if IS_ROOT {
                println!("{}: {moves}", stringify_move(mv));
            }
        }

        if IS_ROOT {
            let elapsed_us = time.elapsed().as_micros() as u64;
            println!("Total: {total}");
            println!(
                "Time taken: {:.0} ms; NPS: {}",
                elapsed_us / 1_000,
                1_000_000 * total / elapsed_us
            );
        }
        total
    }
}
