export type DocEntry = {
  slug: string[];
  sourcePath: string;
};

export const DOCS: DocEntry[] = [
  {
    slug: ["mission"],
    sourcePath: "docs/mission.md"
  },
  {
    slug: ["getting-started"],
    sourcePath: "docs/getting-started.md"
  },
  {
    slug: ["language", "tour"],
    sourcePath: "docs/language/tour.md"
  },
  {
    slug: ["cli"],
    sourcePath: "docs/cli/README.md"
  },
  {
    slug: ["architecture"],
    sourcePath: "docs/architecture.md"
  },
  {
    slug: ["wasm-backend"],
    sourcePath: "docs/wasm_backend.md"
  },
  {
    slug: ["web", "overview"],
    sourcePath: "docs/web/overview.md"
  },
  {
    slug: ["web", "testing"],
    sourcePath: "docs/web/testing.md"
  }
];
