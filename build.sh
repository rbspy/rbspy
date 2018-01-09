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


echo "============"
echo "distro tests"
echo "============"
echo ""

for distro in ubuntu1404 ubuntu1704 fedora arch2018
do
   docker build -t rb-stacktrace-$distro -f ./docker/Dockerfile.$distro  ./docker/ >> /tmp/output 2>&1
   echo -n "${distro}... "
   docker run -v=/tmp/artifacts:/stuff rb-stacktrace-$distro  env RUST_LOG=debug RUST_BACKTRACE=1 /stuff/ruby-stacktrace stackcollapse /usr/bin/ruby /stuff/short_program.rb >> /tmp/output 2>&1
   if [ $? -eq 0 ]
   then
       echo "Success!"
   else
       echo "Failure!"
   fi
done

echo "=================="
echo "ruby version tests"
echo "=================="
echo ""

for version in 1.9.3 2.1.0 2.1.1 2.1.2 2.1.3 2.1.4 2.1.5 2.1.6 2.1.7 2.1.8 2.1.9 2.1.10 2.2.0 2.2.1 2.2.2 2.2.3 2.2.4 2.2.5 2.2.6 2.2.7 2.2.8 2.2.9 2.3.0 2.3.1 2.3.2 2.3.3 2.3.4 2.3.5 2.3.6 2.4.0 2.4.1 2.4.2 2.4.3 2.5.0
do
   echo -n "${version}... "
   docker run -v=/tmp/artifacts:/stuff -t rb-stacktrace-ubuntu1604 env PATH=/root/.rbenv/shims:/usr/bin:/bin RUST_LOG=debug RUST_BACKTRACE=1 RBENV_VERSION=$version /stuff/ruby-stacktrace stackcollapse ruby /stuff/short_program.rb >> /tmp/output 2>&1
   if [ $? -eq 0 ]
   then
       echo "Success!"
   else
       echo "Failure!"
   fi
done
