use std::collections::HashSet;

use crate::{consts::*, fingerprint::Peak};
use image::{imageops, RgbImage};
use rayon::prelude::*;

/// Function to plot the spectrogram using HSL color space with high values having higher H value
pub fn plot_spectrogram(
    spec: &[f32],
    w: usize,
    h: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let buff: Vec<u8> = spec
        .into_par_iter()
        .enumerate()
        .map(|(i, v)| {
            let h = 1.0 - v;
            let (r, g, b) = hsl_to_rgb(h * 180.0, 1.0, 0.5);
            let y = i / w;
            let x = i % w;

            if GRID {
                if y % FOOTPRINT_SIZE == 0 {
                    return [0, 0, 255];
                }
                if x % FOOTPRINT_SIZE == 0 {
                    return [0, 0, 255];
                }
            }

            [r, g, b]
        })
        .flatten()
        .collect();

    imageops::rotate270(
        &RgbImage::from_raw(w as u32, h as u32, buff).expect("Failed to create image from data"),
    )
    .save("spectrogram.png")
    .expect("Failed to write image");

    Ok(())
}

pub fn plot_peaks(data: &[Peak], w: usize, h: usize) -> Result<(), Box<dyn std::error::Error>> {
    let set: HashSet<(usize, usize)> = data
        .iter()
        .map(|p| (p.freq, p.time))
        .collect::<HashSet<_>>();

    let buff = (0..(w * h))
        .flat_map(|i| {
            let x = i % w;
            let y = i / w;

            if GRID {
                if y % FOOTPRINT_SIZE == 0 {
                    return [0, 0, 255];
                }
                if x % FOOTPRINT_SIZE == 0 {
                    return [0, 0, 255];
                }
            }

            if set.contains(&(x, y)) {
                [0, 0, 0]
            } else {
                [255, 255, 255]
            }
        })
        .collect();

    imageops::rotate270(
        &RgbImage::from_raw(w as u32, h as u32, buff).expect("Failed to create image from data"),
    )
    .save("peaks.png")
    .expect("Failed to write image (peaks.png)");

    Ok(())
}

/// Convert HSL color space to RGB
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = l - c / 2.0;

    let (r_prime, g_prime, b_prime) = if (0.0..60.0).contains(&h) {
        (c, x, 0.0)
    } else if (60.0..120.0).contains(&h) {
        (x, c, 0.0)
    } else if (120.0..180.0).contains(&h) {
        (0.0, c, x)
    } else if (180.0..240.0).contains(&h) {
        (0.0, x, c)
    } else if (240.0..300.0).contains(&h) {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let r = ((r_prime + m) * 255.0).round() as u8;
    let g = ((g_prime + m) * 255.0).round() as u8;
    let b = ((b_prime + m) * 255.0).round() as u8;
    (r, g, b)
}
