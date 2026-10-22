// Session detail (SCR-002). Reads one session by id, runs terminate after confirmation, and reports each outcome.
// An invalid id sends no request. A terminate request is never resent. An unknown, failed, or rejected outcome is followed by a detail read.
// A response that arrives after a sign-out, a 401, or a dispose is dropped (epoch check). A response from an older query run is dropped (generation check).

import { createAlert } from './components/alert.js';
import { createButton } from './components/button.js';
import { createDialog } from './components/dialog.js';
import { createLink } from './components/link.js';
import { createStat } from './components/stat.js';
import {
  ApiError,
  BOOTSTRAP_TIMEOUT_MS,
  getQuery,
  getSession,
  isUnauthorized,
  sessionPath,
  terminateSession,
} from './api.js';
import { COPY } from './copy.js';
import { fill, formatAddress, formatBytes, formatMode, formatTime, formatTransport } from './format.js';
import { LANGUAGES, chooseLanguage, readSavedLanguage, saveLanguage } from './language.js';
import { isSessionId, parseDetail, parseSession, parseTerminate } from './model.js';
import { applyChrome, el, panelCard, setContent } from './panels.js';
import { clearTimer, contentKey, newQuery, viewOf } from './query.js';
import { LOGIN_URL, buildNavbar, defaultStorage, signOutUrl } from './shell.js';

export function mountSession(root, id, deps = {}) {
  const request = deps.request ?? ((path, init) => globalThis.fetch(path, init));
  const storage = deps.storage === undefined ? defaultStorage() : deps.storage;
  const preferred = deps.preferredLanguages ?? globalThis.navigator?.languages ?? [];
  const navigate = deps.navigate ?? ((url) => globalThis.location.assign(url));

  const state = {
    id,
    lang: 'en',
    epoch: 0,
    invalid: !isSessionId(id),
    gone: false,
    ready: false,
    bootstrapping: false,
    csrfToken: null,
    pollMs: null,
    queryTimeoutMs: null,
    terminateTimeoutMs: null,
    busy: false,
    outcome: null,
    logoutBusy: false,
    ended: false,
  };

  const query = newQuery(sessionPath(id), parseDetail);
  // Each detail run and each terminate start takes a new generation. Older responses are dropped.
  let generation = 0;
  let page = null;
  let focusBack = false;

  function copy() {
    return COPY[state.lang];
  }

  function applyDocumentLanguage() {
    document.documentElement.lang = state.lang;
    document.title = copy().sessionDocumentTitle;
  }

  function buildPage() {
    const t = copy();
    root.replaceChildren();

    const { navbar, select, logout } = buildNavbar({
      t,
      lang: state.lang,
      idPrefix: 'session',
      onLanguageChange,
      onSignOut: signOut,
    });

    const back = createLink({ href: '/', label: t.back });

    const refresh = createButton({ variant: 'outline', label: t.recheck, icon: 'refresh' });
    refresh.id = 'session-refresh';
    refresh.addEventListener('click', onRefresh);

    const terminate = createButton({ variant: 'destructive', label: t.terminate });
    terminate.id = 'session-terminate';

    const dialog = createDialog({
      title: t.terminateTitle,
      description: fill(t.terminateBody, { session_id: state.id }),
      confirmLabel: t.terminateConfirm,
      cancelLabel: t.cancel,
      destructive: true,
      onConfirm: () => {
        queueMicrotask(() => page?.heading.focus());
        void terminateSessionNow();
      },
    });
    terminate.addEventListener('click', () => dialog.show());

    const heading = el('h1', 'page-title');
    heading.tabIndex = -1;
    const meta = el('p', 'page-meta');
    const headText = el('div', 'page-head-text');
    headText.append(heading, meta);
    const actions = el('div', 'page-actions');
    const head = el('header', 'page-head');
    head.append(headText, actions);

    const notice = el('div', 'session-notice');
    const info = panelCard({ title: t.sessionInfoTitle, span: 'span-12', chrome: true });
    const grid = el('div', 'grid');
    const main = el('main', 'app-main');
    main.append(head, notice, grid, dialog.element);
    root.append(navbar, main);

    page = {
      select,
      logout,
      back,
      refresh,
      terminate,
      dialog,
      heading,
      meta,
      actions,
      notice,
      grid,
      infoNode: info.node,
      info: info.slots,
    };
  }

  function canPoll() {
    return !state.ended && state.ready && !state.invalid && !state.gone && !state.busy;
  }

  function canTerminate() {
    return canPoll() && !state.logoutBusy && !query.inFlight && query.data !== null;
  }

  function schedule() {
    clearTimer(query);
    if (!canPoll()) {
      return;
    }
    query.timer = setTimeout(() => {
      query.timer = null;
      void runQuery(false);
    }, state.pollMs);
  }

  // `manual` runs without X-Relay-Poll, so the server counts it as user activity.
  async function runQuery(manual) {
    clearTimer(query);
    generation += 1;
    const run = generation;
    const epoch = state.epoch;
    query.inFlight = true;
    renderAll();
    try {
      const body = await getQuery(request, query.path, { poll: !manual, timeoutMs: state.queryTimeoutMs });
      if (epoch !== state.epoch || run !== generation) {
        return;
      }
      query.data = query.parse(body);
      query.failed = false;
      query.lastSuccessAt = Date.now();
    } catch (error) {
      if (epoch !== state.epoch || run !== generation) {
        return;
      }
      if (isUnauthorized(error)) {
        endSession(LOGIN_URL);
        return;
      }
      if (error instanceof ApiError && error.status === 404) {
        markGone();
        return;
      }
      if (error instanceof ApiError && error.status === 400) {
        markInvalid();
        return;
      }
      query.failed = true;
    } finally {
      if (epoch === state.epoch && run === generation) {
        query.inFlight = false;
        renderAll();
        schedule();
      }
    }
  }

  // Reads the session settings (poll interval, query timeout, terminate timeout, CSRF token). Then the first detail read starts.
  async function bootstrap() {
    if (state.bootstrapping || state.ended || state.invalid) {
      return;
    }
    state.bootstrapping = true;
    const epoch = state.epoch;
    query.inFlight = true;
    renderAll();
    try {
      const session = parseSession(await getSession(request, { timeoutMs: BOOTSTRAP_TIMEOUT_MS }));
      if (epoch !== state.epoch) {
        return;
      }
      state.csrfToken = session.csrfToken;
      state.pollMs = session.pollMs;
      state.queryTimeoutMs = session.queryTimeoutMs;
      state.terminateTimeoutMs = session.terminateTimeoutMs;
      state.ready = true;
      query.inFlight = false;
      void runQuery(true);
    } catch (error) {
      if (epoch !== state.epoch) {
        return;
      }
      if (isUnauthorized(error)) {
        endSession(LOGIN_URL);
        return;
      }
      query.inFlight = false;
      query.failed = true;
    } finally {
      state.bootstrapping = false;
      renderAll();
    }
  }

  function onRefresh() {
    if (state.logoutBusy || state.ended || state.busy) {
      return;
    }
    if (!state.ready) {
      void bootstrap();
      return;
    }
    if (!query.inFlight) {
      void runQuery(true);
    }
  }

  // Maps a failed terminate request to an outcome. Only a response with a status says what the server did.
  function classify(error) {
    if (isUnauthorized(error)) {
      return 'unauthorized';
    }
    if (!(error instanceof ApiError) || error.status === null) {
      return 'unknown';
    }
    if (error.status === 404) {
      return 'gone';
    }
    if (error.status === 400) {
      return 'invalid';
    }
    if (error.status === 500 && error.code === 'terminate_failed') {
      return 'failed';
    }
    if (error.status >= 400 && error.status < 500) {
      return 'rejected';
    }
    return 'unknown';
  }

  // Sends terminate once. Returns closed, gone, invalid, rejected, failed, unauthorized, unknown, or stale.
  // A token fetch that fails sends no terminate, so it counts as failed.
  async function sendTerminate() {
    const epoch = state.epoch;
    if (state.csrfToken === null) {
      try {
        state.csrfToken = parseSession(await getSession(request, { timeoutMs: BOOTSTRAP_TIMEOUT_MS })).csrfToken;
      } catch (error) {
        return isUnauthorized(error) ? 'unauthorized' : 'failed';
      }
      if (epoch !== state.epoch) {
        return 'stale';
      }
    }
    try {
      parseTerminate(
        await terminateSession(request, id, { csrfToken: state.csrfToken, timeoutMs: state.terminateTimeoutMs }),
      );
      return 'closed';
    } catch (error) {
      return classify(error);
    }
  }

  // Runs after the confirm dialog. Duplicate starts are blocked. A completed terminate shows not-found and stops polling.
  async function terminateSessionNow() {
    if (!canTerminate()) {
      return;
    }
    const epoch = state.epoch;
    state.busy = true;
    state.outcome = null;
    generation += 1;
    query.inFlight = false;
    clearTimer(query);
    renderAll();

    const result = await sendTerminate();
    if (epoch !== state.epoch || result === 'stale') {
      return;
    }
    state.busy = false;
    switch (result) {
      case 'unauthorized':
        endSession(LOGIN_URL);
        return;
      case 'closed':
      case 'gone':
        markGone();
        renderAll();
        return;
      case 'invalid':
        markInvalid();
        renderAll();
        return;
      case 'rejected':
        state.csrfToken = null;
        state.outcome = 'rejected';
        break;
      case 'failed':
        state.outcome = 'failed';
        break;
      default:
        state.outcome = 'unknown';
        break;
    }
    renderAll();
    void runQuery(true);
  }

  // The session is gone (closed or not found). No data, no polling, no terminate. Focus moves to the back link.
  function markGone() {
    state.gone = true;
    retire();
    focusBack = true;
  }

  // The id was rejected (400). The invalid-id notice is shown and no further request is sent.
  function markInvalid() {
    state.invalid = true;
    retire();
  }

  function retire() {
    state.outcome = null;
    clearTimer(query);
    query.data = null;
    page?.dialog.hide();
  }

  // Sign-out succeeds on 204. A 401 also ends the session. Any other result leaves the outcome unconfirmed.
  async function signOut() {
    if (state.logoutBusy || state.ended) {
      return;
    }
    state.logoutBusy = true;
    state.epoch += 1;
    generation += 1;
    query.inFlight = false;
    clearTimer(query);
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

  // Ends the page: drops protected data, stops polling, and moves to the given URL.
  function endSession(url) {
    state.ended = true;
    state.epoch += 1;
    clearTimer(query);
    query.data = null;
    query.lastSuccessAt = null;
    root.replaceChildren();
    page = null;
    navigate(url);
  }

  // Called by the router before another screen mounts. Drops late responses and stops polling.
  function dispose() {
    state.ended = true;
    state.epoch += 1;
    clearTimer(query);
    page = null;
  }

  function noticeAlert(t) {
    if (state.invalid) {
      return createAlert({ tone: 'warning', title: t.invalidSessionId });
    }
    if (state.gone) {
      return createAlert({ tone: 'neutral', title: t.notFound });
    }
    if (state.outcome === 'failed') {
      return createAlert({ tone: 'destructive', title: t.terminateFailed });
    }
    if (state.outcome === 'unknown') {
      return createAlert({ tone: 'warning', title: t.terminateUnknown });
    }
    if (state.outcome === 'rejected') {
      return createAlert({ tone: 'warning', title: t.terminateRejected });
    }
    return null;
  }

  // Facts of the session. Values are shown as received. Only labels and the address note are translated.
  function factsBlock(t, session) {
    const stats = el('div', 'summary-grid session-facts');
    stats.append(
      createStat({ label: t.stateLabel, value: session.state }),
      createStat({ label: t.modeLabel, value: formatMode(session.mode) }),
      createStat({ label: t.transportLabel, value: formatTransport(session.viewerTransport) }),
      createStat({ label: t.startedLabel, value: formatTime(session.startedAt) }),
      createStat({ label: t.viewerAddressLabel, value: formatAddress(session.viewerAddress) }),
      createStat({ label: t.targetAddressLabel, value: formatAddress(session.targetAddress) }),
      createStat({ label: t.directionViewerToTarget, value: formatBytes(session.bytesViewerToTarget) }),
      createStat({ label: t.directionTargetToViewer, value: formatBytes(session.bytesTargetToViewer) }),
    );
    const block = el('div', 'card-stack');
    block.append(stats, el('p', 'card-note', t.addressNote));
    return block;
  }

  function actionNodes() {
    if (state.invalid) {
      return [page.back];
    }
    if (state.gone) {
      return [page.back, page.terminate];
    }
    return [page.back, page.refresh, page.terminate];
  }

  function renderAll() {
    if (!page) {
      return;
    }
    const t = copy();
    const view = viewOf(query);
    const showInfo = !state.invalid && !state.gone;

    page.heading.textContent = state.invalid ? t.sessionScreen : fill(t.sessionTitle, { session_id: state.id });
    page.meta.textContent = fill(t.lastSuccess, {
      time: query.lastSuccessAt === null ? t.unavailable : formatTime(query.lastSuccessAt),
    });

    const alert = noticeAlert(t);
    page.notice.replaceChildren(...(alert ? [alert] : []));

    if (showInfo) {
      applyChrome(page.info, query, view, t);
      setContent(page.info, contentKey(query, view, state.lang), () =>
        query.data ? factsBlock(t, query.data.session) : null,
      );
    }
    page.grid.replaceChildren(...(showInfo ? [page.infoNode] : []));

    page.actions.replaceChildren(...actionNodes());
    page.logout.disabled = state.logoutBusy;
    page.refresh.disabled = state.busy || state.logoutBusy;
    page.terminate.disabled = !canTerminate();
    page.terminate.textContent = state.busy ? t.terminating : t.terminate;

    if (focusBack) {
      focusBack = false;
      page.back.focus();
    }
  }

  state.lang = chooseLanguage(readSavedLanguage(storage), preferred);
  applyDocumentLanguage();
  buildPage();
  renderAll();
  void bootstrap();
  return { dispose };
}
