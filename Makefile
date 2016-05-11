export C_INCLUDE_PATH=/home/bork/clones/ruby-2.1.9

compile:
	gcc -g -o perf-syscall perf-syscall.c
	gcc -I /home/bork/clones/ruby-2.1.9/include -I /home/bork/.rbenv/versions/2.1.6/include/ruby-2.1.0/x86_64-linux/ -g -o look-at-ruby look-at-ruby.c

blah:
	bindgen -I /home/bork/clones/ruby-2.1.9/include -I /usr/lib/llvm-3.4/lib/clang/3.4/include/ -I /home/bork/.rbenv/versions/2.1.6/include/ruby-2.1.0/x86_64-linux/ -match ruby -o ruby_vm.rs