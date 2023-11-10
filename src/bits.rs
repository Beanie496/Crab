use crate::{
    defs::{ Bitboard, Bitboards, Directions, Files, Nums, Square, Ranks },
};

/// Initialises a lookup table with ray attacks for each direction for each
/// square.
pub fn init_ray_attacks(ray_attacks: &mut [[Bitboard; Nums::SQUARES]; Nums::DIRECTIONS]) {
    // north
    {
        let a2_a8 = Bitboards::FILE_BB[Files::FILE1 as usize] ^ 1;
        for square in 0..Nums::SQUARES {
            ray_attacks[Directions::N][square] = a2_a8 << square;
        }
    }
    // north-east
    {
        let mut b2_h8 = 0x8040201008040200u64;
        for file in 0..7 {
            let top_rank = file + 56;
            for square in (file..top_rank).step_by(8) {
                ray_attacks[Directions::NE][square] = b2_h8 << square;
            }
            b2_h8 &= !Bitboards::FILE_BB[7 - file];
        }
    }
    // east
    {
        let mut b1_h1 = Bitboards::RANK_BB[Ranks::RANK1 as usize] ^ 1;
        // no need to loop over the final file
        for file in 0..7 {
            let top_rank = file + 56;
            for square in (file..=top_rank).step_by(8) {
                ray_attacks[Directions::E][square] = b1_h1 << square;
            }
            b1_h1 &= !Bitboards::FILE_BB[7 - file];
        }
    }
    // south-east
    {
        let mut b7_h1 = 0x0002040810204080u64;
        let mut square = 56;
        while b7_h1 != 0 {
            for rank in (0..56).step_by(8) {
                ray_attacks[Directions::SE][square] = b7_h1 >> rank;
                square -= 8;
            }
            b7_h1 <<= 1;
            b7_h1 &= !Bitboards::FILE_BB[Files::FILE1 as usize];
            square += 57;
        }
    }
    // south
    {
        let h7_h1 = Bitboards::FILE_BB[Files::FILE8 as usize] ^ (1 << 63);
        for square in 8..64 {
            ray_attacks[Directions::S][square] = h7_h1 >> (square ^ 63);
        }
    }
    // south-west
    {
        let mut g7_a1 = 0x0040201008040201u64;
        let mut square = 63;
        while g7_a1 != 0 {
            for file in (0..56).step_by(8) {
                ray_attacks[Directions::SW][square] = g7_a1 >> file;
                square -= 8;
            }
            g7_a1 >>= 1;
            g7_a1 &= !Bitboards::FILE_BB[Files::FILE8 as usize];
            square += 55;
        }
    }
    // west
    {
        let mut square = 7;
        let mut h1_a1 = Bitboards::RANK_BB[Ranks::RANK1 as usize] ^ 1 << square;
        while h1_a1 != 0 {
            for rank in (0..=56).step_by(8) {
                ray_attacks[Directions::W][square] = h1_a1 << rank;
                square += 8;
            }
            h1_a1 >>= 1;
            square -= 65;
        }
    }
    // north-west
    {
        let mut square = 7;
        let mut g2_a8 = 0x0102040810204000u64;
        while g2_a8 != 0 {
            for rank in (0..56).step_by(8) {
                ray_attacks[Directions::NW][square] = g2_a8 << rank;
                square += 8;
            }
            g2_a8 >>= 1;
            g2_a8 &= !Bitboards::FILE_BB[Files::FILE8 as usize];
            square -= 57;
        }
    }
}

/// Returns a given bitboard shifted one square east without wrapping.
pub fn east(bb: Bitboard) -> Bitboard {
    (bb << 1) & !Bitboards::FILE_BB[Files::FILE1 as usize]
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
    (bb >> 1) & !Bitboards::FILE_BB[Files::FILE8 as usize]
}

#[cfg(test)]
mod tests {
    use crate::defs::{ Bitboards, Directions, Nums, Squares };
    use super::init_ray_attacks;

    #[test]
    fn ray_attacks() {
        let mut ray_attacks = [[Bitboards::EMPTY; Nums::SQUARES]; Nums::DIRECTIONS];
        init_ray_attacks(&mut ray_attacks);
        assert_eq!(ray_attacks[Directions::N][Squares::A1], 0x0101010101010100);
        assert_eq!(ray_attacks[Directions::N][Squares::H1], 0x8080808080808000);
        assert_eq!(ray_attacks[Directions::N][Squares::E4], 0x1010101000000000);
        assert_eq!(ray_attacks[Directions::N][Squares::D5], 0x0808080000000000);
        assert_eq!(ray_attacks[Directions::N][Squares::A8], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::N][Squares::H8], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::NE][Squares::A1], 0x8040201008040200);
        assert_eq!(ray_attacks[Directions::NE][Squares::H1], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::NE][Squares::E4], 0x0080402000000000);
        assert_eq!(ray_attacks[Directions::NE][Squares::D5], 0x4020100000000000);
        assert_eq!(ray_attacks[Directions::NE][Squares::A8], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::NE][Squares::H8], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::E][Squares::A1], 0x00000000000000fe);
        assert_eq!(ray_attacks[Directions::E][Squares::H1], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::E][Squares::E4], 0x00000000e0000000);
        assert_eq!(ray_attacks[Directions::E][Squares::D5], 0x000000f000000000);
        assert_eq!(ray_attacks[Directions::E][Squares::A8], 0xfe00000000000000);
        assert_eq!(ray_attacks[Directions::E][Squares::H8], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::SE][Squares::A1], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::SE][Squares::H1], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::SE][Squares::E4], 0x0000000000204080);
        assert_eq!(ray_attacks[Directions::SE][Squares::D5], 0x0000000010204080);
        assert_eq!(ray_attacks[Directions::SE][Squares::A8], 0x0002040810204080);
        assert_eq!(ray_attacks[Directions::SE][Squares::H8], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::S][Squares::A1], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::S][Squares::H1], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::S][Squares::E4], 0x0000000000101010);
        assert_eq!(ray_attacks[Directions::S][Squares::D5], 0x0000000008080808);
        assert_eq!(ray_attacks[Directions::S][Squares::A8], 0x0001010101010101);
        assert_eq!(ray_attacks[Directions::S][Squares::H8], 0x0080808080808080);
        assert_eq!(ray_attacks[Directions::SW][Squares::A1], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::SW][Squares::H1], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::SW][Squares::E4], 0x0000000000080402);
        assert_eq!(ray_attacks[Directions::SW][Squares::D5], 0x0000000004020100);
        assert_eq!(ray_attacks[Directions::SW][Squares::A8], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::SW][Squares::H8], 0x0040201008040201);
        assert_eq!(ray_attacks[Directions::W][Squares::A1], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::W][Squares::H1], 0x000000000000007f);
        assert_eq!(ray_attacks[Directions::W][Squares::E4], 0x000000000f000000);
        assert_eq!(ray_attacks[Directions::W][Squares::D5], 0x0000000700000000);
        assert_eq!(ray_attacks[Directions::W][Squares::A8], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::W][Squares::H8], 0x7f00000000000000);
        assert_eq!(ray_attacks[Directions::NW][Squares::A1], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::NW][Squares::H1], 0x0102040810204000);
        assert_eq!(ray_attacks[Directions::NW][Squares::E4], 0x0102040800000000);
        assert_eq!(ray_attacks[Directions::NW][Squares::D5], 0x0102040000000000);
        assert_eq!(ray_attacks[Directions::NW][Squares::A8], 0x0000000000000000);
        assert_eq!(ray_attacks[Directions::NW][Squares::H8], 0x0000000000000000);
    }
}
