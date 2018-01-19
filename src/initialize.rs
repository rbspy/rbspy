use copy::*;
use copy;
use failure::Error;
use failure::ResultExt;
use libc::{c_char, pid_t};
use std::fmt;
use std::time::Duration;
use std;
use ruby_version;
use address_finder::*;
use address_finder;
use proc_maps::*;
use read_process_memory::*;

/**
 * Initialization code for the profiler.
 *
 * The only public function here is `initialize(pid: pid_t)`, which returns a struct which you can
 * call `get_trace()` on to get a stack trace.
 *
 * Core responsibilities of this code:
 *   * Get the Ruby version
 *   * Get the address of the current thread
 *   * Find the right stack trace function for the Ruby version we found
 *   * Package all that up into a struct that the user can use to get stack traces.
 */
pub fn initialize(pid: pid_t) -> Result<StackTraceGetter, Error> {
    let version = get_ruby_version_retry(pid).context("Couldn't determine Ruby version")?;
    let is_maybe_thread = is_maybe_thread_function(&version);

    debug!("version: {}", version);
    Ok(StackTraceGetter {
        pid: pid,
        current_thread_addr_location: address_finder::current_thread_address(
            pid,
            &version,
            is_maybe_thread,
        )?,
        stack_trace_function: get_stack_trace_function(&version),
    })
}

#[derive(Debug, PartialEq)]
pub struct StackFrame {
    pub name: String,
    pub path: String,
    pub lineno: Option<u32>,
}

// Use a StackTraceGetter to get stack traces
pub struct StackTraceGetter {
    pid: pid_t,
    current_thread_addr_location: usize,
    stack_trace_function:
        Box<Fn(usize, &ProcessHandle) -> Result<Vec<StackFrame>, MemoryCopyError>>,
}

impl fmt::Display for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.lineno {
            Some(line) => write!(f, "{} - {} line {}", self.name, self.path, line),
            None => write!(f, "{} - {}", self.name, self.path),
        }
    }
}

impl StackTraceGetter {
    pub fn get_trace(&self) -> Result<Vec<StackFrame>, MemoryCopyError> {
        let stack_trace_function = &self.stack_trace_function;
        stack_trace_function(
            self.current_thread_addr_location,
            &self.pid.try_into_process_handle().unwrap(),
        )
    }
}

// Everything below here is private

fn get_ruby_version_retry(pid: pid_t) -> Result<String, Error> {
    // this exists because sometimes rbenv takes a while to exec the right Ruby binary.
    // we are dumb right now so we just... wait until it seems to work out.
    let mut i = 0;
    loop {
        let version = get_ruby_version(pid);
        let mut ret = false;
        match &version {
            &Err(ref err) => {
                match err.root_cause().downcast_ref::<AddressFinderError>() {
                    Some(&AddressFinderError::PermissionDenied(_)) => {
                        ret = true;
                    }
                    Some(&AddressFinderError::NoSuchProcess(_)) => {
                        ret = true;
                    }
                    _ => {}
                }
                match err.root_cause().downcast_ref::<MemoryCopyError>() {
                    Some(&MemoryCopyError::PermissionDenied) => {
                        ret = true;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        if i > 100 || version.is_ok() || ret {
            return Ok(version?);
        }
        // if it doesn't work, sleep for 1ms and try again
        i += 1;
        std::thread::sleep(Duration::from_millis(1));
    }
}

pub fn get_ruby_version(pid: pid_t) -> Result<String, Error> {
    let addr = address_finder::get_ruby_version_address(pid)?;
    debug!("ruby version addr: {:x}", addr);
    let x: [c_char; 15] = copy_struct(addr, &pid.try_into_process_handle().unwrap())?;
    debug!("ruby version struct: {:?}", x);
    Ok(unsafe {
        std::ffi::CStr::from_ptr(x.as_ptr() as *mut c_char)
            .to_str()?
            .to_owned()
    })
}

#[test]
fn test_get_nonexistent_process() {
    let version = get_ruby_version_retry(10000);
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
fn test_get_disallowed_process() {
    let version = get_ruby_version_retry(1);
    match version
        .unwrap_err()
        .root_cause()
        .downcast_ref::<AddressFinderError>()
        .unwrap()
    {
        &AddressFinderError::PermissionDenied(1) => {}
        _ => assert!(false, "Expected NoSuchProcess error"),
    }
}
#[test]
fn test_current_thread_address() {
    let mut process = std::process::Command::new("/usr/bin/ruby").spawn().unwrap();
    let pid = process.id() as pid_t;
    let version = get_ruby_version_retry(pid).expect("version should exist");
    let is_maybe_thread = is_maybe_thread_function(&version);
    let result = address_finder::current_thread_address(pid, &version, is_maybe_thread);
    assert!(result.is_ok(), format!("result not ok: {:?}", result));
    process.kill().unwrap();
}

fn is_maybe_thread_function<T: 'static>(
    version: &str,
) -> Box<Fn(usize, &T, &MapRange, &Vec<MapRange>) -> bool>
where
    T: CopyAddress,
{
    let function = match version.as_ref() {
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
        "2.3.0" => ruby_version::ruby_2_3_0::is_maybe_thread,
        "2.3.1" => ruby_version::ruby_2_3_1::is_maybe_thread,
        "2.3.2" => ruby_version::ruby_2_3_2::is_maybe_thread,
        "2.3.3" => ruby_version::ruby_2_3_3::is_maybe_thread,
        "2.3.4" => ruby_version::ruby_2_3_4::is_maybe_thread,
        "2.3.5" => ruby_version::ruby_2_3_5::is_maybe_thread,
        "2.3.6" => ruby_version::ruby_2_3_6::is_maybe_thread,
        "2.4.0" => ruby_version::ruby_2_4_0::is_maybe_thread,
        "2.4.1" => ruby_version::ruby_2_4_1::is_maybe_thread,
        "2.4.2" => ruby_version::ruby_2_4_2::is_maybe_thread,
        "2.4.3" => ruby_version::ruby_2_4_3::is_maybe_thread,
        "2.5.0" => ruby_version::ruby_2_5_0_rc1::is_maybe_thread,
        _ => panic!("oh no"),
    };
    Box::new(function)
}

fn get_stack_trace_function<T: 'static>(
    version: &str,
) -> Box<Fn(usize, &T) -> Result<Vec<StackFrame>, copy::MemoryCopyError>>
where
    T: CopyAddress,
{
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
        "2.3.0" => ruby_version::ruby_2_3_0::get_stack_trace,
        "2.3.1" => ruby_version::ruby_2_3_1::get_stack_trace,
        "2.3.2" => ruby_version::ruby_2_3_2::get_stack_trace,
        "2.3.3" => ruby_version::ruby_2_3_3::get_stack_trace,
        "2.3.4" => ruby_version::ruby_2_3_4::get_stack_trace,
        "2.3.5" => ruby_version::ruby_2_3_5::get_stack_trace,
        "2.3.6" => ruby_version::ruby_2_3_6::get_stack_trace,
        "2.4.0" => ruby_version::ruby_2_4_0::get_stack_trace,
        "2.4.1" => ruby_version::ruby_2_4_1::get_stack_trace,
        "2.4.2" => ruby_version::ruby_2_4_2::get_stack_trace,
        "2.4.3" => ruby_version::ruby_2_4_3::get_stack_trace,
        "2.5.0" => ruby_version::ruby_2_5_0_rc1::get_stack_trace,
        _ => panic!("oh no"),
    };
    Box::new(stack_trace_function)
}
