use crate::core::types::{Header, StackTrace};
use std::io::prelude::*;
use std::io::BufReader;

use super::*;

pub(crate) struct Data {
    #[allow(dead_code)]
    pub header: Header,
    pub traces: Vec<StackTrace>,
}

impl Storage for Data {
    fn from_reader<R: Read>(r: R) -> Result<Data, Error> {
        let reader = BufReader::new(r);
        let mut result = Vec::new();
        let mut lines = reader.lines();
        let header_line = lines.next().unwrap().unwrap();
        for line in lines {
            let trace: StackTrace = serde_json::from_str(&line?)?;
            result.push(trace);
        }
        Ok(Data {
            header: serde_json::from_str(&header_line)?,
            traces: result,
        })
    }
    fn version() -> Version {
        Version(2)
    }
}
