// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io;
use std::path::{Path, PathBuf};
use std::slice;

use crate::error::*;
use crate::subfeature::{Subfeature, SubfeatureType};
use crate::sysfs;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum FeatureType {
    Fan,
    Pwm,
    Temperature,
    Voltage,
    Current,
    Power,
    Energy,
    Humidity,
    Cpu,
    Intrusion,
    BeepEnable,
}

impl From<SubfeatureType> for FeatureType {
    fn from(sf_type: SubfeatureType) -> FeatureType {
        match sf_type {
            SubfeatureType::Fan(_) => FeatureType::Fan,
            SubfeatureType::Pwm(_) => FeatureType::Pwm,
            SubfeatureType::Temperature(_) => FeatureType::Temperature,
            SubfeatureType::Voltage(_) => FeatureType::Voltage,
            SubfeatureType::Current(_) => FeatureType::Current,
            SubfeatureType::Power(_) => FeatureType::Power,
            SubfeatureType::Energy(_) => FeatureType::Energy,
            SubfeatureType::Humidity(_) => FeatureType::Humidity,
            SubfeatureType::Cpu => FeatureType::Cpu,
            SubfeatureType::Intrusion(_) => FeatureType::Intrusion,
            SubfeatureType::BeepEnable => FeatureType::BeepEnable,
        }
    }
}

pub struct SubfeatureIter<'a> {
    inner: slice::Iter<'a, Subfeature>,
}

impl<'a> Iterator for SubfeatureIter<'a> {
    type Item = &'a Subfeature;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

#[derive(Debug)]
pub struct Feature {
    dir: PathBuf,
    name: String,
    number: u32,
    feature_type: FeatureType,
    subfeatures: Vec<Subfeature>,
}

impl Feature {
    /// Feature name
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Feature number
    pub fn number(&self) -> u32 {
        self.number
    }

    /// Get the feature type
    pub fn get_type(&self) -> FeatureType {
        self.feature_type
    }

    /// Look up the label of the feature in config files or in sysfs.
    /// If no label exists for this feature, its name is returned itself.
    pub fn label(&self) -> String {
        // TODO check user specified label

        if let Ok(label) = self.read_sysfs_label() {
            label
        } else {
            self.name.to_owned()
        }
    }

    /// Return the subfeature of the given type, if it exists, `None` otherwise.
    pub fn subfeature(&self, subfeature_type: SubfeatureType) -> Option<&Subfeature> {
        for subfeature in &self.subfeatures {
            if subfeature.get_type() == subfeature_type {
                return Some(subfeature);
            }
        }

        None
    }

    /// An iterator visiting all subfeatures in arbitrary order.
    pub fn subfeatures_iter(&self) -> SubfeatureIter {
        SubfeatureIter {
            inner: self.subfeatures.iter(),
        }
    }

    pub(crate) fn new(dir: &Path, feature_type: FeatureType, number: u32) -> Feature {
        let name = match feature_type {
            FeatureType::Voltage => format!("in{}", number),
            FeatureType::Fan => format!("fan{}", number),
            FeatureType::Pwm => format!("pwm{}", number),
            FeatureType::Temperature => format!("temp{}", number),
            FeatureType::Power => format!("power{}", number),
            FeatureType::Energy => format!("energy{}", number),
            FeatureType::Current => format!("curr{}", number),
            FeatureType::Humidity => format!("humidity{}", number),
            FeatureType::Cpu => format!("cpu{}_vid", number),
            FeatureType::Intrusion => format!("intrusion{}", number),
            FeatureType::BeepEnable => String::from("beep_enable"),
        };

        Feature {
            dir: dir.to_owned(),
            name,
            number,
            feature_type,
            subfeatures: Default::default(),
        }
    }

    ///
    /// Return `None` if
    pub(crate) fn push_subfeature(&mut self, subfeature: Subfeature) -> Result<(), FeatureError> {
        if FeatureType::from(subfeature.get_type()) == self.feature_type {
            log::debug!(
                "Add subfeature '{}' to feature '{}'",
                subfeature.name(),
                self.name()
            );
            self.subfeatures.push(subfeature);
            Ok(())
        } else {
            Err(FeatureError::SubfeatureType)
        }
    }

    fn read_sysfs_label(&self) -> io::Result<String> {
        let attr = format!("{}_label", self.name);
        sysfs::sysfs_read_attr(self.dir.as_ref(), attr.as_ref())
    }
}
