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

use std::{
    sync::{mpsc::channel, Mutex},
    time::Duration,
};

use crate::{
    board::Board,
    search::{Limits, SharedState, Worker, ZobristStack},
    transposition_table::TranspositionTable,
};

/// Test positions for benchmarks.
static BENCH_POSITIONS: &str = include_str!("../bench_positions.epd");
/// The default limit of each benched position.
pub const LIMIT: u64 = 8;
/// The default limit type.
pub const LIMIT_TYPE: &str = "depth";
/// The default hash size of each benched position.
pub const TT_SIZE: usize = 32;

/// Runs a benchmark on all the positions in [`BENCH_POSITIONS`].
pub fn bench<'a, T>(mut options: T)
where
    T: Iterator<Item = &'a str>,
{
    let tt_size = options
        .next()
        .and_then(|t| t.parse::<usize>().ok())
        .unwrap_or(TT_SIZE);
    let limit = options
        .next()
        .and_then(|l| l.parse::<u64>().ok())
        .unwrap_or(LIMIT);
    let limit_type = options.next().unwrap_or(LIMIT_TYPE);

    let mut limits = Limits::default();
    match limit_type {
        "depth" => {
            if let Ok(limit) = u8::try_from(limit) {
                limits.set_depth(Some(limit));
            } else {
                return;
            }
        }
        "nodes" => limits.set_nodes(Some(limit)),
        "movetime" => limits.set_movetime(Some(Duration::from_millis(limit))),
        _ => return,
    }
    let rx = Mutex::new(channel().1);
    let tt = TranspositionTable::with_capacity(tt_size);
    let mut state = SharedState::new(rx, tt);

    let mut total_time = Duration::ZERO;
    let mut total_nodes = 0;

    for position in BENCH_POSITIONS.lines() {
        println!("Position: {position}");

        let board = position.parse::<Board>().expect("Malformed test position");

        let mut worker = Worker::new(&state)
            .with_board(ZobristStack::new(), &board)
            .with_limits(limits);
        worker.start_search();
        let elapsed = worker.elapsed_time();
        let nodes = worker.nodes();

        total_time += elapsed;
        total_nodes += nodes;
        state.tt.clear();
    }

    let total_time = total_time.as_millis();
    let nps = (total_nodes * 1000) / total_time.max(1) as u64;
    println!("{total_nodes} nodes {nps} nps {total_time} ms");
}

#[cfg(test)]
mod test {
    use std::{
        iter,
        sync::{mpsc::channel, Arc, Mutex},
        thread::{available_parallelism, spawn},
    };

    use crate::perft::perft;

    /// The depth to which each position will run `perft`.
    const PERFT_DEPTH: u8 = 4;
    /// Test positions with an expected depth 4 perft result at the end.
    static TEST_POSITIONS: &str = include_str!("../test_positions.epd");

    /// A FEN string and its expected result at depth [`PERFT_DEPTH`].
    struct TestPosition {
        position: String,
        perft_result: u64,
    }

    impl TestPosition {
        /// Creates a new [`TestPosition`].
        const fn new(position: String, perft_result: u64) -> Self {
            Self {
                position,
                perft_result,
            }
        }

        /// Runs [`perft`] on the internal FEN string and asserts the stored
        /// result and the perft result match.
        fn run_test(&self) {
            let board = self.position.parse().unwrap();
            assert_eq!(
                perft::<false, false>(&board, PERFT_DEPTH),
                self.perft_result,
                "incorrect result for position {}",
                self.position,
            );
        }
    }

    /// Runs perft to depth 4 on all positions in [`TEST_POSITIONS`].
    ///
    /// It treats the first 6 tokens of a line as the FEN string and the last
    /// token as the expected node count.
    #[test]
    fn test_positions() {
        let (tx, rx) = channel();
        let rx = Arc::new(Mutex::new(rx));
        let mut handles = Vec::new();

        // add all test positions to the queue
        for position in TEST_POSITIONS.lines() {
            let mut tokens = position.split_whitespace();
            let mut fen_str = String::new();

            // get the expected perft result
            let result = tokens
                .next_back()
                .and_then(|result| result.parse::<u64>().ok())
                .unwrap();

            // get the FEN string
            for token in tokens.take(6) {
                fen_str.push_str(token);
                fen_str.push(' ');
            }

            let test_pos = TestPosition::new(fen_str, result);
            tx.send(test_pos).unwrap();
        }

        for _ in 0..available_parallelism().map_or(1, |p| p.get()) {
            let rx = Arc::clone(&rx);

            handles.push(spawn(move || {
                for test_pos in iter::from_fn(|| rx.lock().unwrap().try_recv().ok()) {
                    test_pos.run_test();
                }
            }));
        }

        for handle in handles {
            handle.join().expect("A position has failed!");
        }
    }
}
