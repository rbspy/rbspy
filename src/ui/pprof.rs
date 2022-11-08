use flate2::{write::GzEncoder, Compression};
use std::collections::HashMap;
use std::io::prelude::*;
use std::time::SystemTime;

use crate::core::types::{StackFrame, StackTrace};

use anyhow::Result;

use prost::Message; // for encode and decode methods below

pub mod pprofs {
    include!("perftools.profiles.rs");
}
use self::pprofs::{Function, Label, Line, Location, Profile, Sample, ValueType};

#[derive(Default)]
pub struct Stats {
    profile: Profile,
    known_frames: HashMap<StackFrame, u64>,
    prev_time: Option<SystemTime>,
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            profile: Profile {
                // string index 0 must point to "" according to the .proto spec, while "wall" and "nanoseconds" are for our sample_type field
                string_table: vec![
                    "".to_string(),
                    "wall".to_string(),
                    "nanoseconds".to_string(),
                ],
                sample_type: vec![ValueType { r#type: 1, unit: 2 }], // 1 and 2 are indexes from string_table
                ..Profile::default()
            },
            ..Stats::default()
        }
    }

    pub fn record(&mut self, stack: &StackTrace) -> Result<()> {
        let this_time = stack.time.unwrap_or_else(SystemTime::now);
        let ns_since_last_sample = match self.prev_time {
            Some(prev_time) => match this_time.duration_since(prev_time) {
                Ok(duration) => duration.as_nanos(),
                Err(e) => {
                    // It's possible that samples will arrive out of order, e.g. if we're sampling
                    // from multiple processes.
                    warn!("sample arrived out of order: {}", e);
                    0
                }
            },
            None => 0,
        } as i64;
        self.add_sample(stack, ns_since_last_sample);
        self.prev_time = Some(this_time);
        Ok(())
    }

    fn add_sample(&mut self, stack: &StackTrace, sample_time: i64) {
        let s = Sample {
            location_id: self.location_ids(stack),
            value: vec![sample_time],
            label: self.labels(stack),
        };
        self.profile.sample.push(s);
    }

    fn location_ids(&mut self, stack: &StackTrace) -> Vec<u64> {
        let mut ids = <Vec<u64>>::new();

        for frame in &stack.trace {
            ids.push(self.get_or_create_location_id(frame));
        }
        ids
    }

    fn get_or_create_location_id(&mut self, frame: &StackFrame) -> u64 {
        // our lookup table has the arbitrary ids (1..n) we use for location ids
        if let Some(id) = self.known_frames.get(frame) {
            *id
        } else {
            let next_id = self.known_frames.len() as u64 + 1; //ids must be non-0, so start at 1
            self.known_frames.insert(frame.clone(), next_id); // add to our lookup table
            let newloc = self.new_location(next_id, frame); // use the same id for the location table
            self.profile.location.push(newloc);
            next_id
        }
    }

    fn new_location(&mut self, id: u64, frame: &StackFrame) -> Location {
        let new_line = Line {
            function_id: self.get_or_create_function_id(frame),
            line: frame.lineno as i64,
        };
        Location {
            id,
            line: vec![new_line],
            ..Location::default()
        }
    }

    fn get_or_create_function_id(&mut self, frame: &StackFrame) -> u64 {
        let strings = &self.profile.string_table;
        let mut functions = self.profile.function.iter();
        if let Some(function) = functions.find(|f| {
            frame.name == strings[f.name as usize]
                && frame.relative_path == strings[f.filename as usize]
        }) {
            function.id
        } else {
            let functions = self.profile.function.iter();
            let mapped_iter = functions.map(|f| f.id);
            let max_map = mapped_iter.max();
            let next_id = match max_map {
                Some(id) => id + 1,
                None => 1,
            };
            let f = self.new_function(next_id, frame);
            self.profile.function.push(f);
            next_id
        }
    }

    fn new_function(&mut self, id: u64, frame: &StackFrame) -> Function {
        Function {
            id,
            name: self.string_id(&frame.name),
            filename: self.string_id(&frame.relative_path),
            ..Function::default()
        }
    }

    fn string_id(&mut self, text: &str) -> i64 {
        let strings = &mut self.profile.string_table;
        if let Some(id) = strings.iter().position(|s| *s == *text) {
            id as i64
        } else {
            let next_id = strings.len() as i64;
            strings.push((*text).to_owned());
            next_id
        }
    }

    fn labels(&mut self, stack: &StackTrace) -> Vec<Label> {
        let mut labels: Vec<Label> = Vec::new();
        if let Some(pid) = stack.pid {
            labels.push(Label {
                key: self.string_id(&"pid".to_string()),
                num: pid as i64,
                ..Label::default()
            });
        }
        if let Some(thread_id) = stack.thread_id {
            labels.push(Label {
                key: self.string_id(&"thread_id".to_string()),
                num: thread_id as i64,
                ..Label::default()
            });
        }
        labels
    }

    pub fn write(&mut self, w: &mut dyn Write) -> Result<()> {
        let mut pprof_data = Vec::new();
        let mut gzip = GzEncoder::new(Vec::new(), Compression::default());

        self.profile.encode(&mut pprof_data)?;
        gzip.write_all(&pprof_data)?;
        w.write_all(&gzip.finish()?)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::ui::pprof::*;
    use flate2::read::GzDecoder;
    use std::time::Duration;

    // Build a test stacktrace
    fn s(frames: Vec<StackFrame>, time: SystemTime) -> StackTrace {
        StackTrace {
            trace: frames,
            pid: Some(9),
            thread_id: Some(999),
            time: Some(time),
        }
    }

    // Build a test stackframe
    fn f(i: u32) -> StackFrame {
        StackFrame {
            name: format!("func{}", i),
            relative_path: format!("file{}.rb", i),
            absolute_path: None,
            lineno: i,
        }
    }

    // A stack frame from the same file as another one
    fn fdup() -> StackFrame {
        StackFrame {
            name: "funcX".to_owned(),
            relative_path: "file1.rb".to_owned(),
            absolute_path: None,
            lineno: 42,
        }
    }

    fn test_stats() -> Stats {
        let mut stats = Stats::new();
        let mut time = SystemTime::now();
        stats.record(&s(vec![f(1)], time)).unwrap();
        time += Duration::new(0, 200);
        stats.record(&s(vec![f(3), f(2), f(1)], time)).unwrap();
        time += Duration::new(0, 400);
        stats.record(&s(vec![f(2), f(1)], time)).unwrap();
        time += Duration::new(0, 600);
        stats.record(&s(vec![f(3), f(1)], time)).unwrap();
        time += Duration::new(0, 800);
        stats.record(&s(vec![f(2), f(1)], time)).unwrap();
        time += Duration::new(0, 1000);
        stats.record(&s(vec![f(3), fdup(), f(1)], time)).unwrap();

        stats
    }

    #[test]
    fn tolerate_stacktrace_timestamps_arriving_out_of_order() {
        let mut stats = Stats::new();
        let mut time = SystemTime::now();
        stats.record(&s(vec![f(1)], time)).unwrap();
        time -= Duration::new(0, 200);
        stats.record(&s(vec![f(3), f(2), f(1)], time)).unwrap();
    }

    #[test]
    fn can_collect_traces_and_write_to_pprof_format() {
        let mut gz_stats_buf: Vec<u8> = Vec::new();
        let mut stats = test_stats();
        stats.write(&mut gz_stats_buf).expect("write failed");

        let mut gz = GzDecoder::new(&*gz_stats_buf);
        let mut stats_buf = Vec::new();
        gz.read_to_end(&mut stats_buf).unwrap();

        let actual = pprofs::Profile::decode(&*stats_buf).expect("decode failed");
        let expected = Profile {
            sample_type: vec![ValueType { r#type: 1, unit: 2 }],
            sample: vec![
                Sample {
                    location_id: vec![1],
                    value: vec![0],
                    label: vec![
                        Label {
                            key: 5,
                            str: 0,
                            num: 9,
                            num_unit: 0,
                        },
                        Label {
                            key: 6,
                            str: 0,
                            num: 999,
                            num_unit: 0,
                        },
                    ],
                },
                Sample {
                    location_id: vec![2, 3, 1],
                    value: vec![200],
                    label: vec![
                        Label {
                            key: 5,
                            str: 0,
                            num: 9,
                            num_unit: 0,
                        },
                        Label {
                            key: 6,
                            str: 0,
                            num: 999,
                            num_unit: 0,
                        },
                    ],
                },
                Sample {
                    location_id: vec![3, 1],
                    value: vec![400],
                    label: vec![
                        Label {
                            key: 5,
                            str: 0,
                            num: 9,
                            num_unit: 0,
                        },
                        Label {
                            key: 6,
                            str: 0,
                            num: 999,
                            num_unit: 0,
                        },
                    ],
                },
                Sample {
                    location_id: vec![2, 1],
                    value: vec![600],
                    label: vec![
                        Label {
                            key: 5,
                            str: 0,
                            num: 9,
                            num_unit: 0,
                        },
                        Label {
                            key: 6,
                            str: 0,
                            num: 999,
                            num_unit: 0,
                        },
                    ],
                },
                Sample {
                    location_id: vec![3, 1],
                    value: vec![800],
                    label: vec![
                        Label {
                            key: 5,
                            str: 0,
                            num: 9,
                            num_unit: 0,
                        },
                        Label {
                            key: 6,
                            str: 0,
                            num: 999,
                            num_unit: 0,
                        },
                    ],
                },
                Sample {
                    location_id: vec![2, 4, 1],
                    value: vec![1000],
                    label: vec![
                        Label {
                            key: 5,
                            str: 0,
                            num: 9,
                            num_unit: 0,
                        },
                        Label {
                            key: 6,
                            str: 0,
                            num: 999,
                            num_unit: 0,
                        },
                    ],
                },
            ],
            mapping: vec![],
            location: vec![
                Location {
                    id: 1,
                    mapping_id: 0,
                    address: 0,
                    line: vec![Line {
                        function_id: 1,
                        line: 1,
                    }],
                    is_folded: false,
                },
                Location {
                    id: 2,
                    mapping_id: 0,
                    address: 0,
                    line: vec![Line {
                        function_id: 2,
                        line: 3,
                    }],
                    is_folded: false,
                },
                Location {
                    id: 3,
                    mapping_id: 0,
                    address: 0,
                    line: vec![Line {
                        function_id: 3,
                        line: 2,
                    }],
                    is_folded: false,
                },
                Location {
                    id: 4,
                    mapping_id: 0,
                    address: 0,
                    line: vec![Line {
                        function_id: 4,
                        line: 42,
                    }],
                    is_folded: false,
                },
            ],
            function: vec![
                Function {
                    id: 1,
                    name: 3,
                    system_name: 0,
                    filename: 4,
                    start_line: 0,
                },
                Function {
                    id: 2,
                    name: 7,
                    system_name: 0,
                    filename: 8,
                    start_line: 0,
                },
                Function {
                    id: 3,
                    name: 9,
                    system_name: 0,
                    filename: 10,
                    start_line: 0,
                },
                Function {
                    id: 4,
                    name: 11,
                    system_name: 0,
                    filename: 4,
                    start_line: 0,
                },
            ],
            string_table: vec![
                "".to_string(),
                "wall".to_string(),
                "nanoseconds".to_string(),
                "func1".to_string(),
                "file1.rb".to_string(),
                "pid".to_string(),
                "thread_id".to_string(),
                "func3".to_string(),
                "file3.rb".to_string(),
                "func2".to_string(),
                "file2.rb".to_string(),
                "funcX".to_string(),
            ],
            drop_frames: 0,
            keep_frames: 0,
            time_nanos: 0,
            duration_nanos: 0,
            period_type: None,
            period: 0,
            comment: vec![],
            default_sample_type: 0,
        };
        assert_eq!(actual, expected, "stats don't match");
    }
}
