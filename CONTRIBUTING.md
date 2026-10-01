# Development

The core (`src/`) is usable without Python: `cargo test --locked`. Python support
is an optional Cargo feature. `python/coarsen/` contains the wrapper and type
information; `tests/` contains the original GEOS fixtures, projected real parcels,
and Python parity tests. `benchmarks/` contains the reproducible CPU benchmark.
No CLI, Parquet reader, projection implementation, or native-library loader is
included in the runtime package.

Use Rust ≥1.85 and Python ≥3.10. From the repository root:

```sh
uv venv --python 3.13
uv pip install maturin '.[dev,benchmark,docs]'
uv run --no-sync maturin develop --release
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
uv run --no-sync ruff check .
uv run --no-sync ruff format --check .
uv run --no-sync ty check
uv run --no-sync pytest --cov=coarsen tests/
uv run --no-sync zensical build --strict
```

If a Conda environment is active, deactivate it before `maturin develop`.
Maturin rejects simultaneous `CONDA_PREFIX` and `VIRTUAL_ENV` settings.
An alternative that avoids editable-install environment selection:

```sh
uv run --no-sync maturin build --release --locked --out dist
uv run --no-sync maturin sdist --out dist
uv venv --python 3.13 /tmp/coarsen-wheel-test
uv pip install --python /tmp/coarsen-wheel-test/bin/python dist/*.whl pytest shapely==2.1.2
/tmp/coarsen-wheel-test/bin/python -m pytest tests/
```

The parity suite deliberately requires GEOS 3.13.1, rather than silently testing
against a newer algorithm. Install the Shapely 2.1.2 binary wheel for this gate.
Generated wheels contain the LGPL license, attribution, type hints, and `py.typed`.
Source distributions contain Rust sources, Cargo.lock, Python sources, and fixtures.

# Release

CI checks rustfmt, Clippy, Cargo tests, Ruff, ty, and wheel-installed Python tests
on Linux, macOS, and Windows with Python 3.10–3.13. The release workflow builds
CPython abi3 wheels (3.10+) for manylinux x86_64/aarch64, musllinux x86_64,
macOS x86_64/arm64, and Windows x86_64, plus an sdist. All wheel targets except
musllinux are tested in their build jobs; the sdist is built and tested separately.
`workflow_dispatch` builds artifacts without publishing. Only `v*` tags publish.

Before publishing, the owner must configure a
PyPI project/pending trusted publisher for `coarsen`, specifying that repository,
owner `taylor-geospatial`, repository `coarsen`, workflow `release.yml`, and
environment `pypi`. Create the matching GitHub
`pypi` environment and any desired approval rules. No API token or secret is needed.
Set matching versions in Cargo.toml, pyproject.toml, and the Python `__version__`;
inspect the built artifacts and passing CI before creating a release tag.
Nothing is uploaded by local builds.

# Algorithm notes

Shared edges are extracted once in GEOS encounter order. The simplifier uses a
triangle-area threshold of tolerance squared, GEOS corner tie-breaking, and GEOS
3.13.1's double-double orientation predicate. Intersecting original edge envelopes
produce dependencies from earlier to later edges. Independent rounds run with
Rayon; immutable edges remain constraints. In inner-only mode only edges used by
exactly two rings are simplified and edges used by exactly one ring constrain them.

WKB is copied into owned Rust storage before detaching from Python. The core has
no Python objects, GEOS calls, or shared mutable global geometry state. Explicit
thread counts are local to a call; the default Rayon pool obeys `RAYON_NUM_THREADS`.
The wrapper restores precision-grid metadata lost in the WKB round trip using
Shapely's pointwise precision mode.

# General simplifier

`coarsen.simplify` ports GEOS 3.13.1's TopologyPreservingSimplifier. The tagged-line
port uses immutable packed input envelopes plus live-segment masks and an output
R-tree. Index traversal order does not choose vertices: queries are boolean
intersection checks. Line traversal, furthest-point ties, minimum-size decisions,
and ring endpoint handling follow GEOS 3.13.1. Points, collections, empty members,
and coordinate dimensions follow its GeometryTransformer. WKB loses LinearRing
identity, so ring paths are carried separately and restored by the Python transport.

`preserve_topology=False` calls `shapely.simplify`. DouglasPeuckerSimplifier's
`createValidArea` calls `isValid()` and, for invalid or non-area results,
`buffer(0)`, and neither engine exists in the Rust core. For example, at tolerance
zero GEOS transforms `POLYGON ((0 0, 2 2, 0 2, 2 0, 0 0))` into
`POLYGON ((0 0, 1 1, 2 0, 0 0))`, which vertex removal alone cannot produce. A
native Douglas-Peucker mode would need those GEOS validity and zero-buffer semantics.

# Documentation and security

`make docs-serve` previews the minimal Zensical site; `make docs` builds it strictly.
GitHub Pages needs Actions enabled as its source before the first docs deployment.
The configured canonical URL is `https://research.taylorgeospatial.org/coarsen/`.
See [SECURITY.md](SECURITY.md) for private reporting and resource boundaries.

Release builds disable debug-info stripping to avoid `mis-aligned LINKEDIT string pool`
imports with affected Rust/macOS 27 toolchains
([upstream issue](https://github.com/rust-lang/rust/issues/157750)).
Do not add maturin `--strip` without verifying the resulting wheel on macOS.
