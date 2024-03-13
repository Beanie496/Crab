use crate::{
    bitboard::Bitboard,
    defs::{Direction, File, PieceType, Rank, Square},
    movegen::magic::MAX_BLOCKERS,
};

/// Generates all combinations of attacks from `square` and puts them in
/// `attacks`. It starts with a full blocker board that goes from the
/// square to the edge exclusive and uses the Carry-Rippler trick to
/// generate each subsequent attack.
pub fn gen_all_sliding_attacks<const PIECE: u8>(
    square: Square,
    attacks: &mut [Bitboard; MAX_BLOCKERS],
) {
    let excluded_ranks_bb = (Bitboard::file_bb(File::FILE1) | Bitboard::file_bb(File::FILE8))
        & !Bitboard::file_bb(File::from(square));
    let excluded_files_bb = (Bitboard::rank_bb(Rank::RANK1) | Bitboard::rank_bb(Rank::RANK8))
        & !Bitboard::rank_bb(Rank::from(square));
    let edges = excluded_ranks_bb | excluded_files_bb;
    let mask = sliding_attacks::<PIECE>(square, Bitboard::empty()) & !edges;

    let mut blockers = mask;
    let mut first_empty = 0;
    while !blockers.is_empty() {
        attacks[first_empty] = sliding_attacks::<PIECE>(square, blockers);
        first_empty += 1;
        blockers = Bitboard(blockers.0.wrapping_sub(1)) & mask;
    }
    attacks[first_empty] = sliding_attacks::<PIECE>(square, Bitboard::empty());
}

/// Checks if `square` can go in the given direction.
pub fn is_valid<const DIRECTION: i8>(square: Square) -> bool {
    // I want to lose the sign and intentially overflow `square`, since that
    // would have the same effect as just adding the i8 to the u8
    let dest = Square(square.0.wrapping_add(DIRECTION as u8));
    // credit to Stockfish, as I didn't come up with this code.
    // It checks if `square` is still within the board, and if it is, it checks
    // if it hasn't wrapped (because if it has wrapped, the distance will be
    // larger than 2).
    dest.is_valid() && square.horizontal_distance(dest) <= 1
}

/// Generates an attack from `square` in the given direction up to and
/// including the first encountered bit set in `blockers`. `blockers` is
/// assumed not to include `square` itself.
pub fn ray_attack<const DIRECTION: i8>(mut square: Square, blockers: Bitboard) -> Bitboard {
    let mut attacks = Bitboard::empty();
    // checks if the next square is valid and if the piece can move from the
    // square
    while is_valid::<DIRECTION>(square) && (Bitboard::from(square) & blockers).is_empty() {
        // I want to lose the sign and intentially overflow `square`, since
        // that would have the same effect as just adding the i8 to the u8
        square = Square(square.0.wrapping_add(DIRECTION as u8));
        attacks |= Bitboard::from(square);
    }
    attacks
}

/// Generates the attack set for `piece` on `square` up to and including the
/// given blockers. Includes the edge.
pub fn sliding_attacks<const PIECE: u8>(square: Square, blockers: Bitboard) -> Bitboard {
    let piece = PieceType(PIECE);
    let mut ray = Bitboard::empty();
    if piece == PieceType::BISHOP {
        ray |= ray_attack::<{ Direction::NE.0 }>(square, blockers);
        ray |= ray_attack::<{ Direction::SE.0 }>(square, blockers);
        ray |= ray_attack::<{ Direction::SW.0 }>(square, blockers);
        ray |= ray_attack::<{ Direction::NW.0 }>(square, blockers);
    } else {
        ray |= ray_attack::<{ Direction::N.0 }>(square, blockers);
        ray |= ray_attack::<{ Direction::E.0 }>(square, blockers);
        ray |= ray_attack::<{ Direction::S.0 }>(square, blockers);
        ray |= ray_attack::<{ Direction::W.0 }>(square, blockers);
    };
    ray
}