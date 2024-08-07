// SPDX-FileCopyrightText: 2018 Camille019
// SPDX-License-Identifier: MPL-2.0

use std::error;
use std::fmt;
use std::io;
use std::num;

use crate::bus::BusType;

#[derive(Debug)]
pub enum Error {
    Access(&'static str),
    Io(io::Error),
    ParseFloat(num::ParseFloatError),
    ParseInt(num::ParseIntError),
    ParseBusName(BusType),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::Io(ref err) => Some(err),
            Error::ParseFloat(ref err) => Some(err),
            Error::ParseInt(ref err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Access(ref err) => write!(f, "Access error: {}", err),
            Error::Io(ref err) => write!(f, "IO error: {}", err),
            Error::ParseFloat(ref err) => write!(f, "ParseFloat error: {}", err),
            Error::ParseInt(ref err) => write!(f, "ParseInt error: {}", err),
            Error::ParseBusName(ref bus) => write!(f, "Failed to parse {} bus name", bus),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<num::ParseFloatError> for Error {
    fn from(err: num::ParseFloatError) -> Error {
        Error::ParseFloat(err)
    }
}

impl From<num::ParseIntError> for Error {
    fn from(err: num::ParseIntError) -> Error {
        Error::ParseInt(err)
    }
}

#[derive(Debug)]
pub(crate) enum ChipError {
    Io(io::Error),
    ParseBusInfo(BusType),
    ParseInt(num::ParseIntError),
    UnknownDevice,
}

impl error::Error for ChipError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            ChipError::Io(ref err) => Some(err),
            ChipError::ParseBusInfo(_) => None,
            ChipError::ParseInt(ref err) => Some(err),
            ChipError::UnknownDevice => None,
        }
    }
}

impl fmt::Display for ChipError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ChipError::Io(ref err) => write!(f, "IO error: {}", err),
            ChipError::ParseBusInfo(ref bus) => write!(f, "Failed to read {} bus info", bus),
            ChipError::ParseInt(ref err) => write!(f, "ParseInt error: {}", err),
            ChipError::UnknownDevice => write!(f, "Unknown device"),
        }
    }
}

impl From<io::Error> for ChipError {
    fn from(err: io::Error) -> ChipError {
        ChipError::Io(err)
    }
}

impl From<num::ParseIntError> for ChipError {
    fn from(err: num::ParseIntError) -> ChipError {
        ChipError::ParseInt(err)
    }
}

#[derive(Debug)]
pub(crate) enum FeatureError {
    SubfeatureType,
}

impl error::Error for FeatureError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            FeatureError::SubfeatureType => None,
        }
    }
}

impl fmt::Display for FeatureError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FeatureError::SubfeatureType => {
                write!(f, "The subfeature type does not match the feature type")
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum SubfeatureError {
    Io(io::Error),
    Invalid,
    ParseInt(num::ParseIntError),
    Unknown,
}

impl error::Error for SubfeatureError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            SubfeatureError::Io(ref err) => Some(err),
            SubfeatureError::Invalid => None,
            SubfeatureError::ParseInt(ref err) => Some(err),
            SubfeatureError::Unknown => None,
        }
    }
}

impl fmt::Display for SubfeatureError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SubfeatureError::Io(ref err) => write!(f, "IO error: {}", err),
            SubfeatureError::Invalid => write!(f, "Invalid subfeature"),
            SubfeatureError::ParseInt(ref err) => write!(f, "ParseInt error: {}", err),
            SubfeatureError::Unknown => write!(f, "Unknown subfeature"),
        }
    }
}

impl From<io::Error> for SubfeatureError {
    fn from(err: io::Error) -> SubfeatureError {
        SubfeatureError::Io(err)
    }
}

impl From<num::ParseIntError> for SubfeatureError {
    fn from(err: num::ParseIntError) -> SubfeatureError {
        SubfeatureError::ParseInt(err)
    }
}
