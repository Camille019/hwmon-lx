// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fs::OpenOptions;
use std::io::{self, Read};
use std::path::Path;

pub const SYSFS_MOUNT: &str = "/sys";

pub fn sysfs_read_file(path: &Path) -> io::Result<String> {
    let mut file = OpenOptions::new().read(true).write(false).open(path)?;
    let mut buf: String = String::new();
    file.read_to_string(&mut buf)?;
    let len = buf.trim_right().len();
    buf.truncate(len);

    Ok(buf)
}

pub fn sysfs_read_attr(path: &Path, attr: &str) -> io::Result<String> {
    let mut path = path.to_owned();
    path.push(attr);

    sysfs_read_file(path.as_ref())
}
