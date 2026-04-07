# Stage 1: Build
FROM rust:1-slim-bookworm AS builder

# Install build tools and dependencies for libvips
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    curl \
    tar \
    ca-certificates \
    libssl-dev \
    build-essential \
    ninja-build \
    python3-pip \
    bc \
    wget \
    libfftw3-dev \
    libopenexr-dev \
    libgsf-1-dev \
    libglib2.0-dev \
    liborc-dev \
    libopenslide-dev \
    libmatio-dev \
    libwebp-dev \
    libjpeg62-turbo-dev \
    libexpat1-dev \
    libexif-dev \
    libtiff-dev \
    libcfitsio-dev \
    libpoppler-glib-dev \
    librsvg2-dev \
    libpango1.0-dev \
    libopenjp2-7-dev \
    liblcms2-dev \
    libimagequant-dev \
    libcgif-dev \
    && pip3 install meson --break-system-packages \
    && rm -rf /var/lib/apt/lists/*

# Build libvips from source
ENV VIPS_VERSION=8.18.2
RUN wget https://github.com/libvips/libvips/releases/download/v${VIPS_VERSION}/vips-${VIPS_VERSION}.tar.xz && \
    tar xf vips-${VIPS_VERSION}.tar.xz && \
    cd vips-${VIPS_VERSION} && \
    meson setup build --prefix=/vips --libdir=lib --buildtype=release -Dintrospection=disabled && \
    meson compile -C build && \
    meson install -C build && \
    cd .. && rm -rf vips-${VIPS_VERSION}*

# Configure build environment
ENV PKG_CONFIG_PATH=/vips/lib/pkgconfig
ENV LD_LIBRARY_PATH=/vips/lib

WORKDIR /usr/src/app

# --- OPTIMASI: Caching dependencies ---
COPY Cargo.toml Cargo.lock ./
# Create dummy build script and main.rs to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > build.rs
RUN cargo build --release
RUN rm -rf src/ build.rs
# --------------------------------------

# Copy original source
COPY . .

# Build with compiled libvips
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install minimal runtime libraries for libvips and the app
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    libglib2.0-0 \
    libwebp7 \
    libjpeg62-turbo \
    libtiff6 \
    libexif12 \
    libgsf-1-114 \
    liborc-0.4-0 \
    liblcms2-2 \
    libimagequant0 \
    libcgif0 \
    libexpat1 \
    libfftw3-double3 \
    libpng16-16 \
    libopenjp2-7 \
    libpoppler-glib8 \
    librsvg2-2 \
    libpango-1.0-0 \
    libpangocairo-1.0-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /usr/src/app/target/release/images /app/evetry-images

# Copy compiled libvips shared libraries
COPY --from=builder /vips/lib /app/lib

# Ensure the binary finds our bundled libraries
ENV LD_LIBRARY_PATH=/app/lib
ENV PORT=3000
ENV RUST_LOG=info

EXPOSE 3000

CMD ["./evetry-images"]
