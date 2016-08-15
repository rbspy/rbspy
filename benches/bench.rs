#![feature(test)]

extern crate test;
extern crate ruby_stacktrace;
extern crate gimli;

use test::Bencher;
use gimli::LittleEndian;

use ruby_stacktrace::dwarf::{get_all_entries, create_lookup_table};

static DEBUG_INFO: &'static [u8] = include_bytes!("../testdata/debug_info");
static DEBUG_ABBREV: &'static [u8] = include_bytes!("../testdata/debug_abbrev");
static DEBUG_STR: &'static [u8] = include_bytes!("../testdata/debug_str");

// At 96b6d9d:
// test bench_create_lookup ... bench: 104,556,803 ns/iter (+/- 2,776,767)
#[bench]
fn bench_create_lookup(b: &mut Bencher) {
    let entries = get_all_entries::<LittleEndian>(DEBUG_INFO, DEBUG_ABBREV, DEBUG_STR);

    b.iter(|| {
        let _lookup = test::black_box(create_lookup_table(&entries));
    });
}
