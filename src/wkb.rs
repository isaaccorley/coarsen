//! Bounded, endian-aware Polygon/MultiPolygon WKB codec, including Z/M and SRID.
use crate::geom::{Geometry, Point, Polygon};
use anyhow::{Result, bail, ensure};

const Z: u32 = 0x80000000;
const M: u32 = 0x40000000;
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    le: bool,
}
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N]> {
        ensure!(
            N <= self.bytes.len().saturating_sub(self.pos),
            "truncated WKB"
        );
        let bytes = self.bytes[self.pos..self.pos + N].try_into().unwrap();
        self.pos += N;
        Ok(bytes)
    }
    fn u32(&mut self) -> Result<u32> {
        let bytes = self.take()?;
        Ok(if self.le {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        })
    }
    fn f64(&mut self) -> Result<f64> {
        let bytes = self.take()?;
        Ok(if self.le {
            f64::from_le_bytes(bytes)
        } else {
            f64::from_be_bytes(bytes)
        })
    }
    fn geom(&mut self, child: bool) -> Result<Geometry> {
        self.le = match self.take::<1>()?[0] {
            0 => false,
            1 => true,
            _ => bail!("invalid WKB endian"),
        };
        let code = self.u32()?;
        let iso = (code & 0xffff) / 1000;
        let kind = (code & 0xffff) % 1000;
        let dims = (code & (Z | M))
            | if iso == 1 || iso == 3 { Z } else { 0 }
            | if iso == 2 || iso == 3 { M } else { 0 };
        ensure!(
            kind == 3 || (kind == 6 && !child),
            "expected Polygon/MultiPolygon WKB"
        );
        if code & 0x20000000 != 0 {
            self.u32()?;
        }
        let n = self.u32()? as usize;
        ensure!(
            n <= self.bytes.len().saturating_sub(self.pos) / 4,
            "invalid WKB count"
        );
        if kind == 6 {
            let mut g = Geometry {
                multi: true,
                ..Geometry::default()
            };
            for _ in 0..n {
                let child = self.geom(true)?;
                g.polygons.extend(child.polygons);
                g.dimensions.extend(child.dimensions);
            }
            return Ok(g);
        }
        let mut polygon = Vec::with_capacity(n);
        let stride = 16 + 8 * usize::from(dims & Z != 0) + 8 * usize::from(dims & M != 0);
        for _ in 0..n {
            let count = self.u32()? as usize;
            ensure!(
                count <= self.bytes.len().saturating_sub(self.pos) / stride,
                "invalid ring size"
            );
            let mut ring = Vec::with_capacity(count);
            for _ in 0..count {
                let p = Point {
                    x: self.f64()?,
                    y: self.f64()?,
                    z: if dims & Z != 0 { self.f64()? } else { f64::NAN },
                    m: if dims & M != 0 { self.f64()? } else { f64::NAN },
                };
                ensure!(
                    p.x.is_finite() && p.y.is_finite(),
                    "nonfinite XY coordinate"
                );
                ring.push(p);
            }
            ensure!(
                ring.is_empty() || (ring.len() >= 3 && ring.first() == ring.last()),
                "unclosed or short ring"
            );
            polygon.push(ring);
        }
        Ok(Geometry {
            polygons: vec![polygon],
            multi: false,
            dimensions: vec![dims],
        })
    }
}
pub fn read(bytes: &[u8]) -> Result<Geometry> {
    let mut reader = Reader {
        bytes,
        pos: 0,
        le: true,
    };
    let geometry = reader.geom(false)?;
    ensure!(reader.pos == bytes.len(), "trailing WKB");
    Ok(geometry)
}
fn num(bytes: &mut Vec<u8>, n: u32) {
    bytes.extend_from_slice(&n.to_le_bytes());
}
fn polygon(bytes: &mut Vec<u8>, polygon: &Polygon, dims: u32) {
    bytes.push(1);
    num(bytes, 3 | dims);
    num(bytes, polygon.len() as u32);
    for ring in polygon {
        num(bytes, ring.len() as u32);
        for p in ring {
            bytes.extend_from_slice(&p.x.to_le_bytes());
            bytes.extend_from_slice(&p.y.to_le_bytes());
            if dims & Z != 0 {
                bytes.extend_from_slice(&p.z.to_le_bytes());
            }
            if dims & M != 0 {
                bytes.extend_from_slice(&p.m.to_le_bytes());
            }
        }
    }
}
pub fn write(geometry: &Geometry) -> Vec<u8> {
    let mut bytes = Vec::new();
    if geometry.multi {
        bytes.push(1);
        num(
            &mut bytes,
            6 | geometry.dimensions.iter().fold(0, |a, b| a | b),
        );
        num(&mut bytes, geometry.polygons.len() as u32);
    }
    for (p, &dims) in geometry.polygons.iter().zip(&geometry.dimensions) {
        polygon(&mut bytes, p, dims);
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_counts_and_recursive_collections() {
        assert!(read(&[1, 3, 0, 0, 0, 255, 255, 255, 255]).is_err());
        let mut nested = vec![1, 6, 0, 0, 0, 1, 0, 0, 0];
        nested.extend_from_slice(&[1, 6, 0, 0, 0, 0, 0, 0, 0]);
        assert!(read(&nested).is_err());
    }
}
