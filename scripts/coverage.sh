#!/usr/bin/env bash

set -euo pipefail

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "error: cargo-llvm-cov not found. Install with \`cargo install cargo-llvm-cov\`." >&2
    exit 1
fi

if ! rustup component list | grep -q '^llvm-tools.*(installed)' >/dev/null 2>&1; then
    echo "error: rustup component llvm-tools-preview not installed. Run \`rustup component add llvm-tools-preview\`." >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUTPUT_DIR="${OUTPUT_DIR:-${REPO_DIR}/coverage}"

mkdir -p "${OUTPUT_DIR}"

echo "[coverage] cleaning previous artifacts"
cargo llvm-cov clean --workspace

TEST_ARGS=("$@")
if [ "${#TEST_ARGS[@]}" -eq 0 ]; then
    TEST_ARGS=(-- --test-threads=1)
else
    TEST_ARGS=(-- "${TEST_ARGS[@]}")
fi

if [ -n "${COVERAGE_ARGS:-}" ]; then
    # shellcheck disable=SC2206
    RUN_ARGS=(${COVERAGE_ARGS})
else
    RUN_ARGS=(--package chic --lib)
fi
echo "[coverage] running cargo llvm-cov --lcov ${TEST_ARGS[*]}"
cargo llvm-cov \
    "${RUN_ARGS[@]}" \
    --lcov \
    --output-path "${OUTPUT_DIR}/coverage.lcov" \
    "${TEST_ARGS[@]}"

echo "[coverage] running cargo llvm-cov --html ${TEST_ARGS[*]}"
cargo llvm-cov \
    "${RUN_ARGS[@]}" \
    --html \
    --output-dir "${OUTPUT_DIR}/html" \
    "${TEST_ARGS[@]}"

echo "[coverage] artifacts:"
echo "  LCOV report : ${OUTPUT_DIR}/coverage.lcov"
echo "  HTML report : ${OUTPUT_DIR}/html/index.html"
