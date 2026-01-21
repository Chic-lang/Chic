# Chic

Chic is an **alpha** AI-first programming language and toolchain.

This project is not production-ready yet. Expect sharp edges, incomplete features, and breaking changes as the language, runtime, and tooling stabilize.

- Project website: `https://chic-lang.com/`
- Spec: `SPEC.md`
- Docs index: `docs/README.md`
- XML docs → Markdown mapping: `docs/tooling/xml_to_markdown.md`
- Language tour: `docs/language/tour.md`
- Website development: `docs/website.md`

## Mission and goals

Chic’s mission is to be a first-class, modern programming language with a toolchain that’s explicitly built for AI-assisted development.

Core goals:

- AI-first feedback loops: high-quality diagnostics, structured outputs, deterministic builds, and workflows that are safe to drive from automated agents.
- Modern language design: expressive syntax, strong static checking, and an emphasis on clarity over ceremony.
- Native performance: a runtime and compilation model that targets “close to the metal” performance where it matters.
- First-class compute: strong foundations for numeric computing and accelerator-aware execution (CPU/GPU/distributed) as part of the standard ecosystem.
- Self-hosting: over time, move the compiler/toolchain and standard library implementation into Chic itself.

## Why Chic exists

Over the last few years, building software with AI in the loop has become normal. One thing that kept coming up: most programming languages and build pipelines were not designed for fast, structured, AI-assisted iteration. Even when the language is great, the tooling and feedback loops often aren’t.

Chic is built to make that workflow first-class: clear conventions, manifest-driven builds, explicit dependency graphs, deterministic runtime behavior, and a toolchain that’s designed to be driven by automated agents.

Chic aims to keep the performance and safety ceiling high while improving approachability and making “AI-first development” practical.

Design principles are captured in the spec (see “Philosophy” in `SPEC.md`).

## AI-first development (dogfooding)

This project is intentionally developed with AI-driven iteration as the default workflow. Large tasks are routinely executed as long-running agent sessions (often 10–20 hours end-to-end) that:

- apply repo-specific conventions automatically,
- keep the spec and implementation aligned,
- enforce artifact hygiene and deterministic caching,
- and raise the overall quality bar so work done by agents stays maintainable.

## Origins

Chic was inspired by a recurring tension in real-world systems: teams want high performance, predictability, and strong safety guarantees without sacrificing developer velocity and tooling quality.

Chic is an attempt to keep the ceiling high while making the day-to-day workflow simple, with first-class tooling for AI-assisted development. The project also builds on years of compiler experiments and prototypes that informed the design and the spec.

Chic is authored and maintained by Chad Vogel, building on decades of compiler and language experimentation. This repo is the first effort aimed at turning that accumulated work into a first-class, public language and toolchain.

## Current status

This repo is a monorepo that includes the `chic` CLI/compiler (currently implemented in Rust) plus Chic packages under `packages/`. The long-term goal is a self-hosted toolchain and standard library written in Chic.

This is the working repo used to build and stabilize the toolchain.

## Getting started (macOS/Linux)

```sh
cargo build --bin chic
./target/debug/chic --help
./target/debug/chic build
```

Create a small project from a template:

```sh
./target/debug/chic init --template app-console --output ./hello
./target/debug/chic build ./hello
```

More CLI examples: `docs/cli/README.md` and `chic help <command>`.

Language tour: `docs/language/tour.md`.

## Repository layout

- `src/`: Rust bootstrap compiler implementation
- `packages/`: Chic packages (Std, runtimes, tooling libraries)
- `docs/`: specification + guides + architecture notes
- `website/`: chic-lang.com (Next.js)
- `chic-vscode/`: VS Code extension (syntax + LSP client)

## Contributing

See `CONTRIBUTING.md` and `docs/CLA.md`. Issues use templates (bug/feature/docs) under `.github/ISSUE_TEMPLATE/`. Pull requests must be labeled `cla-signed` after the CLA is confirmed.

## License

Code is currently licensed under Apache-2.0 (`LICENSE`). Contributions are governed by a CLA (`docs/CLA.md`) intended to keep future licensing options open (for example, to better protect the project from hostile re-licensing/hosting scenarios while keeping the compiler usable for building applications).

## Maintainer

Chad Vogel (founder / maintainer)

- Project website: `https://chic-lang.com/`
- `https://www.linkedin.com/in/chad-vogel/`
