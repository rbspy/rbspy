#[cfg(windows)]
use anyhow::format_err;
use anyhow::{Context, Error, Result};
use spytools::ProcessInfo;

use crate::core::process::{Pid, Process, ProcessRetry};
use crate::core::types::{MemoryCopyError, StackTrace};
use crate::StackFrame;
use std::collections::HashSet;
use symbolic::common::Name;
use symbolic::demangle::{Demangle, DemangleOptions};

pub struct RubySpy {
    process: Process,
    current_thread_addr_location: usize,
    ruby_vm_addr_location: usize,
    global_symbols_addr_location: Option<usize>,
    stack_trace_function: crate::core::types::StackTraceFn,
    native_profiling: bool,
    visited_ips: HashSet<u64>,
    #[cfg(any(windows, target_os = "linux"))]
    unwinder: remoteprocess::Unwinder,
    #[cfg(any(windows, target_os = "linux"))]
    symbolicator: remoteprocess::Symbolicator,
}

impl RubySpy {
    pub fn new(pid: Pid, force_version: Option<String>, native_profiling: bool) -> Result<Self> {
        #[cfg(all(windows, target_arch = "x86_64"))]
        if is_wow64_process(pid).context("check wow64 process")? {
            return Err(format_err!(
                "Unable to profile 32-bit Ruby with 64-bit rbspy"
            ));
        }
        let process =
            Process::new_with_retry(pid).context("Failed to find process. Is it running?")?;

        let process_info = ProcessInfo::new::<spytools::process::RubyProcessType>(&process)?;

        let (
            version,
            current_thread_addr_location,
            ruby_vm_addr_location,
            global_symbols_addr_location,
        ) = crate::core::address_finder::inspect_ruby_process(
            &process,
            &process_info,
            force_version,
        )
        .context("get ruby VM state")?;

        #[cfg(any(windows, target_os = "linux"))]
        let unwinder = process.unwinder()?;
        #[cfg(any(windows, target_os = "linux"))]
        let symbolicator = process.symbolicator()?;

        let stack_trace_function = crate::core::ruby_version::get_stack_trace_function(&version);
        Ok(Self {
            process,
            current_thread_addr_location,
            ruby_vm_addr_location,
            global_symbols_addr_location,
            stack_trace_function,
            native_profiling,
            visited_ips: HashSet::new(),
            #[cfg(any(windows, target_os = "linux"))]
            unwinder,
            #[cfg(any(windows, target_os = "linux"))]
            symbolicator,
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
        native_profiling: bool,
    ) -> Result<Self, Error> {
        let mut retries = 0;
        loop {
            let err = match Self::new(pid, force_version.clone(), native_profiling) {
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

    pub fn get_stack_trace(&mut self, lock_process: bool) -> Result<StackTrace> {
        match self.get_trace_from_current_thread(lock_process) {
            Ok(mut trace) => {
                return {
                    trace.pid = Some(self.process.pid);
                    Ok(trace)
                };
            }
            Err(e) => {
                if self.process.exe().is_err() {
                    return Err(MemoryCopyError::ProcessEnded.into());
                }
                return Err(e.into());
            }
        }
    }

    fn get_trace_from_current_thread(&mut self, lock_process: bool) -> Result<StackTrace> {
        if self.native_profiling && cfg!(any(windows, target_os = "linux")) {
            let thread = self
                .process
                .threads()
                .unwrap_or_default()
                .into_iter()
                .find(|t| t.active().unwrap_or_default());
            if let Some(thread) = thread {
                let lock = self
                    .process
                    .lock()
                    .context("locking process during stack trace retrieval")?;
                let ruby_trace = self.get_ruby_trace()?;

                let native_frames = self.get_native_frames(thread, lock)?;
                return Ok(merge_ruby_and_native_frames(ruby_trace, native_frames));
            }
        }
        let _lock;
        if lock_process {
            _lock = self
                .process
                .lock()
                .context("locking process during stack trace retrieval")?;
        }
        self.get_ruby_trace()
    }

    fn get_ruby_trace(&mut self) -> Result<StackTrace> {
        (&self.stack_trace_function)(
            self.current_thread_addr_location,
            self.ruby_vm_addr_location,
            self.global_symbols_addr_location,
            &self.process,
            self.process.pid,
        )
    }

    #[cfg(any(windows, target_os = "linux"))]
    fn get_native_frames(
        &mut self,
        thread: remoteprocess::Thread,
        lock: remoteprocess::Lock,
    ) -> Result<Vec<StackFrame>> {
        let mut native_ips: Vec<u64> = Vec::new();
        for ip in self.unwinder.cursor(&thread)? {
            let ip = ip?;
            native_ips.push(ip)
        }

        drop(lock);

        let mut native_frames: Vec<StackFrame> = Vec::new();
        for ip in native_ips {
            self.symbolicate(ip, &mut |sf| {
                let name: &str = sf
                    .function
                    .as_ref()
                    .map(|s| s as &str)
                    .unwrap_or_else(|| "cfunc (unnamed)".into());
                let name = Name::from(name);
                let name = name.try_demangle(DemangleOptions::complete());

                native_frames.push(StackFrame {
                    name: name.into(),
                    relative_path: sf.filename.clone().unwrap_or_default(),
                    absolute_path: None,
                    lineno: sf.line.unwrap_or_default() as u32,
                });
            })
            .unwrap_or_else(|_| {
                native_frames.push(StackFrame {
                    name: format!("0x{:x}", ip),
                    relative_path: "".to_string(),
                    absolute_path: None,
                    lineno: 0,
                });
            });
        }
        Ok(native_frames)
    }

    fn symbolicate(
        &mut self,
        ip: u64,
        func: &mut dyn FnMut(&remoteprocess::StackFrame),
    ) -> Result<(), remoteprocess::Error> {
        match self.symbolicator.symbolicate(ip, true, func) {
            Ok(_) => Ok(()),
            Err(e) => {
                // Do not reload more than once per IP which is not found
                if self.visited_ips.contains(&ip) {
                    return Err(e);
                }
                self.symbolicator.reload()?;
                self.visited_ips.insert(ip);
                self.symbolicator.symbolicate(ip, true, func)
            }
        }
    }
}

#[cfg(any(windows, target_os = "linux"))]
fn merge_ruby_and_native_frames(
    mut ruby_trace: StackTrace,
    native_frames: Vec<StackFrame>,
) -> StackTrace {
    let ruby_frames = ruby_trace.trace;
    let mut native_frames_iter = native_frames.iter();
    let mut frames = vec![];
    for (i, ruby_frame) in ruby_frames.iter().enumerate() {
        let is_cfunc = ruby_frame.name.contains("[c function]");
        let mut found_start_of_cfunc = false || (i == 0 && is_cfunc);
        if is_cfunc {
            while let Some(native_frame) = native_frames_iter.next() {
                if native_frame.name == "vm_call_cfunc_with_frame" {
                    native_frames_iter.next();
                    break;
                }

                // TODO: find a way to filter out internal ruby core functions, and make it configurable?
                if found_start_of_cfunc  {
                    frames.push(native_frame.clone());
                }

                if native_frame.name == "rb_vm_exec" {
                    found_start_of_cfunc = true;
                }
            }
        }
        frames.push(ruby_frame.clone())
    }
    ruby_trace.trace = frames;
    ruby_trace
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
    fn test_get_trace() {
        #[cfg(target_os = "macos")]
        if !nix::unistd::Uid::effective().is_root() {
            println!("Skipping test because we're not running as root");
            return;
        }

        let cmd = RubyScript::new("./ci/ruby-programs/infinite.rb");
        let pid = cmd.id() as Pid;
        let mut spy = RubySpy::retry_new(pid, 100, None, false).expect("couldn't initialize spy");
        spy.get_stack_trace(false)
            .expect("couldn't get stack trace");
    }

    #[test]
    fn test_get_trace_when_process_has_exited() {
        #[cfg(target_os = "macos")]
        if !nix::unistd::Uid::effective().is_root() {
            println!("Skipping test because we're not running as root");
            return;
        }

        let mut cmd = RubyScript::new("./ci/ruby-programs/infinite.rb");
        let mut getter = RubySpy::retry_new(cmd.id(), 100, None, false).unwrap();

        cmd.kill().expect("couldn't clean up test process");

        let mut i = 0;
        loop {
            match getter.get_stack_trace(true) {
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
