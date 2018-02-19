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
extern crate flate2;

use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};

use core::initialize::StackFrame;

use self::flate2::write::GzEncoder;
use self::flate2::Compression;
use failure::Error;
use serde_json;

pub struct Store {
    encoder: GzEncoder<File>,
}

impl Store {
    pub fn new(out_path: &Path) -> Result<Store, io::Error> {
        let file = File::create(out_path)?;
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder.write("rbspy00\n".as_bytes())?;
        Ok(Store { encoder })
    }

    pub fn write(&mut self, trace: &Vec<StackFrame>) -> Result<(), Error> {
        let json = serde_json::to_string(trace)?;
        self.encoder.write(json.as_bytes())?;
        Ok(())
    }

    pub fn complete(self) {
        drop(self.encoder)
    }
}
