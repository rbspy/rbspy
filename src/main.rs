extern crate libc;
use libc::*;
use std::env;
mod ruby_vm;

fn copy_address(addr: *mut c_void, length: usize, pid: pid_t) -> Vec<u8> {
    let mut copy = vec![0;length];
    let local_iov = iovec {
        iov_base: copy.as_mut_ptr() as *mut c_void,
        iov_len: length
    };
    let remote_iov = iovec {
        iov_base: addr,
        iov_len: length
    };
    unsafe {
        let ret = process_vm_readv(pid, &local_iov, 1, &remote_iov, 1, 0);
        println!("ret: {}", ret);
    }
    copy
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let pid: pid_t = args[1].parse().unwrap();
    println!("pid is {}!\n", pid);
    let result = copy_address(0x7ffe6b1d17f0 as *mut c_void, 1000, pid);
    println!("result: {:?}", result)
}