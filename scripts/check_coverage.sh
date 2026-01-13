#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
BASELINE_FILE="${REPO_DIR}/coverage/baseline.json"
MODULE_COVERAGE_FILE="${REPO_DIR}/coverage/modules.json"

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "error: cargo-llvm-cov not found. Install with \`cargo install cargo-llvm-cov\`." >&2
    exit 1
fi

if ! rustup component list | grep -q '^llvm-tools.*(installed)'; then
    echo "error: rustup component llvm-tools-preview not installed. Run \`rustup component add llvm-tools-preview\`." >&2
    exit 1
fi

if [ ! -f "${BASELINE_FILE}" ]; then
    echo "error: baseline coverage file missing at ${BASELINE_FILE}" >&2
    exit 1
fi

TMP_JSON="$(mktemp)"
trap 'rm -f "${TMP_JSON}"' EXIT

echo "[coverage] collecting summary via cargo llvm-cov --summary-only --json"
cargo llvm-cov --workspace --summary-only --json >"${TMP_JSON}"

export TMP_JSON
export BASELINE_FILE
export MODULE_COVERAGE_FILE
export REPO_DIR

python3 - <<'PY'
import json
import os
import sys
from datetime import datetime, timezone

tmp_path = os.environ["TMP_JSON"]
baseline_path = os.environ["BASELINE_FILE"]
module_path = os.environ["MODULE_COVERAGE_FILE"]
repo_dir = os.environ["REPO_DIR"]
tolerance = float(os.environ.get("COVERAGE_TOLERANCE", "0.10"))

with open(tmp_path, "r", encoding="utf-8") as fh:
    payload = json.load(fh)

summary = payload["data"][0]["totals"]
current_lines = float(summary["lines"]["percent"])

with open(baseline_path, "r", encoding="utf-8") as fh:
    baseline = json.load(fh)

baseline_lines = float(baseline["metrics"]["lines_percent"])
delta = current_lines - baseline_lines

# Aggregate per-module coverage (top-level directories under src/ and tests/).
files = payload["data"][0].get("files", [])
modules: dict[str, dict[str, float]] = {}

for entry in files:
    filename = entry.get("filename")
    if not filename or not os.path.isabs(filename):
        continue
    if not filename.startswith(repo_dir):
        continue
    rel = os.path.relpath(filename, repo_dir)
    parts = rel.split(os.sep)
    if not parts or parts[0] not in {"src", "tests"}:
        continue
    module_key = parts[0]
    if len(parts) >= 2:
        module_key = f"{parts[0]}/{parts[1]}"

    lines = entry.get("summary", {}).get("lines", {})
    covered = float(lines.get("covered", 0.0))
    total = float(lines.get("count", 0.0))
    if total <= 0:
        continue

    bucket = modules.setdefault(module_key, {"covered": 0.0, "total": 0.0})
    bucket["covered"] += covered
    bucket["total"] += total

module_rows = []
for module, stats in modules.items():
    covered = stats["covered"]
    total = stats["total"]
    percent = 0.0 if total == 0 else (covered / total) * 100.0
    module_rows.append(
        {
            "module": module,
            "lines_percent": percent,
            "lines_covered": covered,
            "lines_total": total,
        }
    )

module_rows.sort(key=lambda row: row["lines_percent"])

with open(module_path, "w", encoding="utf-8") as fh:
    json.dump(
        {
            "generated": datetime.now(timezone.utc).isoformat(),
            "modules": module_rows,
        },
        fh,
        indent=2,
    )

print("[coverage] per-module summary (lowest 5 by line %):")
for row in module_rows[:5]:
    print(
        f"  {row['module']:<20} {row['lines_percent']:6.2f}% "
        f"({row['lines_covered']:.0f}/{row['lines_total']:.0f})"
    )

print(f"[coverage] baseline lines % : {baseline_lines:.2f}")
print(f"[coverage] current  lines % : {current_lines:.2f}")
print(f"[coverage] delta             : {delta:+.2f}")

if current_lines + 1e-9 < baseline_lines - tolerance:
    print("[coverage] ERROR: coverage regression exceeds tolerance.", file=sys.stderr)
    sys.exit(1)

print("[coverage] OK: coverage meets or exceeds baseline (within tolerance).")
PY
