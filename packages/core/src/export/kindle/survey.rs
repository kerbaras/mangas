//! What a counting run learns about one chapter: its pages, the files of its regular pages (the
//! estimate samples those afterwards) and the JPEG size of every strip page, which only exists
//! while its strip is cut. So that a plan can size the strips at another quality too, a spread
//! few strip pages are also encoded at each of the other qualities: every `stride`-th one, the
//! stride doubling whenever more than [`SAMPLE`] are kept.

use std::path::PathBuf;

use image::RgbaImage;
use rayon::prelude::*;

use super::pool;
use super::process::{encode, ImageSettings};
use crate::domain::Result;

/// Most pages a sample keeps: of a chapter's strip pages, and of a book's regular pages.
pub(super) const SAMPLE: usize = 16;

/// A strip page's JPEG bytes at the base quality, and at the other qualities when sampled.
pub(super) type Encoded = (u64, Option<Vec<u64>>);

#[derive(Debug, Clone)]
pub(super) struct Survey {
    pub pages: usize,
    pub regular: Vec<PathBuf>,
    /// Every strip page's JPEG bytes at `base`, in order.
    pub strip_jpegs: Vec<u64>,
    pub base: u8,
    /// The other qualities the sampled strip pages were encoded at.
    pub qualities: Vec<u8>,
    /// Sampled strip pages: their index, their bytes at `base` and at each of `qualities`.
    samples: Vec<(usize, u64, Vec<u64>)>,
    stride: usize,
}

impl Survey {
    pub fn new(base: u8, qualities: Vec<u8>) -> Survey {
        Survey {
            pages: 0,
            regular: Vec::new(),
            strip_jpegs: Vec::new(),
            base,
            qualities: qualities.into_iter().filter(|&q| q != base).collect(),
            samples: Vec::new(),
            stride: 1,
        }
    }

    pub fn add_regular(&mut self, files: &[PathBuf]) {
        self.pages += files.len();
        self.regular.extend_from_slice(files);
    }

    /// Whether the strip page `i` pages after the ones counted so far is sampled.
    pub fn picker(&self) -> impl Fn(usize) -> bool + Sync {
        let (first, stride) = (self.strip_jpegs.len(), self.stride);
        move |i| (first + i) % stride == 0
    }

    /// Counts the next strip pages, `encoded` as [`sample_bytes`] did with [`Self::picker`].
    pub fn add_strip(&mut self, encoded: Vec<Encoded>) {
        for (bytes, others) in encoded {
            let i = self.strip_jpegs.len();
            if let Some(others) = others {
                self.samples.push((i, bytes, others));
            }
            self.strip_jpegs.push(bytes);
            self.pages += 1;
        }
        while self.samples.len() > SAMPLE {
            self.stride *= 2;
            let stride = self.stride;
            self.samples.retain(|(i, _, _)| i % stride == 0);
        }
    }

    /// Counts the strip pages `rendered` next, encoded at `settings`.
    pub fn count_strip(&mut self, rendered: Vec<RgbaImage>, settings: ImageSettings) -> Result<()> {
        let picked = self.picker();
        let encoded = sample_bytes(rendered, settings, &self.qualities, &picked)?;
        self.add_strip(encoded);
        Ok(())
    }

    /// Whether the strip pages can be sized at `quality`.
    pub fn sampled(&self, quality: u8) -> bool {
        self.strip_jpegs.is_empty() || quality == self.base || self.qualities.contains(&quality)
    }

    /// Every strip page's JPEG bytes at `quality`: as encoded at the base quality, or scaled by
    /// how much the samples grew or shrank at `quality`. A chapter without strips has none at
    /// every quality.
    pub fn strip_jpegs_at(&self, quality: u8) -> Option<Vec<u64>> {
        if quality == self.base || self.strip_jpegs.is_empty() {
            return Some(self.strip_jpegs.clone());
        }
        let q = self.qualities.iter().position(|&x| x == quality)?;
        let base: u64 = self.samples.iter().map(|s| s.1).sum();
        let at: u64 = self.samples.iter().map(|s| s.2[q]).sum();
        let ratio = at as f64 / base.max(1) as f64;
        Some(
            (self.strip_jpegs.iter())
                .map(|&b| (b as f64 * ratio).round() as u64)
                .collect(),
        )
    }
}

/// Every drawn page of `rendered` encoded at `settings`' quality, and the `picked` ones (by
/// their index there) at each of `others` too.
pub(super) fn sample_bytes(
    rendered: Vec<RgbaImage>,
    settings: ImageSettings,
    others: &[u8],
    picked: &(dyn Fn(usize) -> bool + Sync),
) -> Result<Vec<Encoded>> {
    let size = |img: &RgbaImage, quality: u8| {
        encode(
            img,
            &ImageSettings {
                quality,
                ..settings
            },
        )
        .map(|p| p.jpeg.len() as u64)
    };
    pool::install(|| {
        (rendered.into_par_iter().enumerate())
            .map(|(i, img)| {
                let others = match picked(i) {
                    true => Some(
                        others
                            .iter()
                            .map(|&q| size(&img, q))
                            .collect::<Result<_>>()?,
                    ),
                    false => None,
                };
                Ok((size(&img, settings.quality)?, others))
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sample_stays_spread_over_every_strip_page() {
        let mut s = Survey::new(90, vec![80, 90]);
        for i in 0..100u64 {
            let others = s.picker()(0).then(|| vec![i / 2]);
            s.add_strip(vec![(i, others)]);
        }
        assert_eq!((s.pages, s.strip_jpegs.len()), (100, 100));
        let kept: Vec<usize> = s.samples.iter().map(|s| s.0).collect();
        assert!(kept.len() <= SAMPLE && kept.len() > SAMPLE / 2, "{kept:?}");
        assert_eq!(kept, (0..100).step_by(s.stride).collect::<Vec<_>>());
        assert_eq!(s.strip_jpegs_at(90).map(|j| j[99]), Some(99));
        assert_eq!(s.strip_jpegs_at(80).map(|j| j[98]), Some(49));
        assert!(s.strip_jpegs_at(70).is_none() && !s.sampled(70) && s.sampled(80));
    }

    #[test]
    fn a_chapter_without_strips_is_sized_at_any_quality() {
        let mut s = Survey::new(90, vec![80]);
        s.add_regular(&[PathBuf::from("001.png")]);
        assert!(s.sampled(85));
        assert_eq!(s.strip_jpegs_at(85), Some(Vec::new()));
    }
}
