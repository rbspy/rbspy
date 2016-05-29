import sys

function, in_type, out_type =  sys.argv[1:]

primitive_types = ['Dwarf_Bool',
'Dwarf_Off',
'Dwarf_Unsigned',
'Dwarf_Half',
'Dwarf_Small',
'Dwarf_Signed',
'Dwarf_Addr',
'Dwarf_Ptr']

if out_type in primitive_types:
    out_type_real = ": {t} = 0".format(t=out_type)
else:
    out_type_real = "= ptr::null::<Struct_{t}_s>() as {t}".format(t=out_type)

print """
fn my_%s(arg: %s) -> %s {
    let mut ret %s;
    unsafe {
        let res = %s(arg, &mut ret as *mut %s, dwarf_error());
        if (res != DW_DLV_OK) {
            panic!("Error in %s");
        }
    }
    ret
}""" % (function, in_type, out_type, out_type_real, function, out_type, function)

