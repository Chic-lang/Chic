import fs from "node:fs";
import path from "node:path";
import matter from "gray-matter";
import { DOCS, type DocEntry } from "@/content/docs";
import { DEFAULT_LOCALE, type Locale } from "@/i18n/locales";
import { getWorkspaceRoot } from "@/lib/workspace";
import { parseOptionalBoolean, parseRelatedLinks, type RelatedLink } from "@/lib/frontmatter";

export type DocFrontmatter = {
  title: string;
  description?: string;
  sourcePath?: string;
  relatedLinks?: RelatedLink[];
  contactBlock?: boolean;
};

export type DocPage = {
  slug: string[];
  locale: Locale;
  sourceLocale: Locale;
  isFallback: boolean;
  sourcePath: string;
  frontmatter: DocFrontmatter;
  content: string;
};

export type DocSummary = {
  slug: string[];
  locale: Locale;
  sourceLocale: Locale;
  isFallback: boolean;
  title: string;
  description?: string;
  sourcePath: string;
};

function getDocsDir(locale: Locale): string {
  return path.join(getWorkspaceRoot(), "website", "content", "docs", locale);
}

function getDocFilePath(locale: Locale, slug: string[]): string {
  return path.join(getDocsDir(locale), ...slug) + ".mdx";
}

function tryReadDoc(locale: Locale, slug: string[]): { frontmatter: DocFrontmatter; content: string } | null {
  const filePath = getDocFilePath(locale, slug);
  if (!fs.existsSync(filePath)) return null;

  const raw = fs.readFileSync(filePath, "utf8");
  const parsed = matter(raw);
  const data = parsed.data as Record<string, unknown>;
  const relatedLinks = parseRelatedLinks(data.relatedLinks);
  const contactBlock = parseOptionalBoolean(data.contactBlock);

  return {
    frontmatter: {
      ...(parsed.data as DocFrontmatter),
      relatedLinks,
      contactBlock
    },
    content: parsed.content
  };
}

export function findDocEntryBySlug(slug: string[]): DocEntry | undefined {
  return DOCS.find((doc) => doc.slug.join("/") === slug.join("/"));
}

export function listDocs(locale: Locale): DocSummary[] {
  return DOCS.map((doc) => {
    const localized = tryReadDoc(locale, doc.slug);
    if (localized) {
      return {
        slug: doc.slug,
        locale,
        sourceLocale: locale,
        isFallback: false,
        title: localized.frontmatter.title,
        description: localized.frontmatter.description,
        sourcePath: localized.frontmatter.sourcePath ?? doc.sourcePath
      };
    }

    const fallback = tryReadDoc(DEFAULT_LOCALE, doc.slug);
    if (fallback) {
      return {
        slug: doc.slug,
        locale,
        sourceLocale: DEFAULT_LOCALE,
        isFallback: locale !== DEFAULT_LOCALE,
        title: fallback.frontmatter.title,
        description: fallback.frontmatter.description,
        sourcePath: fallback.frontmatter.sourcePath ?? doc.sourcePath
      };
    }

    return {
      slug: doc.slug,
      locale,
      sourceLocale: DEFAULT_LOCALE,
      isFallback: true,
      title: doc.slug.join("/"),
      description: undefined,
      sourcePath: doc.sourcePath
    };
  });
}

export function getDocBySlug(locale: Locale, slug: string[]): DocPage | undefined {
  const entry = findDocEntryBySlug(slug);
  if (!entry) return undefined;

  const localized = tryReadDoc(locale, slug);
  if (localized) {
    return {
      slug,
      locale,
      sourceLocale: locale,
      isFallback: false,
      sourcePath: localized.frontmatter.sourcePath ?? entry.sourcePath,
      frontmatter: localized.frontmatter,
      content: localized.content
    };
  }

  const fallback = tryReadDoc(DEFAULT_LOCALE, slug);
  if (!fallback) return undefined;

  return {
    slug,
    locale,
    sourceLocale: DEFAULT_LOCALE,
    isFallback: locale !== DEFAULT_LOCALE,
    sourcePath: fallback.frontmatter.sourcePath ?? entry.sourcePath,
    frontmatter: fallback.frontmatter,
    content: fallback.content
  };
}
