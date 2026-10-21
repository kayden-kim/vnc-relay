// Gauge: circular usage meter exposed as a progressbar.
// Geometry matches --space-gauge-size (120) and --space-gauge-stroke (12).
const SIZE = 120;
const STROKE = 12;
const RADIUS = (SIZE - STROKE) / 2;
const CENTER = SIZE / 2;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;
const SVG_NS = 'http://www.w3.org/2000/svg';

function svgElement(tag, attributes) {
  const node = document.createElementNS(SVG_NS, tag);
  for (const [name, value] of Object.entries(attributes)) {
    node.setAttribute(name, String(value));
  }
  return node;
}

// Track ring plus, for value > 0, an arc that starts at 12 o'clock (CSS rotates the svg).
function ring(value, max) {
  const svg = svgElement('svg', {
    class: 'gauge-svg',
    viewBox: `0 0 ${SIZE} ${SIZE}`,
    'aria-hidden': 'true',
    focusable: 'false',
  });
  svg.append(svgElement('circle', { class: 'gauge-track', cx: CENTER, cy: CENTER, r: RADIUS }));
  if (value > 0) {
    const arc = svgElement('circle', { class: 'gauge-arc', cx: CENTER, cy: CENTER, r: RADIUS });
    // Fill length goes through the CSSOM; markup carries no style attribute.
    arc.style.setProperty('stroke-dasharray', `${CIRCUMFERENCE} ${CIRCUMFERENCE}`);
    arc.style.setProperty('stroke-dashoffset', String(CIRCUMFERENCE * (1 - value / max)));
    svg.append(arc);
  }
  return svg;
}

/**
 * @param {{label: string, value?: number | null, max: number, valueText?: string,
 *   limitText: string, unknownText?: string}} options
 *   value null means unknown: no ring; unknownText is then required and shown in a badge.
 * @returns {HTMLDivElement}
 */
export function createGauge({
  label, value = null, max, valueText, limitText, unknownText,
} = {}) {
  if (typeof label !== 'string' || label === '') {
    throw new TypeError('gauge label is required');
  }
  if (!Number.isFinite(max) || max <= 0) {
    throw new TypeError('gauge max must be a positive number');
  }
  const known = value !== null;
  if (known && !(Number.isFinite(value) && value >= 0 && value <= max)) {
    throw new TypeError('gauge value must be between 0 and max');
  }
  if (typeof limitText !== 'string' || limitText === '') {
    throw new TypeError('gauge limitText is required');
  }
  if (!known && (typeof unknownText !== 'string' || unknownText === '')) {
    throw new TypeError('gauge unknownText is required when value is null');
  }
  const text = valueText ?? (known ? String(value) : unknownText);

  const root = document.createElement('div');
  root.className = 'gauge';
  root.setAttribute('role', 'progressbar');
  root.setAttribute('aria-label', label);
  root.setAttribute('aria-valuemin', '0');
  root.setAttribute('aria-valuemax', String(max));
  root.setAttribute('aria-valuetext', text);

  const center = document.createElement('div');
  center.className = 'gauge-center';
  if (known) {
    root.setAttribute('aria-valuenow', String(value));
    root.append(ring(value, max));
    const metric = document.createElement('span');
    metric.className = 'gauge-value';
    metric.textContent = text;
    center.append(metric);
  } else {
    // Badge markup is built here, not imported, to keep gauge.js dependency-free.
    const badge = document.createElement('span');
    badge.className = 'badge badge-tone-neutral';
    badge.textContent = text;
    center.append(badge);
  }

  const limit = document.createElement('span');
  limit.className = 'gauge-max';
  limit.textContent = limitText;
  center.append(limit);
  root.append(center);
  return root;
}
