use super::Board;
use crate::{
    defs::{Piece, Square},
    evaluation::{Score, PHASE_WEIGHTS, PIECE_SQUARE_TABLES},
    out_of_bounds_is_unreachable,
};

impl Board {
    /// Calculates the current material + piece-square table balance.
    ///
    /// Since this value is incrementally upadated, this function is zero-cost
    /// to call.
    pub const fn psq(&self) -> Score {
        self.psq_accumulator
    }

    /// Gets the phase of the game. 0 is midgame and 24 is endgame.
    ///
    /// Since this value is incrementally upadated, this function is zero-cost
    /// to call.
    pub const fn phase(&self) -> u8 {
        self.phase_accumulator
    }

    /// Recalculates the accumulators from scratch. Prefer to use functions
    /// that incrementally update both if possible.
    pub fn refresh_accumulators(&mut self) {
        let mut score = Score(0, 0);
        let mut phase = 0;

        for (square, piece) in self.mailbox.iter().enumerate() {
            score += PIECE_SQUARE_TABLES[piece.to_index()][square];
            phase += PHASE_WEIGHTS[piece.to_index()];
        }

        self.psq_accumulator = score;
        self.phase_accumulator = phase;
    }

    /// Updates the piece-square table accumulator by adding the difference
    /// between the psqt value of the start and end square (which can be
    /// negative).
    pub fn move_psq_piece(&mut self, start: Square, end: Square, piece: Piece) {
        self.remove_psq_piece(start, piece);
        self.add_psq_piece(end, piece);
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

    /// Adds `piece` to `self.phase`.
    pub fn add_phase_piece(&mut self, piece: Piece) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(piece.to_index(), PHASE_WEIGHTS.len()) };
        self.phase_accumulator += PHASE_WEIGHTS[piece.to_index()];
    }

    /// Removes `piece` from `self.phase`.
    pub fn remove_phase_piece(&mut self, piece: Piece) {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(piece.to_index(), PHASE_WEIGHTS.len()) };
        self.phase_accumulator -= PHASE_WEIGHTS[piece.to_index()];
    }
}