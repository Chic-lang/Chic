export type DocEntry = {
  title: string;
  slug: string[];
  sourcePath: string;
  description?: string;
};

export const DOCS: DocEntry[] = [
  {
    title: "Mission",
    slug: ["mission"],
    sourcePath: "docs/mission.md",
    description: "High-level statement of intent for Chic."
  },
  {
    title: "Getting started",
    slug: ["getting-started"],
    sourcePath: "docs/getting-started.md",
    description: "First build, first program, and how a Chic project is laid out."
  },
  {
    title: "Language tour",
    slug: ["language", "tour"],
    sourcePath: "docs/language/tour.md",
    description: "A practical, developer-friendly tour of the language."
  },
  {
    title: "CLI overview",
    slug: ["cli"],
    sourcePath: "docs/cli/README.md",
    description: "CLI surface area and common commands."
  },
  {
    title: "Architecture overview",
    slug: ["architecture"],
    sourcePath: "docs/architecture.md",
    description: "How the compiler is structured and how design decisions are captured."
  },
  {
    title: "WASM backend notes",
    slug: ["wasm-backend"],
    sourcePath: "docs/wasm_backend.md",
    description: "WASM backend design notes and current status."
  },
  {
    title: "Web overview",
    slug: ["web", "overview"],
    sourcePath: "docs/web/overview.md",
    description: "Notes on Chic web work (overview)."
  },
  {
    title: "Web testing",
    slug: ["web", "testing"],
    sourcePath: "docs/web/testing.md",
    description: "Notes on web testing strategy."
  }
];

