// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

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
