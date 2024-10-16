#!/bin/bash

# Set CARGO_BUILD_JOBS environment variable to utilize 95% of available CPU cores for building 
# Rust projects. Comment out or change to your discretion.
export CARGO_BUILD_JOBS=$(($(nproc) * 95 / 100))
echo "CARGO_BUILD_JOBS set to $CARGO_BUILD_JOBS"

# Verify and build for the required Rust target for ARMv6L (arm-unknown-linux-gnueabihf)
rustup target add arm-unknown-linux-gnueabi && \
echo "Installed Target(s):" && \
rustup target list --installed && \
cargo build \
    --target=arm-unknown-linux-gnueabi \
    --release