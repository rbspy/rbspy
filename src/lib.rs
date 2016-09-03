#![cfg_attr(rustc_nightly, feature(test))]
#[macro_use] extern crate log;

#[cfg(test)]
#[macro_use] extern crate lazy_static;

extern crate libc;
extern crate regex;
extern crate term;
extern crate gimli;
extern crate fnv;
extern crate rand;
extern crate leb128;

extern crate clap;
extern crate byteorder;

pub mod dwarf;

use libc::*;
use std::process;
use std::os::unix::prelude::*;
use std::ffi::{OsString, CStr};
use std::process::Command;
use std::process::Stdio;
use regex::Regex;
use byteorder::{NativeEndian, ReadBytesExt};
use std::io::{self, Cursor};
use std::collections::HashMap;

use dwarf::{DwarfLookup, Entry};

pub mod test_utils;

pub trait CopyAddress {
    fn copy_address(&self, addr: usize, buf: &mut [u8]) -> io::Result<()>;
}

pub struct Process {
    pid: pid_t,
}

impl Process {
    pub fn new(pid: pid_t) -> Process {
        Process {
            pid: pid,
        }
    }
}

#[cfg(target_os="linux")]
mod platform {
    use std::io;
    use std::process;

    use libc::{c_void, iovec, process_vm_readv};

    use super::{CopyAddress, Process};

    static mut READ_EVER_SUCCEEDED: bool = false;

    impl CopyAddress for Process {
        fn copy_address(&self, addr: usize, buf: &mut [u8]) -> io::Result<()> {
            let local_iov = iovec {
                iov_base: buf.as_mut_ptr() as *mut c_void,
                iov_len: buf.len(),
            };
            let remote_iov = iovec {
                iov_base: addr as *mut c_void,
                iov_len: buf.len(),
            };
            unsafe {
                let result = process_vm_readv(self.pid, &local_iov, 1, &remote_iov, 1, 0);
                if result == -1 {
                    if !READ_EVER_SUCCEEDED {
                        println!("Failed to read from pid {}. Are you root?", self.pid);
                        process::exit(1);
                    } else {
                        return Err(io::Error::last_os_error());
                    }
                }
                READ_EVER_SUCCEEDED = true;
            }
            Ok(())
        }
    }
}

#[cfg(target_os="macos")]
mod platform {
    extern crate libc;
    extern crate mach;

    use self::mach::kern_return::{kern_return_t, KERN_SUCCESS};
    use self::mach::port::{mach_port_t, mach_port_name_t, MACH_PORT_NULL};
    use self::mach::vm_types::{mach_vm_address_t, mach_vm_size_t};
    use self::mach::message::{mach_msg_type_number_t};
    use std::io;

    use super::{CopyAddress, Process};

    #[allow(non_camel_case_types)]
    type vm_map_t = mach_port_t;

    extern "C" {
        fn vm_read(target_task: vm_map_t, address: mach_vm_address_t, size: mach_vm_size_t, data: *mut u8, data_size: *mut mach_msg_type_number_t) -> kern_return_t;
    }

    fn task_for_pid(addr: libc::c_int) -> io::Result<mach_port_name_t> {
        let mut task: mach_port_name_t = MACH_PORT_NULL;

        unsafe {
            let result = mach::traps::task_for_pid(mach::traps::mach_task_self(), addr as libc::c_int, &mut task);
            if result != KERN_SUCCESS {
                return Err(io::Error::last_os_error())
            }
        }

        Ok(task)
    }

    impl CopyAddress for Process {
        fn copy_address(&self, addr: usize, buf: &mut [u8]) -> io::Result<()> {
            let task = task_for_pid(self.pid);
            if task.is_err() { return task.map(|_| ()) }

            unsafe {
                let mut read: mach_msg_type_number_t = 0;

                let result = vm_read(task.unwrap(), addr as u64, buf.len() as u64, buf.as_mut_ptr(), &mut read);
                if result != KERN_SUCCESS {
                    return Err(io::Error::last_os_error())
                }
            }

            Ok(())
        }
    }
}

pub fn copy_address_raw<T>(addr: *const c_void, length: usize, source: &T) -> Vec<u8>
    where T: CopyAddress
{
    if length > 100000 {
        // something very unusual has happened.
        // Do not respect requests for huge amounts of memory.
        return Vec::new();
    }
    let mut copy = vec![0; length];

    if source.copy_address(addr as usize, &mut copy).is_err() {
        warn!("copy_address failed for {:p}", addr);
    }

    copy
}

// These three functions (get_cfps, get_iseq, and get_ruby_string) are the
// core of how the program works. They're essentially a straight port of
// this gdb script:
// https://gist.github.com/csfrancis/11376304/raw/7a0450d11e64e3bb7c982b7ad2778f3603188c0f/gdb_ruby_backtrace.yp
// except without using gdb!!
//
// `get_iseq` is the simplest method  here -- it's just trying to run (cfp->iseq). But to do that
// you need to dereference the `cfp` pointer, and that memory is actually in another process
// so we call `copy_address` to copy the memory for that pointer out of
// the other process. The other methods do the same thing
// except that they're more copmlicated and sometimes call `copy_address_raw`.
//
// `get_cfps` corresponds to
// (* const rb_thread t *(ruby_current_thread_address_location))->cfp
//
// `get_ruby_string` is doing ((Struct RString *) address) and then
// trying one of two ways to get the actual Ruby string out depending
// on how it's stored
//

fn get_nm_address(pid: pid_t) -> u64 {
    let nm_command = Command::new("nm")
        .arg(format!("/proc/{}/exe", pid))
        .stdout(Stdio::piped())
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {}", e));
    let nm_output = String::from_utf8(nm_command.stdout).unwrap();
    let re = Regex::new(r"(\w+) b ruby_current_thread").unwrap();
    let cap = re.captures(&nm_output).unwrap_or_else(|| {
        println!("Error: Couldn't find current thread in Ruby process. This is probably because \
                  either this isn't a Ruby process or you have a Ruby version compiled with no \
                  symbols.");
        process::exit(1)
    });
    let address_str = cap.at(1).unwrap();
    let addr = u64::from_str_radix(address_str, 16).unwrap();
    debug!("get_nm_address: {:x}", addr);
    addr
}

fn get_maps_address(pid: pid_t) -> u64 {
    let cat_command = Command::new("cat")
        .arg(format!("/proc/{}/maps", pid))
        .stdout(Stdio::piped())
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {}", e));
    let output = String::from_utf8(cat_command.stdout).unwrap();
    let re = Regex::new(r"(\w+).+xp.+?bin/ruby").unwrap();
    let cap = re.captures(&output).unwrap();
    let address_str = cap.at(1).unwrap();
    let addr = u64::from_str_radix(address_str, 16).unwrap();
    debug!("get_maps_address: {:x}", addr);
    addr
}

pub fn get_ruby_current_thread_address(pid: pid_t) -> u64 {
    // Get the address of the `ruby_current_thread` global variable. It works
    // by looking up the address in the Ruby binary's symbol table with `nm
    // /proc/$pid/exe` and then finding out which address the Ruby binary is
    // mapped to by looking at `/proc/$pid/maps`. If we add these two
    // addresses together we get our answers! All this is Linux-specific but
    // this program only works on Linux anyway because of process_vm_readv.
    //
    let addr = get_nm_address(pid) + get_maps_address(pid);
    debug!("get_ruby_current_thread_address: {:x}", addr);
    addr
}

pub fn print_method_stats(method_stats: &HashMap<String, u32>,
                      method_own_time_stats: &HashMap<String, u32>,
                      n_terminal_lines: usize) {
    println!("[{}c", 27 as char); // clear the screen
    let mut count_vec: Vec<_> = method_own_time_stats.iter().collect();
    count_vec.sort_by(|a, b| b.1.cmp(a.1));
    println!(" {:4} | {:4} | {}", "self", "tot", "method");
    let self_sum: u32 = method_own_time_stats.values().fold(0, std::ops::Add::add);
    let total_sum: u32 = *method_stats.values().max().unwrap();
    for &(method, count) in count_vec.iter().take(n_terminal_lines - 1) {
        let total_count = method_stats.get(&method[..]).unwrap();
        println!(" {:02.1}% | {:02.1}% | {}",
                 100.0 * (*count as f32) / (self_sum as f32),
                 100.0 * (*total_count as f32) / (total_sum as f32),
                 method);
    }
}

pub fn print_stack_trace(trace: &[String]) {
    for x in trace {
        println!("{}", x);
    }
    println!("{}", 1);
}

// Some field types are typedef DIEs, which have no size information. This traverses through
// typedefs in an attempt to find the type DIE of the actual type, which does have a size.
fn get_size(lookup_table: &DwarfLookup, entry: &Entry) -> Option<usize> {
    let mut current_entry: &Entry = entry;
    while current_entry.byte_size == None {
        match lookup_table.lookup_entry(current_entry) {
            None => return None,
            Some(entry) => {
                current_entry = entry;
            }
        }
    }
    return current_entry.byte_size
}

unsafe fn copy_address_dynamic<'a, T>(
        addr: *const c_void, lookup_table: &DwarfLookup,
        source: &T, dwarf_type: &Entry) -> HashMap<String, Vec<u8>>
    where T: CopyAddress
{
    trace!("copy_address_dynamic {:p} {:?}", addr, dwarf_type.name);
    let size = dwarf_type.byte_size.unwrap() + 200; // todo: is a hack
    let bytes = copy_address_raw(addr as *mut c_void, size, source);
    let ret = map_bytes_to_struct(&bytes, lookup_table, dwarf_type);

    trace!("copy_address_dynamic return: {:?}", ret);
    ret
}

fn map_bytes_to_struct<'a>(
        bytes: &[u8],
        lookup_table: &DwarfLookup,
        dwarf_type: &Entry) -> HashMap<String, Vec<u8>> {
    // println!("{:#?}", dwarf_type);
    let mut struct_map = HashMap::new();
    for entry in dwarf_type.children.iter() {
        match get_size(&lookup_table, entry) {
            None => break,
            Some(size) => {
                let name = entry.name.clone().unwrap_or("unknownnnn".to_string());
                let offset = entry.offset.unwrap() as usize;
                let b = bytes[offset..offset + size].to_vec();
                struct_map.insert(name, b);
            }
        }
    }
    struct_map

}

fn read_pointer_address(vec: &[u8]) -> u64 {
    let mut rdr = Cursor::new(vec);
    rdr.read_u64::<NativeEndian>().unwrap()
}

fn get_child<'a>(entry: &'a Entry, name: &'a str) -> Option<&'a Entry>{
    for child in &entry.children {
        if child.name == Some(name.to_string()) {
            return Some(child);
        }
    }
    None
}

fn get_ruby_string2<T>(addr: u64, source: &T, lookup_table: &DwarfLookup, types: &DwarfTypes) -> OsString
    where T: CopyAddress
{
     let vec = unsafe {
        let rstring = copy_address_dynamic(addr as *const c_void, lookup_table, source, &types.rstring);
        let basic =  map_bytes_to_struct(&rstring["basic"], lookup_table, &types.rbasic);
        let is_array = (read_pointer_address(&basic["flags"]) & 1 << 13) == 0;
        if is_array  {
           // println!("it's an array!!!!");
           // println!("rstring {:#?}", rstring);
           CStr::from_ptr(rstring["as"].as_ptr() as *const i8).to_bytes().to_vec()
        } else {
            let entry = &types.rstring;
                // println!("entry: {:?}", entry);
            let as_type = get_child(&entry, "as").unwrap();
            let blah = lookup_table.lookup_entry(as_type).unwrap();
            // println!("blah: {:?}", blah);
            let heap_type = get_child(&blah, "heap").unwrap();
            let blah2 = lookup_table.lookup_entry(heap_type).unwrap();
            let hashmap = map_bytes_to_struct(&rstring["as"], lookup_table, blah2);
            copy_address_raw(
                read_pointer_address(&hashmap["ptr"]) as *const c_void,
                read_pointer_address(&hashmap["len"]) as usize,
                source)
        }
    };
    OsString::from_vec(vec)
}

pub struct DwarfTypes {
    rbasic: Entry,
    rstring: Entry,
    rb_thread_struct: Entry,
    rb_iseq_constant_body: Option<Entry>,
    rb_iseq_location_struct: Entry,
    rb_iseq_struct: Entry,
    rb_control_frame_struct: Entry,
}

pub fn get_types(lookup_table: &DwarfLookup) -> DwarfTypes {
    DwarfTypes {
        rbasic: lookup_table.lookup_thing("RBasic").unwrap().clone(),
        rstring: lookup_table.lookup_thing("RString").unwrap().clone(),
        rb_thread_struct: lookup_table.lookup_thing("rb_thread_struct").unwrap().clone(),
        rb_iseq_constant_body: lookup_table.lookup_thing("rb_iseq_constant_body").map(|x| x.clone()),
        rb_iseq_location_struct: lookup_table.lookup_thing("rb_iseq_location_struct").unwrap().clone(),
        rb_iseq_struct: lookup_table.lookup_thing("rb_iseq_struct").unwrap().clone(),
        rb_control_frame_struct: lookup_table.lookup_thing("rb_control_frame_struct").unwrap().clone(),
    }
}

unsafe fn get_label_and_path<T>(cfp_bytes: &[u8], source: &T, lookup_table: &DwarfLookup, types: &DwarfTypes) -> Option<(OsString, OsString)>
    where T: CopyAddress
{
    trace!("get_label_and_path {:?}", cfp_bytes);
    let blah2 = map_bytes_to_struct(&cfp_bytes, &lookup_table, &types.rb_control_frame_struct);
    // println!("{:?}", blah2);
    let iseq_address = read_pointer_address(&blah2["iseq"]);
    let blah3 = copy_address_dynamic(iseq_address as *const c_void, &lookup_table, source, &types.rb_iseq_struct);
    let location = if blah3.contains_key("location") {
        map_bytes_to_struct(&blah3["location"], &lookup_table, &types.rb_iseq_location_struct)
    } else if blah3.contains_key("body") {
        let body = read_pointer_address(&blah3["body"]);
        match types.rb_iseq_constant_body {
            Some(ref constant_body_type) => {
                let blah4 = copy_address_dynamic(body as *const c_void, &lookup_table, source, &constant_body_type);
                map_bytes_to_struct(&blah4["location"], &lookup_table, &types.rb_iseq_location_struct)
            }
            None => panic!("rb_iseq_constant_body shouldn't be missing"),
        }
    } else {
        panic!("DON'T KNOW WHERE LOCATION IS");
    };
    let label_address = read_pointer_address(&location["label"]);
    let path_address = read_pointer_address(&location["path"]);
    let label = get_ruby_string2(label_address, source, lookup_table, types);
    let path = get_ruby_string2(path_address, source, lookup_table, types);
    if path.to_string_lossy() == "" {
        trace!("get_label_and_path ret None");
        return None;
    }
    // println!("label_address: {}, path_address: {}", label_address, path_address);
    // println!("location hash: {:#?}", location);
    let ret = Some((label, path));

    trace!("get_label_and_path ret {:?}", ret);
    ret
}

// Ruby stack grows down, starting at
//   ruby_current_thread->stack + ruby_current_thread->stack_size - 1 * sizeof(rb_control_frame_t)
// I don't know what the -1 is about. Also note that the stack_size is *not* in bytes! stack is a
// VALUE*, and so stack_size is in units of sizeof(VALUE).
//
// The base of the call stack is therefore at
//   stack + stack_size * sizeof(VALUE) - sizeof(rb_control_frame_t)
// (with everything in bytes).
unsafe fn get_cfps<T>(ruby_current_thread_address_location: u64, source: &T, lookup_table: &DwarfLookup, types: &DwarfTypes) -> (Vec<u8>, usize)
    where T: CopyAddress
{
    let ruby_current_thread_address: u64 = read_pointer_address(&copy_address_raw(ruby_current_thread_address_location as *const c_void, 8, source));
    let blah = copy_address_dynamic(ruby_current_thread_address as *const c_void, &lookup_table, source, &types.rb_thread_struct);
    // println!("{:?}", blah);
    let cfp_address = read_pointer_address(&blah["cfp"]);
    let ref cfp_struct = types.rb_control_frame_struct;
    let cfp_size = cfp_struct.byte_size.unwrap();

    let stack = read_pointer_address(&blah["stack"]) as usize;
    let stack_size = read_pointer_address(&blah["stack_size"]) as usize;
    let value_size = get_size(lookup_table, lookup_table.lookup_thing("VALUE").unwrap()).unwrap();

    let stack_base = stack + (stack_size) * value_size;
    let stack_base = stack_base - 1 * cfp_size;

    let ret = (copy_address_raw(cfp_address as *const c_void, stack_base - cfp_address as usize, source), cfp_size);

    trace!("get_cfps ret ([{} bytes], {})", ret.0.len(), ret.1);
    ret
}

pub fn get_stack_trace<T>(ruby_current_thread_address_location: u64, source: &T, lookup_table: &DwarfLookup, types: &DwarfTypes) -> Vec<String>
    where T: CopyAddress
{
    unsafe {

    let (cfp_bytes, cfp_size) = get_cfps(ruby_current_thread_address_location, source, lookup_table, types);
    let mut trace: Vec<String> = Vec::new();
    for i in 0..cfp_bytes.len() / cfp_size {
        match get_label_and_path(&cfp_bytes[(cfp_size*i)..cfp_size * (i+1)].to_vec(), source, lookup_table, types) {
            None => continue,
            Some((label, path)) => {
                let current_location =
                    format!("{} : {}", label.to_string_lossy(), path.to_string_lossy()).to_string();
                trace.push(current_location);
            }
        }
    }
    trace
    }
}


#[cfg(test)]
mod tests {
    extern crate env_logger;
    use gimli::LittleEndian;

    use test_utils::data::{COREDUMP, DEBUG_INFO, DEBUG_ABBREV,
                           DEBUG_STR, RUBY_CURRENT_THREAD_ADDR};
    use dwarf::{DwarfLookup, Entry, get_all_entries, create_lookup_table};

    use super::{DwarfTypes, get_types, get_stack_trace};

    lazy_static! {
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

    #[test]
    fn test_get_types() {
        let _ = env_logger::init();

        let types = get_types(&LOOKUP);

        assert_eq!(types.rbasic.name.unwrap(), "RBasic");
        assert_eq!(types.rstring.name.unwrap(), "RString");
        assert_eq!(types.rb_thread_struct.name.unwrap(), "rb_thread_struct");
        assert_eq!(types.rb_iseq_constant_body.unwrap().name.unwrap(),
                   "rb_iseq_constant_body");
        assert_eq!(types.rb_iseq_location_struct.name.unwrap(), "rb_iseq_location_struct");
        assert_eq!(types.rb_iseq_struct.name.unwrap(), "rb_iseq_struct");
        assert_eq!(types.rb_control_frame_struct.name.unwrap(), "rb_control_frame_struct");
    }

    #[test]
    fn test_get_stack_trace() {
        let _ = env_logger::init();

        let stack_trace = get_stack_trace(RUBY_CURRENT_THREAD_ADDR as u64,
                                          &*COREDUMP,
                                          &LOOKUP,
                                          &TYPES);
        assert_eq!(stack_trace.len(), 6);
        assert!(stack_trace[0].starts_with("aaaaaaaaa"));
    }

    #[cfg(rustc_nightly)]
    mod benches {
        extern crate test;

        use test_utils::data::{COREDUMP, RUBY_CURRENT_THREAD_ADDR};
        use super::{LOOKUP, TYPES};

        use get_stack_trace;

        use self::test::Bencher;

        // At 5defba5:
        // test tests::benches::bench_get_stack_trace ... bench:      86,612 ns/iter (+/- 17,621)

        #[bench]
        fn bench_get_stack_trace(b: &mut Bencher) {
            b.iter(|| {
                let _ = get_stack_trace(RUBY_CURRENT_THREAD_ADDR as u64,
                                        &*COREDUMP,
                                        &LOOKUP,
                                        &TYPES);
            });
        }
    }
}
