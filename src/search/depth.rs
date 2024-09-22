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
    ops::{Add, AddAssign, Div, Mul, Sub, SubAssign},
};

use crate::evaluation::{CompressedEvaluation, Evaluation};

/// A [`Depth`] with half the size.
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct CompressedDepth(pub u8);

/// The difference between leaf node and the current node.
#[derive(Clone, Copy, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Depth(pub i16);

/// The difference between the root node and the current node.
#[derive(Clone, Copy, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Height(pub u8);

impl From<Depth> for CompressedDepth {
    fn from(depth: Depth) -> Self {
        debug_assert!(
            depth >= 0 && depth <= Depth::MAX,
            "converting a Depth ({depth}) outside the permissible range for a CompressedDepth",
        );
        Self(depth.0 as u8)
    }
}

impl Add for Depth {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl Add<u8> for Depth {
    type Output = Self;

    fn add(self, other: u8) -> Self::Output {
        self + Self(other.into())
    }
}

impl AddAssign for Depth {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl AddAssign<u8> for Depth {
    fn add_assign(&mut self, other: u8) {
        *self += Self(other.into());
    }
}

impl Display for Depth {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Div for Depth {
    type Output = Self;

    fn div(self, other: Self) -> Self::Output {
        Self(self.0 / other.0)
    }
}

impl Div<u8> for Depth {
    type Output = Self;

    fn div(self, other: u8) -> Self::Output {
        self / Self(other.into())
    }
}

impl From<bool> for Depth {
    fn from(b: bool) -> Self {
        Self(b.into())
    }
}

impl From<CompressedDepth> for Depth {
    fn from(depth: CompressedDepth) -> Self {
        Self(depth.0.into())
    }
}

impl From<Evaluation> for Depth {
    fn from(eval: Evaluation) -> Self {
        Self(CompressedEvaluation::from(eval).0)
    }
}

impl Mul for Depth {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        Self(self.0 * other.0)
    }
}

impl PartialEq<u8> for Depth {
    fn eq(&self, other: &u8) -> bool {
        self.0 == (*other).into()
    }
}

impl PartialOrd<u8> for Depth {
    fn partial_cmp(&self, other: &u8) -> Option<Ordering> {
        Some(self.0.cmp(&(*other).into()))
    }
}

impl Sub for Depth {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl Sub<u8> for Depth {
    type Output = Self;

    fn sub(self, other: u8) -> Self::Output {
        self - Self(other.into())
    }
}

impl SubAssign for Depth {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl SubAssign<u8> for Depth {
    fn sub_assign(&mut self, other: u8) {
        *self -= Self(other.into());
    }
}

impl Add for Height {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl Add<u8> for Height {
    type Output = Self;

    fn add(self, other: u8) -> Self::Output {
        self + Self(other)
    }
}

impl Depth {
    /// The maximum depth permissible.
    pub const MAX: Self = Self(u8::MAX as i16);
}

impl Depth {
    /// Converts the depth to a usize.
    pub const fn to_index(self) -> usize {
        self.0 as usize
    }
}

impl Height {
    /// Checks if the height has reached [`Depth::MAX`].
    pub fn is_maximum(self) -> bool {
        Depth(self.0.into()) == Depth::MAX
    }

    /// Converts the height to a usize.
    pub const fn to_index(self) -> usize {
        self.0 as usize
    }
}
