//! TaggedLinesSimplifier and ComponentJumpChecker, GEOS 3.13.1 encounter order.
use crate::geom::{Bounds, Item, Packed, Point, orient};
use crate::line_math::{crossing, distance, furthest, invalid_intersection};
use crate::simple_geom::{RING, Shape};
use rstar::RTree;

#[derive(Clone, Copy)]
struct Segment {
    a: Point,
    b: Point,
    output: Option<usize>,
}
impl Segment {
    fn new(a: Point, b: Point) -> Self {
        Self { a, b, output: None }
    }
    fn bounds(self) -> Bounds {
        Bounds::points([self.a, self.b])
    }
    fn intersects(self, other: Self) -> bool {
        invalid_intersection(self.a, self.b, other.a, other.b)
    }
}
struct Line {
    points: Vec<Point>,
    ring: bool,
    minimum: usize,
    offset: usize,
    result: Vec<Segment>,
}
impl Line {
    fn size(&self) -> usize {
        if self.result.is_empty() {
            0
        } else {
            self.result.len() + 1
        }
    }
    fn component_point(&self) -> Point {
        self.result.first().map_or(self.points[1], |s| s.a)
    }
}
struct Simplifier {
    lines: Vec<Line>,
    input: Vec<(Segment, usize, usize)>,
    alive: Vec<bool>,
    packed: Packed,
    output: Vec<Segment>,
    output_alive: Vec<bool>,
    tree: RTree<Item>,
}
impl Simplifier {
    fn new(g: &Shape) -> Self {
        let mut lines = Vec::new();
        fn collect(g: &Shape, lines: &mut Vec<Line>) {
            if (g.kind == 2 || g.kind == RING) && !g.points.is_empty() {
                lines.push(Line {
                    points: g.points.clone(),
                    ring: g.kind == RING,
                    minimum: if g.points.first() == g.points.last() {
                        4
                    } else {
                        2
                    },
                    offset: 0,
                    result: Vec::new(),
                });
            }
            for part in &g.parts {
                collect(part, lines);
            }
        }
        collect(g, &mut lines);
        let mut input = Vec::new();
        for (id, line) in lines.iter_mut().enumerate() {
            line.offset = input.len();
            for (i, p) in line.points.windows(2).enumerate() {
                input.push((Segment::new(p[0], p[1]), id, i));
            }
        }
        let packed = Packed::new(input.iter().map(|(s, _, _)| s.bounds()));
        Self {
            lines,
            alive: vec![true; input.len()],
            packed,
            input,
            output: Vec::new(),
            output_alive: Vec::new(),
            tree: RTree::new(),
        }
    }
    fn add_output(&mut self, mut s: Segment) -> Segment {
        let id = self.output.len();
        s.output = Some(id);
        self.output.push(s);
        self.output_alive.push(true);
        self.tree.insert(Item {
            id,
            bounds: s.bounds(),
        });
        s
    }
    fn valid(&self, s: Segment, exclude: Option<(usize, usize, usize)>) -> bool {
        if self
            .tree
            .locate_in_envelope_intersecting(s.bounds().aabb())
            .any(|item| self.output_alive[item.id] && s.intersects(self.output[item.id]))
        {
            return false;
        }
        !self.packed.any(s.bounds(), |id| {
            let (candidate, line, index) = self.input[id];
            if !self.alive[id] {
                return false;
            }
            if exclude.is_some_and(|(l, i, j)| l == line && i <= index && index < j) {
                return false;
            }
            s.intersects(candidate)
        })
    }
    fn jumps(&self, line: usize, points: &[Point], flat: Segment) -> bool {
        let bounds = Bounds::points(points.iter().copied());
        self.lines.iter().enumerate().any(|(id, other)| {
            if id == line {
                return false;
            }
            let p = other.component_point();
            bounds.contains(p)
                && points
                    .windows(2)
                    .map(|s| crossing(p, s[0], s[1]))
                    .sum::<usize>()
                    % 2
                    != crossing(p, flat.a, flat.b) % 2
        })
    }
    fn simplify_line(&mut self, id: usize, tolerance: f64) {
        // Explicit stack has GEOS's recursive left-before-right visit order.
        let mut stack = vec![(0, self.lines[id].points.len() - 1, 0)];
        while let Some((i, j, depth)) = stack.pop() {
            let depth = depth + 1;
            let line = &self.lines[id];
            if i + 1 >= j {
                if i < j {
                    let s = Segment::new(line.points[i], line.points[j]);
                    self.lines[id].result.push(s);
                }
                continue;
            }
            let (k, max) = furthest(&line.points, i, j);
            if max < 0.0 {
                let segments: Vec<_> = line.points[i..=j]
                    .windows(2)
                    .map(|p| Segment::new(p[0], p[1]))
                    .collect();
                self.lines[id].result.extend(segments);
                continue;
            }
            let flat = Segment::new(line.points[i], line.points[j]);
            let enough = line.size() >= line.minimum || depth + 1 >= line.minimum;
            if enough
                && max <= tolerance
                && self.valid(flat, Some((id, i, j)))
                && !self.jumps(id, &line.points[i..=j], flat)
            {
                let offset = self.lines[id].offset;
                self.alive[offset + i..offset + j].fill(false);
                let flat = self.add_output(flat);
                self.lines[id].result.push(flat);
            } else {
                stack.push((k, j, depth));
                stack.push((i, k, depth));
            }
        }
        let line = &self.lines[id];
        if !line.ring
            || line.points.len() < 4
            || line.points.first() != line.points.last()
            || line.size() <= line.minimum
        {
            return;
        }
        let first = *line.result.first().unwrap();
        let last = *line.result.last().unwrap();
        let flat = Segment::new(last.a, first.b);
        if distance(first.a, flat.a, flat.b) > tolerance {
            return;
        }
        if orient(flat.a, flat.b, first.a) != 0
            && (!self.valid(flat, None) || self.endpoint_jumps(id, first, last, flat))
        {
            return;
        }
        for s in [first, last] {
            if let Some(index) = s.output {
                self.output_alive[index] = false;
            }
        }
        let flat = self.add_output(flat);
        self.lines[id].result.pop();
        self.lines[id].result[0] = flat;
    }
    fn endpoint_jumps(&self, line: usize, a: Segment, b: Segment, flat: Segment) -> bool {
        let bounds = Bounds::points([a.a, a.b, b.a, b.b]);
        self.lines.iter().enumerate().any(|(id, other)| {
            if id == line {
                return false;
            }
            let p = other.component_point();
            bounds.contains(p)
                && (crossing(p, a.a, a.b) + crossing(p, b.a, b.b)) % 2
                    != crossing(p, flat.a, flat.b) % 2
        })
    }
}
pub fn simplify(g: Shape, tolerance: f64) -> Shape {
    if g.is_empty() {
        return g;
    }
    let mut simplifier = Simplifier::new(&g);
    for id in 0..simplifier.lines.len() {
        simplifier.simplify_line(id, tolerance);
    }
    let mut lines = simplifier.lines.into_iter();
    fn transform(mut g: Shape, lines: &mut impl Iterator<Item = Line>) -> Shape {
        if g.kind == 2 || g.kind == RING {
            if !g.points.is_empty() {
                let line = lines.next().unwrap();
                g.points = line.result.iter().map(|s| s.a).collect();
                if let Some(s) = line.result.last() {
                    g.points.push(s.b);
                }
                g.inferred_dimensions();
            } else {
                g.dims = 0;
            }
            if g.kind == RING && !g.points.is_empty() && g.points.len() < 4 {
                g.kind = 2;
            }
        } else if g.kind == 3 {
            let mut parts = g.parts.into_iter().map(|p| transform(p, lines));
            let shell = parts.next().unwrap_or_else(|| Shape::empty(RING, 0));
            let mut valid = shell.kind == RING && !shell.is_empty();
            let mut components = vec![shell];
            for hole in parts {
                if hole.is_empty() {
                    continue;
                }
                if hole.kind == RING {
                    components.push(hole);
                } else {
                    valid = false;
                }
            }
            if valid {
                g.parts = components;
                g.dims = 0;
            } else {
                g = Shape::build(components, 7);
            }
        } else if g.kind >= 4 {
            let parts: Vec<_> = g
                .parts
                .into_iter()
                .map(|p| transform(p, lines))
                .filter(|p| !p.is_empty())
                .collect();
            if g.kind == 7 {
                g.parts = parts;
                g.dims = 0;
            } else {
                g = Shape::build(parts, g.kind);
            }
        }
        g
    }
    transform(g, &mut lines)
}
