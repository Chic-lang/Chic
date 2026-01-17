import { HomeTemplate } from "@/components/templates/HomeTemplate/HomeTemplate";
import { getLocaleFromParams } from "@/i18n/serverLocale";
import { loadMessages } from "@/i18n/messages";

export default async function Page({ params }: { params: Promise<{ locale: string }> }) {
  const locale = await getLocaleFromParams(params);
  const messages = await loadMessages(locale);
  const copy = (messages as any).pages.home;

  return <HomeTemplate locale={locale} copy={copy} />;
}
