extern crate libc;
use libc::*;
use std::env;

// void * copy_address(void* addr, int length, pid_t pid) {
//     void * copy = malloc(length);
//     struct iovec local_iov;
//     local_iov.iov_base = copy;
//     local_iov.iov_len = length;
//     unsigned long liovcnt = 1;
//     struct iovec remote_iov;
//     remote_iov.iov_base = addr;
//     remote_iov.iov_len = length;
//     unsigned long riovcnt = 1;
//     process_vm_readv(pid,
//         &local_iov,
//         liovcnt,
//         &remote_iov,
//         riovcnt,
//         0);
//     return copy;
// }

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
        process_vm_readv(pid, &local_iov, 1, &remote_iov, 1, 0);
    }
    copy
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let pid: pid_t = args[1].parse().unwrap();
    println!("pid is {}!\n", pid);
}