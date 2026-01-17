import type { NextRequest } from "next/server";
import { listAllBlogPosts } from "@/lib/blog";
import { DEFAULT_LOCALE, isLocale, type Locale } from "@/i18n/locales";
import { getSiteUrl } from "@/lib/siteUrl";

function escapeXml(text: string): string {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll("\"", "&quot;")
    .replaceAll("'", "&apos;");
}

export async function GET(_request: NextRequest, { params }: { params: Promise<{ locale: string }> }) {
  // Route handlers don't run through `layout.tsx`, so validate the locale here.
  const { locale: localeRaw } = await params;
  const locale: Locale = isLocale(localeRaw) ? localeRaw : DEFAULT_LOCALE;

  const siteUrl = getSiteUrl();
  const posts = listAllBlogPosts(locale);

  const items = posts
    .map((post) => {
      const url = `${siteUrl}/${locale}/blog/${post.slug}`;
      const title = escapeXml(post.frontmatter.title);
      const description = escapeXml(post.frontmatter.description ?? "");
      const pubDate = new Date(post.frontmatter.date).toUTCString();

      return `\n    <item>\n      <title>${title}</title>\n      <link>${url}</link>\n      <guid>${url}</guid>\n      <pubDate>${pubDate}</pubDate>\n      <description>${description}</description>\n    </item>`;
    })
    .join("");

  const xml = `<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\">\n  <channel>\n    <title>Chic Blog</title>\n    <link>${siteUrl}/${locale}/blog</link>\n    <description>Updates and roadmap notes as Chic evolves.</description>\n    ${items}\n  </channel>\n</rss>\n`;

  return new Response(xml, {
    headers: {
      "Content-Type": "application/rss+xml; charset=utf-8",
      "Cache-Control": "public, max-age=0, must-revalidate"
    }
  });
}
