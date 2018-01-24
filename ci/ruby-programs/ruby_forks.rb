SLEEP_TIME = 2

pid1 = fork do
  pid1_1 = fork { sleep SLEEP_TIME }

  sleep SLEEP_TIME
end

pid2 = fork { sleep SLEEP_TIME }

sleep SLEEP_TIME
