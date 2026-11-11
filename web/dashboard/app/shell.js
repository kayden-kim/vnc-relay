// Shared page chrome for protected screens: the navigation bar (help, language, sign-out) and the sign-out request.

import { createButton } from './components/button.js';
import { createLanguageSelect } from './components/language-select.js';
import { createNavbar } from './components/navbar.js';
import { createLink } from './components/link.js';
import { LOGOUT_TIMEOUT_MS, deleteSession, getSession, isUnauthorized } from './api.js';
import { BRAND_NAME, LANGUAGE_LABEL, LANGUAGE_NAMES } from './copy.js';
import { LANGUAGES } from './language.js';
import { parseSession } from './model.js';

export const LOGIN_URL = '/login/';
export const LOGOUT_UNCONFIRMED_URL = '/login/?notice=logout-unconfirmed';

export function defaultStorage() {
  try {
    return globalThis.localStorage;
  } catch {
    return null;
  }
}

// Navigation bar of one screen. `idPrefix` keeps element ids unique per screen.
export function buildNavbar({ t, lang, idPrefix, active = 'operations', onLanguageChange, onSignOut }) {
  const language = createLanguageSelect({
    id: `${idPrefix}-language`,
    label: LANGUAGE_LABEL,
    value: lang,
    options: LANGUAGES.map((value) => ({ value, label: LANGUAGE_NAMES[value] })),
    variant: 'navbar',
  });
  const select = language.querySelector('select');
  for (const option of select.options) {
    option.lang = option.value;
  }
  select.addEventListener('change', () => onLanguageChange(select.value));

  const logout = createButton({ variant: 'navbar', label: t.signOut, icon: 'logout' });
  logout.id = `${idPrefix}-logout`;
  logout.addEventListener('click', () => void onSignOut());

  const help = createLink({
    href: `https://kayden-kim.github.io/vnc-relay/${lang}/`,
    label: t.help,
    icon: 'help',
    iconSize: 'md',
    newTabText: t.opensInNewTab,
  });
  help.className = 'navbar-item help-link';

  const navbar = createNavbar({
    brand: BRAND_NAME,
    items: [
      { label: t.operations, href: '/', current: active === 'operations', icon: 'grid' },
      { label: t.logs, href: '/#/logs', current: active === 'logs', icon: 'log' },
      { label: t.settings, href: '/#/settings', current: active === 'settings', icon: 'settings' },
    ],
    help,
    language,
    action: logout,
    label: t.navLabel,
  });
  return { navbar, select, logout };
}

// Sign-out request. Resolves to the URL to move to: the login page on 204 or 401,
// or the login page with the unconfirmed notice when the outcome is unknown.
export async function signOutUrl(request, csrfToken) {
  try {
    const token =
      csrfToken ?? parseSession(await getSession(request, { timeoutMs: LOGOUT_TIMEOUT_MS })).csrfToken;
    await deleteSession(request, { csrfToken: token, timeoutMs: LOGOUT_TIMEOUT_MS });
    return LOGIN_URL;
  } catch (error) {
    return isUnauthorized(error) ? LOGIN_URL : LOGOUT_UNCONFIRMED_URL;
  }
}
