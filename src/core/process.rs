use anyhow::Result;
pub use remoteprocess::{Pid, Process, ProcessMemory};

pub trait ProcessRetry {
    fn new_with_retry(pid: Pid) -> Result<Process>;
}

impl ProcessRetry for remoteprocess::Process {
    // It can take a moment for the ruby process to spin up, so new_with_retry automatically
    // retries for a few seconds. This delay mostly seems to affect macOS and Windows and is
    // especially common in CI environments.
    fn new_with_retry(pid: Pid) -> Result<Process> {
        let retry_interval = std::time::Duration::from_millis(10);
        let mut retries = 500;
        loop {
            match Process::new(pid) {
                Ok(p) => return Ok(p),
                Err(e) => {
                    if retries == 0 {
                        return Err(e)?;
                    }
                    std::thread::sleep(retry_interval);
                    retries -= 1;
                }
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::ops::{Deref, DerefMut};
    use std::process::Child;

    pub struct ManagedProcess(pub Child);

    impl Drop for ManagedProcess {
        fn drop(&mut self) {
            match self.0.kill() {
                Err(e) => debug!("Failed to kill process {}: {:?}", self.0.id(), e),
                _ => (),
            }
            match self.0.wait() {
                Err(e) => debug!("Failed to wait for process {}: {:?}", self.0.id(), e),
                _ => (),
            }
        }
    }

    impl Deref for ManagedProcess {
        type Target = Child;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for ManagedProcess {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
}
