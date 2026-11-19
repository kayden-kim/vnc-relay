// Display formats for the operations overview. Inputs are values that model.js has already checked.
// Byte and time formats follow DESIGN.md.

const MODE_LABELS = new Map([
  ['mode1', 'Mode I'],
  ['mode2', 'Mode II'],
]);

const TRANSPORT_LABELS = new Map([
  ['tcp', 'TCP'],
  ['ws', 'WS'],
  ['wss', 'WSS'],
]);

const ROLE_LABELS = new Map([
  ['viewer', 'Viewer'],
  ['server', 'Server'],
]);

const KIB = 1024n;
const MIB = 1048576n;

function groupDigits(value) {
  return new Intl.NumberFormat().format(value);
}

// Formats tenths of a unit, for example 15n -> "1.5".
function tenths(value) {
  return `${groupDigits(value / 10n)}.${value % 10n}`;
}

// KiB while the rounded value is below 1024.0 KiB, then MiB (DESIGN.md 표시 형식). BigInt keeps the rounding exact above 2^53 - 1.
export function formatBytes(value) {
  const bytes = BigInt(value);
  // Implementation choice: round half up at one decimal place.
  const tenthsKib = (bytes * 10n + KIB / 2n) / KIB;
  if (tenthsKib < 10240n) {
    return `${tenths(tenthsKib)} KiB`;
  }
  const tenthsMib = (bytes * 10n + MIB / 2n) / MIB;
  return `${tenths(tenthsMib)} MiB`;
}

export function formatCount(value) {
  return groupDigits(value);
}

// Local time as YYYY-MM-DD HH:mm:ss, followed by the zone name (for example GMT+9).
export function formatTime(value) {
  const date = value instanceof Date ? value : new Date(value);
  const pad = (number) => String(number).padStart(2, '0');
  const stamp = [
    `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`,
    `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`,
  ].join(' ');
  const zone = new Intl.DateTimeFormat('en-US', { timeZoneName: 'short' })
    .formatToParts(date)
    .find((part) => part.type === 'timeZoneName');
  return zone ? `${stamp} ${zone.value}` : stamp;
}

// Duration as HH:MM:SS, with a day prefix (DESIGN l.491: "3일 01:02:03", "3d 01:02:03") when at least one day has passed.
export function formatDuration(ms, dayUnit) {
  const total = Math.floor(ms / 1000);
  const days = Math.floor(total / 86400);
  const hours = Math.floor((total % 86400) / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  const pad = (number) => String(number).padStart(2, '0');
  const clock = `${pad(hours)}:${pad(minutes)}:${pad(seconds)}`;
  return days > 0 ? `${days}${dayUnit} ${clock}` : clock;
}

export function formatMode(mode) {
  return MODE_LABELS.get(mode) ?? mode;
}

export function formatTransport(transport) {
  return TRANSPORT_LABELS.get(transport) ?? transport;
}

export function formatRole(role) {
  return ROLE_LABELS.get(role) ?? role;
}

// Address as ip:port. IPv6 addresses are wrapped in brackets: [ip]:port.
export function formatAddress({ ip, port }) {
  return ip.includes(':') ? `[${ip}]:${port}` : `${ip}:${port}`;
}

// Replaces {name} tokens with values[name]. Tokens without a value stay as written.
export function fill(template, values) {
  return template.replace(/\{(\w+)\}/g, (match, key) =>
    Object.hasOwn(values, key) ? String(values[key]) : match,
  );
}

// One decimal place, rounded half up, grouped by the browser locale (DESIGN.md 표시 형식: 비율·연결 1분당 증가량·눈금).
// `+ 0` turns a rounded -0 into 0.
export function formatDecimal(value) {
  const rounded = Math.round(value * 10) / 10 + 0;
  return new Intl.NumberFormat(undefined, { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(rounded);
}

// Bytes that may carry a fraction (a per-second rate): the same KiB/MiB boundary as formatBytes, in floating point.
export function formatBytesRate(value) {
  const tenthsKib = Math.round((value * 10) / 1024);
  if (tenthsKib < 10240) {
    return `${formatDecimal(tenthsKib / 10)} KiB`;
  }
  return `${formatDecimal(Math.round((value * 10) / 1048576) / 10)} MiB`;
}

// Whole bytes of the API (memory) keep the exact BigInt format. Anything else uses the floating-point one.
export function formatBytesValue(value) {
  return Number.isSafeInteger(value) && value >= 0 ? formatBytes(value) : formatBytesRate(value);
}

// CPU usage: a percentage with one decimal. 100.0% is one fully used core and is not capped.
export function formatPercent(value) {
  return `${formatDecimal(value)}%`;
}

// Short duration from seconds (DESIGN.md 표시 형식): milliseconds below 1 s, one-decimal seconds below 60 s,
// then `D일 HH:MM:SS`. The switch points use the rounded value. `unit` holds the copy templates.
export function formatShortSeconds(seconds, { secondsUnit, dayUnit }) {
  const millis = Math.round(seconds * 1000);
  if (millis < 1000) {
    return `${millis} ms`;
  }
  const tenths = Math.round(seconds * 10);
  if (tenths < 600) {
    return fill(secondsUnit, { n: formatDecimal(tenths / 10) });
  }
  return formatDuration(Math.round(seconds) * 1000, dayUnit);
}

// Local clock time HH:mm of a chart x tick, without a zone.
export function formatClock(ms) {
  const date = new Date(ms);
  const pad = (number) => String(number).padStart(2, '0');
  return `${pad(date.getHours())}:${pad(date.getMinutes())}`;
}
