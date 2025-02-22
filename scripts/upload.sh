#!/bin/bash
set -euxo pipefail

SERVER=$1
scp -i ~/.ssh/developer.pem ./install.sh "ubuntu@${SERVER}:~/"
scp -i ~/.ssh/developer.pem ../target/debug/webserver "ubuntu@${SERVER}:~/"
scp -i ~/.ssh/developer.pem ../.env "ubuntu@${SERVER}:~/"