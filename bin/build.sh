#!/bin/sh
# Builds helix.

echo "Starting to build..."
date
cargo install --path helix-term --locked
date
echo "Done building."
