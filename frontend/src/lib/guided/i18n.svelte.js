/**
 * Languages of the guided view: a small store and t(). The texts live in
 * texts.en.js and texts.de.js. A text is either a string with {name}
 * placeholders or a function of the values (for plurals).
 */
import { STORAGE_KEYS } from '../constants.js';
import en from './texts.en.js';
import de from './texts.de.js';

const DICTS = { en, de };
export const LANGS = ['en', 'de'];

function initialLang() {
  try {
    const saved = localStorage.getItem(STORAGE_KEYS.lang);
    if (DICTS[saved]) return saved;
  } catch {
    // No storage: follow the browser
  }
  return String(navigator.language || 'en').toLowerCase().startsWith('de') ? 'de' : 'en';
}

export const i18n = $state({ lang: initialLang() });
document.documentElement.lang = i18n.lang;

export function setLang(lang) {
  if (!DICTS[lang]) return;
  i18n.lang = lang;
  document.documentElement.lang = lang;
  try {
    localStorage.setItem(STORAGE_KEYS.lang, lang);
  } catch {
    // The choice just isn't remembered
  }
}

/** The text for key in the current language, English if it is missing there. */
export function t(key, vars = {}) {
  const text = DICTS[i18n.lang][key] ?? en[key];
  if (text === undefined) {
    console.warn(`Missing text: ${key}`);
    return key;
  }
  if (typeof text === 'function') return text(vars);
  return text.replace(/\{(\w+)\}/g, (m, name) => (vars[name] ?? m));
}

/** Whether a text exists. */
export const has = (key) => key in en;

/** Locale for numbers and dates. */
export const locale = () => (i18n.lang === 'de' ? 'de-CH' : 'en-US');

/** A whole number with thousands separators of the current language. */
export const fmt = (n) => Number(n).toLocaleString(locale());
