// Scenario check for the hash router: the operations list opens a session detail and returns to the list
// (dashboard.md DASH-006). The page mounts app/router.js, the same entry as app/main.js, and drives it with
// hash changes. One page load runs one case, ?case=dash-006&lang=ko|en, against a fake management API.
// The result goes to <main id="result">. Wording comes from app/copy.js, which holds the spec §9.3 CPY table.
import { mountRouter } from '../app/router.js';
import { COPY } from '../app/copy.js';

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
const SESSION_ID = 'sess_1';
const LIST = 'GET /api/v1/sessions';
const DETAIL = `GET /api/v1/sessions/${SESSION_ID}`;
const DETAIL_HASH = `#/sessions/${SESSION_ID}`;

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
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

// Poll interval 60 s, so no automatic poll adds a list read during the check.
const DEFAULT_ROUTES = {
  'GET /api/v1/session': () =>
    ok({
      csrf_token: 'csrf-token-1',
      dashboard: { poll_secs: 60, query_timeout_secs: 5, terminate_timeout_secs: 10 },
    }),
  'GET /api/v1/summary': () =>
    ok({
      as_of: AS_OF,
      server: { state: 'ready', uptime_ms: 90061000 },
      counts: { waiting: 0, active: 1, handshaking: 0, target_connecting: null },
      usage: {
        relay_connections: { current: 1, limit: 100 },
        admin_connections: { current: 1, limit: 20 },
      },
    }),
  'GET /api/v1/waiting': () => ok({ as_of: AS_OF, items: [] }),
  [LIST]: () =>
    ok({
      as_of: AS_OF,
      items: [
        {
          session_id: SESSION_ID,
          state: 'active',
          mode: 'mode1',
          viewer_transport: 'wss',
          started_at: STARTED,
          bytes_viewer_to_target: 1536,
          bytes_target_to_viewer: 1048575,
          viewer_address: { ip: '192.0.2.10', port: 51000 },
          target_address: { ip: '198.51.100.5', port: 5900 },
        },
      ],
    }),
  'GET /api/v1/closed-sessions': () => ok({ as_of: AS_OF, max_entries: 100, items: [] }),
  [DETAIL]: () =>
    ok({
      as_of: AS_OF,
      session: {
        session_id: SESSION_ID,
        state: 'active',
        mode: 'mode1',
        viewer_transport: 'wss',
        started_at: STARTED,
        bytes_viewer_to_target: 1536,
        bytes_target_to_viewer: 1048575,
        viewer_address: { ip: '192.0.2.10', port: 51000 },
        target_address: { ip: '198.51.100.5', port: 5900 },
      },
    }),
};

function fakeApi() {
  const calls = [];
  const counts = new Map();
  const request = (path, init = {}) => {
    const key = `${init.method ?? 'GET'} ${path}`;
    const n = (counts.get(key) ?? 0) + 1;
    counts.set(key, n);
    calls.push({ key });
    const route = DEFAULT_ROUTES[key];
    return Promise.resolve().then(() => (route ? route(n) : fail(404, 'not_found')));
  };
  return { request, calls, count: (key) => counts.get(key) ?? 0 };
}

const detailLinkOf = (root) =>
  [...root.querySelectorAll('a')].find((link) => link.textContent.trim() === t.details) ?? null;
const backOf = (root) => [...root.querySelectorAll('a')].find((link) => link.textContent.trim() === t.back) ?? null;
const titleOf = (root) => root.querySelector('.page-title')?.textContent.trim() ?? null;
const terminateOf = (root) => root.querySelector('#session-terminate');
const enabled = (root) => terminateOf(root) !== null && !terminateOf(root).disabled;

function checkLayout(label) {
  const page = document.documentElement;
  check(`${label}: no horizontal scroll`, page.scrollWidth <= page.clientWidth);
}

// DASH-006: a list row opens the selected active session through the router, and the back link returns to the list.
async function caseDash006() {
  const api = fakeApi();
  const navigations = [];
  mountRouter(host, {
    request: api.request,
    storage: { getItem: () => null, setItem() {} },
    preferredLanguages: [lang === 'en' ? 'en-US' : 'ko-KR'],
    navigate: (url) => navigations.push(url),
  });

  await waitFor(() => detailLinkOf(host) !== null);
  const link = detailLinkOf(host);
  check('DASH-006: list row links to the selected session', link?.getAttribute('href') === `/${DETAIL_HASH}`);

  // A click would leave this page, because the link is root-relative and this page lives under /tests/.
  // The hash that the link names is applied instead. That hash change is what the router handles.
  location.hash = DETAIL_HASH;
  await waitFor(() => enabled(host));
  check(
    'DASH-006: detail title shows the selected session id',
    titleOf(host) === t.sessionTitle.replace('{session_id}', SESSION_ID),
  );
  check('DASH-006: terminate action enabled for an active session', enabled(host));
  check('DASH-006: list screen removed on detail', detailLinkOf(host) === null);
  check('DASH-006: one detail read', api.count(DETAIL) === 1);
  check('DASH-006: list not queried again for the detail', api.count(LIST) === 1);
  check('DASH-006: back link goes to the list', backOf(host)?.getAttribute('href') === '/');

  location.hash = '';
  await waitFor(() => detailLinkOf(host) !== null);
  check('DASH-006: back shows the list again', detailLinkOf(host) !== null);
  check('DASH-006: detail removed on back', terminateOf(host) === null);
  check('DASH-006: list queried again on return', api.count(LIST) === 2);
  check('DASH-006: router stayed on this page', navigations.length === 0);
  checkLayout('DASH-006');
}

const CASES = {
  'dash-006': caseDash006,
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
