import { Button } from "@/components/atoms/Button/Button";
import styles from "./HomeTemplate.module.css";

const REPO = "https://github.com/Chic-lang/Chic";

export function HomeTemplate() {
  return (
    <div>
      <section className={styles.hero} aria-labelledby="hero-title">
        <div>
          <div className={styles.kicker}>Alpha · AI-first development</div>
          <h1 id="hero-title">Chic is an alpha programming language and toolchain.</h1>
          <p>
            Chic is designed for tight, structured feedback loops: clear diagnostics, deterministic builds, and
            workflows that are safe to automate.
          </p>
          <div className={styles.actions}>
            <Button href="/learn" variant="primary">
              Get started
            </Button>
            <Button href="/docs" variant="secondary">
              Read the docs
            </Button>
            <Button href={REPO} variant="secondary" external>
              View on GitHub
            </Button>
          </div>
        </div>
        <div className={styles.heroCard}>
          <h2>Quick links</h2>
          <div className={styles.grid}>
            <div className={styles.tile}>
              <h3 className={styles.tileTitle}>Install</h3>
              <p>Build the `chic` CLI from source and run your first program.</p>
            </div>
            <div className={styles.tile}>
              <h3 className={styles.tileTitle}>Learn</h3>
              <p>Take a short tour of the language and how projects are structured.</p>
            </div>
            <div className={styles.tile}>
              <h3 className={styles.tileTitle}>Docs</h3>
              <p>Read curated docs from this repo, including the mission and getting-started guide.</p>
            </div>
            <div className={styles.tile}>
              <h3 className={styles.tileTitle}>Blog</h3>
              <p>Updates and roadmap notes as Chic evolves.</p>
            </div>
          </div>
        </div>
      </section>

      <section className={styles.section} aria-labelledby="why-title">
        <header className={styles.sectionHeader}>
          <h2 id="why-title">Why Chic</h2>
          <p>Chic is built around predictable behavior, tooling-first ergonomics, and long-term self-hosting.</p>
        </header>
        <ul className={styles.list}>
          <li>AI-first feedback loops: structured outputs and diagnostics designed for automation.</li>
          <li>Correctness by design: clear ownership/borrowing and explicit runtime behavior.</li>
          <li>Determinism: builds and runtime behavior are designed to be predictable and cache-friendly.</li>
          <li>Self-hosting: over time, more of the toolchain and standard library moves into Chic itself.</li>
        </ul>
      </section>

      <section className={styles.section} aria-labelledby="build-title">
        <header className={styles.sectionHeader}>
          <h2 id="build-title">Build with Chic</h2>
          <p>Explore the ecosystem areas Chic is building toward.</p>
        </header>
        <div className={styles.grid}>
          <div className={styles.tile}>
            <h3 className={styles.tileTitle}>CLI tools</h3>
            <p>Manifests, deterministic builds, and test workflows designed for tight iteration.</p>
          </div>
          <div className={styles.tile}>
            <h3 className={styles.tileTitle}>WebAssembly</h3>
            <p>WASM backend work in progress, with clear docs on constraints and goals.</p>
          </div>
          <div className={styles.tile}>
            <h3 className={styles.tileTitle}>Embedded</h3>
            <p>Explicit runtime layers and `no_std` planning to support constrained environments.</p>
          </div>
          <div className={styles.tile}>
            <h3 className={styles.tileTitle}>Tooling &amp; compiler</h3>
            <p>Design notes and spec-backed implementation to keep behavior intentional and documented.</p>
          </div>
        </div>
      </section>

      <section className={styles.section} aria-labelledby="involved-title">
        <header className={styles.sectionHeader}>
          <h2 id="involved-title">Get involved</h2>
          <p>Chic is developed in the open. Issues and PRs are the primary collaboration path today.</p>
        </header>
        <div className={styles.actions}>
          <Button href={`${REPO}/issues`} variant="secondary" external>
            Browse issues
          </Button>
          <Button href={`${REPO}/blob/main/CONTRIBUTING.md`} variant="secondary" external>
            Contributing guide
          </Button>
          <Button href="/blog" variant="secondary">
            Read the blog
          </Button>
        </div>
      </section>
    </div>
  );
}

