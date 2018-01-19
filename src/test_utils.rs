extern crate elf;
extern crate read_process_memory;

use std::io;

use libc;

use read_process_memory::CopyAddress;

// Data for use in tests and benchmarks :-)
#[cfg(test)]
pub mod data {
    extern crate elf;
    extern crate flate2;

    use std::fs::File;
    use std::io::{Cursor, Read};

    use self::flate2::read::GzDecoder;

    use super::*;

    const COREDUMP_FILE_2_4_0: &'static str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/ci/testdata/ruby-coredump.2.4.0.gz"
    );

    const COREDUMP_FILE_1_9_3: &'static str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/ci/testdata/ruby-coredump-1.9.3.gz"
    );

    const COREDUMP_FILE_2_5_0: &'static str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/ci/testdata/ruby-coredump-2.5.0.gz"
    );

    const COREDUMP_FILE_2_1_6: &'static str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/ci/testdata/ruby-coredump-2.1.6.gz"
    );

    lazy_static! {
        pub static ref COREDUMP_2_4_0: CoreDump = {
            let file = File::open(COREDUMP_FILE_2_4_0).unwrap();
            let mut buf = vec![];
            GzDecoder::new(file).unwrap().read_to_end(&mut buf).unwrap();

            CoreDump::from(elf::File::open_stream(&mut Cursor::new(buf)).unwrap())
        };
    }

    lazy_static! {
        pub static ref COREDUMP_1_9_3: CoreDump = {
            let file = File::open(COREDUMP_FILE_1_9_3).unwrap();
            let mut buf = vec![];
            GzDecoder::new(file).unwrap().read_to_end(&mut buf).unwrap();

            CoreDump::from(elf::File::open_stream(&mut Cursor::new(buf)).unwrap())
        };
    }

    lazy_static! {
        pub static ref COREDUMP_2_5_0: CoreDump = {
            let file = File::open(COREDUMP_FILE_2_5_0).unwrap();
            let mut buf = vec![];
            GzDecoder::new(file).unwrap().read_to_end(&mut buf).unwrap();

            CoreDump::from(elf::File::open_stream(&mut Cursor::new(buf)).unwrap())
        };
    }

    lazy_static! {
        pub static ref COREDUMP_2_1_6: CoreDump = {
            let file = File::open(COREDUMP_FILE_2_1_6).unwrap();
            let mut buf = vec![];
            GzDecoder::new(file).unwrap().read_to_end(&mut buf).unwrap();

            CoreDump::from(elf::File::open_stream(&mut Cursor::new(buf)).unwrap())
        };
    }
}

/// Allows testing offline with a core dump of a Ruby process.
pub struct CoreDump {
    file: elf::File,
}

impl From<elf::File> for CoreDump {
    fn from(file: elf::File) -> CoreDump {
        CoreDump { file: file }
    }
}

impl CopyAddress for CoreDump {
    fn copy_address(&self, addr: usize, buf: &mut [u8]) -> io::Result<()> {
        let start = addr as u64;
        let end = (addr + buf.len()) as u64;
        match self.file.sections.iter().find(|section| {
            section.shdr.addr <= start && end <= section.shdr.addr + section.shdr.size
        }) {
            Some(sec) => {
                let start = addr - sec.shdr.addr as usize;
                let end = addr + buf.len() - sec.shdr.addr as usize;
                buf.copy_from_slice(&sec.data[start..end]);
                Ok(())
            }
            None => Err(io::Error::from_raw_os_error(libc::EFAULT)),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate byteorder;

    use super::data::*;

    use ruby_version;

    use initialize::StackFrame;

    fn real_stack_trace_main() -> Vec<StackFrame> {
        vec![
            StackFrame {
                name: "aaa".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string()),
                lineno: 2,
            },
            StackFrame {
                name: "bbb".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string()),
                lineno: 6,
            },
            StackFrame {
                name: "ccc".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string()),
                lineno: 10,
            },
            StackFrame {
                name: "block in <main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string()),
                lineno: 14,
            },
            StackFrame {
                name: "<main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string()),
                lineno: 13,
            },
        ]
    }

    fn real_stack_trace() -> Vec<StackFrame> {
        vec![
            StackFrame {
                name: "aaa".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string()),
                lineno: 2,
            },
            StackFrame {
                name: "bbb".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string()),
                lineno: 6,
            },
            StackFrame {
                name: "ccc".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string()),
                lineno: 10,
            },
            StackFrame {
                name: "block in <main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string()),
                lineno: 14,
            },
        ]
    }

    #[test]
    fn test_get_ruby_stack_trace_2_1_6() {
        let current_thread_addr = 0x562658abd7f0;
        let stack_trace =
            ruby_version::ruby_2_1_6::get_stack_trace(current_thread_addr, &*COREDUMP_2_1_6)
                .unwrap();
        assert_eq!(real_stack_trace_main(), stack_trace);
    }
    #[test]
    fn test_get_ruby_stack_trace_1_9_3() {
        let current_thread_addr = 0x823930;
        let stack_trace =
            ruby_version::ruby_1_9_3_0::get_stack_trace(current_thread_addr, &*COREDUMP_1_9_3)
                .unwrap();
        assert_eq!(real_stack_trace_main(), stack_trace);
    }

    #[test]
    fn test_get_ruby_stack_trace_2_5_0() {
        let current_thread_addr = 0x55dd8c3b7758;
        let stack_trace =
            ruby_version::ruby_2_5_0_rc1::get_stack_trace(current_thread_addr, &*COREDUMP_2_5_0)
                .unwrap();
        assert_eq!(real_stack_trace(), stack_trace);
    }

    #[test]
    fn test_get_ruby_stack_trace_2_4_0() {
        let current_thread_addr = 0x55df44959920;
        let stack_trace =
            ruby_version::ruby_2_4_0::get_stack_trace(current_thread_addr, &*COREDUMP_2_4_0)
                .unwrap();
        assert_eq!(real_stack_trace(), stack_trace);
    }
}
