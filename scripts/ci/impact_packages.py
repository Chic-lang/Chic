#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import re
import subprocess
from collections import defaultdict, deque
from pathlib import Path


def run_git(args: list[str]) -> str:
    result = subprocess.run(
        ["git", *args],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return result.stdout


def repo_root() -> Path:
    return Path(run_git(["rev-parse", "--show-toplevel"]).strip())


def list_changed_files(base: str, head: str) -> list[str]:
    out = run_git(["diff", "--name-only", base, head])
    return [line.strip() for line in out.splitlines() if line.strip()]


def strip_quotes(value: str) -> str:
    value = value.strip()
    if (value.startswith('"') and value.endswith('"')) or (value.startswith("'") and value.endswith("'")):
        return value[1:-1]
    return value


def parse_manifest_package_name(manifest_path: Path) -> str | None:
    in_package = False
    for raw in manifest_path.read_text(encoding="utf-8").splitlines():
        line = raw.rstrip("\n")
        if not in_package:
            if line.strip() == "package:":
                in_package = True
            continue
        if re.match(r"^\S", line):
            break
        m = re.match(r"^\s*name:\s*(.+?)\s*$", line)
        if m:
            return strip_quotes(m.group(1))
    return None


def parse_manifest_dependency_paths(manifest_path: Path) -> list[Path]:
    deps: list[Path] = []
    in_deps = False
    for raw in manifest_path.read_text(encoding="utf-8").splitlines():
        line = raw.rstrip("\n")
        if not in_deps:
            if line.strip() == "dependencies:":
                in_deps = True
            continue

        if re.match(r"^\S", line):
            break

        m = re.match(r"^\s{4}path:\s*(.+?)\s*$", line)
        if m:
            rel = strip_quotes(m.group(1))
            deps.append((manifest_path.parent / rel).resolve())
    return deps


def discover_package_manifests(root: Path) -> list[Path]:
    packages_root = root / "packages"
    if not packages_root.is_dir():
        return []
    manifests = []
    for child in sorted(packages_root.iterdir()):
        if not child.is_dir():
            continue
        manifest = child / "manifest.yaml"
        if manifest.is_file():
            manifests.append(manifest)
    return manifests


def build_package_graph(root: Path) -> tuple[dict[str, Path], dict[Path, set[Path]]]:
    manifests = discover_package_manifests(root)
    dir_to_name: dict[Path, str] = {}
    name_to_dir: dict[str, Path] = {}

    for manifest in manifests:
        pkg_dir = manifest.parent
        name = parse_manifest_package_name(manifest)
        if not name:
            continue
        dir_to_name[pkg_dir.resolve()] = name
        name_to_dir[name] = pkg_dir.resolve()

    deps: dict[Path, set[Path]] = defaultdict(set)
    for manifest in manifests:
        pkg_dir = manifest.parent.resolve()
        if pkg_dir not in dir_to_name:
            continue
        for dep_path in parse_manifest_dependency_paths(manifest):
            dep_manifest = dep_path / "manifest.yaml"
            dep_dir = dep_path.resolve()
            if dep_manifest.is_file() and dep_dir in dir_to_name:
                deps[pkg_dir].add(dep_dir)

    return name_to_dir, deps


def impacted_packages(root: Path, base: str, head: str) -> tuple[list[Path], list[Path]]:
    changed_files = list_changed_files(base, head)

    manifests = discover_package_manifests(root)
    all_package_dirs = sorted({m.parent.resolve() for m in manifests})

    if "manifest.workspace.yaml" in changed_files:
        return all_package_dirs, all_package_dirs

    directly_changed: set[Path] = set()
    for file in changed_files:
        parts = file.split("/")
        if len(parts) >= 2 and parts[0] == "packages":
            if len(parts) >= 3 and parts[2] == "manifest.lock":
                continue
            directly_changed.add((root / "packages" / parts[1]).resolve())

    directly_changed = {p for p in directly_changed if (p / "manifest.yaml").is_file()}

    _, deps = build_package_graph(root)
    reverse: dict[Path, set[Path]] = defaultdict(set)
    for pkg, pkg_deps in deps.items():
        for dep in pkg_deps:
            reverse[dep].add(pkg)

    impacted: set[Path] = set(directly_changed)
    queue = deque(directly_changed)
    while queue:
        cur = queue.popleft()
        for dependent in reverse.get(cur, set()):
            if dependent not in impacted:
                impacted.add(dependent)
                queue.append(dependent)

    return sorted(directly_changed), sorted(impacted)


def main() -> int:
    parser = argparse.ArgumentParser(description="Compute impacted Chic packages from a git diff.")
    parser.add_argument("--base", required=True, help="Base git SHA/ref for diff")
    parser.add_argument("--head", required=True, help="Head git SHA/ref for diff")
    parser.add_argument(
        "--mode",
        choices=["direct", "impacted"],
        default="impacted",
        help="Whether to output only directly changed packages or include dependents",
    )
    parser.add_argument(
        "--format",
        choices=["lines", "json"],
        default="lines",
        help="Output format for impacted packages",
    )
    args = parser.parse_args()

    root = repo_root()
    os.chdir(root)

    direct, impacted = impacted_packages(root, args.base, args.head)
    selected = direct if args.mode == "direct" else impacted

    if args.format == "json":
        import json

        payload = {
            "base": args.base,
            "head": args.head,
            "direct": [str(p.relative_to(root)) for p in direct],
            "impacted": [str(p.relative_to(root)) for p in impacted],
            "mode": args.mode,
            "selected": [str(p.relative_to(root)) for p in selected],
        }
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 0

    for p in selected:
        print(str(p.relative_to(root)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
