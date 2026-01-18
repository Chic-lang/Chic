import { describe, expect, it } from "vitest";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { getWorkspaceRoot, readWorkspaceTextFile, workspaceFileExists } from "@/lib/workspace";

function mkTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "chic-workspace-test-"));
}

describe("workspace helpers", () => {
  it("returns CHIC_WORKSPACE_ROOT when set", () => {
    const tmp = mkTempDir();
    process.env.CHIC_WORKSPACE_ROOT = tmp;
    expect(getWorkspaceRoot()).toBe(tmp);
    fs.rmSync(tmp, { recursive: true, force: true });
  });

  it("discovers workspace root by marker files when env var is not set", () => {
    const tmp = mkTempDir();
    delete process.env.CHIC_WORKSPACE_ROOT;

    fs.mkdirSync(path.join(tmp, "docs"), { recursive: true });
    fs.writeFileSync(path.join(tmp, "docs", "mission.md"), "Mission");
    fs.writeFileSync(path.join(tmp, "SPEC.md"), "Spec");
    fs.mkdirSync(path.join(tmp, "website", "content", "blog"), { recursive: true });

    const deep = path.join(tmp, "website", "src", "x", "y");
    fs.mkdirSync(deep, { recursive: true });

    const cwd = process.cwd();
    try {
      process.chdir(deep);
      expect(fs.realpathSync(getWorkspaceRoot())).toBe(fs.realpathSync(tmp));
    } finally {
      process.chdir(cwd);
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("reads workspace files relative to root", () => {
    const tmp = mkTempDir();
    process.env.CHIC_WORKSPACE_ROOT = tmp;
    fs.mkdirSync(path.join(tmp, "docs"), { recursive: true });
    fs.writeFileSync(path.join(tmp, "docs", "mission.md"), "Hello");
    expect(workspaceFileExists("docs/mission.md")).toBe(true);
    expect(readWorkspaceTextFile("docs/mission.md")).toBe("Hello");
    fs.rmSync(tmp, { recursive: true, force: true });
  });

  it("returns start directory when no workspace markers are found", () => {
    const tmp = mkTempDir();
    delete process.env.CHIC_WORKSPACE_ROOT;

    const cwd = process.cwd();
    try {
      process.chdir(tmp);
      expect(fs.realpathSync(getWorkspaceRoot())).toBe(fs.realpathSync(tmp));
    } finally {
      process.chdir(cwd);
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });
});
