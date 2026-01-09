import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

import de from './locales/de/common.json';
import en from './locales/en/common.json';

export const defaultNS = 'common';

export const resources = {
  de: { common: de },
  en: { common: en },
} as const;

i18n.use(initReactI18next).init({
  resources,
  lng: 'de',
  fallbackLng: 'en',
  defaultNS,
  interpolation: {
    escapeValue: false,
  },
});

export { i18n };
export { useTranslation } from 'react-i18next';
