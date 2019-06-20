set -eux
rm -rf /tmp/artifacts
mkdir /tmp/artifacts
cp target/$TARGET/debug/rbspy /tmp/artifacts
cp ci/ruby-programs/* /tmp/artifacts

rm -f /tmp/output
touch /tmp/output

set +x

echo "============"
echo "distro tests"
echo "============"
echo ""

set -x

for distro in ubuntu1404 ubuntu1704 fedora arch2018
do
   docker build -t rb-stacktrace-$distro -f ./ci/docker/Dockerfile.$distro  ./ci/docker/ >> /tmp/output 2>&1
   echo -n "${distro}... "
   docker run -v=/tmp/artifacts:/stuff rb-stacktrace-$distro env RUST_BACKTRACE=1 /stuff/rbspy record --file /tmp/stacks.txt /usr/bin/ruby /stuff/short_program.rb

   echo -n "subprocess identification"
   docker run -v=/tmp/artifacts:/stuff rb-stacktrace-$distro env RUST_BACKTRACE=1 /stuff/rbspy record --subprocesses /usr/bin/ruby /stuff/ruby_forks.rb

   echo -n "unicode test"
   docker run -v=/tmp/artifacts:/stuff rb-stacktrace-$distro env RUST_BACKTRACE=1 /stuff/rbspy record --file /tmp/stacks.txt /usr/bin/ruby /stuff/unicode_stack.rb
   grep € /tmp/stacks.txt
done

echo "=================="
echo "ruby version tests"
echo "=================="
echo ""


docker run -v=/tmp/artifacts:/stuff -t jvns/rbspy-ci:ubuntu1604 ls /root/.rbenv/versions

for version in 2.1.0 2.1.1 2.1.2 2.1.3 2.1.4 2.1.5 2.1.6 2.1.7 2.1.8 2.1.9 2.1.10 2.2.0 2.2.1 2.2.2 2.2.3 2.2.4 2.2.5 2.2.6 2.2.7 2.2.8 2.2.9 2.3.0 2.3.1 2.3.2 2.3.3 2.3.4 2.3.5 2.3.6 2.4.0 2.4.1 2.4.2 2.4.3 2.5.0 1.9.3-p551
do
   echo -n "${version}... "
   docker run -v=/tmp/artifacts:/stuff -t jvns/rbspy-ci:ubuntu1604 env RUST_BACKTRACE=1 /stuff/rbspy record --file /tmp/stacks.txt  /root/.rbenv/versions/$version/bin/ruby /stuff/short_program.rb
done
