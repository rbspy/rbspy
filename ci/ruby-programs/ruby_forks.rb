SLEEP_TIME = 2

suprocess_cmd = <<-CMD
  pid1_1 = spawn(ENV, RbConfig.ruby, "-esleep(#{SLEEP_TIME})")

  sleep #{SLEEP_TIME}
  Process.wait(pid1_1)
CMD

pid1 = spawn(ENV, RbConfig.ruby, "-e#{suprocess_cmd}")

pid2 = spawn(ENV, RbConfig.ruby, "-esleep(#{SLEEP_TIME})")

sleep SLEEP_TIME

Process.wait(pid1)
Process.wait(pid2)
