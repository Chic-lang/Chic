import fs from "node:fs";
import path from "node:path";
import matter from "gray-matter";
import { getWorkspaceRoot } from "@/lib/workspace";

export type BlogPostFrontmatter = {
  title: string;
  date: string;
  author?: string;
  tags?: string[];
  description?: string;
};

export type BlogPost = {
  slug: string;
  frontmatter: BlogPostFrontmatter;
  content: string;
};

function getBlogDir(): string {
  return path.join(getWorkspaceRoot(), "website", "content", "blog");
}

export function listAllBlogPosts(): BlogPost[] {
  const blogDir = getBlogDir();
  const entries = fs.readdirSync(blogDir, { withFileTypes: true });
  const posts = entries
    .filter((entry) => entry.isFile() && entry.name.endsWith(".md"))
    .map((entry) => {
      const slug = entry.name.replace(/\\.md$/, "");
      const filePath = path.join(blogDir, entry.name);
      const raw = fs.readFileSync(filePath, "utf8");
      const parsed = matter(raw);

      return {
        slug,
        frontmatter: parsed.data as BlogPostFrontmatter,
        content: parsed.content
      };
    });

  return posts.sort((a, b) => (a.frontmatter.date < b.frontmatter.date ? 1 : -1));
}

export function getBlogPostBySlug(slug: string): BlogPost | undefined {
  return listAllBlogPosts().find((p) => p.slug === slug);
}
