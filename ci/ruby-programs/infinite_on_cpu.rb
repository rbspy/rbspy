if ARGV.length == 1
    $sleep_time = ARGV[0].to_i
else
    $sleep_time = 0.5
end

def aaa()
    sleep($sleep_time)
end

def bbb()
    aaa()
end

def ccc()
    bbb()
end

loop do
    ccc()
end
