#!/usr/bin/env bash

set -euo pipefail

shopt -s nullglob

paths=(
  packages/std*/src
  packages/runtime.native/src
  src/frontend
  src/mir
  src/codegen
)

targets=()
for path in "${paths[@]}"; do
  for expanded in $path; do
    if [[ -d "${expanded}" ]]; then
      targets+=("${expanded}")
    fi
  done
done

if [[ ${#targets[@]} -eq 0 ]]; then
  echo "No scan targets found"
  exit 0
fi

pattern='(TODO|FIXME|HACK)'
glob_excludes=(
  '--glob'
  '!**/generated/**'
  '--glob'
  '!**/tests/fixtures/**'
)

if matches=$(rg --pcre2 "${pattern}" "${glob_excludes[@]}" "${targets[@]}" || true); then
  if [[ -n "${matches}" ]]; then
    echo "Detected TODO/FIXME/HACK markers in critical paths:"
    echo "${matches}"
    exit 1
  fi
fi

echo "TODO scan passed: critical paths are clean."
