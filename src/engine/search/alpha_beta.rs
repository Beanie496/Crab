use super::{Pv, SearchInfo};
use crate::{
    board::{Board, Moves},
    defs::MoveType,
    evaluation::{evaluate, mated_in, Eval, DRAW, MATE},
};

/// Performs negamax on `board`. Returns the evaluation of after searching
/// to the given depth.
#[allow(clippy::module_name_repetitions)]
pub fn alpha_beta_search(
    search_info: &mut SearchInfo,
    board: &Board,
    mut alpha: Eval,
    beta: Eval,
    depth: u8,
) -> Eval {
    if depth == 0 {
        return quiescent_search(search_info, board, alpha, beta);
    }

    search_info.nodes += 1;

    // Stop if needed. The return value isn't important because it will be
    // discarded anyway.
    if search_info.should_stop() {
        return 0;
    }

    let mut pv = Pv::new();
    let mut moves = Moves::new();
    board.generate_moves::<{ MoveType::ALL }>(&mut moves);
    let mut total_moves = 0;

    for mv in moves {
        let mut copy = board.clone();
        if !copy.make_move(mv) {
            continue;
        }

        let result = -alpha_beta_search(search_info, &copy, -beta, -alpha, depth - 1);

        // This position is too good - our opponent is guaranteed a worse
        // position if they pick this position, so they'll never pick it -
        // meaning we can stop searching.
        if result >= beta {
            return beta;
        }

        // We've found a better move for us, but not good enough to raise beta.
        if result > alpha {
            alpha = result;
            pv.clear();
            pv.enqueue(mv);
            pv.append_pv(&mut search_info.pv);
        }

        total_moves += 1;
    }

    if total_moves == 0 {
        return if board.is_in_check() {
            mated_in(search_info.depth - depth)
        } else {
            DRAW
        };
    }

    search_info.pv = pv;

    alpha
}

/// Perform a quiescent search on the current position. This is similar to
/// alpha-beta but it only examines captures.
fn quiescent_search(
    search_info: &mut SearchInfo,
    board: &Board,
    mut alpha: Eval,
    beta: Eval,
) -> Eval {
    search_info.nodes += 1;

    let stand_pat = if board.is_in_check() {
        // assume we're getting mated
        -MATE
    } else {
        evaluate(board)
    };

    if stand_pat >= beta {
        return beta;
    }
    if stand_pat > alpha {
        alpha = stand_pat;
    }

    let mut moves = Moves::new();
    board.generate_moves::<{ MoveType::CAPTURES }>(&mut moves);

    for mv in moves {
        let mut copy = board.clone();
        if !copy.make_move(mv) {
            continue;
        }

        let result = -quiescent_search(search_info, &copy, -beta, -alpha);
        if result >= beta {
            return beta;
        }
        if result > alpha {
            alpha = result;
        }
    }
    alpha
}
