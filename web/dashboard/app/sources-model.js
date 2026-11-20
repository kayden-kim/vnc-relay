// SCR-008 sources: the response check and the small rules behind the summary, the IP information database rows, and
// the three tables. No DOM here. Rules come from dashboard.md SCR-008 and management API contract 1.17.

import { ApiError } from './api.js';

const TIMESTAMP = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/;

// Summary card keys in CPY-260 order.
export const SUMMARY_KEYS = ['attempts', 'rejected', 'aborted', 'sessions', 'addresses', 'countries', 'asns'];

// Database rows in response order, with the configuration key of CPY-269.
export const DATABASE_ROWS = [
  { key: 'country', configKey: 'ip_info.country_db' },
  { key: 'asn', configKey: 'ip_info.asn_db' },
];

// Providers that carry an attribution (CPY-276~278). `maxmind`, `unknown`, and unknown values have none.
const ATTRIBUTED = new Set(['dbip', 'ipinfo', 'maxmind_geolite']);

function invalid() {
  return new ApiError('invalid');
}

function object(value) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw invalid();
  }
  return value;
}

function array(value) {
  if (!Array.isArray(value)) {
    throw invalid();
  }
  return value;
}

function integer(value, min = 0) {
  if (!Number.isInteger(value) || value < min) {
    throw invalid();
  }
  return value;
}

function text(value) {
  if (typeof value !== 'string') {
    throw invalid();
  }
  return value;
}

function nullable(value, check) {
  return value === null ? null : check(value);
}

function boolean(value) {
  if (typeof value !== 'boolean') {
    throw invalid();
  }
  return value;
}

// A contract timestamp as a Date.
function timestamp(value) {
  if (typeof value !== 'string' || !TIMESTAMP.test(value)) {
    throw invalid();
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    throw invalid();
  }
  return date;
}

function counts(raw) {
  return {
    attempts: integer(raw.attempts),
    rejected: integer(raw.rejected),
    aborted: integer(raw.aborted),
    sessions: integer(raw.sessions),
  };
}

function database(raw) {
  const db = object(raw);
  return {
    state: text(db.state),
    error: nullable(db.error, text),
    databaseType: nullable(db.database_type, text),
    provider: nullable(db.provider, text),
    buildTime: nullable(db.build_time, timestamp),
    loadedAt: nullable(db.loaded_at, timestamp),
  };
}

function list(raw, item) {
  const part = object(raw);
  return { total: integer(part.total), items: array(part.items).map((entry) => item(object(entry))) };
}

function addressItem(raw) {
  return {
    ip: text(raw.ip),
    local: boolean(raw.local),
    country: nullable(raw.country, text),
    asn: nullable(raw.asn, (value) => integer(value, 1)),
    asName: nullable(raw.as_name, text),
    ...counts(raw),
    lastSeenAt: timestamp(raw.last_seen_at),
    denyEntries: array(raw.deny_entries).map(text),
  };
}

function countryItem(raw) {
  return { country: nullable(raw.country, text), addresses: integer(raw.addresses), ...counts(raw) };
}

function asnItem(raw) {
  return {
    asn: nullable(raw.asn, (value) => integer(value, 1)),
    asName: nullable(raw.as_name, text),
    addresses: integer(raw.addresses),
    ...counts(raw),
  };
}

// Response of GET /api/v1/sources?window=<period> (management API contract 1.17). Values and item order stay as
// received. Anything that does not fit, including the answer of another window, becomes an 'invalid' failure.
export function parseSources(body, period) {
  const data = object(body);
  if (text(data.window) !== period) {
    throw invalid();
  }
  const databases = object(data.databases);
  const totals = object(data.totals);
  return {
    asOf: timestamp(data.as_of),
    windowStart: timestamp(data.window_start),
    denyEditable: boolean(data.deny_editable),
    capacity: integer(data.capacity),
    stored: integer(data.stored),
    droppedBefore: nullable(data.dropped_before, timestamp),
    databases: { country: database(databases.country), asn: database(databases.asn) },
    totals: counts(totals),
    addresses: list(data.addresses, addressItem),
    countries: list(data.countries, countryItem),
    asns: list(data.asns, asnItem),
  };
}

// CPY-258 shows only when dropped events reach into the period.
export function storageLimited(data) {
  return data.droppedBefore !== null && data.droppedBefore.getTime() >= data.windowStart.getTime();
}

// Summary card values in SUMMARY_KEYS order: the four totals, then the three `total` values.
export function summaryValues(data) {
  return [
    data.totals.attempts,
    data.totals.rejected,
    data.totals.aborted,
    data.totals.sessions,
    data.addresses.total,
    data.countries.total,
    data.asns.total,
  ];
}

// Title count of one table: CPY-289 when the response holds only the top items, otherwise CPY-084·085 with `total`.
export function tableCount(part) {
  return part.total > part.items.length
    ? { partial: true, total: part.total, shown: part.items.length }
    : { partial: false, total: part.total };
}

// Providers with an attribution, once each, in database row order.
export function attributedProviders(databases) {
  const providers = [];
  for (const { key } of DATABASE_ROWS) {
    const provider = databases[key].provider;
    if (provider !== null && ATTRIBUTED.has(provider) && !providers.includes(provider)) {
      providers.push(provider);
    }
  }
  return providers;
}

const regionNames = new Map();

// Country cell text: `{name} ({code})` with the browser's region name in the screen language, or the code alone when
// the browser has no name for it.
export function countryLabel(code, lang) {
  if (!regionNames.has(lang)) {
    regionNames.set(lang, new Intl.DisplayNames([lang], { type: 'region', fallback: 'none' }));
  }
  let name;
  try {
    name = regionNames.get(lang).of(code);
  } catch {
    // A value that is not a region code has no name.
    name = undefined;
  }
  return name && name !== code ? `${name} (${code})` : code;
}

// A deny entry with `/` or `-` covers a range, not one address (dashboard.md SCR-008 주소 차단과 해제).
export function isRangeEntry(entry) {
  return entry.includes('/') || entry.includes('-');
}

// What the block status cell of one address row shows (dashboard.md SCR-008 table of conditions).
// `blocked`: the blocked badge. `action`: 'block', 'unblock', or null. `rangeNote`: CPY-301.
export function denyCell(item, denyEditable) {
  const blocked = item.denyEntries.length > 0;
  if (!denyEditable || item.local) return { blocked, action: null, rangeNote: false };
  if (!blocked) return { blocked, action: 'block', rangeNote: false };
  if (item.denyEntries.some(isRangeEntry)) return { blocked, action: null, rangeNote: true };
  return { blocked, action: 'unblock', rangeNote: false };
}

// The deny list to send for a block: the current list with the address at the end. `null` when the current list already
// holds the same text, so nothing is sent and the block counts as done.
export function listForBlock(current, ip) {
  return current.includes(ip) ? null : [...current, ip];
}

// The deny list to send for an unblock: the current list without every text of `entries`. `null` when none of them is in
// the current list, so nothing is sent.
export function listForUnblock(current, entries) {
  const next = current.filter((entry) => !entries.includes(entry));
  return next.length === current.length ? null : next;
}
