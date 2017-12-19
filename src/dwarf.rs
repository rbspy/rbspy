pub use self::obj::{get_executable_path};

#[cfg(target_os="linux")]
mod obj {
    use std::path::{PathBuf};

    pub fn get_executable_path(pid: usize) -> Result<PathBuf, String> {
        Ok(PathBuf::from(format!("/proc/{}/exe", pid)))
    }
}

#[cfg(target_os="macos")]
mod obj {
    extern crate libproc;

    use std::path::{PathBuf};

    pub fn get_executable_path(pid: usize) -> Result<PathBuf, String> {
        libproc::libproc::proc_pid::pidpath(pid as i32)
            .map(|path| PathBuf::from(&path))
    }
}
