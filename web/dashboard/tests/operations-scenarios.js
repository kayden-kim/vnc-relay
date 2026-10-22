// Scenario checks for the operations overview (dashboard.md §7: DASH-001 to 005, 008, 016, 019 to 022).
// One page load runs one case, ?case=dash-0xx&lang=ko|en, against a fake management API.
// The result goes to <main id="result">.
import { mountOperations } from '../app/operations.js';
import { formatBytes } from '../app/format.js';
import { STORAGE_KEY } from '../app/language.js';

// Exact copy from dashboard.md §9.3.1 and §9.3.2. {시각} and {time} are the time placeholders.
const COPY = {
  ko: {
    operations: '운영 현황',
    refresh: '새로고침',
    signOut: '로그아웃',
    lastSuccess: '마지막 성공 갱신: {시각}',
    loading: '조회 중',
    failed: '조회 실패',
    stale: '오래된 데이터',
    staleMessage: '조회에 실패했습니다. 아래 값은 마지막 성공 시각 {시각}의 이전 데이터입니다.',
    failedMessage: '조회에 실패했습니다. 새로고침으로 다시 시도하십시오.',
    summaryTitle: '서버 요약',
    serverState: '실행 상태',
    stateReady: '준비됨',
    stateShutdownGrace: '종료 유예 중',
    uptime: '가동 시간',
    waitingCount: '대기 연결 수',
    activeCount: '활성 세션 수',
    handshaking: '초기 교환 중',
    targetConnecting: 'Mode I 대상 연결 중',
    relayTitle: '중계 연결',
    relayNote: '중계 측 연결 소켓의 현재 수와 상한입니다.',
    adminTitle: '관리 연결',
    adminNote: '관리·상태 리스너의 연결 소켓 현재 수와 상한입니다.',
    unavailable: '확인 불가',
    waitingTitle: '대기 연결',
    waitingEmpty: '대기 없음',
    activeTitle: '활성 세션',
    activeEmpty: '활성 없음',
    closedTitle: '최근 종료 목록',
    closedEmpty: '최근 종료 없음',
    causePeerEof: '정상 EOF',
    causeConnectionError: '연결 오류',
    causeAdmin: '관리자 종료',
  },
  en: {
    operations: 'Operations overview',
    refresh: 'Refresh',
    signOut: 'Sign out',
    lastSuccess: 'Last successful update: {time}',
    loading: 'Loading',
    failed: 'Failed to load',
    stale: 'Stale data',
    staleMessage: 'Loading failed. The values below are from the last successful update at {time}.',
    failedMessage: 'Loading failed. Use Refresh to try again.',
    summaryTitle: 'Server summary',
    serverState: 'Server state',
    stateReady: 'Ready',
    stateShutdownGrace: 'Shutting down (grace period)',
    uptime: 'Uptime',
    waitingCount: 'Waiting connections',
    activeCount: 'Active sessions',
    handshaking: 'Initial exchange',
    targetConnecting: 'Connecting to Mode I target',
    relayTitle: 'Relay connections',
    relayNote: 'Current count and limit of relay-side connection sockets.',
    adminTitle: 'Management connections',
    adminNote: 'Current count and limit of management and status listener sockets.',
    unavailable: 'Unavailable',
    waitingTitle: 'Waiting connections',
    waitingEmpty: 'No waiting connections',
    activeTitle: 'Active sessions',
    activeEmpty: 'No active sessions',
    closedTitle: 'Recently closed sessions',
    closedEmpty: 'No recently closed sessions',
    causePeerEof: 'Normal EOF',
    causeConnectionError: 'Connection error',
    causeAdmin: 'Terminated by administrator',
  },
};

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
const CONNECTED = '2026-10-22T01:00:00.000Z';
const MAX_BYTES = Number.MAX_SAFE_INTEGER;
const TIME_IN = /\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} \S/;
const TIME_ONLY = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} \S+$/;

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
// Waits until `ready()` holds or `ms` passes; the caller's check then reports the outcome either way.
async function waitFor(ready, ms = 5000) {
  const until = Date.now() + ms;
  while (!ready() && Date.now() < until) await wait(20);
}
// The first load is over: cards are on screen and none still reads as loading.
const settled = (root) => root.querySelectorAll('section.card').length > 0 && !root.textContent.includes(t.loading);
const check = (label, ok) => {
  if (!ok) failures.push(label);
};
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

// A template such as "Last successful update: {time}" matches when the placeholder holds a local time.
function isFilledTemplate(text, template) {
  const [before, after] = template.split(/\{[^}]+\}/);
  if (!text.startsWith(before) || !text.endsWith(after)) return false;
  return TIME_ONLY.test(text.slice(before.length, text.length - after.length));
}

function checkLayout(label) {
  const page = document.documentElement;
  check(`${label}: no horizontal scroll`, page.scrollWidth <= page.clientWidth);
}

function cardOf(root, title) {
  return (
    [...root.querySelectorAll('section.card')].find(
      (card) => card.querySelector('.card-heading')?.textContent.trim() === title,
    ) ?? null
  );
}

const badgeOf = (card) => card?.querySelector('.card-status .badge')?.textContent.trim() ?? null;

function alertOf(card) {
  const alert = card?.querySelector('.card-alert .alert');
  if (!alert) return null;
  return {
    title: alert.querySelector('.alert-title')?.textContent.trim() ?? null,
    message: alert.querySelector('.alert-message')?.textContent.trim() ?? null,
  };
}

const dataRowsOf = (card) => [...(card?.querySelectorAll('tbody tr:not(.data-table-empty)') ?? [])];
const emptyOf = (card) => card?.querySelector('.data-table-empty')?.textContent.trim() ?? null;

function statValue(root, label) {
  const stat = [...root.querySelectorAll('.stat')].find((node) => node.textContent.includes(label));
  return stat?.querySelector('.stat-value')?.textContent.trim() ?? null;
}

const gaugeOf = (root, title) => root.querySelector(`.gauge[aria-label="${title}"]`);
const gaugeValue = (gauge) => gauge?.querySelector('.gauge-value')?.textContent.trim() ?? null;
const gaugeMax = (gauge) => gauge?.querySelector('.gauge-max')?.textContent.trim() ?? '';
const metaText = (root) => root.querySelector('.page-meta')?.textContent.trim() ?? '';

// Fake management API. Routes are keyed "METHOD path" and receive their call number for that key.
const json = (status, body) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });
const ok = (body) => json(200, body);
const fail = (status) => json(status, { error: { code: status === 401 ? 'unauthenticated' : 'internal' } });

function session(pollSecs = 2) {
  return ok({
    csrf_token: 'csrf-token-1',
    dashboard: { poll_secs: pollSecs, query_timeout_secs: 5, terminate_timeout_secs: 10 },
  });
}

function summaryBody(overrides = {}) {
  return {
    as_of: AS_OF,
    server: { state: 'ready', uptime_ms: 90061000 },
    counts: { waiting: 1, active: 1, handshaking: 2, target_connecting: null },
    usage: {
      relay_connections: { current: 3, limit: 100 },
      admin_connections: { current: 1, limit: 20 },
    },
    ...overrides,
  };
}

const waitingItem = (overrides = {}) => ({
  role: 'viewer',
  relay_id: 'relay-7',
  connected_at: CONNECTED,
  waited_ms: 62000,
  ...overrides,
});

const activeItem = (overrides = {}) => ({
  session_id: 'sess_1',
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

const closedItem = (overrides = {}) => ({
  session_id: 'sess_0',
  mode: 'mode2',
  viewer_transport: 'tcp',
  started_at: STARTED,
  ended_at: ENDED,
  bytes_viewer_to_target: 0,
  bytes_target_to_viewer: 1572864,
  close_cause: 'peer_eof',
  ...overrides,
});

const DEFAULT_ROUTES = {
  'GET /api/v1/session': () => session(2),
  'GET /api/v1/summary': () => ok(summaryBody()),
  'GET /api/v1/waiting': () => ok({ as_of: AS_OF, items: [waitingItem()] }),
  'GET /api/v1/sessions': () => ok({ as_of: AS_OF, items: [activeItem()] }),
  'GET /api/v1/closed-sessions': () => ok({ as_of: AS_OF, max_entries: 100, items: [closedItem()] }),
  'DELETE /api/v1/session': () => new Response(null, { status: 204 }),
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
    return Promise.resolve().then(() => (route ? route(n) : fail(404)));
  };
  return { request, calls, count: (key) => counts.get(key) ?? 0 };
}

async function freshPage() {
  const response = await fetch('../app/index.html', { cache: 'no-store' });
  const parsed = new DOMParser().parseFromString(await response.text(), 'text/html');
  return document.importNode(parsed.getElementById('app-root'), true);
}

async function mount(
  api,
  { storage = { getItem: () => null, setItem() {} }, preferred = [lang === 'en' ? 'en-US' : 'ko-KR'] } = {},
) {
  const root = await freshPage();
  host.replaceChildren(root);
  const navigations = [];
  mountOperations(root, {
    request: api.request,
    storage,
    preferredLanguages: preferred,
    navigate: (url) => navigations.push(url),
  });
  return { root, navigations, api };
}

// DASH-001: first load shows the summary, waiting and active rows, the closed list, and usage.
async function caseDash001() {
  const bigBytes = new Intl.NumberFormat().format(8589934592) + '.0 MiB';
  const api = fakeApi({
    'GET /api/v1/sessions': () =>
      ok({
        as_of: AS_OF,
        items: [
          activeItem(),
          activeItem({
            session_id: 'sess_2',
            bytes_viewer_to_target: MAX_BYTES,
            bytes_target_to_viewer: MAX_BYTES,
          }),
        ],
      }),
  });
  const { root } = await mount(api);
  await waitFor(() => settled(root));

  check('DASH-001: page title is the operations overview', root.querySelector('.page-title')?.textContent.trim() === t.operations);
  check('DASH-001: document language follows the choice', document.documentElement.lang === lang);
  check('DASH-001: first load sends no poll header', api.calls.every((c) => !c.poll));
  check('DASH-001: last success time shown in local time', isFilledTemplate(metaText(root), t.lastSuccess));

  check('DASH-001: server state shown as ready', statValue(root, t.serverState) === t.stateReady);
  check('DASH-001: uptime shown as days and HH:MM:SS', /^1\S* 01:01:01$/.test(statValue(root, t.uptime) ?? ''));
  check(
    'DASH-001: waiting and active counts shown',
    statValue(root, t.waitingCount) === '1' && statValue(root, t.activeCount) === '1',
  );
  check('DASH-001: known handshaking count shown', statValue(root, t.handshaking) === '2');
  check(
    'DASH-001: unknown Mode I target count shown as unavailable, not 0',
    statValue(root, t.targetConnecting) === t.unavailable,
  );

  const relay = gaugeOf(root, t.relayTitle);
  check('DASH-001: relay usage current shown', gaugeValue(relay) === '3');
  check('DASH-001: relay usage limit shown', /100/.test(gaugeMax(relay)));
  const admin = gaugeOf(root, t.adminTitle);
  check('DASH-001: admin usage current shown', gaugeValue(admin) === '1');
  check('DASH-001: admin usage limit shown', /20/.test(gaugeMax(admin)));

  const waitingRows = dataRowsOf(cardOf(root, t.waitingTitle));
  check(
    'DASH-001: waiting row shows relay ID, connect time, and waiting time',
    waitingRows.length === 1
      && waitingRows[0].textContent.includes('relay-7')
      && TIME_IN.test(waitingRows[0].textContent)
      && waitingRows[0].textContent.includes('00:01:02'),
  );

  const activeRows = dataRowsOf(cardOf(root, t.activeTitle));
  check('DASH-001: two active rows', activeRows.length === 2);
  check(
    'DASH-001: per-direction bytes shown in KiB and MiB',
    activeRows[0]?.textContent.includes('1.5 KiB') && activeRows[0]?.textContent.includes('1.0 MiB'),
  );
  check(
    'DASH-001: bytes at the largest safe integer shown in full',
    (activeRows[1]?.textContent.split(bigBytes).length ?? 0) - 1 === 2,
  );

  const closedRows = dataRowsOf(cardOf(root, t.closedTitle));
  check(
    'DASH-001: closed row shows cause, MiB bytes, and end time',
    closedRows.length === 1
      && closedRows[0].textContent.includes(t.causePeerEof)
      && closedRows[0].textContent.includes('1.5 MiB')
      && TIME_IN.test(closedRows[0].textContent),
  );

  check(
    'DASH-001: list cards show no status badge',
    [t.waitingTitle, t.activeTitle, t.closedTitle].every((title) => badgeOf(cardOf(root, title)) === null),
  );
  checkLayout('DASH-001');
}

// DASH-002: three empty lists show their empty text, not an error.
async function caseDash002() {
  const { root } = await mount(
    fakeApi({
      'GET /api/v1/waiting': () => ok({ as_of: AS_OF, items: [] }),
      'GET /api/v1/sessions': () => ok({ as_of: AS_OF, items: [] }),
      'GET /api/v1/closed-sessions': () => ok({ as_of: AS_OF, max_entries: 100, items: [] }),
    }),
  );
  await waitFor(
    () =>
      emptyOf(cardOf(root, t.waitingTitle)) === t.waitingEmpty &&
      emptyOf(cardOf(root, t.activeTitle)) === t.activeEmpty &&
      emptyOf(cardOf(root, t.closedTitle)) === t.closedEmpty,
  );
  check('DASH-002: empty waiting list text', emptyOf(cardOf(root, t.waitingTitle)) === t.waitingEmpty);
  check('DASH-002: empty active list text', emptyOf(cardOf(root, t.activeTitle)) === t.activeEmpty);
  check('DASH-002: empty closed list text', emptyOf(cardOf(root, t.closedTitle)) === t.closedEmpty);
  check('DASH-002: empty lists show no error alert', root.querySelectorAll('.card-alert .alert').length === 0);
  checkLayout('DASH-002');
}

// DASH-003: a failure with no earlier data shows only the error; a later failure keeps the old values as stale.
async function caseDash003() {
  const first = await mount(fakeApi({ 'GET /api/v1/waiting': () => fail(500) }));
  await waitFor(() => settled(first.root));
  const failedCard = cardOf(first.root, t.waitingTitle);
  const failedAlert = alertOf(failedCard);
  check(
    'DASH-003 initial: failure alert with failed copy',
    failedAlert?.title === t.failed && failedAlert?.message === t.failedMessage,
  );
  check('DASH-003 initial: no rows and no empty text', dataRowsOf(failedCard).length === 0 && emptyOf(failedCard) === null);
  check('DASH-003 initial: no stale badge', badgeOf(failedCard) === null);

  const stale = await mount(
    fakeApi({
      'GET /api/v1/session': () => session(1),
      'GET /api/v1/waiting': (n) => (n === 1 ? ok({ as_of: AS_OF, items: [waitingItem()] }) : fail(500)),
    }),
  );
  await wait(1500);
  const staleCard = cardOf(stale.root, t.waitingTitle);
  check('DASH-003 stale: stale badge shown', badgeOf(staleCard) === t.stale);
  const staleAlert = alertOf(staleCard);
  check('DASH-003 stale: failure alert title shown', staleAlert?.title === t.failed);
  check('DASH-003 stale: alert names the last successful time', isFilledTemplate(staleAlert?.message ?? '', t.staleMessage));
  check(
    'DASH-003 stale: previous rows kept',
    dataRowsOf(staleCard).some((row) => row.textContent.includes('relay-7')),
  );
  checkLayout('DASH-003');
}

// DASH-004: after a failure, an explicit refresh restores the values and clears the alert; polling is not used.
async function caseDash004() {
  const { root, api } = await mount(
    fakeApi({
      'GET /api/v1/session': () => session(30),
      'GET /api/v1/waiting': (n) => (n === 1 ? fail(500) : ok({ as_of: AS_OF, items: [waitingItem({ relay_id: 'relay-9' })] })),
    }),
  );
  await waitFor(() => settled(root));
  check('DASH-004: failure shown before the retry', alertOf(cardOf(root, t.waitingTitle))?.title === t.failed);

  root.querySelector('#operations-refresh').click();
  await wait(300);
  const card = cardOf(root, t.waitingTitle);
  check('DASH-004: refreshed value restored', dataRowsOf(card).some((row) => row.textContent.includes('relay-9')));
  check('DASH-004: failure alert removed after success', alertOf(card) === null);
  check('DASH-004: no stale or failure badge left', badgeOf(card) === null);
  const waitingCalls = api.calls.filter((c) => c.key === 'GET /api/v1/waiting');
  check('DASH-004: explicit refresh sends no poll header', waitingCalls.at(-1)?.poll === false);
  checkLayout('DASH-004');
}

// DASH-005: one query per list at a time, and a late response from an ended session does not reach the screen.
async function caseDash005() {
  let inFlight = 0;
  let peak = 0;
  const slow = fakeApi({
    'GET /api/v1/session': () => session(1),
    'GET /api/v1/waiting': async (n) => {
      inFlight += 1;
      peak = Math.max(peak, inFlight);
      try {
        if (n === 1) await wait(2500);
        return ok({ as_of: AS_OF, items: [waitingItem()] });
      } finally {
        inFlight -= 1;
      }
    },
  });
  const first = await mount(slow);
  await waitFor(() => slow.count('GET /api/v1/waiting') === 1);
  first.root.querySelector('#operations-refresh').click();
  await wait(100);
  check('DASH-005 overlap: no second waiting query while one is in flight', slow.count('GET /api/v1/waiting') === 1);
  await wait(4400);
  check('DASH-005 overlap: never more than one waiting query at a time', peak === 1);
  check('DASH-005 overlap: queries resume after the response', slow.count('GET /api/v1/waiting') >= 2);

  // Late response: the summary poll gets 401 while a waiting response is still pending.
  const late = await mount(
    fakeApi({
      'GET /api/v1/session': () => session(1),
      'GET /api/v1/summary': (n) => (n === 1 ? ok(summaryBody()) : fail(401)),
      'GET /api/v1/waiting': () =>
        wait(1500).then(() => ok({ as_of: AS_OF, items: [waitingItem({ relay_id: 'relay-old' })] })),
    }),
  );
  await wait(2000);
  check('DASH-005 late: sent to the sign-in screen', same(late.navigations, ['/login/']));
  check('DASH-005 late: no cards left on the screen', late.root.querySelectorAll('section.card').length === 0);
  check('DASH-005 late: late response does not reach the screen', !late.root.textContent.includes('relay-old'));
  checkLayout('DASH-005');
}

// DASH-008: an expired session sends the user to the sign-in screen and removes protected data.
async function caseDash008() {
  const first = await mount(fakeApi({ 'GET /api/v1/session': () => fail(401) }));
  await wait(300);
  check('DASH-008 bootstrap: sent to the sign-in screen', same(first.navigations, ['/login/']));
  check('DASH-008 bootstrap: no data shown', first.root.childElementCount === 0);
  check('DASH-008 bootstrap: no management query sent', first.api.calls.length === 1);

  const shown = await mount(
    fakeApi({
      'GET /api/v1/session': () => session(1),
      'GET /api/v1/summary': (n) => (n === 1 ? ok(summaryBody()) : fail(401)),
    }),
  );
  await waitFor(() => settled(shown.root));
  check('DASH-008 expiry: data shown before expiry', cardOf(shown.root, t.summaryTitle) !== null);
  await wait(1200);
  check('DASH-008 expiry: sent to the sign-in screen', same(shown.navigations, ['/login/']));
  check('DASH-008 expiry: protected data removed', shown.root.childElementCount === 0);
  const seen = shown.api.calls.length;
  await wait(2200);
  check('DASH-008 expiry: no further management requests', shown.api.calls.length === seen);
  checkLayout('DASH-008');
}

// DASH-016: a relay state change appears after the next poll; the screen offers no remote-screen or settings actions.
async function caseDash016() {
  const api = fakeApi({
    'GET /api/v1/session': () => session(1),
    'GET /api/v1/summary': (n) =>
      ok(
        n === 1
          ? summaryBody()
          : summaryBody({
              server: { state: 'shutdown_grace', uptime_ms: 90061000 },
              counts: { waiting: 2, active: 0, handshaking: 0, target_connecting: null },
            }),
      ),
  });
  const { root } = await mount(api);
  await waitFor(() => statValue(root, t.serverState) === t.stateReady);
  check('DASH-016: state before the change', statValue(root, t.serverState) === t.stateReady);
  await waitFor(() => statValue(root, t.serverState) === t.stateShutdownGrace);
  check('DASH-016: changed state shown after the next poll', statValue(root, t.serverState) === t.stateShutdownGrace);
  check(
    'DASH-016: changed counts shown',
    statValue(root, t.waitingCount) === '2' && statValue(root, t.activeCount) === '0',
  );
  check(
    'DASH-016: the change arrived through a poll',
    api.calls.filter((c) => c.key === 'GET /api/v1/summary').at(-1)?.poll === true,
  );
  check('DASH-016: last success time shown', isFilledTemplate(metaText(root), t.lastSuccess));

  const actionLabels = [...root.querySelectorAll('button')]
    .filter((button) => !button.closest('table'))
    .map((button) => button.querySelector('.button-label')?.textContent.trim())
    .sort();
  check('DASH-016: only refresh and sign-out actions outside the tables', same(actionLabels, [t.refresh, t.signOut].sort()));
  check('DASH-016: no input fields for settings or remote screen', root.querySelectorAll('input, textarea, form').length === 0);
  checkLayout('DASH-016');
}

// DASH-019: sign-out success removes data and goes to sign-in; a lost or failed response goes to the unconfirmed notice.
async function caseDash019() {
  const success = await mount(fakeApi());
  await waitFor(() => settled(success.root));
  const logout = success.root.querySelector('#operations-logout');
  logout.click();
  logout.click();
  await wait(300);
  const deletes = success.api.calls.filter((c) => c.key === 'DELETE /api/v1/session');
  check('DASH-019 success: one sign-out request', deletes.length === 1);
  check('DASH-019 success: request carries the CSRF token', deletes[0]?.csrf === 'csrf-token-1');
  check('DASH-019 success: sent to the sign-in screen', same(success.navigations, ['/login/']));
  check('DASH-019 success: data removed', success.root.childElementCount === 0);
  const seen = success.api.calls.length;
  await wait(2200);
  check('DASH-019 success: no polling after sign-out', success.api.calls.length === seen);

  const rejected = await mount(fakeApi({ 'DELETE /api/v1/session': () => fail(500) }));
  await waitFor(() => settled(rejected.root));
  rejected.root.querySelector('#operations-logout').click();
  await wait(300);
  check(
    'DASH-019 failed response: unconfirmed notice on the sign-in screen',
    same(rejected.navigations, ['/login/?notice=logout-unconfirmed']),
  );

  const lost = await mount(
    fakeApi({
      'DELETE /api/v1/session': () => {
        throw new TypeError('network down');
      },
    }),
  );
  await waitFor(() => settled(lost.root));
  lost.root.querySelector('#operations-logout').click();
  await wait(300);
  check(
    'DASH-019 lost response: unconfirmed notice on the sign-in screen',
    same(lost.navigations, ['/login/?notice=logout-unconfirmed']),
  );

  // A refused sign-out (403 csrf_invalid) did not finish: no completion is shown and data is hidden (spec §5).
  const refused = await mount(fakeApi({ 'DELETE /api/v1/session': () => fail(403) }));
  await waitFor(() => settled(refused.root));
  refused.root.querySelector('#operations-logout').click();
  await wait(300);
  check(
    'DASH-019 refused (403): unconfirmed notice on the sign-in screen',
    same(refused.navigations, ['/login/?notice=logout-unconfirmed']),
  );

  const expired = await mount(fakeApi({ 'DELETE /api/v1/session': () => fail(401) }));
  await waitFor(() => settled(expired.root));
  expired.root.querySelector('#operations-logout').click();
  await wait(300);
  check('DASH-019 expired session: sent to the sign-in screen', same(expired.navigations, ['/login/']));
  checkLayout('DASH-019');
}

// DASH-020: closed rows show each cause, byte counts, and end time; the closed card has no actions.
async function caseDash020() {
  const { root } = await mount(
    fakeApi({
      'GET /api/v1/closed-sessions': () =>
        ok({
          as_of: AS_OF,
          max_entries: 100,
          items: [
            closedItem(),
            closedItem({
              session_id: 'sess_3',
              close_cause: 'admin_terminated',
              bytes_viewer_to_target: 2048,
              bytes_target_to_viewer: 0,
            }),
            closedItem({ session_id: 'sess_4', close_cause: 'connection_error', bytes_viewer_to_target: 0, bytes_target_to_viewer: 0 }),
            closedItem({ session_id: 'sess_5', close_cause: 'future_cause', bytes_viewer_to_target: 0, bytes_target_to_viewer: 0 }),
          ],
        }),
    }),
  );
  await waitFor(() => settled(root));
  const card = cardOf(root, t.closedTitle);
  const rows = dataRowsOf(card);
  check('DASH-020: four closed rows', rows.length === 4);
  check('DASH-020: normal EOF cause shown', rows[0]?.textContent.includes(t.causePeerEof));
  check('DASH-020: administrator termination shown', rows[1]?.textContent.includes(t.causeAdmin));
  check('DASH-020: connection error shown', rows[2]?.textContent.includes(t.causeConnectionError));
  check('DASH-020: unknown cause shown as raw text', rows[3]?.textContent.includes('future_cause'));
  check('DASH-020: viewer-to-target bytes in KiB', rows[1]?.textContent.includes('2.0 KiB'));
  check('DASH-020: target-to-viewer bytes in MiB', rows[0]?.textContent.includes('1.5 MiB'));
  check('DASH-020: end time shown in local time', TIME_IN.test(rows[0]?.textContent ?? ''));
  check('DASH-020: closed card has no buttons or links', (card?.querySelectorAll('button, a').length ?? -1) === 0);

  const empty = await mount(
    fakeApi({ 'GET /api/v1/closed-sessions': () => ok({ as_of: AS_OF, max_entries: 100, items: [] }) }),
  );
  await waitFor(() => settled(empty.root));
  check('DASH-020: empty closed list text', emptyOf(cardOf(empty.root, t.closedTitle)) === t.closedEmpty);
  // Byte formats follow DESIGN.md and match in every language (format.js).
  check('DASH-020 format: 0 bytes', formatBytes(0) === '0.0 KiB');
  check('DASH-020 format: 256 bytes', formatBytes(256) === '0.3 KiB');
  check('DASH-020 format: 1536 bytes', formatBytes(1536) === '1.5 KiB');
  check('DASH-020 format: 1048575 bytes', formatBytes(1048575) === '1.0 MiB');
  check('DASH-020 format: 1572864 bytes', formatBytes(1572864) === '1.5 MiB');
  // The screen keeps byte counts as Numbers, so values above 2^53 are rounded when parsed.
  const wholeMib = (count) => `${new Intl.NumberFormat().format(count)}.0 MiB`;
  check('DASH-020 format: 2^53-1 as BigInt', formatBytes(2n ** 53n - 1n) === wholeMib(2 ** 33));
  check(
    'DASH-020 format: JSON 9007199254740993 read as 2^53',
    formatBytes(JSON.parse('9007199254740993')) === wholeMib(2 ** 33),
  );
  check('DASH-020 format: 2^60 bytes', formatBytes(2 ** 60) === wholeMib(2 ** 40));
  check('DASH-020 format: 2^70 bytes', formatBytes(2 ** 70) === wholeMib(2 ** 50));
  checkLayout('DASH-020');
}

// DASH-021: a known zero is shown as 0; an unknown value is shown as unavailable, never as 0.
async function caseDash021() {
  const { root } = await mount(
    fakeApi({
      'GET /api/v1/summary': () =>
        ok(
          summaryBody({
            counts: { waiting: 1, active: 1, handshaking: 2, target_connecting: 3 },
            usage: {
              relay_connections: { current: 0, limit: 100 },
              admin_connections: { current: null, limit: 20 },
            },
          }),
        ),
    }),
  );
  await waitFor(() => settled(root));
  check('DASH-021: handshaking count shown', statValue(root, t.handshaking) === '2');
  check('DASH-021: Mode I target count shown', statValue(root, t.targetConnecting) === '3');

  const relay = gaugeOf(root, t.relayTitle);
  check('DASH-021: relay zero shown as 0', gaugeValue(relay) === '0');
  check('DASH-021: relay limit shown', /100/.test(gaugeMax(relay)));
  check('DASH-021: relay description shown', relay?.closest('.gauge-wrap')?.textContent.includes(t.relayNote) === true);

  const admin = gaugeOf(root, t.adminTitle);
  check('DASH-021: unknown admin usage has no value', admin !== null && admin.querySelector('.gauge-value') === null);
  check('DASH-021: unknown admin usage shows unavailable', admin?.textContent.includes(t.unavailable) === true);
  check('DASH-021: admin limit shown', /20/.test(gaugeMax(admin)));
  check('DASH-021: admin description shown', admin?.closest('.gauge-wrap')?.textContent.includes(t.adminNote) === true);
  checkLayout('DASH-021');
}

// Milliseconds from the first summary call to the second; NaN until there are two calls.
const intervalOf = (calls) => (calls.length >= 2 ? calls[1].at - calls[0].at : NaN);

// DASH-022: the poll interval is the configured P; no overlap; the default is 2 s when P is not set.
async function caseDash022() {
  const custom = await mount(fakeApi({ 'GET /api/v1/session': () => session(3) }));
  await waitFor(() => custom.api.count('GET /api/v1/summary') >= 2, 6000);
  const customCalls = custom.api.calls.filter((c) => c.key === 'GET /api/v1/summary');
  check('DASH-022 P=3: one poll after the first load', customCalls.length === 2 && customCalls[1].poll === true);
  check('DASH-022 P=3: poll interval is 3 s', intervalOf(customCalls) >= 2950 && intervalOf(customCalls) < 3750);

  const dflt = await mount(fakeApi({ 'GET /api/v1/session': () => session(2) }));
  await waitFor(() => dflt.api.count('GET /api/v1/summary') >= 2, 6000);
  const dfltCalls = dflt.api.calls.filter((c) => c.key === 'GET /api/v1/summary');
  check('DASH-022 P=2: one poll after the first load', dfltCalls.length === 2 && dfltCalls[1].poll === true);
  check('DASH-022 P=2: poll interval is 2 s', intervalOf(dfltCalls) >= 1950 && intervalOf(dfltCalls) < 2750);
  checkLayout('DASH-022');
}

// Browser storage stand-in that keeps the saved language in `items`.
function storageOf(items) {
  return {
    getItem: (key) => items.get(key) ?? null,
    setItem: (key, value) => items.set(key, String(value)),
  };
}

// DASH-024: the first screen follows the browser language; nothing is saved until the user chooses.
async function caseDash024() {
  const cases = [
    [['ko-KR'], 'ko'],
    [['en-US'], 'en'],
    [['ja-JP'], 'en'],
    [['ja-JP', 'ko'], 'ko'],
  ];
  for (const [preferred, expected] of cases) {
    const items = new Map();
    const { root } = await mount(fakeApi(), { storage: storageOf(items), preferred });
    await waitFor(() => root.textContent.includes('1.5 MiB'));
    const label = `DASH-024 ${preferred.join(',')}`;
    check(`${label}: document language`, document.documentElement.lang === expected);
    check(`${label}: language select`, root.querySelector('#operations-language')?.value === expected);
    check(`${label}: page title in that language`, root.textContent.includes(COPY[expected].operations));
    check(`${label}: byte format is the same`, root.textContent.includes('1.5 MiB'));
    check(`${label}: nothing saved before a choice`, items.size === 0);
  }
  checkLayout('DASH-024');
}

// DASH-025: a chosen language applies at once and stays after reload and sign-out; no request carries it.
async function caseDash025() {
  const items = new Map();
  const storage = storageOf(items);
  const api = fakeApi();
  const first = await mount(api, { storage, preferred: ['ko-KR'] });
  await waitFor(() => settled(first.root));
  check('DASH-025 start: browser preference gives Korean', document.documentElement.lang === 'ko');
  const select = first.root.querySelector('#operations-language');
  select.value = 'en';
  select.dispatchEvent(new Event('change'));
  await wait(300);
  check('DASH-025 change: document language is English', document.documentElement.lang === 'en');
  check('DASH-025 change: copy is English at once', first.root.textContent.includes(COPY.en.operations));
  check('DASH-025 change: choice saved', items.get(STORAGE_KEY) === 'en');

  const again = await mount(api, { storage, preferred: ['ko-KR'] });
  await waitFor(() => settled(again.root));
  check('DASH-025 reload: saved choice beats browser preference', document.documentElement.lang === 'en');
  check('DASH-025 reload: select keeps English', again.root.querySelector('#operations-language')?.value === 'en');

  again.root.querySelector('#operations-logout').click();
  await wait(300);
  check('DASH-025 sign-out: sent to sign-in', same(again.navigations, ['/login/']));
  check('DASH-025 sign-out: choice still saved', items.get(STORAGE_KEY) === 'en');
  check('DASH-025: no request carries a language value', api.calls.every((c) => !/lang/i.test(`${c.key} ${c.raw}`)));
  checkLayout('DASH-025');
}

const CASES = {
  'dash-001': caseDash001,
  'dash-002': caseDash002,
  'dash-003': caseDash003,
  'dash-004': caseDash004,
  'dash-005': caseDash005,
  'dash-008': caseDash008,
  'dash-016': caseDash016,
  'dash-019': caseDash019,
  'dash-020': caseDash020,
  'dash-021': caseDash021,
  'dash-022': caseDash022,
  'dash-024': caseDash024,
  'dash-025': caseDash025,
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
