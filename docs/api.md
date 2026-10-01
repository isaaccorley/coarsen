---
description: Python API for coarsen.coverage_simplify and coarsen.simplify, including tolerance, threading, broadcasting, and Shapely compatibility.
---
# Python API

## `coverage_simplify`

```python
coarsen.coverage_simplify(
    geometry, tolerance, *, simplify_boundary=True, threads=None
)
```

Simplify Polygon/MultiPolygon inputs as one coverage. Shared edges are simplified
once and reused. Array dimensions do not partition a coverage into independent groups.

| Parameter | Meaning |
| --- | --- |
| `geometry` | A polygon, multipolygon, or array-like of polygonal geometries |
| `tolerance` | Simplification tolerance in coordinate units; normally a numeric scalar |
| `simplify_boundary` | `False` freezes exterior edges and simplifies edges shared by exactly two rings |
| `threads` | Positive integer for a per-call pool; `None` uses Rayon's default pool |

Returns a scalar geometry for scalar Shapely input, otherwise a NumPy object array
with the input shape. Like Shapely 2.1, missing entries are omitted during collection
construction; inputs containing `None` and array-valued tolerances can consequently
fail the final reshape. This is not elementwise tolerance broadcasting.
Use a scalar tolerance for a complete coverage.

Negative, NaN, and infinite coverage tolerances follow the reference GEOS behavior;
prefer finite, nonnegative values for meaningful results. Other geometry types raise
`TypeError`. Nonfinite XY coordinates raise `ValueError`. No `out` or `where` keywords.
Precision-grid metadata is restored after WKB transport. Coverage output does not
promise preservation of SRID metadata; retain your CRS separately.

## `simplify`

```python
coarsen.simplify(
    geometry, tolerance, preserve_topology=True, *, threads=None, **kwargs
)
```

Simplify individual geometries, parallel across array elements. Accepts every
Shapely geometry type. Geometry and tolerance broadcast together using NumPy rules.

| Parameter | Meaning |
| --- | --- |
| `geometry` | A Shapely geometry, `None`, or array-like |
| `tolerance` | Numeric scalar or array broadcastable with geometry |
| `preserve_topology` | `True` uses Rust; `False` delegates to `shapely.simplify` |
| `threads` | Positive integer or `None`; applies to the Rust path |
| `out` | Writable object ndarray, or a one-element tuple containing it |
| `where` | Boolean mask selecting output entries |

Scalar inputs return a scalar; broadcast inputs return an object array. `None` and
NaN tolerances produce `None`. Negative tolerances on active, nonmissing inputs raise
`shapely.errors.GEOSException`. SRID and precision-grid metadata are restored.
LinearRing identity is restored after transport. Z/M reconstruction follows GEOS
3.13.1 and may drop coordinate dimensions.

`where=False` leaves `out` entries unchanged. Without `out`, skipped entries are
initialized to `None`; Shapely may leave them uninitialized. Other ufunc keywords
such as `casting`, `dtype`, and `order` are not supported by this wrapper.

```python
import numpy as np
import shapely
import coarsen

lines = [shapely.LineString([(0, 0), (1, 0.1), (2, 0)])] * 2
output = np.full(2, None, dtype=object)
coarsen.simplify(lines, [0.2, 0.5], out=output, where=[True, False])
assert output[1] is None
```

## Threads and memory

`threads=None` uses Rayon's process-wide pool. Set `RAYON_NUM_THREADS` before the
first Rust call to control its size. An explicit count creates and destroys a pool
for that call, so it adds overhead on small inputs. Avoid oversubscription when an
outer process or thread pool already schedules multiple calls.

The wrapper copies geometry pointers and WKB before releasing the GIL. Parsing,
index construction, and simplification run in Rust without the GIL; Python/Shapely
conversion still costs time and memory. Coverage inputs cannot be arbitrarily
chunked without losing shared-edge constraints across chunks.
