// Language choice for the help site root (docs/specs/help-site.md section 2.1).
// Same rule, storage key, and values as the dashboard (web/dashboard/app/language.js):
// the saved choice, then the first Korean or English browser preference, then English.
// The root page (HLP-002) moves to the chosen book at once. The not-found page (HLP-003)
// only saves the language of the link that the reader picks.

const STORAGE_KEY = 'vnc-relay.language';
const LANGUAGES = ['ko', 'en'];
const DEFAULT_LANGUAGE = 'en';

function savedLanguage() {
  try {
    return window.localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

function saveLanguage(language) {
  try {
    window.localStorage.setItem(STORAGE_KEY, language);
  } catch {
    // Storage is blocked. The link still opens the chosen book.
  }
}

function languageFromPreferences(tags) {
  for (const tag of tags) {
    const primary = String(tag).toLowerCase().split('-')[0];
    if (LANGUAGES.includes(primary)) {
      return primary;
    }
  }
  return DEFAULT_LANGUAGE;
}

for (const link of document.querySelectorAll('a[data-lang]')) {
  link.addEventListener('click', () => saveLanguage(link.dataset.lang));
}

if (document.documentElement.dataset.redirect === 'language') {
  const saved = savedLanguage();
  const preferences = navigator.languages?.length ? navigator.languages : [navigator.language];
  const language = LANGUAGES.includes(saved) ? saved : languageFromPreferences(preferences);
  // `replace` keeps the root out of the history, so Back does not return here.
  window.location.replace(new URL(`${language}/`, document.baseURI).href);
}
