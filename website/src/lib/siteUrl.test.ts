import { describe, expect, it } from "vitest";
import { getSiteUrl } from "@/lib/siteUrl";

describe("getSiteUrl", () => {
  it("uses SITE_URL when set and strips trailing slash", () => {
    process.env.SITE_URL = "https://example.com/";
    delete process.env.NEXT_PUBLIC_SITE_URL;
    expect(getSiteUrl()).toBe("https://example.com");
  });

  it("keeps URLs without trailing slashes unchanged", () => {
    process.env.SITE_URL = "https://example.com";
    delete process.env.NEXT_PUBLIC_SITE_URL;
    expect(getSiteUrl()).toBe("https://example.com");
  });

  it("falls back to NEXT_PUBLIC_SITE_URL when SITE_URL is not set", () => {
    delete process.env.SITE_URL;
    process.env.NEXT_PUBLIC_SITE_URL = "https://public.example.com/";
    expect(getSiteUrl()).toBe("https://public.example.com");
  });

  it("falls back to the default when env vars are not set", () => {
    delete process.env.SITE_URL;
    delete process.env.NEXT_PUBLIC_SITE_URL;
    expect(getSiteUrl()).toBe("https://chic-lang.com");
  });
});
