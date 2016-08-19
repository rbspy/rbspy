#[macro_use] extern crate lazy_static;

extern crate env_logger;
extern crate elf;
extern crate flate2;
extern crate gimli;

extern crate ruby_stacktrace;

use std::fs::File;
use std::io::{Cursor, Read};

use flate2::read::GzDecoder;
use gimli::LittleEndian;

use ruby_stacktrace::dwarf::{DwarfLookup, Entry, get_all_entries, create_lookup_table};
use ruby_stacktrace::{DwarfTypes, get_types, get_stack_trace};
use ruby_stacktrace::test_utils::CoreDump;

pub static DEBUG_INFO: &'static [u8] = include_bytes!("../testdata/debug_info");
pub static DEBUG_ABBREV: &'static [u8] = include_bytes!("../testdata/debug_abbrev");
pub static DEBUG_STR: &'static [u8] = include_bytes!("../testdata/debug_str");

pub const RUBY_CURRENT_THREAD_ADDR: usize = 0x55f35c094040;

const COREDUMP_FILE: &'static str = concat!(env!("CARGO_MANIFEST_DIR"),
"/testdata/ruby-coredump.14341.gz");

lazy_static! {
    pub static ref COREDUMP: CoreDump = {
        let file = File::open(COREDUMP_FILE).unwrap();
        let mut buf = vec![];
        GzDecoder::new(file).unwrap().read_to_end(&mut buf).unwrap();

        CoreDump::from(elf::File::open_stream(&mut Cursor::new(buf)).unwrap())
    };

    static ref ENTRIES: Vec<Entry> = {
        get_all_entries::<LittleEndian>(DEBUG_INFO, DEBUG_ABBREV, DEBUG_STR)
    };

    static ref LOOKUP: DwarfLookup<'static> = {
        create_lookup_table(&ENTRIES)
    };

    static ref TYPES: DwarfTypes = {
        get_types(&LOOKUP)
    };
}


fn main() {
    let _ = env_logger::init();
    loop {
        let _ = get_stack_trace(RUBY_CURRENT_THREAD_ADDR as u64,
                                &*COREDUMP,
                                &LOOKUP,
                                &TYPES);
    }
}

