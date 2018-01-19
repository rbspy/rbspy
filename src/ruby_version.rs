/*
 * Ruby version specific code for reading a stack trace out of a Ruby process's memory.
 *
 * Implemented through a series of macros, because there are subtle differences in struct layout
 * between similar Ruby versions (like 2.2.1 vs 2.2.2) that mean it's easiest to compile a
 * different function for every Ruby minor version.
 *
 * Defines a bunch of submodules, one per Ruby version (`ruby_1_9_3`, `ruby_2_2_0`, etc.)
 */

macro_rules! ruby_version_v_1_9_x(
    ($ruby_version:ident) => (
        pub mod $ruby_version {
            use std;
            use copy::*;
            use bindings::$ruby_version::*;
            use copy::MemoryCopyError;
            use read_process_memory::CopyAddress;

            get_stack_trace!(rb_thread_struct);
            get_ruby_string!();
            get_cfps!();
            get_lineno_1_9_0!();
            get_stack_frame_1_9_0!();
            is_stack_base_1_9_0!();
        }
        ));

macro_rules! ruby_version_v_2_0_to_2_2(
    ($ruby_version:ident) => (
       pub mod $ruby_version {
            use std;
            use copy::*;
            use bindings::$ruby_version::*;
            use copy::MemoryCopyError;
            use read_process_memory::CopyAddress;


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
            get_ruby_string!();
            get_cfps!();
            get_lineno_2_0_0!();
            get_stack_frame_2_0_0!();
            is_stack_base_1_9_0!();
        }
));

macro_rules! ruby_version_v_2_3_to_2_4(
    ($ruby_version:ident) => (
       pub mod $ruby_version {
            use std;
            use copy::*;
            use bindings::$ruby_version::*;
            use copy::MemoryCopyError;
            use read_process_memory::CopyAddress;

            get_stack_trace!(rb_thread_struct);
            get_ruby_string!();
            get_cfps!();
            get_lineno_2_3_0!();
            get_stack_frame_2_3_0!();
            is_stack_base_1_9_0!();
        }
        ));

macro_rules! ruby_version_v2_5_x(
    ($ruby_version:ident) => (
       pub mod $ruby_version {
            use std;
            use copy::*;
            use bindings::$ruby_version::*;
            use copy::MemoryCopyError;
            use read_process_memory::CopyAddress;

            get_stack_trace!(rb_execution_context_struct);
            get_ruby_string!();
            get_cfps!();
            get_lineno_2_5_0!();
            get_stack_frame_2_5_0!();
            is_stack_base_2_5_0!();
            get_ruby_string_array_2_5_0!();
        }
        ));

macro_rules! get_stack_trace(
    ($thread_type:ident) => (

        use initialize::StackFrame;

        pub fn get_stack_trace<T>(
            ruby_current_thread_address_location: usize,
            source: &T,
            ) -> Result<Vec<StackFrame>, MemoryCopyError> where T: CopyAddress{
            debug!(
                "current address location: {:x}",
                ruby_current_thread_address_location
                );
            let current_thread_addr: usize =
                copy_struct(ruby_current_thread_address_location, source)?;
            debug!("{:x}", current_thread_addr);
            let thread: $thread_type = copy_struct(current_thread_addr, source)?;
            debug!("thread: {:?}", thread);
            let mut trace = Vec::new();
            let cfps = get_cfps(thread.cfp as usize, stack_base(&thread) as usize, source)?;
            for cfp in cfps.iter() {
                if cfp.iseq as usize == 0  || cfp.pc as usize == 0 {
                    debug!("huh."); // TODO: fixmeup
                    continue;
                }
                let iseq_struct: rb_iseq_struct = copy_struct(cfp.iseq as usize, source)?;
                debug!("iseq_struct: {:?}", iseq_struct);
                let label_path  = get_stack_frame(&iseq_struct, &cfp, source);
                match label_path {
                    Ok(call)  => trace.push(call),
                    Err(x) => {
                        // this is a heuristic: the intent of this is that it skips function calls into C extensions
                        if trace.len() > 0 {
                            debug!("guess that one didn't work; skipping");
                        } else {
                            return Err(x);
                        }
                    }
                }
            }
            Ok(trace)
        }

use proc_maps::{maps_contain_addr, MapRange};

pub fn is_maybe_thread<T>(x: usize, source: &T, heap_map: &MapRange, all_maps: &Vec<MapRange>) -> bool where T: CopyAddress{
    if !heap_map.contains_addr(x) {
        return false;
    }

    let thread: $thread_type = match copy_struct(x, source) {
        Ok(x) => x,
        _ => { return false; },
    };

    if !is_reasonable_thing(&thread, all_maps) {
        return false;
    }

    let stack_base = stack_base(&thread);
    let diff = stack_base - thread.cfp as i64;
    debug!("diff: {}", diff);
    if diff < 0 || diff > 3000000 {
        return false;
    }

    return true;
}
));

macro_rules! is_stack_base_1_9_0(
    () => (
        fn is_reasonable_thing(thread: &rb_thread_struct,  all_maps: &Vec<MapRange>) -> bool {
            maps_contain_addr(thread.vm as usize, all_maps) &&
                maps_contain_addr(thread.cfp as usize, all_maps) &&
                maps_contain_addr(thread.stack as usize, all_maps) &&
                maps_contain_addr(thread.self_ as usize, all_maps) &&
                thread.stack_size < 3000000 && thread.state >= 0
        }

        fn stack_base(thread: &rb_thread_struct) -> i64 {
            thread.stack as i64 + thread.stack_size as i64 * std::mem::size_of::<VALUE>() as i64 - 1 * std::mem::size_of::<rb_control_frame_t>() as i64
        }
        ));

macro_rules! is_stack_base_2_5_0(
    () => (
        fn is_reasonable_thing(thread: &rb_execution_context_struct, all_maps: &Vec<MapRange>) -> bool {
            maps_contain_addr(thread.tag as usize, all_maps) &&
                maps_contain_addr(thread.cfp as usize, all_maps) &&
                maps_contain_addr(thread.vm_stack as usize, all_maps) &&
                thread.vm_stack_size < 3000000
        }

        fn stack_base(thread: &rb_execution_context_struct) -> i64 {
            thread.vm_stack as i64 + thread.vm_stack_size as i64 * std::mem::size_of::<VALUE>() as i64 - 1 * std::mem::size_of::<rb_control_frame_t>() as i64
        }
        ));

macro_rules! get_ruby_string_array_2_5_0(
    () => (
        fn get_ruby_string_array<T>(addr: usize, string_class: usize, source: &T) -> Result<String, MemoryCopyError> where T: CopyAddress{
            // todo: we're doing an extra copy here for no reason
            let rstring: RString = copy_struct(addr, source)?;
            if rstring.basic.klass as usize == string_class {
                return get_ruby_string(addr, source);
            }
            // otherwise it's an RArray
            let rarray: RArray = copy_struct(addr, source)?;
            debug!("blah: {}, array: {:?}", addr, unsafe { rarray.as_.ary });
            // TODO: this assumes that the array contents are stored inline and not on the heap
            // I think this will always be true but we should check instead
            // the reason I am not checking is that I don't know how to check yet
            let addr: usize = unsafe { rarray.as_.ary[1] as usize }; // 1 means get the absolute path, not the relative path
            get_ruby_string(addr, source)
        }
        ));

macro_rules! get_ruby_string(
    () => (
        use std::ffi::CStr;

        fn get_ruby_string<T>(addr: usize, source: &T) -> Result<String, MemoryCopyError> where T: CopyAddress{
            let vec = {
                let rstring: RString = copy_struct(addr, source)?;
                let basic = rstring.basic;
                let is_array = basic.flags & 1 << 13 == 0;
                if is_array {
                    unsafe { CStr::from_ptr(rstring.as_.ary.as_ref().as_ptr() as *const i8) }
                    .to_bytes()
                        .to_vec()
                } else {
                    unsafe {
                        let addr = rstring.as_.heap.ptr as usize;
                        let len = rstring.as_.heap.len as usize;
                        copy_address_raw(addr as usize, len, source)?
                    }
                }
            };
            Ok(String::from_utf8(vec).map_err(|x| {MemoryCopyError::InvalidStringError(x)})?)
        }
));

macro_rules! get_stack_frame_1_9_0(
    () => (
        fn get_stack_frame<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
            ) -> Result<StackFrame, MemoryCopyError> where T: CopyAddress{
            Ok(StackFrame{
                name: get_ruby_string(iseq_struct.name as usize, source)?,
                path: get_ruby_string(iseq_struct.filename as usize, source)?,
                lineno: Some(get_lineno(iseq_struct, cfp, source)?),
            })
        }
        ));

macro_rules! get_lineno_1_9_0(
    () => (
        fn get_lineno<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
            ) -> Result<u32, MemoryCopyError> where T: CopyAddress{
            let mut pos = cfp.pc as usize - iseq_struct.iseq_encoded as usize;
            if pos != 0 {
                pos -= 1;
            }
            let t_size = iseq_struct.insn_info_size as usize;
            if t_size == 0 {
                Ok(0) //TODO: really?
            } else if t_size == 1 {
                let table: [iseq_insn_info_entry; 1] = copy_struct(iseq_struct.insn_info_table as usize, source)?;
                Ok(table[0].line_no as u32)
            } else {
                let table: Vec<iseq_insn_info_entry> = copy_vec(iseq_struct.insn_info_table as usize, t_size as usize, source)?;
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
));


macro_rules! get_lineno_2_0_0(
    () => (
        fn get_lineno<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
            ) -> Result<u32, MemoryCopyError> where T: CopyAddress{
            let mut pos = cfp.pc as usize - iseq_struct.iseq_encoded as usize;
            if pos != 0 {
                pos -= 1;
            }
            let t_size = iseq_struct.line_info_size as usize;
            if t_size == 0 {
                Ok(0) //TODO: really?
            } else if t_size == 1 {
                let table: [iseq_line_info_entry; 1] = copy_struct(iseq_struct.line_info_table as usize, source)?;
                Ok(table[0].line_no)
            } else {
                let table: Vec<iseq_line_info_entry> = copy_vec(iseq_struct.line_info_table as usize, t_size as usize, source)?;
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
));

macro_rules! get_lineno_2_3_0(
    () => (
        fn get_lineno<T>(
            iseq_struct: &rb_iseq_constant_body,
            cfp: &rb_control_frame_t,
            source: &T,
            ) -> Result<u32, MemoryCopyError> where T: CopyAddress{
            if iseq_struct.iseq_encoded as usize > cfp.pc as usize {
                return Err(MemoryCopyError::Other);
            }
            let mut pos = cfp.pc as usize - iseq_struct.iseq_encoded as usize; // TODO: investigate panic here
            if pos != 0 {
                pos -= 1;
            }
            let t_size = iseq_struct.line_info_size as usize;
            if t_size == 0 {
                Ok(0) //TODO: really?
            } else if t_size == 1 {
                let table: [iseq_line_info_entry; 1] = copy_struct(iseq_struct.line_info_table as usize, source)?;
                Ok(table[0].line_no)
            } else {
                let table: Vec<iseq_line_info_entry> = copy_vec(iseq_struct.line_info_table as usize, t_size as usize, source)?;
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
));

macro_rules! get_lineno_2_5_0(
    () => (
        fn get_lineno<T>(
            iseq_struct: &rb_iseq_constant_body,
            cfp: &rb_control_frame_t,
            source: &T,
            ) -> Result<u32, MemoryCopyError> where T: CopyAddress{
            let mut pos = cfp.pc as usize - iseq_struct.iseq_encoded as usize;
            if pos != 0 {
                pos -= 1;
            }
            let t_size = iseq_struct.insns_info_size as usize;
            if t_size == 0 {
                Ok(0) //TODO: really?
            } else if t_size == 1 {
                let table: [iseq_insn_info_entry; 1] = copy_struct(iseq_struct.insns_info as usize, source)?;
                Ok(table[0].line_no as u32)
            } else {
                let table: Vec<iseq_insn_info_entry> = copy_vec(iseq_struct.insns_info as usize, t_size as usize, source)?;
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
));

macro_rules! get_stack_frame_2_0_0(
    () => (
        fn get_stack_frame<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
            ) -> Result<StackFrame, MemoryCopyError> where T: CopyAddress{
            Ok(StackFrame{
                name: get_ruby_string(iseq_struct.location.label as usize, source)?,
                path: get_ruby_string(iseq_struct.location.path as usize, source)?,
                lineno: Some(get_lineno(iseq_struct, cfp, source)?),
            })
        }
        ));

macro_rules! get_stack_frame_2_3_0(
    () => (
        fn get_stack_frame<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
            ) -> Result<StackFrame, MemoryCopyError> where T: CopyAddress{
            let body: rb_iseq_constant_body = copy_struct(iseq_struct.body as usize, source)?;
            Ok(StackFrame{
                name: get_ruby_string(body.location.label as usize, source)?,
                path: get_ruby_string(body.location.path as usize, source)?,
                lineno: Some(get_lineno(&body, cfp, source)?),
            })
        }
        ));

macro_rules! get_stack_frame_2_5_0(
    () => (
        fn get_stack_frame<T>(
            iseq_struct: &rb_iseq_struct,
            cfp: &rb_control_frame_t,
            source: &T,
            ) -> Result<StackFrame, MemoryCopyError> where T: CopyAddress{
            let body: rb_iseq_constant_body = copy_struct(iseq_struct.body as usize, source)?;
            let rstring: RString = copy_struct(body.location.label as usize, source)?;
            Ok(StackFrame{
                name: get_ruby_string(body.location.label as usize, source)?,
                path:  get_ruby_string_array(body.location.pathobj as usize, rstring.basic.klass as usize, source)?,
                lineno: Some(get_lineno(&body, cfp, source)?),
            })
        }
        ));

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
        fn get_cfps<T>(cfp_address: usize, stack_base: usize, source: &T) -> Result<Vec<rb_control_frame_t>, MemoryCopyError> where T: CopyAddress{
            if (stack_base as usize) <= cfp_address {
                // this probably means we've hit some kind of race, return an error so we can try
                // again
                return Err(MemoryCopyError::Other);
            }
            Ok(copy_vec(cfp_address, (stack_base as usize - cfp_address) as usize / std::mem::size_of::<rb_control_frame_t>(), source)?)
        }
        ));

ruby_version_v_1_9_x!(ruby_1_9_1_0);
ruby_version_v_1_9_x!(ruby_1_9_2_0);
ruby_version_v_1_9_x!(ruby_1_9_3_0);
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
ruby_version_v_2_3_to_2_4!(ruby_2_3_0);
ruby_version_v_2_3_to_2_4!(ruby_2_3_1);
ruby_version_v_2_3_to_2_4!(ruby_2_3_2);
ruby_version_v_2_3_to_2_4!(ruby_2_3_3);
ruby_version_v_2_3_to_2_4!(ruby_2_3_4);
ruby_version_v_2_3_to_2_4!(ruby_2_3_5);
ruby_version_v_2_3_to_2_4!(ruby_2_3_6);
ruby_version_v_2_3_to_2_4!(ruby_2_4_0);
ruby_version_v_2_3_to_2_4!(ruby_2_4_1);
ruby_version_v_2_3_to_2_4!(ruby_2_4_2);
ruby_version_v_2_3_to_2_4!(ruby_2_4_3);
ruby_version_v2_5_x!(ruby_2_5_0_rc1);
