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

use super::{Board, CastlingRights, Key};
use crate::{
    cfor,
    defs::{Piece, Square},
    evaluation::{piece_phase, piece_score, Phase, Score},
    index_unchecked, out_of_bounds_is_unreachable,
};

#[allow(clippy::doc_markdown)]
/// Returns a pseudo-random number from `seed` using the Xorshift64 algorithm.
macro_rules! rand {
    ($seed:tt) => {{
        // ripped straight from wikipedia
        $seed ^= $seed << 13;
        $seed ^= $seed >> 7;
        $seed ^= $seed << 17;
        $seed
    }};
}

/// A container for the zobrist keys. Initialised at program startup.
struct ZobristKeys {
    /// The keys for each of the pieces on each of the squares and the side to
    /// move, plus a 13th table for no piece.
    ///
    /// Since the first and last 8 indicies of the pawn table are never used,
    /// they can be reused. I'm using the A1 square of the Black pawn table
    /// for the side to move key.
    piece_and_side: [[Key; Piece::TOTAL + 1]; Square::TOTAL],
    /// The castling rights keys. One for each combination for fast lookup.
    castling_rights: [Key; 16],
    /// The en passant keys. 65 for fast lookup.
    ep: [Key; Square::TOTAL + 1],
}

/// The program's zobrist keys.
static ZOBRIST_KEYS: ZobristKeys = ZobristKeys::new();

impl Board {
    /// Returns the accumuted phase of the board.
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    /// Returns the accumuted score of the board.
    pub const fn score(&self) -> Score {
        self.score
    }

    /// Gets the zobrist key.
    pub const fn zobrist(&self) -> Key {
        self.zobrist
    }

    /// Moves the accumulated `piece` from `start` to `end`.
    pub fn move_accumulated_piece(&mut self, start: Square, end: Square, piece: Piece) {
        self.move_piece_score(start, end, piece);
        self.move_piece_zobrist(start, end, piece);
    }

    /// Adds `piece` on `square` to the accumulators.
    pub fn add_accumulated_piece(&mut self, square: Square, piece: Piece) {
        self.add_piece_phase(piece);
        self.add_piece_score(square, piece);
        self.toggle_piece_zobrist(square, piece);
    }

    /// Removes `piece` on `square` from the accumulators.
    pub fn remove_accumulated_piece(&mut self, square: Square, piece: Piece) {
        self.remove_piece_phase(piece);
        self.remove_piece_score(square, piece);
        self.toggle_piece_zobrist(square, piece);
    }

    /// Adds the value of `piece` to the phase accumulator.
    fn add_piece_phase(&mut self, piece: Piece) {
        self.phase += piece_phase(piece);
    }

    /// Removes the value of `piece` to the phase accumulator.
    fn remove_piece_phase(&mut self, piece: Piece) {
        self.phase -= piece_phase(piece);
    }

    /// Adds the value of `piece` to the score accumulator.
    fn move_piece_score(&mut self, start: Square, end: Square, piece: Piece) {
        self.remove_piece_score(start, piece);
        self.add_piece_score(end, piece);
    }

    /// Adds the value of `piece` to the score accumulator.
    fn add_piece_score(&mut self, square: Square, piece: Piece) {
        self.score += piece_score(square, piece);
    }

    /// Removes the value of `piece` to the score accumulator.
    fn remove_piece_score(&mut self, square: Square, piece: Piece) {
        self.score -= piece_score(square, piece);
    }

    /// Removes the zobrist key of `piece` on `start` and adds it to `end`.
    fn move_piece_zobrist(&mut self, start: Square, end: Square, piece: Piece) {
        self.toggle_piece_zobrist(start, piece);
        self.toggle_piece_zobrist(end, piece);
    }

    /// Toggles the zobrist key of the given piece on the given square.
    ///
    /// `piece` can be [`Piece::NONE`] but `square` has to be a valid square.
    fn toggle_piece_zobrist(&mut self, square: Square, piece: Piece) {
        self.zobrist ^= ZOBRIST_KEYS.piece_key(square, piece);
    }

    /// Toggles the side to move zobrist key.
    pub fn toggle_side_zobrist(&mut self) {
        self.zobrist ^= ZOBRIST_KEYS.side_key();
    }

    /// Toggles the zobrist keys of the given castling rights.
    pub fn toggle_castling_rights_zobrist(&mut self, rights: CastlingRights) {
        self.zobrist ^= ZOBRIST_KEYS.castling_rights_key(rights);
    }

    /// Toggles the zobrist keys of the given en passant square.
    pub fn toggle_ep_square_zobrist(&mut self, square: Square) {
        self.zobrist ^= ZOBRIST_KEYS.ep_square_key(square);
    }
}

impl ZobristKeys {
    /// Generates new pseudo-random zobrist keys.
    const fn new() -> Self {
        // arbitrary 8 bytes from /dev/random
        let mut seed = 0xc815_1848_573b_e077u64;
        let mut piece_and_side = [[0u64; Piece::TOTAL + 1]; Square::TOTAL];
        let mut castling_rights = [0u64; 16];
        let mut ep = [0u64; Square::TOTAL + 1];

        cfor!(let mut square = 0; square < Square::TOTAL; square += 1; {
            cfor!(let mut piece = 0; piece < Piece::TOTAL; piece += 1; {
                piece_and_side[square][piece] = rand!(seed);
            });
        });

        cfor!(let mut i = 0; i < castling_rights.len(); i += 1; {
            castling_rights[i] = rand!(seed);
        });

        cfor!(let mut square = Square::A3.0; square <= Square::H3.0; square += 1; {
            ep[square as usize] = rand!(seed);
        });
        cfor!(let mut square = Square::A6.0; square <= Square::H6.0; square += 1; {
            ep[square as usize] = rand!(seed);
        });

        Self {
            piece_and_side,
            castling_rights,
            ep,
        }
    }

    /// Calculates the key of the given piece on the given square.
    fn piece_key(&self, square: Square, piece: Piece) -> Key {
        out_of_bounds_is_unreachable!(square.to_index(), self.piece_and_side.len());
        out_of_bounds_is_unreachable!(piece.to_index(), self.piece_and_side[0].len());
        self.piece_and_side[square.to_index()][piece.to_index()]
    }

    /// Calculates the side to move key.
    const fn side_key(&self) -> Key {
        self.piece_and_side[Square::A1.to_index()][Piece::BPAWN.to_index()]
    }

    /// Calculates the key of the given castling rights.
    fn castling_rights_key(&self, rights: CastlingRights) -> Key {
        index_unchecked!(self.castling_rights, rights.0 as usize)
    }

    /// Calculates the key of the given square.
    fn ep_square_key(&self, square: Square) -> Key {
        index_unchecked!(self.ep, square.to_index())
    }
}
