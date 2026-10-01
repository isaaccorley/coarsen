---
title: Geometry simplification for Shapely
description: Simplify Shapely polygons and lines in parallel with Rust, preserving topology and shared coverage boundaries.
---
# Geometry simplification with coarsen

<img class="brand" src="assets/logo.png" alt="coarsen polygon simplification logo">

coarsen reduces geometry vertices using Rust implementations of GEOS 3.13.1's
coverage and topology-preserving simplifiers, with a small Shapely interface.

```sh
pip install coarsen
```

Requires Python ≥3.10, NumPy, and Shapely ≥2.1. Source builds also need Rust ≥1.85.

## Choose an operation

| Input | Operation | What stays aligned |
| --- | --- | --- |
| Non-overlapping polygons with matching shared edges | `coverage_simplify` | Shared polygon boundaries |
| Lines, polygons, points, or collections | `simplify` | Topology within each geometry |

```python
import coarsen
import shapely

coverage = [
    shapely.Polygon([(0, 0), (5, 0), (5.1, 2), (5, 5), (0, 5)]),
    shapely.Polygon([(5, 0), (10, 0), (10, 5), (5, 5), (5.1, 2)]),
]
assert shapely.coverage_is_valid(coverage)
result = coarsen.coverage_simplify(coverage, 1.0, threads=4)
assert shapely.coverage_is_valid(result)

line = shapely.LineString([(0, 0), (1, 0.1), (2, 0)])
assert len(coarsen.simplify(line, 0.2).coords) == 2
```

## Coordinate units matter

The library does not project coordinates or infer a CRS. A tolerance of `1` means
one input coordinate unit. Reproject geographic coordinates before working in metres.
Coverage simplification uses a triangle-area threshold of tolerance squared;
the general simplifier uses point-to-segment distance.

## Compatibility has a specific target

The parity reference is Shapely 2.1.2 with GEOS 3.13.1. Tests compare WKB bytes
without normalization at one and eight threads. Newer GEOS algorithms may differ.
Invalid coverages are not repaired, and Z/M dimensions may be dropped as in GEOS.
Use the [API notes](api.md) for array and metadata behavior, and
[performance notes](performance.md) for measured costs and scaling limits.

The Rust core does not link GEOS or PROJ. The Python interface still uses Shapely
and its GEOS dependency for geometry conversion and non-topology-preserving simplification.

[Source code](https://github.com/taylor-geospatial/coarsen) ·
[PyPI](https://pypi.org/project/coarsen/) ·
[Security policy](https://github.com/taylor-geospatial/coarsen/blob/main/SECURITY.md) ·
[LGPL-2.1-or-later](https://github.com/taylor-geospatial/coarsen/blob/main/LICENSE)
