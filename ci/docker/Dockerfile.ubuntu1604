FROM ubuntu:16.04

ADD ./sources.list.xenial /etc/apt/sources.list

RUN apt-get update
RUN apt-get install -y --force-yes curl build-essential git zlib1g-dev libssl-dev libreadline-dev libyaml-dev libxml2-dev libxslt-dev
RUN apt-get clean

# Install rbenv and ruby-build
RUN git clone https://github.com/sstephenson/rbenv.git /root/.rbenv
RUN git clone https://github.com/sstephenson/ruby-build.git /root/.rbenv/plugins/ruby-build
RUN /root/.rbenv/plugins/ruby-build/install.sh
ENV PATH /root/.rbenv/bin:$PATH
RUN echo 'eval "$(rbenv init -)"' >> /etc/profile.d/rbenv.sh # or /etc/profile
RUN echo 'eval "$(rbenv init -)"' >> .bashrc

# Install multiple versions of ruby
ENV CONFIGURE_OPTS --disable-install-doc

RUN rbenv install 2.5.0
RUN rbenv install 2.4.3
RUN rbenv install 2.4.2
RUN rbenv install 2.4.1
RUN rbenv install 2.4.0
RUN rbenv install 2.3.6
RUN rbenv install 2.3.5
RUN rbenv install 2.3.4
RUN rbenv install 2.3.3
RUN rbenv install 2.3.2
RUN rbenv install 2.3.1
RUN rbenv install 2.3.0
RUN rbenv install 2.2.9
RUN rbenv install 2.2.8
RUN rbenv install 2.2.7
RUN rbenv install 2.2.6
RUN rbenv install 2.2.5
RUN rbenv install 2.2.4
RUN rbenv install 2.2.3
RUN rbenv install 2.2.2
RUN apt-get install -y --force-yes libffi-dev
RUN apt-get clean
RUN rbenv install 2.2.1
RUN rbenv install 2.2.0
RUN rbenv install 2.1.10
RUN rbenv install 2.1.9
RUN rbenv install 2.1.8
RUN rbenv install 2.1.7
RUN rbenv install 2.1.6
RUN rbenv install 2.1.5
RUN rbenv install 2.1.4
RUN rbenv install 2.1.3
RUN rbenv install 2.1.2
RUN rbenv install 2.1.1
RUN rbenv install 2.1.0
RUN rbenv install 1.9.3-p551
 
RUN apt-get update
RUN apt-get install -y --force-yes ruby2.3

ENV PATH /root/.rbenv/shims:$PATH

RUN ln -s /root/.rbenv/versions/1.9.3-p551 /root/.rbenv/versions/1.9.3
