// Contract-shaped metrics responses for the fake API pages (metrics.html and router.html). Values are examples, not server data.
// Points follow metrics contract 1.8: one per 10 s, nine series keys, cumulative counters.
export const T0 = Date.parse('2026-11-19T10:00:00.000Z');
export const iso = (ms) => new Date(ms).toISOString();

// One retained point. `at` is T0 plus 10 s per seq, so seq 360 is one hour after seq 0.
export function historyPoint(seq, overrides = {}) {
  return {
    seq,
    at: iso(T0 + seq * 10000),
    values: {
      session_count: 3,
      waiting_count: 1,
      connection_accepted: seq * 2,
      connection_rejected: Math.floor(seq / 3),
      connection_aborted: 0,
      session_io_viewer_to_target: seq * 10240,
      session_io_target_to_viewer: seq * 20480,
      process_cpu_time: seq,
      process_memory_usage: 52428800,
      ...overrides,
    },
  };
}

// Points `from` to `to`, both included. `overrides` may be a function of seq.
export function points(from, to, overrides = {}) {
  return Array.from({ length: to - from + 1 }, (_, index) => {
    const seq = from + index;
    return historyPoint(seq, typeof overrides === 'function' ? overrides(seq) : overrides);
  });
}

export const scalar = (name, type, unit, rows, available = true) => ({
  name, type, unit, available, points: rows.map(([attributes, value]) => ({ attributes, value })),
});

export const histogram = (name, bounds, rows, available = true) => ({
  name,
  type: 'histogram',
  unit: 's',
  available,
  bounds,
  points: rows.map(([attributes, count, sum, bucketCounts]) => ({
    attributes, count, sum, bucket_counts: bucketCounts,
  })),
});

// Histogram bounds from metrics contract 1.5.
export const DURATION_BOUNDS = [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10, 30, 60, 120];
export const WAITING_BOUNDS = [1, 5, 10, 30, 60, 300, 600, 1800, 3600, 7200, 21600, 43200, 86400];
export const SESSION_BOUNDS = [1, 10, 60, 300, 600, 1800, 3600, 7200, 14400, 28800, 86400, 259200, 604800];
// `placed` maps bucket indexes to counts. Every other bucket, up to the one above the last bound, is 0.
export const buckets = (bounds, placed) => Array.from({ length: bounds.length + 1 }, (_, index) => placed[index] ?? 0);

// Every metric the screen reads, with known values. `overrides` replaces a metric by name; `null` removes it.
// Types, units, attributes, and attribute values follow metrics contract 1.3 and 1.4. `relay_reset` is the one
// close cause outside the contract: it stands for a future value the screen must keep as written.
export function defaultMetrics(overrides = {}) {
  const upgrade = { listener: 'ws', stage: 'upgrade' };
  const list = [
    scalar('vnc_relay.session.count', 'updowncounter', '{session}', [
      [{ mode: 'mode1', viewer_transport: 'ws', target_transport: 'tcp' }, 2],
      [{ mode: 'mode2', viewer_transport: 'tcp', target_transport: 'tcp' }, 1],
    ]),
    scalar('vnc_relay.waiting.count', 'updowncounter', '{connection}', [
      [{ role: 'viewer', transport: 'ws' }, 3], [{ role: 'server', transport: 'tcp' }, 1],
    ]),
    scalar('vnc_relay.relay_connection.usage', 'updowncounter', '{connection}', [[{}, 5]]),
    scalar('vnc_relay.relay_connection.limit', 'updowncounter', '{connection}', [[{}, 100]]),
    scalar('vnc_relay.admin_connection.usage', 'updowncounter', '{connection}', [[{}, 1]]),
    scalar('vnc_relay.admin_connection.limit', 'updowncounter', '{connection}', [[{}, 10]]),
    scalar('vnc_relay.connection.rejected', 'counter', '{connection}', [
      [{ ...upgrade, reason: 'ticket_invalid' }, 7], [{ ...upgrade, reason: 'origin_not_allowed' }, 1],
    ]),
    scalar('vnc_relay.connection.aborted', 'counter', '{connection}', [
      [{ listener: 'ws', stage: 'initial_exchange', reason: 'peer_eof' }, 2],
    ]),
    scalar('vnc_relay.session.closed', 'counter', '{session}', [
      [{ mode: 'mode1', close_cause: 'peer_eof' }, 12],
      [{ mode: 'mode2', close_cause: 'relay_reset' }, 3],
      [{ mode: 'mode1', close_cause: 'connection_error' }, 1],
    ]),
    scalar('vnc_relay.admin.logins', 'counter', '{attempt}', [[{ result: 'succeeded' }, 5]]),
    scalar('vnc_relay.admin.lockouts', 'counter', '{lockout}', [[{ scope: 'account' }, 1]]),
    scalar('vnc_relay.admin.terminations', 'counter', '{request}', [[{ result: 'terminated' }, 2]]),
    scalar('vnc_relay.config.reloads', 'counter', '{reload}', [[{ result: 'applied', trigger: 'file_watch' }, 2]]),
    scalar('vnc_relay.config.edits', 'counter', '{request}', [[{ result: 'edited' }, 1]]),
    scalar('vnc_relay.websocket.ping_timeouts', 'counter', '{connection}', [[{ listener: 'ws' }, 1]]),
    scalar('vnc_relay.ticket.accepted', 'counter', '{ticket}', [[{ listener: 'ws', role: 'viewer' }, 4]]),
    // p95 inside a bucket (≤ 1 s); 11 values whose sum, 1.1 s, fits their buckets.
    histogram('vnc_relay.handshake.duration', DURATION_BOUNDS, [
      [{ listener: 'ws', mode: 'mode2' }, 11, 1.1, buckets(DURATION_BOUNDS, { 0: 3, 1: 2, 4: 5, 7: 1 })],
    ]),
    histogram('vnc_relay.waiting.duration', WAITING_BOUNDS, []),
    // p95 above the last bound (7 days): four sessions of 8 days each.
    histogram('vnc_relay.session.duration', SESSION_BOUNDS, [
      [{ mode: 'mode1' }, 4, 2764800, buckets(SESSION_BOUNDS, { 13: 4 })],
    ]),
    // p95 inside the first bucket (≤ 5 ms).
    histogram('vnc_relay.target_connect.duration', DURATION_BOUNDS, [
      [{ result: 'connected' }, 1, 0.004, buckets(DURATION_BOUNDS, { 0: 1 })],
    ]),
    scalar('process.uptime', 'gauge', 's', [[{}, 93784]]),
    scalar('process.cpu.time', 'counter', 's', [[{ 'cpu.mode': 'user' }, 12.3], [{ 'cpu.mode': 'system' }, 4.5]]),
    scalar('process.memory.usage', 'updowncounter', 'By', [[{}, 52428800]]),
    scalar('process.memory.virtual', 'updowncounter', 'By', [[{}, 209715200]]),
    scalar('process.thread.count', 'updowncounter', '{thread}', [[{}, 17]]),
    scalar('process.windows.handle.count', 'updowncounter', '{handle}', [[{}, 220]]),
  ];
  const named = new Map(list.map((metric) => [metric.name, metric]));
  for (const [name, replacement] of Object.entries(overrides)) {
    if (replacement === null) named.delete(name);
    else named.set(name, replacement);
  }
  return [...named.values()];
}

export function metricsBody({ metrics = defaultMetrics(), points: retained = [] } = {}) {
  return {
    as_of: iso(T0 + 3600000),
    resource: { 'service.version': '0.9.0-test' },
    metrics,
    history: {
      interval_secs: 10,
      capacity: 360,
      latest_seq: retained.length > 0 ? retained[retained.length - 1].seq : null,
      points: retained,
    },
  };
}
