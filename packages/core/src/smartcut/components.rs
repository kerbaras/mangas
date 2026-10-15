//! 8-connected component labelling with union-find.

use serde::{Deserialize, Serialize};

/// Half-open pixel box `[x0, x1) x [y0, y1)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PixelBox {
    pub x0: usize,
    pub y0: usize,
    pub x1: usize,
    pub y1: usize,
}

impl PixelBox {
    pub fn width(&self) -> usize {
        self.x1.saturating_sub(self.x0)
    }
    pub fn height(&self) -> usize {
        self.y1.saturating_sub(self.y0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Component {
    pub bbox: PixelBox,
    pub area: usize,
}

struct UnionFind(Vec<u32>);

impl UnionFind {
    fn find(&mut self, mut x: u32) -> u32 {
        while self.0[x as usize] != x {
            let p = self.0[x as usize];
            self.0[x as usize] = self.0[p as usize];
            x = self.0[x as usize];
        }
        x
    }
    fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
            self.0[hi as usize] = lo;
        }
    }
}

fn first_pass(m: &[u8], w: usize, h: usize) -> (Vec<u32>, UnionFind) {
    let mut labels = vec![0u32; w * h];
    let mut uf = UnionFind(vec![0]);
    let mut next = 1u32;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if m[i] == 0 {
                continue;
            }
            let mut lab = 0u32;
            let mut consider = |l: u32, uf: &mut UnionFind| {
                if l == 0 {
                    return;
                }
                if lab == 0 {
                    lab = l;
                } else {
                    uf.union(lab, l);
                }
            };
            if y > 0 {
                for nx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    consider(labels[(y - 1) * w + nx], &mut uf);
                }
            }
            if x > 0 {
                consider(labels[i - 1], &mut uf);
            }
            if lab == 0 {
                lab = next;
                next += 1;
                uf.0.push(lab);
            }
            labels[i] = lab;
        }
    }
    (labels, uf)
}

/// Returns dense per-pixel labels (0 = background) and the components indexed by `label - 1`.
pub fn label(m: &[u8], w: usize, h: usize) -> (Vec<u32>, Vec<Component>) {
    if w == 0 || h == 0 {
        return (Vec::new(), Vec::new());
    }
    let (mut labels, mut uf) = first_pass(m, w, h);
    let n = uf.0.len() as u32;
    let mut remap = vec![0u32; n as usize];
    let mut comps: Vec<Component> = Vec::new();
    for l in 1..n {
        let r = uf.find(l);
        if remap[r as usize] == 0 {
            comps.push(Component {
                bbox: PixelBox {
                    x0: w,
                    y0: h,
                    x1: 0,
                    y1: 0,
                },
                area: 0,
            });
            remap[r as usize] = comps.len() as u32;
        }
        remap[l as usize] = remap[r as usize];
    }
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if labels[i] == 0 {
                continue;
            }
            let l = remap[labels[i] as usize];
            labels[i] = l;
            let c = &mut comps[l as usize - 1];
            c.area += 1;
            c.bbox.x0 = c.bbox.x0.min(x);
            c.bbox.y0 = c.bbox.y0.min(y);
            c.bbox.x1 = c.bbox.x1.max(x + 1);
            c.bbox.y1 = c.bbox.y1.max(y + 1);
        }
    }
    (labels, comps)
}
