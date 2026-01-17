import React from "react";
import { describe, expect, it } from "vitest";
import { render } from "@testing-library/react";
import { axe, toHaveNoViolations } from "jest-axe";
import type { Locale } from "@/i18n/locales";
import { SUPPORTED_LOCALES } from "@/i18n/locales";
import { HomeTemplate } from "@/components/templates/HomeTemplate/HomeTemplate";
import { SimplePageTemplate } from "@/components/templates/SimplePageTemplate/SimplePageTemplate";
import { Prose } from "@/components/molecules/Prose/Prose";
import { FallbackNotice } from "@/components/molecules/FallbackNotice/FallbackNotice";
import { RelatedLinks } from "@/components/molecules/RelatedLinks/RelatedLinks";
import { ContactBlockView } from "@/components/molecules/ContactBlock/ContactBlockView";

import enUS from "@/messages/en-US.json";
import esES from "@/messages/es-ES.json";
import frFR from "@/messages/fr-FR.json";
import itIT from "@/messages/it-IT.json";
import jaJP from "@/messages/ja-JP.json";
import ptBR from "@/messages/pt-BR.json";
import ruRU from "@/messages/ru-RU.json";
import trTR from "@/messages/tr-TR.json";
import zhCN from "@/messages/zh-CN.json";

expect.extend(toHaveNoViolations);

const MESSAGES: Record<Locale, any> = {
  "en-US": enUS,
  "es-ES": esES,
  "fr-FR": frFR,
  "it-IT": itIT,
  "ja-JP": jaJP,
  "pt-BR": ptBR,
  "ru-RU": ruRU,
  "tr-TR": trTR,
  "zh-CN": zhCN
};

function renderInDocument({ skipLabel, children }: { skipLabel: string; children: React.ReactNode }) {
  return render(
    <div>
      <a href="#main">{skipLabel}</a>
      <main id="main">{children}</main>
    </div>
  );
}

describe.each(SUPPORTED_LOCALES)("a11y smoke (%s)", (locale) => {
  it("homepage template has no obvious violations", async () => {
    const messages = MESSAGES[locale];
    const { container } = renderInDocument({
      skipLabel: messages.a11y.skipToContent,
      children: <HomeTemplate locale={locale} copy={messages.pages.home} />
    });

    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it("docs article layout has no obvious violations", async () => {
    const messages = MESSAGES[locale];
    const { container } = renderInDocument({
      skipLabel: messages.a11y.skipToContent,
      children: (
        <SimplePageTemplate title="Mission" lede="High-level statement of intent for Chic.">
          <Prose>
            <p>
              {messages.pages.docs.sourceLabel} <a href="https://example.com/source">docs/mission.md</a>
            </p>
            <FallbackNotice message={messages.i18n.fallbackNotice} />
            <h2>Purpose</h2>
            <p>Chic is built for deterministic, automation-friendly workflows.</p>
            <RelatedLinks
              locale={locale}
              title={messages.blocks.relatedLinks.title}
              links={[
                { title: "Getting started", href: "/docs/getting-started", description: "Build the CLI and run a build." },
                { title: "Language tour", href: "/docs/language/tour", description: "A quick tour of Chic syntax." }
              ]}
            />
            <ContactBlockView
              title={messages.blocks.contact.title}
              body={messages.blocks.contact.body}
              links={[
                { label: messages.blocks.contact.reportIssue, href: "https://example.com/issues/new", external: true },
                { label: messages.blocks.contact.browseIssues, href: "https://example.com/issues", external: true },
                { label: messages.blocks.contact.contributingGuide, href: "https://example.com/contributing", external: true },
                { label: messages.blocks.contact.community, href: "/community" }
              ]}
            />
          </Prose>
        </SimplePageTemplate>
      )
    });

    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });
});

