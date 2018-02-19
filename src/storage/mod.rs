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
/// changes _way_ to much.
extern crate flate2;

use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::path::Path;

use core::initialize::StackFrame;

use self::flate2::Compression;
use failure::Error;
use serde_json;

mod v0;

pub struct Store {
    encoder: flate2::write::GzEncoder<File>,
}

impl Store {
    pub fn new(out_path: &Path) -> Result<Store, io::Error> {
        let file = File::create(out_path)?;
        let mut encoder = flate2::write::GzEncoder::new(file, Compression::default());
        encoder.write("rbspy00\n".as_bytes())?;
        Ok(Store { encoder })
    }

    pub fn write(&mut self, trace: &Vec<StackFrame>) -> Result<(), Error> {
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

// Write impls like this for every storage version (no need for the internal
// one, since `impl<T> From<T> for T` exists.
//
// impl From<v7::Data> for JuliaData {
//     fn from(d: v7::Data) -> JuliaData {
//         unimplemented!();
//     }
// }


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
        } else {
            Err(StorageError::Invalid)
        }
    }
}

#[derive(Fail, Debug)]
pub(crate) enum StorageError {
    /// The file doesn't begin with the magic tag `rbspy` + version number.
    #[fail(display = "Invalid rbpsy file")]
    Invalid,
    /// The version of the rbspy file can't be handled by this version of rbspy.
    #[fail(display = "Cannot handle rbspy format {}", _0)]
    UnknownVersion(Version),
    /// An IO error occurred.
    #[fail(display = "IO error {:?}", _0)]
    Io(#[cause] io::Error),
}

/// Types that can be deserialized from an `io::Read` into something convertible
/// to the current internal form.
pub(crate) trait Storage: Into<v0::Data> {
    fn from_reader<R: Read>(r: R) -> Result<Self, Error>;
    fn version() -> Version;
}

fn read_version(r: &mut Read) -> Result<Version, StorageError> {
    let mut buf = [0u8; 8];
    // TODO: I don't know how to failure good, so this doesn't work.
    r.read(&mut buf).map_err(StorageError::Io)?;
    match &buf[..5] {
        b"rbspy" => Ok(Version::try_from(&buf[5..])?),
        _ => Err(StorageError::Invalid),
    }
}

pub(crate) fn from_reader<R: Read>(r: R) -> Result<Vec<Vec<StackFrame>>, Error> {
    // This will read 8 bytes, leaving the reader's cursor at the start of the
    // "real" data.
    let mut reader = flate2::read::GzDecoder::new(r);
    let version = read_version(&mut reader)?;
    match version {
        Version(0) => {
            let intermediate = v0::Data::from_reader(reader)?;
            Ok(intermediate.into())
        }
        v => Err(StorageError::UnknownVersion(v).into()),
    }
}
