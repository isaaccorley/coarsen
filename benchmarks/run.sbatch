#!/bin/bash
#SBATCH --job-name=coarsen-benchmark
#SBATCH --account=bgtj-tgirails
#SBATCH --partition=cpu,cpu_amd
#SBATCH --exclude=rails[04-15]
#SBATCH --cpus-per-task=8
#SBATCH --mem=32G
#SBATCH --time=1:00:00
#SBATCH --output=benchmarks/%j.out
set -euo pipefail
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1
"${COARSEN_BENCH_PYTHON:-.venv/bin/python}" benchmarks/benchmark.py "$@"
