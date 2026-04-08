# Stage 1: Build
FROM rust:1-slim-bookworm AS builder

# Install build tools and essential dependencies for libvips (Web-focused)
RUN echo "deb http://deb.debian.org/debian bookworm-backports main" > /etc/apt/sources.list.d/backports.list
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config curl tar ca-certificates libssl-dev build-essential ninja-build python3-pip bc wget \
    libfftw3-dev libglib2.0-dev liborc-dev libwebp-dev libjpeg62-turbo-dev \
    libexpat1-dev libexif-dev libtiff-dev librsvg2-dev libpango1.0-dev \
    libopenjp2-7-dev liblcms2-dev libimagequant-dev libcgif-dev \
    libheif-dev libaom-dev libde265-dev libx265-dev \
    && apt-get install -y -t bookworm-backports --no-install-recommends \
       libheif-plugin-aomenc libheif-plugin-aomdec libheif-plugin-x265 \
    && pip3 install meson --break-system-packages \
    && rm -rf /var/lib/apt/lists/*

# Build libvips from source
ENV VIPS_VERSION=8.18.2
RUN wget https://github.com/libvips/libvips/releases/download/v${VIPS_VERSION}/vips-${VIPS_VERSION}.tar.xz && \
    tar xf vips-${VIPS_VERSION}.tar.xz && \
    cd vips-${VIPS_VERSION} && \
    # We install to /vips as the final destination prefix
    meson setup build --prefix=/vips --libdir=lib --buildtype=release \
        -Dintrospection=disabled \
        -Dheif=enabled \
        -Dmodules=disabled && \
    meson compile -C build && \
    meson install -C build && \
    # VERIFICATION: Ensure heifsave is built-in (not a module) or correctly registered
    LD_LIBRARY_PATH=/vips/lib /vips/bin/vips -l | grep -q heifsave && \
    cd .. && rm -rf vips-${VIPS_VERSION}*

WORKDIR /usr/src/app

# --- OPTIMASI: Caching dependencies ---
COPY Cargo.toml Cargo.lock ./
# Strip build script during dummy build to avoid caching empty link directives
RUN sed -i '/^build = /d' Cargo.toml && \
    mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src/ target/release/build/images-* target/release/.fingerprint/images-* target/release/deps/images-*
# --------------------------------------

# Copy original source and build with compiled libvips
COPY . .
ENV PKG_CONFIG_PATH=/vips/lib/pkgconfig
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

# Enable backports for runtime plugins
RUN echo "deb http://deb.debian.org/debian bookworm-backports main" > /etc/apt/sources.list.d/backports.list

# Install runtime versions of essential dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 ca-certificates libglib2.0-0 libjpeg62-turbo libpng16-16 libtiff6 \
    libwebp7 libwebpdemux2 libwebpmux3 libopenjp2-7 libcgif0 libexif12 \
    libimagequant0 liborc-0.4-0 librsvg2-2 libpango-1.0-0 libpangocairo-1.0-0 \
    liblcms2-2 libfftw3-double3 libheif1 libaom3 libde265-0 libx265-199 libexpat1 \
    && apt-get install -y -t bookworm-backports --no-install-recommends \
       libheif-plugin-aomenc libheif-plugin-aomdec libheif-plugin-x265 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /usr/src/app/target/release/images /app/evetry-images

# Copy entire /vips directory to the SAME path to avoid relocation issues
COPY --from=builder /vips /vips

# Environment variables
ENV LD_LIBRARY_PATH=/vips/lib
ENV PORT=3000
ENV RUST_LOG=info

EXPOSE 3000

CMD ["./evetry-images"]
