.PHONY: install build check test docs docs-serve dist

install:
	uv venv --python 3.13
	uv pip install '.[dev,benchmark,docs]'
	$(MAKE) build

build:
	uv run --no-sync maturin develop --release --locked

check:
	cargo fmt --check
	cargo clippy --locked --all-targets --all-features -- -D warnings
	cargo test --locked
	uv run --no-sync ruff check .
	uv run --no-sync ruff format --check .
	uv run --no-sync ty check
	$(MAKE) test docs

test:
	uv run --no-sync pytest --cov=coarsen tests/

docs:
	uv run --no-sync zensical build --strict

docs-serve:
	uv run --no-sync zensical serve

dist:
	uv run --no-sync maturin build --release --locked --out dist
	uv run --no-sync maturin sdist --out dist
