// Alert: inline status message with a tone, an icon, a title, optional text, and an optional action.
import { icon } from './icons.js';

const TONES = ['success', 'info', 'warning', 'destructive', 'neutral'];
const ICONS = {
  success: 'check',
  info: 'clock',
  warning: 'warning',
  destructive: 'alert',
  neutral: 'clock',
};
// Warning and destructive alerts interrupt; the rest are polite status messages.
const URGENT = new Set(['warning', 'destructive']);

function element(tag, className, text) {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

/**
 * @param {{tone?: string, title: string, message?: string, action?: HTMLElement}} options
 * @returns {HTMLDivElement}
 */
export function createAlert({ tone = 'neutral', title, message, action } = {}) {
  if (!TONES.includes(tone)) {
    throw new TypeError(`unknown tone: ${String(tone)}`);
  }
  if (typeof title !== 'string' || title === '') {
    throw new TypeError('alert title is required');
  }

  const root = document.createElement('div');
  root.className = `alert alert-tone-${tone}`;
  root.dataset.tone = tone;
  root.setAttribute('role', URGENT.has(tone) ? 'alert' : 'status');

  const body = document.createElement('div');
  body.className = 'alert-body';
  body.append(element('p', 'alert-title', title));
  if (message) body.append(element('p', 'alert-message', message));

  root.append(icon(ICONS[tone]), body);
  if (action) root.append(action);
  return root;
}
