// Dialog: native modal dialog with a title, optional description, and confirm/cancel buttons.
import { createButton } from './button.js';

// Module-level counter gives each dialog unique heading and description ids.
let seq = 0;

function requireText(value, name) {
  if (typeof value !== 'string' || value === '') {
    throw new TypeError(`${name} is required`);
  }
}

/**
 * @param {{title: string, description?: string, confirmLabel: string, cancelLabel: string,
 *   destructive?: boolean, onConfirm?: () => void, onCancel?: () => void}} options
 * @returns {{element: HTMLDialogElement, show: () => void, hide: () => void}}
 */
export function createDialog({
  title, description, confirmLabel, cancelLabel, destructive = false, onConfirm, onCancel,
} = {}) {
  requireText(title, 'dialog title');
  requireText(confirmLabel, 'dialog confirm label');
  requireText(cancelLabel, 'dialog cancel label');
  for (const callback of [onConfirm, onCancel]) {
    if (callback !== undefined && typeof callback !== 'function') {
      throw new TypeError('dialog callbacks must be functions');
    }
  }

  const id = ++seq;
  const element = document.createElement('dialog');
  element.className = 'dialog';

  const heading = document.createElement('h2');
  heading.className = 'dialog-title';
  heading.id = `dialog-title-${id}`;
  heading.textContent = title;
  element.setAttribute('aria-labelledby', heading.id);
  element.append(heading);

  if (description) {
    const text = document.createElement('p');
    text.className = 'dialog-description';
    text.id = `dialog-description-${id}`;
    text.textContent = description;
    element.setAttribute('aria-describedby', text.id);
    element.append(text);
  }

  const cancel = createButton({ variant: 'outline', label: cancelLabel });
  const confirm = createButton({
    variant: destructive ? 'destructive' : 'primary',
    label: confirmLabel,
  });
  const actions = document.createElement('div');
  actions.className = 'dialog-actions';
  actions.append(cancel, confirm);
  element.append(actions);

  // close() restores focus to the element that opened the dialog.
  const show = () => {
    if (!element.open) element.showModal();
  };
  const hide = () => {
    if (element.open) element.close();
  };

  cancel.addEventListener('click', () => {
    hide();
    onCancel?.();
  });
  confirm.addEventListener('click', () => {
    onConfirm?.();
    hide();
  });
  // Escape fires 'cancel'; the browser closes the dialog itself.
  element.addEventListener('cancel', () => onCancel?.());
  // Tab stays inside the modal. Without this, the browser moves focus out of the page after the last action.
  element.addEventListener('keydown', (event) => {
    if (event.key !== 'Tab') return;
    if (event.shiftKey && document.activeElement === cancel) {
      event.preventDefault();
      confirm.focus();
    } else if (!event.shiftKey && document.activeElement === confirm) {
      event.preventDefault();
      cancel.focus();
    }
  });

  return { element, show, hide };
}
