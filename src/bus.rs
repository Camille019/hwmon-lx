// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use context::Context;
use error::*;
use sysfs::*;

#[allow(non_snake_case)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BusType {
    I2C,
    ISA,
    PCI,
    SPI,
    Virtual,
    ACPI,
    HID,
    MDIO,
    SCSI,
}

impl fmt::Display for BusType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BusType::I2C => write!(f, "I2C"),
            BusType::ISA => write!(f, "ISA"),
            BusType::PCI => write!(f, "PCI"),
            BusType::SPI => write!(f, "SPI"),
            BusType::Virtual => write!(f, "Virtual"),
            BusType::ACPI => write!(f, "ACPI"),
            BusType::HID => write!(f, "HID"),
            BusType::MDIO => write!(f, "MDIO"),
            BusType::SCSI => write!(f, "SCSI"),
        }
    }
}

#[derive(Clone)]
pub struct Bus {
    bus_type: BusType,
    bus_number: i16,
    context: Context,
}

impl Bus {
    pub fn new(bus_type: BusType, bus_number: i16, context: Context) -> Bus {
        Bus {
            bus_type,
            bus_number,
            context,
        }
    }

    /// Return the bus type
    pub fn get_type(&self) -> BusType {
        self.bus_type
    }

    /// Return the bus number
    pub fn number(&self) -> i16 {
        self.bus_number
    }

    /// Return the adapter name of the bus. If it could not be found, it returns `None`
    pub fn adapter_name(&self) -> Option<&str> {
        match self.bus_type {
            BusType::ISA => Some("ISA adapter"),
            BusType::PCI => Some("PCI adapter"),
            // SPI should not be here, but for now SPI adapters have no name
            // so we don't have any custom string to return.
            BusType::SPI => Some("SPI adapter"),
            BusType::Virtual => Some("Virtual device"),
            BusType::ACPI => Some("ACPI interface"),
            // HID should probably not be there either, but I don't know if
            // HID buses have a name nor where to find it.
            BusType::HID => Some("HID adapter"),
            BusType::MDIO => Some("MDIO adapter"),
            BusType::SCSI => Some("SCSI adapter"),
            // Bus types with several instances
            BusType::I2C => {
                for adapter in self.context.adapters().iter() {
                    if adapter.bus_type() == self.bus_type
                        && adapter.bus_number() == self.bus_number
                    {
                        return Some(adapter.name());
                    }
                }
                None
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BusAdapter {
    name: String,
    bus_type: BusType,
    bus_number: i16,
}

impl BusAdapter {
    fn from_sysfs_i2c(path: &Path) -> Result<Option<BusAdapter>, Error> {
        let classdev = path.file_name().and_then(|s| s.to_str()).unwrap();

        let prefix = "i2c-";
        if !classdev.starts_with(prefix) || !(classdev.len() > prefix.len()) {
            return Err(Error::ParseBusName(BusType::I2C));
        }
        let (_, digits) = classdev.split_at(prefix.len());

        let bus_number = i16::from_str(digits)?;

        if bus_number == 9191 {
            return Ok(None); // legacy ISA
        }

        // Get the adapter name from the classdev "name" attribute
        // (Linux 2.6.20 and later). If it fails, fall back to
        // the device "name" attribute (for older kernels).
        let name =
            sysfs_read_attr(path, "name").or_else(|_| sysfs_read_attr(path, "device/name"))?;

        Ok(Some(BusAdapter {
            name,
            bus_type: BusType::I2C,
            bus_number,
        }))
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn bus_type(&self) -> BusType {
        self.bus_type
    }

    pub fn bus_number(&self) -> i16 {
        self.bus_number
    }
}

pub(crate) fn read_sysfs_busses() -> Result<Vec<BusAdapter>, Error> {
    let mut res = Vec::new();

    let mut adapter_path = PathBuf::from(SYSFS_MOUNT);
    adapter_path.push("class/i2c-adapter");

    if adapter_path.is_dir() {
        for entry in fs::read_dir(adapter_path)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(bus) = BusAdapter::from_sysfs_i2c(path.as_ref())? {
                res.push(bus);
            }
        }
    } else {
        let mut i2c_path = PathBuf::from(SYSFS_MOUNT);
        i2c_path.push("bus/i2c/devices");

        for entry in fs::read_dir(i2c_path)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(bus) = BusAdapter::from_sysfs_i2c(path.as_ref())? {
                res.push(bus);
            }
        }
    }

    Ok(res)
}
