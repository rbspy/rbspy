use bcc::perf;
use bcc::core::BPF;
use bcc::table::Table;
use failure::Error;
use libc::pid_t;

use initialize;
use output;
use output::Outputter;

use std::fs::File;

fn connect(pid: pid_t) -> Result<Table, Error> {
    let code = "
#include <uapi/linux/ptrace.h>

typedef struct data {
    size_t mem_ptr;
} data_t;

BPF_PERF_OUTPUT(events);

int track_memory_allocation(struct pt_regs *ctx) {
    data_t data = {};
    data.mem_ptr = PT_REGS_PARM1(ctx);
    events.perf_submit(ctx, &data, sizeof(data));
    return 0;
};
    ";
    let mut module = BPF::new(code)?;
    let uprobe = module.load_uprobe("track_memory_allocation")?;
    module.attach_uprobe(&format!("/proc/{}/exe", pid), "newobj_slowpath", uprobe, pid)?;
    Ok(module.table("events"))
}

struct FileOutputter {
    file: File,
    outputter: Box<Outputter>,
    getter: initialize::StackTraceGetter,
}

fn perf_data_callback() -> Box<FnMut(&[u8])> {
    let getter = initialize::initialize(31101).unwrap();
    let file = File::open("/tmp/out.txt").unwrap();
    let outputter = output::Flamegraph;
    let mut fo = FileOutputter{file, outputter: Box::new(outputter), getter};
    Box::new(move |_| {
        match fo.getter.get_trace() {
            Ok(stack) => {
                fo.outputter.record(&mut fo.file, &stack);
            }
            Err(x) => { println!("oh no: {:?}", x); } ,
        }
    })
}

pub fn trace_new_objects(pid: pid_t) -> Result<(), Error> {
    let table = connect(pid)?;
    let mut perf_map = perf::init_perf_map(table, perf_data_callback)?;
    loop {
        perf_map.poll(2000);
    }
}
