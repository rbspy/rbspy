pub use self::os_impl::*;
use core::types::pid_t;

/*
 * Operating-system specific code for getting
 * a) the address of the current thread, and
 * b) the address of the Ruby version of a PID
 *
 * from a running Ruby process. Involves a lot of reading memory maps and symbols.
 */

#[derive(Fail, Debug)]
pub enum AddressFinderError {
    #[fail(display = "No process with PID: {}", _0)] NoSuchProcess(pid_t),
    #[fail(display = "Permission denied when reading from process {}. If you're not running as root, try again with sudo. If you're using Docker, try passing `--cap-add=SYS_PTRACE` to `docker run`", _0)]
    PermissionDenied(pid_t),
    #[fail(display = "Couldn't get port for PID {}. Possibilities: that process doesn't exist or you have SIP enabled and you're trying to profile system Ruby (try rbenv instead).", _0)]
    #[cfg(target_os = "macos")]
    MacPermissionDenied(pid_t),
    #[fail(display = "Error reading /proc/{}/maps", _0)] ProcMapsError(pid_t),
}

#[cfg(target_os = "macos")]
mod os_impl {
    use core::address_finder::AddressFinderError;
    use proc_maps::{get_process_maps, MapRange};
    use proc_maps::mac_maps::{get_symbols, Symbol};

    use failure::Error;
    use libc::pid_t;
    use read_process_memory::*;

    pub fn get_ruby_version_address(pid: pid_t) -> Result<usize, Error> {
        let proginfo = &get_program_info(pid)?;
        proginfo.symbol_addr("_ruby_version")
    }

    pub fn current_thread_address(
        pid: pid_t,
        version: &str,
        _is_maybe_thread: Box<Fn(usize, usize, ProcessHandle, &[MapRange]) -> bool>,
    ) -> Result<usize, Error> {
        let proginfo = &get_program_info(pid)?;
        if version >= "2.5.0" {
            proginfo.symbol_addr("_ruby_current_execution_context_ptr")
        } else {
            proginfo.symbol_addr("_ruby_current_thread")
        }
    }

    struct Binary {
        pub start_addr: usize,
        pub symbols: Vec<Symbol>,
    }

    impl ProgramInfo {
        pub fn symbol_addr(&self, symbol_name: &str) -> Result<usize, Error> {
            let offset = self.ruby_binary
                .symbol_value_mach("__mh_execute_header")
                .expect("Couldn't find __mh_execute_header symbol");
            if let Ok(try_1) = self.ruby_binary.symbol_addr(symbol_name, offset) {
                Ok(try_1)
            } else if let Some(ref binary) = self.libruby_binary {
                binary.symbol_addr(symbol_name, 0)
            } else {
                Err(format_err!(
                    "No libruby binary found, are you using system Ruby?"
                ))
            }
        }
    }

    impl Binary {
        pub fn from(start_addr: usize, filename: &str) -> Result<Binary, Error> {
            Ok(Binary {
                start_addr,
                symbols: get_symbols(filename)?,
            })
        }

        pub fn symbol_addr(&self, symbol_name: &str, offset: usize) -> Result<usize, Error> {
            let addr = self.symbol_value_mach(symbol_name).ok_or(format_err!(
                "Couldn't find symbol"
            ))?;
            Ok(addr + self.start_addr - offset)
        }

        pub fn symbol_value_mach(&self, symbol_name: &str) -> Option<usize> {
            for sym in &self.symbols {
                if sym.name == symbol_name && !sym.value.is_none() {
                    return Some(sym.value.unwrap());
                }
            }
            None
        }
    }

    struct ProgramInfo {
        ruby_binary: Binary,
        libruby_binary: Option<Binary>,
    }

    fn get_program_info(pid: pid_t) -> Result<ProgramInfo, Error> {
        let maps = get_process_maps(pid).map_err(|_| {
            AddressFinderError::MacPermissionDenied(pid)
        })?;
        let ruby_binary = get_ruby_binary(&maps)?;
        let libruby_binary = get_libruby_binary(&maps);
        Ok(ProgramInfo {
            ruby_binary,
            libruby_binary,
        })
    }

    fn get_ruby_binary(maps: &Vec<MapRange>) -> Result<Binary, Error> {
        let map: &MapRange = maps.iter()
            .find(|ref m| if let Some(ref pathname) = m.filename() {
                pathname.contains("bin/ruby") && m.is_exec()
            } else {
                false
            })
            .ok_or(format_err!("Couldn't find ruby map"))?;
        Binary::from(map.start(), map.filename().as_ref().unwrap())
    }

    fn get_libruby_binary(maps: &Vec<MapRange>) -> Option<Binary> {
        let maybe_map = maps.iter().find(
            |ref m| if let Some(ref pathname) = m.filename() {
                pathname.contains("libruby") && m.is_exec()
            } else {
                false
            },
        );
        match maybe_map.as_ref() {
            Some(map) => Some(
                Binary::from(map.start(), map.filename().as_ref().unwrap()).unwrap(),
            ),
            None => None,
        }
    }
}

#[cfg(target_os = "linux")]
mod os_impl {
    use core::address_finder::AddressFinderError;
    use core::copy::*;
    use proc_maps::{MapRange, get_process_maps};

    use elf;
    use failure::Error;
    use libc::pid_t;
    use std;
    use read_process_memory::*;

    pub fn current_thread_address(
        pid: pid_t,
        version: &str,
        is_maybe_thread: Box<Fn(usize, usize, ProcessHandle, &[MapRange]) -> bool>,
    ) -> Result<usize, Error> {
        let proginfo = &get_program_info(pid)?;
        match current_thread_address_symbol_table(proginfo, version) {
            Some(addr) => Ok(addr),
            None => {
                debug!("Trying to find address location another way");
                Ok(current_thread_address_search_bss(
                    proginfo,
                    is_maybe_thread,
                )?)
            }
        }
    }

    pub fn get_ruby_version_address(pid: pid_t) -> Result<usize, Error> {
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
                        .ok_or_else(|| format_err!("Missing libruby map. Please report this!"))?,
                    proginfo
                        .libruby_elf
                        .as_ref()
                        .ok_or_else(|| format_err!("Missing libruby ELF. Please report this!"))?,
                    ruby_version_symbol,
                ).ok_or_else(|| format_err!("Couldn't find ruby version."))
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
            if s.shdr.name == ".bss" {
                return Some(s.shdr.clone());
            }
        }
        None
    }

    fn current_thread_address_search_bss(
        proginfo: &ProgramInfo,
        is_maybe_thread: Box<Fn(usize, usize, ProcessHandle, &[MapRange]) -> bool>,
    ) -> Result<usize, Error> {
        // Used when there's no symbol table. Looks through the .bss and uses a search_bss (found in
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
        let read_addr = map.start() + bss_section.addr as usize - load_header.vaddr as usize;

        debug!("read_addr: {:x}", read_addr);
        let source = proginfo.pid.try_into_process_handle().unwrap();
        let mut data = copy_address_raw(read_addr as usize, bss_section.size as usize, &source)?;
        debug!("successfully read data");
        let slice: &[usize] = unsafe {
            std::slice::from_raw_parts(
                data.as_mut_ptr() as *mut usize,
                data.capacity() as usize / std::mem::size_of::<usize>() as usize,
            )
        };

        let i = slice
            .iter().enumerate()
            .position({ |(i, &x)| is_maybe_thread(x, (i as usize) * (std::mem::size_of::<usize>() as usize) + read_addr, source, &proginfo.all_maps) })
            .ok_or_else(|| format_err!(
                "Current thread address not found in process {}",
                &proginfo.pid
            ))?;
        Ok((i as usize) * (std::mem::size_of::<usize>() as usize) + read_addr)
    }

    fn current_thread_address_symbol_table(
        // Uses the symbol table to get the address of the current thread
        proginfo: &ProgramInfo,
        version: &str,
    ) -> Option<usize> {
        // TODO: comment this somewhere
        if version >= "2.5.0" {
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
            map.start() + addr - load_header.vaddr as usize
        })
    }

    fn elf_load_header(elf_file: &elf::File) -> elf::types::ProgramHeader {
        *elf_file
            .phdrs
            .iter()
            .find(|ref ph| {
                ph.progtype == elf::types::PT_LOAD && (ph.flags.0 & elf::types::PF_X.0) != 0
            })
            .expect("No executable LOAD header found in ELF file. Please report this!")
    }

    // struct to hold everything we know about the program
    pub struct ProgramInfo {
        pub pid: pid_t,
        pub all_maps: Vec<MapRange>,
        pub ruby_map: Box<MapRange>,
        pub libruby_map: Box<Option<MapRange>>,
        pub ruby_elf: elf::File,
        pub libruby_elf: Option<elf::File>,
    }

    fn open_elf_file(pid: pid_t, map: &MapRange) -> Result<elf::File, Error> {
        // Read binaries from `/proc/PID/root` because the target process might be in a different
        // mount namespace. /proc/PID/root is the view of the filesystem that the target process
        // has. (see the proc man page for more)
        // So we read /usr/bin/ruby from /proc/PID/root/usr/bin/ruby
        let map_path = map.filename().as_ref().expect("map's pathname shouldn't be None");
        let elf_path = format!("/proc/{}/root{}", pid, map_path);
        elf::File::open_path(&elf_path)
            .map_err(|_| format_err!("Couldn't open ELF file: {:?}", elf_path))
    }

    pub fn get_program_info(pid: pid_t) -> Result<ProgramInfo, Error> {
        let all_maps = get_process_maps(pid).map_err(|x| match x.kind() {
            std::io::ErrorKind::NotFound => AddressFinderError::NoSuchProcess(pid),
            std::io::ErrorKind::PermissionDenied => AddressFinderError::PermissionDenied(pid),
            _ => AddressFinderError::ProcMapsError(pid),
        })?;
        let ruby_map = Box::new(get_map(&all_maps, "bin/ruby")
            .ok_or_else(|| format_err!("Ruby map not found for PID: {}", pid))?);
        let all_maps = get_process_maps(pid).unwrap();
        let ruby_elf = open_elf_file(pid, &ruby_map)?;
        let libruby_map = Box::new(get_map(&all_maps, "libruby"));
        let libruby_elf = match *libruby_map {
            Some(ref map) => {
                Some(open_elf_file(pid, &map)?)
            }
            _ => None,
        };
        Ok(ProgramInfo {
            pid,
            all_maps,
            ruby_map,
            libruby_map,
            ruby_elf,
            libruby_elf,
        })
    }

    fn get_map(maps: &[MapRange], contains: &str) -> Option<MapRange> {
        maps.iter()
            .find(|ref m| {
                if let Some(ref pathname) = m.filename() {
                    pathname.contains(contains) && m.is_exec()
                } else {
                    false
                }
            })
            .map(std::clone::Clone::clone)
    }
}

#[cfg(windows)]
mod os_impl {
    use failure::Error;
    use read_process_memory::ProcessHandle;
    use proc_maps::{get_process_maps, MapRange, Pid};
    use proc_maps::win_maps::SymbolLoader;

    pub fn get_ruby_version_address(pid: u32) -> Result<usize, Error> {
        get_symbol_address(pid, "ruby_version")
    }

    pub fn current_thread_address(
        pid: u32,
        version: &str,
        _is_maybe_thread: Box<Fn(usize, usize, ProcessHandle, &[MapRange]) -> bool>,
    ) -> Result<usize, Error> {
        let symbol_name = if version >= "2.5.0" {
            "ruby_current_execution_context_ptr"
        } else {
            "ruby_current_thread"
        };

        get_symbol_address(pid, symbol_name)
    }

    fn get_symbol_address(pid: u32, symbol_name: &str) -> Result<usize, Error> {
        let maps = get_process_maps(pid)?;
        let ruby = get_ruby_binary(&maps)?;
        if let Some(ref filename) = ruby.filename() {
            let handler = SymbolLoader::new(pid as Pid)?;
            let _module = handler.load_module(filename)?; // need to keep this module in scope
            if let Ok((base, addr)) = handler.address_from_name(symbol_name) {
                // If we have a module base (ie from PDB), need to adjust by the offset
                // otherwise seems like we can take address directly
                let addr = if base == 0 { addr } else { ruby.start() as u64 + addr - base };
                return Ok(addr as usize);
            }
        }

        // try again loading symbols from the ruby DLL if found
        if let Some(libruby) = get_libruby_binary(&maps) {
            if let Some(ref filename) = libruby.filename() {
                let handler = SymbolLoader::new(pid as Pid)?;
                let _module = handler.load_module(filename)?;
                if let Ok((base, addr)) = handler.address_from_name(symbol_name) {
                    let addr = if base == 0 { addr } else { libruby.start() as u64 + addr - base };
                    return Ok(addr as usize);
                }
            }
        }

        Err(format_err!("failed to find {} symbol", symbol_name))
    }

    fn get_ruby_binary(maps: &[MapRange]) -> Result<&MapRange, Error> {
        Ok(maps.iter()
            .find(|ref m| if let Some(ref pathname) = m.filename() {
                pathname.contains("ruby.exe")
            } else {
                false
            })
            .ok_or(format_err!("Couldn't find ruby binary"))?)
    }


    fn get_libruby_binary(maps: &[MapRange]) -> Option<&MapRange> {
        maps.iter().find(
            |ref m| if let Some(ref pathname) = m.filename() {
                // pathname is something like "C:\Ruby24-x64\bin\x64-msvcrt-ruby240.dll"
                pathname.contains("-ruby") && pathname.ends_with(".dll")
            } else {
                false
            },
        )
    }
}
