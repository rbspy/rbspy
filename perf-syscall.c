#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include <sys/mman.h>
#include <string.h>
#include <errno.h>
#include <sys/ioctl.h>
#include <sys/syscall.h>
#include <linux/perf_event.h>
#include <asm/unistd.h>


#define u64 __u64
#define u32 __u32

struct perf_record_sample {
    struct perf_event_header header;
    u64   ip;         /* if PERF_SAMPLE_IP */
    u32   pid, tid;   /* if PERF_SAMPLE_TID */
    u64   time;       /* if PERF_SAMPLE_TIME */
    u64   addr;       /* if PERF_SAMPLE_ADDR */
    u64   id;         /* if PERF_SAMPLE_ID */
    u64   stream_id;  /* if PERF_SAMPLE_STREAM_ID */
    u32   cpu, res;   /* if PERF_SAMPLE_CPU */
    u64   period;     /* if PERF_SAMPLE_PERIOD */
    // struct read_format v;  if PERF_SAMPLE_READ 
    // u64   nr;          if PERF_SAMPLE_CALLCHAIN 
    // u64   ips[nr];    /* if PERF_SAMPLE_CALLCHAIN */
    // u32   size;       /* if PERF_SAMPLE_RAW */
    // char  data[size]; /* if PERF_SAMPLE_RAW */
    // u64   bnr;        /* if PERF_SAMPLE_BRANCH_STACK */
    // struct perf_branch_entry lbr[bnr];
    //                   /* if PERF_SAMPLE_BRANCH_STACK */
    // u64   abi;        /* if PERF_SAMPLE_REGS_USER */
    // u64   regs[weight(mask)];
    //                   /* if PERF_SAMPLE_REGS_USER */
    // u64   size;       /* if PERF_SAMPLE_STACK_USER */
    // char  data[size]; /* if PERF_SAMPLE_STACK_USER */
    // u64   dyn_size;   /* if PERF_SAMPLE_STACK_USER */
    // u64   weight;     /* if PERF_SAMPLE_WEIGHT */
    // u64   data_src;   /* if PERF_SAMPLE_DATA_SRC */
    // u64   transaction;/* if PERF_SAMPLE_TRANSACTION */
    // u64   abi;        /* if PERF_SAMPLE_REGS_INTR */
    // u64   regs[weight(mask)];
    //                   /* if PERF_SAMPLE_REGS_INTR */
};

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

void print_perf_data(void *data_start) {
  struct perf_event_header header;
  struct perf_record_sample sample;
}

void read_mmap_thing(void *mmap_address) {
  // struct perf_event_mmap_page pemp = (* struct perf_event_mmap_page)) mmap_address;
  // sleep(0.5);
  // void * data_start = mmap_address + 4096;
  // return; 
}

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
   pe.sample_type = PERF_SAMPLE_IP | PERF_SAMPLE_TID | PERF_SAMPLE_TIME | PERF_SAMPLE_ADDR | PERF_SAMPLE_ID | PERF_SAMPLE_STREAM_ID | PERF_SAMPLE_CPU; // ??????? what does it meeeeeeeeeeean
   pe.disabled = 1;
   pe.inherit = 1;
   pe.mmap = 1;
   pe.comm = 1;
   pe.freq = 1;
   pe.sample_id_all = 1;
   pe.exclude_guest = 1;
   pe.exclude_kernel = 1;

   fd = perf_event_open(&pe, -1, 0, -1, 0);
   // mmap(NULL, 528384, PROT_READ|PROT_WRITE, MAP_SHARED, 4, 0) = 0x7f846af88000
   if (fd == -1) {
      fprintf(stderr, "Error opening leader %llx\n", pe.config);
      fprintf(stderr, "errno: %d\n", errno);
      exit(EXIT_FAILURE);
   }

   void * mmap_address = mmap(NULL, (129 * 4096), PROT_READ|PROT_WRITE, MAP_SHARED, fd, 0);
   if (mmap_address == -1) {
    printf("mmap failed!!!!!!!!: %d\n", errno);
    exit(1);
   }
   if (fd == -1) {
      fprintf(stderr, "Error opening leader %llx\n", pe.config);
      exit(EXIT_FAILURE);
   }

   ioctl(fd, PERF_EVENT_IOC_RESET, 0);
   ioctl(fd, PERF_EVENT_IOC_ENABLE, 0);

   printf("Measuring instruction count for this printf\n");
   read_mmap_thing(mmap_address);

   ioctl(fd, PERF_EVENT_IOC_DISABLE, 0);
   read(fd, &count, sizeof(long long));

   printf("Used %lld instructions\n", count);

   close(fd);
}

