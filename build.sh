set -u

set -e
env CARGO_INCREMENTAL=1 cargo build
cp target/debug/ruby-stacktrace docker/
cp examples/short_program.rb docker/
set +e

for distro in ubuntu1604 ubuntu1404 fedora
do
   echo "Distro: $distro"
   rm -f /tmp/output
   touch /tmp/output
   echo "Building Dockerfile..."
   docker build -t rb-stracktrace-$distro -f ./docker/Dockerfile.$distro  ./docker/ >> /tmp/output 2>&1
   echo "Running rbenv 2.3.1..."
   docker run -t rb-stacktrace-ubuntu1404 env PATH=/root/.rbenv/shims:/usr/bin:/bin RUST_BACKTRACE=1 RBENV_VERSION=2.3.1 /stuff/ruby-stacktrace stackcollapse ruby /stuff/short_program.rb >> /tmp/output 2>&1
   if [ $? -eq 0 ]
   then
       echo "Success!"
   else
       echo "Failure!"
   fi
   echo "Running system ruby..."
   docker run -t rb-stacktrace-ubuntu1404 env RUST_BACKTRACE=1 /stuff/ruby-stacktrace stackcollapse /usr/bin/ruby /stuff/short_program.rb >> /tmp/output 2>&1
   if [ $? -eq 0 ]
   then
       echo "Success!"
   else
       echo "Failure!"
   fi
done
