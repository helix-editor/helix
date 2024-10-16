#!/bin/bash

# Run the container
docker run \
    --name rust-debian-bookworm-slim-crosscompile-armv6l \
    --hostname crosscompile-helix \
    --rm \
    -it \
    --user $(id -u):$(id -g) \
    -v "$PWD"/:/helix-armv6l \
    -w /helix-armv6l \
    neilpandya/rust:debian-bookworm-slim-crosscompile-armv6l