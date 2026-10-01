# Code review — 2026-10-01

Scope: all Rust modules, Python wrappers and stubs, tests, package metadata,
benchmark scripts/results, and GitHub Actions. Manual source review plus local
execution on macOS arm64. This is not a formal security audit or proof of GEOS parity.

## Findings addressed

| Finding | Impact | Change and evidence |
| --- | --- | --- |
| General WKB decoder accepted one-point lines | A malformed line inside a collection reached `Line::component_point` and raised a native `PanicException` from an out-of-bounds access | Reproduced before the fix; `src/simple_geom.rs` now rejects short lines before simplification. `tests/test_wkb.py` includes the reproducer. |
| General WKB decoder accepted short/unclosed rings and wrong multi-geometry child types | Invalid structures reached algorithms and serialization that assume valid geometry structure | Validate ring structure and collection child types; regressions also cover truncation, trailing bytes, and the existing nesting limit. |
| Release extension failed to import on the local macOS 27 toolchain | `ImportError: mis-aligned LINKEDIT string pool` | Disable debug-info stripping in the release profile. Rebuilt wheel and source distributions both import and pass the full parity suite in fresh environments. Upstream Rust issue: https://github.com/rust-lang/rust/issues/157750. |
| Coverage simplification constructed indexes even when no edge could change | Avoidable index, atomic-state, and neighbor-list allocations | Return normalized edge coordinates before index construction when the active set is empty. Existing zero-tolerance, boundary-mode, and degenerate-geometry parity cases pass. No new speedup claim is made. |
| Workflow actions used mutable tags | Upstream tag movement could change code executed with release permissions | Pin actions to resolved commit SHAs; add Dependabot updates for Actions, Cargo, and Python. Retain read-only permissions except scoped publication jobs. |
| README compatibility and benchmark claims exceeded documented evidence | Users could infer complete ufunc compatibility or reproducible general-simplifier speedups | Document supported keywords, initialized skipped outputs, dimension behavior, GEOS target, and historical benchmark conditions. Remove the unsupported single-run topology timing table; update the benchmark to import the public API. |

## Remaining limitations

- `src/tpvw.rs` materializes per-edge intersecting-envelope neighbors. Dense
  overlapping envelopes can cause quadratic storage and largely serial rounds.
  Replacing this requires representative profiling and careful preservation of
  GEOS encounter order; arbitrary coverage chunking would change constraints.
- `src/topology.rs` component-jump checks scan other lines in the geometry.
  Large collections can be expensive. No measured bottleneck justifies replacing
  this scan in this change.
- Explicit `threads` creates a new Rayon pool per call. Reuse the default pool for
  repeated small work and avoid multiplying pools in concurrent services.
- Geometry pointers, WKB, parsed coordinates, indexes, and output buffers coexist.
  WKB parsing bounds allocation by available bytes but imposes no service-level
  CPU or memory budget. `SECURITY.md` documents worker/input limits for hostile data.
- Public Rust structs and low-level algorithms assume structurally valid geometry.
  They can panic when constructed inconsistently by a Rust caller. The checked WKB
  boundary is the supported route for serialized inputs; this review does not
  redesign all public structs into validated types.
- Invalid coverages are deliberately not repaired. Byte parity is tested against
  GEOS 3.13.1; later GEOS versions and extreme untested floating-point inputs may differ.
  Z/M reconstruction follows GEOS quirks rather than guaranteeing ordinate retention.

No project-owned `unsafe` blocks, shell execution, network access, or dynamic
library loading were found in the runtime source. Python still depends on Shapely
and GEOS. This observation does not establish safety of every transitive dependency.

## Verification

- `make check`: rustfmt, Clippy with warnings denied, four Rust tests, Ruff, ty,
  1,267 Python tests, and strict Zensical build pass.
- Python wrapper statement coverage: 92% overall; coverage wrapper 100%, general
  simplifier wrapper 89%. Rust behavior is exercised by native tests and Python parity;
  these percentages describe Python statements only.
- Fresh wheel install on Python 3.10.21: 1,267 tests pass.
- Fresh source-archive install on Python 3.13.14: 1,267 tests pass.
- 2,400 seeded mutated-WKB calls: 706 accepted, 1,694 rejected with ValueError,
  no native panic. A bounded mutation smoke check, not exhaustive fuzzing.
- `pip-audit` on the development environment: no known vulnerabilities found.
- `cargo audit`: 40 locked dependencies scanned against 1,278 RustSec advisories,
  no known vulnerabilities reported. Advisory results are a dated snapshot.
- `actionlint` and `git diff --check`: pass.
- Wheel inspected for license files, type hints, and package URLs. Source archive
  inspected for Rust source, lockfile, tests, security policy, docs, and logo.
- README and documentation examples executed; local generated links, canonical URLs,
  descriptions, Open Graph metadata, sitemap, and robots.txt checked.
- Docs visually inspected in the browser; search returned API results for `threads`.

Linux/Windows CI and full-scale FTW benchmarks were not run as part of this
local review. Release CI and deployment status are tracked separately in GitHub Actions.
