// SPDX-FileCopyrightText: 2018 Camille019
// SPDX-License-Identifier: MPL-2.0

#![forbid(unsafe_code)]

mod bus;
mod chip;
mod context;
mod error;
mod feature;
mod prefix;
mod ratio;
pub mod subfeature;
mod sysfs;

#[cfg(feature = "sensorsconf")]
mod parser;

pub use crate::bus::{Bus, BusType};
pub use crate::chip::{read_sysfs_chips, Chip, FeatureIter};
pub use crate::context::Context;
pub use crate::error::Error;
pub use crate::feature::{Feature, FeatureType, SubfeatureIter};
pub use crate::subfeature::{Subfeature, SubfeatureType};
