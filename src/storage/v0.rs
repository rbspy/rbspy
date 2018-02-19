use std::io::prelude::*;
use std::io::BufReader;
use core::initialize::StackFrame;

use failure::Error;
use serde_json;

use super::*;

// The first version is just your internal representation.
pub(crate) type Data = Vec<Vec<StackFrame>>;

impl Storage for Data {
    fn from_reader<R: Read>(r: R) -> Result<Data, Error> {
        let reader = BufReader::new(r);
        let mut result = Vec::new();
        for line in reader.lines() {
            let trace = serde_json::from_str(&line?)?;
            result.push(trace);
        }
        Ok(result)
    }
    fn version() -> Version {
        // The "version" for the internal representation is bumped every time
        // you make a change to it, copying the old struct into a new file at
        // `src/vblah`.
        Version(0)
    }
}
