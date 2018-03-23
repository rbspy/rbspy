use std::io::prelude::*;
use std::io::BufReader;
use crate::core::types::{StackTrace, StackFrame};

use failure::Error;
use serde_json;

use super::*;

pub(crate) struct Data(Vec<Vec<StackFrame>>);

impl Storage for Data {
    fn from_reader<R: Read>(r: R) -> Result<Data, Error> {
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

impl From<Data> for v1::Data {
    fn from(d: Data) -> v1::Data {
        let x: Vec<StackTrace> = d.0.into_iter().map(std::convert::Into::into).collect();
        v1::Data(x)
    }
}

impl From<Vec<StackFrame>> for StackTrace {
    fn from(trace: Vec<StackFrame>) -> StackTrace {
        StackTrace{pid: None, on_cpu: None, thread_id: None, trace}
    }
}
