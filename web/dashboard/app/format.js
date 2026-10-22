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
