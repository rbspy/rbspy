set -eux
echo "#include </tmp/headers/$1/vm_core.h>" > /tmp/wrapper.h
rm -rf /tmp/headers/$1
mkdir -p /tmp/headers/$1
cd ~/clones/ruby
git checkout v$1
cp -R include /tmp/headers/$1
cp *.h /tmp/headers/$1
OUT=src/bindings/ruby_${1}.rs
bindgen /tmp/wrapper.h \
    -o /tmp/bindings.rs \
    --impl-debug true \
    --no-doc-comments \
    --whitelist-type rb_iseq_constant_body \
    --whitelist-type rb_iseq_location_struct \
    --whitelist-type rb_thread_struct \
    --whitelist-type rb_thread_t \
    --whitelist-type rb_iseq_struct \
    --whitelist-type rb_control_frame_struct \
    --whitelist-type rb_thread_struct \
    --whitelist-type RString \
    --whitelist-type VALUE \
    -- \
    -I/tmp/headers/$1/include \
    -I/home/bork/scratch/ruby-header-files/general -I/tmp/headers/$1/ \
    -I/usr/lib/llvm-3.8/lib/clang/3.8.0/include/

#rustfmt --force src/bindings/ruby_${1}.rs

cd ~/work/ruby-stacktrace

echo "#![allow(non_upper_case_globals)]" > $OUT
echo "#![allow(non_camel_case_types)]" >> $OUT
echo "#![allow(non_snake_case)]" >> $OUT
cat /tmp/bindings.rs >> $OUT
