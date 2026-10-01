"""Measure public topology-preserving simplification on complete projected tiles."""

import gc
import hashlib
import json
import platform
import time
from pathlib import Path

import numpy as np
import pyarrow.parquet as pq
import shapely
from pyproj import Transformer

from coarsen import simplify


def main() -> None:
    assert shapely.__version__ == '2.1.2'
    assert shapely.geos_version == (3, 13, 1)
    print(json.dumps(dict(host=platform.node(), python=platform.python_version())), flush=True)
    root = Path('/projects/bgtj/isaaccorley/ftw-polygons/2025')
    for tile in ['34UEU_0_0', '22JBM_0_0']:
        raw = pq.read_table(root / f'{tile}.parquet', columns=['geometry'])['geometry'].to_numpy()
        geometry = shapely.from_wkb(raw)
        epsg = int(f'32{6 if tile[2] >= "N" else 7}{tile[:2]}')
        transform = Transformer.from_crs(4326, epsg, always_xy=True)
        geometry = shapely.transform(geometry, transform.transform, interleaved=False)
        for tolerance in [1.2, 5.0]:
            expected = None
            for engine, threads in [('shapely', 0), ('coarsen-1', 1), ('coarsen-8', 8)]:
                samples = []
                for _ in range(3):
                    gc.collect()
                    start = time.perf_counter()
                    result = (
                        shapely.simplify(geometry, tolerance, preserve_topology=True)
                        if threads == 0
                        else simplify(geometry, tolerance, preserve_topology=True, threads=threads)
                    )
                    samples.append(time.perf_counter() - start)
                    encoded = shapely.to_wkb(result)
                    if expected is None:
                        expected = encoded.copy()
                    different = np.flatnonzero(encoded != expected)
                    assert len(different) == 0, (tile, tolerance, engine, different[:20])
                assert expected is not None
                print(
                    json.dumps(
                        dict(
                            tile=tile,
                            rows=len(geometry),
                            epsg=epsg,
                            preserve_topology=True,
                            tolerance=tolerance,
                            engine=engine,
                            seconds=samples,
                            median_seconds=float(np.median(samples)),
                            differences=0,
                            output_sha256=hashlib.sha256(b''.join(expected)).hexdigest(),
                        )
                    ),
                    flush=True,
                )


if __name__ == '__main__':
    main()
