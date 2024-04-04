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

use super::{Depth, Node, OtherNode, Pv, SearchInfo, SearchStatus};
use crate::{
    board::Board,
    defs::MoveType,
    evaluation::{evaluate, Eval, DRAW, INF_EVAL},
    movegen::generate_moves,
};

/// Performs a search on `board`.
///
/// Returns the evaluation of after searching to the given depth. If `NodeType`
/// is `Root`, `pv` will always have at least one legal move in it after the
/// search.
pub fn search<NodeType: Node>(
    search_info: &mut SearchInfo,
    pv: &mut Pv,
    board: &Board,
    mut alpha: Eval,
    beta: Eval,
    depth: Depth,
) -> Eval {
    if depth == 0 {
        return evaluate(board);
    }

    if !NodeType::IS_ROOT && search_info.is_draw(board.halfmoves()) {
        return DRAW;
    }

    let mut best_score = -INF_EVAL;
    let mut new_pv = Pv::new();
    let moves = generate_moves::<{ MoveType::ALL }>(board);

    let mut total_moves = 0;
    for mv in moves {
        let mut copy = *board;
        if !copy.make_move(mv) {
            continue;
        }
        search_info.past_zobrists.push(copy.zobrist());

        // make sure we always have at least one legal move ready to play
        if NodeType::IS_ROOT && total_moves == 0 {
            pv.enqueue(mv);
        }

        let eval = -search::<OtherNode>(search_info, &mut new_pv, &copy, -beta, -alpha, depth - 1);

        search_info.past_zobrists.pop();
        search_info.nodes += 1;

        // if the search was stopped early, we can't trust its results
        if search_info.check_status() != SearchStatus::Continue {
            return 0;
        }

        // the move is the best so far at this node
        if eval > best_score {
            best_score = eval;

            // the move is even better than what we originally had
            if eval > alpha {
                alpha = eval;
                pv.clear();
                pv.enqueue(mv);
                pv.append_pv(&mut new_pv);

                // the move is too good: our opponent is never going to pick
                // the move that leads to this node because it is guaranteed to
                // result in a worse position for them, so we can safely prune
                // this node
                if eval >= beta {
                    // fail-soft
                    return eval;
                }
            }
        }

        new_pv.clear();
        total_moves += 1;
    }

    if !NodeType::IS_ROOT && total_moves == 0 {
        -INF_EVAL
    } else {
        DRAW
    };

    best_score
}
