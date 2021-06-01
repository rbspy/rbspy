coordination_path = ARGV[0].gsub('\\', '/')

# rbspy's test will indicate that it's gotten a stack trace from this PID by
# creating a file on disk. Sleep until we see our file.
wait_for_ack_func = %Q(
  while !File.exists?(File.join("#{coordination_path}", "rbspy_ack." + Process.pid.to_s))
    sleep(0.25)
  end
)

subprocess_cmd = <<-CMD
  pid1_1 = spawn(ENV, RbConfig.ruby, '-e #{wait_for_ack_func}')
  eval '#{wait_for_ack_func}'
  Process.wait(pid1_1)
CMD

pid1 = spawn(ENV, RbConfig.ruby, "-e #{subprocess_cmd}")
pid2 = spawn(ENV, RbConfig.ruby, "-e #{wait_for_ack_func}")

eval wait_for_ack_func

Process.wait(pid1)
Process.wait(pid2)
