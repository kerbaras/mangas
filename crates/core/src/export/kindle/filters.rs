//! E-ink filters on raw pixel buffers: BT.601 grayscale, contrast + gamma folded into one LUT,
//! and a 3x3 sharpen. Pure functions over `Vec<u8>`; the caller owns image geometry.

/// BT.601 luma of a packed RGBA buffer.
pub fn grayscale_rgba(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .map(|p| {
            ((19595 * p[0] as u32 + 38470 * p[1] as u32 + 7471 * p[2] as u32 + (1 << 15)) >> 16)
                as u8
        })
        .collect()
}

fn contrast_channel(v: u8, factor: f64) -> u8 {
    ((v as f64 - 128.0) * factor + 128.0).clamp(0.0, 255.0) as u8
}

fn gamma_channel(v: u8, gamma: f64) -> u8 {
    (255.0 * (v as f64 / 255.0).powf(1.0 / gamma) + 0.5).min(255.0) as u8
}

/// Contrast (around mid-grey) then gamma, as one 256-entry lookup table.
pub fn tone_lut(contrast: f64, gamma: f64) -> [u8; 256] {
    let mut lut = [0u8; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        let mut v = i as u8;
        if contrast != 1.0 {
            v = contrast_channel(v, contrast);
        }
        if gamma != 1.0 {
            v = gamma_channel(v, gamma);
        }
        *slot = v;
    }
    lut
}

pub fn is_identity(lut: &[u8; 256]) -> bool {
    lut.iter().enumerate().all(|(i, &v)| i == v as usize)
}

/// Maps every sample through `lut` (single-channel buffers; alpha is not special-cased).
pub fn apply_lut(gray: &[u8], lut: &[u8; 256]) -> Vec<u8> {
    gray.iter().map(|&v| lut[v as usize]).collect()
}

/// Kernel `[-1 -1 -1; -1 9 -1; -1 -1 -1]` on a single-channel `w x h` buffer; border pixels are
/// copied as-is.
pub fn sharpen_gray(src: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut dst = src.to_vec();
    if w < 3 || h < 3 {
        return dst;
    }
    for y in 1..h - 1 {
        let (up, mid, down) = (&src[(y - 1) * w..], &src[y * w..], &src[(y + 1) * w..]);
        for x in 1..w - 1 {
            let sum = 9 * mid[x] as i32
                - up[x - 1] as i32
                - up[x] as i32
                - up[x + 1] as i32
                - mid[x - 1] as i32
                - mid[x + 1] as i32
                - down[x - 1] as i32
                - down[x] as i32
                - down[x + 1] as i32;
            dst[y * w + x] = sum.clamp(0, 255) as u8;
        }
    }
    dst
}
