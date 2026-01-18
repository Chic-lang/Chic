import { describe, expect, it } from "vitest";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { findDocEntryBySlug, getDocBySlug, listDocs } from "@/lib/docs";
import { DEFAULT_LOCALE } from "@/i18n/locales";

const REPO_ROOT = path.resolve(__dirname, "../../..");

function mkTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "chic-docs-test-"));
}

describe("docs content", () => {
  it("lists docs for en-US without fallback flags", () => {
    process.env.CHIC_WORKSPACE_ROOT = REPO_ROOT;
    const docs = listDocs(DEFAULT_LOCALE);
    expect(docs.length).toBeGreaterThan(0);
    for (const doc of docs) {
      expect(doc.locale).toBe(DEFAULT_LOCALE);
      expect(doc.sourceLocale).toBe(DEFAULT_LOCALE);
      expect(doc.isFallback).toBe(false);
      expect(doc.title).toBeTypeOf("string");
    }
  });

  it("falls back to en-US doc content when locale content is missing", () => {
    process.env.CHIC_WORKSPACE_ROOT = REPO_ROOT;
    const docs = listDocs("fr-FR");
    expect(docs.length).toBeGreaterThan(0);
    for (const doc of docs) {
      expect(doc.locale).toBe("fr-FR");
      expect(doc.sourceLocale).toBe(DEFAULT_LOCALE);
      expect(doc.isFallback).toBe(true);
    }
  });

  it("finds and loads a doc by slug when available", () => {
    process.env.CHIC_WORKSPACE_ROOT = REPO_ROOT;
    const entry = findDocEntryBySlug(["mission"]);
    expect(entry?.sourcePath).toBeTypeOf("string");
    const page = getDocBySlug(DEFAULT_LOCALE, ["mission"]);
    expect(page?.frontmatter.title).toBeTypeOf("string");
    expect(page?.content).toBeTypeOf("string");
  });

  it("returns deterministic placeholders when a doc file is missing", () => {
    const tmp = mkTempDir();
    process.env.CHIC_WORKSPACE_ROOT = tmp;
    // Intentionally do not create any docs files under website/content/docs/* so all docs miss.
    fs.mkdirSync(path.join(tmp, "website", "content", "docs"), { recursive: true });

    const docs = listDocs("ja-JP");
    expect(docs.length).toBeGreaterThan(0);
    expect(docs[0]?.isFallback).toBe(true);
    expect(docs[0]?.title).toBeTypeOf("string");

    const missing = getDocBySlug("ja-JP", ["mission"]);
    expect(missing).toBeUndefined();

    fs.rmSync(tmp, { recursive: true, force: true });
  });

  it("prefers localized docs when they exist", () => {
    const tmp = mkTempDir();
    process.env.CHIC_WORKSPACE_ROOT = tmp;

    fs.mkdirSync(path.join(tmp, "website", "content", "docs", "en-US"), { recursive: true });
    fs.writeFileSync(path.join(tmp, "website", "content", "docs", "en-US", "mission.mdx"), "---\ntitle: Mission\n---\n");

    fs.mkdirSync(path.join(tmp, "website", "content", "docs", "fr-FR"), { recursive: true });
    fs.writeFileSync(path.join(tmp, "website", "content", "docs", "fr-FR", "mission.mdx"), "---\ntitle: Mission FR\n---\n");

    const page = getDocBySlug("fr-FR", ["mission"]);
    expect(page?.sourceLocale).toBe("fr-FR");
    expect(page?.isFallback).toBe(false);
    expect(page?.frontmatter.title).toBe("Mission FR");

    fs.rmSync(tmp, { recursive: true, force: true });
  });

  it("returns undefined for unknown slugs", () => {
    process.env.CHIC_WORKSPACE_ROOT = REPO_ROOT;
    expect(getDocBySlug("en-US", ["does-not-exist"])).toBeUndefined();
  });

  it("uses sourcePath from frontmatter when provided", () => {
    const tmp = mkTempDir();
    process.env.CHIC_WORKSPACE_ROOT = tmp;

    fs.mkdirSync(path.join(tmp, "website", "content", "docs", "en-US"), { recursive: true });
    fs.writeFileSync(
      path.join(tmp, "website", "content", "docs", "en-US", "mission.mdx"),
      "---\ntitle: Mission\nsourcePath: docs/custom-mission.md\n---\n"
    );

    const docs = listDocs("en-US");
    const mission = docs.find((d) => d.slug.join("/") === "mission");
    expect(mission?.sourcePath).toBe("docs/custom-mission.md");

    const page = getDocBySlug("en-US", ["mission"]);
    expect(page?.sourcePath).toBe("docs/custom-mission.md");

    fs.rmSync(tmp, { recursive: true, force: true });
  });
});
