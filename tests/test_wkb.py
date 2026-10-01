"""The private byte boundary must reject malformed input without a Rust panic."""

import numpy as np
import pytest
import shapely

from coarsen._core import simplify_wkb


@pytest.mark.parametrize('byte_order', [0, 1])
@pytest.mark.parametrize('flavor', ['iso', 'extended'])
@pytest.mark.parametrize('dimension', ['', 'Z', 'M', 'ZM'])
def test_wkb_formats(byte_order, flavor, dimension):
    extra = {'': '', 'Z': ' 3', 'M': ' 5', 'ZM': ' 3 5'}[dimension]
    coords = ','.join(f'{x} {y}{extra}' for x, y in [(0, 0), (2, 0), (2, 2), (0, 0)])
    geometry = shapely.from_wkt(f'POLYGON {dimension} (({coords}))')
    encoded = shapely.to_wkb(geometry, byte_order=byte_order, flavor=flavor)
    actual = shapely.from_wkb(simplify_wkb([encoded], 1, True, 1))
    expected = shapely.coverage_simplify([geometry], 1)
    np.testing.assert_array_equal(shapely.to_wkb(actual), shapely.to_wkb(expected))


@pytest.mark.parametrize(
    'encoded',
    [
        b'',
        b'\x02',
        bytes.fromhex('0103000000FFFFFFFF'),
        bytes.fromhex('010600000001000000010600000000000000'),
    ],
)
def test_bad_wkb(encoded):
    with pytest.raises(ValueError):
        simplify_wkb([encoded], 1, True, 1)


def test_truncated_and_trailing_wkb():
    encoded = shapely.to_wkb(shapely.box(0, 0, 1, 1))
    for bad in [encoded[:i] for i in range(len(encoded))] + [encoded + b'\x00']:
        with pytest.raises(ValueError):
            simplify_wkb([bad], 1, True, 1)


def test_nonfinite_xy_rejected():
    with np.errstate(invalid='ignore'):
        geometry = shapely.from_wkt('POLYGON ((0 0,1 0,NaN 1,0 1,0 0))')
    with pytest.raises(ValueError, match='nonfinite XY'):
        simplify_wkb([shapely.to_wkb(geometry)], 1, True, 1)


def test_topology_rejects_malformed_geometry_structure():
    import struct

    from coarsen._core import topology_wkb

    short_line = struct.pack('<BII2d', 1, 2, 1, 0, 0)
    normal_line = shapely.to_wkb(shapely.LineString([(0, 0), (1, 0.1), (2, 0)]))
    collection = struct.pack('<BII', 1, 7, 2) + normal_line + short_line
    wrong_child = struct.pack('<BII', 1, 4, 1) + normal_line
    short_ring = struct.pack('<BIII2d', 1, 3, 1, 1, 0, 0)
    unclosed_ring = struct.pack('<BIII6d', 1, 3, 1, 3, 0, 0, 1, 0, 1, 1)
    for encoded in [short_line, collection, wrong_child, short_ring, unclosed_ring]:
        with pytest.raises(ValueError):
            topology_wkb([encoded], [1], [[]], 1)


def test_topology_bounds_and_nesting():
    import struct

    from coarsen._core import topology_wkb

    encoded = shapely.to_wkb(shapely.LineString([(0, 0), (1, 1)]))
    for bad in [encoded[:i] for i in range(len(encoded))] + [encoded + b'\x00']:
        with pytest.raises(ValueError):
            topology_wkb([bad], [1], [[]], 1)
    nested = struct.pack('<BII', 1, 7, 1) * 256 + encoded
    with pytest.raises(ValueError, match='nesting'):
        topology_wkb([nested], [1], [[]], 1)
