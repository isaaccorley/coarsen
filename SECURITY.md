# Security policy

## Reporting a vulnerability

Email [isaac.corley@proton.me](mailto:isaac.corley@proton.me) with the affected
version, a minimal reproducer, and the expected impact. Please report vulnerabilities
privately rather than opening a public issue. Avoid including private datasets or credentials.
We will coordinate a fix and disclosure with the reporter; response times are not guaranteed.

Security fixes target the latest release. Older versions do not have a separate
maintenance branch; upgrade to the latest fixed release when one is available.

## Input and resource boundaries

The Python API accepts Shapely geometries. Its private WKB transport checks byte
bounds, collection nesting, geometry structure, and coordinates used in spatial
indexes. Rust's public geometry structs assume structurally valid inputs; they
are not an untrusted-data validation API. Invalid polygon coverages are not repaired.

Geometry processing can consume substantial CPU and memory. WKB size checks do
not impose an application-wide vertex or memory budget. Services processing
untrusted data should cap input bytes, vertices, collection depth, concurrency,
and worker lifetime. Explicit `threads` creates a pool per call; avoid multiplying
large pools across simultaneous requests. Use isolated workers with resource limits.

The Rust core contains no network client, subprocess launcher, or dynamic-library
loader. The Python package depends on NumPy and Shapely; Shapely uses GEOS for
geometry transport and for `preserve_topology=False`. Keep those dependencies updated.
