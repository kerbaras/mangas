//! The HD image container after the KF8 records (MobileRead wiki "KF8", "HD images"; layout as
//! kindlegen writes it): `BOUNDARY`, a `CONT` header, one slot per standard image record (a
//! `CRES` record holding the original JPEG unchanged after a 12-byte head, or the 4-byte
//! placeholder `A0A0A0A0`), the `kindle:embed` list of the HD images, `CONTBOUNDARY`, EOF.

use super::bytes::{pad4, push32, u32_of};
use super::exth;
use crate::domain::Result;

pub const BOUNDARY: &[u8] = b"BOUNDARY";
pub const CONT_BOUNDARY: &[u8] = b"CONTBOUNDARY";
pub const PLACEHOLDER: [u8; 4] = [0xA0; 4];
pub const CRES_HEAD: usize = 12;

/// The 12 bytes before an HD image in its `CRES` record.
pub fn cres_head() -> Vec<u8> {
    let mut out = b"CRES".to_vec();
    push32(&mut out, 0);
    push32(&mut out, CRES_HEAD as u32);
    out
}

/// What the container holds: per standard image slot, whether it has an HD copy, and the
/// largest HD image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    pub slots: Vec<bool>,
    pub max: (u32, u32),
}

impl Container {
    fn hd_count(&self) -> usize {
        self.slots.iter().filter(|&&hd| hd).count()
    }

    /// Record 0's EXTH 536: the largest size and the container's records after `CONT`.
    pub fn geometry(&self) -> String {
        format!("{}x{}:0-{}|", self.max.0, self.max.1, self.slots.len() + 2)
    }

    /// The `kindle:embed` list of the slots with an HD image, `|`-separated.
    pub fn embed_list(&self) -> Vec<u8> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, &hd)| hd)
            .map(|(i, _)| {
                format!(
                    "kindle:embed:{}?mime=image/jpg|",
                    super::bytes::base32(i + 1, 4)
                )
            })
            .collect::<String>()
            .into_bytes()
    }

    /// The `CONT` header: counts, the container EXTH and the title.
    pub fn header(&self, title: &str) -> Result<Vec<u8>> {
        let n = self.slots.len();
        let exth = exth::block(&[
            (539, b"application/image".to_vec()),
            (538, format!("{}x{}", self.max.0, self.max.1).into_bytes()),
            (543, b"HD_CONTAINER".to_vec()),
            (535, b"mangas".to_vec()),
        ])?;
        let mut name = title.as_bytes().to_vec();
        pad4(&mut name);
        let mut out = b"CONT".to_vec();
        let size = 48 + exth.len() + name.len();
        for value in [
            u32_of(size)?,
            (1 << 16) | u32_of(n + 3)?,
            65001,
            0,
            1,
            u32_of(n)?,
            u32_of(self.hd_count())?,
            u32_of(n + 1)?, // the embed list, counted from CONT
            1,
            u32_of(48 + exth.len())?,
            u32_of(title.len())?,
        ] {
            push32(&mut out, value);
        }
        out.extend(exth);
        out.extend(name);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_describes_its_slots() {
        let c = Container {
            slots: vec![true, true, false],
            max: (1181, 1680),
        };
        assert_eq!(c.geometry(), "1181x1680:0-5|");
        assert_eq!(
            c.embed_list(),
            b"kindle:embed:0001?mime=image/jpg|kindle:embed:0002?mime=image/jpg|"
        );
        let h = c.header("Yotsuba").unwrap();
        assert_eq!(&h[..4], b"CONT");
        assert_eq!(&h[48..52], b"EXTH");
        assert_eq!(
            u32::from_be_bytes(h[4..8].try_into().unwrap()) as usize,
            h.len()
        );
        assert!(h.windows(17).any(|w| w == b"application/image"));
        assert_eq!(cres_head().len(), CRES_HEAD);
    }
}
