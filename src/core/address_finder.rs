pub use self::os_impl::*;
use crate::core::types::Pid;
use thiserror::Error;

/*
 * Operating-system specific code for getting
 * a) the address of the current thread, and
 * b) the address of the Ruby version of a PID
 *
 * from a running Ruby process. Involves a lot of reading memory maps and symbols.
 */

#[derive(Error, Debug)]
pub enum AddressFinderError {
    #[error("No process with PID: {}", _0)]
    #[cfg(not(target_os = "macos"))]
    NoSuchProcess(Pid),
    #[error("Permission denied when reading from process {}. If you're not running as root, try again with sudo. If you're using Docker, try passing `--cap-add=SYS_PTRACE` to `docker run`", _0)]
    #[cfg(not(target_os = "macos"))]
    PermissionDenied(Pid),
    #[error("Couldn't get port for PID {}. Possibilities: that process doesn't exist or you have SIP enabled and you're trying to profile system Ruby (try rbenv instead).", _0)]
    #[cfg(target_os = "macos")]
    MacPermissionDenied(Pid),
    #[error("Error reading /proc/{}/maps", _0)]
    #[cfg(not(target_os = "macos"))]
    ProcMapsError(Pid),
}

#[cfg(target_os = "macos")]
mod os_impl {
    use crate::core::address_finder::AddressFinderError;
    use crate::core::initialize::IsMaybeThreadFn;
    use crate::core::types::Pid;

    use anyhow::{format_err, Result};
    use proc_maps::mac_maps::{get_dyld_info, get_symbols, DyldInfo, Symbol};

    pub fn get_ruby_version_address(pid: Pid) -> Result<usize> {
        let proginfo = &get_program_info(pid)?;
        proginfo.symbol_addr("_ruby_version")
    }

    pub fn get_ruby_global_symbols_address(pid: Pid, version: &str) -> Result<usize> {
        let proginfo = &get_program_info(pid)?;
        if version >= "2.7.0" {
            proginfo.symbol_addr("_ruby_global_symbols")
        } else {
            proginfo.symbol_addr("_global_symbols")
        }
    }

    pub fn get_vm_address(pid: Pid, version: &str) -> Result<usize> {
        let proginfo = &get_program_info(pid)?;

        if version >= "2.5.0" {
            proginfo.symbol_addr("_ruby_current_vm_ptr")
        } else {
            proginfo.symbol_addr("_ruby_current_vm")
        }
    }

    pub fn current_thread_address(
        pid: Pid,
        version: &str,
        _is_maybe_thread: IsMaybeThreadFn,
    ) -> Result<usize> {
        let proginfo = &get_program_info(pid)?;
        if version >= "3.0.0" {
            panic!("Current thread address isn't directly accessible on ruby 3 and newer");
        } else if version >= "2.5.0" {
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
        pub fn symbol_addr(&self, symbol_name: &str) -> Result<usize> {
            let offset = self
                .ruby_binary
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
        pub fn from(start_addr: usize, filename: &str) -> Result<Binary> {
            Ok(Binary {
                start_addr,
                symbols: get_symbols(filename)?,
            })
        }

        pub fn symbol_addr(&self, symbol_name: &str, offset: usize) -> Result<usize> {
            let addr = self
                .symbol_value_mach(symbol_name)
                .ok_or(format_err!("Couldn't find symbol"))?;
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

    fn get_program_info(pid: Pid) -> Result<ProgramInfo> {
        let maps = get_dyld_info(pid).map_err(|_| AddressFinderError::MacPermissionDenied(pid))?;
        let ruby_binary = get_ruby_binary(&maps)?;
        let libruby_binary = get_libruby_binary(&maps);
        Ok(ProgramInfo {
            ruby_binary,
            libruby_binary,
        })
    }

    fn get_ruby_binary(maps: &Vec<DyldInfo>) -> Result<Binary> {
        let map: &DyldInfo = maps
            .iter()
            .find(|ref m| m.filename.contains("bin/ruby"))
            .ok_or(format_err!("Couldn't find ruby map"))?;
        Binary::from(map.address, &map.filename)
    }

    fn get_libruby_binary(maps: &Vec<DyldInfo>) -> Option<Binary> {
        let maybe_map = maps.iter().find(|ref m| m.filename.contains("libruby"));
        match maybe_map.as_ref() {
            Some(map) => Some(Binary::from(map.address, &map.filename).unwrap()),
            None => None,
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
mod os_impl {
    use crate::core::address_finder::AddressFinderError;
    use crate::core::initialize::IsMaybeThreadFn;
    use crate::core::types::{Pid, Process};
    use anyhow::{format_err, Context, Result};
    use proc_maps::{get_process_maps, MapRange};
    use remoteprocess::ProcessMemory;

    pub fn get_vm_address(pid: Pid, version: &str) -> Result<usize> {
        let proginfo = &get_program_info(pid)?;

        if version >= "2.5.0" {
            proginfo.get_symbol_addr("ruby_current_vm_ptr")
        } else {
            proginfo.get_symbol_addr("ruby_current_vm")
        }
    }

    pub fn current_thread_address(
        pid: Pid,
        version: &str,
        is_maybe_thread: IsMaybeThreadFn,
    ) -> Result<usize> {
        if version >= "3.0.0" {
            panic!("Current thread address isn't directly accessible on ruby 3 and newer");
        } else {
            let proginfo = &get_program_info(pid)?;
            match current_thread_address_symbol_table(proginfo, version) {
                Some(addr) => Ok(addr),
                None => {
                    debug!("Trying to find address location another way");
                    current_thread_address_search_bss(proginfo, is_maybe_thread)
                }
            }
        }
    }

    pub fn get_ruby_version_address(pid: Pid) -> Result<usize> {
        let proginfo = &get_program_info(pid)?;
        let ruby_version_symbol = "ruby_version";
        proginfo.get_symbol_addr(ruby_version_symbol)
    }

    pub fn get_ruby_global_symbols_address(pid: Pid, version: &str) -> Result<usize> {
        let proginfo = &get_program_info(pid)?;
        let symbol_name = if version >= "2.7.0" {
            "ruby_global_symbols"
        } else {
            "global_symbols"
        };
        proginfo.get_symbol_addr(symbol_name)
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
        is_maybe_thread: IsMaybeThreadFn,
    ) -> Result<usize> {
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
        let process = Process::new(proginfo.pid)?;
        let mut data = process
            .copy(read_addr as usize, bss_section.size as usize)
            .context(read_addr as usize)?;
        debug!("successfully read data");
        let slice: &[usize] = unsafe {
            std::slice::from_raw_parts(
                data.as_mut_ptr() as *mut usize,
                data.capacity() as usize / std::mem::size_of::<usize>() as usize,
            )
        };

        let i = slice
            .iter()
            .enumerate()
            .position(|(i, &x)| {
                is_maybe_thread(
                    x,
                    (i as usize) * (std::mem::size_of::<usize>() as usize) + read_addr,
                    &process,
                    &proginfo.all_maps,
                )
            })
            .ok_or_else(|| {
                format_err!(
                    "Current thread address not found in process {}",
                    &proginfo.pid
                )
            })?;
        Ok((i as usize) * (std::mem::size_of::<usize>() as usize) + read_addr)
    }

    fn current_thread_address_symbol_table(
        // Uses the symbol table to get the address of the current thread
        proginfo: &ProgramInfo,
        version: &str,
    ) -> Option<usize> {
        // TODO: comment this somewhere
        if version >= "3.0.0" {
            panic!("Current thread address isn't directly accessible on ruby 3 and newer");
        } else if version >= "2.5.0" {
            proginfo
                .get_symbol_addr("ruby_current_execution_context_ptr")
                .ok()
        } else {
            proginfo.get_symbol_addr("ruby_current_thread").ok()
        }
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
        pub pid: Pid,
        pub all_maps: Vec<MapRange>,
        pub ruby_map: Box<MapRange>,
        pub libruby_map: Box<Option<MapRange>>,
        pub ruby_elf: elf::File,
        pub libruby_elf: Option<elf::File>,
    }

    impl ProgramInfo {
        pub fn get_symbol_addr(&self, symbol_name: &str) -> Result<usize> {
            if let Some(addr) = elf_symbol_value(&self.ruby_elf, symbol_name).map(|addr| {
                let load_header = elf_load_header(&self.ruby_elf);
                debug!("load header: {}", load_header);
                self.ruby_map.start() + addr - load_header.vaddr as usize
            }) {
                return Ok(addr);
            }

            let libruby_map = (*self.libruby_map)
                .as_ref()
                .ok_or_else(|| format_err!("Missing libruby map. Please report this!"))?;
            let libruby_elf = self
                .libruby_elf
                .as_ref()
                .ok_or_else(|| format_err!("Missing libruby ELF. Please report this!"))?;
            elf_symbol_value(libruby_elf, symbol_name)
                .map(|addr| {
                    let load_header = elf_load_header(libruby_elf);
                    debug!("load header: {}", load_header);
                    libruby_map.start() + addr - load_header.vaddr as usize
                })
                .ok_or_else(|| format_err!("Could not find address for symbol {}", symbol_name))
        }
    }

    fn open_elf_file(pid: Pid, map: &MapRange) -> Result<elf::File> {
        // Read binaries from `/proc/PID/root` because the target process might be in a different
        // mount namespace. /proc/PID/root is the view of the filesystem that the target process
        // has. (see the proc man page for more)
        // So we read /usr/bin/ruby from /proc/PID/root/usr/bin/ruby
        let map_path = map
            .filename()
            .as_ref()
            .expect(&format!("[{}] map's pathname shouldn't be None", pid));
        #[cfg(target_os = "linux")]
        let elf_path = format!("/proc/{}/root{}", pid, map_path);
        #[cfg(target_os = "freebsd")]
        let elf_path = map_path;

        elf::File::open_path(&elf_path)
            .map_err(|_| format_err!("Couldn't open ELF file: {:?}", elf_path))
    }

    pub fn get_program_info(pid: Pid) -> Result<ProgramInfo> {
        let all_maps = get_process_maps(pid).map_err(|x| match x.kind() {
            std::io::ErrorKind::NotFound => AddressFinderError::NoSuchProcess(pid),
            std::io::ErrorKind::PermissionDenied => AddressFinderError::PermissionDenied(pid),
            _ => AddressFinderError::ProcMapsError(pid),
        })?;
        let ruby_map = Box::new(
            get_map(&all_maps, "bin/ruby")
                .ok_or_else(|| format_err!("Ruby map not found for PID: {}", pid))?,
        );
        let all_maps = get_process_maps(pid).unwrap();
        let ruby_elf = open_elf_file(pid, &ruby_map)?;
        let libruby_map = Box::new(get_map(&all_maps, "libruby"));
        let libruby_elf = match *libruby_map {
            Some(ref map) => Some(open_elf_file(pid, &map)?),
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
    use crate::core::address_finder::AddressFinderError;
    use crate::core::initialize::IsMaybeThreadFn;
    use anyhow::{format_err, Result};
    use proc_maps::win_maps::SymbolLoader;
    use proc_maps::{get_process_maps, MapRange, Pid};

    pub fn get_ruby_version_address(pid: u32) -> Result<usize> {
        get_symbol_address(pid, "ruby_version")
    }

    pub fn get_vm_address(pid: Pid, version: &str) -> Result<usize> {
        let symbol_name = if version >= "2.5.0" {
            "ruby_current_vm_ptr"
        } else {
            "ruby_current_vm"
        };

        get_symbol_address(pid, symbol_name)
    }

    pub fn current_thread_address(
        pid: u32,
        version: &str,
        _is_maybe_thread: IsMaybeThreadFn,
    ) -> Result<usize> {
        let symbol_name = if version >= "3.0.0" {
            panic!("Current thread address isn't directly accessible on ruby 3 and newer");
        } else if version >= "2.5.0" {
            "ruby_current_execution_context_ptr"
        } else {
            "ruby_current_thread"
        };

        get_symbol_address(pid, symbol_name)
    }

    pub fn get_ruby_global_symbols_address(pid: Pid, version: &str) -> Result<usize> {
        let symbol_name = if version >= "2.7.0" {
            "ruby_global_symbols"
        } else {
            "global_symbols"
        };
        get_symbol_address(pid, symbol_name)
    }

    fn get_symbol_address(pid: u32, symbol_name: &str) -> Result<usize> {
        let maps = get_process_maps(pid).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => AddressFinderError::NoSuchProcess(pid),
            std::io::ErrorKind::PermissionDenied => AddressFinderError::PermissionDenied(pid),
            _ => AddressFinderError::ProcMapsError(pid),
        })?;
        let ruby = get_ruby_binary(&maps)?;
        if let Some(ref filename) = ruby.filename() {
            let handler = SymbolLoader::new(pid as Pid)?;
            let _module = handler.load_module(filename)?; // need to keep this module in scope
            if let Ok((base, addr)) = handler.address_from_name(symbol_name) {
                // If we have a module base (ie from PDB), need to adjust by the offset
                // otherwise seems like we can take address directly
                let addr = if base == 0 {
                    addr
                } else {
                    ruby.start() as u64 + addr - base
                };
                return Ok(addr as usize);
            }
        }

        // try again loading symbols from the ruby DLL if found
        if let Some(libruby) = get_libruby_binary(&maps) {
            if let Some(ref filename) = libruby.filename() {
                let handler = SymbolLoader::new(pid as Pid)?;
                let _module = handler.load_module(filename)?;
                if let Ok((base, addr)) = handler.address_from_name(symbol_name) {
                    let addr = if base == 0 {
                        addr
                    } else {
                        libruby.start() as u64 + addr - base
                    };
                    return Ok(addr as usize);
                }
            }
        }

        Err(format_err!("failed to find {} symbol", symbol_name))
    }

    fn get_ruby_binary(maps: &[MapRange]) -> Result<&MapRange> {
        Ok(maps
            .iter()
            .find(|ref m| {
                if let Some(ref pathname) = m.filename() {
                    pathname.contains("ruby.exe")
                } else {
                    false
                }
            })
            .ok_or(format_err!("Couldn't find ruby binary"))?)
    }

    fn get_libruby_binary(maps: &[MapRange]) -> Option<&MapRange> {
        maps.iter().find(|ref m| {
            if let Some(ref pathname) = m.filename() {
                // pathname is something like "C:\Ruby24-x64\bin\x64-msvcrt-ruby240.dll"
                pathname.contains("-ruby") && pathname.ends_with(".dll")
            } else {
                false
            }
        })
    }
}
