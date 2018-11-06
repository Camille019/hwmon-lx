// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use bus::{Bus, BusType};
use context::Context;
use error::*;
use feature::{Feature, FeatureType};
use subfeature::Subfeature;
use sysfs::*;

pub struct FeatureIter<'a> {
    inner: std::collections::hash_map::Values<'a, (FeatureType, u32), Feature>,
}

impl<'a> Iterator for FeatureIter<'a> {
    type Item = &'a Feature;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

pub struct Chip {
    path: PathBuf,
    prefix: String,
    bus: Bus,
    address: u32,
    features: HashMap<(FeatureType, u32), Feature>,
}

impl Chip {
    /// Chip prefix
    pub fn prefix(&self) -> &str {
        self.prefix.as_ref()
    }

    /// The chip address on the bus.
    pub fn address(&self) -> u32 {
        self.address
    }

    /// Return the sysfs directory path of the chip.
    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }

    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    /// Chip name from its internal representation.
    pub fn name(&self) -> String {
        match self.bus().get_type() {
            BusType::ISA => format!("{}-isa-{:04x}", self.prefix(), self.address()),
            BusType::PCI => format!("{}-pci-{:04x}", self.prefix(), self.address()),
            BusType::I2C => format!(
                "{}-i2C-{}-{:02x}",
                self.prefix(),
                self.bus.number(),
                self.address()
            ),
            BusType::SPI => format!(
                "{}-spi-{}-{:x}",
                self.prefix(),
                self.bus.number(),
                self.address()
            ),
            BusType::HID => format!(
                "{}-hid-{}-{:x}",
                self.prefix(),
                self.bus.number(),
                self.address()
            ),
            BusType::ACPI => format!("{}-acpi-{:x}", self.prefix(), self.address()),
            BusType::MDIO => format!("{}-mdio-{:x}", self.prefix(), self.address()),
            BusType::SCSI => format!(
                "{}-scsi-{}-{:x}",
                self.prefix(),
                self.bus.number(),
                self.address()
            ),
            BusType::Virtual => format!("{}-virtual-{:x}", self.prefix(), self.address()),
        }
    }

    /// Return the feature of the given type, if it exists, `None` otherwise.
    pub fn feature(&self, ftype: FeatureType, number: u32) -> Option<&Feature> {
        self.features.get(&(ftype, number))
    }

    /// An iterator visiting all features in arbitrary order.
    pub fn features_iter(&self) -> FeatureIter {
        FeatureIter {
            inner: self.features.values(),
        }
    }

    pub(crate) fn from_path<'a, T: Into<Option<&'a Path>>>(
        hwmon_path: &Path,
        dev_path: T,
        context: &Context,
    ) -> Result<Chip, ChipError> {
        let dev_path = dev_path.into();

        let prefix = sysfs_read_attr(hwmon_path, "name")?;

        // Find bus type
        let mut bus = Bus::new(BusType::Virtual, 0, context.clone());
        let mut address = 0u32;

        if let Some(dev_path) = dev_path {
            let dev_link_path = dev_path.read_link()?;
            let dev_name = dev_link_path.file_name().and_then(|s| s.to_str()).unwrap();

            let mut link_path = dev_path.to_owned();
            link_path.push("subsystem");
            let subsys_path = link_path.read_link()?;
            let subsys = subsys_path.file_name().and_then(|s| s.to_str()).unwrap();

            let (_bus, _address) = get_chip_bus_from_name(subsys, dev_name, context)?;
            bus = _bus;
            address = _address;
        }

        // read_dynamic_chip
        let mut chip = Chip {
            path: hwmon_path.to_owned(),
            prefix,
            bus,
            address,
            features: Default::default(),
        };

        chip.read_dynamic_chip()?;

        Ok(chip)
    }

    fn read_dynamic_chip(&mut self) -> Result<(), ChipError> {
        for entry in self
            .path
            .read_dir()?
            .filter_map(|x| x.ok())
            .filter(|entry| {
                entry
                    .file_type()
                    .map(|ftype| ftype.is_file())
                    .unwrap_or(false)
            }) {
            let path = entry.path();

            if let Ok((feature_number, subfeature)) = Subfeature::from_path(&path) {
                let feature_type = FeatureType::from(subfeature.get_type());
                let feature_path = self.path.as_ref();

                self.features
                    .entry((feature_type, feature_number))
                    .or_insert_with(|| Feature::new(feature_path, feature_type, feature_number))
                    .push_subfeature(subfeature)
                    .unwrap();
            } else {
                debug!("Skip file {:?}", &path);
            }
        }

        Ok(())
    }
}

fn get_chip_bus_from_name(
    subsytem: &str,
    device_name: &str,
    context: &Context,
) -> Result<(Bus, u32), ChipError> {
    let mut bus_type: BusType;
    let mut bus_number: i16;
    let address: u32;

    match subsytem {
        "i2c" => {
            // Device name Regex: "^[[:digit:]]+-[[:xdigit:]]+$"

            let args: Vec<&str> = device_name.split('-').collect();

            bus_number = i16::from_str(args.get(0).ok_or(ChipError::ParseBusInfo(BusType::SCSI))?)?;
            address = u32::from_str_radix(args.get(1).ok_or(ChipError::ParseBusInfo(BusType::SCSI))?, 16)?;

            // find out if legacy ISA or not
            if bus_number == 9191 {
                bus_type = BusType::ISA;
                bus_number = 0;
            } else {
                bus_type = BusType::I2C;
                let mut bus_path = PathBuf::from(SYSFS_MOUNT);
                bus_path.push(format!("class/i2c-adapter/i2c-{}/device/name", bus_number));

                if let Ok(mut bus_file) = std::fs::File::open(bus_path) {
                    let mut bus_name: String = String::new();
                    bus_file.read_to_string(&mut bus_name)?;
                    if bus_name == "ISA" {
                        bus_type = BusType::ISA;
                        bus_number = 0;
                    }
                }
            }
        }
        "spi" => {
            // Device name Regex "^spi[[:digit:]]+\.[[:digit:]]+$"

            let prefix = "spi";
            if !device_name.starts_with(prefix) || !(device_name.len() > prefix.len()) {
                return Err(ChipError::ParseBusInfo(BusType::SPI));
            }
            let (_, end) = device_name.split_at(prefix.len());
            let args: Vec<&str> = end.split('.').collect();

            address = u32::from_str(args.get(1).ok_or(ChipError::ParseBusInfo(BusType::SPI))?)?;
            bus_number = i16::from_str(args.get(0).ok_or(ChipError::ParseBusInfo(BusType::SPI))?)?;
            bus_type = BusType::SPI;
        }
        "pci" => {
            // Device name Regex: "^[[:xdigit:]]+:[[:xdigit:]]+:[[:xdigit:]]+\.[[:xdigit:]]+$"

            let args: Vec<&str> = device_name.split(':').collect();
            let args_bis: Vec<&str> = args.last().ok_or(ChipError::ParseBusInfo(BusType::PCI))?.split('.').collect();

            let _domain = u32::from_str_radix(args.get(0).ok_or(ChipError::ParseBusInfo(BusType::SCSI))?, 16)?;
            let _bus = u32::from_str_radix(args.get(1).ok_or(ChipError::ParseBusInfo(BusType::SCSI))?, 16)?;
            let _slot = u32::from_str_radix(args_bis.get(0).ok_or(ChipError::ParseBusInfo(BusType::SCSI))?, 16)?;
            let _fn = u32::from_str_radix(args_bis.get(1).ok_or(ChipError::ParseBusInfo(BusType::SCSI))?, 16)?;

            address = (_domain << 16) + (_bus << 8) + (_slot << 3) + _fn;
            bus_type = BusType::PCI;
            bus_number = 0;
        }
        "scsi" => {
            // Device name Regex: "^[[:digit:]]+:[[:digit:]]+:[[:digit:]]+:[[:xdigit:]]+$"

            let args: Vec<&str> = device_name.split(':').collect();

            let _bus = u32::from_str(args.get(1).ok_or(ChipError::ParseBusInfo(BusType::SCSI))?)?;
            let _slot = u32::from_str(args.get(2).ok_or(ChipError::ParseBusInfo(BusType::SCSI))?)?;
            let _fn = u32::from_str_radix(
                args.get(3).ok_or(ChipError::ParseBusInfo(BusType::SCSI))?,
                16,
            )?;

            address = (_bus << 8) + (_slot << 4) + _fn;
            bus_number = i16::from_str(args.get(0).ok_or(ChipError::ParseBusInfo(BusType::SCSI))?)?;
            bus_type = BusType::SCSI;
        }
        "platform" | "of_platform" => {
            bus_type = BusType::ISA;
            bus_number = 0;
            address = 0;
        }
        "acpi" => {
            bus_type = BusType::ACPI;
            bus_number = 0;
            address = 0;
        }
        "hid" => {
            bus_type = BusType::HID;
            bus_number = 0;
            address = 0;
        }
        "mdio_bus" => {
            bus_type = BusType::MDIO;
            bus_number = 0;
            address = 0;
        }
        _ => return Err(ChipError::UnknownDevice),
    }

    Ok((Bus::new(bus_type, bus_number, context.clone()), address))
}

pub fn read_sysfs_chips(context: &Context) -> Result<Vec<Chip>, Error> {
    let mut hwmon_path = PathBuf::from(SYSFS_MOUNT);
    hwmon_path.push("class/hwmon");

    let mut chips: Vec<Chip> = Vec::new();

    for entry in std::fs::read_dir(hwmon_path)? {
        let entry = entry?;
        let path = entry.path();

        let mut link_path = path.clone();
        link_path.push("device");
        let chip = if link_path.read_link().is_ok() {
            debug!("{:?}.read_link() -> Ok", link_path);

            // The attributes we want might be those of the hwmon class
            // device, or those of the device itself.
            match Chip::from_path(path.as_ref(), link_path.as_ref(), context) {
                Ok(chip) => Ok(chip),
                Err(e) => {
                    debug!("{:?}", e);
                    Chip::from_path(link_path.as_ref(), link_path.as_ref(), context)
                }
            }
        } else {
            // No device link? Treat as virtual
            debug!("{:?}.read_link() -> Err", link_path);
            Chip::from_path(path.as_ref(), None, context)
        };

        if let Ok(chip) = chip {
            debug!("Add chip '{}'", chip.name());
            chips.push(chip);
        }
    }

    Ok(chips)
}
