import fs from "node:fs";
import path from "node:path";

export function getWorkspaceRoot(): string {
  if (process.env.CHIC_WORKSPACE_ROOT) {
    return process.env.CHIC_WORKSPACE_ROOT;
  }

  const cwd = process.cwd();
  const parent = path.join(cwd, "..");

  if (fs.existsSync(path.join(cwd, "docs")) && fs.existsSync(path.join(cwd, "README.md"))) {
    return cwd;
  }

  if (fs.existsSync(path.join(parent, "docs")) && fs.existsSync(path.join(parent, "README.md"))) {
    return parent;
  }

  return cwd;
}

export function readWorkspaceTextFile(relativePath: string): string {
  const workspaceRoot = getWorkspaceRoot();
  const absolutePath = path.join(workspaceRoot, relativePath);
  return fs.readFileSync(absolutePath, "utf8");
}

export function workspaceFileExists(relativePath: string): boolean {
  const workspaceRoot = getWorkspaceRoot();
  return fs.existsSync(path.join(workspaceRoot, relativePath));
}

