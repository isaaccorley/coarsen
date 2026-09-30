//! GEOS 3.13.1 segment distance and topology predicates; arithmetic order matters.
use crate::geom::{Bounds, Point, orient};

pub fn distance(p: Point, a: Point, b: Point) -> f64 {
    let point_distance = |a: Point, b: Point| {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        (dx * dx + dy * dy).sqrt()
    };
    if a == b {
        return point_distance(p, a);
    }
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let length = dx * dx + dy * dy;
    let r = ((p.x - a.x) * dx + (p.y - a.y) * dy) / length;
    if r <= 0.0 {
        return point_distance(p, a);
    }
    if r >= 1.0 {
        return point_distance(p, b);
    }
    let s = ((a.y - p.y) * dx - (a.x - p.x) * dy) / length;
    s.abs() * length.sqrt()
}
pub fn furthest(points: &[Point], i: usize, j: usize) -> (usize, f64) {
    let (mut index, mut max) = (i, -1.0);
    for k in i + 1..j {
        let d = distance(points[k], points[i], points[j]);
        if d > max {
            index = k;
            max = d;
        }
    }
    (index, max)
}
pub fn invalid_intersection(a: Point, b: Point, c: Point, d: Point) -> bool {
    if (a == c && b == d) || (a == d && b == c) {
        return true;
    }
    let ab = Bounds::points([a, b]);
    let cd = Bounds::points([c, d]);
    if !ab.intersects(cd) {
        return false;
    }
    let ac = orient(a, b, c);
    let ad = orient(a, b, d);
    let ca = orient(c, d, a);
    let cb = orient(c, d, b);
    if ac * ad > 0 || ca * cb > 0 {
        return false;
    }
    if ac * ad < 0 && ca * cb < 0 {
        return true;
    }
    // Endpoint and collinear intersections are interior only if the point lies
    // inside at least one segment. Shared endpoints alone are permitted.
    [
        (c, ac, ab, a, b),
        (d, ad, ab, a, b),
        (a, ca, cd, c, d),
        (b, cb, cd, c, d),
    ]
    .into_iter()
    .any(|(p, o, env, x, y)| o == 0 && env.contains(p) && p != x && p != y)
}
pub fn crossing(p: Point, a: Point, b: Point) -> usize {
    if (a.x < p.x && b.x < p.x) || p == b || (a.y == p.y && b.y == p.y) {
        return 0;
    }
    if (a.y > p.y && b.y <= p.y) || (b.y > p.y && a.y <= p.y) {
        let sign = orient(a, b, p) * if b.y < a.y { -1 } else { 1 };
        return usize::from(sign > 0);
    }
    0
}
