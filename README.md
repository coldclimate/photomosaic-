# Photomosaic

A high-performance photomosaic generator written in Rust. Creates artistic images composed of smaller tile images, with support for multiple color spaces, distance metrics, and advanced features like fair tile distribution and interactive tile maps.

## What It Does

Photomosaic takes a destination image and a directory of tile images, then assembles a mosaic where each region of the destination image is replaced with the best-matching tile. Features include:

- **Multiple color spaces**: LAB, BGR, HSV, HSL, LUV for color matching
- **Distance metrics**: Euclidean, Cityblock, Chebyshev, Cosine
- **Fair tile distribution**: Ensures balanced usage of available tiles
- **Blending modes**: Alpha and brightness blending with the original image
- **Dithering**: Floyd-Steinberg dithering for improved color quality
- **Interactive maps**: Generate HTML visualizations showing which tiles were used where
- **Tile statistics**: Export CSV data about tile usage and placement

## Installation

### Prerequisites

You'll need Rust installed. Install it via Homebrew:

```bash
brew install rust
```

This installs both `rustc` (the Rust compiler) and `cargo` (the package manager).

### Building from Source

Clone the repository and build the project:

```bash
git clone <repository-url>
cd photomosaic
cargo build --release
```

The compiled binary will be available at `target/release/photomosaic`.

Optionally, install it to your system PATH:

```bash
cargo install --path .
```

This allows you to run `photomosaic` from anywhere.

## Basic Usage

### Simplest Example

```bash
./target/release/photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --out result.png
```

This creates a mosaic using:
- Tiles from the `tiles/` directory
- Target image `target.jpg`
- Output saved as `result.png`
- Default tile size: 50×50 pixels
- Default color space: LAB
- Default distance metric: Euclidean

### Common Options

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --out result.png \
  --size 25 \              # Smaller tiles for more detail (25×25 px)
  --size 100 200 \         # Or specify width and height separately
  --max-width 160 \        # Max grid width in tiles (default: 80)
  --recursive              # Search subdirectories for tiles
```

### Controlling Tile Distribution

#### Unfair Mode (Fast, Default)
Best-fit greedy algorithm - tiles are matched based on color similarity:

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --out result.png
```

#### Fair Mode (Balanced)
Uses the Hungarian algorithm to ensure all tiles are used equally:

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --out result.png \
  --fair \
  --dup 6        # Use each tile ~6 times (duplication factor)
```

⚠️ **Note**: Fair mode is slow for large tile sets (O(n³) complexity). For balanced tile distribution without the overhead, use the `--freq-mul` option instead:

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --out result.png \
  --freq-mul 0.8    # Penalizes frequently-used tiles (0.0-1.0, higher = more balanced)
```

### Color Space and Distance Metrics

Choose the color space for matching:

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --colorspace lab        # Default: LAB (perceptually uniform)
  --colorspace hsv        # HSV (hue-saturation-value)
  --colorspace hsl        # HSL (hue-saturation-lightness)
  --colorspace bgr        # BGR (standard RGB order)
  --colorspace luv        # LUV (another perceptual model)
  --metric euclidean      # Default: Euclidean distance
  --metric cityblock      # Manhattan distance
  --metric chebyshev      # Chebyshev distance
  --metric cosine         # Cosine similarity
  --out result.png
```

### Tile Sizing and Positioning

Control how tiles are resized to match the grid:

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --resize-opt center     # Default: center crop (no stretching)
  --resize-opt stretch    # Stretch to fit grid cell
  --resize-opt fit        # Fit inside cell (letterboxing)
  --out result.png
```

Auto-rotate tiles to better match the destination:

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --auto-rotate 1         # Rotate 90° to match best
  --out result.png
```

### Blending with Original

Blend the mosaic with the original image for a hybrid effect:

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --blending alpha \
  --blending-level 0.3    # 0.0 = 100% tiles, 1.0 = 100% original
  --out result.png
```

Brightness blending:

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --blending brightness \
  --blending-level 0.5 \
  --out result.png
```

### Advanced Options

Enable dithering for better color approximation:

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --dither \
  --out result.png
```

Use transparency masking (for tiles with alpha channels):

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --transparent \
  --out result.png
```

Control parallel processing:

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --num-threads 4 \       # Use 4 threads (default: CPU count / 2)
  --out result.png
```

Memory limit for distance computation:

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --mem-limit 2048 \      # Limit to 2GB (default: 4GB)
  --out result.png
```

Suppress progress output:

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --quiet \
  --out result.png
```

### Generating Tile Maps and Statistics

Export information about which tiles were used where:

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --out result.png \
  --tile-info-out tiles.csv \        # CSV with tile usage stats
  --map interactive.html             # Interactive HTML visualization
```

This generates:
- `tiles.csv` - Tile information and statistics
- `interactive.html` - Interactive map showing tile placement
- `interactive.json` - Map data

## Examples

### High-Quality Portrait Mosaic

```bash
photomosaic \
  --path portrait_tiles/ \
  --dest-img face.jpg \
  --size 20 \
  --max-width 200 \
  --colorspace lab \
  --metric euclidean \
  --freq-mul 0.6 \
  --out portrait_mosaic.png
```

### Fast, Balanced Mosaic

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --size 50 \
  --freq-mul 0.8 \
  --out result.png
```

### Artistic Blend

```bash
photomosaic \
  --path artistic_tiles/ \
  --dest-img original.jpg \
  --size 30 \
  --blending alpha \
  --blending-level 0.4 \
  --dither \
  --out artistic_blend.png
```

### Fair Mode (for small tile sets)

```bash
photomosaic \
  --path my_photos/ \
  --dest-img target.jpg \
  --size 25 \
  --fair \
  --dup 6 \
  --out fair_mosaic.png
```

### Deterministic Output

```bash
photomosaic \
  --path tiles/ \
  --dest-img target.jpg \
  --deterministic \
  --out result.png
```

## How It Works

1. **Load Tiles**: Scans the specified directory for image files and resizes them to the specified tile size
2. **Analyze Destination**: Reads the target image and divides it into a grid
3. **Match Colors**: For each grid cell, computes the color distance to all available tiles using the selected color space and metric
4. **Select Tiles**: Chooses the best-matching tile for each cell (greedy by default, or optimal via Hungarian algorithm in fair mode)
5. **Assemble**: Composites all tiles together into the final mosaic
6. **Optional Blending**: Blends the result with the original if requested
7. **Save & Export**: Saves the final image and optional tile statistics/maps

## Performance Notes

- **Tile size impact**: Smaller tiles = more detail but slower processing
- **Fair mode**: O(n³) complexity; practical for < 200 tiles. Use `--freq-mul` instead for balanced distribution with many tiles
- **Threads**: Automatically uses half your CPU cores. Adjust with `--num-threads`
- **Memory**: Tile analysis can be memory-intensive for very large tile sets. Adjust with `--mem-limit`

## Architecture

The project is organized into modules:

- `main.rs` - CLI argument parsing and orchestration
- `tiles.rs` - Tile loading and preprocessing
- `mosaic.rs` - Core mosaic generation algorithm
- `colorspace.rs` - Color space conversions (LAB, HSV, HSL, LUV)
- `blend.rs` - Blending operations
- `html_map.rs` - Interactive map generation
- `cache.rs` - Distance computation caching
- `approx_lap.rs` - Approximate algorithms for large tile sets

## Dependencies

- `image` - Image loading and manipulation
- `clap` - Command-line argument parsing
- `rayon` - Parallel processing
- `indicatif` - Progress bars
- `walkdir` - Recursive directory traversal
- `palette` - Color space conversions
- `imagesize` - Image dimension detection
- `num_cpus` - CPU count detection
- `hungarian` - Hungarian algorithm for fair mode
- `serde_json` - JSON serialization for maps

## Troubleshooting

### "no tiles loaded"
- Check that your `--path` directory exists and contains image files
- Supported formats: PNG, JPG, BMP, GIF, TIFF, WebP
- Use `--recursive` if tiles are in subdirectories

### Fair mode takes too long
- Fair mode is O(n³) - use `--freq-mul` instead for balanced distribution
- Or reduce tile count, duplication factor, or grid size

### Out of memory
- Reduce `--mem-limit`
- Use smaller tiles
- Use fewer tiles (sample or reduce tile set size)

### Poor color matching
- Try different `--colorspace` (LAB usually works best)
- Adjust `--size` for more/less detail
- Use `--dither` for better color approximation

## License

Check the repository for license information.
