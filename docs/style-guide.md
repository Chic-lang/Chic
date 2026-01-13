# Documentation style guide

This repository is becoming public. Documentation should be written for a new developer who has never used Chic before.

## Voice and tone

- Use clear, direct language. Prefer short sentences and concrete examples.
- Write in the second person when describing actions (“Run…”, “Create…”, “Add…”).
- Avoid marketing language, internal project history, or planning notes.
- Prefer “Chic” for the language and “`chic`” for the CLI.

## Structure (recommended)

Use a consistent outline so readers can scan quickly.

- **Title**: describes the goal (“Build and run a project”, “Write your first library”).
- **Prerequisites** (when needed): tools or setup required.
- **In this guide** (optional): 3–6 bullets describing what the reader will learn.
- **Steps**: numbered sections with short headings.
- **Next steps**: links to related docs.
- **See also** (optional): spec/CLI reference links.

## Examples

- Keep examples small and runnable.
- Use fenced code blocks with the `chic` language tag for Chic code.
- For shell commands, use `sh` and include the expected working directory when it matters.

## Link hygiene

- Prefer repository-relative links (`docs/...`, `SPEC.md`).
- Link to the spec for authoritative language rules, but do not make the spec required reading for basic tasks.

## What does not belong in public docs

- Task lists, roadmaps, “stub” placeholders, or implementation TODOs.
- Migration guides or compatibility notes for unreleased versions.
- Internal-only commands or workflows that do not apply to end users.

