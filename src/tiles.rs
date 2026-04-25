use image::{Rgba, RgbaImage};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use indicatif::ProgressBar;
use crate::ResizeOpt;

pub struct TileSet {
    pub tiles: Vec<RgbaImage>,
    pub paths: Vec<PathBuf>,
}

impl TileSet {
    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    pub fn load(
        path: &Path,
        recursive: bool,
        tile_width: u32,
        tile_height_opt: Option<u32>,
        resize_opt: ResizeOpt,
        auto_rotate: i8,
        show_progress: bool,
    ) -> TileSet {
        // Collect image paths
        let mut image_paths = Vec::new();
        let mut walker = WalkDir::new(path);
        if !recursive {
            walker = walker.max_depth(1);
        }

        for entry in walker
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let entry_path = entry.path();
            if is_supported_image(entry_path) {
                image_paths.push(entry_path.to_path_buf());
            }
        }

        if show_progress {
            println!("Scanning files...");
            println!("Found {} image files", image_paths.len());
        }

        // If tile_height is not specified, infer from aspect ratios
        let tile_height = if let Some(h) = tile_height_opt {
            h
        } else {
            // Infer height from aspect ratios of images
            infer_tile_height(&image_paths, tile_width, show_progress)
        };

        // Load and resize tiles in parallel
        let pb = if show_progress {
            Some(ProgressBar::new(image_paths.len() as u64))
        } else {
            None
        };

        let tiles_data: Vec<_> = image_paths
            .par_iter()
            .map(|img_path| {
                let result = load_and_resize_tile(
                    img_path,
                    tile_width,
                    tile_height,
                    resize_opt,
                    auto_rotate,
                );
                if let Some(ref pb) = &pb {
                    pb.inc(1);
                }
                result
            })
            .filter_map(|r| r.ok())
            .collect();

        if let Some(pb) = pb {
            pb.finish();
        }

        let (tiles, paths): (Vec<_>, Vec<_>) = tiles_data.into_iter().unzip();

        if show_progress {
            println!(
                "Loaded {} tiles ({} files failed to load)",
                tiles.len(),
                image_paths.len() - tiles.len()
            );
        }

        TileSet { tiles, paths }
    }
}

fn is_supported_image(path: &Path) -> bool {
    match path.extension() {
        Some(ext) => matches!(
            ext.to_str().unwrap_or("").to_lowercase().as_str(),
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp"
        ),
        None => false,
    }
}

fn infer_tile_height(paths: &[PathBuf], tile_width: u32, show_progress: bool) -> u32 {
    if show_progress {
        println!("Inferring tile height from image aspect ratios...");
    }

    let mut aspects = Vec::new();

    for path in paths {
        if let Ok(size) = imagesize::size(path) {
            let aspect = size.width as f32 / size.height as f32;
            aspects.push(aspect);
        }
    }

    // Find the median aspect ratio
    if aspects.is_empty() {
        return tile_width; // Default to square
    }

    aspects.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let most_common_aspect = aspects[aspects.len() / 2];

    let inferred_height = (tile_width as f32 / most_common_aspect).round() as u32;
    if show_progress {
        println!(
            "Inferred tile size: {}x{}",
            tile_width, inferred_height
        );
    }
    inferred_height
}

fn load_and_resize_tile(
    path: &Path,
    target_width: u32,
    target_height: u32,
    resize_opt: ResizeOpt,
    auto_rotate: i8,
) -> Result<(RgbaImage, PathBuf), String> {
    let mut img = image::open(path)
        .map_err(|e| format!("Failed to load {}: {}", path.display(), e))?
        .to_rgba8();

    let (width, height) = img.dimensions();
    let target_aspect = target_width as f32 / target_height as f32;
    let image_aspect = width as f32 / height as f32;

    // Auto-rotate if needed
    if auto_rotate != 0 {
        let should_rotate = if auto_rotate > 0 {
            // Rotate counterclockwise if image aspect is closer to portrait after rotation
            (height as f32 / width as f32 - target_aspect).abs()
                < (image_aspect - target_aspect).abs()
        } else {
            // Rotate clockwise
            (height as f32 / width as f32 - target_aspect).abs()
                < (image_aspect - target_aspect).abs()
        };

        if should_rotate {
            img = if auto_rotate > 0 {
                image::imageops::rotate90(&img)
            } else {
                image::imageops::rotate270(&img)
            };
        }
    }

    let result = match resize_opt {
        ResizeOpt::Center => resize_center_crop(&img, target_width, target_height),
        ResizeOpt::Stretch => resize_stretch(&img, target_width, target_height),
        ResizeOpt::Fit => resize_fit(&img, target_width, target_height),
    };

    Ok((result, path.to_path_buf()))
}

fn resize_center_crop(img: &RgbaImage, target_width: u32, target_height: u32) -> RgbaImage {
    let (width, height) = img.dimensions();
    let target_aspect = target_width as f32 / target_height as f32;
    let image_aspect = width as f32 / height as f32;

    let (crop_width, crop_height) = if image_aspect > target_aspect {
        // Image is wider, crop width
        ((height as f32 * target_aspect).round() as u32, height)
    } else {
        // Image is taller, crop height
        (width, (width as f32 / target_aspect).round() as u32)
    };

    let x = (width.saturating_sub(crop_width)) / 2;
    let y = (height.saturating_sub(crop_height)) / 2;

    let cropped = image::imageops::crop_imm(img, x, y, crop_width, crop_height).to_image();
    image::imageops::resize(&cropped, target_width, target_height, image::imageops::FilterType::Lanczos3)
}

fn resize_stretch(img: &RgbaImage, target_width: u32, target_height: u32) -> RgbaImage {
    image::imageops::resize(img, target_width, target_height, image::imageops::FilterType::Lanczos3)
}

fn resize_fit(img: &RgbaImage, target_width: u32, target_height: u32) -> RgbaImage {
    let (width, height) = img.dimensions();
    let target_aspect = target_width as f32 / target_height as f32;
    let image_aspect = width as f32 / height as f32;

    let (new_width, new_height) = if image_aspect > target_aspect {
        // Image is wider, fit to width
        (target_width, (target_width as f32 / image_aspect).round() as u32)
    } else {
        // Image is taller, fit to height
        ((target_height as f32 * image_aspect).round() as u32, target_height)
    };

    let resized = image::imageops::resize(
        img,
        new_width,
        new_height,
        image::imageops::FilterType::Lanczos3,
    );

    // Pad with white background
    let mut result = RgbaImage::from_pixel(target_width, target_height, Rgba([255, 255, 255, 255]));
    let x = (target_width.saturating_sub(new_width)) / 2;
    let y = (target_height.saturating_sub(new_height)) / 2;

    for (px, py, pixel) in resized.enumerate_pixels() {
        if px + x < target_width && py + y < target_height {
            result.put_pixel(px + x, py + y, *pixel);
        }
    }

    result
}
