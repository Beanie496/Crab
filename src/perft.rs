use std::time::Instant;

use crate::{board::Board, defs::MoveType};

/// Outputs and returns the number of leaf nodes `depth` moves in the future.
///
/// If `IS_TIMED`, it will also output the time taken and the average NPS.
pub fn perft<const SHOULD_PRINT: bool, const IS_TIMED: bool>(board: &mut Board, depth: u8) -> u64 {
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

    let moves = board.generate_moves::<{ MoveType::ALL }>();

    let mut total = 0;
    for mv in moves {
        if !board.make_move(mv) {
            continue;
        }

        let moves = perft::<false, false>(board, depth - 1);
        total += moves;

        if SHOULD_PRINT {
            println!("{mv}: {moves}");
        }

        board.unmake_move();
    }
    if SHOULD_PRINT {
        println!("Total: {total}");
    }
    total
}
