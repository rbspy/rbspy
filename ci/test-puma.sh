#!/usr/bin/env bash
# Integration test: attach rbspy to a puma server under load and sanity check
# the recorded stacks.
#
#   ci/test-puma.sh
#
# Cluster mode (rbspy follows the forked workers with --subprocesses):
#   WORKERS=2 ci/test-puma.sh
#
# On macOS, attaching to a process needs root. Only rbspy runs under sudo; the
# server and clients run as the current user:
#   USE_SUDO=1 ci/test-puma.sh
#
# Keep the raw trace, collapsed stacks, and logs for inspection:
#   KEEP_TMP=1 ci/test-puma.sh
#
# Requires ruby, bundler, and curl. The first run installs puma and rack.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="$ROOT/ci/puma"
RBSPY="${RBSPY:-$ROOT/target/debug/rbspy}"
RECORD_SECONDS="${RECORD_SECONDS:-5}"
RATE="${RATE:-99}"
THREADS="${THREADS:-4}"
WORKERS="${WORKERS:-0}"
CLIENTS="${CLIENTS:-4}"
BOOT_TIMEOUT="${BOOT_TIMEOUT:-30}"

export BUNDLE_GEMFILE="$APP_DIR/Gemfile"
# Install exactly what Gemfile.lock pins (with its checksums); fail rather than
# re-resolve if the lockfile is stale.
export BUNDLE_FROZEN=true

cd "$ROOT"
if [[ ! -x "$RBSPY" ]]; then
  cargo build --quiet
fi
command -v curl >/dev/null || { echo "curl is required" >&2; exit 2; }

bundle check >/dev/null 2>&1 || bundle install --quiet

tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/rbspy-puma.XXXXXX")"
puma_pid=""
client_pids=()
cleanup() {
  touch "$tmpdir/stop"
  for pid in ${client_pids[@]+"${client_pids[@]}"}; do
    kill "$pid" 2>/dev/null || true
  done
  if [[ -n "$puma_pid" ]]; then
    kill -TERM "$puma_pid" 2>/dev/null || true
    wait "$puma_pid" 2>/dev/null || true
  fi
  if [[ "${KEEP_TMP:-}" == "1" ]]; then
    echo "Kept temporary files in $tmpdir"
  else
    rm -rf "$tmpdir"
  fi
}
trap cleanup EXIT

puma_log="$tmpdir/puma.log"
raw="$tmpdir/out.raw.gz"
collapsed="$tmpdir/out.collapsed.txt"
rbspy_log="$tmpdir/rbspy.log"
codes="$tmpdir/status-codes.txt"

# Bind to port 0 and read the chosen port from puma's log so that concurrent
# runs can't collide. `bundle exec` execs puma, so $! is the puma (master) pid.
bundle exec puma \
  --bind tcp://127.0.0.1:0 \
  --threads "$THREADS:$THREADS" \
  --workers "$WORKERS" \
  --environment production \
  "$APP_DIR/config.ru" >"$puma_log" 2>&1 &
puma_pid=$!

port=""
for _ in $(seq $((BOOT_TIMEOUT * 10))); do
  if ! kill -0 "$puma_pid" 2>/dev/null; then
    cat "$puma_log" >&2
    echo "puma exited during boot" >&2
    exit 1
  fi
  port="$(sed -n 's|.*Listening on http://127\.0\.0\.1:\([0-9][0-9]*\).*|\1|p' "$puma_log" | head -n1)"
  if [[ -n "$port" ]] && curl -fsS -o /dev/null "http://127.0.0.1:$port/"; then
    break
  fi
  port=""
  sleep 0.1
done
if [[ -z "$port" ]]; then
  cat "$puma_log" >&2
  echo "puma did not start serving within ${BOOT_TIMEOUT}s" >&2
  exit 1
fi

if [[ "$WORKERS" -gt 0 ]]; then
  # Workers boot after the master starts listening. Wait for all of them so
  # that rbspy sees every worker when it attaches.
  for _ in $(seq $((BOOT_TIMEOUT * 10))); do
    booted="$(grep -c 'Worker [0-9]* (PID: [0-9]*) booted' "$puma_log" || true)"
    [[ "$booted" -ge "$WORKERS" ]] && break
    sleep 0.1
  done
fi

echo "puma pid $puma_pid listening on port $port (threads=$THREADS workers=$WORKERS)"

for _ in $(seq "$CLIENTS"); do
  (
    while [[ ! -e "$tmpdir/stop" ]]; do
      curl -sS -o /dev/null -w '%{http_code}\n' "http://127.0.0.1:$port/" || echo "curl-failed"
    done >>"$codes"
  ) &
  client_pids+=($!)
done

# Let the clients saturate the thread pool before sampling.
sleep 1

cmd=(
  "$RBSPY" record
  --pid "$puma_pid"
  --duration "$RECORD_SECONDS"
  --rate "$RATE"
  --raw-file "$raw"
  --file "$collapsed"
  --format collapsed
  --silent
)
if [[ "$WORKERS" -gt 0 ]]; then
  cmd+=(--subprocesses)
fi
if [[ "${USE_SUDO:-}" == "1" ]]; then
  cmd=(sudo "${cmd[@]}")
fi

set +e
"${cmd[@]}" >"$rbspy_log" 2>&1
status=$?
set -e

touch "$tmpdir/stop"
for pid in ${client_pids[@]+"${client_pids[@]}"}; do
  wait "$pid" 2>/dev/null || true
done
client_pids=()

if [[ "$status" -ne 0 ]]; then
  cat "$rbspy_log" >&2
  echo "rbspy record exited with status $status" >&2
  exit 1
fi

failed=0

if ! kill -0 "$puma_pid" 2>/dev/null; then
  cat "$puma_log" >&2
  echo "FAIL puma died while rbspy was attached" >&2
  failed=1
fi

total_requests="$(wc -l <"$codes" | tr -d ' ')"
bad_requests="$(grep -cv '^200$' "$codes" || true)"
echo "Requests: $total_requests total, $bad_requests non-200"
if [[ "$bad_requests" -gt 0 ]]; then
  sort "$codes" | uniq -c >&2
  echo "FAIL some requests did not return 200" >&2
  failed=1
fi

ruby -rzlib -rjson - "$raw" "$APP_DIR/puma_workload.rb" "$puma_pid" "$THREADS" "$WORKERS" "$RATE" "$RECORD_SECONDS" <<'RUBY' || failed=1
raw_path, app_path, puma_pid, threads, workers, rate, seconds = ARGV
puma_pid = Integer(puma_pid)
threads = Integer(threads)
workers = Integer(workers)
rate = Integer(rate)
seconds = Integer(seconds)

APP_FILE = File.basename(app_path)
app_lines = File.readlines(app_path)

traces = []
Zlib::GzipReader.open(raw_path) do |gz|
  magic = gz.gets
  abort "FAIL raw file has bad magic: #{magic.inspect}" unless magic&.start_with?("rbspy")
  JSON.parse(gz.gets) # header
  gz.each_line { |line| traces << JSON.parse(line) }
end

$failures = 0
def check(ok, msg)
  puts "#{ok ? "ok  " : "FAIL"} #{msg}"
  $failures += 1 unless ok
end

def app_frame?(f)
  File.basename(f["absolute_path"] || f["relative_path"].to_s) == APP_FILE
end

def puma_frame?(f)
  (f["absolute_path"] || f["relative_path"].to_s).include?("/puma/")
end

# Frame 0 is the innermost (leaf) frame.
def index_of(trace, &blk)
  trace["trace"].index(&blk)
end

# C frames are named like "Kernel#sleep [c function]".
def method?(f, name)
  f["name"].delete_suffix(" [c function]") =~ /(\A|[#.])#{name}\z/
end

min_samples = rate * seconds / 4
check traces.size >= min_samples, "#{traces.size} samples (want >= #{min_samples})"

empty = traces.count { |t| t["trace"].empty? }
check empty.zero?, "#{empty} empty stacks"

app = traces.select { |t| t["trace"].any? { |f| app_frame?(f) } }
check app.size >= 20, "#{app.size} samples include frames from #{APP_FILE} (want >= 20)"

# The request path should always be called from puma's thread pool, so a puma
# frame must sit below (at a higher index than) every app frame. A corrupted or
# truncated stack would break this.
misordered = app.reject do |t|
  deepest_app = t["trace"].rindex { |f| app_frame?(f) }
  t["trace"][(deepest_app + 1)..].any? { |f| puma_frame?(f) }
end
check misordered.empty?, "#{misordered.size} app stacks without a puma frame below the app frames"

# call -> cpu_work -> block in cpu_work -> String#sub (C) -> ... ordering.
cpu = app.select { |t| t["trace"].any? { |f| method?(f, "cpu_work") } }
check cpu.size >= 5, "#{cpu.size} samples in cpu_work (want >= 5)"

bad_chain = cpu.reject do |t|
  call_i = index_of(t) { |f| app_frame?(f) && method?(f, "call") }
  work_i = index_of(t) { |f| method?(f, "cpu_work") }
  call_i && work_i < call_i
end
check bad_chain.empty?, "#{bad_chain.size} cpu_work stacks without PumaWorkload#call below cpu_work"

blocks = cpu.count { |t| t["trace"].any? { |f| f["name"] =~ /\Ablock .*in .*cpu_work\z/ } }
check blocks >= 5, "#{blocks} samples inside cpu_work's regex block (want >= 5)"

# A C frame sandwiched between Ruby frames:
# cpu_work -> String#sub [c function] -> block in cpu_work.
# (Off-GVL frames like Kernel#sleep under io_wait aren't checked: rbspy samples
# only the thread that last held the GVL, so how often they show up depends on
# scheduling and was ~1 sample per run on Linux CI runners.)
sandwiched = cpu.count do |t|
  block_i = index_of(t) { |f| f["name"] =~ /\Ablock in .*cpu_work\z/ }
  sub_i = index_of(t) { |f| f["name"].end_with?(" [c function]") && method?(f, "sub") }
  work_i = index_of(t) { |f| method?(f, "cpu_work") }
  block_i && sub_i && work_i && block_i < sub_i && sub_i < work_i
end
check sandwiched >= 5, "#{sandwiched} samples with String#sub between cpu_work and its block (want >= 5)"

# Every app frame should point at a real line of code, not a blank line,
# comment, or a line past the end of the file.
bad_lines = app.flat_map { |t| t["trace"].select { |f| app_frame?(f) } }.reject do |f|
  n = f["lineno"]
  n && n >= 1 && n <= app_lines.size && app_lines[n - 1] !~ /\A\s*(#.*)?\z/
end
check bad_lines.empty?,
  "#{bad_lines.size} app frames with implausible line numbers" +
  (bad_lines.empty? ? "" : ": #{bad_lines.first(5).map { |f| "#{f["name"]}:#{f["lineno"].inspect}" }.uniq.join(", ")}")

if threads > 1
  tids = app.map { |t| [t["pid"], t["thread_id"]] }.uniq.size
  check tids >= 2, "app frames seen on #{tids} distinct threads (want >= 2)"
end

pids = app.map { |t| t["pid"] }.uniq
if workers > 0
  check !pids.include?(puma_pid), "no app frames attributed to the puma master (pid #{puma_pid})"
  check pids.size == workers, "app frames seen in #{pids.size} worker processes (want #{workers})"
else
  check pids == [puma_pid], "app frames attributed to puma pid #{puma_pid} (got #{pids.inspect})"
end

exit($failures.zero? ? 0 : 1)
RUBY

if [[ "$failed" -ne 0 ]]; then
  echo "Top collapsed stacks:" >&2
  awk '{ print $NF "\t" $0 }' "$collapsed" 2>/dev/null | sort -nr | head -n 10 | cut -f2- >&2 || true
  exit 1
fi

echo "PASS: rbspy produced sane stacks for puma under load"
