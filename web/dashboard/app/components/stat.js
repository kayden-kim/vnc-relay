// Stat: a label with one metric value and an optional unit.
import { createBadge } from './badge.js';

const TONES = ['success', 'info', 'warning', 'destructive', 'neutral'];

function element(tag, className, text) {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

/**
 * @param {{label: string, value: string | number, unit?: string, tone?: string, badge?: boolean}} options
 * @returns {HTMLDivElement}
 */
export function createStat({ label, value, unit, tone, badge = false } = {}) {
  if (typeof label !== 'string' || label === '') {
    throw new TypeError('stat label is required');
  }
  if (typeof value !== 'string' && typeof value !== 'number') {
    throw new TypeError('stat value must be a string or number');
  }
  if (tone !== undefined && !TONES.includes(tone)) {
    throw new TypeError(`unknown tone: ${String(tone)}`);
  }

  const root = document.createElement('div');
  root.className = 'stat';

  const valueEl = document.createElement('p');
  valueEl.className = 'stat-value';
  if (badge) {
    // Status values are badges: the server state and placeholders (unknown or loading).
    valueEl.append(createBadge({ tone: tone ?? 'neutral', label: String(value) }));
  } else {
    valueEl.append(String(value));
  }
  if (tone !== undefined) valueEl.dataset.tone = tone;
  if (unit) valueEl.append(element('span', 'stat-unit', unit));

  root.append(element('p', 'stat-label', label), valueEl);
  return root;
}
