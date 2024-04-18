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

use super::{Depth, Node, OtherNode, Pv, SearchReferences, SearchStatus};
use crate::{
    board::Board,
    defs::MoveType,
    evaluation::{evaluate, is_mate, mate_in, mated_in, Eval, DRAW, INF_EVAL},
    movegen::{generate_moves, Move},
    transposition_table::{Bound, TranspositionEntry},
};

/// Performs a search on `board`.
///
/// Returns the evaluation of after searching to the given depth. If `NodeType`
/// is `Root`, `pv` will always have at least one legal move in it after the
/// search.
pub fn search<NodeType: Node>(
    search_refs: &mut SearchReferences<'_>,
    pv: &mut Pv,
    board: &Board,
    mut alpha: Eval,
    mut beta: Eval,
    depth: Depth,
) -> Eval {
    if depth == 0 {
        return quiescence_search(search_refs, board, alpha, beta, search_refs.depth);
    }

    let height = search_refs.depth - depth;

    if !NodeType::IS_ROOT {
        // mate distance pruning
        // if the score of mating in the next move (`mate_in(height + 1)`) is
        // still unable to exceed alpha, we can prune. Likewise, if we're
        // getting mated right now (`mated_in(height)`) and we're still
        // exceeding beta, we can prune.
        alpha = alpha.max(mated_in(height));
        beta = beta.min(mate_in(height + 1));
        if alpha >= beta {
            return alpha;
        }

        // draw by repetition or 50mr
        if search_refs.is_draw(board.halfmoves()) {
            return DRAW;
        }
    }

    // load from tt
    if let Some(entry) = search_refs.tt.load(board.zobrist()) {
        if entry.depth() >= depth
            && (entry.bound() == Bound::Exact
                || entry.bound() == Bound::Lower && entry.score() >= beta
                || entry.bound() == Bound::Upper && entry.score() <= alpha)
            && !is_mate(entry.score())
        {
            if NodeType::IS_ROOT {
                pv.enqueue(entry.mv());
            }
            return entry.score();
        }
    }

    let mut best_score = -INF_EVAL;
    let mut best_move = Move::null();
    let mut new_pv = Pv::new();
    let moves = generate_moves::<{ MoveType::ALL }>(board);

    let mut total_moves = 0;
    for mv in moves {
        let mut copy = *board;
        if !copy.make_move(mv) {
            continue;
        }
        search_refs.past_zobrists.push(copy.zobrist());
        total_moves += 1;

        // make sure we always have at least one legal move ready to play
        if NodeType::IS_ROOT && total_moves == 0 {
            pv.enqueue(mv);
        }

        let score = -search::<OtherNode>(search_refs, &mut new_pv, &copy, -beta, -alpha, depth - 1);

        search_refs.past_zobrists.pop();
        search_refs.nodes += 1;

        // if the search was stopped early, we can't trust its results
        if search_refs.check_status() != SearchStatus::Continue {
            return if NodeType::IS_ROOT { alpha } else { 0 };
        }

        best_score = best_score.max(score);
        // the move is even better than what we originally had
        if score > alpha {
            alpha = score;
            best_move = mv;

            pv.clear();
            pv.enqueue(mv);
            pv.append_pv(&mut new_pv);

            // the move is too good: our opponent is never going to pick
            // the move that leads to this node because it is guaranteed to
            // result in a worse position for them, so we can safely prune
            // this node
            if alpha >= beta {
                break;
            }
        }

        new_pv.clear();
    }

    if !NodeType::IS_ROOT && total_moves == 0 {
        return if board.is_in_check() {
            mated_in(height)
        } else {
            DRAW
        };
    }

    // store into tt
    let bound = if best_score >= beta {
        Bound::Lower
    // this only happens if we fail to raise alpha
    } else if best_move == Move::null() {
        Bound::Upper
    } else {
        Bound::Exact
    };
    let tt_entry = TranspositionEntry::new(board.zobrist(), best_score, best_move, bound, depth);
    search_refs.tt.store(board.zobrist(), tt_entry);

    best_score
}

/// Performs a search that only considers captures and uses a static evaluation
/// at the leaf nodes.
///
/// This should be called at the leaf nodes of the main search.
fn quiescence_search(
    search_refs: &mut SearchReferences<'_>,
    board: &Board,
    mut alpha: Eval,
    beta: Eval,
    height: Depth,
) -> Eval {
    search_refs.seldepth = search_refs.seldepth.max(height);

    let mut best_score = evaluate(board);

    alpha = alpha.max(best_score);
    if alpha >= beta {
        return alpha;
    }

    let moves = generate_moves::<{ MoveType::CAPTURES }>(board);

    for mv in moves {
        let mut copy = *board;
        if !copy.make_move(mv) {
            continue;
        }

        let score = -quiescence_search(search_refs, &copy, -beta, -alpha, height + 1);

        search_refs.nodes += 1;

        if search_refs.check_status() != SearchStatus::Continue {
            return 0;
        }

        best_score = best_score.max(score);
        alpha = alpha.max(score);
        if alpha >= beta {
            return alpha;
        }
    }

    best_score
}
