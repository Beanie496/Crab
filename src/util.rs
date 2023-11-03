use crate::defs::{ Bitboard, Files, Move, Piece, Ranks, Side, Square };

/// Returns a Move given a start square, end square, piece and side.
pub fn create_move(start: Square, end: Square, piece: Piece, side: Side) -> Move {
    start as Move
        | ((end as Move) << 6)
        | ((piece as Move) << 12)
        | ((side as Move) << 15)
}

/// Returns a tuple of a start square, end square, piece and side given a Move.
pub fn decompose_move(mv: Move) -> (Square, Square, Piece, Side) {
    let start = (mv & 0x3f) as u8;
    let end = ((mv >> 6) & 0x3f) as u8;
    let piece = ((mv >> 12) & 0x7) as u8;
    let side = ((mv >> 15) & 0x1) as u8;
    (start, end, piece, side)
}

/// Clears the LSB and returns it.
pub fn pop_lsb(bb: &mut Bitboard) -> Bitboard {
    let popped_bit = *bb & bb.wrapping_neg();
    *bb ^= popped_bit;
    popped_bit
}

/// Clears the LSB and returns the 0-indexed position of that bit.
pub fn pop_next_square(bb: &mut Bitboard) -> u8 {
    let shift: u8 = bb.trailing_zeros() as u8;
    *bb ^= 1u64 << shift;
    shift
}

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

/// Returns a string representation of a move.
pub fn stringify_move(mv: Move) -> String {
    let start = (mv as u8) & 0x3f;
    let end = ((mv >> 6) as u8) & 0x3f;
    let mut ret = String::with_capacity(4);
    ret += &stringify_square(start);
    ret += &stringify_square(end);
    ret
}

/// Returns a string representation of a square.
pub fn stringify_square(sq: Square) -> String {
    let mut ret = String::with_capacity(2);
    ret.push((b'a' + (sq & 7)) as char);
    ret.push((b'1' + (sq >> 3)) as char);
    ret
}

/// Converts a Bitboard into a Square. This should only be done on Bitboards
/// that have a single bit set.
pub fn to_square(bb: Bitboard) -> Square {
    bb.trailing_zeros() as Square
}

#[cfg(test)]
mod tests {
    use super::{ create_move, pop_lsb };
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
