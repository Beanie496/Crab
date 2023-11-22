use crate::{
    defs::{
        Bitboard, Bitboards, Direction, Directions, File, Files, Piece, Pieces, Rank, Ranks, Square,
    },
    util::{file_of, is_valid, rank_of},
};

/// A thin wrapper over a [`Bitboard`] to allow iteration over it.
pub struct BitIter {
    board: Bitboard,
}

impl BitIter {
    /// Wraps a [`Bitboard`] in a [`BitIter`].
    pub fn new(bb: Bitboard) -> BitIter {
        Self { board: bb }
    }
}

impl Iterator for BitIter {
    type Item = Square;

    /// Clears the LSB of the wrapped [`Bitboard`] and returns the position of
    /// that bit. Returns [`None`] if there are no set bits.
    fn next(&mut self) -> Option<Self::Item> {
        if self.board == 0 {
            None
        } else {
            Some(pop_next_square(&mut self.board))
        }
    }
}

/// Converts `square` into its corresponding bitboard.
pub fn as_bitboard(square: Square) -> Bitboard {
    1 << square
}

/// Converts `rank` and `file` into a bitboard with the bit in the given
/// position set.
pub fn bitboard_from_pos(rank: Rank, file: File) -> Bitboard {
    1 << (rank * 8 + file)
}

/// Shifts `bb` one square east without wrapping.
pub fn east(bb: Bitboard) -> Bitboard {
    (bb << 1) & !Bitboards::FILE_BB[Files::FILE1]
}

/// Generates all combinations of attacks from `square` and puts them in
/// `attacks`. It starts with a full blocker board that goes from the
/// square to the edge exclusive and uses the Carry-Rippler trick to
/// generate each subsequent attack.
pub fn gen_all_sliding_attacks<const PIECE: Piece>(
    square: Square,
    attacks: &mut [Bitboard; crate::board::movegen::magic::MAX_BLOCKERS],
) {
    let edges = ((Bitboards::FILE_BB[Files::FILE1] | Bitboards::FILE_BB[Files::FILE8])
        & !Bitboards::FILE_BB[file_of(square)])
        | ((Bitboards::RANK_BB[Ranks::RANK1] | Bitboards::RANK_BB[Ranks::RANK8])
            & !Bitboards::RANK_BB[rank_of(square)]);
    let mask = sliding_attacks::<PIECE>(square, 0) & !edges;

    let mut blockers = mask;
    let mut first_empty = 0;
    while blockers != 0 {
        attacks[first_empty] = sliding_attacks::<PIECE>(square, blockers);
        first_empty += 1;
        blockers = (blockers - 1) & mask;
    }
    attacks[first_empty] = sliding_attacks::<PIECE>(square, 0);
}

/// Shifts `bb` one square north without wrapping.
pub fn north(bb: Bitboard) -> Bitboard {
    bb << 8
}

/// Calculates the square one step forward, depending on the pawn side.
pub fn pawn_push<const IS_WHITE: bool>(bb: Bitboard) -> Bitboard {
    if IS_WHITE {
        north(bb)
    } else {
        south(bb)
    }
}

/// Clears the least significant bit of `bb` and returns it.
pub fn pop_lsb(bb: &mut Bitboard) -> Bitboard {
    let popped_bit = *bb & bb.wrapping_neg();
    *bb ^= popped_bit;
    popped_bit
}

/// Clears the least significant bit of `bb` and returns the position of it.
pub fn pop_next_square(bb: &mut Bitboard) -> Square {
    let shift = bb.trailing_zeros();
    *bb ^= 1 << shift;
    shift as Square
}

/// Generates the attack set for `piece` on `square` up to and including the
/// given blockers. Includes the edge.
pub fn sliding_attacks<const PIECE: Piece>(square: Square, blockers: Bitboard) -> Bitboard {
    let mut ray = Bitboards::EMPTY;
    if PIECE == Pieces::BISHOP {
        ray |= ray_attack::<{ Directions::NE }>(square, blockers);
        ray |= ray_attack::<{ Directions::SE }>(square, blockers);
        ray |= ray_attack::<{ Directions::SW }>(square, blockers);
        ray |= ray_attack::<{ Directions::NW }>(square, blockers);
    } else {
        ray |= ray_attack::<{ Directions::N }>(square, blockers);
        ray |= ray_attack::<{ Directions::E }>(square, blockers);
        ray |= ray_attack::<{ Directions::S }>(square, blockers);
        ray |= ray_attack::<{ Directions::W }>(square, blockers);
    };
    ray
}

/// Shifts `bb` one square south without wrapping.
pub fn south(bb: Bitboard) -> Bitboard {
    bb >> 8
}

/// Finds the position of the least significant bit of `bb`.
pub fn to_square(bb: Bitboard) -> Square {
    bb.trailing_zeros() as Square
}

/// Generates an attack from `square` in the given direction up to and
/// including the first encountered bit set in `blockers`. `blockers` is
/// assumed not to include `square` itself.
pub fn ray_attack<const DIRECTION: Direction>(mut square: Square, blockers: Bitboard) -> Bitboard {
    let mut attacks = Bitboards::EMPTY;
    // checks if the next square is valid and if the piece can move from the
    // square
    while is_valid::<DIRECTION>(square) && as_bitboard(square) & blockers == 0 {
        square = square.wrapping_add(DIRECTION as usize);
        attacks |= as_bitboard(square);
    }
    attacks
}

/// Shifts `bb` one square west without wrapping.
pub fn west(bb: Bitboard) -> Bitboard {
    (bb >> 1) & !Bitboards::FILE_BB[Files::FILE8]
}

#[cfg(test)]
mod tests {
    use super::ray_attack;
    use crate::defs::{Directions, Squares};

    #[test]
    fn ray_attacks() {
        assert_eq!(
            ray_attack::<{ Directions::N }>(Squares::A1, 0),
            0x0101010101010100
        );
        assert_eq!(
            ray_attack::<{ Directions::N }>(Squares::A1, 0x0100000000000000),
            0x0101010101010100
        );
        assert_eq!(
            ray_attack::<{ Directions::N }>(Squares::A1, 0x0101000000000000),
            0x0001010101010100
        );
        assert_eq!(
            ray_attack::<{ Directions::NE }>(Squares::A1, 0x8040201008040200),
            0x0000000000000200
        );
        assert_eq!(
            ray_attack::<{ Directions::NE }>(Squares::A1, 0x8000000000000000),
            0x8040201008040200
        );
        assert_eq!(
            ray_attack::<{ Directions::NE }>(Squares::A1, 0x0000000000000000),
            0x8040201008040200
        );
        assert_eq!(
            ray_attack::<{ Directions::SE }>(Squares::A8, 0x0002040801020408),
            0x0002000000000000
        );
        assert_eq!(
            ray_attack::<{ Directions::SW }>(Squares::H8, 0x0040201008040201),
            0x0040000000000000
        );
    }
}
