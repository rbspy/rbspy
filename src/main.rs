extern crate libc;
use libc::*;
use std::env;

// void * copy_address(void* addr, int length, pid_t pid) {
//     int amount_to_copy = 1000;
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

fn main() {
    let args: Vec<_> = env::args().collect();
    let pid: pid_t = args[1].parse().unwrap();
    println!("hello {}!\n", pid);
    let size_to_copy = 1000;

}