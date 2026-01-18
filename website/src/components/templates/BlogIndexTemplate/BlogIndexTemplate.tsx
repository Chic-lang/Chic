import Link from "next/link";
import { listAllBlogPosts } from "@/lib/blog";
import { Prose } from "@/components/molecules/Prose/Prose";
import type { Locale } from "@/i18n/locales";
import { withLocale } from "@/i18n/paths";
import styles from "./BlogIndexTemplate.module.css";
import { getTranslations } from "next-intl/server";

const PAGE_SIZE = 10;

export async function BlogIndexTemplate({ locale, page }: { locale: Locale; page: number }) {
  const tBlog = await getTranslations({ locale, namespace: "pages.blog" });
  const tA11y = await getTranslations({ locale, namespace: "a11y" });

  const posts = listAllBlogPosts(locale);
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
        <a href={withLocale(locale, "/blog/rss.xml")}>{tBlog("rss")}</a>
        <span>
          {tBlog("pageOf", { page: safePage, total: totalPages })}
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
        <nav className={styles.pager} aria-label={tA11y("paginationNav")}>
          {prevHref ? <Link href={prevHref}>{tBlog("previous")}</Link> : <span aria-hidden="true">{tBlog("previous")}</span>}
          {nextHref ? <Link href={nextHref}>{tBlog("next")}</Link> : <span aria-hidden="true">{tBlog("next")}</span>}
        </nav>
      ) : null}
    </Prose>
  );
}
