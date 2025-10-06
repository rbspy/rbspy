/*
 * Ruby version specific code for reading a stack trace out of a Ruby process's memory.
 *
 * Implemented through a series of macros, because there are subtle differences in struct layout
 * between similar Ruby versions (like 2.2.1 vs 2.2.2) that mean it's easiest to compile a
 * different function for every Ruby minor version.
 *
 * Defines a bunch of submodules, one per Ruby version (`ruby_1_9_3`, `ruby_2_2_0`, etc.)
 */

macro_rules! ruby_version_v_1_9_1(
    ($ruby_version:ident) => (
        pub mod $ruby_version {
            use std;
            use anyhow::{Context, format_err, Result};
            use bindings::$ruby_version::*;
            use crate::core::process::ProcessMemory;

            get_stack_trace!(rb_thread_struct);
            get_execution_context_from_thread!(rb_thread_struct);
            rstring_as_array_1_9_1!();
            get_ruby_string_1_9_1!();
            get_cfps!();
            get_pos!(rb_iseq_struct);
            get_lineno_1_9_0!();
            get_stack_frame_1_9_1!();
            stack_field_1_9_0!();
            get_thread_status_1_9_0!();
            get_thread_id_1_9_0!();
            get_cfunc_name_unsupported!();
        }
    )
);

macro_rules! ruby_version_v_1_9_2_to_3(
    // support for absolute paths appears for 1.9.2
    ($ruby_version:ident) => (
        pub mod $ruby_version {
            use std;
            use anyhow::{Context, format_err, Result};
            use bindings::$ruby_version::*;
            use crate::core::process::ProcessMemory;

            get_stack_trace!(rb_thread_struct);
            get_execution_context_from_thread!(rb_thread_struct);
            rstring_as_array_1_9_1!();
            get_ruby_string_1_9_1!();
            get_cfps!();
            get_pos!(rb_iseq_struct);
            get_lineno_1_9_0!();
            get_stack_frame_1_9_2!();
            stack_field_1_9_0!();
            get_thread_status_1_9_0!();
            get_thread_id_1_9_0!();
            get_cfunc_name_unsupported!();
        }
    )
);

macro_rules! ruby_version_v_2_0_to_2_2(
    ($ruby_version:ident) => (
       pub mod $ruby_version {
           use std;
           use anyhow::{Context, format_err, Result};
           use bindings::$ruby_version::*;
           use crate::core::process::ProcessMemory;

            // These 4 functions are the
            // core of how the program works. They're essentially a straight port of
            // this gdb script:
            // https://gist.github.com/csfrancis/11376304/raw/7a0450d11e64e3bb7c982b7ad2778f3603188c0f/gdb_ruby_backtrace.py
            // except without using gdb!!
            //
            // `get_cfps` corresponds to
            // (* const rb_thread_struct *(ruby_current_thread_address_location))->cfp
            //
            // `get_ruby_string` is doing ((Struct RString *) address) and then
            // trying one of two ways to get the actual Ruby string out depending
            // on how it's stored
            get_stack_trace!(rb_thread_struct);
            get_execution_context_from_thread!(rb_thread_struct);
            rstring_as_array_1_9_1!();
            get_ruby_string_1_9_1!();
            get_cfps!();
            get_pos!(rb_iseq_struct);
            get_lineno_2_0_0!();
            get_stack_frame_2_0_0!();
            stack_field_1_9_0!();
            get_thread_status_1_9_0!();
            get_thread_id_1_9_0!();
            get_cfunc_name_unsupported!();
        }
    )
);

macro_rules! ruby_version_v_2_3_to_2_4(
    ($ruby_version:ident) => (
       pub mod $ruby_version {
           use std;
           use anyhow::{Context, format_err, Result};
           use bindings::$ruby_version::*;
           use crate::core::process::ProcessMemory;

            get_stack_trace!(rb_thread_struct);
            get_execution_context_from_thread!(rb_thread_struct);
            rstring_as_array_1_9_1!();
            get_ruby_string_1_9_1!();
            get_cfps!();
            get_pos!(rb_iseq_constant_body);
            get_lineno_2_3_0!();
            get_stack_frame_2_3_0!();
            stack_field_1_9_0!();
            get_thread_status_1_9_0!();
            get_thread_id_1_9_0!();
            get_cfunc_name_unsupported!();
        }
    )
);

macro_rules! ruby_version_v2_5_x(
    ($ruby_version:ident) => (
       pub mod $ruby_version {
           use std;
           use anyhow::{Context, format_err, Result};
           use bindings::$ruby_version::*;
           use crate::core::process::ProcessMemory;

            get_stack_trace!(rb_execution_context_struct);
            get_execution_context_from_thread!(rb_execution_context_struct);
            rstring_as_array_1_9_1!();
            get_ruby_string_1_9_1!();
            get_cfps!();
            get_pos!(rb_iseq_constant_body);
            get_lineno_2_5_0!();
            get_stack_frame_2_5_0!();
            stack_field_2_5_0!();
            get_ruby_string_array_2_5_0!();
            get_thread_status_2_5_0!();
            get_thread_id_2_5_0!();
            #[cfg(any(target_os = "freebsd", target_os = "macos", target_os = "windows"))]
            get_cfunc_name_unsupported!();
            #[cfg(target_os = "linux")]
            get_cfunc_name!();
            #[cfg(target_os = "linux")]
            get_classpath_unsupported!();
        }
    )
);

macro_rules! ruby_version_v2_6_x(
    ($ruby_version:ident) => (
       pub mod $ruby_version {
           use std;
           use anyhow::{Context, format_err, Result};
           use bindings::$ruby_version::*;
           use crate::core::process::ProcessMemory;

            get_stack_trace!(rb_execution_context_struct);
            get_execution_context_from_thread!(rb_execution_context_struct);
            rstring_as_array_1_9_1!();
            get_ruby_string_1_9_1!();
            get_ruby_string_array_2_5_0!();
            get_cfps!();
            get_pos!(rb_iseq_constant_body);
            get_lineno_2_6_0!();
            get_stack_frame_2_5_0!();
            stack_field_2_5_0!();
            get_thread_status_2_6_0!();
            get_thread_id_2_5_0!();
            #[cfg(any(target_os = "freebsd", target_os = "macos", target_os = "windows"))]
            get_cfunc_name_unsupported!();
            #[cfg(target_os = "linux")]
            get_cfunc_name!();
            #[cfg(target_os = "linux")]
            get_classpath_unsupported!();
        }
    )
);

macro_rules! ruby_version_v2_7_x(
    ($ruby_version:ident) => (
        pub mod $ruby_version {
            use std;
            use anyhow::{Context, format_err, Result};
            use bindings::$ruby_version::*;
            use crate::core::process::ProcessMemory;

            get_stack_trace!(rb_execution_context_struct);
            get_execution_context_from_thread!(rb_execution_context_struct);
            rstring_as_array_1_9_1!();
            get_ruby_string_1_9_1!();
            get_ruby_string_array_2_5_0!();
            get_cfps!();
            get_pos!(rb_iseq_constant_body);
            get_lineno_2_6_0!();
            get_stack_frame_2_5_0!();
            stack_field_2_5_0!();
            get_thread_status_2_6_0!();
            get_thread_id_2_5_0!();
            get_cfunc_name!();
            get_classpath_unsupported!();
        }
    )
);

macro_rules! ruby_version_v3_0_x(
    ($ruby_version:ident) => (
        pub mod $ruby_version {
            use std;
            use anyhow::{Context, format_err, Result};
            use bindings::$ruby_version::*;
            use crate::core::process::ProcessMemory;

            get_stack_trace!(rb_execution_context_struct);
            get_execution_context_from_vm!();
            rstring_as_array_1_9_1!();
            get_ruby_string_1_9_1!();
            get_ruby_string_array_2_5_0!();
            get_cfps!();
            get_pos!(rb_iseq_constant_body);
            get_lineno_2_6_0!();
            get_stack_frame_2_5_0!();
            stack_field_2_5_0!();
            get_thread_status_2_6_0!();
            get_thread_id_2_5_0!();
            get_cfunc_name!();

            #[allow(non_upper_case_globals)]
            const ruby_fl_type_RUBY_FL_USHIFT: ruby_fl_type = ruby_fl_ushift_RUBY_FL_USHIFT as i32;
            get_classpath_unsupported!();
        }
    )
);

macro_rules! ruby_version_v3_1_x(
    ($ruby_version:ident) => (
        pub mod $ruby_version {
            use std;
            use anyhow::{Context, format_err, Result};
            use bindings::$ruby_version::*;
            use crate::core::process::ProcessMemory;

            get_stack_trace!(rb_execution_context_struct);
            get_execution_context_from_vm!();
            rstring_as_array_3_1_0!();
            get_ruby_string_1_9_1!();
            get_ruby_string_array_2_5_0!();
            get_cfps!();
            get_pos!(rb_iseq_constant_body);
            get_lineno_2_6_0!();
            get_stack_frame_2_5_0!();
            stack_field_2_5_0!();
            get_thread_status_2_6_0!();
            get_thread_id_2_5_0!();
            get_cfunc_name!();

            #[allow(non_upper_case_globals)]
            const ruby_fl_type_RUBY_FL_USHIFT: ruby_fl_type = ruby_fl_ushift_RUBY_FL_USHIFT as i32;
            get_classpath_unsupported!();
        }
    )
);

macro_rules! ruby_version_v3_2_x(
    ($ruby_version:ident) => (
        pub mod $ruby_version {
            use std;
            use anyhow::{Context, format_err, Result};
            use bindings::$ruby_version::*;
            use crate::core::process::ProcessMemory;

            get_stack_trace!(rb_execution_context_struct);
            get_execution_context_from_vm!();
            get_ruby_string_3_2_0!();
            get_ruby_string_array_3_2_0!();
            get_cfps!();
            get_pos!(rb_iseq_constant_body);
            get_lineno_2_6_0!();
            get_stack_frame_2_5_0!();
            stack_field_2_5_0!();
            get_thread_status_2_6_0!();
            get_thread_id_3_2_0!();
            get_cfunc_name!();

            #[allow(non_upper_case_globals)]
            const ruby_fl_type_RUBY_FL_USHIFT: ruby_fl_type = ruby_fl_ushift_RUBY_FL_USHIFT as i32;
            get_classpath_unsupported!();
        }
    )
);

macro_rules! ruby_version_v3_3_x(
    ($ruby_version:ident) => (
        pub mod $ruby_version {
            use std;
            use anyhow::{Context, format_err, Result};
            use bindings::$ruby_version::*;
            use crate::core::process::ProcessMemory;

            get_stack_trace!(rb_execution_context_struct);
            get_execution_context_from_vm!();
            get_ruby_string_3_3_0!();
            get_ruby_string_array_3_2_0!();
            get_cfps!();
            get_pos!(rb_iseq_constant_body);
            get_lineno_2_6_0!();
            get_stack_frame_3_3_0!();
            stack_field_2_5_0!();
            get_thread_status_2_6_0!();
            get_thread_id_3_2_0!();
            get_cfunc_name!();

            #[allow(non_upper_case_globals)]
            const ruby_fl_type_RUBY_FL_USHIFT: ruby_fl_type = ruby_fl_ushift_RUBY_FL_USHIFT as i32;
            get_classpath!();
        }
    )
);

macro_rules! get_execution_context_from_thread(
    ($thread_type:ident) => (
        pub fn get_execution_context<T: ProcessMemory>(
            current_thread_address_ptr: usize,
            _ruby_vm_address_ptr: usize,
            source: &T
        ) -> Result<usize> {
            source.copy_struct(current_thread_address_ptr)
                .context("couldn't read current thread pointer")
        }
    )
);

macro_rules! get_execution_context_from_vm(
    () => (
        pub fn get_execution_context<T: ProcessMemory>(
            _current_thread_address_ptr: usize,
            ruby_vm_address_ptr: usize,
            source: &T
        ) -> Result<usize> {
            // This is a roundabout way to get the execution context address, but it helps us
            // avoid platform-specific structures in memory (e.g. pthread types) that would
            // require us to maintain separate ruby-structs bindings for each platform due to
            // their varying sizes and alignments.
            let vm_addr: usize = source.copy_struct(ruby_vm_address_ptr)
                .context("couldn't read Ruby VM pointer")?;
            let vm: rb_vm_struct = source.copy_struct(vm_addr as usize)
                .context("couldn't read Ruby VM struct")?;

            // Seek forward in the ractor struct, looking for the main thread's address. There
            // may be other copies of the main thread address in the ractor struct, so it's
            // important to jump as close as possible to the main_thread struct field before we
            // search, which is the purpose of the initial offset. The execution context pointer
            // is in the memory word just before main_thread (see rb_ractor_struct).
            //
            // The initial offsets were found by experimenting.
            const INITIAL_OFFSET: usize =
                if cfg!(target_os = "windows") {
                    32
                } else {
                    48
                };
            const ADDRESSES_TO_CHECK: usize = 32;
            let offset = INITIAL_OFFSET * std::mem::size_of::<usize>();
            let main_ractor_address = vm.ractor.main_ractor as usize;
            let candidate_addresses: [usize; ADDRESSES_TO_CHECK] =
                source.copy_struct(main_ractor_address + offset)
                    .context("couldn't read main ractor struct")?;

            candidate_addresses
            .iter()
            .enumerate()
            .filter(|(idx, &addr)| *idx > 0 && addr == vm.ractor.main_thread as usize)
            .map(|(idx, _)| candidate_addresses[idx - 1])
            .filter(|&addr| addr != 0)
            .filter(|&addr| source.copy_struct::<rb_execution_context_struct>(addr as usize).is_ok())
            .collect::<Vec<usize>>()
            .first()
            .map(|&addr| addr as usize)
            .ok_or_else(|| format_err!("couldn't find execution context"))
        }
    )
);

macro_rules! get_stack_trace(
    ($thread_type:ident) => (
        use crate::core::process::Pid;
        use crate::core::types::{StackFrame, StackTrace};

        pub fn get_stack_trace<T: ProcessMemory>(
            ruby_current_thread_address_location: usize,
            ruby_vm_address_location: usize,
            ruby_global_symbols_address_location: Option<usize>,
            source: &T,
            pid: Pid,
            on_cpu: bool,
        ) -> Result<Option<StackTrace>, anyhow::Error> {
            let current_thread_addr: usize = get_execution_context(ruby_current_thread_address_location, ruby_vm_address_location, source)
                .context("couldn't get execution context")?;
            let thread: $thread_type = source.copy_struct(current_thread_addr)
                .context("couldn't get current thread")?;

            if on_cpu && get_thread_status(&thread, source)? != 0 /* THREAD_RUNNABLE */ {
                // This is in addition to any OS-specific checks for thread activity, and provides
                // an extra measure of reliability for targets that don't have them. It also works
                // for coredump targets.
                return Ok(None);
            }

            let thread_id = match get_thread_id(&thread, source) {
                Ok(tid) => Some(tid),
                Err(e) => {
                    debug!("Couldn't get thread ID: {}", e);
                    None
                },
            };
            if stack_field(&thread) as usize == 0 {
                return Ok(Some(StackTrace {
                    pid: Some(pid),
                    trace: vec!(StackFrame::unknown_c_function()),
                    thread_id: thread_id,
                    time: Some(SystemTime::now()),
                    on_cpu: None,
                }));
            }
            let mut trace = Vec::new();
            let cfps = get_cfps(thread.cfp as usize, stack_base(&thread) as usize, source)?;
            for cfp in cfps.iter() {
                if cfp.iseq as usize == 0 {
                    let mut frame = StackFrame::unknown_c_function();
                    if let Some(global_symbols_addr) = ruby_global_symbols_address_location {
                        match get_cfunc_name(cfp, global_symbols_addr, source, pid) {
                            Ok(name) => {
                                frame = StackFrame{
                                    name: format!("{} [c function]", name),
                                    relative_path: "(unknown)".to_string(),
                                    absolute_path: None,
                                    lineno: None,
                                };
                            },
                            Err(e) => {
                                debug!("Unknown C function: {:?}", e);
                            }
                        }
                    }
                    trace.push(frame);
                    continue;
                }
                if cfp.pc as usize == 0 {
                    debug!("pc was 0. Not sure what that means, but skipping CFP");
                    continue;
                }
                let iseq_struct: rb_iseq_struct = source.copy_struct(cfp.iseq as usize)
                    .context("couldn't copy iseq struct")?;

                let label_path  = get_stack_frame(&iseq_struct, &cfp, source);
                match label_path {
                    Ok(call)  => trace.push(call),
                    Err(x) => {
                        debug!("Error: {:#?}", x);
                        debug!("cfp: {:?}", cfp);
                        debug!("thread: {:?}", thread);
                        debug!("iseq struct: {:?}", iseq_struct);
                        // this is a heuristic: the intent of this is that it skips function calls into C extensions
                        if trace.len() > 0 {
                            debug!("Skipping function call, possibly into C extension");
                        } else {
                            return Err(x);
                        }
                    }
                }
            }
            let thread_id = match get_thread_id(&thread, source) {
                Ok(tid) => Some(tid),
                Err(e) => {
                    debug!("Couldn't get thread ID: {}", e);
                    None
                },
            };
            Ok(Some(StackTrace{trace, pid: Some(pid), thread_id, time: Some(SystemTime::now()), on_cpu: Some(on_cpu)}))
        }

        use proc_maps::{maps_contain_addr, MapRange};
        use std::time::SystemTime;

        // Checks whether the address looks even vaguely like a thread struct, mostly by making sure its
        // addresses are reasonable
        fn could_be_thread(thread: &$thread_type, all_maps: &[MapRange]) -> bool {
            maps_contain_addr(thread.tag as usize, all_maps) &&
                maps_contain_addr(thread.cfp as usize, all_maps) &&
                maps_contain_addr(stack_field(thread) as usize, all_maps) &&
                stack_size_field(thread) < 3_000_000
        }

        fn stack_base(thread: &$thread_type) -> i64 {
            // Ruby stack grows down, starting at
            //   ruby_current_thread->stack + ruby_current_thread->stack_size - 1 * sizeof(rb_control_frame_t)
            // I don't know what the -1 is about. Also note that the stack_size is *not* in bytes! stack is a
            // VALUE*, and so stack_size is in units of sizeof(VALUE).
            //
            // The base of the call stack is therefore at
            //   stack + stack_size * sizeof(VALUE) - sizeof(rb_control_frame_t)
            // (with everything in bytes).
            stack_field(thread) + stack_size_field(thread) * std::mem::size_of::<VALUE>() as i64 - 1 * std::mem::size_of::<rb_control_frame_t>() as i64
        }

        pub fn is_maybe_thread<T>(candidate_thread_addr: usize, candidate_thread_addr_ptr: usize, source: &T, all_maps: &[MapRange]) -> bool where T: ProcessMemory {
            if !maps_contain_addr(candidate_thread_addr, all_maps) {
                return false;
            }

            let thread: $thread_type = match source.copy_struct(candidate_thread_addr) {
                Ok(x) => x,
                _ => { return false; },
            };

            if !could_be_thread(&thread, &all_maps) {
                return false;
            }

            // finally, try to get an actual stack trace from the source and see if it works
            get_stack_trace(candidate_thread_addr_ptr, 0, None, source, 0, false).is_ok()
        }
    )
);

macro_rules! stack_field_1_9_0(
    () => (
        fn stack_field(thread: &rb_thread_struct) -> i64 {
            thread.stack as i64
        }

        fn stack_size_field(thread: &rb_thread_struct) -> i64 {
            thread.stack_size as i64
        }
    )
);

macro_rules! stack_field_2_5_0(
    () => (
        fn stack_field(thread: &rb_execution_context_struct) -> i64 {
            thread.vm_stack as i64
        }

        fn stack_size_field(thread: &rb_execution_context_struct) -> i64 {
            thread.vm_stack_size as i64
        }
    )
);

macro_rules! get_thread_status_1_9_0(
    () => (
        fn get_thread_status<T>(thread_struct: &rb_thread_struct, _source: &T) -> Result<u32> {
            Ok(thread_struct.status as u32)
        }
    )
);

macro_rules! get_thread_status_2_5_0(
    () => (
        fn get_thread_status<T>(thread_struct: &rb_execution_context_struct, source: &T)
                            -> Result<u32> where T: ProcessMemory {
            let thread: rb_thread_struct = source.copy_struct(thread_struct.thread_ptr as usize)
                .context(thread_struct.thread_ptr as usize)?;
            Ok(thread.status as u32)
        }
    )
);

// ->status changed into a bitfield
macro_rules! get_thread_status_2_6_0(
    () => (
        fn get_thread_status<T>(thread_struct: &rb_execution_context_struct, source: &T)
                            -> Result<u32> where T: ProcessMemory {
            let thread: rb_thread_struct = source.copy_struct(thread_struct.thread_ptr as usize)
                .context(thread_struct.thread_ptr as usize)?;
            Ok(thread.status() as u32)
        }
    )
);

macro_rules! get_thread_id_1_9_0(
    () => (
        fn get_thread_id<T>(thread_struct: &rb_thread_struct, _source: &T) -> Result<usize> {
            Ok(thread_struct.thread_id as usize)
        }
    )
);

macro_rules! get_thread_id_2_5_0(
    () => (
        fn get_thread_id<T>(thread_struct: &rb_execution_context_struct, source: &T)
                            -> Result<usize> where T: ProcessMemory {
            let thread: rb_thread_struct = source.copy_struct(thread_struct.thread_ptr as usize)
                .context("couldn't copy thread struct")?;
            Ok(thread.thread_id as usize)
        }
    )
);

macro_rules! get_thread_id_3_2_0(
    () => (
        fn get_thread_id<T>(thread_struct: &rb_execution_context_struct, source: &T)
                            -> Result<usize> where T: ProcessMemory {
            let thread: rb_thread_struct = source.copy_struct(thread_struct.thread_ptr as usize)
                .context("couldn't copy thread struct")?;
            if thread.nt.is_null() {
                return Err(format_err!("native thread pointer is NULL"));
            }
            let native_thread: rb_native_thread = source.copy_struct(thread.nt as usize)
                .context("couldn't copy native thread struct")?;
            Ok(native_thread.thread_id as usize)
        }
    )
);

macro_rules! get_ruby_string_array_2_5_0(
    () => (
        // Returns (path, absolute_path)
        fn get_ruby_string_array<T>(addr: usize, string_class: usize, source: &T) -> Result<(String, String)> where T: ProcessMemory {
            // todo: we're doing an extra copy here for no reason
            let rstring: RString = source.copy_struct(addr).context("couldn't copy RString")?;
            if rstring.basic.klass as usize == string_class {
                let s = get_ruby_string(addr, source)?;
                return Ok((s.clone(), s))
            }
            // otherwise it's an RArray
            let rarray: RArray = source.copy_struct(addr).context("couldn't copy RArray")?;
            // TODO: this assumes that the array contents are stored inline and not on the heap
            // I think this will always be true but we should check instead
            // the reason I am not checking is that I don't know how to check yet
            let path_addr: usize = unsafe { rarray.as_.ary[0] as usize }; // 0 => relative path
            let abs_path_addr: usize = unsafe { rarray.as_.ary[1] as usize }; // 1 => absolute path
            let rel_path = get_ruby_string(path_addr, source)?;
            // In the case of internal ruby functions (and maybe others), we may not get a valid
            // pointer here
            let abs_path = get_ruby_string(abs_path_addr, source)
                .unwrap_or(String::from("unknown"));
            Ok((rel_path, abs_path))
        }
    )
);

macro_rules! get_ruby_string_array_3_2_0(
    () => (
        // Returns (path, absolute_path)
        fn get_ruby_string_array<T>(addr: usize, string_class: usize, source: &T) -> Result<(String, String)> where T: ProcessMemory {
            let rstring: RString = source.copy_struct(addr).context("couldn't copy RString")?;
            if rstring.basic.klass as usize == string_class {
                let s = get_ruby_string(addr, source)?;
                return Ok((s.clone(), s))
            }

            // Due to VWA in ruby 3.2, we can't get the exact length of the RArray. So,
            // we use these inline structs and assume that there are at least two array
            // elements when we're reading a pathobj.
            #[repr(C)]
            #[derive(Copy, Clone)]
            struct PaddedRArray {
                pub basic: RBasic,
                pub as_: PaddedRArray__bindgen_ty_1,
            }
            #[repr(C)]
            #[derive(Copy, Clone)]
            union PaddedRArray__bindgen_ty_1 {
                pub heap: RArray__bindgen_ty_1__bindgen_ty_1,
                pub ary: [VALUE; 2usize],
            }

            // otherwise it's an RArray
            let rarray: PaddedRArray = source.copy_struct(addr).context("couldn't copy RArray")?;
            // TODO: this assumes that the array contents are stored inline and not on the heap
            // I think this will always be true but we should check instead
            // the reason I am not checking is that I don't know how to check yet
            let path_addr: usize = unsafe { rarray.as_.ary[0] as usize }; // 0 => relative path
            let abs_path_addr: usize = unsafe { rarray.as_.ary[1] as usize }; // 1 => absolute path

            let rel_path = get_ruby_string(path_addr, source)?;
            // In the case of internal ruby functions (and maybe others), we may not get a valid
            // pointer here
            let abs_path = get_ruby_string(abs_path_addr, source)
                .unwrap_or(String::from("unknown"));

            Ok((rel_path, abs_path))
        }
    )
);

macro_rules! rstring_as_array_1_9_1(
    () => (
        unsafe fn rstring_as_array(rstring: RString) -> [::std::os::raw::c_char; 24usize] {
            rstring.as_.ary
        }
    )
);

macro_rules! rstring_as_array_3_1_0(
    () => (
        unsafe fn rstring_as_array(rstring: RString) -> [::std::os::raw::c_char; 24usize] {
            rstring.as_.embed.ary
        }
    )
);

macro_rules! get_ruby_string_1_9_1(
    () => (
        fn get_ruby_string<T>(
            addr: usize,
            source: &T
        ) -> Result<String> where T: ProcessMemory {
            let vec = {
                let rstring: RString = source.copy_struct(addr).context("couldn't copy rstring")?;
                // See RSTRING_NOEMBED and RUBY_FL_USER1
                let is_embedded_string = rstring.basic.flags & 1 << 13 == 0;
                if is_embedded_string {
                    unsafe { std::ffi::CStr::from_ptr(rstring_as_array(rstring).as_ref().as_ptr()) }
                    .to_bytes()
                    .to_vec()
                } else {
                    unsafe {
                        let addr = rstring.as_.heap.ptr as usize;
                        let len = rstring.as_.heap.len as usize;
                        source.copy(addr as usize, len).context("couldn't copy ruby string from heap")?
                    }
                }
            };

            String::from_utf8(vec).context("couldn't convert ruby string bytes to string")
        }
    )
);

macro_rules! get_ruby_string_3_2_0(
    () => (
        fn get_ruby_string<T>(
            addr: usize,
            source: &T
        ) -> Result<String> where T: ProcessMemory {
            let rstring: RString = source.copy_struct(addr).context("couldn't copy rstring")?;
            // See RSTRING_NOEMBED and RUBY_FL_USER1
            let is_embedded_string = rstring.basic.flags & 1 << 13 == 0;
            if is_embedded_string {
                // Workaround for Windows strings until we have OS-specific bindings
                #[cfg(target_os = "windows")]
                let addr = addr + 4;

                // The introduction of Variable Width Allocation (VWA) for strings means that
                // the length of embedded strings varies at runtime. Instead of assuming a
                // constant length, we need to read the length from the struct.
                //
                // See https://bugs.ruby-lang.org/issues/18239
                let embedded_str_bytes = source.copy(
                    addr + std::mem::size_of::<RBasic>() + std::mem::size_of::<std::os::raw::c_long>(),
                    unsafe { rstring.as_.embed.len } as usize
                ).context("couldn't copy rstring")?;
                return String::from_utf8(embedded_str_bytes).context("couldn't convert ruby string bytes to string")
            } else {
                unsafe {
                    let addr = rstring.as_.heap.ptr as usize;
                    let len = rstring.as_.heap.len as usize;
                    let heap_str_bytes = source.copy(addr as usize, len).context("couldn't copy ruby string from heap")?;
                    return String::from_utf8(heap_str_bytes).context("couldn't convert ruby string bytes to string");
                }
            }
        }
    )
);

macro_rules! get_ruby_string_3_3_0(
    () => (
        fn get_ruby_string<T>(
            addr: usize,
            source: &T
        ) -> Result<String> where T: ProcessMemory {
            let rstring: RString = source.copy_struct(addr).context("couldn't copy rstring")?;
            if rstring.len > 1_000_000 {
                return Err(anyhow::anyhow!("string length {} for string at {:X} appears invalid", rstring.len, addr));
            }
            // See RSTRING_NOEMBED and RUBY_FL_USER1
            let is_embedded_string = rstring.basic.flags & 1 << 13 == 0;
            if is_embedded_string {
                // Workaround for Windows strings until we have OS-specific bindings
                #[cfg(target_os = "windows")]
                let addr = addr + 4;

                // The introduction of Variable Width Allocation (VWA) for strings means that
                // the length of embedded strings varies at runtime. Instead of assuming a
                // constant length, we need to read the length from the struct.
                //
                // See https://bugs.ruby-lang.org/issues/18239
                let embedded_str_bytes = source.copy(
                    addr + std::mem::size_of::<RBasic>() + std::mem::size_of::<std::os::raw::c_long>(),
                    rstring.len as usize
                ).context("couldn't copy rstring")?;
                return String::from_utf8(embedded_str_bytes).context("couldn't convert ruby string bytes to string")
            } else {
                unsafe {
                    let addr = rstring.as_.heap.ptr as usize;
                    let len = rstring.len as usize;
                    let heap_str_bytes = source.copy(addr as usize, len).context("couldn't copy ruby string from heap")?;
                    return String::from_utf8(heap_str_bytes).context("couldn't convert ruby string bytes to string");
                }
            }
        }
    )
);

macro_rules! get_stack_frame_1_9_1(
    () => (
        fn get_stack_frame<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
            ) -> Result<StackFrame> where T: ProcessMemory {
            Ok(StackFrame{
                name: get_ruby_string(iseq_struct.name as usize, source)?,
                relative_path: get_ruby_string(iseq_struct.filename as usize, source)?,
                absolute_path: None,
                lineno: match get_lineno(iseq_struct, cfp, source) {
                    Ok(lineno) => Some(lineno),
                    Err(e) => {
                        warn!("couldn't get lineno: {}", e);
                        None
                    },
                }
            })
        }
    )
);

macro_rules! get_stack_frame_1_9_2(
    () => (
        fn get_stack_frame<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
            ) -> Result<StackFrame> where T: ProcessMemory {
            Ok(StackFrame{
                name: get_ruby_string(iseq_struct.name as usize, source)?,
                relative_path: get_ruby_string(iseq_struct.filename as usize, source)?,
                absolute_path: Some(get_ruby_string(iseq_struct.filepath as usize, source)?),
                lineno: match get_lineno(iseq_struct, cfp, source) {
                    Ok(lineno) => Some(lineno),
                    Err(e) => {
                        warn!("couldn't get lineno: {}", e);
                        None
                    },
                }
            })
        }
    )
);

macro_rules! get_stack_frame_2_0_0(
    () => (
        fn get_stack_frame<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
        ) -> Result<StackFrame> where T: ProcessMemory {
            Ok(StackFrame{
                name: get_ruby_string(iseq_struct.location.label as usize, source)?,
                relative_path: get_ruby_string(iseq_struct.location.path as usize, source)?,
                absolute_path: Some(get_ruby_string(iseq_struct.location.absolute_path as usize, source)?),
                lineno: match get_lineno(iseq_struct, cfp, source) {
                    Ok(lineno) => Some(lineno),
                    Err(e) => {
                        warn!("couldn't get lineno: {}", e);
                        None
                    },
                }
            })
        }
    )
);

macro_rules! get_stack_frame_2_3_0(
    () => (
        fn get_stack_frame<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
        ) -> Result<StackFrame> where T: ProcessMemory {
            let body: rb_iseq_constant_body = source.copy_struct(iseq_struct.body as usize)
                .context(iseq_struct.body as usize)?;
            Ok(StackFrame{
                name: get_ruby_string(body.location.label as usize, source)?,
                relative_path: get_ruby_string(body.location.path as usize, source)?,
                absolute_path: Some(get_ruby_string(body.location.absolute_path as usize, source)?),
                lineno: match get_lineno(&body, cfp, source) {
                    Ok(lineno) => Some(lineno),
                    Err(e) => {
                        warn!("couldn't get lineno: {}", e);
                        None
                    },
                }
            })
        }
    )
);

macro_rules! get_stack_frame_2_5_0(
    () => (
        fn get_stack_frame<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
        ) -> Result<StackFrame> where T: ProcessMemory {
            if iseq_struct.body == std::ptr::null_mut() {
                return Err(format_err!("iseq body is null"));
            }
            let body: rb_iseq_constant_body = source.copy_struct(iseq_struct.body as usize)
                .context("couldn't copy rb_iseq_constant_body")?;
            let rstring: RString = source.copy_struct(body.location.label as usize)
                .context("couldn't copy RString")?;
            let (path, absolute_path) = get_ruby_string_array(
                body.location.pathobj as usize,
                rstring.basic.klass as usize,
                source
            ).context("couldn't get ruby string from iseq body")?;
            Ok(StackFrame{
                name: get_ruby_string(body.location.label as usize, source)?,
                relative_path: path,
                absolute_path: Some(absolute_path),
                lineno: match get_lineno(&body, cfp, source) {
                    Ok(lineno) => Some(lineno),
                    Err(e) => {
                        warn!("couldn't get lineno: {}", e);
                        None
                    },
                }
            })
        }
    )
);

macro_rules! get_classpath_unsupported(
    () => (
        fn get_classpath<T>(
            _cme: usize,
            _cfunc: bool,
            _source: &T,
        ) -> Result<(String, bool)> where T: ProcessMemory {
            return Err(format_err!("classpath resolution is not supported for this version of Ruby").into());
        }
    )
);

macro_rules! get_classpath(
    () => (
        fn get_classpath<T>(
            cme: usize,
            cfunc: bool,
            source: &T,
        ) -> Result<(String, bool)> where T: ProcessMemory {
            //https://github.com/ruby/ruby/blob/c149708018135595b2c19c5f74baf9475674f394/include/ruby/internal/value_type.h#L114¬
            const RUBY_T_CLASS: usize = 0x2;
            // https://github.com/ruby/ruby/blob/c149708018135595b2c19c5f74baf9475674f394/include/ruby/internal/value_type.h#L115C5-L115C74¬
            const RUBY_T_MODULE: usize = 0x3;
            //https://github.com/ruby/ruby/blob/c149708018135595b2c19c5f74baf9475674f394/include/ruby/internal/value_type.h#L138¬
            const RUBY_T_ICLASS: usize = 0x1c;
            // https://github.com/ruby/ruby/blob/c149708018135595b2c19c5f74baf9475674f394/include/ruby/internal/value_type.h#L142¬
            const RUBY_T_MASK: usize = 0x1f;

            //TODO replace these with the flushift ocnstants ones already referenced
            // https://github.com/ruby/ruby/blob/1d1529629ce1550fad19c2d9410c4bf4995230d2/include/ruby/internal/fl_type.h#L158¬
            const RUBY_FL_USHIFT: usize = 12;
            // https://github.com/ruby/ruby/blob/1d1529629ce1550fad19c2d9410c4bf4995230d2/include/ruby/internal/fl_type.h#L323-L324¬
            const RUBY_FL_USER1: usize = 1 << (RUBY_FL_USHIFT + 1);
            // https://github.com/ruby/ruby/blob/1d1529629ce1550fad19c2d9410c4bf4995230d2/include/ruby/internal/fl_type.h#L394¬
            const RUBY_FL_SINGLETON: usize = RUBY_FL_USER1;

            //let mut ep = ep.clone() as *mut usize;
            let mut singleton = false;
            let mut classpath_ptr = 0usize;

            let imemo: rb_method_entry_struct = source.copy_struct(cme).context(cme)?;
            // Read the class structure to get flags
            let class_addr = if cfunc {
                imemo.owner
            } else {
                imemo.defined_class
            };
            let klass: RClass_and_rb_classext_t = source.copy_struct(class_addr).context(class_addr)?;

            let rbasic: RBasic = source.copy_struct(class_addr).context(class_addr)?;
            // Get flags from the RClass structure (assuming flags is the first field)
            // You may need to adjust this based on your actual struct definition
            let class_flags = rbasic.flags;
            let class_mask = class_flags & RUBY_T_MASK;

            // TODO have test cases that cover class and module, singleton, etc.
            // see also how bmethod is handled
            match class_mask {
                RUBY_T_CLASS | RUBY_T_MODULE => {
                    classpath_ptr = klass.classext.classpath as usize;

                    // Check if it's a singleton class
                    if class_flags & RUBY_FL_SINGLETON != 0 {
                        log::debug!("Got singleton class");
                        singleton = true;

                        // For singleton classes, get the classpath from the attached object
                        let singleton_object_addr: usize = unsafe {
                            klass.classext.as_.singleton_class.attached_object as usize
                        };

                        if singleton_object_addr != 0 {
                            let singleton_obj: RClass_and_rb_classext_t = source
                                .copy_struct(singleton_object_addr)
                                .context(singleton_object_addr)?;
                            classpath_ptr = singleton_obj.classext.classpath as usize;
                        }
                    }
                }
                RUBY_T_ICLASS => {
                    // For iclass, get the classpath from the klass field
                    // Assuming RClass has a 'basic' field with 'klass' member
                    let klass_addr = rbasic.klass as usize;  // Adjust field name as needed

                    if klass_addr != 0 {
                        log::debug!("Using klass for iclass type");
                        let actual_klass: RClass_and_rb_classext_t = source
                            .copy_struct(klass_addr)
                            .context(klass_addr)?;
                        classpath_ptr = actual_klass.classext.classpath as usize;
                    }
                }
                _ => {
                    // Handle other types or anonymous classes
                    classpath_ptr = klass.classext.classpath as usize;
                }
            }

            if classpath_ptr == 0 {
                return Err(anyhow::anyhow!("classpath was empty"));
            }
            let class_path = get_ruby_string(classpath_ptr as usize, source)?;
            Ok((class_path.to_string(), singleton))
        }

        // TODO make some tests for profile_full_label_name to cover the various cases it needs
        // to handle correctly
        // TODO this should be saved in the cache, we shouldn't unconditionally run this
        fn profile_frame_full_label(
            class_path: &str,
            label: &str,
            base_label: &str,
            method_name: &str,
            singleton: bool,
        ) -> String {
            let qualified = qualified_method_name(class_path, method_name, singleton);

            if qualified.is_empty() || qualified == base_label {
                return label.to_string();
            }

            let label_length = label.len();
            let base_label_length = base_label.len();
            let mut prefix_len = label_length.saturating_sub(base_label_length);

            // Ensure prefix_len doesn't exceed label length (defensive programming)
            // Note: saturating_sub above already handles the < 0 case
            if prefix_len > label_length {
                prefix_len = label_length;
            }

            let profile_label = format!("{}{}", &label[..prefix_len], qualified);

            if profile_label.is_empty() {
                return String::new();
            }

            // Get the prefix from label and concatenate with qualified_method_name
            profile_label
        }
    )
);

macro_rules! get_stack_frame_3_3_0(
    () => (

        fn get_stack_frame<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
        ) -> Result<StackFrame> where T: ProcessMemory {
            if iseq_struct.body == std::ptr::null_mut() {
                return Err(format_err!("iseq body is null"));
            }
            let body: rb_iseq_constant_body = source.copy_struct(iseq_struct.body as usize)
                .context("couldn't copy rb_iseq_constant_body")?;
            let rstring: RString = source.copy_struct(body.location.label as usize)
                .context("couldn't copy RString")?;

            let mut method_name = "".to_string();
            if body.local_iseq != std::ptr::null_mut() {
                let local_iseq: rb_iseq_t = source.copy_struct(body.local_iseq as usize)
                    .context("couldn't read local iseq")?;
                if local_iseq.body != std::ptr::null_mut() {
                    let local_body: rb_iseq_constant_body = source.copy_struct(iseq_struct.body as usize)
                        .context("couldn't copy rb_iseq_constant_body")?;
                    method_name = get_ruby_string(local_body.location.base_label as usize, source)?;
                }

            }

            let (path, absolute_path) = get_ruby_string_array(
                body.location.pathobj as usize,
                rstring.basic.klass as usize,
                source
            ).context("couldn't get ruby string from iseq body")?;

            let cme = locate_method_entry(&cfp.ep, source)?;
            let (class_path, singleton) = get_classpath(cme, false, source).unwrap_or(("".to_string(), false));
            let label = get_ruby_string(body.location.label as usize, source)?;
            let base_label = get_ruby_string(body.location.base_label as usize, source)?;

            let full_label = profile_frame_full_label(&class_path, &label, &base_label, &method_name, singleton);

            Ok(StackFrame{
                name: full_label,
                relative_path: path,
                absolute_path: Some(absolute_path),
                lineno: match get_lineno(&body, cfp, source) {
                    Ok(lineno) => Some(lineno),
                    Err(e) => {
                        warn!("couldn't get lineno: {}", e);
                        None
                    },
                }
            })
        }
    )
);

macro_rules! get_pos(
    ($iseq_type:ident) => (
        #[allow(unused)] // this doesn't get used in every ruby version
        fn get_pos(iseq_struct: &$iseq_type, cfp: &rb_control_frame_t) -> Result<usize> {
            if (cfp.pc as usize) < (iseq_struct.iseq_encoded as usize) {
                return Err(crate::core::types::MemoryCopyError::Message(format!("program counter and iseq are out of sync")).into());
            }
            let mut pos = cfp.pc as usize - iseq_struct.iseq_encoded as usize;
            if pos != 0 {
                pos -= 1;
            }
            Ok(pos)
        }
    )
);

macro_rules! get_lineno_1_9_0(
    () => (
        fn get_lineno<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
        ) -> Result<usize> where T: ProcessMemory {
            let pos = get_pos(iseq_struct, cfp)?;
            let t_size = iseq_struct.insn_info_size as usize;
            if t_size == 0 {
                Err(format_err!("line number is not available"))
            } else if t_size == 1 {
                let table: [iseq_insn_info_entry; 1] = source.copy_struct(iseq_struct.insn_info_table as usize)
                    .context("couldn't copy instruction table")?;
                Ok(table[0].line_no as usize)
            } else {
                let table: Vec<iseq_insn_info_entry> = source.copy_vec(iseq_struct.insn_info_table as usize, t_size as usize)
                    .context("couldn't copy instruction table")?;
                for i in 0..t_size {
                    if pos == table[i].position as usize {
                        return Ok(table[i].line_no as usize)
                    } else if table[i].position as usize > pos {
                        return Ok(table[i-1].line_no as usize)
                    }
                }
                Ok(table[t_size-1].line_no as usize)
            }
        }
    )
);

macro_rules! get_lineno_2_0_0(
    () => (
        fn get_lineno<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
        ) -> Result<usize> where T: ProcessMemory {
            let pos = get_pos(iseq_struct, cfp)?;
            let t_size = iseq_struct.line_info_size as usize;
            if t_size == 0 {
                Err(format_err!("line number is not available"))
            } else if t_size == 1 {
                let table: [iseq_line_info_entry; 1] = source.copy_struct(iseq_struct.line_info_table as usize)
                    .context("couldn't copy instruction table")?;
                Ok(table[0].line_no as usize)
            } else {
                let table: Vec<iseq_line_info_entry> = source.copy_vec(iseq_struct.line_info_table as usize, t_size as usize)
                    .context("couldn't copy instruction table")?;
                for i in 0..t_size {
                    if pos == table[i].position as usize {
                        return Ok(table[i].line_no as usize)
                    } else if table[i].position as usize > pos {
                        return Ok(table[i-1].line_no as usize)
                    }
                }
                Ok(table[t_size-1].line_no as usize)
            }
        }
    )
);

macro_rules! get_lineno_2_3_0(
    () => (
        fn get_lineno<T>(
            iseq_struct: &rb_iseq_constant_body,
            cfp: &rb_control_frame_t,
            source: &T,
        ) -> Result<usize> where T: ProcessMemory {
            let pos = get_pos(iseq_struct, cfp)?;
            let t_size = iseq_struct.line_info_size as usize;
            if t_size == 0 {
                Err(format_err!("line number is not available"))
            } else if t_size == 1 {
                let table: [iseq_line_info_entry; 1] = source.copy_struct(iseq_struct.line_info_table as usize)
                    .context("couldn't copy instruction table")?;
                Ok(table[0].line_no as usize)
            } else {
                let table: Vec<iseq_line_info_entry> = source.copy_vec(iseq_struct.line_info_table as usize, t_size as usize)
                    .context("couldn't copy instruction table")?;
                for i in 0..t_size {
                    if pos == table[i].position as usize {
                        return Ok(table[i].line_no as usize)
                    } else if table[i].position as usize > pos {
                        return Ok(table[i-1].line_no as usize)
                    }
                }
                Ok(table[t_size-1].line_no as usize)
            }
        }
    )
);

macro_rules! get_lineno_2_5_0(
    () => (
        fn get_lineno<T>(
            iseq_struct: &rb_iseq_constant_body,
            cfp: &rb_control_frame_t,
            source: &T,
        ) -> Result<usize> where T: ProcessMemory {
            let pos = get_pos(iseq_struct, cfp)?;
            let t_size = iseq_struct.insns_info_size as usize;
            if t_size == 0 {
                Err(format_err!("line number is not available"))
            } else if t_size == 1 {
                let table: [iseq_insn_info_entry; 1] = source.copy_struct(iseq_struct.insns_info as usize)
                    .context("couldn't copy instruction table")?;
                Ok(table[0].line_no as usize)
            } else {
                let table: Vec<iseq_insn_info_entry> = source.copy_vec(iseq_struct.insns_info as usize, t_size as usize)
                    .context("couldn't copy instruction table")?;
                for i in 0..t_size {
                    if pos == table[i].position as usize {
                        return Ok(table[i].line_no as usize)
                    } else if table[i].position as usize > pos {
                        return Ok(table[i-1].line_no as usize)
                    }
                }
                Ok(table[t_size-1].line_no as usize)
            }
        }
    )
);

macro_rules! get_lineno_2_6_0(
    () => (
        fn get_lineno<T>(
            iseq_struct: &rb_iseq_constant_body,
            _cfp: &rb_control_frame_t,
            source: &T,
        ) -> Result<usize> where T: ProcessMemory {
            let t_size = iseq_struct.insns_info.size as usize;
            if t_size == 0 {
                Err(format_err!("line number is not available"))
            } else if t_size == 1 {
                let table: [iseq_insn_info_entry; 1] = source.copy_struct(iseq_struct.insns_info.body as usize)
                    .context("couldn't copy instruction table")?;
                Ok(table[0].line_no as usize)
            } else {
                // TODO: To handle this properly, we need to imitate ruby's succinct bit vector lookup.
                // See https://github.com/rbspy/rbspy/issues/213#issuecomment-826363857
                let table: Vec<iseq_insn_info_entry> = source.copy_vec(iseq_struct.insns_info.body as usize, t_size as usize)
                    .context(iseq_struct.insns_info.body as usize)?;
                Ok(table[t_size-1].line_no as usize)
            }
        }
    )
);

macro_rules! get_cfps(
    () => (
        fn get_cfps<T>(
            cfp_address: usize,
            stack_base: usize,
            source: &T
        ) -> Result<Vec<rb_control_frame_t>> where T: ProcessMemory {
            // If we fail these safety checks, it probably means we've hit some kind of
            // race condition. Return an error so that we can try again.
            if (stack_base as usize) <= cfp_address {
                return Err(crate::core::types::MemoryCopyError::Message(format!("stack base and cfp address out of sync. stack base: {:x}, cfp address: {:x}", stack_base as usize, cfp_address)).into());
            }
            let cfp_size = (stack_base as usize - cfp_address) as usize / std::mem::size_of::<rb_control_frame_t>();
            if cfp_size > 1_000_000 {
                return Err(crate::core::types::MemoryCopyError::Message(format!("invalid cfp vector length: {}", cfp_size)).into());
            }

            source.copy_vec(cfp_address, cfp_size).context("couldn't copy cfp vector")
        }
    )
);

macro_rules! get_cfunc_name_unsupported(
    () => (
        fn get_cfunc_name<T: ProcessMemory>(_cfp: &rb_control_frame_t, _global_symbols_address: usize, _source: &T, _pid: Pid) -> Result<String> {
            return Err(format_err!("C function resolution is not supported for this version of Ruby").into());
        }
    )
);

macro_rules! get_cfunc_name(
    () => (
        fn qualified_method_name(class_path: &str, method_name: &str, singleton: bool) -> String {
            if method_name.is_empty() {
                return method_name.to_string();
            }

            if !class_path.is_empty() {
                let join_char = if singleton { "." } else { "#" };
                return format!("{}{}{}", class_path, join_char, method_name);
            }

            method_name.to_string()
        }

        fn locate_method_entry<T>(
            ep: &*const usize,
            source: &T,
        ) -> Result<usize> where T: ProcessMemory {
            const VM_ENV_FLAG_LOCAL: usize = 0x2;
            let mut ep = ep.clone() as *mut usize;
            //let env_me_cref: usize = 0;
            let mut env_specval: usize = unsafe {
                source.copy_struct(ep.offset(-1) as usize).context(ep.offset(-1) as usize)?
            };
            let mut env_me_cref: usize = unsafe {
                source.copy_struct(ep.offset(-2) as usize).context(ep.offset(-1) as usize)?
            };


            while env_specval & VM_ENV_FLAG_LOCAL != 0 {
                if !check_method_entry(env_me_cref, source)?.is_null() {
                    break;
                }
                unsafe {
                    // https://github.com/ruby/ruby/blob/v3_4_5/vm_core.h#L1356
                    // we must strip off the GC marking bits from the EP, and mimic
                    // https://github.com/ruby/ruby/blob/v3_4_5/vm_core.h#L1501
                    ep = (env_specval.clone() & !0x03) as *mut usize;
                    env_specval = source.copy_struct(ep.offset(-1) as usize).context(ep.offset(-1) as usize)?;
                    env_me_cref = source.copy_struct(ep.offset(-2) as usize).context(ep.offset(-2) as usize)?;
                }
            }

            Ok(env_me_cref)
        }
        fn check_method_entry<T: ProcessMemory>(
            raw_imemo: usize,
            source: &T
        ) -> Result<*const rb_method_entry_struct> {
            //https://github.com/ruby/ruby/blob/v3_4_5/internal/imemo.h#L21
            const IMEMO_MASK: usize = 0x0f;
            let imemo: rb_method_entry_struct = source.copy_struct(raw_imemo).context(raw_imemo)?;

            // These type constants are defined in ruby's internal/imemo.h
            #[allow(non_upper_case_globals)]
            match ((imemo.flags >> ruby_fl_type_RUBY_FL_USHIFT) & IMEMO_MASK) as u32 {
                imemo_type_imemo_ment => Ok(&imemo as *const rb_method_entry_struct),
                imemo_type_imemo_svar => {
                    let svar: vm_svar = source.copy_struct(raw_imemo).context(raw_imemo)?;
                    check_method_entry(svar.cref_or_me as usize, source)
                },
                _ => Ok(raw_imemo as *const rb_method_entry_struct)
            }
        }

        // FIXME - cfunc_name should also be able to get classpath from CME
        // we should read that here and return the qualified name
        fn get_cfunc_name<T: ProcessMemory>(
            cfp: &rb_control_frame_t,
            global_symbols_address: usize,
            source: &T,
            _pid: Pid
        ) -> Result<String> {
            const IMEMO_MASK: usize = 0x0f;

            // The logic in this function is adapted from the .gdbinit script in
            // github.com/ruby/ruby, in particular the print_id function.
            let frame_flag: usize = unsafe {
                source.copy_struct(cfp.ep.offset(0) as usize).context(cfp.ep.offset(0) as usize)?
            };

            // if VM_FRAME_TYPE($cfp->flag) != VM_FRAME_MAGIC_CFUNC
            if frame_flag & 0xffff0001 != 0x55550001 {
                return Err(format_err!("Not a C function control frame").into());
            }

            let cme = locate_method_entry(&cfp.ep, source)?;
            let (class_path, singleton) = get_classpath(cme, true, source).unwrap_or(("".to_string(), false));

            let imemo: rb_method_entry_struct = source.copy_struct(cme).context(cme)?;
            if imemo.def.is_null() {
                return Err(format_err!("No method definition").into());
            }


            // FIXME - i'm pretty sure this mask is wrong, it should be 0xff
            let ttype = ((imemo.flags >> ruby_fl_type_RUBY_FL_USHIFT) & IMEMO_MASK) as usize;
            if ttype != imemo_type_imemo_ment as usize {
                return Err(format_err!("Not a method entry").into());
            }
            // TODO check the memo entry is of the CFUNC type now


            #[allow(non_camel_case_types)]
            type rb_id_serial_t = u32;

            // Declared in symbol.c prior to ruby 2.7.0, so not accessible by bindgen
            #[allow(non_camel_case_types)]
            #[repr(C)]
            #[derive(Debug, Copy, Clone)]
            struct rb_symbols_t {
                last_id: rb_id_serial_t,
                str_sym: *mut st_table,
                ids: VALUE,
                dsymbol_fstr_hash: VALUE,
            }

            let global_symbols: rb_symbols_t = source.copy_struct(global_symbols_address as usize).context(global_symbols_address as usize)?;
            let def: rb_method_definition_struct = source.copy_struct(imemo.def as usize).context(imemo.def as usize)?;
            let method_id = def.original_id as usize;

            // rb_id_to_serial
            let mut serial = method_id;
            if method_id > ruby_method_ids_tLAST_OP_ID as usize {
                serial = method_id >> ruby_id_types_RUBY_ID_SCOPE_SHIFT;
            }

            if serial > global_symbols.last_id as usize {
                return Err(format_err!("Invalid method ID").into());
            }

            // ID_ENTRY_UNIT is defined in symbol.c, so not accessible by bindgen
            let id_entry_unit = 512;
            let idx = serial / id_entry_unit;
            let ids: RArray = source.copy_struct(global_symbols.ids as usize).context(global_symbols.ids as usize)?;
            let flags = ids.basic.flags as usize;

            // string2cstring
            let mut ids_ptr = unsafe { ids.as_.heap.ptr as usize };
            let mut ids_len = unsafe { ids.as_.heap.len as usize };
            if (flags & ruby_fl_type_RUBY_FL_USER1 as usize) > 0 {
                ids_ptr = unsafe { ids.as_.ary[0] as usize };
                ids_len = (flags & (ruby_fl_type_RUBY_FL_USER3|ruby_fl_type_RUBY_FL_USER4|ruby_fl_type_RUBY_FL_USER5|ruby_fl_type_RUBY_FL_USER6|ruby_fl_type_RUBY_FL_USER7|ruby_fl_type_RUBY_FL_USER8|ruby_fl_type_RUBY_FL_USER9) as usize) >> (ruby_fl_type_RUBY_FL_USHIFT+3);
            }
            if idx >= ids_len {
                return Err(format_err!("Invalid index in IDs array").into());
            }

            // ids is an array of pointers to RArray. First jump to the right index to get the
            // pointer, then copy the _pointer_ into our memory space, and then finally copy the
            // pointed-to array into our memory space
            let array_remote_ptr = (ids_ptr as usize) + (idx as usize) * std::mem::size_of::<usize>();
            let array_ptr: usize = source.copy_struct(array_remote_ptr).context(array_remote_ptr)?;
            let array: RArray = source.copy_struct(array_ptr).context(array_ptr)?;

            let mut array_ptr = unsafe { array.as_.heap.ptr };
            let flags = array.basic.flags as usize;
            if (flags & ruby_fl_type_RUBY_FL_USER1 as usize) > 0 {
                array_ptr = unsafe { &ids.as_.ary[0] };
            }

            let offset = (serial % 512) * 2;
            let rstring_remote_ptr = (array_ptr as usize) + offset * std::mem::size_of::<usize>();
            let rstring_ptr: usize = source.copy_struct(rstring_remote_ptr as usize).context(rstring_remote_ptr as usize)?;
            let method_name = get_ruby_string(rstring_ptr as usize, source)?;
            Ok(qualified_method_name(&class_path, &method_name, singleton))
        }
    )
);

ruby_version_v_1_9_1!(ruby_1_9_1_0);
ruby_version_v_1_9_2_to_3!(ruby_1_9_2_0);
ruby_version_v_1_9_2_to_3!(ruby_1_9_3_0);
ruby_version_v_2_0_to_2_2!(ruby_2_0_0_0);
ruby_version_v_2_0_to_2_2!(ruby_2_1_0);
ruby_version_v_2_0_to_2_2!(ruby_2_1_1);
ruby_version_v_2_0_to_2_2!(ruby_2_1_2);
ruby_version_v_2_0_to_2_2!(ruby_2_1_3);
ruby_version_v_2_0_to_2_2!(ruby_2_1_4);
ruby_version_v_2_0_to_2_2!(ruby_2_1_5);
ruby_version_v_2_0_to_2_2!(ruby_2_1_6);
ruby_version_v_2_0_to_2_2!(ruby_2_1_7);
ruby_version_v_2_0_to_2_2!(ruby_2_1_8);
ruby_version_v_2_0_to_2_2!(ruby_2_1_9);
ruby_version_v_2_0_to_2_2!(ruby_2_1_10);
ruby_version_v_2_0_to_2_2!(ruby_2_2_0);
ruby_version_v_2_0_to_2_2!(ruby_2_2_1);
ruby_version_v_2_0_to_2_2!(ruby_2_2_2);
ruby_version_v_2_0_to_2_2!(ruby_2_2_3);
ruby_version_v_2_0_to_2_2!(ruby_2_2_4);
ruby_version_v_2_0_to_2_2!(ruby_2_2_5);
ruby_version_v_2_0_to_2_2!(ruby_2_2_6);
ruby_version_v_2_0_to_2_2!(ruby_2_2_7);
ruby_version_v_2_0_to_2_2!(ruby_2_2_8);
ruby_version_v_2_0_to_2_2!(ruby_2_2_9);
ruby_version_v_2_0_to_2_2!(ruby_2_2_10);
ruby_version_v_2_3_to_2_4!(ruby_2_3_0);
ruby_version_v_2_3_to_2_4!(ruby_2_3_1);
ruby_version_v_2_3_to_2_4!(ruby_2_3_2);
ruby_version_v_2_3_to_2_4!(ruby_2_3_3);
ruby_version_v_2_3_to_2_4!(ruby_2_3_4);
ruby_version_v_2_3_to_2_4!(ruby_2_3_5);
ruby_version_v_2_3_to_2_4!(ruby_2_3_6);
ruby_version_v_2_3_to_2_4!(ruby_2_3_7);
ruby_version_v_2_3_to_2_4!(ruby_2_3_8);
ruby_version_v_2_3_to_2_4!(ruby_2_4_0);
ruby_version_v_2_3_to_2_4!(ruby_2_4_1);
ruby_version_v_2_3_to_2_4!(ruby_2_4_2);
ruby_version_v_2_3_to_2_4!(ruby_2_4_3);
ruby_version_v_2_3_to_2_4!(ruby_2_4_4);
ruby_version_v_2_3_to_2_4!(ruby_2_4_5);
ruby_version_v_2_3_to_2_4!(ruby_2_4_6);
ruby_version_v_2_3_to_2_4!(ruby_2_4_7);
ruby_version_v_2_3_to_2_4!(ruby_2_4_8);
ruby_version_v_2_3_to_2_4!(ruby_2_4_9);
ruby_version_v_2_3_to_2_4!(ruby_2_4_10);
ruby_version_v2_5_x!(ruby_2_5_0);
ruby_version_v2_5_x!(ruby_2_5_1);
ruby_version_v2_5_x!(ruby_2_5_2);
ruby_version_v2_5_x!(ruby_2_5_3);
ruby_version_v2_5_x!(ruby_2_5_4);
ruby_version_v2_5_x!(ruby_2_5_5);
ruby_version_v2_5_x!(ruby_2_5_6);
ruby_version_v2_5_x!(ruby_2_5_7);
ruby_version_v2_5_x!(ruby_2_5_8);
ruby_version_v2_5_x!(ruby_2_5_9);
ruby_version_v2_6_x!(ruby_2_6_0);
ruby_version_v2_6_x!(ruby_2_6_1);
ruby_version_v2_6_x!(ruby_2_6_2);
ruby_version_v2_6_x!(ruby_2_6_3);
ruby_version_v2_6_x!(ruby_2_6_4);
ruby_version_v2_6_x!(ruby_2_6_5);
ruby_version_v2_6_x!(ruby_2_6_6);
ruby_version_v2_6_x!(ruby_2_6_7);
ruby_version_v2_6_x!(ruby_2_6_8);
ruby_version_v2_6_x!(ruby_2_6_9);
ruby_version_v2_6_x!(ruby_2_6_10);
ruby_version_v2_7_x!(ruby_2_7_0);
ruby_version_v2_7_x!(ruby_2_7_1);
ruby_version_v2_7_x!(ruby_2_7_2);
ruby_version_v2_7_x!(ruby_2_7_3);
ruby_version_v2_7_x!(ruby_2_7_4);
ruby_version_v2_7_x!(ruby_2_7_5);
ruby_version_v2_7_x!(ruby_2_7_6);
ruby_version_v2_7_x!(ruby_2_7_7);
ruby_version_v2_7_x!(ruby_2_7_8);
ruby_version_v3_0_x!(ruby_3_0_0);
ruby_version_v3_0_x!(ruby_3_0_1);
ruby_version_v3_0_x!(ruby_3_0_2);
ruby_version_v3_0_x!(ruby_3_0_3);
ruby_version_v3_0_x!(ruby_3_0_4);
ruby_version_v3_0_x!(ruby_3_0_5);
ruby_version_v3_0_x!(ruby_3_0_6);
ruby_version_v3_0_x!(ruby_3_0_7);
ruby_version_v3_1_x!(ruby_3_1_0);
ruby_version_v3_1_x!(ruby_3_1_1);
ruby_version_v3_1_x!(ruby_3_1_2);
ruby_version_v3_1_x!(ruby_3_1_3);
ruby_version_v3_1_x!(ruby_3_1_4);
ruby_version_v3_1_x!(ruby_3_1_5);
ruby_version_v3_1_x!(ruby_3_1_6);
ruby_version_v3_1_x!(ruby_3_1_7);
ruby_version_v3_2_x!(ruby_3_2_0);
ruby_version_v3_2_x!(ruby_3_2_1);
ruby_version_v3_2_x!(ruby_3_2_2);
ruby_version_v3_2_x!(ruby_3_2_3);
ruby_version_v3_2_x!(ruby_3_2_4);
ruby_version_v3_2_x!(ruby_3_2_5);
ruby_version_v3_2_x!(ruby_3_2_6);
ruby_version_v3_2_x!(ruby_3_2_7);
ruby_version_v3_2_x!(ruby_3_2_8);
ruby_version_v3_2_x!(ruby_3_2_9);
ruby_version_v3_3_x!(ruby_3_3_0);
ruby_version_v3_3_x!(ruby_3_3_1);
ruby_version_v3_3_x!(ruby_3_3_2);
ruby_version_v3_3_x!(ruby_3_3_3);
ruby_version_v3_3_x!(ruby_3_3_4);
ruby_version_v3_3_x!(ruby_3_3_5);
ruby_version_v3_3_x!(ruby_3_3_6);
ruby_version_v3_3_x!(ruby_3_3_7);
ruby_version_v3_3_x!(ruby_3_3_8);
ruby_version_v3_3_x!(ruby_3_3_9);
ruby_version_v3_3_x!(ruby_3_4_0);
ruby_version_v3_3_x!(ruby_3_4_1);
ruby_version_v3_3_x!(ruby_3_4_2);
ruby_version_v3_3_x!(ruby_3_4_3);
ruby_version_v3_3_x!(ruby_3_4_4);
ruby_version_v3_3_x!(ruby_3_4_5);
ruby_version_v3_3_x!(ruby_3_4_6);
ruby_version_v3_3_x!(ruby_3_4_7);

#[cfg(not(debug_assertions))]
#[cfg(test)]
mod tests {
    use rbspy_testdata::*;

    use crate::core::ruby_version;
    use crate::core::types::StackFrame;

    fn real_stack_trace_1_9_3() -> Vec<StackFrame> {
        vec![
            StackFrame::unknown_c_function(),
            StackFrame {
                name: "aaa".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(2),
            },
            StackFrame {
                name: "bbb".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(6),
            },
            StackFrame {
                name: "ccc".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(10),
            },
            StackFrame {
                name: "block in <main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(14),
            },
            StackFrame::unknown_c_function(),
            StackFrame::unknown_c_function(),
            StackFrame {
                name: "<main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(13),
            },
            StackFrame::unknown_c_function(),
        ]
    }

    fn real_stack_trace_2_7_2() -> Vec<StackFrame> {
        vec![
            StackFrame {
                name: "sleep [c function]".to_string(),
                relative_path: "(unknown)".to_string(),
                absolute_path: None,
                lineno: None,
            },
            StackFrame {
                name: "aaa".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/vagrant/ci/ruby-programs/infinite.rb".to_string()),
                lineno: Some(3),
            },
            StackFrame {
                name: "bbb".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/vagrant/ci/ruby-programs/infinite.rb".to_string()),
                lineno: Some(7),
            },
            StackFrame {
                name: "ccc".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/vagrant/ci/ruby-programs/infinite.rb".to_string()),
                lineno: Some(11),
            },
            StackFrame {
                name: "block in <main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/vagrant/ci/ruby-programs/infinite.rb".to_string()),
                lineno: Some(15),
            },
            StackFrame {
                name: "loop [c function]".to_string(),
                relative_path: "(unknown)".to_string(),
                absolute_path: None,
                lineno: None,
            },
        ]
    }

    fn real_stack_trace_3_1_0() -> Vec<StackFrame> {
        vec![
            StackFrame {
                name: "sleep [c function]".to_string(),
                relative_path: "(unknown)".to_string(),
                absolute_path: None,
                lineno: None,
            },
            StackFrame {
                name: "aaa".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/acj/workspace/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(3),
            },
            StackFrame {
                name: "bbb".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/acj/workspace/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(7),
            },
            StackFrame {
                name: "ccc".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/acj/workspace/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(11),
            },
            StackFrame {
                name: "block in <main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/acj/workspace/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(15),
            },
            StackFrame {
                name: "loop [c function]".to_string(),
                relative_path: "(unknown)".to_string(),
                absolute_path: None,
                lineno: None,
            },
        ]
    }

    fn real_stack_trace_3_2_0() -> Vec<StackFrame> {
        vec![
            StackFrame {
                name: "sleep [c function]".to_string(),
                relative_path: "(unknown)".to_string(),
                absolute_path: None,
                lineno: None,
            },
            StackFrame {
                name: "aaa".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/parallels/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(3),
            },
            StackFrame {
                name: "bbb".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/parallels/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(7),
            },
            StackFrame {
                name: "ccc".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/parallels/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(11),
            },
            StackFrame {
                name: "block in <main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/parallels/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(15),
            },
            StackFrame {
                name: "loop [c function]".to_string(),
                relative_path: "(unknown)".to_string(),
                absolute_path: None,
                lineno: None,
            },
            StackFrame {
                name: "<main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/parallels/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(13),
            },
        ]
    }

    fn real_stack_trace_3_3_0() -> Vec<StackFrame> {
        vec![
            StackFrame {
                name: "sleep [c function]".to_string(),
                relative_path: "(unknown)".to_string(),
                absolute_path: None,
                lineno: None,
            },
            StackFrame {
                name: "Object#aaa".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/runner/work/rbspy/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(3),
            },
            StackFrame {
                name: "Object#bbb".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/runner/work/rbspy/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(7),
            },
            StackFrame {
                name: "Object#ccc".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/runner/work/rbspy/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(11),
            },
            StackFrame {
                name: "block in <main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/runner/work/rbspy/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(15),
            },
            StackFrame {
                name: "loop".to_string(),
                relative_path: "<internal:kernel>".to_string(),
                absolute_path: Some("unknown".to_string()),
                lineno: Some(192),
            },
        ]
    }

    fn real_stack_trace_with_classes_3_3_0() -> Vec<StackFrame> {
        vec![
            StackFrame {
                name: "sleep [c function]".to_string(),
                relative_path: "(unknown)".to_string(),
                absolute_path: None,
                lineno: None,
            },
            StackFrame {
                name: "A#aaa".to_string(),
                relative_path: "ci/ruby-programs/infinite_on_cpu_with_classes.rb".to_string(),
                absolute_path: Some("/home/runner/work/rbspy/rbspy/ci/ruby-programs/infinite_on_cpu_with_classes.rb".to_string()),
                lineno: Some(10)
            },
            StackFrame {
                name: "B::Ab#bbb".to_string(),
                relative_path: "ci/ruby-programs/infinite_on_cpu_with_classes.rb".to_string(),
                absolute_path: Some("/home/runner/work/rbspy/rbspy/ci/ruby-programs/infinite_on_cpu_with_classes.rb".to_string()),
                lineno: Some(17)
            },
            StackFrame {
                name: "C::Cb#ccc".to_string(),
                relative_path: "ci/ruby-programs/infinite_on_cpu_with_classes.rb".to_string(),
                absolute_path: Some("/home/runner/work/rbspy/rbspy/ci/ruby-programs/infinite_on_cpu_with_classes.rb".to_string()),
                lineno: Some(25)
            },
            StackFrame {
                name: "block in looper".to_string(),
                relative_path: "ci/ruby-programs/infinite_on_cpu_with_classes.rb".to_string(),
                absolute_path: Some("/home/runner/work/rbspy/rbspy/ci/ruby-programs/infinite_on_cpu_with_classes.rb".to_string()),
                lineno: Some(32)
            },
            StackFrame {
                name: "loop".to_string(),
                relative_path: "<internal:kernel>".to_string(),
                absolute_path: Some("unknown".to_string()),
                lineno: Some(192)
            },
            StackFrame {
                name: "Object#looper".to_string(),
                relative_path: "ci/ruby-programs/infinite_on_cpu_with_classes.rb".to_string(),
                absolute_path: Some("/home/runner/work/rbspy/rbspy/ci/ruby-programs/infinite_on_cpu_with_classes.rb".to_string()),
                lineno: Some(33)
            },
        ]
    }

    fn real_stack_trace_main() -> Vec<StackFrame> {
        vec![
            StackFrame::unknown_c_function(),
            StackFrame {
                name: "aaa".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(2),
            },
            StackFrame {
                name: "bbb".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(6),
            },
            StackFrame {
                name: "ccc".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(10),
            },
            StackFrame {
                name: "block in <main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(14),
            },
            StackFrame::unknown_c_function(),
            StackFrame {
                name: "<main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(13),
            },
        ]
    }

    fn real_stack_trace() -> Vec<StackFrame> {
        vec![
            StackFrame::unknown_c_function(),
            StackFrame {
                name: "aaa".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(2),
            },
            StackFrame {
                name: "bbb".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(6),
            },
            StackFrame {
                name: "ccc".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(10),
            },
            StackFrame {
                name: "block in <main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: Some(14),
            },
            StackFrame::unknown_c_function(),
        ]
    }

    // These tests on core dumps don't work on 32bit platforms (error is
    // "Not enough memory resources are available to complete this operation.")
    // disable.
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_1_9_3() {
        let current_thread_addr = 0x823930;
        let stack_trace = ruby_version::ruby_1_9_3_0::get_stack_trace::<CoreDump>(
            current_thread_addr,
            0,
            None,
            &coredump_1_9_3(),
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_1_9_3(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_2_1_6() {
        let current_thread_addr = 0x562658abd7f0;
        let stack_trace = ruby_version::ruby_2_1_6::get_stack_trace::<CoreDump>(
            current_thread_addr,
            0,
            None,
            &coredump_2_1_6(),
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_main(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_2_1_6_2() {
        // this stack is from a ruby program that is just running `select`
        let current_thread_addr = 0x562efcd577f0;
        let stack_trace = ruby_version::ruby_2_1_6::get_stack_trace(
            current_thread_addr,
            0,
            None,
            &coredump_2_1_6_c_function(),
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(vec!(StackFrame::unknown_c_function()), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_2_4_0() {
        let current_thread_addr = 0x55df44959920;
        let stack_trace = ruby_version::ruby_2_4_0::get_stack_trace::<CoreDump>(
            current_thread_addr,
            0,
            None,
            &coredump_2_4_0(),
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_2_5_0() {
        let current_thread_addr = 0x55dd8c3b7758;
        let stack_trace = ruby_version::ruby_2_5_0::get_stack_trace::<CoreDump>(
            current_thread_addr,
            0,
            None,
            &coredump_2_5_0(),
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_2_7_2() {
        let current_thread_addr = 0x7fdd8d626070;
        let global_symbols_addr = Some(0x7fdd8d60eb80);
        let stack_trace = ruby_version::ruby_2_7_2::get_stack_trace::<CoreDump>(
            current_thread_addr,
            0,
            global_symbols_addr,
            &coredump_2_7_2(),
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_2_7_2(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_2_7_3() {
        let current_thread_addr = 0x7fdd8d626070;
        let global_symbols_addr = Some(0x7fdd8d60eb80);
        let stack_trace = ruby_version::ruby_2_7_3::get_stack_trace::<CoreDump>(
            current_thread_addr,
            0,
            global_symbols_addr,
            &coredump_2_7_2(),
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_2_7_2(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_2_7_4() {
        let current_thread_addr = 0x7fdd8d626070;
        let global_symbols_addr = Some(0x7fdd8d60eb80);
        let stack_trace = ruby_version::ruby_2_7_4::get_stack_trace::<CoreDump>(
            current_thread_addr,
            0,
            global_symbols_addr,
            &coredump_2_7_2(),
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_2_7_2(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_2_7_5() {
        let current_thread_addr = 0x7fdd8d626070;
        let global_symbols_addr = Some(0x7fdd8d60eb80);
        let stack_trace = ruby_version::ruby_2_7_5::get_stack_trace::<CoreDump>(
            current_thread_addr,
            0,
            global_symbols_addr,
            &coredump_2_7_2(),
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_2_7_2(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_2_7_6() {
        let current_thread_addr = 0x7fdd8d626070;
        let global_symbols_addr = Some(0x7fdd8d60eb80);
        let stack_trace = ruby_version::ruby_2_7_6::get_stack_trace::<CoreDump>(
            current_thread_addr,
            0,
            global_symbols_addr,
            &coredump_2_7_2(),
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_2_7_2(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_2_7_7() {
        let current_thread_addr = 0x7fdd8d626070;
        let global_symbols_addr = Some(0x7fdd8d60eb80);
        let stack_trace = ruby_version::ruby_2_7_7::get_stack_trace::<CoreDump>(
            current_thread_addr,
            0,
            global_symbols_addr,
            &coredump_2_7_2(),
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_2_7_2(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_2_7_8() {
        let current_thread_addr = 0x7fdd8d626070;
        let global_symbols_addr = Some(0x7fdd8d60eb80);
        let stack_trace = ruby_version::ruby_2_7_8::get_stack_trace::<CoreDump>(
            current_thread_addr,
            0,
            global_symbols_addr,
            &coredump_2_7_2(),
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_2_7_2(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_0_0() {
        let source = coredump_3_0_0();
        let vm_addr = 0x7fdacdab7470;
        let global_symbols_addr = Some(0x7fdacdaa9d80);
        let stack_trace = ruby_version::ruby_3_0_0::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_2_7_2(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_0_1() {
        let source = coredump_3_0_0();
        let vm_addr = 0x7fdacdab7470;
        let global_symbols_addr = Some(0x7fdacdaa9d80);
        let stack_trace = ruby_version::ruby_3_0_1::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_2_7_2(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_0_2() {
        let source = coredump_3_0_0();
        let vm_addr = 0x7fdacdab7470;
        let global_symbols_addr = Some(0x7fdacdaa9d80);
        let stack_trace = ruby_version::ruby_3_0_2::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_2_7_2(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_0_3() {
        let source = coredump_3_0_0();
        let vm_addr = 0x7fdacdab7470;
        let global_symbols_addr = Some(0x7fdacdaa9d80);
        let stack_trace = ruby_version::ruby_3_0_3::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_2_7_2(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_0_4() {
        let source = coredump_3_0_0();
        let vm_addr = 0x7fdacdab7470;
        let global_symbols_addr = Some(0x7fdacdaa9d80);
        let stack_trace = ruby_version::ruby_3_0_4::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_2_7_2(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_0_5() {
        let source = coredump_3_0_0();
        let vm_addr = 0x7fdacdab7470;
        let global_symbols_addr = Some(0x7fdacdaa9d80);
        let stack_trace = ruby_version::ruby_3_0_5::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_2_7_2(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_0_6() {
        let source = coredump_3_0_0();
        let vm_addr = 0x7fdacdab7470;
        let global_symbols_addr = Some(0x7fdacdaa9d80);
        let stack_trace = ruby_version::ruby_3_0_6::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_2_7_2(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_0_7() {
        let source = coredump_3_0_0();
        let vm_addr = 0x7fdacdab7470;
        let global_symbols_addr = Some(0x7fdacdaa9d80);
        let stack_trace = ruby_version::ruby_3_0_7::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_2_7_2(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_1_0() {
        let source = coredump_3_1_0();
        let vm_addr = 0x7f0dc0c83c58;
        let global_symbols_addr = Some(0x7f0dc0c75e80);
        let stack_trace = ruby_version::ruby_3_1_0::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_1_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_1_1() {
        let source = coredump_3_1_0();
        let vm_addr = 0x7f0dc0c83c58;
        let global_symbols_addr = Some(0x7f0dc0c75e80);
        let stack_trace = ruby_version::ruby_3_1_1::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_1_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_1_2() {
        let source = coredump_3_1_0();
        let vm_addr = 0x7f0dc0c83c58;
        let global_symbols_addr = Some(0x7f0dc0c75e80);
        let stack_trace = ruby_version::ruby_3_1_2::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_1_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_1_3() {
        let source = coredump_3_1_0();
        let vm_addr = 0x7f0dc0c83c58;
        let global_symbols_addr = Some(0x7f0dc0c75e80);
        let stack_trace = ruby_version::ruby_3_1_3::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_1_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_1_4() {
        let source = coredump_3_1_0();
        let vm_addr = 0x7f0dc0c83c58;
        let global_symbols_addr = Some(0x7f0dc0c75e80);
        let stack_trace = ruby_version::ruby_3_1_4::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_1_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_1_5() {
        let source = coredump_3_1_0();
        let vm_addr = 0x7f0dc0c83c58;
        let global_symbols_addr = Some(0x7f0dc0c75e80);
        let stack_trace = ruby_version::ruby_3_1_5::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_1_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_1_6() {
        let source = coredump_3_1_0();
        let vm_addr = 0x7f0dc0c83c58;
        let global_symbols_addr = Some(0x7f0dc0c75e80);
        let stack_trace = ruby_version::ruby_3_1_6::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_1_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_1_7() {
        let source = coredump_3_1_0();
        let vm_addr = 0x7f0dc0c83c58;
        let global_symbols_addr = Some(0x7f0dc0c75e80);
        let stack_trace = ruby_version::ruby_3_1_7::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_1_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_2_0() {
        let source = coredump_3_2_0();
        let vm_addr = 0xffffb8034578;
        let global_symbols_addr = Some(0xffffb8025340);
        let stack_trace = ruby_version::ruby_3_2_0::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_2_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_2_1() {
        let source = coredump_3_2_0();
        let vm_addr = 0xffffb8034578;
        let global_symbols_addr = Some(0xffffb8025340);
        let stack_trace = ruby_version::ruby_3_2_1::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_2_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_2_2() {
        let source = coredump_3_2_0();
        let vm_addr = 0xffffb8034578;
        let global_symbols_addr = Some(0xffffb8025340);
        let stack_trace = ruby_version::ruby_3_2_2::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_2_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_2_3() {
        let source = coredump_3_2_0();
        let vm_addr = 0xffffb8034578;
        let global_symbols_addr = Some(0xffffb8025340);
        let stack_trace = ruby_version::ruby_3_2_3::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_2_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_2_4() {
        let source = coredump_3_2_0();
        let vm_addr = 0xffffb8034578;
        let global_symbols_addr = Some(0xffffb8025340);
        let stack_trace = ruby_version::ruby_3_2_4::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_2_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_2_5() {
        let source = coredump_3_2_0();
        let vm_addr = 0xffffb8034578;
        let global_symbols_addr = Some(0xffffb8025340);
        let stack_trace = ruby_version::ruby_3_2_5::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_2_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_2_6() {
        let source = coredump_3_2_0();
        let vm_addr = 0xffffb8034578;
        let global_symbols_addr = Some(0xffffb8025340);
        let stack_trace = ruby_version::ruby_3_2_6::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_2_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_2_7() {
        let source = coredump_3_2_0();
        let vm_addr = 0xffffb8034578;
        let global_symbols_addr = Some(0xffffb8025340);
        let stack_trace = ruby_version::ruby_3_2_7::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_2_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_2_8() {
        let source = coredump_3_2_0();
        let vm_addr = 0xffffb8034578;
        let global_symbols_addr = Some(0xffffb8025340);
        let stack_trace = ruby_version::ruby_3_2_8::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_2_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_3_0() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_3_0::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_with_classes_3_3_0() {
        let source = coredump_with_classes_3_3_0();
        let vm_addr = 0x7f58cb7f4988;
        let global_symbols_addr = Some(0x7f58cb7e3c60);
        let stack_trace = ruby_version::ruby_3_3_0::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(
            real_stack_trace_with_classes_3_3_0(),
            stack_trace.unwrap().trace
        );
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_3_1() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_3_1::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_3_2() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_3_2::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_3_3() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_3_3::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_3_4() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_3_4::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_3_5() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_3_5::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_3_6() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_3_6::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_3_7() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_3_7::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_3_8() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_3_8::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_4_0() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_4_0::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_4_1() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_4_1::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_4_2() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_4_2::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_4_3() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_4_3::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_4_4() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_4_4::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_4_5() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_4_5::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_4_6() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_4_6::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.unwrap().trace);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_get_ruby_stack_trace_3_4_7() {
        let source = coredump_3_3_0();
        let vm_addr = 0x7f7ff21f1868;
        let global_symbols_addr = Some(0x7f7ff21e0c60);
        let stack_trace = ruby_version::ruby_3_4_7::get_stack_trace::<CoreDump>(
            0,
            vm_addr,
            global_symbols_addr,
            &source,
            0,
            false,
        )
        .unwrap();
        assert_eq!(real_stack_trace_3_3_0(), stack_trace.unwrap().trace);
    }
}
