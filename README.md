# Images 🖼️

**Images** is a high-performance, enterprise-grade image processing and proxy microservice built with **Rust** and **libvips**. Designed for speed, security, and smart media handling within the **Evetry** ecosystem.

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Docker](https://img.shields.io/badge/Docker-Ready-blue.svg)](./Dockerfile)
[![Framework](https://img.shields.io/badge/Framework-Axum-orange.svg)](https://github.com/tokio-rs/axum)
[![Language](https://img.shields.io/badge/Language-Rust-black.svg)](https://www.rust-lang.org/)

## ✨ Features

- 🌐 **Multi-Origin Support**: Fetch images from Cloudflare R2, AWS S3, or any external remote URL.
- 🎞️ **Smart Animation Engine**: Auto-detects and preserves animations (GIF/WebP/AVIF) by default.
- 📐 **Vector Rendering**: High-quality SVG rasterization at any scale.
- 🧩 **Advanced Metadata (JSON)**: Get technical image details via `output=json`.
- 🖼️ **Overlay & Watermarks**: Add branding layers using `overlay`, `ox`, and `oy` parameters.
- 🎭 **Shape Masking**: Apply precise masks using `mask=circle`, `ellipse`, or custom SVG `path`.
- 🧠 **Smart Cropping**: Intelligent focusing using `a=attention` or `a=entropy`.
- ⚡ **Edge Optimized**: Aggressive `Cache-Control` headers for maximum performance with **Cloudflare Edge**.
- 🔐 **Secure by Default**: Built-in HMAC-SHA256 signature validation with `APP_SECRET`.

## 🚀 Quick Start

### Local Setup (Docker)

1. **Configure Environment**:
   Copy `.env.example` to `.env` and set your S3/R2 credentials and `APP_SECRET`.
   ```bash
   cp .env.example .env
   ```

2. **Run Container**:
   ```bash
   docker build -t evetry-images .
   docker run -p 3000:3000 --env-file .env evetry-images
   ```

## 📡 API Documentation

### Image Processing
`GET /<path>?<params>` or `GET /url/<remote_url>?<params>`

#### URL Examples
| Type | Example URL |
| :--- | :--- |
| **S3/R2 Animation** | `http://localhost:3000/dance.gif?w=300&output=webp` |
| **JSON Metadata** | `http://localhost:3000/photo.jpg?output=json` |
| **Mask (Circle)** | `http://localhost:3000/avatar.jpg?w=200&h=200&mask=circle` |
| **Custom Path** | `http://localhost:3000/img.jpg?mask=path&d=M50,0 L100,100 L0,100 Z` |
| **Watermark** | `http://localhost:3000/photo.jpg?w=800&overlay=logo.png&ox=10&oy=10` |
| **SVG Target** | `http://localhost:3000/logo.svg?w=1000&output=avif` |

#### Supported Parameters
| Param | Type | Description |
| :--- | :--- | :--- |
| `w`, `h` | number | Target width/height. |
| `fit` | string | `cover`, `contain`, `fill`, `inside`, `outside`. |
| `a` | string | Alignment/Smart Focus: `entropy`, `attention`. |
| `n` | number | All frames (`-1`) or first frame (`1`). |
| `blur` | number | Gaussian blur sigma. |
| `sharp` | number | Sharpening sigma. |
| `overlay` | string | S3 key or URL for watermark image. |
| `mask` | string | Apply shape mask: `circle`, `ellipse`, `path`. |
| `d` | string | SVG path data for `mask=path`. |
| `output` | string | `webp`, `avif`, `png`, `jpg`, `gif`, `jxl`, `json`. |
| `q` | number | Quality (0-100). |
| `sig` | string | HMAC-SHA256 signature. |

### JSON Metadata Example (`output=json`)
```json
{
  "status": "success",
  "data": {
    "format": "vips-internal",
    "width": 1200,
    "height": 630,
    "isAnimated": true,
    "frameCount": 24,
    "hasAlpha": true,
    "space": "sRGB",
    "channels": 3
  }
}
```

## 📝 License

Licensed under the **Apache License 2.0**. See [LICENSE](./LICENSE) for more information.

---
Built with ❤️ for the **Evetry** ecosystem.
