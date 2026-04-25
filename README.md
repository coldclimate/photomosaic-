# Photomosaic Maker (Rust)

A high-performance, portable photomosaic maker written in Rust. Creates image collages by arranging small tile images to recreate a target image.

**Features**:
- **Two modes**:
  - Unfair (best-fit, greedy assignment) - fast, photorealistic
  - Fair (Hungarian algorithm) - balanced tile usage, maximum variety
- Rank-based frequency penalty for balanced tile usage (unfair mode)
- Floyd–Steinberg dithering
- Transparency masking
- Alpha and brightness blending
- Multiple color spaces (BGR, LAB, HSV, HSL, LUV)
- Multiple distance metrics (Euclidean, Cityblock, Chebyshev, Cosine)
- Cross-platform compilation (especially M1/M3 macOS support)

## Installation

### Prerequisites
Install Rust from https://rustup.rs/

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Build

```bash
cd photomosaic
cargo build --release
```

The binary will be at `target/release/photomosaic`.

## Building for M3 (arm64)

```bash
# Add the M3/arm64 target (one-time)
rustup target add aarch64-apple-darwin

# Build natively on M3 Mac
cargo build --release

# Or cross-compile from Intel Mac
cargo build --release --target aarch64-apple-darwin
```

## Usage

```bash
photomosaic \
  --path <tiles-directory> \
  --dest-img <target-image> \
  --out <output-image> \
  [options]
```

### Required Arguments

- `--path` — Directory containing tile images
- `--dest-img` — Target image to recreate

### Optional Arguments

| Argument | Type | Default | Description |
|----------|------|---------|-------------|
| `--out` | PATH | result.png | Output image path |
| `--size` | INT [INT] | 50 | Tile size: width or width height |
| `--recursive` | flag | false | Scan subdirectories for tiles |
| `--max-width` | INT | 80 | Grid width (in tiles) |
| `--freq-mul` | FLOAT | 0.0 | Frequency multiplier (tile fairness) |
| `--deterministic` | flag | false | No random shuffling |
| `--dither` | flag | false | Floyd–Steinberg dithering |
| `--colorspace` | ENUM | lab | Color space: bgr, lab, hsv, hsl, luv |
| `--metric` | ENUM | euclidean | Distance metric: euclidean, cityblock, chebyshev, cosine |
| `--resize-opt` | ENUM | center | Tile resize: center, stretch, fit |
| `--auto-rotate` | INT | 0 | Auto-rotate tiles: -1 (cw), 0 (no), 1 (ccw) |
| `--blending` | ENUM | alpha | Blending: alpha, brightness |
| `--blending-level` | FLOAT | 0.0 | Blending strength: 0.0–1.0 |
| `--transparent` | flag | false | Transparency masking |
| `--fair` | flag | false | Fair tile usage (Hungarian algorithm) |
| `--dup` | FLOAT | 1.0 | Tile duplication (for fair mode) |
| `--num-threads` | INT | cpu/2 | Parallel threads |
| `--quiet` | flag | false | Suppress progress output |
| `--tile-info-out` | PATH | - | Save tile assignment CSV |
| `--mem-limit` | INT | 4096 | Memory limit (MB) |

## Examples

### Basic unfair photomosaic
```bash
photomosaic \
  --path img/tiles \
  --dest-img img/target.jpg \
  --size 25 \
  --max-width 56
```

### With frequency balancing (more variety, less quality)
```bash
photomosaic \
  --path img/tiles \
  --dest-img img/target.jpg \
  --size 25 \
  --max-width 56 \
  --freq-mul 1.0
```

### With dithering and blending
```bash
photomosaic \
  --path img/tiles \
  --dest-img img/target.jpg \
  --size 10 \
  --max-width 200 \
  --dither \
  --freq-mul 0.0 \
  --blending alpha \
  --blending-level 0.25
```

### Transparency masking
```bash
photomosaic \
  --path img/tiles \
  --dest-img img/target-transparent.png \
  --size 25 \
  --max-width 56 \
  --transparent
```

### Fair mode (optimal assignment, limited fairness)
```bash
photomosaic \
  --path img/tiles \
  --dest-img img/target.jpg \
  --size 25 \
  --fair \
  --dup 6
```

Uses Hungarian algorithm for optimal cost assignment. **Warning**: Does NOT guarantee balanced tile usage—optimizes for color match first. Only practical for < 200 tiles due to O(n³) complexity.

### Balanced tile usage (recommended)
For truly balanced tile distribution, use unfair mode with frequency penalty:
```bash
photomosaic \
  --path img/tiles \
  --dest-img img/target.jpg \
  --size 25 \
  --freq-mul 0.8  # penalizes repeated tiles
```

This actually enforces tile fairness while maintaining good color quality.

## Performance Notes

- **Tile loading** is parallelized with Rayon
- **Distance computation** uses chunking to respect `--mem-limit`
- **Colorspace conversion** is done on-the-fly (no memory overhead)
- **Dithering** adds ~20% overhead but improves visual quality with small tiles

For large tile counts (>10k tiles) or high-resolution destination images (>8MP), GPU acceleration would be beneficial (future feature).
