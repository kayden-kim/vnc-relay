// Scenario checks for the session detail screen (dashboard.md SCR-002: DASH-006, 007, 009 to 014, 016, 019, 022, 024, 025, 026).
// One page load runs one case, ?case=dash-0xx&lang=ko|en, against a fake management API.
// The result goes to <main id="result">. Wording comes from app/copy.js, which holds the spec §9.3 CPY table.
import { mountSession } from '../app/session.js';
import { BRAND_NAME, COPY } from '../app/copy.js';
import { formatAddress, formatMode, formatTransport } from '../app/format.js';
import { STORAGE_KEY } from '../app/language.js';
import { checkHelpLanguageChange } from './help-link-checks.js';

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
const TIME_ONLY = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} \S+$/;

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const delayed = (ms, response) => new Promise((resolve) => setTimeout(() => resolve(response), ms));
// Waits until `ready()` holds or `ms` passes; the caller's check then reports the outcome either way.
// Each step is one 20 ms timer, so the bound is page time. A wall-clock bound expired early when the
// browser run was slowed by load, before the page's own poll timer had fired.
async function waitFor(ready, ms = 5000) {
  for (let steps = Math.ceil(ms / 20); steps > 0 && !ready(); steps -= 1) await wait(20);
}
const check = (label, passed) => {
  if (!passed) failures.push(label);
};
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

// Fake management API. Routes are keyed "METHOD path" and receive their call number for that key.
const json = (status, body) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });
const ok = (body) => json(200, body);
const fail = (status, code) => json(status, { error: { code } });

// The session check answers with the dashboard settings; the poll interval comes from poll_secs.
function session(pollSecs = 60) {
  return ok({
    csrf_token: 'csrf-token-1',
    dashboard: { poll_secs: pollSecs, query_timeout_secs: 5, terminate_timeout_secs: 10 },
  });
}

const activeSession = (overrides = {}) => ({
  session_id: SESSION_ID,
  state: 'active',
  mode: 'mode1',
  viewer_transport: 'wss',
  target_transport: 'tcp',
  viewer_forwarded_ip: null,
  target_forwarded_ip: null,
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
  target_transport: 'tcp',
  started_at: STARTED,
  ended_at: ENDED,
  bytes_viewer_to_target: 1536,
  bytes_target_to_viewer: 1048575,
  close_cause: 'peer_eof',
  ...overrides,
});

const DEFAULT_ROUTES = {
  'GET /api/v1/session': () => session(),
  'DELETE /api/v1/session': () => new Response(null, { status: 204 }),
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
    calls.push({
      key,
      at: performance.now(),
      poll: init.headers?.['X-Relay-Poll'] === '1',
      csrf: init.headers?.['X-CSRF-Token'] ?? null,
      raw: JSON.stringify(init),
    });
    const route = routes[key];
    return Promise.resolve().then(() => (route ? route(n) : fail(404, 'not_found')));
  };
  return { request, calls, count: (key) => counts.get(key) ?? 0 };
}

// Browser storage stand-in that keeps the saved language in `items`.
function storageOf(items) {
  return {
    getItem: (key) => items.get(key) ?? null,
    setItem: (key, value) => items.set(key, String(value)),
  };
}

const intervalOf = (calls) => (calls.length >= 2 ? calls[1].at - calls[0].at : NaN);

function mount(
  api,
  { id = SESSION_ID, storage = { getItem: () => null, setItem() {} }, preferred = [lang === 'en' ? 'en-US' : 'ko-KR'] } = {},
) {
  const root = document.createElement('div');
  host.replaceChildren(root);
  const navigations = [];
  const handle = mountSession(root, id, {
    request: api.request,
    storage,
    preferredLanguages: preferred,
    navigate: (url) => navigations.push(url),
  });
  return { root, navigations, api, handle };
}

const titleOf = (root) => root.querySelector('.page-title')?.textContent.trim() ?? null;
const noticeOf = (root) => root.querySelector('.session-notice .alert-title')?.textContent.trim() ?? null;
const terminateOf = (root) => root.querySelector('#session-terminate');
const refreshOf = (root) => root.querySelector('#session-refresh');
const noticeActionOf = (root) => root.querySelector('.session-notice .alert > .button');
const labelOf = (node) => node?.querySelector('.button-label')?.textContent.trim() ?? null;
const enabled = (root) => terminateOf(root) !== null && !terminateOf(root).disabled;
const dialogOf = (root) => root.querySelector('dialog.dialog');
const confirmOf = (root) => dialogOf(root)?.querySelector('.dialog-actions button:last-child');
const cancelOf = (root) => dialogOf(root)?.querySelector('.dialog-actions button:first-child');
const backOf = (root) => [...root.querySelectorAll('a')].find((link) => link.textContent.trim() === t.back) ?? null;
const statValue = (root, label) => {
  const stat = [...root.querySelectorAll('.stat')].find((node) => node.textContent.includes(label));
  return stat?.querySelector('.stat-value')?.textContent.trim() ?? null;
};

const factValue = (root, label) => {
  const row = [...root.querySelectorAll('.fact')].find(
    (node) => node.querySelector('.fact-label')?.textContent.trim() === label,
  );
  return row?.querySelector('.fact-value')?.textContent.trim() ?? null;
};

// Before the first success every fact and byte value is the info loading badge of the card head (DESIGN 상태 색, Q143).
function checkLoadingMarks(label, root) {
  const isInfoBadge = (node) => node?.dataset.tone === 'info' && node.querySelector('svg') !== null
    && node.textContent.trim() === t.loading;
  const facts = [...root.querySelectorAll('.fact')];
  check(`${label}: six fact labels shown while loading`, facts.length === 6);
  check(`${label}: every fact value is the loading badge`,
    facts.every((row) => isInfoBadge(row.querySelector('.fact-value > .badge'))));
  const stats = [...root.querySelectorAll('.stat')];
  check(`${label}: both byte directions shown while loading`, stats.length === 2
    && [t.directionViewerToTarget, t.directionTargetToViewer].every((name) => stats.some((s) => s.textContent.includes(name))));
  check(`${label}: every byte value is the loading badge`,
    stats.every((stat) => isInfoBadge(stat.querySelector('.stat-value > .badge'))));
  const marks = [...root.querySelectorAll('.badge')].filter((node) => node.textContent.trim() === t.loading);
  check(`${label}: every loading mark is the same info badge`, marks.length === 9 && marks.every(isInfoBadge));
}

const metaOf = (root) => root.querySelector('.page-meta')?.textContent.trim() ?? '';

// A template such as "Last successful update: {time}" matches when the placeholder holds a local time.
function isFilledTemplate(text, template) {
  const [before, after] = template.split(/\{[^}]+\}/);
  if (!text.startsWith(before) || !text.endsWith(after)) return false;
  return TIME_ONLY.test(text.slice(before.length, text.length - after.length));
}

function checkLayout(label) {
  const page = document.documentElement;
  check(`${label}: no horizontal scroll`, page.scrollWidth <= page.clientWidth);
  for (const navbar of host.querySelectorAll('.navbar')) {
    const links = [...navbar.querySelectorAll('.navbar-nav .navbar-item')];
    const copy = COPY[document.documentElement.lang];
    check(`${label}: three navigation links remain available`, links.length === 3
      && links[0].textContent === copy.operations && links[1].textContent === copy.logs && links[2].textContent === copy.settings
      && links.every((link) => getComputedStyle(link).display !== 'none' && link.getBoundingClientRect().height > 0));
    check(`${label}: detail keeps operations navigation active`, links[0]?.getAttribute('aria-current') === 'page'
      && !links[1]?.hasAttribute('aria-current'));
    check(`${label}: settings link uses the settings route`, links[2]?.getAttribute('href') === '/#/settings');
    if (window.innerWidth < 1024) {
      check(`${label}: mobile navigation occupies a second row`,
        navbar.querySelector('.navbar-nav').getBoundingClientRect().top >= navbar.querySelector('.navbar-tools').getBoundingClientRect().bottom);
    }
  }
}

// DASH-006: the detail shows the session facts and an enabled terminate action for an active session.
async function caseDash006() {
  const api = fakeApi();
  const { root } = mount(api);
  await waitFor(() => enabled(root));
  check('DASH-006: title shows the session id', titleOf(root) === t.sessionTitle.replace('{session_id}', SESSION_ID));
  check(
    'DASH-006: document title is the brand and the session title',
    document.title === `${BRAND_NAME} ${t.sessionTitle.replace('{session_id}', SESSION_ID)}`,
  );
  check(
    'DASH-006: session id sits in its own data span',
    root.querySelector('.page-title .session-id')?.textContent === SESSION_ID,
  );
  check('DASH-006: header refresh uses CPY-008', labelOf(refreshOf(root)) === t.refresh);
  check('DASH-006: six fact rows, no state row', root.querySelectorAll('.fact').length === 6);
  check('DASH-006: mode label', factValue(root, t.modeLabel) === formatMode('mode1'));
  check('DASH-006: transport label', factValue(root, t.transportLabel) === formatTransport('wss'));
  check('DASH-006: target transport label', factValue(root, t.targetTransportLabel) === formatTransport('tcp'));
  check('DASH-006: started time', TIME_IN.test(factValue(root, t.startedLabel) ?? ''));
  check(
    'DASH-006: viewer address',
    factValue(root, t.viewerAddressLabel) === formatAddress({ ip: '192.0.2.10', port: 51000 }),
  );
  check(
    'DASH-006: target address',
    factValue(root, t.targetAddressLabel) === formatAddress({ ip: '198.51.100.5', port: 5900 }),
  );
  check('DASH-006: viewer to target bytes in KiB', statValue(root, t.directionViewerToTarget) === '1.5 KiB');
  check('DASH-006: target to viewer bytes in MiB', statValue(root, t.directionTargetToViewer) === '1.0 MiB');
  check(
    'DASH-006: terminate sits in the info card footer',
    terminateOf(root)?.parentElement?.classList.contains('card-foot') === true,
  );
  check('DASH-006: view logs link in footer', root.querySelector('#session-view-logs')?.getAttribute('href') === `/#/logs?session_id=${encodeURIComponent(SESSION_ID)}`);
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
  // DASH-015: Tab and Shift+Tab stay between the two dialog actions. The keydown is synthetic, so this checks the trap handler only; real key presses were checked by hand in a browser, not in this harness.
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
  // A mouse click in WebKit (Safari) does not focus the button, so the dialog opens with no focused opener.
  // Cancel must still return focus to the terminate button (spec §6), not leave it on the page body.
  document.activeElement?.blur();
  terminateOf(root).click();
  await wait(50);
  cancelOf(root).click();
  await wait(100);
  check('DASH-009: cancel without a focused opener closes the dialog', dialogOf(root)?.open === false);
  check('DASH-009: focus returns to terminate without a focused opener', document.activeElement === terminateOf(root));
  check('DASH-009: cancel without a focused opener sends no request', api.count(TERMINATE) === 0);
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
  check('DASH-010: terminate stays disabled after completion', terminateOf(root)?.disabled === true);
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
  check(
    'DASH-007: gone before read hides the facts and bytes',
    loaded.root.querySelector('.fact-list') === null && loaded.root.querySelector('.stat') === null,
  );
  check('DASH-007: gone before read keeps terminate disabled', terminateOf(loaded.root)?.disabled === true);
  check('DASH-007: list link kept', backOf(loaded.root)?.getAttribute('href') === '/');
  check(
    'DASH-007: gone notice keeps the list action',
    labelOf(noticeActionOf(loaded.root)) === t.back && noticeActionOf(loaded.root)?.getAttribute('href') === '/',
  );
  check('DASH-007: focus moves to the notice list action', document.activeElement === noticeActionOf(loaded.root));
  loaded.handle.dispose();

  const api = fakeApi({
    [DETAIL]: (n) => (n === 1 ? ok({ as_of: AS_OF, session: activeSession() }) : fail(404, 'not_found')),
  });
  const { root } = mount(api);
  await waitFor(() => enabled(root));
  root.querySelector('#session-refresh').click();
  await waitFor(() => noticeOf(root) === t.notFound);
  check('DASH-007: gone during refresh shows not-found', noticeOf(root) === t.notFound);
  check('DASH-007: gone during refresh keeps terminate disabled', terminateOf(root)?.disabled === true);
  check('DASH-007: gone during refresh sends no terminate', api.count(TERMINATE) === 0);
  check('DASH-007: list link kept after refresh', backOf(root)?.getAttribute('href') === '/');
  check('DASH-007: focus moves to the notice list action after refresh', document.activeElement === noticeActionOf(root));
  checkLayout('DASH-007');
}

// DASH-014: a direct entry without a management session sends no operational request and goes to sign-in.
async function caseDash014() {
  const api = fakeApi({ 'GET /api/v1/session': () => fail(401, 'unauthenticated') });
  const { root, navigations } = mount(api);
  await waitFor(() => navigations.length > 0);
  check('DASH-014: sent to sign-in', JSON.stringify(navigations) === JSON.stringify(['/login/']));
  check('DASH-014: only the session check was sent', api.calls.every((c) => c.key === 'GET /api/v1/session'));
  check('DASH-014: no operational data shown', root.querySelector('.fact-list') === null);
  check('DASH-014: no bytes shown', root.querySelector('.stat') === null);
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
  check('DASH-011: terminate disabled after gone', terminateOf(root)?.disabled === true);
  check('DASH-011: focus moves to the notice list action', document.activeElement === noticeActionOf(root));
  check('DASH-011: terminate sent once', api.count(TERMINATE) === 1);
  check('DASH-011: no re-read after not-found', api.count(DETAIL) === 1);
  checkLayout('DASH-011');
}

// DASH-012: a failed terminate is not treated as complete. The state is re-read and the action is available again.
// The notice's re-check action runs the same read and keeps focus. A rejected terminate (403) is checked after it.
async function caseDash012() {
  const api = fakeApi({ [TERMINATE]: () => fail(500, 'terminate_failed') });
  const { root, handle } = mount(api);
  await waitFor(() => enabled(root));
  terminateOf(root).click();
  confirmOf(root).click();
  await waitFor(() => api.count(DETAIL) === 2 && enabled(root));
  check('DASH-012: failure shows the failed notice', noticeOf(root) === t.terminateFailed);
  check('DASH-012: terminate sent once', api.count(TERMINATE) === 1);
  check('DASH-012: state re-read after failure', api.count(DETAIL) === 2);
  check('DASH-012: session still shown', factValue(root, t.modeLabel) === formatMode('mode1'));
  check('DASH-012: action available again', enabled(root));
  check('DASH-012: failed notice offers the re-check action', labelOf(noticeActionOf(root)) === t.recheck);
  const action = noticeActionOf(root);
  action.focus();
  action.click();
  await waitFor(() => api.count(DETAIL) === 3 && enabled(root));
  check('DASH-012: re-check action reads the state', api.count(DETAIL) === 3);
  check('DASH-012: focus stays on the re-check action', document.activeElement === action);
  check('DASH-012: re-check sends no terminate', api.count(TERMINATE) === 1);
  checkLayout('DASH-012');
  handle.dispose();
  await rejectedNotice();
}

// DASH-012 (rejected): a 403 terminate is not complete either. The notice names the rejection and offers the re-check action.
async function rejectedNotice() {
  const api = fakeApi({ [TERMINATE]: () => fail(403, 'forbidden') });
  const { root } = mount(api);
  await waitFor(() => enabled(root));
  terminateOf(root).click();
  confirmOf(root).click();
  await waitFor(() => api.count(DETAIL) === 2 && enabled(root));
  check('DASH-012: rejected shows the rejected notice', noticeOf(root) === t.terminateRejected);
  check('DASH-012: rejected offers the re-check action', labelOf(noticeActionOf(root)) === t.recheck);
  check('DASH-012: rejected sends terminate once', api.count(TERMINATE) === 1);
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
  check('DASH-013: unknown offers the re-check action', labelOf(noticeActionOf(root)) === t.recheck);
  check('DASH-013: terminate not resent', api.count(TERMINATE) === 1);
  check('DASH-013: state read after unknown', api.count(DETAIL) === 2);
  checkLayout('DASH-013');
}

// DASH-026: an invalid session id sends no request and shows the invalid-id notice.
async function caseDash026() {
  const api = fakeApi();
  const { root } = mount(api, { id: 'bad id/../x' });
  await wait(200);
  check('DASH-026: no request sent', api.calls.length === 0);
  check('DASH-026: invalid-id notice shown', noticeOf(root) === t.invalidSessionId);
  check('DASH-026: title falls back to the screen name', titleOf(root) === t.sessionScreen);
  check('DASH-026: no terminate action', terminateOf(root) === null);
  check('DASH-026: back link present', backOf(root) !== null);
  checkLayout('DASH-026');
}

// DASH-016: a state change appears after the next poll, with the last success time; the screen offers no remote-screen or settings actions.
async function caseDash016() {
  const api = fakeApi({
    'GET /api/v1/session': () => session(1),
    [DETAIL]: (n) =>
      ok({
        as_of: AS_OF,
        session: activeSession(n === 1 ? {} : { bytes_viewer_to_target: 4096, bytes_target_to_viewer: 2097152 }),
      }),
  });
  const { root, handle } = mount(api);
  await waitFor(() => statValue(root, t.directionViewerToTarget) === '1.5 KiB');
  check('DASH-016: bytes before the change', statValue(root, t.directionViewerToTarget) === '1.5 KiB');
  await waitFor(() => statValue(root, t.directionViewerToTarget) === '4.0 KiB');
  check('DASH-016: changed viewer to target bytes shown after the next poll', statValue(root, t.directionViewerToTarget) === '4.0 KiB');
  check('DASH-016: changed target to viewer bytes shown', statValue(root, t.directionTargetToViewer) === '2.0 MiB');
  check('DASH-016: the change arrived through a poll', api.calls.filter((c) => c.key === DETAIL).at(-1)?.poll === true);
  check('DASH-016: last success time shown', isFilledTemplate(metaOf(root), t.lastSuccess));
  const actionLabels = [...root.querySelectorAll('button')]
    .filter((button) => !button.closest('dialog'))
    .map((button) => labelOf(button))
    .sort();
  check('DASH-016: only refresh, terminate and sign-out actions', same(actionLabels, [t.refresh, t.terminate, t.signOut].sort()));
  check('DASH-016: no input fields for settings or remote screen', root.querySelectorAll('input, textarea, form').length === 0);
  handle.dispose();

  // The session is gone on a poll: the not-found notice replaces the detail and terminate stays off.
  const gone = mount(
    fakeApi({
      'GET /api/v1/session': () => session(1),
      [DETAIL]: (n) => (n === 1 ? ok({ as_of: AS_OF, session: activeSession() }) : fail(404, 'not_found')),
    }),
  );
  await waitFor(() => noticeOf(gone.root) === t.notFound);
  check('DASH-016: gone on a poll shows not-found', noticeOf(gone.root) === t.notFound);
  check('DASH-016: gone on a poll keeps terminate disabled', terminateOf(gone.root)?.disabled === true);
  check('DASH-016: gone on a poll sends no terminate', gone.api.count(TERMINATE) === 0);
  checkLayout('DASH-016');
}

// DASH-019: sign-out success removes the data, goes to sign-in and reads nothing more. A failed, lost, or refused response goes to the unconfirmed notice. Sign-out never terminates the relay session.
async function caseDash019() {
  const success = mount(fakeApi({ 'GET /api/v1/session': () => session(1) }));
  await waitFor(() => enabled(success.root));
  const logout = success.root.querySelector('#session-logout');
  logout.click();
  logout.click();
  await wait(300);
  const deletes = success.api.calls.filter((c) => c.key === 'DELETE /api/v1/session');
  check('DASH-019 success: one sign-out request', deletes.length === 1);
  check('DASH-019 success: request carries the CSRF token', deletes[0]?.csrf === 'csrf-token-1');
  check('DASH-019 success: sent to the sign-in screen', same(success.navigations, ['/login/']));
  check('DASH-019 success: data removed', success.root.childElementCount === 0);
  const seen = success.api.calls.length;
  await wait(1200);
  check('DASH-019 success: no polling after sign-out', success.api.calls.length === seen);
  check('DASH-019 success: relay session not terminated', success.api.count(TERMINATE) === 0);

  const lost = () => {
    throw new TypeError('network down');
  };
  const outcomes = [
    ['failed response', () => fail(500, 'internal_error'), '/login/?notice=logout-unconfirmed'],
    ['lost response', lost, '/login/?notice=logout-unconfirmed'],
    ['refused (403)', () => fail(403, 'csrf_invalid'), '/login/?notice=logout-unconfirmed'],
    ['expired session (401)', () => fail(401, 'unauthenticated'), '/login/'],
  ];
  for (const [name, route, target] of outcomes) {
    const { root, navigations, api } = mount(fakeApi({ 'DELETE /api/v1/session': route }));
    await waitFor(() => enabled(root));
    root.querySelector('#session-logout').click();
    await wait(300);
    check(`DASH-019 ${name}: sent to ${target}`, same(navigations, [target]));
    check(`DASH-019 ${name}: relay session not terminated`, api.count(TERMINATE) === 0);
  }
  checkLayout('DASH-019');
}

// DASH-022: the poll interval is the configured P (3 s here) or 2 s when P is not set. A read never starts while the previous one is in flight.
async function caseDash022() {
  const custom = mount(fakeApi({ 'GET /api/v1/session': () => session(3) }));
  await waitFor(() => custom.api.count(DETAIL) >= 2, 6000);
  const customCalls = custom.api.calls.filter((c) => c.key === DETAIL);
  check('DASH-022 P=3: one poll after the first load', customCalls.length === 2 && customCalls[1].poll === true);
  check('DASH-022 P=3: poll interval is 3 s', intervalOf(customCalls) >= 2950 && intervalOf(customCalls) < 3750);
  custom.handle.dispose();

  const dflt = mount(fakeApi({ 'GET /api/v1/session': () => session(2) }));
  await waitFor(() => dflt.api.count(DETAIL) >= 2, 6000);
  const dfltCalls = dflt.api.calls.filter((c) => c.key === DETAIL);
  check('DASH-022 P=2: one poll after the first load', dfltCalls.length === 2 && dfltCalls[1].poll === true);
  check('DASH-022 P=2: poll interval is 2 s', intervalOf(dfltCalls) >= 1950 && intervalOf(dfltCalls) < 2750);
  dflt.handle.dispose();

  // Each read takes 1.5 s, longer than P=1 s. A poll that started during a read would overlap it.
  let inFlight = 0;
  let maxInFlight = 0;
  const slowRead = () => {
    inFlight += 1;
    maxInFlight = Math.max(maxInFlight, inFlight);
    return new Promise((resolve) =>
      setTimeout(() => {
        inFlight -= 1;
        resolve(ok({ as_of: AS_OF, session: activeSession() }));
      }, 1500),
    );
  };
  const slow = mount(fakeApi({ 'GET /api/v1/session': () => session(1), [DETAIL]: slowRead }));
  await waitFor(() => inFlight === 1 && slow.api.count(DETAIL) === 1);
  checkLoadingMarks('DASH-022 slow first read', slow.root);
  await waitFor(() => slow.api.count(DETAIL) >= 2, 6000);
  await waitFor(() => inFlight === 0 && slow.api.count(DETAIL) >= 2, 3000);
  const slowCalls = slow.api.calls.filter((c) => c.key === DETAIL);
  check('DASH-022 slow read: no read starts while one is in flight', maxInFlight === 1);
  check('DASH-022 slow read: next read starts after the previous one ends', intervalOf(slowCalls) >= 2400);
  slow.handle.dispose();
  checkLayout('DASH-022');
}

// DASH-024: the first screen follows the browser language until the user chooses. Time and byte formats stay the same in both languages.
async function caseDash024() {
  const cases = [
    [['ko-KR'], 'ko'],
    [['en-US'], 'en'],
    [['ja-JP'], 'en'],
    [['ja-JP', 'ko'], 'ko'],
  ];
  for (const [preferred, expected] of cases) {
    const items = new Map();
    const copy = COPY[expected];
    const { root, handle } = mount(fakeApi(), { storage: storageOf(items), preferred });
    await waitFor(() => enabled(root));
    const label = `DASH-024 ${preferred.join(',')}`;
    check(`${label}: document language`, document.documentElement.lang === expected);
    check(`${label}: language select`, root.querySelector('#session-language')?.value === expected);
    check(`${label}: title in that language`, titleOf(root) === copy.sessionTitle.replace('{session_id}', SESSION_ID));
    check(`${label}: refresh label in that language`, labelOf(refreshOf(root)) === copy.refresh);
    check(`${label}: started time shown`, TIME_IN.test(factValue(root, copy.startedLabel) ?? ''));
    check(
      `${label}: byte format is the same`,
      statValue(root, copy.directionViewerToTarget) === '1.5 KiB' && statValue(root, copy.directionTargetToViewer) === '1.0 MiB',
    );
    check(`${label}: nothing saved before a choice`, items.size === 0);
    handle.dispose();
  }
  checkLayout('DASH-024');
}

// DASH-025: a chosen language applies at once, survives reload and sign-out, and beats the browser preference. Data and unknown values are not translated, and no request carries the language.
async function caseDash025() {
  const items = new Map();
  const storage = storageOf(items);
  const api = fakeApi();
  const first = mount(api, { storage, preferred: ['ko-KR'] });
  await waitFor(() => enabled(first.root));
  check('DASH-025 start: browser preference gives Korean', document.documentElement.lang === 'ko');
  const select = first.root.querySelector('#session-language');
  const detailReads = api.count(DETAIL);
  select.value = 'en';
  select.dispatchEvent(new Event('change'));
  await wait(300);
  check('DASH-025 change: document language is English', document.documentElement.lang === 'en');
  check(
    'DASH-025 change: copy is English at once',
    titleOf(first.root) === COPY.en.sessionTitle.replace('{session_id}', SESSION_ID),
  );
  check('DASH-025 change: no state read for the change', api.count(DETAIL) === detailReads);
  check('DASH-025 change: choice saved', items.get(STORAGE_KEY) === 'en');
  first.handle.dispose();

  const again = mount(api, { storage, preferred: ['ko-KR'] });
  await waitFor(() => enabled(again.root));
  check('DASH-025 reload: saved choice beats browser preference', document.documentElement.lang === 'en');
  check('DASH-025 reload: select keeps English', again.root.querySelector('#session-language')?.value === 'en');
  check(
    'DASH-025 reload: mode and transport are not translated',
    factValue(again.root, COPY.en.modeLabel) === formatMode('mode1') &&
      factValue(again.root, COPY.en.transportLabel) === formatTransport('wss'),
  );
  check('DASH-025 reload: session id is not translated', titleOf(again.root).includes(SESSION_ID));
  again.root.querySelector('#session-logout').click();
  await wait(300);
  check('DASH-025 sign-out: sent to sign-in', same(again.navigations, ['/login/']));
  check('DASH-025 sign-out: choice still saved', items.get(STORAGE_KEY) === 'en');
  check('DASH-025: no request carries a language value', api.calls.every((c) => !/lang/i.test(`${c.key} ${c.raw}`)));

  for (const language of ['ko', 'en']) {
    const unknown = fakeApi({
      [DETAIL]: () => ok({ as_of: AS_OF, session: activeSession({ mode: 'mode9', viewer_transport: 'quic' }) }),
    });
    const { root, handle } = mount(unknown, {
      storage: storageOf(new Map([[STORAGE_KEY, language]])),
      preferred: ['ko-KR'],
    });
    await waitFor(() => enabled(root));
    check(`DASH-025 unknown ${language}: mode shown as received`, factValue(root, COPY[language].modeLabel) === 'mode9');
    check(`DASH-025 unknown ${language}: transport shown as received`, factValue(root, COPY[language].transportLabel) === 'quic');
    handle.dispose();
  }
  checkLayout('DASH-025');
}

// Capture-only cases: each one sets up a single screen state for the screenshot run and then ends. They make no DASH claim.
const alertTitlesOf = (root) => [...root.querySelectorAll('.alert-title')].map((node) => node.textContent.trim());

async function shotDialog() {
  const { root } = mount(fakeApi());
  await waitFor(() => enabled(root));
  terminateOf(root).click();
  await waitFor(() => dialogOf(root)?.open === true);
}

async function shotRefreshing() {
  const api = fakeApi({ [DETAIL]: (n) => (n === 1 ? ok({ as_of: AS_OF, session: activeSession() }) : new Promise(() => {})) });
  const { root } = mount(api);
  await waitFor(() => enabled(root));
  root.querySelector('#session-refresh').click();
  await waitFor(() => root.textContent.includes(t.refreshing));
}

async function shotQueryFailed() {
  const { root } = mount(fakeApi({ [DETAIL]: () => fail(500, 'internal_error') }));
  await waitFor(() => alertTitlesOf(root).includes(t.failed));
}

async function shotStaleFailed() {
  const api = fakeApi({ [DETAIL]: (n) => (n === 1 ? ok({ as_of: AS_OF, session: activeSession() }) : fail(500, 'internal_error')) });
  const { root } = mount(api);
  await waitFor(() => enabled(root));
  root.querySelector('#session-refresh').click();
  await waitFor(() => alertTitlesOf(root).includes(t.failed));
  check('shot-stale-failed: terminate disabled while the query has failed', !enabled(root));
}

async function shotTerminating() {
  const { root } = mount(fakeApi({ [TERMINATE]: () => new Promise(() => {}) }));
  await waitFor(() => enabled(root));
  terminateOf(root).click();
  confirmOf(root).click();
  await waitFor(() => root.textContent.includes(t.terminating));
}

async function shotOutcome(route, message) {
  const api = fakeApi({ [TERMINATE]: route });
  const { root } = mount(api);
  await waitFor(() => enabled(root));
  terminateOf(root).click();
  confirmOf(root).click();
  await waitFor(() => noticeOf(root) === message && enabled(root));
}

const shotFailed = () => shotOutcome(() => fail(500, 'terminate_failed'), t.terminateFailed);
const shotRejected = () => shotOutcome(() => fail(403, 'forbidden'), t.terminateRejected);
const shotUnknown = () =>
  shotOutcome(() => {
    throw new TypeError('connection lost');
  }, t.terminateUnknown);

async function shotGone() {
  const { root } = mount(fakeApi({ [DETAIL]: () => fail(404, 'not_found') }));
  await waitFor(() => noticeOf(root) === t.notFound);
}
async function caseDash062() {
  const sessionWithForwarded = {
    ...activeSession(),
    mode: 'mode2',
    viewer_transport: 'wss',
    target_transport: 'ws',
    viewer_forwarded_ip: '203.0.113.195',
    target_forwarded_ip: '198.51.100.22',
  };
  const api = fakeApi({
    [DETAIL]: () => ok({ as_of: AS_OF, session: sessionWithForwarded }),
  });
  const { root } = mount(api);
  await waitFor(() => enabled(root));
  check('DASH-062: eight fact rows when both forwarded IPs present', root.querySelectorAll('.fact').length === 8);
  check('DASH-062: target transport is WS', factValue(root, t.targetTransportLabel) === 'WS');
  check('DASH-062: viewer forwarded ip', factValue(root, t.viewerForwardedAddressLabel) === '203.0.113.195');
  check('DASH-062: target forwarded ip', factValue(root, t.targetForwardedAddressLabel) === '198.51.100.22');
  check('DASH-062: address note is CPY-188', root.querySelector('.card-note')?.textContent === t.addressNote);
  checkLayout('DASH-062', root);
}

async function caseDash064() {
  const m = mount(fakeApi());
  await waitFor(() => enabled(m.root));
  checkHelpLanguageChange(m.root, lang, check);
  check('DASH-064: no help API requests', m.api.calls.every((call) => call.key.startsWith('GET /api/v1/')));
  check('DASH-064: current screen remains mounted', m.navigations.length === 0);
  m.handle.dispose();
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
  'dash-016': caseDash016,
  'dash-019': caseDash019,
  'dash-022': caseDash022,
  'dash-024': caseDash024,
  'dash-025': caseDash025,
  'dash-026': caseDash026,
  'dash-062': caseDash062,
  'dash-064': caseDash064,
  'shot-loading': async () => { mount(fakeApi({ [DETAIL]: () => new Promise(() => {}) })); },
  'shot-normal': async () => {
    const { root } = mount(fakeApi({ [DETAIL]: () => ok({ as_of: AS_OF, session: activeSession({ mode: 'mode2', target_transport: 'ws', target_forwarded_ip: '198.51.100.22' }) }) }));
    await waitFor(() => enabled(root));
  },
  'shot-forwarded': caseDash062,
  'shot-bad-id': async () => { mount(fakeApi(), { id: 'bad/id' }); },
  'shot-denied': async () => {
    const m = mount(fakeApi({ [DETAIL]: () => fail(401, 'unauthenticated') }));
    await waitFor(() => m.navigations.length > 0);
    const { mountLogin } = await import('../login/login.js');
    mountLogin(m.root, { preferredLanguages: [lang], storage: { getItem: () => lang, setItem() {} }, search: '' });
  },
  'shot-dialog': shotDialog,
  'shot-refreshing': shotRefreshing,
  'shot-query-failed': shotQueryFailed,
  'shot-stale-failed': shotStaleFailed,
  'shot-terminating': shotTerminating,
  'shot-failed': shotFailed,
  'shot-unknown': shotUnknown,
  'shot-rejected': shotRejected,
  'shot-gone': shotGone,
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
