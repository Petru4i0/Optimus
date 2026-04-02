import { en } from "../locales/en";
import { ru } from "../locales/ru";
import type { Translation } from "../locales/types";
import { useAppStore } from "../store/appStore";

type SupportedLocale = "en" | "ru";

const dictionaries: Record<SupportedLocale, Translation> = {
  en,
  ru,
};

export function useTranslation(): Translation {
  const locale = useAppStore((state) => state.locale);
  return dictionaries[locale] ?? en;
}
