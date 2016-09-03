use leb128;
use rand;
use gimli;
use std::hash::BuildHasherDefault;
use fnv::FnvHasher;
use byteorder::{NativeEndian, ReadBytesExt};
use std::collections::HashMap;
use std::io::Cursor;

type HashMapFnv<K, V> = HashMap<K, V, BuildHasherDefault<FnvHasher>>;


fn get_attr_name<Endian>(die: &gimli::DebuggingInformationEntry<Endian>, debug_str: gimli::DebugStr<Endian>) -> Option<String>
where Endian: gimli::Endianity
{
    let mut attrs = die.attrs();
    while let Some(attr) = attrs.next().expect("Should parse attribute OK") {
        match attr.name() {
            gimli::DW_AT_name => {
                match attr.value() {
                    gimli::AttributeValue::String(s) => return Some(s.to_string_lossy().to_string()),
                    gimli::AttributeValue::DebugStrRef(o) => {
                        match debug_str.get_str(o) {
                            Ok(s) => return Some(s.to_string_lossy().to_string()),
                            Err(_) => continue,
                        }
                    }
                    _ => continue,
                }
                // if die.offset() == 6642 {
                //     println!("---------- it happened ---------------");
                //     println!("{:?}", attr.value());
                // }
                // if let             },
            },
            _ => continue,
        }
    }
    None
}

fn read_pointer_address(vec: &[u8]) -> usize {
    let mut rdr = Cursor::new(vec);
    rdr.read_uint::<NativeEndian>(vec.len()).unwrap() as usize
}

fn get_attr_byte_size<Endian>(die: &gimli::DebuggingInformationEntry<Endian>) -> Option<usize>
where Endian: gimli::Endianity
{
    let mut attrs = die.attrs();
    while let Some(attr) = attrs.next().expect("Should parse attribute OK") {
        match attr.name() {
            gimli::DW_AT_byte_size => {
                if let gimli::AttributeValue::Data(o) = attr.value() {
                    return Some(read_pointer_address(o))
                }
            },
            _ => continue,
        }
    }
    None
}

fn get_attr_type<Endian>(die: &gimli::DebuggingInformationEntry<Endian>) -> Option<usize>
where Endian: gimli::Endianity
{
    let mut attrs = die.attrs();
    while let Some(attr) = attrs.next().expect("Should parse attribute OK") {
        match attr.name() {
            gimli::DW_AT_type => {
                if let gimli::AttributeValue::UnitRef(gimli::UnitOffset(o)) = attr.value() {
                    return Some(o as usize)
                }
            },
            _ => continue,
        }
    }
    None
}


fn get_data_member_location<Endian>(die: &gimli::DebuggingInformationEntry<Endian>) -> Option<usize>
where Endian: gimli::Endianity
{
    let mut attrs = die.attrs();
    while let Some(attr) = attrs.next().expect("Should parse attribute OK") {
       match attr.name() {
          gimli::DW_AT_data_member_location => {
             match attr.value() {

                gimli::AttributeValue::Block(o) => {
                   return Some(leb128::read::unsigned(&mut &o[1..]).expect("couldn't parse leb") as usize);
                }
                gimli::AttributeValue::Data(o) => {
                   return Some(read_pointer_address(&o));
                }
                _ => panic!("unexpected location value type"),
             }
          }
          _ => continue,
       }
    }
    None
}

fn get_entry_list<Endian>(mut entries: gimli::EntriesCursor<Endian>, group_id: u32, debug_str: gimli::DebugStr<Endian>) -> Vec<(isize, Entry)>
    where Endian: gimli::Endianity
{
    let mut vec: Vec<(isize, Entry)> = Vec::new();
    while let Some((delta_depth, die)) = entries.next_dfs().expect("Should parse next dfs") {
        let entry = Entry {
            children: vec!(),
            id: die.offset(),
            type_id: get_attr_type(die),
            byte_size: get_attr_byte_size(die),
            tag: die.tag(),
            name: get_attr_name(die, debug_str),
            offset: get_data_member_location(die),
            group_id: group_id,
        };
        vec.push((delta_depth, entry));
    }
    vec
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub children: Vec<Entry>,
    pub id: usize,
    pub type_id: Option<usize>,
    pub byte_size: Option<usize>,
    pub name: Option<String>,
    pub tag: gimli::DwTag,
    pub group_id: u32,
    pub offset: Option<usize>
}


pub struct DwarfLookup<'a> {
    lookup_table: HashMapFnv<(usize, u32), &'a Entry>,
    name_lookup: HashMapFnv<String, (usize, u32)>,
}

impl<'a> DwarfLookup<'a> {
    pub fn lookup_thing(&self, name: &str) -> Option<&Entry> {
        match self.name_lookup.get(name) {
            None => None,
            Some(pair) => self.lookup_id(*pair),
        }
    }

    pub fn lookup_id(&self, id: (usize, u32)) -> Option<&Entry> {
        self.lookup_table.get(&id).map(|x| *x)
    }

    pub fn lookup_entry(&self, entry: &Entry) -> Option<&Entry>{
        if entry.type_id == None {
            return None;
        } else {
            self.lookup_table.get(&(entry.type_id.unwrap(), entry.group_id)).map(|x| *x)
        }

    }
}


pub fn create_lookup_table(root_entries: &Vec<Entry>) -> DwarfLookup {
    let mut lookup_table = HashMapFnv::default();
    let mut name_lookup = HashMapFnv::default();
    for root_entry in root_entries.iter() {
        index_entry(&mut lookup_table, root_entry);
        index_name(&mut name_lookup, root_entry);
    }
    DwarfLookup {
        lookup_table: lookup_table,
        name_lookup: name_lookup,
    }
}

fn get_siblings (vec: &[(isize, Entry)]) -> Vec<usize> {
    let depth = vec[0].0;
    let mut sibs = vec!();
    let mut cum_depth = 0;
    for (i, &(d, _)) in vec.iter().enumerate() {
        cum_depth += d;
        if cum_depth < depth {
            break;
        }
        if depth == cum_depth {
            sibs.push(i)
        }
    }
    sibs

}

fn get_child (vec: &[(isize, Entry)]) -> Option<&Entry> {
    let ref second = vec[1];
    if second.0 == 1 {
        Some(&second.1)
    } else {
        None
    }
}

// returns vec[0] with children set to that thing's children
fn make_into_tree(vec: &[(isize, Entry)]) -> Entry {
    // println!("it's me {}", vec.len());
    let children = if vec.len() == 1 {
        vec!()
    }
    else {
        match get_child(&vec) {
            None => vec!(),
            Some(_) => {
                let mut ch = vec!();
                let sibs = get_siblings(&vec[1..]);
                for i in sibs {
                    ch.push(make_into_tree(&vec[i + 1..]))
                }
                ch
            }
        }
    };
    let mut cl = vec[0].1.clone();
    cl.children = children;
    cl
}


pub fn get_all_entries<Endian>(debug_info: &[u8],
                           debug_abbrev: &[u8],
                           debug_str: &[u8]) -> Vec<Entry>
    where Endian: gimli::Endianity
{
    let debug_info = gimli::DebugInfo::<Endian>::new(&debug_info);
    let debug_abbrev = gimli::DebugAbbrev::<Endian>::new(debug_abbrev);
    let debug_str = gimli::DebugStr::<Endian>::new(debug_str);

    let mut root_entries = vec![];

    for unit in debug_info.units() {
       let group_id = rand::random::<u32>();
       let unit = unit.expect("Should parse the unit OK");

       let abbrevs = unit.abbreviations(debug_abbrev)
          .expect("Error parsing abbreviations");
       let vec = get_entry_list(unit.entries(&abbrevs), group_id, debug_str);
       let entry = make_into_tree(vec.as_slice());
       // println!("{:#?}", entry);
       root_entries.push(entry);
    }
    root_entries
}

fn index_entry<'a>(lookup_table: &mut HashMapFnv<(usize, u32), &'a Entry>, entry: &'a Entry) {
    lookup_table.insert((entry.id, entry.group_id), entry);
    for child in entry.children.iter() {
        index_entry(lookup_table, child);
    }
}

fn index_name(name_lookup: &mut HashMapFnv<String, (usize, u32)>, entry: &Entry) {
    let name2 = entry.name.clone();
    if let Some(name) = name2 {
       if !name_lookup.contains_key(&name) {
           name_lookup.insert(name, (entry.id , entry.group_id));
       }
   }
    for child in entry.children.iter() {
        index_name(name_lookup, child);
    }
}


pub fn get_dwarf_entries(pid: usize) -> Vec<Entry> {
    let file_path = format!("/proc/{}/exe", pid);
    let file = obj::open(&file_path);

    let debug_info = obj::get_section(&file, ".debug_info")
        .expect("Does not have .debug_info section");
    let debug_abbrev = obj::get_section(&file, ".debug_abbrev")
        .expect("Does not have .debug_abbrev section");
    let debug_str = obj::get_section(&file, ".debug_str")
        .expect("Does not have .debug_str section");

    if obj::is_little_endian(&file) {
        get_all_entries::<gimli::LittleEndian>(debug_info, debug_abbrev, debug_str)
    } else {
        get_all_entries::<gimli::BigEndian>(debug_info, debug_abbrev, debug_str)
    }
}

mod obj {
    extern crate elf;
    use std::path::Path;

    /// The parsed object file type.
    pub type File = elf::File;

    /// Open and parse the object file at the given path.
    pub fn open<P>(path: P) -> File
        where P: AsRef<Path>
    {
        let path = path.as_ref();
        elf::File::open_path(path).expect("Could not open file")
    }

    /// Get the contents of the section named `section_name`, if such
    /// a section exists.
    pub fn get_section<'a>(file: &'a File, section_name: &str) -> Option<&'a [u8]> {
        file.sections
            .iter()
            .find(|s| s.shdr.name == section_name)
            .map(|s| &s.data[..])
    }

    /// Return true if the file is little endian, false if it is big endian.
    pub fn is_little_endian(file: &File) -> bool {
        match file.ehdr.data {
            elf::types::ELFDATA2LSB => true,
            elf::types::ELFDATA2MSB => false,
            otherwise => panic!("Unknown endianity: {}", otherwise),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Entry, create_lookup_table};
    use gimli;
    use test_utils::data::{DEBUG_INFO, DEBUG_ABBREV, DEBUG_STR};

    fn get_all_entries() -> Vec<Entry> {
        super::get_all_entries::<gimli::LittleEndian>(DEBUG_INFO, DEBUG_ABBREV, DEBUG_STR)
    }

    #[test]
    fn test_get_all_entries() {
        let entries = get_all_entries();
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_dwarf_lookup() {
        let entries = get_all_entries();

        let dwarf_lookup = create_lookup_table(&entries);

        let rb_thread_struct = dwarf_lookup.lookup_thing("rb_thread_struct");
        assert!(rb_thread_struct.is_some());

        assert!(rb_thread_struct.unwrap().children.iter().any(|e| e.name == Some("stack_size".to_string())));
    }
}
