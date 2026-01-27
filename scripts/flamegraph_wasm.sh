#!/usr/bin/env bash

set -euo pipefail

if ! command -v cargo-flamegraph >/dev/null 2>&1; then
    >&2 echo "error: cargo-flamegraph not found. Install with \`cargo install flamegraph\`."
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

SOURCE_FILE="${1:-${REPO_DIR}/profiling/fixtures/wasm_hot_switch.ch}"
OUTPUT_DIR="${OUTPUT_DIR:-${REPO_DIR}/profiling}"
BACKEND="${BACKEND:-wasm}"
ENTRY="${ENTRY:-chic_main}"

mkdir -p "${OUTPUT_DIR}"

export CHIC_WASM_TRACE="${CHIC_WASM_TRACE:-}"

echo "[flamegraph] building ${SOURCE_FILE} with backend=${BACKEND}"
cargo flamegraph \
    --bin chic \
    -- \
    build \
    "${SOURCE_FILE}" \
    --backend "${BACKEND}" \
    --output "${OUTPUT_DIR}/flamegraph.${BACKEND}.artifact"

SVG_PATH="${REPO_DIR}/flamegraph.svg"
if [[ -f "${SVG_PATH}" ]]; then
    mv "${SVG_PATH}" "${OUTPUT_DIR}/flamegraph.${BACKEND}.svg"
    echo "[flamegraph] profile written to ${OUTPUT_DIR}/flamegraph.${BACKEND}.svg"
else
    echo "[flamegraph] expected flamegraph.svg was not produced"
fi
