use std::collections::{HashMap, HashSet};
use std::io;
use std::io::Write;
use std::fs::File;

use core::types::StackFrame;

use failure::{Error, ResultExt};

use serde_json;

/*
 * This file contains code to export rbspy profiles for use in https://speedscope.app
 *
 * The speedscope file format is specified via a JSON schema.
 * The latest schema can be found here: https://speedscope.app/file-format-schema.json
 *
 * This JSON schema conveniently allows to generate type bindings for generating JSON.
 * You can use https://app.quicktype.io/ to generate serde_json Rust bindings for the
 * given JSON schema.
 *
 * There are multiple variants of the file format. The variant we're going to generate
 * is the "type: sampled" profile, since it most closely maps to rbspy's data recording
 * structure.
 */

#[derive(Debug, Serialize, Deserialize)]
struct SpeedscopeFile {
    #[serde(rename = "$schema")]
    schema: String,
    profiles: Vec<Profile>,
    shared: Shared,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Profile {
    #[serde(rename = "type")]
    profile_type: ProfileType,

    name: String,
    unit: ValueUnit,

    #[serde(rename = "startValue")]
    start_value: f64,

    #[serde(rename = "endValue")]
    end_value: f64,

    samples: Vec<Vec<usize>>,
    weights: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Shared {
    frames: Vec<Frame>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Frame {
    name: String,
    file: Option<String>,
    line: Option<u32>,
    col: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
enum ProfileType {
    #[serde(rename = "evented")]
    Evented,
    #[serde(rename = "sampled")]
    Sampled,
}

#[derive(Debug, Serialize, Deserialize)]
enum ValueUnit {
    #[serde(rename = "bytes")]
    Bytes,
    #[serde(rename = "microseconds")]
    Microseconds,
    #[serde(rename = "milliseconds")]
    Milliseconds,
    #[serde(rename = "nanoseconds")]
    Nanoseconds,
    #[serde(rename = "none")]
    None,
    #[serde(rename = "seconds")]
    Seconds,
}

impl SpeedscopeFile {
  pub fn new(profile: Profile, frames: Vec<Frame>) -> SpeedscopeFile {
    SpeedscopeFile {
      // This is always the same
      schema: "https://www.speedscope.app/file-format-schema.json".to_string(),

      // This is the version of the file format we're targeting
      version: "0.2.0".to_string(),

      profiles: vec![profile],

      shared: Shared {
        frames
      }
    }
  }
}

impl Profile {
    pub fn new() -> Profile {
        Profile {
            profile_type: ProfileType::Sampled,

            name: "".to_string(),
            unit: ValueUnit::None,

            start_value: 0.0,
            end_value: 0.0,

            samples: vec![],
            weights: vec![],
        }
    }
}

pub struct Stats {
    pub samples: Vec<Vec<usize>>,
    pub frames: Vec<Frame>,
    pub frameToIndex: HashMap<StackFrame, usize>
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            samples: vec![],
            frames: vec![],
            frameToIndex: HashMap::new()
        }
    }

    pub fn record(&mut self, stack: &Vec<StackFrame>) -> Result<(), io::Error> {
        let frameIndices = stack.into_iter().map(|frame| {
            match self.frameToIndex.get(frame) {
                Some(index) => index,
                None => {
                    let index = self.frames.len();
                    self.frameToIndex.insert(*frame, index);
                    self.frames.push(Frame::new(frame.name, frame.relative_path, frame.lineno));
                    index
                }
            }
        });
        self.samples.push(frameIndices);
        Ok(())
    }

    pub fn write(&self, w: File) -> Result<(), Error> {
        Ok(())
    }
}