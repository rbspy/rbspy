use std;
use std::io;
use std::mem;
use failure::Error;
use libc::{c_int, pid_t};
use mach::kern_return::KERN_SUCCESS;
use mach::port::{mach_port_name_t, mach_port_t, MACH_PORT_NULL};
use mach::vm_types::{mach_vm_address_t, mach_vm_size_t};
use mach::message::mach_msg_type_number_t;
use mach::vm_region::{vm_region_basic_info_data_t, vm_region_info_t,
                      vm_region_basic_info_data_64_t, VM_REGION_BASIC_INFO};
use mach::types::vm_task_entry_t;
use libproc::libproc::proc_pid::regionfilename;
use mach;

#[derive(Debug, Clone)]
pub struct MacMapRange {
    pub size: mach_vm_size_t,
    pub info: vm_region_basic_info_data_t,
    pub start: mach_vm_address_t,
    pub count: mach_msg_type_number_t,
    pub filename: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub value: Option<usize>,
    pub typ: String,
    pub name: String,
}

fn parse_nm_output(output: &str) -> Vec<Symbol> {
    let mut vec = vec!();
    for line in output.split("\n") {
        let mut split: Vec<&str> = line.split_whitespace().collect();
        let sym = if split.len() == 2 {
            Symbol{
                value: None, typ: split[0].to_string(), name: split[1].to_string()
            }
        } else if split.len() == 3 {
            let value = usize::from_str_radix(split[0], 16).unwrap();
            Symbol{
                value: Some(value), typ: split[1].to_string(), name: split[2].to_string()
            }
        } else {
            continue;
        };
        vec.push(sym);
    }
    vec
}

pub fn get_symbols(filename: &str) -> Result<Vec<Symbol>, Error> {
    let output = std::process::Command::new("nm").arg(filename).output()?;
    Ok(parse_nm_output(&String::from_utf8_lossy(&output.stdout)))
}

impl MacMapRange {
    pub fn end(&self) -> mach_vm_address_t {
        self.start + self.size as mach_vm_address_t
    }

    pub fn is_read(&self) -> bool {
        self.info.protection & mach::vm_prot::VM_PROT_READ != 0
    }
    pub fn is_write(&self) -> bool {
        self.info.protection & mach::vm_prot::VM_PROT_WRITE != 0
    }
    pub fn is_exec(&self) -> bool {
        self.info.protection & mach::vm_prot::VM_PROT_EXECUTE != 0
    }
}

pub fn get_process_maps(pid: pid_t, task: mach_port_name_t) -> Vec<MacMapRange> {
    let init_region = mach_vm_region(pid, task, 1).unwrap();
    let mut vec = vec![];
    let mut region = init_region.clone();
    vec.push(init_region);
    loop {
        match mach_vm_region(pid, task, region.end()) {
            Some(r) => {
                vec.push(r.clone());
                region = r;
            }
            _ => return vec,
        }
    }
}

fn mach_vm_region(
    pid: pid_t,
    target_task: mach_port_name_t,
    mut address: mach_vm_address_t,
) -> Option<MacMapRange> {
    let mut count = mem::size_of::<vm_region_basic_info_data_64_t>() as mach_msg_type_number_t;
    let mut object_name: mach_port_t = 0;
    let mut size = unsafe { mem::zeroed::<mach_vm_size_t>() };
    let mut info = unsafe { mem::zeroed::<vm_region_basic_info_data_t>() };
    let result = unsafe {
        mach::vm::mach_vm_region(
            target_task as vm_task_entry_t,
            &mut address,
            &mut size,
            VM_REGION_BASIC_INFO,
            &mut info as *mut vm_region_basic_info_data_t as vm_region_info_t,
            &mut count,
            &mut object_name,
        )
    };
    if result != KERN_SUCCESS {
        return None;
    }
    let filename = match regionfilename(pid, address) {
        Ok(x) => Some(x),
        _ => None,
    };
    Some(MacMapRange {
        size: size,
        info: info,
        start: address,
        count: count,
        filename: filename,
    })
}

pub fn task_for_pid(pid: pid_t) -> io::Result<mach_port_name_t> {
    let mut task: mach_port_name_t = MACH_PORT_NULL;
    // sleep for 5ms to make sure we don't get into a race between `task_for_pid` and execing a new
    // process. Races here can freeze the OS because of a Mac kernel bug.
    std::thread::sleep(std::time::Duration::from_millis(5));
    unsafe {
        let result =
            mach::traps::task_for_pid(mach::traps::mach_task_self(), pid as c_int, &mut task);
        if result != KERN_SUCCESS {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(task)
}
