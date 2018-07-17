use std::collections::{HashMap};
use std::io;
use std::io::Write;
use std::fs::File;

use core::types::StackFrame;

use failure::{Error};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
  pub fn new(samples: Vec<Vec<usize>>, frames: Vec<Frame>) -> SpeedscopeFile {
    let end_value = samples.len().clone();

    let weights: Vec<f64> = (&samples).into_iter().map(|_s| 1 as f64).collect();

    SpeedscopeFile {
      // This is always the same
      schema: "https://www.speedscope.app/file-format-schema.json".to_string(),

      // This is the version of the file format we're targeting
      version: "0.2.0".to_string(),

      profiles: vec![Profile {
        profile_type: ProfileType::Sampled,

        name: "".to_string(),
        unit: ValueUnit::None,

        start_value: 0.0,
        end_value: end_value as f64,

        samples: samples,
        weights: weights
      }],

      shared: Shared {
          frames: frames
      }
    }
  }
}

impl Frame {
    pub fn new(stack_frame: &StackFrame) -> Frame {
        Frame {
            name: stack_frame.name.clone(),
            file: Some(stack_frame.relative_path.clone()),
            line: None,
            col: None
        }
    }
}

pub struct Stats {
    samples: Vec<Vec<usize>>,
    frames: Vec<Frame>,
    frame_to_index: HashMap<StackFrame, usize>
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            samples: vec![],
            frames: vec![],
            frame_to_index: HashMap::new()
        }
    }

    pub fn record(&mut self, stack: &Vec<StackFrame>) -> Result<(), io::Error> {
        let frame_indices: Vec<usize> = stack.into_iter().map(|frame| {
            let frames = &mut self.frames;
            self.frame_to_index.entry(frame.clone()).or_insert_with(|| {
                let len = frames.len();
                frames.push(Frame::new(frame));
                len
            }).clone()
        }).collect();
        self.samples.push(frame_indices);
        Ok(())
    }

    pub fn write(&self, mut w: File) -> Result<(), Error> {
        let json = serde_json::to_string(&SpeedscopeFile::new(self.samples.clone(), self.frames.clone()))?;
        writeln!(&mut w, "{}", json)?;
        Ok(())
    }
}