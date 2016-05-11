extern crate libc;
use libc::*;
use std::env;
use std::mem;
use std::slice;
mod ruby_vm;
use ruby_vm::{rb_iseq_t, rb_control_frame_t, rb_thread_t, Struct_RString, VALUE};

fn copy_address_raw(addr: *const c_void, length: usize, pid: pid_t) -> Vec<u8> {
    let mut copy = vec![0;length];
    let local_iov = iovec {
        iov_base: copy.as_mut_ptr() as *mut c_void,
        iov_len: length
    };
    let remote_iov = iovec {
        iov_base: addr as *mut c_void,
        iov_len: length
    };
    unsafe {
        let ret = process_vm_readv(pid, &local_iov, 1, &remote_iov, 1, 0);
    }
    copy
}

unsafe fn copy_address<T>(addr: * const T, pid: pid_t) -> T {
    let mut value: T = mem::uninitialized();
    let local_iov = iovec {
        iov_base: &mut value as *mut _ as * mut c_void,
        iov_len: mem::size_of::<T>()
    };
    let remote_iov = iovec {
        iov_base: addr as *mut c_void,
        iov_len: mem::size_of::<T>()
    };
    let ret = process_vm_readv(pid, &local_iov, 1, &remote_iov, 1, 0);
    value
}

fn get_ruby_string(address: VALUE, pid: pid_t) -> Vec<u8> {
    unsafe {
        let result = copy_address_raw(address as *mut c_void, mem::size_of::<ruby_vm::Struct_RString>(), pid);
        let mut rstring = result.as_ptr() as *mut Struct_RString;
        if (*rstring).basic.flags & (1 << 13) != 0 {
            return copy_address_raw((*(*rstring)._as.heap()).ptr as *const c_void, (*(*rstring)._as.heap()).len as usize, pid);
        } else {
            return slice::from_raw_parts((*(*rstring)._as.ary()).as_ptr() as * const u8, 24).to_vec();
        }
    }
}

fn get_iseq(cfp: &rb_control_frame_t, pid: pid_t) -> rb_iseq_t {
    unsafe {
        copy_address(cfp.iseq as *const rb_iseq_t, pid)
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let pid: pid_t = args[1].parse().unwrap();
    println!("pid is {}!\n", pid);
    let thread = unsafe {
        copy_address(0x7fa684d9b5b0 as *const rb_thread_t, pid)
    };
    println!("cfp address: {:?}", thread.cfp);
    let cfps = unsafe {
        let result = copy_address_raw(thread.cfp as *mut c_void, 100 * mem::size_of::<ruby_vm::rb_control_frame_t>(), pid);
         slice::from_raw_parts(result.as_ptr() as *const ruby_vm::rb_control_frame_t, 100)
    };
    for i in 0..15 {
        let iseq = get_iseq(&cfps[i], pid);
        unsafe {
            libc::puts(get_ruby_string((iseq).location.label as VALUE, pid).as_ptr() as * const c_char);
            libc::puts(get_ruby_string((iseq).location.path as VALUE, pid).as_ptr() as * const c_char);
        }
    }
    println!("{:?}", cfps[1].iseq)
}