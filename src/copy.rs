use std;
use libc::pid_t;
use read_process_memory::*;

/**
 * Utility functions for copying memory out of a process
 */

#[derive(Fail, Debug)]
pub enum MemoryCopyError {
    #[fail(display = "Permission denied when reading from process {}. Try again with sudo?", _0)]
    PermissionDenied(pid_t),
    #[fail(display = "Failed to copy memory address {:x} from PID {}", _1, _0)]
    Io(pid_t, usize, #[cause] std::io::Error),
    #[fail(display = "Process isn't running")] ProcessEnded,
    #[fail(display = "Other")] Other,
    #[fail(display = "Tried to read invalid string")]
    InvalidStringError(#[cause] std::string::FromUtf8Error),
}

pub fn copy_vec<T>(
    addr: usize,
    length: usize,
    source_pid: pid_t,
) -> Result<Vec<T>, MemoryCopyError> {
    let mut vec = copy_address_raw(addr, length * std::mem::size_of::<T>(), source_pid)?;
    let capacity = vec.capacity() as usize / std::mem::size_of::<T>() as usize;
    let ptr = vec.as_mut_ptr() as *mut T;
    std::mem::forget(vec);
    unsafe { Ok(Vec::from_raw_parts(ptr, capacity, capacity)) }
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
        } else if x.kind() == std::io::ErrorKind::PermissionDenied {
            MemoryCopyError::PermissionDenied(source_pid)
        } else {
            MemoryCopyError::Io(source_pid, addr, x)
        }
    })?;
    Ok(copy)
}

pub fn copy_struct<U>(addr: usize, source_pid: pid_t) -> Result<U, MemoryCopyError> {
    let result = copy_address_raw(addr as usize, std::mem::size_of::<U>(), source_pid)?;
    let s: U = unsafe { std::ptr::read(result.as_ptr() as *const _) };
    Ok(s)
}
