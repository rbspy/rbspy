#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/syscall.h>
#include <linux/perf_event.h>
#include <asm/unistd.h>

static long
perf_event_open(struct perf_event_attr *hw_event, pid_t pid,
               int cpu, int group_fd, unsigned long flags)
{
   int ret;

   ret = syscall(SYS_perf_event_open, hw_event, pid, cpu,
                  group_fd, flags);
   return ret;
}

// syscall number: 298

int
main(int argc, char **argv)
{
   struct perf_event_attr pe;
   long long count;
   int fd;

//   $22 = {type = 0, size = 96, config = 0, {sample_period = 4000, sample_freq =
// 4000}, sample_type = 391, read_format = 0, disabled = 1,    inherit = 1,
// pinned = 0, exclusive = 0, exclude_user = 0, exclude_kernel = 0, exclude_hv =
// 0, exclude_idle = 0, mmap = 1, comm = 1,    freq = 1, inherit_stat = 0,
// enable_on_exec = 0, task = 0, watermark = 0, precise_ip = 0, mmap_data = 0,
// sample_id_all = 1,    exclude_host = 0, exclude_guest = 1, __reserved_1 = 0,
// {wakeup_events = 0, wakeup_watermark = 0}, bp_type = 0, {bp_addr = 0, config1
// = 0},    {bp_len = 0, config2 = 0}, branch_sample_type = 0}
 

   memset(&pe, 0, sizeof(struct perf_event_attr));
   pe.type = PERF_TYPE_HARDWARE;
   pe.size = sizeof(struct perf_event_attr);
   pe.config = PERF_COUNT_HW_CPU_CYCLES;
   pe.sample_period = 4000;
   pe.sample_freq = 4000;
   pe.sample_type = 391; // ??????? what does it meeeeeeeeeeean
   pe.disabled = 1;
   pe.inherit = 1;
   pe.mmap = 1;
   pe.comm = 1;
   pe.freq = 1;
   pe.sample_id_all = 1;
   pe.exclude_guest = 1;
   pe.exclude_kernel = 1;

   fd = perf_event_open(&pe, 0, -1, -1, 0);
   if (fd == -1) {
      fprintf(stderr, "Error opening leader %llx\n", pe.config);
      exit(EXIT_FAILURE);
   }

   ioctl(fd, PERF_EVENT_IOC_RESET, 0);
   ioctl(fd, PERF_EVENT_IOC_ENABLE, 0);

   printf("Measuring instruction count for this printf\n");

   ioctl(fd, PERF_EVENT_IOC_DISABLE, 0);
   read(fd, &count, sizeof(long long));

   printf("Used %lld instructions\n", count);

   close(fd);
}