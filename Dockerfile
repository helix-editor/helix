FROM rust:slim-bookworm

LABEL org.opencontainers.image.source=https://github.com/neilpandya/helix-armv6l.git

# Set arguments for dynamic UID, GID, and username
ARG USERNAME
ARG UID
ARG GID

# Install required packages for cross-compilation and static linking
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    git \
    wget \
    gcc-arm-linux-gnueabi \
    g++-arm-linux-gnueabi \
    libc6-dev-armel-cross \
    libgcc-12-dev-armel-cross \
    libstdc++-12-dev-armel-cross \
    libssl-dev \
    zlib1g-dev \
    ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Install cross for cross-compilation and set up the ARM target
RUN cargo install cross && \
    echo "Installed Target(s):" && \
    rustup target list --installed

# Set permissions for cargo
RUN chmod -v -R 777 /usr/local/cargo

# Create non-root user with the provided UID and GID
RUN groupadd -g $GID $USERNAME && \
    useradd -u $UID -g $GID -s /bin/bash $USERNAME

ENTRYPOINT ["bash"]


