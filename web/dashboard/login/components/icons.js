// 생성 파일이다. 직접 고치지 않는다. 원본: web/dashboard/app/components/icons.js
// Inline SVG icons. Paths come from the dashboard mockup (viewBox 0 0 16 16).
// Markup is a static string, parsed once into a cached template and cloned per use.

const PATHS = {
  check: '<circle cx="8" cy="8" r="6.25"/><path d="M5.25 8.25l1.9 1.9 3.6-4.15"/>',
  clock: '<circle cx="8" cy="8" r="6.25"/><path d="M8 4.75V8l2.25 1.5"/>',
  warning: '<path d="M8 2.25l6.25 11H1.75z"/><path d="M8 6.5v3M8 11.25h.01"/>',
  alert: '<circle cx="8" cy="8" r="6.25"/><path d="M8 5v3.5M8 10.9h.01"/>',
  refresh: '<path d="M13.25 8a5.25 5.25 0 1 1-1.54-3.71"/><path d="M13.25 2.75v2.5h-2.5"/>',
  logout: '<path d="M6.25 13.25h-2.5a1 1 0 0 1-1-1v-8.5a1 1 0 0 1 1-1h2.5"/><path d="M10.25 11l3-3-3-3"/><path d="M13.25 8h-7"/>',
  back: '<path d="M12.75 8h-9.5"/><path d="M7.25 4l-4 4 4 4"/>',
  // Not in the mockup: mirrors chevron-down.
  'chevron-right': '<path d="M6.25 4.5 9.75 8l-3.5 3.5"/>',
  'chevron-down': '<path d="M4.5 6.25 8 9.75l3.5-3.5"/>',
  cancel: '<circle cx="8" cy="8" r="6.25"/><path d="M5.5 8h5"/>',
  globe: '<circle cx="8" cy="8" r="6.25"/><path d="M1.75 8h12.5"/><path d="M8 1.75c1.8 1.7 2.7 3.8 2.7 6.25S9.8 12.55 8 14.25C6.2 12.55 5.3 10.45 5.3 8S6.2 3.45 8 1.75z"/>',
  grid: '<rect x="2.25" y="2.25" width="4.5" height="4.5" rx="1"/><rect x="9.25" y="2.25" width="4.5" height="4.5" rx="1"/><rect x="2.25" y="9.25" width="4.5" height="4.5" rx="1"/><rect x="9.25" y="9.25" width="4.5" height="4.5" rx="1"/>',
  brand: '<path d="M2.75 5.5h9.5"/><path d="M9.75 3l2.5 2.5-2.5 2.5"/><path d="M13.25 10.5h-9.5"/><path d="M6.25 8l-2.5 2.5 2.5 2.5"/>',
};

const SIZE_CLASS = { md: 'icon-md', sm: 'icon-sm' };
const cache = new Map();

function template(name) {
  if (!Object.hasOwn(PATHS, name)) {
    throw new TypeError(`unknown icon name: ${String(name)}`);
  }
  let tpl = cache.get(name);
  if (tpl === undefined) {
    tpl = document.createElement('template');
    // Static markup only: PATHS holds constant strings.
    tpl.innerHTML = '<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" '
      + 'stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">'
      + `${PATHS[name]}</svg>`;
    cache.set(name, tpl);
  }
  return tpl;
}

/**
 * Returns a new inline SVG element for a named icon.
 * @param {string} name one of the PATHS keys
 * @param {{size?: 'md' | 'sm'}} [options] size defaults to 'md'
 * @returns {SVGSVGElement}
 */
export function icon(name, { size = 'md' } = {}) {
  const tpl = template(name);
  if (!Object.hasOwn(SIZE_CLASS, size)) {
    throw new TypeError(`unknown icon size: ${String(size)}`);
  }
  const svg = tpl.content.firstElementChild.cloneNode(true);
  svg.setAttribute('class', `icon ${SIZE_CLASS[size]}`);
  return svg;
}
