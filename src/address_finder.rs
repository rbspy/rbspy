use copy::*;
use failure::Error;
use failure::ResultExt;
use libc::{c_char, pid_t};
use stack_trace;
use std::time::Duration;
use std;
use std::fmt;

pub struct StackFrame {
    pub name: String,
    pub path: String,
    pub lineno: Option<u32>,
}

// Use a StackTraceGetter to get stack traces
pub struct StackTraceGetter {
    pid: pid_t,
    current_thread_addr_location: usize,
    stack_trace_function: Box<Fn(usize, pid_t) -> Result<Vec<StackFrame>, MemoryCopyError>>,
}

impl fmt::Display for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.lineno {
            Some(line) => write!(f, "{} - {} line {}", self.name, self.path, line),
            None => write!(f, "{} - {}", self.name, self.path),
        }
    }
}

impl StackTraceGetter {
    pub fn get(&self) -> Result<Vec<StackFrame>, MemoryCopyError> {
        let stack_trace_function = &self.stack_trace_function;
        stack_trace_function(self.current_thread_addr_location, self.pid)
    }
}

pub fn stack_trace_getter(pid: pid_t) -> Result<StackTraceGetter, Error> {
    let version = get_api_version_retry(pid).context("Couldn't determine Ruby version")?;
    debug!("version: {}", version);
    Ok(StackTraceGetter {
        pid: pid,
        current_thread_addr_location: os_impl::current_thread_address_location(pid, &version)?,
        stack_trace_function: stack_trace::get_stack_trace_function(&version),
    })
}

// Everything below here is private

#[derive(Fail, Debug)]
enum AddressFinderError {
    #[fail(display = "No process with PID: {}", _0)] NoSuchProcess(pid_t),
    #[fail(display = "Permission denied when reading from process {}. Try again with sudo?", _0)]
    PermissionDenied(pid_t),
    #[fail(display = "Error reading /proc/{}/maps", _0)] ProcMapsError(pid_t),
}

fn get_api_version_retry(pid: pid_t) -> Result<String, Error> {
    // this exists because sometimes rbenv takes a while to exec the right Ruby binary.
    // we are dumb right now so we just... wait until it seems to work out.
    let mut i = 0;
    loop {
        let version = get_api_version(pid);
        let mut ret = false;
        match &version {
            &Err(ref err) => {
                match err.root_cause().downcast_ref::<AddressFinderError>() {
                    Some(&AddressFinderError::PermissionDenied(_)) => {
                        ret = true;
                    }
                    Some(&AddressFinderError::NoSuchProcess(_)) => {
                        ret = true;
                    }
                    _ => {}
                }
                match err.root_cause().downcast_ref::<MemoryCopyError>() {
                    Some(&MemoryCopyError::PermissionDenied(_)) => {
                        ret = true;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        if i > 100 || version.is_ok() || ret {
            return Ok(version?);
        }
        // if it doesn't work, sleep for 1ms and try again
        i += 1;
        std::thread::sleep(Duration::from_millis(1));
    }
}

pub fn get_api_version(pid: pid_t) -> Result<String, Error> {
    let addr = os_impl::get_api_address(pid)?;
    debug!("api addr: {:x}", addr);
    let x: [c_char; 15] = copy_struct(addr, pid)?;
    debug!("api struct: {:?}", x);
    Ok(unsafe {
        std::ffi::CStr::from_ptr(x.as_ptr() as *mut c_char)
            .to_str()?
            .to_owned()
    })
}

#[test]
fn test_get_nonexistent_process() {
    let version = get_api_version_retry(10000);
    match version
        .unwrap_err()
        .root_cause()
        .downcast_ref::<AddressFinderError>()
        .unwrap()
    {
        &AddressFinderError::NoSuchProcess(10000) => {}
        _ => assert!(false, "Expected NoSuchProcess error"),
    }
}

#[test]
fn test_get_disallowed_process() {
    let version = get_api_version_retry(1);
    match version
        .unwrap_err()
        .root_cause()
        .downcast_ref::<AddressFinderError>()
        .unwrap()
    {
        &AddressFinderError::PermissionDenied(1) => {}
        _ => assert!(false, "Expected NoSuchProcess error"),
    }
}

#[test]
fn test_current_thread_address_location() {
    let mut process = std::process::Command::new("/usr/bin/ruby").spawn().unwrap();
    let pid = process.id() as pid_t;
    let version = get_api_version_retry(pid);
    assert!(version.is_ok());
    let result = os_impl::current_thread_address_location(pid, &version.unwrap());
    assert!(result.is_ok());
    process.kill().unwrap();
}

#[cfg(target_os = "macos")]
mod os_impl {
    // TODO: fill this in.
    fn get_maps_address(pid: pid_t) -> usize {
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
        let addr = usize::from_str_radix(address_str, 16).unwrap();
        debug!("get_maps_address: {:x}", addr);
        addr
    }

    #[cfg(target_os = "macos")]
    fn current_thread_address_location(pid: pid_t) -> Result<usize, Error> {
        // TODO: Make this actually look up the `__mh_execute_header` base
        //   address in the binary via `nm`.
        let base_address = 0x100000000;
        let addr = get_nm_address(pid)? + (get_maps_address(pid)? - base_address);
        debug!("get_ruby_current_thread_address: {:x}", addr);
        addr
    }
}

#[cfg(target_os = "linux")]
mod os_impl {
    use copy::*;
    use elf;
    use proc_maps::*;
    use failure::Error;
    use libc::pid_t;
    use stack_trace;
    use std;
    use self::program_info::*;

    pub fn current_thread_address_location(pid: pid_t, version: &str) -> Result<usize, Error> {
        let proginfo = &program_info::get_program_info(pid)?;
        match current_thread_address_location_default(proginfo, version) {
            Some(addr) => Ok(addr),
            None => {
                debug!("Trying to find address location another way");
                Ok(current_thread_address_alt(proginfo, version)?)
            }
        }
    }

    pub fn get_api_address(pid: pid_t) -> Result<usize, Error> {
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
                ).ok_or(format_err!("Couldn't find ruby version."))
            }
        }
    }

    fn elf_symbol_value(elf_file: &elf::File, symbol_name: &str) -> Option<usize> {
        // TODO: maybe move this to goblin so that it works on OS X & BSD, not just linux
        let sections = &elf_file.sections;
        for s in sections {
            for sym in elf_file
                .get_symbols(&s)
                .expect("Failed to get symbols from section")
            {
                if sym.name == symbol_name {
                    debug!("symbol: {}", sym);
                    return Some(sym.value as usize);
                }
            }
        }
        None
    }

    fn get_bss_section(elf_file: &elf::File) -> Option<elf::types::SectionHeader> {
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

    fn current_thread_address_alt(proginfo: &ProgramInfo, version: &str) -> Result<usize, Error> {
        // Used when there's no symbol table. Looks through the .bss and uses a heuristic (found in
        // `is_maybe_thread`) to find the address of the current thread.
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
        let read_addr = map.range_start + bss_section.addr as usize - load_header.vaddr as usize;

        debug!("read_addr: {:x}", read_addr);
        let mut data =
            copy_address_raw(read_addr as usize, bss_section.size as usize, proginfo.pid)?;
        debug!("successfully read data");
        let slice: &[usize] = unsafe {
            std::slice::from_raw_parts(
                data.as_mut_ptr() as *mut usize,
                data.capacity() as usize / std::mem::size_of::<usize>() as usize,
            )
        };

        let is_maybe_thread = stack_trace::is_maybe_thread_function(version);

        let i = slice
            .iter()
            .position({
                |&x| is_maybe_thread(x, proginfo.pid, &proginfo.heap_map, &proginfo.all_maps)
            })
            .ok_or(format_err!(
                "Current thread address not found in process {}",
                &proginfo.pid
            ))?;
        Ok((i as usize) * (std::mem::size_of::<usize>() as usize) + read_addr)
    }

    fn current_thread_address_location_default(
        // uses the symbol table to get the address of the current thread
        proginfo: &ProgramInfo,
        version: &str,
    ) -> Option<usize> {
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

    fn get_symbol_addr(map: &MapRange, elf_file: &elf::File, symbol_name: &str) -> Option<usize> {
        elf_symbol_value(elf_file, symbol_name).map(|addr| {
            let load_header = elf_load_header(elf_file);
            debug!("load header: {}", load_header);
            map.range_start + addr - load_header.vaddr as usize
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

    mod program_info {
        use elf;
        use libc::pid_t;
        use std;
        use proc_maps::*;
        use failure::Error;
        use address_finder::AddressFinderError;

        // struct to hold everything we know about the program
        pub struct ProgramInfo {
            pub pid: pid_t,
            pub all_maps: Vec<MapRange>,
            pub ruby_map: Box<MapRange>,
            pub heap_map: Box<MapRange>,
            pub libruby_map: Box<Option<MapRange>>,
            pub ruby_elf: elf::File,
            pub libruby_elf: Option<elf::File>,
        }

        pub fn get_program_info(pid: pid_t) -> Result<ProgramInfo, Error> {
            let all_maps = get_proc_maps(pid).map_err(|x| match x.kind() {
                std::io::ErrorKind::NotFound => AddressFinderError::NoSuchProcess(pid),
                std::io::ErrorKind::PermissionDenied => AddressFinderError::PermissionDenied(pid),
                _ => AddressFinderError::ProcMapsError(pid),
            })?;
            let ruby_map = Box::new(get_ruby_map(&all_maps)
                .ok_or(format_err!("Ruby map not found for PID: {}", pid))?);
            let heap_map = Box::new(get_heap_map(&all_maps)
                .ok_or(format_err!("Heap map not found for PID: {}", pid))?);
            let ruby_path = &ruby_map
                .pathname
                .clone()
                .expect("ruby map's pathname shouldn't be None");
            let ruby_elf = elf::File::open_path(ruby_path)
                .map_err(|_| format_err!("Couldn't open ELF file: {}", ruby_path))?;
            let libruby_map = Box::new(libruby_map(&all_maps));
            let libruby_elf = match *libruby_map {
                Some(ref map) => {
                    let path = &map.pathname
                        .clone()
                        .expect("libruby map's pathname shouldn't be None");
                    Some(elf::File::open_path(path)
                        .map_err(|_| format_err!("Couldn't open ELF file: {}", path))?)
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
    }
}
