"""Unnormalized WKB parity for the general geometry simplifier."""

import inspect
import json
from pathlib import Path

import numpy as np
import pytest
import shapely
from shapely import GeometryCollection, LinearRing, LineString, MultiPolygon, Point, Polygon
from shapely.affinity import translate

import coarsen
from coarsen._simplify import simplify


def corpus():
    rng = np.random.default_rng(98243)
    geometries = []
    for i in range(160):
        points = rng.normal(size=(int(rng.integers(4, 100)), 2)).cumsum(axis=0)
        line = LineString(points)
        angles = np.sort(rng.uniform(0, 2 * np.pi, 40))
        radius = rng.uniform(5, 10, 40)
        shell = np.column_stack([np.cos(angles) * radius, np.sin(angles) * radius])
        hole = shapely.box(-0.01, -1, 0.01, 1) if i % 2 else shapely.box(-1, -1, 1, 1)
        polygon = Polygon(shell, [hole.exterior.coords])
        geometries.extend(
            [
                line,
                LinearRing(points),
                Polygon(points),
                polygon,
                MultiPolygon([polygon, translate(polygon, 25, 0)]),
                GeometryCollection([polygon, translate(line, 25, 0), Point(100, 100)]),
                shapely.force_3d(line, z=5),
                GeometryCollection([LineString(points), LineString(points[::-1] + 0.1)]),
            ]
        )
    return np.asarray(geometries, dtype=object)


CORPUS = corpus()
TOLERANCES = [0, 0.01, 0.5, 1.2, 5, 100, np.inf]


def parity(geometry, tolerance, threads=1, **kwargs):
    expected = shapely.simplify(geometry, tolerance, **kwargs)
    actual = simplify(geometry, tolerance, threads=threads, **kwargs)
    assert type(actual) is type(expected)
    np.testing.assert_array_equal(shapely.to_wkb(actual), shapely.to_wkb(expected))
    return actual


@pytest.mark.parametrize('threads', [1, 8])
@pytest.mark.parametrize('tolerance', TOLERANCES)
def test_randomized(tolerance, threads):
    parity(CORPUS, tolerance, threads)


@pytest.mark.parametrize('path', sorted(Path(__file__).with_name('real').glob('*.json')))
@pytest.mark.parametrize('tolerance', [1.2, 5])
@pytest.mark.parametrize('threads', [1, 8])
def test_projected_fields(path, tolerance, threads):
    geometry = shapely.from_wkb(json.loads(path.read_text())['input'])
    parity(geometry, tolerance, threads)


@pytest.mark.parametrize(
    'wkt',
    [
        'POINT EMPTY',
        'POINT Z EMPTY',
        'POINT M EMPTY',
        'POINT ZM EMPTY',
        'POINT M (1 2 9)',
        'LINESTRING EMPTY',
        'LINESTRING Z EMPTY',
        'LINESTRING M EMPTY',
        'LINEARRING EMPTY',
        'LINEARRING Z EMPTY',
        'LINEARRING M EMPTY',
        'LINEARRING ZM EMPTY',
        'POLYGON EMPTY',
        'POLYGON Z EMPTY',
        'MULTIPOINT EMPTY',
        'MULTILINESTRING EMPTY',
        'MULTIPOLYGON EMPTY',
        'GEOMETRYCOLLECTION EMPTY',
        'GEOMETRYCOLLECTION (POINT EMPTY, LINESTRING EMPTY, POLYGON EMPTY)',
        'GEOMETRYCOLLECTION (POINT (1 2), LINESTRING EMPTY, POLYGON EMPTY)',
        'MULTILINESTRING (EMPTY, (0 0, 1 1))',
        'MULTIPOLYGON (EMPTY, ((0 0, 1 0, 0 1, 0 0)))',
        'MULTIPOINT (EMPTY, (1 2))',
        'LINESTRING (1 2, 1 2)',
        'POLYGON ((1 2, 1 2, 1 2, 1 2))',
        'POLYGON ((0 0, 1 1, 0 0))',
        'POLYGON ((0 0, 1 1, 0 0), (0 0, 1 1, 2 0, 0 0))',
        'POLYGON ((0 0, 4 0, 4 4, 0 0), (1 1, 2 2, 1 1))',
        'POLYGON ((0 0, 4 4, 0 4, 4 0, 0 0))',
        'POLYGON ((0 0, 4 0, 4 4, 2 2, 0 4, 2 2, 0 0))',
        'POLYGON Z ((0 0 NaN, 2 0 2, 2 2 3, 0 0 NaN))',
        'LINESTRING Z (0 0 NaN, 1 1 2)',
        'LINESTRING ZM (0 0 5 1, 1 1 6 2, 2 2 7 3)',
    ],
)
@pytest.mark.parametrize('tolerance', [0, 1.2, 5, 100])
def test_edge_cases(wkt, tolerance):
    parity(shapely.from_wkt(wkt), tolerance)


@pytest.mark.parametrize(
    'geometry',
    [None, Point(1, 2), [None], [], [[Point(1, 2)], [None]], np.array(Point(1, 2))],
)
@pytest.mark.parametrize('tolerance', [0, np.nan, [0, 1, 2], np.array([[0], [1]])])
def test_broadcasting(geometry, tolerance):
    try:
        shapely.simplify(geometry, tolerance)
    except Exception as error:
        with pytest.raises(type(error)):
            simplify(geometry, tolerance)
    else:
        parity(geometry, tolerance)


def test_signatures():
    for actual, expected in [
        (simplify, shapely.simplify),
        (coarsen.coverage_simplify, shapely.coverage_simplify),
    ]:
        ours = inspect.signature(actual).parameters
        for name, parameter in inspect.signature(expected).parameters.items():
            assert ours[name].kind == parameter.kind
            assert ours[name].default == parameter.default
        assert ours['threads'].kind == inspect.Parameter.KEYWORD_ONLY
    parity(CORPUS[0], 1)
    coarsen.coverage_simplify(geometry=shapely.box(0, 0, 5, 5), tolerance=1)
    with pytest.raises(TypeError):
        coarsen.coverage_simplify(geometries=[], tolerance=1)  # ty: ignore[no-matching-overload]


def test_out_where_and_aliasing():
    geometry = CORPUS[:6].reshape(2, 3)
    expected, actual = geometry.copy(), geometry.copy()
    mask = [[True, False, True], [False, True, False]]
    shapely.simplify(geometry, [0, 1, 5], out=expected, where=mask)
    assert simplify(actual, [0, 1, 5], out=(actual,), where=mask) is actual
    np.testing.assert_array_equal(shapely.to_wkb(actual), shapely.to_wkb(expected))
    parity(geometry, 1, where=mask)
    with pytest.raises(TypeError, match='Unsupported'):
        simplify(geometry, 1, casting='unsafe')


def test_rings_and_metadata():
    ring = LinearRing([(0, 0, 2), (0, 10, 3), (5, 10, 4), (10, 0, 5)])
    for geometry in [ring, GeometryCollection([Point(), GeometryCollection([ring])])]:
        geometry = shapely.set_srid(shapely.set_precision(geometry, 0.5), 4326)
        result = parity(geometry, 5)
        assert shapely.get_srid(result) == 4326
        assert shapely.get_precision(result) == 0.5


def test_no_geos_simplifier(monkeypatch):
    def fail(*args, **kwargs):
        raise AssertionError('GEOS simplifier called')

    for target, name in [
        (shapely, 'simplify'),
        (shapely.lib, 'simplify'),
        (shapely.lib, 'simplify_preserve_topology'),
    ]:
        monkeypatch.setattr(target, name, fail)
    result = simplify(CORPUS[:8], 1, threads=8)
    assert isinstance(result, np.ndarray)
    assert result.shape == (8,)


@pytest.mark.parametrize('coordinate', [(np.nan, 1), (np.inf, 1), (1, -np.inf)])
def test_nonfinite_index_coordinates(coordinate):
    with np.errstate(invalid='ignore'):
        line = LineString([(0, 0), coordinate, (2, 2)])
    for function in [shapely.simplify, simplify]:
        with pytest.raises(shapely.errors.GEOSException, match='Non-finite envelope bounds'):
            function(line, 1)
    parity(Point(*coordinate), 1)


def test_large_finite_coordinates():
    parity(LineString([(0, 0), (1e200, 1), (2, 2)]), 1)


def test_unimplemented_mode_is_explicit():
    with pytest.raises(NotImplementedError, match='polygon repair'):
        simplify(CORPUS[0], 1, preserve_topology=False)
