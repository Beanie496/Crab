use crate::{
    defs::{Bitboard, Bitboards, Direction, File, Files, Rank, Square},
    util::is_valid,
};

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

/// Shifts `bb` one square north without wrapping.
pub fn north(bb: Bitboard) -> Bitboard {
    bb << 8
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
pub fn ray_attack(mut square: Square, direction: Direction, blockers: Bitboard) -> Bitboard {
    let mut attacks = Bitboards::EMPTY;
    // checks if the next square is valid and if the piece can move from the
    // square
    while is_valid(square, direction) && as_bitboard(square) & blockers == 0 {
        square = square.wrapping_add(direction as usize);
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
            ray_attack(Squares::A1, Directions::N, 0),
            0x0101010101010100
        );
        assert_eq!(
            ray_attack(Squares::A1, Directions::N, 0x0100000000000000),
            0x0101010101010100
        );
        assert_eq!(
            ray_attack(Squares::A1, Directions::N, 0x0101000000000000),
            0x0001010101010100
        );
        assert_eq!(
            ray_attack(Squares::A1, Directions::NE, 0x8040201008040200),
            0x0000000000000200
        );
        assert_eq!(
            ray_attack(Squares::A1, Directions::NE, 0x8000000000000000),
            0x8040201008040200
        );
        assert_eq!(
            ray_attack(Squares::A1, Directions::NE, 0x0000000000000000),
            0x8040201008040200
        );
        assert_eq!(
            ray_attack(Squares::A8, Directions::SE, 0x0002040801020408),
            0x0002000000000000
        );
        assert_eq!(
            ray_attack(Squares::H8, Directions::SW, 0x0040201008040201),
            0x0040000000000000
        );
    }
}
