use super::{Pv, SearchInfo};
use crate::{
    board::{Board, Moves},
    defs::MoveType,
    evaluation::{evaluate, Eval},
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

    let mut pv = Pv::new();
    let mut moves = Moves::new();
    board.generate_moves::<{ MoveType::ALL }>(&mut moves);

    for mv in moves {
        let mut copy = board.clone();
        if !copy.make_move(mv) {
            continue;
        }

        let result = -alpha_beta_search(search_info, &copy, -beta, -alpha, depth - 1);
        if result >= beta {
            // we can play a move that makes this position worse for our
            // opponent than what they have currently, so they would never pick
            // this node: return
            return beta;
        }
        if result > alpha {
            // we've found a better move for us, but not too good to cause a
            // beta cutoff
            alpha = result;
            pv.clear();
            pv.enqueue(mv);
            pv.append_pv(&mut search_info.pv);
        }
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

    let stand_pat = evaluate(board);
    if stand_pat >= beta {
        return beta;
    }
    if alpha < stand_pat {
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
