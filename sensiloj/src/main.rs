// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use hwmon::subfeature::*;
use hwmon::{Chip, Feature, FeatureType, SubfeatureType};

use lazy_static::lazy_static;

static HYST_STR: &str = "hyst";

fn main() -> Result<(), hwmon::Error> {
    env_logger::init();

    let context = hwmon::Context::new(None)?;

    match hwmon::read_sysfs_chips(&context) {
        Ok(chips) => {
            for chip in chips.iter() {
                println!("{}", chip.name());
                if let Some(name) = chip.bus().adapter_name() {
                    println!("Adapter: {}", name);
                } else {
                    eprintln!("Can't get adapter name");
                }
                print_chip(chip);
                println!();
            }
        }
        Err(e) => println!("{:?}", e),
    }

    Ok(())
}

#[derive(Debug)]
struct SubfeatureData {
    /// Subfeature value. Not used for alarms.
    value: f64,
    /// Subfeature name
    name: String,
    /// Unit to be displayed for this subfeature.
    /// This field is optional.
    unit: String,
}

#[derive(Debug)]
struct SubfeatureList {
    sf_type: SubfeatureType,
    /// Complementary subfeatures to be displayed if subfeature exists
    comp: Vec<SubfeatureList>,
    /// Subfeature name to be printed
    name: String,
}

fn scale_value(value: &mut f64, prefix: &mut String) {
    lazy_static! {
        static ref PREFIX_SCALE: Vec<(f64, &'static str)> = vec![
            (1e-6, "n"),
            (1e-3, "u"),
            (1.0, "m"),
            (1e3, ""),
            (1e6, "k"),
            (1e9, "M"),
            (0.0, "G")
        ];
    }

    let abs_value = value.abs();
    let mut divisor = 1e-9;

    if abs_value == 0.0 {
        prefix.clear();
        return;
    }

    let mut idx = 0;
    for (upper_bound, _) in PREFIX_SCALE.iter() {
        if *upper_bound == 0.0 || abs_value <= *upper_bound {
            break;
        }
        divisor = *upper_bound;
        idx += 1;
    }

    *value /= divisor;
    *prefix = String::from(PREFIX_SCALE.get(idx).unwrap().1);
}

fn get_label_length(chip: &Chip) -> usize {
    let mut max_len = 11;
    for feature in chip.features_iter() {
        let len = feature.label().len();
        if len > max_len {
            max_len = len;
        }
    }

    // One more for the colon, and one more to guarantee at least one
    // space between that colon and the value */
    max_len + 2
}

fn print_label(label: &str, length: usize) {
    print!("{}{:len$}", label, ":", len = (length - label.len()));
}

fn print_alarms(alarms: &[SubfeatureData], leading_spaces: usize) {
    print!("{:>len$}", "ALARM", len = (leading_spaces + 7));
    if !alarms.is_empty() {
        let mut printed = false;
        for alarm in alarms {
            if !alarm.name.is_empty() {
                if !printed {
                    print!(" (");
                } else {
                    print!(", ");
                }
                print!("{}", alarm.name);
                printed = true;
            }
        }
        if printed {
            print!(")");
        }
    }
}

macro_rules! print_limits {
    ($limits:ident, $alarms:ident, $label_length:expr, $fmt:expr) => {
        let mut alarms_printed = false;

        // We print limits on two columns, filling lines first, except for
        // hysteresis which must always go on the right column, with the
        // limit it relates to being in the left column on the same line.
        let mut i = 0;
        let mut slot = 0;
        for limit in $limits.iter() {
            if (slot & 1) != 1 {
                if slot != 0 {
                    print!("\n{:>len$}", "", len = ($label_length + 10));
                }
                print!("(");
            } else {
                print!(", ");
            }
            print!($fmt, limit.name, limit.value, limit.unit);

            let skip = if $limits
                .get(i + 2)
                .map(|s| s.name == HYST_STR)
                .map(|b| (b && !((slot & 1) == 1)))
                .unwrap_or(false)
            {
                1
            } else {
                0
            };

            if (((slot + skip) & 1) == 1) || (i == ($limits.len() - 1)) {
                print!(")");
                if !$alarms.is_empty() && !alarms_printed {
                    print_alarms(&$alarms, if (slot & 1) == 1 { 0 } else { 16 });
                    alarms_printed = true;
                }
            }
            slot += skip + 1;
            i += 1;
        }
        if !$alarms.is_empty() && !alarms_printed {
            print_alarms(&$alarms, 32);
        }
    };
}

fn get_sensor_limit_data(
    feature: &Feature,
    sfl_vec: &[SubfeatureList],
    limits: &mut Vec<SubfeatureData>,
    alarms: &mut Vec<SubfeatureData>,
) {
    for sfl in sfl_vec.iter() {
        if let Some(value) = feature
            .subfeature(sfl.sf_type)
            .and_then(|sf| sf.read_value().ok())
        {
            if sfl.sf_type.is_alarm() {
                // Only queue alarm subfeatures if the alarm
                // is active, and don't store the alarm value
                // (it is implied to be active if queued).
                if value != 0.0 {
                    let alarm = SubfeatureData {
                        value,
                        name: sfl.name.clone(),
                        unit: Default::default(),
                    };
                    alarms.push(alarm);
                }
            } else {
                // Always queue limit subfeatures with their value.
                let limit = SubfeatureData {
                    value,
                    name: sfl.name.clone(),
                    unit: Default::default(),
                };
                limits.push(limit);
            }
            get_sensor_limit_data(feature, &sfl.comp, limits, alarms);
        }
    }
}

fn print_feature_fan(feature: &Feature, label_length: usize) {
    let label = feature.label();
    print_label(label.as_ref(), label_length);

    let fault = feature
        .subfeature(SubfeatureType::Fan(Fan::Fault))
        .and_then(|sf| sf.read_value().map(|val| val != 0.0).ok())
        .unwrap_or(false);
    if fault {
        print!("   FAULT");
    } else if let Some(input) = feature
        .subfeature(SubfeatureType::Fan(Fan::Input))
        .and_then(|sf| sf.read_value().ok())
    {
        print!("{:4.0} RPM", input);
    } else {
        print!("     N/A");
    }

    // Print limits
    let sfmin = feature
        .subfeature(SubfeatureType::Fan(Fan::Min))
        .and_then(|sf| sf.read_value().ok());
    let sfmax = feature
        .subfeature(SubfeatureType::Fan(Fan::Max))
        .and_then(|sf| sf.read_value().ok());
    let sfdiv = feature
        .subfeature(SubfeatureType::Fan(Fan::Div))
        .and_then(|sf| sf.read_value().ok());

    if sfmin.is_some() || sfmax.is_some() || sfdiv.is_some() {
        print!("  (");
        if let Some(value) = sfmin {
            print!("min = {:4.0} RPM", value);
        }
        if let Some(value) = sfmax {
            if sfmin.is_some() {
                print!(", ")
            }
            print!("min = {:4.0} RPM", value);
        }
        if let Some(value) = sfdiv {
            if sfmin.is_some() || sfmax.is_some() {
                print!(", ")
            }
            print!("min = {:1.0} RPM", value);
        }
        print!(")");
    }

    let sf_alarm = feature
        .subfeature(SubfeatureType::Fan(Fan::Alarm))
        .and_then(|sf| sf.read_value().map(|val| val != 0.0).ok())
        .unwrap_or(false);
    let sfmin_alarm = feature
        .subfeature(SubfeatureType::Fan(Fan::Min_Alarm))
        .and_then(|sf| sf.read_value().map(|val| val != 0.0).ok())
        .unwrap_or(false);
    let sfmax_alarm = feature
        .subfeature(SubfeatureType::Fan(Fan::Max_Alarm))
        .and_then(|sf| sf.read_value().map(|val| val != 0.0).ok())
        .unwrap_or(false);
    if sf_alarm || sfmin_alarm || sfmax_alarm {
        print!("  ALARM")
    }

    println!();
}

macro_rules! make_sflist_item {
    (feature: $Feature:ident, properties: { $SfType:ident } ) => {
        SubfeatureList {
            sf_type: SubfeatureType::$Feature($Feature::$SfType),
            name: String::new(),
            comp: Vec::new(),
        }
    };
    (feature: $Feature:ident, properties: { $SfType:ident, $name:expr } ) => {
        SubfeatureList {
            sf_type: SubfeatureType::$Feature($Feature::$SfType),
            name: String::from($name),
            comp: Vec::new(),
        }
    };
    (feature: $Feature:ident, properties: { $SfType:ident, $name:expr, $comp:tt }) => {
        SubfeatureList {
            sf_type: SubfeatureType::$Feature($Feature::$SfType),
            name: String::from($name),
            comp: make_sflist! {
                feature: $Feature,
                list = $comp
            },
        }
    };
}

macro_rules! make_sflist {
    (feature: $Feature:ident, list = [ $($properties:tt),* $(,)* ] ) => {
        vec![
            $(make_sflist_item!{
                feature: $Feature,
                properties: $properties
            },)*
        ];
    };
}

lazy_static! {
    static ref TEMP_SENSORS: Vec<SubfeatureList> = make_sflist! {
        feature: Temperature,
        list = [
            { Alarm },
            { Crit_Min_Alarm, "LCRIT" },
            { Min_Alarm, "LOW" },
            { Max_Alarm, "HIGH" },
            { Crit_Max_Alarm, "CRIT" },
            { Emergency_Alarm, "EMERGENCY" },
            { Min, "low", [ {Min_Hyst, HYST_STR} ] },
            { Max, "high", [ {Max_Hyst, HYST_STR} ] },
            { Crit_Min, "crit low", [ {Crit_Min_Hyst, HYST_STR} ] },
            { Crit_Max, "crit", [ {Crit_Max_Hyst, HYST_STR} ] },
            { Emergency, "emerg" , [ {Emergency_Hyst, HYST_STR} ] },
            { Lowest, "lowest" },
            { Highest, "highest" },
        ]
    };
}

fn print_feature_temp(feature: &Feature, label_length: usize) {
    let label = feature.label();
    print_label(label.as_ref(), label_length);

    let fault = feature
        .subfeature(SubfeatureType::Temperature(Temperature::Fault))
        .and_then(|sf| sf.read_value().map(|val| val != 0.0).ok())
        .unwrap_or(false);
    if fault {
        print!("   FAULT  ");
    } else if let Some(input) = feature
        .subfeature(SubfeatureType::Temperature(Temperature::Input))
        .and_then(|sf| sf.read_value().ok())
    {
        print!("{:+6.1}°C  ", input);
    } else {
        print!("     N/A  ");
    }

    // Print limits
    let mut alarms = Vec::new();
    let mut sensors = Vec::new();

    get_sensor_limit_data(feature, &TEMP_SENSORS, &mut sensors, &mut alarms);

    print_limits!(sensors, alarms, label_length, "{:-4} = {:+5.1}°C{}");

    // print out temperature sensor info
    if let Some(sens) = feature
        .subfeature(SubfeatureType::Temperature(Temperature::Type))
        .and_then(|sf| sf.read_value().ok())
    {
        let mut sens = sens as i32;

        // older kernels / drivers sometimes report a beta value for thermistors
        if sens > 1000 {
            sens = 4;
        }

        let buff = match sens {
            0 => "disabled",
            1 => "CPU diode",
            2 => "transistor",
            3 => "thermal diode",
            4 => "thermistor",
            5 => "AMD AMDSI",
            6 => "Intel PECI",
            _ => "unknown",
        };

        print!("  sensor = {}", buff);
    }

    println!();
}

lazy_static! {
    static ref VOLTAGE_SENSORS: Vec<SubfeatureList> = make_sflist! {
        feature: Voltage,
        list = [
            { Alarm },
            { Crit_Min_Alarm, "LCRIT" },
            { Min_Alarm, "MIN" },
            { Max_Alarm, "MAX" },
            { Crit_Max_Alarm, "CRIT" },
            { Crit_Min, "crit min" },
            { Min, "min" },
            { Max, "max" },
            { Crit_Max, "crit max" },
            { Average, "avg" },
            { Lowest, "lowest" },
            { Highest, "highest" },
        ]
    };
}

fn print_feature_volt(feature: &Feature, label_length: usize) {
    let label = feature.label();
    print_label(label.as_ref(), label_length);

    if let Some(input) = feature
        .subfeature(SubfeatureType::Voltage(Voltage::Input))
        .and_then(|sf| sf.read_value().ok())
    {
        print!("{:+6.2} V  ", input);
    } else {
        print!("     N/A  ");
    }

    // Print limits
    let mut alarms = Vec::new();
    let mut sensors = Vec::new();

    get_sensor_limit_data(feature, &VOLTAGE_SENSORS, &mut sensors, &mut alarms);

    print_limits!(sensors, alarms, label_length, "{} = {:+6.2} V{}");

    println!();
}

lazy_static! {
    static ref CURRENT_SENSORS: Vec<SubfeatureList> = make_sflist! {
        feature: Current,
        list = [
            { Alarm },
            { Crit_Min_Alarm, "LCRIT" },
            { Min_Alarm, "MIN" },
            { Max_Alarm, "MAX" },
            { Crit_Max_Alarm, "CRIT" },
            { Crit_Min, "crit min" },
            { Min, "min" },
            { Max, "max" },
            { Crit_Max, "crit max" },
            { Average, "avg" },
            { Lowest, "lowest" },
            { Highest, "highest" },
        ]
    };
}

fn print_feature_curr(feature: &Feature, label_length: usize) {
    let label = feature.label();
    print_label(label.as_ref(), label_length);

    if let Some(input) = feature
        .subfeature(SubfeatureType::Current(Current::Input))
        .and_then(|sf| sf.read_value().ok())
    {
        print!("{:+6.2} A  ", input);
    } else {
        print!("     N/A  ");
    }

    // Print limits
    let mut alarms = Vec::new();
    let mut sensors = Vec::new();

    get_sensor_limit_data(feature, &CURRENT_SENSORS, &mut sensors, &mut alarms);

    print_limits!(sensors, alarms, label_length, "{} = {:+6.2} A{}");

    println!();
}

lazy_static! {
    static ref POWER_COMMON_SENSORS: Vec<SubfeatureList> = make_sflist! {
        feature: Power,
        list = [
            { Alarm },
            { Min_Alarm, "MIN" },
            { Max_Alarm, "MAX" },
            { Crit_Min_Alarm, "LCRIT" },
            { Crit_Max_Alarm, "CRIT" },
            { Cap_Alarm, "CAP" },
            { Max, "max" },
            { Min, "min" },
            { Crit_Min, "lcrit" },
            { Crit_Max, "crit" },
            { Cap, "cap" },
        ]
    };
    static ref POWER_INST_SENSORS: Vec<SubfeatureList> = make_sflist! {
        feature: Power,
        list = [
            { Input_Lowest, "lowest" },
            { Input_Highest, "highest" },
            { Average, "avg" },
            { Average_Lowest, "avg lowest" },
            { Average_Highest, "avg highest" },
            { Average_Interval, "interval" },
        ]
    };
    static ref POWER_AVG_SENSORS: Vec<SubfeatureList> = make_sflist! {
        feature: Power,
        list = [
            { Average_Lowest, "lowest" },
            { Average_Highest, "highest" },
            { Average_Interval, "interval" },
        ]
    };
}

fn print_feature_power(feature: &Feature, label_length: usize) {
    let label = feature.label();
    print_label(label.as_ref(), label_length);

    let mut alarms = Vec::new();
    let mut sensors = Vec::new();

    // Power sensors come in 2 flavors: instantaneous and averaged.
    // Most devices only support one flavor, so we try to display the
    // average power if the instantaneous power attribute does not exist.
    // If both instantaneous power and average power are supported,
    // average power is displayed as limit.
    let mut sf = feature
        .subfeature(SubfeatureType::Power(Power::Input))
        .and_then(|sf| sf.read_value().ok());

    if sf.is_some() {
        get_sensor_limit_data(feature, &POWER_INST_SENSORS, &mut sensors, &mut alarms);
    } else {
        get_sensor_limit_data(feature, &POWER_AVG_SENSORS, &mut sensors, &mut alarms);
    }
    // Add sensors common to both flavors.
    get_sensor_limit_data(feature, &POWER_COMMON_SENSORS, &mut sensors, &mut alarms);

    if sf.is_none() {
        sf = feature
            .subfeature(SubfeatureType::Power(Power::Average))
            .and_then(|sf| sf.read_value().ok());
    }

    if let Some(mut value) = sf {
        let mut unit = String::new();
        scale_value(&mut value, &mut unit);
        print!("{:6.2} {}{:len$}", value, unit, "W", len = (3 - unit.len()));
    } else {
        print!("     N/A  ");
    }

    for sens in sensors.iter_mut() {
        // Unit is W and needs to be scaled for all attributes except
        // interval, which does not need to be scaled and is reported in
        // seconds.
        if sens.name != "interval" {
            scale_value(&mut sens.value, &mut sens.unit);
            sens.unit.push('W');
        } else {
            sens.unit = "s".to_string();
        }
    }

    // Print limits
    print_limits!(sensors, alarms, label_length, "{} = {:6.2} {}");

    println!();
}

fn print_feature_energy(feature: &Feature, label_length: usize) {
    if let Some(sf) = feature.subfeature(SubfeatureType::Energy(Energy::Input)) {
        let label = feature.label();
        if let Ok(mut val) = sf.read_value() {
            let mut unit = String::new();
            print_label(label.as_ref(), label_length);
            scale_value(&mut val, &mut unit);
            println!("{:6.2} {}J", val, unit);
            return;
        }
    }

    println!("     N/A");
}

fn print_feature_humidity(feature: &Feature, label_length: usize) {
    if let Some(sf) = feature.subfeature(SubfeatureType::Humidity(Humidity::Input)) {
        let label = feature.label();
        if let Ok(val) = sf.read_value() {
            print_label(label.as_ref(), label_length);
            println!("{:6.1} %RH", val);
        }
    }
}

fn print_feature_cpu(feature: &Feature, label_length: usize) {
    if let Some(sf) = feature.subfeature(SubfeatureType::Cpu) {
        let label = feature.label();
        if let Ok(val) = sf.read_value() {
            print_label(label.as_ref(), label_length);
            println!("{:+6.3} V", val);
        }
    }
}

fn print_feature_intrusion(feature: &Feature, label_length: usize) {
    if let Some(sf) = feature.subfeature(SubfeatureType::Intrusion(Intrusion::Alarm)) {
        let label = feature.label();
        if let Ok(val) = sf.read_value() {
            print_label(label.as_ref(), label_length);
            if val == 0.0 {
                println!("OK");
            } else {
                println!("ALARM");
            }
        }
    }
}

fn print_feature_beep_enable(feature: &Feature, label_length: usize) {
    if let Some(sf) = feature.subfeature(SubfeatureType::BeepEnable) {
        let label = feature.label();
        if let Ok(val) = sf.read_value() {
            print_label(label.as_ref(), label_length);
            if val == 0.0 {
                println!("disabled");
            } else {
                println!("enabled");
            }
        }
    }
}

fn print_chip(chip: &Chip) {
    let label_length = get_label_length(chip);

    for feature in chip.features_iter() {
        match feature.get_type() {
            FeatureType::Fan => print_feature_fan(feature, label_length),
            FeatureType::Temperature => print_feature_temp(feature, label_length),
            FeatureType::Voltage => print_feature_volt(feature, label_length),
            FeatureType::Current => print_feature_curr(feature, label_length),
            FeatureType::Power => print_feature_power(feature, label_length),
            FeatureType::Energy => print_feature_energy(feature, label_length),
            FeatureType::Humidity => print_feature_humidity(feature, label_length),
            FeatureType::Cpu => print_feature_cpu(feature, label_length),
            FeatureType::Intrusion => print_feature_intrusion(feature, label_length),
            FeatureType::BeepEnable => print_feature_beep_enable(feature, label_length),
        }
    }
}
