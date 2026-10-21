// Button: native <button> with a variant, optional icon, and busy/disabled states.
import { icon } from './icons.js';

const VARIANTS = ['primary', 'outline', 'ghost', 'destructive'];
const TYPES = ['button', 'submit', 'reset'];

/**
 * @param {{variant?: string, label: string, icon?: string, disabled?: boolean,
 *   busy?: boolean, type?: string}} options
 * @returns {HTMLButtonElement}
 */
export function createButton({
  variant = 'primary', label, icon: iconName, disabled = false, busy = false, type = 'button',
} = {}) {
  if (!VARIANTS.includes(variant)) {
    throw new TypeError(`unknown button variant: ${String(variant)}`);
  }
  if (!TYPES.includes(type)) {
    throw new TypeError(`unknown button type: ${String(type)}`);
  }
  if (typeof label !== 'string' || label === '') {
    throw new TypeError('button label is required');
  }

  const button = document.createElement('button');
  button.type = type;
  button.className = `button button-${variant}`;
  if (iconName) button.append(icon(iconName));

  const text = document.createElement('span');
  text.className = 'button-label';
  text.textContent = label;
  button.append(text);

  // Busy blocks repeat clicks the same way disabled does.
  button.disabled = disabled || busy;
  if (busy) button.setAttribute('aria-busy', 'true');
  return button;
}
