use bcc::perf;
use bcc::core::BPF;
use bcc::table::Table;
use failure::Error;
use libc::{pid_t, size_t};
use read_process_memory::*;

use callgrind;
use initialize;
use output;
use output::Outputter;
use copy::*;
use bindings;

use std::fs::File;
use std::path::Path;
use std::ptr;
use std;

#[repr(C)]
struct data_t {
    mem_ptr: size_t,
    cfp: size_t,
    cfps: [u8; 600],
}

fn connect(pid: pid_t, current_thread_address: usize, cfp_offset: usize) -> Result<Table, Error> {
    let code = "
#include <uapi/linux/ptrace.h>

typedef struct data {
    size_t mem_ptr;
    size_t cfp;
    u8 cfps[400];

} data_t;

BPF_PERF_OUTPUT(events);

int track_memory_allocation(struct pt_regs *ctx) {
    data_t data = {};
    size_t thread_addr = ADDRESS;
    data.mem_ptr = PT_REGS_PARM1(ctx);
    bpf_probe_read(&data.cfp, sizeof(size_t), (void*) (thread_addr + CFP_OFFSET));
    bpf_probe_read(&data.cfps, sizeof(data.cfps), (void*) data.cfp);
    events.perf_submit(ctx, &data, sizeof(data));
    return 0;
};
    ";
    let code = code.replace("ADDRESS", &format!("{}", current_thread_address));
    let code = code.replace("CFP_OFFSET", &format!("{}", cfp_offset));
    let mut module = BPF::new(&code)?;
    let uprobe = module.load_uprobe("track_memory_allocation")?;
    module.attach_uprobe(&format!("/proc/{}/exe", pid), "newobj_slowpath", uprobe, pid)?;
    Ok(module.table("events"))
}

struct FileOutputter {
    file: File,
    outputter: Box<Outputter>,
    getter: initialize::StackTraceGetter,
}
use bindings::ruby_2_4_0::rb_control_frame_t;
use ruby_version;

fn perf_data_callback() -> Box<FnMut(&[u8])> {
    // let getter = initialize::initialize(4019).unwrap();
    // let outputter = output::Callgrind(callgrind::Stats::new());
    // let file = File::create("/tmp/out.txt").unwrap();
    // let mut fo = FileOutputter{file, outputter: Box::new(outputter), getter};
    Box::new(move |x| {
        let data = parse_struct(x);
        println!("{:x} {:x}", data.mem_ptr, data.cfp);
        let slice: &[rb_control_frame_t] = unsafe {std::slice::from_raw_parts(x.as_ptr() as *const rb_control_frame_t, 20)};
        let stack = ruby_version::ruby_2_4_0::parse_cfps(slice);
        println!("{:?}", stack);
        // match fo.getter.get_trace() {
        //     Ok(stack) => {
        //         fo.outputter.record(&mut fo.file, &stack);
        //     }
        //     Err(MemoryCopyError::ProcessEnded) => {
        //         let f = File::create("/tmp/blah1.txt").unwrap();
        //         fo.outputter.complete(Path::new("xxx"), f);
        //     } ,
        //     Err(_) => {},
        // }
    })
}

fn parse_struct(x: &[u8]) -> data_t {
    unsafe { ptr::read(x.as_ptr() as *const data_t) }
}

macro_rules! offset_of {
    ($ty:ty, $field:ident) => {
        &(*(0 as *const $ty)).$field as *const _ as usize
    }
}

pub fn trace_new_objects(pid: pid_t) -> Result<(), Error> {
    let getter = initialize::initialize(pid)?;
    let source = pid.try_into_process_handle().unwrap();
    let thread_addr: usize = copy_struct(getter.current_thread_addr_location, &source)?;
    let cfp_offset = unsafe { offset_of!(bindings::ruby_2_4_0::rb_thread_t, cfp)};
    println!("cfp offset {:?}", cfp_offset);
    println!("thread addr {:x}", thread_addr);
    let table = connect(pid, thread_addr, cfp_offset)?;
    let mut perf_map = perf::init_perf_map(table, perf_data_callback)?;
    getter.get_trace();
    loop {
        perf_map.poll(2000);
    }
}
