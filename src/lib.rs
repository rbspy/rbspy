#![cfg_attr(rustc_nightly, feature(test))]

#[macro_use]
extern crate log;

extern crate elf;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate libc;
extern crate read_process_memory;
#[cfg(target_os = "macos")]
extern crate regex;
extern crate ruby_bindings as bindings;

pub mod user_interface {
    use std;
    use stack_trace;

    pub fn print_stack_trace(output: &mut std::io::Write, trace: &[stack_trace::FunctionCall]) {
        for x in trace.iter().rev() {
            write!(output, "{}", x);
            write!(output, ";");
        }
        writeln!(output, " {}", 1);
    }
}

pub mod address_finder {
    use copy::*;
    use libc::*;
    use std::fs::File;
    use std::io::Read;
    use failure::Error;
    use std;
    use elf;
    use stack_trace;

    #[derive(Fail, Debug)]
    pub enum AddressFinderError {
        #[fail(display = "Failed to open ELF file: {}", _0)]
        ELFFileError(String),
        #[fail(display = "Couldn't read /proc/{}/maps", _0)]
        ProcMapsError(pid_t, #[cause] std::io::Error),
        #[fail(display = "Ruby map not found for PID {}. Perhaps that process isn't a Ruby program?", _0)]
        RubyMapNotFound(pid_t),
        #[fail(display = "Heap map not found for PID {}", _0)]
        HeapMapNotFound(pid_t),
        #[fail(display = "Ruby version not found in either Ruby or libruby")]
        RubyVersionMissing,
        #[fail(display = "Couldn't find address of current thread")]
        CurrentThreadNotFound,
}

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

    fn get_program_info(pid: pid_t) -> Result<ProgramInfo, AddressFinderError> {
        let all_maps = get_proc_maps(pid)?;
        let ruby_map =
            Box::new(get_ruby_map(&all_maps).ok_or(AddressFinderError::RubyMapNotFound(pid))?);
        let heap_map =
            Box::new(get_heap_map(&all_maps).ok_or(AddressFinderError::HeapMapNotFound(pid))?);
        let ruby_path = &ruby_map
            .pathname
            .clone()
            .expect("ruby map's pathname shouldn't be None");
        let ruby_elf = elf::File::open_path(ruby_path)
            .map_err(|_| AddressFinderError::ELFFileError(ruby_path.to_string()))?;
        let libruby_map = Box::new(libruby_map(&all_maps));
        let libruby_elf = match *libruby_map {
            Some(ref map) => {
                let path = &map.pathname
                    .clone()
                    .expect("libruby map's pathname shouldn't be None");
                Some(elf::File::open_path(path)
                    .map_err(|_| AddressFinderError::ELFFileError(path.to_string()))?)
            }
            _ => None,
        };
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

    fn get_proc_maps(pid: pid_t) -> Result<Vec<MapRange>, AddressFinderError> {
        // Parses /proc/PID/maps into a Vec<MapRange>
        // TODO: factor this out into a crate and make it work on Mac too
        let maps_file = format!("/proc/{}/maps", pid);
        let mut file =
            File::open(maps_file).map_err(|x| AddressFinderError::ProcMapsError(pid, x))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|x| AddressFinderError::ProcMapsError(pid, x))?;
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
        Ok(vec)
    }

    #[cfg(target_os = "linux")]
    fn elf_symbol_value(elf_file: &elf::File, symbol_name: &str) -> Option<u64> {
        // TODO: maybe move this to goblin so that it works on OS X & BSD, not just linux
        let sections = &elf_file.sections;
        for s in sections {
            for sym in elf_file
                .get_symbols(&s)
                .expect("Failed to get symbols from section")
            {
                if sym.name == symbol_name {
                    debug!("symbol: {}", sym);
                    return Some(sym.value);
                }
            }
        }
        None
    }

    pub fn get_api_version(pid: pid_t) -> Result<String, Error> {
        let addr = get_api_address(pid)?;
        debug!("api addr: {:x}", addr);
        let x: [c_char; 15] = copy_struct(addr, pid)?;
        debug!("api struct: {:?}", x);
        Ok(unsafe {
            std::ffi::CStr::from_ptr(x.as_ptr() as *mut c_char)
                .to_str()?
                .to_owned()
        })
    }

    #[cfg(target_os = "linux")]
    fn get_api_address(pid: pid_t) -> Result<u64, AddressFinderError> {
        // TODO: implement OS X version of this
        let proginfo = &get_program_info(pid)?;
        let ruby_version_symbol = "ruby_version";
        let symbol_addr =
            get_symbol_addr(&proginfo.ruby_map, &proginfo.ruby_elf, ruby_version_symbol);
        match symbol_addr {
            Some(addr) => Ok(addr),
            _ => {
                get_symbol_addr(
                    // if we have a ruby map but `ruby_version` isn't in it, we expect there to be
                    // a libruby map. If that's not true, that's a bug.
                    (*proginfo.libruby_map)
                        .as_ref()
                        .expect("Missing libruby map. Please report this!"),
                    proginfo
                        .libruby_elf
                        .as_ref()
                        .expect("Missing libruby ELF. Please report this!"),
                    ruby_version_symbol,
                ).ok_or(AddressFinderError::RubyVersionMissing)
            }
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

    fn get_thread_address_alt(proginfo: &ProgramInfo, version: &str) -> Result<u64, Error> {
        let map = (*proginfo.libruby_map).as_ref().expect(
            "No libruby map: symbols are stripped so we expected to have one. Please report this!",
        );
        let libruby_elf = proginfo.libruby_elf.as_ref().expect(
            "No libruby elf: symbols are stripped so we expected to have one. Please report this!",
        );
        let bss_section = get_bss_section(libruby_elf).expect(
            "No BSS section (every Ruby ELF file should have a BSS section?). Please report this!",
        );
        let load_header = elf_load_header(libruby_elf);
        debug!("bss_section header: {:?}", bss_section);
        let read_addr = map.range_start + bss_section.addr - load_header.vaddr;

        debug!("read_addr: {:x}", read_addr);
        let mut data =
            copy_address_raw(read_addr as usize, bss_section.size as usize, proginfo.pid)?;
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
            .ok_or(AddressFinderError::CurrentThreadNotFound)?;
        Ok((i as u64) * (std::mem::size_of::<u64>() as u64) + read_addr)
    }

    pub fn in_memory_maps(x: u64, maps: &Vec<MapRange>) -> bool {
        maps.iter().any({ |map| is_heap_addr(x, map) })
    }

    pub fn is_heap_addr(x: u64, map: &MapRange) -> bool {
        x >= map.range_start && x <= map.range_end
    }

    #[cfg(target_os = "linux")]
    fn libruby_map(maps: &Vec<MapRange>) -> Option<MapRange> {
        maps.iter()
            .find(|ref m| {
                if let Some(ref pathname) = m.pathname {
                    pathname.contains("libruby") && &m.flags == "r-xp"
                } else {
                    false
                }
            })
            .map({ |x| x.clone() })
    }

    #[cfg(target_os = "linux")]
    fn get_heap_map(maps: &Vec<MapRange>) -> Option<MapRange> {
        maps.iter()
            .find(|ref m| {
                if let Some(ref pathname) = m.pathname {
                    return pathname == "[heap]";
                } else {
                    return false;
                }
            })
            .map({ |x| x.clone() })
    }

    #[cfg(target_os = "linux")]
    fn get_ruby_map(maps: &Vec<MapRange>) -> Option<MapRange> {
        maps.iter()
            .find(|ref m| {
                if let Some(ref pathname) = m.pathname {
                    pathname.contains("bin/ruby") && &m.flags == "r-xp"
                } else {
                    false
                }
            })
            .map(|x| x.clone())
    }

    #[cfg(target_os = "macos")]
    fn get_maps_address(pid: pid_t) -> u64 {
        let vmmap_command = Command::new("vmmap")
            .arg(format!("{}", pid))
            .stdout(Stdio::piped())
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .expect(format!("failed to execute process: {}", e));
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
        let line = lines
            .first()
            .expect("No `__TEXT` line found for `bin/ruby` in vmmap output");

        let re = Regex::new(r"([0-9a-f]+)").unwrap();
        let cap = re.captures(&line).unwrap();
        let address_str = cap.at(1).unwrap();
        let addr = u64::from_str_radix(address_str, 16).unwrap();
        debug!("get_maps_address: {:x}", addr);
        addr
    }

    #[cfg(target_os = "linux")]
    pub fn current_thread_address_location(pid: pid_t, version: &str) -> Result<u64, Error> {
        let proginfo = &get_program_info(pid)?;
        match current_thread_address_location_default(proginfo, version) {
            Some(addr) => Ok(addr),
            None => {
                debug!("Trying to find address location another way");
                Ok(get_thread_address_alt(proginfo, version)?)
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn current_thread_address_location_default(
        proginfo: &ProgramInfo,
        version: &str,
    ) -> Option<u64> {
        // TODO: comment this somewhere
        if version == "2.5.0" {
            // TODO: make this more robust
            get_symbol_addr(
                &proginfo.ruby_map,
                &proginfo.ruby_elf,
                "ruby_current_execution_context_ptr",
            )
        } else {
            get_symbol_addr(
                &proginfo.ruby_map,
                &proginfo.ruby_elf,
                "ruby_current_thread",
            )
        }
    }

    fn get_symbol_addr(map: &MapRange, elf_file: &elf::File, symbol_name: &str) -> Option<u64> {
        elf_symbol_value(elf_file, symbol_name).map(|addr| {
            let load_header = elf_load_header(elf_file);
            debug!("load header: {}", load_header);
            map.range_start + addr - load_header.vaddr
        })
    }

    fn elf_load_header(elf_file: &elf::File) -> elf::types::ProgramHeader {
        elf_file
            .phdrs
            .iter()
            .find(|ref ph| {
                ph.progtype == elf::types::PT_LOAD && (ph.flags.0 & elf::types::PF_X.0) != 0
            })
            .expect("No executable LOAD header found in ELF file. Please report this!")
            .clone()
    }

    #[cfg(target_os = "macos")]
    pub fn current_thread_address_location(pid: pid_t) -> Result<u64, Error> {
        // TODO: Make this actually look up the `__mh_execute_header` base
        //   address in the binary via `nm`.
        let base_address = 0x100000000;
        let addr = get_nm_address(pid)? + (get_maps_address(pid)? - base_address);
        debug!("get_ruby_current_thread_address: {:x}", addr);
        addr
    }
}

pub mod copy {
    use std;
    use libc::pid_t;
    use read_process_memory::*;

    #[derive(Fail, Debug)]
    pub enum MemoryCopyError {
        #[fail(display = "Failed to copy memory address {:x} from PID {}", _1, _0)]
        Io(pid_t, usize, #[cause] std::io::Error),
        #[fail(display = "Process isn't running")] ProcessEnded,
        #[fail(display = "Other")] Other,
        #[fail(display = "Tried to read invalid string")]
        InvalidStringError(#[cause] std::string::FromUtf8Error),
    }

    // #[derive(Fail, Debug)]
    // #[fail(display = "Failed to copy memory address")]
    // pub enum MemoryCopyError{
    //     #[fail(display = "Failed to copy memory address {} from PID {}", _0, _1)]
    //     Io(pid_t, usize, #[cause] std::io::Error),
    // }

    pub fn copy_vec<T>(addr: usize, length: usize, source_pid: pid_t) -> Result<Vec<T>, MemoryCopyError> {
        let mut vec = copy_address_raw(addr, length * std::mem::size_of::<T>(), source_pid)?;
        let capacity = vec.capacity() as usize / std::mem::size_of::<T>() as usize;
        let ptr = vec.as_mut_ptr() as *mut T;
        std::mem::forget(vec);
        unsafe {
            Ok(Vec::from_raw_parts(
                ptr,
                capacity,
                capacity,
            ))
        }
    }

    pub fn copy_address_raw(
        addr: usize,
        length: usize,
        source_pid: pid_t,
    ) -> Result<Vec<u8>, MemoryCopyError> {
        let source = source_pid
            .try_into_process_handle()
            .expect("Failed to convert PID into process handle. This should never happen.");
        debug!("copy_address_raw: addr: {:x}", addr as usize);
        let mut copy = vec![0; length];
        source.copy_address(addr as usize, &mut copy).map_err(|x| {
            if x.raw_os_error() == Some(3) {
                MemoryCopyError::ProcessEnded
            } else {
                MemoryCopyError::Io(source_pid, addr, x)
            }
        })?;
        Ok(copy)
    }

    pub fn copy_struct<U>(addr: u64, source_pid: pid_t) -> Result<U, MemoryCopyError> {
        let result = copy_address_raw(addr as usize, std::mem::size_of::<U>(), source_pid)?;
        let s: U = unsafe { std::ptr::read(result.as_ptr() as *const _) };
        Ok(s)
    }
}

macro_rules! ruby_bindings_v_1_9_x(
($ruby_version:ident) => (
pub mod $ruby_version {
    use std;
    use copy::*;
    use bindings::$ruby_version::*;
    use libc::pid_t;
    use copy::MemoryCopyError;

    get_stack_trace!(rb_thread_struct);
    get_ruby_string!();
    get_cfps!();
    get_label_and_path_1_9_0!();
    is_stack_base_1_9_0!();
}
));

macro_rules! ruby_bindings(
($ruby_version:ident) => (
mod $ruby_version {
    use std;
    use copy::*;
    use bindings::$ruby_version::*;
    use libc::pid_t;
    use copy::MemoryCopyError;


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
    get_stack_trace!(rb_thread_struct);
    get_ruby_string!();
    get_cfps!();
    get_lineno_2_0_0!();
    get_label_and_path_2_0_0!();
    is_stack_base_1_9_0!();
}
));

macro_rules! ruby_bindings_v2(
($ruby_version:ident) => (
mod $ruby_version {
    use std;
    use copy::*;
    use bindings::$ruby_version::*;
    use libc::pid_t;
    use copy::MemoryCopyError;

    get_stack_trace!(rb_thread_struct);
    get_ruby_string!();
    get_cfps!();
    get_lineno_2_3_0!();
    get_label_and_path_2_3_0!();
    is_stack_base_1_9_0!();
}
));

macro_rules! ruby_bindings_v2_5_x(
($ruby_version:ident) => (
mod $ruby_version {
    use std;
    use copy::*;
    use bindings::$ruby_version::*;
    use libc::pid_t;
    use copy::MemoryCopyError;

    get_stack_trace!(rb_execution_context_struct);
    get_ruby_string!();
    get_cfps!();
    get_lineno_2_5_0!();
    get_label_and_path_2_5_0!();
    is_stack_base_2_5_0!();
    get_ruby_string_array_2_5_0!();
}
));

macro_rules! get_stack_trace(
($thread_type:ident) => (

    use stack_trace::FunctionCall;

    pub fn get_stack_trace(
        ruby_current_thread_address_location: u64,
        source_pid: pid_t,
    ) -> Result<Vec<FunctionCall>, MemoryCopyError> {
        debug!(
            "current address location: {:x}",
            ruby_current_thread_address_location
        );
        let current_thread_addr: u64 =
            copy_struct(ruby_current_thread_address_location, source_pid)?;
        debug!("{:x}", current_thread_addr);
        let thread: $thread_type = copy_struct(current_thread_addr, source_pid)?;
        debug!("thread: {:?}", thread);
        let mut trace = Vec::new();
        let cfps = get_cfps(thread.cfp as usize, stack_base(&thread) as u64, source_pid)?;
        for cfp in cfps.iter() {
            if cfp.iseq as usize == 0  || cfp.pc as usize == 0 {
                debug!("huh."); // TODO: fixmeup
                continue;
            }
            let iseq_struct: rb_iseq_struct = copy_struct(cfp.iseq as u64, source_pid)?;
            debug!("iseq_struct: {:?}", iseq_struct);
            let label_path  = get_label_and_path(&iseq_struct, &cfp, source_pid);
            match label_path {
                Ok(call)  => trace.push(call),
                Err(x) => {
                    // this is a heuristic: the intent of this is that it skips function calls into C extensions
                    if trace.len() > 0 {
                        debug!("guess that one didn't work; skipping");
                    } else {
                        return Err(x);
                    }
                }
            }
        }
        Ok(trace)
    }

    use address_finder::*;

    pub fn is_maybe_thread(x: u64, pid: pid_t, heap_map: &MapRange, all_maps: &Vec<MapRange>) -> bool {
        if !is_heap_addr(x, heap_map) {
            return false;
        }

        let thread: $thread_type = match copy_struct(x, pid) {
            Ok(x) => x,
            _ => { return false; },
        };

        if !is_reasonable_thing(&thread, all_maps) {
            return false;
        }

        let stack_base = stack_base(&thread);
        let diff = stack_base - thread.cfp as i64;
        debug!("diff: {}", diff);
        if diff < 0 || diff > 3000000 {
            return false;
        }

        return true;
    }
));

macro_rules! is_stack_base_1_9_0(
() => (
    fn is_reasonable_thing(thread: &rb_thread_struct,  all_maps: &Vec<MapRange>) -> bool {
        in_memory_maps(thread.vm as u64, all_maps) &&
            in_memory_maps(thread.cfp as u64, all_maps) &&
            in_memory_maps(thread.stack as u64, all_maps) &&
            in_memory_maps(thread.self_ as u64, all_maps) &&
            thread.stack_size < 3000000 && thread.state >= 0
    }

    fn stack_base(thread: &rb_thread_struct) -> i64 {
        thread.stack as i64 + thread.stack_size as i64 * std::mem::size_of::<VALUE>() as i64 - 1 * std::mem::size_of::<rb_control_frame_t>() as i64
    }
));

macro_rules! is_stack_base_2_5_0(
() => (
    fn is_reasonable_thing(thread: &rb_execution_context_struct, all_maps: &Vec<MapRange>) -> bool {
        in_memory_maps(thread.tag as u64, all_maps) &&
            in_memory_maps(thread.cfp as u64, all_maps) &&
            in_memory_maps(thread.vm_stack as u64, all_maps) &&
            thread.vm_stack_size < 3000000
    }

    fn stack_base(thread: &rb_execution_context_struct) -> i64 {
        thread.vm_stack as i64 + thread.vm_stack_size as i64 * std::mem::size_of::<VALUE>() as i64 - 1 * std::mem::size_of::<rb_control_frame_t>() as i64
    }
));

macro_rules! get_ruby_string_array_2_5_0(
() => (
    fn get_ruby_string_array(addr: u64, string_class: u64, source_pid: pid_t) -> Result<String, MemoryCopyError> {
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

macro_rules! get_ruby_string(
() => (
    use std::ffi::CStr;

    fn get_ruby_string(addr: u64, source_pid: pid_t) -> Result<String, MemoryCopyError> {
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
                    copy_address_raw(addr as usize, len, source_pid)?
                }
            }
        };
        Ok(String::from_utf8(vec).map_err(|x| {MemoryCopyError::InvalidStringError(x)})?)
    }
));

macro_rules! get_label_and_path_1_9_0(
() => (
    fn get_label_and_path(
        iseq_struct: &rb_iseq_struct,
        cfp: &rb_control_frame_t,
        source_pid: pid_t,
    ) -> Result<FunctionCall, MemoryCopyError> {
        Ok(FunctionCall{
            name: get_ruby_string(iseq_struct.name as u64, source_pid)?,
            path: get_ruby_string(iseq_struct.filename as u64, source_pid)?,
            lineno: None,
        })
    }
));

macro_rules! get_lineno_2_0_0(
() => (
    fn get_lineno(
        iseq_struct: &rb_iseq_struct,
        cfp: &rb_control_frame_t,
        source_pid: pid_t,
    ) -> Result<u32, MemoryCopyError> {
        let mut pos = cfp.pc as usize - iseq_struct.iseq_encoded as usize;
        if pos != 0 {
            pos -= 1;
        }
        let t_size = iseq_struct.line_info_size as usize;
        if t_size == 0 {
            Ok(0) //TODO: really?
        } else if t_size == 1 {
            let table: [iseq_line_info_entry; 1] = copy_struct(iseq_struct.line_info_table as u64, source_pid)?;
            Ok(table[0].line_no)
        } else {
            let table: Vec<iseq_line_info_entry> = copy_vec(iseq_struct.line_info_table as usize, t_size as usize, source_pid)?;
            for i in 0..t_size {
                if pos == table[i].position as usize {
                    return Ok(table[i].line_no)
                } else if table[i].position as usize > pos {
                    return Ok(table[i-1].line_no)
                }
            }
            Ok(table[t_size-1].line_no)
        }
    }
));

macro_rules! get_lineno_2_3_0(
() => (
    fn get_lineno(
        iseq_struct: &rb_iseq_constant_body,
        cfp: &rb_control_frame_t,
        source_pid: pid_t,
    ) -> Result<u32, MemoryCopyError> {
        if iseq_struct.iseq_encoded as usize > cfp.pc as usize {
            return Err(MemoryCopyError::Other);
        }
        let mut pos = cfp.pc as usize - iseq_struct.iseq_encoded as usize; // TODO: investigate panic here
        if pos != 0 {
            pos -= 1;
        }
        let t_size = iseq_struct.line_info_size as usize;
        if t_size == 0 {
            Ok(0) //TODO: really?
        } else if t_size == 1 {
            let table: [iseq_line_info_entry; 1] = copy_struct(iseq_struct.line_info_table as u64, source_pid)?;
            Ok(table[0].line_no)
        } else {
            let table: Vec<iseq_line_info_entry> = copy_vec(iseq_struct.line_info_table as usize, t_size as usize, source_pid)?;
            for i in 0..t_size {
                if pos == table[i].position as usize {
                    return Ok(table[i].line_no)
                } else if table[i].position as usize > pos {
                    return Ok(table[i-1].line_no)
                }
            }
            Ok(table[t_size-1].line_no)
        }
    }
));

macro_rules! get_lineno_2_5_0(
() => (
    fn get_lineno(
        iseq_struct: &rb_iseq_constant_body,
        cfp: &rb_control_frame_t,
        source_pid: pid_t,
    ) -> Result<u32, MemoryCopyError> {
        let mut pos = cfp.pc as usize - iseq_struct.iseq_encoded as usize;
        if pos != 0 {
            pos -= 1;
        }
        let t_size = iseq_struct.insns_info_size as usize;
        if t_size == 0 {
            Ok(0) //TODO: really?
        } else if t_size == 1 {
            let table: [iseq_insn_info_entry; 1] = copy_struct(iseq_struct.insns_info as u64, source_pid)?;
            Ok(table[0].line_no as u32)
        } else {
            let table: Vec<iseq_insn_info_entry> = copy_vec(iseq_struct.insns_info as usize, t_size as usize, source_pid)?;
            for i in 0..t_size {
                if pos == table[i].position as usize {
                    return Ok(table[i].line_no as u32)
                } else if table[i].position as usize > pos {
                    return Ok(table[i-1].line_no as u32)
                }
            }
            Ok(table[t_size-1].line_no as u32)
        }
    }
));



macro_rules! get_label_and_path_2_0_0(
() => (
   fn get_label_and_path(
        iseq_struct: &rb_iseq_struct,
        cfp: &rb_control_frame_t,
        source_pid: pid_t,
    ) -> Result<FunctionCall, MemoryCopyError> {
        Ok(FunctionCall{
            name: get_ruby_string(iseq_struct.location.label as u64, source_pid)?,
            path: get_ruby_string(iseq_struct.location.path as u64, source_pid)?,
            lineno: Some(get_lineno(iseq_struct, cfp, source_pid)?),
        })
    }
));

macro_rules! get_label_and_path_2_3_0(
() => (
    fn get_label_and_path(
        iseq_struct: &rb_iseq_struct,
        cfp: &rb_control_frame_t,
        source_pid: pid_t,
    ) -> Result<FunctionCall, MemoryCopyError> {
        let body: rb_iseq_constant_body = copy_struct(iseq_struct.body as u64, source_pid)?;
        Ok(FunctionCall{
            name: get_ruby_string(body.location.label as u64, source_pid)?,
            path: get_ruby_string(body.location.path as u64, source_pid)?,
            lineno: Some(get_lineno(&body, cfp, source_pid)?),
        })
    }
));

macro_rules! get_label_and_path_2_5_0(
() => (
    fn get_label_and_path(
        iseq_struct: &rb_iseq_struct,
        cfp: &rb_control_frame_t,
        source_pid: pid_t,
    ) -> Result<FunctionCall, MemoryCopyError> {
        let body: rb_iseq_constant_body = copy_struct(iseq_struct.body as u64, source_pid)?;
        let rstring: RString = copy_struct(body.location.label as u64, source_pid)?;
        Ok(FunctionCall{
            name: get_ruby_string(body.location.label as u64, source_pid)?,
            path:  get_ruby_string_array(body.location.pathobj as u64, rstring.basic.klass as u64, source_pid)?,
            lineno: Some(get_lineno(&body, cfp, source_pid)?),
        })
    }
));

macro_rules! get_cfps(
() => (
    // Ruby stack grows down, starting at
    //   ruby_current_thread->stack + ruby_current_thread->stack_size - 1 * sizeof(rb_control_frame_t)
    // I don't know what the -1 is about. Also note that the stack_size is *not* in bytes! stack is a
    // VALUE*, and so stack_size is in units of sizeof(VALUE).
    //
    // The base of the call stack is therefore at
    //   stack + stack_size * sizeof(VALUE) - sizeof(rb_control_frame_t)
    // (with everything in bytes).
    fn get_cfps(cfp_address: usize, stack_base: u64, source_pid: pid_t) -> Result<Vec<rb_control_frame_t>, MemoryCopyError> {
        Ok(copy_vec(cfp_address, (stack_base as usize - cfp_address) as usize / std::mem::size_of::<rb_control_frame_t>(), source_pid)?)
    }
));

pub mod stack_trace {
    use libc::pid_t;
    use address_finder;
    use copy;
    use std::fmt;

    pub struct FunctionCall {
        name: String,
        path: String,
        lineno: Option<u32>,
    }

    impl fmt::Display for FunctionCall {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self.lineno {
                Some(line) => write!(f, "{} - {} line {}", self.name, self.path, line),
                None => write!(f, "{} - {}", self.name, self.path),
            }
        }
}

    pub fn is_maybe_thread_function(
        version: &str,
    ) -> Box<Fn(u64, pid_t, &address_finder::MapRange, &Vec<address_finder::MapRange>) -> bool>
    {
        let function = match version.as_ref() {
            "1.9.1" => self::ruby_1_9_1_0::is_maybe_thread,
            "1.9.2" => self::ruby_1_9_2_0::is_maybe_thread,
            "1.9.3" => self::ruby_1_9_3_0::is_maybe_thread,
            "2.0.0" => self::ruby_2_0_0_0::is_maybe_thread,
            "2.1.0" => self::ruby_2_1_0::is_maybe_thread,
            "2.1.1" => self::ruby_2_1_1::is_maybe_thread,
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
    ) -> Box<Fn(u64, pid_t) -> Result<Vec<FunctionCall>, copy::MemoryCopyError>> {
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
