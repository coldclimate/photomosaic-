use image::RgbaImage;
use palette::{IntoColor, Lab, Luv, Srgb, Hsv, Hsl};
use crate::Colorspace;

pub enum ColorVector {
    Bgr(Vec<f32>),
    Lab(Vec<f32>),
    Hsv(Vec<f32>),
    Hsl(Vec<f32>),
    Luv(Vec<f32>),
}

impl ColorVector {
    pub fn as_slice(&self) -> &[f32] {
        match self {
            ColorVector::Bgr(v) => v,
            ColorVector::Lab(v) => v,
            ColorVector::Hsv(v) => v,
            ColorVector::Hsl(v) => v,
            ColorVector::Luv(v) => v,
        }
    }
}

pub fn extract_features(img: &RgbaImage, colorspace: Colorspace) -> ColorVector {
    let pixels: Vec<f32> = img
        .pixels()
        .flat_map(|p| {
            let r = p[0] as f32 / 255.0;
            let g = p[1] as f32 / 255.0;
            let b = p[2] as f32 / 255.0;

            match colorspace {
                Colorspace::Bgr => {
                    vec![b, g, r]
                }
                Colorspace::Lab => {
                    let rgb = Srgb::new(r, g, b);
                    let lab: Lab = rgb.into_color();
                    vec![lab.l, lab.a, lab.b]
                }
                Colorspace::Hsv => {
                    let rgb = Srgb::new(r, g, b);
                    let hsv: Hsv = rgb.into_color();
                    vec![
                        hsv.hue.into_positive_degrees() / 360.0,
                        hsv.saturation,
                        hsv.value,
                    ]
                }
                Colorspace::Hsl => {
                    let rgb = Srgb::new(r, g, b);
                    let hsl: Hsl = rgb.into_color();
                    vec![
                        hsl.hue.into_positive_degrees() / 360.0,
                        hsl.saturation,
                        hsl.lightness,
                    ]
                }
                Colorspace::Luv => {
                    let rgb = Srgb::new(r, g, b);
                    let luv: Luv = rgb.into_color();
                    vec![luv.l, luv.u, luv.v]
                }
            }
        })
        .collect();

    match colorspace {
        Colorspace::Bgr => ColorVector::Bgr(pixels),
        Colorspace::Lab => ColorVector::Lab(pixels),
        Colorspace::Hsv => ColorVector::Hsv(pixels),
        Colorspace::Hsl => ColorVector::Hsl(pixels),
        Colorspace::Luv => ColorVector::Luv(pixels),
    }
}

pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    let sum: f32 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum();
    sum // skip sqrt for monotonicity
}

pub fn cityblock_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .sum()
}

pub fn chebyshev_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f32::max)
}

pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (x, y) in a.iter().zip(b.iter()) {
        dot_product += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    norm_a = norm_a.sqrt();
    norm_b = norm_b.sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        1.0 // max distance if either vector is zero
    } else {
        1.0 - (dot_product / (norm_a * norm_b))
    }
}

pub fn compute_distance(
    a: &[f32],
    b: &[f32],
    metric: crate::Metric,
) -> f32 {
    use crate::Metric;
    match metric {
        Metric::Euclidean => euclidean_distance(a, b),
        Metric::Cityblock => cityblock_distance(a, b),
        Metric::Chebyshev => chebyshev_distance(a, b),
        Metric::Cosine => cosine_distance(a, b),
    }
}
