// 생성 파일이다. 직접 고치지 않는다. 원본: web/dashboard/app/components/language-select.js
// Language select: visually hidden label, native select, globe icon, and chevron.
import { icon } from './icons.js';

const VARIANTS = ['navbar', 'form'];

/**
 * @param {{id: string, label: string, value?: string,
 *   options: Array<{value: string, label: string}>, variant?: 'navbar' | 'form'}} options
 * @returns {HTMLDivElement}
 */
export function createLanguageSelect({
  id, label, value, options, variant = 'form',
} = {}) {
  if (typeof id !== 'string' || id === '') {
    throw new TypeError('language select id is required');
  }
  if (typeof label !== 'string' || label === '') {
    throw new TypeError('language select label is required');
  }
  if (!VARIANTS.includes(variant)) {
    throw new TypeError(`unknown language select variant: ${String(variant)}`);
  }
  if (!Array.isArray(options) || options.length === 0) {
    throw new TypeError('language select options are required');
  }

  const root = document.createElement('div');
  root.className = `language-select language-select-${variant}`;

  const labelEl = document.createElement('label');
  labelEl.className = 'sr-only';
  labelEl.htmlFor = id;
  labelEl.textContent = label;

  const select = document.createElement('select');
  select.className = 'language-select-control';
  select.id = id;
  for (const option of options) {
    if (typeof option?.value !== 'string' || typeof option?.label !== 'string') {
      throw new TypeError('each language option needs a string value and label');
    }
    const opt = document.createElement('option');
    opt.value = option.value;
    opt.textContent = option.label;
    select.append(opt);
  }
  if (value !== undefined) select.value = value;

  root.append(labelEl, icon('globe'), select, icon('chevron-down', { size: 'sm' }));
  return root;
}
