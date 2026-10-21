// Badge: small status label with an optional icon.
import { icon } from './icons.js';

const TONES = ['success', 'info', 'warning', 'destructive', 'neutral'];

// Default icon per tone. Neutral (count) badges have none.
const DEFAULT_ICON = {
  success: 'check',
  info: 'clock',
  warning: 'warning',
  destructive: 'alert',
  neutral: null,
};

/**
 * @param {{tone?: string, label: string, icon?: string | false}} options
 *   icon: a name overrides the tone default; false removes it.
 * @returns {HTMLSpanElement}
 */
export function createBadge({ tone = 'neutral', label, icon: iconName } = {}) {
  if (!TONES.includes(tone)) {
    throw new TypeError(`unknown tone: ${String(tone)}`);
  }
  if (typeof label !== 'string' || label === '') {
    throw new TypeError('badge label is required');
  }

  const badge = document.createElement('span');
  badge.className = `badge badge-tone-${tone}`;
  badge.dataset.tone = tone;

  const name = iconName === undefined ? DEFAULT_ICON[tone] : iconName;
  if (name) badge.append(icon(name, { size: 'sm' }));
  badge.append(label);
  return badge;
}
