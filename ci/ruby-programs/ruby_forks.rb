SLEEP_TIME = 2

pid1 = fork do
  pid1_1 = fork { loop { sleep SLEEP_TIME } }

  loop { sleep SLEEP_TIME }
end

pid2 = fork { loop { sleep SLEEP_TIME } }

loop { sleep SLEEP_TIME }
