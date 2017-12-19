#![cfg_attr(rustc_nightly, feature(test))]
#[macro_use] extern crate log;

#[cfg(test)]
#[macro_use] extern crate lazy_static;

extern crate libc;
extern crate regex;
extern crate fnv;
extern crate rand;
extern crate read_process_memory;

extern crate clap;

pub mod dwarf;
pub mod ruby_bindings;

use libc::*;
use std::process;
use std::mem;
use std::os::unix::prelude::*;
// use std::ffi::{OsString, CStr};
use std::process::Command;
use std::process::Stdio;
use regex::Regex;
use std::collections::HashMap;

use ruby_bindings::ruby_bindings::*;
use read_process_memory::*;

pub mod test_utils;



// These three functions (get_cfps, get_iseq, and get_ruby_string) are the
// core of how the program works. They're essentially a straight port of
// this gdb script:
// https://gist.github.com/csfrancis/11376304/raw/7a0450d11e64e3bb7c982b7ad2778f3603188c0f/gdb_ruby_backtrace.py
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
    let exe = dwarf::get_executable_path(pid as usize).unwrap();
    let nm_command = Command::new("nm")
        .arg(exe)
        .stdout(Stdio::piped())
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {}", e));
    if !nm_command.status.success() {
        panic!("failed to execute process: {}", String::from_utf8(nm_command.stderr).unwrap())
    }

    let nm_output = String::from_utf8(nm_command.stdout).unwrap();
    let re = Regex::new(r"(\w+) [bs] _?ruby_current_thread").unwrap();
    let cap = re.captures(&nm_output).unwrap_or_else(|| {
        println!("Error: Couldn't find current thread in Ruby process. This is probably because \
                  either this isn't a Ruby process or you have a Ruby version compiled with no \
                  symbols.");
        process::exit(1)
    });
    let address_str = cap.get(1).unwrap().as_str();
    let addr = u64::from_str_radix(address_str, 16).unwrap();
    println!("get_nm_address: {:x}", addr);
    addr
}

#[cfg(target_os="linux")]
fn get_maps_address(pid: pid_t) -> u64 {
    let cat_command = Command::new("cat")
        .arg(format!("/proc/{}/maps", pid))
        .stdout(Stdio::piped())
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {}", e));
    if !cat_command.status.success() {
        panic!("failed to execute process: {}", String::from_utf8(cat_command.stderr).unwrap())
    }

    let output = String::from_utf8(cat_command.stdout).unwrap();
    let re = Regex::new(r"(\w+).+xp.+?bin/ruby").unwrap();
    let cap = re.captures(&output).unwrap();
    let address_str = cap.get(1).unwrap().as_str();
    let addr = u64::from_str_radix(address_str, 16).unwrap();
    println!("get_maps_address: {:x}", addr);
    addr
}

use std::iter::Iterator;

#[cfg(target_os="macos")]
fn get_maps_address(pid: pid_t) -> u64 {
    let vmmap_command = Command::new("vmmap")
        .arg(format!("{}", pid))
        .stdout(Stdio::piped())
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {}", e));
    if !vmmap_command.status.success() {
        panic!("failed to execute process: {}", String::from_utf8(vmmap_command.stderr).unwrap())
    }

    let output = String::from_utf8(vmmap_command.stdout).unwrap();

    let lines: Vec<&str> = output.split("\n")
        .filter(|line| line.contains("bin/ruby"))
        .filter(|line| line.contains("__TEXT"))
        .collect();
    let line = lines.first().expect("No `__TEXT` line found for `bin/ruby` in vmmap output");

    let re = Regex::new(r"([0-9a-f]+)").unwrap();
    let cap = re.captures(&line).unwrap();
    let address_str = cap.at(1).unwrap();
    let addr = u64::from_str_radix(address_str, 16).unwrap();
    debug!("get_maps_address: {:x}", addr);
    addr
}

#[cfg(target_os="linux")]
pub fn get_ruby_current_thread_address(pid: pid_t) -> u64 {
    // Get the address of the `ruby_current_thread` global variable. It works
    // by looking up the address in the Ruby binary's symbol table with `nm
    // /proc/$pid/exe` and then finding out which address the Ruby binary is
    // mapped to by looking at `/proc/$pid/maps`. If we add these two
    // addresses together we get our answers! All this is Linux-specific but
    // this program only works on Linux anyway because of process_vm_readv.
    //
    println!("{:x}", get_nm_address(pid));
    println!("{:x}", get_maps_address(pid));
    let addr = get_nm_address(pid) + get_maps_address(pid);
    debug!("get_ruby_current_thread_address: {:x}", addr);
    addr
}

#[cfg(target_os="macos")]
pub fn get_ruby_current_thread_address(pid: pid_t) -> u64 {
    // TODO: Make this actually look up the `__mh_execute_header` base
    //   address in the binary via `nm`.
    let base_address = 0x100000000;
    let addr = get_nm_address(pid) + (get_maps_address(pid) - base_address);
    println!("get_ruby_current_thread_address: {:x}", addr);
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

// fn read_pointer_address(vec: &[u8]) -> u64 {
//     let mut rdr = Cursor::new(vec);
//     rdr.read_u64::<NativeEndian>().unwrap()
// }
// 
// fn get_child<'a>(entry: &'a Entry, name: &'a str) -> Option<&'a Entry>{
//     for child in &entry.children {
//         if child.name == Some(name.to_string()) {
//             return Some(child);
//         }
//     }
//     None
// }

//fn get_ruby_string2(addr: u64, source_pid: &ProcessHandle, lookup_table: &DwarfLookup, types: &DwarfTypes) -> OsString
//{
//     let vec = {
//        let rstring = copy_address_dynamic(addr as *const c_void, lookup_table, source_pid, &types.rstring);
//        let basic =  map_bytes_to_struct(&rstring["basic"], lookup_table, &types.rbasic);
//        let is_array = (read_pointer_address(&basic["flags"]) & 1 << 13) == 0;
//        if is_array {
//           // println!("it's an array!!!!");
//           // println!("rstring {:#?}", rstring);
//           unsafe { CStr::from_ptr(rstring["as"].as_ptr() as *const i8) }.to_bytes().to_vec()
//        } else {
//            let entry = &types.rstring;
//                // println!("entry: {:?}", entry);
//            let as_type = get_child(&entry, "as").unwrap();
//            let blah = lookup_table.lookup_entry(as_type).unwrap();
//            // println!("blah: {:?}", blah);
//            let heap_type = get_child(&blah, "heap").unwrap();
//            let blah2 = lookup_table.lookup_entry(heap_type).unwrap();
//            let hashmap = map_bytes_to_struct(&rstring["as"], lookup_table, blah2);
//            copy_address_raw(
//                read_pointer_address(&hashmap["ptr"]) as *const c_void,
//                read_pointer_address(&hashmap["len"]) as usize,
//                source_pid)
//        }
//    };
//    OsString::from_vec(vec)
//}

//fn get_label_and_path(cfp_bytes: &ruby_bindings::rb_control_frame_struct, source_pid: &ProcessHandle) -> Option<(OsString, OsString)>
//{
//    trace!("get_label_and_path {:?}", cfp_bytes);
//    let iseq_address = cfp_bytes.iseq;
//    let blah3: ruby_bindings::rb_iseq_struct = copy_struct(iseq_address as *const c_void, source_pid);
//    let location = if blah3.contains_key("location") {
//        map_bytes_to_struct(&blah3["location"], &lookup_table, &types.rb_iseq_location_struct)
//    } else if blah3.contains_key("body") {
//        let body = read_pointer_address(&blah3["body"]);
//        match types.rb_iseq_constant_body {
//            Some(ref constant_body_type) => {
//                let blah4 = copy_address_dynamic(body as *const c_void, &lookup_table, source_pid, &constant_body_type);
//                map_bytes_to_struct(&blah4["location"], &lookup_table, &types.rb_iseq_location_struct)
//            }
//            None => panic!("rb_iseq_constant_body shouldn't be missing"),
//        }
//    } else {
//        panic!("DON'T KNOW WHERE LOCATION IS");
//    };
//    let label_address = read_pointer_address(&location["label"]);
//    let path_address = read_pointer_address(&location["path"]);
//    let label = get_ruby_string2(label_address, source_pid, lookup_table, types);
//    let path = get_ruby_string2(path_address, source_pid, lookup_table, types);
//    if path.to_string_lossy() == "" {
//        trace!("get_label_and_path ret None");
//        return None;
//    }
//    // println!("label_address: {}, path_address: {}", label_address, path_address);
//    // println!("location hash: {:#?}", location);
//    let ret = Some((label, path));
//
//    trace!("get_label_and_path ret {:?}", ret);
//    ret
//}

// Ruby stack grows down, starting at
//   ruby_current_thread->stack + ruby_current_thread->stack_size - 1 * sizeof(rb_control_frame_t)
// I don't know what the -1 is about. Also note that the stack_size is *not* in bytes! stack is a
// VALUE*, and so stack_size is in units of sizeof(VALUE).
//
// The base of the call stack is therefore at
//   stack + stack_size * sizeof(VALUE) - sizeof(rb_control_frame_t)
// (with everything in bytes).
 fn get_cfps(ruby_current_thread_address_location: u64, source_pid: &ProcessHandle) -> (Vec<u8>, usize)
 {
     let thread: rb_thread_t = copy_struct(ruby_current_thread_address_location, source_pid);
     let cfp_address = thread.cfp as u64;

     let stack = thread.stack as u64;
     let stack_size = thread.stack_size as u64;
     let value_size = mem::size_of::<VALUE>() as u64;
     let cfp_size = mem::size_of::<rb_control_frame_struct>() as u64;
 
     let stack_base = stack + stack_size * value_size - 1 * cfp_size;
 
     let ret = copy_address_raw(cfp_address as *const c_void, (stack_base - cfp_address) as usize, source_pid);
 
     (ret, 3)
 }

fn copy_address_raw(addr: *const c_void, length: usize, source_pid: &ProcessHandle) -> Vec<u8>
{
    debug!("copy_address_raw: addr: {:x}", addr as usize);
    let mut copy = vec![0; length];
    match source_pid.copy_address(addr as usize, &mut copy) {
        Ok(_) => {}
        Err(e) => warn!("copy_address failed for {:p}: {:?}", addr, e),
    }
    copy
}

fn copy_struct<U>(addr: u64, source_pid: &ProcessHandle) -> U {
    let result = copy_address_raw(addr as *const c_void, mem::size_of::<U>(), source_pid);
    let s: U = unsafe { std::ptr::read(result.as_ptr() as *const _) };
    s
}

pub fn get_stack_trace(ruby_current_thread_address_location: u64, source_pid: &ProcessHandle)//  -> Vec<String>
{
    debug!("current address location: {:x}", ruby_current_thread_address_location);
    let current_thread_addr: u64 = copy_struct(ruby_current_thread_address_location, source_pid);
    let thread: rb_thread_t = copy_struct(current_thread_addr, source_pid);
    println!("{:?}", thread);
//     let (cfp_bytes, cfp_size) = get_cfps(ruby_current_thread_address_location, source_pid, lookup_table, types);
//     let mut trace: Vec<String> = Vec::new();
//     for i in 0..cfp_bytes.len() / cfp_size {
//         match get_label_and_path(&cfp_bytes[(cfp_size*i)..cfp_size * (i+1)].to_vec(), source_pid, lookup_table, types) {
//             None => continue,
//             Some((label, path)) => {
//                 let current_location =
//                     format!("{} : {}", label.to_string_lossy(), path.to_string_lossy()).to_string();
//                 trace.push(current_location);
//             }
//         }
//     }
//     trace
}
