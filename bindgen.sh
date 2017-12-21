echo "#include </tmp/headers/$1/vm_core.h>" > /tmp/wrapper.h
bindgen /tmp/wrapper.h \
    -o src/bindings/ruby_${1}.rs \
    --impl-debug true \
    --no-doc-comments \
    --whitelist-type rb_iseq_constant_body \
    --whitelist-type rb_iseq_location_struct \
    --whitelist-type rb_thread_struct \
    --whitelist-type rb_iseq_struct \
    --whitelist-type rb_control_frame_struct \
    --whitelist-type rb_thread_struct \
    --whitelist-type RString \
    --whitelist-type VALUE \
    -- \
    -I/home/bork/scratch/ruby-header-files/$1/include \
    -I/home/bork/scratch/ruby-header-files/general -I/home/bork/scratch/ruby-header-files/$1/ \
    -I/usr/lib/llvm-3.8/lib/clang/3.8.0/include/

rustfmt --force src/bindings/ruby_${1}.rs
