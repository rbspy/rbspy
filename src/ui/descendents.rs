use std::collections::HashMap;
use std::fs::read_dir;
use std::fs::File;
use std::io::Read;

use failure::Error;
#[cfg(unix)]
use libc::pid_t;
#[cfg(windows)]
type pid_t = u32;

pub fn descendents_of(parent_pid: pid_t) -> Result<Vec<pid_t>, Error> {
    let parents_to_children = map_parents_to_children()?;
    get_descendents(parent_pid, parents_to_children)
}

fn get_descendents(
    parent_pid: pid_t,
    parents_to_children: HashMap<pid_t, Vec<pid_t>>,
) -> Result<Vec<pid_t>, Error> {
    let mut result = Vec::<pid_t>::new();
    let mut queue = Vec::<pid_t>::new();
    queue.push(parent_pid);

    loop {
        match queue.pop() {
            None => {
                return Ok(result);
            }
            Some(current_pid) => {
                match parents_to_children.get(&current_pid) {
                    Some(children) => for child in children {
                        queue.push(*child);
                    },
                    None => (),
                }
                result.push(current_pid);
            }
        }
    }
}

fn map_parents_to_children() -> Result<HashMap<pid_t, Vec<pid_t>>, Error> {
    let mut pid_map: HashMap<pid_t, Vec<pid_t>> = HashMap::new();

    for (pid, ppid) in get_proc_children()? {
        pid_map.entry(ppid).or_insert(vec![]).push(pid);
    }
    return Ok(pid_map);
}

#[test]
fn test_get_descendents() {
    let mut map = HashMap::new();
    map.insert(1, vec![2]);
    let desc = get_descendents(1, map).unwrap();
    assert_eq!(desc, vec![1, 2]);
}

#[test]
fn test_get_descendents_depth_2() {
    let mut map = HashMap::new();
    map.insert(1, vec![2, 3]);
    map.insert(2, vec![4]);
    let desc = get_descendents(1, map).unwrap();
    assert_eq!(desc, vec![1, 3, 2, 4]);
}

// parses /proc/<pid>/status format
fn status_file_ppid(status: &str) -> Result<pid_t, Error> {
    let ppid_line = status.split("\n").find(|x| x.starts_with("PPid:"));
    match ppid_line {
        Some(line) => {
            let parts: Vec<&str> = line.split("\t").collect();
            Ok(parts[1].parse::<pid_t>()?)
        }
        None => Err(format_err!("PPid: line not found in {}", status)),
    }
}

#[test]
fn test_status_file_ppid() {
    let status = "Name:	kthreadd\nState:	S (sleeping)\nTgid:	2\nNgid:	0\nPid:	0\nPPid:	1234\n";
    assert_eq!(status_file_ppid(status).unwrap(), 1234)
}

/// Returns pairs of <pid, parent pid>
#[cfg(target_os = "linux")]
fn get_proc_children() -> Result<Vec<(pid_t, pid_t)>, Error> {
    let mut process_pairs = vec![];
    for entry in read_dir("/proc")? {
        let entry = entry?;
        // try parsing the directory name as a PID and see if it works
        let maybe_pid = entry.file_name().to_string_lossy().parse::<pid_t>();
        if let Ok(pid) = maybe_pid {
            let mut contents = String::new();
            if let Ok(mut f) = File::open(entry.path().join("status")) {
                f.read_to_string(&mut contents)?;
                let ppid = status_file_ppid(&contents)?;
                process_pairs.push((pid, ppid));
            }
        }
    }
    Ok(process_pairs)
}

#[cfg(target_os = "macos")]
fn get_proc_children() -> Result<Vec<(pid_t, pid_t)>, Error> {
    use libproc::libproc::proc_pid::{listpids, pidinfo, BSDInfo, ProcType};

    let convert_error = |err| {
        format_err!("Unable to retrieve process parent PID ({})", err)
    };

    let pids = listpids(ProcType::ProcAllPIDS).map_err(convert_error)?;

    let ppids = pids
        .iter()
        .map(|&pid| {
            pidinfo::<BSDInfo>(pid as pid_t, 1).map(|res| res.pbi_ppid as pid_t)
        })
        .collect::<Result<Vec<pid_t>, String>>()
        .map_err(convert_error)?;

    Ok(pids.iter().map(|&pid| pid as pid_t).zip(ppids).collect())
}

#[cfg(windows)]
fn get_proc_children() -> Result<Vec<(pid_t, pid_t)>, Error> {
    // TODO: this isn't currently implemented, but can be done
    // using the tlhelp32 Process32First/Process32Next api from the
    // winapi-rs crate: https://github.com/retep998/winapi-rs/blob/0.3/src/um/tlhelp32.rs#L94
    Err(format_err!("--subprocesses option is not yet supported on windows"))
}
