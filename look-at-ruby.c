#include <sys/uio.h>
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include </home/bork/clones/ruby-2.1.9/vm_core.h>

pid_t PID;

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

void breakk() {

}

char* get_ruby_string(VALUE address) {
    struct RString* rstring = ((struct RString *) copy_address((void*)address, sizeof(struct RString), PID));
    breakk();
    if (rstring->basic.flags & (1 << 13)) {
        void* address = (void*) rstring->as.heap.ptr;
        int len = rstring->as.heap.len;
        return (char*) copy_address(address, len, PID);
    } else {
        return rstring->as.ary;
    }
}

rb_iseq_t* get_iseq(rb_control_frame_t * cfp) {
    void* iseq_addr = cfp->iseq;
    return (rb_iseq_t *) copy_address(iseq_addr, sizeof(rb_iseq_t), PID);
}


int main(int argc, char** argv) {
    pid_t pid = atoi(argv[1]);
    PID = pid;
    printf("reading from PID: %d\n", pid);
    rb_thread_t* thread = (rb_thread_t *) copy_address((void*) 0x7f929df445b0, sizeof(rb_thread_t), pid);
    while(1) {
    void *stack_start =  copy_address(thread->cfp, 1000, pid);
    rb_control_frame_t * cfp = (rb_control_frame_t *) stack_start;
    int i;
    printf("Stack trace:\n");
    printf("--------------------------------------");
    for (i = 0; i < 15; i++) {
        rb_iseq_t * iseq = get_iseq(cfp + i);
        rb_iseq_location_t iseq_location = iseq->location;
        char* label = get_ruby_string(iseq_location.label);
        char* path = get_ruby_string(iseq_location.path);
        if (path != NULL && label != NULL) {
            printf("file: %s, method: %s\n", path, label);
        }
    }
    }
    // rb_iseq_location_t* iseq_location = get_iseq_location(iseq);
}

