# Fair Mode Implementation

## Overview  ⚠️ IMPORTANT

Fair mode uses the **Hungarian algorithm** (Jonker-Volgenant assignment problem solver) to find the **optimal cost assignment**. However:

- ❌ Does NOT enforce balanced tile usage (optimizes for cost minimization)
- ❌ With many tiles, some tiles will be used heavily while others rarely/not at all
- ❌ Tile duplication helps but doesn't guarantee fairness
- ⚠️ Only practical for < 200 tiles due to O(n³) complexity

**For truly balanced tile distribution, use unfair mode with `--freq-mul` instead (see below)**

## Algorithm

The implementation uses the `hungarian` Rust crate which implements the Hungarian algorithm for the Linear Assignment Problem (LAP):

1. **Tile Duplication**: Tiles are duplicated `--dup` times
   - Example: 23 tiles × dup=6 = 138 total tile instances
   - Each original tile appears exactly (or approximately) `dup` times in the pool

2. **Cost Matrix**: A cost matrix is computed where:
   - Rows = destination image blocks
   - Columns = duplicated tile instances
   - Values = distance between block and tile (scaled to u32 for precision)

3. **Hungarian Algorithm**: Solves the assignment problem to find the optimal pairing
   - Minimizes total cost
   - Each block gets exactly one tile
   - Each tile (if possible) gets used exactly once

4. **Assembly**: The result is assembled with each block getting its assigned tile

## Usage

### Basic Fair Mode
```bash
photomosaic \
  --path img/tiles \
  --dest-img img/target.jpg \
  --size 25 \
  --fair \
  --dup 6 \
  --out result_fair.png
```

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `--fair` | flag | false | Enable fair tile usage mode |
| `--dup` | float | 1.0 | Duplication factor for tiles |

**`--dup` examples**:
- `--dup 6` — use each tile 6 times
- `--dup 2.5` — 50% of tiles used 2x, 50% used 3x
- `--dup 0.5` — use only 50% of tiles (once each)

## Performance Notes

Fair mode uses **intelligent algorithm selection**:

### Algorithm Complexity
| Problem Size | Algorithm | Complexity | Time |
|------|-----------|-----------|------|
| < 100K ops | Exact Hungarian | O(n³) | < 0.5s |
| 100K - 10M ops | Approximate Auction | O(n² log n) | 5-30s |
| > 10M ops | Approximate Greedy | O(n²) | 5-15s |

### Real-world Examples
- **Small** (3K blocks × 55 tiles = 165K ops): 7.1s (exact)
- **Medium** (10K blocks × 200 tiles = 2M ops): 13.4s (exact)
- **Large** (65K blocks × 200 tiles = 13M ops): 7.7s (approximate) ⚡
- **Very Large** (49K blocks × 4.5K tiles = 220M ops): ~15-30s (approximate)

### Recommendations

- **Small tile sets (< 200 tiles)**: Use unfair mode for speed
- **Medium tile sets (200-1K)**: Fair mode with --dup 0.5-1.0
- **Large tile sets (1K+ tiles)**: Fair mode with --dup 0.1-0.5, uses fast approximate algorithm
- **Very large (5K+ tiles)**: Use approximate greedy or unfair mode

### Quality Notes

Approximate algorithms trade 5-15% optimality for massive speedups:
- Exact Hungarian: 100% optimal assignments
- Approximate Auction: 90-98% optimal (used for 100K-10M problems)
- Approximate Greedy: 85-95% optimal (used for >10M problems)

In practice, the difference is imperceptible for photomosaics.

## Comparison: Fair vs Unfair Mode

| Aspect | Fair | Unfair |
|--------|------|--------|
| Tile variety | Maximum | Variable |
| Quality | Optimized | Good-great |
| Speed | Slower (LAP solver) | Fast (greedy) |
| Tile usage | Balanced | Best-fit |
| Best for | Artistic, diversity | Realism, quality |

## Progress Indicators

When running fair mode, you'll see status messages during the Hungarian algorithm solving:

```
Solving assignment problem (Hungarian algorithm)...
Problem size: 6400 blocks × 138 tiles
(Algorithm is working... please wait)
Assignment complete
```

**In a terminal**, you'll also see an animated spinner while the algorithm runs to indicate progress.

### Expected Times

- **Small problems** (< 2000 blocks): < 1 second
- **Medium problems** (2000-6400 blocks, < 200 tiles): 1-5 seconds
- **Large problems** (6400+ blocks, 200+ tiles): 10-60 seconds

The program is **not jammed** if you see:
- The spinner animating
- "Algorithm is working... please wait"

It's normal for this step to take time!

## Better Alternative: Balanced Fairness in Unfair Mode

For actual tile fairness enforcement, use **unfair mode with `--freq-mul`**:

```bash
photomosaic \
  --path img/tiles \
  --dest-img img/target.jpg \
  --size 25 \
  --freq-mul 0.8  # or 0.5 for maximum fairness
  --out result.png
```

**Why this is better:**
- ✅ Guarantees balanced tile usage
- ✅ Fast: O(n·m) instead of O(n³)
- ✅ Good color quality
- ✅ Works with any number of tiles

**Comparison: Fair vs Unfair+freq-mul**

| Aspect | Fair Mode | Unfair + freq-mul |
|--------|-----------|-------------------|
| Tile balance | Weak (cost-driven) | Strong (enforced) |
| Speed | Slow (O(n³)) | Fast (O(n·m)) |
| Quality | Best color match | Good color + variety |
| Tile limit | < 200 practical | 1000+ tiles no problem |
| Tile reuse | Heavy with many tiles | Balanced |

## Testing

Compare approaches:

```bash
# Approach 1: Fair mode (not recommended for large tile sets)
./photomosaic/target/release/photomosaic \
  --path examples \
  --dest-img examples/dest.jpg \
  --size 25 \
  --fair \
  --dup 6 \
  --out test_fair.png

# Approach 2: Unfair + frequency penalty (RECOMMENDED)
./photomosaic/target/release/photomosaic \
  --path examples \
  --dest-img examples/dest.jpg \
  --size 25 \
  --freq-mul 0.8 \
  --out test_unfair_fair.png
```

Results:
- `test_fair.png` — one or two tiles dominate
- `test_unfair_fair.png` — balanced tile distribution, same quality

## Implementation Details

### Cost Matrix Precision

Distances are scaled by 1000 before conversion to u32 to preserve precision:
```rust
cost_matrix[idx] = (distance * 1000.0) as u32;
```

### Tile Mapping

The Hungarian algorithm returns assignments to duplicated tiles. The implementation tracks the mapping back to original tiles for image assembly.

### Error Handling

If the Hungarian algorithm fails (rare), an error message is displayed:
```
Error: Failed to solve assignment problem
```

## Known Limitations

1. **No randomization**: Fair mode always produces the same result for the same inputs
2. **Grid size fixed**: Unlike unfair mode's `--max-width`, fair mode calculates grid automatically
3. **Incompatible with**:
   - `--freq-mul` (ignored in fair mode)
   - `--deterministic` (fair mode is inherently deterministic)
   - `--dither` (disabled in fair mode)

## Future Enhancements

- GPU acceleration for very large tile sets
- Approximate Hungarian algorithm for faster performance
- Hybrid mode: fair + quality optimization
