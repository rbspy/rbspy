use std::io::prelude::*;
use std::io::BufReader;
use crate::core::types::{Header, StackTrace};

use failure::Error;
use serde_json;

use super::*;

pub(crate) struct Data {
    pub header: Header,
    pub traces: Vec<StackTrace>,
}

impl Storage for Data {
    fn from_reader<R: Read>(r: R) -> Result<Data, Error> {
        let reader = BufReader::new(r);
        let mut result = Vec::new();
        let mut lines = reader.lines();
        let mut header_line = lines.next().unwrap().unwrap();

        for line in lines {
            let trace: StackTrace = serde_json::from_str(&line?)?;
            result.push(trace);
        }

        Ok(Data {
            header: serde_json::from_str(&header_line)?,
            traces: result,
        })
    }

    fn append(mut self, mut rhs: Self) -> Result<Data, Error> {
        let sample_rate = if self.header.sample_rate == rhs.header.sample_rate {
            self.header.sample_rate
        } else {
            None
        };

        let rbspy_version = if self.header.rbspy_version == rhs.header.rbspy_version {
            self.header.rbspy_version.clone()
        } else {
            None
        };

        self.header = Header {
            sample_rate: sample_rate,
            rbspy_version: rbspy_version,
            start_time: None
        };
        self.traces.append(&mut rhs.traces);

        Ok(self)
    }

    fn version() -> Version {
        Version(2)
    }
}
