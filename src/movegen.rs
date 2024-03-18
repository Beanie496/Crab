use std::fmt::{self, Display, Formatter};

use crate::{
    bitboard::Bitboard,
    board::Board,
    defs::{File, MoveType, PieceType, Rank, Side, Square},
    out_of_bounds_is_unreachable,
    util::Stack,
};
use magic::{Magic, BISHOP_MAGICS, MAX_BLOCKERS, ROOK_MAGICS};
use util::{gen_all_sliding_attacks, sliding_attacks};

/// Items related to magic bitboards.
pub mod magic;
/// Useful functions for move generation specifically.
mod util;

/// Contains lookup tables for each piece.
pub struct Lookup {
    /// The pawn attack table. `pawn_attacks[side][square] == attack bitboard
    /// for that square`.
    pawn_attacks: [[Bitboard; Square::TOTAL]; Side::TOTAL],
    /// The knight attack table. `knight_attacks[square] == attack bitboard for
    /// that square`.
    knight_attacks: [Bitboard; Square::TOTAL],
    /// The king attack table. `king_attacks[square] == attack bitboard for
    /// that square`.
    king_attacks: [Bitboard; Square::TOTAL],
    /// The magic lookup table for rooks and bishops.
    ///
    /// The rook attacks are before all the bishop attacks. It uses the 'fancy'
    /// approach. See <https://www.chessprogramming.org/Magic_Bitboards>.
    magic_table: [Bitboard; ROOK_SIZE + BISHOP_SIZE],
    /// The (wrapped) magic numbers for the bishop. One per square. See
    /// <https://www.chessprogramming.org/Magic_Bitboards>.
    bishop_magics: [Magic; Square::TOTAL],
    /// The (wrapped) magic numbers for the rook. One per square. See
    /// <https://www.chessprogramming.org/Magic_Bitboards>.
    rook_magics: [Magic; Square::TOTAL],
}

/// A wrapper for a move and associated methods.
///
/// Order is important here, which is why I've added the `repr` attribute -
/// swapping the order of the fields, or swapping the squares, or both, will
/// result in a slowdown.
///
/// If `is_castling`, the extra bits will be the rook offset from the king dest
/// square, plus 2 (to fit in the 2 bits). If `is_promotion`, the extra bits
/// will be the promotion piece: Knight == `0b00`, bishop == `0b01`, etc.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Move {
    /// Contains: start square (0-63), in `0b00XX_XXXX`; and flags, in
    /// `0bXX00_0000`.
    lower: u8,
    /// Contains: end square (0-63), in `0b00XX_XXXX`; and extra bits, in
    /// `0bXX00_0000`.
    upper: u8,
}

/// An stack of [`Move`]s.
pub type Moves = Stack<Move, MAX_LEGAL_MOVES>;

/// The number of bitboards required to store all bishop attacks, where each
/// element corresponds to one permutation of blockers. (This means some
/// elements will be duplicates, as different blockers can have the same
/// attacks.) Repeated once per quadrant: `2.pow(6)` blocker permutations for
/// the corner, `2.pow(5)` for each non-corner edge and each square adjacent to
/// an edge, `2.pow(7)` for the squares adjacent or diagonal to a corner and
/// `2.pow(9)` for the centre.
const BISHOP_SIZE: usize = 5_248;
/// The number of bitboards required to store all rook attacks, where each
/// element corresponds to one permutation of blockers. (This means some
/// elements will be duplicates, as different blockers can have the same
/// attacks.) There are `2.pow(12)` blocker permutations for each corner,
/// `2.pow(11)` for each non-corner edge and `2.pow(10)` for all others.
const ROOK_SIZE: usize = 102_400;
/// Maximum number of legal moves that can be reached in a standard chess game.
///
/// Example: `R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 w - - 0 1`
pub const MAX_LEGAL_MOVES: usize = 218;
/// The lookup tables used at runtime.
// initialised at runtime
pub static mut LOOKUPS: Lookup = Lookup::empty();

impl Display for Move {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let start = self.start();
        let end = self.end();
        if self.is_promotion() {
            // we want the lowercase letter here
            write!(f, "{start}{end}{}", char::from(self.promotion_piece()))
        } else {
            write!(f, "{start}{end}")
        }
    }
}

impl Move {
    /// Flag for castling.
    const CASTLING: u8 = 0b0100_0000;
    /// Flag for en passant.
    const EN_PASSANT: u8 = 0b1000_0000;
    /// Flag for promotion.
    const PROMOTION: u8 = 0b1100_0000;
    /// No flags.
    const NORMAL: u8 = 0b0000_0000;
    /// Mask for the start square.
    const SQUARE_MASK: u8 = 0b11_1111;
    /// Mask for the flags. They do not need a shift because they simply need
    /// to be set or unset.
    const FLAG_MASK: u8 = 0b1100_0000;
    /// Shift for the promotion piece. It does not need a mask because shifting
    /// already removes unwanted bits.
    const EXTRA_BITS_SHIFT: usize = 6;
}

impl Lookup {
    /// Initialises the tables of [`LOOKUPS`].
    pub fn init() {
        // SAFETY: These functions write to a mutable static before anything
        // else reads from it.
        #[allow(clippy::multiple_unsafe_ops_per_block)]
        unsafe {
            LOOKUPS.init_pawn_attacks();
            LOOKUPS.init_knight_attacks();
            LOOKUPS.init_king_attacks();
            LOOKUPS.init_magics();
        };
    }

    /// Returns a [`Lookup`] with empty tables.
    // used to initialise a static `Lookup` variable
    #[allow(clippy::large_stack_frames)]
    const fn empty() -> Self {
        Self {
            pawn_attacks: [[Bitboard::empty(); Square::TOTAL]; Side::TOTAL],
            knight_attacks: [Bitboard::empty(); Square::TOTAL],
            king_attacks: [Bitboard::empty(); Square::TOTAL],
            // allowed because, after testing, a vector was slightly slower
            #[allow(clippy::large_stack_arrays)]
            magic_table: [Bitboard::empty(); ROOK_SIZE + BISHOP_SIZE],
            bishop_magics: [Magic::default(); Square::TOTAL],
            rook_magics: [Magic::default(); Square::TOTAL],
        }
    }

    /// Initialises pawn attack lookup table. First and last rank are ignored.
    fn init_pawn_attacks(&mut self) {
        for (square, bb) in self.pawn_attacks[Side::WHITE.to_index()]
            .iter_mut()
            .enumerate()
            .take(Square::TOTAL - File::TOTAL)
        {
            let pushed = Bitboard::from(Square(square as u8 + 8));
            *bb = pushed.east() | pushed.west();
        }
        for (square, bb) in self.pawn_attacks[Side::BLACK.to_index()]
            .iter_mut()
            .enumerate()
            .skip(File::TOTAL)
        {
            let pushed = Bitboard::from(Square(square as u8 - 8));
            *bb = pushed.east() | pushed.west();
        }
    }

    /// Initialises knight attack lookup table.
    fn init_knight_attacks(&mut self) {
        for (square, bb) in self.knight_attacks.iter_mut().enumerate() {
            let square = Square(square as u8);
            let knight = Bitboard::from(square);
            let mut east = knight.east();
            let mut west = knight.west();

            let mut attacks = (east | west).north().north();
            attacks |= (east | west).south().south();

            east = east.east();
            west = west.west();
            attacks |= (east | west).north();
            attacks |= (east | west).south();

            *bb = attacks;
        }
    }

    /// Initialises king attack lookup table.
    fn init_king_attacks(&mut self) {
        for (square, bb) in self.king_attacks.iter_mut().enumerate() {
            let square = Square(square as u8);
            let king = Bitboard::from(square);

            let mut attacks = king.east() | king.west() | king;
            attacks |= attacks.north() | attacks.south();
            attacks ^= king;

            *bb = attacks;
        }
    }

    /// Initialises the magic lookup tables with attacks and initialises a
    /// [`Magic`] object for each square.
    fn init_magics(&mut self) {
        let mut b_offset = ROOK_SIZE;
        let mut r_offset = 0;

        for square in 0..Square::TOTAL {
            let square = Square(square as u8);
            let mut attacks = [Bitboard::empty(); MAX_BLOCKERS];
            let edges = Bitboard::edges_without(square);
            let b_mask =
                sliding_attacks::<{ PieceType::BISHOP.0 }>(square, Bitboard::empty()) & !edges;
            let r_mask =
                sliding_attacks::<{ PieceType::ROOK.0 }>(square, Bitboard::empty()) & !edges;
            let b_mask_bits = b_mask.0.count_ones();
            let r_mask_bits = r_mask.0.count_ones();
            let b_perms = 2usize.pow(b_mask_bits);
            let r_perms = 2usize.pow(r_mask_bits);
            let b_magic = Magic::new(
                BISHOP_MAGICS[square.to_index()],
                b_mask,
                b_offset,
                64 - b_mask_bits,
            );
            let r_magic = Magic::new(
                ROOK_MAGICS[square.to_index()],
                r_mask,
                r_offset,
                64 - r_mask_bits,
            );

            gen_all_sliding_attacks::<{ PieceType::BISHOP.0 }>(square, &mut attacks);
            let mut blockers = b_mask;
            for attack in attacks.iter().take(b_perms) {
                let index = b_magic.get_table_index(blockers);
                self.magic_table[index] = *attack;
                blockers = Bitboard(blockers.0.wrapping_sub(1)) & b_mask;
            }
            self.bishop_magics[square.to_index()] = b_magic;
            b_offset += b_perms;

            gen_all_sliding_attacks::<{ PieceType::ROOK.0 }>(square, &mut attacks);
            let mut blockers = r_mask;
            for attack in attacks.iter().take(r_perms) {
                let index = r_magic.get_table_index(blockers);
                self.magic_table[index] = *attack;
                blockers = Bitboard(blockers.0.wrapping_sub(1)) & r_mask;
            }
            self.rook_magics[square.to_index()] = r_magic;
            r_offset += r_perms;
        }
    }

    /// Finds the pawn attacks from `square`.
    pub fn pawn_attacks(&self, side: Side, square: Square) -> Bitboard {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(side.to_index(), self.pawn_attacks.len()) };
        // SAFETY: Ditto.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.pawn_attacks[0].len()) };
        self.pawn_attacks[side.to_index()][square.to_index()]
    }

    /// Finds the knight attacks from `square`.
    pub fn knight_attacks(&self, square: Square) -> Bitboard {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.knight_attacks.len()) };
        self.knight_attacks[square.to_index()]
    }

    /// Finds the king attacks from `square`.
    pub fn king_attacks(&self, square: Square) -> Bitboard {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.king_attacks.len()) };
        self.king_attacks[square.to_index()]
    }

    /// Finds the bishop attacks from `square` with the given blockers.
    pub fn bishop_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.bishop_magics.len()) };
        let index = self.bishop_magics[square.to_index()].get_table_index(blockers);
        // SAFETY: Ditto.
        unsafe { out_of_bounds_is_unreachable!(index, self.magic_table.len()) };
        self.magic_table[index]
    }

    /// Finds the rook attacks from `square` with the given blockers.
    pub fn rook_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        // SAFETY: If it does get reached, it will panic in debug.
        unsafe { out_of_bounds_is_unreachable!(square.to_index(), self.rook_magics.len()) };
        let index = self.rook_magics[square.to_index()].get_table_index(blockers);
        // SAFETY: Ditto.
        unsafe { out_of_bounds_is_unreachable!(index, self.magic_table.len()) };
        self.magic_table[index]
    }

    /// Finds the queen attacks from `square` with the given blockers.
    pub fn queen_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        self.bishop_attacks(square, blockers) | self.rook_attacks(square, blockers)
    }
}

impl Move {
    /// Creates a normal [`Move`] from `start` to `end`.
    ///
    /// This function cannot be used for special moves like castling.
    pub const fn new(start: Square, end: Square) -> Self {
        Self::base(start, end).flag(Self::NORMAL)
    }

    /// Creates an en passant [`Move`] from `start` to `end`.
    pub const fn new_en_passant(start: Square, end: Square) -> Self {
        Self::base(start, end).flag(Self::EN_PASSANT)
    }

    /// Creates a castling [`Move`] from `start` to `end`, given if the side is
    /// White and if the side of the board is kingside.
    pub const fn new_castle<const IS_WHITE: bool, const IS_KINGSIDE: bool>() -> Self {
        #[allow(clippy::collapsible_else_if)]
        if IS_WHITE {
            if IS_KINGSIDE {
                Self::base(Square::E1, Square::G1)
                    .flag(Self::CASTLING)
                    .extra_bits(3)
            } else {
                Self::base(Square::E1, Square::C1)
                    .flag(Self::CASTLING)
                    .extra_bits(0)
            }
        } else {
            if IS_KINGSIDE {
                Self::base(Square::E8, Square::G8)
                    .flag(Self::CASTLING)
                    .extra_bits(3)
            } else {
                Self::base(Square::E8, Square::C8)
                    .flag(Self::CASTLING)
                    .extra_bits(0)
            }
        }
    }

    /// Creates a promotion [`Move`] to the given piece type from `start` to
    /// `end`.
    pub const fn new_promo<const PIECE: u8>(start: Square, end: Square) -> Self {
        Self::base(start, end)
            .flag(Self::PROMOTION)
            .extra_bits(PIECE - 1)
    }

    /// Creates a promotion [`Move`] to the given piece type from `start` to
    /// `end`.
    pub const fn new_promo_any(start: Square, end: Square, promotion_piece: PieceType) -> Self {
        Self::base(start, end)
            .flag(Self::PROMOTION)
            .extra_bits(promotion_piece.0 - 1)
    }

    /// Creates a null [`Move`].
    pub const fn null() -> Self {
        Self::base(Square(0), Square(0))
    }

    /// Calculates the start square of `self`.
    pub const fn start(self) -> Square {
        Square(self.upper & Self::SQUARE_MASK)
    }

    /// Calculates the end square of `self`.
    pub const fn end(self) -> Square {
        Square(self.lower & Self::SQUARE_MASK)
    }

    /// Checks if the move is castling.
    pub const fn is_castling(self) -> bool {
        self.upper & Self::FLAG_MASK == Self::CASTLING
    }

    /// Checks if the move is en passant.
    pub const fn is_en_passant(self) -> bool {
        self.upper & Self::FLAG_MASK == Self::EN_PASSANT
    }

    /// Checks if the move is a promotion.
    pub const fn is_promotion(self) -> bool {
        self.upper & Self::FLAG_MASK == Self::PROMOTION
    }

    /// Returns the difference from the king destination square to the rook
    /// starting square. Assumes `self.is_castling()`.
    ///
    /// Can only return -2 or 1.
    pub const fn rook_offset(self) -> i8 {
        (self.lower >> Self::EXTRA_BITS_SHIFT) as i8 - 2
    }

    /// Returns the piece to be promoted to. Assumes `self.is_promotion()`.
    ///
    /// The piece will only ever be a valid piece.
    pub const fn promotion_piece(self) -> PieceType {
        PieceType((self.lower >> Self::EXTRA_BITS_SHIFT) + 1)
    }

    /// Checks if the given start and end square match the start and end square
    /// contained within `self`.
    pub const fn is_moving_from_to(self, start: Square, end: Square) -> bool {
        let other = Self::new(start, end);
        // if the start and end square are the same, xoring them together
        // will be 0
        (self.lower ^ other.lower) & Self::SQUARE_MASK
            | (self.upper ^ other.upper) & Self::SQUARE_MASK
            == 0
    }

    /// Creates a base [`Move`] with the given start and end square.
    const fn base(start: Square, end: Square) -> Self {
        Self {
            lower: end.0,
            upper: start.0,
        }
    }

    /// Adds the given flag to `self`.
    const fn flag(mut self, flag: u8) -> Self {
        self.upper |= flag;
        self
    }

    /// Adds the given extra bits to `self`.
    const fn extra_bits(mut self, extra_bits: u8) -> Self {
        self.lower |= extra_bits << Self::EXTRA_BITS_SHIFT;
        self
    }
}

impl Moves {
    /// Finds and returns, if it exists, the move that has start square `start`
    /// and end square `end`.
    ///
    /// Returns `Some(mv)` if a `Move` does match the start and end square;
    /// returns `None` otherwise.
    pub fn move_with(&mut self, start: Square, end: Square) -> Option<Move> {
        self.find(|&mv| mv.is_moving_from_to(start, end))
    }

    /// Finds and returns, if it exists, the move that has start square
    /// `start`, end square `end` and promotion piece `piece_type`.
    ///
    /// Returns `Some(mv)` if a `Move` does match the criteria; returns `None`
    /// otherwise.
    pub fn move_with_promo(
        &mut self,
        start: Square,
        end: Square,
        piece_type: PieceType,
    ) -> Option<Move> {
        self.find(|&mv| mv == Move::new_promo_any(start, end, piece_type))
    }
}

/// Generates all legal moves for the current position and puts them in
/// `moves`.
pub fn generate_moves<const MOVE_TYPE: u8>(board: &Board) -> Moves {
    let mut moves = Moves::new();
    if board.side_to_move() == Side::WHITE {
        generate_pawn_moves::<true, MOVE_TYPE>(board, &mut moves);
        generate_non_sliding_moves::<true, MOVE_TYPE>(board, &mut moves);
        generate_sliding_moves::<true, MOVE_TYPE>(board, &mut moves);
        if MOVE_TYPE == MoveType::ALL {
            generate_castling::<true>(board, &mut moves);
        }
    } else {
        generate_pawn_moves::<false, MOVE_TYPE>(board, &mut moves);
        generate_non_sliding_moves::<false, MOVE_TYPE>(board, &mut moves);
        generate_sliding_moves::<false, MOVE_TYPE>(board, &mut moves);
        if MOVE_TYPE == MoveType::ALL {
            generate_castling::<false>(board, &mut moves);
        }
    }
    moves
}

/// Generates the castling moves for the given side.
fn generate_castling<const IS_WHITE: bool>(board: &Board, moves: &mut Moves) {
    let occupancies = board.occupancies();

    if board.can_castle_kingside::<IS_WHITE>()
        && (occupancies & Bitboard::castling_space::<IS_WHITE, true>()).is_empty()
    {
        moves.push(Move::new_castle::<IS_WHITE, true>());
    }
    if board.can_castle_queenside::<IS_WHITE>()
        && (occupancies & Bitboard::castling_space::<IS_WHITE, false>()).is_empty()
    {
        moves.push(Move::new_castle::<IS_WHITE, false>());
    }
}

/// Generates all legal knight and king moves (excluding castling) for `board`
/// and puts them in `moves`.
fn generate_non_sliding_moves<const IS_WHITE: bool, const MOVE_TYPE: u8>(
    board: &Board,
    moves: &mut Moves,
) {
    let us_bb = board.side::<IS_WHITE>();
    let target_squares = if MOVE_TYPE == MoveType::ALL {
        // all squares that aren't us
        !us_bb
    } else if MOVE_TYPE == MoveType::CAPTURES {
        // the opponent's piece
        if IS_WHITE {
            board.side::<false>()
        } else {
            board.side::<true>()
        }
    } else {
        panic!("Unknown movetype");
    };

    let knights = board.piece::<{ PieceType::KNIGHT.to_index() }>() & us_bb;
    for knight in knights {
        // SAFETY: Instantiating `board` initialises `LOOKUP`.
        let targets = unsafe { LOOKUPS.knight_attacks(knight) } & target_squares;
        for target in targets {
            moves.push(Move::new(knight, target));
        }
    }

    let kings = board.piece::<{ PieceType::KING.to_index() }>() & us_bb;
    for king in kings {
        // SAFETY: Instantiating `board` initialises `LOOKUP`.
        let targets = unsafe { LOOKUPS.king_attacks(king) } & target_squares;
        for target in targets {
            moves.push(Move::new(king, target));
        }
    }
}

/// Generates all legal pawn moves for `board` and puts them in `moves`.
fn generate_pawn_moves<const IS_WHITE: bool, const MOVE_TYPE: u8>(
    board: &Board,
    moves: &mut Moves,
) {
    let us_bb = board.side::<IS_WHITE>();
    let occupancies = board.occupancies();
    let them_bb = occupancies ^ us_bb;
    let ep_square = board.ep_square();
    let ep_square_bb = if ep_square == Square::NONE {
        Bitboard::empty()
    } else {
        Bitboard::from(ep_square)
    };
    let empty = !occupancies;

    let mut pawns = board.piece::<{ PieceType::PAWN.to_index() }>() & us_bb;
    while !pawns.is_empty() {
        let pawn = pawns.pop_lsb();
        let pawn_sq = Square::from(pawn);

        let potential_captures = if IS_WHITE {
            // SAFETY: Instantiating `board` initialises `LOOKUP`.
            unsafe { LOOKUPS.pawn_attacks(Side::WHITE, pawn_sq) }
        } else {
            // SAFETY: Same thing.
            unsafe { LOOKUPS.pawn_attacks(Side::BLACK, pawn_sq) }
        };
        let normal_captures = potential_captures & them_bb;
        let ep_targets = potential_captures & ep_square_bb;

        // if we're just looking at captures, loop through all captures
        // early. Otherwise, wait a bit longer to loop through pushes and
        // captures in the same loop.
        if MOVE_TYPE == MoveType::CAPTURES {
            for target in normal_captures {
                moves.push(Move::new(pawn_sq, target));
            }
            for target in ep_targets {
                moves.push(Move::new_en_passant(pawn_sq, target));
            }
            continue;
        }

        let single_push = pawn.pawn_push::<IS_WHITE>() & empty;

        let double_push_rank = if IS_WHITE {
            Bitboard::rank_bb(Rank::RANK4)
        } else {
            Bitboard::rank_bb(Rank::RANK5)
        };
        let double_push = single_push.pawn_push::<IS_WHITE>() & empty & double_push_rank;

        let targets = single_push | normal_captures | double_push;
        let promotion_targets =
            targets & (Bitboard::rank_bb(Rank::RANK1) | Bitboard::rank_bb(Rank::RANK8));
        let normal_targets = targets ^ promotion_targets;

        for target in normal_targets {
            moves.push(Move::new(pawn_sq, target));
        }
        for target in ep_targets {
            moves.push(Move::new_en_passant(pawn_sq, target));
        }
        for target in promotion_targets {
            moves.push(Move::new_promo::<{ PieceType::KNIGHT.0 }>(pawn_sq, target));
            moves.push(Move::new_promo::<{ PieceType::BISHOP.0 }>(pawn_sq, target));
            moves.push(Move::new_promo::<{ PieceType::ROOK.0 }>(pawn_sq, target));
            moves.push(Move::new_promo::<{ PieceType::QUEEN.0 }>(pawn_sq, target));
        }
    }
}

/// Generates all legal bishop, rook and queen moves for `board` and puts them
/// in `moves`.
fn generate_sliding_moves<const IS_WHITE: bool, const MOVE_TYPE: u8>(
    board: &Board,
    moves: &mut Moves,
) {
    let us_bb = board.side::<IS_WHITE>();
    let occupancies = board.occupancies();
    let target_squares = if MOVE_TYPE == MoveType::ALL {
        !us_bb
    } else if MOVE_TYPE == MoveType::CAPTURES {
        us_bb ^ occupancies
    } else {
        panic!("Unknown movetype");
    };

    let bishops = board.piece::<{ PieceType::BISHOP.to_index() }>() & us_bb;
    for bishop in bishops {
        // SAFETY: Instantiating `board` initialises `LOOKUP`.
        let targets = unsafe { LOOKUPS.bishop_attacks(bishop, occupancies) } & target_squares;
        for target in targets {
            moves.push(Move::new(bishop, target));
        }
    }

    let rooks = board.piece::<{ PieceType::ROOK.to_index() }>() & us_bb;
    for rook in rooks {
        // SAFETY: Instantiating `board` initialises `LOOKUP`.
        let targets = unsafe { LOOKUPS.rook_attacks(rook, occupancies) } & target_squares;
        for target in targets {
            moves.push(Move::new(rook, target));
        }
    }

    let queens = board.piece::<{ PieceType::QUEEN.to_index() }>() & us_bb;
    for queen in queens {
        // SAFETY: Instantiating `board` initialises `LOOKUP`.
        let targets = unsafe { LOOKUPS.queen_attacks(queen, occupancies) } & target_squares;
        for target in targets {
            moves.push(Move::new(queen, target));
        }
    }
}
