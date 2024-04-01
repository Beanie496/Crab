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

/// An error that occurs when a string cannot be parsed.
#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum ParseError {
    /// A token was outside an expected range.
    ErroneousToken,
    /// Expected a token but found nothing.
    ExpectedToken,
    /// Expected a different token.
    InvalidToken,
}
