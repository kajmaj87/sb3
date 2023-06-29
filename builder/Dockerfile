FROM ubuntu:latest

RUN apt-get update && apt-get install -y pkg-config libudev-dev libasound2-dev build-essential alsa \
libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev mingw-w64
RUN apt-get install -y curl

WORKDIR /usr/src/myapp
