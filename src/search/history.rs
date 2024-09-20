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
    board::Key,
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

/// Information needed for indexing into counter moves.
#[derive(Clone, Copy)]
pub struct CounterMoveInfo {
    /// The piece being moved.
    piece: Piece,
    /// The destination square of that piece.
    dest: Square,
}

/// An item of the board history.
#[derive(Clone, Copy)]
pub struct HistoryItem {
    /// The key of the item.
    pub key: Key,
    /// Information for counter moves.
    ///
    /// It's an [`Option`] because of null moves.
    pub counter_move_info: Option<CounterMoveInfo>,
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
    // `/ 2` to prevent overflow
    const MAX_HISTORY_VAL: CompressedEvaluation = CompressedEvaluation(i16::MAX / 2);
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

impl CounterMoveInfo {
    /// Creates new [`CounterMoveInfo`].
    pub const fn new(piece: Piece, dest: Square) -> Self {
        Self { piece, dest }
    }
}

impl HistoryItem {
    /// Creates a new [`HistoryItem`] with the given fields.
    pub const fn new(key: Key, counter_move_info: Option<CounterMoveInfo>) -> Self {
        Self {
            key,
            counter_move_info,
        }
    }
}

impl Histories {
    /// Creates new, empty [`Histories`].
    pub fn new() -> Self {
        Self {
            butterfly_history: Box::new(
                [[[CompressedEvaluation(0); Square::TOTAL]; Square::TOTAL]; Side::TOTAL],
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

    /// Clears all the histories apart from the board history.
    pub fn clear(&mut self) {
        self.butterfly_history =
            Box::new([[[CompressedEvaluation(0); Square::TOTAL]; Square::TOTAL]; Side::TOTAL]);
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
            let abs_bonus = Self::bonus(depth);
            let signed_bonus = if best_move == mv {
                abs_bonus
            } else {
                -abs_bonus
            };

            let val = &mut self.butterfly_history[side][start][end];
            // val cannot exceed MAX_HISTORY_VAL, so the bonus is lerped
            // between its original value (for val == 0) and 0 (for val ==
            // MAX_HISTORY_VAL)
            let delta = signed_bonus
                - abs_bonus * Evaluation::from(*val) / Evaluation::from(Self::MAX_HISTORY_VAL);
            *val += CompressedEvaluation::from(delta);
        }
    }

    /// Returns the butterfly score of a move by the given side from `start` to
    /// `end`.
    pub fn get_butterfly_score(
        &self,
        side: Side,
        start: Square,
        end: Square,
    ) -> CompressedEvaluation {
        self.butterfly_history[side.to_index()][start.to_index()][end.to_index()]
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
        if let Some(counter_move_info) = history_item.counter_move_info {
            let piece = counter_move_info.piece.to_index();
            let square = counter_move_info.dest.to_index();

            self.counter_moves[piece][square] = Some(mv);
        }
    }

    /// Gets the counter move as indexed by `history_item`.
    pub fn get_counter_move(&self, history_item: HistoryItem) -> Option<Move> {
        history_item.counter_move_info.and_then(|info| {
            let piece = info.piece.to_index();
            let square = info.dest.to_index();
            self.counter_moves[piece][square]
        })
    }
}
