use crate::defs::{Bitboard, File, Rank, Square};

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

/// Finds the position of the least significant bit of `bb`.
pub fn to_square(bb: Bitboard) -> Square {
    bb.trailing_zeros() as Square
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
