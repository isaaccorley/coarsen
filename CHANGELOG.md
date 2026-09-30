# Changelog

## 0.2.0

- Add `simplify(geometry, tolerance, preserve_topology=True)`, a drop-in for
  `shapely.simplify`. The default topology-preserving mode is GEOS 3.13.1's
  TopologyPreservingSimplifier ported to Rust and parallel across geometries;
  `preserve_topology=False` calls `shapely.simplify`.
- Rename `coverage_simplify`'s first parameter to `geometry`, matching Shapely.
