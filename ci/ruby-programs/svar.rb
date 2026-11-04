def matcher
  i = 0
  while i < 2_000_000
    "abc" =~ /b/
    i += 1
  end
end

loop { matcher }
