// Link: anchor with an optional leading icon.
import { icon } from './icons.js';

/**
 * @param {{href: string, label: string, icon?: string}} options
 * @returns {HTMLAnchorElement}
 */
export function createLink({ href, label, icon: iconName } = {}) {
  if (typeof href !== 'string' || href === '') {
    throw new TypeError('link href is required');
  }
  if (typeof label !== 'string' || label === '') {
    throw new TypeError('link label is required');
  }

  const link = document.createElement('a');
  link.className = 'link';
  link.setAttribute('href', href);
  if (iconName) link.append(icon(iconName, { size: 'sm' }));
  link.append(label);
  return link;
}
