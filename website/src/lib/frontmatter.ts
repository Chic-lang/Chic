export type RelatedLink = {
  title: string;
  href: string;
  description?: string;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function parseRelatedLinks(value: unknown): RelatedLink[] {
  if (!Array.isArray(value)) return [];

  return value
    .map((item): RelatedLink | null => {
      if (!isRecord(item)) return null;
      const title = typeof item.title === "string" ? item.title : null;
      const href = typeof item.href === "string" ? item.href : null;
      const description = typeof item.description === "string" ? item.description : undefined;
      if (!title || !href) return null;
      return { title, href, description };
    })
    .filter((v): v is RelatedLink => Boolean(v));
}

export function parseOptionalBoolean(value: unknown): boolean | undefined {
  return typeof value === "boolean" ? value : undefined;
}

