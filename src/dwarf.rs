use leb128;
use rand;
use gimli;
use std::hash::BuildHasherDefault;
use fnv::FnvHasher;
use byteorder::{NativeEndian, ReadBytesExt};
use std::collections::HashMap;
use std::io::Cursor;

pub use self::obj::{get_executable_path};

#[cfg(target_os="linux")]
mod obj {
    extern crate elf;
    use gimli;

    use std::path::{Path, PathBuf};

    use super::Entry;
    use super::get_all_entries;

    /// The parsed object file type.
    type File = elf::File;

    pub fn get_executable_path(pid: usize) -> Result<PathBuf, String> {
        Ok(PathBuf::from(format!("/proc/{}/exe", pid)))
    }
}

#[cfg(target_os="macos")]
mod obj {
    extern crate gimli;
    extern crate libarchive;
    extern crate libarchive3_sys;
    extern crate libc;
    extern crate libproc;
    extern crate object;

    use self::object::Object;
    use self::libarchive::archive::{Entry as ArchiveEntry, Handle, ReadFormat};
    use self::libarchive::reader::{self, Reader};
    use self::libarchive3_sys::ffi;
    use std::ffi::CStr;
    use std::fs;
    use std::io::Read;
    use std::path::{Path, PathBuf};

    use super::{Entry, get_all_entries};

    pub fn get_executable_path(pid: usize) -> Result<PathBuf, String> {
        libproc::libproc::proc_pid::pidpath(pid as i32)
            .map(|path| PathBuf::from(&path))
    }
}
