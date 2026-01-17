import { Button } from "@/components/atoms/Button/Button";
import type { Locale } from "@/i18n/locales";
import { withLocale } from "@/i18n/paths";
import styles from "./HomeTemplate.module.css";

const REPO = "https://github.com/Chic-lang/Chic";

export type HomeCopy = {
  kicker: string;
  title: string;
  lede: string;
  getStarted: string;
  readDocs: string;
  viewOnGitHub: string;
  quickLinksTitle: string;
  quickLinks: {
    installTitle: string;
    installBody: string;
    learnTitle: string;
    learnBody: string;
    docsTitle: string;
    docsBody: string;
    blogTitle: string;
    blogBody: string;
  };
  whyTitle: string;
  whyLede: string;
  whyBullets: {
    aiFirst: string;
    correctness: string;
    determinism: string;
    selfHosting: string;
  };
  buildTitle: string;
  buildLede: string;
  buildTiles: {
    cliTitle: string;
    cliBody: string;
    wasmTitle: string;
    wasmBody: string;
    embeddedTitle: string;
    embeddedBody: string;
    toolingTitle: string;
    toolingBody: string;
  };
  involvedTitle: string;
  involvedLede: string;
  browseIssues: string;
  contributingGuide: string;
  readBlog: string;
};

export function HomeTemplate({ locale, copy }: { locale: Locale; copy: HomeCopy }) {
  return (
    <div>
      <section className={styles.hero} aria-labelledby="hero-title">
        <div>
          <div className={styles.kicker}>{copy.kicker}</div>
          <h1 id="hero-title">{copy.title}</h1>
          <p>{copy.lede}</p>
          <div className={styles.actions}>
            <Button href={withLocale(locale, "/learn")} variant="primary">
              {copy.getStarted}
            </Button>
            <Button href={withLocale(locale, "/docs")} variant="secondary">
              {copy.readDocs}
            </Button>
            <Button href={REPO} variant="secondary" external>
              {copy.viewOnGitHub}
            </Button>
          </div>
        </div>
        <div className={styles.heroCard}>
          <h2>{copy.quickLinksTitle}</h2>
          <div className={styles.grid}>
            <div className={styles.tile}>
              <h3 className={styles.tileTitle}>{copy.quickLinks.installTitle}</h3>
              <p>{copy.quickLinks.installBody}</p>
            </div>
            <div className={styles.tile}>
              <h3 className={styles.tileTitle}>{copy.quickLinks.learnTitle}</h3>
              <p>{copy.quickLinks.learnBody}</p>
            </div>
            <div className={styles.tile}>
              <h3 className={styles.tileTitle}>{copy.quickLinks.docsTitle}</h3>
              <p>{copy.quickLinks.docsBody}</p>
            </div>
            <div className={styles.tile}>
              <h3 className={styles.tileTitle}>{copy.quickLinks.blogTitle}</h3>
              <p>{copy.quickLinks.blogBody}</p>
            </div>
          </div>
        </div>
      </section>

      <section className={styles.section} aria-labelledby="why-title">
        <header className={styles.sectionHeader}>
          <h2 id="why-title">{copy.whyTitle}</h2>
          <p>{copy.whyLede}</p>
        </header>
        <ul className={styles.list}>
          <li>{copy.whyBullets.aiFirst}</li>
          <li>{copy.whyBullets.correctness}</li>
          <li>{copy.whyBullets.determinism}</li>
          <li>{copy.whyBullets.selfHosting}</li>
        </ul>
      </section>

      <section className={styles.section} aria-labelledby="build-title">
        <header className={styles.sectionHeader}>
          <h2 id="build-title">{copy.buildTitle}</h2>
          <p>{copy.buildLede}</p>
        </header>
        <div className={styles.grid}>
          <div className={styles.tile}>
            <h3 className={styles.tileTitle}>{copy.buildTiles.cliTitle}</h3>
            <p>{copy.buildTiles.cliBody}</p>
          </div>
          <div className={styles.tile}>
            <h3 className={styles.tileTitle}>{copy.buildTiles.wasmTitle}</h3>
            <p>{copy.buildTiles.wasmBody}</p>
          </div>
          <div className={styles.tile}>
            <h3 className={styles.tileTitle}>{copy.buildTiles.embeddedTitle}</h3>
            <p>{copy.buildTiles.embeddedBody}</p>
          </div>
          <div className={styles.tile}>
            <h3 className={styles.tileTitle}>{copy.buildTiles.toolingTitle}</h3>
            <p>{copy.buildTiles.toolingBody}</p>
          </div>
        </div>
      </section>

      <section className={styles.section} aria-labelledby="involved-title">
        <header className={styles.sectionHeader}>
          <h2 id="involved-title">{copy.involvedTitle}</h2>
          <p>{copy.involvedLede}</p>
        </header>
        <div className={styles.actions}>
          <Button href={`${REPO}/issues`} variant="secondary" external>
            {copy.browseIssues}
          </Button>
          <Button href={`${REPO}/blob/main/CONTRIBUTING.md`} variant="secondary" external>
            {copy.contributingGuide}
          </Button>
          <Button href={withLocale(locale, "/blog")} variant="secondary">
            {copy.readBlog}
          </Button>
        </div>
      </section>
    </div>
  );
}
