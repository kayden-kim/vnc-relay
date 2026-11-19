// SCR-007 metrics: the response check and the pure calculations behind cards, charts, and tables. No DOM here.
// Rules come from dashboard.md SCR-007 (cards, charts, distribution, cause tables) and DESIGN.md 표시 형식.

import { ApiError } from './api.js';

const TIMESTAMP = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/;

// Metrics contract 1.8: the server keeps 360 points, one per 10 seconds, so the chart spans one hour.
export const HISTORY_CAPACITY = 360;
export const HISTORY_INTERVAL_SECS = 10;
export const HISTORY_SPAN_MS = 3600 * 1000;
// A connection chart group is seven neighbouring points, that is six pairs, one minute of history.
export const GROUP_STEP = 6;

// Plot geometry shared with the renderer. One x unit is 10 seconds of the one-hour span.
export const PLOT_WIDTH = 360;
export const PLOT_HEIGHT = 100;

// Smallest bar of a non-zero value is spacing.chart-stroke (2px). The distribution bar is 32px tall and the count bar
// is 64px wide (DESIGN.md 메트릭 화면 확장), so the minimum in their 100-unit boxes is 2/32 and 2/64 of 100.
export const MIN_DIST_BAR_HEIGHT = (2 / 32) * 100;
export const MIN_COUNT_BAR_WIDTH = (2 / 64) * 100;
export const DIST_VIEWBOX_HEIGHT = 100;
export const COUNT_VIEWBOX_WIDTH = 100;

// History series keys of metrics contract 1.8.
const SERIES_KEYS = [
  'session_count',
  'waiting_count',
  'connection_accepted',
  'connection_rejected',
  'connection_aborted',
  'session_io_viewer_to_target',
  'session_io_target_to_viewer',
  'process_cpu_time',
  'process_memory_usage',
];

export const HISTOGRAM_NAMES = [
  'vnc_relay.handshake.duration',
  'vnc_relay.waiting.duration',
  'vnc_relay.session.duration',
  'vnc_relay.target_connect.duration',
];

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

function finite(value) {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
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

// Epoch milliseconds of a contract timestamp.
function timestampMs(value) {
  if (typeof value !== 'string' || !TIMESTAMP.test(value)) {
    throw invalid();
  }
  const ms = Date.parse(value);
  if (Number.isNaN(ms)) {
    throw invalid();
  }
  return ms;
}

// Attribute pairs in response order. Values stay as received.
function attributes(value) {
  return Object.entries(object(value)).map(([name, raw]) => [name, text(raw)]);
}

function scalarPoint(raw) {
  const point = object(raw);
  return { attributes: attributes(point.attributes), value: finite(point.value) };
}

function histogramPoint(raw, bucketCount) {
  const point = object(raw);
  const counts = array(point.bucket_counts).map((count) => integer(count));
  if (counts.length !== bucketCount) {
    throw invalid();
  }
  return {
    attributes: attributes(point.attributes),
    count: integer(point.count),
    sum: finite(point.sum),
    bucketCounts: counts,
  };
}

function parseMetric(raw) {
  const metric = object(raw);
  const type = text(metric.type);
  text(metric.unit);
  if (typeof metric.available !== 'boolean') {
    throw invalid();
  }
  const points = array(metric.points);
  const parsed = { name: text(metric.name), type, available: metric.available, bounds: null, points: [] };
  if (type === 'histogram') {
    parsed.bounds = array(metric.bounds).map(finite);
    parsed.points = points.map((point) => histogramPoint(point, parsed.bounds.length + 1));
  } else if (type === 'counter' || type === 'updowncounter' || type === 'gauge') {
    parsed.points = points.map(scalarPoint);
  }
  // An unknown type is ignored: the screen reads only the metrics it knows by name (metrics contract 3).
  return parsed;
}

function parseHistoryPoint(raw) {
  const point = object(raw);
  const values = object(point.values);
  const checked = {};
  for (const key of SERIES_KEYS) {
    if (!Object.hasOwn(values, key)) {
      throw invalid();
    }
    checked[key] = values[key] === null ? null : finite(values[key]);
  }
  return { seq: integer(point.seq), atMs: timestampMs(point.at), values: checked };
}

// Response of GET /api/v1/metrics (metrics contract 1.7·1.8). Anything that does not fit becomes an 'invalid' failure.
export function parseMetrics(body) {
  const data = object(body);
  timestampMs(data.as_of);
  const resource = object(data.resource);
  const metrics = new Map();
  for (const raw of array(data.metrics)) {
    const metric = parseMetric(raw);
    if (!metrics.has(metric.name)) {
      metrics.set(metric.name, metric);
    }
  }
  const history = object(data.history);
  integer(history.interval_secs, 1);
  integer(history.capacity, 1);
  if (history.latest_seq !== null) {
    integer(history.latest_seq);
  }
  const points = array(history.points).map(parseHistoryPoint);
  for (let index = 1; index < points.length; index += 1) {
    if (points[index].seq <= points[index - 1].seq) {
      throw invalid();
    }
  }
  return {
    version: typeof resource['service.version'] === 'string' ? resource['service.version'] : null,
    metrics,
    points,
  };
}

// Retained points after a successful response (dashboard.md 4.1). A request without history_after (`replace`) gets
// every retained point, so the response replaces the kept ones. A poll appends the points that are newer than the
// largest kept seq, so no seq enters twice. Points 360 or more behind the largest seq are dropped.
export function mergeHistory(kept, incoming, replace) {
  const merged = replace ? [] : [...kept];
  let largest = merged.length > 0 ? merged[merged.length - 1].seq : -1;
  for (const point of incoming) {
    if (point.seq > largest) {
      merged.push(point);
      largest = point.seq;
    }
  }
  const floor = largest - HISTORY_CAPACITY;
  return merged.filter((point) => point.seq > floor);
}

// `history_after` of an automatic poll: the largest kept seq, or null when nothing is kept.
export function historyAfter(points) {
  return points.length > 0 ? points[points.length - 1].seq : null;
}

// Sum of every point of a metric ("모든 점의 합"). Null when the metric is missing or unavailable.
export function metricTotal(metrics, name) {
  const metric = metrics.get(name);
  if (!metric || !metric.available) {
    return null;
  }
  return metric.points.reduce((sum, point) => sum + point.value, 0);
}

// Value of a metric that has one point. Null when the metric is missing, unavailable, or without a point.
export function metricValue(metrics, name) {
  const metric = metrics.get(name);
  if (!metric || !metric.available || metric.points.length === 0) {
    return null;
  }
  return metric.points[0].value;
}

function usageOf(metrics, usageName, limitName) {
  const limit = metricValue(metrics, limitName);
  return {
    current: metricValue(metrics, usageName),
    // A limit below 1 cannot scale a gauge, so it counts as unknown.
    limit: limit !== null && limit >= 1 ? limit : null,
  };
}

// The most recent two retained points whose seq differ by 1 (dashboard.md SCR-007 "이웃한 두 점").
export function latestPair(points) {
  for (let index = points.length - 1; index > 0; index -= 1) {
    if (points[index].seq - points[index - 1].seq === 1) {
      return [points[index - 1], points[index]];
    }
  }
  return null;
}

// Per-second change of a cumulative series between two points, times `scale`. Null when a value is null
// or the points have no positive time difference.
function rateBetween(earlier, later, key, scale = 1) {
  const from = earlier.values[key];
  const to = later.values[key];
  if (from === null || to === null) {
    return null;
  }
  const seconds = (later.atMs - earlier.atMs) / 1000;
  if (!(seconds > 0)) {
    return null;
  }
  return ((to - from) / seconds) * scale;
}

// The eight key indicator cards. `state` of the rate cards: 'collecting' (no neighbouring pair), 'unavailable',
// or 'value'.
export function keyIndicators(parsed, points) {
  const { metrics } = parsed;
  const pair = latestPair(points);
  let throughput = { state: 'collecting', viewerToTarget: null, targetToViewer: null };
  let cpu = { state: 'collecting', percent: null };
  if (pair) {
    const viewerToTarget = rateBetween(pair[0], pair[1], 'session_io_viewer_to_target');
    const targetToViewer = rateBetween(pair[0], pair[1], 'session_io_target_to_viewer');
    throughput = viewerToTarget === null || targetToViewer === null
      ? { state: 'unavailable', viewerToTarget: null, targetToViewer: null }
      : { state: 'value', viewerToTarget, targetToViewer };
    const percent = rateBetween(pair[0], pair[1], 'process_cpu_time', 100);
    cpu = percent === null ? { state: 'unavailable', percent: null } : { state: 'value', percent };
  }
  return {
    active: metricTotal(metrics, 'vnc_relay.session.count'),
    waiting: metricTotal(metrics, 'vnc_relay.waiting.count'),
    relay: usageOf(metrics, 'vnc_relay.relay_connection.usage', 'vnc_relay.relay_connection.limit'),
    admin: usageOf(metrics, 'vnc_relay.admin_connection.usage', 'vnc_relay.admin_connection.limit'),
    throughput,
    rejected: metricTotal(metrics, 'vnc_relay.connection.rejected'),
    cpu,
    memory: metricValue(metrics, 'process.memory.usage'),
  };
}

// ---- Charts ----

// Value drawn at every retained point.
function directSeries(key) {
  return (points) => points.map((point) => ({ seq: point.seq, atMs: point.atMs, value: point.values[key] }));
}

// Per-second change between neighbouring points, drawn at the later point's time. A null value or a seq that
// does not follow leaves the place without a value.
function rateSeries(key, scale = 1) {
  return (points) => points.slice(1).map((point, index) => {
    const earlier = points[index];
    const value = point.seq - earlier.seq === 1 ? rateBetween(earlier, point, key, scale) : null;
    return { seq: point.seq, atMs: point.atMs, value };
  });
}

// Per-minute increase of one connection series. Groups of seven points are counted back from the newest seq in steps of
// six, and neighbouring groups share their boundary point. The oldest group that reaches before the oldest retained
// seq is dropped. A group with a missing seq or a null value has no value. Drawn at the group's last point.
function groupSeries(key) {
  return (points) => {
    const bySeq = new Map(points.map((point) => [point.seq, point]));
    const oldest = points[0].seq;
    const entries = [];
    for (let end = points[points.length - 1].seq; end - GROUP_STEP >= oldest; end -= GROUP_STEP) {
      const first = bySeq.get(end - GROUP_STEP);
      const last = bySeq.get(end);
      let value = null;
      let complete = first !== undefined && last !== undefined;
      for (let seq = end - GROUP_STEP; complete && seq <= end; seq += 1) {
        const point = bySeq.get(seq);
        complete = point !== undefined && point.values[key] !== null;
      }
      if (complete) {
        const seconds = (last.atMs - first.atMs) / 1000;
        value = seconds > 0 ? ((last.values[key] - first.values[key]) / seconds) * 60 : null;
      }
      entries.push({ seq: end, atMs: last ? last.atMs : null, value });
    }
    return entries.reverse();
  };
}

// Charts in screen order. `min` is the fewest retained points that can draw one line (dashboard.md SCR-007).
const CHARTS = [
  { id: 'sessions', kind: 'count', min: 2, step: 1, series: [directSeries('session_count'), directSeries('waiting_count')] },
  {
    id: 'connections',
    kind: 'decimal',
    min: 13,
    step: GROUP_STEP,
    series: [groupSeries('connection_accepted'), groupSeries('connection_rejected'), groupSeries('connection_aborted')],
  },
  {
    id: 'throughput',
    kind: 'bytes',
    min: 3,
    step: 1,
    series: [rateSeries('session_io_viewer_to_target'), rateSeries('session_io_target_to_viewer')],
  },
  { id: 'cpu', kind: 'percent', min: 3, step: 1, series: [rateSeries('process_cpu_time', 100)] },
  { id: 'memory', kind: 'bytes', min: 2, step: 1, series: [directSeries('process_memory_usage')] },
];

// Runs of drawable neighbours. A null value or a seq distance other than `step` ends a run, so the line breaks there.
export function runsOf(entries, step) {
  const runs = [];
  let run = [];
  for (const entry of entries) {
    if (entry.value === null) {
      if (run.length > 0) runs.push(run);
      run = [];
      continue;
    }
    if (run.length > 0 && entry.seq - run[run.length - 1].seq !== step) {
      runs.push(run);
      run = [];
    }
    run.push(entry);
  }
  if (run.length > 0) runs.push(run);
  return runs;
}

// Axis top of DESIGN.md 선 차트: the smallest of 2, 4, 6, 8, 10 × 10^k at or above `value`, and 2 when there is no
// positive value. Every step is even, so the middle tick is a whole multiple. Below 10 the search starts at the
// value's own power of ten, so a count axis never tops at 1 (its middle tick stays whole). From 10 up it starts one
// power lower, so an exact 10, 100, ... draws to itself. Negative exponents divide, so 6 × 10^-1 is exactly 0.6.
const AXIS_STEPS = [2, 4, 6, 8, 10];

export function niceCeil(value) {
  if (!(value > 0)) {
    return 2;
  }
  const exponent = Math.floor(Math.log10(value));
  for (let power = exponent >= 1 ? exponent - 1 : exponent; power <= exponent + 1; power += 1) {
    for (const step of AXIS_STEPS) {
      const candidate = power < 0 ? step / 10 ** -power : step * 10 ** power;
      if (candidate >= value) {
        return candidate;
      }
    }
  }
  return 2 * 10 ** (exponent + 1);
}

// Axis of one chart: the top is at or above the largest visible value. The middle tick is half of it. A byte axis
// computes the KiB top first and uses MiB when that top is 1024 KiB or more, else KiB. `unit` and `divisor` name the
// byte unit.
export function axisOf(kind, largest) {
  const max = largest ?? 0;
  if (kind === 'bytes') {
    const kibTop = niceCeil(max / 1024);
    if (kibTop < 1024) {
      return { top: kibTop * 1024, divisor: 1024, unit: 'KiB' };
    }
    return { top: niceCeil(max / 1048576) * 1048576, divisor: 1048576, unit: 'MiB' };
  }
  return { top: niceCeil(max) };
}

// The five charts. A chart with fewer retained points than its minimum is not `enough`: the screen shows CPY-220.
// Each series keeps its visible entries (inside the last hour), its runs, the current value (newest entry),
// and the largest visible value. Null means "unavailable" (CPY-035).
export function buildCharts(points) {
  const latestMs = points.length > 0 ? points[points.length - 1].atMs : 0;
  const from = latestMs - HISTORY_SPAN_MS;
  return CHARTS.map((chart) => {
    if (points.length < chart.min) {
      return { id: chart.id, kind: chart.kind, enough: false, series: [], axis: null, from, to: latestMs };
    }
    const series = chart.series.map((build) => {
      const all = build(points);
      const visible = all.filter((entry) => entry.atMs !== null && entry.atMs >= from);
      const values = visible.filter((entry) => entry.value !== null).map((entry) => entry.value);
      const newest = all.length > 0 ? all[all.length - 1] : null;
      return {
        entries: visible,
        runs: runsOf(visible, chart.step),
        current: newest ? newest.value : null,
        max: values.length > 0 ? Math.max(...values) : null,
      };
    });
    const largest = series.reduce((top, one) => (one.max !== null && one.max > top ? one.max : top), 0);
    return {
      id: chart.id,
      kind: chart.kind,
      enough: true,
      series,
      axis: axisOf(chart.kind, largest),
      from,
      to: latestMs,
    };
  });
}

// x of a time in plot units: the newest point sits at the right edge.
export function plotX(atMs, from) {
  return ((atMs - from) / HISTORY_SPAN_MS) * PLOT_WIDTH;
}

export function plotY(value, top) {
  return PLOT_HEIGHT - (value / top) * PLOT_HEIGHT;
}

// ---- Distribution ----

// Index of the first bucket whose running sum reaches 95% of `count`, or -1 when none does. Integer math keeps
// the comparison exact.
export function p95Index(bucketCounts, count) {
  let running = 0;
  for (let index = 0; index < bucketCounts.length; index += 1) {
    running += bucketCounts[index];
    if (running * 100 >= count * 95) {
      return index;
    }
  }
  return -1;
}

// One row of the distribution table. `p95` is { bound, over } with over true for the bucket above the last bound.
export function distributionRow(metric, point) {
  const { count, sum, bucketCounts } = point;
  let p95 = null;
  if (count > 0) {
    const index = p95Index(bucketCounts, count);
    if (index >= 0) {
      p95 = index < metric.bounds.length
        ? { bound: metric.bounds[index], over: false }
        : { bound: metric.bounds[metric.bounds.length - 1], over: true };
    }
  }
  const largest = Math.max(...bucketCounts);
  return {
    attributes: point.attributes,
    count,
    mean: count > 0 ? sum / count : null,
    p95,
    buckets: bucketCounts.map((value, index) => ({
      index,
      count: value,
      // Same-width bars in a box 10 units per bucket: bar 8, gap 2.
      x: index * 10 + 1,
      height: value === 0 ? 0 : Math.max(MIN_DIST_BAR_HEIGHT, (value / largest) * DIST_VIEWBOX_HEIGHT),
      bound: index < metric.bounds.length ? { value: metric.bounds[index], over: false }
        : { value: metric.bounds[metric.bounds.length - 1], over: true },
    })),
    viewWidth: bucketCounts.length * 10,
  };
}

// The four histograms in contract order. `rows` is empty when the metric has no recorded point.
export function distributionItems(metrics) {
  return HISTOGRAM_NAMES.map((name) => {
    const metric = metrics.get(name);
    const rows = metric && metric.available ? metric.points.map((point) => distributionRow(metric, point)) : [];
    return { name, rows };
  });
}

// ---- Cause tables ----

// Cause tables in screen order. `sources` are metric names (response order decides ties), `item` marks tables whose
// first column is an item name, and `cells` lists the attribute names of each value column ('a|b' reads a, else b).
export const CAUSE_TABLES = [
  { id: 'rejected', sources: ['vnc_relay.connection.rejected'], item: false, cells: ['listener', 'stage', 'reason'] },
  { id: 'aborted', sources: ['vnc_relay.connection.aborted'], item: false, cells: ['listener', 'stage', 'reason'] },
  { id: 'closed', sources: ['vnc_relay.session.closed'], item: false, cells: ['mode', 'close_cause'] },
  {
    id: 'admin',
    sources: ['vnc_relay.admin.logins', 'vnc_relay.admin.lockouts', 'vnc_relay.admin.terminations'],
    item: true,
    cells: ['result|scope'],
  },
  { id: 'config', sources: ['vnc_relay.config.reloads', 'vnc_relay.config.edits'], item: true, cells: ['result', 'trigger|reason'] },
  { id: 'websocket', sources: ['vnc_relay.websocket.ping_timeouts', 'vnc_relay.ticket.accepted'], item: true, cells: ['listener', 'role'] },
];

function cellValue(point, spec) {
  for (const name of spec.split('|')) {
    const found = point.attributes.find(([key]) => key === name);
    if (found) {
      return { name, value: found[1] };
    }
  }
  return null;
}

// Rows of one cause table, largest count first, ties in response order. A missing attribute leaves its cell null.
// `itemIndex` is the position of the source metric in the table's source list.
export function causeRows(metrics, table) {
  const rows = [];
  for (const [name, metric] of metrics) {
    const itemIndex = table.sources.indexOf(name);
    if (itemIndex < 0 || !metric.available) {
      continue;
    }
    for (const point of metric.points) {
      rows.push({
        order: rows.length,
        itemIndex: table.item ? itemIndex : null,
        cells: table.cells.map((spec) => cellValue(point, spec)),
        count: point.value,
      });
    }
  }
  return rows.sort((a, b) => b.count - a.count || a.order - b.order);
}

// Count bar fill in the 100-unit box: proportional to the table's largest count, never below the 2px minimum.
export function countBarWidth(count, largest) {
  if (count === 0 || largest === 0) {
    return 0;
  }
  return Math.max(MIN_COUNT_BAR_WIDTH, (count / largest) * COUNT_VIEWBOX_WIDTH);
}

// ---- Process ----

// Rows of the process card. A row is present only when the response holds its metric. Handles (Windows) and file
// descriptors (Linux) are mutually exclusive in practice, and each shows only when present. `value` null is unavailable.
export function processRows(parsed) {
  const { metrics } = parsed;
  const rows = [{ key: 'uptime', value: metricValue(metrics, 'process.uptime') }];
  const cpuMetric = metrics.get('process.cpu.time');
  let cpuTime = null;
  if (cpuMetric && cpuMetric.available) {
    const mode = (name) => {
      const found = cpuMetric.points.find((point) => point.attributes.some(([key, value]) => key === 'cpu.mode' && value === name));
      return found ? found.value : null;
    };
    cpuTime = { user: mode('user'), system: mode('system') };
  }
  rows.push({ key: 'cpuTime', value: cpuTime });
  rows.push({ key: 'memory', value: metricValue(metrics, 'process.memory.usage') });
  rows.push({ key: 'virtual', value: metricValue(metrics, 'process.memory.virtual') });
  rows.push({ key: 'threads', value: metricValue(metrics, 'process.thread.count') });
  if (metrics.has('process.windows.handle.count')) {
    rows.push({ key: 'handles', value: metricValue(metrics, 'process.windows.handle.count') });
  }
  if (metrics.has('process.unix.file_descriptor.count')) {
    rows.push({ key: 'descriptors', value: metricValue(metrics, 'process.unix.file_descriptor.count') });
  }
  rows.push({ key: 'version', value: parsed.version });
  return rows;
}
