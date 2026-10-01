"""Drop-in, parallel replacements for shapely.coverage_simplify and shapely.simplify."""

import operator
from typing import overload

import numpy as np
import numpy.typing as npt
import shapely
from shapely import Geometry

from ._core import simplify_wkb
from ._simplify import simplify

__all__ = ['coverage_simplify', 'simplify']
__version__ = '0.3.0'


@overload
def coverage_simplify(
    geometry: Geometry,
    tolerance: npt.ArrayLike,
    *,
    simplify_boundary: bool = True,
    threads: int | None = None,
) -> Geometry: ...


@overload
def coverage_simplify(
    geometry: npt.ArrayLike,
    tolerance: npt.ArrayLike,
    *,
    simplify_boundary: bool = True,
    threads: int | None = None,
) -> npt.NDArray[np.object_]: ...


def coverage_simplify(
    geometry: Geometry | npt.ArrayLike,
    tolerance: npt.ArrayLike,
    *,
    simplify_boundary: bool = True,
    threads: int | None = None,
) -> Geometry | npt.NDArray[np.object_]:
    """Simplify a polygonal coverage with shared edges kept identical.

    Tolerance uses input coordinate units; the removal threshold is tolerance².
    All array elements form one coverage, regardless of array shape. Return the
    original shape (or a Geometry for scalar Geometry input), like Shapely 2.1.
    With simplify_boundary=False, simplify only edges used by exactly two rings.
    Threads must be a positive integer or None (Rayon's default pool).

    The input should form a valid coverage; invalid inputs are not repaired.
    This is the GEOS 3.13.1 algorithm; later GEOS releases can produce other bytes.
    WKB conversion uses Shapely; Rust parses, simplifies, and writes without the GIL.
    """
    if threads is not None:
        threads = operator.index(threads)
        if threads <= 0:
            raise ValueError('threads must be positive')
    scalar = isinstance(geometry, Geometry)
    array = np.asarray(geometry)
    # Shapely temporarily toggles writeability during ufunc calls. Own this
    # pointer array so simultaneous calls cannot mutate each other's flags.
    flat = array.flatten()
    # Match Shapely's collection construction, including omission of None.
    kinds = shapely.get_type_id(flat)
    if np.any((kinds != -1) & (kinds != 3) & (kinds != 6)):
        raise TypeError('One of the Geometry inputs is of incorrect geometry type.')
    encoded = shapely.to_wkb(flat[kinds != -1]).tolist()
    tolerances, boundaries = np.broadcast_arrays(tolerance, simplify_boundary)
    # Shapely's ufunc accepts numeric values but does not coerce strings/objects.
    if (
        not np.can_cast(tolerances.dtype, np.float64, casting='safe')
        or boundaries.dtype.kind != 'b'
    ):
        raise TypeError('tolerance must be numeric and simplify_boundary must be boolean')
    output = []
    for tol, boundary in zip(tolerances.flat, boundaries.flat):
        output.extend(simplify_wkb(encoded, float(tol), bool(boundary), threads))
    parts = shapely.from_wkb(np.asarray(output, dtype=object)).reshape(array.shape)
    precision = shapely.get_precision(flat[kinds != -1])
    if np.any(precision > 0):
        # WKB carries no precision-model metadata. Pointwise mode retains vertex
        # order, including repeated coordinates and collapsed polygonal output.
        grids = np.tile(precision, tolerances.size).reshape(array.shape)
        parts = shapely.set_precision(parts, grids, mode='pointwise')
    return parts.item() if scalar else parts
