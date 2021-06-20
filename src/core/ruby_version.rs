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
            use crate::core::types::ProcessMemory;

            get_stack_trace!(rb_thread_struct);
            get_execution_context_from_thread!(rb_thread_struct);
            get_ruby_string!();
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
            use crate::core::types::ProcessMemory;

            get_stack_trace!(rb_thread_struct);
            get_execution_context_from_thread!(rb_thread_struct);
            get_ruby_string!();
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
           use crate::core::types::ProcessMemory;

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
            get_ruby_string!();
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
           use crate::core::types::ProcessMemory;

            get_stack_trace!(rb_thread_struct);
            get_execution_context_from_thread!(rb_thread_struct);
            get_ruby_string!();
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
           use crate::core::types::ProcessMemory;

            get_stack_trace!(rb_execution_context_struct);
            get_execution_context_from_thread!(rb_execution_context_struct);
            get_ruby_string!();
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
        }
    )
);

macro_rules! ruby_version_v2_6_x(
    ($ruby_version:ident) => (
       pub mod $ruby_version {
           use std;
           use anyhow::{Context, format_err, Result};
           use bindings::$ruby_version::*;
           use crate::core::types::ProcessMemory;

            get_stack_trace!(rb_execution_context_struct);
            get_execution_context_from_thread!(rb_execution_context_struct);
            get_ruby_string!();
            get_cfps!();
            get_pos!(rb_iseq_constant_body);
            get_lineno_2_6_0!();
            get_stack_frame_2_5_0!();
            stack_field_2_5_0!();
            get_ruby_string_array_2_5_0!();
            get_thread_status_2_6_0!();
            get_thread_id_2_5_0!();
            #[cfg(any(target_os = "freebsd", target_os = "macos", target_os = "windows"))]
            get_cfunc_name_unsupported!();
            #[cfg(target_os = "linux")]
            get_cfunc_name!();
        }
    )
);

macro_rules! ruby_version_v2_7_x(
    ($ruby_version:ident) => (
        pub mod $ruby_version {
            use std;
            use anyhow::{Context, format_err, Result};
            use bindings::$ruby_version::*;
            use crate::core::types::ProcessMemory;

            get_stack_trace!(rb_execution_context_struct);
            get_execution_context_from_thread!(rb_execution_context_struct);
            get_ruby_string!();
            get_cfps!();
            get_pos!(rb_iseq_constant_body);
            get_lineno_2_6_0!();
            get_stack_frame_2_5_0!();
            stack_field_2_5_0!();
            get_ruby_string_array_2_5_0!();
            get_thread_status_2_6_0!();
            get_thread_id_2_5_0!();
            get_cfunc_name!();
        }
    )
);

macro_rules! ruby_version_v3_0_x(
    ($ruby_version:ident) => (
        pub mod $ruby_version {
            use std;
            use anyhow::{Context, format_err, Result};
            use bindings::$ruby_version::*;
            use crate::core::types::ProcessMemory;

            get_stack_trace!(rb_execution_context_struct);
            get_execution_context_from_vm!();
            get_ruby_string!();
            get_cfps!();
            get_pos!(rb_iseq_constant_body);
            get_lineno_2_6_0!();
            get_stack_frame_2_5_0!();
            stack_field_2_5_0!();
            get_ruby_string_array_2_5_0!();
            get_thread_status_2_6_0!();
            get_thread_id_2_5_0!();
            get_cfunc_name!();

            #[allow(non_upper_case_globals)]
            const ruby_fl_type_RUBY_FL_USHIFT: ruby_fl_type = ruby_fl_ushift_RUBY_FL_USHIFT as i32;
        }
    )
);

macro_rules! get_execution_context_from_thread(
    ($thread_type:ident) => (
        pub fn get_execution_context<T: ProcessMemory>(
            ruby_current_thread_address_location: usize,
            _ruby_vm_address_location: usize,
            source: &T
        ) -> Result<$thread_type, MemoryCopyError> {
            let current_thread_addr: usize = source.copy_struct(ruby_current_thread_address_location)
                .context(ruby_current_thread_address_location)?;
            let thread: $thread_type = source.copy_struct(current_thread_addr)
                .context(current_thread_addr)?;
            Ok(thread)
        }
    )
);

macro_rules! get_execution_context_from_vm(
    () => (
        pub fn get_execution_context<T: ProcessMemory>(
            _ruby_current_thread_address_location: usize,
            ruby_vm_address_location: usize,
            source: &T
        ) -> Result<rb_execution_context_struct, MemoryCopyError> {
            // This is a roundabout way to get the execution context address, but it helps us
            // avoid platform-specific structures in memory (e.g. pthread types) that would
            // require us to maintain separate ruby-structs bindings for each platform due to
            // their varying sizes and alignments.
            let vm_addr: usize = source.copy_struct(ruby_vm_address_location)
                .context(ruby_vm_address_location)?;
            let vm: rb_vm_struct = source.copy_struct(vm_addr as usize)
                .context(vm_addr)?;

            // Seek forward in the ractor struct, looking for the main thread's address. There
            // may be other copies of the main thread address in the ractor struct, so it's
            // important to jump as close as possible to the main_thread struct field before we
            // search, which is the purpose of the initial offset. The execution context pointer
            // is in the memory word just before main_thread (see rb_ractor_struct).
            const ADDRESSES_TO_CHECK: usize = 64;
            let initial_offset = 520; // Found through experiment
            let main_ractor_address = vm.ractor.main_ractor as usize;
            let candidate_addresses: [usize; ADDRESSES_TO_CHECK] =
                source.copy_struct(main_ractor_address + initial_offset)
                    .context(main_ractor_address)?;
            let matching_index =
                candidate_addresses
                .iter()
                .position(|&addr| addr == vm.ractor.main_thread as usize)
                .ok_or(format_err!("couldn't find current execution context"))?;
            let running_ec_address = candidate_addresses[matching_index - 1];
            let ec: rb_execution_context_struct = source.copy_struct(running_ec_address as usize)
                .context(running_ec_address)?;
            Ok(ec)
        }
    )
);

macro_rules! get_stack_trace(
    ($thread_type:ident) => (
        use crate::core::types::*;
        use crate::core::types::StackFrame;

        pub fn get_stack_trace<T: ProcessMemory>(
            ruby_current_thread_address_location: usize,
            ruby_vm_address_location: usize,
            ruby_global_symbols_address_location: Option<usize>,
            source: &T,
            pid: Pid,
            on_cpu: bool,
        ) -> Result<Option<StackTrace>, MemoryCopyError> {
            let thread: $thread_type = get_execution_context(ruby_current_thread_address_location, ruby_vm_address_location, source)
                .context(ruby_current_thread_address_location)?;

            // testing the thread state in the interpreter.
            if on_cpu && get_thread_status(&thread, source)? != rb_thread_status_THREAD_RUNNABLE /* THREAD_RUNNABLE */ {
                /* This is in addition to any OS-specific checks for thread activity,
                 * and provides an extra measure of reliability for targets that haven't got them.
                 * Another added value for doing this is that it works for coredump targets. */
                return Ok(None);
            }

            let thread_id = get_thread_id(&thread, source)?;

            if stack_field(&thread) as usize == 0 {
                return Ok(Some(StackTrace {
                    pid: Some(pid),
                    trace: vec!(StackFrame::unknown_c_function()),
                    thread_id: Some(thread_id),
                    time: Some(SystemTime::now())
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
                                    lineno: 0
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
                    .context(cfp.iseq as usize)?;

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
            Ok(Some(StackTrace{trace, pid: Some(pid), thread_id: Some(thread_id), time: Some(SystemTime::now())}))
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
            stack_field(thread) + stack_size_field(thread) * std::mem::size_of::<VALUE>() as i64 - 1 * std::mem::size_of::<rb_control_frame_t>() as i64
        }

        pub fn is_maybe_thread<T>(x: usize, x_addr: usize, source: &T, all_maps: &[MapRange]) -> bool where T: ProcessMemory {
            if !maps_contain_addr(x, all_maps) {
                return false;
            }

            let thread: $thread_type = match source.copy_struct(x) {
                Ok(x) => x,
                _ => { return false; },
            };

            if !could_be_thread(&thread, &all_maps) {
                return false;
            }

            // finally, try to get an actual stack trace from the source and see if it works
            get_stack_trace(x_addr, 0, None, source, 0, false).is_ok()
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

        fn get_thread_status<T>(thread_struct: &rb_thread_struct, _source: &T) -> Result<u32, MemoryCopyError> {
            Ok(thread_struct.status as u32)
        }
    )
);

macro_rules! get_thread_status_2_5_0(
    () => (

        fn get_thread_status<T>(thread_struct: &rb_execution_context_struct, source: &T)
                            -> Result<u32, MemoryCopyError> where T: ProcessMemory {
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
                            -> Result<u32, MemoryCopyError> where T: ProcessMemory {
            let thread: rb_thread_struct = source.copy_struct(thread_struct.thread_ptr as usize)
                .context(thread_struct.thread_ptr as usize)?;
            Ok(thread.status() as u32)
        }
    )
);

macro_rules! get_thread_id_1_9_0(
    () => (

        fn get_thread_id<T>(thread_struct: &rb_thread_struct, _source: &T) -> Result<usize, MemoryCopyError> {
            Ok(thread_struct.thread_id as usize)
        }
    )
);

macro_rules! get_thread_id_2_5_0(
    () => (

        fn get_thread_id<T>(thread_struct: &rb_execution_context_struct, source: &T)
                            -> Result<usize, MemoryCopyError> where T: ProcessMemory {
            let thread: rb_thread_struct = source.copy_struct(thread_struct.thread_ptr as usize)
                .context(thread_struct.thread_ptr as usize)?;
            Ok(thread.thread_id as usize)
        }
    )
);

macro_rules! get_ruby_string_array_2_5_0(
    () => (
        // Returns (path, absolute_path)
        fn get_ruby_string_array<T>(addr: usize, string_class: usize, source: &T) -> Result<(String, String), MemoryCopyError> where T: ProcessMemory {
            // todo: we're doing an extra copy here for no reason
            let rstring: RString = source.copy_struct(addr)
                .context(addr)?;
            if rstring.basic.klass as usize == string_class {
                let s = get_ruby_string(addr, source)?;
                return Ok((s.clone(), s))
            }
            // otherwise it's an RArray
            let rarray: RArray = source.copy_struct(addr)
                .context(addr)?;
            // TODO: this assumes that the array contents are stored inline and not on the heap
            // I think this will always be true but we should check instead
            // the reason I am not checking is that I don't know how to check yet
            let path_addr: usize = unsafe { rarray.as_.ary[0] as usize }; // 1 means get the absolute path, not the relative path
            let abs_path_addr: usize = unsafe { rarray.as_.ary[1] as usize }; // 1 means get the absolute path, not the relative path
            let rel_path = get_ruby_string(path_addr, source)?;
            // In the case of internal ruby functions (and maybe others), we may not get a valid
            // pointer here
            let abs_path = get_ruby_string(abs_path_addr, source)
                .unwrap_or(String::from("unknown"));
            Ok((rel_path, abs_path))
        }
    )
);

macro_rules! get_ruby_string(
    () => (
        use std::ffi::CStr;

        fn get_ruby_string<T>(
            addr: usize,
            source: &T
        ) -> Result<String, MemoryCopyError> where T: ProcessMemory {
            let vec = {
                let rstring: RString = source.copy_struct(addr)
                    .context(addr)?;
                let basic = rstring.basic;
                let is_array = basic.flags & 1 << 13 == 0;
                if is_array {
                    unsafe { CStr::from_ptr(rstring.as_.ary.as_ref().as_ptr() as *const libc::c_char) }
                    .to_bytes()
                    .to_vec()
                } else {
                    unsafe {
                        let addr = rstring.as_.heap.ptr as usize;
                        let len = rstring.as_.heap.len as usize;
                        let result = source.copy(addr as usize, len);
                        match result {
                            Err(x) => {
                                debug!("Error: Failed to get ruby string.\nrstring: {:?}, addr: {}, len: {}", rstring, addr, len);
                                return Err(x).context(addr)?;
                            }
                            Ok(x) => x
                        }
                    }
                }
            };

            let error =
                MemoryCopyError::Message("Ruby string is invalid".to_string());

            String::from_utf8(vec).or(Err(error))
        }
    )
);

macro_rules! get_stack_frame_1_9_1(
    () => (
        fn get_stack_frame<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
            ) -> Result<StackFrame, MemoryCopyError> where T: ProcessMemory {
            Ok(StackFrame{
                name: get_ruby_string(iseq_struct.name as usize, source)?,
                relative_path: get_ruby_string(iseq_struct.filename as usize, source)?,
                absolute_path: None,
                lineno: get_lineno(iseq_struct, cfp, source)?,
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
            ) -> Result<StackFrame, MemoryCopyError> where T: ProcessMemory {
            Ok(StackFrame{
                name: get_ruby_string(iseq_struct.name as usize, source)?,
                relative_path: get_ruby_string(iseq_struct.filename as usize, source)?,
                absolute_path: Some(get_ruby_string(iseq_struct.filepath as usize, source)?),
                lineno: get_lineno(iseq_struct, cfp, source)?,
            })
        }
    )
);

macro_rules! get_lineno_1_9_0(
    () => (
        fn get_lineno<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
        ) -> Result<u32, MemoryCopyError> where T: ProcessMemory {
            let pos = get_pos(iseq_struct, cfp)?;
            let t_size = iseq_struct.insn_info_size as usize;
            if t_size == 0 {
                Ok(0) //TODO: really?
            } else if t_size == 1 {
                let table: [iseq_insn_info_entry; 1] = source.copy_struct(iseq_struct.insn_info_table as usize)
                    .context(iseq_struct.insn_info_table as usize)?;
                Ok(table[0].line_no as u32)
            } else {
                let table: Vec<iseq_insn_info_entry> = source.copy_vec(iseq_struct.insn_info_table as usize, t_size as usize)
                    .context(iseq_struct.insn_info_table as usize)?;
                for i in 0..t_size {
                    if pos == table[i].position as usize {
                        return Ok(table[i].line_no as u32)
                    } else if table[i].position as usize > pos {
                        return Ok(table[i-1].line_no as u32)
                    }
                }
                Ok(table[t_size-1].line_no as u32)
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
        ) -> Result<u32, MemoryCopyError> where T: ProcessMemory {
            let pos = get_pos(iseq_struct, cfp)?;
            let t_size = iseq_struct.line_info_size as usize;
            if t_size == 0 {
                Ok(0) //TODO: really?
            } else if t_size == 1 {
                let table: [iseq_line_info_entry; 1] = source.copy_struct(iseq_struct.line_info_table as usize)
                    .context(iseq_struct.line_info_table as usize)?;
                Ok(table[0].line_no)
            } else {
                let table: Vec<iseq_line_info_entry> = source.copy_vec(iseq_struct.line_info_table as usize, t_size as usize)
                    .context(iseq_struct.line_info_table as usize)?;
                for i in 0..t_size {
                    if pos == table[i].position as usize {
                        return Ok(table[i].line_no)
                    } else if table[i].position as usize > pos {
                        return Ok(table[i-1].line_no)
                    }
                }
                Ok(table[t_size-1].line_no)
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
        ) -> Result<u32, MemoryCopyError> where T: ProcessMemory {
            let pos = get_pos(iseq_struct, cfp)?;
            let t_size = iseq_struct.line_info_size as usize;
            if t_size == 0 {
                Ok(0) //TODO: really?
            } else if t_size == 1 {
                let table: [iseq_line_info_entry; 1] = source.copy_struct(iseq_struct.line_info_table as usize)
                    .context(iseq_struct.line_info_table as usize)?;
                Ok(table[0].line_no)
            } else {
                let table: Vec<iseq_line_info_entry> = source.copy_vec(iseq_struct.line_info_table as usize, t_size as usize)
                    .context(iseq_struct.line_info_table as usize)?;
                for i in 0..t_size {
                    if pos == table[i].position as usize {
                        return Ok(table[i].line_no)
                    } else if table[i].position as usize > pos {
                        return Ok(table[i-1].line_no)
                    }
                }
                Ok(table[t_size-1].line_no)
            }
        }
    )
);

macro_rules! get_pos(
    ($iseq_type:ident) => (
        #[allow(unused)] // this doesn't get used in every ruby version
        fn get_pos(iseq_struct: &$iseq_type, cfp: &rb_control_frame_t) -> Result<usize, MemoryCopyError> {
            if (cfp.pc as usize) < (iseq_struct.iseq_encoded as usize) {
                return Err(MemoryCopyError::Message(format!("program counter and iseq are out of sync")));
            }
            let mut pos = cfp.pc as usize - iseq_struct.iseq_encoded as usize;
            if pos != 0 {
                pos -= 1;
            }
            Ok(pos)
        }
    )
);

macro_rules! get_lineno_2_5_0(
    () => (
        fn get_lineno<T>(
            iseq_struct: &rb_iseq_constant_body,
            cfp: &rb_control_frame_t,
            source: &T,
        ) -> Result<u32, MemoryCopyError> where T: ProcessMemory {
            let pos = get_pos(iseq_struct, cfp)?;
            let t_size = iseq_struct.insns_info_size as usize;
            if t_size == 0 {
                Ok(0) //TODO: really?
            } else if t_size == 1 {
                let table: [iseq_insn_info_entry; 1] = source.copy_struct(iseq_struct.insns_info as usize)
                    .context(iseq_struct.insns_info as usize)?;
                Ok(table[0].line_no as u32)
            } else {
                let table: Vec<iseq_insn_info_entry> = source.copy_vec(iseq_struct.insns_info as usize, t_size as usize)
                    .context(iseq_struct.insns_info as usize)?;
                for i in 0..t_size {
                    if pos == table[i].position as usize {
                        return Ok(table[i].line_no as u32)
                    } else if table[i].position as usize > pos {
                        return Ok(table[i-1].line_no as u32)
                    }
                }
                Ok(table[t_size-1].line_no as u32)
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
        ) -> Result<u32, MemoryCopyError> where T: ProcessMemory {
            //let pos = get_pos(iseq_struct, cfp)?;
            let t_size = iseq_struct.insns_info.size as usize;
            if t_size == 0 {
                Ok(0) //TODO: really?
            } else if t_size == 1 {
                let table: [iseq_insn_info_entry; 1] = source.copy_struct(iseq_struct.insns_info.body as usize)
                    .context(iseq_struct.insns_info.body as usize)?;
                Ok(table[0].line_no as u32)
            } else {
                let table: Vec<iseq_insn_info_entry> = source.copy_vec(iseq_struct.insns_info.body as usize, t_size as usize)
                    .context(iseq_struct.insns_info.body as usize)?;
                // TODO: fix this. I'm not sure why it doesn't extract the table properly.
                /*let positions: Vec<usize> = source.copy_vec(iseq_struct.insns_info.positions as usize, t_size as usize)?;
                for i in 0..t_size {
                    if pos == positions[i] as usize {
                        return Ok(table[i].line_no as u32)
                    } else if positions[i] as usize > pos {
                        return Ok(table[i-1].line_no as u32)
                    }
                }*/
                Ok(table[t_size-1].line_no as u32)
            }
        }
    )
);

macro_rules! get_stack_frame_2_0_0(
    () => (
        fn get_stack_frame<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
        ) -> Result<StackFrame, MemoryCopyError> where T: ProcessMemory {
            Ok(StackFrame{
                name: get_ruby_string(iseq_struct.location.label as usize, source)?,
                relative_path: get_ruby_string(iseq_struct.location.path as usize, source)?,
                absolute_path: Some(get_ruby_string(iseq_struct.location.absolute_path as usize, source)?),
                lineno: get_lineno(iseq_struct, cfp, source)?,
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
        ) -> Result<StackFrame, MemoryCopyError> where T: ProcessMemory {
            let body: rb_iseq_constant_body = source.copy_struct(iseq_struct.body as usize)
                .context(iseq_struct.body as usize)?;
            Ok(StackFrame{
                name: get_ruby_string(body.location.label as usize, source)?,
                relative_path: get_ruby_string(body.location.path as usize, source)?,
                absolute_path: Some(get_ruby_string(body.location.absolute_path as usize, source)?),
                lineno: get_lineno(&body, cfp, source)?,
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
        ) -> Result<StackFrame, MemoryCopyError> where T: ProcessMemory {
            let body: rb_iseq_constant_body = source.copy_struct(iseq_struct.body as usize)
                .context(iseq_struct.body as usize)?;
            let rstring: RString = source.copy_struct(body.location.label as usize)
                .context(body.location.label as usize)?;

            let (path, absolute_path) = get_ruby_string_array(body.location.pathobj as usize, rstring.basic.klass as usize, source)?;
            Ok(StackFrame{
                name: get_ruby_string(body.location.label as usize, source)?,
                relative_path: path,
                absolute_path: Some(absolute_path),
                lineno: get_lineno(&body, cfp, source)?,
            })
        }
    )
);

macro_rules! get_cfps(
    () => (
        // Ruby stack grows down, starting at
        //   ruby_current_thread->stack + ruby_current_thread->stack_size - 1 * sizeof(rb_control_frame_t)
        // I don't know what the -1 is about. Also note that the stack_size is *not* in bytes! stack is a
        // VALUE*, and so stack_size is in units of sizeof(VALUE).
        //
        // The base of the call stack is therefore at
        //   stack + stack_size * sizeof(VALUE) - sizeof(rb_control_frame_t)
        // (with everything in bytes).
        fn get_cfps<T>(
            cfp_address: usize,
            stack_base: usize,
            source: &T
        ) -> Result<Vec<rb_control_frame_t>, MemoryCopyError> where T: ProcessMemory {
            if (stack_base as usize) <= cfp_address {
                // this probably means we've hit some kind of race, return an error so we can try
                // again
                return Err(MemoryCopyError::Message(format!("stack base and cfp address out of sync. stack base: {:x}, cfp address: {:x}", stack_base as usize, cfp_address)));
            }

            Ok(
                source
                    .copy_vec(cfp_address, (stack_base as usize - cfp_address) as usize / std::mem::size_of::<rb_control_frame_t>())
                    .context(cfp_address)?
            )
        }
    )
);

macro_rules! get_cfunc_name_unsupported(
    () => (
        fn get_cfunc_name<T: ProcessMemory>(_cfp: &rb_control_frame_t, _global_symbols_address: usize, _source: &T, _pid: Pid) -> Result<String, MemoryCopyError> {
            return Err(format_err!("C function resolution is not supported for this version of Ruby").into());
        }
    )
);

macro_rules! get_cfunc_name(
    () => (
        fn check_method_entry<T: ProcessMemory>(
            raw_imemo: usize,
            source: &T
        ) -> Result<*const rb_method_entry_struct, MemoryCopyError> {
            let imemo: rb_method_entry_struct = source.copy_struct(raw_imemo).context(raw_imemo)?;

            // These type constants are defined in ruby's internal/imemo.h
            #[allow(non_upper_case_globals)]
            match ((imemo.flags >> 12) & 0x07) as u32 {
                imemo_type_imemo_ment => Ok(&imemo as *const rb_method_entry_struct),
                imemo_type_imemo_svar => {
                    let svar: vm_svar = source.copy_struct(raw_imemo).context(raw_imemo)?;
                    check_method_entry(svar.cref_or_me, source)
                },
                _ => Ok(raw_imemo as *const rb_method_entry_struct)
            }
        }

        fn get_cfunc_name<T: ProcessMemory>(
            cfp: &rb_control_frame_t,
            global_symbols_address: usize,
            source: &T,
            _pid: Pid
        ) -> Result<String, MemoryCopyError> {
            // The logic in this function is adapted from the .gdbinit script in
            // github.com/ruby/ruby, in particular the print_id function.

            let mut ep = cfp.ep as *mut usize;
            let frame_flag: usize = unsafe {
                source.copy_struct(ep.offset(0) as usize).context(ep.offset(0) as usize)?
            };

            // if VM_FRAME_TYPE($cfp->flag) != VM_FRAME_MAGIC_CFUNC
            if frame_flag & 0xffff0001 != 0x55550001 {
                return Err(format_err!("Not a C function control frame").into());
            }

            let mut env_specval: usize = unsafe {
                source.copy_struct(ep.offset(-1) as usize).context(ep.offset(-1) as usize)?
            };
            let mut env_me_cref: usize = unsafe {
                source.copy_struct(ep.offset(-2) as usize).context(ep.offset(-1) as usize)?
            };

            // #define VM_ENV_FLAG_LOCAL 0x02
            while env_specval & 0x02 != 0 {
                if !check_method_entry(env_me_cref, source)?.is_null() {
                    break;
                }
                unsafe {
                    ep = ep.offset(0) as *mut usize;
                    env_specval = source.copy_struct(ep.offset(-1) as usize).context(ep.offset(-1) as usize)?;
                    env_me_cref = source.copy_struct(ep.offset(-2) as usize).context(ep.offset(-2) as usize)?;
                }
            }

            let imemo: rb_method_entry_struct = source.copy_struct(env_me_cref).context(env_me_cref)?;
            if imemo.def.is_null() {
                return Err(format_err!("No method definition").into());
            }

            let ttype = ((imemo.flags >> 12) & 0x07) as usize;
            if ttype != imemo_type_imemo_ment as usize {
                return Err(format_err!("Not a method entry").into());
            }

            #[allow(non_camel_case_types)]
            type rb_id_serial_t = u32;

            // Declared in symbol.c prior to ruby 2.7.0, so not accessible by bindgen
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
                ids_len = (flags & (ruby_fl_type_RUBY_FL_USER3|ruby_fl_type_RUBY_FL_USER4) as usize) >> (ruby_fl_type_RUBY_FL_USHIFT+3);
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

            Ok(get_ruby_string(rstring_ptr as usize, source)?)
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
ruby_version_v2_7_x!(ruby_2_7_0);
ruby_version_v2_7_x!(ruby_2_7_1);
ruby_version_v2_7_x!(ruby_2_7_2);
ruby_version_v2_7_x!(ruby_2_7_3);
ruby_version_v3_0_x!(ruby_3_0_0);
ruby_version_v3_0_x!(ruby_3_0_1);

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
                lineno: 2,
            },
            StackFrame {
                name: "bbb".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: 6,
            },
            StackFrame {
                name: "ccc".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: 10,
            },
            StackFrame {
                name: "block in <main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: 14,
            },
            StackFrame::unknown_c_function(),
            StackFrame::unknown_c_function(),
            StackFrame {
                name: "<main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: 13,
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
                lineno: 0,
            },
            StackFrame {
                name: "aaa".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/vagrant/ci/ruby-programs/infinite.rb".to_string()),
                lineno: 3,
            },
            StackFrame {
                name: "bbb".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/vagrant/ci/ruby-programs/infinite.rb".to_string()),
                lineno: 7,
            },
            StackFrame {
                name: "ccc".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/vagrant/ci/ruby-programs/infinite.rb".to_string()),
                lineno: 11,
            },
            StackFrame {
                name: "block in <main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some("/vagrant/ci/ruby-programs/infinite.rb".to_string()),
                lineno: 15,
            },
            StackFrame {
                name: "loop [c function]".to_string(),
                relative_path: "(unknown)".to_string(),
                absolute_path: None,
                lineno: 0,
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
                lineno: 2,
            },
            StackFrame {
                name: "bbb".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: 6,
            },
            StackFrame {
                name: "ccc".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: 10,
            },
            StackFrame {
                name: "block in <main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: 14,
            },
            StackFrame::unknown_c_function(),
            StackFrame {
                name: "<main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: 13,
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
                lineno: 2,
            },
            StackFrame {
                name: "bbb".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: 6,
            },
            StackFrame {
                name: "ccc".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: 10,
            },
            StackFrame {
                name: "block in <main>".to_string(),
                relative_path: "ci/ruby-programs/infinite.rb".to_string(),
                absolute_path: Some(
                    "/home/bork/work/rbspy/ci/ruby-programs/infinite.rb".to_string(),
                ),
                lineno: 14,
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
}
