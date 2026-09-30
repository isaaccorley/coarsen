# coarsen

Fast, multithreaded polygon coverage simplification. A pure Rust port of
GEOS 3.13.1's CoverageSimplifier, exposed through a thin Shapely wrapper.
The Rust core links neither GEOS nor PROJ and performs no dynamic library loading.

```sh
pip install coarsen
```

```python
import coarsen
import shapely

polygons = [shapely.box(0, 0, 10, 10), shapely.box(10, 0, 20, 10)]
result = coarsen.coverage_simplify(polygons, 5.0, threads=8)
```

`coverage_simplify(geometry, tolerance, *, simplify_boundary=True, threads=None)`
returns a NumPy object array with the input shape; scalar geometry input returns
a scalar geometry, matching Shapely. `simplify_boundary=False` implements GEOS
`simplifyInner`. `threads=None` uses Rayon's default pool; a positive integer
creates a pool for that call. Rust parses, simplifies, and writes WKB without the GIL.
Tolerance uses input coordinate units; project longitude/latitude data before
using a tolerance in metres. Requires Python ≥3.10, NumPy, and Shapely ≥2.1.

The parity target is **GEOS 3.13.1**, bundled in Shapely 2.1.2's reference wheels.
Tests compare WKB bytes without normalization at one and eight threads, including
both boundary modes, the original 174 fixtures, real UTM parcels, Voronoi cells,
grids, holes, MultiPolygons, empty input, zero tolerance, and Z/M coordinates.
Z/M rebuilding follows GEOS's quirks. Other GEOS versions can produce different
results. Parity is verified on these cases, not a proof for every floating-point
input. Nonfinite XY coordinates are rejected. Invalid coverages are not repaired.
The Rust library retains coverage validation; no Python invalid-edges helper is
exposed because the port computes an invalid-polygon mask rather than edge geometry.

The requested 0.2.0 general `simplify` API is still under development and is not
exported. A private topology-preserving prototype and parity tests are present;
Douglas–Peucker polygon validity checking and buffer(0) repair are not implemented.
See the [implementation status](CONTRIBUTING.md#unreleased-general-simplifier).
The package version remains 0.1.0; it is not yet a drop-in for `shapely.simplify`.

| Tile | Polygons | Shapely (s) | coarsen 1 thread (s) | coarsen 8 threads (s) | Speedup (8) |
| --- | ---: | ---: | ---: | ---: | ---: |
| 34UEU_0_0 | 42,858 | 92.03 | 22.74 | 9.65 | 9.5× |
| 22JBM_0_0 | 33,919 | 94.71 | 21.94 | 8.24 | 11.5× |
| 48RWV_0_0 | 183,423 | 92.90 | 17.80 | 6.42 | 14.5× |

Measured timings cover the complete public API, including WKB conversion, at
5 m in each tile's UTM CRS. See [benchmark details](benchmarks/README.md).

LGPL-2.1-or-later. Derived from GEOS work by Martin Davis, Paul Ramsey, and other
contributors; see [NOTICE](NOTICE) and [LICENSE](LICENSE). Build, test, and release
instructions are in [CONTRIBUTING.md](CONTRIBUTING.md).
