import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";
import { DEFAULT_LOCALE, isLocale } from "./src/i18n/locales";
import { pickLocaleFromAcceptLanguage } from "./src/i18n/acceptLanguage";

const STATIC_FILE_EXTENSIONS = new Set([
  "avif",
  "css",
  "gif",
  "ico",
  "jpeg",
  "jpg",
  "js",
  "map",
  "otf",
  "png",
  "svg",
  "txt",
  "ttf",
  "webp",
  "woff",
  "woff2"
]);

function isStaticAssetPath(pathname: string): boolean {
  const lastSegment = pathname.split("/").pop() ?? "";
  const dotIndex = lastSegment.lastIndexOf(".");
  if (dotIndex <= 0) return false;
  const ext = lastSegment.slice(dotIndex + 1).toLowerCase();
  return STATIC_FILE_EXTENSIONS.has(ext);
}

function looksLikeLocaleSegment(segment: string): boolean {
  return /^[a-zA-Z]{2}-[a-zA-Z]{2}$/.test(segment);
}

function pathnameWithoutLocale(pathname: string): string {
  const [_, first, ...rest] = pathname.split("/");
  if (!first) return "/";
  if (!isLocale(first)) return pathname;
  const remainder = `/${rest.join("/")}`;
  return remainder === "/" ? "/" : remainder.replace(/\/$/, "");
}

export function middleware(request: NextRequest) {
  const { pathname } = request.nextUrl;
  const [_, first] = pathname.split("/");

  if (isStaticAssetPath(pathname)) {
    return NextResponse.next();
  }

  // Redirect `/` based on Accept-Language (optional), otherwise default.
  if (!first) {
    const preferred = pickLocaleFromAcceptLanguage(request.headers.get("accept-language"));
    return NextResponse.redirect(new URL(`/${preferred}`, request.url));
  }

  // Redirect any non-prefixed path deterministically to the default locale.
  if (!isLocale(first)) {
    // If the path already looks like a locale segment but isn't supported, allow it through
    // so the `[locale]` route can 404 deterministically.
    if (looksLikeLocaleSegment(first)) {
      return NextResponse.next();
    }

    return NextResponse.redirect(new URL(`/${DEFAULT_LOCALE}${pathname === "/" ? "" : pathname}`, request.url));
  }

  // Pass locale/path info to server components (e.g. for `<html lang>`).
  const requestHeaders = new Headers(request.headers);
  requestHeaders.set("x-chic-locale", first);
  requestHeaders.set("x-chic-pathname", pathname);
  requestHeaders.set("x-chic-pathname-no-locale", pathnameWithoutLocale(pathname));

  return NextResponse.next({
    request: {
      headers: requestHeaders
    }
  });
}

export const config = {
  matcher: ["/((?!_next).*)"]
};

