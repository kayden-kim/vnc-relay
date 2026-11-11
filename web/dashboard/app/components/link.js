// Link: anchor with an optional leading icon and an accessible new-tab notice.
import { icon } from './icons.js';

/**
 * @param {{href: string, label: string, icon?: string, iconSize?: 'md' | 'sm', newTabText?: string}} options
 * @returns {HTMLAnchorElement}
 */
export function createLink({ href, label, icon: iconName, iconSize = 'sm', newTabText } = {}) {
  if (typeof href !== 'string' || href === '') {
    throw new TypeError('link href is required');
  }
  if (typeof label !== 'string' || label === '') {
    throw new TypeError('link label is required');
  }

  const link = document.createElement('a');
  link.className = 'link';
  link.setAttribute('href', href);
  if (iconName) link.append(icon(iconName, { size: iconSize }));
  const text = document.createElement('span');
  text.textContent = label;
  link.append(text);
  if (newTabText) {
    link.target = '_blank';
    link.rel = 'noopener noreferrer';
    const notice = document.createElement('span');
    notice.className = 'sr-only';
    notice.textContent = ` (${newTabText})`;
    link.append(icon('external-link', { size: 'sm' }), notice);
  }
  return link;
}
