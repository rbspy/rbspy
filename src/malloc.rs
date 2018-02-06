use bcc::perf;
use bcc::core::BPF;
use bcc::table::Table;
use failure::Error;
use libc::pid_t;

use initialize;
use output;
use output::Outputter;

use std::sync::Mutex;
use std::sync::Arc;
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

pub fn trace_new_objects(pid: pid_t) -> Result<(), Error> {
    let getter = initialize::initialize(pid)?;
    let file = File::open("/tmp/out.txt")?;

    let table = connect(pid)?;
    let outputter = output::Flamegraph;
    let file_mutex = Arc::new(Mutex::new(&mut file));
    let mut perf_map = perf::init_perf_map(table, || Box::new(|_| {
        let stack = getter.get_trace().unwrap();
        if let Ok(ref mutex) = file_mutex.try_lock() {
            outputter.record(**mutex, &stack);
        } else {
            println!("try_lock failed");
        }
    }
    ))?;
    loop {
        perf_map.poll(2000);
    }
}
