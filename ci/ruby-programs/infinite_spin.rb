# Stays on CPU, unlike infinite_on_cpu.rb, which sleeps between calls.

def spin
  x = 0
  loop { x += 1 }
end

spin
