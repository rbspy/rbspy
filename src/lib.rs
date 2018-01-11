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
    use elf;
    use stack_trace;

    // struct to hold everything we know about the program
    struct ProgramInfo {
        pid: pid_t,
        all_maps: Vec<MapRange>,
        ruby_map: Box<MapRange>,
        heap_map: Box<MapRange>,
        libruby_map: Box<Option<MapRange>>,
        ruby_elf: elf::File,
        libruby_elf: Option<elf::File>,
    }

    fn get_program_info(pid: pid_t) -> Result<ProgramInfo, Box<std::error::Error>> {
        let all_maps = get_proc_maps(pid);
        let ruby_map = Box::new(get_ruby_map(&all_maps)?);
        let heap_map = Box::new(get_heap_map(&all_maps)?);
        let ruby_elf = elf::File::open_path(ruby_map.pathname.clone().unwrap()).unwrap();
        let libruby_map = Box::new(libruby_map(&all_maps));
        let libruby_elf = (*libruby_map).as_ref().map(|map| {
            elf::File::open_path(map.pathname.clone().unwrap()).unwrap()
        });
        Ok(ProgramInfo {
            pid: pid,
            all_maps: all_maps,
            ruby_map: ruby_map,
            heap_map: heap_map,
            libruby_map: libruby_map,
            ruby_elf: ruby_elf,
            libruby_elf: libruby_elf,
        })
    }


    #[derive(Debug, Clone)]
    pub struct MapRange {
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
    fn elf_symbol_value(
        elf_file: &elf::File,
        symbol_name: &str,
    ) -> Result<u64, Box<std::error::Error>> {
        // TODO: maybe move this to goblin so that it works on OS X & BSD, not just linux
        let sections = &elf_file.sections;
        for s in sections {
            for sym in elf_file.get_symbols(&s).ok().ok_or("parse error")? {
                if sym.name == symbol_name {
                    debug!("symbol: {}", sym);
                    return Ok(sym.value);
                }
            }
        }
        None.ok_or("No symbol found")?
    }

    pub fn get_api_version(pid: pid_t) -> Result<String, Box<std::error::Error>> {
        let addr = get_api_address(pid)?;
        debug!("api addr: {:x}", addr);
        let x: [c_char; 15] = copy_struct(addr, pid)?;
        debug!("api struct: {:?}", x);
        Ok(unsafe {
            std::ffi::CStr::from_ptr(x.as_ptr() as *mut c_char)
                .to_str()
                .unwrap()
                .to_owned()
        })
    }



    #[cfg(target_os = "linux")]
    fn get_api_address(pid: pid_t) -> Result<u64, Box<std::error::Error>> {
        // TODO: implement OS X version of this
        let proginfo = &get_program_info(pid)?;
        let ruby_version_symbol = "ruby_version";
        let symbol_addr =
            get_symbol_addr(&proginfo.ruby_map, &proginfo.ruby_elf, ruby_version_symbol);
        if symbol_addr.is_ok() {
            Ok(symbol_addr.unwrap())
        } else {
            Ok(get_symbol_addr(
                (*proginfo.libruby_map).as_ref().unwrap(),
                proginfo.libruby_elf.as_ref().unwrap(),
                ruby_version_symbol,
            )?)
        }
    }

    #[cfg(target_os = "linux")]
    pub fn get_bss_section(elf_file: &elf::File) -> Option<elf::types::SectionHeader> {
        for s in &elf_file.sections {
            match s.shdr.name.as_ref() {
                ".bss" => {
                    return Some(s.shdr.clone());
                }
                _ => {}
            }
        }
        None
    }

    fn get_thread_address_alt(proginfo: &ProgramInfo, version: &str) -> Result<u64, Box<std::error::Error>> {
        let map = (*proginfo.libruby_map).as_ref().unwrap(); // TODO: don't unwrap
        let bss_section = get_bss_section(proginfo.libruby_elf.as_ref().unwrap()).unwrap();
        let load_header = elf_load_header(proginfo.libruby_elf.as_ref().unwrap());
        debug!("bss_section header: {:?}", bss_section);
        let read_addr = map.range_start + bss_section.addr - load_header.vaddr;

        debug!("read_addr: {:x}", read_addr);
        let mut data = copy_address_raw(
            read_addr as *const c_void,
            bss_section.size as usize,
            proginfo.pid,
        )?;
        debug!("successfully read data");
        let slice: &[u64] = unsafe {
            std::slice::from_raw_parts(
                data.as_mut_ptr() as *mut u64,
                data.capacity() as usize / std::mem::size_of::<u64>() as usize,
            )
        };

        let is_maybe_thread = stack_trace::is_maybe_thread_function(version);

        let i = slice
            .iter()
            .position({
                |&x| is_maybe_thread(x, proginfo.pid, &proginfo.heap_map, &proginfo.all_maps)
            })
            .ok_or("didn't find a current thread")?;
        Ok((i as u64) * (std::mem::size_of::<u64>() as u64) + read_addr)
    }


    pub fn in_memory_maps(x: u64, maps: &Vec<MapRange>) -> bool {
        maps.iter().any({
            |map| is_heap_addr(x, map)
        })
    }

    pub fn is_heap_addr(x: u64, map: &MapRange) -> bool {
        x >= map.range_start && x <= map.range_end
    }

    #[cfg(target_os = "linux")]
    fn libruby_map(maps: &Vec<MapRange>) -> Option<MapRange> {
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
    fn get_heap_map(maps: &Vec<MapRange>) -> Result<MapRange, Box<std::error::Error>> {
        let map = maps.iter()
            .find(|ref m| {
                m.pathname != None && (m.pathname.clone().unwrap() == "[heap]")
            })
            .ok_or("couldn't find heap map")?;
        Ok(map.clone())
    }

    #[cfg(target_os = "linux")]
    fn get_ruby_map(maps: &Vec<MapRange>) -> Result<MapRange, Box<std::error::Error>> {
        let map = maps.iter()
            .find(|ref m| {
                m.pathname != None && m.pathname.clone().unwrap().contains("bin/ruby") &&
                    &m.flags == "r-xp"
            })
            .ok_or("couldn't find ruby map")?;
        debug!("map: {:?}", map);
        Ok(map.clone())
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
    pub fn current_thread_address_location(
        pid: pid_t,
        version: &str,
    ) -> Result<u64, Box<std::error::Error>> {
        let proginfo = &get_program_info(pid)?;
        let try_1 = current_thread_address_location_default(proginfo, version);
        if try_1.is_ok() {
            Ok(try_1.unwrap())
        } else {
            debug!("Trying to find address location another way");
            Ok(get_thread_address_alt(proginfo, version)?)
        }
    }

    #[cfg(target_os = "linux")]
    fn current_thread_address_location_default(
        proginfo: &ProgramInfo,
        version: &str,
    ) -> Result<u64, Box<std::error::Error>> {
        // TODO: comment this somewhere
        if version == "2.5.0" { // TODO: make this more robust
            Ok(get_symbol_addr(
                &proginfo.ruby_map,
                &proginfo.ruby_elf,
                "ruby_current_execution_context_ptr",
            )?)
        } else {
            Ok(get_symbol_addr(
                &proginfo.ruby_map,
                &proginfo.ruby_elf,
                "ruby_current_thread",
            )?)
        }
    }

    fn get_symbol_addr(
        map: &MapRange,
        elf_file: &elf::File,
        symbol_name: &str,
    ) -> Result<u64, Box<std::error::Error>> {
        let symbol_addr = elf_symbol_value(elf_file, symbol_name)?;
        let load_header = elf_load_header(elf_file);
        debug!("load header: {}", load_header);
        Ok(map.range_start + symbol_addr - load_header.vaddr)
    }

    fn elf_load_header(elf_file: &elf::File) -> elf::types::ProgramHeader {
        elf_file
            .phdrs
            .iter()
            .find(|ref ph| {
                ph.progtype == elf::types::PT_LOAD && (ph.flags.0 & elf::types::PF_X.0) != 0
            })
            .unwrap()
            .clone()
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

macro_rules! ruby_bindings_v_1_9_x(
($ruby_version:ident) => (
pub mod $ruby_version {
    use copy::*;
    use std;
    use bindings::$ruby_version::*;
    use libc::*;
    use std::ffi::{OsString, CStr};
    use std::mem;
    use std::os::unix::prelude::*;

    get_stack_trace_2_0_0!();
    get_ruby_string_2_0_0!();
    get_label_and_path_1_9_0!();
    get_cfps_2_0_0!();
    is_maybe_thread_1_9_0!();
}
));

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
    // (* const rb_thread_struct *(ruby_current_thread_address_location))->cfp
    //
    // `get_ruby_string` is doing ((Struct RString *) address) and then
    // trying one of two ways to get the actual Ruby string out depending
    // on how it's stored
    get_stack_trace_2_0_0!();
    get_ruby_string_2_0_0!();
    get_label_and_path_2_0_0!();
    get_cfps_2_0_0!();
    is_maybe_thread_1_9_0!();
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
    is_maybe_thread_1_9_0!();
}
));

macro_rules! ruby_bindings_v2_5_x(
($ruby_version:ident) => (
mod $ruby_version {
    use copy::*;
    use std;
    use bindings::$ruby_version::*;
    use libc::*;
    use std::ffi::{OsString, CStr};
    use std::mem;
    use std::os::unix::prelude::*;

    get_stack_trace_2_5_0!();
    get_ruby_string_2_0_0!();
    get_label_and_path_2_5_0!();
    get_cfps_2_5_0!();
    is_maybe_thread_2_5_0!();
    get_ruby_string_array_2_0_0!();
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
        let thread: rb_thread_struct = copy_struct(current_thread_addr, source_pid)?;
        debug!("{:?}", thread);
        let mut trace = Vec::new();
        let mut cfps = get_cfps(&thread, source_pid)?;
        let slice: &[rb_control_frame_t] = unsafe {
            std::slice::from_raw_parts(
                cfps.as_mut_ptr() as *mut rb_control_frame_t,
                cfps.capacity() as usize / mem::size_of::<rb_control_frame_t>() as usize,
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
                Err(x) => {
                    warn!("failed to get label and path, ignoring, {}", x);
                }
            }
        }
        Ok(trace)
    }
));



macro_rules! get_stack_trace_2_5_0(
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
        let thread: rb_execution_context_struct = copy_struct(current_thread_addr, source_pid)?;
        debug!("{:?}", thread);
        let mut trace = Vec::new();
        let mut cfps = get_cfps(&thread, source_pid)?;
        let slice: &[rb_control_frame_t] = unsafe {
            std::slice::from_raw_parts(
                cfps.as_mut_ptr() as *mut rb_control_frame_t,
                cfps.capacity() as usize / mem::size_of::<rb_control_frame_t>() as usize,
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
                Err(x) => {
                    warn!("failed to get label and path, ignoring, {}", x);
                }
            }
        }
        Ok(trace)
    }
));

macro_rules! is_maybe_thread_1_9_0(
() => (
    use address_finder::*;
    pub fn is_maybe_thread(x: u64, pid: pid_t, heap_map: &MapRange, all_maps: &Vec<MapRange>) -> bool {
        if !is_heap_addr(x, heap_map) {
            return false;
        }
        let thread = copy_struct(x, pid);
        if !thread.is_ok() {
            return false;
        }
        // TODO: stop hardcoding ruby 2.3.1 here
        let thread: rb_thread_struct = thread.unwrap();
        debug!("thread addr: {:x}, thread: {:?}", x, thread);

        debug!("{}", is_heap_addr(thread.vm as u64, heap_map));
        debug!("{}", in_memory_maps(thread.cfp as u64, all_maps));
        debug!("{}", in_memory_maps(thread.stack as u64, all_maps));
        debug!("{}", in_memory_maps(thread.self_ as u64, all_maps));
        if !( is_heap_addr(thread.vm as u64, heap_map) &&
              in_memory_maps(thread.cfp as u64, all_maps) &&
              in_memory_maps(thread.stack as u64, all_maps) &&
              in_memory_maps(thread.self_ as u64, all_maps) &&
              thread.stack_size < 3000000 && thread.state >= 0)
        {
            return false;
        }
        let stack = thread.stack as u64;
        let stack_size = thread.stack_size as u64;
        let value_size = mem::size_of::<VALUE>() as u64;
        let cfp_size = mem::size_of::<rb_control_frame_t>() as u64;

        let stack_base = stack + stack_size * value_size - 1 * cfp_size;
        if stack_base < thread.cfp as u64 {
            return false;
        }

        return true;
    }
));

macro_rules! is_maybe_thread_2_5_0(
() => (
    use address_finder::*;
    pub fn is_maybe_thread(x: u64, pid: pid_t, heap_map: &MapRange, all_maps: &Vec<MapRange>) -> bool {
        if !is_heap_addr(x, heap_map) {
            return false;
        }
        let thread = copy_struct(x, pid);
        if !thread.is_ok() {
            return false;
        }
        let thread: rb_execution_context_struct = thread.unwrap();
        debug!("thread addr: {:x}, thread: {:?}", x, thread);

        debug!(
            "matches: {} {} {}",
            in_memory_maps(thread.tag as u64, all_maps),
            in_memory_maps(thread.cfp as u64, all_maps),
            in_memory_maps(thread.vm_stack as u64, all_maps)
        );

        if !( in_memory_maps(thread.tag as u64, all_maps) &&
              in_memory_maps(thread.cfp as u64, all_maps) &&
              in_memory_maps(thread.vm_stack as u64, all_maps) &&
              thread.vm_stack_size < 3000000)
        {
            return false;
        }
        let stack = thread.vm_stack as u64;
        let vm_stack_size = thread.vm_stack_size as u64;
        let value_size = mem::size_of::<VALUE>() as u64;
        let cfp_size = mem::size_of::<rb_control_frame_t>() as u64;

        let stack_base = stack + vm_stack_size * value_size - 1 * cfp_size;
        if stack_base < thread.cfp as u64 { // TODO: bound this, less than 5MB or something
            return false;
        }

        return true;
    }
));

macro_rules! get_ruby_string_array_2_0_0(
() => (
    fn get_ruby_string_array(addr: u64, string_class: u64, source_pid: pid_t) -> Result<OsString, Box<std::error::Error>> {
        // todo: we're doing an extra copy here for no reason
        let rstring: RString = copy_struct(addr, source_pid)?;
        if rstring.basic.klass as u64 == string_class {
            return get_ruby_string(addr, source_pid);
        }
        // otherwise it's an RArray
        let rarray: RArray = copy_struct(addr, source_pid)?;
        debug!("blah: {}, array: {:?}", addr, unsafe { rarray.as_.ary });
        // TODO: this assumes that the array contents are stored inline and not on the heap
        // I think this will always be true but we should check instead
        // the reason I am not checking is that I don't know how to check yet
        let addr: u64 = unsafe { rarray.as_.ary[1] }; // 1 means get the absolute path, not the relative path
        get_ruby_string(addr, source_pid)
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

macro_rules! get_label_and_path_1_9_0(
() => (
    fn get_label_and_path(
        cfp: &rb_control_frame_t,
        source_pid: pid_t,
    ) -> Result<(OsString, OsString), Box<std::error::Error>> {
        debug!("get_label_and_path {:?}", cfp);
        let iseq_address = cfp.iseq as u64;
        debug!("iseq_address: {:?}", iseq_address);
        let iseq_struct: rb_iseq_struct = copy_struct(iseq_address, source_pid)?;
        let label: OsString = get_ruby_string(iseq_struct.name as u64, source_pid)?;
        let path: OsString = get_ruby_string(iseq_struct.filename as u64, source_pid)?;
        Ok((label, path))
    }
));

macro_rules! get_label_and_path_2_0_0(
() => (
    fn get_label_and_path(
        cfp: &rb_control_frame_t,
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

macro_rules! get_label_and_path_2_5_0(
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
        let rstring: RString = copy_struct(location.label as u64, source_pid)?;
        let string_class = rstring.basic.klass as u64;
        let label: OsString = get_ruby_string(location.label as u64, source_pid)?;
        let path: OsString = get_ruby_string_array(location.pathobj as u64, string_class, source_pid)?;
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
    fn get_cfps(thread: &rb_thread_struct, source_pid: pid_t) -> Result<Vec<u8>, Box<std::error::Error>> {
        let cfp_address = thread.cfp as u64;

        let stack = thread.stack as u64;
        let stack_size = thread.stack_size as u64;
        let value_size = mem::size_of::<VALUE>() as u64;
        let cfp_size = mem::size_of::<rb_control_frame_t>() as u64;

        let stack_base = stack + stack_size * value_size - 1 * cfp_size;
        debug!("cfp addr: {:x}", cfp_address as usize);
        Ok(copy_address_raw(
            cfp_address as *const c_void,
            (stack_base - cfp_address) as usize,
            source_pid,
        )?)
    }
));

macro_rules! get_cfps_2_5_0(
() => (
    fn get_cfps(thread: &rb_execution_context_struct, source_pid: pid_t) -> Result<Vec<u8>, Box<std::error::Error>> {
        let cfp_address = thread.cfp as u64;

        let stack = thread.vm_stack as u64;
        let vm_stack_size = thread.vm_stack_size as u64;
        let value_size = mem::size_of::<VALUE>() as u64;
        let cfp_size = mem::size_of::<rb_control_frame_t>() as u64;

        let stack_base = stack + vm_stack_size * value_size - 1 * cfp_size;
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

    pub fn is_maybe_thread_function(
        version: &str,
    ) -> Box<
        Fn(u64,
           pid_t,
           &address_finder::MapRange,
           &Vec<address_finder::MapRange>)
           -> bool,
    > {
        let function = match version.as_ref() {
            "1.9.1" => self::ruby_1_9_1_0::is_maybe_thread,
            "1.9.2" => self::ruby_1_9_2_0::is_maybe_thread,
            "1.9.3" => self::ruby_1_9_3_0::is_maybe_thread,
            "2.0.0" => self::ruby_2_0_0_0::is_maybe_thread,
            "2.1.0" => self::ruby_2_1_0::is_maybe_thread,
            "2.1.2" => self::ruby_2_1_2::is_maybe_thread,
            "2.1.3" => self::ruby_2_1_3::is_maybe_thread,
            "2.1.4" => self::ruby_2_1_4::is_maybe_thread,
            "2.1.5" => self::ruby_2_1_5::is_maybe_thread,
            "2.1.6" => self::ruby_2_1_6::is_maybe_thread,
            "2.1.7" => self::ruby_2_1_7::is_maybe_thread,
            "2.1.8" => self::ruby_2_1_8::is_maybe_thread,
            "2.1.9" => self::ruby_2_1_9::is_maybe_thread,
            "2.1.10" => self::ruby_2_1_10::is_maybe_thread,
            "2.2.0" => self::ruby_2_2_0::is_maybe_thread,
            "2.2.1" => self::ruby_2_2_1::is_maybe_thread,
            "2.2.2" => self::ruby_2_2_2::is_maybe_thread,
            "2.2.3" => self::ruby_2_2_3::is_maybe_thread,
            "2.2.4" => self::ruby_2_2_4::is_maybe_thread,
            "2.2.5" => self::ruby_2_2_5::is_maybe_thread,
            "2.2.6" => self::ruby_2_2_6::is_maybe_thread,
            "2.2.7" => self::ruby_2_2_7::is_maybe_thread,
            "2.2.8" => self::ruby_2_2_8::is_maybe_thread,
            "2.2.9" => self::ruby_2_2_9::is_maybe_thread,
            "2.3.0" => self::ruby_2_3_0::is_maybe_thread,
            "2.3.1" => self::ruby_2_3_1::is_maybe_thread,
            "2.3.2" => self::ruby_2_3_2::is_maybe_thread,
            "2.3.3" => self::ruby_2_3_3::is_maybe_thread,
            "2.3.4" => self::ruby_2_3_4::is_maybe_thread,
            "2.3.5" => self::ruby_2_3_5::is_maybe_thread,
            "2.3.6" => self::ruby_2_3_6::is_maybe_thread,
            "2.4.0" => self::ruby_2_4_0::is_maybe_thread,
            "2.4.1" => self::ruby_2_4_1::is_maybe_thread,
            "2.4.2" => self::ruby_2_4_2::is_maybe_thread,
            "2.4.3" => self::ruby_2_4_3::is_maybe_thread,
            "2.5.0" => self::ruby_2_5_0_rc1::is_maybe_thread,
            _ => panic!("oh no"),
        };
        Box::new(function)
    }



    pub fn get_stack_trace_function(
        version: &str,
    ) -> Box<Fn(u64, pid_t) -> Result<Vec<String>, Box<std::error::Error>>> {
        let stack_trace_function = match version {
            "1.9.1" => self::ruby_1_9_1_0::get_stack_trace,
            "1.9.2" => self::ruby_1_9_2_0::get_stack_trace,
            "1.9.3" => self::ruby_1_9_3_0::get_stack_trace,
            "2.0.0" => self::ruby_2_0_0_0::get_stack_trace,
            "2.1.0" => self::ruby_2_1_0::get_stack_trace,
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
            "2.2.1" => self::ruby_2_2_1::get_stack_trace,
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
            "2.5.0" => self::ruby_2_5_0_rc1::get_stack_trace,
            _ => panic!("oh no"),
        };
        Box::new(stack_trace_function)
    }


    ruby_bindings_v_1_9_x!(ruby_1_9_1_0);
    ruby_bindings_v_1_9_x!(ruby_1_9_2_0);
    ruby_bindings_v_1_9_x!(ruby_1_9_3_0);
    ruby_bindings!(ruby_2_0_0_0);
    ruby_bindings!(ruby_2_1_0);
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
    ruby_bindings_v2_5_x!(ruby_2_5_0_rc1);
}
