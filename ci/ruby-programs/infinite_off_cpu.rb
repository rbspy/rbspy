require 'fileutils'

def aaa()
    if ARGV.length == 1
        # Create a file to notify the caller that the ruby process is up and running
        FileUtils.touch(ARGV[0])
    end
    sleep
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
