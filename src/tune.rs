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

//! Tuning the evaluation values.
//!
//! This module is for tuning the hardcoded values used in
//! [`crate::evaluation`]. It does this with a gradient descent method, using
//! large number of positions along with their result.
//!
//! # Overview of the mathematical terms
//!
//! Each value is called a weight, and in this case, it's either a piece value
//! or a piece-square table value. Each weight is multiplied by a coefficient:
//! the sum of the weights multiplied by the coefficients is the evaluation.
//! This coefficient is equal to a White coefficient minus a Black coefficient.
//! For example, the coefficient of the pawn weight is the number of White
//! pawns minus the number of Black pawns. In mathematical terms, the weights
//! and both coefficients are linear vectors: I will call the weights `L`,
//! White coefficients `Cw`, Black coefficients `Cb` and the evaluation (from
//! the perspective of White) `E`. `E` is the dot product of `L` and `Cw - Cb`:
//! `E = L . (Cw - Cb)`.
//!
//! # Rough explanation of gradient descent
//!
//! The evaluation can be used to predict a game result via a sigmoid:
//! `1 / (1 + e^(-KE))`. (Graph it if you want.) The error is the result of a
//! game (1.0 for White winning, 0.5 for a draw and 0.0 for Black winning)
//! minus the sigmoid, squared. To minimise the error, the partial derivative
//! of the error with respect to the weights is calcuated (which can also be
//! called the gradient), and each weight is adjusted by this derivative
//! multiplied by the learning rate. The learning rate is used to control the
//! rate at which the weights are adjusted. The learning rate can be lowered
//! indefinitely but there are diminishing returns.
//!
//! Think of a small ball bouncing downwards into a bowl: the ball is the
//! weights, the gradient is how steep the walls of the bowl are, the learning
//! rate is the distance between the bounces and the error is the height of the
//! ball. (The time between bounces is constant no matter what the learning
//! rate is.) The error needs to be minimised, which corresponds to the lowest
//! point of the bowl. If the ball bounces past the minimum, the height will
//! start increasing. If the distance between the bounces is kept high, it will
//! continue to bounce back and forth past the minumum without getting any
//! closer. Conversely, if the distance starts low, it will take an extremely
//! large number of steps to get close enough to the minimum. For the same
//! reason, the learning rate starts high initially but decreases each time the
//! error increases (i.e. bounces past the minimum).

use std::{
    convert::From,
    fs,
    io::{BufRead, BufReader},
    iter::Sum,
    num::NonZero,
    ops::{Add, AddAssign, Div, Mul},
    str::FromStr,
    thread::{available_parallelism, scope},
};

use crate::{
    board::Board,
    defs::{self, Piece, PieceType, Rank, Side, Square},
    error::ParseError,
    evaluation::{
        values::{BASE_PIECE_VALUES, INITIAL_PIECE_SQUARE_TABLES},
        Eval, Score,
    },
};

/// A vector of integer coefficients.
type Coefficients = Vec<Coefficient>;
/// A vector of tune entries.
type TuneEntries = Vec<TuneEntry>;
/// A vector of linear weights.
type Weights = Vec<ScoreF64>;

/// An integer coefficient.
struct Coefficient {
    /// The value.
    ///
    /// 0 should never happen (since 0 * weight = 0), positive is for White,
    /// negative is for Black.
    value: i8,
    /// Which weight it corresponds to.
    weight_index: usize,
}

/// A [`Score`] but with [`f64`]s.
#[derive(Clone, Copy)]
struct ScoreF64(pub f64, pub f64);

/// All the relevant information about a position required for tuning.
struct TuneEntry {
    /// How the game ended: 1.0 for White winning, 0.5 for a draw and 0.0 for
    /// Black winning.
    result: f64,
    /// The static evaluation of the position.
    eval: Eval,
    /// The phase of the game: see [`Phase`](crate::evaluation::Phase).
    phase: f64,
    /// The non-zero integer coefficients of the position.
    coefficients: Coefficients,
}

/// How many weights there are (i.e. how many parameters are being tuned).
///
/// The 6 piece values plus the 64 piece-square table values for each of the 6
/// pieces.
const TOTAL_WEIGHTS: usize = PieceType::TOTAL * Square::TOTAL + PieceType::TOTAL;

impl From<ScoreF64> for Score {
    fn from(score: ScoreF64) -> Self {
        Self(score.0.round() as Eval, score.1.round() as Eval)
    }
}

impl Add for ScoreF64 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl AddAssign for ScoreF64 {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl Div<f64> for ScoreF64 {
    type Output = Self;

    fn div(self, rhs: f64) -> Self {
        Self(self.0 / rhs, self.1 / rhs)
    }
}

impl Div for ScoreF64 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        Self(self.0 / rhs.0, self.1 / rhs.1)
    }
}

impl From<Score> for ScoreF64 {
    fn from(score: Score) -> Self {
        Self(f64::from(score.0), f64::from(score.1))
    }
}

impl Mul<f64> for ScoreF64 {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        Self(self.0 * rhs, self.1 * rhs)
    }
}

impl Mul<i8> for ScoreF64 {
    type Output = Self;

    fn mul(self, rhs: i8) -> Self {
        Self(self.0 * f64::from(rhs), self.1 * f64::from(rhs))
    }
}

impl Sum for ScoreF64 {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        let mut total_score = Self(0.0, 0.0);

        for score in iter {
            total_score += score;
        }

        total_score
    }
}

impl FromStr for TuneEntry {
    type Err = ParseError;

    /// Converts a string slice into a [`TuneEntry`].
    ///
    /// The slice is expected to be in the format
    /// `format!("{board} {result}")`. Excess whitespace is ignored.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let result_str = s.split_whitespace().next_back().ok_or(ParseError)?;
        let result = match result_str {
            "1-0" => 1.0,
            "1/2-1/2" => 0.5,
            "0-1" => 0.0,
            _ => return Err(ParseError),
        };

        // SAFETY: we're searching for a substring we just found in the string
        let result_str_idx = unsafe { s.rfind(result_str).unwrap_unchecked() };
        let fen = s.get(0..result_str_idx).ok_or(ParseError)?;
        let board = fen.parse::<Board>()?;
        let phase = board.phase();
        let eval = board.score().lerp_to(phase);
        let phase = f64::from(phase.min(24)) / 24.0;
        let coefficients = initialise_coefficients(&board);

        Ok(Self {
            result,
            eval,
            phase,
            coefficients,
        })
    }
}

impl Coefficient {
    /// Creates a new [`Coefficient`].
    const fn new(value: i8, weight_index: usize) -> Self {
        Self {
            value,
            weight_index,
        }
    }
}

impl ScoreF64 {
    /// Lerps the score between its middlegame and endgame value depending on
    /// the phase.
    fn lerp_to(self, phase: f64) -> f64 {
        let diff = self.1 - self.0;
        diff.mul_add(-phase, self.1) // self.1 - diff * phase
    }
}

/// Given a file of positions, tune the piece values and piece-square table
/// values given in [`crate::evaluation::values`] with the given intial
/// learning rate.
///
/// The tables are printed to stdout and diagnostic/error messages are printed
/// to stderr.
#[allow(clippy::print_stderr, clippy::similar_names)]
pub fn tune(positions: &str, mut learning_rate: f64) {
    let mut best_error = f64::MAX;
    let mut error;
    let mut adaptive_gradients = vec![ScoreF64(0.0, 0.0); TOTAL_WEIGHTS];
    let learning_rate_drop = 2.0;

    eprintln!("initialising weights...");
    let mut weights = initialise_current_weights();
    eprintln!("initialising tuner entries...");
    let tune_entries = initialise_tuner_entries(positions);
    eprintln!("calculating optimal K...");
    let (k, k_error) = calculate_optimal_k(&tune_entries);
    error = k_error;

    for iteration in 0.. {
        let gradients = calculate_gradient(&tune_entries, &weights, k);
        for ((weight, adaptive_gradient), gradient) in weights
            .iter_mut()
            .zip(adaptive_gradients.iter_mut())
            .zip(gradients.iter())
        {
            // this is the -2/N * K that was omitted from
            // `calculate_gradients()`
            // `weight` will be subtracting this later so `*weight -= -2 * xyz`
            // cancels both negatives
            let full_gradient = *gradient * 2.0 * k / tune_entries.len() as f64;

            let ScoreF64(mg, eg) = full_gradient;
            *adaptive_gradient += ScoreF64(mg.powi(2), eg.powi(2));
            let ScoreF64(ag_mg, ag_eg) = *adaptive_gradient;
            let adaptive_gradient_sqrt =
                ScoreF64(ag_mg.sqrt().max(0.0001), ag_eg.sqrt().max(0.0001));

            // `adaptive_gradient_sqrt` is the root of the sum of the squares
            // of the previous gradients for this weight. It's used to tailor
            // the learning rate for each weight individually.
            *weight += full_gradient * learning_rate / adaptive_gradient_sqrt;
        }

        eprintln!("iterations: {iteration}");

        if iteration % 64 == 0 {
            error = total_error(&tune_entries, k, |entry| evaluation(entry, &weights));
            // we've reached the furthest we can with this learning rate, so drop
            // it
            if error > best_error {
                learning_rate /= learning_rate_drop;
            }
            print_weights(&weights);
            best_error = error;
        }

        eprintln!("current error: {error}");
        eprintln!("learning rate: {learning_rate}");
    }
}

/// Given a file of positions, create a vector of [`TuneEntry`]s: one entry per
/// position & result.
///
/// The vector will be shrunk to fit.
fn initialise_tuner_entries(positions: &str) -> TuneEntries {
    let mut tune_entries = TuneEntries::new();
    let positions = BufReader::new(fs::File::open(positions).expect("could not open {positions}"));

    for line in positions
        .lines()
        .map(|line| line.expect("error while iterating through the file's lines"))
    {
        let tune_entry = line
            .parse()
            .expect("error while parsing a line into a TuneEntry");
        tune_entries.push(tune_entry);
    }

    tune_entries.shrink_to_fit();
    tune_entries
}

/// Given a board state, create a vector of non-zero [`Coefficient`]s.
///
/// The vector's length will always be equal to or smaller than the number of
/// pieces on the board. It will be shrunk to fit.
fn initialise_coefficients(board: &Board) -> Coefficients {
    let mut coefficients = Coefficients::new();

    for piece_type in 0..PieceType::TOTAL as u8 {
        let pieces = board.piece_any(PieceType(piece_type));
        let white_pieces = pieces & board.side::<true>();
        let piece_count_difference =
            white_pieces.0.count_ones() as i8 * 2 - pieces.0.count_ones() as i8;

        // if there is no difference, dE_dL in `update_gradients()` would be 0,
        // so it wouldn't contribute to the gradient at all
        if piece_count_difference != 0 {
            let coefficient = Coefficient::new(piece_count_difference, usize::from(piece_type));
            coefficients.push(coefficient);
        }
    }

    let starting_index = PieceType::TOTAL;
    // this is looping through the squares of the PSQT. The White piece index
    // of the PSQT is the same as `square`, and the Black index of the PSQT is
    // `square.flip()`.
    // For the same reason as the for loop above, the coefficient will only be added if
    // there's a White piece on the square and/or a Black piece on the opposite
    // square, unless they are the same piece.
    for square in 0..Square::TOTAL as u8 {
        let square = Square(square);
        let piece = board.piece_on(square);
        let piece_type = PieceType::from(piece);
        let piece_side = Side::from(piece);

        let opposite_square = square.flip();
        let opposite_piece = board.piece_on(opposite_square);
        let opposite_piece_type = PieceType::from(opposite_piece);
        let opposite_piece_side = Side::from(opposite_piece);

        // equal pieces of different sides cancel
        if piece_type == opposite_piece_type
            && (piece == Piece::NONE || piece_side != opposite_piece_side)
        {
            continue;
        }

        let index = usize::from(square.0) + starting_index;

        if piece != Piece::NONE && piece_side == Side::WHITE {
            let weight_index = usize::from(piece_type.0) * Square::TOTAL + index;
            let value = 1;
            let coefficient = Coefficient::new(value, weight_index);
            coefficients.push(coefficient);
        }
        if opposite_piece != Piece::NONE && opposite_piece_side == Side::BLACK {
            let weight_index = usize::from(opposite_piece_type.0) * Square::TOTAL + index;
            let value = -1;
            let coefficient = Coefficient::new(value, weight_index);
            coefficients.push(coefficient);
        }
    }

    coefficients.shrink_to_fit();
    coefficients
}

/// Create a vector of the initial weights from the constants in
/// [`crate::evaluation::values`].
///
/// The vector will be shrunk to fit.
fn initialise_current_weights() -> Weights {
    let mut weights = Weights::new();

    for score in BASE_PIECE_VALUES {
        weights.push(ScoreF64::from(score));
    }

    for table in INITIAL_PIECE_SQUARE_TABLES {
        for score in table {
            weights.push(score.into());
        }
    }

    weights.shrink_to_fit();
    weights
}

/// Calculate the optimal value of K used in the sigmoid.
///
/// Precision is given to about 12 decimal places.
///
/// Returns `(K, final error)`.
#[allow(clippy::print_stderr)]
fn calculate_optimal_k(tune_entries: &TuneEntries) -> (f64, f64) {
    let mut start = -10.0;
    let mut end = 10.0;
    let mut delta = 1.0;
    let mut epsilon = delta / 2.0;
    let mut best_value = start;
    let mut best_error = f64::MAX;

    let precision = 12;
    for _ in 0..precision {
        let mut current_value = start;

        // range `[start, end]` with step `delta`. `epsilon` makes sure `end`
        // is included.
        while current_value < end + epsilon {
            let error = total_error(tune_entries, current_value, |entry| f64::from(entry.eval));

            if error < best_error {
                best_error = error;
                best_value = current_value;
            }

            current_value += delta;
        }

        start = best_value - delta;
        end = best_value + delta;
        delta /= 10.0;
        epsilon /= 10.0;
    }

    (best_value, best_error)
}

/// Calculates the total error from the given evaluation function for each
/// position in `tune_entries`.
fn total_error<F>(tune_entries: &TuneEntries, k: f64, eval: F) -> f64
where
    F: Fn(&TuneEntry) -> f64,
{
    tune_entries
        .iter()
        .map(|entry| (entry.result - sigmoid(eval(entry), k)).powi(2))
        .sum::<f64>()
        .div(tune_entries.len() as f64)
}

/// Given a vector of [`TuneEntry`]s and a vector of weights, calculate the
/// gradient for each weight from the partial derivative of the error.
fn calculate_gradient(tune_entries: &TuneEntries, weights: &Weights, k: f64) -> Weights {
    let mut gradients = vec![ScoreF64(0.0, 0.0); TOTAL_WEIGHTS];

    let total_threads = available_parallelism().map_or(1, NonZero::get);
    let slice_len = tune_entries.len() / total_threads;

    scope(|s| {
        let mut handles = Vec::new();

        for thread in 0..(total_threads - 1) {
            let slice_index = (thread * slice_len)..((thread + 1) * slice_len);
            let tune_entries_slice = &tune_entries[slice_index];
            handles.push(
                s.spawn(move || calculate_gradient_for_slice(tune_entries_slice, weights, k)),
            );
        }
        let slice_index = ((total_threads - 1) * slice_len)..;
        let tune_entries_slice = &tune_entries[slice_index];
        handles.push(s.spawn(move || calculate_gradient_for_slice(tune_entries_slice, weights, k)));

        for handle in handles {
            for (gradient, local_gradient) in gradients
                .iter_mut()
                .zip(handle.join().expect("a thread panicked, somehow").iter())
            {
                *gradient += *local_gradient;
            }
        }
    });

    gradients
}

/// Same as [`calculate_gradient()`] but takes a slice of [`TuneEntry`]s
/// instead of a vector.
fn calculate_gradient_for_slice(tune_entries: &[TuneEntry], weights: &Weights, k: f64) -> Weights {
    let mut gradients = vec![ScoreF64(0.0, 0.0); TOTAL_WEIGHTS];

    // error = 1/N * sum of (R - sigmoid(E))², where N is tune_entries.len()
    // derror/dL = -2/N * sum of (R - sigmoid(E)) * dsigmoid/dE * dE/dL
    // this for loop goes over each E, and since dE/dL is the vector of
    // coefficients times the phase (for mg and eg), the inner for loop goes
    // over each coefficient
    // dsigmoid/dE contains a constant K, so it can be moved outside the
    // summation along with -2/N and both can be applied later
    #[allow(non_snake_case)]
    for entry in tune_entries {
        let sig = sigmoid(evaluation(entry, weights), k);
        let diff = entry.result - sig;
        let dsigmoid_dE = sig * (1.0 - sig);
        let mg_factor = entry.phase;
        let eg_factor = 1.0 - entry.phase;

        for coefficient in &entry.coefficients {
            let dE_dL = ScoreF64(mg_factor, eg_factor) * coefficient.value;
            gradients[coefficient.weight_index] += dE_dL * diff * dsigmoid_dE;
        }
    }

    gradients
}

/// Calculate the evaluation of a position from the given coefficients of
/// `tune_entry` and the weights.
fn evaluation(tune_entry: &TuneEntry, weights: &Weights) -> f64 {
    // `E = L . (Cw - Cb)`.
    let score = tune_entry
        .coefficients
        .iter()
        .map(|c| weights[c.weight_index] * c.value)
        .sum::<ScoreF64>();

    score.lerp_to(tune_entry.phase)
}

/// Calculates the sigmoid of an evaluation multiplied by the constant `k`.
fn sigmoid(eval: f64, k: f64) -> f64 {
    1.0 / (1.0 + (-k * eval).exp())
}

/// Prints the current best weights to stdout in the same format as is in the
/// source of [`crate::evaluation::values`].
#[allow(clippy::use_debug)]
fn print_weights(weights: &Weights) {
    let mut iter = weights.iter().map(|&w| Score::from(w));

    println!("pub const BASE_PIECE_VALUES: [Score; PieceType::TOTAL] = [");
    print!("   ");
    for _ in 0..PieceType::TOTAL {
        // SAFETY: first 6 terms of an array of length 390
        print!(" {:?},", unsafe { iter.next().unwrap_unchecked() });
    }
    println!("\n];");

    println!(
        "\npub const INITIAL_PIECE_SQUARE_TABLES: [[Score; Square::TOTAL]; PieceType::TOTAL] = ["
    );
    for _ in 0..PieceType::TOTAL {
        println!("    [");
        for _ in 0..Rank::TOTAL {
            print!("       ");
            for _ in 0..defs::File::TOTAL {
                // SAFETY: last 384 terms of an array of length 390
                print!(" {:?},", unsafe { iter.next().unwrap_unchecked() });
            }
            println!();
        }
        println!("    ],");
    }
    println!("];");
}
