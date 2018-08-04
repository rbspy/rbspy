use libc;
use std;
use std::fs::File;
use std::io::Read;
use core::proc_maps::IMapRange;

pub type Pid = libc::pid_t;


#[derive(Debug, Clone, PartialEq)]
pub struct MapRange {
    range_start: usize,
    range_end: usize,
    offset: usize,
    dev: String,
    flags: String,
    inode: usize,
    pathname: Option<String>,
}

impl IMapRange for MapRange {
    fn size(&self) -> usize { self.range_end - self.range_start }
    fn start(&self) -> usize { self.range_start }
    fn filename(&self) -> &Option<String> { &self.pathname }
    fn is_exec(&self) -> bool { &self.flags[2..3] == "x" }
    fn is_write(&self) -> bool { &self.flags[1..2] == "w" }
    fn is_read(&self) -> bool { &self.flags[0..1] == "r" }
}

pub fn get_process_maps(pid: Pid) -> std::io::Result<Vec<MapRange>> {
    // Parses /proc/PID/maps into a Vec<MapRange>
    let maps_file = format!("/proc/{}/maps", pid);
    let mut file = File::open(maps_file)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(parse_proc_maps(&contents))
}

fn parse_proc_maps(contents: &str) -> Vec<MapRange> {
    let mut vec: Vec<MapRange> = Vec::new();
    for line in contents.split("\n") {
        let mut split = line.split_whitespace();
        let range = split.next();
        if range == None {
            break;
        }
        let mut range_split = range.unwrap().split("-");
        let range_start = range_split.next().unwrap();
        let range_end = range_split.next().unwrap();
        let flags = split.next().unwrap();
        let offset = split.next().unwrap();
        let dev = split.next().unwrap();
        let inode = split.next().unwrap();

        vec.push(MapRange {
            range_start: usize::from_str_radix(range_start, 16).unwrap(),
            range_end: usize::from_str_radix(range_end, 16).unwrap(),
            offset: usize::from_str_radix(offset, 16).unwrap(),
            dev: dev.to_string(),
            flags: flags.to_string(),
            inode: usize::from_str_radix(inode, 10).unwrap(),
            pathname: split.next().map(|x| x.to_string()),
        });
    }
    vec
}

#[test]
fn test_parse_maps() {
    let contents = include_str!("../../../ci/testdata/map.txt");
    let vec = parse_proc_maps(contents);
    let expected = vec![
        MapRange {
            range_start: 0x00400000,
            range_end: 0x00507000,
            offset: 0,
            dev: "00:14".to_string(),
            flags: "r-xp".to_string(),
            inode: 205736,
            pathname: Some("/usr/bin/fish".to_string()),
        },
        MapRange {
            range_start: 0x00708000,
            range_end: 0x0070a000,
            offset: 0,
            dev: "00:00".to_string(),
            flags: "rw-p".to_string(),
            inode: 0,
            pathname: None,
        },
        MapRange {
            range_start: 0x0178c000,
            range_end: 0x01849000,
            offset: 0,
            dev: "00:00".to_string(),
            flags: "rw-p".to_string(),
            inode: 0,
            pathname: Some("[heap]".to_string()),
        },
    ];
    assert_eq!(vec, expected);
}
