// SPDX-FileCopyrightText: 2018 Camille019
// SPDX-License-Identifier: MPL-2.0

use std::io;
use std::path::{Path, PathBuf};
use std::slice;

use crate::error::*;
use crate::subfeature::{Subfeature, SubfeatureType};
use crate::sysfs;

#[cfg(feature = "sensorsconf")]
use crate::parser::StmtChip;

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

impl FeatureType {
    /// Return the name of the feature as it appears in sysfs.
    pub(crate) fn sysfs_name(self, number: u32) -> String {
        match self {
            FeatureType::Voltage => format!("in{number}"),
            FeatureType::Fan => format!("fan{number}"),
            FeatureType::Pwm => format!("pwm{number}"),
            FeatureType::Temperature => format!("temp{number}"),
            FeatureType::Power => format!("power{number}"),
            FeatureType::Energy => format!("energy{number}"),
            FeatureType::Current => format!("curr{number}"),
            FeatureType::Humidity => format!("humidity{number}"),
            FeatureType::Cpu => format!("cpu{number}_vid"),
            FeatureType::Intrusion => format!("intrusion{number}"),
            FeatureType::BeepEnable => String::from("beep_enable"),
        }
    }
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
    label_override: Option<String>,
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
        if let Some(ref label) = self.label_override {
            label.clone()
        } else if let Ok(label) = self.read_sysfs_label() {
            label
        } else {
            self.name.to_owned()
        }
    }

    /// Return the subfeature of the given type, if it exists, `None` otherwise.
    pub fn subfeature(&self, subfeature_type: SubfeatureType) -> Option<&Subfeature> {
        self.subfeatures
            .iter()
            .find(|&subfeature| subfeature.get_type() == subfeature_type)
    }

    /// An iterator visiting all subfeatures in arbitrary order.
    pub fn subfeatures_iter(&self) -> SubfeatureIter<'_> {
        SubfeatureIter {
            inner: self.subfeatures.iter(),
        }
    }

    pub(crate) fn new(
        dir: &Path,
        feature_type: FeatureType,
        number: u32,
        #[cfg(feature = "sensorsconf")] config: Option<&StmtChip>,
    ) -> Feature {
        let name = feature_type.sysfs_name(number);

        #[cfg(feature = "sensorsconf")]
        let label_override = config.and_then(|c| c.label_of(&name).map(str::to_string));
        #[cfg(not(feature = "sensorsconf"))]
        let label_override = None;

        Feature {
            dir: dir.to_owned(),
            name,
            number,
            feature_type,
            subfeatures: Default::default(),
            label_override,
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

#[cfg(all(test, feature = "sensorsconf"))]
mod tests {
    use super::*;

    #[test]
    fn label_prefers_chip_configuration() {
        let conf = crate::parser::parse_configuration_str(
            r#"
chip "lm78-*"
    label in0 "Main voltage"
"#,
        )
        .unwrap();
        let config = conf.chip_config("lm78-isa-0290").unwrap();

        let feature = Feature::new(
            Path::new("/dev/null"),
            FeatureType::Voltage,
            0,
            Some(config),
        );

        assert_eq!(feature.label(), "Main voltage");
    }

    #[test]
    fn label_falls_back_to_feature_name() {
        let feature = Feature::new(Path::new("/dev/null"), FeatureType::Voltage, 0, None);

        assert_eq!(feature.label(), "in0");
    }
}
