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

//! Extracting positions from games.
//!
//! This module allows for parsing through a large number of games and
//! extracting positions from them. A [`BufferedReader`] feeds games to a
//! [`GameSampler`], which periodically does a high-depth search on the
//! positions inside the game, applies the PV, and prints the result. Given a
//! large number of games, this means it will produce a large number of
//! positions with their results.

use std::{
    fs, io,
    num::NonZero,
    str::from_utf8,
    sync::{
        mpsc::{channel, Receiver},
        Mutex,
    },
    thread::{available_parallelism, scope},
    time::{Duration, Instant},
};

use pgn_reader::{BufferedReader, CastlingSide, RawHeader, Role, San, SanPlus, Visitor};

use crate::{
    bitboard::Bitboard,
    board::{Board, STARTPOS},
    defs::{self, Piece, PieceType, Side},
    engine::ZobristStack,
    movegen::{Move, LOOKUPS},
    search::{iterative_deepening::iterative_deepening, Depth, Limits, SearchReferences},
    transposition_table::TranspositionTable,
};

/// A cache of parameters used for [`SearchReferences::new()`].
#[allow(clippy::missing_docs_in_private_items)]
struct SearchReferencesCache {
    start: Instant,
    can_print: bool,
    limits: Limits,
    allocated: Duration,
    uci_rx: Mutex<Receiver<String>>,
    tt: TranspositionTable,
}

/// Parses a game, periodically doing a high-depth search on each position,
/// applying the PV, and outputting the resulting position and result.
///
/// It extracts a maximum of [`SAMPLE_LIMIT`](Self::SAMPLE_LIMIT) positions and
/// does a depth [`SEARCH_DEPTH`](SearchReferencesCache::SEARCH_DEPTH) search.
///
/// The output is to stdout in the format `format!("{board} {result}")`, where
/// `result` is `1-0`, `0-1` or `1/2-1/2`.
struct GameSampler {
    /// Cached fields of [`SearchReferences`].
    search_refs_cache: SearchReferencesCache,
    /// The state of the game so far.
    board: Board,
    /// How many halfmoves to go until the next analysis.
    moves_until_sample: u8,
    /// How many positions have been sampled from this game.
    sampled_positions: u8,
    /// The result of the current game.
    result: String,
}

impl GameSampler {
    /// The period (in halfmoves) of sampling a game.
    const SAMPLE_INTERVAL: u8 = 16;
    /// The maximum number of positions that can be sampled from a game.
    const SAMPLE_LIMIT: u8 = 3;
}

impl SearchReferencesCache {
    /// The hash size (in MiB) of the transposition table.
    const HASH_SIZE_MIB: usize = 16;
    /// The depth to which each position is searched.
    const SEARCH_DEPTH: Depth = 10;
}

impl From<pgn_reader::File> for defs::File {
    fn from(file: pgn_reader::File) -> Self {
        Self(file as u8)
    }
}

impl Visitor for GameSampler {
    type Result = Result<(), ()>;

    fn end_game(&mut self) -> Self::Result {
        self.search_refs_cache.tt.clear();
        self.moves_until_sample = Self::SAMPLE_LIMIT / 2;
        self.sampled_positions = 0;
        Ok(())
    }

    fn header(&mut self, key: &[u8], value: RawHeader<'_>) {
        match from_utf8(key).expect("could not convert key to UTF-8") {
            "FEN" => {
                self.board = value
                    .decode_utf8()
                    .expect("could not convert value of header into UTF-8")
                    .parse()
                    .expect("could not parse FEN string");
            }
            "Result" => {
                self.result = value
                    .decode_utf8()
                    .expect("could not convert value of header into UTF-8")
                    .into_owned();
            }
            _ => (),
        }
    }

    fn san(&mut self, san_plus: SanPlus) {
        if self.has_reached_sample_limit() {
            return;
        }

        self.try_sampling();
        self.make_san_move(san_plus.san);
    }
}

impl From<Role> for PieceType {
    fn from(role: Role) -> Self {
        // they have the same order, except it's an enum so each value is 1
        // higher
        Self(role as u8 - 1)
    }
}

impl From<pgn_reader::Rank> for defs::Rank {
    fn from(rank: pgn_reader::Rank) -> Self {
        Self(rank as u8)
    }
}

impl From<pgn_reader::Square> for defs::Square {
    fn from(square: pgn_reader::Square) -> Self {
        Self(square as u8)
    }
}

impl GameSampler {
    /// Creates a new [`GameSampler`].
    fn new() -> Self {
        Self {
            search_refs_cache: SearchReferencesCache::new(),
            // SAFETY: a hardcoded startpos cannot fail to be parsed
            board: unsafe { STARTPOS.parse().unwrap_unchecked() },
            moves_until_sample: Self::SAMPLE_LIMIT / 2,
            result: String::from("1/2-1/2"),
            sampled_positions: 0,
        }
    }

    /// Checks if the game has reached the maximum number of sampled positions.
    const fn has_reached_sample_limit(&self) -> bool {
        self.sampled_positions == Self::SAMPLE_LIMIT
    }

    /// Decrements the sample counter if it's above 0 or re-sets the counter
    /// and samples the position if it's at 0.
    fn try_sampling(&mut self) {
        if self.moves_until_sample > 0 {
            self.moves_until_sample -= 1;
            return;
        }

        self.sample_position();
        self.moves_until_sample = Self::SAMPLE_INTERVAL;
    }

    /// Runs a deep-ish search on the current position, applies the moves in
    /// the PV, then prints the resulting board and its result to stdout in the
    /// format `format!("{board} {result}")`.
    fn sample_position(&mut self) {
        let pv = iterative_deepening(self.search_refs_cache.new_search_refs(), self.board).pv;

        let mut copy = self.board;
        for mv in pv {
            copy.make_move(mv);
        }
        println!("{copy} {}", self.result);

        self.sampled_positions += 1;
    }

    /// Makes a move that is in Standard Algebraic Notation.
    #[allow(clippy::needless_pass_by_value)]
    fn make_san_move(&mut self, san: San) {
        match san {
            San::Normal {
                role,
                file,
                rank,
                capture,
                to,
                promotion,
            } => {
                let file = file.map(defs::File::from);
                let rank = rank.map(defs::Rank::from);
                let end = defs::Square::from(to);
                let promotion_piece = promotion.map(PieceType::from);

                self.make_non_castle_move(role, file, rank, capture, end, promotion_piece);
            }
            San::Castle(side) => self.make_castling_move(side),
            San::Put { .. } => panic!("there shouldn't be a Put in standard chess"),
            San::Null => panic!("null move"),
        }
    }

    /// Makes a non-castling move from the given information about it.
    fn make_non_castle_move(
        &mut self,
        role: Role,
        file: Option<defs::File>,
        rank: Option<defs::Rank>,
        capture: bool,
        end: defs::Square,
        promotion: Option<PieceType>,
    ) {
        let possible_pieces = match role {
            Role::Pawn => {
                let pawns = self.board.piece::<{ PieceType::PAWN.to_index() }>();
                let us = self.board.side_to_move();
                let them = us.flip();

                if capture {
                    LOOKUPS.pawn_attacks(them, end) & pawns
                } else {
                    let end_bb = Bitboard::from(end);
                    if us == Side::WHITE {
                        let start_bb = end_bb.south();
                        if (start_bb & pawns).0.count_ones() == 1 {
                            start_bb
                        } else {
                            start_bb.south()
                        }
                    } else {
                        let start_bb = end_bb.north();
                        if (start_bb & pawns).0.count_ones() == 1 {
                            start_bb
                        } else {
                            start_bb.north()
                        }
                    }
                }
            }
            Role::Knight => {
                LOOKUPS.knight_attacks(end) & self.board.piece::<{ PieceType::KNIGHT.to_index() }>()
            }
            Role::Bishop => {
                LOOKUPS.bishop_attacks(end, self.board.occupancies())
                    & self.board.piece::<{ PieceType::BISHOP.to_index() }>()
            }
            Role::Rook => {
                LOOKUPS.rook_attacks(end, self.board.occupancies())
                    & self.board.piece::<{ PieceType::ROOK.to_index() }>()
            }
            Role::Queen => {
                LOOKUPS.queen_attacks(end, self.board.occupancies())
                    & self.board.piece::<{ PieceType::QUEEN.to_index() }>()
            }
            Role::King => {
                LOOKUPS.king_attacks(end) & self.board.piece::<{ PieceType::KING.to_index() }>()
            }
        } & self.board.side_any(self.board.side_to_move());

        #[allow(clippy::option_if_let_else)]
        let start_bb = if let Some(file) = file {
            possible_pieces & Bitboard::file_bb(file)
        } else if let Some(rank) = rank {
            possible_pieces & Bitboard::rank_bb(rank)
        } else {
            possible_pieces
        };

        // there can be more than 1 piece (rarely) since one piece can be
        // pinned (meaning they didn't need to specify the rank or file)
        for start in start_bb {
            let mut copy = self.board;
            let mv = if let Some(piece_type) = promotion {
                Move::new_promo_any(start, end, piece_type)
            } else if capture && self.board.piece_on(end) == Piece::NONE {
                Move::new_en_passant(start, end)
            } else {
                Move::new(start, end)
            };

            if copy.make_move(mv) {
                self.board = copy;
                return;
            }
        }

        panic!("illegal move: {role:?} to {end}");
    }

    /// Makes a castling move for the given side.
    fn make_castling_move(&mut self, side: CastlingSide) {
        let mv = if self.board.side_to_move() == Side::WHITE {
            match side {
                CastlingSide::KingSide => Move::new_castle::<true, true>(),
                CastlingSide::QueenSide => Move::new_castle::<true, false>(),
            }
        } else {
            match side {
                CastlingSide::KingSide => Move::new_castle::<false, true>(),
                CastlingSide::QueenSide => Move::new_castle::<false, false>(),
            }
        };

        assert!(self.board.make_move(mv), "illegal move: \"{mv}\"");
    }
}

impl SearchReferencesCache {
    /// Creates a new [`SearchReferencesCache`].
    fn new() -> Self {
        Self {
            start: Instant::now(),
            can_print: false,
            limits: Limits::Depth(Self::SEARCH_DEPTH),
            allocated: Duration::MAX,
            uci_rx: Mutex::new(channel().1),
            tt: TranspositionTable::with_capacity(Self::HASH_SIZE_MIB),
        }
    }

    /// Creates a new [`SearchReferences`] out of the cached information.
    const fn new_search_refs(&self) -> SearchReferences<'_> {
        SearchReferences::new(
            self.start,
            self.can_print,
            self.limits,
            self.allocated,
            &self.uci_rx,
            ZobristStack::new(),
            &self.tt,
        )
    }
}

/// Sample and analyse the given games.
///
/// `args` is expected to be a list of files each containing a series of games
/// in PGN. Each file is read through and each game is fed to a
/// [`GameSampler`]. It is done concurrently.
#[allow(clippy::print_stderr)]
pub fn sample_from_games<T>(args: T)
where
    T: Iterator<Item = String>,
{
    let total_threads = available_parallelism().map_or(1, NonZero::get);

    for file_name in args {
        let file_name = &file_name;

        eprintln!("Sampling file {file_name}");
        scope(|s| {
            let mut handles = Vec::new();

            eprintln!("Spawning threads...");
            for thread_id in 0..total_threads {
                handles.push(s.spawn(move || sample_file(file_name, thread_id, total_threads)));
            }

            eprintln!("Waiting for threads to finish...");
            for (thread_id, handle) in handles.into_iter().enumerate() {
                if let Err(e) = handle.join() {
                    panic!("error while joining thread id {thread_id}: {e:?}");
                }
            }
            eprintln!("Joined threads");
        });
    }
}

/// Samples an individual file with the given thread ID and total number of
/// threads.
///
/// Across all threads, `thread_id` is expected to be unique and encompass
/// every ID in the range `0..total_threads`. For example, if `total_threads`
/// is 4, there are expected to be 4 threads with `thread_id`s of `0`, `1`, `2`
/// and `3`.
fn sample_file(file_name: &str, thread_id: usize, total_threads: usize) -> io::Result<()> {
    let file = fs::File::open(file_name)?;
    let mut game_reader = BufferedReader::new(file);
    let mut sampler = GameSampler::new();

    // skip over an initial number of games unique to this thread
    for _ in 0..thread_id {
        if !game_reader.skip_game::<GameSampler>()? {
            return Ok(());
        }
    }

    loop {
        if game_reader.read_game(&mut sampler)?.is_none() {
            break;
        }

        // skip over the games that the other `total_threads - 1`
        // threads will handle
        for _ in 0..(total_threads - 1) {
            if !game_reader.skip_game::<GameSampler>()? {
                break;
            }
        }
    }

    Ok(())
}
