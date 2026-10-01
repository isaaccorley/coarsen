# Reproducing the benchmark

`benchmark.py` reads only the geometry column of raw 2025 FTW Parquet tiles,
projects EPSG:4326 WKB to each tile's local UTM zone with pyproj `always_xy=True`,
and compares complete calls to Shapely and coarsen at tolerance 5.0.
It applies no coverage filtering, repairs, normalization, or inverse projection.
The selected tiles are the first three entries in the pipeline's
`configs/simplify_2025_xl.txt`.

Times exclude reading, projection, garbage collection, and byte comparison.
They include coarsen's Python wrapper, WKB conversion in both directions, pool
construction, parsing, edge extraction, and simplification. Every output WKB is
compared to the Shapely result on every repetition, with a SHA-256 of all output
bytes recorded in the log. The table reports medians of three sequential runs
per engine, ordered Shapely, coarsen one thread, coarsen eight threads. There is
no separate warm-up or randomized order. Cache state is uncontrolled, and RAILS
CPU nodes are shared with other jobs.

Use an installed wheel in a separate environment: rebuilding an editable
extension while a benchmark has it mapped can crash that process.

```sh
uv run --no-sync maturin build --release --locked --out dist
uv venv --python 3.13 .venv-benchmark
uv pip install --python .venv-benchmark/bin/python dist/*.whl shapely==2.1.2 pyarrow pyproj
COARSEN_BENCH_PYTHON="$PWD/.venv-benchmark/bin/python" sbatch benchmarks/run.sbatch
```

The supplied script requests account `bgtj-tgirails`, partitions `cpu,cpu_amd`,
excludes `rails[04-15]`, and requests eight CPUs, 32 GiB, and one hour. It uses no
GPU nodes. Outside RAILS, run the Python script directly with `--input-root` and
`--tiles`. Inputs are read-only; logs are written under `benchmarks/`.

`--fixtures tests/real` saves the first 16 projected parcels per selected tile,
with their reference simplified WKB and coverage-invalid mask. These compact
fixtures run in both Cargo and pytest without access to the original data.

Measured on 2026-09-30, RAILS node `rails02`, job `226934`, eight allocated CPUs.
All three tiles match WKB bytes at both thread counts on all repetitions:
260,200 polygons and 42,429,412 input vertices.

| Tile | Polygons | Shapely (s) | coarsen 1 thread (s) | coarsen 8 threads (s) | Speedup (8) |
| --- | ---: | ---: | ---: | ---: | ---: |
| 34UEU_0_0 | 42,858 | 92.03 | 22.74 | 9.65 | 9.5× |
| 22JBM_0_0 | 33,919 | 94.71 | 21.94 | 8.24 | 11.5× |
| 48RWV_0_0 | 183,423 | 92.90 | 17.80 | 6.42 | 14.5× |

Raw samples and output hashes: [results.jsonl](results.jsonl). Job accounting:
[slurm.txt](slurm.txt).

The frozen benchmark environment is recorded in [environment.json](environment.json).
The final wrapper additionally copies the input pointer array to make simultaneous
calls safe; the measured Rust extension is unchanged. The earlier editable-install
attempt (job 226918) failed while its extension was rebuilt and is excluded.

Local verification used Python 3.10.20, 3.11.5, 3.12.13, and 3.13.12: all 1,072
pytest cases passed on each, with byte comparison against GEOS 3.13.1. The Python
wrapper has 100% statement coverage. Four Cargo tests pass, including the 174
original cases and three real UTM fixtures; rustfmt, Clippy, Ruff, ty, and actionlint
pass. Wheel and sdist installations were tested in fresh environments. macOS and
Windows execution remains for the configured GitHub Actions matrix.

## General geometry simplification

`benchmark_topology.py` measures the public `coarsen.simplify` API against Shapely
on complete tiles `34UEU_0_0` and `22JBM_0_0`, at 1.2 m and 5 m, with one and
eight threads. It checks every WKB on all three repetitions and prints timings
and hashes as JSON lines. This benchmark uses `preserve_topology=True`.

Run against an installed development wheel, with pyarrow and pyproj installed:

```sh
sbatch --account=bgtj-tgirails --partition=cpu,cpu_amd \
  --exclude='rails[04-15]' -c 8 --mem=32G --time=1:00:00 \
  --output=benchmarks/topology-%j.out \
  --wrap='OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 .venv-benchmark/bin/python benchmarks/benchmark_topology.py'
```

No archived full-tile topology results accompany this repository. Earlier README
single-run numbers lacked the raw samples and environment needed for a reproducible
comparison and are not presented as release benchmarks. Coverage timings above
remain historical results from the recorded coverage implementation.
