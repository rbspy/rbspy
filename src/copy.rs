use std;
use read_process_memory::*;

/**
 * Utility functions for copying memory out of a process
 */

const MAX_COPY_LENGTH: usize = 1000000;

#[derive(Fail, Debug)]
pub enum MemoryCopyError {
    #[fail(display = "Permission denied when reading from process. Try again with sudo?")]
    PermissionDenied,
    #[fail(display = "Failed to copy memory address {:x}", _0)] Io(usize, #[cause] std::io::Error),
    #[fail(display = "Process isn't running")] ProcessEnded,
    #[fail(display = "Other")] Other,
    #[fail(display = "Too much memory requested when copying: {}", _0)] RequestTooLarge(usize),
    #[fail(display = "Tried to read invalid string")]
    InvalidStringError(#[cause] std::string::FromUtf8Error),
}

pub fn copy_vec<U, T>(addr: usize, length: usize, source: &T) -> Result<Vec<U>, MemoryCopyError>
where
    T: CopyAddress,
{
    let mut vec = copy_address_raw(addr, length * std::mem::size_of::<U>(), source)?;
    let capacity = vec.capacity() as usize / std::mem::size_of::<U>() as usize;
    let ptr = vec.as_mut_ptr() as *mut U;
    std::mem::forget(vec);
    unsafe { Ok(Vec::from_raw_parts(ptr, capacity, capacity)) }
}

pub fn copy_address_raw<T>(
    addr: usize,
    length: usize,
    source: &T,
) -> Result<Vec<u8>, MemoryCopyError>
where
    T: CopyAddress,
{
    debug!("copy_address_raw: addr: {:x}", addr as usize);
    if length > MAX_COPY_LENGTH {
        return Err(MemoryCopyError::RequestTooLarge(length));
    }
    let mut copy = vec![0; length];
    source.copy_address(addr as usize, &mut copy).map_err(|x| {
        if x.raw_os_error() == Some(3) {
            MemoryCopyError::ProcessEnded
        } else if x.raw_os_error() == Some(60) {
            // On Mac code 60 seems to more or less correspond to "process ended"
            MemoryCopyError::ProcessEnded
        } else if x.kind() == std::io::ErrorKind::PermissionDenied {
            MemoryCopyError::PermissionDenied
        } else {
            MemoryCopyError::Io(addr, x)
        }
    })?;
    Ok(copy)
}

pub fn copy_struct<U, T>(addr: usize, source: &T) -> Result<U, MemoryCopyError>
where
    T: CopyAddress,
{
    let result = copy_address_raw(addr as usize, std::mem::size_of::<U>(), source)?;
    let s: U = unsafe { std::ptr::read(result.as_ptr() as *const _) };
    Ok(s)
}
