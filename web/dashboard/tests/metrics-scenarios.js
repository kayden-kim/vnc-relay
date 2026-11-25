// Contract-shaped fake API evidence only. Real server coverage belongs to dashboard_live.
// One page load runs one case: ?case=dash-069&lang=ko|en, or ?case=shot-<state>&lang=ko|en for the F-C screens.
import { mountMetrics } from '../app/metrics.js';
import { COPY, BRAND_NAME } from '../app/copy.js';
import { STORAGE_KEY } from '../app/language.js';
import { LOGIN_URL } from '../app/shell.js';
import {
  fill,
  formatBytesRate,
  formatBytesValue,
  formatClock,
  formatCount,
  formatDecimal,
  formatDuration,
  formatPercent,
  formatShortSeconds,
} from '../app/format.js';
import { axisOf, niceCeil } from '../app/metrics-model.js';
import { checkHelpLanguageChange } from './help-link-checks.js';
import {
  DURATION_BOUNDS, T0, buckets, defaultMetrics, histogram, iso, metricsBody, points as pointsOf, scalar,
} from './metrics-fixture.js';

const host = document.getElementById('host');
const result = document.getElementById('result');
const params = new URLSearchParams(location.search);
const lang = params.get('lang') === 'en' ? 'en' : 'ko';
const t = COPY[lang];
const failures = [];
let cspViolations = 0;
document.addEventListener('securitypolicyviolation', () => { cspViolations += 1; });
const check = (label, condition) => { if (!condition) failures.push(label); };
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
const fail = (status) => json(status, {
  error: { code: status === 401 ? 'unauthenticated' : 'invalid_request', message: 'error' },
});
const body = (points = pointsOf(1, 3), metrics = defaultMetrics()) => metricsBody({ metrics, points });
const okBody = (...args) => ok(body(...args));
// The last 24 hours: points one minute apart in a body with that window's interval and capacity.
const dayPoints = (from, to, overrides = {}) => pointsOf(from, to, overrides, 60);
const dayBody = (retained = dayPoints(1, 3), metrics = defaultMetrics()) => ok(metricsBody({ metrics, points: retained, period: '24h' }));
const isDay = (call) => call.query.get('history_window') === '24h';

// Fake management API. `handler(call, n)` answers the n-th metrics request. `maxInflight` is the largest number of
// metrics requests that were open at the same moment.
function fakeApi(handler = () => okBody(), { pollSecs = 3600, timeoutSecs = 5 } = {}) {
  const calls = [];
  let inflight = 0;
  const api = { calls, maxInflight: 0 };
  api.request = (path, init = {}) => {
    const url = new URL(path, location.origin);
    if (url.pathname === '/api/v1/session') {
      return Promise.resolve(ok({
        csrf_token: 'fake-csrf-token',
        dashboard: { poll_secs: pollSecs, query_timeout_secs: timeoutSecs, terminate_timeout_secs: 1 },
      }));
    }
    if (url.pathname !== '/api/v1/metrics') throw new Error(`unexpected path ${path}`);
    const call = { path, init, query: url.searchParams, at: performance.now() };
    calls.push(call);
    inflight += 1;
    api.maxInflight = Math.max(api.maxInflight, inflight);
    return Promise.resolve(handler(call, calls.length)).finally(() => { inflight -= 1; });
  };
  return api;
}
// A request that never answers until the page aborts it, like a stalled server.
const stalled = (call) => new Promise((_, reject) => {
  call.init.signal.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')));
});

const saved = [];
const storage = () => ({ getItem: (key) => (key === STORAGE_KEY ? lang : null), setItem: (key, value) => saved.push([key, value]) });
let current;
function mount(api = fakeApi()) {
  current?.handle.dispose();
  const navigations = [];
  const handle = mountMetrics(host, {
    request: api.request, storage: storage(), preferredLanguages: [lang], navigate: (url) => navigations.push(url),
  });
  current = { handle, api, navigations };
  return current;
}

const settled = () => host.querySelector('#metrics-refresh')?.disabled === false;
const status = () => host.querySelector('#metrics-status')?.textContent.trim() ?? null;
const refreshLabel = () => host.querySelector('#metrics-refresh .button-label')?.textContent.trim() ?? null;
const timeText = () => host.querySelector('#metrics-time')?.textContent ?? '';
const kpi = (key) => host.querySelector(`[data-kpi="${key}"]`);
const metricOf = (key) => kpi(key)?.querySelector('.metric')?.textContent ?? null;
const chart = (id) => host.querySelector(`figure[data-chart="${id}"]`);
const chartIds = () => [...host.querySelectorAll('figure[data-chart]')].map((figure) => figure.dataset.chart);
const emptyCharts = () => host.querySelectorAll('[data-chart-card] .chart-empty').length;
const card = (name) => host.querySelector(`[data-metrics-card="${name}"]`);
const cause = (id) => host.querySelector(`[data-cause="${id}"]`);
const isPoll = (call) => call.init.headers?.['X-Relay-Poll'] === '1';
const perSecond = (value) => fill(t.perSecond, { value });
const rowsOf = (root) => [...root.querySelectorAll('tbody tr')];
const cellsOf = (row) => [...row.children].map((cell) => cell.textContent.trim());
const periodSelect = () => host.querySelector('#metrics-period');
const rangeText = () => host.querySelector('#metrics-range')?.textContent ?? '';
const trendsTitle = () => [...host.querySelectorAll('.metrics-section > h2')][1]?.textContent ?? null;
// Chooses a trend period the way a keyboard or pointer user does: focus, new value, change event.
function choosePeriod(value) {
  const select = periodSelect();
  select.focus();
  select.value = value;
  select.dispatchEvent(new Event('change'));
}

function checkLayout(label) {
  check(`${label}: no horizontal scroll`, document.documentElement.scrollWidth <= document.documentElement.clientWidth);
  const links = host.querySelectorAll('.navbar-nav .navbar-item');
  check(`${label}: five navigation items, metrics current`,
    links.length === 5 && links[1].getAttribute('aria-current') === 'page'
    && [...links].filter((link) => link.hasAttribute('aria-current')).length === 1);
  for (const box of [...host.querySelectorAll('section.card')].map((node) => node.getBoundingClientRect())) {
    if (box.width) check(`${label}: card inside viewport`, box.left >= 0 && box.right <= innerWidth + 1);
  }
  for (const control of host.querySelectorAll('button, select, a')) {
    const box = control.getBoundingClientRect();
    if (box.width && box.height) check(`${label}: control inside viewport`, box.left >= 0 && box.right <= innerWidth + 1);
  }
  for (const node of host.querySelectorAll('.table-scroll, .kpi-body, [data-chart-card] .card-body')) {
    check(`${label}: no inner horizontal overflow`, node.scrollWidth <= node.clientWidth + 1);
  }
  const period = host.querySelector('#metrics-period')?.getBoundingClientRect();
  const refresh = host.querySelector('#metrics-refresh')?.getBoundingClientRect();
  if (period && refresh && innerWidth <= 1023.98) {
    check(`${label}: period select spans the row above refresh at the touch height`, period.bottom <= refresh.top
      && Math.abs(period.width - refresh.width) < 1 && period.height >= 44);
  } else if (period && refresh) {
    check(`${label}: period select left of refresh with aligned bottoms`, period.right <= refresh.left
      && Math.abs(period.bottom - refresh.bottom) < 1);
  }
  if (innerWidth <= 1023.98) {
    const button = host.querySelector('#metrics-refresh');
    if (button) check(`${label}: refresh uses the touch height`, button.getBoundingClientRect().height >= 44);
    const lefts = [...host.querySelectorAll('[data-chart-card], [data-cause]')].map((node) => Math.round(node.getBoundingClientRect().left));
    check(`${label}: cards stack in one column`, lefts.length === 0 || lefts.every((left) => left === lefts[0]));
    const nav = host.querySelector('.navbar-nav');
    const tools = host.querySelector('.navbar-tools');
    check(`${label}: mobile navigation below the tools`, nav.getBoundingClientRect().top >= tools.getBoundingClientRect().bottom);
  }
}

// ---- DASH-028, 064, 068 ----

async function case028() {
  const m = mount();
  await waitFor(settled);
  const links = [...host.querySelectorAll('.navbar-nav .navbar-item')];
  check('028: five items in the adopted order',
    links.map((link) => link.textContent.trim()).join('|') === [t.navOperations, t.metrics, t.sources, t.logs, t.settings].join('|'));
  check('028: item destinations', links.map((link) => link.getAttribute('href')).join('|') === '/|/#/metrics|/#/sources|/#/logs|/#/settings');
  check('028: only the metrics item is current', links[1].getAttribute('aria-current') === 'page'
    && links.filter((link) => link.hasAttribute('aria-current')).length === 1);
  check('028: every item is visible', links.every((link) => link.getBoundingClientRect().width > 0 && link.getBoundingClientRect().height > 0));
  check('028: protected data shown with a session', kpi('active') !== null);
  checkLayout('028');
  m.handle.dispose();
  check('028: dispose removes the protected data', host.childElementCount === 0);

  const denied = mount(fakeApi(() => fail(401)));
  await waitFor(() => denied.navigations.length > 0);
  check('028: no session goes to the login page', denied.navigations.join('|') === LOGIN_URL);
  check('028: no protected data without a session', host.childElementCount === 0 && kpi('active') === null);
}

async function case064() {
  const m = mount();
  await waitFor(settled);
  const before = m.api.calls.length;
  checkHelpLanguageChange(host, lang, check);
  check('064: no help API requests', m.api.calls.length === before);
  check('064: current screen remains mounted', m.navigations.length === 0 && kpi('active') !== null);
}

async function case068() {
  const gate = deferred();
  const m = mount(fakeApi((call, n) => (n === 2 ? gate.promise : okBody(pointsOf(1, 5))), { pollSecs: 1 }));
  await waitFor(settled);
  check('068: metrics item is current', host.querySelectorAll('.navbar-item')[1]?.getAttribute('aria-current') === 'page');
  check('068: first request has no history_after and no X-Relay-Poll',
    m.api.calls[0].path === '/api/v1/metrics' && !m.api.calls[0].init.headers?.['X-Relay-Poll']);
  await waitFor(() => m.api.calls.length === 2, 3000);
  check('068: automatic poll carries X-Relay-Poll and the largest seq',
    isPoll(m.api.calls[1]) && m.api.calls[1].query.get('history_after') === '5');
  m.handle.dispose();
  gate.resolve(okBody(pointsOf(1, 9)));
  await wait(300);
  check('068: a late response draws nothing after leaving', host.childElementCount === 0);
  await wait(1500);
  check('068: leaving stops the automatic polls', m.api.calls.length === 2);
}

// ---- DASH-069: key indicators ----

async function case069() {
  mount();
  await waitFor(settled);
  check('069: eight cards in order',
    [...host.querySelectorAll('[data-kpi]')].map((node) => node.dataset.kpi).join('|') === 'active|waiting|relay|admin|throughput|rejected|cpu|memory');
  check('069: active is the sum of all points', metricOf('active') === '3');
  check('069: waiting is the sum of all points', metricOf('waiting') === '4');
  check('069: rejected is the sum of all points', metricOf('rejected') === '8');
  for (const [key, now, max, limit] of [['relay', '5', '100', '100'], ['admin', '1', '10', '10']]) {
    const gauge = kpi(key)?.querySelector('[role="progressbar"]');
    check(`069: ${key} gauge value and limit`, gauge?.getAttribute('aria-valuenow') === now && gauge.getAttribute('aria-valuemax') === max
      && kpi(key).textContent.includes(fill(t.limit, { n: limit })));
  }
  check('069: connection card descriptions', kpi('relay').textContent.includes(t.relayNote) && kpi('admin').textContent.includes(t.adminNote));
  const names = [...kpi('throughput').querySelectorAll('dt')].map((node) => node.textContent.trim());
  const values = [...kpi('throughput').querySelectorAll('dd .metric')].map((node) => node.textContent);
  check('069: throughput names the two directions', names.join('|') === [t.directionViewerToTarget, t.directionTargetToViewer].join('|'));
  check('069: throughput is the per-second change of the latest neighbouring pair',
    values.join('|') === [perSecond(formatBytesRate(1024)), perSecond(formatBytesRate(2048))].join('|'));
  check('069: throughput marks are decorative', [...kpi('throughput').querySelectorAll('.direction-mark')].every((mark) => mark.getAttribute('aria-hidden') === 'true'));
  check('069: cpu is the CPU time change as a percentage', metricOf('cpu') === formatPercent(10));
  check('069: memory value', metricOf('memory') === formatBytesValue(52428800));
  check('069: cpu and memory descriptions', kpi('cpu').textContent.includes(t.cpuNote) && kpi('memory').textContent.includes(t.memoryNote));
  const note = host.querySelector('.metrics-main > .alert');
  check('069: external collection note, always the same text', !!note && note.textContent.trim() === t.externalNote
    && !!note.querySelector('.mono'));
  check('069: the note is not a live region', !note?.hasAttribute('role') && !note?.querySelector('[aria-live]'));
  check('069: section titles', [...host.querySelectorAll('.metrics-section > h2')].map((node) => node.textContent).join('|')
    === [t.kpiTitle, t.trendsTitle, t.causeTitle].join('|'));
  check('069: no h6', host.querySelector('h6') === null);
  checkLayout('069');
}

// ---- DASH-070: trend charts ----

const coords = (path) => path.getAttribute('d').slice(1).split('L').map((pair) => pair.split(' ').map(Number));
// Name and value text of each legend entry (the line-shape swatch is an svg and has no text).
const legendOf = (id) => [...chart(id).querySelectorAll('.chart-legend li')]
  .map((li) => [...li.children].filter((node) => node.tagName.toLowerCase() !== 'svg').map((node) => node.textContent.trim()));

async function case070() {
  const gate = deferred();
  const m = mount(fakeApi((call, n) => {
    if (n === 1) return okBody(pointsOf(1, 359));
    if (n === 2) return okBody(pointsOf(360, 360));
    return gate.promise;
  }, { pollSecs: 1 }));
  await waitFor(settled);
  check('070: five charts', chartIds().join('|') === 'sessions|connections|throughput|cpu|memory');
  check('070: range line counts the kept points', host.querySelector('#metrics-range')?.textContent === fill(t.metricsRange, { n: 359 }));
  const latest = T0 + 3590000;
  const figure = chart('sessions');
  check('070: five time labels from one hour back to the newest point',
    [...figure.querySelectorAll('.chart-x span')].map((node) => node.textContent).join('|')
    === [0, 15, 30, 45, 60].map((minutes) => formatClock(latest - 3600000 + minutes * 60000)).join('|'));
  const line = figure.querySelector('.chart-plot path.chart-line.s1');
  const points = coords(line);
  check('070: newest point at the right edge, constant value on one level',
    points.at(-1)[0] === 360 && points.every(([, y]) => y === 25) && points.every(([x], i) => i === 0 || x > points[i - 1][0]));
  check('070: chart svg geometry', figure.querySelector('svg.chart-plot')?.getAttribute('viewBox') === '0 0 360 100'
    && figure.querySelector('svg.chart-plot').getAttribute('preserveAspectRatio') === 'none'
    && [...figure.querySelectorAll('svg.chart-plot path')].every((path) => path.getAttribute('vector-effect') === 'non-scaling-stroke'));
  check('070: series differ by dash pattern', chart('connections').querySelector('.chart-plot path.chart-line.s2')?.getAttribute('stroke-dasharray') === '6 4'
    && chart('connections').querySelector('.chart-plot path.chart-line.s3')?.getAttribute('stroke-dasharray') === '1 5'
    && !chart('connections').querySelector('.chart-plot path.chart-line.s1').hasAttribute('stroke-dasharray'));
  const legend = (current, max) => fill(t.legendValue, { current, max });
  check('070: sessions legend', legendOf('sessions').map((row) => row.join(' ')).join('|')
    === `${t.activeCount} ${legend('3', '3')}|${t.waitingCount} ${legend('1', '1')}`);
  check('070: connection legend is per minute from groups of six pairs', legendOf('connections').map((row) => row.join(' ')).join('|')
    === t.connectionSeries.map((name, i) => `${name} ${legend(...[formatDecimal([12, 2, 0][i]), formatDecimal([12, 2, 0][i])])}`).join('|'));
  check('070: throughput legend', legendOf('throughput')[0][1] === legend(perSecond(formatBytesRate(1024)), perSecond(formatBytesRate(1024))));
  check('070: cpu legend', legendOf('cpu')[0][1] === legend(formatPercent(10), formatPercent(10)));
  check('070: memory legend', legendOf('memory')[0][1] === legend(formatBytesValue(52428800), formatBytesValue(52428800)));
  check('070: legend swatch and name per series', [...chart('connections').querySelectorAll('.chart-legend li')]
    .every((li) => li.querySelector('svg.chart-swatch path') && li.children.length === 3));
  check('070: chart name holds title and legend text', chart('sessions').getAttribute('role') === 'img'
    && chart('sessions').getAttribute('aria-label') === [t.chartSessions, `${t.activeCount}: ${legend('3', '3')}`, `${t.waitingCount}: ${legend('1', '1')}`].join('. '));
  await waitFor(() => host.querySelector('#metrics-range')?.textContent === fill(t.metricsRange, { n: 360 }), 4000);
  check('070: poll sends the largest kept seq', m.api.calls[1]?.query.get('history_after') === '359' && isPoll(m.api.calls[1]));
  check('070: range line follows the kept points', host.querySelector('#metrics-range')?.textContent === fill(t.metricsRange, { n: 360 }));
  await waitFor(() => m.api.calls.length === 3, 3000);
  check('070: next poll continues from seq 360', m.api.calls[2].query.get('history_after') === '360');
  gate.resolve(okBody(pointsOf(361, 361)));
  await wait(100);
  check('070: at capacity the oldest point leaves (still 360 kept)', host.querySelector('#metrics-range')?.textContent === fill(t.metricsRange, { n: 360 }));
  checkLayout('070');
}

async function case070Gaps() {
  // Sessions: null at seq 10-12 and seq 20 missing. A missing seq breaks every connection group that contains it.
  const retained = pointsOf(1, 40, (seq) => ({ session_count: seq >= 10 && seq <= 12 ? null : 3 })).filter((point) => point.seq !== 20);
  mount(fakeApi(() => okBody(retained)));
  await waitFor(settled);
  const lines = (id, series) => chart(id).querySelectorAll(`.chart-plot path.chart-line.s${series}`);
  check('070 gaps: null and a missing seq break the line (never drawn as zero)', lines('sessions', 1).length === 3);
  check('070 gaps: waiting breaks only at the missing seq', lines('sessions', 2).length === 2);
  check('070 gaps: a connection group with a missing seq is skipped for every series',
    [1, 2, 3].every((series) => lines('connections', series).length === 2));
  check('070 gaps: no run of the sessions chart touches the zero line',
    [...lines('sessions', 1)].every((path) => coords(path).every(([, y]) => y < 100)));
  checkLayout('070 gaps');

  // A null in one connection series breaks only that series.
  mount(fakeApi(() => okBody(pointsOf(1, 40, (seq) => ({ connection_accepted: seq === 20 ? null : seq * 2 })))));
  await waitFor(settled);
  check('070 null: only the series with the null is broken', lines('connections', 1).length === 2
    && lines('connections', 2).length === 1 && lines('connections', 3).length === 1);

  // An isolated value is one dot. Points 20 s apart still give a per-minute value.
  mount(fakeApi(() => okBody(pointsOf(1, 15, (seq) => ({ session_count: seq === 7 ? 3 : null })))));
  await waitFor(settled);
  check('070 dot: a run of one point is a dot', chart('sessions').querySelectorAll('path.chart-dot').length === 1);
  mount(fakeApi(() => okBody(pointsOf(1, 20).map((point) => ({ ...point, at: iso(T0 + point.seq * 20000) })))));
  await waitFor(settled);
  check('070 long gap: a group of 20 s points is still a per-minute value (12 per 120 s is 6.0)',
    legendOf('connections')[0][1] === fill(t.legendValue, { current: formatDecimal(6), max: formatDecimal(6) }));
}

// ---- DASH-071: distribution, cause tables, process ----

const bound = (value, over = false) => fill(over ? t.p95Gt : t.p95Le, { value: formatShortSeconds(value, t) });

async function case071() {
  mount(fakeApi(() => okBody([])));
  await waitFor(settled);
  const dist = card('distribution');
  const rows = rowsOf(dist);
  check('071: distribution card title and note', dist.querySelector('.card-heading')?.textContent === t.distTitle && dist.textContent.includes(t.distNote));
  check('071: distribution caption is available to assistive technology only', dist.querySelector('table caption.sr-only') !== null);
  check('071: distribution column headings', [...dist.querySelectorAll('thead th')].map((node) => node.textContent.trim()).join('|') === t.distColumns.join('|'));
  check('071: one row per item without records', rows.length === 4);
  const first = cellsOf(rows[0]);
  check('071: handshake row item, attributes without translation, count, mean, p95', first[0] === t.distNames[0]
    && first[1] === 'listener=ws, mode=mode2' && first[2] === '11' && first[3] === formatShortSeconds(0.1, t) && first[4] === bound(1));
  check('071: item without records is one row with the no-record text', cellsOf(rows[1]).join('|') === `${t.distNames[1]}|${t.noRecords}`
    && rows[1].querySelector('td.none')?.colSpan === 5);
  check('071: session duration p95 above the last bound', cellsOf(rows[2])[4] === bound(604800, true));
  check('071: target connect p95 inside the first bucket', cellsOf(rows[3])[4] === bound(0.005));
  const bar = rows[0].querySelector('svg.dist');
  const names = [[0.005, 3, false], [0.01, 2, false], [0.1, 5, false], [1, 1, false]]
    .map(([value, count, over]) => fill(t.distBarItem, { bound: bound(value, over), count: formatCount(count) }));
  check('071: distribution bar is an image named by its non-zero buckets', bar?.getAttribute('role') === 'img'
    && bar.getAttribute('aria-label') === fill(t.distBarName, { items: names.join(', ') }));
  const heights = [...bar.querySelectorAll('rect.dist-bar')].map((rect) => Number(rect.getAttribute('height')));
  check('071: bar height follows the largest bucket and zero buckets have no bar', heights.join('|') === '60|40|100|20');
  check('071: bars follow the bucket order', [...bar.querySelectorAll('rect.dist-bar')].map((rect) => rect.getAttribute('x')).join('|') === '1|11|41|71');

  const ids = [...host.querySelectorAll('[data-cause]')].map((node) => node.dataset.cause);
  check('071: six cause tables in order', ids.join('|') === 'rejected|aborted|closed|admin|config|websocket');
  check('071: cause table titles', [...host.querySelectorAll('[data-cause] .card-heading')].map((node) => node.textContent).join('|') === t.causeTables.join('|'));
  const rejected = rowsOf(cause('rejected'));
  check('071: cause rows by count, values untranslated', cellsOf(rejected[0]).slice(0, 4).join('|') === 'ws|upgrade|ticket_invalid|7'
    && cellsOf(rejected[1])[3] === '1');
  check('071: count cell holds a hidden bar and the number', rejected.every((row) => row.querySelector('.count-cell svg.count-bar')?.getAttribute('aria-hidden') === 'true'));
  const closed = rowsOf(cause('closed'));
  check('071: close causes translated, unknown value kept', cellsOf(closed[0])[1] === t.closeCause.peer_eof
    && cellsOf(closed[1])[1] === 'relay_reset' && cellsOf(closed[2])[1] === t.closeCause.connection_error);
  const admin = rowsOf(cause('admin'));
  check('071: admin items by count', cellsOf(admin[0]).join('|') === `${t.adminItems[0]}|succeeded|5`
    && cellsOf(admin[1])[0] === t.adminItems[2] && cellsOf(admin[2])[0] === t.adminItems[1]);
  check('071: config and websocket item names', cellsOf(rowsOf(cause('config'))[0])[0] === t.configItems[0]
    && cellsOf(rowsOf(cause('websocket'))[0])[0] === t.webSocketItems[1]);
  check('071: cause tables have a hidden caption and headings', [...host.querySelectorAll('[data-cause] table')].every((table) => table.querySelector('caption.sr-only') && table.querySelector('thead th')));

  const process = card('process');
  const labels = [...process.querySelectorAll('dt')].map((node) => node.textContent.trim());
  const values = [...process.querySelectorAll('dd')].map((node) => node.textContent.trim());
  check('071: Windows process rows', labels.join('|') === [t.uptime, t.cpuTime, t.memoryUsage, t.virtualMemory, t.threadCount, t.handleCount, t.version].join('|'));
  check('071: process values', values.join('|') === [
    formatDuration(93784000, t.dayUnit),
    `${formatShortSeconds(12.3, t)} / ${formatShortSeconds(4.5, t)}`,
    formatBytesValue(52428800), formatBytesValue(209715200), '17', '220', '0.9.0-test',
  ].join('|'));
  checkLayout('071');

  mount(fakeApi(() => okBody([], defaultMetrics({
    'vnc_relay.websocket.ping_timeouts': null,
    'vnc_relay.ticket.accepted': null,
    'process.windows.handle.count': null,
    'process.unix.file_descriptor.count': scalar('process.unix.file_descriptor.count', 'updowncounter', '{file_descriptor}', [[{}, 64]]),
  }))));
  await waitFor(settled);
  const empty = rowsOf(cause('websocket'));
  check('071: an empty cause table is one no-record row without a bar', empty.length === 1 && empty[0].textContent.trim() === t.noRecords
    && cause('websocket').querySelector('.count-bar') === null);
  const linux = [...card('process').querySelectorAll('dt')].map((node) => node.textContent.trim());
  check('071: Linux process rows show file descriptors, not handles', linux.includes(t.descriptorCount) && !linux.includes(t.handleCount));
}

// ---- DASH-072: collecting and unavailable ----

async function caseAxis() {
  // DESIGN.md 선 차트: the top is the smallest of 2, 4, 6, 8, 10 × 10^k at or above the largest visible value, 2 when
  // no value is visible or all are 0.
  const ceil = [[0, 2], [-1, 2], [Number.NaN, 2], [0.03, 0.04], [0.3, 0.4], [1, 2], [2, 2], [3, 4], [5, 6], [7, 8], [8, 8], [9, 10],
    [10, 10], [11, 20], [100, 100], [101, 200], [1000, 1000]];
  check('axis: niceCeil follows 2, 4, 6, 8, 10 × 10^k', ceil.every(([value, top]) => niceCeil(value) === top));
  const kib = 1024;
  const mib = 1048576;
  check('axis: bytes use KiB while the KiB top is below 1024', axisOf('bytes', 0).top === 2 * kib && axisOf('bytes', 0).unit === 'KiB'
    && axisOf('bytes', 1000 * kib).top === 1000 * kib && axisOf('bytes', 1000 * kib).unit === 'KiB'
    && axisOf('bytes', 700 * kib).top === 800 * kib);
  check('axis: bytes switch to MiB when the KiB top reaches 1024', axisOf('bytes', 1010 * kib).unit === 'MiB'
    && axisOf('bytes', 1010 * kib).top === mib && axisOf('bytes', 7 * mib).top === 8 * mib && axisOf('bytes', 50 * mib).top === 60 * mib);
  check('axis: counts and rates without values top at 2', axisOf('count', null).top === 2 && axisOf('percent', 0).top === 2);

  const ticksOf = (id) => [...chart(id).querySelectorAll('.chart-y span')].map((node) => node.textContent);
  mount(fakeApi(() => okBody(pointsOf(1, 5, { session_count: 7, waiting_count: 1, process_cpu_time: 5 }))));
  await waitFor(settled);
  check('axis: a sessions maximum of 7 draws to 8', ticksOf('sessions').join('|') === [formatCount(8), formatCount(4), formatCount(0)].join('|'));
  check('axis: an all-zero cpu chart tops at 2', ticksOf('cpu').join('|') === [2, 1, 0].map(formatPercent).join('|'));
  check('axis: memory of 50 MiB draws to 60 MiB', ticksOf('memory').join('|') === [60, 30, 0].map((value) => `${formatDecimal(value)} MiB`).join('|'));
  mount(fakeApi(() => okBody(pointsOf(1, 5, (seq) => ({ process_cpu_time: seq * 0.03 })))));
  await waitFor(settled);
  check('axis: a cpu maximum of 0.3 tops at 0.4', ticksOf('cpu').join('|') === [0.4, 0.2, 0].map(formatPercent).join('|'));
}

async function case072() {
  for (const [count, drawn] of [[0, []], [1, []], [2, ['sessions', 'memory']], [12, ['sessions', 'throughput', 'cpu', 'memory']], [13, ['sessions', 'connections', 'throughput', 'cpu', 'memory']]]) {
    mount(fakeApi(() => okBody(count === 0 ? [] : pointsOf(1, count))));
    await waitFor(settled);
    check(`072 ${count} points: charts drawn`, chartIds().join('|') === drawn.join('|'));
    check(`072 ${count} points: the others say there are too few points`,
      emptyCharts() === 5 - drawn.length && [...host.querySelectorAll('[data-chart-card] .chart-empty')].every((node) => node.textContent === t.chartEmpty));
    if (count < 2) {
      check(`072 ${count} points: throughput and cpu are collecting, no zero`,
        kpi('throughput').textContent.includes(t.collecting) && kpi('cpu').textContent.includes(t.collecting)
        && kpi('throughput').querySelector('.metric') === null && metricOf('cpu') === null);
    } else {
      check(`072 ${count} points: neighbouring pair gives rate cards`, metricOf('cpu') === formatPercent(10));
    }
    if (count === 13) check('072 13 points: connections joins two groups', coords(chart('connections').querySelector('.chart-plot path.chart-line.s1')).length === 2);
    check(`072 ${count} points: the other cards still show values`, metricOf('active') === '3');
  }
  mount(fakeApi(() => okBody(pointsOf(1, 5, { process_memory_usage: null }), defaultMetrics({
    'process.memory.usage': scalar('process.memory.usage', 'updowncounter', 'By', [], false),
    'process.uptime': scalar('process.uptime', 'gauge', 's', [], false),
  }))));
  await waitFor(settled);
  check('072 unavailable: memory card is unavailable, never zero', kpi('memory').textContent.includes(t.unavailable) && metricOf('memory') === null);
  const process = [...card('process').querySelectorAll('dd')].map((node) => node.textContent.trim());
  check('072 unavailable: unavailable process values say so, others stay', process[0] === t.unavailable && process[2] === t.unavailable && process[4] === '17');
  const memory = chart('memory');
  check('072 unavailable: memory chart has no line and no too-few-points text',
    !!memory && memory.querySelector('.chart-plot path.chart-line') === null && !memory.closest('[data-chart-card]').textContent.includes(t.chartEmpty));
  check('072 unavailable: memory legend says unavailable', legendOf('memory')[0][1] === fill(t.legendValue, { current: t.unavailable, max: t.unavailable }));
  check('072 unavailable: other charts keep their lines', chart('sessions').querySelector('.chart-plot path.chart-line') !== null);
}

// ---- DASH-073..075, 077: failure, retry, rejection, timing ----

async function case073() {
  const gate = deferred();
  mount(fakeApi((call, n) => (n === 1 ? gate.promise : okBody())));
  check('073 loading: loading badge', status() === t.loading);
  check('073 loading: unknown time, no value or chart', timeText().includes(t.unavailable) && host.querySelectorAll('.metric').length === 0
    && host.querySelector('figure') === null && host.querySelector('#metrics-range').hidden);
  check('073 loading: refresh stays labelled and disabled', refreshLabel() === t.refresh && host.querySelector('#metrics-refresh').disabled);
  gate.resolve(fail(500));
  await waitFor(settled);
  check('073 first failure: destructive badge and message', status() === t.failed && host.querySelector('.metrics-alerts .alert')?.textContent.includes(t.failedMessage));
  check('073 first failure: no zero and no empty chart', host.querySelector('[data-kpi]') === null && host.querySelector('figure') === null);
  check('073 first failure: time unavailable', timeText().includes(t.unavailable));

  for (const [name, handler] of [['network', () => Promise.reject(new TypeError('network'))], ['timeout', stalled]]) {
    const m = mount(fakeApi(handler, { timeoutSecs: 1 }));
    await waitFor(settled, 3000);
    check(`073 ${name}: first failure`, status() === t.failed && m.api.calls.length === 1);
  }

  const m = mount(fakeApi((call, n) => (n === 1 ? okBody() : fail(500)), { pollSecs: 1 }));
  await waitFor(settled);
  const before = { time: timeText(), range: host.querySelector('#metrics-range').textContent, figures: chartIds().join('|') };
  await waitFor(() => status() === t.stale, 4000);
  check('073 stale: warning badge and CPY-020 message', status() === t.stale
    && host.querySelector('.metrics-alerts .alert')?.textContent.includes(t.staleMessage));
  check('073 stale: values, trend and time are kept', metricOf('active') === '3' && timeText() === before.time
    && host.querySelector('#metrics-range').textContent === before.range && chartIds().join('|') === before.figures);
  check('073 stale: automatic polls continue', m.api.calls.length >= 2);
}

async function case074() {
  const m = mount(fakeApi((call, n) => (n === 2 ? fail(500) : okBody(pointsOf(1, n === 1 ? 3 : 6))), { pollSecs: 3600 }));
  await waitFor(settled);
  host.querySelector('#metrics-refresh').click();
  await waitFor(() => m.api.calls.length === 2 && settled());
  check('074: failure after refresh shows stale', status() === t.stale && host.querySelector('.metrics-alerts .alert') !== null);
  host.querySelector('#metrics-alert-refresh').click();
  await waitFor(() => m.api.calls.length === 3 && settled() && host.querySelector('.metrics-alerts').childElementCount === 0);
  const call = m.api.calls[2];
  check('074: refresh sends no history_after and no X-Relay-Poll', !isPoll(call) && !call.query.has('history_after') && call.path === '/api/v1/metrics');
  check('074: success clears the failure and replaces the trend', status() === '' && host.querySelector('.metrics-alerts').childElementCount === 0
    && host.querySelector('#metrics-range').textContent === fill(t.metricsRange, { n: 6 }) && !timeText().includes(t.unavailable));
  check('074: the focus returns to the refresh button', document.activeElement?.id === 'metrics-refresh');

  const p = mount(fakeApi((call, n) => (n === 2 ? fail(500) : okBody(pointsOf(1, 3))), { pollSecs: 1 }));
  await waitFor(() => p.api.calls.length >= 3 && settled() && host.querySelector('.metrics-alerts').childElementCount === 0, 5000);
  check('074: a later automatic poll succeeds after the failure', isPoll(p.api.calls[2]) && p.api.calls[2].query.get('history_after') === '3');
  check('074: success clears the failure', status() === '' && host.querySelector('.metrics-alerts').childElementCount === 0);
}

async function case075() {
  for (const code of [400, 405]) {
    const m = mount(fakeApi((call, n) => (n === 2 || n === 4 ? fail(code) : okBody(pointsOf(1, 3 + n))), { pollSecs: 1 }));
    await waitFor(() => status() === t.failed, 4000);
    check(`075 ${code}: destructive badge and the rejected message`, status() === t.failed
      && host.querySelector('.metrics-alerts .alert')?.textContent.includes(t.metricsRejected));
    check(`075 ${code}: values and time are kept`, metricOf('active') === '3' && !timeText().includes(t.unavailable));
    await wait(2500);
    check(`075 ${code}: the same request is not repeated`, m.api.calls.length === 2);
    host.querySelector('#metrics-refresh').click();
    await waitFor(() => m.api.calls.length === 3 && settled());
    check(`075 ${code}: refresh has no history_after and no X-Relay-Poll`, !isPoll(m.api.calls[2]) && !m.api.calls[2].query.has('history_after'));
    check(`075 ${code}: success clears the failure`, status() === '' && host.querySelector('.metrics-alerts').childElementCount === 0);
    await waitFor(() => m.api.calls.length === 4 && isPoll(m.api.calls[3]), 3000);
    check(`075 ${code}: automatic polls start again`, isPoll(m.api.calls[3]) && m.api.calls[3].query.get('history_after') === '6');
    await waitFor(() => status() === t.failed && settled(), 3000);
    host.querySelector('#metrics-refresh').click();
    await wait(300);
    check(`075 ${code}: a refresh that succeeds after a new rejection restarts the polls`, m.api.calls.length >= 5);
  }
}

async function case076() {
  const m = mount(fakeApi((call, n) => (n === 2 ? fail(401) : okBody()), { pollSecs: 1 }));
  await waitFor(settled);
  await waitFor(() => m.navigations.length > 0, 4000);
  check('076: 401 goes to the login page', m.navigations.join('|') === LOGIN_URL);
  check('076: values, trend and the screen are removed', host.childElementCount === 0 && kpi('active') === null);
  await wait(1500);
  check('076: no further requests', m.api.calls.length === 2);
}

async function case077() {
  const fast = mount(fakeApi((call, n) => (n === 1 ? okBody() : wait(300).then(() => okBody())), { pollSecs: 1 }));
  await waitFor(settled);
  let busySeen = false;
  for (let steps = 130; steps > 0; steps -= 1) {
    busySeen ||= status() !== '' || refreshLabel() !== t.refresh;
    await wait(20);
  }
  check('077 fast: two automatic polls ran', fast.api.calls.filter(isPoll).length >= 2);
  check('077 fast: badge and refresh label do not change under 1 s', !busySeen);

  const slow = mount(fakeApi((call, n) => (n === 2 ? wait(1600).then(() => okBody()) : okBody()), { pollSecs: 1 }));
  await waitFor(settled);
  await waitFor(() => slow.api.calls.length === 2, 3000);
  const startedAt = slow.api.calls[1].at;
  await wait(500);
  check('077 slow: unchanged 0.5 s into the poll', status() === '' && refreshLabel() === t.refresh);
  await waitFor(() => status() === t.refreshing, 1500);
  const shownAfter = performance.now() - startedAt;
  check('077 slow: updating after 1 s on badge and button', status() === t.refreshing && refreshLabel() === t.refreshing && host.querySelector('#metrics-refresh').disabled);
  check('077 slow: updating waits 1 s', shownAfter >= 980 && shownAfter < 1300);
  await waitFor(() => status() === '', 2000);
  check('077 slow: updating removed when the poll settles', status() === '' && refreshLabel() === t.refresh);
  check('077: metrics requests never overlap (fast and slow)', fast.api.maxInflight === 1 && slow.api.maxInflight === 1);
}

// Split from case077: dump-dom stops after 10 s of virtual time (tests/dashboard_browser.rs).
async function case077More() {
  const quick = mount(fakeApi((call, n) => (n === 1 ? okBody() : n === 2 ? fail(500) : wait(300).then(() => fail(500))), { pollSecs: 1 }));
  await waitFor(() => status() === t.stale, 3000);
  let flipped = false;
  const watchUntil = performance.now() + 2200;
  while (performance.now() < watchUntil) { flipped ||= status() === t.refreshing; await wait(20); }
  check('077 stale: a retry that fails within 1 s keeps the stale badge', !flipped && status() === t.stale && quick.api.calls.length >= 3);

  const gate = deferred();
  const user = mount(fakeApi((call, n) => (n === 2 ? gate.promise : okBody())));
  await waitFor(settled);
  host.querySelector('#metrics-refresh').click();
  check('077 user refresh: updating at once', status() === t.refreshing && refreshLabel() === t.refreshing && host.querySelector('#metrics-refresh').disabled);
  gate.resolve(okBody());
  await waitFor(settled);
  check('077 user refresh: settled', status() === '' && refreshLabel() === t.refresh && user.api.calls.length === 2);

  const late = deferred();
  const poll = mount(fakeApi((call, n) => (n === 2 ? late.promise : okBody()), { pollSecs: 1 }));
  await waitFor(settled);
  await waitFor(() => poll.api.calls.length === 2, 3000);
  check('077 refresh during a poll: nothing shown yet', status() === '');
  host.querySelector('#metrics-refresh').click();
  check('077 refresh during a poll: updating at once, no new request', status() === t.refreshing && poll.api.calls.length === 2);
  late.resolve(okBody());
  await waitFor(() => status() === '' && settled(), 2000);
  await wait(1300);
  check('077: metrics requests never overlap (stale, user and poll)', quick.api.maxInflight === 1 && poll.api.maxInflight === 1 && user.api.maxInflight === 1);
}

// ---- DASH-078: layout and accessibility ----

async function case078() {
  mount();
  await waitFor(settled);
  checkLayout('078 normal');
  for (const figure of host.querySelectorAll('figure[data-chart]')) {
    check(`078: ${figure.dataset.chart} chart is an image with title and legend in its name`, figure.getAttribute('role') === 'img'
      && figure.getAttribute('aria-label').split('. ')[0].length > 0 && figure.getAttribute('aria-label').includes(t.legendValue.split('{')[0]));
    check(`078: ${figure.dataset.chart} chart has no tab stop`, figure.querySelector('[tabindex], a, button') === null && figure.getAttribute('tabindex') === null);
  }
  check('078: series are told apart by shape as well as colour', [...host.querySelectorAll('.chart-legend li')].every((li) => li.querySelector('svg.chart-swatch path')));
  check('078: distribution bars are images', [...host.querySelectorAll('svg.dist')].every((bar) => bar.getAttribute('role') === 'img' && bar.hasAttribute('aria-label')));
  check('078: count bars are hidden from assistive technology', [...host.querySelectorAll('svg.count-bar')].every((bar) => bar.getAttribute('aria-hidden') === 'true'));
  check('078: tables have a caption and column headings', [...host.querySelectorAll('table')].every((table) => table.querySelector('caption') && table.querySelector('thead th')));
  if (innerWidth <= 1023.98) {
    check('078: mobile cells carry their column name', [...host.querySelectorAll('[data-cause] tbody td')].every((cell) => cell.hasAttribute('data-label')));
  }
  const live = [...host.querySelectorAll('[aria-live], [role="status"], [role="alert"]')];
  check('078: only the status cell is a live region', live.length === 1 && live[0].id === 'metrics-status');
  for (const selector of ['.navbar-item', '.help-link', '#metrics-language', '#metrics-logout', '#metrics-period', '#metrics-refresh']) {
    const control = host.querySelector(selector);
    control.focus();
    check(`078: keyboard reaches ${selector}`, document.activeElement === control);
  }
  const periodLabel = host.querySelector('label[for="metrics-period"]');
  check('078: the period select has a visible label', periodLabel?.textContent === t.periodLabel
    && periodLabel.getBoundingClientRect().width > 0 && periodLabel.getBoundingClientRect().height > 0);

  mount(fakeApi(() => okBody([])));
  await waitFor(settled);
  checkLayout('078 collecting');
  check('078 collecting: the five chart places are text', emptyCharts() === 5);

  mount(fakeApi(() => fail(500)));
  await waitFor(settled);
  checkLayout('078 failed');
  const live2 = [...host.querySelectorAll('[aria-live], [role="status"], [role="alert"]')];
  check('078 failed: the failure notice is announced once', live2.filter((node) => node.getAttribute('role') === 'alert').length === 1);
  host.querySelector('#metrics-alert-refresh').focus();
  check('078 failed: the alert refresh is reachable', document.activeElement.id === 'metrics-alert-refresh');
}

// ---- DASH-079: language ----

async function case079() {
  const m = mount();
  await waitFor(settled);
  const next = lang === 'ko' ? 'en' : 'ko';
  const n = COPY[next];
  const time = timeText().split(': ').slice(1).join(': ');
  const charts = chartIds().join('|');
  const select = host.querySelector('#metrics-language');
  select.focus();
  select.value = next;
  select.dispatchEvent(new Event('change'));
  check('079: no new request', m.api.calls.length === 1);
  check('079: document language and title', document.documentElement.lang === next && document.title === `${BRAND_NAME} ${n.metrics}`);
  check('079: the choice is saved', saved.some(([, value]) => value === next));
  check('079: title and card names', host.querySelector('h1').textContent === n.metrics && kpi('active').textContent.includes(n.activeCount));
  check('079: unit wording', [...kpi('throughput').querySelectorAll('dd .metric')].map((node) => node.textContent).join('|')
    === [fill(n.perSecond, { value: formatBytesRate(1024) }), fill(n.perSecond, { value: formatBytesRate(2048) })].join('|'));
  check('079: day unit', [...card('process').querySelectorAll('dd')][0].textContent.trim() === formatDuration(93784000, n.dayUnit));
  check('079: p95 wording and day unit', cellsOf(rowsOf(card('distribution'))[2])[4] === fill(n.p95Gt, { value: formatShortSeconds(604800, n) }));
  check('079: close cause translated, attribute values and numbers are not', cellsOf(rowsOf(cause('closed'))[0])[1] === n.closeCause.peer_eof
    && cellsOf(rowsOf(cause('rejected'))[0])[2] === 'ticket_invalid' && cellsOf(rowsOf(card('distribution'))[0])[1] === 'listener=ws, mode=mode2');
  check('079: time, values, and trend stay', timeText().endsWith(time) && metricOf('active') === '3' && chartIds().join('|') === charts
    && host.querySelector('#metrics-range').textContent === fill(n.metricsRange, { n: 3 }));
  check('079: chart names change language', chart('sessions').getAttribute('aria-label').startsWith(n.chartSessions));
  check('079: language focus kept', document.activeElement?.id === 'metrics-language');

  // The chosen trend period stays too, and still without a request.
  const d = mount(fakeApi((call) => (isDay(call) ? dayBody() : okBody())));
  await waitFor(settled);
  choosePeriod('24h');
  await waitFor(() => rangeText() === fill(t.metricsRange24h, { n: 3 }));
  const calls = d.api.calls.length;
  const again = host.querySelector('#metrics-language');
  again.value = next;
  again.dispatchEvent(new Event('change'));
  check('079: the chosen period stays without a request', periodSelect().value === '24h' && d.api.calls.length === calls
    && rangeText() === fill(n.metricsRange24h, { n: 3 }) && trendsTitle() === n.trendsTitle24h
    && host.querySelector('label[for="metrics-period"]').textContent === n.periodLabel);
}

// ---- DASH-080, 081: trend period ----

async function case080() {
  // Last hour → last 24 hours → last hour. The 24-hour answer waits, so the screen in between can be read.
  const gate = deferred();
  const m = mount(fakeApi((call, n) => {
    if (n === 2) return gate.promise;
    return isDay(call) ? dayBody(dayPoints(1, 8)) : okBody(pointsOf(1, 5));
  }));
  await waitFor(settled);
  const select = periodSelect();
  const label = host.querySelector('label[for="metrics-period"]');
  check('080: the select has a visible label', label?.textContent === t.periodLabel && label.getBoundingClientRect().width > 0);
  check('080: two choices, the last hour first and chosen', [...select.options].map((option) => `${option.value}=${option.textContent}`).join('|')
    === `1h=${t.periodHour}|24h=${t.periodDay}` && select.value === '1h');
  const before = { time: timeText(), range: rangeText(), charts: chartIds().join('|') };
  choosePeriod('24h');
  check('080: updating shows at once', status() === t.refreshing && refreshLabel() === t.refreshing);
  check('080: the select stays usable and keeps the focus', !select.disabled && document.activeElement === select);
  const change = m.api.calls[1];
  check('080: the change asks for the last 24 hours without history_after and X-Relay-Poll',
    !!change && isDay(change) && !change.query.has('history_after') && !isPoll(change));
  check('080: until it succeeds the last hour stays', rangeText() === before.range && timeText() === before.time
    && trendsTitle() === t.trendsTitle && chartIds().join('|') === before.charts && metricOf('active') === '3');
  gate.resolve(dayBody(dayPoints(1, 8)));
  await waitFor(settled);
  check('080: success replaces the points with the last 24 hours', rangeText() === fill(t.metricsRange24h, { n: 8 })
    && trendsTitle() === t.trendsTitle24h && status() === '');
  check('080: the focus stays on the select', document.activeElement === select);
  choosePeriod('1h');
  await waitFor(() => m.api.calls.length === 3 && settled());
  const back = m.api.calls[2];
  check('080: back to the last hour asks without history_window, history_after, and X-Relay-Poll',
    back.path === '/api/v1/metrics' && !isPoll(back));
  check('080: the last hour shows again', rangeText() === fill(t.metricsRange, { n: 5 }) && trendsTitle() === t.trendsTitle);

  // A change during an automatic poll waits for it, drops its answer, and then asks for the chosen window.
  const late = deferred();
  const p = mount(fakeApi((call, n) => {
    if (n === 2) return late.promise;
    return isDay(call) ? dayBody(dayPoints(1, 8)) : okBody(pointsOf(1, 5));
  }, { pollSecs: 1 }));
  await waitFor(settled);
  await waitFor(() => p.api.calls.length === 2, 3000);
  check('080 during a poll: the poll runs without the updating state yet', isPoll(p.api.calls[1]) && status() === '');
  const ranges = [];
  const observer = new MutationObserver(() => ranges.push(rangeText()));
  observer.observe(host.querySelector('#metrics-range'), { childList: true, characterData: true, subtree: true });
  choosePeriod('24h');
  check('080 during a poll: updating at once, no second request', status() === t.refreshing && p.api.calls.length === 2);
  late.resolve(okBody(pointsOf(6, 9)));
  await waitFor(() => rangeText() === fill(t.metricsRange24h, { n: 8 }), 3000);
  observer.disconnect();
  const next = p.api.calls[2];
  check('080 during a poll: the chosen window is asked right after', !!next && isDay(next) && !next.query.has('history_after') && !isPoll(next));
  check('080 during a poll: the answer of the earlier window never shows', !ranges.includes(fill(t.metricsRange, { n: 9 })));
  check('080: metrics requests never overlap (change, poll)', m.api.maxInflight === 1 && p.api.maxInflight === 1);

  // A change that fails keeps the last hour with the stale notice. The next automatic poll asks for the chosen window.
  const f = mount(fakeApi((call, n) => {
    if (n === 2) return fail(500);
    return isDay(call) ? dayBody(dayPoints(1, 8)) : okBody(pointsOf(1, 5));
  }, { pollSecs: 1 }));
  await waitFor(settled);
  choosePeriod('24h');
  await waitFor(() => f.api.calls.length === 2 && settled());
  check('080 failed: stale badge and notice while the last hour stays', status() === t.stale
    && host.querySelector('.metrics-alerts .alert')?.textContent.includes(t.staleMessage)
    && rangeText() === fill(t.metricsRange, { n: 5 }) && trendsTitle() === t.trendsTitle);
  check('080 failed: the select keeps the chosen window', periodSelect().value === '24h');
  await waitFor(() => rangeText() === fill(t.metricsRange24h, { n: 8 }), 3000);
  const retry = f.api.calls[2];
  check('080 failed: the next automatic poll asks for the chosen window without history_after',
    !!retry && isPoll(retry) && isDay(retry) && !retry.query.has('history_after'));
  check('080 failed: its success clears the failure', status() === '' && host.querySelector('.metrics-alerts').childElementCount === 0);
  await waitFor(() => f.api.calls.length === 4, 3000);
  check('080 failed: later polls continue the chosen window', isDay(f.api.calls[3]) && f.api.calls[3].query.get('history_after') === '8');

  // Leaving and coming back starts again from the last hour: the router mounts a new screen.
  mount(fakeApi((call) => (isDay(call) ? dayBody() : okBody())));
  await waitFor(settled);
  choosePeriod('24h');
  await waitFor(() => rangeText() === fill(t.metricsRange24h, { n: 3 }));
  const returned = mount(fakeApi((call) => (isDay(call) ? dayBody() : okBody())));
  await waitFor(settled);
  check('080 return: the last hour is chosen and asked for', periodSelect().value === '1h' && returned.api.calls[0].path === '/api/v1/metrics'
    && rangeText() === fill(t.metricsRange, { n: 3 }));
}

async function case081() {
  // 1440 one-minute points, then an automatic poll that brings one new point.
  const gate = deferred();
  const m = mount(fakeApi((call, n) => {
    if (!isDay(call)) return okBody(pointsOf(1, 5));
    if (n === 2) return dayBody(dayPoints(1, 1440));
    if (n === 3) return dayBody(dayPoints(1441, 1441));
    return gate.promise;
  }, { pollSecs: 1 }));
  await waitFor(settled);
  choosePeriod('24h');
  await waitFor(() => rangeText() === fill(t.metricsRange24h, { n: 1440 }));
  check('081: range, section title, and cpu note of the last 24 hours', trendsTitle() === t.trendsTitle24h
    && kpi('cpu').textContent.includes(t.cpuNote24h) && !kpi('cpu').textContent.includes(t.cpuNote));
  check('081: five charts', chartIds().join('|') === 'sessions|connections|throughput|cpu|memory');
  const latest = T0 + 1440 * 60000;
  check('081: five time labels six hours apart, from one day back to the newest point',
    [...chart('sessions').querySelectorAll('.chart-x span')].map((node) => node.textContent).join('|')
    === [0, 6, 12, 18, 24].map((hours) => formatClock(latest - 86400000 + hours * 3600000)).join('|'));
  const sessions = coords(chart('sessions').querySelector('.chart-plot path.chart-line.s1'));
  check('081: one day across the plot, newest point at the right edge', sessions.length === 1440 && sessions[0][0] === 0.25
    && sessions.at(-1)[0] === 360);
  check('081: throughput and cpu are per-second rates of one-minute neighbours',
    [...kpi('throughput').querySelectorAll('dd .metric')].map((node) => node.textContent).join('|')
    === [perSecond(formatBytesRate(10240 / 60)), perSecond(formatBytesRate(20480 / 60))].join('|')
    && metricOf('cpu') === formatPercent(100 / 60));
  const legend = (value) => fill(t.legendValue, { current: formatDecimal(value), max: formatDecimal(value) });
  check('081: connections are the per-minute increase of each neighbouring pair',
    legendOf('connections').map((row) => row[1]).join('|') === [legend(2), legend(1), legend(0)].join('|')
    && coords(chart('connections').querySelector('.chart-plot path.chart-line.s1')).length === 1439);
  await waitFor(() => m.api.calls.length === 4, 4000);
  const poll = m.api.calls[2];
  check('081: the automatic poll asks for the last 24 hours after the largest kept seq', isPoll(poll) && isDay(poll)
    && poll.query.get('history_after') === '1440');
  check('081: the new point joins and the oldest leaves (1440 kept)', rangeText() === fill(t.metricsRange24h, { n: 1440 })
    && coords(chart('sessions').querySelector('.chart-plot path.chart-line.s1')).length === 1440);
  check('081: the next poll continues from seq 1441', isDay(m.api.calls[3]) && m.api.calls[3].query.get('history_after') === '1441');

  for (const [count, drawn] of [[1, []], [2, ['sessions', 'memory']], [3, ['sessions', 'connections', 'throughput', 'cpu', 'memory']]]) {
    mount(fakeApi((call) => (isDay(call) ? dayBody(dayPoints(1, count)) : okBody())));
    await waitFor(settled);
    choosePeriod('24h');
    await waitFor(() => rangeText() === fill(t.metricsRange24h, { n: count }));
    check(`081 ${count} points: charts drawn`, chartIds().join('|') === drawn.join('|'));
    check(`081 ${count} points: the others say a point comes every minute`, emptyCharts() === 5 - drawn.length
      && [...host.querySelectorAll('[data-chart-card] .chart-empty')].every((node) => node.textContent === t.chartEmpty24h));
    if (count === 1) {
      check('081 1 point: throughput and cpu are collecting, no zero', kpi('throughput').textContent.includes(t.collecting)
        && metricOf('cpu') === null);
    }
    if (count === 3) {
      check('081 3 points: the connection line joins two values',
        coords(chart('connections').querySelector('.chart-plot path.chart-line.s1')).length === 2);
    }
  }

  // Seq 5 is missing, and seq 7 comes two minutes after seq 6.
  const gappy = dayPoints(1, 8).filter((point) => point.seq !== 5)
    .map((point) => (point.seq >= 7 ? { ...point, at: iso(Date.parse(point.at) + 60000) } : point));
  mount(fakeApi((call) => (isDay(call) ? dayBody(gappy) : okBody())));
  await waitFor(settled);
  choosePeriod('24h');
  await waitFor(() => rangeText() === fill(t.metricsRange24h, { n: 7 }));
  const lines = (id) => [...chart(id).querySelectorAll('.chart-plot path.chart-line.s1')];
  check('081 gaps: a missing seq breaks the line and is never drawn as zero', lines('sessions').length === 2
    && lines('sessions').every((path) => coords(path).every(([, y]) => y < 100)));
  check('081 gaps: the connection line breaks at the missing seq', lines('connections').length === 2);
  check('081 gaps: a two-minute neighbour still gives a per-minute value (2 per 120 s is 1.0)',
    coords(lines('connections')[1])[0][1] === 50);
  checkLayout('081');

  // An answer with the other window's interval does not fit the request: a response format failure.
  mount(fakeApi(() => okBody()));
  await waitFor(settled);
  choosePeriod('24h');
  await waitFor(() => status() === t.stale);
  check('081: an answer of the last hour fails the last 24 hours and the last hour stays', status() === t.stale
    && rangeText() === fill(t.metricsRange, { n: 3 }));
}

// ---- Smallest bars (DESIGN.md 메트릭 화면 확장) ----

async function caseBars() {
  const ticket = { listener: 'ws', stage: 'upgrade' };
  const huge = [[{ ...ticket, reason: 'ticket_invalid' }, 10000], [{ ...ticket, reason: 'ticket_expired' }, 1], [{ ...ticket, reason: 'ticket_reused' }, 0]];
  mount(fakeApi(() => okBody([], defaultMetrics({
    'vnc_relay.connection.rejected': scalar('vnc_relay.connection.rejected', 'counter', '{connection}', huge),
    // 15001 values: 1 at ≤ 5 ms, 10000 at ≤ 10 ms, 5000 at ≤ 50 ms; the 200 s sum fits those buckets.
    'vnc_relay.handshake.duration': histogram('vnc_relay.handshake.duration', DURATION_BOUNDS, [
      [{ listener: 'ws', mode: 'mode2' }, 15001, 200, buckets(DURATION_BOUNDS, { 0: 1, 1: 10000, 3: 5000 })],
    ]),
  }))));
  await waitFor(settled);
  const near = (value, expected) => Math.abs(value - expected) < 0.06;
  const bars = [...rowsOf(card('distribution'))[0].querySelectorAll('svg.dist rect.dist-bar')];
  const heights = bars.map((rect) => rect.getBoundingClientRect().height);
  check('bars: the zero bucket has no bar', bars.length === 3);
  check('bars: a bucket of 1 next to 10000 is drawn at the 2px minimum', near(heights[0], 2));
  check('bars: the largest bucket keeps its proportional height (32px)', near(heights[1], 32));
  check('bars: a middle bucket stays proportional (16px)', near(heights[2], 16));
  const fills = [...cause('rejected').querySelectorAll('rect.count-fill')];
  const widths = fills.map((rect) => rect.getBoundingClientRect().width);
  check('bars: a count of 0 has no bar, the others do', fills.length === 2 && rowsOf(cause('rejected')).length === 3
    && rowsOf(cause('rejected'))[2].querySelector('rect.count-fill') === null);
  check('bars: the largest count fills the track (64px)', near(widths[0], 64));
  check('bars: a count of 1 next to 10000 is drawn at the 2px minimum', near(widths[1], 2));
  check('bars: the track keeps its width', near(cause('rejected').querySelector('rect.count-track').getBoundingClientRect().width, 64));
  checkLayout('bars');
}

// ---- Persistent screen states for F-C. They call the real mountMetrics and the shared CSS. ----

async function shot(state) {
  const gate = deferred();
  const api = fakeApi((call, n) => {
    if (state === 'loading') return gate.promise;
    if (state === 'failed-first') return fail(500);
    if (state === 'collecting') return okBody([]);
    if (state === 'rejected' && n > 1) return fail(400);
    if (state === 'failed' && n > 1) return fail(500);
    if (state === 'refreshing' && n > 1) return gate.promise;
    if (state === 'partial') {
      // The mockup's partial frame: the memory metrics are unavailable and some history values are null.
      return okBody(pointsOf(1, 30, (seq) => ({
        session_count: seq >= 12 && seq <= 15 ? null : 3,
        connection_accepted: seq === 20 ? null : seq * 2,
        process_memory_usage: null,
      })), defaultMetrics({
        'process.memory.usage': scalar('process.memory.usage', 'updowncounter', 'By', [], false),
        'process.memory.virtual': scalar('process.memory.virtual', 'updowncounter', 'By', [], false),
      }));
    }
    return okBody(pointsOf(1, 30));
  }, { timeoutSecs: 3600, pollSecs: 3600 });
  mount(api);
  if (state === 'loading') return;
  await waitFor(settled);
  if (['failed', 'refreshing', 'rejected'].includes(state)) host.querySelector('#metrics-refresh').click();
  if (state === 'failed' || state === 'rejected') await waitFor(() => status() === t.stale || status() === t.failed);
}

const CASES = {
  'dash-028': case028,
  'dash-064': case064,
  'dash-068': case068,
  'dash-069': case069,
  'dash-070': async () => { await case070(); await case070Gaps(); },
  'dash-071': case071,
  'dash-072': case072,
  'dash-073': case073,
  'dash-074': case074,
  'dash-075': case075,
  'dash-076': case076,
  'dash-077': case077,
  'dash-077-more': case077More,
  'dash-078': case078,
  'dash-079': case079,
  'dash-080': case080,
  'dash-081': case081,
  bars: caseBars,
  axis: caseAxis,
};
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
