# Stage 1: Build
FROM rust:1-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --fix-missing \
    pkg-config \
    libvips-dev \
    librsvg2-dev \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# --- OPTIMASI: Caching dependencies ---
# Copy hanya file manifest untuk build dependensi dulu
COPY Cargo.toml Cargo.lock ./
# Buat file dummy agar cargo bisa build dependensi
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "fn main() {}" > build.rs
RUN cargo build --release
RUN rm -rf src/
# --------------------------------------

# Copy kode sumber asli
COPY . .

# Build ulang binary asli
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime libraries
RUN apt-get update && apt-get install -y --fix-missing \
    libvips42 \
    librsvg2-2 \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/app/target/release/images /app/evetry-images

# Port default Axum
EXPOSE 3000

# Opsional: Tambahkan ENV default
ENV PORT=3000
ENV RUST_LOG=info

CMD ["./evetry-images"]
