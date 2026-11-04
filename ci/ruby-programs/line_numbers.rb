def work
  a = 0
  while a < 5_000_000
    a += 1
  end
  b = 0
  while b < 5_000_000
    b += 1
  end
  a + b
end

loop { work }
