//! Page geometry for the PDF export, in PDF points (1/72 inch). Pure.

use crate::export::Profile;

/// Where an image sits on its page: lower-left corner and drawn size, in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// The device screen in points: pixels at the profile's density, so a viewer at 100 % shows
/// the page at the size of the e-reader's screen.
pub fn page_size(profile: &Profile) -> (f32, f32) {
    let pt = |px: u32| px as f32 * 72.0 / profile.dpi.max(1) as f32;
    (pt(profile.width), pt(profile.height))
}

/// The image scaled to fit the page with its aspect ratio kept, centred.
pub fn place(image: (u32, u32), page: (f32, f32)) -> Placement {
    let (w, h) = (image.0.max(1) as f32, image.1.max(1) as f32);
    let scale = (page.0 / w).min(page.1 / h);
    let (width, height) = (w * scale, h * scale);
    Placement {
        x: (page.0 - width) / 2.0,
        y: (page.1 - height) / 2.0,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::profile;

    #[test]
    fn page_is_the_screen_at_its_density() {
        let pw = profile("kindle-paperwhite12").unwrap();
        let (w, h) = page_size(&pw);
        assert!(
            (w - 303.36).abs() < 0.01 && (h - 403.2).abs() < 0.01,
            "{w}x{h}"
        );
    }

    #[test]
    fn narrow_image_is_centred_horizontally() {
        let p = place((500, 1000), (300.0, 400.0));
        assert_eq!((p.width, p.height), (200.0, 400.0));
        assert_eq!((p.x, p.y), (50.0, 0.0));
    }

    #[test]
    fn wide_image_is_centred_vertically() {
        let p = place((1200, 600), (300.0, 400.0));
        assert_eq!((p.width, p.height), (300.0, 150.0));
        assert_eq!((p.x, p.y), (0.0, 125.0));
    }
}
