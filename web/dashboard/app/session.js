// Session detail (SCR-002). Reads one session by id, runs terminate after confirmation, and reports each outcome.
// An invalid id sends no request. A terminate request is never resent. An unknown, failed, or rejected outcome is followed by a detail read.
// A response that arrives after a sign-out, a 401, or a dispose is dropped (epoch check). A response from an older query run is dropped (generation check).

import { createAlert } from './components/alert.js';
import { createBadge } from './components/badge.js';
import { createButton } from './components/button.js';
import { createDialog } from './components/dialog.js';
import { icon } from './components/icons.js';
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
import { BRAND_NAME, COPY } from './copy.js';
import { fill, formatAddress, formatBytes, formatMode, formatTime, formatTransport } from './format.js';
import { LANGUAGES, chooseLanguage, readSavedLanguage, saveLanguage } from './language.js';
import { isSessionId, parseDetail, parseSession, parseTerminate } from './model.js';
import { applyChrome, el, pageHead, panelCard, setContent } from './panels.js';
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
  let focusList = false;

  function copy() {
    return COPY[state.lang];
  }

  // Screen title as plain text, used for the document title.
  function sessionTitle(t) {
    return state.invalid ? t.sessionScreen : fill(t.sessionTitle, { session_id: state.id });
  }

  // Heading nodes. Only the session id sits in the data-style span; the invalid-id title stays plain text.
  function headingNodes(t) {
    if (state.invalid) {
      return [document.createTextNode(t.sessionScreen)];
    }
    const [before, after] = t.sessionTitle.split('{session_id}');
    return [
      document.createTextNode(before),
      el('span', 'session-id', state.id),
      document.createTextNode(after),
    ];
  }

  function applyDocumentLanguage() {
    document.documentElement.lang = state.lang;
    document.title = `${BRAND_NAME} ${sessionTitle(copy())}`;
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

    const back = el('a', 'button button-ghost');
    back.href = '/';
    back.append(icon('back'), el('span', 'button-label', t.back));

    const refresh = createButton({ variant: 'outline', label: t.refresh, icon: 'refresh' });
    refresh.id = 'session-refresh';
    refresh.addEventListener('click', onRefresh);

    // Re-check action in the failed, unknown, and rejected notices (CPY-014). It runs the same refresh.
    const recheck = createButton({ variant: 'outline', label: t.recheck, icon: 'refresh' });
    recheck.addEventListener('click', onRefresh);

    // List action in the gone notice (CPY-010). Focus moves here when the session is gone (spec §6).
    const listAction = el('a', 'button button-outline');
    listAction.href = '/';
    listAction.append(icon('back'), el('span', 'button-label', t.back));

    const terminate = createButton({ variant: 'destructive', label: t.terminate, icon: 'cancel' });
    terminate.id = 'session-terminate';

    const dialog = createDialog({
      title: t.terminateTitle,
      description: fill(t.terminateBody, { session_id: state.id }),
      confirmLabel: t.terminateConfirm,
      cancelLabel: t.cancel,
      destructive: true,
      returnFocus: terminate,
      onConfirm: () => {
        queueMicrotask(() => page?.heading.focus());
        void terminateSessionNow();
      },
    });
    terminate.addEventListener('click', () => dialog.show());

    const header = pageHead({ back, actions: [refresh] });
    header.heading.tabIndex = -1;

    const notice = el('div', 'session-notice');
    const info = panelCard({ title: t.sessionInfoTitle, span: 'span-8', chrome: true, footer: terminate });
    const transfer = panelCard({ title: t.transferTitle, span: 'span-4', chrome: false });
    const grid = el('div', 'grid');
    const main = el('main', 'app-main');
    main.append(header.head, notice, grid, dialog.element);
    root.append(navbar, main);

    page = {
      select,
      logout,
      back,
      listAction,
      refresh,
      recheck,
      terminate,
      dialog,
      heading: header.heading,
      meta: header.meta,
      actions: header.actions,
      notice,
      noticeKind: null,
      grid,
      infoNode: info.node,
      info: info.slots,
      transferNode: transfer.node,
      transfer: transfer.slots,
    };
  }

  function canPoll() {
    return !state.ended && state.ready && !state.invalid && !state.gone && !state.busy;
  }

  function canTerminate() {
    return canPoll() && !state.logoutBusy && !query.inFlight && !query.failed && query.data !== null;
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

  // The session is gone (closed or not found). No data, no polling, no terminate. Focus moves to the list action in the notice.
  function markGone() {
    state.gone = true;
    retire();
    focusList = true;
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

  // The notice kind for the state: an invalid id, a gone session, or the terminate outcome. Null shows no notice.
  function noticeKind() {
    if (state.invalid) {
      return 'invalid';
    }
    if (state.gone) {
      return 'gone';
    }
    return state.outcome;
  }

  // Gone carries the list action (spec §4). Failed, unknown, and rejected carry the re-check action (CPY-014).
  function noticeAlert(kind, t) {
    switch (kind) {
      case 'invalid':
        return createAlert({ tone: 'warning', title: t.invalidSessionId });
      case 'gone':
        return createAlert({ tone: 'neutral', title: t.notFound, action: page.listAction });
      case 'failed':
        return createAlert({ tone: 'destructive', title: t.terminateFailed, action: page.recheck });
      case 'unknown':
        return createAlert({ tone: 'warning', title: t.terminateUnknown, action: page.recheck });
      case 'rejected':
        return createAlert({ tone: 'warning', title: t.terminateRejected, action: page.recheck });
      default:
        return null;
    }
  }

  // Rebuilds the notice only when its kind changes. The alert and its re-check button stay in place across refreshes, so focus is kept.
  function renderNotice(t) {
    const kind = noticeKind();
    if (kind !== page.noticeKind) {
      page.noticeKind = kind;
      const alert = noticeAlert(kind, t);
      page.notice.replaceChildren(...(alert ? [alert] : []));
    }
    page.recheck.disabled = state.busy || state.logoutBusy;
  }

  // Facts of the session as a label and value list. Mode and transport are neutral badges. Addresses use the data style.
  function factsBlock(t, session) {
    const list = el('dl', 'fact-list');
    list.append(
      factRow(t.modeLabel, createBadge({ tone: 'neutral', label: formatMode(session.mode) })),
      factRow(t.transportLabel, createBadge({ tone: 'neutral', label: formatTransport(session.viewerTransport) })),
      factRow(t.startedLabel, formatTime(session.startedAt)),
      factRow(t.viewerAddressLabel, formatAddress(session.viewerAddress), 'fact-address'),
      factRow(t.targetAddressLabel, formatAddress(session.targetAddress), 'fact-address'),
    );
    const block = el('div', 'card-stack');
    block.append(list, el('p', 'card-note', t.addressNote));
    return block;
  }

  // One row of the facts list. `valueClass` marks the address values.
  function factRow(label, value, valueClass = null) {
    const row = el('div', 'fact');
    const dd = el('dd', 'fact-value');
    if (valueClass) {
      dd.classList.add(valueClass);
    }
    dd.append(value);
    row.append(el('dt', 'fact-label', label), dd);
    return row;
  }

  // Bytes per direction. Each value uses the metric style of the stat component. Each label leads with a decorative direction mark.
  function directionStat(label, bytes, markClass) {
    const stat = createStat({ label, value: formatBytes(bytes) });
    const mark = el('span', `direction-mark ${markClass}`);
    mark.setAttribute('aria-hidden', 'true');
    stat.querySelector('.stat-label').prepend(mark);
    return stat;
  }

  function bytesBlock(t, session) {
    const block = el('div', 'card-stack');
    block.append(
      directionStat(t.directionViewerToTarget, session.bytesViewerToTarget, 'direction-mark-1'),
      directionStat(t.directionTargetToViewer, session.bytesTargetToViewer, 'direction-mark-2'),
    );
    return block;
  }

  // Replaces children only when the list differs. Rebuilding nodes that already match would drop focus.
  function setChildren(parent, nodes) {
    const same =
      parent.children.length === nodes.length && nodes.every((node, index) => parent.children[index] === node);
    if (!same) {
      parent.replaceChildren(...nodes);
    }
  }

  function renderAll() {
    if (!page) {
      return;
    }
    const t = copy();
    const view = viewOf(query);
    const showInfo = !state.invalid;
    const showTransfer = showInfo && !state.gone;

    page.heading.replaceChildren(...headingNodes(t));
    page.meta.textContent = fill(t.lastSuccess, {
      time: query.lastSuccessAt === null ? t.unavailable : formatTime(query.lastSuccessAt),
    });

    renderNotice(t);

    if (showInfo) {
      // Gone has no badge or alert. The notice above the grid carries that state.
      applyChrome(page.info, query, state.gone ? 'gone' : view, t);
      setContent(page.info, contentKey(query, view, state.lang), () =>
        query.data ? factsBlock(t, query.data.session) : null,
      );
    }
    if (showTransfer) {
      setContent(page.transfer, contentKey(query, view, state.lang), () =>
        query.data ? bytesBlock(t, query.data.session) : null,
      );
    }
    const cards = [
      ...(showInfo ? [page.infoNode] : []),
      ...(showTransfer ? [page.transferNode] : []),
    ];
    setChildren(page.grid, cards);
    setChildren(page.actions, state.invalid || state.gone ? [] : [page.refresh]);
    page.logout.disabled = state.logoutBusy;
    page.refresh.disabled = state.busy || state.logoutBusy;
    page.terminate.disabled = !canTerminate();
    page.terminate.querySelector('.button-label').textContent = state.busy ? t.terminating : t.terminate;

    if (focusList) {
      focusList = false;
      page.listAction.focus();
    }
  }

  state.lang = chooseLanguage(readSavedLanguage(storage), preferred);
  applyDocumentLanguage();
  buildPage();
  renderAll();
  void bootstrap();
  return { dispose };
}
