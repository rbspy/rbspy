def aaa()
    sleep(1000000)
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
