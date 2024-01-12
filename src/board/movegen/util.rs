use crate::defs::{Bitboard, Direction, File, Piece, Rank, Square};

/// A thin wrapper over a [`Bitboard`] to allow iteration over it.
pub struct BitIter {
    board: Bitboard,
}

impl Iterator for BitIter {
    type Item = Square;

    /// Clears the LSB of the wrapped [`Bitboard`] and returns the position of
    /// that bit. Returns [`None`] if there are no set bits.
    fn next(&mut self) -> Option<Self::Item> {
        if self.board.inner() == 0 {
            None
        } else {
            Some(self.board.pop_next_square())
        }
    }
}

impl BitIter {
    /// Wraps a [`Bitboard`] in a [`BitIter`].
    pub fn new(bb: Bitboard) -> BitIter {
        Self { board: bb }
    }
}

/// Generates all combinations of attacks from `square` and puts them in
/// `attacks`. It starts with a full blocker board that goes from the
/// square to the edge exclusive and uses the Carry-Rippler trick to
/// generate each subsequent attack.
pub fn gen_all_sliding_attacks<const PIECE: u8>(
    square: Square,
    attacks: &mut [Bitboard; crate::board::movegen::magic::MAX_BLOCKERS],
) {
    let excluded_ranks_bb = (Bitboard::file_bb(File::FILE1) | Bitboard::file_bb(File::FILE8))
        & !Bitboard::file_bb(square.file_of());
    let excluded_files_bb = (Bitboard::rank_bb(Rank::RANK1) | Bitboard::rank_bb(Rank::RANK8))
        & !Bitboard::rank_bb(square.rank_of());
    let edges = excluded_ranks_bb | excluded_files_bb;
    let mask = sliding_attacks::<PIECE>(square, Bitboard::from(0)) & !edges;

    let mut blockers = mask;
    let mut first_empty = 0;
    while blockers != Bitboard::from(0) {
        attacks[first_empty] = sliding_attacks::<PIECE>(square, blockers);
        first_empty += 1;
        blockers = Bitboard::from(blockers.inner().wrapping_sub(1)) & mask;
    }
    attacks[first_empty] = sliding_attacks::<PIECE>(square, Bitboard::from(0));
}

/// Checks if `square` can go in the given direction.
pub fn is_valid<const DIRECTION: i8>(square: Square) -> bool {
    let dest = Square::from(square.inner().wrapping_add(DIRECTION as u8));
    // credit to Stockfish, as I didn't come up with this code.
    // It checks if `square` is still within the board, and if it is, it checks
    // if it hasn't wrapped (because if it has wrapped, the distance will be
    // larger than 2).
    dest.is_valid() && square.horizontal_distance(dest) <= 1
}

/// Generates an attack from `square` in the given direction up to and
/// including the first encountered bit set in `blockers`. `blockers` is
/// assumed not to include `square` itself.
pub fn ray_attack<const DIRECTION: i8>(mut square: Square, blockers: Bitboard) -> Bitboard {
    let mut attacks = Bitboard::EMPTY;
    // checks if the next square is valid and if the piece can move from the
    // square
    while is_valid::<DIRECTION>(square)
        && Bitboard::from_square(square) & blockers == Bitboard::from(0)
    {
        square = Square::from(square.inner().wrapping_add(DIRECTION as u8));
        attacks |= Bitboard::from_square(square);
    }
    attacks
}

/// Generates the attack set for `piece` on `square` up to and including the
/// given blockers. Includes the edge.
pub fn sliding_attacks<const PIECE: u8>(square: Square, blockers: Bitboard) -> Bitboard {
    let piece = Piece::from(PIECE);
    let mut ray = Bitboard::EMPTY;
    if piece == Piece::BISHOP {
        ray |= ray_attack::<{ Direction::NE.inner() }>(square, blockers);
        ray |= ray_attack::<{ Direction::SE.inner() }>(square, blockers);
        ray |= ray_attack::<{ Direction::SW.inner() }>(square, blockers);
        ray |= ray_attack::<{ Direction::NW.inner() }>(square, blockers);
    } else {
        ray |= ray_attack::<{ Direction::N.inner() }>(square, blockers);
        ray |= ray_attack::<{ Direction::E.inner() }>(square, blockers);
        ray |= ray_attack::<{ Direction::S.inner() }>(square, blockers);
        ray |= ray_attack::<{ Direction::W.inner() }>(square, blockers);
    };
    ray
}
