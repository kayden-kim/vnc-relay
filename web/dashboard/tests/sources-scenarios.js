// Contract-shaped fake API evidence only. Real server coverage belongs to dashboard_live.
// One page load runs one case: ?case=dash-083&lang=ko|en, or ?case=shot-<state>&lang=ko|en for the screen states.
import { mountSources } from '../app/sources.js';
import { COPY, BRAND_NAME, PROVIDER_ATTRIBUTION } from '../app/copy.js';
import { STORAGE_KEY } from '../app/language.js';
import { LOGIN_URL } from '../app/shell.js';
import { fill, formatCount, formatTime } from '../app/format.js';
import { checkHelpLanguageChange } from './help-link-checks.js';
import {
  AS_OF, LONG_AS_NAME, LONG_IPV6, NOT_IN_USE, UNNAMED_COUNTRY, WINDOW_MS,
  addressItems, asnItems, countryItems, databases, emptyBody, exampleAsn, sourcesBody,
} from './sources-fixture.js';

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
  error: { code: status === 401 ? 'unauthenticated' : status === 500 ? 'internal' : 'invalid_request', message: 'error' },
});
const windowOf = (call) => call.query.get('window');
// A body for the window the call asked for.
const okBody = (call, options = {}) => ok(sourcesBody({ window: windowOf(call), ...options }));

// Fake management API. `handler(call, n)` answers the n-th sources request. `maxInflight` is the largest number of
// sources requests that were open at the same moment.
function fakeApi(handler = (call) => okBody(call), { pollSecs = 3600, timeoutSecs = 5 } = {}) {
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
    if (url.pathname !== '/api/v1/sources') throw new Error(`unexpected path ${path}`);
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
  const handle = mountSources(host, {
    request: api.request, storage: storage(), preferredLanguages: [lang], navigate: (url) => navigations.push(url),
  });
  current = { handle, api, navigations };
  return current;
}

const settled = () => host.querySelector('#sources-refresh')?.disabled === false;
const status = () => host.querySelector('#sources-status')?.textContent.trim() ?? null;
const refreshLabel = () => host.querySelector('#sources-refresh .button-label')?.textContent.trim() ?? null;
const timeText = () => host.querySelector('#sources-time')?.textContent ?? '';
const rangeText = () => host.querySelector('#sources-range')?.textContent ?? '';
const rangeShown = () => host.querySelector('#sources-range')?.hidden === false;
const kpi = (key) => host.querySelector(`[data-kpi="${key}"]`);
const metricOf = (key) => kpi(key)?.querySelector('.metric')?.textContent ?? null;
const card = (name) => host.querySelector(`[data-sources-card="${name}"]`);
const countOf = (name) => card(name)?.querySelector('.card-title .badge')?.textContent ?? null;
const dbRow = (key) => host.querySelector(`[data-database="${key}"]`);
const notice = () => host.querySelector('.sources-notice .alert');
const alertOf = () => host.querySelector('.sources-alerts .alert');
const rowsOf = (root) => [...(root?.querySelectorAll('tbody tr') ?? [])];
const cellsOf = (row) => [...row.children].map((cell) => cell.textContent.trim());
const isPoll = (call) => call.init.headers?.['X-Relay-Poll'] === '1';
const periodSelect = () => host.querySelector('#sources-period');
const lookupLabel = (value, copy = t) => `${fill(copy.lookupName, { value })} (${copy.opensInNewTab})`;
const rangeOf = (window, stored = 4210, copy = t) => fill(copy.sourcesRange, {
  start: formatTime(AS_OF - WINDOW_MS[window]), end: formatTime(AS_OF), stored: formatCount(stored), capacity: formatCount(100000),
});
// The cause sentence of CPY-270·271 removed, written out per language.
const NO_CAUSE = {
  ko: { stale: '파일을 다시 읽지 못해 이전 DB를 계속 씁니다. 파일을 고치면 자동으로 다시 읽습니다.' },
  en: { stale: 'The file could not be read again, so the previous database is still in use. The file is read again automatically after it is fixed.' },
};
// Browser region names in each language (CLDR).
const REGION = { ko: { KR: '대한민국', US: '미국' }, en: { KR: 'South Korea', US: 'United States' } };

// Chooses a period the way a keyboard or pointer user does: focus, new value, change event.
function choosePeriod(value) {
  const select = periodSelect();
  select.focus();
  select.value = value;
  select.dispatchEvent(new Event('change'));
}

function checkLayout(label) {
  check(`${label}: no horizontal scroll`, document.documentElement.scrollWidth <= document.documentElement.clientWidth);
  const links = host.querySelectorAll('.navbar-nav .navbar-item');
  check(`${label}: five navigation items, sources current`,
    links.length === 5 && links[2].getAttribute('aria-current') === 'page'
    && [...links].filter((link) => link.hasAttribute('aria-current')).length === 1);
  for (const box of [...host.querySelectorAll('section.card')].map((node) => node.getBoundingClientRect())) {
    if (box.width) check(`${label}: card inside viewport`, box.left >= 0 && box.right <= innerWidth + 1);
  }
  for (const control of host.querySelectorAll('button, select, a')) {
    const box = control.getBoundingClientRect();
    if (box.width && box.height) check(`${label}: control inside viewport`, box.left >= 0 && box.right <= innerWidth + 1);
  }
  for (const node of host.querySelectorAll('.table-scroll, .kpi-body, .src-db-info, .src-attrib')) {
    check(`${label}: no inner horizontal overflow`, node.scrollWidth <= node.clientWidth + 1);
  }
  const period = periodSelect()?.getBoundingClientRect();
  const refresh = host.querySelector('#sources-refresh')?.getBoundingClientRect();
  if (period && refresh && innerWidth <= 1023.98) {
    check(`${label}: period select spans the row above refresh at the touch height`, period.bottom <= refresh.top
      && Math.abs(period.width - refresh.width) < 1 && period.height >= 44);
  } else if (period && refresh) {
    check(`${label}: period select left of refresh with aligned bottoms`, period.right <= refresh.left
      && Math.abs(period.bottom - refresh.bottom) < 1);
  }
  if (innerWidth <= 1023.98) {
    const button = host.querySelector('#sources-refresh');
    if (button) check(`${label}: refresh uses the touch height`, button.getBoundingClientRect().height >= 44);
    const nav = host.querySelector('.navbar-nav');
    const tools = host.querySelector('.navbar-tools');
    check(`${label}: mobile navigation below the tools`, nav.getBoundingClientRect().top >= tools.getBoundingClientRect().bottom);
    const names = [...host.querySelectorAll('.src-db-name')].map((node) => node.getBoundingClientRect());
    const infos = [...host.querySelectorAll('.src-db-info')].map((node) => node.getBoundingClientRect());
    check(`${label}: database rows stack the name above the state`, names.every((box, index) => box.bottom <= infos[index].top + 1));
  }
  const tables = [...host.querySelectorAll('[data-sources-card] .card-body')].map((node) => node.getBoundingClientRect());
  check(`${label}: the three tables stack at full width`, tables.length === 0
    || tables.every((box) => Math.abs(box.left - tables[0].left) < 1 && Math.abs(box.width - tables[0].width) < 1));
}

// ---- DASH-028, 064 ----

async function case028() {
  const m = mount();
  await waitFor(settled);
  const links = [...host.querySelectorAll('.navbar-nav .navbar-item')];
  check('028: five items in the adopted order',
    links.map((link) => link.textContent.trim()).join('|') === [t.navOperations, t.metrics, t.sources, t.logs, t.settings].join('|'));
  check('028: item destinations', links.map((link) => link.getAttribute('href')).join('|') === '/|/#/metrics|/#/sources|/#/logs|/#/settings');
  check('028: only the sources item is current', links[2].getAttribute('aria-current') === 'page'
    && links.filter((link) => link.hasAttribute('aria-current')).length === 1);
  check('028: the sources item carries the pin icon', links[2].querySelector('svg') !== null);
  check('028: every item is visible', links.every((link) => link.getBoundingClientRect().width > 0 && link.getBoundingClientRect().height > 0));
  if (innerWidth >= 1024) {
    // The bundled Pretendard uses font-display: swap. Measure after it loads, not in the wider fallback font.
    await document.fonts.ready;
    const tops = links.map((link) => Math.round(link.getBoundingClientRect().top));
    check('028: the five items fit one line', tops.every((top) => top === tops[0]));
  }
  check('028: protected data shown with a session', metricOf('attempts') !== null);
  checkLayout('028');
  m.handle.dispose();
  check('028: dispose removes the protected data', host.childElementCount === 0);

  const denied = mount(fakeApi(() => fail(401)));
  await waitFor(() => denied.navigations.length > 0);
  check('028: no session goes to the login page', denied.navigations.join('|') === LOGIN_URL);
  check('028: no protected data without a session', host.childElementCount === 0 && kpi('attempts') === null);
}

async function case064() {
  const m = mount();
  await waitFor(settled);
  const before = m.api.calls.length;
  checkHelpLanguageChange(host, lang, check);
  check('064: no help API requests', m.api.calls.length === before);
  check('064: current screen remains mounted', m.navigations.length === 0 && kpi('attempts') !== null);
}

// ---- DASH-082: entry, polls, leaving, and returning on the screen itself (the router part is in router.html) ----

async function case082() {
  const gate = deferred();
  const m = mount(fakeApi((call, n) => (n === 2 ? gate.promise : okBody(call)), { pollSecs: 1 }));
  await waitFor(settled);
  check('082: the title and the document title', host.querySelector('h1')?.textContent === t.sources
    && document.title === `${BRAND_NAME} ${t.sources}`);
  check('082: entry asks for the last hour once without X-Relay-Poll',
    m.api.calls.length === 1 && m.api.calls[0].path === '/api/v1/sources?window=1h' && !isPoll(m.api.calls[0]));
  check('082: the period starts at the last hour', periodSelect().value === '1h');
  await waitFor(() => m.api.calls.length === 2, 3000);
  check('082: automatic poll carries X-Relay-Poll and the same window', isPoll(m.api.calls[1]) && windowOf(m.api.calls[1]) === '1h');
  m.handle.dispose();
  gate.resolve(okBody(m.api.calls[1], { totals: { attempts: 9999, rejected: 0, aborted: 0, sessions: 0 } }));
  await wait(300);
  check('082: a late response after leaving draws nothing', host.childElementCount === 0);
  await wait(1300);
  check('082: leaving stops the automatic polls', m.api.calls.length === 2);

  const back = mount(fakeApi((call) => okBody(call)));
  await waitFor(settled);
  check('082: returning asks for the last hour from the start', back.api.calls[0].path === '/api/v1/sources?window=1h'
    && !isPoll(back.api.calls[0]) && periodSelect().value === '1h');
}

// ---- DASH-083: values, counts, range, storage notice, and polls ----

async function case083() {
  const second = {
    totals: { attempts: 2000, rejected: 1, aborted: 2, sessions: 3 },
    addresses: { total: 1, items: addressItems(2).slice(1) },
    countries: { total: 1, items: countryItems().slice(0, 1) },
    asns: { total: 1, items: asnItems(2).slice(0, 1) },
  };
  const m = mount(fakeApi((call, n) => (n === 1 ? okBody(call) : okBody(call, second)), { pollSecs: 1 }));
  await waitFor(settled);
  check('083: the seven summary cards in order', [...host.querySelectorAll('[data-kpi] .card-heading')].map((node) => node.textContent).join('|')
    === t.sourcesSummaryNames.join('|'));
  check('083: summary values are the totals and the three total values',
    ['attempts', 'rejected', 'aborted', 'sessions', 'addresses', 'countries', 'asns'].map(metricOf).join('|')
    === [1234, 56, 78, 90, 150, 7, 120].map(formatCount).join('|'));
  check('083: summary title and note', host.querySelector('.metrics-section > h2')?.textContent === t.sourcesSummaryTitle
    && host.querySelector('.metrics-section > p')?.textContent === t.sourcesSummaryNote);
  const addressItemsSent = addressItems(100);
  check('083: every address row in response order', rowsOf(card('addresses')).length === 100
    && rowsOf(card('addresses')).every((row, index) => row.children[0].querySelector('.mono, a span')?.textContent === addressItemsSent[index].ip));
  check('083: address counts and last event', cellsOf(rowsOf(card('addresses'))[1]).slice(3).join('|')
    === [499, 499 % 3, 499 % 2, Math.floor(499 / 4)].map(formatCount).concat(formatTime(AS_OF - 60000)).join('|'));
  check('083: every country row in response order', rowsOf(card('countries')).length === 7
    && cellsOf(rowsOf(card('countries'))[6])[0] === t.unknownValue && cellsOf(rowsOf(card('countries'))[5])[0] === UNNAMED_COUNTRY);
  check('083: every ASN row in response order', rowsOf(card('asns')).length === 100
    && rowsOf(card('asns')).slice(0, 99).every((row, index) => row.children[0].textContent === `AS${exampleAsn(index)}`)
    && cellsOf(rowsOf(card('asns'))[99])[0] === t.unknownValue);
  check('083: address and ASN titles show the top count, country the plain count',
    countOf('addresses') === fill(t.partialCount, { total: formatCount(150), shown: formatCount(100) })
    && countOf('asns') === fill(t.partialCount, { total: formatCount(120), shown: formatCount(100) })
    && countOf('countries') === fill(t.listCount, { n: formatCount(7) }));
  check('083: null country and ASN cells are unknown', cellsOf(rowsOf(card('addresses'))[4])[1] === t.unknownValue
    && cellsOf(rowsOf(card('addresses'))[3])[2] === t.unknownValue);
  check('083: range is CPY-257 in local time', rangeShown() && rangeText() === rangeOf('1h'));
  check('083: no storage notice when dropped_before is null', notice() === null);
  await waitFor(() => m.api.calls.length === 2 && metricOf('attempts') === formatCount(2000), 3000);
  check('083: automatic poll carries X-Relay-Poll and the same window', isPoll(m.api.calls[1]) && windowOf(m.api.calls[1]) === '1h');
  check('083: a poll replaces every value', metricOf('addresses') === formatCount(1) && rowsOf(card('addresses')).length === 1
    && rowsOf(card('countries')).length === 1 && rowsOf(card('asns')).length === 1);
  check('083: one item is CPY-085', countOf('addresses') === t.listCountOne && countOf('countries') === t.listCountOne);
  await waitFor(() => m.api.calls.length === 3, 3000);
  check('083: requests never overlap', m.api.maxInflight === 1);

  // dropped_before before, at, and after window_start.
  const start = AS_OF - WINDOW_MS['1h'];
  for (const [label, dropped, shown] of [['earlier', start - 1000, false], ['equal', start, true], ['later', start + 600000, true]]) {
    mount(fakeApi((call) => okBody(call, { droppedBefore: dropped, stored: 100000 })));
    await waitFor(settled);
    check(`083 ${label}: storage notice ${shown ? 'shown' : 'hidden'}`, (notice() !== null) === shown);
    if (shown) {
      check(`083 ${label}: CPY-258 with the dropped time`, notice().textContent === fill(t.sourcesDropped, { time: formatTime(dropped) }));
      check(`083 ${label}: a warning, not an error, and not a live region`, notice().classList.contains('alert-tone-warning')
        && !notice().hasAttribute('role') && status() === '' && alertOf() === null);
    }
  }
}

// ---- DASH-084: lookup links and local addresses ----

async function case084() {
  const m = mount();
  await waitFor(settled);
  const rows = rowsOf(card('addresses'));
  const v6 = rows[0].children[0].querySelector('a');
  check('084: a public IPv6 address is an ipinfo.io link', v6?.getAttribute('href') === `https://ipinfo.io/${LONG_IPV6}`
    && v6.textContent === LONG_IPV6);
  const v4 = rows[1].children[0].querySelector('a');
  check('084: a public IPv4 address is an ipinfo.io link', v4?.getAttribute('href') === 'https://ipinfo.io/198.51.100.250');
  check('084: link accessible names are CPY-291 and CPY-196', v6?.getAttribute('aria-label') === lookupLabel(LONG_IPV6)
    && v4?.getAttribute('aria-label') === lookupLabel('198.51.100.250'));
  const local = rows[2].children[0];
  check('084: a local address has no link and shows the local badge', local.querySelector('a') === null
    && local.querySelector('.mono')?.textContent === '127.0.0.1' && local.querySelector('.badge')?.textContent === t.localAddress);
  const asn = rows[0].children[2];
  const asLink = asn.querySelector('a');
  check('084: an ASN is an AS<n> link to ipinfo.io', asLink?.querySelector('span')?.textContent === `AS${exampleAsn(4)}`
    && asLink.getAttribute('href') === `https://ipinfo.io/AS${exampleAsn(4)}` && asLink.getAttribute('aria-label') === lookupLabel(`AS${exampleAsn(4)}`));
  check('084: the address table puts the AS name on the next line', asn.querySelector('.src-asname')?.textContent === LONG_AS_NAME
    && asn.querySelector('.src-asname').getBoundingClientRect().top >= asLink.getBoundingClientRect().bottom - 1);
  check('084: a null ASN has no link', rows[3].children[2].querySelector('a') === null
    && rows[3].children[2].textContent === t.unknownValue);
  const asnRows = rowsOf(card('asns'));
  check('084: the ASN table name cell is the AS name or empty', cellsOf(asnRows[4])[1] === LONG_AS_NAME && cellsOf(asnRows[15])[1] === ''
    && asnRows[4].children[0].querySelector('.src-asname') === null);
  check('084: a null ASN row in the ASN table has no link', asnRows[99].children[0].querySelector('a') === null);
  const lookups = [...host.querySelectorAll('a[href^="https://ipinfo.io/"]')];
  check('084: every lookup opens a new tab without opener or referrer', lookups.length > 0
    && lookups.every((link) => link.target === '_blank' && link.rel === 'noopener noreferrer'
      && link.querySelector('svg')?.getAttribute('aria-hidden') === 'true'));
  check('084: lookup links are in the data and numeric fonts', v6.classList.contains('mono') && asLink.classList.contains('num'));
  check('084: the address card explains the address, local addresses, and lookups',
    [...card('addresses').querySelectorAll('.card-description')].map((node) => node.textContent).join('|')
    === [t.clientAddressNote, t.localAddressNote, t.lookupNote].join('|'));
  v6.focus();
  check('084: a lookup link takes keyboard focus', document.activeElement === v6);
  asLink.focus();
  check('084: an ASN link takes keyboard focus', document.activeElement === asLink);
  check('084: the dashboard never requests ipinfo.io', m.api.calls.every((call) => call.path.startsWith('/api/v1/sources'))
    && performance.getEntriesByType('resource').every((entry) => new URL(entry.name, location.href).origin === location.origin));
}

// ---- DASH-085: IP information database states and attributions ----

function dbBadge(key) {
  return dbRow(key)?.querySelector('.src-db-info .badge') ?? null;
}
function dbFacts(key) {
  return [...(dbRow(key)?.querySelectorAll('dl.summary > div') ?? [])].map((pair) => `${pair.children[0].textContent}=${pair.children[1].textContent}`);
}
function dbNote(key) {
  return dbRow(key)?.querySelector('.src-db-info > p')?.textContent ?? null;
}
const attributions = () => [...host.querySelectorAll('.src-attrib > p')].map((line) => line.dataset.provider);
const loadedFacts = (type) => [
  `${t.ipDbInfoLabels[0]}=${type}`,
  `${t.ipDbInfoLabels[1]}=${formatTime('2026-11-01T00:00:00.000Z')}`,
  `${t.ipDbInfoLabels[2]}=${formatTime('2026-11-20T09:00:00.000Z')}`,
].join('|');

async function showDatabases(country, asn) {
  mount(fakeApi((call) => okBody(call, { dbs: databases(country, asn) })));
  await waitFor(settled);
}

async function case085() {
  // Not configured and loaded from GeoLite: only the address part of CPY-278 is a link.
  await showDatabases({ state: 'not_configured', ...NOT_IN_USE }, { database_type: 'GeoLite2-ASN', provider: 'maxmind_geolite' });
  check('085 A: row names in order', [...host.querySelectorAll('.src-db-name')].map((node) => node.textContent).join('|') === t.ipDbNames.join('|'));
  check('085 A: not configured is a neutral CPY-264 badge with no values', dbBadge('country')?.textContent === t.ipDbNotConfigured
    && dbBadge('country').classList.contains('badge-tone-neutral') && dbFacts('country').length === 0);
  check('085 A: CPY-269 names the key in the data font', dbNote('country') === fill(t.ipDbNotConfiguredNote, { key: 'ip_info.country_db' })
    && dbRow('country').querySelector('p .mono')?.textContent === 'ip_info.country_db');
  check('085 A: loaded is a success CPY-265 badge with type, build and load times', dbBadge('asn')?.textContent === t.ipDbLoaded
    && dbBadge('asn').classList.contains('badge-tone-success') && dbFacts('asn').join('|') === loadedFacts('GeoLite2-ASN')
    && dbNote('asn') === null);
  const geolite = PROVIDER_ATTRIBUTION.maxmind_geolite;
  const line = host.querySelector('.src-attrib > p');
  const link = line?.querySelector('a');
  check('085 A: one GeoLite attribution whose address is the only link', attributions().join('|') === 'maxmind_geolite'
    && link?.getAttribute('href') === geolite.href && link.querySelector('span:not(.sr-only)')?.textContent === geolite.href
    && line.textContent === geolite.text.replace(geolite.href, `${geolite.href} (${t.opensInNewTab})`));
  check('085 A: attribution link opens a new tab and names it', link?.target === '_blank' && link.rel === 'noopener noreferrer'
    && link.querySelector('.sr-only')?.textContent === ` (${t.opensInNewTab})` && line.lang === 'en');
  check('085 A: the screen status does not change', status() === '' && alertOf() === null);

  // Stale with a known cause and unavailable with an unknown cause.
  await showDatabases({ state: 'stale', error: 'file_too_large' }, { state: 'unavailable', error: 'disk_on_fire', ...NOT_IN_USE });
  check('085 B: stale is a warning CPY-266 badge with the values of the previous file', dbBadge('country')?.textContent === t.ipDbStale
    && dbBadge('country').classList.contains('badge-tone-warning') && dbFacts('country').join('|') === loadedFacts('DBIP-Country-Lite'));
  check('085 B: CPY-270 with CPY-273', dbNote('country') === fill(t.ipDbStaleNote, { reason: t.ipDbFileTooLarge }));
  check('085 B: unavailable is an error CPY-267 badge without values', dbBadge('asn')?.textContent === t.ipDbUnavailable
    && dbBadge('asn').classList.contains('badge-tone-destructive') && dbFacts('asn').length === 0);
  check('085 B: an unknown cause stays as received', dbNote('asn') === fill(t.ipDbUnavailableNote, { reason: 'disk_on_fire' }));
  const dbip = host.querySelector('.src-attrib > p a');
  check('085 B: one DB-IP attribution, wholly a link', attributions().join('|') === 'dbip'
    && dbip?.getAttribute('href') === PROVIDER_ATTRIBUTION.dbip.href
    && dbip.querySelector('span:not(.sr-only)')?.textContent === PROVIDER_ATTRIBUTION.dbip.text);
  check('085 B: the screen status does not change', status() === '' && alertOf() === null);

  // An unknown state with only some values, and the same provider twice.
  await showDatabases(
    { state: 'warming_up', database_type: 'IPinfo Lite', provider: 'ipinfo', build_time: null, loaded_at: null },
    { database_type: 'IPinfo Lite', provider: 'ipinfo' },
  );
  check('085 C: an unknown state is shown as received in a neutral badge with the values it has',
    dbBadge('country')?.textContent === 'warming_up' && dbBadge('country').classList.contains('badge-tone-neutral')
    && dbFacts('country').join('|') === `${t.ipDbInfoLabels[0]}=IPinfo Lite` && dbNote('country') === null);
  check('085 C: the same provider is attributed once', attributions().join('|') === 'ipinfo'
    && host.querySelector('.src-attrib a')?.getAttribute('href') === PROVIDER_ATTRIBUTION.ipinfo.href);

  // Providers without an attribution, and the other two known causes.
  await showDatabases(
    { database_type: 'GeoIP2-Country', provider: 'maxmind', state: 'unavailable', error: 'invalid_format' },
    { database_type: 'Custom-ASN', provider: 'unknown', state: 'stale', error: 'unsupported_data' },
  );
  check('085 D: maxmind and unknown have no attribution', host.querySelector('.src-attrib') === null);
  check('085 D: CPY-271 with CPY-274 and CPY-270 with CPY-275', dbNote('country') === fill(t.ipDbUnavailableNote, { reason: t.ipDbInvalidFormat })
    && dbNote('asn') === fill(t.ipDbStaleNote, { reason: t.ipDbUnsupportedData }));
  await showDatabases({ provider: 'acme_geo', state: 'stale', error: null }, { state: 'unavailable', error: 'file_unreadable', ...NOT_IN_USE });
  check('085 E: an unknown provider and a null provider have no attribution', host.querySelector('.src-attrib') === null);
  check('085 E: a null cause drops the cause sentence', dbNote('country') === NO_CAUSE[lang].stale);
  check('085 E: CPY-271 with CPY-272', dbNote('asn') === fill(t.ipDbUnavailableNote, { reason: t.ipDbFileUnreadable }));
  check('085 E: the screen status does not change', status() === '' && alertOf() === null);
  check('085: database types and attributions are never translated', dbFacts('country').length === 3
    && dbFacts('country')[0].endsWith('=DBIP-Country-Lite'));
}

// ---- DASH-086: loading, failures, empty, recovery, 400/405, 401, and the 1-second busy rule ----

async function case086() {
  const gate = deferred();
  mount(fakeApi((call) => gate.promise));
  await waitFor(() => status() === t.loading);
  check('086 loading: CPY-016 and an unavailable time', status() === t.loading && timeText() === fill(t.lastSuccess, { time: t.unavailable }));
  check('086 loading: no range and no zero', !rangeShown() && host.querySelector('.metric') === null
    && ![...host.querySelectorAll('[data-sources-card] tbody td')].some((cell) => cell.textContent.trim() === '0'));
  check('086 loading: every value place shows the loading badge', [...host.querySelectorAll('[data-kpi]')].length === 7
    && [...host.querySelectorAll('[data-kpi] .kpi-body')].every((node) => node.textContent === t.loading)
    && [...host.querySelectorAll('.src-db-info')].every((node) => node.textContent === t.loading)
    && rowsOf(card('addresses')).length === 1 && cellsOf(rowsOf(card('addresses'))[0])[0] === t.loading
    && countOf('addresses') === null);
  check('086 loading: refresh is disabled, the period is not', host.querySelector('#sources-refresh').disabled && !periodSelect().disabled);
  gate.resolve(ok(sourcesBody()));
  await waitFor(settled);
  check('086 loading: the first success shows the values', metricOf('attempts') === formatCount(1234) && status() === '');

  for (const [label, handler, timeoutSecs] of [
    ['500', () => fail(500), 5],
    ['no response', () => Promise.reject(new TypeError('network')), 5],
    ['timeout', stalled, 1],
    ['bad body', () => ok({ as_of: 'soon' }), 5],
  ]) {
    mount(fakeApi(handler, { timeoutSecs }));
    await waitFor(settled, 3000);
    check(`086 first ${label}: CPY-018 and CPY-021`, status() === t.failed && alertOf()?.textContent.includes(t.failedMessage));
    check(`086 first ${label}: no sections, no zero`, host.querySelector('.sources-body').childElementCount === 0 && !rangeShown()
      && timeText() === fill(t.lastSuccess, { time: t.unavailable }));
  }

  mount(fakeApi((call) => ok(emptyBody({ window: windowOf(call) }))));
  await waitFor(settled);
  check('086 empty: the summary is zero', [...host.querySelectorAll('[data-kpi] .metric')].map((node) => node.textContent).join('|') === Array(7).fill('0').join('|'));
  check('086 empty: each table has one CPY-290 row', ['addresses', 'countries', 'asns'].every((name) => rowsOf(card(name)).length === 1
    && cellsOf(rowsOf(card(name))[0])[0] === t.sourcesEmpty && countOf(name) === fill(t.listCount, { n: '0' })));
  check('086 empty: not an error, database rows stay', status() === '' && alertOf() === null && dbBadge('country')?.textContent === t.ipDbLoaded);
}

async function case086More() {
  // A failed poll keeps the values and time; the next poll recovers.
  const m = mount(fakeApi((call, n) => (n === 2 ? fail(500) : okBody(call, n >= 3 ? { totals: { attempts: 7, rejected: 0, aborted: 0, sessions: 0 } } : {})), { pollSecs: 1 }));
  await waitFor(settled);
  const time = timeText();
  await waitFor(() => status() === t.stale, 3000);
  check('086 stale: CPY-019 and CPY-020 with the values and time kept', status() === t.stale
    && alertOf()?.textContent.includes(t.staleMessage.split('{time}')[0]) && metricOf('attempts') === formatCount(1234)
    && timeText() === time && rangeText() === rangeOf('1h'));
  await waitFor(() => metricOf('attempts') === '7', 3000);
  check('086 stale: the next poll recovers and clears the notice', status() === '' && alertOf() === null && timeText() !== time);
  check('086 stale: requests never overlap', m.api.maxInflight === 1);

  // A refresh that recovers.
  const r = mount(fakeApi((call, n) => (n === 2 ? fail(500) : okBody(call))));
  await waitFor(settled);
  host.querySelector('#sources-refresh').click();
  await waitFor(() => status() === t.stale);
  host.querySelector('#sources-alert-refresh').click();
  await waitFor(() => status() === '' && settled());
  check('086 refresh: the alert refresh recovers', r.api.calls.length === 3 && !isPoll(r.api.calls[2]) && alertOf() === null);

  // 401 removes the values and goes to the login page.
  const u = mount(fakeApi((call, n) => (n === 2 ? fail(401) : okBody(call)), { pollSecs: 1 }));
  await waitFor(settled);
  await waitFor(() => u.navigations.length > 0, 3000);
  check('086 401: goes to the login page without data', u.navigations.join('|') === LOGIN_URL && host.childElementCount === 0);
  await wait(1300);
  check('086 401: no further requests', u.api.calls.length === 2);
}

async function case086Rejected() {
  for (const code of [400, 405]) {
    const m = mount(fakeApi((call, n) => (n === 2 ? fail(code) : okBody(call)), { pollSecs: 1 }));
    await waitFor(settled);
    const time = timeText();
    await waitFor(() => status() === t.failed, 3000);
    check(`086 ${code}: CPY-018 and CPY-293 with values and time kept`, alertOf()?.textContent.includes(t.sourcesRejected)
      && metricOf('attempts') === formatCount(1234) && timeText() === time && rangeShown());
    await wait(1400);
    check(`086 ${code}: automatic polls stop`, m.api.calls.length === 2);
    host.querySelector('#sources-refresh').click();
    await waitFor(() => status() === '' && settled());
    check(`086 ${code}: refresh asks again and recovers`, !isPoll(m.api.calls[2]) && alertOf() === null);
    await waitFor(() => m.api.calls.length === 4, 2000);
    check(`086 ${code}: polls start again`, isPoll(m.api.calls[3] ?? { init: {} }));
  }
}

async function case086Busy() {
  const fast = mount(fakeApi((call, n) => (n === 1 ? okBody(call) : wait(300).then(() => okBody(call))), { pollSecs: 1 }));
  await waitFor(settled);
  let busySeen = false;
  for (let steps = 130; steps > 0; steps -= 1) {
    busySeen ||= status() !== '' || refreshLabel() !== t.refresh;
    await wait(20);
  }
  check('086 fast: two automatic polls ran', fast.api.calls.filter(isPoll).length >= 2);
  check('086 fast: badge and refresh label do not change under 1 s', !busySeen);

  const slow = mount(fakeApi((call, n) => (n === 2 ? wait(1600).then(() => okBody(call)) : okBody(call)), { pollSecs: 1 }));
  await waitFor(settled);
  await waitFor(() => slow.api.calls.length === 2, 3000);
  const startedAt = slow.api.calls[1].at;
  await wait(500);
  check('086 slow: unchanged 0.5 s into the poll', status() === '' && refreshLabel() === t.refresh);
  await waitFor(() => status() === t.refreshing, 1500);
  const shownAfter = performance.now() - startedAt;
  check('086 slow: CPY-017 after 1 s on badge and button', status() === t.refreshing && refreshLabel() === t.refreshing
    && host.querySelector('#sources-refresh').disabled && shownAfter >= 980 && shownAfter < 1300);
  check('086 slow: values stay while updating', metricOf('attempts') === formatCount(1234) && rangeShown());
  await waitFor(() => status() === '', 2000);
  check('086: requests never overlap', fast.api.maxInflight === 1 && slow.api.maxInflight === 1);
}

// ---- DASH-087: period and language ----

const dayTotals = { attempts: 4321, rejected: 0, aborted: 0, sessions: 0 };
const byWindow = (call) => okBody(call, windowOf(call) === '24h' ? { totals: dayTotals } : {});

async function case087() {
  const gate = deferred();
  const m = mount(fakeApi((call, n) => (n === 2 ? gate.promise : byWindow(call))));
  await waitFor(settled);
  const select = periodSelect();
  const label = host.querySelector('label[for="sources-period"]');
  check('087: the select has the visible label CPY-256', label?.textContent === t.sourcesPeriodLabel && label.getBoundingClientRect().width > 0);
  check('087: two choices, the last hour first and chosen', [...select.options].map((option) => `${option.value}=${option.textContent}`).join('|')
    === `1h=${t.periodHour}|24h=${t.periodDay}` && select.value === '1h');
  const time = timeText();
  choosePeriod('24h');
  check('087: updating shows at once', status() === t.refreshing && refreshLabel() === t.refreshing);
  check('087: the select stays usable and keeps the focus', !select.disabled && document.activeElement === select);
  check('087: the change asks for the last 24 hours without X-Relay-Poll', windowOf(m.api.calls[1]) === '24h' && !isPoll(m.api.calls[1]));
  check('087: until it succeeds the last hour stays', rangeText() === rangeOf('1h') && timeText() === time && metricOf('attempts') === formatCount(1234));
  gate.resolve(byWindow(m.api.calls[1]));
  await waitFor(settled);
  check('087: success replaces every value with the last 24 hours', rangeText() === rangeOf('24h') && metricOf('attempts') === formatCount(4321)
    && status() === '' && document.activeElement === select);
  choosePeriod('1h');
  await waitFor(() => m.api.calls.length === 3 && settled());
  check('087: back to the last hour', windowOf(m.api.calls[2]) === '1h' && !isPoll(m.api.calls[2]) && rangeText() === rangeOf('1h'));

  // A change during an automatic poll waits for it, drops its answer, and then asks for the chosen period.
  const late = deferred();
  const p = mount(fakeApi((call, n) => (n === 2 ? late.promise : byWindow(call)), { pollSecs: 1 }));
  await waitFor(settled);
  await waitFor(() => p.api.calls.length === 2, 3000);
  check('087 during a poll: the poll runs without the updating state yet', isPoll(p.api.calls[1]) && status() === '');
  const seen = [];
  const observer = new MutationObserver(() => seen.push(metricOf('attempts')));
  observer.observe(host.querySelector('.sources-body'), { childList: true, subtree: true, characterData: true });
  choosePeriod('24h');
  check('087 during a poll: updating at once, no second request', status() === t.refreshing && p.api.calls.length === 2);
  late.resolve(okBody(p.api.calls[1], { totals: { attempts: 999, rejected: 0, aborted: 0, sessions: 0 } }));
  await waitFor(() => metricOf('attempts') === formatCount(4321), 3000);
  observer.disconnect();
  check('087 during a poll: the chosen period is asked right after', windowOf(p.api.calls[2]) === '24h' && !isPoll(p.api.calls[2]));
  check('087 during a poll: the answer of the earlier period never shows', !seen.includes(formatCount(999)));
  check('087: requests never overlap', m.api.maxInflight === 1 && p.api.maxInflight === 1);

  // A change that fails keeps the last hour; the next automatic poll asks for the chosen period.
  const f = mount(fakeApi((call, n) => (n === 2 ? fail(500) : byWindow(call)), { pollSecs: 1 }));
  await waitFor(settled);
  choosePeriod('24h');
  await waitFor(() => f.api.calls.length === 2 && settled());
  check('087 failed: CPY-019 and CPY-020 while the last hour stays', status() === t.stale
    && alertOf()?.textContent.includes(t.staleMessage.split('{time}')[0]) && rangeText() === rangeOf('1h') && periodSelect().value === '24h');
  await waitFor(() => rangeText() === rangeOf('24h'), 3000);
  check('087 failed: the next automatic poll asks for the chosen period', isPoll(f.api.calls[2]) && windowOf(f.api.calls[2]) === '24h'
    && status() === '' && alertOf() === null);
}

async function case087Language() {
  const m = mount(fakeApi(byWindow));
  await waitFor(settled);
  choosePeriod('24h');
  await waitFor(() => rangeText() === rangeOf('24h'));
  const next = lang === 'ko' ? 'en' : 'ko';
  const n = COPY[next];
  const country = (row) => cellsOf(rowsOf(card('countries'))[row])[0];
  check('087: country names in the screen language', country(0) === `${REGION[lang].KR} (KR)` && country(1) === `${REGION[lang].US} (US)`
    && country(5) === UNNAMED_COUNTRY);
  const calls = m.api.calls.length;
  const time = timeText().split(': ').slice(1).join(': ');
  const select = host.querySelector('#sources-language');
  select.focus();
  select.value = next;
  select.dispatchEvent(new Event('change'));
  check('087 language: no new request', m.api.calls.length === calls);
  check('087 language: document language, title, and the saved choice', document.documentElement.lang === next
    && document.title === `${BRAND_NAME} ${n.sources}` && saved.some(([, value]) => value === next));
  check('087 language: texts change at once', host.querySelector('h1').textContent === n.sources
    && card('addresses').querySelector('.card-heading').textContent === n.addressTitle
    && host.querySelector('label[for="sources-period"]').textContent === n.sourcesPeriodLabel);
  check('087 language: country names change, an unnamed code stays', country(0) === `${REGION[next].KR} (KR)`
    && country(1) === `${REGION[next].US} (US)` && country(5) === UNNAMED_COUNTRY);
  check('087 language: addresses, AS names, database types, and attributions are not translated',
    rowsOf(card('addresses'))[0].children[0].textContent === LONG_IPV6
    && cellsOf(rowsOf(card('asns'))[4])[1] === LONG_AS_NAME && dbFacts('country')[0].endsWith('=DBIP-Country-Lite')
    && host.querySelector('.src-attrib > p a span')?.textContent === PROVIDER_ATTRIBUTION.dbip.text);
  check('087 language: the chosen period, values, and time stay', periodSelect().value === '24h' && rangeText() === rangeOf('24h', 4210, n)
    && metricOf('attempts') === formatCount(4321) && timeText().endsWith(time));
  check('087 language: lookup names follow the language', rowsOf(card('addresses'))[0].querySelector('a').getAttribute('aria-label')
    === lookupLabel(LONG_IPV6, n));
  check('087 language: focus stays on the language select', document.activeElement?.id === 'sources-language');
}

// ---- DASH-088: widths, keyboard, and live regions ----

async function case088() {
  mount();
  await waitFor(settled);
  checkLayout('088 normal');
  const tables = [...host.querySelectorAll('[data-sources-card] table')];
  check('088: three tables with a caption and column headings', tables.length === 3
    && tables.every((table) => table.querySelector('caption')?.textContent.length > 0 && table.querySelectorAll('thead th').length > 0));
  check('088: captions are the table titles', tables.map((table) => table.querySelector('caption').textContent).join('|')
    === [t.addressTitle, t.countryTitle, t.asnTitle].join('|'));
  check('088: count cells align right', tables.every((table) => [...table.querySelectorAll('tbody td.data-table-num')]
    .every((cell) => getComputedStyle(cell).textAlign === 'right' || innerWidth <= 1023.98)));
  check('088: every cell carries its column name', tables.every((table) => [...table.querySelectorAll('tbody td')].every((cell) => cell.hasAttribute('data-label'))));
  const v6 = rowsOf(card('addresses'))[0].children[0];
  const asName = rowsOf(card('addresses'))[0].querySelector('.src-asname');
  check('088: the long IPv6 address and AS name stay whole and inside their cells', v6.textContent === LONG_IPV6
    && v6.scrollWidth <= v6.clientWidth + 1 && asName.textContent === LONG_AS_NAME && asName.scrollWidth <= asName.clientWidth + 1);
  check('088: local badge and database states are text', rowsOf(card('addresses'))[2].querySelector('.badge')?.textContent === t.localAddress
    && dbBadge('country')?.textContent === t.ipDbLoaded);
  if (innerWidth <= 1023.98) {
    const emptyName = rowsOf(card('asns'))[15].children[1];
    check('088 mobile: an empty AS name has no line', getComputedStyle(emptyName).display === 'none');
  }
  const live = [...host.querySelectorAll('[aria-live], [role="status"], [role="alert"]')];
  check('088: only the status cell is a live region', live.length === 1 && live[0].id === 'sources-status');
  const selectors = ['.navbar-item', '.help-link', '#sources-language', '#sources-logout', '#sources-period', '#sources-refresh',
    '[data-sources-card="addresses"] a', '[data-sources-card="asns"] a', '.src-attrib a'];
  for (const selector of selectors) {
    const control = host.querySelector(selector);
    control?.focus();
    check(`088: keyboard reaches ${selector}`, !!control && document.activeElement === control);
  }

  mount(fakeApi((call) => okBody(call, { droppedBefore: AS_OF - 1800000, stored: 100000 })));
  await waitFor(settled);
  checkLayout('088 storage limit');
  check('088 storage limit: the notice is not a live region',
    [...host.querySelectorAll('[aria-live], [role="status"], [role="alert"]')].length === 1);

  mount(fakeApi((call) => okBody(call, { dbs: databases({ state: 'not_configured', ...NOT_IN_USE }, { state: 'stale', error: 'file_too_large', provider: 'ipinfo', database_type: 'IPinfo Lite' }) })));
  await waitFor(settled);
  checkLayout('088 database states');

  mount(fakeApi((call) => ok(emptyBody({ window: windowOf(call) }))));
  await waitFor(settled);
  checkLayout('088 empty');

  mount(fakeApi(() => fail(500)));
  await waitFor(settled);
  checkLayout('088 failed');
  const alerts = [...host.querySelectorAll('[role="alert"]')];
  check('088 failed: the failure notice is announced once', alerts.length === 1);
  host.querySelector('#sources-alert-refresh').focus();
  check('088 failed: the alert refresh is reachable', document.activeElement.id === 'sources-alert-refresh');

  mount(fakeApi(() => new Promise(() => {}), { timeoutSecs: 3600 }));
  await waitFor(() => status() === t.loading);
  checkLayout('088 loading');
  periodSelect().focus();
  check('088 loading: the period select is reachable', document.activeElement === periodSelect());
}

// ---- Persistent screen states. They call the real mountSources and the shared CSS. ----

// The adopted frames list seven rows in the address and ASN tables, so the screens here do the same.
async function shot(state) {
  const gate = deferred();
  const compact = { addresses: { total: 150, items: addressItems(7) }, asns: { total: 120, items: asnItems(7) } };
  const api = fakeApi((call, n) => {
    if (state === 'loading') return gate.promise;
    if (state === 'failed-first') return fail(500);
    if (state === 'empty') return ok(emptyBody({ window: windowOf(call) }));
    if (state === 'rejected' && n > 1) return fail(400);
    if (state === 'failed' && n > 1) return fail(500);
    if (state === 'refreshing' && n > 1) return gate.promise;
    if (state === 'storage-limit') return okBody(call, { ...compact, droppedBefore: AS_OF - 1800000, stored: 100000 });
    if (state === 'db-mixed') {
      return okBody(call, { ...compact, dbs: databases({ state: 'not_configured', ...NOT_IN_USE }, { state: 'stale', error: 'file_too_large', provider: 'ipinfo', database_type: 'IPinfo Lite' }) });
    }
    if (state === 'db-failed') {
      return okBody(call, { ...compact, dbs: databases({ database_type: 'GeoLite2-Country', provider: 'maxmind_geolite' }, { state: 'unavailable', error: 'invalid_format', ...NOT_IN_USE }) });
    }
    return okBody(call, compact);
  }, { timeoutSecs: 3600, pollSecs: 3600 });
  mount(api);
  if (state === 'loading') return;
  await waitFor(settled);
  if (['failed', 'refreshing', 'rejected'].includes(state)) host.querySelector('#sources-refresh').click();
  if (state === 'failed' || state === 'rejected') await waitFor(() => status() === t.stale || status() === t.failed);
}

const CASES = {
  'dash-028': case028,
  'dash-064': case064,
  'dash-082': case082,
  'dash-083': case083,
  'dash-084': case084,
  'dash-085': case085,
  'dash-086': case086,
  'dash-086-more': case086More,
  'dash-086-rejected': case086Rejected,
  'dash-086-busy': case086Busy,
  'dash-087': case087,
  'dash-087-language': case087Language,
  'dash-088': case088,
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
