//! Ordered edge-dependency rounds. Intersecting envelopes retain GEOS edge order.
//! A removed corner and all future corners remain inside the original envelope.
//! Therefore edges in the same round cannot observe each other's mutations.
use crate::{
    edges::Edge,
    geom::{Bounds, Item, Packed, Ring, area, triangle_contains},
};
use rayon::prelude::*;
use rstar::RTree;
use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
};
struct Indexed<'a> {
    edge: &'a Edge,
    index: Option<Packed>,
    alive: Vec<AtomicBool>,
    size: AtomicUsize,
}
#[derive(Clone, Copy, Debug)]
struct Corner {
    area: f64,
    i: usize,
    prev: usize,
    next: usize,
}
impl PartialEq for Corner {
    fn eq(&self, b: &Self) -> bool {
        self.area == b.area && self.i == b.i
    }
}
impl Eq for Corner {}
impl PartialOrd for Corner {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for Corner {
    fn cmp(&self, b: &Self) -> Ordering {
        b.area
            .partial_cmp(&self.area)
            .unwrap()
            .then(b.i.cmp(&self.i))
    }
}
fn is_ring(e: &Edge) -> bool {
    e.points.len() >= 4 && e.points.first() == e.points.last()
}

/// An edge without an initial removable-size corner cannot ever change: no
/// vertex removal can create a new corner until its own first removal occurs.
fn can_change(e: &Edge, tolerance: f64) -> bool {
    let ring = is_ring(e);
    let n = e.points.len() - usize::from(ring);
    if n <= if ring { 3 } else { 2 } {
        return false;
    }
    let start = usize::from(!(ring && e.free));
    (start..e.points.len() - 1).any(|i| {
        area(
            e.points[(i + n - 1) % n],
            e.points[i],
            e.points[(i + 1) % n],
        ) <= tolerance
    })
}

pub fn simplify(edges: &[Edge], tolerance: f64) -> (Vec<Ring>, usize) {
    simplify_selected(edges, tolerance, true)
}

pub fn simplify_selected(edges: &[Edge], tolerance: f64, boundary: bool) -> (Vec<Ring>, usize) {
    let active: Vec<bool> = edges
        .par_iter()
        .map(|e| (boundary || e.ring_count == 2) && can_change(e, tolerance * tolerance))
        .collect();
    let index: Vec<_> = edges
        .par_iter()
        .map(|e| {
            let n = e.points.len() - usize::from(is_ring(e));
            Indexed {
                edge: e,
                index: (n > 16)
                    .then(|| Packed::new(e.points[..n].iter().map(|&p| Bounds::points([p])))),
                alive: (0..n).map(|_| AtomicBool::new(true)).collect(),
                size: AtomicUsize::new(n),
            }
        })
        .collect();
    let tree = RTree::bulk_load(
        edges
            .iter()
            .enumerate()
            .map(|(id, e)| Item {
                id,
                bounds: Bounds::points(e.points.iter().copied()),
            })
            .collect(),
    );
    let neighbors: Vec<Vec<usize>> = edges
        .par_iter()
        .enumerate()
        .map(|(i, e)| {
            // Immutable edges still live in the global constraint index.
            if !active[i] {
                return Vec::new();
            }
            tree.locate_in_envelope_intersecting(Bounds::points(e.points.iter().copied()).aabb())
                // simplifyInner excludes edges used by >2 rings from constraints.
                .filter(|v| boundary || edges[v.id].ring_count <= 2)
                .map(|v| v.id)
                .collect()
        })
        .collect();
    let mut levels = vec![0; edges.len()];
    let mut rounds: Vec<Vec<usize>> = vec![];
    for i in 0..edges.len() {
        if !active[i] {
            continue;
        }
        levels[i] = neighbors[i]
            .iter()
            .filter(|&&j| j < i && active[j])
            .map(|&j| levels[j] + 1)
            .max()
            .unwrap_or(0);
        while rounds.len() <= levels[i] {
            rounds.push(vec![]);
        }
        rounds[levels[i]].push(i);
    }
    for round in &rounds {
        round
            .par_iter()
            .for_each(|&id| simplify_edge(id, &index, &neighbors[id], tolerance * tolerance));
    }
    let out = index
        .par_iter()
        .map(|e| {
            let mut pts: Ring = e
                .edge
                .points
                .iter()
                .zip(&e.alive)
                .filter(|(_, alive)| alive.load(Relaxed))
                .map(|(&p, _)| p)
                .collect();
            pts.dedup();
            if is_ring(e.edge) && !pts.is_empty() {
                pts.push(pts[0]);
            }
            pts
        })
        .collect();
    (out, rounds.len())
}
fn simplify_edge(id: usize, edges: &[Indexed<'_>], neighbors: &[usize], tol: f64) {
    let this = &edges[id];
    let e = this.edge;
    let pts = &e.points;
    let ring = is_ring(e);
    let n = this.alive.len();
    let min = if ring { 3 } else { 2 };
    if n <= min {
        return;
    }
    let mut prev: Vec<usize> = (0..n)
        .map(|i| {
            if i == 0 {
                if ring { n - 1 } else { usize::MAX }
            } else {
                i - 1
            }
        })
        .collect();
    let mut next: Vec<usize> = (0..n)
        .map(|i| {
            if i + 1 == n {
                if ring { 0 } else { usize::MAX }
            } else {
                i + 1
            }
        })
        .collect();
    let mut q = BinaryHeap::new();
    let add = |i: usize, prev: &[usize], next: &[usize], q: &mut BinaryHeap<Corner>| {
        if e.free || (i != 0 && i != pts.len() - 1) {
            let a = area(pts[prev[i]], pts[i], pts[next[i]]);
            if a <= tol {
                q.push(Corner {
                    area: a,
                    i,
                    prev: prev[i],
                    next: next[i],
                });
            }
        }
    };
    for i in (if ring && e.free { 0 } else { 1 })..pts.len() - 1 {
        add(i, &prev, &next, &mut q);
    }
    let mut size = n;
    while size > min {
        let Some(c) = q.pop() else {
            break;
        };
        if prev[c.i] != c.prev || next[c.i] != c.next {
            continue;
        }
        if c.area > tol {
            break;
        }
        let a = pts[c.prev];
        let b = pts[c.i];
        let d = pts[c.next];
        let env = Bounds::points([a, b, d]);
        let blocked = neighbors.iter().any(|&j| {
            let other = &edges[j];
            let intersects = |v: usize| {
                if !other.alive[v].load(Relaxed) {
                    return false;
                }
                let p = other.edge.points[v];
                env.contains(p) && p != a && p != b && p != d && triangle_contains(a, b, d, p)
            };
            let hit = match &other.index {
                Some(index) => index.any(env, intersects),
                None => (0..other.alive.len()).any(intersects),
            };
            if hit {
                return true;
            }
            if j != id && other.size.load(Relaxed) == 2 {
                let p = other.edge.points[0];
                let q = *other.edge.points.last().unwrap();
                return (a == p && d == q) || (a == q && d == p);
            }
            false
        });
        if blocked {
            continue;
        }
        next[c.prev] = c.next;
        prev[c.next] = c.prev;
        prev[c.i] = usize::MAX;
        next[c.i] = usize::MAX;
        this.alive[c.i].store(false, Relaxed);
        size -= 1;
        this.size.store(size, Relaxed);
        add(c.prev, &prev, &next, &mut q);
        add(c.next, &prev, &next, &mut q);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Point;
    #[test]
    fn parallel_order_and_all_boundaries() {
        let edges: Vec<_> = (0..40)
            .map(|i| {
                let x = (i % 10) as f64 * 10.;
                let y = (i / 10) as f64 * 10.;
                Edge {
                    points: vec![
                        Point {
                            x,
                            y,
                            ..Point::default()
                        },
                        Point {
                            x: x + 2.,
                            y: y + 0.1,
                            ..Point::default()
                        },
                        Point {
                            x: x + 4.,
                            y,
                            ..Point::default()
                        },
                        Point {
                            x: x + 4.,
                            y: y + 4.,
                            ..Point::default()
                        },
                        Point {
                            x,
                            y: y + 4.,
                            ..Point::default()
                        },
                        Point {
                            x,
                            y,
                            ..Point::default()
                        },
                    ],
                    free: true,
                    ring_count: 1,
                }
            })
            .collect();
        let run = |n| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(n)
                .build()
                .unwrap()
                .install(|| simplify(&edges, 1.).0)
        };
        let a = run(1);
        assert_eq!(a, run(8));
        assert!(a.iter().all(|r| r.len() == 5));
    }
}
