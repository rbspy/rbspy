extern crate libc;
extern crate regex;
extern crate term;
use libc::*;
use std::env;
use std::process;
use std::os::unix::prelude::*;
use std::time::Duration;
use std::thread;
use std::ffi::{OsString, CStr};
use std::mem;
use std::slice;
use std::process::Command;
use std::process::Stdio;
use regex::Regex;
mod ruby_vm;
use std::collections::HashMap;
use ruby_vm::{rb_iseq_t, rb_control_frame_t, rb_thread_t, Struct_RString, VALUE};

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
        iov_len: length
    };
    let remote_iov = iovec {
        iov_base: addr as *mut c_void,
        iov_len: length
    };
    unsafe {
        process_vm_readv(pid, &local_iov, 1, &remote_iov, 1, 0);
    }
    copy
}

unsafe fn copy_address<T>(addr: * const T, pid: pid_t) -> T {
    let mut value: T = mem::uninitialized();
    let local_iov = iovec {
        iov_base: &mut value as *mut _ as * mut c_void,
        iov_len: mem::size_of::<T>()
    };
    let remote_iov = iovec {
        iov_base: addr as *mut c_void,
        iov_len: mem::size_of::<T>()
    };
    process_vm_readv(pid, &local_iov, 1, &remote_iov, 1, 0);
    value
}

fn get_ruby_string(address: VALUE, pid: pid_t) -> OsString {
    let vec = unsafe {
        let mut rstring = copy_address(address as *const Struct_RString, pid);
        if (rstring).basic.flags & (1 << 13) != 0 {
            copy_address_raw((*rstring._as.heap()).ptr as *const c_void, (*rstring._as.heap()).len as usize, pid)
        } else {
            CStr::from_ptr((*rstring._as.ary()).as_ptr()).to_bytes().to_vec()
        }
    };
    OsString::from_vec(vec)
}

fn get_iseq(cfp: &rb_control_frame_t, pid: pid_t) -> rb_iseq_t {
    unsafe {
        copy_address(cfp.iseq as *const rb_iseq_t, pid)
    }
}

fn get_nm_address(pid: pid_t) -> u64 {
    let nm_command = Command::new("nm").arg(format!("/proc/{}/exe", pid))
        .stdout(Stdio::piped())
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .unwrap_or_else(|e| { panic!("failed to execute process: {}", e) });
    let nm_output = String::from_utf8(nm_command.stdout).unwrap();
    let re = Regex::new(r"(\w+) b ruby_current_thread").unwrap();
    let cap = re.captures(&nm_output).unwrap_or_else(|| {
        println!("Error: Couldn't find current thread in Ruby process. This is probably because either this isn't a Ruby process or you have a Ruby version compiled with no symbols.");
        process::exit(1)
    });
    let address_str = cap.at(1).unwrap();
    u64::from_str_radix(address_str, 16).unwrap()
}

fn get_maps_address(pid: pid_t) -> u64 {

    let cat_command = Command::new("cat").arg(format!("/proc/{}/maps", pid))
        .stdout(Stdio::piped())
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .unwrap_or_else(|e| { panic!("failed to execute process: {}", e) });
    let output = String::from_utf8(cat_command.stdout).unwrap();
    let re = Regex::new(r"\n(\w+).+?bin/ruby").unwrap();
    let cap = re.captures(&output).unwrap();
    let address_str = cap.at(1).unwrap();
    u64::from_str_radix(address_str, 16).unwrap()
}

fn get_ruby_current_thread_address(pid: pid_t)->u64 {
    get_nm_address(pid) + get_maps_address(pid)
}

fn get_cfps<'a>(ruby_current_thread_address_location:u64, pid: pid_t) -> &'a[rb_control_frame_t] {
    let ruby_current_thread_address = unsafe {
        copy_address(ruby_current_thread_address_location as * const u64, pid)
    };
    let thread = unsafe {
        copy_address(ruby_current_thread_address as *const rb_thread_t, pid)
    };
    unsafe {
        let result = copy_address_raw(thread.cfp as *mut c_void, 100 * mem::size_of::<ruby_vm::rb_control_frame_t>(), pid);
        slice::from_raw_parts(result.as_ptr() as *const ruby_vm::rb_control_frame_t, 100)
    }
}

fn print_method_stats(method_stats: &HashMap<String, u32>, method_own_time_stats: &HashMap<String, u32>, n_terminal_lines: usize) {
    let mut count_vec: Vec<_> = method_own_time_stats.iter().collect();
    count_vec.sort_by(|a, b| b.1.cmp(a.1));
    println!(" {:4} | {:4} | {}", "self", "tot", "method");
    let self_sum: u32 = method_own_time_stats.values().fold(0, std::ops::Add::add);
    let total_sum: u32 = *method_stats.values().max().unwrap();
    for &(method, count) in count_vec.iter().take(n_terminal_lines - 1) {
        let total_count = method_stats.get(&method[..]).unwrap();
        println!(" {:02.1}% | {:02.1}% | {}", 100.0 * (*count as f32) / (self_sum as f32), 100.0 * (*total_count as f32)  / (total_sum as f32), method);
    }
}

fn get_stack_trace<'a>(ruby_current_thread_address_location: u64, pid: pid_t) -> Vec<String> {
    let cfps = get_cfps(ruby_current_thread_address_location, pid);
    let mut trace: Vec<String> = Vec::new();
    for i in 0..100 {
        let iseq = get_iseq(&cfps[i], pid);
        if !cfps[i].pc.is_null() {
            let label = get_ruby_string(iseq.location.label as VALUE, pid);
            let path = get_ruby_string(iseq.location.path as VALUE, pid);
            if (path.to_str().unwrap() == "") {
                continue;
            }
            let current_location = format!("{} : {}", label.to_string_lossy(), path.to_string_lossy()).to_string();
            trace.push(current_location);
        }
    }
    trace
}

fn print_stack_trace(trace: &Vec<String>) {
    for x in trace {
        println!("{}", x);
    }
    println!("{}", 1);
}


fn main() {
    let args: Vec<_> = env::args().collect();
    let pid: pid_t = args[1].parse().unwrap();
    let ruby_current_thread_address_location = get_ruby_current_thread_address(pid);
    let mut j = 0;
    loop {
        j += 1;
        let trace = get_stack_trace(ruby_current_thread_address_location, pid);
        print_stack_trace(&trace);
        thread::sleep(Duration::from_millis(10));
    }
}
