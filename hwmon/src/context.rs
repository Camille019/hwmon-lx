// SPDX-FileCopyrightText: 2018 Camille019
// SPDX-License-Identifier: MPL-2.0

use std::collections::HashSet;
use std::path::Path;
use std::rc::Rc;

use crate::bus::{self, BusAdapter};
use crate::error::*;

#[cfg(feature = "sensorsconf")]
use crate::parser::CfgFile;

#[cfg(feature = "sensorsconf")]
const DFLT_CONF3_FILE: &str = "/etc/sensors3.conf";
#[cfg(feature = "sensorsconf")]
const DFLT_CONF_FILE: &str = "/etc/sensors.conf";

#[derive(Clone)]
pub struct Context {
    adapters: Rc<HashSet<BusAdapter>>,
    #[cfg(feature = "sensorsconf")]
    config: Rc<CfgFile>,
}

impl Context {
    pub fn new<'a, T: Into<Option<&'a Path>>>(config_file: T) -> Result<Context, Error> {
        #[cfg(feature = "sensorsconf")]
        let config_file = config_file.into();
        #[cfg(not(feature = "sensorsconf"))]
        let _config_file = config_file.into();

        let adapters = Rc::new(bus::read_sysfs_busses()?);

        #[cfg(feature = "sensorsconf")]
        let config: Rc<CfgFile> = Rc::new(if let Some(path) = config_file {
            crate::parser::parse_configuration_file(path)?
        } else {
            if Path::new(DFLT_CONF3_FILE).is_file() {
                crate::parser::parse_configuration_file(DFLT_CONF3_FILE)?
            } else if Path::new(DFLT_CONF_FILE).is_file() {
                crate::parser::parse_configuration_file(DFLT_CONF_FILE)?
            } else {
                CfgFile::default()
            }
        });

        Ok(Context {
            adapters,
            #[cfg(feature = "sensorsconf")]
            config,
        })
    }

    pub(crate) fn adapters(&self) -> &HashSet<BusAdapter> {
        self.adapters.as_ref()
    }
}
