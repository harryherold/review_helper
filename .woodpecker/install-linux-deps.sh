#!/bin/sh
set -eu

apt-get update
apt-get install -y --no-install-recommends \
    libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libxkbcommon-x11-dev \
    libudev-dev libinput-dev libfontconfig-dev libssl-dev pkg-config python3
