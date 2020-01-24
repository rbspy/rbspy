use std::io::prelude::*;
use std::io::BufReader;
use crate::core::types::{Header, StackTrace};

use failure::Error;
use serde_json;

use super::*;

pub(crate) struct Data(pub Vec<StackTrace>);

impl Storage for Data {
    fn from_reader<R: Read>(r: R) -> Result<Data, Error> {
        let reader = BufReader::new(r);
        let mut result = Vec::new();
        for line in reader.lines() {
            let trace: StackTrace = serde_json::from_str(&line?)?;
            result.push(trace);
        }
        Ok(Data(result))
    }

    fn append(mut self, r: Self) -> Result<Data, Error> {
      unimplemented!();
    }

    fn version() -> Version {
        Version(1)
    }
}

impl From<Data> for v2::Data {
    fn from(d: Data) -> v2::Data {
        v2::Data {
            header: Header {
                sample_rate: None,
                rbspy_version: None,
                start_time: None,
            },
            traces: d.0,
        }
    }
}
