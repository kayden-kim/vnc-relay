// Operations overview (SCR-001). Builds the page, polls the management API, and runs sign-out.
// Each query polls P seconds after its previous response settles, so one query never overlaps itself.
// A response that arrives after a sign-out or a 401 is dropped (epoch check).

import { createButton } from './components/button.js';
import { BOOTSTRAP_TIMEOUT_MS, QUERY_PATHS, getQuery, getSession, isUnauthorized } from './api.js';
import { BRAND_NAME, COPY } from './copy.js';
import { fill, formatTime } from './format.js';
import { LANGUAGES, chooseLanguage, readSavedLanguage, saveLanguage } from './language.js';
import { parseClosed, parseSession, parseSessions, parseSummary, parseWaiting } from './model.js';
import { clearTimer, contentKey, newQuery, viewOf } from './query.js';
import {
  activeTable,
  applyChrome,
  applyListCount,
  closedTable,
  el,
  panelCard,
  pageHead,
  setContent,
  statusBadge,
  summaryGrid,
  usageGauge,
  waitingTable,
} from './panels.js';
import { LOGIN_URL, buildNavbar, defaultStorage, signOutUrl } from './shell.js';

export function mountOperations(root, deps = {}) {
  const request = deps.request ?? ((path, init) => globalThis.fetch(path, init));
  const storage = deps.storage === undefined ? defaultStorage() : deps.storage;
  const preferred = deps.preferredLanguages ?? globalThis.navigator?.languages ?? [];
  const navigate = deps.navigate ?? ((url) => globalThis.location.assign(url));

  const state = {
    lang: 'en',
    epoch: 0,
    ready: false,
    bootstrapping: false,
    csrfToken: null,
    pollMs: null,
    queryTimeoutMs: null,
    logoutBusy: false,
    ended: false,
  };

  const queries = {
    summary: newQuery(QUERY_PATHS.summary, parseSummary),
    waiting: newQuery(QUERY_PATHS.waiting, parseWaiting),
    active: newQuery(QUERY_PATHS.sessions, parseSessions),
    closed: newQuery(QUERY_PATHS.closed, parseClosed),
  };

  let page = null;

  function copy() {
    return COPY[state.lang];
  }

  function applyDocumentLanguage() {
    document.documentElement.lang = state.lang;
    document.title = `${BRAND_NAME} ${copy().operations}`;
  }

  function buildPage() {
    const t = copy();
    root.replaceChildren();

    const { navbar, select, logout } = buildNavbar({
      t,
      lang: state.lang,
      idPrefix: 'operations',
      onLanguageChange,
      onSignOut: signOut,
    });

    const refresh = createButton({ variant: 'outline', label: t.refresh, icon: 'refresh' });
    refresh.id = 'operations-refresh';
    refresh.addEventListener('click', onRefresh);

    const header = pageHead({ title: t.operations, actions: [refresh] });

    const summary = panelCard({ title: t.summaryTitle, span: 'span-6', chrome: true });
    const relay = panelCard({ title: t.relayTitle, description: t.relayNote, span: 'span-3', chrome: false });
    const admin = panelCard({ title: t.adminTitle, description: t.adminNote, span: 'span-3', chrome: false });
    const waiting = panelCard({ title: t.waitingTitle, span: 'span-12', chrome: true });
    const active = panelCard({ title: t.activeTitle, span: 'span-12', chrome: true });
    const closed = panelCard({ title: t.closedTitle, span: 'span-12', chrome: true });

    const grid = el('div', 'grid');
    grid.append(summary.node, relay.node, admin.node, waiting.node, active.node, closed.node);
    const main = el('main', 'app-main');
    main.append(header.head, grid);
    root.append(navbar, main);

    page = {
      select,
      logout,
      refresh,
      meta: header.meta,
      summary: summary.slots,
      relay: relay.slots,
      admin: admin.slots,
      waiting: waiting.slots,
      active: active.slots,
      closed: closed.slots,
    };
  }

  function renderSummary(t) {
    const query = queries.summary;
    const view = viewOf(query);
    applyChrome(page.summary, query, view, t);
    const key = contentKey(query, view, state.lang);
    setContent(page.summary, key, () => summaryGrid(t, query.data, view));
    setContent(page.relay, key, () =>
      usageGauge({ t, title: t.relayTitle, usage: query.data?.relay ?? null, kind: view }),
    );
    setContent(page.admin, key, () =>
      usageGauge({ t, title: t.adminTitle, usage: query.data?.admin ?? null, kind: view }),
    );
  }

  // Renders a list card. Before the first success it shows a loading row with the same info badge as the card head;
  // after a failure with no data it shows no table.
  function renderList(key, t, emptyText, build) {
    const query = queries[key];
    const view = viewOf(query);
    const slots = page[key];
    applyChrome(slots, query, view, t);
    applyListCount(slots, query.data ? query.data.items.length : null, t);
    setContent(slots, contentKey(query, view, state.lang), () => {
      if (query.data) {
        return build(query.data.items, emptyText);
      }
      if (view === 'loading') {
        return build([], statusBadge(t, 'loading'));
      }
      return null;
    });
  }

  function renderAll() {
    if (!page) {
      return;
    }
    const t = copy();
    renderSummary(t);
    renderList('waiting', t, t.waitingEmpty, (items, empty) => waitingTable(t, items, empty));
    renderList('active', t, t.activeEmpty, (items, empty) => activeTable(t, items, empty));
    renderList('closed', t, t.closedEmpty, (items, empty) => closedTable(t, items, empty));

    const times = Object.values(queries)
      .map((query) => query.lastSuccessAt)
      .filter((time) => time !== null);
    const latest = times.length > 0 ? Math.max(...times) : null;
    page.meta.textContent = fill(t.lastSuccess, {
      time: latest === null ? t.unavailable : formatTime(latest),
    });

    page.logout.disabled = state.logoutBusy;
    page.refresh.disabled = state.logoutBusy;
  }

  function stopPolling() {
    for (const query of Object.values(queries)) {
      clearTimer(query);
    }
  }

  // Ends the page: drops protected data, stops polling, and moves to the given URL.
  function endSession(url) {
    state.ended = true;
    state.epoch += 1;
    stopPolling();
    for (const query of Object.values(queries)) {
      query.data = null;
      query.lastSuccessAt = null;
    }
    root.replaceChildren();
    page = null;
    navigate(url);
  }

  // Called by the router before another screen mounts. Drops late responses and stops polling.
  function dispose() {
    state.ended = true;
    state.epoch += 1;
    stopPolling();
    page = null;
  }

  function schedule(key) {
    const query = queries[key];
    clearTimer(query);
    if (state.ended || !state.ready) {
      return;
    }
    query.timer = setTimeout(() => {
      query.timer = null;
      void runQuery(key, false);
    }, state.pollMs);
  }

  // `manual` runs without X-Relay-Poll, so the server counts it as user activity.
  async function runQuery(key, manual) {
    const query = queries[key];
    if (query.inFlight || state.ended || !state.ready) {
      return;
    }
    clearTimer(query);
    query.inFlight = true;
    const epoch = state.epoch;
    renderAll();
    try {
      const body = await getQuery(request, query.path, {
        poll: !manual,
        timeoutMs: state.queryTimeoutMs,
      });
      if (epoch !== state.epoch) {
        return;
      }
      query.data = query.parse(body);
      query.failed = false;
      query.lastSuccessAt = Date.now();
    } catch (error) {
      if (epoch !== state.epoch) {
        return;
      }
      if (isUnauthorized(error)) {
        endSession(LOGIN_URL);
        return;
      }
      query.failed = true;
    } finally {
      if (epoch === state.epoch) {
        query.inFlight = false;
        renderAll();
        schedule(key);
      }
    }
  }

  function refreshAll(manual) {
    for (const key of Object.keys(queries)) {
      clearTimer(queries[key]);
      void runQuery(key, manual);
    }
  }

  // Reads the session settings (poll interval, query timeout, CSRF token). Then the first query run starts.
  async function bootstrap() {
    if (state.bootstrapping || state.ended) {
      return;
    }
    state.bootstrapping = true;
    const epoch = state.epoch;
    for (const query of Object.values(queries)) {
      query.inFlight = true;
    }
    renderAll();
    try {
      const session = parseSession(await getSession(request, { timeoutMs: BOOTSTRAP_TIMEOUT_MS }));
      if (epoch !== state.epoch) {
        return;
      }
      state.csrfToken = session.csrfToken;
      state.pollMs = session.pollMs;
      state.queryTimeoutMs = session.queryTimeoutMs;
      state.ready = true;
      for (const query of Object.values(queries)) {
        query.inFlight = false;
      }
      refreshAll(true);
    } catch (error) {
      if (epoch !== state.epoch) {
        return;
      }
      if (isUnauthorized(error)) {
        endSession(LOGIN_URL);
        return;
      }
      for (const query of Object.values(queries)) {
        query.inFlight = false;
        query.failed = true;
      }
    } finally {
      state.bootstrapping = false;
      renderAll();
    }
  }

  function onRefresh() {
    if (state.logoutBusy || state.ended) {
      return;
    }
    if (state.ready) {
      refreshAll(true);
    } else {
      void bootstrap();
    }
  }

  // Sign-out succeeds on 204. A 401 also ends the session. Any other result leaves the outcome unconfirmed.
  async function signOut() {
    if (state.logoutBusy || state.ended) {
      return;
    }
    state.logoutBusy = true;
    state.epoch += 1;
    stopPolling();
    renderAll();
    endSession(await signOutUrl(request, state.csrfToken));
  }

  function onLanguageChange(lang) {
    if (!LANGUAGES.includes(lang) || lang === state.lang) {
      return;
    }
    state.lang = lang;
    saveLanguage(storage, lang);
    applyDocumentLanguage();
    buildPage();
    renderAll();
    page.select.focus();
  }

  state.lang = chooseLanguage(readSavedLanguage(storage), preferred);
  applyDocumentLanguage();
  buildPage();
  renderAll();
  void bootstrap();
  return { dispose };
}
