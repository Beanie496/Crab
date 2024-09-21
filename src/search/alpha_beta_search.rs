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
    AllMovesPicker, Depth, Height, Node, NonPvNode, Pv, PvNode, QuiescenceMovePicker, SearchStatus,
    Worker,
};
use crate::{
    board::Board,
    evaluation::{evaluate, Evaluation},
    lookups::base_reductions,
    movegen::{CapturesOnly, Evasions, Moves},
    transposition_table::{Bound, TranspositionEntry, TranspositionHit},
};

impl Worker<'_> {
    /// Performs a search on `board`.
    ///
    /// Returns the evaluation of after searching to the given depth. If
    /// `NodeType` is `Root`, `pv` will always have at least one legal move in
    /// it after the search.
    #[allow(clippy::cognitive_complexity)]
    pub fn search<NodeType: Node>(
        &mut self,
        pv: &mut Pv,
        board: &Board,
        mut alpha: Evaluation,
        mut beta: Evaluation,
        depth: Depth,
        height: Height,
    ) -> Evaluation {
        if depth <= 0 {
            return self.quiescence_search(board, alpha, beta, height);
        }

        let is_in_check = board.is_in_check();
        self.seldepth = self.seldepth.max(height);
        self.nodes += 1;

        if !NodeType::IS_ROOT {
            // Mate distance pruning: if the score of mating in the next move
            // (`mate_after(height + 1)`) is still unable to exceed alpha, we
            // can prune. Likewise, if we're getting mated right now
            // (`mated_after(height)`) and we're still exceeding beta, we can
            // prune.
            alpha = alpha.max(Evaluation::mated_after(height));
            beta = beta.min(Evaluation::mate_after(height + 1));
            if alpha >= beta {
                return alpha;
            }

            // draw by repetition or 50mr
            if self.is_draw(board.halfmoves(), board.key()) {
                return Evaluation::DRAW;
            }
        }

        // load from tt
        let tt_hit = self.state.tt.load(board.key(), height);
        if let Some(h) = tt_hit {
            if !NodeType::IS_PV
                && h.depth() >= depth
                && (h.bound() == Bound::Exact
                    || h.bound() == Bound::Lower && h.score() >= beta
                    || h.bound() == Bound::Upper && h.score() <= alpha)
            {
                return h.score();
            }
        }
        let tt_move = tt_hit.and_then(TranspositionHit::mv);

        let (original_static_eval, static_eval) = if is_in_check {
            let eval = -Evaluation::INFINITY;
            (eval, eval)
        } else {
            let eval = tt_hit.map_or_else(|| evaluate(board), TranspositionHit::static_eval);
            (eval, eval + self.histories.correction_history_delta(board))
        };

        self.histories.clear_next_killers(height);

        if !NodeType::IS_PV && !is_in_check {
            // Null move pruning: if we can give the opponent a free move (by
            // not moving a piece this move) and the resulting evaluation on a
            // reduced-depth search is above beta, we will probably be able to
            // beat beta if we do have a move. The only time this isn't true is
            // in zugzwang, which usually happens in king + pawn endgames.
            if self.nmp_rights.can_make_null_move(board.side_to_move())
                && depth >= 3
                && static_eval >= beta
                && beta > -Evaluation::MATE_BOUND
                && board.has_non_pawn_pieces()
            {
                let reduction = null_move_reduction(static_eval, beta, depth);

                let mut copy = *board;
                self.make_null_move(&mut copy);

                let mut new_pv = Pv::new();
                let mut score = -self.search::<NonPvNode>(
                    &mut new_pv,
                    &copy,
                    -beta,
                    -alpha,
                    depth - reduction,
                    height + 1,
                );

                self.unmake_null_move(board);

                if score >= beta && score < Evaluation::MATE_BOUND {
                    if depth <= 8 {
                        return score;
                    }

                    self.nmp_rights.remove_right(board.side_to_move());

                    new_pv.clear();
                    // do a verification search at higher depths
                    score = self.search::<NonPvNode>(
                        &mut new_pv,
                        board,
                        alpha,
                        beta,
                        depth - reduction,
                        height,
                    );

                    self.nmp_rights.add_right(board.side_to_move());

                    if score >= beta {
                        return score;
                    }
                }
            }
        }

        let mut best_score = -Evaluation::INFINITY;
        let mut best_move = None;
        let mut new_pv = Pv::new();
        let killers = self.histories.current_killers(height);
        let last_history_item = self.histories.board_history.last();
        let counter_move = last_history_item.and_then(|item| self.histories.counter_move(*item));
        let mut movepicker = AllMovesPicker::new(tt_move, killers, counter_move);

        let mut total_moves: u8 = 0;
        let mut quiet_moves = Moves::new();
        while let Some(mv) = movepicker.next(board, &self.histories) {
            let is_quiet = board.is_quiet(mv);
            let mut copy = *board;
            if !self.make_move(&mut copy, mv) {
                continue;
            }
            self.state.tt.prefetch(copy.key());
            total_moves += 1;

            if NodeType::IS_ROOT && self.should_print() {
                println!("info currmovenumber {total_moves} currmove {mv}");
            }

            let reduction = late_move_reduction::<NodeType>(depth, total_moves);
            let mut new_depth = depth - 1;

            if !NodeType::IS_PV && !is_in_check {
                let lmr_depth = new_depth - reduction;

                // Futility pruning: if the static evaluation is too bad, it's
                // not worth it to search quiet moves, whether or not the score
                // exceeds/can exceed alpha
                if is_quiet
                    && !best_score.is_mate()
                    && lmr_depth <= 5
                    && static_eval + futility_margin(lmr_depth) <= alpha
                {
                    movepicker.skip_quiets();
                }
            }

            if is_quiet {
                quiet_moves.push(mv);
            }

            let extension = extension(is_in_check);
            new_depth += extension;

            // Principle variation search (PVS) + late move reduction (LMR):
            // The first searched move is probably going to be the best because
            // of move ordering. To prove this, on the searches of subsequent
            // moves, we lower beta to alpha + 1 (a zero window) and reduce the
            // new depth depending on some heuristics. If the search then
            // raises alpha, we do a research without reducing in case the
            // lower depth was missing something. If _that_ search still raises
            // alpha, it must have failed to exceed -alpha - 1, but could have
            // exceeded the old beta, so we must do a research without reducing
            // and with a full window. (If that then exceeds alpha, then great:
            // we've found a better move.)
            let mut score = Evaluation::default();
            if !NodeType::IS_PV || total_moves > 1 {
                score = -self.search::<NonPvNode>(
                    &mut new_pv,
                    &copy,
                    -alpha - 1,
                    -alpha,
                    new_depth - reduction,
                    height + 1,
                );

                if score > alpha && reduction > 0 {
                    score = -self.search::<NonPvNode>(
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
                score = -self.search::<PvNode>(
                    &mut new_pv,
                    &copy,
                    -beta,
                    -alpha,
                    new_depth,
                    height + 1,
                );
            }

            self.unmake_move();

            // if the search was stopped early, we can't trust its results
            if self.check_status() != SearchStatus::Continue {
                // in the (admittedly never observed before) scenario where the
                // search was terminated during depth 1 and the PV was never
                // updated, just add whatever move the search is currently on
                if NodeType::IS_ROOT && pv.len() == 0 {
                    pv.enqueue(mv);
                }
                return if NodeType::IS_ROOT {
                    alpha
                } else {
                    Evaluation::default()
                };
            }

            best_score = best_score.max(score);

            // the move is even better than what we originally had
            if score > alpha {
                best_move = Some(mv);

                // if we're in a zero-window search, raising alpha will raise
                // beta and we don't care about the PV
                if !NodeType::IS_PV {
                    break;
                }

                alpha = score;

                pv.clear();
                pv.enqueue(mv);
                pv.append_pv(&new_pv);

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
                Evaluation::mated_after(height)
            } else {
                Evaluation::DRAW
            };
        }

        if let Some(best_move) = best_move {
            if board.is_quiet(best_move) {
                self.histories.insert_into_killers(height, best_move);
                if let Some(&last_item) = self.histories.board_history.last() {
                    self.histories
                        .insert_into_counter_moves(last_item, best_move);
                }

                self.histories.update_butterfly_history(
                    &quiet_moves,
                    best_move,
                    board.side_to_move(),
                    depth,
                );

                self.histories
                    .update_continuation_history(board, &quiet_moves, best_move, depth);
            }
        }

        self.histories
            .update_correction_history(board, best_score - static_eval, depth);

        // store into tt
        let bound = if best_score >= beta {
            Bound::Lower
        // this only happens if we fail to raise alpha
        } else if best_move.is_none() {
            Bound::Upper
        } else {
            Bound::Exact
        };
        // note that the original static eval is saved, since the static eval
        // used from the TT is then corrected
        let tt_entry = TranspositionEntry::new(
            board.key(),
            original_static_eval,
            best_score,
            best_move,
            depth,
            bound,
            height,
        );
        self.state.tt.store(tt_entry);

        best_score
    }

    /// Performs a search that only considers captures and uses a static
    /// evaluation at the leaf nodes.
    ///
    /// This should be called at the leaf nodes of the main search.
    fn quiescence_search(
        &mut self,
        board: &Board,
        mut alpha: Evaluation,
        beta: Evaluation,
        height: Height,
    ) -> Evaluation {
        self.seldepth = self.seldepth.max(height);
        self.nodes += 1;

        let is_in_check = board.is_in_check();
        let mut best_score = if is_in_check {
            Evaluation::mated_after(height)
        } else {
            evaluate(board)
        };

        if height.is_maximum() {
            return best_score;
        }

        alpha = alpha.max(best_score);
        if alpha >= beta {
            return alpha;
        }

        let mut movepicker = if is_in_check {
            QuiescenceMovePicker::new::<Evasions>()
        } else {
            QuiescenceMovePicker::new::<CapturesOnly>()
        };

        while let Some(mv) = movepicker.next(board, &self.histories) {
            let mut copy = *board;
            if !copy.make_move(mv) {
                continue;
            }

            let score = -self.quiescence_search(&copy, -beta, -alpha, height + 1);

            if self.check_status() != SearchStatus::Continue {
                return Evaluation::default();
            }

            best_score = best_score.max(score);
            alpha = alpha.max(score);
            if alpha >= beta {
                return alpha;
            }
        }

        best_score
    }
}

/// Calculates the reduction for a move.
fn null_move_reduction(static_eval: Evaluation, beta: Evaluation, depth: Depth) -> Depth {
    Depth::from(((static_eval - beta) / 200).min(Evaluation(6))) + depth / 3 + 3
}

/// Calculates the margin for futility pruning.
fn futility_margin(depth: Depth) -> Evaluation {
    Evaluation::from(depth) * 80 + 70
}

/// Calculates how much to extend the search by.
fn extension(is_in_check: bool) -> Depth {
    // more to come of course...
    let mut extension = Depth::default();
    if is_in_check {
        extension += 1;
    }
    extension
}

/// Calculates how much to reduce the search by during late move reductions.
fn late_move_reduction<NodeType: Node>(depth: Depth, total_moves: u8) -> Depth {
    if depth >= 3 && total_moves >= 3 {
        base_reductions(depth, total_moves) + Depth::from(!NodeType::IS_PV)
    } else {
        Depth::default()
    }
}
