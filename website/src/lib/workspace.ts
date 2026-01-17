import fs from "node:fs";
import path from "node:path";

export function getWorkspaceRoot(): string {
  if (process.env.CHIC_WORKSPACE_ROOT) {
    return process.env.CHIC_WORKSPACE_ROOT;
  }

  const startDir = process.cwd();
  let current = startDir;

  for (let depth = 0; depth < 6; depth += 1) {
    const hasDocs = fs.existsSync(path.join(current, "docs", "mission.md"));
    const hasSpec = fs.existsSync(path.join(current, "SPEC.md"));
    const hasSiteContent = fs.existsSync(path.join(current, "website", "content", "blog"));

    if (hasDocs && hasSpec && hasSiteContent) {
      return current;
    }

    const parent = path.dirname(current);
    if (parent === current) break;
    current = parent;
  }

  return startDir;
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
