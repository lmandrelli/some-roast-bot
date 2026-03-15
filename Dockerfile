# Stage 1: Build the Rust binary
FROM rust:1.94-bookworm AS builder

WORKDIR /app

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies separately (cache layer)
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy actual source code and rebuild
COPY src/ src/

# Touch main.rs to invalidate the cached dummy binary but keep cached deps
RUN touch src/main.rs
RUN cargo build --release

# Stage 2: Minimal runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/some-roast-bot /usr/local/bin/some-roast-bot

CMD ["some-roast-bot"]
