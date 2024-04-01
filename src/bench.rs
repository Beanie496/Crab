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

#![cfg(test)]

use std::{
    sync::{mpsc, Arc, Mutex},
    thread::{available_parallelism, spawn},
};

use crate::perft::perft;

struct TestPosition {
    position: String,
    perft_depth: u8,
    perft_result: u64,
}

static TEST_POSITIONS: &str = include_str!("../test_positions.epd");

impl TestPosition {
    const fn new(position: String, perft_depth: u8, perft_result: u64) -> Self {
        Self {
            position,
            perft_depth,
            perft_result,
        }
    }

    fn run_test(&self) {
        let board = self.position.parse().unwrap();
        assert_eq!(
            perft::<false, false>(&board, self.perft_depth),
            self.perft_result,
            "incorrect result for position {}",
            self.position,
        );
    }
}

#[test]
fn test_positions() {
    let (tx, rx) = mpsc::channel();
    let rx = Arc::new(Mutex::new(rx));
    let mut handles = Vec::new();

    // add all test positions to the queue
    for position in TEST_POSITIONS.lines() {
        let mut tokens = position.split_whitespace();
        let mut fen_str = String::new();

        // have to do this before `take()` because `take()` takes ownership of
        // `tokens`
        let result = tokens
            .next_back()
            .and_then(|result| result.parse::<u64>().ok())
            .unwrap();

        for token in tokens.take(6) {
            fen_str.push_str(token);
            fen_str.push(' ');
        }

        // each position is just to depth 4
        let depth = 4;

        let test_pos = TestPosition::new(fen_str, depth, result);
        tx.send(test_pos).unwrap();
    }

    for _ in 0..available_parallelism().map_or(1, |p| p.get()) {
        let rx = Arc::clone(&rx);
        // Spawn a thread that dequeues and runs the test positions from the
        // receiver until there are no positions left
        handles.push(spawn(move || loop {
            let test_pos = rx.lock().unwrap().try_recv();
            if let Ok(test_pos) = test_pos {
                test_pos.run_test()
            } else {
                return;
            }
        }));
    }

    for handle in handles {
        handle.join().expect("A position has failed!");
    }
}
