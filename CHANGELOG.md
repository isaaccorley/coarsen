# Changelog

## Unreleased — 0.2.0 incomplete

- Rename `coverage_simplify`'s first parameter to `geometry`, matching Shapely.
- Add a private Rust topology-preserving simplification prototype with general
  geometry WKB transport, broadcasting, `out`/`where`, and Rayon parallelism.
- Add seeded WKB parity tests against Shapely 2.1.2 / GEOS 3.13.1.

`coarsen.simplify` is not exported and the package version remains 0.1.0.
Douglas–Peucker polygon validity checking and buffer(0) repair are not implemented.
The private prototype explicitly raises `NotImplementedError` for
`preserve_topology=False`; it never substitutes an approximate simplifier.
