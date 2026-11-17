// Query state for the management API calls a screen shows. No DOM here: panels.js and the screens render from it.

// An automatic poll shows its busy state only after this delay. A poll that settles sooner leaves the screen as it was,
// so the status badge does not flash on every poll (dashboard.md §4.1).
export const BUSY_DELAY_MS = 1000;

export function newQuery(path, parse) {
  return {
    path,
    parse,
    inFlight: false,
    busyShown: false,
    busyTimer: null,
    failed: false,
    data: null,
    lastSuccessAt: null,
    timer: null,
  };
}

export function clearTimer(query) {
  if (query.timer !== null) {
    clearTimeout(query.timer);
    query.timer = null;
  }
}

// Marks a request as running. `query` is any object with the inFlight, busyShown, and busyTimer fields.
// A user request shows its busy state at once. An automatic poll shows it after BUSY_DELAY_MS and then calls `onShow`.
export function beginQuery(query, manual, onShow) {
  clearBusyTimer(query);
  query.inFlight = true;
  query.busyShown = manual;
  if (!manual) {
    query.busyTimer = setTimeout(() => {
      query.busyTimer = null;
      query.busyShown = true;
      onShow();
    }, BUSY_DELAY_MS);
  }
}

// Shows the busy state of a running request now: the user asked to refresh while an automatic poll ran.
// Returns true when the state changed and the screen must render.
export function showBusy(query) {
  if (!query.inFlight || query.busyShown) {
    return false;
  }
  clearBusyTimer(query);
  query.busyShown = true;
  return true;
}

// Marks a request as settled or dropped.
export function endQuery(query) {
  clearBusyTimer(query);
  query.inFlight = false;
  query.busyShown = false;
}

export function clearBusyTimer(query) {
  if (query.busyTimer !== null) {
    clearTimeout(query.busyTimer);
    query.busyTimer = null;
  }
}

// View of a query for the screen: loading, refreshing, ok, stale, or failed.
// A running request whose busy state is not shown yet keeps the view of the last settled request.
export function viewOf(query) {
  if (query.inFlight && query.busyShown) {
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
