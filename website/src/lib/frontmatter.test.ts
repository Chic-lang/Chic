import { describe, expect, it } from "vitest";
import { parseOptionalBoolean, parseRelatedLinks } from "@/lib/frontmatter";

describe("parseRelatedLinks", () => {
  it("returns [] for non-arrays", () => {
    expect(parseRelatedLinks(null)).toEqual([]);
    expect(parseRelatedLinks({})).toEqual([]);
  });

  it("filters invalid entries and preserves valid ones", () => {
    expect(
      parseRelatedLinks([
        null,
        { title: "Missing href" },
        { href: "/docs", title: "Docs" },
        { href: "https://example.com", title: "External", description: "desc" }
      ])
    ).toEqual([
      { href: "/docs", title: "Docs" },
      { href: "https://example.com", title: "External", description: "desc" }
    ]);
  });
});

describe("parseOptionalBoolean", () => {
  it("returns boolean values and undefined otherwise", () => {
    expect(parseOptionalBoolean(true)).toBe(true);
    expect(parseOptionalBoolean(false)).toBe(false);
    expect(parseOptionalBoolean("true")).toBeUndefined();
  });
});

