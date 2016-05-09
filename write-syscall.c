#include <unistd.h>
#include <sys/syscall.h>
#include <errno.h>
#include <stdio.h>

int main() {

int rc;

char * goodbye = "goodbye";
char * hello = "hello";

if (0) {
 fprintf(stderr, "chmod failed, errno = %d\n", errno);
}

rc = syscall(SYS_write, 2, 0, 90);


printf("rc is %d\n", rc);
}
