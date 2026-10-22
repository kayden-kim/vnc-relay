// Generated file. Do not edit. Source: web/dashboard/app/components/text-field.js
// Text field: label, native input, optional help text, and optional error text.
import { icon } from './icons.js';

const TYPES = ['text', 'password'];

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/**
 * @param {{id: string, label: string, type?: 'text' | 'password', value?: string,
 *   required?: boolean, autocomplete?: string, helpText?: string, error?: string}} options
 * @returns {HTMLDivElement}
 */
export function createTextField({
  id, label, type = 'text', value = '', required = false, autocomplete, helpText, error,
} = {}) {
  if (typeof id !== 'string' || id === '') {
    throw new TypeError('text field id is required');
  }
  if (typeof label !== 'string' || label === '') {
    throw new TypeError('text field label is required');
  }
  if (!TYPES.includes(type)) {
    throw new TypeError(`unknown text field type: ${String(type)}`);
  }

  const root = element('div', 'text-field');
  const labelEl = element('label', 'text-field-label', label);
  labelEl.htmlFor = id;

  const input = element('input', 'text-field-input');
  input.id = id;
  input.type = type;
  input.value = value;
  input.required = required;
  if (autocomplete) input.autocomplete = autocomplete;
  root.append(labelEl, input);

  const help = helpText ? element('p', 'text-field-help', helpText) : null;
  if (help) {
    help.id = `${id}-help`;
    root.append(help);
  }

  if (error) {
    setTextFieldError(root, error);
  } else if (help) {
    input.setAttribute('aria-describedby', help.id);
  }
  return root;
}

/**
 * Shows `error` under a field made by createTextField, or removes the error when `error`
 * is empty. aria-invalid and aria-describedby follow the error.
 * @param {HTMLDivElement} field
 * @param {string | undefined} error
 */
export function setTextFieldError(field, error) {
  const input = field.querySelector('.text-field-input');
  field.querySelector('.text-field-error')?.remove();
  if (error) {
    const err = element('p', 'text-field-error');
    err.id = `${input.id}-error`;
    err.append(icon('alert', { size: 'sm' }), error);
    field.append(err);
    input.setAttribute('aria-invalid', 'true');
    // With an error, the error message is the only description.
    input.setAttribute('aria-describedby', err.id);
    return;
  }
  input.setAttribute('aria-invalid', 'false');
  const help = field.querySelector('.text-field-help');
  if (help) {
    input.setAttribute('aria-describedby', help.id);
  } else {
    input.removeAttribute('aria-describedby');
  }
}
