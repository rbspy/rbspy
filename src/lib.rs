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
}

pub mod address_finder {
    use copy::*;
    use libc::*;
    use std::fs::File;
    use std::io::Read;
    use std;
    use bindings;
    use elf;
    use read_process_memory::*;
    use std::mem;

    fn is_maybe_thread(x: u64, pid: pid_t, heap_map: &MapRange, all_maps: &Vec<MapRange>) -> bool {
        if !is_heap_addr(x, heap_map) {
            return false;
        }
        let thread = copy_struct(x, pid);
        if !thread.is_ok() {
            return false;
        }
        // TODO: stop hardcoding ruby 2.3.1 here
        let thread: bindings::ruby_2_3_1::rb_thread_t = thread.unwrap();
        debug!("thread addr: {:x}, thread: {:?}", x, thread);

        debug!("{}", is_heap_addr(thread.vmlt_node.next as u64, heap_map));
        debug!("{}", in_memory_maps(thread.vmlt_node.prev as u64, all_maps));
        debug!("{}", is_heap_addr(thread.vm as u64, heap_map));
        debug!("{}", in_memory_maps(thread.cfp as u64, all_maps));
        debug!("{}", in_memory_maps(thread.stack as u64, all_maps));
        debug!("{}", in_memory_maps(thread.self_ as u64, all_maps));
        if !(is_heap_addr(thread.vmlt_node.next as u64, heap_map) &&
                 in_memory_maps(thread.vmlt_node.prev as u64, all_maps) &&
                 is_heap_addr(thread.vm as u64, heap_map) &&
                 in_memory_maps(thread.cfp as u64, all_maps) &&
                 in_memory_maps(thread.stack as u64, all_maps) &&
                 in_memory_maps(thread.self_ as u64, all_maps) &&
                 thread.stack_size < 3000000 && thread.state >= 0)
        {
            return false;
        }
        let stack = thread.stack as u64;
        let stack_size = thread.stack_size as u64;
        let value_size = mem::size_of::<bindings::ruby_2_3_1::VALUE>() as u64;
        let cfp_size = mem::size_of::<bindings::ruby_2_3_1::rb_control_frame_struct>() as u64;

        let stack_base = stack + stack_size * value_size - 1 * cfp_size;
        if stack_base < thread.cfp as u64 {
            return false;
        }

        return true;
    }

    #[derive(Debug, Clone)]
    struct MapRange {
        range_start: u64,
        range_end: u64,
        offset: u64,
        dev: String,
        flags: String,
        inode: u64,
        pathname: Option<String>,
    }

    fn get_proc_maps(pid: pid_t) -> Vec<MapRange> {
        // Parses /proc/PID/maps into a Vec<MapRange>
        // TODO: factor this out into a crate and make it work on Mac too
        let maps_file = format!("/proc/{}/maps", pid);
        let mut file = File::open(maps_file).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents);
        let mut vec: Vec<MapRange> = Vec::new();
        for line in contents.split("\n") {
            let mut split = line.split_whitespace();
            let range = split.next();
            if range == None {
                break;
            }
            let mut range_split = range.unwrap().split("-");
            let range_start = range_split.next().unwrap();
            let range_end = range_split.next().unwrap();
            let flags = split.next().unwrap();
            let offset = split.next().unwrap();
            let dev = split.next().unwrap();
            let inode = split.next().unwrap();

            vec.push(MapRange {
                range_start: u64::from_str_radix(range_start, 16).unwrap(),
                range_end: u64::from_str_radix(range_end, 16).unwrap(),
                offset: u64::from_str_radix(offset, 16).unwrap(),
                dev: dev.to_string(),
                flags: flags.to_string(),
                inode: u64::from_str_radix(inode, 10).unwrap(),
                pathname: split.next().map(|x| x.to_string()),
            });
        }
        vec
    }

    #[cfg(target_os = "linux")]
    fn elf_symbol_value(file_name: &str, symbol_name: &str) -> Result<u64, Box<std::error::Error>> {
        // TODO: maybe move this to goblin so that it works on OS X & BSD, not just linux
        let file = elf::File::open_path(file_name).ok().ok_or("parse error")?;
        let sections = &file.sections;
        for s in sections {
            for sym in file.get_symbols(&s).ok().ok_or("parse error")? {
                if sym.name == symbol_name {
                    return Ok(sym.value);
                }
            }
        }
        None.ok_or("No symbol found")?
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

    #[cfg(target_os = "linux")]
    fn libruby_map(pid: pid_t) -> Option<MapRange> {
        let maps = get_proc_maps(pid);
        maps.iter()
            .find(|ref m| {
                m.pathname != None && m.pathname.clone().unwrap().contains("libruby") &&
                    &m.flags == "r-xp"
            })
            .map({
                |x| x.clone()
            })
    }

    #[cfg(target_os = "linux")]
    fn get_api_address(pid: pid_t) -> Result<u64, Box<std::error::Error>> {
        // TODO: implement OS X version of this
        let proc_exe = &format!("/proc/{}/exe", pid);
        let ruby_version_symbol = "ruby_version";
        let symbol_value = elf_symbol_value(proc_exe, ruby_version_symbol);
        if symbol_value.is_ok() {
            Ok(symbol_value.unwrap() + get_maps_address(pid)?)
        } else {
            let map = libruby_map(pid).ok_or("couldn't find libruby map")?;
            Ok(
                elf_symbol_value(&map.pathname.unwrap(), ruby_version_symbol)? + map.range_start,
            )
        }
    }

    #[cfg(target_os = "linux")]
    pub fn get_bss_section(filename: &str) -> Option<Box<elf::Section>> {
        let file = elf::File::open_path(filename).unwrap();
        for s in file.sections {
            match s.shdr.name.as_ref() {
                ".bss" => {
                    return Some(Box::new(s));
                }
                _ => {}
            }
        }
        None
    }

    pub fn get_thread_address_alt(pid: pid_t) -> Result<u64, Box<std::error::Error>> {
        let map = &libruby_map(pid).ok_or("couldn't find libruby map")?;
        let bss_section = get_bss_section(&map.pathname.clone().unwrap()).unwrap();
        let all_maps = &get_proc_maps(pid);
        debug!("bss_section header: {:?}", bss_section.shdr);
        let read_addr = map.range_start + bss_section.shdr.addr;
        let heap_map = &get_heap_map(pid).ok_or("no heap map")?;
        debug!("read_addr: {:x}", read_addr);
        let mut data = copy_address_raw(
            read_addr as *const c_void,
            bss_section.shdr.size as usize,
            pid,
        )?;
        debug!("successfully read data");
        let slice: &[u64] = unsafe {
            std::slice::from_raw_parts(
                data.as_mut_ptr() as *mut u64,
                data.capacity() as usize / std::mem::size_of::<u64>() as usize,
            )
        };

        let i = slice
            .iter()
            .position({
                |&x| is_maybe_thread(x, pid, heap_map, all_maps)
            })
            .ok_or("didn't find a current thread")?;
        Ok((i as u64) * (std::mem::size_of::<u64>() as u64) + read_addr)
    }

    #[cfg(target_os = "linux")]
    fn get_nm_address(pid: pid_t) -> Result<u64, Box<std::error::Error>> {
        Ok(elf_symbol_value(
            &format!("/proc/{}/exe", pid),
            "ruby_current_thread",
        )?)
    }

    fn in_memory_maps(x: u64, maps: &Vec<MapRange>) -> bool {
        maps.iter().any({
            |map| is_heap_addr(x, map)
        })
    }

    fn is_heap_addr(x: u64, map: &MapRange) -> bool {
        x >= map.range_start && x <= map.range_end
    }

    #[cfg(target_os = "linux")]
    fn get_heap_map(pid: pid_t) -> Option<MapRange> {
        let maps = get_proc_maps(pid);
        maps.iter()
            .find(|ref m| {
                m.pathname != None && (m.pathname.clone().unwrap() == "[heap]")
            })
            .map({
                |x| x.clone()
            })
    }

    #[cfg(target_os = "linux")]
    fn get_maps_address(pid: pid_t) -> Result<u64, Box<std::error::Error>> {
        let maps = get_proc_maps(pid);
        let map = maps.iter()
            .find(|ref m| {
                m.pathname != None && m.pathname.clone().unwrap().contains("bin/ruby") &&
                    &m.flags == "r-xp"
            })
            .ok_or("oh no")?;
        debug!("map: {:?}", map);
        Ok(map.range_start)
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
        let try_1 = current_thread_address_location_default(pid);
        if try_1.is_ok() {
            Ok(try_1.unwrap())
        } else {
            debug!("Trying to find address location another way");
            Ok(get_thread_address_alt(pid)?)
        }
    }

    #[cfg(target_os = "linux")]
    pub fn current_thread_address_location_default(
        pid: pid_t,
    ) -> Result<u64, Box<std::error::Error>> {
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
    pub fn copy_address_raw(
        addr: *const c_void,
        length: usize,
        source_pid: pid_t,
    ) -> Result<Vec<u8>, Box<std::error::Error>> {
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
    use copy::*;
    use std;
    use bindings::$ruby_version::*;
    use libc::*;
    use std::ffi::{OsString, CStr};
    use std::mem;
    use std::os::unix::prelude::*;


    // These 4 functions are the
    // core of how the program works. They're essentially a straight port of
    // this gdb script:
    // https://gist.github.com/csfrancis/11376304/raw/7a0450d11e64e3bb7c982b7ad2778f3603188c0f/gdb_ruby_backtrace.py
    // except without using gdb!!
    //
    // `get_cfps` corresponds to
    // (* const rb_thread_t *(ruby_current_thread_address_location))->cfp
    //
    // `get_ruby_string` is doing ((Struct RString *) address) and then
    // trying one of two ways to get the actual Ruby string out depending
    // on how it's stored
    get_stack_trace_2_0_0!();
    get_ruby_string_2_0_0!();
    get_label_and_path_2_0_0!();
    get_cfps_2_0_0!();
}
));

macro_rules! ruby_bindings_v2(
($ruby_version:ident) => (
mod $ruby_version {
    use copy::*;
    use std;
    use bindings::$ruby_version::*;
    use libc::*;
    use std::ffi::{OsString, CStr};
    use std::mem;
    use std::os::unix::prelude::*;

    get_stack_trace_2_0_0!();
    get_ruby_string_2_0_0!();
    get_label_and_path_2_3_0!();
    get_cfps_2_0_0!();
}
));

macro_rules! get_stack_trace_2_0_0(
() => (
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
));

macro_rules! get_ruby_string_2_0_0(
() => (
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
));

macro_rules! get_label_and_path_2_0_0(
() => (
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
        Ok((label, path))
    }
));

macro_rules! get_label_and_path_2_3_0(
() => (
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
));

macro_rules! get_cfps_2_0_0(
() => (
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
));


pub mod stack_trace {
    use libc::pid_t;
    use address_finder;
    use std;

    pub fn get_stack_trace_function(
        pid: pid_t,
    ) -> Box<Fn(u64, pid_t) -> Result<Vec<String>, Box<std::error::Error>>> {
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
