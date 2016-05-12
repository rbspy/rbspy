extern crate libc;
extern crate regex;
use libc::*;
use std::env;
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
    let mut copy = vec![0;length];
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
    let cap = re.captures(&nm_output).unwrap();
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

fn print_hashmap(map: &HashMap<String, u32>) {
    let mut count_vec: Vec<_> = map.iter().collect();
    count_vec.sort_by(|a, b| b.1.cmp(a.1));
    for (a, b) in count_vec {
        println!("{:?}: {}", a, b);
    }
}


fn main() {
    let args: Vec<_> = env::args().collect();
    let pid: pid_t = args[1].parse().unwrap();
    let ruby_current_thread_address_location = get_ruby_current_thread_address(pid);

    let mut method_stats = HashMap::new();
    let mut method_own_time_stats = HashMap::new();
    let mut j = 0;
    loop {
        j += 1;
        let cfps = get_cfps(ruby_current_thread_address_location, pid);
        println!("--------------------------------");
        println!("Stack trace:");
        for i in 0..100 {
            let iseq = get_iseq(&cfps[i], pid);
            if !cfps[i].pc.is_null() {
                let label = get_ruby_string(iseq.location.label as VALUE, pid);
                let path = get_ruby_string(iseq.location.path as VALUE, pid);
                let current_location = format!("{:?} : {:?}", path, label).to_string();
                let counter = method_stats.entry(current_location.clone()).or_insert(0);
                *counter += 1;
                if i == 0 {
                    let counter2 = method_own_time_stats.entry(current_location).or_insert(0);
                    *counter += 1;
                }
            }
        }
        if j % 100 == 0 {
            print_hashmap(&method_stats);
        }
        thread::sleep(Duration::from_millis(50));
    }
}
