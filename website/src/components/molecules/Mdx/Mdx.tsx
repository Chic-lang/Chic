import Link from "next/link";
import { MDXRemote } from "next-mdx-remote/rsc";
import rehypeHighlight from "rehype-highlight";
import remarkGfm from "remark-gfm";
import type { MDXComponents } from "mdx/types";
import type { Locale } from "@/i18n/locales";
import { stripLocaleFromPathname, withLocale } from "@/i18n/paths";

function defaultComponents(locale: Locale | null): MDXComponents {
  return {
    h1: ({ children, ...rest }) => <h2 {...rest}>{children}</h2>,
    h2: ({ children, ...rest }) => <h3 {...rest}>{children}</h3>,
    h3: ({ children, ...rest }) => <h4 {...rest}>{children}</h4>,
    a: ({ href, children, ...rest }) => {
      const url = href ?? "#";
      const isExternal = /^https?:\/\//.test(url);
      if (isExternal) {
        return (
          <a href={url} target="_blank" rel="noreferrer" {...rest}>
            {children}
          </a>
        );
      }

      if (url.startsWith("#")) {
        return (
          <a href={url} {...rest}>
            {children}
          </a>
        );
      }

      if (url.startsWith("/")) {
        const { pathname: pathnameNoLocale } = stripLocaleFromPathname(url);
        const hrefLocalized = locale ? withLocale(locale, pathnameNoLocale) : pathnameNoLocale;
        return (
          <Link href={hrefLocalized} {...rest}>
            {children}
          </Link>
        );
      }

      return (
        <Link href={url} {...rest}>
          {children}
        </Link>
      );
    }
  };
}

export function Mdx({ source, locale, components }: { source: string; locale?: Locale; components?: MDXComponents }) {
  return (
    <MDXRemote
      source={source}
      options={{
        mdxOptions: {
          remarkPlugins: [remarkGfm],
          rehypePlugins: [rehypeHighlight]
        }
      }}
      components={{ ...defaultComponents(locale ?? null), ...components }}
    />
  );
}
