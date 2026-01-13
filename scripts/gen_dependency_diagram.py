#!/usr/bin/env python3
"""
Generate a Graphviz DOT file representing intra-crate module dependencies
based on `use crate::...` statements.

The script scans the `src/` tree, derives module paths from file locations,
and records edges from the defining module to every referenced module it
imports. The resulting DOT graph is written to
`docs/architecture/dependency_diagram.dot`.
"""

from __future__ import annotations

import argparse
import os
import pathlib
import re
from typing import Dict, Iterable, List, Set, Tuple

PROJECT_ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC_ROOT = PROJECT_ROOT / "src"
OUTPUT_PATH = PROJECT_ROOT / "docs" / "architecture" / "dependency_diagram.dot"

USE_PATTERN = re.compile(r"^\s*use\s+crate::([A-Za-z0-9_:\s]+?);")


def discover_rust_files(root: pathlib.Path) -> Iterable[pathlib.Path]:
    for path in root.rglob("*.rs"):
        if "target" in path.parts:
            continue
        yield path


def module_name_for(path: pathlib.Path) -> str:
    relative = path.relative_to(SRC_ROOT)
    if relative.name == "lib.rs":
        return "crate"
    parts = list(relative.parts)
    if parts[-1] == "mod.rs":
        parts = parts[:-1]
    else:
        parts[-1] = parts[-1][:-3]  # strip .rs
    return "crate" + "".join(f"::{p}" for p in parts)


def normalise_target(raw_target: str) -> str:
    # Remove trailing `::` segments introduced by glob imports.
    cleaned = raw_target.strip()
    # Drop trailing as clauses or braces.
    cleaned = re.split(r"\s+as\s+", cleaned)[0]
    cleaned = cleaned.split('{')[0].strip()
    cleaned = cleaned.rstrip(':')
    if cleaned.endswith("::*"):
        cleaned = cleaned[:-3]
    return "crate" + "".join(f"::{segment}" for segment in cleaned.split("::") if segment)


def collect_edges() -> Tuple[Set[str], Set[Tuple[str, str]]]:
    modules: Set[str] = set()
    edges: Set[Tuple[str, str]] = set()

    for rust_file in discover_rust_files(SRC_ROOT):
        module = module_name_for(rust_file)
        modules.add(module)
        with rust_file.open("r", encoding="utf-8") as handle:
            for line in handle:
                match = USE_PATTERN.match(line)
                if not match:
                    continue
                target_path = normalise_target(match.group(1))
                if target_path and not target_path.endswith("self"):
                    edges.add((module, target_path))
    return modules, edges


def write_dot(modules: Set[str], edges: Set[Tuple[str, str]], output: pathlib.Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8") as handle:
        handle.write("digraph chic_dependencies {\n")
        handle.write('  rankdir=LR;\n')
        for module in sorted(modules):
            handle.write(f'  "{module}";\n')
        for src, dst in sorted(edges):
            if src == dst:
                continue
            handle.write(f'  "{src}" -> "{dst}";\n')
        handle.write("}\n")


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate module dependency diagram DOT file.")
    parser.add_argument(
        "--output",
        type=pathlib.Path,
        default=OUTPUT_PATH,
        help=f"Path to write the DOT file (default: {OUTPUT_PATH})",
    )
    args = parser.parse_args()

    modules, edges = collect_edges()
    write_dot(modules, edges, args.output.resolve())
    print(f"Wrote dependency graph with {len(modules)} modules and {len(edges)} edges to {args.output}")


if __name__ == "__main__":
    main()
