import { describe, expect, it } from "vitest";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { hasBlogPostTranslation, hasDocTranslation } from "@/lib/contentAvailability";

function mkTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "chic-content-availability-"));
}

describe("contentAvailability", () => {
  it("returns true when doc/blog files exist", () => {
    const tmp = mkTempDir();
    process.env.CHIC_WORKSPACE_ROOT = tmp;

    fs.mkdirSync(path.join(tmp, "website", "content", "docs", "en-US"), { recursive: true });
    fs.writeFileSync(path.join(tmp, "website", "content", "docs", "en-US", "mission.mdx"), "---\ntitle: Mission\n---\n");

    fs.mkdirSync(path.join(tmp, "website", "content", "blog", "en-US"), { recursive: true });
    fs.writeFileSync(path.join(tmp, "website", "content", "blog", "en-US", "hello.mdx"), "---\ntitle: x\n---\n");

    expect(hasDocTranslation("en-US", ["mission"])).toBe(true);
    expect(hasBlogPostTranslation("en-US", "hello")).toBe(true);
    expect(hasDocTranslation("ja-JP", ["mission"])).toBe(false);

    fs.rmSync(tmp, { recursive: true, force: true });
  });
});

