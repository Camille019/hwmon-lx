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
use regex::Regex;

use feature::FeatureType;
use sysfs::*;

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fan {
    Input,
    Min,
    Max,
    Div,
    Pulses,
    Target,
    // Alarms
    Alarm,
    Min_Alarm,
    Max_Alarm,
    Fault,
    Beep,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Temperature {
    Input,
    Min,
    Min_Hyst,
    Max,
    Max_Hyst,
    Crit_Min,
    Crit_Min_Hyst,
    Crit_Max,
    Crit_Max_Hyst,
    Emergency,
    Emergency_Hyst,
    Lowest,
    Highest,
    Type,
    Offset,
    // Alarms
    Alarm,
    Min_Alarm,
    Max_Alarm,
    Emergency_Alarm,
    Crit_Min_Alarm,
    Crit_Max_Alarm,
    Fault,
    Beep,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Voltage {
    Input,
    Min,
    Max,
    Crit_Min,
    Crit_Max,
    Average,
    Lowest,
    Highest,
    // Alarms
    Alarm,
    Min_Alarm,
    Max_Alarm,
    Crit_Min_Alarm,
    Crit_Max_Alarm,
    Beep,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Current {
    Input,
    Min,
    Max,
    Crit_Min,
    Crit_Max,
    Average,
    Lowest,
    Highest,
    // Alarms
    Alarm,
    Min_Alarm,
    Max_Alarm,
    Crit_Min_Alarm,
    Crit_Max_Alarm,
    Beep,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Power {
    Average,
    Average_Lowest,
    Average_Highest,
    Input,
    Input_Lowest,
    Input_Highest,
    Cap,
    Cap_Min,
    Cap_Max,
    Cap_Hyst,
    Min,
    Max,
    Crit_Min,
    Crit_Max,
    Average_Interval,
    Accuracy,
    // Alarms
    Alarm,
    Cap_Alarm,
    Min_Alarm,
    Max_Alarm,
    Crit_Min_Alarm,
    Crit_Max_Alarm,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Energy {
    Input,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Humidity {
    Input,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Intrusion {
    Alarm,
    Beep,
}

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
    fn scale(self) -> f64 {
        const SCALE_MILLI: f64 = 1000.0;
        const SCALE_MICRO: f64 = 1_000_000.0;

        match self {
            SubfeatureType::Fan(_) => 1.0,
            SubfeatureType::Temperature(sft) => match sft {
                Temperature::Input
                | Temperature::Min
                | Temperature::Min_Hyst
                | Temperature::Max
                | Temperature::Max_Hyst
                | Temperature::Crit_Min
                | Temperature::Crit_Min_Hyst
                | Temperature::Crit_Max
                | Temperature::Crit_Max_Hyst
                | Temperature::Emergency
                | Temperature::Emergency_Hyst
                | Temperature::Lowest
                | Temperature::Highest
                | Temperature::Offset => SCALE_MILLI,

                Temperature::Alarm
                | Temperature::Min_Alarm
                | Temperature::Max_Alarm
                | Temperature::Crit_Min_Alarm
                | Temperature::Crit_Max_Alarm
                | Temperature::Fault
                | Temperature::Beep
                | Temperature::Emergency_Alarm
                | Temperature::Type => 1.0,
            },
            SubfeatureType::Voltage(sft) => match sft {
                Voltage::Input
                | Voltage::Min
                | Voltage::Max
                | Voltage::Crit_Min
                | Voltage::Crit_Max
                | Voltage::Average
                | Voltage::Lowest
                | Voltage::Highest => SCALE_MILLI,

                Voltage::Alarm
                | Voltage::Min_Alarm
                | Voltage::Max_Alarm
                | Voltage::Beep
                | Voltage::Crit_Min_Alarm
                | Voltage::Crit_Max_Alarm => 1.0,
            },
            SubfeatureType::Current(sft) => match sft {
                Current::Input
                | Current::Min
                | Current::Max
                | Current::Crit_Min
                | Current::Crit_Max
                | Current::Average
                | Current::Lowest
                | Current::Highest => SCALE_MILLI,

                Current::Alarm
                | Current::Min_Alarm
                | Current::Max_Alarm
                | Current::Beep
                | Current::Crit_Min_Alarm
                | Current::Crit_Max_Alarm => 1.0,
            },
            SubfeatureType::Power(sft) => match sft {
                Power::Average
                | Power::Average_Lowest
                | Power::Average_Highest
                | Power::Input
                | Power::Input_Lowest
                | Power::Input_Highest
                | Power::Cap
                | Power::Cap_Min
                | Power::Cap_Max
                | Power::Cap_Hyst
                | Power::Min
                | Power::Max
                | Power::Crit_Min
                | Power::Crit_Max => SCALE_MICRO,

                Power::Average_Interval => SCALE_MILLI,

                Power::Alarm
                | Power::Cap_Alarm
                | Power::Min_Alarm
                | Power::Max_Alarm
                | Power::Crit_Min_Alarm
                | Power::Crit_Max_Alarm
                | Power::Accuracy => 1.0,
            },
            SubfeatureType::Energy(sft) => match sft {
                Energy::Input => SCALE_MICRO,
            },
            SubfeatureType::Humidity(sft) => match sft {
                Humidity::Input => SCALE_MILLI,
            },
            SubfeatureType::Cpu => SCALE_MILLI,
            SubfeatureType::Intrusion(_) => 1.0,
            SubfeatureType::BeepEnable => 1.0,
        }
    }
}

lazy_static! {
    static ref FAN_MAP: HashMap<&'static str, SubfeatureType> = {
        use self::Fan::*;
        use self::SubfeatureType::*;

        let mut m = HashMap::new();
        m.insert("input", Fan(Input));
        m.insert("min", Fan(Min));
        m.insert("max", Fan(Max));
        m.insert("div", Fan(Div));
        m.insert("pulses", Fan(Pulses));
        m.insert("target", Fan(Target));
        m.insert("alarm", Fan(Alarm));
        m.insert("min_alarm", Fan(Min_Alarm));
        m.insert("max_alarm", Fan(Max_Alarm));
        m.insert("fault", Fan(Fault));
        m.insert("beep", Fan(Beep));
        m.shrink_to_fit();
        m
    };
    static ref TEMPERATURE_MAP: HashMap<&'static str, SubfeatureType> = {
        use self::SubfeatureType::*;
        use self::Temperature::*;

        let mut m = HashMap::new();
        m.insert("input", Temperature(Input));
        m.insert("max", Temperature(Max));
        m.insert("max_hyst", Temperature(Max_Hyst));
        m.insert("min", Temperature(Min));
        m.insert("min_hyst", Temperature(Min_Hyst));
        m.insert("crit", Temperature(Crit_Max));
        m.insert("crit_hyst", Temperature(Crit_Max_Hyst));
        m.insert("lcrit", Temperature(Crit_Min));
        m.insert("lcrit_hyst", Temperature(Crit_Min_Hyst));
        m.insert("emergency", Temperature(Emergency));
        m.insert("emergency_hyst", Temperature(Emergency_Hyst));
        m.insert("lowest", Temperature(Lowest));
        m.insert("highest", Temperature(Highest));
        m.insert("alarm", Temperature(Alarm));
        m.insert("min_alarm", Temperature(Min_Alarm));
        m.insert("max_alarm", Temperature(Max_Alarm));
        m.insert("crit_alarm", Temperature(Crit_Max_Alarm));
        m.insert("emergency_alarm", Temperature(Emergency_Alarm));
        m.insert("lcrit_alarm", Temperature(Crit_Min_Alarm));
        m.insert("fault", Temperature(Fault));
        m.insert("type", Temperature(Type));
        m.insert("offset", Temperature(Offset));
        m.insert("beep", Temperature(Beep));
        m.shrink_to_fit();
        m
    };
    static ref VOLTAGE_MAP: HashMap<&'static str, SubfeatureType> = {
        use self::SubfeatureType::*;
        use self::Voltage::*;

        let mut m = HashMap::new();
        m.insert("input", Voltage(Input));
        m.insert("min", Voltage(Min));
        m.insert("max", Voltage(Max));
        m.insert("lcrit", Voltage(Crit_Min));
        m.insert("crit", Voltage(Crit_Max));
        m.insert("average", Voltage(Average));
        m.insert("lowest", Voltage(Lowest));
        m.insert("highest", Voltage(Highest));
        m.insert("alarm", Voltage(Alarm));
        m.insert("min_alarm", Voltage(Min_Alarm));
        m.insert("max_alarm", Voltage(Max_Alarm));
        m.insert("lcrit_alarm", Voltage(Crit_Min_Alarm));
        m.insert("crit_alarm", Voltage(Crit_Max_Alarm));
        m.insert("beep", Voltage(Beep));
        m.shrink_to_fit();
        m
    };
    static ref CURRENT_MAP: HashMap<&'static str, SubfeatureType> = {
        use self::Current::*;
        use self::SubfeatureType::*;

        let mut m = HashMap::new();
        m.insert("input", Current(Input));
        m.insert("min", Current(Min));
        m.insert("max", Current(Max));
        m.insert("lcrit", Current(Crit_Min));
        m.insert("crit", Current(Crit_Max));
        m.insert("average", Current(Average));
        m.insert("lowest", Current(Lowest));
        m.insert("highest", Current(Highest));
        m.insert("alarm", Current(Alarm));
        m.insert("min_alarm", Current(Min_Alarm));
        m.insert("max_alarm", Current(Max_Alarm));
        m.insert("lcrit_alarm", Current(Crit_Min_Alarm));
        m.insert("crit_alarm", Current(Crit_Max_Alarm));
        m.insert("beep", Current(Beep));
        m.shrink_to_fit();
        m
    };
    static ref POWER_MAP: HashMap<&'static str, SubfeatureType> = {
        use self::Power::*;
        use self::SubfeatureType::*;

        let mut m = HashMap::new();
        m.insert("average", Power(Average));
        m.insert("average_highest", Power(Average_Highest));
        m.insert("average_lowest", Power(Average_Lowest));
        m.insert("input", Power(Input));
        m.insert("input_highest", Power(Input_Highest));
        m.insert("input_lowest", Power(Input_Lowest));
        m.insert("accuracy", Power(Accuracy));
        m.insert("cap", Power(Cap));
        m.insert("cap_hyst", Power(Cap_Hyst));
        m.insert("cap_max", Power(Cap_Max));
        m.insert("cap_min", Power(Cap_Min));
        m.insert("cap_alarm", Power(Cap_Alarm));
        m.insert("alarm", Power(Alarm));
        m.insert("max", Power(Max));
        m.insert("min", Power(Min));
        m.insert("max_alarm", Power(Max_Alarm));
        m.insert("min_alarm", Power(Min_Alarm));
        m.insert("crit", Power(Crit_Max));
        m.insert("lcrit", Power(Crit_Min));
        m.insert("crit_alarm", Power(Crit_Max_Alarm));
        m.insert("lcrit_alarm", Power(Crit_Min_Alarm));
        m.insert("average_interval", Power(Average_Interval));
        m.shrink_to_fit();
        m
    };
    static ref ENERGY_MAP: HashMap<&'static str, SubfeatureType> = {
        use self::Energy::*;
        use self::SubfeatureType::*;

        let mut m = HashMap::new();
        m.insert("input", Energy(Input));
        m.shrink_to_fit();
        m
    };
    static ref HUMIDITY_MAP: HashMap<&'static str, SubfeatureType> = {
        use self::Humidity::*;
        use self::SubfeatureType::*;

        let mut m = HashMap::new();
        m.insert("input", Humidity(Input));
        m.shrink_to_fit();
        m
    };
    static ref CPU_MAP: HashMap<&'static str, SubfeatureType> = {
        use self::SubfeatureType::*;

        let mut m = HashMap::new();
        m.insert("vid", Cpu);
        m.shrink_to_fit();
        m
    };
    static ref INTRUSION_MAP: HashMap<&'static str, SubfeatureType> = {
        use self::Intrusion::*;
        use self::SubfeatureType::*;

        let mut m = HashMap::new();
        m.insert("alarm", Intrusion(Alarm));
        m.insert("beep", Intrusion(Beep));
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
        sysfs_read_file(&self.path)
            .and_then(|s| Ok(s.parse::<f64>()?))
            .map(|value| value / self.subfeature_type.scale())
    }

    /// Write the value to sysfs file. Before it apply the proper type scaling.
    ///
    /// Note: This function does not take into account the configuration file.
    fn write_sysfs_value(&self, value: f64) -> std::io::Result<()> {
        let i_value = (value * self.subfeature_type.scale()).round() as u32;
        let mut file = OpenOptions::new()
            .read(false)
            .write(true)
            .create(false)
            .open(&self.path)?;
        write!(file, "{}", i_value)
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
