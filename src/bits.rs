use crate::{
    defs::{ Bitboard, Bitboards, Direction, Square },
    util::file_of,
};

/// Returns a bitboard with a ray from the given square to the edge, excluding
/// the square itself.
pub fn ray(direction: Direction, square: Square) -> Bitboard {
    /* North and south go straight up and down, so simply shifting them by the
     * square number will cause the unwanted bits to over/underflow.
     * East subracts the bit of the square from the bit of the highest file in
     * that rank to get 1's from the square to the edge (excluding the edge),
     * then shifts left by one to give the correct bits.
     * West subtracts the bit of the lowest file in the rank from the bit of
     * the square, which immediately gives the correct bits.
     * The 4 diagonal directions follow the same principle:
     * If you start with the line of bits in the direction and rotate left by
     * the correct amount, only the bits in one of the quadrants relative to
     * the square need to be kept. The bits in the other three quadrants can be
     * removed by &-ing the board with the bits on the correct files and on
     * the correct ranks.
     * ```
     * direction: SW
     * 0 0 0 0 0 0 0 S
     * 0 0 0 0 0 0 1 0
     * 0 0 0 0 0 1 0 0
     * 0 0 0 0 1 0 0 0
     * 0 0 0 1 0 0 0 0
     * 0 0 1 0 0 0 0 0
     * 0 1 0 0 0 0 0 0
     * 1 0 0 0 0 0 0 0
     * square: E4
     * g g g g g g g g
     * g g g g g g g g
     * g g g g g g g g
     * g g g g g g g g
     * 0 0 0 0 S g g g
     * 0 0 0 1 0 g g g
     * 0 0 1 0 0 g g g
     * 0 1 0 0 0 0 g g
     * ```
     * Getting the files on the left is a lot easier than the right, so for all
     * 4 diagonal directions, the files on the left are calculated and negated
     * if necessary. The ranks above or below are very easy to calculate - just
     * a full bitboard shifted left by the square or `(1 << square) - 1`
     * respectively. Then just & the correct quadrant with the correct
     * upper/lower ranks and left/right files.
     */
    match direction {
        Direction::N => {
            (Bitboards::FILE1_BB ^ 1) << square
        }
        Direction::NE => {
            // This is the one quadrant where a rotate isn't needed, so no need
            // to calculate the lower ranks.
            let file = file_of(square);
            let mut left_files = (1 << file) - 1;
            left_files |= left_files << 8;
            left_files |= left_files << 16;
            left_files |= left_files << 32;
            let b2_h8_diag = 0x8040201008040200;
            (b2_h8_diag << square) & !left_files
        }
        Direction::E => {
            let square_bb = 1 << square;
            let highest_bit_of_rank = 1 << (square | 7);
            (highest_bit_of_rank - square_bb) << 1
        }
        Direction::SE => {
            let lower_ranks = (1 << square) - 1;
            let file = file_of(square);
            let mut left_files = (1 << file) - 1;
            left_files |= left_files << 8;
            left_files |= left_files << 16;
            left_files |= left_files << 32;
            let b7_h1_diag = 0x0002040810204080u64;
            b7_h1_diag.rotate_left(64 + square as u32 - 56) & !left_files & lower_ranks
        }
        Direction::S => {
            (Bitboards::FILE8_BB ^ 0x8000000000000000) >> (square ^ 63)
        }
        Direction::SW => {
            let lower_ranks = (1 << square) - 1;
            let file = file_of(square);
            let mut left_files = (1 << file) - 1;
            left_files |= left_files << 8;
            left_files |= left_files << 16;
            left_files |= left_files << 32;
            let a1_g7_diag = 0x0040201008040201u64;
            a1_g7_diag.rotate_left(64 + square as u32 - 63) & left_files & lower_ranks
        }
        Direction::W => {
            let square_bb = 1 << square;
            let lowest_bit = 1 << (square & 56);
            square_bb - lowest_bit
        }
        Direction::NW => {
            let upper_ranks = Bitboards::FULL << square;
            let file = file_of(square);
            let mut left_files = (1 << file) - 1;
            left_files |= left_files << 8;
            left_files |= left_files << 16;
            left_files |= left_files << 32;
            let a8_g2_diag = 0x0102040810204000u64;
            a8_g2_diag.rotate_left(64 + square as u32 - 7) & left_files & upper_ranks
        }
    }
}

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
/// 0x0000000000000010 -> 4, etc.
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

#[cfg(test)]
mod tests {
    use crate::defs::{ Direction, Squares };
    use super::ray;

    #[test]
    fn north() {
        assert_eq!(ray(Direction::N, Squares::A1), 0x0101010101010100);
        assert_eq!(ray(Direction::N, Squares::H1), 0x8080808080808000);
        assert_eq!(ray(Direction::N, Squares::E4), 0x1010101000000000);
        assert_eq!(ray(Direction::N, Squares::D5), 0x0808080000000000);
        assert_eq!(ray(Direction::N, Squares::A8), 0x0000000000000000);
        assert_eq!(ray(Direction::N, Squares::H8), 0x0000000000000000);
    }

    #[test]
    fn north_east() {
        assert_eq!(ray(Direction::NE, Squares::A1), 0x8040201008040200);
        assert_eq!(ray(Direction::NE, Squares::H1), 0x0000000000000000);
        assert_eq!(ray(Direction::NE, Squares::E4), 0x0080402000000000);
        assert_eq!(ray(Direction::NE, Squares::D5), 0x4020100000000000);
        assert_eq!(ray(Direction::NE, Squares::A8), 0x0000000000000000);
        assert_eq!(ray(Direction::NE, Squares::H8), 0x0000000000000000);
    }

    #[test]
    fn east() {
        assert_eq!(ray(Direction::E, Squares::A1), 0x00000000000000fe);
        assert_eq!(ray(Direction::E, Squares::H1), 0x0000000000000000);
        assert_eq!(ray(Direction::E, Squares::E4), 0x00000000e0000000);
        assert_eq!(ray(Direction::E, Squares::D5), 0x000000f000000000);
        assert_eq!(ray(Direction::E, Squares::A8), 0xfe00000000000000);
        assert_eq!(ray(Direction::E, Squares::H8), 0x0000000000000000);
    }

    #[test]
    fn south_east() {
        assert_eq!(ray(Direction::SE, Squares::A1), 0x0000000000000000);
        assert_eq!(ray(Direction::SE, Squares::H1), 0x0000000000000000);
        assert_eq!(ray(Direction::SE, Squares::E4), 0x0000000000204080);
        assert_eq!(ray(Direction::SE, Squares::D5), 0x0000000010204080);
        assert_eq!(ray(Direction::SE, Squares::A8), 0x0002040810204080);
        assert_eq!(ray(Direction::SE, Squares::H8), 0x0000000000000000);
    }

    #[test]
    fn south() {
        assert_eq!(ray(Direction::S, Squares::A1), 0x0000000000000000);
        assert_eq!(ray(Direction::S, Squares::H1), 0x0000000000000000);
        assert_eq!(ray(Direction::S, Squares::E4), 0x0000000000101010);
        assert_eq!(ray(Direction::S, Squares::D5), 0x0000000008080808);
        assert_eq!(ray(Direction::S, Squares::A8), 0x0001010101010101);
        assert_eq!(ray(Direction::S, Squares::H8), 0x0080808080808080);
    }

    #[test]
    fn south_west() {
        assert_eq!(ray(Direction::SW, Squares::A1), 0x0000000000000000);
        assert_eq!(ray(Direction::SW, Squares::H1), 0x0000000000000000);
        assert_eq!(ray(Direction::SW, Squares::E4), 0x0000000000080402);
        assert_eq!(ray(Direction::SW, Squares::D5), 0x0000000004020100);
        assert_eq!(ray(Direction::SW, Squares::A8), 0x0000000000000000);
        assert_eq!(ray(Direction::SW, Squares::H8), 0x0040201008040201);
    }

    #[test]
    fn west() {
        assert_eq!(ray(Direction::W, Squares::A1), 0x0000000000000000);
        assert_eq!(ray(Direction::W, Squares::H1), 0x000000000000007f);
        assert_eq!(ray(Direction::W, Squares::E4), 0x000000000f000000);
        assert_eq!(ray(Direction::W, Squares::D5), 0x0000000700000000);
        assert_eq!(ray(Direction::W, Squares::A8), 0x0000000000000000);
        assert_eq!(ray(Direction::W, Squares::H8), 0x7f00000000000000);
    }

    #[test]
    fn north_west() {
        assert_eq!(ray(Direction::NW, Squares::A1), 0x0000000000000000);
        assert_eq!(ray(Direction::NW, Squares::H1), 0x0102040810204000);
        assert_eq!(ray(Direction::NW, Squares::E4), 0x0102040800000000);
        assert_eq!(ray(Direction::NW, Squares::D5), 0x0102040000000000);
        assert_eq!(ray(Direction::NW, Squares::A8), 0x0000000000000000);
        assert_eq!(ray(Direction::NW, Squares::H8), 0x0000000000000000);
    }
}
