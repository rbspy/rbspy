extern crate libc;
extern crate regex;
extern crate term;
extern crate gimli;

extern crate clap;
extern crate byteorder;
use clap::{Arg, App, ArgMatches};
use libc::*;
use std::process;
use std::os::unix::prelude::*;
use std::time::Duration;
use std::thread;
use std::ffi::{OsString, CStr};
use std::process::Command;
use std::process::Stdio;
use regex::Regex;
pub mod dwarf;
use byteorder::{NativeEndian, ReadBytesExt};
use std::io::Cursor;
use dwarf::{create_lookup_table, get_dwarf_entries, DwarfLookup, Entry};
use std::collections::HashMap;

static mut READ_EVER_SUCCEEDED: bool = false;

fn copy_address_raw(addr: *const c_void, length: usize, pid: pid_t) -> Vec<u8> {
    if length > 100000 {
        // something very unusual has happened.
        // Do not respect requests for huge amounts of memory.
        return Vec::new();
    }
    let mut copy: Vec<u8> = unsafe {
        let mut vec = Vec::with_capacity(length);
        vec.set_len(length);
        vec
    };
    let local_iov = iovec {
        iov_base: copy.as_mut_ptr() as *mut c_void,
        iov_len: length,
    };
    let remote_iov = iovec {
        iov_base: addr as *mut c_void,
        iov_len: length,
    };
    unsafe {
        let result = process_vm_readv(pid, &local_iov, 1, &remote_iov, 1, 0);
        if result == -1 && !READ_EVER_SUCCEEDED {
            println!("Failed to read from pid {}. Are you root?", pid);
            process::exit(1);
        }
        READ_EVER_SUCCEEDED = true;
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
    u64::from_str_radix(address_str, 16).unwrap()
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
    u64::from_str_radix(address_str, 16).unwrap()
}

fn get_ruby_current_thread_address(pid: pid_t) -> u64 {
    // Get the address of the `ruby_current_thread` global variable. It works
    // by looking up the address in the Ruby binary's symbol table with `nm
    // /proc/$pid/exe` and then finding out which address the Ruby binary is
    // mapped to by looking at `/proc/$pid/maps`. If we add these two
    // addresses together we get our answers! All this is Linux-specific but
    // this program only works on Linux anyway because of process_vm_readv.
    //
    get_nm_address(pid) + get_maps_address(pid)
}

fn print_method_stats(method_stats: &HashMap<String, u32>,
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

fn print_stack_trace(trace: &Vec<String>) {
    for x in trace {
        println!("{}", x);
    }
    println!("{}", 1);
}

fn parse_args() -> ArgMatches<'static> {
    App::new("ruby-stacktrace")
        .version("0.1")
        .about("Sampling profiler for Ruby programs")
        .arg(Arg::with_name("COMMAND")
            .help("Subcommand you want to run. Options: top, stackcollapse.\n          top \
                   prints a top-like output of what the Ruby process is doing right now\n          \
                   stackcollapse prints out output suitable for piping to stackcollapse.pl \
                   (https://github.com/brendangregg/FlameGraph)")
            .required(true)
            .index(1))
        .arg(Arg::with_name("PID")
            .help("PID of the Ruby process you want to profile")
            .required(true)
            .index(2))
        .get_matches()
}


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

unsafe fn copy_address_dynamic<'a>(
        addr: *const c_void, lookup_table: &DwarfLookup,
        pid: pid_t, struct_name: &str) -> HashMap<String, Vec<u8>> {
    let dwarf_type = lookup_table.lookup_thing(struct_name).unwrap();
    let size = dwarf_type.byte_size.unwrap() + 200; // todo: is a hack
    let bytes = copy_address_raw(addr as *mut c_void, size, pid);
    map_bytes_to_struct(&bytes, lookup_table, struct_name)
}

fn map_bytes_to_struct2<'a>(
        bytes: &Vec<u8>,
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
                struct_map.insert(name, b.to_vec());
            }
        }
    }
    struct_map

}

fn map_bytes_to_struct<'a>(
        bytes: &Vec<u8>,
        lookup_table: &DwarfLookup,
        struct_name: &str) -> HashMap<String, Vec<u8>> {
    let dwarf_type = lookup_table.lookup_thing(struct_name).unwrap();
    // println!("dwarf_type: {:#?}", dwarf_type);
    map_bytes_to_struct2(bytes, lookup_table, dwarf_type)
}

fn read_pointer_address(vec: &Vec<u8>) -> u64 {
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

fn get_ruby_string2(addr: u64, pid: pid_t, lookup_table: &DwarfLookup) -> OsString {
     let vec = unsafe {
        let rstring = copy_address_dynamic(addr as *const c_void, lookup_table, pid, "RString");
        let basic =  map_bytes_to_struct(&rstring["basic"], lookup_table, "RBasic");
        let is_array = (read_pointer_address(&basic["flags"]) & 1 << 13) == 0;
        if is_array  {
           // println!("it's an array!!!!");
           // println!("rstring {:#?}", rstring);
           CStr::from_ptr(rstring["as"].as_ptr() as *const i8).to_bytes().to_vec()
        } else {
            let entry = lookup_table.lookup_thing("RString").unwrap();
                // println!("entry: {:?}", entry);
            let as_type = get_child(&entry, "as").unwrap();
            let blah = lookup_table.lookup_entry(as_type).unwrap();
            // println!("blah: {:?}", blah);
            let heap_type = get_child(&blah, "heap").unwrap();
            let blah2 = lookup_table.lookup_entry(heap_type).unwrap();
            let hashmap = map_bytes_to_struct2(&rstring["as"], lookup_table, blah2);
            copy_address_raw(
                read_pointer_address(&hashmap["ptr"]) as *const c_void,
                read_pointer_address(&hashmap["len"]) as usize,
                pid)
        }
    };
    OsString::from_vec(vec)   
}


unsafe fn get_label_and_path(cfp_bytes: &Vec<u8>, pid: pid_t, lookup_table: &DwarfLookup) -> Option<(OsString, OsString)> {
    let blah2 = map_bytes_to_struct(&cfp_bytes, &lookup_table, "rb_control_frame_struct");
    // println!("{:?}", blah2);
    let iseq_address = read_pointer_address(&blah2["iseq"]);
    let blah3 = copy_address_dynamic(iseq_address as *const c_void, &lookup_table, pid, "rb_iseq_struct");
    let location = if blah3.contains_key("location") {
        map_bytes_to_struct(&blah3["location"], &lookup_table, "rb_iseq_location_struct")
    } else if blah3.contains_key("body") {
    let body = read_pointer_address(&blah3["body"]);
    let blah4 = copy_address_dynamic(body as *const c_void, &lookup_table, pid, "rb_iseq_constant_body");
        map_bytes_to_struct(&blah4["location"], &lookup_table, "rb_iseq_location_struct")
    } else {
        panic!("DON'T KNOW WHERE LOCATION IS");
    };
    let label_address = read_pointer_address(&location["label"]);
    let path_address = read_pointer_address(&location["path"]);
    let label = get_ruby_string2(label_address, pid, lookup_table);
    let path = get_ruby_string2(path_address, pid, lookup_table);
    if path.to_string_lossy() == "" {
        return None;
    }
    // println!("label_address: {}, path_address: {}", label_address, path_address);
    // println!("location hash: {:#?}", location);
    Some((label, path))
}

unsafe fn get_cfps(ruby_current_thread_address_location: u64, pid: pid_t, lookup_table: &DwarfLookup) -> (Vec<u8>, usize) {
    let ruby_current_thread_address: u64 = read_pointer_address(&copy_address_raw(ruby_current_thread_address_location as *const c_void, 8, pid));
    let blah = copy_address_dynamic(ruby_current_thread_address as *const c_void, &lookup_table, pid, "rb_thread_struct");
    // println!("{:?}", blah);
    let cfp_address = read_pointer_address(&blah["cfp"]);
    let cfp_struct = lookup_table.lookup_thing("rb_control_frame_struct").unwrap();
    let cfp_size = cfp_struct.byte_size.unwrap();
    (copy_address_raw(cfp_address as *const c_void, cfp_size * 100, pid), cfp_size)
}

fn get_stack_trace(ruby_current_thread_address_location: u64, pid: pid_t, lookup_table: &DwarfLookup) -> Vec<String> {
    unsafe {

    let (cfp_bytes, cfp_size) = get_cfps(ruby_current_thread_address_location, pid, lookup_table);
    let mut trace: Vec<String> = Vec::new();
    for i in 0..100 {
        match get_label_and_path(&cfp_bytes[(cfp_size*i)..].to_vec(), pid, lookup_table) {
            None => continue,
            Some((label, path)) => {
                let current_location = 
                    format!("{} : {}", label.to_string_lossy(), path.to_string_lossy()).to_string();
                trace.push(current_location);
                if label.to_string_lossy() == "<main>" {
                    break;
                }
            }
        }
    }
    trace
    }
}

fn main() {
    let matches = parse_args();
    let pid: pid_t = matches.value_of("PID").unwrap().parse().unwrap();
    let command = matches.value_of("COMMAND").unwrap();
    if command.clone() != "top" && command.clone() != "stackcollapse" && command.clone() != "parse" {
        println!("COMMAND must be 'top' or 'stackcollapse. Try again!");
        process::exit(1);
    }

    let entries = get_dwarf_entries(pid as usize);
    let lookup_table = create_lookup_table(&entries);
    let ruby_current_thread_address_location: u64 = get_ruby_current_thread_address(pid);

    if command == "parse" {
        return;
    } else if command == "stackcollapse" {
        // This gets a stack trace and then just prints it out
        // in a format that Brendan Gregg's stackcollapse.pl script understands
        loop {
            let trace = get_stack_trace(ruby_current_thread_address_location, pid, &lookup_table);
            print_stack_trace(&trace);
            thread::sleep(Duration::from_millis(10));
        }
    } else {
        // top subcommand!
        // keeps a running histogram of how often we see every method
        // and periodically reports 'self' and 'total' time for each method
        let mut method_stats = HashMap::new();
        let mut method_own_time_stats = HashMap::new();
        let mut j = 0;
        loop {
            j += 1;
            let trace = get_stack_trace(ruby_current_thread_address_location, pid, &lookup_table);
            for item in &trace {
                let counter = method_stats.entry(item.clone()).or_insert(0);
                *counter += 1;
            }
            {
                let counter2 = method_own_time_stats.entry(trace[0].clone()).or_insert(0);
                *counter2 += 1;
            }
            if j % 100 == 0 {
                print_method_stats(&method_stats, &method_own_time_stats, 30);
                method_stats = HashMap::new();
                method_own_time_stats = HashMap::new();
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}
