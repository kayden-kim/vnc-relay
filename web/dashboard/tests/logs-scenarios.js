// Contract-shaped fake API evidence only. Real server coverage belongs to dashboard_live.
import { mountLogs } from '../app/logs.js';
import { COPY } from '../app/copy.js';
import { mountLogin } from '../login/login.js';
import { STORAGE_KEY } from '../app/language.js';
import { checkHelpLanguageChange } from './help-link-checks.js';

const host = document.getElementById('host');
const result = document.getElementById('result');
const params = new URLSearchParams(location.search);
const lang = params.get('lang') === 'en' ? 'en' : 'ko';
const t = COPY[lang];
const failures = [];
let cspViolations = 0;
document.addEventListener('securitypolicyviolation', () => { cspViolations += 1; });
const AS_OF = '2026-11-08T16:00:00.000Z';
const SESSION_ID = 'ses_3k9d2m7q4x8v1b6n5c0z2h4w7y';
const check = (label, condition) => { if (!condition) failures.push(label); };
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
async function waitFor(ready, ms = 5000) {
  for (let n = Math.ceil(ms / 20); n > 0 && !ready(); n -= 1) await wait(20);
}
function deferred() {
  let resolve;
  const promise = new Promise((done) => { resolve = done; });
  return { promise, resolve };
}
const json = (status, body) => new Response(JSON.stringify(body), {
  status, headers: { 'Content-Type': 'application/json' },
});
const ok = (body) => json(200, body);
const fail = (status) => json(status, { error: { code: status === 401 ? 'unauthenticated' : 'invalid_request', message: 'error' } });
function item(seq, entry = {}) {
  return { seq, entry: { ts: AS_OF, level: 'info', event: 'session_started', session_id: SESSION_ID, ...entry } };
}
const ITEMS = [
  item(1, { event: 'server_ready', listeners: [{ name: 'ws', enabled: true }], session_id: null }),
  item(2, { level: 'error', event: 'startup_failed', reason: 'listener_bind_failed', detail: '대상 오류 <script>not executable</script>' }),
  item(3, { level: 'warn', event: 'connection_rejected', reason: 'forwarded_for_invalid', lag_bytes: 65536 }),
  item(4, { event: 'connection_accepted', listener: 'ws', conn_id: 'c-4', details: { role: 'viewer', proxy_hops: 1, values: [true, null, 'MiXeD'] } }),
];
function page(items = ITEMS, extra = {}) {
  return { as_of: AS_OF, max_entries: 1000, oldest_seq: 1, latest_seq: 4, has_more: false, items, ...extra };
}
function fakeApi(handler = () => ok(page()), { pollSecs = 1, timeoutSecs = 1 } = {}) {
  const calls = [];
  const request = (path, init = {}) => {
    const url = new URL(path, location.origin);
    const call = { path, init, query: url.searchParams, at: performance.now() };
    if (url.pathname === '/api/v1/session') return Promise.resolve(ok({
      csrf_token: 'fake-csrf-token', dashboard: { poll_secs: pollSecs, query_timeout_secs: timeoutSecs, terminate_timeout_secs: 1 },
    }));
    if (url.pathname !== '/api/v1/logs') throw new Error(`unexpected path ${path}`);
    calls.push(call);
    return Promise.resolve(handler(call, calls.length));
  };
  return { calls, request };
}
const storage = () => ({ getItem: (key) => key === STORAGE_KEY ? lang : null, setItem() {} });
let current;
function mount(api = fakeApi(), sessionId = null) {
  current?.handle.dispose();
  const navigations = [];
  const urls = [];
  const handle = mountLogs(host, {
    request: api.request, storage: storage(), preferredLanguages: [lang],
    navigate: (url) => navigations.push(url), replaceUrl: (url) => urls.push(url),
  }, { sessionId });
  current = { handle, api, navigations, urls };
  return current;
}
const rows = () => [...host.querySelectorAll('.log-row')].map((row) => Number(row.id.slice(6)));
const ready = () => host.querySelector('#logs-refresh')?.disabled === false;
const text = () => host.textContent;
const query = (api) => api.calls.at(-1).query;
const pause = () => { if (host.querySelector('#logs-auto-refresh')?.getAttribute('aria-checked') === 'true') host.querySelector('#logs-auto-refresh').click(); };
async function submit(value) {
  const input = host.querySelector('#logs-q');
  input.value = value;
  input.dispatchEvent(new Event('input', { bubbles: true }));
  host.querySelector('form').requestSubmit();
  await waitFor(ready);
}
function checkLayout(label) {
  check(`${label}: no horizontal scroll`, document.documentElement.scrollWidth <= document.documentElement.clientWidth);
  check(`${label}: three navigation items`, host.querySelectorAll('.navbar-nav .navbar-item').length === 3);
  for (const control of host.querySelectorAll('button, input, select')) {
    const box = control.getBoundingClientRect();
    if (box.width && box.height) check(`${label}: control inside viewport`, box.left >= 0 && box.right <= innerWidth + 1);
  }
  if (innerWidth <= 1023.98) {
    check(`${label}: level targets use the touch height`, [...host.querySelectorAll('.check')].every((node) => node.getBoundingClientRect().height >= 44));
    check(`${label}: details use one column`, [...host.querySelectorAll('.log-detail')].filter((node) => node.getBoundingClientRect().height > 0).every((node) => getComputedStyle(node).gridTemplateColumns.split(' ').length === 1));
  }
}
async function case054() {
  const gate = deferred();
  const m = mount(fakeApi(() => gate.promise));
  check('054: loading badge', host.querySelector('.log-meta .badge')?.textContent === t.loading);
  check('054: unknown time, not zero count', host.querySelector('.log-meta')?.textContent.includes(t.unavailable) && !host.querySelector('.log-meta')?.textContent.includes('1,000'));
  check('054: no empty-result copy while loading', !text().includes(t.logsEmpty));
  check('054: refresh names the pending load', host.querySelector('#logs-refresh')?.textContent.includes(t.loading));
  gate.resolve(ok(page()));
  await waitFor(ready);
  pause();
  check('054: latest first', same(rows(), [4, 3, 2, 1]));
  check('054: retained count', host.querySelector('.log-meta')?.textContent.includes('1,000'));
  check('054: local date and zone', /\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} \S+/.test(host.querySelector('#log-r-4 .time')?.textContent));
  host.querySelector('#log-r-4 .act button').click();
  check('054: details expanded with all nested fields', !host.querySelector('#log-d-4').hidden && host.querySelector('#log-d-4').textContent.includes('proxy_hops'));
  check('054: details preserve values', host.querySelector('#log-d-4').textContent.includes('MiXeD') && host.querySelector('#log-d-4').textContent.includes('null'));
  host.querySelector('#log-r-4 .act button').click();
  check('054: details collapsed', host.querySelector('#log-d-4').hidden);
  check('054: raw values never create script', host.querySelector('script') === null);
  check('054: initial request is user action', !m.api.calls[0].init.headers['X-Relay-Poll']);
  checkLayout('054');
}
async function case055() {
  const m = mount();
  await waitFor(ready);
  pause();
  for (const level of ['warn', 'error']) {
    host.querySelector(`#logs-level-${level}`).click();
    await waitFor(ready);
  }
  check('055: info only', query(m.api).get('level') === 'info');
  const n = m.api.calls.length;
  host.querySelector('#logs-level-info').click();
  check('055: all unchecked blocks request and download', m.api.calls.length === n && text().includes(t.levelRequiredError) && host.querySelector('#logs-download').disabled);
  host.querySelector('#logs-level-info').click();
  await waitFor(ready);
  const select = host.querySelector('#logs-event');
  select.value = 'server_ready';
  select.dispatchEvent(new Event('change'));
  await waitFor(ready);
  check('055: filters combined without cursors', query(m.api).get('event') === 'server_ready' && query(m.api).get('level') === 'info' && !query(m.api).has('after') && !query(m.api).has('before'));
  check('055: known proxy event available unchanged', !!host.querySelector('option[value="trust_all_proxies"]'));
  // A producer ignoring abort still must not overwrite a new filter.
  const late = deferred();
  const race = mount(fakeApi((call, count) => count === 1 ? late.promise : ok(page([item(20)], { latest_seq: 20 }))));
  await waitFor(() => race.api.calls.length === 1);
  host.querySelector('#logs-level-warn').click();
  await waitFor(ready);
  late.resolve(ok(page()));
  await wait(40);
  check('055: old response ignored and aborted', same(rows(), [20]) && race.api.calls[0].init.signal.aborted);
  pause();
  checkLayout('055');

  const recoveryApi = fakeApi((_call, count) => count === 1
    ? ok(page([item(100, { event: 'connection_accepted' })], { latest_seq: 100 }))
    : count === 2 ? fail(500)
      : ok(page([item(95, { event: 'server_ready' })], { latest_seq: 100 })));
  mount(recoveryApi);
  await waitFor(ready);
  const event = host.querySelector('#logs-event');
  event.value = 'server_ready';
  event.dispatchEvent(new Event('change'));
  await waitFor(ready);
  check('055: failed filter snapshot keeps old rows as stale', same(rows(), [100]) && text().includes(t.staleMessage.split('{')[0]));
  await waitFor(() => recoveryApi.calls.length >= 3 && ready());
  pause();
  const retry = recoveryApi.calls[2];
  check('055: automatic snapshot recovery has no cursor and is marked as poll',
    retry.query.get('event') === 'server_ready' && !retry.query.has('after') &&
    !retry.query.has('before') && retry.init.headers['X-Relay-Poll'] === '1');
  check('055: recovered snapshot replaces the previous filter rows', same(rows(), [95]));
}
async function case056() {
  const m = mount();
  await waitFor(ready);
  pause();
  const initial = m.api.calls.length;
  const input = host.querySelector('#logs-q');
  input.value = 'draft';
  input.dispatchEvent(new Event('input'));
  check('056: typing sends no request', m.api.calls.length === initial);
  for (const value of ['a', '😀'.repeat(256), '  MiXeD  ', '65536', 'true', 'null']) {
    await submit(value);
    check('056: exact scalar search forwarded without trimming', query(m.api).get('q') === value);
  }
  const n = m.api.calls.length;
  await submit('😀'.repeat(257));
  check('056: 257 scalars blocked', m.api.calls.length === n && host.querySelector('#logs-q').getAttribute('aria-invalid') === 'true');
  check('056: error linked and focused', host.querySelector('#logs-q').getAttribute('aria-describedby').includes('logs-q-error') && document.activeElement.id === 'logs-q');
  check('056: search error includes the danger icon', host.querySelector('#logs-q-error .icon') !== null);
  await submit('');
  check('056: empty removes q', !query(m.api).has('q'));
  checkLayout('056');
}
async function case057() {
  const gate = deferred();
  let active = 0;
  let maxActive = 0;
  const api = fakeApi(async (call, n) => {
    active += 1;
    maxActive = Math.max(maxActive, active);
    try {
      if (n === 1) return ok(page());
      if (n === 2) return await gate.promise;
      if (n === 3) return ok(page([item(7)], { latest_seq: 9, oldest_seq: 6 }));
      return ok(page([], { latest_seq: 12, oldest_seq: 8 }));
    } finally { active -= 1; }
  }, { timeoutSecs: 5 });
  mount(api);
  await waitFor(ready);
  await waitFor(() => api.calls.length === 2);
  check('057: P and poll marker', api.calls[1].at - api.calls[0].at >= 900 && api.calls[1].init.headers['X-Relay-Poll'] === '1' && api.calls[1].query.get('after') === '4');
  await wait(1100);
  check('057: slow request has no overlap', api.calls.length === 2 && maxActive === 1);
  gate.resolve(ok(page([item(6)], { latest_seq: 9, oldest_seq: 6, has_more: true })));
  await waitFor(() => rows()[0] === 7);
  check('057: all incremental pages sequential', api.calls[2].query.get('after') === '6' && api.calls[2].init.headers['X-Relay-Poll'] === '1' && maxActive === 1);
  check('057: gap shown, latest prepended without duplicate', text().includes(t.logsGapNotice) && same(rows(), [7, 6, 4, 3, 2, 1]));
  pause();
  const n = api.calls.length;
  await wait(1100);
  check('057: pause prevents next poll', api.calls.length === n && text().includes(t.logsPausedNotice));
  host.querySelector('#logs-refresh').click();
  await waitFor(ready);
  check('057: manual refresh allowed while paused', api.calls.length === n + 1 && !query(api).has('after') && !api.calls.at(-1).init.headers['X-Relay-Poll']);
  checkLayout('057');
  // A non-matching interval advances the confirmed cursor, without a false gap.
  const filtered = fakeApi((call, n) => ok(page([], { oldest_seq: n === 1 ? 1 : 5, latest_seq: n === 1 ? 10 : 20 })));
  mount(filtered);
  await waitFor(() => filtered.calls.length === 2 && ready());
  pause();
  check('057: empty-filter cursor uses buffer latest', filtered.calls[1].query.get('after') === '10' && !text().includes(t.logsGapNotice));
  const restarted = fakeApi((call, n) => n === 1 ? ok(page()) : ok(page([item(1)], { oldest_seq: 1, latest_seq: 1 })));
  mount(restarted);
  await waitFor(() => restarted.calls.length === 3 && ready());
  pause();
  check('057: restart discards old list and fetches fresh', same(rows(), [1]) && !restarted.calls[2].query.has('after'));
  check('057: automatic restart recovery remains a poll', restarted.calls[2].init.headers['X-Relay-Poll'] === '1');
}
async function case058() {
  const api = fakeApi((call, n) => n === 1
    ? ok(page([item(3), item(4)], { has_more: true }))
    : ok(page([item(1), item(2)], { has_more: false })));
  mount(api);
  await waitFor(ready);
  pause();
  host.querySelector('#logs-older').click();
  await waitFor(ready);
  check('058: before oldest, never after', query(api).get('before') === '3' && !query(api).has('after') && !api.calls.at(-1).init.headers['X-Relay-Poll']);
  check('058: older entries append in newest order', same(rows(), [4, 3, 2, 1]) && !host.querySelector('#logs-older'));
  checkLayout('058');
}
async function case059() {
  for (const id of ['', 'not/valid', 'x'.repeat(65), '\uFFFD']) {
    const m = mount(fakeApi(), id);
    check('059: invalid ID sends no request', m.api.calls.length === 0 && text().includes(t.invalidSessionId));
    check('059: invalid ID disables automatic request', host.querySelector('#logs-auto-refresh').disabled && !host.querySelector('#logs-download'));
    check('059: invalid ID shows off, not a pending load', host.querySelector('#logs-auto-refresh').getAttribute('aria-checked') === 'false' && host.querySelector('.log-meta').textContent === '');
    check('059: invalid ID keeps its alert heading style', getComputedStyle(host.querySelector('.logs-session-error .alert-title')).fontWeight === '700');
    check('059: even an empty invalid filter can be cleared', !!host.querySelector('#logs-session-clear'));
  }
  const m = mount(fakeApi(), SESSION_ID);
  await waitFor(ready);
  pause();
  check('059: session filter sent and chip present', query(m.api).get('session_id') === SESSION_ID && host.querySelector('.chip').textContent.includes(SESSION_ID));
  check('059: retained-start limitation explained', text().includes(t.sessionFilterNote));
  await submit('keep me');
  host.querySelector('#logs-level-warn').click();
  await waitFor(ready);
  host.querySelector('#logs-session-clear').click();
  await waitFor(ready);
  check('059: only session filter cleared', !query(m.api).has('session_id') && query(m.api).get('q') === 'keep me' && query(m.api).get('level') === 'info,error' && same(m.urls, ['/#/logs']));
  checkLayout('059');
}
async function captureDownloads(run) {
  const blobs = [];
  const names = [];
  const OriginalBlob = window.Blob;
  const originalCreate = URL.createObjectURL;
  const originalRevoke = URL.revokeObjectURL;
  const originalClick = HTMLAnchorElement.prototype.click;
  // Reading a Blob back waits on the browser process in real time. Under the runner's
  // virtual time an idle page can skip to the budget end during that wait, so keep the
  // parts the page wrote. The real-server DASH-060 runner checks the saved bytes.
  window.Blob = class extends OriginalBlob {
    constructor(parts = [], options) { super(parts, options); this.parts = parts; }
  };
  URL.createObjectURL = (blob) => { blobs.push(blob); return 'blob:fixture'; };
  URL.revokeObjectURL = () => {};
  HTMLAnchorElement.prototype.click = function () { names.push(this.download); };
  try { await run(blobs, names); }
  finally {
    window.Blob = OriginalBlob;
    URL.createObjectURL = originalCreate;
    URL.revokeObjectURL = originalRevoke;
    HTMLAnchorElement.prototype.click = originalClick;
  }
}
async function case060() {
  await captureDownloads(async (blobs, names) => {
    const api = fakeApi((call, n) => {
      if (n === 1) return ok(page());
      if (!call.query.has('before')) return ok(page(Array.from({ length: 1000 }, (_, k) => item(k + 2, { viewer_conn_id: `c-${k + 2}` })), { latest_seq: 1001, has_more: true }));
      return ok(page([item(1, { viewer_conn_id: 'c-1' })], { latest_seq: 1002 }));
    });
    mount(api);
    await waitFor(ready);
    pause();
    host.querySelector('#logs-download').click();
    await waitFor(() => blobs.length === 1);
    check('060: paginated until exhausted', api.calls.length === 3 && api.calls[1].query.get('limit') === '1000' && api.calls[2].query.get('limit') === '1000' && api.calls[2].query.get('before') === '2');
    const content = blobs[0].parts.join('');
    const lines = content.trimEnd().split('\n').map(JSON.parse);
    check('060: all 1001 entries, LF, no wrappers or new entries', lines.length === 1001 && lines.every((entry) => entry.event === 'session_started' && !('seq' in entry)) && !content.includes('\r') && content.endsWith('\n'));
    check('060: file entries are in ascending seq order', lines.every((entry, index) => entry.viewer_conn_id === `c-${index + 1}`));
    check('060: filename uses UTC timestamp', /^vnc-relay-logs-\d{8}T\d{6}Z\.jsonl$/.test(names[0]));
    check('060: user headers only', api.calls.every((call) => !call.init.headers['X-Relay-Poll']));
    const broken = fakeApi((call, n) => n === 1 ? ok(page()) : n === 2 ? ok(page([item(2)], { has_more: true })) : fail(500));
    mount(broken);
    await waitFor(ready);
    host.querySelector('#logs-download').click();
    await waitFor(() => text().includes(t.downloadFailed));
    check('060: no partial file on page failure', blobs.length === 1 && text().includes(t.downloadFailed));
    const late = deferred();
    const exiting = mount(fakeApi((call, n) => n === 1 ? ok(page()) : late.promise));
    await waitFor(ready);
    host.querySelector('#logs-download').click();
    await waitFor(() => exiting.api.calls.length === 2);
    exiting.handle.dispose();
    late.resolve(ok(page()));
    await wait(40);
    check('060: leaving aborts and never saves', exiting.api.calls[1].init.signal.aborted && blobs.length === 1 && host.childElementCount === 0);
    const expired = mount(fakeApi((call, n) => n === 1 ? ok(page()) : fail(401)));
    await waitFor(ready);
    host.querySelector('#logs-download').click();
    await waitFor(() => expired.navigations.length === 1);
    check('060: download 401 clears data and navigates', same(expired.navigations, ['/login/']) && host.childElementCount === 0 && blobs.length === 1);
  });
}
async function case061() {
  for (const max of [0, 1000]) {
    mount(fakeApi(() => ok(page([], { max_entries: max, oldest_seq: null, latest_seq: null }))));
    await waitFor(ready);
    check('061: disabled differs from empty', text().includes(max === 0 ? t.logsDisabled : t.logsEmpty));
    if (max === 0) check('061: disabled allows refresh but not download', !host.querySelector('#logs-refresh').disabled && host.querySelector('#logs-download').disabled);
    if (max === 0) check('061: disabled auto refresh is visibly off', host.querySelector('#logs-auto-refresh').disabled && host.querySelector('#logs-auto-refresh').getAttribute('aria-checked') === 'false' && host.querySelector('#logs-auto-refresh .state').textContent === t.autoRefreshOff);
    if (max === 0) check('061: disabled retention disables filters', [...host.querySelectorAll('.filter-body input, .filter-body select, #logs-search')].every((node) => node.disabled));
    if (max === 0) check('061: disabled inputs use the muted surface', getComputedStyle(host.querySelector('#logs-q')).backgroundColor === getComputedStyle(host.querySelector('#logs-download')).backgroundColor);
    checkLayout('061 empty');
  }
  for (const status of [400, 405, 500]) {
    const m = mount(fakeApi(() => fail(status)));
    await waitFor(ready);
    pause();
    const expected = status === 400 ? t.logsInvalidRequest : status === 405 ? t.logsMethodNotAllowed : t.failedMessage;
    check('061: errors differentiated', text().includes(expected));
    check('061: unknown retained count, not empty result', host.querySelector('.log-meta .badge')?.textContent === t.unavailable && !text().includes(t.logsEmpty));
    check('061: unavailable table value, failure title and explanation', host.querySelector('td.empty .badge')?.textContent === t.unavailable && host.querySelector('.alert[data-tone="destructive"] .alert-title')?.textContent === t.failed && host.querySelector('.alert[data-tone="destructive"] .alert-message')?.textContent === expected);
    check('061: no failed response misrepresented', m.api.calls.length === 1);
  }
  const recovery = fakeApi((call, n) => n === 2 ? fail(500) : ok(page()));
  mount(recovery);
  await waitFor(ready);
  pause();
  const time = host.querySelector('.log-meta').textContent;
  host.querySelector('#logs-refresh').click();
  await waitFor(ready);
  check('061: stale rows and timestamp preserved', same(rows(), [4, 3, 2, 1]) && host.querySelector('.log-meta').textContent === time && text().includes(t.staleMessage.split('{')[0]));
  host.querySelector('#logs-refresh').click();
  await waitFor(ready);
  check('061: manual recovery removes failure', !host.querySelector('.alert[data-tone="destructive"]'));
  const timeout = mount(fakeApi((call) => new Promise((resolve, reject) => call.init.signal.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError'))))));
  await waitFor(() => text().includes(t.failedMessage));
  check('061: timeout is failed not empty', text().includes(t.failedMessage) && !text().includes(t.logsEmpty));
  timeout.handle.dispose();
  const expired = mount(fakeApi((call, n) => n === 1 ? ok(page()) : fail(401)));
  await waitFor(ready);
  host.querySelector('#logs-refresh').click();
  await waitFor(() => expired.navigations.length === 1);
  check('061: expired removes all protected DOM', same(expired.navigations, ['/login/']) && host.childElementCount === 0);
}
async function case063() {
  mount();
  await waitFor(ready);
  pause();
  host.querySelector('#log-r-4 .act button').click();
  const input = host.querySelector('#logs-q');
  input.value = '아직 적용하지 않은 입력';
  input.dispatchEvent(new Event('input'));
  const select = host.querySelector('#logs-language');
  select.focus();
  select.value = lang === 'ko' ? 'en' : 'ko';
  select.dispatchEvent(new Event('change'));
  check('063: document language and draft preserved', document.documentElement.lang === select.value && host.querySelector('#logs-q').value === input.value);
  check('063: language focus retained', document.activeElement.id === 'logs-language');
  check('063: expansion retained and linked', host.querySelector('#log-r-4 .act button').getAttribute('aria-controls') === 'log-d-4' && !host.querySelector('#log-d-4').hidden);
  check('063: explicit table semantics and caption', host.querySelector('table[role="table"] caption') && host.querySelectorAll('[role="columnheader"]').length === 5 && host.querySelectorAll('td:not([role="cell"])').length === 0);
  check('063: no per-entry live-region notifications', !host.querySelector('.log-list [aria-live], .log-list [role="status"]'));
  host.querySelector('#logs-q').focus();
  check('063: keyboard input reachable and labelled', document.activeElement.id === 'logs-q' && !!host.querySelector('label[for="logs-q"]'));
  checkLayout('063');

  const api = fakeApi();
  mount(api);
  await waitFor(ready);
  host.querySelector('#log-r-4 .act button').click();
  host.querySelector('#logs-q').value = 'unsubmitted draft';
  host.querySelector('#logs-q').dispatchEvent(new Event('input'));
  for (const selector of ['#logs-search', '.navbar-item[href="/#/settings"]', '#logs-q']) {
    const control = host.querySelector(selector);
    control.focus();
    if (selector === '#logs-q') control.setSelectionRange(2, 6);
    const before = api.calls.length;
    await waitFor(() => api.calls.length > before && ready());
    check(`063: automatic poll preserves focus on ${selector}`, document.activeElement === host.querySelector(selector));
    check('063: focus check observes an automatic request', api.calls.at(-1).init.headers['X-Relay-Poll'] === '1');
    check('063: automatic render preserves draft and details', host.querySelector('#logs-q').value === 'unsubmitted draft' && !host.querySelector('#log-d-4').hidden);
    if (selector === '#logs-q') check('063: selection range survives automatic render', host.querySelector('#logs-q').selectionStart === 2 && host.querySelector('#logs-q').selectionEnd === 6);
  }
  const search = host.querySelector('#logs-search');
  search.focus();
  search.click();
  await waitFor(ready);
  check('063: keyboard-activated search keeps search button focused', document.activeElement === host.querySelector('#logs-search'));
  pause();
}

// Persistent screen states for F-C. These call the real mountLogs and shared CSS.
async function shot(state) {
  state = ({ 'invalid-session': 'bad-id', download: 'downloading', 'input-error': 'search-error' })[state] ?? state;
  if (state === 'expired') {
    const m = mount(fakeApi(() => fail(401)));
    await waitFor(() => m.navigations.length > 0);
    check('shot expired: removed protected data', host.childElementCount === 0);
    mountLogin(host, { storage: storage(), preferredLanguages: [lang], search: '' });
    return;
  }
  const gate = deferred();
  const api = fakeApi((call, n) => {
    if (state === 'loading') return gate.promise;
    if (state === 'failed-first') return fail(500);
    if (state === 'disabled') return ok(page([], { max_entries: 0, oldest_seq: null, latest_seq: null }));
    if (state === 'request-rejected' && n > 1) return fail(400);
    if (state === 'empty' || state === 'filtered-empty') return ok(page([], { oldest_seq: null, latest_seq: null }));
    if (state === 'failed' && n > 1) return fail(500);
    if (state === 'refreshing' && n > 1) return gate.promise;
    if (state === 'downloading' && call.query.has('limit')) return gate.promise;
    if (state === 'download-failed' && call.query.has('limit')) return fail(500);
    if (state === 'gap' && call.query.has('after')) return ok(page([item(10)], { oldest_seq: 10, latest_seq: 10 }));
    return ok(page(ITEMS, { has_more: state === 'normal' }));
  }, { timeoutSecs: 3600, pollSecs: state === 'gap' ? 1 : 3600 });
  mount(api, state === 'bad-id' ? 'bad/id' : state === 'session' ? SESSION_ID : null);
  if (state === 'loading' || state === 'bad-id') return;
  await waitFor(ready);
  if (state === 'detail') host.querySelector('#log-r-4 .act button').click();
  if (state === 'paused') pause();
  if (state === 'failed' || state === 'refreshing' || state === 'request-rejected') host.querySelector('#logs-refresh').click();
  if (state === 'search-error') await submit('😀'.repeat(257));
  if (state === 'level-error') {
    for (const level of ['info', 'warn', 'error']) { host.querySelector(`#logs-level-${level}`).click(); await waitFor(() => ready() || text().includes(t.levelRequiredError)); }
  }
  if (state === 'filtered-empty') await submit('no matching value');
  if (state === 'downloading' || state === 'download-failed') host.querySelector('#logs-download').click();
  if (state === 'gap') await waitFor(() => text().includes(t.logsGapNotice));
  if (state === 'failed') await waitFor(() => host.querySelector('.alert[data-tone="destructive"]'));
  if (state === 'download-failed') await waitFor(() => text().includes(t.downloadFailed));
}
async function case064() {
  const m = mount();
  await waitFor(ready);
  pause();
  checkHelpLanguageChange(host, lang, check);
  check('DASH-064: no help API requests', m.api.calls.every((call) => new URL(call.path, location.origin).origin === location.origin));
  check('DASH-064: current screen remains mounted', m.navigations.length === 0);
  m.handle.dispose();
}

const CASES = { 'dash-054': case054, 'dash-055': case055, 'dash-056': case056, 'dash-057': case057, 'dash-058': case058, 'dash-059': case059, 'dash-060': case060, 'dash-061': case061, 'dash-063': case063, 'dash-064': case064 };
const caseName = params.get('case');
try {
  if (caseName?.startsWith('shot-')) await shot(caseName.slice(5));
  else if (CASES[caseName]) await CASES[caseName]();
  else failures.push(`unknown case: ${caseName}`);
} catch (error) {
  failures.push(`${caseName} threw: ${error?.message ?? error}`);
} finally {
  if (!caseName?.startsWith('shot-')) current?.handle.dispose();
}
check('no CSP violations', cspViolations === 0);
result.dataset.result = failures.length === 0 ? 'pass' : 'fail';
result.dataset.failures = failures.join('\n');
result.dataset.cspViolations = String(cspViolations);
