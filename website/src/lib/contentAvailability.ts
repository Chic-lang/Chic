import fs from "node:fs";
import path from "node:path";
import type { Locale } from "@/i18n/locales";
import { getWorkspaceRoot } from "@/lib/workspace";

function docFilePath(locale: Locale, slug: string[]): string {
  return path.join(getWorkspaceRoot(), "website", "content", "docs", locale, ...slug) + ".mdx";
}

function blogFilePath(locale: Locale, slug: string): string {
  return path.join(getWorkspaceRoot(), "website", "content", "blog", locale, `${slug}.mdx`);
}

export function hasDocTranslation(locale: Locale, slug: string[]): boolean {
  return fs.existsSync(docFilePath(locale, slug));
}

export function hasBlogPostTranslation(locale: Locale, slug: string): boolean {
  return fs.existsSync(blogFilePath(locale, slug));
}

