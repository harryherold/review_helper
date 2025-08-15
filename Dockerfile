FROM ubuntu:latest

RUN apt-get update

RUN apt-get install -y \
    build-essential \
    curl \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libxkbcommon-dev \
    libxkbcommon-x11-dev \
    libudev-dev \
    libinput-dev \
    libfontconfig-dev


RUN curl https://sh.rustup.rs -sSf | bash -s -- -y

RUN echo 'source $HOME/.cargo/env' >> $HOME/.bashrc

WORKDIR /review_helper
