#![cfg_attr(rustc_nightly, feature(test))]
#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

extern crate libc;
extern crate regex;
extern crate fnv;
extern crate rand;
extern crate read_process_memory;

extern crate clap;

pub mod dwarf;
pub mod bindings;

use libc::*;
use std::process;
use std::process::Command;
use std::process::Stdio;
use regex::Regex;
use std::collections::HashMap;


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
// (* const rb_thread_t *(ruby_current_thread_address_location))->cfp
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
        panic!(
            "failed to execute process: {}",
            String::from_utf8(nm_command.stderr).unwrap()
        )
    }

    let nm_output = String::from_utf8(nm_command.stdout).unwrap();
    let re = Regex::new(r"(\w+) [bs] _?ruby_current_thread").unwrap();
    let cap = re.captures(&nm_output).unwrap_or_else(|| {
        println!(
            "Error: Couldn't find current thread in Ruby process. This is probably because \
                  either this isn't a Ruby process or you have a Ruby version compiled with no \
                  symbols."
        );
        process::exit(1)
    });
    let address_str = cap.get(1).unwrap().as_str();
    let addr = u64::from_str_radix(address_str, 16).unwrap();
    debug!("get_nm_address: {:x}", addr);
    addr
}

#[cfg(target_os = "linux")]
fn get_maps_address(pid: pid_t) -> u64 {
    let cat_command = Command::new("cat")
        .arg(format!("/proc/{}/maps", pid))
        .stdout(Stdio::piped())
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {}", e));
    if !cat_command.status.success() {
        panic!(
            "failed to execute process: {}",
            String::from_utf8(cat_command.stderr).unwrap()
        )
    }

    let output = String::from_utf8(cat_command.stdout).unwrap();
    let re = Regex::new(r"(\w+).+xp.+?bin/ruby").unwrap();
    let cap = re.captures(&output).unwrap();
    let address_str = cap.get(1).unwrap().as_str();
    let addr = u64::from_str_radix(address_str, 16).unwrap();
    debug!("get_maps_address: {:x}", addr);
    addr
}

use std::iter::Iterator;

#[cfg(target_os = "macos")]
fn get_maps_address(pid: pid_t) -> u64 {
    let vmmap_command = Command::new("vmmap")
        .arg(format!("{}", pid))
        .stdout(Stdio::piped())
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {}", e));
    if !vmmap_command.status.success() {
        panic!(
            "failed to execute process: {}",
            String::from_utf8(vmmap_command.stderr).unwrap()
        )
    }

    let output = String::from_utf8(vmmap_command.stdout).unwrap();

    let lines: Vec<&str> = output
        .split("\n")
        .filter(|line| line.contains("bin/ruby"))
        .filter(|line| line.contains("__TEXT"))
        .collect();
    let line = lines.first().expect(
        "No `__TEXT` line found for `bin/ruby` in vmmap output",
    );

    let re = Regex::new(r"([0-9a-f]+)").unwrap();
    let cap = re.captures(&line).unwrap();
    let address_str = cap.at(1).unwrap();
    let addr = u64::from_str_radix(address_str, 16).unwrap();
    debug!("get_maps_address: {:x}", addr);
    addr
}

#[cfg(target_os = "linux")]
pub fn get_ruby_current_thread_address(pid: pid_t) -> u64 {
    // Get the address of the `ruby_current_thread` global variable. It works
    // by looking up the address in the Ruby binary's symbol table with `nm
    // /proc/$pid/exe` and then finding out which address the Ruby binary is
    // mapped to by looking at `/proc/$pid/maps`. If we add these two
    // addresses together we get our answers! All this is Linux-specific but
    // this program only works on Linux anyway because of process_vm_readv.
    //
    debug!("{:x}", get_nm_address(pid));
    debug!("{:x}", get_maps_address(pid));
    let addr = get_nm_address(pid) + get_maps_address(pid);
    debug!("get_ruby_current_thread_address: {:x}", addr);
    addr
}

#[cfg(target_os = "macos")]
pub fn get_ruby_current_thread_address(pid: pid_t) -> u64 {
    // TODO: Make this actually look up the `__mh_execute_header` base
    //   address in the binary via `nm`.
    let base_address = 0x100000000;
    let addr = get_nm_address(pid) + (get_maps_address(pid) - base_address);
    debug!("get_ruby_current_thread_address: {:x}", addr);
    addr
}

pub fn print_method_stats(
    method_stats: &HashMap<String, u32>,
    method_own_time_stats: &HashMap<String, u32>,
    n_terminal_lines: usize,
) {
    println!("[{}c", 27 as char); // clear the screen
    let mut count_vec: Vec<_> = method_own_time_stats.iter().collect();
    count_vec.sort_by(|a, b| b.1.cmp(a.1));
    println!(" {:4} | {:4} | {}", "self", "tot", "method");
    let self_sum: u32 = method_own_time_stats.values().fold(0, std::ops::Add::add);
    let total_sum: u32 = *method_stats.values().max().unwrap();
    for &(method, count) in count_vec.iter().take(n_terminal_lines - 1) {
        let total_count = method_stats.get(&method[..]).unwrap();
        println!(
            " {:02.1}% | {:02.1}% | {}",
            100.0 * (*count as f32) / (self_sum as f32),
            100.0 * (*total_count as f32) / (total_sum as f32),
            method
        );
    }
}

pub fn print_stack_trace(trace: &[String]) {
    for x in trace {
        println!("{}", x);
    }
    println!("{}", 1);
}

pub mod stack_trace {
    use std;
    use bindings::ruby_2_2_0::*;
    use libc::*;
    use read_process_memory::*;
    use std::ffi::{OsString, CStr};
    use std::mem;
    use std::os::unix::prelude::*;

    pub fn get_stack_trace(
        ruby_current_thread_address_location: u64,
        source_pid: &ProcessHandle,
    ) -> Vec<String> {
        debug!(
            "current address location: {:x}",
            ruby_current_thread_address_location
        );
        let current_thread_addr: u64 =
            copy_struct(ruby_current_thread_address_location, source_pid);
        debug!("{:x}", current_thread_addr);
        let thread: rb_thread_t = copy_struct(current_thread_addr, source_pid);
        debug!("{:?}", thread);
        let mut trace = Vec::new();
        let cfps = get_cfps(&thread, source_pid);
        for cfp in cfps.iter() {
            let (label, path) = get_label_and_path(&cfp, source_pid);
            let current_location =
                format!("{} : {}", label.to_string_lossy(), path.to_string_lossy()).to_string();
            trace.push(current_location);
        }
        trace
    }

    fn copy_address_raw(addr: *const c_void, length: usize, source_pid: &ProcessHandle) -> Vec<u8> {
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
        debug!("{:?}", result);
        let s: U = unsafe { std::ptr::read(result.as_ptr() as *const _) };
        s
    }

    fn get_ruby_string(addr: u64, source_pid: &ProcessHandle) -> OsString {
        let vec = {
            let rstring: RString = copy_struct(addr, source_pid);
            let basic = rstring.basic;
            let is_array = basic.flags & 1 << 13 == 0;
            if is_array {
                unsafe { CStr::from_ptr(rstring.as_.ary.as_ref().as_ptr() as *const i8) }
                    .to_bytes()
                    .to_vec()
            } else {
                unsafe {
                    let addr = rstring.as_.heap.ptr as u64;
                    let len = rstring.as_.heap.len as usize;
                    copy_address_raw(addr as *const c_void, len, source_pid)
                }
            }
        };
        OsString::from_vec(vec)
    }

    fn get_label_and_path(
        cfp: &rb_control_frame_struct,
        source_pid: &ProcessHandle,
    ) -> (OsString, OsString) {
        trace!("get_label_and_path {:?}", cfp);
        let iseq_address = cfp.iseq as u64;
        let iseq_struct: rb_iseq_struct = copy_struct(iseq_address, source_pid);
        debug!("{:?}", iseq_struct);
        let location = iseq_struct.location;
        let label: OsString = get_ruby_string(location.label as u64, source_pid);
        let path: OsString = get_ruby_string(location.path as u64, source_pid);
        println!("{:?} - {:?}", label, path);
        (label, path)
    }

    // Ruby stack grows down, starting at
    //   ruby_current_thread->stack + ruby_current_thread->stack_size - 1 * sizeof(rb_control_frame_t)
    // I don't know what the -1 is about. Also note that the stack_size is *not* in bytes! stack is a
    // VALUE*, and so stack_size is in units of sizeof(VALUE).
    //
    // The base of the call stack is therefore at
    //   stack + stack_size * sizeof(VALUE) - sizeof(rb_control_frame_t)
    // (with everything in bytes).
    fn get_cfps(thread: &rb_thread_t, source_pid: &ProcessHandle) -> Vec<rb_control_frame_struct> {
        let cfp_address = thread.cfp as u64;

        let stack = thread.stack as u64;
        let stack_size = thread.stack_size as u64;
        let value_size = mem::size_of::<VALUE>() as u64;
        let cfp_size = mem::size_of::<rb_control_frame_struct>() as u64;

        let stack_base = stack + stack_size * value_size - 1 * cfp_size;
        debug!("cfp addr: {:x}", cfp_address as usize);
        let mut ret = copy_address_raw(
            cfp_address as *const c_void,
            (stack_base - cfp_address) as usize,
            source_pid,
        );

        let p = ret.as_mut_ptr();
        let cap = ret.capacity();

        let rebuilt: Vec<rb_control_frame_struct> = unsafe {
            // Cast `v` into the void: no destructor run, so we are in
            // complete control of the allocation to which `p` points.
            // Put everything back together into a Vec
            mem::forget(ret);
            Vec::from_raw_parts(
                p as *mut rb_control_frame_struct,
                cap / (cfp_size as usize),
                cap,
            )
        };

        rebuilt
    }
}
