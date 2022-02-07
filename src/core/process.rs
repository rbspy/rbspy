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
        let mut retries = 200;
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
    use crate::core::process::{Pid, Process};
    use std::ops::{Deref, DerefMut};
    use std::process::{Child, Command};

    use super::ProcessRetry;

    pub struct RubyScript {
        pub child: Child,
        pub process: Process,
    }

    impl RubyScript {
        pub fn new(ruby_script_path: &str) -> Self {
            let which = if cfg!(target_os = "windows") {
                "C:\\Windows\\System32\\WHERE.exe"
            } else {
                "/usr/bin/which"
            };

            let output = Command::new(which)
                .arg("ruby")
                .output()
                .expect("failed to execute process");

            let ruby_binary_path = String::from_utf8(output.stdout).unwrap();

            let ruby_binary_path_str = ruby_binary_path
                .lines()
                .next()
                .expect("failed to execute ruby process");

            let child = Command::new(ruby_binary_path_str)
                .arg(ruby_script_path)
                .stdin(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            let process = Process::new_with_retry(child.id() as _).unwrap();
            RubyScript { child, process }
        }

        pub fn id(&self) -> Pid {
            self.child.id() as _
        }

        pub fn kill(&mut self) -> std::io::Result<()> {
            self.child.kill()
        }
    }

    impl Drop for RubyScript {
        fn drop(&mut self) {
            match self.child.kill() {
                Err(e) => debug!("Failed to kill process {}: {:?}", self.id(), e),
                _ => (),
            }
            match self.child.wait() {
                Err(e) => debug!("Failed to wait for process {}: {:?}", self.id(), e),
                _ => (),
            }
        }
    }

    impl Deref for RubyScript {
        type Target = Process;

        fn deref(&self) -> &Self::Target {
            &self.process
        }
    }

    impl DerefMut for RubyScript {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.process
        }
    }
}
