import { describe, expect, it } from "vitest";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { getBlogPostBySlug, listAllBlogPosts } from "@/lib/blog";
import { DEFAULT_LOCALE } from "@/i18n/locales";

const REPO_ROOT = path.resolve(__dirname, "../../..");

function mkTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "chic-blog-test-"));
}

describe("blog content", () => {
  it("lists blog posts for the default locale without fallback flags", () => {
    process.env.CHIC_WORKSPACE_ROOT = REPO_ROOT;
    const posts = listAllBlogPosts(DEFAULT_LOCALE);
    expect(posts.length).toBeGreaterThan(0);
    for (const post of posts) {
      expect(post.locale).toBe(DEFAULT_LOCALE);
      expect(post.sourceLocale).toBe(DEFAULT_LOCALE);
      expect(post.isFallback).toBe(false);
      expect(post.frontmatter.title).toBeTypeOf("string");
    }
  });

  it("falls back to en-US content when locale content is missing", () => {
    process.env.CHIC_WORKSPACE_ROOT = REPO_ROOT;
    const posts = listAllBlogPosts("ja-JP");
    expect(posts.length).toBeGreaterThan(0);
    for (const post of posts) {
      expect(post.locale).toBe("ja-JP");
      expect(post.sourceLocale).toBe(DEFAULT_LOCALE);
      expect(post.isFallback).toBe(true);
    }
  });

  it("finds a post by slug", () => {
    process.env.CHIC_WORKSPACE_ROOT = REPO_ROOT;
    const posts = listAllBlogPosts(DEFAULT_LOCALE);
    const first = posts[0];
    expect(first).toBeDefined();
    const found = getBlogPostBySlug(DEFAULT_LOCALE, first.slug);
    expect(found?.slug).toBe(first.slug);
  });

  it("prefers localized posts when they exist", () => {
    const tmp = mkTempDir();
    process.env.CHIC_WORKSPACE_ROOT = tmp;

    fs.mkdirSync(path.join(tmp, "website", "content", "blog", "en-US"), { recursive: true });
    fs.writeFileSync(
      path.join(tmp, "website", "content", "blog", "en-US", "hello.mdx"),
      "---\ntitle: Hello\ndate: 2026-01-01\n---\n"
    );
    fs.mkdirSync(path.join(tmp, "website", "content", "blog", "ja-JP"), { recursive: true });
    fs.writeFileSync(
      path.join(tmp, "website", "content", "blog", "ja-JP", "hello.mdx"),
      "---\ntitle: こんにちは\ndate: 2026-01-01\n---\n"
    );

    const posts = listAllBlogPosts("ja-JP");
    const hello = posts.find((p) => p.slug === "hello");
    expect(hello?.sourceLocale).toBe("ja-JP");
    expect(hello?.isFallback).toBe(false);

    fs.rmSync(tmp, { recursive: true, force: true });
  });
});
