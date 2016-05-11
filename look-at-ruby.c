#include <sys/uio.h>
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include </home/bork/clones/ruby-2.1.9/vm_core.h>


int blah(void* copy) {
    rb_control_frame_t cft;
    rb_thread_t th;
}

 int blah2(void* copy) {
    rb_control_frame_t cft;
    rb_thread_t th;
}

void * copy_address(void* addr, int length, pid_t pid) {
    int amount_to_copy = 1000;
    void * copy = malloc(length);
    struct iovec local_iov;
    local_iov.iov_base = copy;
    local_iov.iov_len = length;
    unsigned long liovcnt = 1;
    struct iovec remote_iov;
    remote_iov.iov_base = addr;
    remote_iov.iov_len = length;
    unsigned long riovcnt = 1;
    process_vm_readv(pid,
        &local_iov,
        liovcnt,
        &remote_iov,
        riovcnt,
        0);
    return copy;
}


int main(int argc, char** argv) {
    pid_t pid = atoi(argv[1]);
    printf("reading from PID: %d\n", pid);
    rb_thread_t* thread = (rb_thread_t *) copy_address((void*) 0x7f5a046855b0, sizeof(rb_thread_t), pid);
    rb_control_frame_t * stack = (rb_control_frame_t * ) copy_address(thread->stack, thread->stack_size, pid);
    blah(thread);

}

