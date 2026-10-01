# Changelog

## 0.3.0 — 2026-10-01

- Reject malformed short lines, unclosed rings, and invalid multi-geometry members
  at the private WKB boundary before topology simplification.
- Skip spatial index and dependency construction when no coverage edges can change.
- Disable release debug-info stripping to avoid affected macOS 27 import failures.
- Add a logo, concise documentation site, package discovery metadata, and security policy.
- Clarify compatibility and historical benchmark limitations.

## 0.2.0

- Add `simplify(geometry, tolerance, preserve_topology=True)`, a drop-in for
  `shapely.simplify`. The default topology-preserving mode is GEOS 3.13.1's
  TopologyPreservingSimplifier ported to Rust and parallel across geometries;
  `preserve_topology=False` calls `shapely.simplify`.
- Rename `coverage_simplify`'s first parameter to `geometry`, matching Shapely.
