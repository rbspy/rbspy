FROM alpine:3.13 as musl

ARG TARGETPLATFORM
ARG VERSION

RUN case $TARGETPLATFORM in \
  "linux/amd64") \
    arch=x86_64 \
    ;; \
  "linux/arm64") \
    arch=aarch64 \
    ;; \
  *) \
    echo "Architecture $TARGETPLATFORM is not supported" \
    exit 1 \
    ;; \
  esac && \
  wget -q -O - https://github.com/rbspy/rbspy/releases/download/v$VERSION/rbspy-$arch-musl.tar.gz | tar -xzf - && \
  install -m755 rbspy-$arch-musl /usr/bin/rbspy && \
  rm -f rbspy-$arch-musl

RUN rbspy --version

FROM ubuntu:focal-20210416 as gnu

ARG TARGETPLATFORM
ARG VERSION

RUN apt update -qq && apt install -y -qq wget
RUN case $TARGETPLATFORM in \
  "linux/amd64") \
    arch=x86_64 \
    ;; \
  "linux/arm64") \
    arch=aarch64 \
    ;; \
  *) \
    echo "Architecture $TARGETPLATFORM is not supported" \
    exit 1 \
    ;; \
  esac && \
  wget -q -O - https://github.com/rbspy/rbspy/releases/download/v$VERSION/rbspy-$arch-unknown-linux-gnu.tar.gz | tar -xzf - && \
  install -m755 rbspy-$arch-unknown-linux-gnu /usr/bin/rbspy && \
  rm -f rbspy-$arch-unknown-linux-gnu

RUN rbspy --version
