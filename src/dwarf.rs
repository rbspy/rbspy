#![warn(unused_parens)]

extern crate rand;

use std::collections::HashMap;
use std::os::unix::prelude::*;
use std::fs::File;
use std::os::raw::c_char;
use std::os::raw::c_void;
use std::os::raw::c_uint;
use std::slice::from_raw_parts;
use dwarf_bindings::*;
use std::ptr;
use std::ffi::CString;
use std::ffi::CStr;

fn dwarf_error() -> *mut *mut Struct_Dwarf_Error_s {
    let mut x: Dwarf_Error = ptr::null::<Struct_Dwarf_Error_s>() as Dwarf_Error;
    &mut x as *mut *mut Struct_Dwarf_Error_s
}

fn indent(level: u32) {
    let mut i = 0;
    while i < level {
        i = i + 1;
        print!("  ");

    }
}

fn my_dwarf_get_FORM_name(tag: c_uint) -> *const c_char {
    let mut tagname = ptr::null::<c_char>() as *const c_char;
    unsafe {
        let res = dwarf_get_FORM_name(tag as u32, &mut tagname as *mut *const c_char);
        if res != DW_DLV_OK {
            panic!("Error in dwarf_get_FORM_name\n");
        }
    }
    tagname
}
fn my_dwarf_get_AT_name(tag: c_uint) -> *const c_char {
    let mut tagname = ptr::null::<c_char>() as *const c_char;
    unsafe {
        let res = dwarf_get_AT_name(tag as u32, &mut tagname as *mut *const c_char);
        if res != DW_DLV_OK {
            panic!("Error in dwarf_get_AT_name\n");
        }
    }
    tagname
}
fn my_dwarf_get_TAG_name(tag: c_uint) -> *const c_char {
    let mut tagname = ptr::null::<c_char>() as *const c_char;
    unsafe {
        let res = dwarf_get_TAG_name(tag as u32, &mut tagname as *mut *const c_char);
        if res != DW_DLV_OK {
            panic!("Error in dwarf_get_TAG_name\n");
        }
    }
    tagname
}


fn my_dwarf_attrlist(die: Dwarf_Die) -> Vec<Dwarf_Attribute> {
    let mut vec: Vec<Dwarf_Attribute> = Vec::new();
    let mut attrlist = ptr::null::<Dwarf_Attribute>() as *mut Dwarf_Attribute;
    let mut length: Dwarf_Signed = 0;
    unsafe {
        let res = dwarf_attrlist(die, &mut attrlist as *mut *mut Dwarf_Attribute,
            &mut length as *mut Dwarf_Signed, dwarf_error());
        let slice = from_raw_parts(attrlist, length as usize);
        vec.extend_from_slice(slice);
        match res {
            DW_DLV_NO_ENTRY => return vec!(),
            DW_DLV_OK => {},
            _ => panic!("Error in dwarf_attrlist"),
        }
    }
    vec
}

fn my_dwarf_tag(die: Dwarf_Die) -> c_uint {
    let mut tag: Dwarf_Half = 0;
    unsafe {
        let res = dwarf_tag(die, &mut tag as *mut u16, dwarf_error());
        if res != DW_DLV_OK {
            panic!("Error in dwarf_tag");
        }
    }
    tag as c_uint
}

fn my_dwarf_diename(die: Dwarf_Die) -> Option<*mut c_char> {
    let mut name = ptr::null::<c_char>() as *mut c_char;
    unsafe {
        let res = dwarf_diename(die, &mut name as *mut *mut c_char, dwarf_error());
        if res == DW_DLV_ERROR {
            panic!("Error in dwarf_diename");
        }
        if res == DW_DLV_NO_ENTRY {
            return None;
        }
        return Some(name)
    }
    
}

fn my_dwarf_sibling_of(dbg: Dwarf_Debug, cur_die: Dwarf_Die) -> Option<Dwarf_Die> {
    let mut sib_die = ptr::null::<Struct_Dwarf_Die_s>() as Dwarf_Die;
    let error = dwarf_error();
    unsafe {
        let res = dwarf_siblingof(dbg,
            cur_die,
            &mut sib_die as *mut Dwarf_Die,
            error as *mut Dwarf_Error);
        if res == DW_DLV_ERROR {
            panic!("Error in dwarf_siblingof");
        }
        if res == DW_DLV_NO_ENTRY {
            return None;
        }
    }
    Some(sib_die)
}

#[derive(Debug, Clone)]
struct DwarfHeader {
    version_stamp: Dwarf_Half,
    abbrev_offset: Dwarf_Unsigned,
    address_size: Dwarf_Half,
    next_cu_header: Dwarf_Unsigned,
    cu_header_length: Dwarf_Unsigned,
}

fn my_dwarf_next_cu_header(dbg: Dwarf_Debug) -> Option<DwarfHeader> {
    let mut cu_header_length: Dwarf_Unsigned = 0;
    let mut version_stamp: Dwarf_Half = 0;
    let mut abbrev_offset: Dwarf_Unsigned = 0;
    let mut address_size: Dwarf_Half = 0;
    let mut next_cu_header: Dwarf_Unsigned = 0;
    let mut error: Dwarf_Error = ptr::null::<Struct_Dwarf_Error_s>() as Dwarf_Error;

    unsafe {
        let mut res = DW_DLV_ERROR;
        res = dwarf_next_cu_header(dbg,
                                   &mut cu_header_length,
                                   &mut version_stamp as *mut Dwarf_Half,
                                   &mut abbrev_offset as *mut Dwarf_Unsigned,
                                   &mut address_size as *mut Dwarf_Half,
                                   &mut next_cu_header as *mut Dwarf_Unsigned,
                                   &mut error as *mut *mut Struct_Dwarf_Error_s);
        if res == DW_DLV_ERROR {
            panic!("Error in dwarf_next_cu_header\n");
        }
        if res == DW_DLV_NO_ENTRY {
            return None;
        }
    }
    Some(DwarfHeader {
        cu_header_length: cu_header_length,
        version_stamp: version_stamp,
        abbrev_offset: abbrev_offset,
        address_size: address_size,
        next_cu_header: next_cu_header,
    })
}


fn my_dwarf_child(die: Dwarf_Die) -> Option<Dwarf_Die> {
    let mut child = ptr::null::<Struct_Dwarf_Die_s>() as Dwarf_Die;
    let error = dwarf_error();
    unsafe {
        let res = dwarf_child(die, &mut child as *mut Dwarf_Die, error);
        if res == DW_DLV_ERROR {
            panic!("Error in dwarf_child");
        }
        if res == DW_DLV_NO_ENTRY {
            return None;
        }
    }
    Some(child)
}

fn my_dwarf_bytesize(die: Dwarf_Die) -> Dwarf_Unsigned {
    let mut size: Dwarf_Unsigned = 0;
    unsafe {
        let res = dwarf_bytesize(die, &mut size as *mut Dwarf_Unsigned, dwarf_error());
        if res == DW_DLV_ERROR {
            panic!("Error in dwarf_bytesize");
        }
    }
    size
}

fn my_dwarf_isbitfield(die: Dwarf_Die) -> Dwarf_Bool {
    let mut size: Dwarf_Bool = 0;
    unsafe {
        let res = dwarf_isbitfield(die, &mut size as *mut Dwarf_Bool, dwarf_error());
        if res == DW_DLV_ERROR {
            panic!("Error in dwarf_isbitfield");
        }
    }
    size
}

fn my_dwarf_bitsize(die: Dwarf_Die) -> Dwarf_Unsigned {
    let mut size: Dwarf_Unsigned = 0;
    unsafe {
        let res = dwarf_bitsize(die, &mut size as *mut Dwarf_Unsigned, dwarf_error());
        if res == DW_DLV_ERROR {
            panic!("Error in my_dwarf_bitsize");
        }
    }
    size
}
fn my_dwarf_bitoffset(die: Dwarf_Die) -> Dwarf_Unsigned {
    let mut size: Dwarf_Unsigned = 0;
    unsafe {
        let res = dwarf_bitoffset(die, &mut size as *mut Dwarf_Unsigned, dwarf_error());
        if res == DW_DLV_ERROR {
            panic!("Error in dwarf_bitoffset");
        }
    }
    size
}
fn my_dwarf_srclang(die: Dwarf_Die) -> Dwarf_Unsigned {
    let mut size: Dwarf_Unsigned = 0;
    unsafe {
        let res = dwarf_bitsize(die, &mut size as *mut Dwarf_Unsigned, dwarf_error());
        if res == DW_DLV_ERROR {
            panic!("Error in dwarf_srclang");
        }
    }
    size
}

fn my_dwarf_formstring(attr: Dwarf_Attribute) -> Option<*mut c_char> {
    let mut name = ptr::null::<c_char>() as *mut c_char;
    unsafe {
        let res = dwarf_formstring(attr, &mut name as *mut *mut c_char, dwarf_error());
        if res == DW_DLV_ERROR {
            return None;
            panic!("Error in formstring: {}", res);
        }
        if res == DW_DLV_NO_ENTRY {
            return None;
        }
    }
    Some(name)
}


fn my_dwarf_whatform(arg: Dwarf_Attribute) -> Dwarf_Half {
    let mut ret : Dwarf_Half = 0;
    unsafe {
        let res = dwarf_whatform(arg, &mut ret as *mut Dwarf_Half, dwarf_error());
        if res != DW_DLV_OK {
            panic!("Error in dwarf_whatform");
        }
    }
    ret
}

fn my_dwarf_whatform_direct(arg: Dwarf_Attribute) -> Dwarf_Half {
    let mut ret : Dwarf_Half = 0;
    unsafe {
        let res = dwarf_whatform_direct(arg, &mut ret as *mut Dwarf_Half, dwarf_error());
        if res != DW_DLV_OK {
            panic!("Error in dwarf_whatform_direct");
        }
    }
    ret
}

fn my_dwarf_whatattr(arg: Dwarf_Attribute) -> Dwarf_Half {
    let mut ret : Dwarf_Half = 0;
    unsafe {
        let res = dwarf_whatattr(arg, &mut ret as *mut Dwarf_Half, dwarf_error());
        if res != DW_DLV_OK {
            panic!("Error in dwarf_whatattr");
        }
    }
    ret
}

fn my_dwarf_formref(arg: Dwarf_Attribute) -> Dwarf_Off {
    let mut ret : Dwarf_Off = 0;
    unsafe {
        let res = dwarf_formref(arg, &mut ret as *mut Dwarf_Off, dwarf_error());
        if res != DW_DLV_OK {
            panic!("Error in dwarf_formref");
        }
    }
    ret
}

fn my_dwarf_global_formref(arg: Dwarf_Attribute) -> Dwarf_Off {
    let mut ret : Dwarf_Off = 0;
    unsafe {
        let res = dwarf_global_formref(arg, &mut ret as *mut Dwarf_Off, dwarf_error());
        if res != DW_DLV_OK {
            panic!("Error in dwarf_global_formref");
        }
    }
    ret
}

fn my_dwarf_formaddr(arg: Dwarf_Attribute) -> Dwarf_Addr {
    let mut ret : Dwarf_Addr = 0;
    unsafe {
        let res = dwarf_formaddr(arg, &mut ret as *mut Dwarf_Addr, dwarf_error());
        if res != DW_DLV_OK {
            panic!("Error in dwarf_formaddr");
        }
    }
    ret
}

fn my_dwarf_formflag(arg: Dwarf_Attribute) -> Dwarf_Bool {
    let mut ret : Dwarf_Bool = 0;
    unsafe {
        let res = dwarf_formflag(arg, &mut ret as *mut Dwarf_Bool, dwarf_error());
        if res != DW_DLV_OK {
            panic!("Error in dwarf_formflag");
        }
    }
    ret
}

fn my_dwarf_formudata(arg: Dwarf_Attribute) -> Dwarf_Unsigned {
    let mut ret : Dwarf_Unsigned = 0;
    unsafe {
        let res = dwarf_formudata(arg, &mut ret as *mut Dwarf_Unsigned, dwarf_error());
        if res != DW_DLV_OK {
            panic!("Error in dwarf_formudata");
        }
    }
    ret
}

fn my_dwarf_formsdata(arg: Dwarf_Attribute) -> Dwarf_Signed {
    let mut ret : Dwarf_Signed = 0;
    unsafe {
        let res = dwarf_formsdata(arg, &mut ret as *mut Dwarf_Signed, dwarf_error());
        if res != DW_DLV_OK {
            panic!("Error in dwarf_formsdata");
        }
    }
    ret
}

fn my_dwarf_die_CU_offset(arg: Dwarf_Die) -> Dwarf_Off {
    let mut ret : Dwarf_Off = 0;
    unsafe {
        let res = dwarf_die_CU_offset(arg, &mut ret as *mut Dwarf_Off, dwarf_error());
        if res != DW_DLV_OK {
            panic!("Error in dwarf_die_CU_offset");
        }
    }
    ret
}


fn my_dwarf_CU_dieoffset_given_die(arg: Dwarf_Die) -> Dwarf_Off {
    let mut ret : Dwarf_Off = 0;
    unsafe {
        let res = dwarf_CU_dieoffset_given_die(arg, &mut ret as *mut Dwarf_Off, dwarf_error());
        if res != DW_DLV_OK {
            panic!("Error in dwarf_CU_dieoffset_given_die");
        }
    }
    ret
}

fn my_dwarf_siblings(dbg: Dwarf_Debug, node: Dwarf_Die) -> Vec<Dwarf_Die> {
    let mut siblings: Vec<Dwarf_Die> = Vec::new();
    let mut cur_die = node;
    siblings.push(node);
    while true {
        match my_dwarf_sibling_of(dbg, cur_die) {
            Some(v) => { 
                siblings.push(v);
                cur_die = v;
            }
            None => break,
        }

    }
    siblings
}


fn get_node_type(die: Dwarf_Die) -> Option<usize> {
    unsafe {
        let attributes = my_dwarf_attrlist(die);
        for attr in attributes {
            let whatattr = my_dwarf_whatattr(attr) as c_uint;
            let at_name = CStr::from_ptr(my_dwarf_get_AT_name(whatattr));
            if at_name.to_str().unwrap() == "DW_AT_type" {
                return Some(my_dwarf_formref(attr) as usize);
            } 
        }
        None
    }
}

fn get_node_name(die: Dwarf_Die) -> Option<*mut c_char> {
    unsafe {
        let attributes = my_dwarf_attrlist(die);
        for attr in attributes {
            let whatattr = my_dwarf_whatattr(attr) as c_uint;
            let at_name = CStr::from_ptr(my_dwarf_get_AT_name(whatattr));
            if at_name.to_str().unwrap() == "DW_AT_name" {
                return my_dwarf_formstring(attr);
            } 
        }
        None
    }
}

fn index_dwarf_data(dbg: Dwarf_Debug, die: Dwarf_Die, group_id: u32) -> Entry<'static> {
    let mut children: Vec<Entry> = Vec::new();
    match my_dwarf_child(die) {
        Some(child) => {
            for node in my_dwarf_siblings(dbg, child) {
                let x = index_dwarf_data(dbg, node, group_id);
                children.push(x);
            }
        }
        None => {},
    }
    let tag = my_dwarf_tag(die);
    let tagname = unsafe {CStr::from_ptr(my_dwarf_get_TAG_name(tag)).to_str().unwrap()};
    let name = match get_node_name(die) {
        Some(s) => unsafe { Some(CStr::from_ptr(s).to_str().unwrap()) } ,
        None => None
    };
    Entry {
        children: children,
        id: my_dwarf_die_CU_offset(die) as usize,
        type_id: get_node_type(die),
        size: my_dwarf_bytesize(die) as usize,
        tagname: tagname,
        name: name,
        group_id: group_id,
    }
}

#[derive(Debug, Clone)]
pub struct Entry<'a> {
    pub children: Vec<Entry<'a>>,
    pub id: usize,
    pub type_id: Option<usize>,
    pub size: usize,
    pub name: Option<&'a str>,
    pub tagname: &'a str,
    pub group_id: u32,
}

pub struct DwarfLookup<'a> {
    lookup_table: HashMap<(usize, u32), &'a Entry<'a>>, 
    name_lookup: HashMap<&'a str, (usize, u32)>,
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

    // pub fn get_size(&self, name: &str) -> usize {
    //     self.lookup_thing(name).size

    // }
}


pub fn create_lookup_table<'a,'b>(root_entries: &'a Vec<Entry<'b>>) -> DwarfLookup<'a> {
    let mut lookup_table = HashMap::new();
    let mut name_lookup = HashMap::new();
    for root_entry in root_entries.iter() {
        index_entry(&mut lookup_table, root_entry);
        index_name(&mut name_lookup, root_entry);
    }
    DwarfLookup {
        lookup_table: lookup_table,
        name_lookup: name_lookup,
    }
}

fn get_all_entries<'a>(dbg: Dwarf_Debug) -> Vec<Entry<'a>> {
    let mut root_entries = vec![];
    let mut group_id: u32 = 0;
    while true {
        group_id = rand::random::<u32>();
        let no_die: Dwarf_Die = ptr::null::<Struct_Dwarf_Die_s>() as Dwarf_Die;
        let mut cu_die: Dwarf_Die = ptr::null::<Struct_Dwarf_Die_s>() as Dwarf_Die;
        match my_dwarf_next_cu_header(dbg) {
            None => break,
            _ => {},
        }

        let cu_die = match my_dwarf_sibling_of(dbg, no_die) {
            Some(v) => v,
            None => panic!("no entry! in dwarf_siblingof on CU die \n"),
        };
        root_entries.push(index_dwarf_data(dbg, cu_die, group_id));
    }
    root_entries
}

fn index_entry<'a, 'b>(lookup_table: &mut HashMap<(usize, u32), &'b Entry<'a>>, entry: &'b Entry<'a>) {
    lookup_table.insert((entry.id, entry.group_id), entry);
    for child in entry.children.iter() {
        index_entry(lookup_table, child);
    }
}

fn index_name<'a>(name_lookup: &mut HashMap<&'a str, (usize, u32)>, entry: &Entry<'a>) {
    if let Some(name) = entry.name {
         name_lookup.insert(name, (entry.id , entry.group_id));
    }
    for child in entry.children.iter() {
        index_name(name_lookup, child);
    }
}


pub fn get_dwarf_entries<'a>(pid: usize) -> Vec<Entry<'a>> {
    let mut dbg: Dwarf_Debug = ptr::null::<Struct_Dwarf_Debug_s>() as Dwarf_Debug;
    let errhand: Dwarf_Handler = None;
    let error_ptr = dwarf_error();
    let errarg: Dwarf_Ptr = ptr::null::<c_void> as *mut c_void;
    let file = match File::open(format!("/proc/{}/exe", pid)) {
        Err(why) => panic!("couldn't open file sryyyy"),
        Ok(file) => file,
    };
    let fd = file.as_raw_fd() as ::std::os::raw::c_int;
    unsafe {
        let res = dwarf_init(fd,
                             0, // 0 means read
                             errhand,
                             errarg,
                             &mut dbg as *mut *mut Struct_Dwarf_Debug_s,
                             error_ptr);
        if res != DW_DLV_OK {
            panic!("Giving up, cannot do DWARF processing\n");
        }
    };
    get_all_entries(dbg)
}
