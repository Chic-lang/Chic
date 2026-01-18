import { describe, expect, it } from "vitest";
import { getDocBySlug } from "@/lib/docs";
import { getBlogPostBySlug } from "@/lib/blog";

describe("i18n fallback policy", () => {
  it("falls back docs to en-US when missing", () => {
    const doc = getDocBySlug("ja-JP", ["mission"]);
    expect(doc).toBeDefined();
    expect(doc?.locale).toBe("ja-JP");
    expect(doc?.isFallback).toBe(true);
    expect(doc?.sourceLocale).toBe("en-US");
  });

  it("falls back blog posts to en-US when missing", () => {
    const post = getBlogPostBySlug("fr-FR", "hello-from-chic");
    expect(post).toBeDefined();
    expect(post?.locale).toBe("fr-FR");
    expect(post?.isFallback).toBe(true);
    expect(post?.sourceLocale).toBe("en-US");
  });

  it("does not fallback when requesting en-US", () => {
    const doc = getDocBySlug("en-US", ["mission"]);
    expect(doc).toBeDefined();
    expect(doc?.isFallback).toBe(false);
    expect(doc?.sourceLocale).toBe("en-US");
  });
});

