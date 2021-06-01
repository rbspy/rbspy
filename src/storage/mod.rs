/// Storage formats, and io functions for rbspy's internal raw storage format.
///
/// rbspy has a versioned "raw" storage format. The versioning info is stored,
/// along with a "magic number" at the start of the file. The magic number plus
/// version are the first 8 bytes of the file, and are represented as
///
///     b"rbspyXY\n"
///
/// Here, `XY` is a decimal number in [0-99]
///
/// The use of b'\n' as a terminator effectively reserves a byte, and provides
/// flexibility to go to a different version encoding scheme if this format
/// changes _way_ too much.
extern crate anyhow;
extern crate flate2;

use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::time::SystemTime;

use crate::core::types::Header;
use crate::core::types::StackTrace;

use self::flate2::Compression;

use anyhow::{Error, Result};
use thiserror::Error;

mod v0;
mod v1;
mod v2;

pub struct Store {
    encoder: flate2::write::GzEncoder<File>,
}

impl Store {
    pub fn new(out_path: &Path, sample_rate: u32) -> Result<Store, io::Error> {
        let file = File::create(out_path)?;
        let mut encoder = flate2::write::GzEncoder::new(file, Compression::default());
        encoder.write_all("rbspy02\n".as_bytes())?;

        let json = serde_json::to_string(&Header {
            sample_rate: Some(sample_rate),
            rbspy_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            start_time: Some(SystemTime::now()),
        })?;
        writeln!(&mut encoder, "{}", json)?;

        Ok(Store { encoder })
    }

    pub fn write(&mut self, trace: &StackTrace) -> Result<(), Error> {
        let json = serde_json::to_string(trace)?;
        writeln!(&mut self.encoder, "{}", json)?;
        Ok(())
    }

    pub fn complete(self) {
        drop(self.encoder)
    }
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct Version(u64);

impl ::std::fmt::Display for Version {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Version {
    /// Parse bytes to a version.
    ///
    /// # Errors
    /// Fails with `StorageError::Invalid` if the version tag is in an unknown
    /// format.
    fn try_from(b: &[u8]) -> Result<Version, StorageError> {
        if &b[0..3] == "00\n".as_bytes() {
            Ok(Version(0))
        } else if &b[0..3] == "01\n".as_bytes() {
            Ok(Version(1))
        } else if &b[0..3] == "02\n".as_bytes() {
            Ok(Version(2))
        } else {
            Err(StorageError::Invalid)
        }
    }
}

#[derive(Error, Debug)]
pub(crate) enum StorageError {
    /// The file doesn't begin with the magic tag `rbspy` + version number.
    #[error("Invalid rbspy file")]
    Invalid,
    /// The version of the rbspy file can't be handled by this version of rbspy.
    #[error("Cannot handle rbspy format {}", _0)]
    UnknownVersion(Version),
    /// An IO error occurred.
    #[error("IO error {:?}", _0)]
    Io(io::Error),
}

/// Types that can be deserialized from an `io::Read` into something convertible
/// to the current internal form.
pub(crate) trait Storage: Into<v2::Data> {
    fn from_reader<R: Read>(r: R) -> Result<Self>;
    fn version() -> Version;
}

fn read_version(r: &mut dyn Read) -> Result<Version, StorageError> {
    let mut buf = [0u8; 8];
    // TODO: I don't know how to failure good, so this doesn't work.
    r.read(&mut buf).map_err(StorageError::Io)?;
    match &buf[..5] {
        b"rbspy" => Ok(Version::try_from(&buf[5..])?),
        _ => Err(StorageError::Invalid),
    }
}

pub(crate) fn from_reader<R: Read>(r: R) -> Result<v2::Data, Error> {
    // This will read 8 bytes, leaving the reader's cursor at the start of the
    // "real" data.
    let mut reader = flate2::read::GzDecoder::new(r);
    let version = read_version(&mut reader)?;
    match version {
        Version(0) => {
            let intermediate = v0::Data::from_reader(reader)?;
            Ok(intermediate.into())
        }
        Version(1) => {
            let intermediate = v1::Data::from_reader(reader)?;
            Ok(intermediate.into())
        }
        Version(2) => {
            let intermediate = v2::Data::from_reader(reader)?;
            Ok(intermediate)
        }
        v => Err(StorageError::UnknownVersion(v).into()),
    }
}
