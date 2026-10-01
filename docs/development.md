---
description: Build, test, review, and publish coarsen's Rust core, Python bindings, and minimal documentation site.
---
# Development

Install uv, Rust ≥1.85, and Python ≥3.10. The default local setup uses Python 3.13.

```sh
git clone https://github.com/isaaccorley/coarsen.git
cd coarsen
make install
make check
```

`make build` rebuilds the editable native extension after Rust changes.
`make docs-serve` previews this site, and `make dist` builds a wheel and source archive.
The parity gate requires the Shapely 2.1.2 wheel with GEOS 3.13.1.

## Architecture

| Location | Responsibility |
| --- | --- |
| `python/coarsen/` | Broadcasting, Shapely conversion, metadata restoration |
| `src/python.rs` | Owned WKB boundary and GIL release |
| `src/wkb.rs`, `src/simple_geom.rs` | Bounded geometry decoding and encoding |
| `src/edges.rs`, `src/tpvw.rs` | Shared-edge extraction and ordered parallel simplification |
| `src/topology.rs` | Topology-preserving simplification of individual geometries |
| `src/geom.rs`, `src/predicates.rs`, `src/line_math.rs` | Spatial indexing and predicates |
| `src/validate.rs` | Rust coverage validation; not a Python invalid-edges API |

Preserve encounter order and floating-point arithmetic when changing the GEOS
ports. A mathematically equivalent rewrite can change byte parity. Test a proposed
performance change against the reference before claiming a speedup.

## Docs and discoverability

The site includes page descriptions, canonical URLs, a sitemap, robots.txt, and
Open Graph metadata. Package metadata supplies repository and documentation URLs.
The configured public URL is `https://isaac.earth/coarsen/`.

The docs workflow builds on pull requests and deploys only from `main` or a manual
run on `main`. The repository owner must enable GitHub Pages with GitHub Actions
as its source before the first deployment. If adopting a custom domain, update
`site_url`, package documentation URLs, and `docs/robots.txt` together.

Useful GitHub topics are `geospatial`, `gis`, `shapely`, `polygon`, `simplification`,
`topology`, `rust`, and `python`. These are repository settings, separate from files.

See [CONTRIBUTING.md](https://github.com/isaaccorley/coarsen/blob/main/CONTRIBUTING.md)
for the full release checklist, licensing, and platform troubleshooting.
