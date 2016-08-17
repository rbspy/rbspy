#![cfg(test)]

extern crate elf;

use std::io;

use libc;

use CopyAddress;


// Data for use in tests and benchmarks :-)
pub mod data {
    extern crate elf;
    extern crate flate2;

    use std::fs::File;
    use std::io::{Cursor, Read};

    use self::flate2::read::GzDecoder;

    use super::*;

    pub static DEBUG_INFO: &'static [u8] = include_bytes!("../testdata/debug_info");
    pub static DEBUG_ABBREV: &'static [u8] = include_bytes!("../testdata/debug_abbrev");
    pub static DEBUG_STR: &'static [u8] = include_bytes!("../testdata/debug_str");

    const COREDUMP_FILE: &'static str = concat!(env!("CARGO_MANIFEST_DIR"),
                                                "/testdata/ruby-coredump.14341.gz");

    lazy_static! {
        pub static ref COREDUMP: CoreDump = {
            let file = File::open(COREDUMP_FILE).unwrap();
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

    use std::mem;

    use byteorder::{ReadBytesExt, LittleEndian};

    use CopyAddress;

    use super::data::COREDUMP;

    // Values are correct for the ruby-coredump.14341.gz file.
    const RUBY_CURRENT_THREAD_ADDR: usize = 0x55f35c094040;
    const RUBY_CURRENT_THREAD_VAL: usize = 0x55f35cb765c0;

    #[test]
    fn test_get_ruby_current_thread() {

        let mut buf = vec![0u8; mem::size_of::<usize>()];
        COREDUMP.copy_address(RUBY_CURRENT_THREAD_ADDR, &mut buf).unwrap();
        assert_eq!(RUBY_CURRENT_THREAD_VAL,
                   buf.as_slice().read_u64::<LittleEndian>().unwrap() as usize);
    }
}
