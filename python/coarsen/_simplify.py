"""Broadcasting and WKB transport for the Rust geometry simplifiers."""

import operator

import numpy as np
import numpy.typing as npt
import shapely
from shapely import Geometry

from ._core import topology_wkb


def _ring_paths(geometry: Geometry, path: tuple[int, ...] = ()) -> list[list[int]]:
    if isinstance(geometry, shapely.LinearRing):
        return [list(path)]
    if isinstance(geometry, shapely.GeometryCollection):
        return [p for i, g in enumerate(geometry.geoms) for p in _ring_paths(g, (*path, i))]
    return []


def _restore_rings(geometry: Geometry, paths: list[list[int]]) -> Geometry:
    if [] in paths:
        if geometry.is_empty:
            dimensions = ('Z' if shapely.has_z(geometry) else '') + (
                'M' if shapely.has_m(geometry) else ''
            )
            return shapely.from_wkt(f'LINEARRING {dimensions} EMPTY')
        return shapely.LinearRing(geometry.coords)
    if paths:
        parts = list(geometry.geoms)
        for i, g in enumerate(parts):
            children = [p[1:] for p in paths if p[0] == i]
            if children:
                parts[i] = _restore_rings(g, children)
        return shapely.GeometryCollection(parts)
    return geometry


def simplify(
    geometry: Geometry | npt.ArrayLike,
    tolerance: npt.ArrayLike,
    preserve_topology: bool = True,
    *,
    threads: int | None = None,
    **kwargs,
) -> Geometry | npt.NDArray[np.object_] | None:
    """Simplify using GEOS 3.13.1 vertex choices, in Rust across geometries.

    Geometry and tolerance broadcast together. Scalars return scalar geometries;
    None and NaN tolerances produce None. Supports ufunc ``out`` and ``where``;
    other ufunc keywords raise TypeError. Threads must be positive or None.
    """
    unsupported = kwargs.keys() - {'out', 'where'}
    if unsupported:
        raise TypeError(f'Unsupported simplify keyword(s): {", ".join(sorted(unsupported))}')
    if threads is not None:
        threads = operator.index(threads)
        if threads <= 0:
            raise ValueError('threads must be positive')
    mode = bool(preserve_topology)
    if not mode:
        raise NotImplementedError('Douglas-Peucker polygon repair is not yet implemented')
    array = np.asarray(geometry, dtype=object)
    tolerances = np.asarray(tolerance)
    if not np.can_cast(tolerances.dtype, np.float64, casting='safe'):
        raise TypeError('tolerance must be numeric')
    where = np.asarray(kwargs.get('where', True), dtype=bool)
    out = kwargs.get('out')
    if isinstance(out, tuple):
        if len(out) != 1:
            raise ValueError('out must be a tuple of length 1')
        out = out[0]
    args = [array, tolerances, where]
    if out is not None:
        if not isinstance(out, np.ndarray):
            raise TypeError('out must be an ndarray')
        if out.dtype != object:
            raise TypeError('out must have object dtype')
        if not out.flags.writeable:
            raise ValueError('output array is read-only')
        args.append(out)
    broadcast = np.broadcast_arrays(*args)
    array, tolerances, where = broadcast[:3]
    if out is not None and out.shape != array.shape:
        raise ValueError('non-broadcastable output operand')
    result = np.full(array.shape, None, dtype=object) if out is None else out
    active = np.asarray(where, dtype=bool)
    selected = array[active].copy()
    # Validate input types even for NaN tolerances, as the Shapely ufunc does.
    kinds = shapely.get_type_id(selected)
    selected_tolerances = tolerances[active].astype(float)
    live = (kinds != -1) & ~np.isnan(selected_tolerances)
    if np.any(selected_tolerances[live] < 0):
        raise shapely.errors.GEOSException(
            'IllegalArgumentException: Tolerance must be non-negative'
        )
    geometries = selected[live]
    encoded = shapely.to_wkb(geometries).tolist()
    try:
        batch = topology_wkb(
            encoded,
            selected_tolerances[live].tolist(),
            [_ring_paths(g) for g in geometries],
            threads,
        )
    except ValueError as error:
        if str(error).startswith('IllegalArgumentException:'):
            raise shapely.errors.GEOSException(str(error)) from error
        raise
    decoded = shapely.from_wkb(np.asarray([b for b, _ in batch], dtype=object))
    for i, (_, paths) in enumerate(batch):
        if paths:
            decoded[i] = _restore_rings(decoded[i], paths)
    precision = shapely.get_precision(geometries)
    if np.any(precision > 0):
        decoded = shapely.set_precision(decoded, precision, mode='pointwise')
    decoded = shapely.set_srid(decoded, shapely.get_srid(geometries))
    values = np.full(selected.shape, None, dtype=object)
    values[live] = decoded
    result[active] = values
    return result.item() if result.ndim == 0 and out is None else result
