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
 *   destructive?: boolean, onConfirm?: () => void, onCancel?: () => void,
 *   returnFocus?: HTMLElement}} options
 *   `returnFocus` is the element that opens the dialog. Cancel and Escape move focus back to it.
 * @returns {{element: HTMLDialogElement, show: () => void, hide: () => void}}
 */
export function createDialog({
  title, description, confirmLabel, cancelLabel, destructive = false, onConfirm, onCancel, returnFocus,
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

  // The browser returns focus to the element focused when the dialog opened. A mouse click in WebKit
  // (Safari) does not focus the button, so that element can be the page body. Cancel and Escape
  // therefore move focus to `returnFocus`. Confirm and hide() leave focus to the caller.
  const restoreFocus = () => {
    if (returnFocus?.isConnected && !returnFocus.disabled) returnFocus.focus();
  };
  // close() restores the browser's choice synchronously, so the cancel button can move focus right after it.
  // Escape closes the dialog after the 'cancel' event, so that path waits for the 'close' event.
  let restoreOnClose = false;
  const show = () => {
    if (!element.open) {
      restoreOnClose = false;
      element.showModal();
    }
  };
  const hide = () => {
    if (element.open) element.close();
  };
  element.addEventListener('close', () => {
    if (!restoreOnClose) return;
    restoreOnClose = false;
    restoreFocus();
  });

  cancel.addEventListener('click', () => {
    hide();
    restoreFocus();
    onCancel?.();
  });
  confirm.addEventListener('click', () => {
    onConfirm?.();
    hide();
  });
  // Escape fires 'cancel'; the browser closes the dialog itself.
  element.addEventListener('cancel', () => {
    restoreOnClose = true;
    onCancel?.();
  });
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
