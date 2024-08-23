/*
 * Crab, a UCI-compatible chess engine
 * Copyright (C) 2024 Jasper Shovelton
 *
 * Crab is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Crab is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE. See the GNU General Public License for more
 * details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Crab. If not, see <https://www.gnu.org/licenses/>.
 */

use std::{
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    num::NonZeroU16,
    ops::{Deref, DerefMut},
};

use arrayvec::ArrayVec;
use oorandom::Rand64;

use crate::{
    bitboard::Bitboard,
    board::Board,
    cfor,
    defs::{Direction, PieceType, Rank, Side, Square},
    evaluation::Eval,
    util::get_unchecked,
};
use magic::{Magic, BISHOP_MAGICS, ROOK_MAGICS};
use util::{bitboard_from_square, east, north, sliding_attacks, south, west};

/// Items related to magic bitboards.
pub mod magic;
/// Useful functions for move generation.
mod util;

/// Moves of a certain type.
pub trait MovesType {
    /// Generate quiet moves for all non-king pieces (so including castling)?
    const NON_KING_QUIETS: bool;
    /// Generate quiet moves exclusively for the king (so excluding castling)?
    const KING_QUIETS: bool;
    /// Generate captures?
    const CAPTURES: bool;
}

/// Generate all legal moves.
pub struct AllMoves;
/// Generate only captures.
pub struct CapturesOnly;
/// Generate captures and quiet king moves.
pub struct Evasions;
/// Generate only quiet king moves.
pub struct KingMovesOnly;
/// Generate only quiet moves.
pub struct QuietsOnly;

impl MovesType for AllMoves {
    const NON_KING_QUIETS: bool = true;
    const KING_QUIETS: bool = true;
    const CAPTURES: bool = true;
}

impl MovesType for CapturesOnly {
    const NON_KING_QUIETS: bool = false;
    const KING_QUIETS: bool = false;
    const CAPTURES: bool = true;
}

impl MovesType for Evasions {
    const NON_KING_QUIETS: bool = false;
    const KING_QUIETS: bool = true;
    const CAPTURES: bool = true;
}

impl MovesType for KingMovesOnly {
    const NON_KING_QUIETS: bool = false;
    const KING_QUIETS: bool = true;
    const CAPTURES: bool = false;
}

impl MovesType for QuietsOnly {
    const NON_KING_QUIETS: bool = true;
    const KING_QUIETS: bool = true;
    const CAPTURES: bool = false;
}

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
    /// The (wrapped) magic numbers for the bishop. One per square.
    ///
    /// See <https://www.chessprogramming.org/Magic_Bitboards>.
    bishop_magics: [Magic; Square::TOTAL],
    /// The (wrapped) magic numbers for the rook. One per square.
    ///
    /// See <https://www.chessprogramming.org/Magic_Bitboards>.
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
    /// From most significant bit: extra bits (2 bits), end square (6 bits),
    /// flags (2 bits) and start square (6 bits). `0beeEEEEEEffSSSSSS`.
    bits: NonZeroU16,
}

/// A [`Move`] that has been given a certain score.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Clone, Copy)]
pub struct ScoredMove {
    pub mv: Move,
    pub score: Eval,
}

/// An stack of [`Move`]s.
#[allow(clippy::missing_docs_in_private_items)]
pub struct Moves {
    moves: ArrayVec<ScoredMove, MAX_LEGAL_MOVES>,
}

impl Deref for Moves {
    type Target = ArrayVec<ScoredMove, MAX_LEGAL_MOVES>;

    fn deref(&self) -> &Self::Target {
        &self.moves
    }
}

impl DerefMut for Moves {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.moves
    }
}

/// The number of bitboards required to store all bishop attacks, where each
/// element corresponds to one permutation of blockers.
///
/// Some elements will be duplicates, as different blockers can have the same
/// attacks. Repeated once per quadrant: `2.pow(6)` blocker permutations for
/// the corner, `2.pow(5)` for each non-corner edge and each square adjacent to
/// an edge, `2.pow(7)` for the squares adjacent or diagonal to a corner and
/// `2.pow(9)` for the centre.
const BISHOP_SIZE: usize = 5_248;
/// The number of bitboards required to store all rook attacks, where each
/// element corresponds to one permutation of blockers.
///
/// Some elements will be duplicates, as different blockers can have the same
/// attacks. There are `2.pow(12)` blocker permutations for each corner,
/// `2.pow(11)` for each non-corner edge and `2.pow(10)` for all others.
const ROOK_SIZE: usize = 102_400;
/// Maximum number of legal moves that can be reached in a standard chess game.
///
/// Example: `R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 w - - 0 1`
pub const MAX_LEGAL_MOVES: usize = 218;
/// The lookup tables.
pub static LOOKUPS: Lookup = Lookup::new();

impl Eq for ScoredMove {}

impl Ord for ScoredMove {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl PartialEq for ScoredMove {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl PartialOrd for ScoredMove {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.score.cmp(&other.score))
    }
}

impl Move {
    /// Flag for castling.
    const CASTLING: u16 = 0b0100_0000;
    /// Flag for en passant.
    const EN_PASSANT: u16 = 0b1000_0000;
    /// Flag for promotion.
    const PROMOTION: u16 = 0b1100_0000;
    /// No flags.
    const NORMAL: u16 = 0b0000_0000;
    /// Shift for the start square.
    const START_SQUARE_SHIFT: usize = 0;
    /// Shift for the end square.
    const END_SQUARE_SHIFT: usize = 8;
    /// Mask for the squares after shifting.
    const SQUARE_MASK: u16 = 0b11_1111;
    /// Mask for the flags.
    const FLAG_MASK: u16 = 0b1100_0000;
    /// Shift for the promotion piece/rook offset.
    const EXTRA_BITS_SHIFT: usize = 14;
}

impl ScoredMove {
    /// The score of a capture with a winning static exchange evaluation.
    pub const WINNING_CAPTURE_SCORE: Eval = 0x2000;
    /// The score of a quiet move.
    pub const QUIET_SCORE: Eval = 0x1000;
}

impl Display for Move {
    /// Displays a move in long algebraic notation.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let start = self.start();
        let end = self.end();
        let promotion_piece = char::from(self.promotion_piece());
        if self.is_promotion() {
            // we want the lowercase letter here
            write!(f, "{start}{end}{promotion_piece}")
        } else {
            write!(f, "{start}{end}")
        }
    }
}

impl Lookup {
    /// Creates new lookup tables.
    ///
    /// This is meant to be called once at compile time.
    #[allow(clippy::large_stack_frames)]
    const fn new() -> Self {
        let pawn_attacks = Self::init_pawn_attacks();
        let king_attacks = Self::init_king_attacks();
        let knight_attacks = Self::init_knight_attacks();
        let (magic_table, bishop_magics, rook_magics) = Self::init_magics();

        Self {
            pawn_attacks,
            knight_attacks,
            king_attacks,
            magic_table,
            bishop_magics,
            rook_magics,
        }
    }

    /// Calculates and returns lookup tables for both pawns.
    ///
    /// `init_pawn_attacks()[Side::WHITE.to_index() == White pawn attack table`
    const fn init_pawn_attacks() -> [[Bitboard; Square::TOTAL]; Side::TOTAL] {
        let mut pawn_attacks = [[Bitboard::empty(); Square::TOTAL]; Side::TOTAL];
        cfor!(let mut square = 0; square < Square::TOTAL; square += 1; {
            let pawn = bitboard_from_square(square as u8);
            let pushed_white = north(pawn);
            let pushed_black = south(pawn);
            pawn_attacks[Side::WHITE.to_index()][square] =
                Bitboard(east(pushed_white) | west(pushed_white));
            pawn_attacks[Side::BLACK.to_index()][square] =
                Bitboard(east(pushed_black) | west(pushed_black));
        });
        pawn_attacks
    }

    /// Calculates and returns a lookup table for the knight.
    const fn init_knight_attacks() -> [Bitboard; Square::TOTAL] {
        let mut knight_attacks = [Bitboard::empty(); Square::TOTAL];
        cfor!(let mut square = 0; square < Square::TOTAL; square += 1; {
            let knight = bitboard_from_square(square as u8);
            let mut e = east(knight);
            let mut w = west(knight);
            let mut attacks = north(north(e | w));
            attacks |= south(south(e | w));

            e = east(e);
            w = west(w);
            attacks |= north(e | w);
            attacks |= south(e | w);

            knight_attacks[square] = Bitboard(attacks);
        });
        knight_attacks
    }

    /// Calculates and returns a lookup table for the king.
    const fn init_king_attacks() -> [Bitboard; Square::TOTAL] {
        let mut king_attacks = [Bitboard::empty(); Square::TOTAL];
        cfor!(let mut square = 0; square < Square::TOTAL; square += 1; {
            let king = bitboard_from_square(square as u8);

            let mut attacks = east(king) | west(king) | king;
            attacks |= north(attacks) | south(attacks);
            attacks ^= king;

            king_attacks[square] = Bitboard(attacks);
        });
        king_attacks
    }

    /// Calculates and returns the magic lookup table and magic structs.
    ///
    /// `init_magics() == (magic_table, bishop_magics, rook_magics)`.
    #[allow(clippy::large_stack_arrays, clippy::large_stack_frames)]
    const fn init_magics() -> (
        [Bitboard; ROOK_SIZE + BISHOP_SIZE],
        [Magic; Square::TOTAL],
        [Magic; Square::TOTAL],
    ) {
        let mut b_offset = ROOK_SIZE;
        let mut r_offset = 0;
        let mut magic_table = [Bitboard::empty(); ROOK_SIZE + BISHOP_SIZE];
        let mut bishop_magics = [Magic::null(); Square::TOTAL];
        let mut rook_magics = [Magic::null(); Square::TOTAL];

        cfor!(let mut square = 0; square < Square::TOTAL; square += 1; {
            let square = Square(square as u8);
            let edges = Bitboard::edges_without(square).0;
            let b_mask =
                sliding_attacks::<{ PieceType::BISHOP.0 }>(square, Bitboard::empty()).0 & !edges;
            let r_mask =
                sliding_attacks::<{ PieceType::ROOK.0 }>(square, Bitboard::empty()).0 & !edges;
            let b_mask_bits = b_mask.count_ones();
            let r_mask_bits = r_mask.count_ones();
            let b_perms = 2_usize.pow(b_mask_bits);
            let r_perms = 2_usize.pow(r_mask_bits);

            let b_magic = Magic::new(
                BISHOP_MAGICS[square.to_index()],
                Bitboard(b_mask),
                b_offset,
                64 - b_mask_bits,
            );
            bishop_magics[square.to_index()] = b_magic;
            let r_magic = Magic::new(
                ROOK_MAGICS[square.to_index()],
                Bitboard(r_mask),
                r_offset,
                64 - r_mask_bits,
            );
            rook_magics[square.to_index()] = r_magic;

            let mut blockers = b_mask;
            cfor!(let mut attack = 0; attack < b_perms; attack += 1; {
                let index = b_magic.get_table_index(Bitboard(blockers));
                magic_table[index] = sliding_attacks::<{ PieceType::BISHOP.0 }>(square, Bitboard(blockers));
                blockers = blockers.wrapping_sub(1) & b_mask;
            });

            let mut blockers = r_mask;
            cfor!(let mut attack = 0; attack < r_perms; attack += 1; {
                let index = r_magic.get_table_index(Bitboard(blockers));
                magic_table[index] = sliding_attacks::<{ PieceType::ROOK.0 }>(square, Bitboard(blockers));
                blockers = blockers.wrapping_sub(1) & r_mask;
            });

            b_offset += b_perms;
            r_offset += r_perms;
        });
        (magic_table, bishop_magics, rook_magics)
    }

    /// Finds the pawn attacks from `square`.
    pub fn pawn_attacks(&self, side: Side, square: Square) -> Bitboard {
        let side_table = get_unchecked(&self.pawn_attacks, side.to_index());
        *get_unchecked(side_table, square.to_index())
    }

    /// Finds the knight attacks from `square`.
    pub fn knight_attacks(&self, square: Square) -> Bitboard {
        *get_unchecked(&self.knight_attacks, square.to_index())
    }

    /// Finds the king attacks from `square`.
    pub fn king_attacks(&self, square: Square) -> Bitboard {
        *get_unchecked(&self.king_attacks, square.to_index())
    }

    /// Finds the bishop attacks from `square` with the given blockers.
    pub fn bishop_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        let index = get_unchecked(&self.bishop_magics, square.to_index()).get_table_index(blockers);
        *get_unchecked(&self.magic_table, index)
    }

    /// Finds the rook attacks from `square` with the given blockers.
    pub fn rook_attacks(&self, square: Square, blockers: Bitboard) -> Bitboard {
        let index = get_unchecked(&self.rook_magics, square.to_index()).get_table_index(blockers);
        *get_unchecked(&self.magic_table, index)
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
    pub fn new(start: Square, end: Square) -> Self {
        Self::base(start, end).flag(Self::NORMAL)
    }

    /// Creates an en passant [`Move`] from `start` to `end`.
    pub fn new_en_passant(start: Square, end: Square) -> Self {
        Self::base(start, end).flag(Self::EN_PASSANT)
    }

    /// Creates a castling [`Move`] from `start` to `end`, given if the side is
    /// White and if the side of the board is kingside.
    pub fn new_castle<const IS_WHITE: bool, const IS_KINGSIDE: bool>() -> Self {
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
    pub fn new_promo<const PIECE: u8>(start: Square, end: Square) -> Self {
        Self::base(start, end)
            .flag(Self::PROMOTION)
            .extra_bits(u16::from(PIECE) - 1)
    }

    /// Creates a promotion [`Move`] to the given piece type from `start` to
    /// `end`.
    pub fn new_promo_any(start: Square, end: Square, promotion_piece: PieceType) -> Self {
        Self::base(start, end)
            .flag(Self::PROMOTION)
            .extra_bits(u16::from(promotion_piece.0) - 1)
    }

    /// Creates a base [`Move`] with the given start and end square.
    fn base(start: Square, end: Square) -> Self {
        debug_assert!(start.0 != 0 || end.0 != 0, "storing a 0 in a NonZeroU16");
        Self {
            // SAFETY: `start` and `end` cannot both be 0
            bits: unsafe {
                NonZeroU16::new_unchecked(
                    u16::from(end.0) << Self::END_SQUARE_SHIFT
                        | u16::from(start.0) << Self::START_SQUARE_SHIFT,
                )
            },
        }
    }

    /// Adds the given flag to the move.
    fn flag(mut self, flag: u16) -> Self {
        self.bits |= flag;
        self
    }

    /// Adds the given extra bits to the move.
    fn extra_bits(mut self, extra_bits: u16) -> Self {
        self.bits |= extra_bits << Self::EXTRA_BITS_SHIFT;
        self
    }

    /// Calculates the start square of the move.
    pub const fn start(self) -> Square {
        Square(((self.bits.get() >> Self::START_SQUARE_SHIFT) & Self::SQUARE_MASK) as u8)
    }

    /// Calculates the end square of the move.
    pub const fn end(self) -> Square {
        Square(((self.bits.get() >> Self::END_SQUARE_SHIFT) & Self::SQUARE_MASK) as u8)
    }

    /// Checks if the move is castling.
    pub const fn is_castling(self) -> bool {
        self.bits.get() & Self::FLAG_MASK == Self::CASTLING
    }

    /// Checks if the move is en passant.
    pub const fn is_en_passant(self) -> bool {
        self.bits.get() & Self::FLAG_MASK == Self::EN_PASSANT
    }

    /// Checks if the move is a promotion.
    pub const fn is_promotion(self) -> bool {
        self.bits.get() & Self::FLAG_MASK == Self::PROMOTION
    }

    /// Returns the difference from the king destination square to the rook
    /// starting square.
    ///
    /// Assumes `self.is_castling()`. Can only return -2 or 1.
    pub const fn rook_offset(self) -> i8 {
        (self.bits.get() >> Self::EXTRA_BITS_SHIFT) as i8 - 2
    }

    /// Returns the piece to be promoted to.
    ///
    /// Assumes `self.is_promotion()`. The piece will only ever be a valid piece.
    pub const fn promotion_piece(self) -> PieceType {
        PieceType(((self.bits.get() >> Self::EXTRA_BITS_SHIFT) + 1) as u8)
    }

    /// Checks if the move is moving from the given start square to the given
    /// end square.
    pub fn is_moving_from_to(self, start: Square, end: Square) -> bool {
        let other = Self::new(start, end);
        // if the start and end square are the same, xoring them together
        // will be 0
        let both_square_mask = (Self::SQUARE_MASK << Self::START_SQUARE_SHIFT)
            | (Self::SQUARE_MASK << Self::END_SQUARE_SHIFT);
        (self.bits.get() ^ other.bits.get()) & both_square_mask == 0
    }
}

impl Moves {
    /// Creates a new list of moves.
    pub fn new() -> Self {
        Self {
            moves: ArrayVec::new(),
        }
    }

    /// Pushes `mv` without bounds checking in release mode.
    pub fn push(&mut self, mv: Move) {
        self.push_scored_move(ScoredMove::new(mv));
    }

    /// Pushes `mv` without bounds checking in release mode.
    pub fn push_scored_move(&mut self, scored_move: ScoredMove) {
        debug_assert!(self.len() < self.capacity(), "stack overflow");
        // SAFETY: we just checked that we are able to push
        unsafe { self.push_unchecked(scored_move) };
    }

    /// Finds and returns, if it exists, the [`Move`] that has start square
    /// `start` and end square `end`.
    ///
    /// Returns `Some(mv)` if a [`Move`] does match the start and end square;
    /// returns `None` otherwise.
    pub fn move_with(&self, start: Square, end: Square) -> Option<Move> {
        self.iter()
            .find(|&scored_move| scored_move.mv.is_moving_from_to(start, end))
            .map(|&scored_move| scored_move.mv)
    }

    /// Finds and returns, if it exists, the [`Move`] that has start square
    /// `start`, end square `end` and promotion piece `piece_type`.
    ///
    /// Returns `Some(mv)` if a [`Move`] does match the criteria; returns `None`
    /// otherwise.
    pub fn move_with_promo(
        &self,
        start: Square,
        end: Square,
        piece_type: PieceType,
    ) -> Option<Move> {
        self.iter()
            .find(|&scored_move| scored_move.mv == Move::new_promo_any(start, end, piece_type))
            .map(|&scored_move| scored_move.mv)
    }

    /// Picks a random item, swaps it with the first item, then pops the
    /// now-first item.
    pub fn pop_random(&mut self, seed: &mut Rand64) -> Option<ScoredMove> {
        let total_moves = self.len();

        let index = if total_moves >= 2 {
            seed.rand_range(0_u64..total_moves as u64) as usize
        } else {
            0
        };

        self.swap_pop(index)
    }
}

impl ScoredMove {
    /// Creates a new [`ScoredMove`] with a score of `0`.
    const fn new(mv: Move) -> Self {
        Self { mv, score: 0 }
    }

    /// Scores `self.mv`.
    #[allow(clippy::assertions_on_constants)]
    pub fn score<Type: MovesType>(&mut self, board: &Board) {
        if !Type::CAPTURES {
            self.score += Self::QUIET_SCORE;
            return;
        }

        let mv = self.mv;

        let captured_piece = if mv.is_en_passant() {
            PieceType::PAWN
        } else {
            PieceType::from(board.piece_on(mv.end()))
        };

        // Pre-emptively give the capture a winning score - it can be
        // checked later.
        // This outer if statement has odd but intentional behaviour - if a
        // move doesn't capture anything but this function is being told it's a
        // capture, it will treat it as a capture, but if it's told it's
        // scoring any type of move, it will treat it as a quiet. This is so
        // queen promotions (even quiet ones) can be treated as captures.
        if !Type::KING_QUIETS && !Type::NON_KING_QUIETS {
            self.score += Self::WINNING_CAPTURE_SCORE + captured_piece.mvv_bonus();
        } else {
            self.score += if captured_piece == PieceType::NONE {
                Self::QUIET_SCORE
            } else {
                Self::WINNING_CAPTURE_SCORE + captured_piece.mvv_bonus()
            };
        }
    }
}

/// Calculates all legal moves for the current position of the given board and
/// appends them to `moves`.
#[allow(clippy::assertions_on_constants)]
pub fn generate_moves<Type: MovesType>(board: &Board, moves: &mut Moves) {
    if board.side_to_move() == Side::WHITE {
        generate_pawn_moves::<Type, true>(board, moves);
        generate_non_sliding_moves::<Type, true>(board, moves);
        generate_sliding_moves::<Type, true>(board, moves);
        generate_castling::<Type, true>(board, moves);
    } else {
        generate_pawn_moves::<Type, false>(board, moves);
        generate_non_sliding_moves::<Type, false>(board, moves);
        generate_sliding_moves::<Type, false>(board, moves);
        generate_castling::<Type, false>(board, moves);
    }
}

/// Calculates all legal pawn moves for `board` and puts them in `moves`.
// god dammit this function could be so much shorter if full const generics
// existed
#[rustfmt::skip]
fn generate_pawn_moves<Type: MovesType, const IS_WHITE: bool>(
    board: &Board,
    moves: &mut Moves,
) {
    let penultimate_rank = if IS_WHITE {
        Bitboard::rank_bb(Rank::RANK7)
    } else {
        Bitboard::rank_bb(Rank::RANK2)
    };
    let double_push_rank = if IS_WHITE {
        Bitboard::rank_bb(Rank::RANK4)
    } else {
        Bitboard::rank_bb(Rank::RANK5)
    };
    let forward = if IS_WHITE { Direction::N } else { Direction::S };
    let forward_right = if IS_WHITE { Direction::NE } else { Direction::SE };
    let forward_left = if IS_WHITE { Direction::NW } else { Direction::SW };
    let us_bb = board.side::<IS_WHITE>();
    let occupancies = board.occupancies();
    let them_bb = occupancies ^ us_bb;
    let empty = !occupancies;
    let ep_square = board.ep_square();
    let pawns = board.piece::<{ PieceType::PAWN.to_index() }>() & us_bb;

    let normal_pawns = pawns & !penultimate_rank;
    let promotion_pawns = pawns & penultimate_rank;

    // regular pushes
    if Type::NON_KING_QUIETS {
        let single_push = normal_pawns.pawn_push::<IS_WHITE>() & empty;
        let double_push = single_push.pawn_push::<IS_WHITE>() & empty & double_push_rank;

        for dest_pawn in single_push {
            moves.push(Move::new(dest_pawn - forward, dest_pawn));
        }
        for dest_pawn in double_push {
            moves.push(Move::new(dest_pawn - forward - forward, dest_pawn));
        }
    }

    if Type::CAPTURES {
        // regular captures
        let right_captures = if IS_WHITE {
            normal_pawns.north().east() & them_bb
        } else {
            normal_pawns.south().east() & them_bb
        };
        let left_captures = if IS_WHITE {
            normal_pawns.north().west() & them_bb
        } else {
            normal_pawns.south().west() & them_bb
        };

        for dest_pawn in right_captures {
            moves.push(Move::new(dest_pawn - forward_right, dest_pawn));
        }
        for dest_pawn in left_captures {
            moves.push(Move::new(dest_pawn - forward_left, dest_pawn));
        }

        // en passant
        if ep_square != Square::NONE {
            let attackers = if IS_WHITE {
                LOOKUPS.pawn_attacks(Side::BLACK, ep_square) & normal_pawns
            } else {
                LOOKUPS.pawn_attacks(Side::WHITE, ep_square) & normal_pawns
            };

            for pawn in attackers {
                moves.push(Move::new_en_passant(pawn, ep_square));
            }
        }
    }

    // promotions: both pushes and captures
    let single_push = promotion_pawns.pawn_push::<IS_WHITE>() & empty;
    let right_captures = if IS_WHITE {
        promotion_pawns.north().east() & them_bb
    } else {
        promotion_pawns.south().east() & them_bb
    };
    let left_captures = if IS_WHITE {
        promotion_pawns.north().west() & them_bb
    } else {
        promotion_pawns.south().west() & them_bb
    };

    if Type::NON_KING_QUIETS {
        for dest_pawn in single_push {
            let origin = dest_pawn - forward;
            moves.push(Move::new_promo::<{ PieceType::KNIGHT.0 }>(origin, dest_pawn));
            moves.push(Move::new_promo::<{ PieceType::BISHOP.0 }>(origin, dest_pawn));
            moves.push(Move::new_promo::<{ PieceType::ROOK.0 }>(origin, dest_pawn));
        }
    }
    if Type::CAPTURES {
        for dest_pawn in single_push {
            let origin = dest_pawn - forward;
            moves.push(Move::new_promo::<{ PieceType::QUEEN.0 }>(origin, dest_pawn));
        }
        for dest_pawn in right_captures {
            let origin = dest_pawn - forward_right;
            moves.push(Move::new_promo::<{ PieceType::KNIGHT.0 }>(origin, dest_pawn));
            moves.push(Move::new_promo::<{ PieceType::BISHOP.0 }>(origin, dest_pawn));
            moves.push(Move::new_promo::<{ PieceType::ROOK.0 }>(origin, dest_pawn));
            moves.push(Move::new_promo::<{ PieceType::QUEEN.0 }>(origin, dest_pawn));
        }
        for dest_pawn in left_captures {
            let origin = dest_pawn - forward_left;
            moves.push(Move::new_promo::<{ PieceType::KNIGHT.0 }>(origin, dest_pawn));
            moves.push(Move::new_promo::<{ PieceType::BISHOP.0 }>(origin, dest_pawn));
            moves.push(Move::new_promo::<{ PieceType::ROOK.0 }>(origin, dest_pawn));
            moves.push(Move::new_promo::<{ PieceType::QUEEN.0 }>(origin, dest_pawn));
        }
    }
}

/// Calculates all legal knight and king moves (excluding castling) for `board`
/// and puts them in `moves`.
#[allow(clippy::assertions_on_constants)]
fn generate_non_sliding_moves<Type: MovesType, const IS_WHITE: bool>(
    board: &Board,
    moves: &mut Moves,
) {
    let us_bb = board.side::<IS_WHITE>();

    if Type::NON_KING_QUIETS || Type::CAPTURES {
        let them_bb = if IS_WHITE {
            board.side::<false>()
        } else {
            board.side::<true>()
        };

        let knight_target_squares = if Type::NON_KING_QUIETS {
            if Type::CAPTURES {
                !us_bb
            } else {
                !us_bb ^ them_bb
            }
        } else {
            them_bb
        };

        let knights = board.piece::<{ PieceType::KNIGHT.to_index() }>() & us_bb;
        for knight in knights {
            let targets = LOOKUPS.knight_attacks(knight) & knight_target_squares;
            for target in targets {
                moves.push(Move::new(knight, target));
            }
        }
    }

    if Type::KING_QUIETS || Type::CAPTURES {
        let them_bb = if IS_WHITE {
            board.side::<false>()
        } else {
            board.side::<true>()
        };

        let king_target_squares = if Type::KING_QUIETS {
            if Type::CAPTURES {
                !us_bb
            } else {
                !us_bb ^ them_bb
            }
        } else {
            them_bb
        };

        let mut kings = board.piece::<{ PieceType::KING.to_index() }>() & us_bb;
        debug_assert!(
            kings.0.count_ones() == 1,
            "Number of kings is not equal to one"
        );
        let king = kings.pop_next_square();
        let targets = LOOKUPS.king_attacks(king) & king_target_squares;
        for target in targets {
            moves.push(Move::new(king, target));
        }
    }
}

/// Generates all legal bishop, rook and queen moves for `board` and puts them
/// in `moves`.
fn generate_sliding_moves<Type: MovesType, const IS_WHITE: bool>(board: &Board, moves: &mut Moves) {
    if !Type::NON_KING_QUIETS && !Type::CAPTURES {
        return;
    }

    let us_bb = board.side::<IS_WHITE>();
    let occupancies = board.occupancies();
    let them_bb = us_bb ^ occupancies;

    let target_squares = if Type::NON_KING_QUIETS {
        if Type::CAPTURES {
            !us_bb
        } else {
            !us_bb ^ them_bb
        }
    } else {
        them_bb
    };

    let bishops = board.piece::<{ PieceType::BISHOP.to_index() }>() & us_bb;
    for bishop in bishops {
        let targets = LOOKUPS.bishop_attacks(bishop, occupancies) & target_squares;
        for target in targets {
            moves.push(Move::new(bishop, target));
        }
    }

    let rooks = board.piece::<{ PieceType::ROOK.to_index() }>() & us_bb;
    for rook in rooks {
        let targets = LOOKUPS.rook_attacks(rook, occupancies) & target_squares;
        for target in targets {
            moves.push(Move::new(rook, target));
        }
    }

    let queens = board.piece::<{ PieceType::QUEEN.to_index() }>() & us_bb;
    for queen in queens {
        let targets = LOOKUPS.queen_attacks(queen, occupancies) & target_squares;
        for target in targets {
            moves.push(Move::new(queen, target));
        }
    }
}

/// Generates the castling moves for the given side and puts them in `moves`.
fn generate_castling<Type: MovesType, const IS_WHITE: bool>(board: &Board, moves: &mut Moves) {
    if !Type::NON_KING_QUIETS {
        return;
    }

    let occupancies = board.occupancies();

    if board.castling_rights().can_castle_kingside::<IS_WHITE>()
        && Bitboard::is_clear_to_castle_const::<IS_WHITE, true>(occupancies)
    {
        moves.push(Move::new_castle::<IS_WHITE, true>());
    }
    if board.castling_rights().can_castle_queenside::<IS_WHITE>()
        && Bitboard::is_clear_to_castle_const::<IS_WHITE, false>(occupancies)
    {
        moves.push(Move::new_castle::<IS_WHITE, false>());
    }
}
