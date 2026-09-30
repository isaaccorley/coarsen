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
