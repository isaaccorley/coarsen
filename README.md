# coarsen

<img src="https://raw.githubusercontent.com/taylor-geospatial/coarsen/main/docs/assets/logo.png" alt="coarsen — detailed polygon boundaries reduced to clean edges" width="600">

[![CI](https://github.com/taylor-geospatial/coarsen/actions/workflows/ci.yml/badge.svg)](https://github.com/taylor-geospatial/coarsen/actions/workflows/ci.yml)
[![PyPI](https://img.shields.io/pypi/v/coarsen)](https://pypi.org/project/coarsen/)
[![Python](https://img.shields.io/pypi/pyversions/coarsen)](https://pypi.org/project/coarsen/)
[![License: LGPL-2.1-or-later](https://img.shields.io/badge/license-LGPL--2.1--or--later-blue)](LICENSE)

Parallel geometry simplification for **Shapely**, powered by **Rust**.
Reduce polygon and line vertices, preserve topology, and keep shared polygon
boundaries aligned. Useful for parcel maps, land-cover polygons, and large vector datasets.

[Documentation](https://research.taylorgeospatial.org/coarsen/) · [API](https://research.taylorgeospatial.org/coarsen/api/) · [Benchmarks](https://research.taylorgeospatial.org/coarsen/performance/) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)

## Install

```sh
pip install coarsen
```

Requires Python ≥3.10, NumPy, and Shapely ≥2.1. The Rust core links neither GEOS
nor PROJ; the Python interface uses Shapely for geometry conversion. Building from
source requires Rust ≥1.85 and maturin.

## Simplify shared boundaries

Use `coverage_simplify` for a polygon coverage: polygons with non-overlapping
interiors and exactly matching shared edges. All input polygons are processed
together so their simplified boundaries remain aligned.

```python
import coarsen
import shapely

polygons = [
    shapely.Polygon([(0, 0), (5, 0), (5.1, 2), (5, 5), (0, 5)]),
    shapely.Polygon([(5, 0), (10, 0), (10, 5), (5, 5), (5.1, 2)]),
]
result = coarsen.coverage_simplify(polygons, 1.0, threads=8)

# Keep the outer coverage boundary unchanged.
inner_only = coarsen.coverage_simplify(polygons, 1.0, simplify_boundary=False)
```

Tolerance uses coordinate units. Project longitude/latitude data to a suitable
projected CRS before using a tolerance in metres. Invalid coverages are not repaired;
check them with `shapely.coverage_is_valid` when validity is uncertain.

## Simplify individual geometries

```python
line = shapely.LineString([(0, 0), (1, 0.1), (2, 0)])
result = coarsen.simplify(line, 0.2)
assert list(result.coords) == [(0.0, 0.0), (2.0, 0.0)]
```

| Function | Use for | Parallel work |
| --- | --- | --- |
| `coverage_simplify` | Matching polygon boundaries | Independent groups of shared edges |
| `simplify` | Individual geometries of any type | Independent geometries |

`simplify` preserves each geometry's topology by default. It supports broadcasting,
`None`, and `out`/`where`; it does not preserve shared boundaries between separate
array elements. `preserve_topology=False` delegates to Shapely. See the
[API reference](docs/api.md) for compatibility details and threading guidance.

## Performance and compatibility

The recorded coverage benchmark processes 260,200 polygons across three projected
FTW tiles. Eight threads took **6.42–9.65 s per tile**, versus **92.03–94.71 s**
for Shapely, a **9.5–14.5×** speedup on that workload. These are historical shared-node
measurements, not a guarantee for other hardware or geometry distributions.
[Methodology, raw samples, and limitations](docs/performance.md).

The algorithms port GEOS **3.13.1**. Tests compare unnormalized WKB bytes against
Shapely 2.1.2 at one and eight threads using original GEOS fixtures, seeded random
geometries, and real projected parcels. Parity is established for the tested cases;
other GEOS versions and untested inputs can differ. Z/M behavior follows the reference
algorithm and may drop dimensions. Nonfinite coordinates used in indexes are rejected.

## Development

```sh
make install
make check
make docs-serve
```

Requires [uv](https://docs.astral.sh/uv/) and [Rust](https://rustup.rs/).
`make check` runs Rust formatting, Clippy, Rust and Python tests, Ruff, ty, and the
strict documentation build. Run `make build` after editing Rust.
See [CONTRIBUTING.md](CONTRIBUTING.md) for release and platform notes.

## License and attribution

[LGPL-2.1-or-later](LICENSE). Derived from GEOS work by Martin Davis, Paul Ramsey,
and other contributors; see [NOTICE](NOTICE).
