// SPDX-FileCopyrightText: 2018 Camille019
// SPDX-License-Identifier: MPL-2.0

use std::path::Path;
use std::rc::Rc;

use crate::bus::{self, BusAdapter};
use crate::error::*;

#[derive(Clone)]
pub struct Context {
    adapters: Rc<Vec<BusAdapter>>,
}

impl Context {
    pub fn new<'a, T: Into<Option<&'a Path>>>(config_file: T) -> Result<Context, Error> {

        #[cfg(feature = "sensorsconf")]
        let config_file = config_file.into();
        #[cfg(not(feature = "sensorsconf"))]
        let _config_file = config_file.into();

        let adapters = Rc::new(bus::read_sysfs_busses()?);

        #[cfg(feature = "sensorsconf")]
        if let Some(path) = config_file {
            unimplemented!()
        } else {
            unimplemented!()
        }

        Ok(Context { adapters })
    }

    pub(crate) fn adapters(&self) -> &Vec<BusAdapter> {
        self.adapters.as_ref()
    }
}
