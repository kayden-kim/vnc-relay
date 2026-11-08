// Hash router for operations (SCR-001), session details (SCR-002), and settings (SCR-004·005).
// The previous screen is disposed before the next one mounts, so late responses from it are dropped.

import { mountOperations } from './operations.js';
import { mountSession } from './session.js';
import { mountSettings } from './settings.js';
import { mountLogs } from './logs.js';

const SESSION_HASH = /^#\/sessions\/([^/]+)$/;

// Route of a hash. The id is decoded. When decoding fails the raw text is kept, and the id check then rejects it.
export function routeOf(hash) {
  if (hash === '#/settings' || hash === '#/settings/edit') {
    return { name: 'settings', edit: hash === '#/settings/edit' };
  }
  if (hash === '#/logs' || hash.startsWith('#/logs?')) {
    const queryPart = hash.includes('?') ? hash.slice(hash.indexOf('?') + 1) : '';
    const params = new URLSearchParams(queryPart);
    const sessionId = params.get('session_id');
    return { name: 'logs', sessionId };
  }
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
  let applied = null;

  // Successful settings edits carry the applied-key count in memory, never in the URL or storage.
  function navigate(url, options = {}) {
    if (url.startsWith('/#/logs') || url === '/#/settings' || url === '/#/settings/edit' || url === '/#/' || url === '/') {
      applied = options.applied ?? null;
      const hash = url.startsWith('/') ? url.slice(1) : url;
      if (location.hash === hash) show();
      else location.hash = hash;
      return;
    }
    (deps.navigate ?? ((target) => globalThis.location.assign(target)))(url);
  }

  const screenDeps = { ...deps, navigate };

  function show() {
    current?.dispose();
    current = null;
    root.replaceChildren();
    const route = routeOf(location.hash);
    const count = applied;
    applied = null;
    if (route.name === 'settings') {
      current = mountSettings(root, screenDeps, { edit: route.edit, applied: count });
    } else if (route.name === 'logs') {
      current = mountLogs(root, screenDeps, { sessionId: route.sessionId });
    } else if (route.name === 'session') {
      current = mountSession(root, route.id, screenDeps);
    } else {
      current = mountOperations(root, screenDeps);
    }
  }

  globalThis.addEventListener('hashchange', show);
  show();
  return {
    dispose() {
      globalThis.removeEventListener('hashchange', show);
      current?.dispose();
      current = null;
    },
  };
}
