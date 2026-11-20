// Contract-shaped sources responses (management API contract 1.17) for the fake API pages (sources.html and
// router.html). Values are examples, not server data: documentation addresses (RFC 5737, RFC 3849), example AS numbers
// (RFC 5398, then the private-use range from 4200000000 once the 32 examples run out), and invented organization names.
export const AS_OF = Date.parse('2026-11-20T10:00:00.000Z');
export const iso = (ms) => new Date(ms).toISOString();
const HOUR = 3600 * 1000;
export const WINDOW_MS = { '1h': HOUR, '24h': 24 * HOUR };

// A 38-character IPv6 address and a long AS name, so wrapping can be checked at 320 px.
export const LONG_IPV6 = '2001:db8:1234:5678:9abc:def0:1234:5678';
export const LONG_AS_NAME = 'Example Very Long Autonomous System Name For Wrapping Checks Incorporated';
// A region code that Chrome has no name for, so the cell shows the code alone.
export const UNNAMED_COUNTRY = 'XZ';

// The 32 example AS numbers, then private-use numbers.
export function exampleAsn(index) {
  if (index < 16) return 64496 + index;
  if (index < 32) return 65536 + index - 16;
  return 4200000000 + index - 32;
}

export function database(overrides = {}) {
  return {
    state: 'loaded',
    error: null,
    database_type: 'DBIP-Country-Lite',
    provider: 'dbip',
    build_time: '2026-11-01T00:00:00.000Z',
    loaded_at: '2026-11-20T09:00:00.000Z',
    ...overrides,
  };
}

export const NOT_IN_USE = { database_type: null, provider: null, build_time: null, loaded_at: null };

// Both databases in use from DB-IP, as in the adopted normal frame.
export function databases(country = {}, asn = {}) {
  return {
    country: database(country),
    asn: database({ database_type: 'DBIP-ASN-Lite', ...asn }),
  };
}

function counts(attempts) {
  return { attempts, rejected: attempts % 3, aborted: attempts % 2, sessions: Math.floor(attempts / 4) };
}

// The first rows carry the special cases; the rest are documentation IPv4 addresses with known countries and ASNs.
const COUNTRY_CYCLE = ['KR', 'US', 'JP', 'DE', 'GB'];
// The entries of `list` that cover `ip`, in list order. Only the shapes the scenarios use: the address itself, a
// documentation /24 network, 2001:db8::/32, and the range 2001:db8::1-2001:db8::ff.
export function denyEntriesFor(list, ip) {
  return list.filter((entry) => entry === ip
    || (entry.endsWith('.0/24') && ip.startsWith(entry.slice(0, -4)))
    || (entry === '2001:db8::/32' && ip.startsWith('2001:db8:'))
    || (entry === '2001:db8::1-2001:db8::ff' && /^2001:db8::[0-9a-f]{1,2}$/.test(ip)));
}

// `denyOf(ip)` gives the `deny_entries` of a row. Nothing is blocked by default.
export function addressItems(count, denyOf = () => []) {
  const special = [
    { ip: LONG_IPV6, local: false, country: 'US', asn: exampleAsn(4), as_name: LONG_AS_NAME },
    { ip: '198.51.100.250', local: false, country: 'KR', asn: exampleAsn(0), as_name: 'Example Transit Networks' },
    { ip: '127.0.0.1', local: true, country: null, asn: null, as_name: null },
    { ip: '192.0.2.44', local: false, country: UNNAMED_COUNTRY, asn: null, as_name: null },
    { ip: '203.0.113.9', local: false, country: null, asn: exampleAsn(15), as_name: null },
  ];
  return Array.from({ length: count }, (_, index) => {
    const base = special[index] ?? {
      ip: index < 200 ? `198.51.100.${index}` : `203.0.113.${index - 200}`,
      local: false,
      country: COUNTRY_CYCLE[index % COUNTRY_CYCLE.length],
      asn: exampleAsn(index % 32),
      as_name: `Example Org ${index % 32}`,
    };
    return {
      ...base,
      ...counts(500 - index),
      deny_entries: denyOf(base.ip),
      last_seen_at: iso(AS_OF - index * 60000),
    };
  });
}

export function countryItems() {
  return [
    ...COUNTRY_CYCLE.map((country, index) => ({ country, addresses: 30 - index, ...counts(200 - index * 20) })),
    { country: UNNAMED_COUNTRY, addresses: 1, ...counts(3) },
    { country: null, addresses: 2, ...counts(3) },
  ];
}

export function asnItems(count) {
  return Array.from({ length: count }, (_, index) => {
    if (index === count - 1) return { asn: null, as_name: null, addresses: 3, ...counts(1) };
    return {
      asn: exampleAsn(index),
      as_name: index === 4 ? LONG_AS_NAME : index === 15 ? null : `Example Org ${index}`,
      addresses: 5,
      ...counts(300 - index),
    };
  });
}

// One response. Defaults: the last hour, 150 addresses with the top 100 listed, seven countries, and 120 ASNs with the
// top 100 listed. `dropped_before` is null, both databases are in use.
export function sourcesBody({
  window = '1h',
  asOf = AS_OF,
  stored = 4210,
  droppedBefore = null,
  dbs = databases(),
  totals = { attempts: 1234, rejected: 56, aborted: 78, sessions: 90 },
  addresses = { total: 150, items: addressItems(100) },
  denyEditable = true,
  countries = { total: 7, items: countryItems() },
  asns = { total: 120, items: asnItems(100) },
} = {}) {
  return {
    as_of: iso(asOf),
    window,
    window_start: iso(asOf - WINDOW_MS[window]),
    capacity: 100000,
    stored,
    dropped_before: droppedBefore === null ? null : iso(droppedBefore),
    deny_editable: denyEditable,
    databases: dbs,
    totals,
    addresses,
    countries,
    asns,
  };
}

// A period without events: zero totals and three empty lists.
export function emptyBody(options = {}) {
  return sourcesBody({
    stored: 0,
    totals: { attempts: 0, rejected: 0, aborted: 0, sessions: 0 },
    addresses: { total: 0, items: [] },
    countries: { total: 0, items: [] },
    asns: { total: 0, items: [] },
    ...options,
  });
}
