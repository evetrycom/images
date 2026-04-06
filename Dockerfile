# Stage 1: Build
FROM rust:1.94-slim-bookworm AS builder

# Install system dependencies for libvips and AWS SDK (OpenSSL)
RUN apt-get update && apt-get install -y --fix-missing \
    pkg-config \
    libvips-dev \
    librsvg2-dev \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app
COPY . .

# Build in release mode
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install only the runtime libvips library
RUN apt-get update && apt-get install -y --fix-missing \
    libvips42 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/app/target/release/images /app/evetry-images

# Port for Axum
EXPOSE 3000

CMD ["./evetry-images"]
