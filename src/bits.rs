use crate::defs::{ Bitboard, Bitboards, Square };

/// Returns a given bitboard shifted one square east without wrapping.
pub fn east(bb: Bitboard) -> Bitboard {
    (bb << 1) & !Bitboards::FILE1_BB
}

/// Returns a given bitboard shifted one square north without wrapping.
pub fn north(bb: Bitboard) -> Bitboard {
    bb << 8
}

/// Clears the LSB of a given bitboard and returns it.
pub fn pop_lsb(bb: &mut Bitboard) -> Bitboard {
    let popped_bit = *bb & bb.wrapping_neg();
    *bb ^= popped_bit;
    popped_bit
}

/// Clears the LSB of a given bitboard and returns the 0-indexed position of that bit.
pub fn pop_next_square(bb: &mut Bitboard) -> Square {
    let shift = bb.trailing_zeros();
    *bb ^= 1u64 << shift;
    shift as Square
}

/// Returns a given bitboard shifted one square south without wrapping.
pub fn south(bb: Bitboard) -> Bitboard {
    bb >> 8
}

/// Returns the square of the LSB of a given bitboard: 0x0000000000000001 -> 0,
/// 0x0000000000000010 = 4, etc.
pub fn square_of(bb: Bitboard) -> Square {
    bb.trailing_zeros() as Square
}

/// Converts a Bitboard into a Square. This should only be done on Bitboards
/// that have a single bit set.
pub fn to_square(bb: Bitboard) -> Square {
    bb.trailing_zeros() as Square
}

/// Returns a given bitboard shifted one square west without wrapping.
pub fn west(bb: Bitboard) -> Bitboard {
    (bb >> 1) & !Bitboards::FILE8_BB
}
