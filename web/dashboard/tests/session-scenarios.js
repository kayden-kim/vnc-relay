// Scenario checks for the session detail screen (dashboard.md SCR-002: DASH-006, 009 to 013, 026).
// One page load runs one case, ?case=dash-0xx&lang=ko|en, against a fake management API.
// The result goes to <main id="result">. Wording comes from app/copy.js, which holds the spec §9.3 CPY table.
import { mountSession } from '../app/session.js';
import { COPY } from '../app/copy.js';
import { formatAddress, formatMode, formatTransport } from '../app/format.js';

const host = document.getElementById('host');
const result = document.getElementById('result');
const failures = [];
let cspViolations = 0;
document.addEventListener('securitypolicyviolation', () => {
  cspViolations += 1;
});

const lang = new URLSearchParams(location.search).get('lang') === 'en' ? 'en' : 'ko';
const t = COPY[lang];

const AS_OF = '2026-10-22T01:02:03.000Z';
const STARTED = '2026-10-22T00:30:00.000Z';
const ENDED = '2026-10-22T00:45:00.000Z';
const SESSION_ID = 'sess_1';
const DETAIL = `GET /api/v1/sessions/${SESSION_ID}`;
const TERMINATE = `POST /api/v1/sessions/${SESSION_ID}/terminate`;
const TIME_IN = /\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} \S/;

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const delayed = (ms, response) => new Promise((resolve) => setTimeout(() => resolve(response), ms));
// Waits until `ready()` holds or `ms` passes; the caller's check then reports the outcome either way.
async function waitFor(ready, ms = 5000) {
  const until = Date.now() + ms;
  while (!ready() && Date.now() < until) await wait(20);
}
const check = (label, passed) => {
  if (!passed) failures.push(label);
};

// Fake management API. Routes are keyed "METHOD path" and receive their call number for that key.
const json = (status, body) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });
const ok = (body) => json(200, body);
const fail = (status, code) => json(status, { error: { code } });

const activeSession = (overrides = {}) => ({
  session_id: SESSION_ID,
  state: 'active',
  mode: 'mode1',
  viewer_transport: 'wss',
  started_at: STARTED,
  bytes_viewer_to_target: 1536,
  bytes_target_to_viewer: 1048575,
  viewer_address: { ip: '192.0.2.10', port: 51000 },
  target_address: { ip: '198.51.100.5', port: 5900 },
  ...overrides,
});

const closedSession = (overrides = {}) => ({
  session_id: SESSION_ID,
  mode: 'mode1',
  viewer_transport: 'wss',
  started_at: STARTED,
  ended_at: ENDED,
  bytes_viewer_to_target: 1536,
  bytes_target_to_viewer: 1048575,
  close_cause: 'peer_eof',
  ...overrides,
});

const DEFAULT_ROUTES = {
  'GET /api/v1/session': () =>
    ok({
      csrf_token: 'csrf-token-1',
      dashboard: { poll_secs: 60, query_timeout_secs: 5, terminate_timeout_secs: 10 },
    }),
  [DETAIL]: () => ok({ as_of: AS_OF, session: activeSession() }),
  [TERMINATE]: () => ok({ as_of: AS_OF, result: 'terminated', closed_session: closedSession() }),
};

function fakeApi(overrides = {}) {
  const routes = { ...DEFAULT_ROUTES, ...overrides };
  const calls = [];
  const counts = new Map();
  const request = (path, init = {}) => {
    const key = `${init.method ?? 'GET'} ${path}`;
    const n = (counts.get(key) ?? 0) + 1;
    counts.set(key, n);
    calls.push({ key });
    const route = routes[key];
    return Promise.resolve().then(() => (route ? route(n) : fail(404, 'not_found')));
  };
  return { request, calls, count: (key) => counts.get(key) ?? 0 };
}

function mount(api, id = SESSION_ID) {
  const root = document.createElement('div');
  host.replaceChildren(root);
  const navigations = [];
  const handle = mountSession(root, id, {
    request: api.request,
    storage: { getItem: () => null, setItem() {} },
    preferredLanguages: [lang === 'en' ? 'en-US' : 'ko-KR'],
    navigate: (url) => navigations.push(url),
  });
  return { root, navigations, api, handle };
}

const titleOf = (root) => root.querySelector('.page-title')?.textContent.trim() ?? null;
const noticeOf = (root) => root.querySelector('.session-notice .alert-title')?.textContent.trim() ?? null;
const terminateOf = (root) => root.querySelector('#session-terminate');
const enabled = (root) => terminateOf(root) !== null && !terminateOf(root).disabled;
const dialogOf = (root) => root.querySelector('dialog.dialog');
const confirmOf = (root) => dialogOf(root)?.querySelector('.dialog-actions button:last-child');
const cancelOf = (root) => dialogOf(root)?.querySelector('.dialog-actions button:first-child');
const backOf = (root) => [...root.querySelectorAll('a')].find((link) => link.textContent.trim() === t.back) ?? null;
const statValue = (root, label) => {
  const stat = [...root.querySelectorAll('.stat')].find((node) => node.textContent.includes(label));
  return stat?.querySelector('.stat-value')?.textContent.trim() ?? null;
};

function checkLayout(label) {
  const page = document.documentElement;
  check(`${label}: no horizontal scroll`, page.scrollWidth <= page.clientWidth);
}

// DASH-006: the detail shows the session facts and an enabled terminate action for an active session.
async function caseDash006() {
  const api = fakeApi();
  const { root } = mount(api);
  await waitFor(() => enabled(root));
  check('DASH-006: title shows the session id', titleOf(root) === t.sessionTitle.replace('{session_id}', SESSION_ID));
  check('DASH-006: state shown as received', statValue(root, t.stateLabel) === 'active');
  check('DASH-006: mode label', statValue(root, t.modeLabel) === formatMode('mode1'));
  check('DASH-006: transport label', statValue(root, t.transportLabel) === formatTransport('wss'));
  check('DASH-006: started time', TIME_IN.test(statValue(root, t.startedLabel) ?? ''));
  check(
    'DASH-006: viewer address',
    statValue(root, t.viewerAddressLabel) === formatAddress({ ip: '192.0.2.10', port: 51000 }),
  );
  check(
    'DASH-006: target address',
    statValue(root, t.targetAddressLabel) === formatAddress({ ip: '198.51.100.5', port: 5900 }),
  );
  check('DASH-006: viewer to target bytes in KiB', statValue(root, t.directionViewerToTarget) === '1.5 KiB');
  check('DASH-006: target to viewer bytes in MiB', statValue(root, t.directionTargetToViewer) === '1.0 MiB');
  check('DASH-006: no notice on a live session', noticeOf(root) === null);
  check('DASH-006: one detail read on load', api.count(DETAIL) === 1);
  checkLayout('DASH-006');
}

// DASH-009: cancel closes the dialog without a request and returns focus to the terminate button.
async function caseDash009() {
  const api = fakeApi();
  const { root } = mount(api);
  await waitFor(() => enabled(root));
  terminateOf(root).focus();
  terminateOf(root).click();
  await wait(50);
  check('DASH-009: confirm dialog opens', dialogOf(root)?.open === true);
  // DASH-015: Tab and Shift+Tab stay between the two dialog actions. The keydown is synthetic; the real keyboard path was checked in the browser.
  confirmOf(root).focus();
  confirmOf(root).dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true }));
  check('DASH-015: Tab wraps from confirm to cancel', document.activeElement === cancelOf(root));
  cancelOf(root).dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true, cancelable: true }));
  check('DASH-015: Shift+Tab wraps from cancel to confirm', document.activeElement === confirmOf(root));
  cancelOf(root).click();
  await wait(100);
  check('DASH-009: cancel closes the dialog', dialogOf(root)?.open === false);
  check('DASH-009: cancel sends no request', api.count(TERMINATE) === 0);
  check('DASH-009: focus returns to terminate', document.activeElement === terminateOf(root));
  check('DASH-009: terminate stays available', enabled(root));
  checkLayout('DASH-009');
}

// DASH-010: the action is blocked while terminate runs. A completed terminate shows not-found and sends no second terminate.
async function caseDash010() {
  const api = fakeApi({
    [TERMINATE]: () => delayed(300, ok({ as_of: AS_OF, result: 'terminated', closed_session: closedSession() })),
  });
  const { root } = mount(api);
  await waitFor(() => enabled(root));
  terminateOf(root).click();
  confirmOf(root).click();
  await wait(50);
  check('DASH-010: action disabled while terminate runs', !enabled(root));
  check('DASH-010: busy label shown', terminateOf(root)?.textContent === t.terminating);
  await waitFor(() => noticeOf(root) === t.notFound);
  check('DASH-010: completed terminate shows not-found', noticeOf(root) === t.notFound);
  check('DASH-010: one terminate sent', api.count(TERMINATE) === 1);
  check('DASH-010: action stays disabled after completion', !enabled(root));
  check('DASH-010: no detail read after completion', api.count(DETAIL) === 1);
  check('DASH-010: back link remains', backOf(root) !== null);
  checkLayout('DASH-010');
}

// DASH-007: a session gone before the detail read, or gone during a refresh, shows not-found, keeps terminate off, and keeps the list link.
async function caseDash007() {
  const loadApi = fakeApi({ [DETAIL]: () => fail(404, 'not_found') });
  const loaded = mount(loadApi);
  await waitFor(() => noticeOf(loaded.root) === t.notFound);
  check('DASH-007: gone before read shows not-found', noticeOf(loaded.root) === t.notFound);
  check('DASH-007: gone before read hides the facts', loaded.root.querySelector('.summary-grid') === null);
  check('DASH-007: gone before read keeps terminate off', !enabled(loaded.root));
  check('DASH-007: list link kept', backOf(loaded.root)?.getAttribute('href') === '/');
  loaded.handle.dispose();

  const api = fakeApi({
    [DETAIL]: (n) => (n === 1 ? ok({ as_of: AS_OF, session: activeSession() }) : fail(404, 'not_found')),
  });
  const { root } = mount(api);
  await waitFor(() => enabled(root));
  root.querySelector('#session-refresh').click();
  await waitFor(() => noticeOf(root) === t.notFound);
  check('DASH-007: gone during refresh shows not-found', noticeOf(root) === t.notFound);
  check('DASH-007: gone during refresh keeps terminate off', !enabled(root));
  check('DASH-007: gone during refresh sends no terminate', api.count(TERMINATE) === 0);
  check('DASH-007: list link kept after refresh', backOf(root)?.getAttribute('href') === '/');
  checkLayout('DASH-007');
}

// DASH-014: a direct entry without a management session sends no operational request and goes to sign-in.
async function caseDash014() {
  const api = fakeApi({ 'GET /api/v1/session': () => fail(401, 'unauthenticated') });
  const { root, navigations } = mount(api);
  await waitFor(() => navigations.length > 0);
  check('DASH-014: sent to sign-in', JSON.stringify(navigations) === JSON.stringify(['/login/']));
  check('DASH-014: only the session check was sent', api.calls.every((c) => c.key === 'GET /api/v1/session'));
  check('DASH-014: no operational data shown', root.querySelector('.summary-grid') === null);
  check('DASH-014: no terminate action', !enabled(root));
  checkLayout('DASH-014');
}

// DASH-011: a target that disappears after confirm shows not-found and sends no further terminate.
async function caseDash011() {
  const api = fakeApi({ [TERMINATE]: () => fail(404, 'not_found') });
  const { root } = mount(api);
  await waitFor(() => enabled(root));
  terminateOf(root).click();
  confirmOf(root).click();
  await waitFor(() => noticeOf(root) === t.notFound);
  check('DASH-011: gone after confirm shows not-found', noticeOf(root) === t.notFound);
  check('DASH-011: terminate sent once', api.count(TERMINATE) === 1);
  check('DASH-011: no re-read after not-found', api.count(DETAIL) === 1);
  checkLayout('DASH-011');
}

// DASH-012: a failed terminate is not treated as complete. The state is re-read and the action is available again.
async function caseDash012() {
  const api = fakeApi({ [TERMINATE]: () => fail(500, 'terminate_failed') });
  const { root } = mount(api);
  await waitFor(() => enabled(root));
  terminateOf(root).click();
  confirmOf(root).click();
  await waitFor(() => api.count(DETAIL) === 2 && enabled(root));
  check('DASH-012: failure shows the failed notice', noticeOf(root) === t.terminateFailed);
  check('DASH-012: terminate sent once', api.count(TERMINATE) === 1);
  check('DASH-012: state re-read after failure', api.count(DETAIL) === 2);
  check('DASH-012: session still shown', statValue(root, t.stateLabel) === 'active');
  check('DASH-012: action available again', enabled(root));
  checkLayout('DASH-012');
}

// DASH-013: an unknown terminate result is not resent. A state read follows.
async function caseDash013() {
  const api = fakeApi({
    [TERMINATE]: () => {
      throw new TypeError('connection lost');
    },
  });
  const { root } = mount(api);
  await waitFor(() => enabled(root));
  terminateOf(root).click();
  confirmOf(root).click();
  await waitFor(() => api.count(DETAIL) === 2 && enabled(root));
  await wait(200);
  check('DASH-013: unknown shows the unknown notice', noticeOf(root) === t.terminateUnknown);
  check('DASH-013: terminate not resent', api.count(TERMINATE) === 1);
  check('DASH-013: state read after unknown', api.count(DETAIL) === 2);
  checkLayout('DASH-013');
}

// DASH-026: an invalid session id sends no request and shows the invalid-id notice.
async function caseDash026() {
  const api = fakeApi();
  const { root } = mount(api, 'bad id/../x');
  await wait(200);
  check('DASH-026: no request sent', api.calls.length === 0);
  check('DASH-026: invalid-id notice shown', noticeOf(root) === t.invalidSessionId);
  check('DASH-026: title falls back to the screen name', titleOf(root) === t.sessionScreen);
  check('DASH-026: no terminate action', terminateOf(root) === null);
  check('DASH-026: back link present', backOf(root) !== null);
  checkLayout('DASH-026');
}

const CASES = {
  'dash-006': caseDash006,
  'dash-007': caseDash007,
  'dash-009': caseDash009,
  'dash-010': caseDash010,
  'dash-011': caseDash011,
  'dash-012': caseDash012,
  'dash-013': caseDash013,
  'dash-014': caseDash014,
  'dash-026': caseDash026,
};

const caseName = new URLSearchParams(location.search).get('case');
const run = CASES[caseName];
if (!run) {
  failures.push(`unknown case: ${caseName}`);
} else {
  try {
    await run();
  } catch (error) {
    failures.push(`${caseName} threw: ${error?.message ?? error}`);
  }
}
check('no CSP violations', cspViolations === 0);
result.dataset.result = failures.length === 0 ? 'pass' : 'fail';
result.dataset.failures = failures.join('\n');
result.dataset.cspViolations = String(cspViolations);
