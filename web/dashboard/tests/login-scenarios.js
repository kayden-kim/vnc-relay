// Scenario checks for the sign-in page (dashboard.md §7: DASH-017, 018, 024, 025, 027).
// One page load runs one case, ?case=dash-0xx, with fake services. The result goes to <main id="result">.
import { mountLogin } from '../login/login.js';
import { STORAGE_KEY } from '../login/language.js';

// Exact copy from dashboard.md §9.3.
const COPY = {
  ko: {
    accountLabel: '계정명',
    passwordLabel: '비밀번호',
    accountRequired: '계정명을 입력하십시오.',
    passwordRequired: '비밀번호를 입력하십시오.',
    checking: '로그인을 확인하는 중입니다.',
    failed: '로그인할 수 없습니다. 계정명과 비밀번호를 확인한 뒤 다시 시도하십시오.',
    required: '로그인이 필요합니다. 다시 로그인하십시오.',
    requestFailed: '로그인 요청을 처리하지 못했습니다. 잠시 뒤 다시 시도하십시오.',
    submit: '로그인',
    submitBusy: '로그인 중…',
  },
  en: {
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

// Maps a notice key (data-notice) to its copy key (data-text).
const NOTICE_TEXT_KEY = {
  required: 'required',
  checking: 'checking',
  failed: 'failed',
  'request-failed': 'requestFailed',
};

const host = document.getElementById('host');
const result = document.getElementById('result');
const failures = [];
let cspViolations = 0;
document.addEventListener('securitypolicyviolation', () => {
  cspViolations += 1;
});

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));
const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const check = (label, ok) => {
  if (!ok) {
    failures.push(label);
  }
};

// Map-backed storage stand-in.
function memoryStorage() {
  const items = new Map();
  return {
    items,
    getItem: (key) => (items.has(key) ? items.get(key) : null),
    setItem: (key, value) => {
      items.set(key, String(value));
    },
  };
}

function jsonResponse(status, body = { error: { code: status === 401 ? 'login_failed' : 'internal' } }) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

// Loads the real markup from login/index.html, so the test does not keep a copy.
async function freshPage() {
  const response = await fetch('../login/index.html', { cache: 'no-store' });
  const parsed = new DOMParser().parseFromString(await response.text(), 'text/html');
  return document.importNode(parsed.getElementById('login-root'), true);
}

async function mount(deps) {
  const root = await freshPage();
  host.replaceChildren(root);
  mountLogin(root, deps);
  return root;
}

function refs(root) {
  return {
    account: root.querySelector('[data-role="account"]'),
    password: root.querySelector('[data-role="password"]'),
    submit: root.querySelector('[data-role="submit"]'),
    language: root.querySelector('[data-role="language"]'),
    form: root.querySelector('form'),
  };
}

function fill(root, account, password) {
  const { account: accountInput, password: passwordInput } = refs(root);
  accountInput.value = account;
  passwordInput.value = password;
}

function submit(root) {
  refs(root).form.requestSubmit();
}

function shownNotices(root) {
  return [...root.querySelectorAll('[data-notice]')]
    .filter((element) => !element.hidden)
    .map((element) => element.dataset.notice);
}

function checkNotice(root, label, key) {
  const lang = document.documentElement.lang;
  const shown = shownNotices(root);
  const text = root.querySelector(`[data-notice="${key}"] [data-text]`)?.textContent;
  check(`${label}: only the ${key} notice is shown`, shown.length === 1 && shown[0] === key);
  check(`${label}: ${key} notice text`, text === COPY[lang][NOTICE_TEXT_KEY[key]]);
}

function checkFieldError(root, field, copyKey, label) {
  const lang = document.documentElement.lang;
  const element = root.querySelector(`[data-field-error="${field}"]`);
  check(
    `${label}: ${field} error shown with copy`,
    element !== null && !element.hidden && element.textContent.trim() === COPY[lang][copyKey],
  );
  check(`${label}: ${field} input marked invalid`, refs(root)[field].getAttribute('aria-invalid') === 'true');
}

// No horizontal scroll at the current width. The harness runs each case at several widths.
function checkLayout(label) {
  const page = document.documentElement;
  check(`${label}: no horizontal scroll`, page.scrollWidth <= page.clientWidth);
}

async function caseDash017() {
  const storage = memoryStorage();
  const calls = [];
  let navigated = null;
  const root = await mount({
    storage,
    preferredLanguages: ['ko-KR'],
    request: async (url, init) => {
      calls.push({ url, init });
      return jsonResponse(200, { account: 'admin' });
    },
    navigate: (url) => {
      navigated = url;
    },
  });
  fill(root, 'admin', 'correct-password');
  submit(root);
  await tick();
  check(
    'DASH-017: one POST to /api/v1/session',
    calls.length === 1 && calls[0].url === '/api/v1/session' && calls[0].init.method === 'POST',
  );
  const init = calls[0]?.init ?? {};
  const sent = JSON.parse(init.body ?? 'null');
  check(
    'DASH-017: body holds only account and password',
    sent !== null
      && Object.keys(sent).sort().join() === 'account,password'
      && sent.account === 'admin'
      && sent.password === 'correct-password',
  );
  check('DASH-017: JSON content type', init.headers?.['Content-Type'] === 'application/json');
  check('DASH-017: navigates to the home page after 200', navigated === '/');
  check('DASH-017: nothing is stored in the browser by sign-in', storage.items.size === 0);
  checkLayout('DASH-017');
}

async function caseDash018() {
  const calls = [];
  const deny = async (url, init) => {
    calls.push(init);
    return jsonResponse(401);
  };
  let root = await mount({ storage: memoryStorage(), preferredLanguages: ['ko-KR'], request: deny, navigate() {} });
  submit(root);
  await tick();
  check('DASH-018: empty input sends nothing', calls.length === 0);
  checkFieldError(root, 'account', 'accountRequired', 'DASH-018 empty');
  checkFieldError(root, 'password', 'passwordRequired', 'DASH-018 empty');

  fill(root, 'admin', '');
  submit(root);
  await tick();
  check('DASH-018: a missing password is not sent', calls.length === 0);

  fill(root, 'admin', 'wrong-password');
  submit(root);
  await tick();
  check('DASH-018: wrong credentials send one request', calls.length === 1);
  checkNotice(root, 'DASH-018 wrong credentials', 'failed');
  check('DASH-018: password cleared after a failure', refs(root).password.value === '');
  check('DASH-018: account name kept after a failure', refs(root).account.value === 'admin');

  // A rate-limited failure gets the same 401 response, so it shows the same notice.
  root = await mount({ storage: memoryStorage(), preferredLanguages: ['ko-KR'], request: deny, navigate() {} });
  fill(root, 'admin', 'another-password');
  submit(root);
  await tick();
  checkNotice(root, 'DASH-018 rate limited', 'failed');
  checkLayout('DASH-018');
}

async function caseDash024() {
  const cases = [
    [['ko-KR'], 'ko'],
    [['en-US'], 'en'],
    [['ja-JP'], 'en'],
    [['ja-JP', 'ko'], 'ko'],
  ];
  for (const [preferred, expected] of cases) {
    const storage = memoryStorage();
    const root = await mount({
      storage,
      preferredLanguages: preferred,
      request: async () => jsonResponse(401),
      navigate() {},
    });
    const label = `DASH-024 ${preferred.join(',')}`;
    check(`${label}: document language`, document.documentElement.lang === expected);
    check(`${label}: language select value`, refs(root).language.value === expected);
    check(`${label}: initial notice copy`, root.querySelector('[data-notice="required"] [data-text]').textContent === COPY[expected].required);
    check(`${label}: nothing is saved without a choice`, storage.items.size === 0);
  }
  checkLayout('DASH-024');
}

async function caseDash025() {
  const storage = memoryStorage();
  const sent = [];
  const deny = async (url, init) => {
    sent.push({ url, init });
    return jsonResponse(401);
  };
  let root = await mount({ storage, preferredLanguages: ['ko-KR'], request: deny, navigate() {} });
  check('DASH-025: starts in Korean from browser preferences', document.documentElement.lang === 'ko');
  fill(root, 'admin', 'pw');

  const select = refs(root).language;
  select.value = 'en';
  select.dispatchEvent(new Event('change'));
  check('DASH-025: copy switches at once', refs(root).submit.textContent === COPY.en.submit);
  check('DASH-025: document language follows the choice', document.documentElement.lang === 'en');
  check('DASH-025: account name kept on language switch', refs(root).account.value === 'admin');
  check('DASH-025: choice saved in browser storage', storage.items.get(STORAGE_KEY) === 'en');

  submit(root);
  await tick();
  const init = sent[0]?.init ?? {};
  const body = JSON.parse(init.body ?? 'null') ?? {};
  check(
    'DASH-025: management API request holds no language value',
    sent.length === 1
      && sent[0].url === '/api/v1/session'
      && Object.keys(body).sort().join() === 'account,password'
      && !JSON.stringify(init.headers ?? {}).toLowerCase().includes('lang'),
  );
  checkNotice(root, 'DASH-025 after sign-in', 'failed');

  // Reload: a new page view with the same storage and a Korean browser preference.
  root = await mount({ storage, preferredLanguages: ['ko-KR'], request: deny, navigate() {} });
  check(
    'DASH-025: saved choice beats browser preferences after reload',
    document.documentElement.lang === 'en' && refs(root).language.value === 'en',
  );
  check('DASH-025: copy follows the saved choice after reload', refs(root).submit.textContent === COPY.en.submit);
  checkLayout('DASH-025');
}

async function caseDash027() {
  const preferred = { storage: memoryStorage(), preferredLanguages: ['ko-KR'], navigate() {} };

  // Server error: not the sign-in failure notice, and the form can be sent again.
  let root = await mount({ ...preferred, request: async () => jsonResponse(500) });
  fill(root, 'admin', 'pw');
  submit(root);
  await tick();
  checkNotice(root, 'DASH-027 server error', 'request-failed');
  check('DASH-027 server error: not the sign-in failure notice', !shownNotices(root).includes('failed'));
  check('DASH-027 server error: button enabled for a retry', !refs(root).submit.disabled);
  check('DASH-027 server error: password kept for a retry', refs(root).password.value === 'pw');

  // No response at all.
  root = await mount({
    ...preferred,
    request: async () => {
      throw new TypeError('network error');
    },
  });
  fill(root, 'admin', 'pw');
  submit(root);
  await tick();
  checkNotice(root, 'DASH-027 no response', 'request-failed');

  // Time limit: the request waits until the page aborts it.
  root = await mount({
    ...preferred,
    timeoutMs: 20,
    request: (url, init) =>
      new Promise((resolve, reject) => {
        init.signal.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')));
      }),
  });
  fill(root, 'admin', 'pw');
  submit(root);
  await wait(80);
  checkNotice(root, 'DASH-027 time limit', 'request-failed');

  // Duplicate sending: the second submit is ignored while the first request waits.
  const calls = [];
  let release = () => {};
  root = await mount({
    ...preferred,
    request: (url, init) => {
      calls.push(init);
      return new Promise((resolve) => {
        release = resolve;
      });
    },
  });
  fill(root, 'admin', 'pw');
  submit(root);
  submit(root);
  await tick();
  check('DASH-027 duplicate: one request while waiting', calls.length === 1);
  check(
    'DASH-027 duplicate: button disabled with busy copy while waiting',
    refs(root).submit.disabled && refs(root).submit.textContent === COPY.ko.submitBusy,
  );
  release(jsonResponse(500));
  await tick();
  checkNotice(root, 'DASH-027 duplicate after response', 'request-failed');
  check('DASH-027 duplicate: button enabled after the response', !refs(root).submit.disabled);
  checkLayout('DASH-027');
}

const CASES = {
  'dash-017': caseDash017,
  'dash-018': caseDash018,
  'dash-024': caseDash024,
  'dash-025': caseDash025,
  'dash-027': caseDash027,
};

const caseName = new URLSearchParams(location.search).get('case') ?? '';
if (!Object.hasOwn(CASES, caseName)) {
  failures.push(`unknown case: ${caseName}`);
} else {
  try {
    await CASES[caseName]();
  } catch (error) {
    failures.push(`${caseName} threw: ${error instanceof Error ? error.message : String(error)}`);
  }
}
check('no CSP violations', cspViolations === 0);
result.dataset.result = failures.length === 0 ? 'pass' : 'fail';
result.dataset.failures = failures.join(' | ');
result.dataset.cspViolations = String(cspViolations);
