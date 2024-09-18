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
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    sync::{mpsc::channel, Mutex},
};

use oorandom::Rand64;

use crate::{
    board::{Board, STARTPOS},
    defs::Side,
    evaluation::Evaluation,
    movegen::{generate_moves, AllMoves, Moves},
    search::{AspirationWindow, Depth, Height, Limits, Pv, RootNode, SharedState, Worker},
    transposition_table::TranspositionTable,
};

/// The closest an opening's score can be to 0 while still being discarded.
const MAX_SCORE: Evaluation = Evaluation(120);
/// The furthest an opening's score can be away from 0 while still being
/// discarded.
const MIN_SCORE: Evaluation = Evaluation(80);
/// How deeply each position should be searched to get an evaluation.
const SEARCH_DEPTH: Depth = Depth(3);

/// Generate a set of FEN strings to be used as openings given `args`.
///
/// This function assumes that the openings generated in this one call are a
/// small batch of many more openings needed. Consequently, the number of total
/// openings needs to be given (even though they won't necessarily be
/// generated) as well as the number of openings to be generated now. These two
/// numbers can be equal, however.
///
/// `args` is in the format `["<N>", "seed", "<S>", "book",
/// "<None|path/to/some_book.epd>", "[T]", "[s]"]`, where `N` is the number of
/// openings to be generated; `S` is a seed; `path/to/some_book.epd` is the
/// path to a given book to be used to generate FEN strings off (or,
/// alternatively, `"None"` to use no opening book); `T` is the number of
/// openings required in total (but not to be generated now), where `T >= N`;
/// `s` is the side that the opening book favours (if any). Note that `T` and
/// `s` can be omitted - this is not true of any of the other arguments.
pub fn generate_fens<'a, T>(args: T)
where
    T: Iterator<Item = &'a str>,
{
    let (total_generated_openings, total_required_openings, seed, book_path, favoured_side) =
        parse_args(args);
    let mut rng = Rand64::new(u128::from(seed));
    let base_openings = book_path.map_or_else(
        || vec![String::from(STARTPOS)],
        |path| {
            let file = File::open(path).expect("could not open the given book path");
            BufReader::new(file)
                .lines()
                .map(|line| line.expect("could not create a string from a line"))
                .collect()
        },
    );
    let total_base_openings = base_openings.len();
    let average_openings_per_base_opening = total_required_openings / total_base_openings + 2;

    let mut pv = Pv::new();
    let rx = Mutex::new(channel().1);
    let tt = TranspositionTable::with_capacity(32);
    let state = SharedState::new(rx, tt);
    let mut worker = Worker::new(&state, 0)
        .with_printing(false)
        .with_limits(Limits::Infinite);

    // generate up to `average_openings_per_base_opening` number of openings
    // per base opening until there are no more left to generate
    let mut remaining_openings = total_generated_openings;
    while remaining_openings > 0 {
        let opening = &base_openings[rng.rand_range(0..total_base_openings as u64) as usize];
        let this_iteration_openings = average_openings_per_base_opening.min(remaining_openings);

        let board = opening
            .parse::<Board>()
            .unwrap_or_else(|_| panic!("Error while parsing \"{opening}\""));
        worker.reset_board(&board);

        let (alpha, beta) = if favoured_side == board.side_to_move() {
            (MIN_SCORE, MAX_SCORE)
        } else if favoured_side != Side::NONE {
            (-MAX_SCORE, -MIN_SCORE)
        } else {
            let base_score =
                worker.aspiration_loop(&mut pv, &mut AspirationWindow::new(), SEARCH_DEPTH);
            if base_score > 0 {
                (MIN_SCORE, MAX_SCORE)
            } else {
                (-MAX_SCORE, -MIN_SCORE)
            }
        };

        remaining_openings -= this_iteration_openings
            - generate_openings_for_board(
                &mut worker,
                &mut pv,
                &board,
                this_iteration_openings,
                // make the function play more and more random moves into the
                // future as the openings required increases
                Depth(2 + f32::ln(this_iteration_openings as f32).ceil() as i16),
                alpha,
                beta,
                &mut rng,
            );
    }
}

/// Parse the arguments as given to `generate_fens()`.
///
/// Returns `(N, T, S, path/to/some_book.epd, s)`.
///
/// See its documentation for more detail.
fn parse_args<'a, T>(mut args: T) -> (usize, usize, u64, Option<&'a Path>, Side)
where
    T: Iterator<Item = &'a str>,
{
    let required_openings = args
        .next()
        .expect("expected number of openings after \"genfens\"")
        .parse()
        .expect("number of openings could not be parsed");

    assert!(
        args.next().is_some_and(|s| s == "seed"),
        "expected token \"seed\" after number of openings"
    );
    let seed = args
        .next()
        .expect("expected seed after \"seed\"")
        .parse()
        .expect("seed could not be parsed");

    assert!(
        args.next().is_some_and(|s| s == "book"),
        "expected token \"book\" after seed"
    );
    let book = args.next().expect("expected book after \"book\"");
    let book_path = if book == "None" {
        None
    } else {
        Some(Path::new(book))
    };

    let total_openings = args.next().map_or(required_openings, |t| {
        t.parse().expect("number of openings could not be parsed")
    });

    let side = args.next().map_or(Side::NONE, |s| {
        s.parse().expect("favoured side could not be parsed")
    });

    (required_openings, total_openings, seed, book_path, side)
}

/// Prints as many openings as it can, up to `required_openings`, that are
/// `depth` halfmoves after the given board, returning the number of openings
/// it failed to generate.
///
/// `search_refs` is used for searches to check the openings are unbalanced but
/// not too unbalanced.
#[allow(clippy::too_many_arguments)]
fn generate_openings_for_board(
    worker: &mut Worker<'_>,
    pv: &mut Pv,
    board: &Board,
    mut required_openings: usize,
    depth: Depth,
    alpha: Evaluation,
    beta: Evaluation,
    rng: &mut Rand64,
) -> usize {
    if depth == 0 {
        println!("info string genfens {board}");
        return required_openings - 1;
    }

    let mut all_moves = Moves::new();
    generate_moves::<AllMoves>(board, &mut all_moves);

    while let Some(mv) = all_moves.pop_random(rng).map(|scored_move| scored_move.mv) {
        let mut copy = *board;
        if !worker.make_move(&mut copy, mv) {
            continue;
        }

        let score =
            -worker.search::<RootNode>(pv, &copy, -beta, -alpha, SEARCH_DEPTH, Height::default());

        worker.unmake_move();

        if score <= alpha || score >= beta {
            continue;
        }

        required_openings = generate_openings_for_board(
            worker,
            pv,
            &copy,
            required_openings,
            depth - 1,
            -beta,
            -alpha,
            rng,
        );

        if required_openings == 0 {
            break;
        }
    }

    required_openings
}
