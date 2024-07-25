// SPDX-FileCopyrightText: 2019 Camille019
// SPDX-License-Identifier: MPL-2.0

/// Represents the ratio between two numbers.
#[derive(Clone, Copy, Debug)]
pub struct Ratio<T> {
    /// Numerator.
    numer: T,
    /// Denominator.
    denom: T,
}

impl<T> Ratio<T> {
    /// Creates a `Ratio` without checking for `denom == 0` or reducing.
    #[inline]
    pub const fn new_raw(numer: T, denom: T) -> Ratio<T> {
        Ratio { numer, denom }
    }

    /// Gets an immutable reference to the numerator.
    #[inline]
    pub const fn numer(&self) -> &T {
        &self.numer
    }

    /// Gets an immutable reference to the denominator.
    #[inline]
    pub const fn denom(&self) -> &T {
        &self.denom
    }
}
