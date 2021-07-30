use crate::core::address_finder;
#[cfg(target_os = "linux")]
use crate::core::address_finder::*;
use crate::core::ruby_version;
use crate::core::types::{MemoryCopyError, Pid, Process, ProcessMemory, ProcessRetry, StackTrace};
use proc_maps::MapRange;

use anyhow::{Context, Result};
use libc::c_char;

use std::time::Duration;

/**
 * Initialization code for the profiler.
 *
 * The only public function here is `initialize`, which returns a struct which you can
 * call `get_trace()` on to get a stack trace.
 *
 * Core responsibilities of this code:
 *   * Get the Ruby version
 *   * Get the address of the current thread
 *   * Find the right stack trace function for the Ruby version we found
 *   * Package all that up into a struct that the user can use to get stack traces.
 */
pub fn initialize(pid: Pid, lock_process: bool) -> Result<StackTraceGetter> {
    let (
        process,
        current_thread_addr_location,
        ruby_vm_addr_location,
        global_symbols_addr_location,
        stack_trace_function,
    ) = get_process_ruby_state(pid).context("get ruby VM state")?;

    Ok(StackTraceGetter {
        process,
        current_thread_addr_location,
        ruby_vm_addr_location,
        global_symbols_addr_location,
        stack_trace_function,
        reinit_count: 0,
        lock_process,
    })
}

// Use a StackTraceGetter to get stack traces
pub struct StackTraceGetter {
    process: Process,
    current_thread_addr_location: usize,
    ruby_vm_addr_location: usize,
    global_symbols_addr_location: Option<usize>,
    stack_trace_function: StackTraceFn,
    reinit_count: u32,
    lock_process: bool,
}

impl StackTraceGetter {
    pub fn get_trace(&mut self) -> Result<StackTrace> {
        match self.get_trace_from_current_thread() {
            Ok(mut trace) => {
                return {
                    /* This is a spike to enrich the trace with the pid.
                     * This is needed, because remoteprocess' ProcessMemory
                     * trait does not expose pid.
                     */
                    trace.pid = Some(self.process.pid);
                    Ok(trace)
                };
            }
            Err(MemoryCopyError::InvalidAddressError(addr))
                if addr == self.current_thread_addr_location => {}
            Err(e) => return Err(e.into()),
        }
        debug!("Thread address location invalid, reinitializing");
        self.reinitialize().context("reinitialize")?;
        Ok(self
            .get_trace_from_current_thread()
            .context("get trace from current thread")?)
    }

    fn get_trace_from_current_thread(&self) -> Result<StackTrace, MemoryCopyError> {
        let stack_trace_function = &self.stack_trace_function;

        let try_lock = |process: &Process| {
            process
                 .lock()
                 .context("locking process during stack trace retrieval")
                 .or_else(|e| {
                     if let Some(source) = e.source() {
                         match source.downcast_ref::<remoteprocess::Error>().ok_or(source) {
                             Ok(remoteprocess::Error::IOError(e)) => {
                                 match e.kind() {
                                     std::io::ErrorKind::NotFound => {
                                         debug!("Process {} was not found : {:?}", self.process.pid, e);
                                         return Err(MemoryCopyError::ProcessEnded)
                                     }
                                     #[cfg(target_os = "windows")]
                                     std::io::ErrorKind::PermissionDenied => {
                                         return Err(MemoryCopyError::ProcessEnded)
                                     }
                                     _ => {}
                                 };
                             }
                             #[cfg(target_os = "linux")]
                             Ok(remoteprocess::Error::NixError(e)) => match e {
                                 nix::Error::Sys(nix::errno::Errno::EPERM) => {
                                     debug!("EPERM for process {} : {:?}", self.process.pid, e);
                                     return Err(MemoryCopyError::ProcessNotLocked);
                                 }
                                 nix::Error::Sys(nix::errno::Errno::ESRCH) => {
                                     match Process::new(self.process.pid) {
                                         Ok(_) => {
                                             debug!("Process {} is actually alive, spurious ESRCH detected", self.process.pid);
                                             return Err(MemoryCopyError::ProcessNotLocked);
                                         },
                                         Err(e) => {
                                             debug!("ESRCH Process {} is not alive: {:?}", self.process.pid, e);
                                             return Err(MemoryCopyError::ProcessEnded);
                                         }
                                     }
                                 }
                                 _ => {}
                             },
                             _ => {}
                         }
                     }
                     debug!("Unhandled error {:?}", e);
                     Err(e.into())
                 })
        };

        let _lock;
        if self.lock_process {
            let mut retries = 20;
            loop {
                match try_lock(&self.process) {
                    Ok(l) => {
                        debug!(
                            "Was able to lock process {}, {} retries remained ",
                            self.process.pid, retries
                        );
                        _lock = l;
                        break;
                    }
                    Err(e) => match e {
                        MemoryCopyError::ProcessNotLocked => {
                            if retries == 0 {
                                debug!("Exhausted retries to lock process");
                                return Err(e)?;
                            }
                            debug!("Retrying lock");
                            retries -= 1;
                        }
                        _ => {
                            debug!("Error is not process lock error {:?}", e);
                            return Err(e)?;
                        }
                    },
                }
            }
        }

        stack_trace_function(
            self.current_thread_addr_location,
            self.ruby_vm_addr_location,
            self.global_symbols_addr_location,
            &self.process,
            self.process.pid,
        )
    }

    fn reinitialize(&mut self) -> Result<()> {
        let (
            process,
            current_thread_addr_location,
            ruby_vm_addr_location,
            ruby_global_symbols_addr_location,
            stack_trace_function,
        ) = get_process_ruby_state(self.process.pid).context("get ruby VM state")?;

        self.process = process;
        self.current_thread_addr_location = current_thread_addr_location;
        self.ruby_vm_addr_location = ruby_vm_addr_location;
        self.global_symbols_addr_location = ruby_global_symbols_addr_location;
        self.stack_trace_function = stack_trace_function;
        self.reinit_count += 1;

        Ok(())
    }
}

pub type IsMaybeThreadFn = Box<dyn Fn(usize, usize, &Process, &[MapRange]) -> bool>;

// Everything below here is private

type StackTraceFn =
    Box<dyn Fn(usize, usize, Option<usize>, &Process, Pid) -> Result<StackTrace, MemoryCopyError>>;

fn get_process_ruby_state(
    pid: Pid,
) -> Result<(Process, usize, usize, Option<usize>, StackTraceFn)> {
    /* This retry loop exists because:
     * a) Sometimes rbenv takes a while to exec the right Ruby binary.
     * b) Dynamic linking takes a nonzero amount of time, so even after the right Ruby binary is
     *    exec'd we still need to wait for the right memory maps to be in place
     * c) On Mac, it can take a while between when the process is 'exec'ed and when we can get a
     *    Mach port for the process (which we need to communicate with it)
     *
     * So we just keep retrying every millisecond and hope eventually it works
     */
    let mut i = 0;
    loop {
        let process = Process::new_with_retry(pid);
        if let Err(e) = process {
            return Err(anyhow::format_err!(
                "Couldn't find process with PID {}. Is it running? Error was {:?}",
                pid,
                e
            ));
        }
        let process = process.unwrap();

        let version = get_ruby_version(&process).context("get Ruby version");
        if let Err(e) = version {
            debug!(
                "[{}] Trying again to get ruby version. Last error was: {:?}",
                process.pid, e
            );
            i += 1;
            if i > 100 {
                match e.root_cause().downcast_ref::<std::io::Error>() {
                    Some(root_cause)
                        if root_cause.kind() == std::io::ErrorKind::PermissionDenied =>
                    {
                        return Err(e.context("Failed to initialize due to a permissions error. If you are running rbspy as a normal (non-root) user, please try running it again with `sudo --preserve-env !!`. If you are running it in a container, e.g. with Docker or Kubernetes, make sure that your container has been granted the SYS_PTRACE capability. See the rbspy documentation for more details."));
                    }
                    _ => {}
                }
                return Err(anyhow::format_err!("Couldn't get ruby version: {:?}", e));
            }
            std::thread::sleep(Duration::from_millis(1));
            continue;
        };
        let version = version.unwrap();

        let current_thread_address = if version.as_str() >= "3.0.0" {
            // There's no symbol for the current thread address on ruby 3+, so we look it up
            // dynamically later
            Ok(0)
        } else {
            let is_maybe_thread = is_maybe_thread_function(&version);
            address_finder::current_thread_address(process.pid, &version, is_maybe_thread)
        };
        let vm_address = address_finder::get_vm_address(process.pid, &version);
        let global_symbols_address =
            address_finder::get_ruby_global_symbols_address(process.pid, &version);

        let addresses_status = format!("version: {:x?}\ncurrent thread address: {:#x?}\nVM address: {:#x?}\nglobal symbols address: {:#x?}\n", version, &current_thread_address, &vm_address, global_symbols_address);

        // The global symbols address lookup is allowed to fail (e.g. on older rubies)
        if (&current_thread_address).is_ok() && (&vm_address).is_ok() {
            debug!("{}", addresses_status);
            return Ok((
                process,
                current_thread_address.unwrap(),
                vm_address.unwrap(),
                global_symbols_address.ok(),
                get_stack_trace_function(&version),
            ));
        }

        if i > 100 {
            return Err(anyhow::format_err!(
                "Couldn't get ruby process state. Please open a GitHub issue for this and include the following information:\n{}",
                addresses_status
            ));
        }

        // if we didn't get the required addresses, sleep for a short time and try again
        debug!("[{}] Trying again to get ruby process state", process.pid);
        i += 1;
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn get_ruby_version(process: &Process) -> Result<String> {
    let addr = address_finder::get_ruby_version_address(process.pid)
        .context("get_ruby_version_address")?;
    let x: [c_char; 15] = process.copy_struct(addr).context("retrieve ruby version")?;
    Ok(unsafe {
        std::ffi::CStr::from_ptr(x.as_ptr() as *mut c_char)
            .to_str()?
            .to_owned()
    })
}

#[test]
#[cfg(target_os = "linux")]
fn test_initialize_with_nonexistent_process() {
    let process = Process::new(10000).expect("Failed to initialize process");
    let version = get_ruby_version(&process);
    match version
        .unwrap_err()
        .root_cause()
        .downcast_ref::<AddressFinderError>()
        .unwrap()
    {
        &AddressFinderError::NoSuchProcess(10000) => {}
        _ => assert!(false, "Expected NoSuchProcess error"),
    }
}

#[test]
#[cfg(target_os = "linux")]
fn test_initialize_with_disallowed_process() {
    let process = Process::new(1).expect("Failed to initialize process");
    let version = get_ruby_version(&process);
    match version
        .unwrap_err()
        .root_cause()
        .downcast_ref::<AddressFinderError>()
        .unwrap()
    {
        &AddressFinderError::PermissionDenied(1) => {}
        _ => assert!(false, "Expected PermissionDenied error"),
    }
}

#[test]
#[cfg(target_os = "linux")]
fn test_current_thread_address() {
    let mut process = std::process::Command::new("ruby")
        .arg("./ci/ruby-programs/infinite.rb")
        .spawn()
        .unwrap();
    let pid = process.id() as Pid;
    let remoteprocess = Process::new(pid).expect("Failed to initialize process");
    let version;
    let mut i = 0;
    loop {
        // It can take a moment for the process to become ready, so retry as needed
        let r = get_ruby_version(&remoteprocess);
        if r.is_ok() {
            version = r.unwrap();
            break;
        }
        if i > 100 {
            panic!("couldn't get ruby version");
        }
        i += 1;
        std::thread::sleep(Duration::from_millis(1));
    }
    if version >= String::from("3.0.0") {
        // We won't be able to get the thread address directly, so skip this
        return;
    }

    let is_maybe_thread = is_maybe_thread_function(&version);
    let result = address_finder::current_thread_address(pid, &version, is_maybe_thread);
    result.expect("unexpected error");
    process.kill().unwrap();
}

#[test]
#[cfg(target_os = "linux")]
fn test_get_trace() {
    // Test getting a stack trace from a real running program using system Ruby
    let mut process = std::process::Command::new("ruby")
        .arg("./ci/ruby-programs/infinite.rb")
        .spawn()
        .unwrap();
    let pid = process.id() as Pid;
    let mut getter = initialize(pid, true).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    let trace = getter.get_trace();
    assert!(trace.is_ok());
    assert_eq!(trace.unwrap().pid, Some(pid));
    process.kill().unwrap();
}

#[test]
#[cfg(target_os = "linux")]
fn test_get_exec_trace() {
    use std::io::Write;

    // Test collecting stack samples across an exec call
    let mut process = std::process::Command::new("ruby")
        .arg("./ci/ruby-programs/ruby_exec.rb")
        .arg("ruby")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let pid = process.id() as Pid;
    let mut getter = initialize(pid, true).expect("initialize");

    std::thread::sleep(std::time::Duration::from_millis(50));
    let trace1 = getter.get_trace();

    assert!(
        trace1.is_ok(),
        "initial trace failed: {:?}",
        trace1.unwrap_err()
    );
    assert_eq!(trace1.unwrap().pid, Some(pid));

    // Trigger the exec
    writeln!(process.stdin.as_mut().unwrap()).expect("write to exec");

    let allowed_attempts = 20;
    for _ in 0..allowed_attempts {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let trace2 = getter.get_trace();

        if getter.reinit_count == 0 {
            continue;
        }

        assert!(
            trace2.is_ok(),
            "post-exec trace failed: {:?}",
            trace2.unwrap_err()
        );
    }

    process.kill().unwrap();

    assert_eq!(
        getter.reinit_count, 1,
        "Trace getter should have detected one reinit"
    );
}

#[test]
#[cfg(target_os = "linux")]
fn test_thread_churn() {
    // Test that even a program churning threads can be locked to be traced
    // Success / error criteria here should match those from `fn record` main.rs
    let mut process = std::process::Command::new("ruby")
        .arg("./ci/ruby-programs/thread_churn.rb")
        .spawn()
        .unwrap();
    let pid = process.id() as Pid;
    let mut getter = initialize(pid, true).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let total_samples = 1000;
    let mut errors = 0;
    for _ in 0..total_samples {
        std::thread::sleep(std::time::Duration::from_millis(1));
        match getter.get_trace() {
            Ok(_t) => {}
            Err(e) => match e.downcast_ref() {
                Some(MemoryCopyError::ProcessEnded) => {
                    panic!("Spurious process exit while testing thread churn");
                }
                _ => {
                    errors += 1;
                }
            },
        }
    }

    // These assertions are based on those from the record function in main.rs
    assert!(errors <= 20);
    assert!((errors as f64) / (total_samples as f64) <= 0.5);
    process.kill().unwrap();
}

#[test]
#[cfg(target_os = "macos")]
fn test_get_nonexistent_process() {
    assert!(Process::new(10000).is_err());
}

#[test]
#[cfg(target_os = "macos")]
fn test_get_disallowed_process() {
    // getting the ruby version isn't allowed on Mac if the process isn't running as root
    let mut process = std::process::Command::new("/usr/bin/ruby").spawn().unwrap();
    let pid = process.id() as Pid;
    assert!(Process::new(pid).is_err());
    process.kill().unwrap();
}

fn is_maybe_thread_function(version: &str) -> IsMaybeThreadFn {
    let function = match version {
        "1.9.1" => ruby_version::ruby_1_9_1_0::is_maybe_thread,
        "1.9.2" => ruby_version::ruby_1_9_2_0::is_maybe_thread,
        "1.9.3" => ruby_version::ruby_1_9_3_0::is_maybe_thread,
        "2.0.0" => ruby_version::ruby_2_0_0_0::is_maybe_thread,
        "2.1.0" => ruby_version::ruby_2_1_0::is_maybe_thread,
        "2.1.1" => ruby_version::ruby_2_1_1::is_maybe_thread,
        "2.1.2" => ruby_version::ruby_2_1_2::is_maybe_thread,
        "2.1.3" => ruby_version::ruby_2_1_3::is_maybe_thread,
        "2.1.4" => ruby_version::ruby_2_1_4::is_maybe_thread,
        "2.1.5" => ruby_version::ruby_2_1_5::is_maybe_thread,
        "2.1.6" => ruby_version::ruby_2_1_6::is_maybe_thread,
        "2.1.7" => ruby_version::ruby_2_1_7::is_maybe_thread,
        "2.1.8" => ruby_version::ruby_2_1_8::is_maybe_thread,
        "2.1.9" => ruby_version::ruby_2_1_9::is_maybe_thread,
        "2.1.10" => ruby_version::ruby_2_1_10::is_maybe_thread,
        "2.2.0" => ruby_version::ruby_2_2_0::is_maybe_thread,
        "2.2.1" => ruby_version::ruby_2_2_1::is_maybe_thread,
        "2.2.2" => ruby_version::ruby_2_2_2::is_maybe_thread,
        "2.2.3" => ruby_version::ruby_2_2_3::is_maybe_thread,
        "2.2.4" => ruby_version::ruby_2_2_4::is_maybe_thread,
        "2.2.5" => ruby_version::ruby_2_2_5::is_maybe_thread,
        "2.2.6" => ruby_version::ruby_2_2_6::is_maybe_thread,
        "2.2.7" => ruby_version::ruby_2_2_7::is_maybe_thread,
        "2.2.8" => ruby_version::ruby_2_2_8::is_maybe_thread,
        "2.2.9" => ruby_version::ruby_2_2_9::is_maybe_thread,
        "2.2.10" => ruby_version::ruby_2_2_10::is_maybe_thread,
        "2.3.0" => ruby_version::ruby_2_3_0::is_maybe_thread,
        "2.3.1" => ruby_version::ruby_2_3_1::is_maybe_thread,
        "2.3.2" => ruby_version::ruby_2_3_2::is_maybe_thread,
        "2.3.3" => ruby_version::ruby_2_3_3::is_maybe_thread,
        "2.3.4" => ruby_version::ruby_2_3_4::is_maybe_thread,
        "2.3.5" => ruby_version::ruby_2_3_5::is_maybe_thread,
        "2.3.6" => ruby_version::ruby_2_3_6::is_maybe_thread,
        "2.3.7" => ruby_version::ruby_2_3_7::is_maybe_thread,
        "2.3.8" => ruby_version::ruby_2_3_8::is_maybe_thread,
        "2.4.0" => ruby_version::ruby_2_4_0::is_maybe_thread,
        "2.4.1" => ruby_version::ruby_2_4_1::is_maybe_thread,
        "2.4.2" => ruby_version::ruby_2_4_2::is_maybe_thread,
        "2.4.3" => ruby_version::ruby_2_4_3::is_maybe_thread,
        "2.4.4" => ruby_version::ruby_2_4_4::is_maybe_thread,
        "2.4.5" => ruby_version::ruby_2_4_5::is_maybe_thread,
        "2.4.6" => ruby_version::ruby_2_4_6::is_maybe_thread,
        "2.4.7" => ruby_version::ruby_2_4_7::is_maybe_thread,
        "2.4.8" => ruby_version::ruby_2_4_8::is_maybe_thread,
        "2.4.9" => ruby_version::ruby_2_4_9::is_maybe_thread,
        "2.4.10" => ruby_version::ruby_2_4_10::is_maybe_thread,
        "2.5.0" => ruby_version::ruby_2_5_0::is_maybe_thread,
        "2.5.1" => ruby_version::ruby_2_5_1::is_maybe_thread,
        "2.5.2" => ruby_version::ruby_2_5_2::is_maybe_thread,
        "2.5.3" => ruby_version::ruby_2_5_3::is_maybe_thread,
        "2.5.4" => ruby_version::ruby_2_5_4::is_maybe_thread,
        "2.5.5" => ruby_version::ruby_2_5_5::is_maybe_thread,
        "2.5.6" => ruby_version::ruby_2_5_6::is_maybe_thread,
        "2.5.7" => ruby_version::ruby_2_5_7::is_maybe_thread,
        "2.5.8" => ruby_version::ruby_2_5_8::is_maybe_thread,
        "2.5.9" => ruby_version::ruby_2_5_9::is_maybe_thread,
        "2.6.0" => ruby_version::ruby_2_6_0::is_maybe_thread,
        "2.6.1" => ruby_version::ruby_2_6_1::is_maybe_thread,
        "2.6.2" => ruby_version::ruby_2_6_2::is_maybe_thread,
        "2.6.3" => ruby_version::ruby_2_6_3::is_maybe_thread,
        "2.6.4" => ruby_version::ruby_2_6_4::is_maybe_thread,
        "2.6.5" => ruby_version::ruby_2_6_5::is_maybe_thread,
        "2.6.6" => ruby_version::ruby_2_6_6::is_maybe_thread,
        "2.6.7" => ruby_version::ruby_2_6_7::is_maybe_thread,
        "2.6.8" => ruby_version::ruby_2_6_8::is_maybe_thread,
        "2.7.0" => ruby_version::ruby_2_7_0::is_maybe_thread,
        "2.7.1" => ruby_version::ruby_2_7_1::is_maybe_thread,
        "2.7.2" => ruby_version::ruby_2_7_2::is_maybe_thread,
        "2.7.3" => ruby_version::ruby_2_7_3::is_maybe_thread,
        "2.7.4" => ruby_version::ruby_2_7_4::is_maybe_thread,
        "3.0.0" => ruby_version::ruby_3_0_0::is_maybe_thread,
        "3.0.1" => ruby_version::ruby_3_0_1::is_maybe_thread,
        "3.0.2" => ruby_version::ruby_3_0_2::is_maybe_thread,
        _ => panic!(
            "Ruby version not supported yet: {}. Please create a GitHub issue and we'll fix it!",
            version
        ),
    };
    Box::new(function)
}

fn get_stack_trace_function(version: &str) -> StackTraceFn {
    let stack_trace_function = match version {
        "1.9.1" => ruby_version::ruby_1_9_1_0::get_stack_trace,
        "1.9.2" => ruby_version::ruby_1_9_2_0::get_stack_trace,
        "1.9.3" => ruby_version::ruby_1_9_3_0::get_stack_trace,
        "2.0.0" => ruby_version::ruby_2_0_0_0::get_stack_trace,
        "2.1.0" => ruby_version::ruby_2_1_0::get_stack_trace,
        "2.1.1" => ruby_version::ruby_2_1_1::get_stack_trace,
        "2.1.2" => ruby_version::ruby_2_1_2::get_stack_trace,
        "2.1.3" => ruby_version::ruby_2_1_3::get_stack_trace,
        "2.1.4" => ruby_version::ruby_2_1_4::get_stack_trace,
        "2.1.5" => ruby_version::ruby_2_1_5::get_stack_trace,
        "2.1.6" => ruby_version::ruby_2_1_6::get_stack_trace,
        "2.1.7" => ruby_version::ruby_2_1_7::get_stack_trace,
        "2.1.8" => ruby_version::ruby_2_1_8::get_stack_trace,
        "2.1.9" => ruby_version::ruby_2_1_9::get_stack_trace,
        "2.1.10" => ruby_version::ruby_2_1_10::get_stack_trace,
        "2.2.0" => ruby_version::ruby_2_2_0::get_stack_trace,
        "2.2.1" => ruby_version::ruby_2_2_1::get_stack_trace,
        "2.2.2" => ruby_version::ruby_2_2_2::get_stack_trace,
        "2.2.3" => ruby_version::ruby_2_2_3::get_stack_trace,
        "2.2.4" => ruby_version::ruby_2_2_4::get_stack_trace,
        "2.2.5" => ruby_version::ruby_2_2_5::get_stack_trace,
        "2.2.6" => ruby_version::ruby_2_2_6::get_stack_trace,
        "2.2.7" => ruby_version::ruby_2_2_7::get_stack_trace,
        "2.2.8" => ruby_version::ruby_2_2_8::get_stack_trace,
        "2.2.9" => ruby_version::ruby_2_2_9::get_stack_trace,
        "2.2.10" => ruby_version::ruby_2_2_10::get_stack_trace,
        "2.3.0" => ruby_version::ruby_2_3_0::get_stack_trace,
        "2.3.1" => ruby_version::ruby_2_3_1::get_stack_trace,
        "2.3.2" => ruby_version::ruby_2_3_2::get_stack_trace,
        "2.3.3" => ruby_version::ruby_2_3_3::get_stack_trace,
        "2.3.4" => ruby_version::ruby_2_3_4::get_stack_trace,
        "2.3.5" => ruby_version::ruby_2_3_5::get_stack_trace,
        "2.3.6" => ruby_version::ruby_2_3_6::get_stack_trace,
        "2.3.7" => ruby_version::ruby_2_3_7::get_stack_trace,
        "2.3.8" => ruby_version::ruby_2_3_8::get_stack_trace,
        "2.4.0" => ruby_version::ruby_2_4_0::get_stack_trace,
        "2.4.1" => ruby_version::ruby_2_4_1::get_stack_trace,
        "2.4.2" => ruby_version::ruby_2_4_2::get_stack_trace,
        "2.4.3" => ruby_version::ruby_2_4_3::get_stack_trace,
        "2.4.4" => ruby_version::ruby_2_4_4::get_stack_trace,
        "2.4.5" => ruby_version::ruby_2_4_5::get_stack_trace,
        "2.4.6" => ruby_version::ruby_2_4_6::get_stack_trace,
        "2.4.7" => ruby_version::ruby_2_4_7::get_stack_trace,
        "2.4.8" => ruby_version::ruby_2_4_8::get_stack_trace,
        "2.4.9" => ruby_version::ruby_2_4_9::get_stack_trace,
        "2.4.10" => ruby_version::ruby_2_4_10::get_stack_trace,
        "2.5.0" => ruby_version::ruby_2_5_0::get_stack_trace,
        "2.5.1" => ruby_version::ruby_2_5_1::get_stack_trace,
        "2.5.2" => ruby_version::ruby_2_5_2::get_stack_trace,
        "2.5.3" => ruby_version::ruby_2_5_3::get_stack_trace,
        "2.5.4" => ruby_version::ruby_2_5_4::get_stack_trace,
        "2.5.5" => ruby_version::ruby_2_5_5::get_stack_trace,
        "2.5.6" => ruby_version::ruby_2_5_6::get_stack_trace,
        "2.5.7" => ruby_version::ruby_2_5_7::get_stack_trace,
        "2.5.8" => ruby_version::ruby_2_5_8::get_stack_trace,
        "2.5.9" => ruby_version::ruby_2_5_9::get_stack_trace,
        "2.6.0" => ruby_version::ruby_2_6_0::get_stack_trace,
        "2.6.1" => ruby_version::ruby_2_6_1::get_stack_trace,
        "2.6.2" => ruby_version::ruby_2_6_2::get_stack_trace,
        "2.6.3" => ruby_version::ruby_2_6_3::get_stack_trace,
        "2.6.4" => ruby_version::ruby_2_6_4::get_stack_trace,
        "2.6.5" => ruby_version::ruby_2_6_5::get_stack_trace,
        "2.6.6" => ruby_version::ruby_2_6_6::get_stack_trace,
        "2.6.7" => ruby_version::ruby_2_6_7::get_stack_trace,
        "2.6.8" => ruby_version::ruby_2_6_8::get_stack_trace,
        "2.7.0" => ruby_version::ruby_2_7_0::get_stack_trace,
        "2.7.1" => ruby_version::ruby_2_7_1::get_stack_trace,
        "2.7.2" => ruby_version::ruby_2_7_2::get_stack_trace,
        "2.7.3" => ruby_version::ruby_2_7_3::get_stack_trace,
        "2.7.4" => ruby_version::ruby_2_7_4::get_stack_trace,
        "3.0.0" => ruby_version::ruby_3_0_0::get_stack_trace,
        "3.0.1" => ruby_version::ruby_3_0_1::get_stack_trace,
        "3.0.2" => ruby_version::ruby_3_0_2::get_stack_trace,
        _ => panic!(
            "Ruby version not supported yet: {}. Please create a GitHub issue and we'll fix it!",
            version
        ),
    };
    Box::new(stack_trace_function)
}
