#!/usr/bin/env bash

if [[ -z "$1" ]]; then
  echo "usage: build-and-test.sh <username-to-check>"
  exit 1
fi

# install pam service config
sudo cp conf/example-rust-pam-service /etc/pam.d

# build
cargo build --release
if [ $? -ne 0 ]; then echo "cargo build failed"; exit 1; fi

# prep the executable
sudo mv target/release/example-rust-pam .
sudo chown root:root example-rust-pam
sudo setcap 'cap_setuid,cap_setgid=ep' example-rust-pam

# debug for sanity check
id; getcap example-rust-pam

# run the test
./example-rust-pam -s example-rust-pam-service "$1"

# cleanup
sudo rm ./example-rust-pam
sudo rm /etc/pam.d/example-rust-pam-service
