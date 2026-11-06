# Rack app for ci/test-puma.sh. The checks in that script look for these
# method names and for frames from this file, so keep them in sync.
class PumaWorkload
  def call(_env)
    cpu_work
    io_wait
    [200, { "content-type" => "text/plain" }, ["ok\n"]]
  end

  # A block passed to a regex method sets up special variables ($~, $&) on the
  # frame, which has broken rbspy's frame handling before.
  def cpu_work
    "foo".sub(/f/) do
      50_000.times { $&.upcase }
    end
  end

  def io_wait
    sleep(0.005 + rand * 0.02)
  end
end
