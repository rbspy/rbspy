set -u

set -e
env CARGO_INCREMENTAL=1 cargo build
rm -rf /tmp/artifacts
mkdir /tmp/artifacts
cp target/debug/ruby-stacktrace /tmp/artifacts
cp examples/short_program.rb /tmp/artifacts
cp examples/infinite.rb /tmp/artifacts
set +e

rm -f /tmp/output
touch /tmp/output
for distro in ubuntu1604 ubuntu1404 fedora
do
   echo "Distro: $distro"
   echo "Building Dockerfile..."
   docker build -t rb-stracktrace-$distro -f ./docker/Dockerfile.$distro  ./docker/ >> /tmp/output 2>&1
   echo "Running rbenv 2.3.1..."
   docker run -v=/tmp/artifacts:/stuff -t rb-stacktrace-$distro  env PATH=/root/.rbenv/shims:/usr/bin:/bin RUST_LOG=debug RUST_BACKTRACE=1 RBENV_VERSION=2.3.1 /stuff/ruby-stacktrace stackcollapse ruby /stuff/short_program.rb >> /tmp/output 2>&1
   if [ $? -eq 0 ]
   then
       echo "Success!"
   else
       echo "Failure!"
   fi
   echo "Running system ruby..."
   docker run -v=/tmp/artifacts:/stuff -t rb-stacktrace-$distro  env RUST_LOG=debug RUST_BACKTRACE=1 /stuff/ruby-stacktrace stackcollapse /usr/bin/ruby /stuff/short_program.rb >> /tmp/output 2>&1
   if [ $? -eq 0 ]
   then
       echo "Success!"
   else
       echo "Failure!"
   fi
done
