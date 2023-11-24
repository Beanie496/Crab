use crate::{
    defs::{Bitboard, Bitboards, Direction, Directions, Files, Piece, Pieces, Ranks, Square},
    util::{as_bitboard, file_of, is_valid, rank_of},
};

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
