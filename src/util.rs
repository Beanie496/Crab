use crate::defs::{ Bitboard, Bitboards, Files, Move, Piece, Ranks, Side, Square };

/// Returns a Move given a start square, end square, piece and side.
pub fn create_move(start: Square, end: Square, piece: Piece, side: Side) -> Move {
    start as Move
        | ((end as Move) << 6)
        | ((piece as Move) << 12)
        | ((side as Move) << 15)
}

/// Returns a tuple of a start square, end square, piece and side given a Move.
pub fn decompose_move(mv: Move) -> (Square, Square, Piece, Side) {
    let start = mv & 0x3f;
    let end = (mv >> 6) & 0x3f;
    let piece = (mv >> 12) & 0x7;
    let side = (mv >> 15) & 0x1;
    (start as Square, end as Square, piece as Piece, side as Side)
}

/// Returns a given bitboard shifted one square east without wrapping.
pub fn east(bb: Bitboard) -> Bitboard {
    (bb << 1) & !Bitboards::FILE1_BB
}

/// Clears the LSB and returns it.
pub fn pop_lsb(bb: &mut Bitboard) -> Bitboard {
    let popped_bit = *bb & bb.wrapping_neg();
    *bb ^= popped_bit;
    popped_bit
}

/// Clears the LSB and returns the 0-indexed position of that bit.
pub fn pop_next_square(bb: &mut Bitboard) -> Square {
    let shift = bb.trailing_zeros();
    *bb ^= 1u64 << shift;
    shift as Square
}

// Allowed dead code because this is occasionally useful for debugging.
#[allow(dead_code)]
/// Pretty prints a given bitboard.
pub fn pretty_print(board: Bitboard) {
    for r in (Ranks::RANK1..=Ranks::RANK8).rev() {
        for f in Files::FILE1..=Files::FILE8 {
            if board & (1 << (r * 8 + f)) != 0 {
                print!("1 ");
            } else {
                print!("0 ");
            }
        }
        println!();
    }
    println!();
}

/// Returns the square of the LSB of a bitboard: 0x0000000000000001 -> 0,
/// 0x0000000000000010 = 4, etc.
pub fn square_of(bb: Bitboard) -> Square {
    bb.trailing_zeros() as Square
}

/// Returns a string representation of a move.
pub fn stringify_move(mv: Move) -> String {
    let start = mv & 0x3f;
    let end = (mv >> 6) & 0x3f;
    let mut ret = String::with_capacity(4);
    ret += &stringify_square(start as Square);
    ret += &stringify_square(end as Square);
    ret
}

/// Returns a string representation of a square.
pub fn stringify_square(sq: Square) -> String {
    let mut ret = String::with_capacity(2);
    ret.push((b'a' + (sq as u8 & 7)) as char);
    ret.push((b'1' + (sq as u8 >> 3)) as char);
    ret
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
    use super::create_move;
    use crate::defs::{ Pieces, Sides, Squares };

    #[test]
    fn create_move_works() {
        // these asserts will use magic values known to be correct
        assert_eq!(
            create_move(Squares::A1, Squares::H8, Pieces::KNIGHT, Sides::BLACK),
            (63 << 6) | (1 << 12) | (1 << 15),
        );
        assert_eq!(
            create_move(Squares::A8, Squares::H1, Pieces::KING, Sides::WHITE),
            (56 << 6) | 7 | (5 << 12),
        );
    }
}
