#!/usr/bin/env python3
from __future__ import annotations

import subprocess
import sys
from pathlib import Path


def run(cmd: list[str], cwd: Path) -> None:
    print(f"[chic-hooks] $ {' '.join(cmd)}")
    subprocess.run(cmd, cwd=cwd, check=True)


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    hooks_dir = repo_root / ".githooks"
    if not hooks_dir.exists():
        print(f"[chic-hooks] error: {hooks_dir} does not exist.", file=sys.stderr)
        return 1

    run(["git", "config", "core.hooksPath", ".githooks"], cwd=repo_root)

    for hook_name in ("pre-commit", "pre-push"):
        hook_path = hooks_dir / hook_name
        if hook_path.exists():
            hook_path.chmod(0o755)

    print("[chic-hooks] Installed repo hooks (core.hooksPath=.githooks).")
    print("[chic-hooks] Hooks now run on every commit/push in this clone.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
