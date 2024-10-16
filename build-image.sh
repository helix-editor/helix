#!/bin/bash

# Function to check if Docker is installed
check_docker() {
    if ! command -v docker &> /dev/null; then
        echo "Docker is not installed. Please install Docker and try again."
        exit 1
    fi
}

# Function to check if BuildKit is enabled
check_buildkit() {
    if [[ -z "${DOCKER_BUILDKIT}" ]]; then
        printf "Docker BuildKit is not enabled.\nFor more information, 
        see: https://docs.docker.com/develop/develop-images/build_enhancements/
        \nIf you are using Docker Desktop, you can enable BuildKit by going to 
        Settings > Docker Engine and adding\n{\"features\": {\"buildkit\": true}} to 
        the JSON configuration.\nThen, restart Docker Desktop.\nFor example, the command
        'docker buildx create --name mybuilder --use' is used to create a new builder 
        instance\nand set it as the default builder. This will allow you to utilize advanced
        build features, including cross-compilation\nand multi-platform support. Please enable 
        it by setting 'export DOCKER_BUILDKIT=1' in your shell or\nin your Docker configuration 
        and try running this script again.\n"
        exit 1
    fi
}

# Check for Docker installation
check_docker

# Check for BuildKit
check_buildkit

# Build the Docker image
docker buildx build \
    --build-arg USERNAME=$(whoami) \
    --build-arg UID=$(id -u) \
    --build-arg GID=$(id -g) \
    -t neilpandya/rust:debian-bookworm-slim-crosscompile-armv6l . \
    --load