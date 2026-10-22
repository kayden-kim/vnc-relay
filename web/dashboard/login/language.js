// Generated file. Do not edit. Source: web/dashboard/app/language.js
// Language choice for the dashboard pages: saved choice, then browser preferences, then English.
// The choice stays in browser storage. It is never sent to the server or to the management API.

export const LANGUAGES = ['ko', 'en'];
export const STORAGE_KEY = 'vnc-relay.language';
const DEFAULT_LANGUAGE = 'en';

/**
 * Returns the first preferred tag that is Korean or English. Other tags are skipped.
 * @param {readonly string[]} tags browser preferences in order, such as `navigator.languages`
 * @returns {'ko' | 'en'}
 */
export function languageFromPreferences(tags) {
  for (const tag of tags) {
    const primary = String(tag).toLowerCase().split('-')[0];
    if (primary === 'ko' || primary === 'en') {
      return primary;
    }
  }
  return DEFAULT_LANGUAGE;
}

/**
 * A saved choice wins when it is a supported language. Otherwise the browser preferences decide.
 * @param {string | null} saved the value read from storage, or null
 * @param {readonly string[]} tags browser preferences in order
 * @returns {'ko' | 'en'}
 */
export function chooseLanguage(saved, tags) {
  return LANGUAGES.includes(saved) ? saved : languageFromPreferences(tags);
}

/**
 * Reads the saved choice. Storage may be blocked; then there is no saved choice.
 * @param {Storage | null} storage
 * @returns {string | null}
 */
export function readSavedLanguage(storage) {
  try {
    return storage?.getItem(STORAGE_KEY) ?? null;
  } catch {
    return null;
  }
}

/**
 * Saves the choice. If storage is blocked, the choice lasts only for this page view.
 * @param {Storage | null} storage
 * @param {'ko' | 'en'} language
 */
export function saveLanguage(storage, language) {
  try {
    storage?.setItem(STORAGE_KEY, language);
  } catch {
    // Storage is blocked or full. The page still shows the chosen language.
  }
}
