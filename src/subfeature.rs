// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::linux::fs::MetadataExt;
use std::path::{Path, PathBuf};

use failure::Error;
use libc;
use ratio::{self, Rational};
use regex::Regex;

use feature::FeatureType;
use sysfs::*;

macro_rules! decl_subfeatures {
    (($SfName:ident, $MAP_NAME:ident) [ $($Variant:ident { $pattern:expr, $ratio:ident, $alarm:expr}),* $(,)* ]) => {
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $SfName {
            $($Variant),*
        }

        impl $SfName {
            fn scale(self) -> f64 {
                match self {
                    $($SfName::$Variant => (ratio::$ratio::DENOM as f64) / (ratio::$ratio::NUM as f64),)*
                }
            }

            /// Return `true` if the subfeature variant is an alarm.
            pub fn alarm(self) -> bool {
                match self {
                    $($SfName::$Variant => $alarm,)*
                }
            }
        }

        lazy_static! {
            static ref $MAP_NAME: HashMap<&'static str, SubfeatureType> = {
                let mut m = HashMap::new();
                $(m.insert($pattern, SubfeatureType::$SfName($SfName::$Variant));)*
                m.shrink_to_fit();
                m
            };
        }
    }
}

decl_subfeatures!((Fan, FAN_MAP) [
    Input { "input", Unity, false },
    Min { "min", Unity, false },
    Max { "max", Unity, false },
    Div { "div", Unity, false },
    Pulses { "pulses", Unity, false },
    Target { "target", Unity, false },
    // Alarms
    Alarm { "alarm", Unity, true },
    Min_Alarm { "min_alarm", Unity, true },
    Max_Alarm { "max_alarm", Unity, true },
    Fault { "fault", Unity, false },
    Beep { "beep", Unity, false },
]);

decl_subfeatures!((Temperature, TEMPERATURE_MAP) [
    Input { "input", Milli, false },
    Max { "max", Milli, false },
    Max_Hyst { "max_hyst", Milli, false },
    Min { "min", Milli, false },
    Min_Hyst { "min_hyst", Milli, false },
    Crit_Max { "crit", Milli, false },
    Crit_Max_Hyst { "crit_hyst", Milli, false },
    Crit_Min { "lcrit", Milli, false },
    Crit_Min_Hyst { "lcrit_hyst", Milli, false },
    Emergency { "emergency", Milli, false },
    Emergency_Hyst { "emergency_hyst", Milli, false },
    Lowest { "lowest", Milli, false },
    Highest { "highest", Milli, false },
    Offset { "offset", Milli, false },
    Type { "type", Unity, false },
    // Alarms
    Alarm { "alarm", Unity, true },
    Max_Alarm { "max_alarm", Unity, true },
    Min_Alarm { "min_alarm", Unity, true },
    Emergency_Alarm { "emergency_alarm", Unity, true },
    Crit_Max_Alarm { "crit_alarm", Unity, true },
    Crit_Min_Alarm { "lcrit_alarm", Unity, true },
    Fault { "fault", Unity, false },
    Beep { "beep", Unity, false },
]);

decl_subfeatures!((Voltage, VOLTAGE_MAP) [
    Input { "input", Milli, false },
    Max { "max", Milli, false },
    Min { "min", Milli, false },
    Crit_Max { "crit", Milli, false },
    Crit_Min { "lcrit", Milli, false },
    Average { "average", Milli, false },
    Highest { "highest", Milli, false },
    Lowest { "lowest", Milli, false },
    // Alarms
    Alarm { "alarm", Unity, true },
    Max_Alarm { "max_alarm", Unity, true },
    Min_Alarm { "min_alarm", Unity, true },
    Crit_Max_Alarm { "crit_alarm", Unity, true },
    Crit_Min_Alarm { "lcrit_alarm", Unity, true },
    Beep { "beep", Unity, false },
]);

decl_subfeatures!((Current, CURRENT_MAP) [
    Input { "input", Milli, false },
    Max { "max", Milli, false },
    Min { "min", Milli, false },
    Crit_Max { "crit", Milli, false },
    Crit_Min { "lcrit", Milli, false },
    Average { "average", Milli, false },
    Highest { "highest", Milli, false },
    Lowest { "lowest", Milli, false },
    // Alarms
    Alarm { "alarm", Unity, true },
    Max_Alarm { "max_alarm", Unity, true },
    Min_Alarm { "min_alarm", Unity, true },
    Crit_Max_Alarm { "crit_alarm", Unity, true },
    Crit_Min_Alarm { "lcrit_alarm", Unity, true },
    Beep { "beep", Unity, false },
]);

decl_subfeatures!((Power, POWER_MAP) [
    Average { "average", Micro, false },
    Average_Highest { "average_highest", Micro, false },
    Average_Lowest { "average_lowest", Micro, false },
    Input { "input", Micro, false },
    Input_Highest { "input_highest", Micro, false },
    Input_Lowest { "input_lowest", Micro, false },
    Cap { "cap", Micro, false },
    Cap_Max { "cap_max", Micro, false },
    Cap_Min { "cap_min", Micro, false },
    Cap_Hyst { "cap_hyst", Micro, false },
    Max { "max", Micro, false },
    Min { "min", Micro, false },
    Crit_Max { "crit", Micro, false },
    Crit_Min { "lcrit", Micro, false },
    Average_Interval { "average_interval", Milli, false },
    Accuracy { "accuracy", Unity, false },
    // Alarms
    Alarm { "alarm", Unity, true },
    Cap_Alarm { "cap_alarm", Unity, true },
    Max_Alarm { "max_alarm", Unity, true },
    Min_Alarm { "min_alarm", Unity, true },
    Crit_Max_Alarm { "crit_alarm", Unity, true },
    Crit_Min_Alarm { "lcrit_alarm", Unity, true },
]);

decl_subfeatures!((Energy, ENERGY_MAP) [
    Input { "input", Micro, false },
]);

decl_subfeatures!((Humidity, HUMIDITY_MAP) [
    Input { "input", Milli, false },
]);

decl_subfeatures!((Intrusion, INTRUSION_MAP) [
    Alarm { "alarm", Micro, false },
    Beep { "beep", Micro, false },
]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubfeatureType {
    Fan(Fan),
    Temperature(Temperature),
    Voltage(Voltage),
    Current(Current),
    Power(Power),
    Energy(Energy),
    Humidity(Humidity),
    Cpu,
    Intrusion(Intrusion),
    BeepEnable,
}

impl SubfeatureType {
    fn to_native(self, value: f64) -> i64 {
        (value * self.scale()).round() as i64
    }

    fn to_unity(self, value: f64) -> f64 {
        value / self.scale()
    }

    fn scale(self) -> f64 {
        match self {
            SubfeatureType::Fan(sft) => sft.scale(),
            SubfeatureType::Temperature(sft) => sft.scale(),
            SubfeatureType::Voltage(sft) => sft.scale(),
            SubfeatureType::Current(sft) => sft.scale(),
            SubfeatureType::Power(sft) => sft.scale(),
            SubfeatureType::Energy(sft) => sft.scale(),
            SubfeatureType::Humidity(sft) => sft.scale(),
            SubfeatureType::Intrusion(sft) => sft.scale(),
            SubfeatureType::Cpu => ratio::Milli::DENOM as f64,
            SubfeatureType::BeepEnable => ratio::Unity::DENOM as f64,
        }
    }

    /// Return `true` if the subfeature variant is an alarm.
    pub fn alarm(self) -> bool {
        match self {
            SubfeatureType::Fan(sft) => sft.alarm(),
            SubfeatureType::Temperature(sft) => sft.alarm(),
            SubfeatureType::Voltage(sft) => sft.alarm(),
            SubfeatureType::Current(sft) => sft.alarm(),
            SubfeatureType::Power(sft) => sft.alarm(),
            SubfeatureType::Energy(sft) => sft.alarm(),
            SubfeatureType::Humidity(sft) => sft.alarm(),
            SubfeatureType::Intrusion(sft) => sft.alarm(),
            SubfeatureType::Cpu => false,
            SubfeatureType::BeepEnable => false,
        }
    }
}

lazy_static!{
    static ref CPU_MAP: HashMap<&'static str, SubfeatureType> = {
        use self::SubfeatureType::*;

        let mut m = HashMap::new();
        m.insert("vid", Cpu);
        m.shrink_to_fit();
        m
    };
    static ref FEATURE_TYPE_MAP: HashMap<&'static str, (FeatureType, &'static HashMap<&'static str, SubfeatureType>)> = {
        let mut m: HashMap<
            &'static str,
            (FeatureType, &'static HashMap<&'static str, SubfeatureType>),
        > = HashMap::new();
        m.insert("temp", (FeatureType::Temperature, &TEMPERATURE_MAP));
        m.insert("in", (FeatureType::Voltage, &VOLTAGE_MAP));
        m.insert("fan", (FeatureType::Fan, &FAN_MAP));
        m.insert("cpu", (FeatureType::Cpu, &CPU_MAP));
        m.insert("power", (FeatureType::Power, &POWER_MAP));
        m.insert("curr", (FeatureType::Current, &CURRENT_MAP));
        m.insert("energy", (FeatureType::Energy, &ENERGY_MAP));
        m.insert("intrusion", (FeatureType::Intrusion, &INTRUSION_MAP));
        m.insert("humidity", (FeatureType::Humidity, &HUMIDITY_MAP));
        m.shrink_to_fit();
        m
    };
}

#[derive(Clone, Debug)]
pub struct Subfeature {
    name: String,
    path: PathBuf,
    subfeature_type: SubfeatureType,
    compute_statement: Option<String>,
    is_readable: bool,
    is_writable: bool,
}

impl Subfeature {
    /// Subfeature name
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Return the sysfs file path
    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }

    /// Get the subfeature type
    pub fn get_type(&self) -> SubfeatureType {
        self.subfeature_type
    }

    /// Return the compute statement string if specified in the configuration file.
    /// Otherwise it return None.
    pub fn compute_statement(&self) -> Option<String> {
        self.compute_statement.clone()
    }

    /// Return true if the subfeature is readable
    pub fn is_readable(&self) -> bool {
        self.is_readable
    }

    /// Return true if the subfeature is writable
    pub fn is_writable(&self) -> bool {
        self.is_writable
    }

    /// Read the value of the subfeature.
    pub fn read_value(&self) -> Result<f64, Error> {
        if self.is_readable() {
            // TODO compute statement
            self.read_sysfs_value()
        } else {
            Err(format_err!("Subfeature not readable"))
        }
    }

    /// Write the value of the subfeature.
    ///
    /// Unsafety: no checks are made on the value before writing it.
    /// Affect a new value at your own risk.
    /// See hwmon and device driver documentation for more informations.
    pub unsafe fn write_value(&self, value: f64) -> Result<(), Error> {
        if self.is_writable() {
            // TODO compute statement
            self.write_sysfs_value(value)?;
            Ok(())
        } else {
            Err(format_err!("Subfeature not writable"))
        }
    }

    /// Read the value from sysfs file and apply the proper type scaling.
    ///
    /// Note: This function does not take into account the configuration file.
    fn read_sysfs_value(&self) -> Result<f64, Error> {
        let value = sysfs_read_file(&self.path)?.parse::<f64>()?;
        Ok(self.subfeature_type.to_unity(value))
    }

    /// Write the value to sysfs file. Before it apply the proper type scaling.
    ///
    /// Note: This function does not take into account the configuration file.
    fn write_sysfs_value(&self, value: f64) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .read(false)
            .write(true)
            .create(false)
            .open(&self.path)?;
        write!(file, "{}", self.subfeature_type.to_native(value))
    }

    pub(crate) fn from_path<P: AsRef<Path>>(path: P) -> Result<(u32, Subfeature), Error> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(format_err!("Subfeature do not exist: {:?}", path));
        }

        let name = path
            .file_name()
            .and_then(|str| str.to_str())
            .map(|str| str.to_string())
            .unwrap();

        let (feature_number, subfeature_type) =
            Subfeature::get_properties_from_name(name.as_ref())?;

        let st_mode = path.metadata().map(|m| m.st_mode())?;
        let is_readable = (st_mode & libc::S_IRUSR) == libc::S_IRUSR;
        let is_writable = (st_mode & libc::S_IWUSR) == libc::S_IWUSR;

        Ok((
            feature_number,
            Subfeature {
                name: name.clone(),
                path: path.to_path_buf(),
                subfeature_type,
                compute_statement: None, // TODO compute statement
                is_readable,
                is_writable,
            },
        ))
    }

    fn get_properties_from_name(name: &str) -> Result<(u32, SubfeatureType), Error> {
        if name == "beep_enable" {
            return Ok((0, SubfeatureType::BeepEnable));
        }

        let re = Regex::new(r"^(\D*)(\d+)_(.*)").unwrap();

        if let Some(caps) = re.captures(name) {
            let feature_str_id = &caps[1];
            let feature_number = caps[2].parse::<u32>().unwrap();
            let subfeature_str_id = &caps[3];

            if let Some(sf_type) = FEATURE_TYPE_MAP
                .get(feature_str_id)
                .and_then(|(_, sf_map)| sf_map.get(subfeature_str_id))
            {
                Ok((feature_number, *sf_type))
            } else {
                Err(format_err!("Unknown subfeature: {}", name))
            }
        } else {
            Err(format_err!("Invalid subfeature: {}", name))
        }
    }
}
