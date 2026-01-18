import { describe, expect, it } from "vitest";
import { alternatesForPath, canonicalUrl } from "@/i18n/seo";

describe("seo helpers", () => {
  it("generates a canonical URL with locale prefix", () => {
    process.env.SITE_URL = "https://example.com/";
    expect(canonicalUrl("en-US", "/en-US/docs/mission")).toBe("https://example.com/en-US/docs/mission");
  });

  it("generates language alternates for all supported locales and x-default", () => {
    process.env.SITE_URL = "https://example.com";
    const alternates = alternatesForPath("fr-FR", "/fr-FR/docs/mission");
    expect(alternates?.canonical).toBe("https://example.com/fr-FR/docs/mission");
    expect(alternates?.languages?.["en-US"]).toBe("https://example.com/en-US/docs/mission");
    expect(alternates?.languages?.["x-default"]).toBe("https://example.com/en-US/docs/mission");
  });
});

