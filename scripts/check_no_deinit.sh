#!/usr/bin/env bash
set -euo pipefail

# Chic deterministic destruction uses `dispose(ref this)`. `deinit` is forbidden in Chic sources.

if command -v rg >/dev/null 2>&1; then
  if rg -n --glob '*.cl' --hidden --no-ignore-vcs '\\bdeinit\\b' packages tests; then
    echo
    echo "error: found forbidden identifier 'deinit' in Chic sources; use 'dispose' instead" >&2
    exit 1
  fi
else
  if grep -RIn --include='*.cl' -E '\\bdeinit\\b' packages tests; then
    echo
    echo "error: found forbidden identifier 'deinit' in Chic sources; use 'dispose' instead" >&2
    exit 1
  fi
fi
