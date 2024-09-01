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
    defs::{Direction, PieceType, Rank, Side, Square},
    evaluation::CompressedEvaluation,
    lookups::ATTACK_LOOKUPS,
    search::Histories,
};

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
    pub score: CompressedEvaluation,
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

/// Maximum number of legal moves that can be reached in a standard chess game.
///
/// Example: `R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 w - - 0 1`
pub const MAX_LEGAL_MOVES: usize = 218;

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
    pub const WINNING_CAPTURE_SCORE: CompressedEvaluation = CompressedEvaluation(0x2000);
    /// The score of a quiet move.
    pub const QUIET_SCORE: CompressedEvaluation = CompressedEvaluation(0x1000);
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
    fn new(mv: Move) -> Self {
        Self {
            mv,
            score: CompressedEvaluation::default(),
        }
    }

    /// Scores `self.mv`.
    #[allow(clippy::assertions_on_constants)]
    pub fn score<Type: MovesType>(&mut self, board: &Board, histories: &Histories) {
        assert!(
            Type::CAPTURES ^ (Type::KING_QUIETS || Type::NON_KING_QUIETS),
            "must be scoring exactly one of quiet moves or captures"
        );

        let mv = self.mv;
        let start = mv.start();
        let end = mv.end();
        // OMG THIS IS DUMB. FULL CONST GENERICS WHEN.
        let captured_type = if Type::CAPTURES {
            Histories::captured_piece_type::<true>(board, mv, end)
        } else {
            Histories::captured_piece_type::<false>(board, mv, end)
        };

        // If a move doesn't capture anything but `Type::CAPTURES` is true, the
        // score will be as if it's a capture. This is so queen promotions
        // (even quiet ones) can be treated as captures.
        self.score += if Type::CAPTURES {
            let piece = board.piece_on(start);

            // Pre-emptively give the capture a winning score - it can be
            // checked later.
            Self::WINNING_CAPTURE_SCORE
                + captured_type.mvv_bonus()
                + histories.get_capture_score(piece, captured_type, end)
        } else {
            Self::QUIET_SCORE + histories.get_butterfly_score(board.side_to_move(), start, end)
        };
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
                ATTACK_LOOKUPS.pawn_attacks(Side::BLACK, ep_square) & normal_pawns
            } else {
                ATTACK_LOOKUPS.pawn_attacks(Side::WHITE, ep_square) & normal_pawns
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
            let targets = ATTACK_LOOKUPS.knight_attacks(knight) & knight_target_squares;
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
        let targets = ATTACK_LOOKUPS.king_attacks(king) & king_target_squares;
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
        let targets = ATTACK_LOOKUPS.bishop_attacks(bishop, occupancies) & target_squares;
        for target in targets {
            moves.push(Move::new(bishop, target));
        }
    }

    let rooks = board.piece::<{ PieceType::ROOK.to_index() }>() & us_bb;
    for rook in rooks {
        let targets = ATTACK_LOOKUPS.rook_attacks(rook, occupancies) & target_squares;
        for target in targets {
            moves.push(Move::new(rook, target));
        }
    }

    let queens = board.piece::<{ PieceType::QUEEN.to_index() }>() & us_bb;
    for queen in queens {
        let targets = ATTACK_LOOKUPS.queen_attacks(queen, occupancies) & target_squares;
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
