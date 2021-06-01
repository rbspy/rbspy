use crate::core::types::{Header, StackFrame, StackTrace};
use std::io::prelude::*;
use std::io::BufReader;

use super::*;

pub(crate) struct Data(Vec<Vec<StackFrame>>);

impl Storage for Data {
    fn from_reader<R: Read>(r: R) -> Result<Data> {
        let reader = BufReader::new(r);
        let mut result = Vec::new();
        for line in reader.lines() {
            let trace: Vec<StackFrame> = serde_json::from_str(&line?)?;
            result.push(trace);
        }
        Ok(Data(result))
    }
    fn version() -> Version {
        Version(0)
    }
}

impl From<Data> for v2::Data {
    fn from(d: Data) -> v2::Data {
        let x: Vec<StackTrace> = d.0.into_iter().map(std::convert::Into::into).collect();
        v2::Data {
            header: Header {
                sample_rate: None,
                rbspy_version: None,
                start_time: None,
            },
            traces: x,
        }
    }
}

impl From<Vec<StackFrame>> for StackTrace {
    fn from(trace: Vec<StackFrame>) -> StackTrace {
        StackTrace {
            pid: None,
            trace,
            thread_id: None,
            time: None,
        }
    }
}
