use image::{RgbaImage, Rgba};
use palette::{IntoColor, Srgb, Hsl};

pub fn alpha_blend(mosaic: &RgbaImage, dest: &RgbaImage, alpha: f32) -> RgbaImage {
    if alpha >= 1.0 {
        mosaic.clone()
    } else if alpha <= 0.0 {
        dest.clone()
    } else {
        let (mosaic_w, mosaic_h) = mosaic.dimensions();

        // Resize dest to match mosaic size
        let dest_resized = image::imageops::resize(
            dest,
            mosaic_w,
            mosaic_h,
            image::imageops::FilterType::Lanczos3,
        );

        let mut result = RgbaImage::new(mosaic_w, mosaic_h);

        for (x, y, mosaic_pixel) in mosaic.enumerate_pixels() {
            let dest_pixel = dest_resized.get_pixel(x, y);

            let mr = (mosaic_pixel[0] as f32) * alpha;
            let mg = (mosaic_pixel[1] as f32) * alpha;
            let mb = (mosaic_pixel[2] as f32) * alpha;

            let dr = (dest_pixel[0] as f32) * (1.0 - alpha);
            let dg = (dest_pixel[1] as f32) * (1.0 - alpha);
            let db = (dest_pixel[2] as f32) * (1.0 - alpha);

            let r = ((mr + dr) as u8).clamp(0, 255);
            let g = ((mg + dg) as u8).clamp(0, 255);
            let b = ((mb + db) as u8).clamp(0, 255);

            result.put_pixel(x, y, Rgba([r, g, b, 255]));
        }

        result
    }
}

pub fn brightness_blend(mosaic: &RgbaImage, dest: &RgbaImage, alpha: f32) -> RgbaImage {
    if alpha >= 1.0 {
        mosaic.clone()
    } else if alpha <= 0.0 {
        dest.clone()
    } else {
        let (mosaic_w, mosaic_h) = mosaic.dimensions();

        // Resize dest to match mosaic size
        let dest_resized = image::imageops::resize(
            dest,
            mosaic_w,
            mosaic_h,
            image::imageops::FilterType::Lanczos3,
        );

        let mut result = RgbaImage::new(mosaic_w, mosaic_h);

        for (x, y, mosaic_pixel) in mosaic.enumerate_pixels() {
            let dest_pixel = dest_resized.get_pixel(x, y);

            // Convert to HLS for lightness blending
            let mosaic_rgb = Srgb::new(
                mosaic_pixel[0] as f32 / 255.0,
                mosaic_pixel[1] as f32 / 255.0,
                mosaic_pixel[2] as f32 / 255.0,
            );
            let dest_rgb = Srgb::new(
                dest_pixel[0] as f32 / 255.0,
                dest_pixel[1] as f32 / 255.0,
                dest_pixel[2] as f32 / 255.0,
            );

            let mut mosaic_hls: Hsl = mosaic_rgb.into_color();
            let dest_hls: Hsl = dest_rgb.into_color();

            // Blend lightness channel
            let blended_lightness = mosaic_hls.lightness * alpha + dest_hls.lightness * (1.0 - alpha);
            mosaic_hls.lightness = blended_lightness;

            // Convert back to RGB
            let result_rgb: Srgb = mosaic_hls.into_color();

            let r = ((result_rgb.red * 255.0) as u8).clamp(0, 255);
            let g = ((result_rgb.green * 255.0) as u8).clamp(0, 255);
            let b = ((result_rgb.blue * 255.0) as u8).clamp(0, 255);

            result.put_pixel(x, y, Rgba([r, g, b, 255]));
        }

        result
    }
}
