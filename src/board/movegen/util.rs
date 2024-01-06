use crate::{
    board::Move,
    defs::{Bitboard, Direction, File, Piece, Rank, Side, Square, PIECE_CHARS},
};

impl Move {
    /// Converts `mv` into its string representation.
    pub fn stringify(&self) -> String {
        let start = Square::new(((self.mv & Self::START_MASK) >> Self::START_SHIFT) as u8);
        let end = Square::new(((self.mv & Self::END_MASK) >> Self::END_SHIFT) as u8);
        let mut ret = String::with_capacity(5);
        ret += &start.stringify();
        ret += &end.stringify();
        if self.is_promotion() {
            // we want the lowercase letter here
            ret.push(PIECE_CHARS[Side::BLACK.to_index()][self.promotion_piece().to_index()]);
        }
        ret
    }
}

/// Generates all combinations of attacks from `square` and puts them in
/// `attacks`. It starts with a full blocker board that goes from the
/// square to the edge exclusive and uses the Carry-Rippler trick to
/// generate each subsequent attack.
pub fn gen_all_sliding_attacks<const PIECE: u8>(
    square: Square,
    attacks: &mut [Bitboard; crate::board::movegen::magic::MAX_BLOCKERS],
) {
    // FIXME: jeez how many times is this code going to crop up
    let edges = ((Bitboard::FILE_BB[File::FILE1.to_index()]
        | Bitboard::FILE_BB[File::FILE8.to_index()])
        & !Bitboard::FILE_BB[square.file_of().to_index()])
        | ((Bitboard::RANK_BB[Rank::RANK1.to_index()] | Bitboard::RANK_BB[Rank::RANK8.to_index()])
            & !Bitboard::RANK_BB[square.rank_of().to_index()]);
    let mask = sliding_attacks::<PIECE>(square, Bitboard::new(0)) & !edges;

    let mut blockers = mask;
    let mut first_empty = 0;
    while blockers != Bitboard::new(0) {
        attacks[first_empty] = sliding_attacks::<PIECE>(square, blockers);
        first_empty += 1;
        blockers = Bitboard::new(blockers.inner().wrapping_sub(1)) & mask;
    }
    attacks[first_empty] = sliding_attacks::<PIECE>(square, Bitboard::new(0));
}

/// Checks if `square` can go in the given direction.
pub fn is_valid<const DIRECTION: i8>(square: Square) -> bool {
    let dest = Square::new(square.inner().wrapping_add(DIRECTION as u8));
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
    let mut attacks = Bitboard::EMPTY;
    // checks if the next square is valid and if the piece can move from the
    // square
    while is_valid::<DIRECTION>(square) && square.to_bitboard() & blockers == Bitboard::new(0) {
        square = Square::new(square.inner().wrapping_add(DIRECTION as u8));
        attacks |= square.to_bitboard();
    }
    attacks
}

/// Generates the attack set for `piece` on `square` up to and including the
/// given blockers. Includes the edge.
pub fn sliding_attacks<const PIECE: u8>(square: Square, blockers: Bitboard) -> Bitboard {
    let piece = Piece::new(PIECE);
    let mut ray = Bitboard::EMPTY;
    if piece == Piece::BISHOP {
        ray |= ray_attack::<{ Direction::NE.inner() }>(square, blockers);
        ray |= ray_attack::<{ Direction::SE.inner() }>(square, blockers);
        ray |= ray_attack::<{ Direction::SW.inner() }>(square, blockers);
        ray |= ray_attack::<{ Direction::NW.inner() }>(square, blockers);
    } else {
        ray |= ray_attack::<{ Direction::N.inner() }>(square, blockers);
        ray |= ray_attack::<{ Direction::E.inner() }>(square, blockers);
        ray |= ray_attack::<{ Direction::S.inner() }>(square, blockers);
        ray |= ray_attack::<{ Direction::W.inner() }>(square, blockers);
    };
    ray
}
