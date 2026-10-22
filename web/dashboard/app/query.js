// Query state for the management API calls a screen shows. No DOM here: panels.js and the screens render from it.

export function newQuery(path, parse) {
  return { path, parse, inFlight: false, failed: false, data: null, lastSuccessAt: null, timer: null };
}

export function clearTimer(query) {
  if (query.timer !== null) {
    clearTimeout(query.timer);
    query.timer = null;
  }
}

// View of a query for the screen: loading, refreshing, ok, stale, or failed.
export function viewOf(query) {
  if (query.inFlight) {
    return query.data ? 'refreshing' : 'loading';
  }
  if (query.failed) {
    return query.data ? 'stale' : 'failed';
  }
  return query.data ? 'ok' : 'loading';
}

// Key that changes only when the rendered content must change. Rebuilding only on change keeps focus steady.
export function contentKey(query, view, lang) {
  return [lang, query.data ? 'data' : view, JSON.stringify(query.data)].join('|');
}
