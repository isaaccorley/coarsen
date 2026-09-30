//! CoverageRingEdges / CoverageEdge, including GEOS 3.13.1 key conventions.
use crate::geom::{Geometry, Point, Ring};
use rayon::prelude::*;
use rustc_hash::FxHashMap;

type ExtractedEdge = (Ring, bool, (u32, u32));

pub struct Edge {
    pub points: Ring,
    pub free: bool,
    pub ring_count: usize,
}
pub struct Coverage {
    pub edges: Vec<Edge>,
    pub rings: Vec<Vec<usize>>,
}
#[derive(Clone, Copy)]
struct Segment {
    a: u32,
    b: u32,
    ring: u32,
    pos: u32,
}
impl Coverage {
    pub fn extract(geoms: &[Geometry], bad: &[bool]) -> Self {
        let rings: Vec<&Ring> = geoms
            .iter()
            .zip(bad)
            .filter(|(_, b)| !**b)
            .flat_map(|(g, _)| g.polygons.iter().flatten())
            .collect();
        // Parallel sort gives deterministic coordinate IDs in XY order.
        let mut vertices: Vec<Point> = rings
            .par_iter()
            .flat_map_iter(|r| r.iter().copied())
            .collect();
        vertices.par_sort_unstable_by(|a, b| a.compare(*b));
        vertices.dedup();
        assert!(vertices.len() < u32::MAX as usize);
        let ids: FxHashMap<_, _> = vertices
            .iter()
            .enumerate()
            .map(|(i, p)| (p.key(), i as u32))
            .collect();
        let mut counts = vec![0u32; vertices.len()];
        let ring_ids: Vec<Vec<u32>> = rings
            .par_iter()
            .map(|r| r.iter().map(|p| ids[&p.key()]).collect())
            .collect();
        drop(ids);
        for r in &ring_ids {
            for &v in r.iter().skip(usize::from(r.len() >= 4)) {
                counts[v as usize] += 1;
            }
        }
        let mut segments: Vec<Segment> = ring_ids
            .par_iter()
            .enumerate()
            .flat_map_iter(|(ri, r)| {
                r.windows(2).enumerate().map(move |(i, w)| Segment {
                    a: w[0].min(w[1]),
                    b: w[0].max(w[1]),
                    ring: ri as u32,
                    pos: i as u32,
                })
            })
            .collect();
        segments.par_sort_unstable_by_key(|s| (s.a, s.b));
        let mut boundary: Vec<Vec<bool>> = ring_ids
            .iter()
            .map(|r| vec![false; r.len().saturating_sub(1)])
            .collect();
        let mut degree = vec![0u32; vertices.len()];
        let mut start = 0;
        while start < segments.len() {
            let s = segments[start];
            let mut end = start + 1;
            while end < segments.len() && (segments[end].a, segments[end].b) == (s.a, s.b) {
                end += 1;
            }
            if (end - start) % 2 == 1 {
                degree[s.a as usize] += 1;
                degree[s.b as usize] += 1;
                for s in &segments[start..end] {
                    boundary[s.ring as usize][s.pos as usize] = true;
                }
            }
            start = end;
        }
        drop(segments);
        // findBoundaryNodes initializes each counter at zero, not one, in 3.13.1.
        let mut nodes: Vec<bool> = counts
            .iter()
            .zip(degree)
            .map(|(&n, d)| n > 2 || d > 3)
            .collect();
        drop(counts);
        for (r, b) in ring_ids.iter().zip(&boundary) {
            if b.is_empty() {
                continue;
            }
            let mut prev = *b.last().unwrap();
            for (i, &v) in b.iter().enumerate() {
                if prev != v {
                    nodes[r[i] as usize] = true;
                }
                prev = v;
            }
        }
        drop(boundary);
        let extracted: Vec<Vec<ExtractedEdge>> = ring_ids
            .into_par_iter()
            .enumerate()
            .map(|(ri, mut r)| {
                r.dedup();
                let mut coords = rings[ri].clone();
                coords.dedup();
                // GEOS CoverageEdge copies Coordinate (XYZ), dropping M.
                for p in &mut coords {
                    p.m = f64::NAN;
                }
                if r.len() < 3 {
                    return vec![];
                }
                let n = r.len() - 1;
                let cuts: Vec<usize> = (0..n).filter(|&i| nodes[r[i] as usize]).collect();
                let ring_key = || {
                    // GEOS comment says lowest; implementation selects highest.
                    let high = (0..n).max_by_key(|&i| r[i]).unwrap();
                    (r[high], r[(high + 1) % n].min(r[(high + n - 1) % n]))
                };
                if cuts.is_empty() {
                    let k = ring_key();
                    return vec![(coords, true, k)];
                }
                cuts.iter()
                    .enumerate()
                    .map(|(ci, &a)| {
                        let b = cuts[(ci + 1) % cuts.len()];
                        let k = if a == b {
                            ring_key()
                        } else if r[a] < r[b] {
                            (r[a], r[(a + 1) % n])
                        } else {
                            (r[b], r[(b + n - 1) % n])
                        };
                        let len = if b > a { b - a + 1 } else { n - a + b + 1 };
                        let pts = (0..len).map(|i| coords[(a + i) % n]).collect();
                        (pts, false, k)
                    })
                    .collect()
            })
            .collect();
        let mut edges = Vec::new();
        let mut ring_edges = Vec::new();
        let mut unique = FxHashMap::default();
        for ring in extracted {
            let mut refs = Vec::new();
            for (pts, free, k) in ring {
                let id = *unique.entry(k).or_insert_with(|| {
                    let id = edges.len();
                    edges.push(Edge {
                        points: pts,
                        free,
                        ring_count: 0,
                    });
                    id
                });
                edges[id].ring_count += 1;
                refs.push(id);
            }
            ring_edges.push(refs);
        }
        Self {
            edges,
            rings: ring_edges,
        }
    }
    pub fn rebuild(&self, geoms: &mut [Geometry], bad: &[bool], simplified: &[Ring]) {
        let mut ri = 0;
        for (g, &bad) in geoms.iter_mut().zip(bad) {
            if bad {
                continue;
            }
            for (pi, polygon) in g.polygons.iter_mut().enumerate() {
                let original_dims = g.dimensions[pi];
                let mut dims = 0;
                for ring in polygon.iter_mut() {
                    let refs = &self.rings[ri];
                    ri += 1;
                    if refs.is_empty() {
                        dims |= original_dims;
                        continue;
                    }
                    let mut pts: Ring = Vec::new();
                    for (i, &e) in refs.iter().enumerate() {
                        let edge = &simplified[e];
                        let forward = if refs.len() <= 2 && i == 0 {
                            true
                        } else if i == 0 {
                            let next = &simplified[refs[1]];
                            edge.last() == next.first() || edge.last() == next.last()
                        } else {
                            pts.last() == edge.first()
                        };
                        let mut append = |p: Point| {
                            if pts.last() != Some(&p) {
                                pts.push(p);
                            }
                        };
                        if forward {
                            for &p in edge {
                                append(p);
                            }
                        } else {
                            for &p in edge.iter().rev() {
                                append(p);
                            }
                        }
                    }
                    // GEOS's dimension-unknown CoordinateSequence inspects only
                    // the first coordinate when inferring Z, even for mixed rings.
                    if pts.first().is_some_and(|p| !p.z.is_nan()) {
                        dims |= 0x80000000;
                    }
                    *ring = pts;
                }
                g.dimensions[pi] = if polygon.is_empty() {
                    original_dims
                } else {
                    dims
                };
            }
        }
    }
}
