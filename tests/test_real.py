"""Projected FTW parcels extracted by the reproducible benchmark runner."""

import json
from pathlib import Path

import numpy as np
import pytest
import shapely

import coarsen

FIXTURES = sorted(Path(__file__).with_name('real').glob('*.json'))


@pytest.mark.parametrize('path', FIXTURES, ids=lambda p: p.stem)
@pytest.mark.parametrize('boundary', [True, False])
@pytest.mark.parametrize('threads', [1, 8])
def test_real_tiles(path, boundary, threads):
    fixture = json.loads(path.read_text())
    geometries = shapely.from_wkb(fixture['input'])
    actual = coarsen.coverage_simplify(geometries, 5.0, simplify_boundary=boundary, threads=threads)
    expected = shapely.coverage_simplify(geometries, 5.0, simplify_boundary=boundary)
    np.testing.assert_array_equal(shapely.to_wkb(actual), shapely.to_wkb(expected))
    if boundary:
        np.testing.assert_array_equal(shapely.to_wkb(actual, hex=True), fixture['output'])
