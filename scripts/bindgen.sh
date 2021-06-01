#!/bin/bash

set -eu

error() { echo "$@" 1>&2; }

ruby_src_dir=~/clones/ruby
if [ ! -d "$ruby_src_dir" ]; then
   error "In order to use a few private header files, Ruby's source code (https://github.com/ruby/ruby.git) must be cloned as $ruby_src_dir."
   exit 1
fi

ruby_header_dir="$(ruby -rrbconfig -e 'puts RbConfig::CONFIG["rubyarchhdrdir"]')"

echo "#define RUBY_JMP_BUF sigjmp_buf" > /tmp/wrapper.h
echo "#include </tmp/headers/$1/vm_core.h>" >> /tmp/wrapper.h
echo "#include </tmp/headers/$1/iseq.h>" >> /tmp/wrapper.h
rm -rf /tmp/headers/$1
mkdir -p /tmp/headers/$1

(cd $ruby_src_dir && git checkout v$1)
cp -R "$ruby_src_dir/include" /tmp/headers/$1
if [ -e "$ruby_src_dir/internal" ]
then
    cp -R "$ruby_src_dir/internal" /tmp/headers/$1
fi
if [ -e "$ruby_src_dir/ccan" ]
then
    cp -R "$ruby_src_dir/ccan" /tmp/headers/$1
fi
cp "$ruby_src_dir"/*.h /tmp/headers/$1

BINDGEN_EXTRA_CLANG_ARGS=-fdeclspec \
bindgen /tmp/wrapper.h \
    -o /tmp/bindings.rs \
    --impl-debug \
    --no-doc-comments \
    --rustfmt-bindings \
    --whitelist-type rb_iseq_constant_body \
    --whitelist-type rb_iseq_location_struct \
    --whitelist-type rb_thread_struct \
    --whitelist-type rb_thread_t \
    --whitelist-type rb_iseq_struct \
    --whitelist-type rb_control_frame_struct \
    --whitelist-type rb_thread_struct \
    --whitelist-type rb_execution_context_struct \
    --whitelist-type rb_method_entry_struct \
    --whitelist-type imemo_type \
    --whitelist-type iseq_insn_info_entry\
    --whitelist-type RString \
    --whitelist-type RArray \
    --whitelist-type VALUE \
    --whitelist-type ruby_method_ids \
    --whitelist-type ruby_fl_type \
    --whitelist-type ruby_fl_ushift \
    --constified-enum tLAST_OP_ID \
    --whitelist-type ruby_id_types \
    --constified-enum RUBY_ID_SCOPE_SHIFT \
    --whitelist-type rb_id_serial_t \
    --whitelist-var ID_ENTRY_UNIT \
    --whitelist-var RUBY_FL_USER1 \
    --whitelist-type vm_svar \
    -- \
    -I/tmp/headers/$1/include \
    -I/tmp/headers/$1/ \
    "-I$ruby_header_dir"

OUT=ruby-structs/src/ruby_${1}.rs
echo "#![allow(non_upper_case_globals)]" > $OUT
echo "#![allow(non_camel_case_types)]" >> $OUT
echo "#![allow(non_snake_case)]" >> $OUT
cat /tmp/bindings.rs >> $OUT

# fix up generated bindings so that they compile/work on windows
perl -pi -e "s/::std::os::raw::c_ulong;/usize;/g" $OUT
perl -pi -e "s/63u8\) as u64/63u8\) as usize/g" $OUT
perl -pi -e "s/let val: u64 =/let val: usize =/g" $OUT
perl -pi -e "s/let num_entries: u64 =/let num_entries: usize =/g" $OUT
