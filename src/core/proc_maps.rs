use std;
use std::fs::File;
use std::io::Read;
use libc::pid_t;

#[derive(Debug, Clone, PartialEq)]
pub struct MapRange {
    pub range_start: usize,
    pub range_end: usize,
    pub offset: usize,
    pub dev: String,
    pub flags: String,
    pub inode: usize,
    pub pathname: Option<String>,
}

impl MapRange {
    pub fn contains_addr(&self, addr: usize) -> bool {
        addr >= self.range_start && addr <= self.range_end
    }
}

pub fn maps_contain_addr(addr: usize, maps: &Vec<MapRange>) -> bool {
    maps.iter().any({ |map| map.contains_addr(addr) })
}

pub fn get_proc_maps(pid: pid_t) -> Result<Vec<MapRange>, std::io::Error> {
    // Parses /proc/PID/maps into a Vec<MapRange>
    // TODO: factor this out into a crate and make it work on Mac too
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
    let contents = include_str!("../../ci/testdata/map.txt");
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
