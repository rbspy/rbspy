def aaa()
    sleep(0.5)
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
