// SPDX-FileCopyrightText: 2018 Camille019
// SPDX-License-Identifier: MPL-2.0

use std::fs::OpenOptions;
use std::io::{self, Read};
use std::path::Path;

pub const SYSFS_MOUNT: &str = "/sys";

pub fn sysfs_read_file(path: &Path) -> io::Result<String> {
    let mut file = OpenOptions::new().read(true).write(false).open(path)?;
    let mut buf: String = String::new();
    file.read_to_string(&mut buf)?;
    let len = buf.trim_end().len();
    buf.truncate(len);

    Ok(buf)
}

pub fn sysfs_read_attr(path: &Path, attr: &str) -> io::Result<String> {
    let mut path = path.to_owned();
    path.push(attr);

    sysfs_read_file(path.as_ref())
}
