import { HomeTemplate } from "@/components/templates/HomeTemplate/HomeTemplate";
import { getLocaleFromParams } from "@/i18n/serverLocale";

export default async function Page({ params }: { params: Promise<{ locale: string }> }) {
  const locale = await getLocaleFromParams(params);
  return <HomeTemplate locale={locale} />;
}
