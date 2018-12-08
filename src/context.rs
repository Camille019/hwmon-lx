// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

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
        let config_file = config_file.into();

        let adapters = Rc::new(bus::read_sysfs_busses()?);

        // TODO
        if let Some(path) = config_file {
        } else {
        }

        Ok(Context { adapters })
    }

    pub(crate) fn adapters(&self) -> &Vec<BusAdapter> {
        &self.adapters.as_ref()
    }
}
