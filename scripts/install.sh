#!/bin/bash
set -euxo pipefail

# install packages
sudo apt update -y \
        && sudo apt upgrade -y \
        && sudo apt install -y build-essential libssl-dev pkg-config

# install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh \
        && chmod +x ./rustup.sh \
        && ./rustup.sh -y \
        && source ~/.bashrc