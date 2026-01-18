import { getRequestConfig } from "next-intl/server";
import { DEFAULT_LOCALE, isLocale, type Locale } from "@/i18n/locales";
import { loadMessages } from "@/i18n/messages";

export default getRequestConfig(async ({ requestLocale }) => {
  const locale =
    typeof requestLocale === "string" && isLocale(requestLocale) ? (requestLocale as Locale) : DEFAULT_LOCALE;

  return {
    locale,
    messages: await loadMessages(locale)
  };
});
