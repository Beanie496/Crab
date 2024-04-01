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

use std::time::Instant;

use crate::{board::Board, defs::MoveType, movegen::generate_moves};

/// Outputs and returns the number of leaf nodes `depth` moves in the future.
///
/// If `IS_TIMED`, it will also output the time taken and the average NPS.
pub fn perft<const SHOULD_PRINT: bool, const IS_TIMED: bool>(board: &Board, depth: u8) -> u64 {
    if IS_TIMED {
        let time = Instant::now();
        let result = perft::<SHOULD_PRINT, false>(board, depth);
        let elapsed_us = time.elapsed().as_micros() as u64;
        println!(
            "Time taken: {} ms; NPS: {}",
            elapsed_us / 1_000,
            1_000_000 * result / elapsed_us
        );
        return result;
    }

    if SHOULD_PRINT {
        println!("Result:");
        if depth == 0 {
            println!("1");
            return 1;
        }
    }

    if depth == 0 {
        return 1;
    }

    let moves = generate_moves::<{ MoveType::ALL }>(board);

    let mut total = 0;
    for mv in moves {
        let mut copy = *board;
        if !copy.make_move(mv) {
            continue;
        }

        let moves = perft::<false, false>(&copy, depth - 1);
        total += moves;

        if SHOULD_PRINT {
            println!("{mv}: {moves}");
        }
    }
    if SHOULD_PRINT {
        println!("Total: {total}");
    }
    total
}
