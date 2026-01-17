import Link from "next/link";
import { listAllBlogPosts } from "@/lib/blog";
import { Prose } from "@/components/molecules/Prose/Prose";
import type { Locale } from "@/i18n/locales";
import { withLocale } from "@/i18n/paths";
import styles from "./BlogIndexTemplate.module.css";

const PAGE_SIZE = 10;

export function BlogIndexTemplate({ locale, page }: { locale: Locale; page: number }) {
  const posts = listAllBlogPosts();
  const totalPages = Math.max(1, Math.ceil(posts.length / PAGE_SIZE));
  const safePage = Math.min(Math.max(1, page), totalPages);

  const startIndex = (safePage - 1) * PAGE_SIZE;
  const pagePosts = posts.slice(startIndex, startIndex + PAGE_SIZE);

  const prevHref =
    safePage > 2
      ? withLocale(locale, `/blog/page/${safePage - 1}`)
      : safePage === 2
        ? withLocale(locale, "/blog")
        : null;
  const nextHref = safePage < totalPages ? withLocale(locale, `/blog/page/${safePage + 1}`) : null;

  return (
    <Prose>
      <div className={styles.meta}>
        <a href={withLocale(locale, "/blog/rss.xml")}>RSS</a>
        <span>
          Page {safePage} of {totalPages}
        </span>
      </div>

      <ul>
        {pagePosts.map((post) => (
          <li key={post.slug}>
            <Link href={withLocale(locale, `/blog/${post.slug}`)}>{post.frontmatter.title}</Link> —{" "}
            <time dateTime={post.frontmatter.date}>{post.frontmatter.date}</time>
          </li>
        ))}
      </ul>

      {totalPages > 1 ? (
        <nav className={styles.pager} aria-label="Pagination">
          {prevHref ? <Link href={prevHref}>Previous</Link> : <span aria-hidden="true">Previous</span>}
          {nextHref ? <Link href={nextHref}>Next</Link> : <span aria-hidden="true">Next</span>}
        </nav>
      ) : null}
    </Prose>
  );
}
