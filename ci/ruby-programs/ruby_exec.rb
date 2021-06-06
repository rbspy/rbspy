# Wait until a newline is input on STDIN, then exec a program that runs forever

target_script = File.join File.dirname($0), "infinite_on_cpu.rb"
STDIN.readline
exec ARGV[0], target_script
