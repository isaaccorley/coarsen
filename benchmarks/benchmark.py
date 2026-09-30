"""Time complete public calls on raw FTW tiles, after projection to local UTM.

No input validation/filtering, repairs, normalization, or inverse projection.
Timings include coarsen's Python/WKB overhead. Byte comparison is outside timing.
"""

import argparse
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

import coarsen


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tiles', nargs='+', default=['34UEU_0_0', '22JBM_0_0', '48RWV_0_0'])
    parser.add_argument(
        '--input-root', type=Path, default=Path('/projects/bgtj/isaaccorley/ftw-polygons/2025')
    )
    parser.add_argument('--repeats', type=int, default=3)
    parser.add_argument(
        '--fixtures', type=Path, help='Save 16 projected parcels per tile as test fixtures'
    )
    args = parser.parse_args()
    assert shapely.geos_version == (3, 13, 1)
    print(
        json.dumps(
            dict(
                host=platform.node(),
                python=platform.python_version(),
                shapely=shapely.__version__,
                geos=shapely.geos_version_string,
                coarsen=coarsen.__version__,
                repeats=args.repeats,
                tolerance=5.0,
                simplify_boundary=True,
            )
        ),
        flush=True,
    )
    for tile in args.tiles:
        path = args.input_root / f'{tile}.parquet'
        raw = pq.read_table(path, columns=['geometry'])['geometry'].to_numpy()
        geometry = shapely.from_wkb(raw)
        epsg = int(f'32{6 if tile[2] >= "N" else 7}{tile[:2]}')
        transform = Transformer.from_crs(4326, epsg, always_xy=True)
        geometry = shapely.transform(geometry, transform.transform, interleaved=False)
        del raw
        if args.fixtures:
            args.fixtures.mkdir(parents=True, exist_ok=True)
            sample = geometry[:16]
            fixture = dict(
                tile=tile,
                epsg=epsg,
                input=shapely.to_wkb(sample, hex=True).tolist(),
                bad=(~shapely.is_empty(shapely.coverage_invalid_edges(sample))).tolist(),
                output=shapely.to_wkb(shapely.coverage_simplify(sample, 5.0), hex=True).tolist(),
            )
            (args.fixtures / f'{tile}.json').write_text(json.dumps(fixture) + '\n')
        expected = None
        for label, threads in [('shapely', 0), ('coarsen-1', 1), ('coarsen-8', 8)]:
            samples = []
            for _ in range(args.repeats):
                gc.collect()
                start = time.perf_counter()
                output = (
                    shapely.coverage_simplify(geometry, 5.0)
                    if threads == 0
                    else coarsen.coverage_simplify(geometry, 5.0, threads=threads)
                )
                samples.append(time.perf_counter() - start)
                encoded = shapely.to_wkb(output)
                if expected is None:
                    expected = encoded.copy()
                differences = np.flatnonzero(encoded != expected)
                assert len(differences) == 0, (tile, label, differences[:30])
                del output, encoded
            assert expected is not None
            digest = hashlib.sha256(b''.join(expected)).hexdigest()
            print(
                json.dumps(
                    dict(
                        tile=tile,
                        epsg=epsg,
                        rows=len(geometry),
                        vertices=int(shapely.count_coordinates(geometry)),
                        engine=label,
                        seconds=samples,
                        median_seconds=float(np.median(samples)),
                        differences=0,
                        output_sha256=digest,
                        input_bytes=path.stat().st_size,
                    )
                ),
                flush=True,
            )
        del geometry, expected


if __name__ == '__main__':
    main()
