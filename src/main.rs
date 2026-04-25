use clap::{Parser, ValueEnum};
use std::path::PathBuf;

mod tiles;
mod colorspace;
mod mosaic;
mod blend;
mod html_map;
mod cache;

use tiles::TileSet;
use mosaic::Mosaic;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Colorspace {
    #[value(name = "bgr")]
    Bgr,
    #[value(name = "lab")]
    Lab,
    #[value(name = "hsv")]
    Hsv,
    #[value(name = "hsl")]
    Hsl,
    #[value(name = "luv")]
    Luv,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Metric {
    #[value(name = "euclidean")]
    Euclidean,
    #[value(name = "cityblock")]
    Cityblock,
    #[value(name = "chebyshev")]
    Chebyshev,
    #[value(name = "cosine")]
    Cosine,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ResizeOpt {
    #[value(name = "center")]
    Center,
    #[value(name = "stretch")]
    Stretch,
    #[value(name = "fit")]
    Fit,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Blending {
    #[value(name = "alpha")]
    Alpha,
    #[value(name = "brightness")]
    Brightness,
}

#[derive(Parser, Debug)]
#[command(name = "photomosaic")]
#[command(about = "Unfair photomosaic maker")]
struct Args {
    /// Path to the tiles directory
    #[arg(long)]
    path: PathBuf,

    /// Path to the destination image
    #[arg(long)]
    dest_img: PathBuf,

    /// Output image path
    #[arg(long, default_value = "result.png")]
    out: PathBuf,

    /// Tile size (1 or 2 values; 1 = infer height)
    #[arg(long, default_value = "50", num_args = 1..=2)]
    size: Vec<u32>,

    /// Recursive directory walk
    #[arg(long, default_value_t = false)]
    recursive: bool,

    /// Maximum width of the grid
    #[arg(long, default_value_t = 80)]
    max_width: u32,

    /// Frequency multiplier for tile fairness (rank-based)
    #[arg(long, default_value_t = 0.0)]
    freq_mul: f32,

    /// Deterministic mode (no randomization)
    #[arg(long, default_value_t = false)]
    deterministic: bool,

    /// Enable dithering
    #[arg(long, default_value_t = false)]
    dither: bool,

    /// Color space for distance computation
    #[arg(long, value_enum, default_value = "lab")]
    colorspace: Colorspace,

    /// Distance metric
    #[arg(long, value_enum, default_value = "euclidean")]
    metric: Metric,

    /// Tile resize option
    #[arg(long, value_enum, default_value = "center")]
    resize_opt: ResizeOpt,

    /// Auto-rotate tiles to best match size
    #[arg(long, default_value_t = 0)]
    auto_rotate: i8,

    /// Blending type
    #[arg(long, value_enum, default_value = "alpha")]
    blending: Blending,

    /// Blending level (0.0 = no blend, 1.0 = full)
    #[arg(long, default_value_t = 0.0)]
    blending_level: f32,

    /// Enable transparency masking
    #[arg(long, default_value_t = false)]
    transparent: bool,

    /// Fair tile usage (each tile used equally)
    #[arg(long, default_value_t = false)]
    fair: bool,

    /// Tile duplication factor (for fair mode)
    #[arg(long, default_value_t = 1.0)]
    dup: f32,

    /// Number of threads for parallel operations
    #[arg(long)]
    num_threads: Option<usize>,

    /// Suppress progress output
    #[arg(long, default_value_t = false)]
    quiet: bool,

    /// Path to save tile info CSV
    #[arg(long)]
    tile_info_out: Option<PathBuf>,

    /// Path to save interactive HTML tile map
    #[arg(long)]
    map: Option<PathBuf>,

    /// Approximate memory limit in MB for distance computation
    #[arg(long, default_value_t = 4096)]
    mem_limit: usize,
}

fn main() {
    let args = Args::parse();

    // Validate arguments
    if args.size.is_empty() || args.size.len() > 2 {
        eprintln!("Error: --size must be 1 or 2 values");
        std::process::exit(1);
    }

    if args.freq_mul < 0.0 {
        eprintln!("Error: --freq_mul must be non-negative");
        std::process::exit(1);
    }

    if args.blending_level < 0.0 || args.blending_level > 1.0 {
        eprintln!("Error: --blending_level must be between 0.0 and 1.0");
        std::process::exit(1);
    }

    if args.auto_rotate != -1 && args.auto_rotate != 0 && args.auto_rotate != 1 {
        eprintln!("Error: --auto_rotate must be -1, 0, or 1");
        std::process::exit(1);
    }

    if args.dither && args.deterministic == false && args.freq_mul > 0.0 {
        // This is allowed but note dithering with freq_mul > 0 is complex
    }

    if args.transparent && args.dither {
        eprintln!("Warning: dithering is not supported with transparency masking; dithering will be disabled");
    }

    // Set up rayon thread pool
    let num_threads = args.num_threads.unwrap_or_else(|| {
        let cpus = num_cpus::get();
        cpus.saturating_div(2).max(1)
    });
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .ok();

    if !args.quiet {
        println!("Loading tiles from: {}", args.path.display());
    }

    // Load tiles
    let tiles = TileSet::load(
        &args.path,
        args.recursive,
        args.size[0],
        if args.size.len() > 1 { Some(args.size[1]) } else { None },
        args.resize_opt,
        args.auto_rotate,
        !args.quiet,
    );

    if tiles.is_empty() {
        eprintln!("Error: no tiles loaded");
        std::process::exit(1);
    }

    if !args.quiet {
        println!("Loaded {} tiles", tiles.len());
    }

    // Validate fair mode options
    if args.fair && args.dup <= 0.0 {
        eprintln!("Error: --dup must be positive");
        std::process::exit(1);
    }

    if args.fair && args.freq_mul > 0.0 {
        eprintln!("Warning: --freq_mul is ignored in fair mode");
    }

    // Check if fair mode problem size is too large
    if args.fair {
        // Estimate grid dimensions
        let dest_img = image::open(&args.dest_img)
            .unwrap_or_else(|_| {
                eprintln!("Error: could not read destination image");
                std::process::exit(1);
            });
        let (dest_w, dest_h) = (dest_img.width(), dest_img.height());

        let grid_w = args.max_width;
        let avg_tile_aspect = tiles.tiles.iter()
            .map(|t| t.width() as f32 / t.height().max(1) as f32)
            .sum::<f32>() / tiles.tiles.len() as f32;
        let grid_h = ((dest_h as f32) * (grid_w as f32 * avg_tile_aspect) / (dest_w as f32)) as u32;
        let total_blocks = (grid_w * grid_h) as usize;

        let num_tiles = (tiles.len() as f32 * args.dup).round() as usize;
        let problem_size = total_blocks * num_tiles;

        if problem_size > 5_000_000 {
            eprintln!("\n⚠️  WARNING: Fair mode problem is too large!");
            eprintln!("   Problem size: {} blocks × {} tiles = {} total",
                     total_blocks, num_tiles, problem_size);
            eprintln!("\n   The Hungarian algorithm will take a very long time (hours/days).");
            eprintln!("   Estimated solving time: {}",
                     match problem_size {
                         n if n > 100_000_000 => "12+ hours".to_string(),
                         n if n > 50_000_000 => "2-12 hours".to_string(),
                         _ => "30+ minutes".to_string(),
                     });
            eprintln!("\n   Recommended solutions:");
            eprintln!("   1. Use unfair mode (fast): remove --fair flag");
            eprintln!("   2. Reduce grid size: --max-width {} (instead of {})",
                     (args.max_width / 2).max(32), args.max_width);
            eprintln!("   3. Reduce duplication: --dup {} (instead of {})",
                     (args.dup / 10.0).max(0.01), args.dup);
            eprintln!("   4. Sample tiles: use only {} random tiles instead of {}",
                     (tiles.len() / 10).max(50), tiles.len());
            eprintln!("\n   Continue anyway? (Ctrl+C to cancel, Enter to proceed)\n");
            let _ = std::io::stdin().read_line(&mut String::new());
        } else if !args.quiet {
            eprintln!("ℹ️  Fair mode problem size: {} blocks × {} tiles ({}M operations)",
                     total_blocks, num_tiles, problem_size / 1_000_000);
        }
    }

    // Create and run mosaic
    let mut mosaic = Mosaic::new(
        tiles,
        args.dest_img.clone(),
        args.max_width,
        args.colorspace,
        args.metric,
        args.freq_mul,
        args.deterministic && !args.dither,
        args.dither && !args.transparent,
        args.transparent,
        args.fair,
        args.dup,
        !args.quiet,
        args.mem_limit,
    );

    match mosaic.generate() {
        Ok((result_img, tile_info)) => {
            // Apply blending
            let final_img = match args.blending {
                Blending::Alpha => {
                    blend::alpha_blend(&result_img, &mosaic.dest_img, 1.0 - args.blending_level)
                }
                Blending::Brightness => {
                    blend::brightness_blend(&result_img, &mosaic.dest_img, 1.0 - args.blending_level)
                }
            };

            // Save output
            if !args.quiet {
                println!("Saving to {}", args.out.display());
            }
            if let Err(e) = image::DynamicImage::ImageRgba8(final_img).save(&args.out) {
                eprintln!("Error saving image: {}", e);
                std::process::exit(1);
            }

            // Save tile info if requested
            if let Some(tile_info_path) = args.tile_info_out {
                if let Err(e) = std::fs::write(&tile_info_path, &tile_info) {
                    eprintln!("Error saving tile info: {}", e);
                    std::process::exit(1);
                }
                if !args.quiet {
                    println!("Tile info saved to {}", tile_info_path.display());
                }
            }

            // Save interactive HTML map if requested
            if let Some(map_path) = args.map {
                if let Err(e) = html_map::generate(&tile_info, &args.out, &map_path) {
                    eprintln!("Error saving map: {}", e);
                    std::process::exit(1);
                }
                if !args.quiet {
                    println!("Interactive map saved to {}", map_path.display());
                    let json_path = map_path.with_extension("json");
                    println!("Map data saved to {}", json_path.display());
                }
            }

            if !args.quiet {
                println!("Done");
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
