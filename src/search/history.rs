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

use std::ops::{Deref, DerefMut};

use arrayvec::ArrayVec;

use super::{Depth, Height};
use crate::{
    board::{Board, Key},
    defs::{Piece, Side, Square},
    evaluation::{CompressedEvaluation, Evaluation},
    movegen::{Move, Moves},
};

/// The history of a board, excluding the current state of the board.
///
/// Each item corresponds to a previous state of the board. Each item is the
/// previous item with a single move applied to it. Applying a move to the most
/// recent item would get the current board state.
#[allow(clippy::missing_docs_in_private_items)]
pub struct BoardHistory {
    history: ArrayVec<HistoryItem, { Depth::MAX.to_index() }>,
}

/// A piece and its destination square.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Clone, Copy)]
pub struct PieceDest {
    piece: Piece,
    dest: Square,
}

/// An item of the board history.
#[derive(Clone, Copy)]
pub struct HistoryItem {
    /// The key of the item.
    pub key: Key,
    /// The last piece that moves and its destination.
    ///
    /// It's an [`Option`] because of null moves.
    pub piece_dest: Option<PieceDest>,
}

/// A struct containing various histories relating to the board.
pub struct Histories {
    /// A history of bonuses for previous quiets.
    ///
    /// Indexed by side to move, start square then end square.
    ///
    /// So called because the wasted space looks a little like a butterfly's
    /// wings.
    pub butterfly_history:
        Box<[[[CompressedEvaluation; Square::TOTAL]; Square::TOTAL]; Side::TOTAL]>,
    /// Continuation history.
    ///
    /// A score for a piece/destination square depending on the previous piece
    /// that moved and its destination square.
    ///
    /// Indexed by previous piece, previous end square, current piece then
    /// current end square.
    pub continuation_history:
        Box<[[[[CompressedEvaluation; Square::TOTAL]; Piece::TOTAL]; Square::TOTAL]; Piece::TOTAL]>,
    /// Killer moves.
    ///
    /// For each depth, the best move from the previous search at the same
    /// depth that originated from the same node.
    pub killers: [[Option<Move>; 2]; Depth::MAX.to_index() + 1],
    /// Counter moves.
    ///
    /// The previous best response to a certain piece landing on a certain
    /// square.
    pub counter_moves: [[Option<Move>; Square::TOTAL]; Piece::TOTAL],
    /// A stack of keys of previous board states, beginning from the initial
    /// `position fen ...` command.
    ///
    /// The first (bottom) element is the initial board and the top element is
    /// the current board.
    pub board_history: BoardHistory,
}

impl Histories {
    /// The maximum of any history value.
    const MAX_HISTORY_VAL: Evaluation = Evaluation(16383);
}

impl Deref for BoardHistory {
    type Target = ArrayVec<HistoryItem, { Depth::MAX.to_index() }>;

    fn deref(&self) -> &Self::Target {
        &self.history
    }
}

impl DerefMut for BoardHistory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.history
    }
}

impl BoardHistory {
    /// Creates a new, empty [`BoardHistory`].
    pub fn new() -> Self {
        Self {
            history: ArrayVec::new(),
        }
    }

    /// Sets the items of `self` to `other`.
    pub fn set_to(&mut self, other: &Self) {
        self.clear();

        for &item in other.iter() {
            // SAFETY: `other.len() <= self.capacity()`
            unsafe {
                self.push_unchecked(item);
            }
        }
    }
}

impl PieceDest {
    /// Creates a new [`PieceDest`].
    pub const fn new(piece: Piece, dest: Square) -> Self {
        Self { piece, dest }
    }
}

impl HistoryItem {
    /// Creates a new [`HistoryItem`] with the given fields.
    pub const fn new(key: Key, piece_dest: Option<PieceDest>) -> Self {
        Self { key, piece_dest }
    }
}

impl Histories {
    /// Creates new, empty [`Histories`].
    // not large enough to overflow the stack in debug
    #[allow(clippy::large_stack_frames, clippy::large_stack_arrays)]
    pub fn new() -> Self {
        Self {
            butterfly_history: Box::new(
                [[[CompressedEvaluation(0); Square::TOTAL]; Square::TOTAL]; Side::TOTAL],
            ),
            continuation_history: Box::new(
                [[[[CompressedEvaluation(0); Square::TOTAL]; Piece::TOTAL]; Square::TOTAL];
                    Piece::TOTAL],
            ),
            killers: [[None; 2]; Depth::MAX.to_index() + 1],
            counter_moves: [[None; Square::TOTAL]; Piece::TOTAL],
            board_history: BoardHistory::new(),
        }
    }

    /// The bonus of a good move.
    ///
    /// It will always be in the range `0..=`[`Self::MAX_HISTORY_VAL`].
    fn bonus(depth: Depth) -> Evaluation {
        Evaluation::from(CompressedEvaluation(depth.0.min(8) * 100))
    }

    /// Updates a particular item of a history table.
    ///
    /// `is_bonus` is if the update should be a bonus (as opposed to a malus).
    fn update_history_value(
        value: &mut CompressedEvaluation,
        depth: Depth,
        is_bonus: bool,
        max_history: Evaluation,
    ) {
        let abs_bonus = Self::bonus(depth);
        let signed_bonus = if is_bonus { abs_bonus } else { -abs_bonus };
        // the value cannot exceed max_history, so the bonus is lerped between
        // its original value (for val == 0) and 0 (for val == max_history)
        let delta = signed_bonus - abs_bonus * Evaluation::from(*value) / max_history;
        *value += CompressedEvaluation::from(delta);
    }

    /// Clears all the histories apart from the board history.
    // not large enough to overflow the stack in debug
    #[allow(clippy::large_stack_frames, clippy::large_stack_arrays)]
    pub fn clear(&mut self) {
        self.butterfly_history =
            Box::new([[[CompressedEvaluation(0); Square::TOTAL]; Square::TOTAL]; Side::TOTAL]);
        self.continuation_history = Box::new(
            [[[[CompressedEvaluation(0); Square::TOTAL]; Piece::TOTAL]; Square::TOTAL];
                Piece::TOTAL],
        );
        self.counter_moves = [[None; Square::TOTAL]; Piece::TOTAL];
        self.killers[0] = [None; 2];
    }

    /// Updates the butterfly history with a bonus for `best_move` and a
    /// penalty for all other moves in `quiet_moves`.
    ///
    /// `quiet_moves` may or may not contain `best_move`.
    pub fn update_butterfly_history(
        &mut self,
        quiet_moves: &Moves,
        best_move: Move,
        side: Side,
        depth: Depth,
    ) {
        let side = side.to_index();

        for mv in quiet_moves.iter().map(|scored_move| scored_move.mv) {
            let start = mv.start().to_index();
            let end = mv.end().to_index();

            Self::update_history_value(
                &mut self.butterfly_history[side][start][end],
                depth,
                best_move == mv,
                Self::MAX_HISTORY_VAL,
            );
        }
    }

    /// Returns the butterfly score of a move by the given side from `start` to
    /// `end`.
    pub fn butterfly_score(&self, side: Side, start: Square, end: Square) -> Evaluation {
        self.butterfly_history[side.to_index()][start.to_index()][end.to_index()].into()
    }

    /// Updates the continuation history with a bonus for `best_move` and a
    /// penalty for all other moves in `quiet_moves`.
    ///
    /// This score may be across more than one ply.
    ///
    /// `quiet_moves` may or may not contain `best_move`.
    pub fn update_continuation_history(
        &mut self,
        board: &Board,
        quiet_moves: &Moves,
        best_move: Move,
        depth: Depth,
    ) {
        self.board_history
            .iter()
            .rev()
            .take(2)
            .filter_map(|item| item.piece_dest)
            .for_each(|piece_dest| {
                let prev_piece = piece_dest.piece.to_index();
                let prev_end = piece_dest.dest.to_index();

                for mv in quiet_moves.iter().map(|scored_move| scored_move.mv) {
                    let piece = board.piece_on(mv.start()).to_index();
                    let end = mv.end().to_index();

                    Self::update_history_value(
                        &mut self.continuation_history[prev_piece][prev_end][piece][end],
                        depth,
                        best_move == mv,
                        Self::MAX_HISTORY_VAL,
                    );
                }
            });
    }

    /// Returns the continuation score of the given piece moving to the given
    /// square.
    ///
    /// This score may be across more than one ply.
    pub fn continuation_history_score(&self, piece: Piece, end: Square) -> Evaluation {
        self.board_history
            .iter()
            .rev()
            .take(2)
            .filter_map(|item| item.piece_dest)
            .map(|piece_dest| {
                let prev_piece = piece_dest.piece.to_index();
                let prev_end = piece_dest.dest.to_index();
                self.continuation_history[prev_piece][prev_end][piece.to_index()][end.to_index()]
            })
            .sum::<CompressedEvaluation>()
            .into()
    }

    /// Replace the second killer of the current height with the given move.
    pub fn insert_into_killers(&mut self, height: Height, mv: Move) {
        let height = height.to_index();
        if self.killers[height][0] == Some(mv) {
            return;
        }
        self.killers[height][1] = self.killers[height][0];
        self.killers[height][0] = Some(mv);
    }

    /// Return the killers of the current height.
    pub const fn current_killers(&self, height: Height) -> [Option<Move>; 2] {
        self.killers[height.to_index()]
    }

    /// Clear the killers of the next height.
    pub fn clear_next_killers(&mut self, height: Height) {
        self.killers[height.to_index() + 1] = [None; 2];
    }

    /// Inserts `mv` into the table as given by `history_item`.
    pub fn insert_into_counter_moves(&mut self, history_item: HistoryItem, mv: Move) {
        if let Some(piece_dest) = history_item.piece_dest {
            let piece = piece_dest.piece.to_index();
            let square = piece_dest.dest.to_index();

            self.counter_moves[piece][square] = Some(mv);
        }
    }

    /// Gets the counter move as indexed by `history_item`.
    pub fn counter_move(&self, history_item: HistoryItem) -> Option<Move> {
        history_item.piece_dest.and_then(|info| {
            let piece = info.piece.to_index();
            let square = info.dest.to_index();
            self.counter_moves[piece][square]
        })
    }
}
