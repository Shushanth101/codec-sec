FROM ubuntu:22.04

# Avoid prompt during installations
ENV DEBIAN_FRONTEND=noninteractive

# Install dependencies and language compilers/runtimes (excluding rustc/cargo from apt)
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    pkg-config \
    libcap-dev \
    libseccomp-dev \
    libsystemd-dev \
    gcc \
    g++ \
    openjdk-17-jdk-headless \
    python3 \
    nodejs \
    ruby \
    && rm -rf /var/lib/apt/lists/*

# Install rustup and Rust stable system-wide
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --default-toolchain stable

# Clone and install ioi/isolate sandbox
RUN git clone https://github.com/ioi/isolate.git /tmp/isolate && \
    cd /tmp/isolate && \
    make isolate && \
    make install && \
    rm -rf /tmp/isolate

# Setup working directory for the Rust application
WORKDIR /app

# Copy Cargo files and build dependencies to cache
COPY Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy real source code and build production binary
COPY src ./src
RUN touch src/main.rs && cargo build --release

# Copy docker helper scripts
COPY entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

# Expose port
EXPOSE 54054

ENTRYPOINT ["/app/entrypoint.sh"]
