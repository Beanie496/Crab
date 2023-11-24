use crate::{
    defs::{
        Bitboard, Bitboards, Direction, Directions, Files, Move, Piece, Pieces, Ranks, Side,
        Square, Squares,
    },
    util::{as_bitboard, file_of, rank_of},
};

/// Creates a [`Move`] given a start square, end square, piece and side.
pub fn create_move<const IS_WHITE: bool, const PIECE: Piece>(start: Square, end: Square) -> Move {
    start as Move | (end as Move) << 6 | (PIECE as Move) << 12 | (IS_WHITE as Move) << 15
}

/// Turns a [`Move`] into its components: start square, end square, piece and
/// side, in that order.
pub fn decompose_move(mv: Move) -> (Square, Square, Piece, Side) {
    let start = mv & 0x3f;
    let end = (mv >> 6) & 0x3f;
    let piece = (mv >> 12) & 0x7;
    let side = (mv >> 15) & 0x1;
    (start as Square, end as Square, piece as Piece, side as Side)
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

/// Finds the horizontal distance between `square_1` and `square_2`
pub fn horizontal_distance(square_1: Square, square_2: Square) -> u8 {
    (file_of(square_1) as i8 - file_of(square_2) as i8).unsigned_abs()
}

/// Checks if `square` can go in the given direction.
pub fn is_valid<const DIRECTION: Direction>(square: Square) -> bool {
    let dest = square.wrapping_add(DIRECTION as usize);
    // credit to Stockfish, as I didn't come up with this code.
    // It checks if `square` is still within the board, and if it is, it checks
    // if it hasn't wrapped (because if it has wrapped, the distance will be
    // larger than 2).
    is_valid_square(dest) && horizontal_distance(square, dest) <= 1
}

/// Checks if `square` is within the board.
pub fn is_valid_square(square: Square) -> bool {
    // `square` is a usize so it can't be less than 0.
    square <= Squares::H8
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

/// Shifts `bb` one square west without wrapping.
pub fn west(bb: Bitboard) -> Bitboard {
    (bb >> 1) & !Bitboards::FILE_BB[Files::FILE8]
}

#[cfg(test)]
mod tests {
    use super::{create_move, decompose_move};
    use crate::defs::{Pieces, Sides, Squares};

    #[test]
    fn create_move_works() {
        // these asserts will use magic values known to be correct
        assert_eq!(
            create_move::<false, { Pieces::KNIGHT }>(Squares::A1, Squares::H8),
            63 << 6 | 1 << 12,
        );
        assert_eq!(
            create_move::<true, { Pieces::KING }>(Squares::A8, Squares::H1),
            56 | 7 << 6 | 5 << 12 | 1 << 15,
        );
    }

    #[test]
    fn decompose_move_works() {
        assert_eq!(
            decompose_move(63 << 6 | 1 << 12 | 1 << 15),
            (Squares::A1, Squares::H8, Pieces::KNIGHT, Sides::WHITE),
        );
        assert_eq!(
            decompose_move(56 | 7 << 6 | 5 << 12),
            (Squares::A8, Squares::H1, Pieces::KING, Sides::BLACK),
        );
    }
}
