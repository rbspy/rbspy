#[cfg(windows)]
use anyhow::format_err;
use anyhow::{Context, Error, Result};
use spytools::ProcessInfo;

use crate::core::process::{Pid, Process, ProcessRetry};
use crate::core::types::{MemoryCopyError, StackTrace};

use super::address_finder::RubyVM;

pub struct RubySpy {
    process: Process,
    vm: super::address_finder::RubyVM,
    on_cpu_only: bool,
}

impl RubySpy {
    pub fn new(pid: Pid, force_version: Option<String>, on_cpu_only: bool) -> Result<Self> {
        #[cfg(all(windows, target_arch = "x86_64"))]
        if is_wow64_process(pid).context("check wow64 process")? {
            return Err(format_err!(
                "Unable to profile 32-bit Ruby with 64-bit rbspy"
            ));
        }
        let process =
            Process::new_with_retry(pid).context("Failed to find process. Is it running?")?;

        let process_info = ProcessInfo::new::<spytools::process::RubyProcessType>(&process)?;

        let vm = crate::core::address_finder::inspect_ruby_process(
            &process,
            &process_info,
            force_version,
        )
        .context("get ruby VM state")?;

        Ok(Self {
            process,
            vm,
            on_cpu_only,
        })
    }

    /// Creates a RubySpy object, retrying up to max_retries times.
    ///
    /// Retrying is useful for a few reasons:
    /// a) Sometimes rbenv takes a while to exec the right Ruby binary.
    /// b) Dynamic linking takes a nonzero amount of time, so even after the right Ruby binary is
    ///    exec'd we still need to wait for the right memory maps to be in place
    /// c) On Mac, it can take a while between when the process is 'exec'ed and when we can get a
    ///    Mach port for the process, which is how rbspy communicates with it
    pub fn retry_new(
        pid: Pid,
        max_retries: u64,
        force_version: Option<String>,
        on_cpu_only: bool,
    ) -> Result<Self, Error> {
        let mut retries = 0;
        loop {
            let err = match Self::new(pid, force_version.clone(), on_cpu_only) {
                Ok(mut process) => {
                    // verify that we can load a stack trace before returning success
                    match process.get_stack_trace(false) {
                        Ok(_) => return Ok(process),
                        Err(err) => err,
                    }
                }
                Err(err) => err,
            };

            // If we failed, retry a couple times before returning the last error
            retries += 1;
            if retries >= max_retries {
                return Err(err);
            }
            info!(
                "Failed to connect to process; will retry. Last error: {}",
                err
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    pub fn get_stack_trace(&mut self, lock_process: bool) -> Result<Option<StackTrace>> {
        // First, try OS-specific checks to determine whether the process is on CPU or not.
        // This comes before locking the process because in most operating systems locking
        // will stop the process and interfere with the on-CPU check.
        if self.on_cpu_only && !self.is_on_cpu()? {
            return Ok(None);
        }
        match self.get_trace_from_current_thread(lock_process) {
            Ok(Some(mut trace)) => {
                return {
                    trace.pid = Some(self.process.pid);
                    Ok(Some(trace))
                };
            }
            Ok(None) => Ok(None),
            Err(e) => {
                if self.process.exe().is_err() {
                    return Err(MemoryCopyError::ProcessEnded.into());
                }
                return Err(e.into());
            }
        }
    }

    fn get_trace_from_current_thread(&self, lock_process: bool) -> Result<Option<StackTrace>> {
        let _lock;
        if lock_process {
            _lock = self
                .process
                .lock()
                .context("locking process during stack trace retrieval")?;
        }

        (&self.vm.ruby_version.get_stack_trace_fn)(
            self.vm.current_thread_addr_location,
            self.vm.ruby_vm_addr_location,
            self.vm.global_symbols_addr_location,
            &self.process,
            self.process.pid,
            self.on_cpu_only,
        )
    }

    fn is_on_cpu(&self) -> Result<bool> {
        if self
            .process
            .threads()?
            .iter()
            .any(|thread| thread.active().unwrap_or(false))
        {
            return Ok(true);
        }

        Ok(false)
    }

    pub fn inspect(&self) -> &RubyVM {
        &self.vm
    }
}

#[cfg(all(windows, target_arch = "x86_64"))]
fn is_wow64_process(pid: Pid) -> Result<bool> {
    use std::os::windows::io::RawHandle;
    use winapi::shared::minwindef::{BOOL, FALSE, PBOOL};
    use winapi::um::processthreadsapi::OpenProcess;
    use winapi::um::winnt::PROCESS_QUERY_INFORMATION;
    use winapi::um::wow64apiset::IsWow64Process;

    let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, pid) };

    if handle == (0 as RawHandle) {
        return Err(format_err!(
            "Unable to fetch process handle for process {}",
            pid
        ));
    }

    let mut is_wow64: BOOL = 0;

    if unsafe { IsWow64Process(handle, &mut is_wow64 as PBOOL) } == FALSE {
        return Err(format_err!("Could not determine process bitness! {}", pid));
    }

    Ok(is_wow64 != 0)
}

#[cfg(test)]
mod tests {
    use crate::core::process::tests::RubyScript;
    #[cfg(any(unix, windows))]
    use crate::core::process::Pid;
    use crate::core::ruby_spy::RubySpy;
    #[cfg(target_os = "macos")]
    use std::process::Command;

    #[test]
    #[cfg(all(windows, target_arch = "x86_64"))]
    fn test_is_wow64_process() {
        let programs = vec![
            "C:\\Program Files (x86)\\Internet Explorer\\iexplore.exe",
            "C:\\Program Files\\Internet Explorer\\iexplore.exe",
        ];

        let results: Vec<bool> = programs
            .iter()
            .map(|path| {
                let mut cmd = std::process::Command::new(path)
                    .spawn()
                    .expect("iexplore failed to start");

                let is_wow64 = crate::core::ruby_spy::is_wow64_process(cmd.id()).unwrap();
                cmd.kill().expect("couldn't clean up test process");
                is_wow64
            })
            .collect();

        assert_eq!(results, vec![true, false]);
    }

    #[test]
    fn test_initialize_with_nonexistent_process() {
        match RubySpy::new(65535, None, false) {
            Ok(_) => assert!(
                false,
                "Expected error because process probably doesn't exist"
            ),
            _ => {}
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_initialize_with_disallowed_process() {
        match RubySpy::new(1, None, false) {
            Ok(_) => assert!(
                false,
                "Expected error because we shouldn't be allowed to profile the init process"
            ),
            _ => {}
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_disallowed_process() {
        // getting the ruby version isn't allowed on Mac if the process isn't running as root
        let mut process = Command::new("/usr/bin/ruby").spawn().unwrap();
        let pid = process.id() as Pid;

        match RubySpy::new(pid, None, false) {
            Ok(_) => assert!(
                false,
                "Expected error because we shouldn't be allowed to profile system processes"
            ),
            _ => {}
        }

        process.kill().expect("couldn't clean up test process");
    }

    #[test]
    fn test_get_trace_on_cpu() {
        #[cfg(target_os = "macos")]
        if !nix::unistd::Uid::effective().is_root() {
            println!("Skipping test because we're not running as root");
            return;
        }

        let cmd = RubyScript::new("./ci/ruby-programs/infinite_on_cpu.rb");
        let pid = cmd.id() as Pid;
        let mut spy = RubySpy::retry_new(pid, 100, None, false).expect("couldn't initialize spy");
        spy.get_stack_trace(false)
            .expect("couldn't get stack trace");
    }

    #[test]
    fn test_get_trace_off_cpu() {
        #[cfg(target_os = "macos")]
        if !nix::unistd::Uid::effective().is_root() {
            println!("Skipping test because we're not running as root");
            return;
        }

        let coordination_dir = tempfile::tempdir().unwrap();
        let coordination_dir_name = coordination_dir.path().to_str().unwrap();
        let coordination_file_path = format!("{}/ready", coordination_dir_name);
        let cp = std::path::Path::new(&coordination_file_path);
        assert!(!cp.exists());

        let cmd = RubyScript::new_with_args(
            "./ci/ruby-programs/infinite_off_cpu.rb",
            &[coordination_file_path.clone()],
        );
        let pid = cmd.id() as Pid;

        loop {
            if cp.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let mut spy = RubySpy::retry_new(pid, 100, None, true).expect("couldn't initialize spy");
        let trace = spy
            .get_stack_trace(false)
            .expect("couldn't get stack trace");
        assert!(trace.is_none());
    }

    #[test]
    fn test_get_trace_when_process_has_exited() {
        #[cfg(target_os = "macos")]
        if !nix::unistd::Uid::effective().is_root() {
            println!("Skipping test because we're not running as root");
            return;
        }

        let mut cmd = RubyScript::new("./ci/ruby-programs/infinite_on_cpu.rb");
        let mut getter = RubySpy::retry_new(cmd.id(), 100, None, false).unwrap();

        cmd.kill().expect("couldn't clean up test process");

        let mut i = 0;
        loop {
            match getter.get_stack_trace(false) {
                Err(e) => {
                    if let Some(crate::core::types::MemoryCopyError::ProcessEnded) =
                        e.downcast_ref()
                    {
                        // This is the expected error
                        return;
                    }
                }
                _ => {}
            };
            std::thread::sleep(std::time::Duration::from_millis(100));
            i += 1;
            if i > 50 {
                panic!("Didn't get ProcessEnded in a reasonable amount of time");
            }
        }
    }
}
