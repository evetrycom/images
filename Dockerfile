# Stage 1: Build
FROM rust:1-slim-bookworm AS builder

# Install build tools and dependencies for libvips
RUN echo "deb http://deb.debian.org/debian bookworm-backports main" > /etc/apt/sources.list.d/backports.list
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
    libheif-dev \
    libaom-dev \
    libde265-dev \
    libx265-dev \
    && apt-get install -y -t bookworm-backports --no-install-recommends \
       libheif-plugin-aomenc \
       libheif-plugin-aomdec \
       libheif-plugin-x265 \
    && pip3 install meson --break-system-packages \
    && rm -rf /var/lib/apt/lists/*

# Configure build environment
ENV PKG_CONFIG_PATH=/vips/lib/pkgconfig
ENV LD_LIBRARY_PATH=/vips/lib

# Build libvips from source
ENV VIPS_VERSION=8.18.2
RUN wget https://github.com/libvips/libvips/releases/download/v${VIPS_VERSION}/vips-${VIPS_VERSION}.tar.xz && \
    tar xf vips-${VIPS_VERSION}.tar.xz && \
    cd vips-${VIPS_VERSION} && \
    meson setup build --prefix=/vips --libdir=lib --buildtype=release \
        -Dintrospection=disabled \
        -Dheif=enabled && \
    meson compile -C build && \
    meson install -C build && \
    # VERIFICATION: Ensure heifsave was actually compiled in
    LD_LIBRARY_PATH=/vips/lib /vips/bin/vips -l | grep -q heifsave && \
    cd .. && rm -rf vips-${VIPS_VERSION}*

WORKDIR /usr/src/app

# --- OPTIMASI: Caching dependencies ---
COPY Cargo.toml Cargo.lock ./
# Strip the build script declaration so the dummy step doesn't run a fake build.rs.
# This avoids caching empty link directives that would shadow the real build.rs output.
RUN sed -i '/^build = /d' Cargo.toml && \
    mkdir src && \
    echo "fn main() {}" > src/main.rs
RUN cargo build --release
# Delete images-specific fingerprint & build cache so the real build.rs is forced to re-run.
RUN rm -rf src/ \
    target/release/build/images-* \
    target/release/.fingerprint/images-* \
    target/release/deps/images-*
# (COPY . . will overwrite Cargo.toml with the real one containing build = "build.rs")
# --------------------------------------

# Copy original source
COPY . .

# Build with compiled libvips
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

# Enable backports for libheif plugins
RUN echo "deb http://deb.debian.org/debian bookworm-backports main" > /etc/apt/sources.list.d/backports.list

# Install runtime versions of ALL dependencies used to compile libvips from source.
# These must match the -dev packages installed in the builder stage.
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Core system
    libssl3 \
    ca-certificates \
    # GLib / GObject
    libglib2.0-0 \
    # Image codecs
    libjpeg62-turbo \
    libpng16-16 \
    libtiff6 \
    libwebp7 \
    libwebpdemux2 \
    libwebpmux3 \
    libopenjp2-7 \
    libcgif0 \
    libexif12 \
    libcfitsio10 \
    libimagequant0 \
    # Scientific / data formats
    libmatio11 \
    liborc-0.4-0 \
    # PDF / SVG / document
    libpoppler-glib8 \
    librsvg2-2 \
    libgsf-1-114 \
    libopenslide0 \
    # Text rendering
    libpango-1.0-0 \
    libpangocairo-1.0-0 \
    # Color management & math
    liblcms2-2 \
    libfftw3-double3 \
    # OpenEXR (HDR)
    libopenexr-3-1-30 \
    # HEIF/AVIF (Main library)
    libheif1 \
    libaom3 \
    libde265-0 \
    libx265-199 \
    # Misc
    libexpat1 \
    && apt-get install -y -t bookworm-backports --no-install-recommends \
       libheif-plugin-aomenc \
       libheif-plugin-aomdec \
       libheif-plugin-x265 \
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
