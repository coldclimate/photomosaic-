use crate::colorspace::{extract_features, compute_distance};
use crate::tiles::TileSet;
use crate::{Colorspace, Metric};
use crate::cache::{TileCache, compute_file_id};
use image::{RgbaImage, Rgba};
use std::path::PathBuf;
use std::io::Write;
use indicatif::ProgressBar;
use hungarian::minimize;
use rayon::prelude::*;

#[allow(dead_code)]
pub struct Mosaic {
    tiles: TileSet,
    pub dest_img: RgbaImage,
    grid_w: u32,
    grid_h: u32,
    tile_w: u32,
    tile_h: u32,
    block_w: u32,
    block_h: u32,
    max_width: u32,
    colorspace: Colorspace,
    metric: Metric,
    freq_mul: f32,
    deterministic: bool,
    dither: bool,
    transparent: bool,
    fair: bool,
    dup: f32,
    show_progress: bool,
    mem_limit: usize,
    tile_features: Vec<Vec<f32>>,
}

impl Mosaic {
    pub fn new(
        tiles: TileSet,
        dest_path: PathBuf,
        max_width: u32,
        colorspace: Colorspace,
        metric: Metric,
        freq_mul: f32,
        deterministic: bool,
        dither: bool,
        transparent: bool,
        fair: bool,
        dup: f32,
        show_progress: bool,
        mem_limit: usize,
    ) -> Self {
        let dest_img_dyn = image::open(&dest_path).expect("Failed to open destination image");
        let dest_img = dest_img_dyn.to_rgba8();
        let (dest_w, dest_h) = dest_img.dimensions();

        // Get tile dimensions
        let (tile_w, tile_h) = if let Some(first_tile) = tiles.tiles.first() {
            first_tile.dimensions()
        } else {
            panic!("No tiles loaded");
        };

        // Calculate grid size
        let grid_w = max_width;
        let grid_h = ((dest_h as f32 * (grid_w as f32 * tile_w as f32 / dest_w as f32)) / tile_h as f32)
            .round() as u32;

        let total_blocks = (grid_w * grid_h) as usize;

        if show_progress {
            println!("Grid size: {}x{}", grid_w, grid_h);
            println!("Tile size: {}x{}", tile_w, tile_h);
            let avg_uses = total_blocks as f32 / tiles.len() as f32;
            println!("Total blocks: {}", total_blocks);
            println!("Tiles available: {}", tiles.len());
            println!("Average uses per tile: {:.1}", avg_uses);
        }

        // Calculate block size
        let block_w = (dest_w + grid_w - 1) / grid_w;
        let block_h = (dest_h + grid_h - 1) / grid_h;

        if show_progress {
            println!("Block size: {}x{}", block_w, block_h);
        }

        // Load or create tile feature cache (use first tile's directory)
        let tile_dir = if !tiles.paths.is_empty() {
            tiles.paths[0]
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::path::PathBuf::from("."))
        } else {
            std::path::PathBuf::from(".")
        };
        let mut cache = TileCache::new(&tile_dir);
        let colorspace_name = format!("{:?}", colorspace).to_lowercase();

        // Extract tile features (with caching)
        let mut tile_features = Vec::new();
        let pb = if show_progress {
            Some(ProgressBar::new(tiles.len() as u64))
        } else {
            None
        };

        for (tile_idx, tile) in tiles.tiles.iter().enumerate() {
            let default_name = format!("tile_{}", tile_idx);
            let filename = tiles.paths
                .get(tile_idx)
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or(default_name.as_str());

            // Try to get features from cache
            let features = match compute_file_id(&tiles.paths[tile_idx]) {
                Ok(file_id) => {
                    match cache.get_features(filename, &file_id, &colorspace_name) {
                        Some(cached_features) => cached_features,
                        None => {
                            // Compute and cache
                            let feat = extract_features(tile, colorspace).as_slice().to_vec();
                            cache.set_features(filename, &file_id, &colorspace_name, &feat);
                            feat
                        }
                    }
                }
                Err(_) => {
                    // If we can't compute file ID, just compute features
                    extract_features(tile, colorspace).as_slice().to_vec()
                }
            };

            tile_features.push(features);
            if let Some(ref pb) = pb {
                pb.inc(1);
            }
        }

        if let Some(pb) = pb {
            pb.finish();
        }

        // Save cache
        let _ = cache.save();

        let tile_features: Vec<Vec<f32>> = tile_features;

        Mosaic {
            tiles,
            dest_img,
            grid_w,
            grid_h,
            tile_w,
            tile_h,
            block_w,
            block_h,
            max_width,
            colorspace,
            metric,
            freq_mul,
            deterministic,
            dither,
            transparent,
            fair,
            dup,
            show_progress,
            mem_limit,
            tile_features,
        }
    }

    pub fn generate(&mut self) -> Result<(RgbaImage, String), String> {
        if self.fair {
            self.generate_fair()
        } else {
            self.generate_unfair()
        }
    }

    fn generate_unfair(&mut self) -> Result<(RgbaImage, String), String> {
        let mut assignments = vec![0usize; (self.grid_w * self.grid_h) as usize];
        let mut tile_usage = vec![0u32; self.tiles.len()];

        // Resize destination image to fit grid exactly
        let dest_resized = image::imageops::resize(
            &self.dest_img,
            self.grid_w * self.block_w,
            self.grid_h * self.block_h,
            image::imageops::FilterType::Lanczos3,
        );

        // Extract destination features
        let mut dest_features = Vec::with_capacity((self.grid_w * self.grid_h) as usize);
        for row in 0..self.grid_h {
            for col in 0..self.grid_w {
                let x = col * self.block_w;
                let y = row * self.block_h;
                let block = image::imageops::crop_imm(
                    &dest_resized,
                    x,
                    y,
                    self.block_w,
                    self.block_h,
                )
                .to_image();
                let features = extract_features(&block, self.colorspace);
                dest_features.push(features.as_slice().to_vec());
            }
        }

        let total_blocks = (self.grid_w * self.grid_h) as usize;

        if self.show_progress {
            println!("Computing assignments...");
        }

        let pb = if self.show_progress {
            Some(ProgressBar::new(total_blocks as u64))
        } else {
            None
        };

        // Quota-constrained greedy assignment (ensures fair tile usage)
        let quota = ((total_blocks + self.tiles.len() - 1) / self.tiles.len()) as u32;

        for (block_idx, dest_feat) in dest_features.iter().enumerate() {
            // Compute distances
            let distances: Vec<f32> = self
                .tile_features
                .iter()
                .map(|tile_feat| compute_distance(dest_feat, tile_feat, self.metric))
                .collect();

            // Find best tile that hasn't hit its quota
            let best_tile = distances
                .iter()
                .enumerate()
                .filter(|(tile_idx, _)| tile_usage[*tile_idx] < quota)
                .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(idx, _)| idx)
                .unwrap_or_else(|| {
                    // Fallback: if all tiles at quota, pick the one with lowest usage
                    tile_usage
                        .iter()
                        .enumerate()
                        .min_by_key(|a| a.1)
                        .unwrap()
                        .0
                });

            assignments[block_idx] = best_tile;
            tile_usage[best_tile] += 1;

            if let Some(ref pb) = pb {
                pb.inc(1);
            }
        }

        if let Some(pb) = pb {
            pb.finish();
        }

        // Assemble result
        let mut result = RgbaImage::from_pixel(
            self.grid_w * self.tile_w,
            self.grid_h * self.tile_h,
            Rgba([255, 255, 255, 255]),
        );

        if self.show_progress {
            println!("Assembling tiles...");
        }

        let pb = if self.show_progress {
            Some(ProgressBar::new(total_blocks as u64))
        } else {
            None
        };

        for (block_idx, &tile_idx) in assignments.iter().enumerate() {
            let row = (block_idx / self.grid_w as usize) as u32;
            let col = (block_idx % self.grid_w as usize) as u32;

            let x = col * self.tile_w;
            let y = row * self.tile_h;

            let tile = &self.tiles.tiles[tile_idx];
            for (px, py, pixel) in tile.enumerate_pixels() {
                if x + px < self.grid_w * self.tile_w && y + py < self.grid_h * self.tile_h {
                    result.put_pixel(x + px, y + py, *pixel);
                }
            }

            if let Some(ref pb) = pb {
                pb.inc(1);
            }
        }

        if let Some(pb) = pb {
            pb.finish();
        }

        // Build tile info
        let mut tile_info = format!("Grid dimension: {}x{}\n", self.grid_w, self.grid_h);
        for (block_idx, &tile_idx) in assignments.iter().enumerate() {
            if block_idx > 0 && block_idx % self.grid_w as usize == 0 {
                tile_info.push('\n');
            }
            if let Some(path) = self.tiles.paths.get(tile_idx) {
                tile_info.push_str(path.file_name().unwrap_or_default().to_str().unwrap_or("unknown"));
            } else {
                tile_info.push_str("unknown");
            }
            if block_idx % self.grid_w as usize != self.grid_w as usize - 1 {
                tile_info.push(',');
            }
        }

        Ok((result, tile_info))
    }

    fn generate_fair(&mut self) -> Result<(RgbaImage, String), String> {
        // Duplicate tiles
        let total_duplicates = (self.tiles.len() as f32 * self.dup).round() as usize;
        let mut duplicated_features = Vec::new();
        let mut duplicated_paths = Vec::new();

        if self.show_progress {
            println!("Duplicating tiles {} times (total: {})", self.dup, total_duplicates);
        }

        let tiles_per_dup = total_duplicates / self.tiles.len();
        let remainder = total_duplicates % self.tiles.len();

        for (idx, features) in self.tile_features.iter().enumerate() {
            let dup_count = tiles_per_dup + if idx < remainder { 1 } else { 0 };
            for _ in 0..dup_count {
                duplicated_features.push(features.clone());
                if let Some(path) = self.tiles.paths.get(idx) {
                    duplicated_paths.push(path.clone());
                }
            }
        }

        // Resize destination image to fit grid exactly
        let dest_resized = image::imageops::resize(
            &self.dest_img,
            self.grid_w * self.block_w,
            self.grid_h * self.block_h,
            image::imageops::FilterType::Lanczos3,
        );

        let total_blocks = (self.grid_w * self.grid_h) as usize;

        // Extract destination features in parallel
        if self.show_progress {
            println!("Extracting destination features...");
        }
        let dest_features: Vec<Vec<f32>> = (0..total_blocks)
            .into_par_iter()
            .map(|block_idx| {
                let row = (block_idx / self.grid_w as usize) as u32;
                let col = (block_idx % self.grid_w as usize) as u32;
                let x = col * self.block_w;
                let y = row * self.block_h;
                let block = image::imageops::crop_imm(
                    &dest_resized,
                    x,
                    y,
                    self.block_w,
                    self.block_h,
                )
                .to_image();
                let features = extract_features(&block, self.colorspace);
                features.as_slice().to_vec()
            })
            .collect();

        if self.show_progress {
            println!("Computing cost matrix ({} blocks × {} tiles)...",
                     total_blocks, duplicated_features.len());
        }

        // Build cost matrix in parallel: rows = dest blocks, cols = duplicated tiles
        let num_tiles = duplicated_features.len();
        let cost_matrix: Vec<u32> = (0..total_blocks)
            .into_par_iter()
            .flat_map(|block_idx| {
                let dest_feat = &dest_features[block_idx];
                (0..num_tiles)
                    .into_iter()
                    .map(|tile_idx| {
                        let tile_feat = &duplicated_features[tile_idx];
                        let dist = compute_distance(dest_feat, tile_feat, self.metric);
                        // Convert to integer cost (scale by 1000 to preserve precision)
                        (dist * 1000.0) as u32
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        // Solve assignment problem using Hungarian algorithm with progress indication
        // NOTE: Hungarian algorithm minimizes COST. For true fairness, use unfair mode.
        // Fair mode duplication + Hungarian tries to balance tiles, but cost still dominates.
        if self.show_progress {
            println!("Solving assignment problem (Hungarian algorithm)...");
            println!("Problem size: {} blocks × {} tiles", total_blocks, num_tiles);
            println!("(Algorithm is working... please wait)");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }

        let start_time = std::time::Instant::now();
        let assignment_raw = if self.show_progress {
            // Start algorithm solving in main thread, show progress bar
            let progress = ProgressBar::new(100);
            progress.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template("{msg} [{bar:40.cyan/blue}] {pos:>3}%")
                    .unwrap()
                    .progress_chars("=>-")
            );
            progress.set_message("Hungarian algorithm");

            // Spawn a thread to increment progress while algorithm runs
            let progress_clone = progress.clone();
            let progress_thread = std::thread::spawn(move || {
                for i in 0..99 {
                    progress_clone.set_position(i);
                    // Print progress dots every 500ms for visibility in all contexts
                    if i % 5 == 0 {
                        eprint!(".");
                        let _ = std::io::stderr().flush();
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            });

            let result = minimize(&cost_matrix, total_blocks, num_tiles);

            // Wait for progress thread and finish up
            progress_thread.join().ok();
            let elapsed = start_time.elapsed();
            eprintln!(); // newline after dots
            progress.set_position(100);
            progress.set_message(format!(
                "✓ Optimal assignment found ({:.2}s)",
                elapsed.as_secs_f64()
            ));
            progress.finish();

            result
        } else {
            minimize(&cost_matrix, total_blocks, num_tiles)
        };

        let assignment: Vec<usize> = assignment_raw
            .into_iter()
            .map(|opt| opt.unwrap_or(0))
            .collect();

        if self.show_progress {
            println!("Assignment complete");
        }

        // Assemble result
        let mut result = RgbaImage::from_pixel(
            self.grid_w * self.tile_w,
            self.grid_h * self.tile_h,
            Rgba([255, 255, 255, 255]),
        );

        if self.show_progress {
            println!("Assembling tiles...");
        }

        let pb = if self.show_progress {
            Some(ProgressBar::new(total_blocks as u64))
        } else {
            None
        };

        for (block_idx, &tile_idx) in assignment.iter().enumerate() {
            let row = (block_idx / self.grid_w as usize) as u32;
            let col = (block_idx % self.grid_w as usize) as u32;

            let x = col * self.tile_w;
            let y = row * self.tile_h;

            // Get the original tile (not the duplicated feature)
            let original_tile_idx = if tile_idx < self.tiles.len() {
                tile_idx
            } else {
                // Find which original tile this duplicate corresponds to
                let mut found = 0;
                let mut count = 0;
                for (orig_idx, _) in self.tiles.tiles.iter().enumerate() {
                    let dup_count = tiles_per_dup + if orig_idx < remainder { 1 } else { 0 };
                    count += dup_count;
                    if tile_idx < count {
                        found = orig_idx;
                        break;
                    }
                }
                found
            };

            let tile = &self.tiles.tiles[original_tile_idx];
            for (px, py, pixel) in tile.enumerate_pixels() {
                if x + px < self.grid_w * self.tile_w && y + py < self.grid_h * self.tile_h {
                    result.put_pixel(x + px, y + py, *pixel);
                }
            }

            if let Some(ref pb) = pb {
                pb.inc(1);
            }
        }

        if let Some(pb) = pb {
            pb.finish();
        }

        // Build tile info
        let mut tile_info = format!("Grid dimension: {}x{}\n", self.grid_w, self.grid_h);
        for (block_idx, &tile_idx) in assignment.iter().enumerate() {
            if block_idx > 0 && block_idx % self.grid_w as usize == 0 {
                tile_info.push('\n');
            }
            if let Some(path) = duplicated_paths.get(tile_idx) {
                tile_info.push_str(path.file_name().unwrap_or_default().to_str().unwrap_or("unknown"));
            } else {
                tile_info.push_str("unknown");
            }
            if block_idx % self.grid_w as usize != self.grid_w as usize - 1 {
                tile_info.push(',');
            }
        }

        Ok((result, tile_info))
    }
}
