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

use super::{
    movepick::MovePicker, Depth, Node, NonPvNode, Pv, PvNode, SearchReferences, SearchStatus,
};
use crate::{
    board::Board,
    defs::MoveType,
    evaluation::{evaluate, mate_in, mated_in, Eval, DRAW, INF_EVAL},
    movegen::Move,
    transposition_table::{Bound, TranspositionEntry, TranspositionHit},
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
    height: Depth,
) -> Eval {
    if depth == 0 {
        return quiescence_search(search_refs, board, alpha, beta, height);
    }

    let is_in_check = board.is_in_check();
    search_refs.seldepth = search_refs.seldepth.max(height);
    search_refs.nodes += 1;

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
    let tt_hit = search_refs.tt.load(board.zobrist(), height);
    if let Some(h) = tt_hit {
        if h.depth() >= depth
            && (h.bound() == Bound::Exact
                || h.bound() == Bound::Lower && h.score() >= beta
                || h.bound() == Bound::Upper && h.score() <= alpha)
        {
            if NodeType::IS_ROOT {
                pv.enqueue(h.mv());
            }
            return h.score();
        }
    }

    let mut best_score = -INF_EVAL;
    let mut best_move = Move::null();
    let mut new_pv = Pv::new();
    let movepicker = MovePicker::new::<{ MoveType::ALL }>(
        board,
        tt_hit.map_or(Move::null(), TranspositionHit::mv),
    );

    let mut total_moves: u8 = 0;
    for mv in movepicker {
        let mut copy = *board;
        if !copy.make_move(mv) {
            continue;
        }
        search_refs.past_zobrists.push(copy.zobrist());
        total_moves += 1;

        // make sure we always have at least one legal move ready to play
        if NodeType::IS_ROOT && total_moves == 1 {
            pv.enqueue(mv);
        }

        if NodeType::IS_ROOT && search_refs.should_print() {
            println!("info currmovenumber {total_moves} currmove {mv}");
        }

        let extension = extension(is_in_check);

        let new_depth = depth + extension - 1;

        // Principle variation search (PVS) + late move reduction (LMR)
        // The first searched move is probably going to be the best because of
        // move ordering. To prove this, on the searches of subsequent moves,
        // we lower beta to alpha + 1 (a zero window) and reduce the new depth
        // depending on some heuristics. If the search then raises alpha, we do
        // a research without reducing in case the lower depth was missing
        // something. If _that_ search still raises alpha, it must have failed
        // to exceed -alpha - 1, but could have exceeded the old beta, so we
        // must do a research without reducing and with a full window. (If that
        // then exceeds alpha, then great: we've found a better move.)
        let mut score = 0;
        if !NodeType::IS_PV || total_moves > 1 {
            let reduction = reduction(search_refs, depth, total_moves);

            score = -search::<NonPvNode>(
                search_refs,
                &mut new_pv,
                &copy,
                -alpha - 1,
                -alpha,
                new_depth.saturating_sub(reduction),
                height + 1,
            );

            if score > alpha && reduction > 0 {
                score = -search::<NonPvNode>(
                    search_refs,
                    &mut new_pv,
                    &copy,
                    -alpha - 1,
                    -alpha,
                    new_depth,
                    height + 1,
                );
            }
        };

        if NodeType::IS_PV && (score > alpha || total_moves == 1) {
            score = -search::<PvNode>(
                search_refs,
                &mut new_pv,
                &copy,
                -beta,
                -alpha,
                new_depth,
                height + 1,
            );
        }

        search_refs.past_zobrists.pop();

        // if the search was stopped early, we can't trust its results
        if search_refs.check_status() != SearchStatus::Continue {
            return if NodeType::IS_ROOT { alpha } else { 0 };
        }

        best_score = best_score.max(score);

        if NodeType::IS_ROOT && search_refs.should_print() {
            println!("info currmovenumber {total_moves} currmove {mv} currscore {score} bestscore {best_score}");
        }

        // the move is even better than what we originally had
        if score > alpha {
            best_move = mv;

            // if we're in a zero-window search, raising alpha will raise beta
            // and we don't care about the PV
            if !NodeType::IS_PV {
                break;
            }

            alpha = score;

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
    let tt_entry =
        TranspositionEntry::new(board.zobrist(), best_score, best_move, depth, bound, height);
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
    search_refs.nodes += 1;

    let is_in_check = board.is_in_check();
    let mut best_score = if is_in_check {
        -INF_EVAL
    } else {
        evaluate(board)
    };

    alpha = alpha.max(best_score);
    if alpha >= beta {
        return alpha;
    }

    let movepicker = if is_in_check {
        MovePicker::new::<{ MoveType::EVASIONS }>(board, Move::null())
    } else {
        MovePicker::new::<{ MoveType::CAPTURES }>(board, Move::null())
    };

    for mv in movepicker {
        let mut copy = *board;
        if !copy.make_move(mv) {
            continue;
        }

        let score = -quiescence_search(search_refs, &copy, -beta, -alpha, height + 1);

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

/// Calculates how much to extend the search by.
const fn extension(is_in_check: bool) -> Depth {
    // more to come of course...
    let mut extension = 0;
    if is_in_check {
        extension += 1;
    }
    extension
}

/// Calculates how much to reduce the search by during late move reductions.
fn reduction(search_refs: &SearchReferences<'_>, depth: Depth, total_moves: u8) -> Depth {
    if depth >= 3 && total_moves >= 3 {
        search_refs.base_reductions[usize::from(depth)][usize::from(total_moves)]
    } else {
        0
    }
}
