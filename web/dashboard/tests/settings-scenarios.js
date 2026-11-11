// SCR-004/005 fake management API scenarios, DASH-028 through DASH-053.
// Each page runs one ?case=dash-0xx&lang=ko|en. Rust runs each at six widths.
// These are partial browser checks: real file writes, relay continuity, cookies,
// TLS, native keyboard navigation, and server atomicity require live-server checks.
import { mountSettings } from '../app/settings.js';
import { BRAND_NAME, COPY } from '../app/copy.js';
import { formatTime } from '../app/format.js';
import { STORAGE_KEY } from '../app/language.js';
import { checkHelpLanguageChange } from './help-link-checks.js';

const host = document.getElementById('host');
const result = document.getElementById('result');
const failures = [];
let cspViolations = 0;
document.addEventListener('securitypolicyviolation', () => { cspViolations += 1; });
const params = new URLSearchParams(location.search);
const lang = params.get('lang') === 'en' ? 'en' : 'ko';
const t = COPY[lang];
// Independent adopted CPY expectations, not implementation translation keys.
const WORDS = {
  ko: {
    view: '설정 보기', edit: '동적 설정 편집', settings: '설정', review: '변경 내용 확인', apply: '적용',
    in_effect: '설정 파일과 실행 중인 설정이 같습니다.',
    pending: '설정 파일에 재시작 없이 적용할 차이가 있습니다. 표시 값은 실행 중인 설정입니다.',
    restart_required: '설정 파일에 재시작이 필요한 차이가 있어 편집할 수 없습니다. 파일을 수정하거나 서버를 재시작한 뒤 다시 불러오십시오.',
    invalid: '설정 파일의 검증에 실패해 편집할 수 없습니다. 진단을 확인하고 파일을 수정한 뒤 다시 불러오십시오.',
    unreadable: '설정 파일을 읽지 못해 편집할 수 없습니다. 파일 접근을 복구한 뒤 다시 불러오십시오.',
    disabled: '편집 기능이 꺼져 있습니다. 설정 파일에서 management.config_edit = true로 바꾸고 서버를 재시작하십시오.',
    noHistory: '이 프로세스의 재로드 이력이 없습니다.', success: (n) => `설정 ${n}개를 적용했습니다.`,
    timing: '바뀐 값은 키별 다음 검사·요청부터 적용됩니다. 이미 끝난 검사·시작한 타이머·설정한 소켓 옵션은 이전 결과를 유지합니다. 기존 중계 세션은 계속 동작합니다.',
    confirm: '설정 변경 확인', previous: '이전 값', next: '새 값', added: '추가한 항목', removed: '뺀 항목',
    emptyWarning: '빈 허용 목록은 해당 키가 검사하는 모든 새 접속·요청을 거절합니다.',
    emptyTrustedProxiesWarning: '빈 trusted_proxies 목록은 전달 헤더를 신뢰하지 않습니다. 소켓 상대 IP로 클라이언트 주소를 검사합니다.',
    emptyAllowedOriginsWarning: '빈 allowed_origins 목록은 Origin이 있는 요청을 거절합니다. Origin이 없는 요청은 IP 검사를 하고, 그 역할에 접속 ticket 확인을 켰으면 ticket도 확인합니다.',
    stringLengthError: '1바이트 이상 256바이트 이하로 입력하십시오.',
    enableTicketWarning: '접속 ticket 확인을 켜면 이 역할의 WS·WSS 업그레이드 요청 가운데 유효한 ticket이 없는 요청을 거절합니다. 기존 중계 세션은 계속 동작합니다.',
    noChanges: '바뀐 설정이 없습니다.', saving: '설정을 저장하고 적용하는 중입니다.',
    conflict: '그사이 설정 파일이 바뀌었습니다. 다시 불러오면 입력 내용을 버리고 현재 설정으로 바꿉니다.',
    writeFailed: '설정 파일을 쓰지 못했습니다. 파일과 실행 값은 바뀌지 않았습니다. 입력을 확인한 뒤 다시 시도하십시오.',
    unknown: '적용 결과를 확인할 수 없습니다. 요청을 자동으로 다시 보내지 않습니다. 설정 보기에서 현재 값을 확인하십시오.',
    integer: '정수를 입력하십시오.', relation: 'limits.waiting은 limits.relay_connections 이하여야 합니다.',
    whitespace: (n) => `${n}줄: 항목에 공백을 넣지 마십시오.`, range: (min, max) => `${min} 이상 ${max} 이하로 입력하십시오.`,
    validation: '입력한 설정의 검증에 실패했습니다. 표시한 입력란을 확인하십시오.',
    line: (n, message) => `${n}줄: ${message}`,
    rejected: '요청을 처리하지 못했습니다. 화면을 다시 불러오십시오. 입력 내용은 버립니다.',
    dynamic: '재시작 없이 적용', restart: '재시작 필요', differs: '파일과 다름', empty: '빈 목록', null: '값 없음',
    applied: '적용', rejectedReload: '거절', fileWatch: '파일 변경 감지', configEdit: '설정 편집',
    groups: ['출발지 IP 허용 목록', '접근 정책', '연결 상한', '시간 제한', '연결 유지', '접속 ticket'],
    diagnostics: [
      '필수 설정 값이 없습니다.', '설정 값의 자료형이 맞지 않습니다.', '설정 값이 허용 범위를 벗어났습니다.',
      '설정 키 사이의 관계를 만족하지 않습니다.', 'IP·CIDR·범위 항목의 형식이 올바르지 않습니다.',
      'IP 범위의 시작 주소는 끝 주소 이하여야 합니다.', 'IP 범위의 두 주소는 모두 IPv4이거나 모두 IPv6여야 합니다.',
      'Origin 항목의 형식이 올바르지 않습니다.', 'Mode I 대상 항목의 형식이 올바르지 않습니다.',
    ],
  },
  en: {
    view: 'View settings', edit: 'Edit dynamic settings', settings: 'Settings', review: 'Review changes', apply: 'Apply',
    in_effect: 'The configuration file matches the settings in effect.',
    pending: 'The configuration file has changes that can be applied without a restart. The values shown are in effect now.',
    restart_required: 'The configuration file has changes that require a restart. Editing is blocked. Update the file or restart the server, then reload.',
    invalid: 'The configuration file failed validation. Editing is blocked. Check the diagnostics, update the file, then reload.',
    unreadable: 'The configuration file could not be read. Editing is blocked. Restore file access, then reload.',
    disabled: 'Editing is disabled. Set management.config_edit = true in the configuration file and restart the server.',
    noHistory: 'This process has no reload history.', success: (n) => `Applied ${n} settings.`,
    timing: 'Changed values apply from the next check or request for each key. Completed checks, started timers, and configured socket options keep their previous results. Existing relay sessions continue.',
    confirm: 'Confirm settings changes', previous: 'Previous value', next: 'New value', added: 'Added items', removed: 'Removed items',
    emptyWarning: 'An empty allow list rejects all new connections or requests checked by this key.',
    emptyTrustedProxiesWarning: 'An empty trusted_proxies list does not trust forwarded headers. The socket peer IP is used as the client address.',
    emptyAllowedOriginsWarning: 'An empty allowed_origins list rejects requests with Origin. Requests without Origin use the IP check, and also the ticket check when access tickets are on for the role.',
    stringLengthError: 'Enter 1 to 256 bytes.',
    enableTicketWarning: "Turning on access ticket checks rejects this role's WS and WSS upgrade requests that have no valid ticket. Existing relay sessions continue.",
    noChanges: 'No settings have changed.', saving: 'Saving and applying settings.',
    conflict: 'The configuration file has changed. Reloading discards your input and loads the current settings.',
    writeFailed: 'The configuration file could not be written. The file and settings in effect are unchanged. Review your input and try again.',
    unknown: 'The apply result cannot be confirmed. The request is not sent again automatically. Check the current values in View settings.',
    integer: 'Enter an integer.', relation: 'limits.waiting must not exceed limits.relay_connections.',
    whitespace: (n) => `Line ${n}: do not include whitespace in an item.`, range: (min, max) => `Enter a value from ${min} to ${max}.`,
    validation: 'The entered settings failed validation. Check the marked fields.',
    line: (n, message) => `Line ${n}: ${message}`,
    rejected: 'The request could not be processed. Reload this screen. Your input will be discarded.',
    dynamic: 'Applies without restart', restart: 'Restart required', differs: 'Differs from file', empty: 'Empty list', null: 'No value',
    applied: 'Applied', rejectedReload: 'Rejected', fileWatch: 'File watch', configEdit: 'Configuration edit',
    groups: ['Source IP allow lists', 'Access policies', 'Connection limits', 'Timeouts', 'Keepalive', 'Access tickets'],
    diagnostics: [
      'A required setting value is missing.', 'The setting value has an invalid type.', 'The setting value is outside its allowed range.',
      'The relationship between setting keys is not valid.', 'The IP, CIDR, or range item has an invalid format.',
      'The start address of an IP range must not exceed its end address.', 'Both addresses in an IP range must be IPv4 or both must be IPv6.',
      'The Origin item has an invalid format.', 'The Mode I target item has an invalid format.',
    ],
  },
};
const w = WORDS[lang];
const CODES = [
  'config_missing_key', 'config_type_mismatch', 'config_value_out_of_range', 'config_relation_violated',
  'config_ip_item_invalid', 'config_ip_range_reversed', 'config_ip_range_family_mixed', 'config_origin_invalid', 'config_target_item_invalid',
];
const GET = 'GET /api/v1/config';
const PATCH = 'PATCH /api/v1/config';
const SESSION = 'GET /api/v1/session';
const DELETE = 'DELETE /api/v1/session';
const AS_OF = '2026-11-08T05:00:00.000Z';
const NEXT_AS_OF = '2026-11-08T05:01:00.000Z';
const RELOADED = '2026-11-08T04:59:00.000Z';
const REV = `sha256:${'0'.repeat(64)}`;
const NEXT_REV = `sha256:${'1'.repeat(64)}`;
const LIST = 'relay.tcp_viewer.allow';
const WAITING = 'limits.waiting';
const LONG_PATH = `/settings/${'very-long-directory-name/'.repeat(9)}private-key.pem`;
const TICKET = 'relay.ticket.viewer';
const TICKET_FIELDS = ['enabled', 'issuer', 'relay_host', 'max_lifetime_secs'];
const ticketPublicPath = (role) => `/settings/${role}-ticket-public-keys.jwks`;
const WAIT = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
async function waitFor(ready, ms = 5000) {
  for (let steps = Math.ceil(ms / 20); steps > 0 && !ready(); steps -= 1) await WAIT(20);
}
const check = (label, passed) => { if (!passed) failures.push(label); };
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const json = (status, body) => new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });
const ok = (body) => json(200, body);
const fail = (status, code, extra = {}) => json(status, { error: { code, message: 'SERVER_ERROR_NOT_UI_COPY', ...extra } });
const session = (token = 'csrf-token-1') => ok({
  csrf_token: token,
  dashboard: { poll_secs: 1, query_timeout_secs: 1, terminate_timeout_secs: 10 },
});
const patchOk = (applied = [WAITING], extra = {}) => ok({ as_of: NEXT_AS_OF, result: 'applied', revision: NEXT_REV, applied, ...extra });
function deferred() {
  let resolve;
  const promise = new Promise((done) => { resolve = done; });
  return { promise, resolve };
}

// Full contract order: 75 keys, 43 reloadable (37 editable and six file paths).
function config(overrides = {}) {
  const items = [];
  const add = (key, value, reload = 'restart', editable = false, min, max) => {
    const item = { key, value, reload, editable };
    if (min !== undefined) Object.assign(item, { min, max });
    items.push(item);
  };
  add('admin.accounts_file', '/settings/accounts.toml');
  add('admin.argon2.memory_kib', 19456, 'restart', false, 19456, 1048576);
  add('admin.argon2.iterations', 2, 'restart', false, 2, 10);
  add('admin.argon2.parallelism', 1, 'restart', false, 1, 16);
  for (const name of ['tcp_viewer', 'tcp_server']) {
    const prefix = `relay.${name}`;
    add(`${prefix}.enabled`, true);
    add(`${prefix}.listen`, `127.0.0.1:${name === 'tcp_viewer' ? 5901 : 5500}`);
    add(`${prefix}.allow`, ['198.51.100.0/24'], 'dynamic', true);
  }
  for (const name of ['ws', 'wss']) {
    const prefix = `relay.${name}`;
    add(`${prefix}.listen`, `127.0.0.1:${name === 'ws' ? 8080 : 8443}`);
    add(`${prefix}.trusted_proxies`, ['192.0.2.10'], 'dynamic', true);
    if (name === 'wss') {
      add(`${prefix}.cert_file`, '/settings/certificate.pem', 'dynamic');
      add(`${prefix}.key_file`, LONG_PATH, 'dynamic');
    }
    for (const role of ['viewer', 'server']) {
      const rolePrefix = `${prefix}.${role}`;
      add(`${rolePrefix}.enabled`, true);
      add(`${rolePrefix}.allow`, ['198.51.100.0/24'], 'dynamic', true);
      add(`${rolePrefix}.allowed_origins`, [], 'dynamic', true);
    }
  }
  for (const role of ['viewer', 'server']) {
    const prefix = `relay.ticket.${role}`;
    add(`${prefix}.enabled`, false, 'dynamic', true);
    add(`${prefix}.issuer`, `${role}.issuer.example`, 'dynamic', true);
    add(`${prefix}.relay_host`, `${role}.relay.example`, 'dynamic', true);
    add(`${prefix}.public_keys_file`, ticketPublicPath(role), 'dynamic');
    add(`${prefix}.max_lifetime_secs`, 60, 'dynamic', true, 10, 600);
  }
  add('mode1.enabled', true);
  add('mode1.allow_targets', ['198.51.100.7:5900'], 'dynamic', true);
  add('mode2.enabled', true);
  for (const [key, value, min, max] of [
    ['limits.relay_connections', 1000, 2, 65535], ['limits.admin_connections', 64, 1, 1024],
    ['limits.waiting', 200, 1, 65535], ['limits.buffer_bytes', 131072, 16384, 4194304],
    ['limits.ws_message_bytes', 1048576, 65536, 16777216],
    ['timeouts.handshake_secs', 10, 1, 120], ['timeouts.tls_upgrade_secs', 10, 1, 120],
    ['timeouts.wait_secs', 3600, 10, 86400], ['timeouts.connect_secs', 10, 1, 120],
    ['timeouts.close_secs', 10, 1, 120], ['timeouts.shutdown_grace_secs', 30, 0, 600],
    ['keepalive.tcp_idle_secs', 60, 10, 3600], ['keepalive.tcp_interval_secs', 10, 1, 300],
    ['keepalive.tcp_probes', 6, 1, 20], ['keepalive.ws_ping_secs', 30, 5, 300], ['keepalive.ws_pong_timeout_secs', 30, 5, 300],
  ]) add(key, value, 'dynamic', true, min, max);
  add('recent_logs.max_entries', 1000, 'restart', false, 0, 10000);
  add('management.listen', '127.0.0.1:9443');
  add('management.cert_file', '/settings/admin-certificate.pem', 'dynamic');
  add('management.key_file', '/settings/admin-private-key.pem', 'dynamic');
  add('management.config_edit', true);
  for (const [key, value, min, max] of [
    ['management.login_limit.per_account', 5, 1, 100], ['management.login_limit.per_source', 20, 1, 100],
    ['management.login_limit.window_secs', 900, 60, 86400], ['management.login_limit.lockout_secs', 900, 60, 86400],
    ['management.session.idle_secs', 1800, 300, 28800], ['management.session.max_lifetime_secs', 43200, 3600, 259200],
    ['dashboard.poll_secs', 2, 1, 60], ['dashboard.query_timeout_secs', 5, 1, 60],
    ['recent_closed.max_entries', 100, 0, 10000],
  ]) add(key, value, 'restart', false, min, max);
  add('status.listen', '127.0.0.1:9080');
  add('log.output', 'stdout');
  add('reload.settle_secs', 5, 'restart', false, 1, 60);
  add('reload.poll_secs', 30, 'restart', false, 5, 600);
  return {
    as_of: AS_OF, revision: REV, edit_enabled: true,
    file: { state: 'in_effect', config_keys: [], diagnostics: [] },
    last_reload: { at: RELOADED, result: 'applied', trigger: 'file_watch', config_keys: [LIST] },
    items, ...overrides,
  };
}
const withValue = (data, key, value) => ({ ...data, items: data.items.map((item) => item.key === key ? { ...item, value } : item) });
const diagnostic = (code, key = `${LIST}[2]`) => ({ code, key, message: `SERVER_DIAGNOSTIC_${code}` });
const unknownDiagnostic = () => ({ code: 'future_config_diagnostic', key: 'future.setting', message: 'Unknown diagnostic text <script>not markup</script>' });

// Normal requests abort like fetch. The disposal cases deliberately ignore cancellation
// so late fulfillment also exercises the page's epoch guard.
function fakeApi(overrides = {}, { ignoreAbort = false } = {}) {
  const routes = {
    [SESSION]: () => session(), [GET]: () => ok(config()), [PATCH]: () => patchOk(),
    [DELETE]: () => new Response(null, { status: 204 }), ...overrides,
  };
  const calls = [];
  const counts = new Map();
  const request = (path, init = {}) => {
    const key = `${init.method ?? 'GET'} ${path}`;
    const n = (counts.get(key) ?? 0) + 1;
    counts.set(key, n);
    const headers = new Headers(init.headers);
    calls.push({ key, at: performance.now(), body: init.body ? JSON.parse(init.body) : null,
      csrf: headers.get('X-CSRF-Token'), poll: headers.has('X-Relay-Poll'),
      contentType: headers.get('Content-Type'), credentials: init.credentials, cache: init.cache, signal: init.signal,
      raw: `${path} ${JSON.stringify(init)}` });
    return new Promise((resolve, reject) => {
      const abort = () => reject(new DOMException('The operation was aborted.', 'AbortError'));
      if (!ignoreAbort && init.signal?.aborted) { abort(); return; }
      if (!ignoreAbort) init.signal?.addEventListener('abort', abort, { once: true });
      Promise.resolve().then(() => routes[key] ? routes[key](n, init) : fail(404, 'unexpected_test_route'))
        .then(resolve, reject)
        .finally(() => init.signal?.removeEventListener('abort', abort));
    });
  };
  return { request, calls, count: (key) => counts.get(key) ?? 0 };
}
const untilAbort = (_n, init) => new Promise((_resolve, reject) => {
  const abort = () => reject(new DOMException('The operation was aborted.', 'AbortError'));
  if (init.signal.aborted) abort();
  else init.signal.addEventListener('abort', abort, { once: true });
});
const storageOf = (items) => ({ getItem: (key) => items.get(key) ?? null, setItem: (key, value) => items.set(key, String(value)) });
let current = null;
function mount(api = fakeApi(), { edit = false, applied = null, storage = storageOf(new Map()), preferred = [lang], follow = false } = {}) {
  current?.handle.dispose();
  const root = document.createElement('div');
  host.replaceChildren(root);
  const navigations = [];
  const mounted = { root, api, navigations, handle: null };
  const deps = {
    request: api.request, storage, preferredLanguages: preferred,
    navigate: (url, options = {}) => {
      navigations.push({ url, options });
      if (follow && (url === '/#/settings' || url === '/#/settings/edit')) {
        mounted.handle.dispose();
        mounted.handle = mountSettings(root, deps, { edit: url.endsWith('/edit'), applied: options.applied ?? null });
      }
    },
  };
  mounted.handle = mountSettings(root, deps, { edit, applied });
  current = mounted;
  return mounted;
}
const text = (root) => root.textContent;
const has = (root, copy) => text(root).includes(copy);
const inputOf = (root, key) => root.querySelector(`input[data-config-key="${key}"], textarea[data-config-key="${key}"]`);
const inputsOf = (root) => [...root.querySelectorAll('input[data-config-key], textarea[data-config-key]')];
const rowOf = (root, key) => root.querySelector(`[data-config-key="${key}"]`);
const buttonOf = (root, id) => root.querySelector(`#settings-${id}`);
const active = (node) => node !== null && !node.disabled;
const canReview = (root) => active(buttonOf(root, 'review'));
const dialogOf = (root) => root.querySelector('dialog.dialog');
const confirmOf = (root) => dialogOf(root)?.querySelector('.dialog-actions button:last-child');
const cancelOf = (root) => dialogOf(root)?.querySelector('.dialog-actions button:first-child');
const titleOf = (root) => root.querySelector('h1.page-title')?.textContent.trim();
const changeOf = (root, key) => [...root.querySelectorAll('.config-change')]
  .find((card) => card.querySelector('.card-title')?.textContent === key);
const comparisonOf = (card) => [...(card?.querySelectorAll('.config-body > span') ?? [])].map((node) => node.textContent);
function setValue(root, key, value) {
  const input = inputOf(root, key);
  if (!input) throw new Error(`missing input: ${key}`);
  input.value = value;
  input.dispatchEvent(new Event('input', { bubbles: true }));
  input.dispatchEvent(new Event('change', { bubbles: true }));
  return input;
}
function setChecked(root, key, checked) {
  const input = inputOf(root, key);
  if (!input || input.type !== 'checkbox') throw new Error(`missing checkbox: ${key}`);
  if (input.checked !== checked) input.click();
  return input;
}
async function ready(mounted, edit = false) {
  await waitFor(() => edit ? canReview(mounted.root) : rowOf(mounted.root, WAITING) !== null);
  check('configuration loaded', edit ? canReview(mounted.root) : rowOf(mounted.root, WAITING) !== null);
}
async function review(root) {
  buttonOf(root, 'review').click();
  await waitFor(() => dialogOf(root)?.open);
  check('review dialog opens', dialogOf(root)?.open === true);
}
async function apply(root) {
  await review(root);
  confirmOf(root).click();
}
function noApply(label, root) {
  check(`${label}: no editable fields`, inputsOf(root).length === 0);
  check(`${label}: review disabled or removed`, !canReview(root));
  check(`${label}: apply disabled or removed`, !active(confirmOf(root) ?? null));
}
function checkLayout(label, root) {
  check(`${label}: no horizontal page scroll`, document.documentElement.scrollWidth <= document.documentElement.clientWidth);
  const nodes = root.querySelectorAll('input, textarea, .navbar-item, dialog[open], .alert');
  for (const node of nodes) {
    const box = node.getBoundingClientRect();
    if (!box.width || !box.height) continue;
    check(`${label}: ${node.id || node.className} remains inside viewport`, box.left >= -1 && box.right <= innerWidth + 1);
  }
}
function checkDiagnostic(label, root, code, index) {
  check(`${label}: ${code} localized`, has(root, w.diagnostics[index]));
  check(`${label}: ${code} server wording hidden`, !has(root, `SERVER_DIAGNOSTIC_${code}`));
}
function checkAria(label, root, key) {
  const input = inputOf(root, key);
  check(`${label}: input has invalid state`, input?.getAttribute('aria-invalid') === 'true');
  const references = input?.getAttribute('aria-describedby')?.split(/\s+/).filter(Boolean) ?? [];
  check(`${label}: error linked`, references.some((id) => id.endsWith('-error')) && references.every((id) => root.querySelector(`[id="${id}"]`) !== null));
  check(`${label}: visible key label linked`, [...root.querySelectorAll('label')].some((label) => label.htmlFor === input?.id && label.textContent.includes(key)));
  check(`${label}: error summary announced`, [...root.querySelectorAll('[role="alert"], [aria-live]')].some((node) => node.textContent.trim().length > 0));
}

async function case028() {
  for (const edit of [false, true]) {
    const m = mount(fakeApi(), { edit });
    await ready(m, edit);
    const links = [...m.root.querySelectorAll('nav a')];
    const selected = links.filter((link) => link.getAttribute('aria-current') === 'page');
    check('DASH-028: settings active in both settings screens', selected.length === 1 && selected[0].textContent.includes(w.settings));
    check('DASH-028: operations link available', links.some((link) => link.textContent.includes(t.operations)));
    check('DASH-028: both nav labels visible at every width', links.filter((link) => link.textContent.includes(t.operations) || link.textContent.includes(w.settings)).every((link) => link.getBoundingClientRect().width > 0));
    checkLayout('DASH-028', m.root);
    const unauth = mount(fakeApi({ [SESSION]: () => fail(401, 'unauthenticated') }), { edit });
    await waitFor(() => unauth.navigations.length > 0);
    check('DASH-028: direct settings entry requires session', same(unauth.navigations.map((n) => n.url), ['/login/']));
    check('DASH-028: no config before authentication', unauth.api.count(GET) === 0);
    check('DASH-028: no protected data before authentication', rowOf(unauth.root, WAITING) === null && inputsOf(unauth.root).length === 0);
  }
}
async function case029() {
  let data = config();
  data = withValue(data, 'relay.tcp_server.enabled', false);
  data = withValue(data, 'relay.tcp_server.listen', null);
  const m = mount(fakeApi({ [GET]: () => ok(data) }));
  await ready(m);
  const rows = [...m.root.querySelectorAll('[data-config-key]')];
  check('DASH-029: all contract keys in order', same(rows.map((row) => row.dataset.configKey), data.items.map((item) => item.key)));
  check('DASH-029: fixture has all 75 contract keys', data.items.length === 75);
  check('DASH-029: fixture has all 43 reloadable keys', data.items.filter((item) => item.reload === 'dynamic').length === 43);
  check('DASH-029: fixture has all 37 potential editable keys', data.items.filter((item) => item.editable).length === 37);
  const sections = [...m.root.querySelectorAll('.card-title')].map((node) => node.textContent);
  const ticketSections = ['[relay.ticket.viewer]', '[relay.ticket.server]'];
  check('DASH-029: ticket sections follow WS/WSS and precede Mode I', ticketSections.every((section, index) =>
    sections.indexOf(section) === sections.indexOf('[relay.wss.server]') + index + 1) &&
    sections.indexOf('[mode1]') === sections.indexOf(ticketSections[1]) + 1);
  for (const role of ['viewer', 'server']) {
    const ticketRows = rows.filter((row) => row.dataset.configKey.startsWith(`relay.ticket.${role}.`));
    check(`DASH-029 ${role}: five ticket keys in contract order`, same(ticketRows.map((row) => row.dataset.configKey),
      ['enabled', 'issuer', 'relay_host', 'public_keys_file', 'max_lifetime_secs'].map((key) => `relay.ticket.${role}.${key}`)));
    const path = rowOf(m.root, `relay.ticket.${role}.public_keys_file`);
    check(`DASH-029 ${role}: public key file is path only`, path?.querySelector('.config-value')?.textContent === ticketPublicPath(role));
  }
  for (const item of data.items) {
    const row = rowOf(m.root, item.key);
    const values = item.value === null ? [w.null] : Array.isArray(item.value) ? item.value.length ? item.value : [w.empty] : [String(item.value)];
    check(`DASH-029: ${item.key} exact value`, values.every((value) => row?.textContent.includes(value)));
    check(`DASH-029: ${item.key} application`, row?.textContent.includes(item.reload === 'dynamic' ? w.dynamic : w.restart));
  }
  check('DASH-029: full long key path visible', has(m.root, LONG_PATH));
  check('DASH-029: no file content or editable controls', !has(m.root, 'BEGIN PRIVATE KEY') &&
    !has(m.root, 'BEGIN PUBLIC KEY') && !has(m.root, '"kty"') && inputsOf(m.root).length === 0);
  check('DASH-029: as_of local time and zone', has(m.root, formatTime(AS_OF)));
  check('DASH-029: reload local time and zone', has(m.root, formatTime(RELOADED)));
  check('DASH-029: reload result and trigger localized', has(m.root, w.applied) && has(m.root, w.fileWatch));
  check('DASH-029: title and brand', titleOf(m.root) === w.view && document.title === `${BRAND_NAME} ${w.view}`);
  check('DASH-029: table sections', [...m.root.querySelectorAll('.card-title')].some((node) => node.textContent.includes('limits')));
  checkLayout('DASH-029', m.root);
}
async function case030() {
  for (const state of ['in_effect', 'pending']) {
    const gate = deferred();
    const data = config({ file: { state, config_keys: state === 'pending' ? [WAITING] : [], diagnostics: [] } });
    const api = fakeApi({ [GET]: (n) => n === 1 ? ok(data) : gate.promise });
    const m = mount(api);
    await ready(m);
    check(`DASH-030 ${state}: state notice`, has(m.root, w[state]));
    check(`DASH-030 ${state}: editing enabled`, active(buttonOf(m.root, 'edit')));
    check(`DASH-030 ${state}: difference only on changed row`, rowOf(m.root, WAITING)?.textContent.includes(w.differs) === (state === 'pending'));
    await WAIT(1150);
    check(`DASH-030 ${state}: no automatic polling`, api.count(GET) === 1);
    buttonOf(m.root, 'refresh').click();
    buttonOf(m.root, 'refresh').click();
    await waitFor(() => api.count(GET) === 2);
    check(`DASH-030 ${state}: duplicate refresh blocked`, api.count(GET) === 2);
    check(`DASH-030 ${state}: no poll header`, api.calls.filter((call) => call.key === GET).every((call) => !call.poll));
    gate.resolve(ok(data));
    await WAIT(30);
    checkLayout('DASH-030', m.root);
  }
}
async function case031() {
  for (const edit of [false, true]) {
    const data = config({ file: { state: 'restart_required', config_keys: ['management.listen'], diagnostics: [] } });
    const m = mount(fakeApi({ [GET]: () => ok(data) }), { edit });
    await waitFor(() => has(m.root, w.restart_required));
    check('DASH-031: restart-required cause and key', has(m.root, w.restart_required) && has(m.root, 'management.listen'));
    check('DASH-031: no edit', !active(buttonOf(m.root, 'edit')));
    if (edit) noApply('DASH-031', m.root);
    else check('DASH-031: execution value retained', rowOf(m.root, 'management.listen')?.textContent.includes('127.0.0.1:9443'));
    checkLayout('DASH-031', m.root);
  }
}
async function case032() {
  for (const edit of [false, true]) {
    for (const state of ['invalid', 'unreadable']) {
      const diagnostics = state === 'invalid' ? [...CODES.map((code) => diagnostic(code, WAITING)), unknownDiagnostic()] : [];
      const m = mount(fakeApi({ [GET]: () => ok(config({ revision: state === 'unreadable' ? null : REV, file: { state, config_keys: [], diagnostics } })) }), { edit });
      await waitFor(() => has(m.root, w[state]));
      check(`DASH-032 ${state}: correct notice`, has(m.root, w[state]) && !has(m.root, w[state === 'invalid' ? 'unreadable' : 'invalid']));
      if (state === 'invalid') {
        CODES.forEach((code, index) => checkDiagnostic('DASH-032', m.root, code, index));
        check('DASH-032: unknown diagnostic includes original and code', has(m.root, unknownDiagnostic().message) && has(m.root, unknownDiagnostic().code));
        check('DASH-032: server text is inert', m.root.querySelector('script') === null);
      }
      check('DASH-032: edit unavailable', !active(buttonOf(m.root, 'edit')));
      if (edit) noApply('DASH-032', m.root);
      else check('DASH-032: execution values remain', rowOf(m.root, WAITING)?.textContent.includes('200'));
      checkLayout('DASH-032', m.root);
    }
  }
}
async function case033() {
  const m = mount(fakeApi({ [GET]: () => ok(config({ last_reload: null })) }));
  await ready(m);
  check('DASH-033: no-history text', has(m.root, w.noHistory));
  check('DASH-033: no fake zero timestamp', !has(m.root, '1970-01-01') && !has(m.root, 'Invalid Date'));
  for (const [resultValue, expected, trigger, expectedTrigger] of [
    ['rejected', w.rejectedReload, 'config_edit', w.configEdit],
    ['restart_required', w.restart, 'file_watch', w.fileWatch],
    ['future_result', 'future_result', 'future_trigger', 'future_trigger'],
  ]) {
    const other = mount(fakeApi({ [GET]: () => ok(config({ last_reload: { at: RELOADED, result: resultValue, trigger, config_keys: [WAITING] } })) }));
    await ready(other);
    check('DASH-033: reload result and trigger preserved/localized', has(other.root, expected) && has(other.root, expectedTrigger));
    check('DASH-033: history independent from current file state', has(other.root, w.in_effect));
  }
}
async function case034() {
  for (const edit of [false, true]) {
    const m = mount(fakeApi({ [GET]: () => ok(config({ edit_enabled: false })) }), { edit });
    await waitFor(() => has(m.root, w.disabled));
    check('DASH-034: setting and restart instruction', has(m.root, w.disabled));
    check('DASH-034: edit unavailable', !active(buttonOf(m.root, 'edit')));
    if (edit) noApply('DASH-034', m.root);
    else check('DASH-034: view still available', rowOf(m.root, WAITING) !== null);
  }
}
async function case035() {
  for (const [role, listener] of [['viewer', 'ws'], ['server', 'wss']]) {
    const data = config();
    for (const item of data.items) {
      const wsRole = /^relay\.(ws|wss)\.(viewer|server)\./.exec(item.key);
      const disabled = item.key.startsWith('relay.tcp_server.') || item.key.startsWith('mode1.') ||
        (wsRole && (wsRole[1] !== listener || wsRole[2] !== role)) ||
        (item.key.startsWith(`relay.${listener === 'ws' ? 'wss' : 'ws'}.`) && !wsRole);
      if (disabled) {
        item.editable = false;
        item.value = item.key.endsWith('.enabled') ? false : null;
      }
      if (item.key.startsWith('relay.ticket.') && !item.key.startsWith(`relay.ticket.${role}.`)) item.editable = false;
    }
    const m = mount(fakeApi({ [GET]: () => ok(data) }), { edit: true });
    await ready(m, true);
    const inputs = inputsOf(m.root);
    const expectedKeys = [
      (key) => /^relay\..+\.allow$/.test(key),
      (key) => key.endsWith('.trusted_proxies'),
      (key) => key.endsWith('.allowed_origins') || key === 'mode1.allow_targets',
      (key) => key.startsWith('limits.'), (key) => key.startsWith('timeouts.'), (key) => key.startsWith('keepalive.'),
      (key) => key.startsWith('relay.ticket.'),
    ].flatMap((belongs) => data.items.filter((item) => item.editable && belongs(item.key)).map((item) => item.key));
    check(`DASH-035 ${role}: exactly editable keys in six groups`, same(inputs.map((node) => node.dataset.configKey), expectedKeys));
    const sections = [...m.root.querySelectorAll('.card')].filter((card) => card.querySelector('[data-config-key]'));
    check(`DASH-035 ${role}: all six section titles in order`, same(sections.map((card) => card.querySelector('.card-title')?.textContent), w.groups));
    check(`DASH-035 ${role}: only enabled role's four ticket inputs last`, same(inputs.slice(-4).map((node) => node.dataset.configKey),
      TICKET_FIELDS.map((key) => `relay.ticket.${role}.${key}`)) &&
      inputs.filter((node) => node.dataset.configKey.startsWith('relay.ticket.')).length === 4);
    check('DASH-035: no TLS/public key paths or restart key inputs', !inputs.some((node) =>
      /cert_file|key_file|public_keys_file|management\./.test(node.dataset.configKey) ||
      (node.dataset.configKey.endsWith('.enabled') && !node.dataset.configKey.startsWith('relay.ticket.'))));
    for (const input of inputs) {
      const item = data.items.find((item) => item.key === input.dataset.configKey);
      check('DASH-035: stable input ID', input.id === `config-${item.key.replaceAll('.', '-')}`);
      check('DASH-035: visible label includes key', [...m.root.querySelectorAll('label')].some((label) => label.htmlFor === input.id && label.textContent.includes(item.key)));
      if (typeof item.value === 'boolean') check('DASH-035: enabled uses an unchecked boolean checkbox', input.type === 'checkbox' && input.checked === false);
      else {
        check('DASH-035: readable minimum input font', parseFloat(getComputedStyle(input).fontSize) >= 16);
        if (typeof item.value === 'string') check('DASH-035: ticket string is a single-line text input', input.tagName === 'INPUT' && input.type === 'text');
      }
      const help = input.getAttribute('aria-describedby')?.split(/\s+/).map((id) => document.getElementById(id)?.textContent ?? '').join(' ') ?? '';
      if (Array.isArray(item.value) || typeof item.value === 'number') check('DASH-035: connected help', help.length > 0);
      if (item.min !== undefined) check('DASH-035: min/max in help', help.includes(String(item.min)) && help.includes(String(item.max)));
    }
    check('DASH-035: timing guidance', has(m.root, w.timing));
    checkLayout('DASH-035', m.root);
  }
}
async function case036() {
  const m = mount(fakeApi(), { edit: true });
  await ready(m, true);
  for (const value of ['', '1.5', '1e2', '1x', '-1', '65536']) {
    setValue(m.root, WAITING, value);
    buttonOf(m.root, 'review').click();
    await WAIT(20);
    check(`DASH-036 ${value}: no dialog or request`, !dialogOf(m.root)?.open && m.api.count(PATCH) === 0);
    check(`DASH-036 ${value}: error focused`, document.activeElement === inputOf(m.root, WAITING));
    checkAria(`DASH-036 ${value}`, m.root, WAITING);
    check(`DASH-036 ${value}: proper message`, has(m.root, ['-1', '65536'].includes(value) ? w.range(1, 65535) : w.integer));
  }
  setValue(m.root, WAITING, '1001');
  buttonOf(m.root, 'review').click();
  await WAIT(20);
  check('DASH-036: waiting/relay relation', has(m.root, w.relation));
  setValue(m.root, WAITING, '200');
  for (const line of [' ', ' 192.0.2.1', '192.0.2.1 ', '192.0. 2.1', '\t']) {
    setValue(m.root, LIST, `192.0.2.1\n\n${line}`);
    buttonOf(m.root, 'review').click();
    await WAIT(20);
    check('DASH-036: whitespace rejected at original line', has(m.root, w.whitespace(3)));
    check('DASH-036: first displayed field focused', document.activeElement === inputOf(m.root, LIST));
    checkAria('DASH-036 whitespace', m.root, LIST);
    const input = inputOf(m.root, LIST);
    check('DASH-036: whitespace line selected', input.selectionStart === '192.0.2.1\n\n'.length && input.selectionEnd === input.value.length);
  }
  setValue(m.root, WAITING, '');
  buttonOf(m.root, 'review').click();
  await WAIT(20);
  check('DASH-036: simultaneous errors focus first screen field', document.activeElement === inputOf(m.root, LIST));
  check('DASH-036: simultaneous errors both connected', inputOf(m.root, LIST)?.getAttribute('aria-invalid') === 'true' && inputOf(m.root, WAITING)?.getAttribute('aria-invalid') === 'true');
  setValue(m.root, WAITING, '200');
  setValue(m.root, LIST, '\n192.0.2.1\n\n198.51.100.7\n');
  await review(m.root);
  check('DASH-036: blank lines alone ignored', m.api.count(PATCH) === 0);
  cancelOf(m.root).click();
  checkLayout('DASH-036', m.root);

  const tickets = mount(fakeApi(), { edit: true });
  await ready(tickets, true);
  const issuer = `${TICKET}.issuer`;
  const relayHost = `${TICKET}.relay_host`;
  setValue(tickets.root, issuer, '');
  setValue(tickets.root, relayHost, 'a'.repeat(257));
  buttonOf(tickets.root, 'review').click();
  await WAIT(20);
  for (const key of [issuer, relayHost]) {
    checkAria(`DASH-036 ticket ${key}`, tickets.root, key);
    check(`DASH-036 ${key}: adopted CPY-193 field error`, inputOf(tickets.root, key)?.closest('.config-field')?.textContent.includes(w.stringLengthError));
  }
  check('DASH-036: empty issuer and 257-byte host focus first error', document.activeElement === inputOf(tickets.root, issuer));
  check('DASH-036: string errors prevent dialog and PATCH', !dialogOf(tickets.root)?.open && tickets.api.count(PATCH) === 0);
  setValue(tickets.root, issuer, 'viewer.issuer.example');
  for (const value of ['a'.repeat(257), `${'가'.repeat(85)}ab`, `${'😀'.repeat(64)}a`]) {
    setValue(tickets.root, relayHost, value);
    buttonOf(tickets.root, 'review').click();
    await WAIT(20);
    check('DASH-036: 257 UTF-8 bytes rejected regardless of character count', has(tickets.root, w.stringLengthError) &&
      inputOf(tickets.root, relayHost)?.getAttribute('aria-invalid') === 'true');
    check('DASH-036: UTF-8 error focuses host without dialog or PATCH', document.activeElement === inputOf(tickets.root, relayHost) &&
      !dialogOf(tickets.root)?.open && tickets.api.count(PATCH) === 0);
  }
  for (const value of [' ', 'a'.repeat(256), `${'가'.repeat(85)}a`, '😀'.repeat(64)]) {
    setValue(tickets.root, issuer, value);
    setValue(tickets.root, relayHost, value);
    await review(tickets.root);
    for (const key of [issuer, relayHost]) {
      check(`DASH-036 ${key}: 1..256 bytes retained without trimming`, comparisonOf(changeOf(tickets.root, key))[1] === value);
    }
    check('DASH-036: valid byte boundaries do not PATCH before confirmation', tickets.api.count(PATCH) === 0);
    cancelOf(tickets.root).click();
  }
  setValue(tickets.root, issuer, ' issuer with spaces ');
  setValue(tickets.root, relayHost, ' relay.example ');
  const enabled = `${TICKET}.enabled`;
  check('DASH-036: checkbox starts unchecked', inputOf(tickets.root, enabled)?.checked === false);
  setChecked(tickets.root, enabled, true);
  check('DASH-036: checkbox can be selected', inputOf(tickets.root, enabled)?.checked === true);
  setChecked(tickets.root, enabled, false);
  check('DASH-036: checkbox can be cleared', inputOf(tickets.root, enabled)?.checked === false);
  await review(tickets.root);
  check('DASH-036: toggling checkbox back is unchanged', changeOf(tickets.root, enabled) === undefined);
  cancelOf(tickets.root).click();
  setChecked(tickets.root, enabled, true);
  await review(tickets.root);
  check('DASH-036: checkbox comparison is false to true', same(comparisonOf(changeOf(tickets.root, enabled)), ['false', 'true']));
  checkLayout('DASH-036 ticket review', tickets.root);
  confirmOf(tickets.root).click();
  await waitFor(() => tickets.api.count(PATCH) === 1);
  const ticketChanges = tickets.api.calls.find((call) => call.key === PATCH)?.body?.changes;
  check('DASH-036: PATCH preserves strings and sends actual boolean', same(ticketChanges, {
    [enabled]: true, [issuer]: ' issuer with spaces ', [relayHost]: ' relay.example ',
  }));

  let nullData = config();
  for (const role of ['viewer', 'server']) {
    for (const key of ['issuer', 'relay_host']) nullData = withValue(nullData, `relay.ticket.${role}.${key}`, null);
  }
  const unchanged = mount(fakeApi({ [GET]: () => ok(nullData) }), { edit: true });
  await ready(unchanged, true);
  for (const role of ['viewer', 'server']) {
    for (const key of ['issuer', 'relay_host']) {
      const fullKey = `relay.ticket.${role}.${key}`;
      check('DASH-036: null ticket string starts empty', inputOf(unchanged.root, fullKey)?.value === '');
      setValue(unchanged.root, fullKey, '');
    }
  }
  await review(unchanged.root);
  check('DASH-036: null plus empty is unchanged without length error', has(dialogOf(unchanged.root), w.noChanges) &&
    unchanged.root.querySelectorAll('.config-change').length === 0 && !has(unchanged.root, w.stringLengthError));
  confirmOf(unchanged.root).click();
  await waitFor(() => unchanged.navigations.length > 0);
  check('DASH-036: null plus empty never sends PATCH', unchanged.api.count(PATCH) === 0 &&
    unchanged.navigations.some((navigation) => navigation.url === '/#/settings'));
}
async function case037() {
  const baseline = withValue(withValue(config(), 'relay.ws.viewer.allowed_origins', ['https://viewer.example']), 'relay.ticket.server.enabled', true);
  const m = mount(fakeApi({ [GET]: () => ok(baseline) }), { edit: true });
  await ready(m, true);
  setValue(m.root, LIST, '192.0.2.7\n198.51.100.8');
  setValue(m.root, 'relay.tcp_server.allow', '');
  setValue(m.root, 'relay.ws.trusted_proxies', '');
  setValue(m.root, 'relay.ws.viewer.allowed_origins', '');
  setValue(m.root, WAITING, '250');
  setChecked(m.root, `${TICKET}.enabled`, true);
  setChecked(m.root, 'relay.ticket.server.enabled', false);
  setValue(m.root, `${TICKET}.issuer`, ' next issuer ');
  setValue(m.root, `${TICKET}.relay_host`, 'next.relay.example');
  setValue(m.root, `${TICKET}.max_lifetime_secs`, '90');
  await review(m.root);
  const dialog = dialogOf(m.root);
  check('DASH-037: confirmation title', dialog.textContent.includes(w.confirm));
  check('DASH-037: before and after labels', dialog.textContent.includes(w.previous) && dialog.textContent.includes(w.next));
  check('DASH-037: old/new numbers and changed keys', [LIST, 'relay.tcp_server.allow', WAITING, '200', '250'].every((part) => dialog.textContent.includes(part)));
  check('DASH-037: added/deleted list values', [w.added, w.removed, '192.0.2.7', '198.51.100.8', '198.51.100.0/24'].every((part) => dialog.textContent.includes(part)));
  const allowChange = changeOf(m.root, 'relay.tcp_server.allow');
  const proxiesChange = changeOf(m.root, 'relay.ws.trusted_proxies');
  const originsChange = changeOf(m.root, 'relay.ws.viewer.allowed_origins');
  check('DASH-037: empty allow warning belongs to allow entry', has(allowChange, w.emptyWarning));
  check('DASH-037: empty trusted proxy warning has its distinct meaning', has(proxiesChange, w.emptyTrustedProxiesWarning) && !has(proxiesChange, w.emptyWarning));
  check('DASH-037: adopted CPY-192 belongs to empty Origin entry', has(originsChange, w.emptyAllowedOriginsWarning) && !has(originsChange, w.emptyWarning));
  const enableChange = changeOf(m.root, `${TICKET}.enabled`);
  const disableChange = changeOf(m.root, 'relay.ticket.server.enabled');
  check('DASH-037: boolean changes compare exact false/true values', same(comparisonOf(enableChange), ['false', 'true']) &&
    same(comparisonOf(disableChange), ['true', 'false']));
  check('DASH-037: adopted CPY-194 is inside false-to-true change only', has(enableChange, w.enableTicketWarning) &&
    !has(disableChange, w.enableTicketWarning) &&
    [...dialog.querySelectorAll('.alert')].filter((alert) => has(alert, w.enableTicketWarning)).length === 1);
  for (const change of [originsChange, enableChange]) {
    const warning = change.querySelector('.alert-body > p');
    check('DASH-037: standalone warnings use adopted body typography', getComputedStyle(warning).fontWeight === '400' &&
      getComputedStyle(warning).color === getComputedStyle(dialog).color);
  }
  check('DASH-037: issuer string diff preserves surrounding spaces', same(comparisonOf(changeOf(m.root, `${TICKET}.issuer`)),
    ['viewer.issuer.example', ' next issuer ']));
  check('DASH-037: relay host string diff', same(comparisonOf(changeOf(m.root, `${TICKET}.relay_host`)),
    ['viewer.relay.example', 'next.relay.example']));
  check('DASH-037: ticket lifetime numeric diff', same(comparisonOf(changeOf(m.root, `${TICKET}.max_lifetime_secs`)), ['60', '90']));
  check('DASH-037: existing connections/timers/options guidance', dialog.textContent.includes(w.timing));
  check('DASH-037: no unchanged key in comparison', !dialog.textContent.includes('keepalive.tcp_probes'));
  check('DASH-037: no request before confirmation', m.api.count(PATCH) === 0);
  checkLayout('DASH-037', m.root);
  confirmOf(m.root).click();
  await waitFor(() => m.api.count(PATCH) === 1);
  const changes = m.api.calls.find((call) => call.key === PATCH)?.body?.changes;
  const expected = {
    [LIST]: ['192.0.2.7', '198.51.100.8'], 'relay.tcp_server.allow': [],
    'relay.ws.trusted_proxies': [], 'relay.ws.viewer.allowed_origins': [], [WAITING]: 250,
    [`${TICKET}.enabled`]: true, [`${TICKET}.issuer`]: ' next issuer ',
    [`${TICKET}.relay_host`]: 'next.relay.example', [`${TICKET}.max_lifetime_secs`]: 90,
    'relay.ticket.server.enabled': false,
  };
  check('DASH-037: PATCH contains only changed keys with boolean/string/list/number types', changes !== undefined &&
    same(Object.keys(changes).sort(), Object.keys(expected).sort()) &&
    Object.entries(expected).every(([key, value]) => same(changes[key], value)));
}
async function case038() {
  const m = mount(fakeApi(), { edit: true });
  await ready(m, true);
  setValue(m.root, WAITING, '250');
  for (const escape of [false, true]) {
    buttonOf(m.root, 'review').focus();
    await review(m.root);
    check('DASH-038: cancel is initial focus', document.activeElement === cancelOf(m.root));
    confirmOf(m.root).focus();
    confirmOf(m.root).dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true }));
    check('DASH-038: Tab wraps inside dialog', document.activeElement === cancelOf(m.root));
    cancelOf(m.root).dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true, cancelable: true }));
    check('DASH-038: Shift+Tab wraps inside dialog', document.activeElement === confirmOf(m.root));
    if (escape) {
      // Model Escape's cancel/default-close sequence. Native key dispatch remains a live-browser check.
      const dialog = dialogOf(m.root);
      if (dialog.dispatchEvent(new Event('cancel', { cancelable: true }))) dialog.close();
    } else cancelOf(m.root).click();
    await WAIT(20);
    check('DASH-038: cancel closes dialog', !dialogOf(m.root)?.open);
    check('DASH-038: input preserved', inputOf(m.root, WAITING)?.value === '250');
    check('DASH-038: focus returns to caller', document.activeElement === buttonOf(m.root, 'review'));
    check('DASH-038: no PATCH', m.api.count(PATCH) === 0);
  }
}
async function case039() {
  const gate = deferred();
  const api = fakeApi({ [GET]: (n) => ok(n === 1 ? config() : withValue(config({ revision: NEXT_REV, as_of: NEXT_AS_OF }), WAITING, 250)), [PATCH]: () => gate.promise });
  const m = mount(api, { edit: true, follow: true });
  await ready(m, true);
  setValue(m.root, WAITING, '250');
  setValue(m.root, LIST, '198.51.100.0/24\n\n192.0.2.7\n');
  await apply(m.root);
  await waitFor(() => api.count(PATCH) === 1);
  check('DASH-039: busy state announced', has(m.root, w.saving));
  check('DASH-039: all inputs frozen', inputsOf(m.root).every((input) => input.disabled || input.readOnly));
  check('DASH-039: review/apply unavailable while saving', !canReview(m.root) && !active(confirmOf(m.root) ?? null));
  buttonOf(m.root, 'review')?.click();
  confirmOf(m.root)?.click();
  const call = api.calls.find((call) => call.key === PATCH);
  check('DASH-039: revision and only changed typed keys', call?.body?.revision === REV
    && same(Object.keys(call.body.changes).sort(), [LIST, WAITING].sort())
    && same(call.body.changes[LIST], ['198.51.100.0/24', '192.0.2.7']) && call.body.changes[WAITING] === 250);
  check('DASH-039: CSRF, JSON, cookie and no-store', call?.csrf === 'csrf-token-1' && call.contentType === 'application/json' && call.credentials === 'same-origin' && call.cache === 'no-store');
  check('DASH-039: PATCH never marked poll', call?.poll === false);
  gate.resolve(patchOk([WAITING]));
  await waitFor(() => titleOf(m.root) === w.view && rowOf(m.root, WAITING)?.textContent.includes('250') && has(m.root, w.success(1)));
  check('DASH-039: routed to settings with actual applied count', same(m.navigations, [{ url: '/#/settings', options: { applied: 1 } }]));
  check('DASH-039: fresh GET and new effective value', api.count(GET) === 2 && rowOf(m.root, WAITING)?.textContent.includes('250'));
  check('DASH-039: actual applied length, not submitted length', has(m.root, w.success(1)) && !has(m.root, w.success(2)));
  check('DASH-039: one PATCH only', api.count(PATCH) === 1);
  check('DASH-039: title focused after apply', document.activeElement === m.root.querySelector('h1.page-title'));
}
async function case040() {
  for (const [index, code] of CODES.entries()) {
    const key = index === 2 || index === 3 ? WAITING : index === 7 ?
      'relay.ws.viewer.allowed_origins' : index === 8 ? 'mode1.allow_targets' : LIST;
    const lineDiagnostic = index >= 4;
    const serverKey = lineDiagnostic ? `${key}[2]` : key;
    const api = fakeApi({ [PATCH]: () => fail(400, 'config_invalid', { diagnostics: [diagnostic(code, serverKey)] }) });
    const m = mount(api, { edit: true });
    await ready(m, true);
    const value = key === WAITING ? '250' : index === 7 ?
      'https://one.example.test\n\nhttps://two.example.test\nhttps://three.example.test' :
      index === 8 ? '192.0.2.1:5900\n\n198.51.100.2:5900\n203.0.113.3:5900' :
        '192.0.2.1\n\n198.51.100.2\n203.0.113.3';
    setValue(m.root, key, value);
    await apply(m.root);
    await waitFor(() => has(m.root, w.validation));
    checkDiagnostic('DASH-040', m.root, code, index);
    const input = inputOf(m.root, key);
    check('DASH-040: first error focused', document.activeElement === input);
    if (lineDiagnostic) {
      check('DASH-040: original line number', has(m.root, w.line(4, w.diagnostics[index])));
      const start = value.lastIndexOf('\n') + 1;
      check('DASH-040: line 4 selected', input.selectionStart === start && input.selectionEnd === value.length);
    }
    check('DASH-040: original input unchanged', input?.value === value);
    checkAria('DASH-040', m.root, key);
    check('DASH-040: no success/no automatic resend', !has(m.root, w.success(1)) && api.count(PATCH) === 1 && m.navigations.length === 0);
    checkLayout('DASH-040', m.root);
  }
  const unknown = unknownDiagnostic();
  const m = mount(fakeApi({ [PATCH]: () => fail(400, 'config_invalid', { diagnostics: [unknown, diagnostic(CODES[4], `${LIST}[99]`)] }) }), { edit: true });
  await ready(m, true);
  setValue(m.root, LIST, '192.0.2.1');
  await apply(m.root);
  await waitFor(() => has(m.root, unknown.message));
  check('DASH-040: unknown key/code/message in top notice', has(m.root, unknown.key) && has(m.root, unknown.code) && has(m.root, unknown.message));
  check('DASH-040: out-of-range server index does not crash', has(m.root, w.validation) && inputOf(m.root, LIST)?.value === '192.0.2.1');
  check('DASH-040: diagnostic not interpreted as HTML', m.root.querySelector('script') === null);
  const unknownLine = { ...unknownDiagnostic(), key: `${LIST}[2]` };
  const line = mount(fakeApi({ [PATCH]: () => fail(400, 'config_invalid', { diagnostics: [unknownLine] }) }), { edit: true });
  await ready(line, true);
  const value = '192.0.2.1\n\n198.51.100.2\n203.0.113.3';
  setValue(line.root, LIST, value);
  await apply(line.root);
  await waitFor(() => has(line.root, unknownLine.message));
  check('DASH-040: unknown diagnostic uses original message, code and source line', has(line.root, unknownLine.message) && has(line.root, unknownLine.code) && has(line.root, lang === 'ko' ? '4줄:' : 'Line 4:'));
  check('DASH-040: unknown diagnostic selects source line', inputOf(line.root, LIST)?.selectionStart === value.lastIndexOf('\n') + 1 && inputOf(line.root, LIST)?.selectionEnd === value.length);
}
async function conflict() {
  const api = fakeApi({
    [GET]: (n) => ok(n === 1 ? config() : withValue(config({ revision: NEXT_REV, as_of: NEXT_AS_OF }), WAITING, 300)),
    [PATCH]: (n) => n === 1 ? fail(409, 'config_conflict') : patchOk(),
  });
  const m = mount(api, { edit: true });
  await ready(m, true);
  setValue(m.root, WAITING, '250');
  await apply(m.root);
  await waitFor(() => has(m.root, w.conflict));
  return m;
}
async function case041() {
  const m = await conflict();
  check('DASH-041: conflict and input-discard warning', has(m.root, w.conflict));
  check('DASH-041: draft retained', inputOf(m.root, WAITING)?.value === '250');
  check('DASH-041: reload available', active(buttonOf(m.root, 'reload')));
  await WAIT(100);
  buttonOf(m.root, 'review')?.click();
  confirmOf(m.root)?.click();
  check('DASH-041: no immediate resend or automatic GET', m.api.count(PATCH) === 1 && m.api.count(GET) === 1);
}
async function case042() {
  const m = await conflict();
  buttonOf(m.root, 'reload').click();
  await waitFor(() => inputOf(m.root, WAITING)?.value === '300');
  check('DASH-042: explicit reload replaces draft and revision', m.api.count(GET) === 2 && inputOf(m.root, WAITING)?.value === '300');
  check('DASH-042: clears conflict and restores review', !has(m.root, w.conflict) && canReview(m.root));
  setValue(m.root, WAITING, '350');
  await apply(m.root);
  await waitFor(() => m.api.count(PATCH) === 2);
  check('DASH-042: subsequent explicit request uses new revision', m.api.calls.filter((call) => call.key === PATCH)[1]?.body.revision === NEXT_REV);

  // A timed-out query may still finish after recovery if the transport ignores abort.
  const late = deferred();
  const stale = mount(fakeApi({ [GET]: (n) => n === 1 ? late.promise : ok(withValue(config({ revision: NEXT_REV }), WAITING, 400)) }), { edit: true });
  await waitFor(() => has(stale.root, t.failed), 2500);
  buttonOf(stale.root, 'reload')?.click();
  if (stale.api.count(GET) === 1) buttonOf(stale.root, 'refresh')?.click();
  await waitFor(() => inputOf(stale.root, WAITING)?.value === '400');
  late.resolve(ok(config()));
  await WAIT(50);
  check('DASH-042: late stale GET cannot replace recovery', inputOf(stale.root, WAITING)?.value === '400');
}
async function case043() {
  const outcomes = [
    [403, 'config_edit_disabled', {}, w.disabled],
    [409, 'config_restart_pending', { config_keys: ['management.listen'] }, w.restart_required],
    [409, 'config_file_invalid', { diagnostics: [...CODES.map((code) => diagnostic(code, WAITING)), unknownDiagnostic()] }, w.invalid],
    [409, 'config_file_invalid', { diagnostics: [] }, w.unreadable],
  ];
  for (const [status, code, extra, expected] of outcomes) {
    const api = fakeApi({
      [PATCH]: () => fail(status, code, extra),
      [GET]: (n) => ok(n > 1 && code === 'config_file_invalid' && !extra.diagnostics.length ? config({ revision: null, file: { state: 'unreadable', config_keys: [], diagnostics: [] } }) : config()),
    });
    const m = mount(api, { edit: true });
    await ready(m, true);
    setValue(m.root, WAITING, '250');
    await apply(m.root);
    await waitFor(() => has(m.root, expected));
    check(`DASH-043 ${code}: exact cause`, has(m.root, expected));
    noApply(`DASH-043 ${code}`, m.root);
    if (extra.config_keys) check('DASH-043: restart key shown', has(m.root, extra.config_keys[0]));
    if (extra.diagnostics?.length) {
      CODES.forEach((diagnosticCode, index) => checkDiagnostic('DASH-043', m.root, diagnosticCode, index));
      check('DASH-043: unknown diagnostic includes code and original', has(m.root, unknownDiagnostic().code) && has(m.root, unknownDiagnostic().message));
    }
    check('DASH-043: no success/no resend', m.navigations.length === 0 && api.count(PATCH) === 1);
    checkLayout('DASH-043', m.root);
  }
}
async function writeFailure() {
  const m = mount(fakeApi({ [PATCH]: (n) => n === 1 ? fail(500, 'config_write_failed') : patchOk() }), { edit: true, follow: true });
  await ready(m, true);
  setValue(m.root, WAITING, '250');
  await apply(m.root);
  await waitFor(() => has(m.root, w.writeFailed));
  return m;
}
async function case044() {
  const m = await writeFailure();
  check('DASH-044: unchanged-file/execution warning', has(m.root, w.writeFailed));
  check('DASH-044: input retained', inputOf(m.root, WAITING)?.value === '250');
  check('DASH-044: no fake success', m.navigations.length === 0 && !has(m.root, w.success(1)));
  await WAIT(100);
  check('DASH-044: no auto resend', m.api.count(PATCH) === 1);
}
async function case045() {
  const m = await writeFailure();
  await review(m.root);
  check('DASH-045: explicit re-confirm before retry', m.api.count(PATCH) === 1);
  confirmOf(m.root).click();
  await waitFor(() => has(m.root, w.success(1)));
  check('DASH-045: one explicit retry and returns to view', m.api.count(PATCH) === 2 && titleOf(m.root) === w.view && has(m.root, w.success(1)));
}
async function unknownResult(route) {
  const m = mount(fakeApi({ [PATCH]: route }), { edit: true, follow: true });
  await ready(m, true);
  setValue(m.root, WAITING, '250');
  await apply(m.root);
  await waitFor(() => has(m.root, w.unknown), 2500);
  return m;
}
async function case046() {
  const routes = [
    ['no response', () => { throw new TypeError('connection lost'); }],
    ['timeout', untilAbort], ['unknown server error', () => fail(503, 'future_server_error')],
    ['unknown success result', () => patchOk([WAITING], { result: 'future_result' })],
    ['malformed success response', () => new Response('not JSON', { status: 200 })],
  ];
  for (const [label, route] of routes) {
    const m = await unknownResult(route);
    check(`DASH-046 ${label}: unknown outcome`, has(m.root, w.unknown));
    check(`DASH-046 ${label}: no success or failure assumption`, !has(m.root, w.writeFailed) && !has(m.root, w.success(1)) && m.navigations.length === 0);
    check(`DASH-046 ${label}: resubmission disabled`, !canReview(m.root));
    check(`DASH-046 ${label}: view action offered`, buttonOf(m.root, 'view') !== null);
    buttonOf(m.root, 'review')?.click();
    confirmOf(m.root)?.click();
    await WAIT(50);
    check(`DASH-046 ${label}: no re-send or automatic read`, m.api.count(PATCH) === 1 && m.api.count(GET) === 1);
    if (label === 'timeout') check('DASH-046: configured query timeout aborts PATCH', m.api.calls.find((call) => call.key === PATCH)?.signal.aborted === true);
  }
  for (const [status, code] of [[400, 'invalid_request'], [409, 'future_client_error'], [403, 'future_forbidden']]) {
    const m = mount(fakeApi({ [PATCH]: () => fail(status, code) }), { edit: true });
    await ready(m, true);
    setValue(m.root, WAITING, '250');
    await apply(m.root);
    await waitFor(() => has(m.root, w.rejected));
    check(`DASH-046 ${code}: HTTP client category rejected, not success`, has(m.root, w.rejected) && !has(m.root, w.unknown) && m.navigations.length === 0);
    check(`DASH-046 ${code}: retains input`, inputOf(m.root, WAITING)?.value === '250');
    check(`DASH-046 ${code}: no resend`, m.api.count(PATCH) === 1);
  }
}
async function case047() {
  const data = withValue(config({ revision: NEXT_REV, as_of: NEXT_AS_OF,
    file: { state: 'pending', config_keys: [LIST], diagnostics: [] },
    last_reload: { at: NEXT_AS_OF, result: 'applied', trigger: 'config_edit', config_keys: [WAITING] },
  }), WAITING, 250);
  const api = fakeApi({ [GET]: (n) => ok(n === 1 ? config() : data), [PATCH]: () => { throw new TypeError('response lost'); } });
  const m = mount(api, { edit: true, follow: true });
  await ready(m, true);
  setValue(m.root, WAITING, '250');
  await apply(m.root);
  await waitFor(() => has(m.root, w.unknown));
  buttonOf(m.root, 'view').click();
  await waitFor(() => titleOf(m.root) === w.view && rowOf(m.root, WAITING) !== null);
  check('DASH-047: view re-reads actual effective value', api.count(GET) === 2 && rowOf(m.root, WAITING)?.textContent.includes('250'));
  check('DASH-047: current file and reload result shown', has(m.root, w.pending) && has(m.root, w.configEdit) && has(m.root, formatTime(NEXT_AS_OF)));
  check('DASH-047: unknown outcome never claims applied count', !has(m.root, w.success(1)));
  check('DASH-047: PATCH never resent', api.count(PATCH) === 1);
}
async function case048() {
  for (const edit of [false, true]) {
    for (const route of [() => fail(500, 'internal'), () => { throw new TypeError('offline'); }, untilAbort]) {
      const gate = deferred();
      const m = mount(fakeApi({ [GET]: async (n, init) => {
        if (n === 1) await gate.promise;
        return route(n, init);
      } }), { edit });
      await waitFor(() => m.api.count(GET) === 1);
      check('DASH-048: loading is visible before first response', has(m.root, t.loading));
      check('DASH-048: no fabricated values or inputs while loading', rowOf(m.root, WAITING) === null && inputsOf(m.root).length === 0);
      gate.resolve();
      await waitFor(() => has(m.root, t.failed), 2500);
      check('DASH-048: initial GET failure distinguished from empty config', has(m.root, t.failed) && rowOf(m.root, WAITING) === null);
      check('DASH-048: failed read blocks edit/review', !active(buttonOf(m.root, 'edit')) && !canReview(m.root));
    }
    const api = fakeApi({ [GET]: (n) => n === 1 ? ok(config()) : fail(500, 'future_get_error') });
    const stale = mount(api, { edit });
    await ready(stale, edit);
    buttonOf(stale.root, 'refresh').click();
    await waitFor(() => has(stale.root, t.failed));
    check('DASH-048: stale values keep last success timestamp', has(stale.root, formatTime(AS_OF)));
    check('DASH-048: stale state explicitly marked', has(stale.root, t.staleMessage.replace('{time}', formatTime(AS_OF))));
    check('DASH-048: stale read blocks edit/apply', !active(buttonOf(stale.root, 'edit')) && !canReview(stale.root));
    if (!edit) check('DASH-048: previous effective value retained', rowOf(stale.root, WAITING)?.textContent.includes('200'));
  }
}
async function case049() {
  for (const edit of [false, true]) {
    for (const initial of ['query_failed', 'invalid', 'unreadable', 'restart_required']) {
      const api = fakeApi({ [GET]: (n) => n === 1 ? initial === 'query_failed' ? fail(500, 'internal') : ok(config({
        revision: initial === 'unreadable' ? null : REV,
        file: { state: initial, config_keys: initial === 'restart_required' ? ['management.listen'] : [], diagnostics: initial === 'invalid' ? [diagnostic(CODES[0], WAITING)] : [] },
      })) : ok(withValue(config({ revision: NEXT_REV, as_of: NEXT_AS_OF }), WAITING, 350)) });
      const m = mount(api, { edit });
      await waitFor(() => initial === 'query_failed' ? has(m.root, t.failed) : has(m.root, w[initial]));
      const trigger = buttonOf(m.root, 'reload') ?? buttonOf(m.root, 'refresh');
      trigger.click();
      await waitFor(() => edit ? inputOf(m.root, WAITING)?.value === '350' : rowOf(m.root, WAITING)?.textContent.includes('350'));
      check(`DASH-049 ${initial}: explicit GET recovers`, api.count(GET) === 2);
      check(`DASH-049 ${initial}: new timestamp shown`, has(m.root, formatTime(NEXT_AS_OF)));
      check(`DASH-049 ${initial}: actions restored`, edit ? canReview(m.root) : active(buttonOf(m.root, 'edit')));
      check(`DASH-049 ${initial}: stale/error cleared`, !has(m.root, t.failed) && !has(m.root, t.stale));
    }
  }
  // csrf_invalid does not mean editing is disabled and never automatically resends.
  const api = fakeApi({ [SESSION]: (n) => session(n === 1 ? 'csrf-old' : 'csrf-new'), [PATCH]: (n) => n === 1 ? fail(403, 'csrf_invalid') : patchOk() });
  const m = mount(api, { edit: true });
  await ready(m, true);
  setValue(m.root, WAITING, '250');
  await apply(m.root);
  await waitFor(() => has(m.root, w.rejected));
  check('DASH-049 csrf: rejection retains input and does not disable feature', inputOf(m.root, WAITING)?.value === '250' && !has(m.root, w.disabled));
  check('DASH-049 csrf: no automatic PATCH retry', api.count(PATCH) === 1);
  buttonOf(m.root, 'reload').click();
  await waitFor(() => api.count(SESSION) >= 2 && canReview(m.root) && inputOf(m.root, WAITING)?.value === '200');
  setValue(m.root, WAITING, '250');
  await apply(m.root);
  await waitFor(() => api.count(PATCH) === 2);
  check('DASH-049 csrf: fresh token after explicit reload/review', api.calls.filter((call) => call.key === PATCH)[1]?.csrf === 'csrf-new');
}
async function case050() {
  const saved = new Map();
  const storage = storageOf(saved);
  const longItems = Array.from({ length: 20 }, (_, i) => `https://host-${i}.${'long-domain-label.'.repeat(5)}example.test:8443`);
  const data = withValue(config(), 'relay.ws.viewer.allowed_origins', longItems);
  const m = mount(fakeApi({ [GET]: () => ok(data), [PATCH]: () => fail(400, 'config_invalid', { diagnostics: [diagnostic(CODES[7], 'relay.ws.viewer.allowed_origins[1]')] }) }), { edit: true, storage });
  await ready(m, true);
  const value = `${longItems.join('\n')}\nhttps://new.example.test`;
  setValue(m.root, 'relay.ws.viewer.allowed_origins', value);
  setValue(m.root, WAITING, '250');
  const reads = m.api.count(GET);
  const other = lang === 'ko' ? 'en' : 'ko';
  const select = m.root.querySelector('#settings-language');
  select.value = other;
  select.dispatchEvent(new Event('change'));
  await WAIT(30);
  check('DASH-050: language/document/title changed immediately', document.documentElement.lang === other && titleOf(m.root) === WORDS[other].edit);
  check('DASH-050: choice stored', saved.get(STORAGE_KEY) === other);
  check('DASH-050: draft survives language change', inputOf(m.root, WAITING)?.value === '250' && inputOf(m.root, 'relay.ws.viewer.allowed_origins')?.value === value);
  check('DASH-050: language does not trigger GET', m.api.count(GET) === reads);
  await review(m.root);
  const dialog = dialogOf(m.root);
  check('DASH-050: dialog accessible title and description', Boolean(dialog.getAttribute('aria-labelledby')) && Boolean(dialog.getAttribute('aria-describedby')));
  for (const attribute of ['aria-labelledby', 'aria-describedby']) check('DASH-050: dialog reference resolves', dialog.getAttribute(attribute).split(/\s+/).every((id) => m.root.querySelector(`[id="${id}"]`) !== null));
  check('DASH-050: long list not omitted in review', dialog.textContent.includes(longItems[19]) && dialog.textContent.includes('https://new.example.test'));
  check('DASH-050: dialog fits viewport height', dialog.getBoundingClientRect().height <= innerHeight);
  checkLayout('DASH-050 other language dialog', m.root);
  cancelOf(m.root).click();
  await apply(m.root);
  await waitFor(() => has(m.root, WORDS[other].validation));
  check('DASH-050: diagnostics use current language', has(m.root, WORDS[other].diagnostics[7]));
  checkAria('DASH-050', m.root, 'relay.ws.viewer.allowed_origins');
  const back = m.root.querySelector('#settings-language');
  back.value = lang;
  back.dispatchEvent(new Event('change'));
  await WAIT(30);
  check('DASH-050: error state retranslated without data loss', has(m.root, w.diagnostics[7]) && inputOf(m.root, 'relay.ws.viewer.allowed_origins')?.value === value);
  check('DASH-050: no API language parameters/headers', m.api.calls.every((call) => !/lang/i.test(call.raw)));
  checkLayout('DASH-050 diagnostics', m.root);
  const view = mount(fakeApi({ [GET]: () => ok(data) }), { storage });
  await ready(view);
  const language = view.root.querySelector('#settings-language');
  language.value = other;
  language.dispatchEvent(new Event('change'));
  await WAIT(20);
  check('DASH-050: view values preserved on language change', rowOf(view.root, 'relay.ws.viewer.allowed_origins')?.textContent.includes(longItems[19]));
  check('DASH-050: view document language updated', document.documentElement.lang === other && titleOf(view.root) === WORDS[other].view);
  checkLayout('DASH-050 view', view.root);
}
async function case051() {
  for (const edit of [false, true]) {
    const expired = mount(fakeApi({ [GET]: (n) => n === 1 ? ok(config()) : fail(401, 'unauthenticated') }), { edit });
    await ready(expired, edit);
    if (edit) setValue(expired.root, WAITING, '250');
    buttonOf(expired.root, 'refresh').click();
    await waitFor(() => expired.navigations.length > 0);
    check('DASH-051 expiry: login and protected data removed', same(expired.navigations.map((entry) => entry.url), ['/login/']) && expired.root.childElementCount === 0);

    for (const [route, target] of [
      [() => new Response(null, { status: 204 }), '/login/'], [() => fail(401, 'unauthenticated'), '/login/'],
      [() => fail(403, 'csrf_invalid'), '/login/?notice=logout-unconfirmed'], [() => fail(500, 'internal'), '/login/?notice=logout-unconfirmed'],
      [() => { throw new TypeError('logout response lost'); }, '/login/?notice=logout-unconfirmed'],
    ]) {
      const late = deferred();
      const api = fakeApi({ [DELETE]: route, [GET]: (n) => n === 1 ? ok(config()) : late.promise }, { ignoreAbort: true });
      const m = mount(api, { edit });
      await ready(m, edit);
      buttonOf(m.root, 'refresh').click();
      await waitFor(() => api.count(GET) === 2);
      const logout = buttonOf(m.root, 'logout');
      logout.click();
      logout.click();
      await waitFor(() => m.navigations.length > 0);
      late.resolve(ok(withValue(config(), WAITING, 999)));
      await WAIT(30);
      check('DASH-051 logout: standard login outcome', same(m.navigations.map((entry) => entry.url), [target]));
      check('DASH-051 logout: data and late response removed', m.root.childElementCount === 0);
      check('DASH-051 logout: one CSRF-authenticated DELETE', api.count(DELETE) === 1 && api.calls.find((call) => call.key === DELETE)?.csrf === 'csrf-token-1');
      check('DASH-051 logout: no save or relay termination', api.count(PATCH) === 0 && api.calls.every((call) => !call.key.includes('/terminate')));
    }
  }
  const duringSave = deferred();
  const m = mount(fakeApi({ [PATCH]: () => duringSave.promise }, { ignoreAbort: true }), { edit: true });
  await ready(m, true);
  setValue(m.root, WAITING, '250');
  await apply(m.root);
  await waitFor(() => m.api.count(PATCH) === 1);
  buttonOf(m.root, 'logout').click();
  await waitFor(() => m.navigations.length > 0);
  duringSave.resolve(patchOk());
  await WAIT(30);
  check('DASH-051: late save after logout never navigates to settings', same(m.navigations.map((entry) => entry.url), ['/login/']) && m.root.childElementCount === 0);
  const unauthorizedPatch = mount(fakeApi({ [PATCH]: () => fail(401, 'unauthenticated') }), { edit: true });
  await ready(unauthorizedPatch, true);
  setValue(unauthorizedPatch.root, WAITING, '250');
  await apply(unauthorizedPatch.root);
  await waitFor(() => unauthorizedPatch.navigations.length > 0);
  check('DASH-051: PATCH expiry also clears draft', unauthorizedPatch.root.childElementCount === 0 && unauthorizedPatch.navigations[0]?.url === '/login/');
}
async function case052() {
  const noChange = mount(fakeApi(), { edit: true, follow: true });
  await ready(noChange, true);
  setValue(noChange.root, LIST, '\n198.51.100.0/24\n\n');
  setValue(noChange.root, WAITING, '0200');
  await review(noChange.root);
  check('DASH-052: normalized no-change review', has(noChange.root, w.noChanges));
  confirmOf(noChange.root).click();
  await waitFor(() => titleOf(noChange.root) === w.view && rowOf(noChange.root, WAITING) !== null);
  check('DASH-052: no PATCH for equivalent values', noChange.api.count(PATCH) === 0);
  check('DASH-052: zero applied and fresh view GET', has(noChange.root, w.success(0)) && noChange.api.count(GET) === 2);
  const emptyApplied = mount(fakeApi({ [PATCH]: () => patchOk([]) }), { edit: true, follow: true });
  await ready(emptyApplied, true);
  setValue(emptyApplied.root, WAITING, '250');
  await apply(emptyApplied.root);
  await waitFor(() => has(emptyApplied.root, w.success(0)));
  check('DASH-052: empty applied response is zero success', titleOf(emptyApplied.root) === w.view && has(emptyApplied.root, w.success(0)) && emptyApplied.api.count(PATCH) === 1);
  const failedRead = mount(fakeApi({ [GET]: (n) => n === 1 ? ok(config()) : fail(500, 'internal') }), { edit: true, follow: true });
  await ready(failedRead, true);
  setValue(failedRead.root, WAITING, '250');
  await apply(failedRead.root);
  await waitFor(() => has(failedRead.root, t.failed));
  check('DASH-052: subsequent GET failure does not revoke known save', has(failedRead.root, w.success(1)) && has(failedRead.root, t.failed));
}
async function case053() {
  const m = mount(fakeApi(), { edit: true, follow: true });
  await ready(m, true);
  setValue(m.root, WAITING, '250');
  buttonOf(m.root, 'cancel').click();
  await waitFor(() => titleOf(m.root) === w.view && rowOf(m.root, WAITING) !== null);
  check('DASH-053: cancel goes directly to view without save', titleOf(m.root) === w.view && m.api.count(PATCH) === 0 && !dialogOf(m.root)?.open);
  check('DASH-053: abandoned input not shown as effective', rowOf(m.root, WAITING)?.textContent.includes('200'));
  // Router link handling belongs to router-scenarios; here dispose is its page boundary.
  for (const saving of [false, true]) {
    const late = deferred();
    const api = fakeApi({ [GET]: () => saving ? ok(config()) : late.promise, [PATCH]: () => late.promise }, { ignoreAbort: true });
    const old = mount(api, { edit: true });
    if (saving) {
      await ready(old, true);
      setValue(old.root, WAITING, '250');
      await apply(old.root);
      await waitFor(() => api.count(PATCH) === 1);
    } else await waitFor(() => api.count(GET) === 1);
    old.handle.dispose();
    old.root.replaceChildren(document.createTextNode('NEW_SCREEN_SENTINEL'));
    late.resolve(saving ? patchOk() : ok(config()));
    await WAIT(50);
    check('DASH-053: disposed response does not overwrite next screen', old.root.textContent === 'NEW_SCREEN_SENTINEL');
    check('DASH-053: disposed save does not navigate', old.navigations.length === 0);
    check('DASH-053: disposed request aborted', api.calls.find((call) => call.key === (saving ? PATCH : GET))?.signal.aborted === true);
  }
}

// Persistent real-component states for the F-C frame comparison.
async function shot(frame) {
  const edit = frame.startsWith('scr005-') || ['edit', 'confirm'].includes(frame);
  const state = frame.replace(/^scr00[45]-/, '');
  const gate = deferred();
  let baseline = withValue(config(), 'relay.ws.viewer.allowed_origins', ['https://viewer.example']);
  if (state === 'disabled') baseline.edit_enabled = false;
  const fileState = ({ pending: 'pending', restart: 'restart_required', blocked: 'invalid',
    invalid: 'invalid', unreadable: 'unreadable', 'restart-blocked': 'restart_required',
    'unreadable-blocked': 'unreadable' })[state];
  if (fileState) baseline.file = { state: fileState, config_keys: fileState === 'restart_required' ? ['management.listen'] : fileState === 'pending' ? [WAITING] : [],
    diagnostics: fileState === 'invalid' ? [diagnostic('config_ip_item_invalid')] : [] };
  if (state === 'no-reload') baseline.last_reload = null;
  const outcomes = {
    'server-error': () => fail(400, 'config_invalid', { diagnostics: [diagnostic('config_ip_item_invalid')] }),
    conflict: () => fail(409, 'config_conflict'), 'write-failed': () => fail(500, 'config_write_failed'),
    unknown: () => { throw new TypeError('response lost'); },
    'request-rejected': () => fail(400, 'invalid_request'), saving: () => gate.promise,
  };
  const api = fakeApi({
    [SESSION]: () => ok({ csrf_token: 'shot-token',
      dashboard: { poll_secs: 3600, query_timeout_secs: 3600, terminate_timeout_secs: 3600 } }),
    [GET]: (n) => state === 'loading' ? gate.promise : state === 'expired' ? fail(401, 'unauthenticated')
      : state === 'failed' || (state === 'failed-stale' && n > 1) ? fail(500, 'internal') : ok(baseline),
    [PATCH]: outcomes[state] ?? (() => patchOk()),
  });
  const m = mount(api, { edit, applied: state === 'success' ? 3 : null });
  if (state === 'loading') return;
  if (state === 'expired') {
    await waitFor(() => m.navigations.length > 0);
    const { mountLogin } = await import('../login/login.js');
    mountLogin(m.root, { storage: storageOf(new Map([[STORAGE_KEY, lang]])), search: '' });
    return;
  }
  if (fileState || state === 'disabled' || state === 'failed') {
    await waitFor(() => m.root.querySelector('.alert') !== null);
    return;
  }
  await ready(m, edit);
  if (state === 'failed-stale') {
    buttonOf(m.root, 'refresh').click();
    await waitFor(() => has(m.root, t.failed));
  } else if (state === 'client-error') {
    setValue(m.root, LIST, 'not an ip with spaces');
    setValue(m.root, `${TICKET}.issuer`, '');
    setValue(m.root, `${TICKET}.relay_host`, 'a'.repeat(257));
    buttonOf(m.root, 'review').click();
  } else if (state === 'confirm' || state === 'unchanged') {
    if (state === 'confirm') {
      for (const key of ['relay.tcp_viewer.allow', 'relay.ws.trusted_proxies', 'relay.ws.viewer.allowed_origins']) setValue(m.root, key, '');
      setChecked(m.root, `${TICKET}.enabled`, true);
      setValue(m.root, `${TICKET}.issuer`, 'next.issuer.example');
      setValue(m.root, `${TICKET}.relay_host`, 'next.relay.example');
    }
    await review(m.root);
  } else if (outcomes[state]) {
    setValue(m.root, LIST, '192.0.2.7');
    await apply(m.root);
    if (state !== 'saving') await waitFor(() => !dialogOf(m.root)?.open);
  }
}

async function case064() {
  for (const edit of [false, true]) {
    const m = mount(fakeApi(), { edit });
    await ready(m, edit);
    if (edit) setValue(m.root, WAITING, '250');
    checkHelpLanguageChange(m.root, lang, check);
    check('DASH-064: no help API requests', m.api.calls.every((call) => call.key.startsWith('GET /api/v1/')));
    check('DASH-064: current screen remains mounted', m.navigations.length === 0);
    if (edit) check('DASH-064: draft preserved', inputOf(m.root, WAITING)?.value === '250');
  }
  current.handle.dispose();
}

const CASES = {
  'dash-028': case028, 'dash-029': case029, 'dash-030': case030, 'dash-031': case031,
  'dash-032': case032, 'dash-033': case033, 'dash-034': case034, 'dash-035': case035,
  'dash-036': case036, 'dash-037': case037, 'dash-038': case038, 'dash-039': case039,
  'dash-040': case040, 'dash-041': case041, 'dash-042': case042, 'dash-043': case043,
  'dash-044': case044, 'dash-045': case045, 'dash-046': case046, 'dash-047': case047,
  'dash-048': case048, 'dash-049': case049, 'dash-050': case050, 'dash-051': case051,
  'dash-052': case052, 'dash-053': case053,
  'dash-064': case064,
};
const caseName = params.get('case');
try {
  if (caseName?.startsWith('shot-')) await shot(caseName.slice(5));
  else {
    const run = CASES[caseName];
    if (!run) failures.push(`unknown case: ${caseName}`);
    else await run();
  }
} catch (error) {
  failures.push(`${caseName} threw: ${error?.message ?? error}`);
} finally {
  if (!caseName?.startsWith('shot-')) current?.handle.dispose();
}
check('no CSP violations', cspViolations === 0);
result.dataset.result = failures.length === 0 ? 'pass' : 'fail';
result.dataset.failures = failures.join('\n');
result.dataset.cspViolations = String(cspViolations);
