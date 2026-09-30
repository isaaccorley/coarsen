//! General geometry storage for the GEOS simplifiers (rings retain their identity).
use crate::geom::Point;
use anyhow::{Result, bail, ensure};

pub const Z: u32 = 0x80000000;
pub const M: u32 = 0x40000000;
pub const RING: u32 = 8;

#[derive(Clone, Debug)]
pub struct Shape {
    pub kind: u32,
    pub dims: u32,
    pub points: Vec<Point>,
    pub parts: Vec<Shape>,
}

impl Shape {
    pub fn empty(kind: u32, dims: u32) -> Self {
        Self {
            kind,
            dims,
            points: Vec::new(),
            parts: Vec::new(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.points.is_empty() && self.parts.iter().all(Self::is_empty)
    }
    pub fn dimensions(&self) -> u32 {
        if self.parts.is_empty() {
            self.dims
        } else {
            self.parts.iter().fold(0, |d, p| d | p.dimensions())
        }
    }
    pub fn validate_index_coordinates(&self) -> Result<()> {
        if self.kind == 2 || self.kind == RING {
            ensure!(
                self.points
                    .iter()
                    .all(|p| p.x.is_finite() && p.y.is_finite()),
                "IllegalArgumentException: Non-finite envelope bounds passed to index insert"
            );
        }
        for part in &self.parts {
            part.validate_index_coordinates()?;
        }
        Ok(())
    }
    /// GEOS's newly populated CoordinateSequence infers Z from its first point.
    pub fn inferred_dimensions(&mut self) {
        self.dims = if self.points.first().is_none_or(|p| !p.z.is_nan()) {
            Z
        } else {
            0
        };
        for p in &mut self.points {
            p.m = f64::NAN;
        }
    }
    pub fn build(mut parts: Vec<Self>, empty_kind: u32) -> Self {
        if parts.len() == 1 {
            return parts.pop().unwrap();
        }
        let kind = parts.first().map_or(empty_kind, |first| {
            if parts.iter().all(|p| p.kind == first.kind) {
                match first.kind {
                    1 => 4,
                    2 | RING => 5,
                    3 => 6,
                    _ => 7,
                }
            } else {
                7
            }
        });
        Self {
            parts,
            ..Self::empty(kind, 0)
        }
    }
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    le: bool,
}
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N]> {
        ensure!(
            N <= self.data.len().saturating_sub(self.pos),
            "truncated WKB"
        );
        let data = self.data[self.pos..self.pos + N].try_into().unwrap();
        self.pos += N;
        Ok(data)
    }
    fn u32(&mut self) -> Result<u32> {
        let b = self.take()?;
        Ok(if self.le {
            u32::from_le_bytes(b)
        } else {
            u32::from_be_bytes(b)
        })
    }
    fn f64(&mut self) -> Result<f64> {
        let b = self.take()?;
        Ok(if self.le {
            f64::from_le_bytes(b)
        } else {
            f64::from_be_bytes(b)
        })
    }
    fn count(&mut self, stride: usize) -> Result<usize> {
        let n = self.u32()? as usize;
        ensure!(
            n <= self.data.len().saturating_sub(self.pos) / stride,
            "invalid WKB count"
        );
        Ok(n)
    }
    fn coords(&mut self, dims: u32, n: usize) -> Result<Vec<Point>> {
        (0..n)
            .map(|_| {
                Ok(Point {
                    x: self.f64()?,
                    y: self.f64()?,
                    z: if dims & Z != 0 { self.f64()? } else { f64::NAN },
                    m: if dims & M != 0 { self.f64()? } else { f64::NAN },
                })
            })
            .collect()
    }
    fn shape(&mut self, depth: usize) -> Result<Shape> {
        ensure!(depth < 256, "WKB nesting exceeds 256");
        self.le = match self.take::<1>()?[0] {
            0 => false,
            1 => true,
            _ => bail!("invalid WKB endian"),
        };
        let code = self.u32()?;
        let iso = (code & 0xffff) / 1000;
        let kind = (code & 0xffff) % 1000;
        ensure!((1..=7).contains(&kind), "unsupported WKB type");
        let dims = code & (Z | M)
            | if iso == 1 || iso == 3 { Z } else { 0 }
            | if iso == 2 || iso == 3 { M } else { 0 };
        if code & 0x20000000 != 0 {
            self.u32()?;
        }
        let stride = 16 + 8 * usize::from(dims & Z != 0) + 8 * usize::from(dims & M != 0);
        let mut g = Shape::empty(kind, dims);
        match kind {
            1 => {
                g.points = self.coords(dims, 1)?;
                if g.points[0].x.is_nan() && g.points[0].y.is_nan() {
                    g.points.clear();
                }
            }
            2 => {
                let n = self.count(stride)?;
                g.points = self.coords(dims, n)?;
            }
            3 => {
                for _ in 0..self.count(4)? {
                    let n = self.count(stride)?;
                    let mut ring = Shape::empty(RING, dims);
                    ring.points = self.coords(dims, n)?;
                    g.parts.push(ring);
                }
            }
            _ => {
                for _ in 0..self.count(5)? {
                    g.parts.push(self.shape(depth + 1)?);
                }
            }
        }
        Ok(g)
    }
}
pub fn read(data: &[u8]) -> Result<Shape> {
    let mut r = Reader {
        data,
        pos: 0,
        le: true,
    };
    let g = r.shape(0)?;
    ensure!(r.pos == data.len(), "trailing WKB");
    Ok(g)
}
fn num(out: &mut Vec<u8>, n: u32) {
    out.extend_from_slice(&n.to_le_bytes());
}
fn point(out: &mut Vec<u8>, p: Point, dims: u32) {
    out.extend_from_slice(&p.x.to_le_bytes());
    out.extend_from_slice(&p.y.to_le_bytes());
    if dims & Z != 0 {
        out.extend_from_slice(&p.z.to_le_bytes());
    }
    if dims & M != 0 {
        out.extend_from_slice(&p.m.to_le_bytes());
    }
}
fn shape(out: &mut Vec<u8>, g: &Shape) {
    let dims = g.dimensions();
    out.push(1);
    num(out, (if g.kind == RING { 2 } else { g.kind }) | dims);
    match g.kind {
        1 => point(
            out,
            g.points.first().copied().unwrap_or(Point {
                x: f64::NAN,
                y: f64::NAN,
                ..Point::default()
            }),
            dims,
        ),
        2 | RING => {
            num(out, g.points.len() as u32);
            for &p in &g.points {
                point(out, p, dims);
            }
        }
        3 => {
            num(out, g.parts.len() as u32);
            for ring in &g.parts {
                num(out, ring.points.len() as u32);
                for &p in &ring.points {
                    point(out, p, dims);
                }
            }
        }
        _ => {
            num(out, g.parts.len() as u32);
            for p in &g.parts {
                shape(out, p);
            }
        }
    }
}
pub fn write(g: &Shape) -> Vec<u8> {
    let mut out = Vec::new();
    shape(&mut out, g);
    out
}
