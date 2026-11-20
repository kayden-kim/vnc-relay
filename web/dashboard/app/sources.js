// SCR-008 Sources screen: connection counts by client address, country, and ASN for the last hour or the last 24
// hours, with the state of the IP information databases.
// State, requests, and rendering live here. The response check and the small rules live in sources-model.js.
import { BRAND_NAME, COPY, PROVIDER_ATTRIBUTION } from './copy.js';
import { chooseLanguage, readSavedLanguage, saveLanguage } from './language.js';
import { fill, formatCount, formatTime } from './format.js';
import { parseSession } from './model.js';
import { ApiError, BOOTSTRAP_TIMEOUT_MS, getQuery, getSession, isUnauthorized } from './api.js';
import { createAlert } from './components/alert.js';
import { createBadge } from './components/badge.js';
import { createButton } from './components/button.js';
import { createCard } from './components/card.js';
import { createDataTable } from './components/data-table.js';
import { createDialog } from './components/dialog.js';
import { createLink } from './components/link.js';
import { icon } from './components/icons.js';
import { buildNavbar, defaultStorage, signOutUrl, LOGIN_URL } from './shell.js';
import { beginQuery, clearBusyTimer, endQuery, showBusy, viewOf } from './query.js';
import { el, statusBadge, timeStack } from './panels.js';
import {
  DATABASE_ROWS,
  SUMMARY_KEYS,
  attributedProviders,
  countryLabel,
  denyCell,
  parseSources,
  storageLimited,
  summaryValues,
  tableCount,
} from './sources-model.js';
import { changeDenyList } from './sources-deny.js';
import { diagnosticText } from './settings-model.js';

const SOURCES_PATH = '/api/v1/sources';
const LOOKUP_URL = 'https://ipinfo.io/';

// Badge tone and copy key of each known database state (dashboard.md SCR-008 IP 정보 DB 규칙).
const DB_STATES = {
  not_configured: ['neutral', 'ipDbNotConfigured'],
  loaded: ['success', 'ipDbLoaded'],
  stale: ['warning', 'ipDbStale'],
  unavailable: ['destructive', 'ipDbUnavailable'],
};

// Copy key of each known cause code (CPY-272~275).
const DB_REASONS = {
  file_unreadable: 'ipDbFileUnreadable',
  file_too_large: 'ipDbFileTooLarge',
  invalid_format: 'ipDbInvalidFormat',
  unsupported_data: 'ipDbUnsupportedData',
};

// The cause sentence of CPY-270·271 ("원인: {reason}." / "Cause: {reason}."), removed when `error` is null.
const CAUSE_SENTENCE = / ?[^.]*\{reason\}\./;

export function mountSources(root, deps = {}) {
  const rawRequest = deps.request ?? ((path, init) => globalThis.fetch(path, init));
  const storage = deps.storage === undefined ? defaultStorage() : deps.storage;
  const navigate = deps.navigate ?? ((url) => globalThis.location.assign(url));
  const preferred = deps.preferredLanguages ?? globalThis.navigator?.languages ?? [];
  const lifecycle = new AbortController();
  const request = (path, init) =>
    rawRequest(path, { ...init, signal: AbortSignal.any([lifecycle.signal, init?.signal].filter(Boolean)) });

  // `inFlight`, `busyShown`, `busyTimer`, `failed`, `data`, and `lastSuccessAt` follow query.js, so its helpers apply.
  const state = {
    lang: chooseLanguage(readSavedLanguage(storage), preferred),
    epoch: 0,
    ended: false,
    pollMs: null,
    timeoutMs: null,
    csrfToken: null,
    pollTimer: null,
    inFlight: false,
    busyShown: false,
    busyTimer: null,
    failed: false,
    // The last failure was a 400 or 405: automatic polling stays stopped until a refresh or a period change
    // succeeds (dashboard.md 4.1·5.4).
    rejected: false,
    stopped: false,
    data: null,
    lastSuccessAt: null,
    // The period the select names. It starts at the last hour on every entry (dashboard.md 4.1).
    selected: '1h',
    // Counts successful responses, so the body rebuilds only when there is new data.
    dataVersion: 0,
    refocusRefresh: false,
    // Block and unblock (dashboard.md SCR-008 주소 차단과 해제). `dialog`: the open confirm dialog as {action, address,
    // entries}. `busy`: the config request runs. `result`: the notice above the cards. `focus`: the row button to focus
    // once the new rows are in. `requerying`: a sources query for the new state is wanted or running. `pendingRequery`:
    // it waits for the sources request that was running.
    deny: { dialog: null, busy: false, result: null, focus: null, requerying: false, pendingRequery: false },
  };
  let ui = null;

  function t() {
    return COPY[state.lang];
  }

  function renderIfLive() {
    if (!state.ended) update();
  }

  function applyDocumentLanguage() {
    document.title = `${BRAND_NAME} ${t().sources}`;
    document.documentElement.lang = state.lang;
  }

  // The language changes the screen texts and country names at once. No request is made and the values stay.
  function onLanguageChange(next) {
    if (next === state.lang) return;
    state.lang = next;
    saveLanguage(storage, next);
    applyDocumentLanguage();
    build();
    update();
  }

  function clearProtected() {
    state.data = null;
    state.csrfToken = null;
    root.replaceChildren();
  }

  function dispose() {
    state.ended = true;
    lifecycle.abort();
    clearTimeout(state.pollTimer);
    clearBusyTimer(state);
    clearProtected();
  }

  function leave(url) {
    if (state.ended) return;
    dispose();
    navigate(url);
  }

  async function onSignOut() {
    if (state.ended) return;
    const csrfToken = state.csrfToken;
    dispose();
    navigate(await signOutUrl(rawRequest, csrfToken));
  }

  function schedulePoll() {
    clearTimeout(state.pollTimer);
    if (state.ended || state.stopped || state.pollMs === null) return;
    state.pollTimer = setTimeout(() => void load(false), state.pollMs);
  }

  // One sources request for the chosen period. Entry, refresh, and a period change (`manual`) send no X-Relay-Poll;
  // an automatic poll sends X-Relay-Poll: 1. Every success replaces all values, since the response is not
  // incremental. Requests never overlap. A request that ends after the period changed is dropped and the chosen
  // period is asked for at once (dashboard.md 4.1).
  async function load(manual) {
    if (state.ended || state.inFlight) return;
    clearTimeout(state.pollTimer);
    const forDeny = manual && state.deny.requerying && !state.deny.pendingRequery;
    const epoch = ++state.epoch;
    const period = state.selected;
    beginQuery(state, manual, renderIfLive);
    update();
    try {
      if (state.timeoutMs === null) {
        const session = parseSession(await getSession(request, { timeoutMs: BOOTSTRAP_TIMEOUT_MS }));
        if (epoch !== state.epoch || state.ended) return;
        state.csrfToken = session.csrfToken;
        state.timeoutMs = session.queryTimeoutMs;
        state.pollMs = session.pollMs;
      }
      const path = `${SOURCES_PATH}?window=${period}`;
      const parsed = parseSources(await getQuery(request, path, { poll: !manual, timeoutMs: state.timeoutMs }), period);
      if (epoch !== state.epoch || state.ended) return;
      if (period !== state.selected) {
        endQuery(state);
        void load(true);
        return;
      }
      state.data = parsed;
      state.dataVersion += 1;
      state.lastSuccessAt = new Date();
      state.failed = false;
      state.rejected = false;
      state.stopped = false;
      endQuery(state);
      update();
      if (forDeny) settleDeny();
      schedulePoll();
    } catch (error) {
      if (epoch !== state.epoch || state.ended) return;
      endQuery(state);
      if (isUnauthorized(error)) {
        leave(LOGIN_URL);
        return;
      }
      if (period !== state.selected) {
        void load(true);
        return;
      }
      state.failed = true;
      state.rejected = error instanceof ApiError && error.kind === 'failed' && (error.status === 400 || error.status === 405);
      if (state.rejected) state.stopped = true;
      if (forDeny) {
        state.deny.requerying = false;
        state.deny.focus = null;
      }
      update();
      schedulePoll();
    } finally {
      // A block or unblock ended while this request ran: its sources query goes out now, never before (dashboard.md SCR-008).
      if (!state.ended && state.deny.pendingRequery && !state.inFlight) {
        state.deny.pendingRequery = false;
        void load(true);
      }
    }
  }

  // The refresh button. While an automatic poll runs it only shows that poll as busy (no second request).
  function onRefresh() {
    if (state.ended) return;
    state.refocusRefresh = true;
    if (state.inFlight) {
      if (showBusy(state)) update();
      return;
    }
    void load(true);
  }

  // The period select (DASH-087). The chosen period is asked for at once, or right after the running request, which
  // load() then drops. CPY-017 shows at once and the focus stays on the select.
  function onPeriodChange(next) {
    if (state.ended || next === state.selected) return;
    state.selected = next;
    state.refocusRefresh = false;
    if (state.inFlight) {
      if (showBusy(state)) update();
      return;
    }
    void load(true);
  }

  // ---- Rendering ----

  function stateBadge(view) {
    const copy = t();
    if (view === 'loading') return statusBadge(copy, 'loading');
    if (view === 'refreshing') return statusBadge(copy, 'refreshing');
    if (view === 'failed' || (view === 'stale' && state.rejected)) {
      return createBadge({ tone: 'destructive', label: copy.failed });
    }
    return statusBadge(copy, view === 'stale' ? 'stale' : 'ok');
  }

  function refreshButton(id, view) {
    const copy = t();
    const busy = view === 'refreshing' || view === 'loading';
    const button = createButton({
      variant: 'outline',
      label: view === 'refreshing' ? copy.refreshing : copy.refresh,
      icon: 'refresh',
      disabled: busy,
    });
    button.id = id;
    button.addEventListener('click', onRefresh);
    return button;
  }

  // Builds the fixed part of the page: navigation bar, head, and the slots that update() fills.
  function build() {
    const copy = t();
    const focusId = root.contains(document.activeElement) ? document.activeElement.id : '';
    const navbar = buildNavbar({
      t: copy,
      lang: state.lang,
      active: 'sources',
      idPrefix: 'sources',
      onLanguageChange,
      onSignOut,
    });
    const main = el('main', 'app-main sources-main');

    const title = el('h1', 'page-title', copy.sources);
    const status = el('span');
    status.id = 'sources-status';
    status.setAttribute('role', 'status');
    const time = el('span', 'num muted');
    time.id = 'sources-time';
    const meta = el('p', 'log-meta');
    meta.append(status, time);
    const range = el('p', 'num muted');
    range.id = 'sources-range';
    const text = el('div', 'page-head-text');
    text.append(title, meta, range);
    // The period select is built here once. A status change swaps only the refresh button, so the select keeps focus.
    const select = el('select', 'text-field-input');
    select.id = 'sources-period';
    for (const [value, label] of [['1h', copy.periodHour], ['24h', copy.periodDay]]) {
      const option = el('option', undefined, label);
      option.value = value;
      option.selected = value === state.selected;
      select.append(option);
    }
    select.addEventListener('change', () => onPeriodChange(select.value));
    const selectWrap = el('div', 'select');
    selectWrap.append(select, icon('chevron-down', { size: 'sm' }));
    const periodLabel = el('label', 'text-field-label', copy.sourcesPeriodLabel);
    periodLabel.setAttribute('for', select.id);
    const period = el('div', 'field period-field');
    period.append(periodLabel, selectWrap);
    const actions = el('div', 'page-actions');
    actions.append(period);
    const head = el('header', 'page-head');
    head.append(text, actions);

    const result = el('div', 'sources-result');
    result.tabIndex = -1;
    const notice = el('div', 'sources-notice');
    const alerts = el('div', 'sources-alerts');
    const body = el('div', 'sources-body');
    main.append(head, result, notice, alerts, body);
    root.replaceChildren(navbar.navbar, main);

    ui = {
      main,
      status,
      time,
      range,
      actions,
      refresh: null,
      result,
      notice,
      alerts,
      body,
      dialog: null,
      dialogLive: null,
      statusKey: null,
      actionKey: null,
      resultKey: null,
      noticeKey: null,
      alertKey: null,
      bodyKey: null,
    };
    if (focusId) document.getElementById(focusId)?.focus();
    renderDialog();
  }

  function updateHead(view) {
    const copy = t();
    const statusKey = `${view}|${state.rejected}`;
    if (ui.statusKey !== statusKey) {
      ui.statusKey = statusKey;
      const badge = stateBadge(view);
      ui.status.replaceChildren(...(badge ? [badge] : []));
    }
    ui.time.textContent = fill(copy.lastSuccess, {
      time: state.lastSuccessAt ? formatTime(state.lastSuccessAt) : copy.unavailable,
    });
    ui.range.textContent = state.data
      ? fill(copy.sourcesRange, {
          start: formatTime(state.data.windowStart),
          end: formatTime(state.data.asOf),
          stored: formatCount(state.data.stored),
          capacity: formatCount(state.data.capacity),
        })
      : '';
    ui.range.hidden = !state.data;
    const actionKey = view;
    if (ui.actionKey !== actionKey) {
      // A focused button that turns busy (disabled) loses focus. Ask for focus again once it is enabled.
      if (ui.refresh !== null && document.activeElement === ui.refresh) state.refocusRefresh = true;
      ui.actionKey = actionKey;
      const button = refreshButton('sources-refresh', view);
      if (ui.refresh === null) ui.actions.append(button);
      else ui.refresh.replaceWith(button);
      ui.refresh = button;
      if (state.refocusRefresh && view !== 'refreshing' && view !== 'loading') {
        state.refocusRefresh = false;
        button.focus();
      }
    }
  }

  // CPY-258: a warning that the start of the period is missing. It is not an error and not a live region, since it
  // follows the values that automatic polls replace (dashboard.md §6).
  function updateNotice() {
    const shown = state.data !== null && storageLimited(state.data);
    const key = [state.lang, shown ? state.data.droppedBefore.getTime() : 'none'].join('|');
    if (ui.noticeKey === key) return;
    ui.noticeKey = key;
    if (!shown) {
      ui.notice.replaceChildren();
      return;
    }
    const notice = createAlert({
      tone: 'warning',
      title: fill(t().sourcesDropped, { time: formatTime(state.data.droppedBefore) }),
    });
    notice.removeAttribute('role');
    ui.notice.replaceChildren(notice);
  }

  function updateAlerts(view) {
    const copy = t();
    const busy = view === 'refreshing' || view === 'loading';
    const key = [state.lang, state.failed, state.rejected, state.lastSuccessAt?.getTime(), busy].join('|');
    if (ui.alertKey === key) return;
    ui.alertKey = key;
    if (!state.failed) {
      ui.alerts.replaceChildren();
      return;
    }
    let message = copy.failedMessage;
    if (state.rejected) message = copy.sourcesRejected;
    else if (state.data) message = fill(copy.staleMessage, { time: formatTime(state.lastSuccessAt) });
    const action = refreshButton('sources-alert-refresh', busy ? 'refreshing' : 'ok');
    const hadFocus = ui.alerts.contains(document.activeElement);
    ui.alerts.replaceChildren(createAlert({ tone: 'destructive', title: message, action }));
    if (hadFocus) action.focus();
  }

  function updateBody(view) {
    const key = [state.lang, state.data ? `data:${state.dataVersion}` : view].join('|');
    if (ui.bodyKey === key) return;
    ui.bodyKey = key;
    if (!state.data && view !== 'loading') {
      ui.body.replaceChildren();
      return;
    }
    const copy = t();
    // Every new response rebuilds the rows. A block or unblock button that has the focus keeps it.
    const keep = activeDenyKey();
    ui.body.replaceChildren(
      summarySection(copy),
      databaseCard(copy),
      addressCard(copy),
      countryCard(copy),
      asnCard(copy),
    );
    if (keep !== null) findDenyButton(keep)?.focus({ preventScroll: true });
  }

  function update() {
    if (state.ended || ui === null) return;
    const view = viewOf(state);
    updateHead(view);
    updateResult();
    updateNotice();
    updateAlerts(view);
    updateBody(view);
  }

  // ---- Block and unblock (dashboard.md SCR-008 주소 차단과 해제, 5.5) ----

  function activeDenyKey() {
    const active = document.activeElement;
    if (!(active instanceof HTMLButtonElement) || !active.dataset.denyAction || !ui.body.contains(active)) return null;
    return { action: active.dataset.denyAction, address: active.dataset.denyAddress };
  }

  function findDenyButton(key) {
    const buttons = [...ui.body.querySelectorAll('[data-deny-action]')];
    return buttons.find((button) => button.dataset.denyAction === key.action && button.dataset.denyAddress === key.address) ?? null;
  }

  // Focus goes back to the button that opened the dialog. When the new rows have no such button, it goes to the result.
  function focusDenyCaller(key) {
    const button = key ? findDenyButton(key) : null;
    if (button !== null && !button.disabled) button.focus({ preventScroll: true });
    else if (state.deny.result !== null) ui.result.focus({ preventScroll: true });
  }

  // The sources query for the new state has ended and the rows are in.
  function settleDeny() {
    const key = state.deny.focus;
    state.deny.focus = null;
    state.deny.requerying = false;
    focusDenyCaller(key);
  }

  // Every row button and the dialog follow the running request (DESIGN.md 요청 중 상태).
  function syncDeny() {
    const busy = state.deny.busy;
    for (const button of ui.body.querySelectorAll('[data-deny-action]')) button.disabled = busy;
    const target = state.deny.dialog;
    if (ui.dialog === null || target === null) return;
    const label = target.action === 'block' ? t().denyBlocking : t().denyUnblocking;
    ui.dialog.setBusy(busy ? label : null);
    ui.dialogLive.textContent = busy ? label : '';
  }

  function removeDialog() {
    if (ui?.dialog) {
      ui.dialog.hide();
      ui.dialog.element.remove();
    }
    if (ui) {
      ui.dialog = null;
      ui.dialogLive = null;
    }
  }

  // The confirm dialog of the open block or unblock. Built again with the screen language on a language change.
  function renderDialog() {
    removeDialog();
    const target = state.deny.dialog;
    if (target === null) return;
    const copy = t();
    const block = target.action === 'block';
    const dialog = createDialog({
      title: block ? copy.denyBlockTitle : copy.denyUnblockTitle,
      description: block ? copy.denyBlockBody : copy.denyUnblockBody,
      confirmLabel: block ? copy.denyBlock : copy.denyUnblock,
      cancelLabel: copy.cancel,
      destructive: block,
      closeOnConfirm: false,
      returnFocus: () => findDenyButton(target) ?? ui.refresh,
      onConfirm: () => void confirmDeny(),
      onCancel: () => {
        state.deny.dialog = null;
      },
    });
    const description = dialog.element.querySelector('.dialog-description');
    const live = el('p', 'sr-only');
    live.setAttribute('role', 'status');
    dialog.element.insertBefore(el('p', 'label muted', block ? copy.denyBlockLabel : copy.denyUnblockLabel), description);
    dialog.element.insertBefore(el('p', 'mono', target.address), description);
    dialog.element.insertBefore(live, dialog.element.querySelector('.dialog-actions'));
    ui.main.append(dialog.element);
    ui.dialog = dialog;
    ui.dialogLive = live;
    dialog.show();
    if (state.deny.busy) syncDeny();
    else dialog.cancel.focus();
  }

  // [차단] and [차단 해제] open the dialog and send nothing.
  function openDeny(action, item) {
    if (state.ended || state.deny.busy || state.deny.dialog !== null) return;
    state.deny.dialog = { action, address: item.ip, entries: item.denyEntries };
    renderDialog();
  }

  async function confirmDeny() {
    const target = state.deny.dialog;
    if (target === null || state.deny.busy || state.ended) return;
    state.deny.busy = true;
    state.deny.result = null;
    syncDeny();
    updateResult();
    const outcome = await changeDenyList(request, {
      csrfToken: state.csrfToken,
      timeoutMs: state.timeoutMs,
      action: target.action,
      address: target.address,
      entries: target.entries,
    });
    if (state.ended) return;
    state.deny.busy = false;
    if (outcome.kind === 'unauthorized') {
      leave(LOGIN_URL);
      return;
    }
    finishDeny(target, outcome);
  }

  // Any result closes the dialog and shows its notice above the cards. A result that changed or may have changed the list
  // asks for one sources query without X-Relay-Poll, after the running sources request.
  function finishDeny(target, outcome) {
    const block = target.action === 'block';
    let requery = true;
    if (outcome.kind === 'done') {
      state.deny.result = { key: block ? 'denyBlockDone' : 'denyUnblockDone', tone: 'success', address: target.address };
    } else if (outcome.kind === 'nothing') {
      state.deny.result = { key: 'denyNothingToRemove', tone: 'warning' };
    } else {
      state.deny.result = { key: outcome.key, tone: outcome.tone };
      if (outcome.key === 'denyInvalid') {
        state.deny.result.diagnostics = outcome.diagnostics;
        state.deny.result.fallback = outcome.fallback;
      }
      requery = outcome.requery === true;
    }
    state.deny.dialog = null;
    removeDialog();
    syncDeny();
    updateResult();
    ui.result.scrollIntoView?.({ block: 'nearest' });
    const key = { action: target.action, address: target.address };
    focusDenyCaller(key);
    if (!requery) return;
    state.deny.focus = key;
    state.deny.requerying = true;
    if (state.inFlight) state.deny.pendingRequery = true;
    else void load(true);
  }

  function diagnosticSentence(copy, result) {
    const texts = (result.diagnostics ?? [])
      .filter((diagnostic) => diagnostic && typeof diagnostic.code === 'string')
      .map((diagnostic) => diagnosticText(diagnostic, copy));
    return texts.length > 0 ? [...new Set(texts)].join(' ') : `${result.fallback ?? ''} (config_invalid)`.trim();
  }

  function resultAlert(copy, result) {
    const title = result.key === 'denyInvalid'
      ? fill(copy.denyInvalid, { diagnostic: diagnosticSentence(copy, result) })
      : fill(copy[result.key], { address: result.address });
    const alert = createAlert({ tone: result.tone, title });
    if (result.address !== undefined) {
      const [before, after] = copy[result.key].split('{address}');
      alert.querySelector('.alert-title').replaceChildren(before, el('span', 'mono', result.address), after);
    }
    return alert;
  }

  // The result notice. Success is a polite status; the others interrupt (createAlert). It stays until the next block or
  // unblock starts or the screen is left.
  function updateResult() {
    const result = state.deny.result;
    const key = [state.lang, JSON.stringify(result)].join('|');
    if (ui.resultKey === key) return;
    ui.resultKey = key;
    ui.result.replaceChildren(...(result === null ? [] : [resultAlert(t(), result)]));
  }

  // ---- Sections ----

  function loadingBadge(copy) {
    return statusBadge(copy, 'loading');
  }

  function summarySection(copy) {
    const heading = el('h2', 'card-heading', copy.sourcesSummaryTitle);
    heading.id = 'sources-summary-title';
    const section = el('section', 'metrics-section');
    section.setAttribute('aria-labelledby', heading.id);
    const grid = el('div', 'kpi-grid');
    const values = state.data ? summaryValues(state.data) : null;
    SUMMARY_KEYS.forEach((key, index) => {
      const content = values ? el('span', 'metric', formatCount(values[index])) : loadingBadge(copy);
      const card = createCard({ title: copy.sourcesSummaryNames[index], headingLevel: 3, body: content });
      card.dataset.kpi = key;
      card.querySelector('.card-body').classList.add('kpi-body');
      grid.append(card);
    });
    section.append(heading, el('p', 'label muted', copy.sourcesSummaryNote), grid);
    return section;
  }

  // ---- IP information databases ----

  function databaseBadge(copy, db) {
    const known = DB_STATES[db.state];
    return known ? createBadge({ tone: known[0], label: copy[known[1]] }) : createBadge({ tone: 'neutral', label: db.state });
  }

  // Type, build time, and load time (CPY-268). Known states with no database in use show none. `loaded`, `stale`, and
  // an unknown state show the values the response has.
  function databaseFacts(copy, db) {
    if (db.state === 'not_configured' || db.state === 'unavailable') return null;
    const facts = [
      [copy.ipDbInfoLabels[0], db.databaseType, 'mono'],
      [copy.ipDbInfoLabels[1], db.buildTime === null ? null : formatTime(db.buildTime), 'num'],
      [copy.ipDbInfoLabels[2], db.loadedAt === null ? null : formatTime(db.loadedAt), 'num'],
    ].filter(([, value]) => value !== null);
    if (facts.length === 0) return null;
    const list = el('dl', 'summary');
    for (const [label, value, cls] of facts) {
      const pair = el('div');
      pair.append(el('dt', undefined, label), el('dd', cls, value));
      list.append(pair);
    }
    return list;
  }

  function reasonText(copy, error) {
    const key = DB_REASONS[error];
    return key ? copy[key] : error;
  }

  // CPY-269 with the key in the data font, or CPY-270·271 with the cause.
  function databaseNote(copy, db, configKey) {
    if (db.state === 'not_configured') {
      const [before, after] = copy.ipDbNotConfiguredNote.split('{key}');
      const note = el('p');
      note.append(before, el('span', 'mono', configKey), after);
      return note;
    }
    const template = db.state === 'stale' ? copy.ipDbStaleNote : db.state === 'unavailable' ? copy.ipDbUnavailableNote : null;
    if (template === null) return null;
    const textOf = db.error === null ? template.replace(CAUSE_SENTENCE, '') : fill(template, { reason: reasonText(copy, db.error) });
    return el('p', undefined, textOf);
  }

  function databaseRow(copy, row, index) {
    const wrap = el('div', 'src-db-row');
    wrap.dataset.database = row.key;
    const info = el('div', 'src-db-info');
    if (!state.data) {
      info.append(loadingBadge(copy));
    } else {
      const db = state.data.databases[row.key];
      const badge = el('div');
      badge.append(databaseBadge(copy, db));
      info.append(badge);
      const facts = databaseFacts(copy, db);
      if (facts) info.append(facts);
      const note = databaseNote(copy, db, row.configKey);
      if (note) info.append(note);
    }
    wrap.append(el('h3', 'src-db-name', copy.ipDbNames[index]), info);
    return wrap;
  }

  // One attribution line. The provider wording is English in both languages. A wording that holds its address
  // (CPY-278) links only that address; the others link the whole wording.
  function attribution(copy, provider) {
    const { text: wording, href } = PROVIDER_ATTRIBUTION[provider];
    const line = el('p');
    line.dataset.provider = provider;
    line.lang = 'en';
    const at = wording.indexOf(href);
    const link = createLink({ href, label: at >= 0 ? href : wording, newTabText: copy.opensInNewTab });
    link.querySelector('.sr-only').lang = state.lang;
    if (at >= 0) line.append(wording.slice(0, at), link, wording.slice(at + href.length));
    else line.append(link);
    return line;
  }

  function databaseCard(copy) {
    const body = el('div');
    const rows = el('div', 'src-db');
    DATABASE_ROWS.forEach((row, index) => rows.append(databaseRow(copy, row, index)));
    body.append(rows);
    const providers = state.data ? attributedProviders(state.data.databases) : [];
    if (providers.length > 0) {
      const attrib = el('div', 'src-attrib label');
      for (const provider of providers) attrib.append(attribution(copy, provider));
      body.append(attrib);
    }
    const card = createCard({ title: copy.ipDbTitle, headingLevel: 2, body });
    card.dataset.sourcesCard = 'databases';
    return card;
  }

  // ---- Tables ----

  // An ipinfo.io lookup link. The visible text is the value; the accessible name is CPY-291 with CPY-196.
  function lookupLink(copy, value, cls) {
    const link = el('a', `link ${cls}`);
    link.href = `${LOOKUP_URL}${value}`;
    link.target = '_blank';
    link.rel = 'noopener noreferrer';
    link.setAttribute('aria-label', `${fill(copy.lookupName, { value })} (${copy.opensInNewTab})`);
    link.append(el('span', undefined, value), icon('external-link', { size: 'sm' }));
    return link;
  }

  function unknownCell(copy) {
    return el('span', 'muted', copy.unknownValue);
  }

  function addressCell(copy, item) {
    if (!item.local) return lookupLink(copy, item.ip, 'mono');
    const cell = el('span', 'src-address');
    cell.append(el('span', 'mono', item.ip), createBadge({ tone: 'neutral', label: copy.localAddress }));
    return cell;
  }

  function countryCell(copy, country) {
    return country === null ? unknownCell(copy) : countryLabel(country, state.lang);
  }

  // `AS{n}` lookup link. The address table puts the AS name on the next line.
  function asnCell(copy, asn, asName) {
    if (asn === null) return unknownCell(copy);
    const link = lookupLink(copy, `AS${asn}`, 'num');
    if (asName === null) return link;
    const cell = el('div');
    cell.append(link, el('span', 'label src-asname', asName));
    return cell;
  }

  // The count next to a table title: CPY-289, or CPY-084·085.
  function countText(copy, part) {
    const count = tableCount(part);
    if (count.partial) return fill(copy.partialCount, { total: formatCount(count.total), shown: formatCount(count.shown) });
    return count.total === 1 ? copy.listCountOne : fill(copy.listCount, { n: formatCount(count.total) });
  }

  // A table card. The caption repeats the title for assistive technology and is hidden on screen (DESIGN.md 표).
  function tableCard(copy, { key, title, labels, keys, numeric, part, row, notes = [], denyColumn = false }) {
    const table = createDataTable({
      caption: title,
      columns: labels.map((label, index) => ({ key: keys[index], label, ...(index === 0 ? { mobileHead: 'primary' } : {}) })),
      rows: state.data ? part.items.map(row) : [],
      emptyText: state.data ? copy.sourcesEmpty : loadingBadge(copy),
      numericColumns: numeric,
    });
    table.classList.add('src-table');
    table.querySelector('caption').classList.add('sr-only');
    if (denyColumn) {
      for (const cell of table.querySelectorAll('thead th:last-child, tbody tr:not(.data-table-empty) td:last-child')) {
        cell.classList.add('src-deny');
      }
    }
    const wrap = el('div', 'table-scroll');
    wrap.append(table);
    const card = createCard({
      title,
      headingLevel: 2,
      ...(state.data ? { count: countText(copy, part) } : {}),
      ...(notes.length > 0 ? { description: notes[0] } : {}),
      body: wrap,
    });
    const head = card.querySelector('.card-head');
    for (const note of notes.slice(1)) head.append(el('p', 'card-description', note));
    card.dataset.sourcesCard = key;
    return card;
  }

  const COUNT_KEYS = ['attempts', 'rejected', 'aborted', 'sessions'];

  function counts(item) {
    return COUNT_KEYS.map((key) => formatCount(item[key]));
  }

  // The block status cell (CPY-295~301): the badge, the button, and the range note, in this order. A cell with nothing
  // to show stays empty so that the mobile card shows no line for it.
  function denyStatusCell(copy, item) {
    const cell = denyCell(item, state.data.denyEditable);
    if (!cell.blocked && cell.action === null) return '';
    const wrap = el('div', 'src-deny-cell');
    if (cell.blocked) wrap.append(createBadge({ tone: 'destructive', label: copy.denyBlocked, icon: 'cancel' }));
    if (cell.action !== null) {
      const block = cell.action === 'block';
      const button = createButton({
        variant: 'outline',
        label: block ? copy.denyBlock : copy.denyUnblock,
        disabled: state.deny.busy,
      });
      button.setAttribute('aria-label', fill(block ? copy.denyBlockName : copy.denyUnblockName, { address: item.ip }));
      button.dataset.denyAction = cell.action;
      button.dataset.denyAddress = item.ip;
      button.addEventListener('click', () => openDeny(cell.action, item));
      wrap.append(button);
    }
    if (cell.rangeNote) wrap.append(el('p', 'label muted src-deny-note', copy.denyRangeNote));
    return wrap;
  }

  function addressCard(copy) {
    const editOff = state.data && !state.data.denyEditable ? [copy.denyEditOff] : [];
    return tableCard(copy, {
      key: 'addresses',
      title: copy.addressTitle,
      labels: copy.addressColumns,
      keys: ['ip', 'country', 'asn', ...COUNT_KEYS, 'last_seen_at', 'deny'],
      numeric: COUNT_KEYS,
      part: state.data?.addresses,
      notes: [copy.clientAddressNote, copy.localAddressNote, copy.lookupNote, ...editOff],
      denyColumn: true,
      row: (item) => [
        addressCell(copy, item),
        countryCell(copy, item.country),
        asnCell(copy, item.asn, item.asName),
        ...counts(item),
        timeStack(formatTime(item.lastSeenAt)),
        denyStatusCell(copy, item),
      ],
    });
  }

  function countryCard(copy) {
    return tableCard(copy, {
      key: 'countries',
      title: copy.countryTitle,
      labels: copy.countryColumns,
      keys: ['country', 'addresses', ...COUNT_KEYS],
      numeric: ['addresses', ...COUNT_KEYS],
      part: state.data?.countries,
      row: (item) => [countryCell(copy, item.country), formatCount(item.addresses), ...counts(item)],
    });
  }

  function asnCard(copy) {
    return tableCard(copy, {
      key: 'asns',
      title: copy.asnTitle,
      labels: copy.asnColumns,
      keys: ['asn', 'as_name', 'addresses', ...COUNT_KEYS],
      numeric: ['addresses', ...COUNT_KEYS],
      part: state.data?.asns,
      row: (item) => [asnCell(copy, item.asn, null), item.asName ?? '', formatCount(item.addresses), ...counts(item)],
    });
  }

  applyDocumentLanguage();
  build();
  update();
  void load(true);

  return { dispose };
}
