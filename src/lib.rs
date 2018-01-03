#![cfg_attr(rustc_nightly, feature(test))]
#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

extern crate elf;
extern crate libc;
extern crate regex;
extern crate read_process_memory;

pub mod bindings;

pub mod test_utils;

pub mod user_interface {
    use std;
    use std::collections::HashMap;
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
}

pub mod address_finder {
    use copy::copy_struct;
    use libc::*;
    use regex::Regex;
    use std::process::Command;
    use std::process::Stdio;
    use std;

    use self::obj::get_executable_path;
    #[cfg(target_os = "linux")]
    mod obj {
        use std::path::PathBuf;

        pub fn get_executable_path(pid: usize) -> Result<PathBuf, String> {
            Ok(PathBuf::from(format!("/proc/{}/exe", pid)))
        }
    }

    #[cfg(target_os = "macos")]
    mod obj {
        extern crate libproc;

        use std::path::PathBuf;

        pub fn get_executable_path(pid: usize) -> Result<PathBuf, String> {
            libproc::libproc::proc_pid::pidpath(pid as i32).map(|path| PathBuf::from(&path))
        }
    }

    fn get_nm_output(pid: pid_t) -> String {
        let exe = get_executable_path(pid as usize).unwrap();
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

        String::from_utf8(nm_command.stdout).unwrap()
    }

    fn get_api_version_addr(pid: pid_t) -> Result<u64, Box<std::error::Error>> {
        let nm_output = get_nm_output(pid);
        let re = Regex::new(r"(\w+) R _?ruby_version")?;
        let cap = re.captures(&nm_output).ok_or("failed to match regex")?;
        let address_str = cap.get(1).ok_or("oh no")?.as_str();
        let addr = u64::from_str_radix(address_str, 16)?;
        debug!("get_api_version: {:x}", addr);
        Ok(addr)
    }

    pub fn get_api_version(pid: pid_t) -> Result<String, Box<std::error::Error>> {
        let addr = get_api_address(pid)?;
        let x: [c_char; 15] = copy_struct(addr, pid)?;
        Ok(unsafe {
            std::ffi::CStr::from_ptr(x.as_ptr() as *mut c_char)
                .to_str()
                .unwrap()
                .to_owned()
        })
    }

    fn get_nm_address(pid: pid_t) -> Result<u64, Box<std::error::Error>> {
        let nm_output = get_nm_output(pid);
        let re = Regex::new(r"(\w+) [bs] _?ruby_current_thread")?;
        let address_str = re.captures(&nm_output)
            .ok_or("regexp didn't match")?
            .get(1)
            .ok_or("regexp didn't match")?
            .as_str();
        let addr = u64::from_str_radix(address_str, 16)?;
        debug!("get_nm_address: {:x}", addr);
        Ok(addr)
    }

    fn is_heap_addr(x: u64, heap_start: u64, heap_end: u64) -> bool {
        x >= heap_start && x <= heap_end
    }

    #[cfg(target_os = "linux")]
    fn get_maps_output(pid: pid_t) -> Result<String, Box<std::error::Error>> {
        let cat_command = Command::new("cat")
            .arg(format!("/proc/{}/maps", pid))
            .stdout(Stdio::piped())
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .output()?;
        if !cat_command.status.success() {
            None.ok_or("cat command failed")?;
        }
        Ok(String::from_utf8(cat_command.stdout)?)
    }

    #[cfg(target_os = "linux")]
    fn get_heap_range(pid: pid_t) -> Result<(u64, u64), Box<std::error::Error>> {
        let output = get_maps_output(pid)?;
        let re = Regex::new(r"\n([a-f0-9]+)-([a-f0-9]+).+heap")?;
        let cap = re.captures(&output).ok_or("Failed to match regular expression")?;

        let start_str = cap.get(1).unwrap().as_str();
        let start = u64::from_str_radix(start_str, 16).unwrap();

        let end_str = cap.get(2).unwrap().as_str();
        let end = u64::from_str_radix(end_str, 16).unwrap();
        Ok((start, end))
    }

    #[cfg(target_os = "linux")]
    fn get_maps_address(pid: pid_t) -> Result<u64, Box<std::error::Error>> {
        let output = get_maps_output(pid)?;
        let re = Regex::new(r"(\w+).+xp.+?bin/ruby")?;
        let cap = re.captures(&output).ok_or("failed to parse regexp")?;
        let address_str = cap.get(1).unwrap().as_str();
        let addr = u64::from_str_radix(address_str, 16).unwrap();
        debug!("get_maps_address: {:x}", addr);
        Ok(addr)
    }

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
    pub fn current_thread_address_location(pid: pid_t) -> Result<u64, Box<std::error::Error>> {
        // Get the address of the `ruby_current_thread` global variable. It works
        // by looking up the address in the Ruby binary's symbol table with `nm
        // /proc/$pid/exe` and then finding out which address the Ruby binary is
        // mapped to by looking at `/proc/$pid/maps`. If we add these two
        // addresses together we get our answers! All this is Linux-specific but
        // this program only works on Linux anyway because of process_vm_readv.
        //
        let nm_address = get_nm_address(pid)?;
        let maps_address = get_maps_address(pid)?;
        debug!("nm_address: {:x}", nm_address);
        debug!("maps_address: {:x}", maps_address);
        Ok(nm_address + maps_address)
    }

    #[cfg(target_os = "linux")]
    fn get_api_address(pid: pid_t) -> Result<u64, Box<std::error::Error>> {
        Ok(get_api_version_addr(pid)? + get_maps_address(pid)?)
    }

    #[cfg(target_os = "macos")]
    pub fn current_thread_address_location(pid: pid_t) -> Result<u64, Box<std::error::Error>> {
        // TODO: Make this actually look up the `__mh_execute_header` base
        //   address in the binary via `nm`.
        let base_address = 0x100000000;
        let addr = get_nm_address(pid)? + (get_maps_address(pid)? - base_address);
        debug!("get_ruby_current_thread_address: {:x}", addr);
        addr
    }
}

mod copy {
    use libc::*;
    use read_process_memory::*;
    use std::mem;
    use std;
    pub fn copy_address_raw(addr: *const c_void, length: usize, source_pid: pid_t) -> Result<Vec<u8>, Box<std::error::Error>> {
        let source = source_pid.try_into_process_handle().unwrap();
        debug!("copy_address_raw: addr: {:x}", addr as usize);
        let mut copy = vec![0; length];
        source.copy_address(addr as usize, &mut copy)?;
        Ok(copy)
    }

    pub fn copy_struct<U>(addr: u64, source_pid: pid_t) -> Result<U, Box<std::error::Error>> {
        let result = copy_address_raw(addr as *const c_void, mem::size_of::<U>(), source_pid)?;
        debug!("{:?}", result);
        let s: U = unsafe { std::ptr::read(result.as_ptr() as *const _) };
        Ok(s)
    }
}

macro_rules! ruby_bindings(
($ruby_version:ident) => (
mod $ruby_version {
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

    use copy::{copy_address_raw, copy_struct};
    use std;
    use bindings::$ruby_version::*;
    use libc::*;
    use std::ffi::{OsString, CStr};
    use std::mem;
    use std::os::unix::prelude::*;

    pub fn get_stack_trace(
        ruby_current_thread_address_location: u64,
        source_pid: pid_t,
    ) -> Result<Vec<String>, Box<std::error::Error>> {
        debug!(
            "current address location: {:x}",
            ruby_current_thread_address_location
        );
        let current_thread_addr: u64 =
            copy_struct(ruby_current_thread_address_location, source_pid)?;
        get_stack_trace2(current_thread_addr, source_pid)
    }

    pub fn get_stack_trace2(
        current_thread_addr: u64,
        source_pid: pid_t) -> Result<Vec<String>, Box<std::error::Error>> {
        debug!("{:x}", current_thread_addr);
        let thread: rb_thread_t = copy_struct(current_thread_addr, source_pid)?;
        debug!("{:?}", thread);
        let mut trace = Vec::new();
        let mut cfps = get_cfps(&thread, source_pid)?;
        let slice: &[rb_control_frame_struct] = unsafe {
            std::slice::from_raw_parts(
                cfps.as_mut_ptr() as *mut rb_control_frame_struct,
                cfps.capacity() as usize / mem::size_of::<rb_control_frame_struct>() as usize,
            )
        };
        for cfp in slice.iter() {
            let result  = get_label_and_path(&cfp, source_pid);
            match result {
                Ok((label, path)) => {
                    let current_location =
                        format!("{} : {}", label.to_string_lossy(), path.to_string_lossy()).to_string();
                    trace.push(current_location);
                }
                Err(_) => {
                    warn!("failed to get label and path, ignoring");
                }
            }
        }
        Ok(trace)
    }




    fn get_ruby_string(addr: u64, source_pid: pid_t) -> Result<OsString, Box<std::error::Error>> {
        let vec = {
            let rstring: RString = copy_struct(addr, source_pid)?;
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
                    copy_address_raw(addr as *const c_void, len, source_pid)?
                }
            }
        };
        Ok(OsString::from_vec(vec))
    }

    fn get_label_and_path(
        cfp: &rb_control_frame_struct,
        source_pid: pid_t,
    ) -> Result<(OsString, OsString), Box<std::error::Error>> {
        debug!("get_label_and_path {:?}", cfp);
        let iseq_address = cfp.iseq as u64;
        debug!("iseq_address: {:?}", iseq_address);
        let iseq_struct: rb_iseq_struct = copy_struct(iseq_address, source_pid)?;
        debug!("{:?}", iseq_struct);
        let location = iseq_struct.location;
        let label: OsString = get_ruby_string(location.label as u64, source_pid)?;
        let path: OsString = get_ruby_string(location.path as u64, source_pid)?;
        // println!("{:?} - {:?}", label, path);
        Ok((label, path))
    }

    // Ruby stack grows down, starting at
    //   ruby_current_thread->stack + ruby_current_thread->stack_size - 1 * sizeof(rb_control_frame_t)
    // I don't know what the -1 is about. Also note that the stack_size is *not* in bytes! stack is a
    // VALUE*, and so stack_size is in units of sizeof(VALUE).
    //
    // The base of the call stack is therefore at
    //   stack + stack_size * sizeof(VALUE) - sizeof(rb_control_frame_t)
    // (with everything in bytes).
    fn get_cfps(thread: &rb_thread_t, source_pid: pid_t) -> Result<Vec<u8>, Box<std::error::Error>> {
        let cfp_address = thread.cfp as u64;

        let stack = thread.stack as u64;
        let stack_size = thread.stack_size as u64;
        let value_size = mem::size_of::<VALUE>() as u64;
        let cfp_size = mem::size_of::<rb_control_frame_struct>() as u64;

        let stack_base = stack + stack_size * value_size - 1 * cfp_size;
        debug!("cfp addr: {:x}", cfp_address as usize);
        Ok(copy_address_raw(
            cfp_address as *const c_void,
            (stack_base - cfp_address) as usize,
            source_pid,
        )?)
    }
}
));

macro_rules! ruby_bindings_v2(
($ruby_version:ident) => (
mod $ruby_version {
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

    use std;
    use bindings::$ruby_version::*;
    use libc::*;
    use std::ffi::{OsString, CStr};
    use std::mem;
    use std::os::unix::prelude::*;
    use copy::{copy_address_raw, copy_struct};

    pub fn get_stack_trace(
        ruby_current_thread_address_location: u64,
        source_pid: pid_t,
    ) -> Result<Vec<String>, Box<std::error::Error>> {
        debug!(
            "current address location: {:x}",
            ruby_current_thread_address_location
        );
        let current_thread_addr: u64 =
            copy_struct(ruby_current_thread_address_location, source_pid)?;
        debug!("{:x}", current_thread_addr);
        let thread: rb_thread_t = copy_struct(current_thread_addr, source_pid)?;
        debug!("{:?}", thread);
        let mut trace = Vec::new();
        let mut cfps = get_cfps(&thread, source_pid)?;
        let slice: &[rb_control_frame_struct] = unsafe {
            std::slice::from_raw_parts(
                cfps.as_mut_ptr() as *mut rb_control_frame_struct,
                cfps.capacity() as usize / mem::size_of::<rb_control_frame_struct>() as usize,
            )
        };
        for cfp in slice.iter() {
            let result  = get_label_and_path(&cfp, source_pid);
            match result {
                Ok((label, path)) => {
                    let current_location =
                        format!("{} : {}", label.to_string_lossy(), path.to_string_lossy()).to_string();
                    trace.push(current_location);
                }
                Err(_) => {
                    warn!("failed to get label and path, ignoring");
                }
            }
        }
        Ok(trace)
    }




    fn get_ruby_string(addr: u64, source_pid: pid_t) -> Result<OsString, Box<std::error::Error>> {
        let vec = {
            let rstring: RString = copy_struct(addr, source_pid)?;
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
                    copy_address_raw(addr as *const c_void, len, source_pid)?
                }
            }
        };
        Ok(OsString::from_vec(vec))
    }

    fn get_label_and_path(
        cfp: &rb_control_frame_struct,
        source_pid: pid_t,
    ) -> Result<(OsString, OsString), Box<std::error::Error>> {
        trace!("get_label_and_path {:?}", cfp);
        let iseq_address = cfp.iseq as u64;
        let iseq_struct: rb_iseq_struct = copy_struct(iseq_address, source_pid)?;
        debug!("{:?}", iseq_struct);
        let body_address = iseq_struct.body as u64;
        let body: rb_iseq_constant_body = copy_struct(body_address, source_pid)?;
        let location = body.location;
        let label: OsString = get_ruby_string(location.label as u64, source_pid)?;
        let path: OsString = get_ruby_string(location.path as u64, source_pid)?;
        // println!("{:?} - {:?}", label, path);
        Ok((label, path))
    }

    // Ruby stack grows down, starting at
    //   ruby_current_thread->stack + ruby_current_thread->stack_size - 1 * sizeof(rb_control_frame_t)
    // I don't know what the -1 is about. Also note that the stack_size is *not* in bytes! stack is a
    // VALUE*, and so stack_size is in units of sizeof(VALUE).
    //
    // The base of the call stack is therefore at
    //   stack + stack_size * sizeof(VALUE) - sizeof(rb_control_frame_t)
    // (with everything in bytes).
    fn get_cfps(thread: &rb_thread_t, source_pid: pid_t) -> Result<Vec<u8>, Box<std::error::Error>> {
        let cfp_address = thread.cfp as u64;

        let stack = thread.stack as u64;
        let stack_size = thread.stack_size as u64;
        let value_size = mem::size_of::<VALUE>() as u64;
        let cfp_size = mem::size_of::<rb_control_frame_struct>() as u64;

        let stack_base = stack + stack_size * value_size - 1 * cfp_size;
        debug!("cfp addr: {:x}", cfp_address as usize);
        Ok(copy_address_raw(
            cfp_address as *const c_void,
            (stack_base - cfp_address) as usize,
            source_pid,
        )?)
    }
}
));

pub mod stack_trace {
    use libc::pid_t;
    use address_finder;
    use std;

    pub fn get_stack_trace_function(pid: pid_t) -> Box<Fn(u64, pid_t) -> Result<Vec<String>, Box<std::error::Error>>> {
        let version = address_finder::get_api_version(pid).unwrap();
        println!("version: {}", version);
        let stack_trace_function = match version.as_ref() {
            "2.1.1" => self::ruby_2_1_1::get_stack_trace,
            "2.1.2" => self::ruby_2_1_2::get_stack_trace,
            "2.1.3" => self::ruby_2_1_3::get_stack_trace,
            "2.1.4" => self::ruby_2_1_4::get_stack_trace,
            "2.1.5" => self::ruby_2_1_5::get_stack_trace,
            "2.1.6" => self::ruby_2_1_6::get_stack_trace,
            "2.1.7" => self::ruby_2_1_7::get_stack_trace,
            "2.1.8" => self::ruby_2_1_8::get_stack_trace,
            "2.1.9" => self::ruby_2_1_9::get_stack_trace,
            "2.1.10" => self::ruby_2_1_10::get_stack_trace,
            "2.2.0" => self::ruby_2_2_0::get_stack_trace,
            "2.2.2" => self::ruby_2_2_2::get_stack_trace,
            "2.2.3" => self::ruby_2_2_3::get_stack_trace,
            "2.2.4" => self::ruby_2_2_4::get_stack_trace,
            "2.2.5" => self::ruby_2_2_5::get_stack_trace,
            "2.2.6" => self::ruby_2_2_6::get_stack_trace,
            "2.2.7" => self::ruby_2_2_7::get_stack_trace,
            "2.2.8" => self::ruby_2_2_8::get_stack_trace,
            "2.2.9" => self::ruby_2_2_9::get_stack_trace,
            "2.3.0" => self::ruby_2_3_0::get_stack_trace,
            "2.3.1" => self::ruby_2_3_1::get_stack_trace,
            "2.3.2" => self::ruby_2_3_2::get_stack_trace,
            "2.3.3" => self::ruby_2_3_3::get_stack_trace,
            "2.3.4" => self::ruby_2_3_4::get_stack_trace,
            "2.3.5" => self::ruby_2_3_5::get_stack_trace,
            "2.3.6" => self::ruby_2_3_6::get_stack_trace,
            "2.4.0" => self::ruby_2_4_0::get_stack_trace,
            "2.4.1" => self::ruby_2_4_1::get_stack_trace,
            "2.4.2" => self::ruby_2_4_2::get_stack_trace,
            "2.4.3" => self::ruby_2_4_3::get_stack_trace,
            _ => panic!("oh no"),
        };
        Box::new(stack_trace_function)
    }


    ruby_bindings!(ruby_2_1_1);
    ruby_bindings!(ruby_2_1_2);
    ruby_bindings!(ruby_2_1_3);
    ruby_bindings!(ruby_2_1_4);
    ruby_bindings!(ruby_2_1_5);
    ruby_bindings!(ruby_2_1_6);
    ruby_bindings!(ruby_2_1_7);
    ruby_bindings!(ruby_2_1_8);
    ruby_bindings!(ruby_2_1_9);
    ruby_bindings!(ruby_2_1_10);
    ruby_bindings!(ruby_2_2_0);
    ruby_bindings!(ruby_2_2_1);
    ruby_bindings!(ruby_2_2_2);
    ruby_bindings!(ruby_2_2_3);
    ruby_bindings!(ruby_2_2_4);
    ruby_bindings!(ruby_2_2_5);
    ruby_bindings!(ruby_2_2_6);
    ruby_bindings!(ruby_2_2_7);
    ruby_bindings!(ruby_2_2_8);
    ruby_bindings!(ruby_2_2_9);
    ruby_bindings_v2!(ruby_2_3_0);
    ruby_bindings_v2!(ruby_2_3_1);
    ruby_bindings_v2!(ruby_2_3_2);
    ruby_bindings_v2!(ruby_2_3_3);
    ruby_bindings_v2!(ruby_2_3_4);
    ruby_bindings_v2!(ruby_2_3_5);
    ruby_bindings_v2!(ruby_2_3_6);
    ruby_bindings_v2!(ruby_2_4_0);
    ruby_bindings_v2!(ruby_2_4_1);
    ruby_bindings_v2!(ruby_2_4_2);
    ruby_bindings_v2!(ruby_2_4_3);
}
