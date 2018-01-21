FROM ubuntu:14.04

ADD ./sources.list.trusty /etc/apt/sources.list

RUN apt-get  update
RUN apt-get install -y --force-yes ruby
RUN apt-get clean
