// Card: titled section with an optional count, header actions, body, and footer.
import { createBadge } from './badge.js';

// Module-level counter gives each heading a unique id for aria-labelledby.
let seq = 0;

/**
 * @param {{title: string, description?: string, headingLevel?: number, count?: number | string, actions?: Node,
 *   body?: Node | string, footer?: Node | string}} options
 * @returns {HTMLElement}
 */
export function createCard({
  title, description, headingLevel = 2, count, actions, body, footer,
} = {}) {
  if (typeof title !== 'string' || title === '') {
    throw new TypeError('card title is required');
  }
  if (!Number.isInteger(headingLevel) || headingLevel < 1 || headingLevel > 6) {
    throw new TypeError(`invalid heading level: ${String(headingLevel)}`);
  }
  if (description !== undefined && typeof description !== 'string') {
    throw new TypeError('card description must be a string');
  }

  const section = document.createElement('section');
  section.className = 'card';
  const headingId = `card-title-${++seq}`;
  section.setAttribute('aria-labelledby', headingId);

  const head = document.createElement('header');
  head.className = 'card-head';

  const titleWrap = document.createElement('div');
  titleWrap.className = 'card-title';
  const heading = document.createElement(`h${headingLevel}`);
  heading.className = 'card-heading';
  heading.id = headingId;
  heading.textContent = title;
  titleWrap.append(heading);

  if (count !== undefined) {
    titleWrap.append(createBadge({ label: String(count) }));
  }
  head.append(titleWrap);

  if (actions) {
    const wrap = document.createElement('div');
    wrap.className = 'card-actions';
    wrap.append(actions);
    head.append(wrap);
  }
  if (description !== undefined) {
    const note = document.createElement('p');
    note.className = 'card-description';
    note.textContent = description;
    head.append(note);
  }
  section.append(head);

  if (body !== undefined) {
    const bodyEl = document.createElement('div');
    bodyEl.className = 'card-body';
    bodyEl.append(body);
    section.append(bodyEl);
  }

  if (footer !== undefined) {
    const foot = document.createElement('footer');
    foot.className = 'card-foot';
    foot.append(footer);
    section.append(foot);
  }
  return section;
}
