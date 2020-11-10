FROM debian:buster

RUN apt-get update
RUN apt-get install -y git ruby build-essential autoconf bison libssl-dev zlib1g-dev
RUN apt-get install -y clang libclang-dev curl

RUN git clone https://github.com/ruby/ruby.git ~/clones/ruby

ENV RUBY_VERSION=2_7_2

WORKDIR /root/clones/ruby
RUN git checkout v${RUBY_VERSION}
RUN autoconf
RUN ./configure --disable-install-doc
RUN make
RUN make install

RUN git clone https://github.com/rbspy/rbspy.git /opt/rbspy

WORKDIR /opt/rbspy
ENV PATH=~/clones/ruby:${PATH}

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly-2020-10-08
RUN ~/.cargo/bin/cargo install bindgen
ENV PATH=~/.cargo/bin:${PATH}

COPY scripts/bindgen.sh /opt/rbspy/scripts/bindgen.sh

CMD bash /opt/rbspy/scripts/bindgen.sh ${RUBY_VERSION}
