FROM ubuntu:latest

RUN apt-get update

RUN apt-get install -y \
    build-essential \
    curl


RUN curl https://sh.rustup.rs -sSf | bash -s -- -y

RUN echo 'source $HOME/.cargo/env' >> $HOME/.bashrc

WORKDIR /review_todo
