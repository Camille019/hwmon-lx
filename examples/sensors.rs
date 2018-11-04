// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#[macro_use]
extern crate lazy_static;

extern crate hwmonlx;

use std::alloc::System;

use hwmonlx::subfeature::*;
use hwmonlx::{Chip, Feature, FeatureType, SubfeatureType};

#[global_allocator]
static GLOBAL: System = System;

static HYST_STR: &'static str = "hyst";

fn main() {
    let context = hwmonlx::Context::new(None).unwrap();

    match hwmonlx::read_sysfs_chips(&context) {
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
        static ref PREFIX_SCALE: Vec<(f64, &'static str)> = {
            let mut m = Vec::new();
            m.push((1e-6, "n"));
            m.push((1e-3, "u"));
            m.push((1.0, "m"));
            m.push((1e3, ""));
            m.push((1e6, "k"));
            m.push((1e9, "M"));
            m.push((0.0, "G"));
            m.shrink_to_fit();
            m
        };
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

    /* One more for the colon, and one more to guarantee at least one
	   space between that colon and the value */
    max_len + 2
}

fn print_label(label: &str, length: usize) {
    print!("{}{:len$}", label, ":", len = (length - label.len()));
}

fn print_alarms(alarms: &Vec<SubfeatureData>, leading_spaces: usize) {
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
    sfl_vec: &Vec<SubfeatureList>,
    limits: &mut Vec<SubfeatureData>,
    alarms: &mut Vec<SubfeatureData>,
) {
    for sfl in sfl_vec.iter() {
        //        println!("sf: {:?}", sfl);
        if let Some(value) = feature
            .subfeature(sfl.sf_type)
            .and_then(|sf| sf.read_value().ok())
        {
            //            println!("sf value: {:?}", value);
            if sfl.sf_type.alarm() {
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
    } else {
        if let Some(input) = feature
            .subfeature(SubfeatureType::Fan(Fan::Input))
            .and_then(|sf| sf.read_value().ok())
        {
            print!("{:4.0} RPM", input);
        } else {
            print!("     N/A");
        }
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

lazy_static! {
    static ref TEMP_SENSORS: Vec<SubfeatureList> = vec![
        SubfeatureList {
            sf_type: SubfeatureType::Temperature(Temperature::Alarm),
            comp: Vec::new(),
            name: String::new(),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Temperature(Temperature::Crit_Min_Alarm),
            comp: Vec::new(),
            name: String::from("LCRIT"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Temperature(Temperature::Min_Alarm),
            comp: Vec::new(),
            name: String::from("LOW"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Temperature(Temperature::Max_Alarm),
            comp: Vec::new(),
            name: String::from("HIGH"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Temperature(Temperature::Crit_Max_Alarm),
            comp: Vec::new(),
            name: String::from("CRIT"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Temperature(Temperature::Emergency_Alarm),
            comp: Vec::new(),
            name: String::from("EMERGENCY"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Temperature(Temperature::Min),
            comp: vec![SubfeatureList {
                sf_type: SubfeatureType::Temperature(Temperature::Min_Hyst),
                comp: Vec::new(),
                name: String::from(HYST_STR),
            }],
            name: String::from("low"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Temperature(Temperature::Max),
            comp: vec![SubfeatureList {
                sf_type: SubfeatureType::Temperature(Temperature::Max_Hyst),
                comp: Vec::new(),
                name: String::from(HYST_STR),
            }],
            name: String::from("high"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Temperature(Temperature::Crit_Min),
            comp: vec![SubfeatureList {
                sf_type: SubfeatureType::Temperature(Temperature::Crit_Min_Hyst),
                comp: Vec::new(),
                name: String::from(HYST_STR),
            }],
            name: String::from("crit low"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Temperature(Temperature::Crit_Max),
            comp: vec![SubfeatureList {
                sf_type: SubfeatureType::Temperature(Temperature::Crit_Max_Hyst),
                comp: Vec::new(),
                name: String::from(HYST_STR),
            }],
            name: String::from("crit"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Temperature(Temperature::Emergency),
            comp: vec![SubfeatureList {
                sf_type: SubfeatureType::Temperature(Temperature::Emergency_Hyst),
                comp: Vec::new(),
                name: String::from(HYST_STR),
            }],
            name: String::from("emerg"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Temperature(Temperature::Lowest),
            comp: Vec::new(),
            name: String::from("lowest"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Temperature(Temperature::Highest),
            comp: Vec::new(),
            name: String::from("highest"),
        },
    ];
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
    } else {
        if let Some(input) = feature
            .subfeature(SubfeatureType::Temperature(Temperature::Input))
            .and_then(|sf| sf.read_value().ok())
        {
            print!("{:+6.1}°C  ", input);
        } else {
            print!("     N/A  ");
        }
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
    static ref VOLTAGE_SENSORS: Vec<SubfeatureList> = vec![
        SubfeatureList {
            sf_type: SubfeatureType::Voltage(Voltage::Alarm),
            comp: Vec::new(),
            name: String::new(),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Voltage(Voltage::Crit_Min_Alarm),
            comp: Vec::new(),
            name: String::from("LCRIT"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Voltage(Voltage::Min_Alarm),
            comp: Vec::new(),
            name: String::from("MIN"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Voltage(Voltage::Max_Alarm),
            comp: Vec::new(),
            name: String::from("MAX"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Voltage(Voltage::Crit_Max_Alarm),
            comp: Vec::new(),
            name: String::from("CRIT"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Voltage(Voltage::Crit_Min),
            comp: Vec::new(),
            name: String::from("crit min"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Voltage(Voltage::Min),
            comp: Vec::new(),
            name: String::from("min"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Voltage(Voltage::Max),
            comp: Vec::new(),
            name: String::from("max"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Voltage(Voltage::Crit_Max),
            comp: Vec::new(),
            name: String::from("crit max"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Voltage(Voltage::Average),
            comp: Vec::new(),
            name: String::from("avg"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Voltage(Voltage::Lowest),
            comp: Vec::new(),
            name: String::from("lowest"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Voltage(Voltage::Highest),
            comp: Vec::new(),
            name: String::from("highest"),
        },
    ];
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
    static ref CURRENT_SENSORS: Vec<SubfeatureList> = vec![
        SubfeatureList {
            sf_type: SubfeatureType::Current(Current::Alarm),
            comp: Vec::new(),
            name: String::new(),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Current(Current::Crit_Min_Alarm),
            comp: Vec::new(),
            name: String::from("LCRIT"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Current(Current::Min_Alarm),
            comp: Vec::new(),
            name: String::from("MIN"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Current(Current::Max_Alarm),
            comp: Vec::new(),
            name: String::from("MAX"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Current(Current::Crit_Max_Alarm),
            comp: Vec::new(),
            name: String::from("CRIT"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Current(Current::Crit_Min),
            comp: Vec::new(),
            name: String::from("crit min"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Current(Current::Min),
            comp: Vec::new(),
            name: String::from("min"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Current(Current::Max),
            comp: Vec::new(),
            name: String::from("max"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Current(Current::Crit_Max),
            comp: Vec::new(),
            name: String::from("crit max"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Current(Current::Average),
            comp: Vec::new(),
            name: String::from("avg"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Current(Current::Lowest),
            comp: Vec::new(),
            name: String::from("lowest"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Current(Current::Highest),
            comp: Vec::new(),
            name: String::from("highest"),
        },
    ];
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
    static ref POWER_COMMON_SENSORS: Vec<SubfeatureList> = vec![
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Alarm),
            comp: Vec::new(),
            name: String::new(),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Min_Alarm),
            comp: Vec::new(),
            name: String::from("MIN"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Max_Alarm),
            comp: Vec::new(),
            name: String::from("MAX"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Crit_Min_Alarm),
            comp: Vec::new(),
            name: String::from("LCRIT"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Crit_Max_Alarm),
            comp: Vec::new(),
            name: String::from("CRIT"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Cap_Alarm),
            comp: Vec::new(),
            name: String::from("CAP"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Max),
            comp: Vec::new(),
            name: String::from("max"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Min),
            comp: Vec::new(),
            name: String::from("min"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Crit_Min),
            comp: Vec::new(),
            name: String::from("lcrit"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Crit_Max),
            comp: Vec::new(),
            name: String::from("crit"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Cap),
            comp: Vec::new(),
            name: String::from("cap"),
        },
    ];
    static ref POWER_INST_SENSORS: Vec<SubfeatureList> = vec![
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Input_Lowest),
            comp: Vec::new(),
            name: String::from("lowest"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Input_Highest),
            comp: Vec::new(),
            name: String::from("highest"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Average),
            comp: Vec::new(),
            name: String::from("avg"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Average_Lowest),
            comp: Vec::new(),
            name: String::from("avg lowest"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Average_Highest),
            comp: Vec::new(),
            name: String::from("avg highest"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Average_Interval),
            comp: Vec::new(),
            name: String::from("interval"),
        },
    ];
    static ref POWER_AVG_SENSORS: Vec<SubfeatureList> = vec![
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Average_Lowest),
            comp: Vec::new(),
            name: String::from("lowest"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Average_Highest),
            comp: Vec::new(),
            name: String::from("highest"),
        },
        SubfeatureList {
            sf_type: SubfeatureType::Power(Power::Average_Interval),
            comp: Vec::new(),
            name: String::from("interval"),
        },
    ];
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

    let mut features = chip.features_iter().collect::<Vec<&Feature>>();
    features.sort_by(|a, b| a.get_type().cmp(&b.get_type()).then(a.name().cmp(b.name())));
    for feature in features.iter() {
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
