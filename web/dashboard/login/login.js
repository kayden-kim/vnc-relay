// Sign-in page behavior: language choice, input checks, and the login request.
// The page is built here from the shared components in ./components/ (copies of
// app/components, made by `cargo xtask tokens`).
import { createAlert } from './components/alert.js';
import { createButton } from './components/button.js';
import { icon } from './components/icons.js';
import { createLanguageSelect } from './components/language-select.js';
import { createTextField, setTextFieldError } from './components/text-field.js';
import { LANGUAGES, chooseLanguage, readSavedLanguage, saveLanguage } from './language.js';

const LOGIN_PATH = '/api/v1/session';
const HOME_PATH = '/';
// 구현 판단(사용자 결정 없음): 응답이 없으면 이 시간 뒤에 요청을 끊는다.
const DEFAULT_TIMEOUT_MS = 10000;

// Copy from dashboard.md §9.3. The CPY number is in each comment.
// Keys are the copy names that update() uses.
const STRINGS = {
  ko: {
    title: 'VNC Relay 관리자 로그인', // CPY-006
    accountLabel: '계정명', // CPY-069
    passwordLabel: '비밀번호', // CPY-070
    accountRequired: '계정명을 입력하십시오.', // CPY-071
    passwordRequired: '비밀번호를 입력하십시오.', // CPY-072
    checking: '로그인을 확인하는 중입니다.', // CPY-073
    failed: '로그인할 수 없습니다. 계정명과 비밀번호를 확인한 뒤 다시 시도하십시오.', // CPY-074
    required: '로그인이 필요합니다. 다시 로그인하십시오.', // CPY-075
    requestFailed: '로그인 요청을 처리하지 못했습니다. 잠시 뒤 다시 시도하십시오.', // CPY-077
    submit: '로그인', // CPY-012
    submitBusy: '로그인 중…', // CPY-013
  },
  en: {
    title: 'VNC Relay administrator sign in',
    accountLabel: 'Account name',
    passwordLabel: 'Password',
    accountRequired: 'Enter an account name.',
    passwordRequired: 'Enter a password.',
    checking: 'Checking sign-in.',
    failed: 'Sign-in failed. Check the account name and password, then try again.',
    required: 'Sign-in is required. Sign in again.',
    requestFailed: 'The sign-in request could not be processed. Try again later.',
    submit: 'Sign in',
    submitBusy: 'Signing in…',
  },
};

// CPY-001: the language select's accessible name, the same in both languages.
const LANGUAGE_LABEL = '언어 (Language)';
// Each language is named in its own language.
const LANGUAGE_NAMES = { ko: '한국어', en: 'English' };
const BRAND_NAME = 'VNC Relay'; // CPY-003

// Each notice is an alert: its tone and the copy key of its title.
// The key is also set as data-notice on the alert.
const NOTICES = {
  required: { tone: 'neutral', copy: 'required' },
  checking: { tone: 'info', copy: 'checking' },
  failed: { tone: 'destructive', copy: 'failed' },
  'request-failed': { tone: 'destructive', copy: 'requestFailed' },
};

/** Browser storage, or null when the browser blocks it. */
function defaultStorage() {
  try {
    return globalThis.localStorage;
  } catch {
    return null;
  }
}

/** Sends the login request. Resolves to the HTTP status, or null when no response came. */
async function sendLogin(request, account, password, timeoutMs) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await request(LOGIN_PATH, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ account, password }),
      credentials: 'same-origin',
      cache: 'no-store',
      signal: controller.signal,
    });
    return response.status;
  } catch {
    return null;
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Builds the sign-in page in `root` (the `#login-root` element of index.html) and attaches
 * its behavior.
 * @param {HTMLElement} root
 * @param {object} [deps] browser services. Tests pass fakes.
 * @param {(input: string, init: RequestInit) => Promise<Response>} [deps.request]
 * @param {Storage | null} [deps.storage] `null` means no storage
 * @param {readonly string[]} [deps.preferredLanguages]
 * @param {(url: string) => void} [deps.navigate]
 * @param {number} [deps.timeoutMs]
 */
export function mountLogin(root, deps = {}) {
  const doc = root.ownerDocument;
  const request = deps.request ?? ((input, init) => globalThis.fetch(input, init));
  const storage = deps.storage === undefined ? defaultStorage() : deps.storage;
  const preferred = deps.preferredLanguages ?? globalThis.navigator?.languages ?? [];
  const navigate = deps.navigate ?? ((url) => globalThis.location.assign(url));
  const timeoutMs = deps.timeoutMs ?? DEFAULT_TIMEOUT_MS;

  const state = {
    lang: chooseLanguage(readSavedLanguage(storage), preferred),
    // Starts with CPY-075: the page was opened without a session.
    notice: 'required',
    accountError: false,
    passwordError: false,
    busy: false,
  };

  function el(tag, className, text) {
    const node = doc.createElement(tag);
    if (className) {
      node.className = className;
    }
    if (text !== undefined) {
      node.textContent = text;
    }
    return node;
  }

  // The controls are built once. Later changes update them in place, so the focused
  // control and its caret stay where they are.
  const initial = STRINGS[state.lang];
  const languageSelect = createLanguageSelect({
    id: 'login-language',
    label: LANGUAGE_LABEL,
    value: state.lang,
    options: LANGUAGES.map((value) => ({ value, label: LANGUAGE_NAMES[value] })),
    variant: 'form',
  });
  const select = languageSelect.querySelector('select');
  select.dataset.role = 'language';
  for (const option of select.querySelectorAll('option')) {
    option.lang = option.value;
  }

  const title = el('h1', 'login-title');
  title.id = 'login-title';

  // The alert lives in a live region. Its node is replaced only when the notice kind changes.
  const notices = el('div', 'login-notices');
  notices.setAttribute('aria-live', 'polite');
  let shownNotice = null;

  const accountField = createTextField({
    id: 'login-account',
    label: initial.accountLabel,
    required: true,
    autocomplete: 'username',
  });
  const accountInput = accountField.querySelector('input');
  accountInput.dataset.role = 'account';
  accountInput.setAttribute('autocapitalize', 'none');
  accountInput.setAttribute('spellcheck', 'false');

  const passwordField = createTextField({
    id: 'login-password',
    label: initial.passwordLabel,
    type: 'password',
    required: true,
    autocomplete: 'current-password',
  });
  const passwordInput = passwordField.querySelector('input');
  passwordInput.dataset.role = 'password';

  const submitButton = createButton({ variant: 'primary', type: 'submit', label: initial.submit });
  submitButton.id = 'login-submit';
  submitButton.dataset.role = 'submit';

  const form = el('form', 'login-form');
  form.noValidate = true;
  form.append(accountField, passwordField, submitButton);
  form.addEventListener('submit', (event) => {
    event.preventDefault();
    void submit();
  });

  const brandMark = el('span', 'login-mark');
  brandMark.append(icon('brand'));
  const brand = el('div', 'login-brand');
  brand.append(brandMark, el('span', 'login-brand-text', BRAND_NAME));

  const top = el('div', 'login-top');
  top.append(languageSelect);

  const card = el('section', 'login-card');
  card.setAttribute('aria-labelledby', 'login-title');
  card.append(title, notices, form);

  const main = el('main', 'login-main');
  main.append(brand, card);
  root.replaceChildren(top, main);

  /** Shows the notice for `state.notice`. The node is replaced only when the kind changes. */
  function renderNotice(copy) {
    if (state.notice !== shownNotice) {
      notices.replaceChildren();
      shownNotice = state.notice;
      if (state.notice) {
        const alert = createAlert({
          tone: NOTICES[state.notice].tone,
          title: copy[NOTICES[state.notice].copy],
        });
        alert.dataset.notice = state.notice;
        notices.append(alert);
      }
      return;
    }
    if (state.notice) {
      // Same kind: only the copy can change, so the live region sees a text change.
      const titleElement = notices.querySelector('.alert-title');
      const text = copy[NOTICES[state.notice].copy];
      if (titleElement.textContent !== text) {
        titleElement.textContent = text;
      }
    }
  }

  /** Puts `state` on the page. Text and attributes change in place. */
  function update() {
    const copy = STRINGS[state.lang];
    doc.documentElement.lang = state.lang;
    doc.title = copy.title;
    select.value = state.lang;
    title.textContent = copy.title;
    accountField.querySelector('.text-field-label').textContent = copy.accountLabel;
    passwordField.querySelector('.text-field-label').textContent = copy.passwordLabel;
    setTextFieldError(accountField, state.accountError ? copy.accountRequired : undefined);
    setTextFieldError(passwordField, state.passwordError ? copy.passwordRequired : undefined);
    renderNotice(copy);
    submitButton.querySelector('.button-label').textContent = copy[state.busy ? 'submitBusy' : 'submit'];
    submitButton.disabled = state.busy;
    if (state.busy) {
      submitButton.setAttribute('aria-busy', 'true');
    } else {
      submitButton.removeAttribute('aria-busy');
    }
  }

  async function submit() {
    if (state.busy) {
      return;
    }
    const account = accountInput.value;
    const password = passwordInput.value;
    state.accountError = account.trim() === '';
    state.passwordError = password === '';
    state.notice = null;
    if (state.accountError || state.passwordError) {
      // Nothing is sent while a field is empty.
      update();
      (state.accountError ? accountInput : passwordInput).focus();
      return;
    }

    state.busy = true;
    state.notice = 'checking';
    update();
    const status = await sendLogin(request, account, password, timeoutMs);
    if (status === 200) {
      // Stays busy: the browser is leaving this page.
      navigate(HOME_PATH);
      return;
    }
    state.busy = false;
    if (status === 401) {
      // The same notice covers a wrong password and an unknown account.
      passwordInput.value = '';
      state.notice = 'failed';
    } else {
      // No response, a time limit, or a server error. This is not a sign-in failure.
      state.notice = 'request-failed';
    }
    update();
    if (status === 401) {
      passwordInput.focus();
    }
  }

  accountInput.addEventListener('input', () => {
    if (state.accountError) {
      state.accountError = false;
      update();
    }
  });
  passwordInput.addEventListener('input', () => {
    if (state.passwordError) {
      state.passwordError = false;
      update();
    }
  });
  // Listens on the select itself. A change event does not bubble out of it in every browser.
  select.addEventListener('change', () => {
    if (!LANGUAGES.includes(select.value)) {
      return;
    }
    state.lang = select.value;
    saveLanguage(storage, state.lang);
    update();
  });

  update();
}
