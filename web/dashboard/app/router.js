// Hash router of the dashboard. `#/sessions/<id>` shows the session detail (SCR-002). Any other hash shows the operations overview (SCR-001).
// The previous screen is disposed before the next one mounts, so late responses from it are dropped.

import { mountOperations } from './operations.js';
import { mountSession } from './session.js';

const SESSION_HASH = /^#\/sessions\/([^/]+)$/;

// Route of a hash. The id is decoded. When decoding fails the raw text is kept, and the id check then rejects it.
export function routeOf(hash) {
  const match = SESSION_HASH.exec(hash);
  if (!match) {
    return { name: 'operations' };
  }
  let id = match[1];
  try {
    id = decodeURIComponent(id);
  } catch {
    // Keep the raw text. The session id check rejects it before any request.
  }
  return { name: 'session', id };
}

// Mounts the screen of the current hash, and again on each hash change.
export function mountRouter(root, deps = {}) {
  const location = deps.location ?? globalThis.location;
  let current = null;

  function show() {
    current?.dispose();
    current = null;
    root.replaceChildren();
    const route = routeOf(location.hash);
    current = route.name === 'session' ? mountSession(root, route.id, deps) : mountOperations(root, deps);
  }

  globalThis.addEventListener('hashchange', show);
  show();
}
