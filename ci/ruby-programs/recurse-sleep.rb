def recurse_sleep(seconds)
  return if seconds <= 0.00001
  sleep(seconds/2.0)
  recurse_sleep(seconds/2.0)
end

recurse_sleep(30)
