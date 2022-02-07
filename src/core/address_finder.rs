use anyhow::{anyhow, format_err, Context, Error, Result};
use remoteprocess::{Process, ProcessMemory};
use semver::Version;
use spytools::ProcessInfo;

/// Inspect a running Ruby process, finding key memory addresses that are needed for profiling
pub fn inspect_ruby_process(
    process: &Process,
    process_info: &ProcessInfo,
    force_version: Option<String>,
) -> Result<(Version, usize, usize, Option<usize>)> {
    let version = match force_version {
        Some(ref v) => {
            info!("Assuming Ruby version is {}", v);
            Version::parse(v)?
        }
        None => {
            let version_addr = process_info
                .get_symbol(&ruby_version_symbol())
                .context("Failed to locate Ruby version symbol");
            if let Err(e) = version_addr {
                match e.root_cause().downcast_ref::<std::io::Error>() {
                    Some(root_cause)
                        if root_cause.kind() == std::io::ErrorKind::PermissionDenied =>
                    {
                        return Err(e.context("Failed to initialize due to a permissions error. If you are running rbspy as a normal (non-root) user, please try running it again with `sudo --preserve-env !!`. If you are running it in a container, e.g. with Docker or Kubernetes, make sure that your container has been granted the SYS_PTRACE capability. See the rbspy documentation for more details."));
                    }
                    _ => {}
                }
                return Err(anyhow::format_err!("Couldn't get ruby version: {:?}", e));
            };
            let version_addr = version_addr.unwrap();
            let raw_version: [u8; 15] = process
                .copy_struct(*version_addr as usize)
                .context("Failed to read Ruby version symbol")?;
            let raw_version: Vec<u8> = match raw_version.iter().position(|c| *c == 0) {
                Some(pos) => raw_version[0..=pos].to_vec(),
                None => {
                    return Err(anyhow!(
                        "Version data doesn't seem to contain a valid string"
                    ))
                }
            };
            let version = std::ffi::CStr::from_bytes_with_nul(&raw_version)?
                .to_str()
                .context("Failed to convert ruby version from raw string")?
                .to_owned();
            let version = Version::parse(&version)?;
            info!("Found ruby version {}", version);
            version
        }
    };

    let vm_address = match process_info.get_symbol(&ruby_current_vm_symbol(&version)) {
        Some(addr) => *addr as usize,
        None => return Err(anyhow::format_err!("Couldn't find Ruby VM address")),
    };
    let current_thread_address =
        get_current_thread_address(process_info, process, &version, vm_address)?;
    let global_symbols_address = match process_info.get_symbol(&ruby_globals_symbol(&version)) {
        Some(addr) => Some(*addr as usize),
        // The global symbols address lookup is allowed to fail (e.g. on older rubies)
        None => None,
    };

    let addresses_status = format!(
        "version: {:x?}\n\
        current thread address: {:#x?}\n\
        VM address: {:#x?}\n\
        global symbols address: {:#x?}\n",
        version, &current_thread_address, &vm_address, global_symbols_address
    );

    info!("Ruby VM addresses: {}", addresses_status);
    return Ok((
        version,
        current_thread_address,
        vm_address,
        global_symbols_address,
    ));
}

fn get_current_thread_address(
    process_info: &ProcessInfo,
    process: &remoteprocess::Process,
    version: &Version,
    vm_address: usize,
) -> Result<usize> {
    if *version >= Version::new(3, 0, 0) {
        // Current thread is not directly accessible on Ruby 3+, so get it from the VM
        let get_execution_context = crate::core::ruby_version::get_execution_context(&version);
        return get_execution_context(0, vm_address, process);
    }

    let symbol = ruby_execution_context_symbol(&version);

    // get the address of the current ruby thread from loaded symbols if we can
    // (this tends to be faster than scanning through the bss section)
    if let Some(&addr) = process_info.get_symbol(&symbol) {
        #[cfg(windows)]
        return Ok(addr as usize);

        #[cfg(not(windows))]
        match check_thread_addresses(
            &[addr as usize],
            &process_info.maps,
            process,
            crate::core::ruby_version::is_maybe_thread_function(&version),
        ) {
            Ok(addr) => return Ok(addr),
            Err(e) => {
                warn!(
                    "Thread address from {} symbol is invalid {:016x}: {:?}",
                    symbol, addr, e
                );
            }
        };
    }
    info!("Failed to get current thread address from symbols, so scanning BSS section from main binary");

    // Try scanning the BSS section of the binary for things that might be a thread
    let err = if let Some(ref binary) = process_info.binary {
        match get_thread_address_from_binary(
            binary,
            &process_info.maps,
            process,
            crate::core::ruby_version::is_maybe_thread_function(&version),
        ) {
            Ok(addr) => return Ok(addr),
            Err(err) => Some(Err(err)),
        }
    } else {
        None
    };
    // Before giving up, try again if there is a shared library
    if let Some(ref library) = process_info.library {
        info!("Failed to get current thread from binary BSS, so scanning libruby BSS");
        match get_thread_address_from_binary(
            library,
            &process_info.maps,
            process,
            crate::core::ruby_version::is_maybe_thread_function(&version),
        ) {
            Ok(addr) => return Ok(addr),
            Err(lib_err) => Err(err).unwrap_or(Err(lib_err)),
        }
    } else {
        err.expect("Both ruby and libruby are invalid.")
    }
}

use proc_maps::MapRange;
use spytools::binary_parser::BinaryInfo;

fn get_thread_address_from_binary(
    binary: &BinaryInfo,
    maps: &[MapRange],
    process: &remoteprocess::Process,
    is_maybe_thread: crate::core::types::IsMaybeThreadFn,
) -> Result<usize, Error> {
    // We're going to scan the BSS/data section for things, and try to narrowly scan things that
    // look like pointers to a ruby thread
    let bss = process.copy(binary.bss_addr as usize, binary.bss_size as usize)?;

    #[allow(clippy::cast_ptr_alignment)]
    let addrs = unsafe {
        std::slice::from_raw_parts(
            bss.as_ptr() as *const usize,
            bss.len() / std::mem::size_of::<usize>(),
        )
    };
    check_thread_addresses(addrs, maps, process, is_maybe_thread)
}

// Checks whether a block of memory (from BSS/.data etc) contains pointers that are pointing
// to a valid thread
fn check_thread_addresses(
    addrs: &[usize],
    maps: &[MapRange],
    process: &remoteprocess::Process,
    is_maybe_thread: crate::core::types::IsMaybeThreadFn,
) -> Result<usize, Error> {
    // On windows, we can't just check if a pointer is valid by looking to see if it points
    // to something in the virtual memory map. Brute-force it instead
    #[cfg(windows)]
    fn maps_contain_addr(_: usize, _: &[MapRange]) -> bool {
        true
    }

    #[cfg(not(windows))]
    use proc_maps::maps_contain_addr;

    fn check(
        addrs: &[usize],
        maps: &[MapRange],
        process: &remoteprocess::Process,
        is_maybe_thread: crate::core::types::IsMaybeThreadFn,
    ) -> Result<usize, Error> {
        for &addr in addrs {
            if maps_contain_addr(addr, maps) {
                let thread_addr = match process.copy_struct(addr) {
                    Ok(thread_addr) => thread_addr,
                    Err(_) => continue,
                };

                if is_maybe_thread(thread_addr, addr, &process, maps) {
                    return Ok(addr);
                }
            }
        }
        Err(format_err!(
            "Failed to find the current ruby thread in the .data section"
        ))
    }

    check(addrs, maps, process, is_maybe_thread)
}

fn ruby_version_symbol() -> String {
    "ruby_version".to_string()
}

fn ruby_globals_symbol(version: &Version) -> String {
    if *version >= Version::new(2, 7, 0) {
        "ruby_global_symbols".to_string()
    } else {
        "global_symbols".to_string()
    }
}

fn ruby_current_vm_symbol(version: &Version) -> String {
    if *version >= Version::new(2, 5, 0) {
        "ruby_current_vm_ptr".to_string()
    } else {
        "ruby_current_vm".to_string()
    }
}

fn ruby_execution_context_symbol(version: &Version) -> String {
    if *version >= Version::new(3, 0, 0) {
        panic!("Current thread is not directly accessible on ruby 3+");
    } else if *version >= Version::new(2, 5, 0) {
        "ruby_current_execution_context_ptr".to_string()
    } else {
        "ruby_current_thread".to_string()
    }
}
