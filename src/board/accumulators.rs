use super::Board;
use crate::{
    defs::{Piece, Square},
    evaluation::{Score, PHASE_WEIGHTS, PIECE_SQUARE_TABLES},
    index_unchecked, out_of_bounds_is_unreachable,
};

impl Board {
    /// Gets the phase of the game. 0 is midgame and 24 is endgame.
    ///
    /// Since this value is incrementally upadated, this function is zero-cost
    /// to call.
    pub const fn phase(&self) -> u8 {
        self.phase_accumulator
    }

    /// Calculates the current material + piece-square table balance.
    ///
    /// Since this value is incrementally upadated, this function is zero-cost
    /// to call.
    pub const fn psq(&self) -> Score {
        self.psq_accumulator
    }

    /// Recalculates the accumulators from scratch. Prefer to use functions
    /// that incrementally update both if possible.
    pub fn refresh_accumulators(&mut self) {
        self.clear_accumulators();

        // the compiler should realise the clone is pointless and remove it
        for (square, piece) in self.mailbox.clone().iter().enumerate() {
            self.add_psq_piece(Square(square as u8), *piece);
            self.add_phase_piece(*piece);
        }
    }

    /// Updates the piece-square table accumulator by adding the difference
    /// between the psqt value of the start and end square (which can be
    /// negative).
    pub fn move_psq_piece(&mut self, start: Square, end: Square, piece: Piece) {
        self.remove_psq_piece(start, piece);
        self.add_psq_piece(end, piece);
    }

    /// Adds `piece` to `self.phase`.
    pub fn add_phase_piece(&mut self, piece: Piece) {
        self.phase_accumulator += index_unchecked!(PHASE_WEIGHTS, piece.to_index());
    }

    /// Removes `piece` from `self.phase`.
    pub fn remove_phase_piece(&mut self, piece: Piece) {
        self.phase_accumulator -= index_unchecked!(PHASE_WEIGHTS, piece.to_index());
    }

    /// Adds the piece-square table value for `piece` at `square` to the psqt
    /// accumulator.
    pub fn add_psq_piece(&mut self, square: Square, piece: Piece) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(piece.to_index(), PIECE_SQUARE_TABLES.len()) };
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), PIECE_SQUARE_TABLES[0].len()) };
        self.psq_accumulator += PIECE_SQUARE_TABLES[piece.to_index()][square.to_index()];
    }

    /// Removes the piece-square table value for `piece` at `square` from the
    /// psqt accumulator.
    pub fn remove_psq_piece(&mut self, square: Square, piece: Piece) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(piece.to_index(), PIECE_SQUARE_TABLES.len()) };
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), PIECE_SQUARE_TABLES[0].len()) };
        self.psq_accumulator -= PIECE_SQUARE_TABLES[piece.to_index()][square.to_index()];
    }

    /// Clears the phase and psq accumulators.
    pub fn clear_accumulators(&mut self) {
        self.phase_accumulator = 0;
        self.psq_accumulator = Score(0, 0);
    }
}
