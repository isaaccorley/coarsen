//! Geometry storage and predicates. The hot path never constructs GEOS objects.
use rstar::{AABB, RTreeObject};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub m: f64,
}
impl Default for Point {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: f64::NAN,
            m: f64::NAN,
        }
    }
}
impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}
impl Point {
    pub fn key(self) -> (u64, u64) {
        (
            if self.x == 0.0 { 0 } else { self.x.to_bits() },
            if self.y == 0.0 { 0 } else { self.y.to_bits() },
        )
    }
    pub fn compare(self, b: Self) -> Ordering {
        self.x
            .partial_cmp(&b.x)
            .unwrap()
            .then(self.y.partial_cmp(&b.y).unwrap())
    }
}
pub type Ring = Vec<Point>;
pub type Polygon = Vec<Ring>;

#[derive(Clone, Debug, Default)]
pub struct Geometry {
    pub polygons: Vec<Polygon>,
    pub multi: bool,
    /// WKB dimension flags per polygon (also retained for empty polygons).
    pub dimensions: Vec<u32>,
}
impl Geometry {
    pub fn points(&self) -> impl Iterator<Item = &Point> {
        self.polygons.iter().flatten().flatten()
    }
    pub fn points_mut(&mut self) -> impl Iterator<Item = &mut Point> {
        self.polygons.iter_mut().flatten().flatten()
    }
    pub fn bounds(&self) -> Bounds {
        Bounds::points(self.points().copied())
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub min: [f64; 2],
    pub max: [f64; 2],
}
impl Default for Bounds {
    fn default() -> Self {
        Self {
            min: [f64::INFINITY; 2],
            max: [f64::NEG_INFINITY; 2],
        }
    }
}
impl Bounds {
    pub fn points(pts: impl IntoIterator<Item = Point>) -> Self {
        let mut b = Self::default();
        for p in pts {
            b.add(p);
        }
        b
    }
    pub fn add(&mut self, p: Point) {
        self.min[0] = self.min[0].min(p.x);
        self.min[1] = self.min[1].min(p.y);
        self.max[0] = self.max[0].max(p.x);
        self.max[1] = self.max[1].max(p.y);
    }
    pub fn union(&mut self, b: Self) {
        if !b.empty() {
            self.add(Point {
                x: b.min[0],
                y: b.min[1],
                ..Point::default()
            });
            self.add(Point {
                x: b.max[0],
                y: b.max[1],
                ..Point::default()
            });
        }
    }
    pub fn empty(self) -> bool {
        self.min[0] > self.max[0]
    }
    pub fn intersects(self, b: Self) -> bool {
        self.min[0] <= b.max[0]
            && self.max[0] >= b.min[0]
            && self.min[1] <= b.max[1]
            && self.max[1] >= b.min[1]
    }
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.min[0] && p.x <= self.max[0] && p.y >= self.min[1] && p.y <= self.max[1]
    }
    pub fn aabb(self) -> AABB<[f64; 2]> {
        AABB::from_corners(self.min, self.max)
    }
}
#[derive(Clone)]
pub struct Item {
    pub id: usize,
    pub bounds: Bounds,
}
impl RTreeObject for Item {
    type Envelope = AABB<[f64; 2]>;
    fn envelope(&self) -> Self::Envelope {
        self.bounds.aabb()
    }
}

pub use crate::predicates::orient;
pub fn triangle_contains(a: Point, b: Point, c: Point, p: Point) -> bool {
    let exterior = if orient(a, b, c) > 0 { -1 } else { 1 };
    orient(a, b, p) != exterior && orient(b, c, p) != exterior && orient(c, a, p) != exterior
}
pub fn area(a: Point, b: Point, c: Point) -> f64 {
    (((c.x - a.x) * (b.y - a.y) - (b.x - a.x) * (c.y - a.y)) / 2.0).abs()
}

/// Packed envelope tree, sequence order retained (16 entries per leaf).
/// Only envelopes per block, not a heap allocation or R-tree entry per vertex.
pub struct Packed {
    levels: Vec<Vec<Bounds>>,
    len: usize,
}
impl Packed {
    pub fn new(bounds: impl IntoIterator<Item = Bounds>) -> Self {
        let mut leaves = Vec::new();
        let mut b = Bounds::default();
        let mut n = 0;
        for v in bounds {
            b.union(v);
            n += 1;
            if n % 16 == 0 {
                leaves.push(b);
                b = Bounds::default();
            }
        }
        if n % 16 != 0 {
            leaves.push(b);
        }
        let mut levels = vec![leaves];
        while levels.last().unwrap().len() > 1 {
            let upper = levels
                .last()
                .unwrap()
                .chunks(16)
                .map(|c| {
                    let mut b = Bounds::default();
                    for &v in c {
                        b.union(v);
                    }
                    b
                })
                .collect();
            levels.push(upper);
        }
        Self { levels, len: n }
    }
    pub fn any(&self, b: Bounds, mut test: impl FnMut(usize) -> bool) -> bool {
        if self.len == 0 {
            return false;
        }
        self.visit(self.levels.len() - 1, 0, b, &mut test)
    }
    fn visit(&self, l: usize, i: usize, b: Bounds, test: &mut impl FnMut(usize) -> bool) -> bool {
        if !self.levels[l][i].intersects(b) {
            return false;
        }
        if l == 0 {
            return (i * 16..((i + 1) * 16).min(self.len)).any(test);
        }
        (i * 16..((i + 1) * 16).min(self.levels[l - 1].len()))
            .any(|j| self.visit(l - 1, j, b, test))
    }
}
