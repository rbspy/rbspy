while true
    t1 = Thread.new {
        (0..rand(100)).map { Thread.new { sleep(0.01) } }.each(&:join)
    }
    t2 = Thread.new {
        (0..rand(100)).map { Thread.new { sleep(0.02) } }.each(&:join)
    }
    t3 = Thread.new {
        (0..rand(100)).map { Thread.new { sleep(0.03) } }.each(&:join)
    }

    t1.join()
    t2.join()
    t3.join()
end
