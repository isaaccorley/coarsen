//! Pure Rust GEOS 3.13.1 coverage simplification. No native geometry libraries.
pub mod edges;
pub mod geom;
mod line_math;
mod predicates;
#[cfg(feature = "python")]
mod python;
pub mod simple_geom;
pub mod topology;
pub mod tpvw;
pub mod validate;
pub mod wkb;

use geom::Geometry;

/// Simplify the entire coverage, including invalid polygonal coverages, like GEOS.
/// Use a Rayon pool's `install` to control parallelism.
pub fn coverage_simplify(geometries: &mut [Geometry], tolerance: f64, simplify_boundary: bool) {
    let mask = vec![false; geometries.len()];
    let coverage = edges::Coverage::extract(geometries, &mask);
    let (rings, _) = tpvw::simplify_selected(&coverage.edges, tolerance, simplify_boundary);
    coverage.rebuild(geometries, &mask, &rings);
}
