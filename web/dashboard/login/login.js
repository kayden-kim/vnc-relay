// Sign-in page behavior: language choice, input checks, and the login request.
// The markup lives in index.html. This module sets text, visibility, and attributes only.
import { LANGUAGES, chooseLanguage, readSavedLanguage, saveLanguage } from './language.js';

const LOGIN_PATH = '/api/v1/session';
const HOME_PATH = '/';
// Time limit for the sign-in request. management-api-v1 §2.3 names a time limit for it
// but sets no value. This value is a 구현 판단 until the user decides.
const DEFAULT_TIMEOUT_MS = 10000;

// Copy from dashboard.md §9.3. The CPY number is in each comment.
// Keys match the data-text attributes in index.html.
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

/** Browser storage, or null when the browser blocks it. */
function defaultStorage() {
  try {
    return globalThis.localStorage;
  } catch {
    return null;
  }
}

/**
 * Attaches the sign-in behavior to the markup in `root` (the `#login-root` element of index.html).
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

  const select = root.querySelector('[data-role="language"]');
  const accountInput = root.querySelector('[data-role="account"]');
  const passwordInput = root.querySelector('[data-role="password"]');
  const submitButton = root.querySelector('[data-role="submit"]');
  const form = root.querySelector('form');

  const state = {
    lang: chooseLanguage(readSavedLanguage(storage), preferred),
    // Starts with CPY-075: the page was opened without a session.
    notice: 'required',
    accountError: false,
    passwordError: false,
    busy: false,
  };

  function setFieldError(input, errorElement, on) {
    errorElement.hidden = !on;
    input.setAttribute('aria-invalid', String(on));
  }

  function render() {
    const strings = STRINGS[state.lang];
    doc.documentElement.lang = state.lang;
    doc.title = strings.title;
    select.value = state.lang;
    for (const element of root.querySelectorAll('[data-text]')) {
      element.textContent = strings[element.dataset.text];
    }
    submitButton.textContent = strings[state.busy ? 'submitBusy' : 'submit'];
    submitButton.disabled = state.busy;
    submitButton.setAttribute('aria-busy', String(state.busy));
    for (const element of root.querySelectorAll('[data-notice]')) {
      element.hidden = element.dataset.notice !== state.notice;
    }
    setFieldError(accountInput, root.querySelector('[data-field-error="account"]'), state.accountError);
    setFieldError(passwordInput, root.querySelector('[data-field-error="password"]'), state.passwordError);
  }

  /**
   * Sends the login request. Resolves to the HTTP status, or null when there is
   * no response or the time limit passes.
   */
  async function sendLogin(account, password) {
    const controller = new AbortController();
    const timer = globalThis.setTimeout(() => controller.abort(), timeoutMs);
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
      globalThis.clearTimeout(timer);
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
      // Nothing is sent when a field is empty.
      render();
      (state.accountError ? accountInput : passwordInput).focus();
      return;
    }

    state.busy = true;
    state.notice = 'checking';
    render();
    const status = await sendLogin(account, password);
    if (status === 200) {
      // Stays busy: the browser is leaving this page.
      navigate(HOME_PATH);
      return;
    }
    state.busy = false;
    if (status === 401) {
      // Same notice for a wrong password, an unknown account, and a rate limit.
      passwordInput.value = '';
      state.notice = 'failed';
    } else {
      // No response, a time limit, or a server error. Not a sign-in failure.
      state.notice = 'request-failed';
    }
    render();
    if (status === 401) {
      passwordInput.focus();
    }
  }

  accountInput.addEventListener('input', () => {
    if (state.accountError) {
      state.accountError = false;
      render();
    }
  });
  passwordInput.addEventListener('input', () => {
    if (state.passwordError) {
      state.passwordError = false;
      render();
    }
  });
  select.addEventListener('change', () => {
    if (!LANGUAGES.includes(select.value)) {
      return;
    }
    state.lang = select.value;
    saveLanguage(storage, state.lang);
    render();
  });
  form.addEventListener('submit', (event) => {
    event.preventDefault();
    void submit();
  });

  render();
}
