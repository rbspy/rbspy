set -eux

error() { echo "$@" 1>&2; }

ruby_src_dir=~/clones/ruby
if [ ! -d "$ruby_src_dir" ]; then
   error "In order to use a few private header files, Ruby's source code (https://github.com/ruby/ruby.git) must be cloned as $ruby_src_dir."
   exit 1
fi

ruby_header_dir="$(ruby -rrbconfig -e 'puts RbConfig::CONFIG["rubyarchhdrdir"]')"

echo "#include </tmp/headers/$1/vm_core.h>" > /tmp/wrapper.h
echo "#include </tmp/headers/$1/iseq.h>" >> /tmp/wrapper.h
rm -rf /tmp/headers/$1
mkdir -p /tmp/headers/$1

(cd $ruby_src_dir && git checkout v$1)
cp -R "$ruby_src_dir/include" /tmp/headers/$1
if [ -e "$ruby_src_dir/ccan" ]
then
    cp -R "$ruby_src_dir/ccan" /tmp/headers/$1
fi
cp "$ruby_src_dir"/*.h /tmp/headers/$1

OUT=ruby-structs/src/ruby_${1}.rs
bindgen /tmp/wrapper.h \
    -o /tmp/bindings.rs \
    --impl-debug \
    --no-doc-comments \
    --whitelist-type rb_iseq_constant_body \
    --whitelist-type rb_iseq_location_struct \
    --whitelist-type rb_thread_struct \
    --whitelist-type rb_thread_t \
    --whitelist-type rb_iseq_struct \
    --whitelist-type rb_control_frame_struct \
    --whitelist-type rb_thread_struct \
    --whitelist-type rb_execution_context_struct \
    --whitelist-type iseq_insn_info_entry\
    --whitelist-type RString \
    --whitelist-type RArray \
    --whitelist-type VALUE \
    -- \
    -I/tmp/headers/$1/include \
    -I/home/bork/monorepo/ruby-header-files -I/tmp/headers/$1/ \
    -I/usr/lib/llvm-3.8/lib/clang/3.8.0/include/ \
    "-I$ruby_header_dir"

rustfmt /tmp/bindings.rs

echo "#![allow(non_upper_case_globals)]" > $OUT
echo "#![allow(non_camel_case_types)]" >> $OUT
echo "#![allow(non_snake_case)]" >> $OUT
cat /tmp/bindings.rs >> $OUT

# fix up generated bindings so that they compile/work on windows
perl -pi -e "s/::std::os::raw::c_ulong;/usize;/g" $OUT
perl -pi -e "s/63u8\) as u64/63u8\) as usize/g" $OUT
perl -pi -e "s/let val: u64 =/let val: usize =/g" $OUT
perl -pi -e "s/let num_entries: u64 =/let num_entries: usize =/g" $OUT
