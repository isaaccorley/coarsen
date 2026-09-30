"""Byte parity against the actual GEOS 3.13.1 implementation, without normalization."""

import json
from pathlib import Path

import numpy as np
import pytest
import shapely
from shapely import MultiPoint, MultiPolygon, Polygon, box

import coarsen

CASES = json.loads(Path(__file__).with_name('golden.json').read_text())


@pytest.fixture(scope='session', autouse=True)
def reference_version():
    assert shapely.geos_version == (3, 13, 1), shapely.geos_version_string


def assert_parity(geometries, tolerance, boundary=True, threads=1):
    expected = shapely.coverage_simplify(geometries, tolerance, simplify_boundary=boundary)
    actual = coarsen.coverage_simplify(
        geometries, tolerance, simplify_boundary=boundary, threads=threads
    )
    assert type(actual) is type(expected)
    np.testing.assert_array_equal(shapely.to_wkb(actual), shapely.to_wkb(expected))
    return actual


@pytest.mark.parametrize('case', CASES, ids=lambda c: c['name'])
@pytest.mark.parametrize('boundary', [True, False])
@pytest.mark.parametrize('threads', [1, 8])
def test_original_fixtures(case, boundary, threads):
    # Includes invalid coverages: simplification must not invoke repair/filtering.
    geometries = shapely.from_wkb([bytes.fromhex(s) for s in case['input']])
    assert_parity(geometries, case['tolerance'], boundary, threads)


def synthetic():
    grid = [box(x, y, x + 1, y + 1) for x in range(5) for y in range(5)]
    rng = np.random.default_rng(983)
    voronoi = shapely.get_parts(shapely.voronoi_polygons(MultiPoint(rng.random((80, 2)) * 20)))
    hole = Polygon(
        [(0, 0), (10, 0), (10, 10), (0, 10)],
        [[(2, 2), (2, 8), (5, 8.1), (8, 8), (8, 2)]],
    )
    shared = [
        Polygon([(0, 0), (5, 0), (5.1, 2), (5, 5), (0, 5)]),
        Polygon([(5, 0), (10, 0), (10, 5), (5, 5), (5.1, 2)]),
    ]
    return [
        grid,
        voronoi,
        [hole],
        [hole, Polygon(hole.interiors[0])],
        shared,
        [MultiPolygon([grid[0], grid[-1]]), grid[1]],
        [],
        [Polygon(), MultiPolygon()],
        shapely.from_wkt(['MULTIPOLYGON (EMPTY, ((0 0, 1 0, 0 1, 0 0)))']),
    ]


@pytest.mark.parametrize('geometries', synthetic())
@pytest.mark.parametrize('tolerance', [0, 0.1, 1, 5, -1, np.nan, np.inf])
@pytest.mark.parametrize('boundary', [True, False])
@pytest.mark.parametrize('threads', [1, 8])
def test_synthetic(geometries, tolerance, boundary, threads):
    assert_parity(geometries, tolerance, boundary, threads)


@pytest.mark.parametrize(
    'wkt',
    [
        'POLYGON Z ((0 0 1, 2 0 2, 2 2 3, 1 2 4, 0 0 1))',
        'POLYGON M ((0 0 1, 2 0 2, 2 2 3, 1 2 4, 0 0 1))',
        'POLYGON ZM ((0 0 1 5, 2 0 2 6, 2 2 3 7, 1 2 4 8, 0 0 1 5))',
        'POLYGON Z ((0 0 NaN, 1 0 NaN, 0 1 NaN, 0 0 NaN))',
        'POLYGON M ((0 0 1, 0 0 1, 0 0 1, 0 0 1))',
        'POLYGON Z EMPTY',
        'POLYGON M EMPTY',
        'MULTIPOLYGON Z EMPTY',
    ],
)
@pytest.mark.parametrize('boundary', [True, False])
@pytest.mark.parametrize('tolerance', [0, 1])
def test_dimensions(wkt, boundary, tolerance):
    assert_parity(shapely.from_wkt(wkt), tolerance, boundary)


@pytest.mark.parametrize(
    'value',
    [
        None,
        [None],
        [box(0, 0, 1, 1), None],
        [],
        box(0, 0, 1, 1),
        [[box(0, 0, 1, 1)], [box(1, 0, 2, 1)]],
    ],
)
@pytest.mark.parametrize('tolerance', [1, [1], [1, 2], [], 'a'])
def test_array_semantics(value, tolerance):
    try:
        shapely.coverage_simplify(value, tolerance)
    except Exception as error:
        with pytest.raises(type(error)):
            coarsen.coverage_simplify(value, tolerance)
    else:
        assert_parity(value, tolerance)


@pytest.mark.parametrize(
    'value',
    [
        shapely.Point(),
        shapely.LineString([(0, 0), (1, 1)]),
        shapely.GeometryCollection(),
        'bad',
        12,
    ],
)
def test_wrong_geometry(value):
    with pytest.raises(TypeError):
        coarsen.coverage_simplify([value], 1)


@pytest.mark.parametrize('threads,error', [(0, ValueError), (-1, ValueError), (1.5, TypeError)])
def test_threads(threads, error):
    with pytest.raises(error):
        coarsen.coverage_simplify([], 1, threads=threads)


def test_default_pool_and_srid():
    assert_parity(shapely.set_srid(box(0, 0, 10, 10), 4326), 1, threads=None)


def test_boundary_is_frozen():
    coverage = synthetic()[4]
    result = assert_parity(coverage, 5, False, 8)
    assert shapely.union_all(result).equals(shapely.union_all(coverage))


def test_no_shapely_simplification_fallback(monkeypatch):
    def fail(*args, **kwargs):
        raise AssertionError('GEOS simplification was called')

    monkeypatch.setattr(shapely, 'coverage_simplify', fail)
    monkeypatch.setattr(shapely.lib, 'coverage_simplify', fail)
    result = coarsen.coverage_simplify(synthetic()[4], 1, threads=8)
    assert result.shape == (2,)


@pytest.mark.parametrize('boundary', [True, False])
@pytest.mark.parametrize('reverse', [True, False])
def test_mixed_shared_z(boundary, reverse):
    geometries = shapely.from_wkt(
        [
            'POLYGON Z ((0 0 1,5 0 2,5.1 2 3,5 5 4,0 5 5,0 0 1))',
            'POLYGON ((5 0,10 0,10 5,5 5,5.1 2,5 0))',
        ]
    )
    assert_parity(geometries[::-1] if reverse else geometries, 1, boundary)


@pytest.mark.parametrize('boundary', [0, 1, None, [], 'yes', 0.5, [True], [False]])
def test_boundary_argument(boundary):
    geometry = [box(0, 0, 1, 1)]
    try:
        shapely.coverage_simplify(geometry, 1, simplify_boundary=boundary)
    except Exception as error:
        with pytest.raises(type(error)):
            coarsen.coverage_simplify(geometry, 1, simplify_boundary=boundary)
    else:
        assert_parity(geometry, 1, boundary)


def test_simultaneous_calls():
    from concurrent.futures import ThreadPoolExecutor

    geometries = synthetic()[1]
    with ThreadPoolExecutor(max_workers=3) as executor:
        results = list(executor.map(lambda n: assert_parity(geometries, 1, threads=n), [1, 2, 8]))
    for result in results[1:]:
        np.testing.assert_array_equal(shapely.to_wkb(results[0]), shapely.to_wkb(result))


def test_precision_metadata():
    geometries = [
        shapely.set_precision(box(0, 0, 10, 10), 1),
        shapely.set_precision(box(10, 0, 20, 10), 0.5),
    ]
    result = assert_parity(geometries, 1)
    np.testing.assert_array_equal(shapely.get_precision(result), [1, 0.5])


@pytest.mark.parametrize('tolerance', [np.float32(1), np.uint64(2), np.longdouble(1)])
def test_numpy_tolerances(tolerance):
    geometry = [box(0, 0, 1, 1)]
    try:
        shapely.coverage_simplify(geometry, tolerance)
    except Exception as error:
        with pytest.raises(type(error)):
            coarsen.coverage_simplify(geometry, tolerance)
    else:
        assert_parity(geometry, tolerance)
