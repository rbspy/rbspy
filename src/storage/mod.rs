/// Storage formats, and io functions for rbspy's internal raw storage format.
///
/// rbspy has a versioned "raw" storage format. The versioning info is stored,
/// along with a "magic number" at the start of the file. The magic number plus
/// version are the first 8 bytes of the file, and are represented as
///
///     b"rbspyXY\n"
///
/// Here, `XY` is [Julia: your choice of...]
///   - decimal number in [0-99]
///   - hex number in [0-255]
///   - base32 number in [0-1023]
///   - base64 number in [0-4096]
///
/// The use of b'\n' as a terminator effectively reserves a byte, and provides
/// flexibility to go to a different version encoding scheme if this format
/// changes _way_ to much.

use std::io;
use std::io::prelude::*;

use failure::{Fail, ResultExt};

mod v0;

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd)]
struct Version(u64);

/// The internal representation; probably actually in another file.
struct JuliaData;

// You have impls like this for every storage version (no need for the internal
// one, since `impl<T> From<T> for T` exists.
//
// impl From<v7::Data> for JuliaData {
//     fn from(d: v7::Data) -> JuliaData {
//         unimplemented!();
//     }
// }

impl Storage for JuliaData {
    fn from_reader<R: Read>(r: R) -> Result<JuliaData, StorageError> {
        // Probably something like `serde_json::from_reader(r) together with
        // error translation.
        unimplemented!();
    }
    fn version() -> Version {
        // The "version" for the internal representation is bumped every time
        // you make a change to it, copying the old struct into a new file at
        // `src/vblah`.
        Version(0)
    }
}

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
        unimplemented!();
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
/// to `JuliaData`.
pub(crate) trait Storage: Into<JuliaData> {
    fn from_reader<R: Read>(r: R) -> Result<Self, StorageError>;
    fn version() -> Version;
}

fn read_version(r: &Read) -> Result<Version, StorageError> {
    let buf = [0u8; 8];
    // TODO: I don't know how to failure good, so this doesn't work.
    r.read(&mut buf)
        .context("Could not read magic number")?;
    match &buf[..5] {
        b"rbspy" => Ok(Version::try_from(&buf[5..])?),
        _ => Err(StorageError::Invalid),
    }
}

pub(crate) fn from_reader<R: Read>(r: R) -> Result<JuliaData, StorageError> {
    // This will read 8 bytes, leaving the reader's cursor at the start of the
    // "real" data.
    let version = read_version(&r)?;
    match version {
        Version(0) => {
            let intermediate = v0::Data::from_reader(r)?;
            Ok(intermediate.into())
        }
        v => Err(StorageError::UnknownVersion(v)),
    }
}
