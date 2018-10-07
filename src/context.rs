// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str::FromStr;

use failure::Error;
use regex::Regex;

use bus::BusType;
use sysfs::*;

#[derive(Clone)]
pub struct Context {
    buses: Rc<Vec<Bus>>,
}

impl Context {
    pub fn new<'a, T: Into<Option<&'a Path>>>(config_file: T) -> Result<Context, Error> {
        let config_file = config_file.into();

        let buses = Rc::new(read_sysfs_buses()?);

        // TODO
        if let Some(path) = config_file {
        } else {
        }

        Ok(Context { buses })
    }

    pub(crate) fn buses(&self) -> Rc<Vec<Bus>> {
        Rc::clone(&self.buses)
    }
}

#[derive(Clone, Debug)]
pub struct Bus {
    adapter: String,
    bus_type: BusType,
    bus_number: i16,
}

impl Bus {
    fn from_path(path: &Path) -> Result<Option<Bus>, Error> {
        lazy_static! {
            static ref RE_I2C: Regex = Regex::new(r"^i2c\-([[:digit:]]+)").unwrap();
        }

        let classdev = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format_err!(""))?;

        let caps = RE_I2C
            .captures(classdev)
            .ok_or_else(|| format_err!("Failed to read I2C bus"))?;

        let bus_number = i16::from_str(&caps[1])?;

        if bus_number == 9191 {
            return Ok(None); // legacy ISA
        }

        // Get the adapter name from the classdev "name" attribute
        // (Linux 2.6.20 and later). If it fails, fall back to
        // the device "name" attribute (for older kernels).
        let adapter =
            sysfs_read_attr(path, "name").or_else(|_| sysfs_read_attr(path, "device/name"))?;

        Ok(Some(Bus {
            adapter,
            bus_type: BusType::I2C,
            bus_number,
        }))
    }

    pub fn adapter(&self) -> &str {
        self.adapter.as_ref()
    }

    pub fn get_type(&self) -> BusType {
        self.bus_type
    }

    pub fn number(&self) -> i16 {
        self.bus_number
    }
}

fn read_sysfs_buses() -> Result<Vec<Bus>, Error> {
    let mut res = Vec::new();

    let mut adapter_path = PathBuf::from(SYSFS_MOUNT);
    adapter_path.push("class/i2c-adapter");

    if adapter_path.is_dir() {
        for entry in std::fs::read_dir(adapter_path)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(bus) = Bus::from_path(path.as_ref())? {
                res.push(bus);
            }
        }
    } else {
        let mut i2c_path = PathBuf::from(SYSFS_MOUNT);
        i2c_path.push("bus/i2c/devices");

        for entry in std::fs::read_dir(i2c_path)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(bus) = Bus::from_path(path.as_ref())? {
                res.push(bus);
            }
        }
    }

    Ok(res)
}
