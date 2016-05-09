compile:
	gcc -g -o perf-syscall perf-syscall.c

run: compile
	./perf-syscall
