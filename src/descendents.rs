extern crate procinfo;
extern crate glob;

use libc::pid_t;
use std::collections::HashMap;

pub fn descendents_of(parent_pid: pid_t) -> Vec<pid_t> {
    let mut result = Vec::<pid_t>::new();
    let mut queue = Vec::<pid_t>::new();

    let parents_to_children = map_parents_to_children();
    queue.push(parent_pid);

    loop {
        match queue.pop() {
            None => {
 				return result;
			},
            Some(current_pid) => {
                match parents_to_children.get(&current_pid) {
                    Some(children) => {
						for child in children {
							queue.push(*child);
						}
					},
					None => ()
				}
				result.push(parent_pid);
            }
        }
    }
}

fn map_parents_to_children() -> HashMap<pid_t, Vec<pid_t>> {
    let mut pid_map: HashMap<pid_t, Vec<pid_t>> = HashMap::new();
    let proc_files = glob::glob("/proc/[0-9]*/").expect("Could not read glob pattern");

    for proc_result in proc_files {
        match proc_result {
            Err(_) => (),
            Ok(proc_folder) => {
                let pid_os_str = proc_folder.file_stem().expect("Glob pattern yields unexpected result");
                let pid: pid_t = ::std::ffi::OsStr::to_str(pid_os_str).expect("huh").parse().expect("Glob pattern??");
                let pid_status = ::procinfo::pid::status(pid).expect("Did not load pid status info");

                pid_map.entry(pid_status.ppid).or_insert(vec!()).push(pid);
            }
        }
    }
    return pid_map;
}
