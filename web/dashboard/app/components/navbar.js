// Navbar: top bar with a brand, a labelled nav of links, and optional language and action slots.
import { icon } from './icons.js';
import { createLink } from './link.js';

/**
 * @param {{brand?: string, items?: Array<{label: string, href: string, current?: boolean,
 *   desktopOnly?: boolean}>,
 *   language?: Node, action?: Node, label?: string}} options
 * @returns {HTMLElement}
 */
export function createNavbar({
  brand, items = [], language, action, label = '주 메뉴',
} = {}) {
  if (typeof label !== 'string' || label === '') {
    throw new TypeError('navbar label is required');
  }
  if (brand !== undefined && typeof brand !== 'string') {
    throw new TypeError('navbar brand must be a string');
  }
  if (!Array.isArray(items)) {
    throw new TypeError('navbar items must be an array');
  }

  const header = document.createElement('header');
  header.className = 'navbar';

  if (brand) {
    const home = document.createElement('div');
    home.className = 'navbar-brand';
    const mark = document.createElement('span');
    mark.className = 'navbar-mark';
    mark.append(icon('brand'));
    const name = document.createElement('span');
    name.className = 'navbar-brand-text';
    name.textContent = brand;
    home.append(mark, name);
    header.append(home);
  }

  const nav = document.createElement('nav');
  nav.className = 'navbar-nav';
  nav.setAttribute('aria-label', label);
  for (const item of items) {
    const link = createLink({ href: item.href, label: item.label });
    link.className = 'navbar-item';
    if (item.desktopOnly) link.classList.add('navbar-desktop-only');
    if (item.current) link.setAttribute('aria-current', 'page');
    nav.append(link);
  }
  header.append(nav);

  if (language || action) {
    const tools = document.createElement('div');
    tools.className = 'navbar-tools';
    if (language) tools.append(language);
    if (action) tools.append(action);
    header.append(tools);
  }
  return header;
}
