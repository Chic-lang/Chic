#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
import os
import re
import shlex
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable
import xml.etree.ElementTree as ET

MIN_COVERAGE = float(os.environ.get("CHIC_MIN_COVERAGE", "95"))
CHIC_TEST_TIMEOUT_SEC = int(os.environ.get("CHIC_TEST_TIMEOUT_SEC", "900"))
CHIC_COVERAGE_TIMEOUT_SEC = int(os.environ.get("CHIC_COVERAGE_TIMEOUT_SEC", "900"))
ZERO_SHA = "0" * 40
PROTECTED_BRANCHES = {"main", "master"}


@dataclass
class Issue:
    title: str
    details: str
    fix: str | None = None


class QualityGate:
    def __init__(self, stage: str, remote_name: str | None, remote_url: str | None) -> None:
        self.stage = stage
        self.remote_name = remote_name
        self.remote_url = remote_url
        self.root = Path(__file__).resolve().parents[2]
        self.issues: list[Issue] = []
        self.warnings: list[str] = []

        self.has_rust = (self.root / "Cargo.toml").exists()
        self.has_chic_workspace = (self.root / "manifest.workspace.yaml").exists() or (self.root / "packages").exists()
        self.node_projects = self._discover_node_projects()
        self.dotnet_targets = self._discover_dotnet_targets()
        self.push_refs = self._read_push_refs_once() if stage == "pre-push" else []

    def run(self) -> int:
        print(f"[chic-hooks] stage={self.stage}")
        print(f"[chic-hooks] repo={self.root}")
        print(
            "[chic-hooks] detected: "
            f"rust={self.has_rust}, "
            f"chic_workspace={self.has_chic_workspace}, "
            f"node_projects={len(self.node_projects)}, "
            f"dotnet_targets={len(self.dotnet_targets)}"
        )

        if self.stage == "pre-commit":
            self.run_pre_commit_checks()
        elif self.stage == "pre-push":
            self.run_pre_push_checks()
        else:
            self.add_issue(
                "Unsupported hook stage",
                f"Unknown stage '{self.stage}'.",
                "Invoke this script with either 'pre-commit' or 'pre-push'.",
            )

        return self.finish()

    def run_pre_commit_checks(self) -> None:
        self.section("Pre-Commit: Format + Lint")
        self.run_rust_format_lint()
        self.run_node_format_lint()
        self.run_dotnet_format_lint()

    def run_pre_push_checks(self) -> None:
        self.section("Pre-Push: Branch + PR policy")
        self.enforce_branch_policy()
        self.enforce_pr_visibility_policy()

        self.run_pre_commit_checks()

        self.section("Pre-Push: Dependency freshness")
        self.check_rust_dependencies()
        self.check_node_dependencies()
        self.check_dotnet_dependencies()

        self.section(f"Pre-Push: Tests + Coverage >= {MIN_COVERAGE:.2f}%")
        self.run_chic_tests_and_coverage()
        self.run_rust_tests_and_coverage()
        self.run_node_tests_and_coverage()
        self.run_dotnet_tests_and_coverage()

    def run_chic_tests_and_coverage(self) -> None:
        if not self.has_chic_workspace:
            return

        chic_cmd = self.resolve_chic_command()
        if chic_cmd is None:
            self.add_issue(
                "Missing Chic CLI",
                "Chic workspace checks require `chic` (or a buildable Rust `chic` binary) for `chic test` / `chic coverage`.",
                "Build and expose the compiler (`cargo build --bin chic`) or install `chic` on PATH.",
            )
            return

        self.run_cmd_checked(
            [*chic_cmd, "test", "--workspace"],
            title="Chic tests failed",
            fix="Run `chic test --workspace` and fix all failing Chic tests before pushing.",
            timeout_sec=CHIC_TEST_TIMEOUT_SEC,
        )

        min_percent = str(self.min_coverage_int())
        self.run_cmd_checked(
            [*chic_cmd, "coverage", "--workspace", "--min", min_percent],
            title="Chic coverage failed",
            fix=(
                f"Run `chic coverage --workspace --min {min_percent}` and raise coverage "
                f"to at least {min_percent}%."
            ),
            timeout_sec=CHIC_COVERAGE_TIMEOUT_SEC,
        )

    def run_rust_format_lint(self) -> None:
        if not self.has_rust:
            return
        if not self.require_tool("cargo", "Rust checks require Cargo in PATH."):
            return

        self.run_cmd_checked(
            ["cargo", "fmt", "--all", "--", "--check"],
            title="Rust formatting failed",
            fix="Run `cargo fmt --all`, restage changes, and commit again.",
        )
        # Align local lint gates with CI-required xtask checks for this repository.
        rust_lints = [
            ("lint-ll1", "LL(1) guardrails", True),
            ("lint-runtime-shim", "runtime shim guard", True),
            ("lint-shim-state", "shim state guard", True),
            ("lint-runtime-calls", "runtime call guard", False),
            ("lint-runtime-symbols", "runtime symbol guard", False),
            ("lint-stdlib-rust-tests", "Stdlib Rust test guard", True),
        ]
        for lint_name, lint_desc, is_blocking in rust_lints:
            cmd = ["cargo", "xtask", lint_name]
            result = self.run_cmd(cmd, capture=False)
            if result.returncode == 0:
                continue
            if is_blocking:
                self.add_issue(
                    f"Rust lint failed ({lint_desc})",
                    f"Command failed (exit {result.returncode}): {self.format_cmd(cmd)}",
                    f"Run `cargo xtask {lint_name}` and resolve reported issues.",
                )
            else:
                self.warnings.append(
                    f"Non-blocking Rust lint reported issues ({lint_desc}): {self.format_cmd(cmd)}"
                )

    def run_node_format_lint(self) -> None:
        if not self.node_projects:
            return
        if not self.require_tool("npm", "Node checks require npm in PATH."):
            return

        for project in self.node_projects:
            scripts = self.load_package_scripts(project)
            label = self.rel(project)

            format_script = self.select_format_check_script(scripts)
            if format_script:
                self.run_cmd_checked(
                    ["npm", "run", format_script],
                    cwd=project,
                    title=f"Node format check failed ({label})",
                    fix=f"Run `cd {label} && npm run {format_script}` and fix formatting issues.",
                )
            else:
                self.warnings.append(
                    f"No format check script found in {label}. Add one (e.g. format:check) to enforce formatter gates."
                )

            if "lint" not in scripts:
                self.add_issue(
                    f"Missing lint script ({label})",
                    "package.json has no `lint` script.",
                    "Add a `lint` script and ensure it exits non-zero on violations.",
                )
            else:
                self.run_cmd_checked(
                    ["npm", "run", "lint"],
                    cwd=project,
                    title=f"Node lint failed ({label})",
                    fix=f"Run `cd {label} && npm run lint` and resolve lint errors.",
                )

            if "typecheck" in scripts:
                self.run_cmd_checked(
                    ["npm", "run", "typecheck"],
                    cwd=project,
                    title=f"Node typecheck failed ({label})",
                    fix=f"Run `cd {label} && npm run typecheck` and resolve typing errors.",
                )

    def run_dotnet_format_lint(self) -> None:
        if not self.dotnet_targets:
            return
        if not self.require_tool("dotnet", ".NET checks require the .NET SDK in PATH."):
            return

        for target in self.dotnet_targets:
            rel_target = self.rel(target)
            self.run_cmd_checked(
                ["dotnet", "format", str(target), "--verify-no-changes"],
                title=f".NET formatting/lint failed ({rel_target})",
                fix=f"Run `dotnet format {shlex.quote(rel_target)}` and commit updated formatting/analyzers output.",
            )

    def check_rust_dependencies(self) -> None:
        if not self.has_rust:
            return
        if not self.require_tool("cargo", "Rust dependency checks require Cargo in PATH."):
            return

        has_outdated = shutil.which("cargo-outdated") is not None
        if not has_outdated:
            probe = self.run_cmd(["cargo", "outdated", "--version"], capture=True)
            has_outdated = probe.returncode == 0

        if not has_outdated:
            self.add_issue(
                "Missing cargo-outdated",
                "Rust dependency freshness requires cargo-outdated to detect any stale crates.",
                "Install it with `cargo install cargo-outdated --locked`.",
            )
            return

        result = self.run_cmd(["cargo", "outdated", "--workspace", "--exit-code", "1"], capture=True)
        if result.returncode == 0:
            return
        if result.returncode == 1:
            self.add_issue(
                "Outdated Rust crates detected",
                self.summarize_output(result.stdout or result.stderr),
                "Run `cargo outdated --workspace` and update every outdated crate (no blanket exceptions).",
            )
            return
        self.add_issue(
            "Rust dependency check failed",
            self.summarize_output((result.stdout or "") + "\n" + (result.stderr or "")),
            "Fix the command failure and rerun `cargo outdated --workspace --exit-code 1`.",
        )

    def check_node_dependencies(self) -> None:
        if not self.node_projects:
            return
        if not self.require_tool("npm", "Node dependency checks require npm in PATH."):
            return

        for project in self.node_projects:
            label = self.rel(project)
            result = self.run_cmd(["npm", "outdated", "--json"], cwd=project, capture=True)

            if result.returncode == 0:
                continue

            if result.returncode != 1:
                self.add_issue(
                    f"Node dependency check failed ({label})",
                    self.summarize_output((result.stdout or "") + "\n" + (result.stderr or "")),
                    f"Run `cd {label} && npm outdated --json` and resolve errors.",
                )
                continue

            payload = (result.stdout or "").strip()
            if not payload:
                self.add_issue(
                    f"Node dependency check failed ({label})",
                    self.summarize_output(result.stderr or "npm outdated returned exit code 1 with no JSON payload."),
                    f"Run `cd {label} && npm install` (if needed), then rerun `npm outdated --json`.",
                )
                continue

            try:
                data = json.loads(payload)
            except json.JSONDecodeError:
                self.add_issue(
                    f"Node dependency check parse error ({label})",
                    self.summarize_output(payload),
                    f"Run `cd {label} && npm outdated --json` and ensure JSON output is valid.",
                )
                continue

            if not isinstance(data, dict) or not data:
                continue

            lines: list[str] = []
            for package_name, meta in sorted(data.items()):
                if not isinstance(meta, dict):
                    lines.append(f"- {package_name}: {meta}")
                    continue
                current = meta.get("current", "?")
                wanted = meta.get("wanted", "?")
                latest = meta.get("latest", "?")
                lines.append(f"- {package_name}: current={current}, wanted={wanted}, latest={latest}")

            self.add_issue(
                f"Outdated npm packages detected ({label})",
                "\n".join(lines),
                f"Run `cd {label} && npm outdated` and update every outdated dependency.",
            )

    def check_dotnet_dependencies(self) -> None:
        if not self.dotnet_targets:
            return
        if not self.require_tool("dotnet", "NuGet dependency checks require the .NET SDK in PATH."):
            return

        for target in self.dotnet_targets:
            rel_target = self.rel(target)
            result = self.run_cmd(
                [
                    "dotnet",
                    "list",
                    str(target),
                    "package",
                    "--outdated",
                    "--include-transitive",
                    "--format",
                    "json",
                ],
                capture=True,
            )
            if result.returncode != 0:
                self.add_issue(
                    f"NuGet dependency check failed ({rel_target})",
                    self.summarize_output((result.stdout or "") + "\n" + (result.stderr or "")),
                    f"Run `dotnet list {shlex.quote(rel_target)} package --outdated --include-transitive --format json` and fix the failure.",
                )
                continue

            data = self.parse_json_from_mixed_output(result.stdout or "")
            if not data:
                self.add_issue(
                    f"NuGet dependency output parse error ({rel_target})",
                    self.summarize_output(result.stdout or result.stderr or "No output produced."),
                    "Ensure the command outputs valid JSON.",
                )
                continue

            outdated = self.collect_dotnet_outdated_packages(data)
            if outdated:
                self.add_issue(
                    f"Outdated NuGet packages detected ({rel_target})",
                    "\n".join(outdated),
                    f"Run `dotnet list {shlex.quote(rel_target)} package --outdated --include-transitive` and update all listed packages.",
                )

    def run_rust_tests_and_coverage(self) -> None:
        if not self.has_rust:
            return
        if not self.require_tool("cargo", "Rust tests require Cargo in PATH."):
            return
        if not self.require_tool(
            "cargo-llvm-cov",
            "Rust coverage gate requires cargo-llvm-cov. Install with `cargo install cargo-llvm-cov --locked`.",
        ):
            return

        result = self.run_cmd(
            ["cargo", "llvm-cov", "--workspace", "--all-targets", "--summary-only", "--json"],
            capture=True,
        )
        if result.returncode != 0:
            self.add_issue(
                "Rust tests/coverage failed",
                self.summarize_output((result.stdout or "") + "\n" + (result.stderr or "")),
                "Run `cargo llvm-cov --workspace --all-targets --summary-only --json` and fix failing tests/build issues.",
            )
            return

        percent = self.parse_rust_coverage_percent(result.stdout or "")
        if percent is None:
            self.add_issue(
                "Rust coverage parse error",
                self.summarize_output(result.stdout or "Could not parse coverage JSON."),
                "Verify cargo-llvm-cov output and ensure JSON summary contains line coverage percent.",
            )
            return

        print(f"[chic-hooks] Rust line coverage: {percent:.2f}%")
        if percent < MIN_COVERAGE:
            self.add_issue(
                "Rust coverage below threshold",
                f"Current Rust line coverage is {percent:.2f}% (required >= {MIN_COVERAGE:.2f}%).",
                "Add or fix tests until coverage reaches the minimum threshold.",
            )

    def run_node_tests_and_coverage(self) -> None:
        if not self.node_projects:
            return
        if not self.require_tool("npm", "Node tests require npm in PATH."):
            return

        for project in self.node_projects:
            scripts = self.load_package_scripts(project)
            label = self.rel(project)
            test_script = scripts.get("test")

            if not test_script:
                self.add_issue(
                    f"Missing test script ({label})",
                    "package.json has no `test` script.",
                    "Add a real test script that runs the unit test suite with coverage output.",
                )
                continue

            if self.is_placeholder_test_script(test_script):
                self.add_issue(
                    f"Placeholder test script ({label})",
                    f"Found non-testing script: `{test_script}`.",
                    "Replace the placeholder with real automated tests and coverage generation.",
                )
                continue

            result = self.run_cmd(["npm", "test"], cwd=project, env={"CI": "1"}, capture=False)
            if result.returncode != 0:
                self.add_issue(
                    f"Node tests failed ({label})",
                    "`npm test` returned non-zero.",
                    f"Run `cd {label} && npm test` and fix failing tests.",
                )
                continue

            coverage_percent = self.read_node_coverage_percent(project)
            if coverage_percent is None:
                self.add_issue(
                    f"Node coverage missing ({label})",
                    "Could not find coverage summary JSON (expected `coverage/coverage-summary.json`).",
                    "Configure the test runner to emit `coverage/coverage-summary.json` with total line coverage.",
                )
                continue

            print(f"[chic-hooks] Node line coverage ({label}): {coverage_percent:.2f}%")
            if coverage_percent < MIN_COVERAGE:
                self.add_issue(
                    f"Node coverage below threshold ({label})",
                    f"Current Node line coverage is {coverage_percent:.2f}% (required >= {MIN_COVERAGE:.2f}%).",
                    "Add or fix tests until coverage reaches the minimum threshold.",
                )

    def run_dotnet_tests_and_coverage(self) -> None:
        if not self.dotnet_targets:
            return
        if not self.require_tool("dotnet", ".NET tests require the .NET SDK in PATH."):
            return

        results_root = self.root / ".artifacts" / "hook-dotnet-coverage"
        if results_root.exists():
            shutil.rmtree(results_root)
        results_root.mkdir(parents=True, exist_ok=True)

        for target in self.dotnet_targets:
            rel_target = self.rel(target)
            result = self.run_cmd(
                [
                    "dotnet",
                    "test",
                    str(target),
                    "--collect:XPlat Code Coverage",
                    "--results-directory",
                    str(results_root),
                ],
                capture=False,
            )
            if result.returncode != 0:
                self.add_issue(
                    f".NET tests failed ({rel_target})",
                    "`dotnet test` returned non-zero.",
                    f"Run `dotnet test {shlex.quote(rel_target)} --collect:\"XPlat Code Coverage\"` and fix failing tests.",
                )

        coverage_files = list(results_root.rglob("coverage.cobertura.xml"))
        if not coverage_files:
            if self.dotnet_targets:
                self.add_issue(
                    "NuGet/.NET coverage missing",
                    f"No `coverage.cobertura.xml` found under {self.rel(results_root)}.",
                    "Ensure `dotnet test --collect:XPlat Code Coverage` runs and emits Cobertura coverage artifacts.",
                )
            return

        total_valid = 0
        total_covered = 0
        for coverage_file in coverage_files:
            try:
                tree = ET.parse(coverage_file)
                root = tree.getroot()
                valid = int(root.attrib.get("lines-valid", "0"))
                covered = int(root.attrib.get("lines-covered", "0"))
                total_valid += valid
                total_covered += covered
            except Exception as exc:  # pragma: no cover - defensive parsing in hook
                self.add_issue(
                    f".NET coverage parse error ({self.rel(coverage_file)})",
                    str(exc),
                    "Ensure Cobertura XML output is valid.",
                )

        if total_valid <= 0:
            self.add_issue(
                ".NET coverage invalid",
                "Cobertura output reported zero valid lines.",
                "Ensure projects include instrumentable source and coverage collection is configured correctly.",
            )
            return

        coverage_percent = (total_covered / total_valid) * 100.0
        print(f"[chic-hooks] .NET line coverage: {coverage_percent:.2f}%")
        if coverage_percent < MIN_COVERAGE:
            self.add_issue(
                ".NET coverage below threshold",
                f"Current .NET line coverage is {coverage_percent:.2f}% (required >= {MIN_COVERAGE:.2f}%).",
                "Add or fix tests until coverage reaches the minimum threshold.",
            )

    def enforce_branch_policy(self) -> None:
        refs = self.read_push_refs()
        if not refs:
            branch = self.current_branch()
            if branch and self.is_protected_branch(branch):
                self.add_issue(
                    "Push to protected branch denied",
                    f"Current branch is `{branch}`.",
                    "Create/use a feature branch and open a draft PR.",
                )
            return

        for _, _, remote_ref, _ in refs:
            if not remote_ref.startswith("refs/heads/"):
                continue
            branch = remote_ref.split("refs/heads/", 1)[1]
            if self.is_protected_branch(branch):
                self.add_issue(
                    "Push to protected branch denied",
                    f"Attempted push target: `{branch}`.",
                    "Push to a feature branch and merge via PR instead of direct protected-branch pushes.",
                )

    def enforce_pr_visibility_policy(self) -> None:
        refs = self.read_push_refs()
        if not refs:
            return

        if not self.remote_url or "github" not in self.remote_url.lower():
            return

        if not shutil.which("gh"):
            self.warnings.append(
                "GitHub CLI (`gh`) is not installed; unable to verify draft/open PR state from hook."
            )
            return

        for _, _, remote_ref, remote_sha in refs:
            if not remote_ref.startswith("refs/heads/"):
                continue
            branch = remote_ref.split("refs/heads/", 1)[1]
            if self.is_protected_branch(branch):
                continue

            if remote_sha == ZERO_SHA:
                self.warnings.append(
                    f"First push for `{branch}` detected. Create a draft PR immediately after push: `gh pr create --draft --fill`"
                )
                continue

            result = self.run_cmd(
                ["gh", "pr", "view", branch, "--json", "state,isDraft,url"],
                capture=True,
            )
            if result.returncode != 0:
                self.add_issue(
                    f"Missing PR for branch `{branch}`",
                    self.summarize_output(result.stderr or result.stdout or "No PR found."),
                    "Create a draft PR (`gh pr create --draft --fill`) and keep work associated with an open PR.",
                )
                continue

            try:
                pr_data = json.loads(result.stdout)
            except json.JSONDecodeError:
                self.add_issue(
                    f"PR metadata parse failure for `{branch}`",
                    self.summarize_output(result.stdout),
                    "Run `gh pr view --json state,isDraft,url` and verify GitHub CLI output.",
                )
                continue

            if pr_data.get("state") != "OPEN":
                self.add_issue(
                    f"PR is not open for branch `{branch}`",
                    f"Current PR state: {pr_data.get('state')}",
                    "Reopen or create a draft/open PR before pushing more changes.",
                )

    def require_tool(self, tool: str, message: str) -> bool:
        if shutil.which(tool):
            return True
        self.add_issue(f"Missing required tool: {tool}", message)
        return False

    def resolve_chic_command(self) -> list[str] | None:
        local_candidates = [
            self.root / "target" / "debug" / "chic",
            self.root / "target" / "release" / "chic",
        ]
        for candidate in local_candidates:
            if candidate.exists() and os.access(candidate, os.X_OK):
                return [str(candidate)]

        if shutil.which("chic"):
            return ["chic"]

        if self.has_rust and shutil.which("cargo"):
            return ["cargo", "run", "--quiet", "--bin", "chic", "--"]

        return None

    def min_coverage_int(self) -> int:
        bounded = max(0.0, min(100.0, MIN_COVERAGE))
        return int(math.ceil(bounded))

    def run_cmd_checked(
        self,
        cmd: list[str],
        *,
        cwd: Path | None = None,
        env: dict[str, str] | None = None,
        title: str,
        fix: str,
        timeout_sec: int | None = None,
    ) -> None:
        result = self.run_cmd(cmd, cwd=cwd, env=env, capture=False, timeout_sec=timeout_sec)
        if result.returncode != 0:
            detail = f"Command failed (exit {result.returncode}): {self.format_cmd(cmd)}"
            if result.stderr:
                detail = f"{detail}\n{self.summarize_output(result.stderr)}"
            self.add_issue(title, detail, fix)

    def run_cmd(
        self,
        cmd: list[str],
        *,
        cwd: Path | None = None,
        env: dict[str, str] | None = None,
        capture: bool,
        timeout_sec: int | None = None,
    ) -> subprocess.CompletedProcess[str]:
        workdir = cwd or self.root
        merged_env = os.environ.copy()
        if env:
            merged_env.update(env)

        print(f"[chic-hooks] $ (cd {self.rel(workdir)} && {self.format_cmd(cmd)})")
        try:
            return subprocess.run(
                cmd,
                cwd=workdir,
                env=merged_env,
                text=True,
                capture_output=capture,
                check=False,
                timeout=timeout_sec,
            )
        except subprocess.TimeoutExpired as exc:
            stdout = self._timeout_stream_to_text(exc.stdout)
            stderr = self._timeout_stream_to_text(exc.stderr)
            timed_out = f"Command timed out after {timeout_sec} seconds."
            stderr = f"{stderr}\n{timed_out}".strip()
            return subprocess.CompletedProcess(cmd, 124, stdout=stdout, stderr=stderr)

    def add_issue(self, title: str, details: str, fix: str | None = None) -> None:
        self.issues.append(Issue(title=title, details=details.strip(), fix=fix))

    def finish(self) -> int:
        if self.warnings:
            print("\n[chic-hooks] warnings:")
            for warning in self.warnings:
                print(f"- {warning}")

        if not self.issues:
            print("\n[chic-hooks] PASS")
            return 0

        print("\n[chic-hooks] FAIL")
        for index, issue in enumerate(self.issues, start=1):
            print(f"\n{index}. {issue.title}")
            print(issue.details)
            if issue.fix:
                print(f"Fix: {issue.fix}")

        return 1

    def current_branch(self) -> str | None:
        result = self.run_cmd(["git", "rev-parse", "--abbrev-ref", "HEAD"], capture=True)
        if result.returncode != 0:
            return None
        branch = (result.stdout or "").strip()
        if branch == "HEAD":
            return None
        return branch

    def read_push_refs(self) -> list[tuple[str, str, str, str]]:
        return self.push_refs

    def _read_push_refs_once(self) -> list[tuple[str, str, str, str]]:
        refs: list[tuple[str, str, str, str]] = []
        if sys.stdin is None or sys.stdin.closed or sys.stdin.isatty():
            return refs
        try:
            data = sys.stdin.read()
        except Exception:
            return refs
        for raw_line in data.splitlines():
            line = raw_line.strip()
            if not line:
                continue
            parts = line.split()
            if len(parts) != 4:
                continue
            refs.append((parts[0], parts[1], parts[2], parts[3]))
        return refs

    def is_protected_branch(self, branch: str) -> bool:
        return branch in PROTECTED_BRANCHES or branch.startswith("release/")

    def load_package_scripts(self, project: Path) -> dict[str, str]:
        package_json_path = project / "package.json"
        try:
            payload = json.loads(package_json_path.read_text(encoding="utf-8"))
        except Exception as exc:
            self.add_issue(
                f"Invalid package.json ({self.rel(project)})",
                str(exc),
                f"Fix JSON syntax in {self.rel(package_json_path)}.",
            )
            return {}

        scripts = payload.get("scripts", {})
        if isinstance(scripts, dict):
            return {str(k): str(v) for k, v in scripts.items()}
        return {}

    def select_format_check_script(self, scripts: dict[str, str]) -> str | None:
        preferred = ["format:check", "fmt:check", "prettier:check", "style:check"]
        for name in preferred:
            if name in scripts:
                return name
        return None

    def is_placeholder_test_script(self, script: str) -> bool:
        normalized = re.sub(r"\s+", " ", script.strip().lower())
        placeholders = (
            "echo \"no tests\"",
            "echo 'no tests'",
            "echo no tests",
            "exit 0",
            "true",
        )
        return normalized in placeholders

    def read_node_coverage_percent(self, project: Path) -> float | None:
        candidates = [
            project / "coverage" / "coverage-summary.json",
            project / "coverage-summary.json",
        ]
        for candidate in candidates:
            if not candidate.exists():
                continue
            try:
                payload = json.loads(candidate.read_text(encoding="utf-8"))
            except Exception:
                continue
            total = payload.get("total", {})
            lines = total.get("lines", {})
            pct = lines.get("pct")
            if isinstance(pct, (int, float)):
                return float(pct)
        return None

    def parse_rust_coverage_percent(self, payload: str) -> float | None:
        try:
            data = json.loads(payload)
        except json.JSONDecodeError:
            return None

        if not isinstance(data, dict):
            return None

        try:
            totals = data["data"][0]["totals"]["lines"]
            pct = totals.get("percent")
            if isinstance(pct, (int, float)):
                return float(pct)
        except Exception:
            return None
        return None

    def parse_json_from_mixed_output(self, text: str) -> dict | None:
        if not text:
            return None
        idx = text.find("{")
        if idx < 0:
            return None
        maybe_json = text[idx:]
        try:
            parsed = json.loads(maybe_json)
        except json.JSONDecodeError:
            return None
        if isinstance(parsed, dict):
            return parsed
        return None

    def collect_dotnet_outdated_packages(self, payload: dict) -> list[str]:
        lines: list[str] = []
        projects = payload.get("projects", [])
        if not isinstance(projects, list):
            return lines

        for project in projects:
            if not isinstance(project, dict):
                continue
            project_name = str(project.get("path", "unknown"))
            frameworks = project.get("frameworks", [])
            if not isinstance(frameworks, list):
                continue

            for framework in frameworks:
                if not isinstance(framework, dict):
                    continue
                tfm = str(framework.get("framework", "unknown"))
                for key in ("topLevelPackages", "transitivePackages"):
                    packages = framework.get(key, [])
                    if not isinstance(packages, list):
                        continue
                    for package in packages:
                        if not isinstance(package, dict):
                            continue
                        name = str(package.get("id", package.get("name", "unknown")))
                        resolved = str(
                            package.get("resolvedVersion", package.get("requestedVersion", package.get("version", "?")))
                        )
                        latest = str(package.get("latestVersion", package.get("latest", "?")))
                        if latest != "?" and resolved != latest:
                            lines.append(
                                f"- {name} ({key}, {tfm}, {project_name}): current={resolved}, latest={latest}"
                            )

        return sorted(set(lines))

    def section(self, name: str) -> None:
        print(f"\n=== {name} ===")

    def summarize_output(self, text: str, *, max_lines: int = 30) -> str:
        lines = [line.rstrip() for line in text.splitlines() if line.strip()]
        if not lines:
            return "No output captured."
        if len(lines) <= max_lines:
            return "\n".join(lines)
        trimmed = lines[:max_lines]
        trimmed.append(f"... ({len(lines) - max_lines} more lines omitted)")
        return "\n".join(trimmed)

    def _timeout_stream_to_text(self, value: str | bytes | None) -> str:
        if value is None:
            return ""
        if isinstance(value, bytes):
            return value.decode("utf-8", errors="replace")
        return value

    def _discover_node_projects(self) -> list[Path]:
        package_files = self.git_ls_files(["**/package.json"])
        projects: set[Path] = set()
        for rel_path in package_files:
            rel = Path(rel_path)
            if "node_modules" in rel.parts:
                continue
            projects.add(self.root / rel.parent)
        return sorted(projects)

    def _discover_dotnet_targets(self) -> list[Path]:
        sln_files = [self.root / rel for rel in self.git_ls_files(["**/*.sln"])]
        if sln_files:
            return sorted(sln_files)
        csproj_files = [self.root / rel for rel in self.git_ls_files(["**/*.csproj"])]
        return sorted(csproj_files)

    def git_ls_files(self, patterns: Iterable[str]) -> list[str]:
        cmd = ["git", "ls-files", "-z", "--", *patterns]
        result = subprocess.run(
            cmd,
            cwd=self.root,
            text=False,
            capture_output=True,
            check=False,
        )
        if result.returncode != 0:
            return []
        output = result.stdout.decode("utf-8", errors="replace")
        return [entry for entry in output.split("\x00") if entry]

    def rel(self, path: Path) -> str:
        try:
            return str(path.resolve().relative_to(self.root)) or "."
        except ValueError:
            return str(path)

    def format_cmd(self, cmd: list[str]) -> str:
        return " ".join(shlex.quote(part) for part in cmd)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Chic local quality gates for git hooks.")
    parser.add_argument("stage", choices=["pre-commit", "pre-push"], help="Hook stage to execute")
    parser.add_argument("remote_name", nargs="?", help="Git remote name (pre-push only)")
    parser.add_argument("remote_url", nargs="?", help="Git remote URL (pre-push only)")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    gate = QualityGate(args.stage, args.remote_name, args.remote_url)
    return gate.run()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
