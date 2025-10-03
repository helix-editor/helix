FROM rust:latest

# Set working directory to where we'll copy the repo
WORKDIR /src

# Copy Helix repo from host
# Assumes the Dockerfile is in the root of the repo
COPY . /src/helix

# Set environment variables
ENV HELIX_DISABLE_AUTO_GRAMMAR_BUILD=true

# Now, cargo install will put binaries into /src/helix/out inside the container
ENV CARGO_INSTALL_ROOT=/src/helix/out

# Build and install Helix (cargo install)
WORKDIR /src/helix
RUN cargo install \
    --profile opt \
    --config 'build.rustflags="-C target-cpu=native"' \
    --path helix-term \
    --locked
