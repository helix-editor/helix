FROM quay.io/fedora/fedora:latest

# Install dependencies
RUN dnf install -y git cargo

# Set working directory
WORKDIR /src

# Clone Helix repo
RUN git clone https://github.com/D4ario0/helix helix

# Set environment variables to match your workflow
ENV HELIX_DISABLE_AUTO_GRAMMAR_BUILD=true
ENV CARGO_INSTALL_ROOT=/out

# Build and install Helix (cargo install)
WORKDIR /src/helix
RUN cargo install \
    --profile opt \
    --config 'build.rustflags="-C target-cpu=native"' \
    --path helix-term \
    --locked

# Default working directory when container runs
WORKDIR /out
