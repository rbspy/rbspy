use std::collections::HashMap;
use std::io;

use core::types::StackFrame;

use serde_json;

/*
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

// Modifications to generated code
// - renamed Frame to Frame
// - made Frame derive Hash
// START OF GENERATED TYPED BINDINGS
#[derive(Debug, Serialize, Deserialize)]
pub struct SpeedscopeFile {
    #[serde(rename = "$schema")]
    schema: Schema,
    profiles: Vec<Profile>,
    shared: Shared,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Profile {
    #[serde(rename = "endValue")]
    end_value: f64,
    events: Option<Vec<Event>>,
    name: String,
    #[serde(rename = "startValue")]
    start_value: f64,
    #[serde(rename = "type")]
    profile_type: ProfileType,
    unit: ValueUnit,
    samples: Option<Vec<Vec<f64>>>,
    weights: Option<Vec<f64>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    at: f64,
    frame: f64,
    #[serde(rename = "type")]
    event_type: EventType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Shared {
    frames: Vec<FormatFrame>,
}

#[derive(Hash, Debug, Serialize, Deserialize)]
pub struct Frame {
    col: Option<f64>,
    file: Option<String>,
    line: Option<f64>,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EventType {
    C,
    O,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ProfileType {
    #[serde(rename = "evented")]
    Evented,
    #[serde(rename = "sampled")]
    Sampled,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ValueUnit {
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

#[derive(Debug, Serialize, Deserialize)]
pub enum Schema {
    #[serde(rename = "https://www.speedscope.app/file-format-schema.json")]
    HttpsWwwSpeedscopeAppSchemaJson,
}
// END OF GENERATED TYPED BINDINGS

impl SpeedscopeFile {
  pub fn new(profile: Sampled, frames: HashSet<Frame>) -> SpeedscopeFile {
    SpeedscopeFile {
      // This is always the same
      schema: HttpsWwwSpeedscopeAppSchemaJson,

      // This is the version of the file format we're targeting
      version: "0.2.0",

      profiles: vec![profile],

      shared: Shared {
        frames
      }
    }
  }
}

pub struct Stats {
  pub profile: Sampled
  pub frames: Vec<Frame>
  pub frameToIndex: HashMap<Frame, u32>
}

impl Stats {
  pub fn new() -> Stats {
    Stats {
      file: SpeedscopeFile
    }
  }

  pub fn record(&mut self, stack: &Vec<StackFrame>) -> Result<(), io::Error> {
  }

  pub fn write(&self, w: File) -> Result<(), Error> {
  }
}