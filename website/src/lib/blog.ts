import fs from "node:fs";
import path from "node:path";
import matter from "gray-matter";
import { getWorkspaceRoot } from "@/lib/workspace";
import { DEFAULT_LOCALE, type Locale } from "@/i18n/locales";

export type BlogPostFrontmatter = {
  title: string;
  date: string;
  author?: string;
  tags?: string[];
  description?: string;
};

export type BlogPost = {
  slug: string;
  locale: Locale;
  sourceLocale: Locale;
  isFallback: boolean;
  frontmatter: BlogPostFrontmatter;
  content: string;
};

function getBlogDir(locale: Locale): string {
  return path.join(getWorkspaceRoot(), "website", "content", "blog", locale);
}

function getBlogPostPath(locale: Locale, slug: string): string {
  return path.join(getBlogDir(locale), `${slug}.mdx`);
}

function listCanonicalBlogSlugs(): string[] {
  const blogDir = getBlogDir(DEFAULT_LOCALE);
  const entries = fs.readdirSync(blogDir, { withFileTypes: true });
  return entries
    .filter((entry) => entry.isFile() && entry.name.endsWith(".mdx"))
    .map((entry) => entry.name.replace(/\.mdx$/, ""))
    .sort();
}

function tryReadBlogPost(locale: Locale, slug: string): { frontmatter: BlogPostFrontmatter; content: string } | null {
  const filePath = getBlogPostPath(locale, slug);
  if (!fs.existsSync(filePath)) return null;

  const raw = fs.readFileSync(filePath, "utf8");
  const parsed = matter(raw);

  return {
    frontmatter: parsed.data as BlogPostFrontmatter,
    content: parsed.content
  };
}

export function listAllBlogPosts(locale: Locale): BlogPost[] {
  const slugs = listCanonicalBlogSlugs();

  const posts = slugs
    .map((slug) => {
      const localized = tryReadBlogPost(locale, slug);
      if (localized) {
        return {
          slug,
          locale,
          sourceLocale: locale,
          isFallback: false,
          frontmatter: localized.frontmatter,
          content: localized.content
        };
      }

      const fallback = tryReadBlogPost(DEFAULT_LOCALE, slug);
      if (!fallback) return null;

      return {
        slug,
        locale,
        sourceLocale: DEFAULT_LOCALE,
        isFallback: locale !== DEFAULT_LOCALE,
        frontmatter: fallback.frontmatter,
        content: fallback.content
      };
    })
    .filter((p): p is BlogPost => Boolean(p));

  return posts.sort((a, b) => (a.frontmatter.date < b.frontmatter.date ? 1 : -1));
}

export function getBlogPostBySlug(locale: Locale, slug: string): BlogPost | undefined {
  return listAllBlogPosts(locale).find((p) => p.slug === slug);
}
