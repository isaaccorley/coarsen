---
description: Reproducible coarsen coverage simplification benchmarks, WKB parity checks, threading costs, and memory limitations.
---
# Performance and verification

## Recorded coverage benchmark

Three FTW tiles containing 260,200 polygons and 42.4 million vertices, simplified
at a 5 m tolerance in each tile's local UTM zone.

| Tile | Polygons | Shapely (s) | coarsen 1 thread (s) | coarsen 8 threads (s) |
| --- | ---: | ---: | ---: | ---: |
| 34UEU_0_0 | 42,858 | 92.03 | 22.74 | 9.65 |
| 22JBM_0_0 | 33,919 | 94.71 | 21.94 | 8.24 |
| 48RWV_0_0 | 183,423 | 92.90 | 17.80 | 6.42 |

Median times across three runs, including geometry conversion and simplification.
Every result was checked against Shapely's output. Performance varies with hardware
and geometry; see the [benchmark details](https://github.com/isaaccorley/coarsen/blob/main/benchmarks/README.md)
for raw results and reproduction instructions.

## Scaling limits

Coverage parallelism depends on disjoint edge envelopes. Overlapping envelopes
create ordered dependencies and reduce concurrency. The neighbor lists can approach
quadratic size on pathological coverages. A single large geometry in `simplify`
runs serially; multiple geometries can run in parallel. Component-jump checks scan
other lines in a geometry, which can be costly for large collections.

Owned WKB, parsed points, indexes, and output buffers coexist during a call.
Choose thread counts using representative data and peak memory measurements.
For untrusted geometry, use worker-level CPU/memory limits as described in the
[security policy](https://github.com/isaaccorley/coarsen/blob/main/SECURITY.md).

## Parity tests

`make check` includes byte comparisons against GEOS 3.13.1 from Shapely 2.1.2:
original GEOS fixtures, real UTM parcels, seeded random geometries, holes, empty
inputs, dimensions, precision metadata, and one/eight-thread determinism.
Tests also exercise malformed WKB at the private native boundary. Passing these
cases is evidence of compatibility, not a proof for every floating-point input.
