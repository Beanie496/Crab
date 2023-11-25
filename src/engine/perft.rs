use std::time::Instant;

use super::Engine;
use crate::{movelist::Movelist, util::stringify_move};

impl Engine {
    /// Runs perft on the current position. Prints each move followed by the
    /// number of leaf nodes reaches from that move, or just prints "1" if
    /// `depth == 0`. Prints total node count, time and NPS at the end.
    pub fn perft(&mut self, depth: u8) {
        println!("Result:");
        if depth == 0 {
            println!("1");
            return;
        }

        let time = Instant::now();
        let mut ml = Movelist::new();
        self.board.generate_moves(&mut ml);

        let mut total = 0;
        for mv in ml {
            self.board.make_move(mv);
            if depth == 1 {
                println!("{}: 1", stringify_move(mv));
            } else {
                let moves = self.perft_inner(depth - 1);
                total += moves;
                println!("{}: {moves}", stringify_move(mv));
            }
            self.board.unmake_move();
        }
        println!("Total: {total}");
        let elapsed_us = time.elapsed().as_micros() as u64;
        println!(
            "Time taken: {:.0} ms; NPS: {}",
            elapsed_us / 1_000,
            1_000_000 * total / elapsed_us
        );
    }
}

impl Engine {
    /// Runs perft on the current position and returns the number of legal
    /// moves.
    fn perft_inner(&mut self, depth: u8) -> u64 {
        let mut ml = Movelist::new();
        self.board.generate_moves(&mut ml);

        if depth == 1 {
            return ml.moves() as u64;
        }

        let mut total = 0;
        for mv in ml {
            self.board.make_move(mv);
            total += self.perft_inner(depth - 1);
            self.board.unmake_move();
        }
        total
    }
}
