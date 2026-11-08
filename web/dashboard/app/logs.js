// SCR-006 Recent logs screen.
import {
  BRAND_NAME,
  COPY,
} from './copy.js';
import {
  chooseLanguage,
  readSavedLanguage,
  saveLanguage,
} from './language.js';
import {
  formatCount,
  formatTime,
  fill,
} from './format.js';
import {
  isSessionId,
  parseLogs,
  parseSession,
} from './model.js';
import {
  BOOTSTRAP_TIMEOUT_MS,
  getLogs,
  getSession,
  isUnauthorized,
} from './api.js';
import {
  createAlert,
} from './components/alert.js';
import {
  createBadge,
} from './components/badge.js';
import {
  createButton,
} from './components/button.js';
import {
  icon,
} from './components/icons.js';
import {
  buildNavbar,
  defaultStorage,
  signOutUrl,
  LOGIN_URL,
} from './shell.js';
import {
  el,
  statusBadge,
} from './panels.js';

const KNOWN_EVENTS = [
  'server_starting',
  'server_ready',
  'startup_failed',
  'shutdown_started',
  'shutdown_forced',
  'server_stopped',
  'plaintext_ws_enabled',
  'allow_all_source',
  'trust_all_proxies',
  'allow_all_target',
  'status_listen_not_loopback',
  'reload_change_detected',
  'reload_applied',
  'reload_rejected',
  'reload_restart_required',
  'reload_watch_failed',
  'log_reopen_failed',
  'connection_accepted',
  'connection_rejected',
  'connection_aborted',
  'waiting_registered',
  'waiting_expired',
  'waiting_removed',
  'session_started',
  'target_connect_failed',
  'session_closed',
  'admin_login_succeeded',
  'admin_login_failed',
  'admin_login_limited',
  'admin_terminate',
  'config_edited',
  'config_edit_rejected',
];

const SUMMARY_FIELDS = [
  'session_id',
  'conn_id',
  'listener',
  'role',
  'reason',
  'detail',
];

function formatUtcTimestamp(date) {
  const pad = (n) => String(n).padStart(2, '0');
  const y = date.getUTCFullYear();
  const m = pad(date.getUTCMonth() + 1);
  const d = pad(date.getUTCDate());
  const h = pad(date.getUTCHours());
  const min = pad(date.getUTCMinutes());
  const s = pad(date.getUTCSeconds());
  return `${y}${m}${d}T${h}${min}${s}Z`;
}

function countScalars(str) {
  return [...str].length;
}

export function mountLogs(root, deps = {}, { sessionId = null } = {}) {
  const rawRequest = deps.request ?? ((path, init) => globalThis.fetch(path, init));
  const storage = deps.storage === undefined ? defaultStorage() : deps.storage;
  const navigate = deps.navigate ?? ((url) => globalThis.location.assign(url));
  const preferred = deps.preferredLanguages ?? globalThis.navigator?.languages ?? [];
  const lifecycle = new AbortController();
  const request = (path, init) =>
    rawRequest(path, {
      ...init,
      signal: AbortSignal.any([lifecycle.signal, init?.signal].filter(Boolean)),
    });

  const state = {
    lang: chooseLanguage(readSavedLanguage(storage), preferred),
    epoch: 0,
    ended: false,
    pollIntervalMs: null,
    timeoutMs: null,
    csrfToken: null,
    sessionId,
    validSession: sessionId === null ? true : isSessionId(sessionId),
    // Filters
    levels: { info: true, warn: true, error: true },
    levelError: false,
    event: '',
    q: '',
    qInput: '',
    qError: false,
    // Auto-refresh
    autoRefresh: true,
    pausedNotice: false,
    pollTimer: null,
    inFlight: false,
    // Data
    logs: [],
    expandedSeqs: new Set(),
    hasMoreOlder: false,
    maxEntries: null,
    oldestRetainedSeq: null,
    latestRetainedSeq: null,
    lastSuccessAt: null,
    lastSeenSeq: null,
    snapshotReady: false,
    gapNotice: false,
    loadingOlder: false,
    // Alerts / Errors
    initialLoading: true,
    queryFailed: false,
    lastError: null, // { status, code }
    // Download
    downloading: false,
    downloadFailed: false,
    downloadController: null,
    queryController: null,
  };

  function t() {
    return COPY[state.lang];
  }

  function applyDocumentLanguage() {
    document.title = `${BRAND_NAME} ${t().logs}`;
    document.documentElement.lang = state.lang;
  }

  function onLanguageChange(next) {
    if (next === state.lang) return;
    state.lang = next;
    saveLanguage(storage, next);
    applyDocumentLanguage();
    render();
  }

  function clearProtected() {
    state.logs = [];
    state.expandedSeqs.clear();
    state.lastSeenSeq = null;
    state.snapshotReady = false;
    state.csrfToken = null;
    if (state.downloadBuffer) state.downloadBuffer.length = 0;
    state.downloadBuffer = [];
    state.downloadController?.abort();
    state.queryController?.abort();
    root.replaceChildren();
  }

  async function onSignOut() {
    if (state.ended) return;
    const csrfToken = state.csrfToken;
    dispose();
    navigate(await signOutUrl(rawRequest, csrfToken));
  }

  function leave(url) {
    if (state.ended) return;
    dispose();
    navigate(url);
  }

  function schedulePoll() {
    clearTimeout(state.pollTimer);
    if (!state.autoRefresh || state.ended || !state.validSession || state.levelError ||
        state.maxEntries === 0 || state.pollIntervalMs === null ||
        state.lastError?.status === 400 || state.lastError?.status === 405) return;
    state.pollTimer = setTimeout(() => {
      void pollNewLogs();
    }, state.pollIntervalMs);
  }

  function currentFilterParams() {
    const params = {};
    const selectedLevels = Object.entries(state.levels)
      .filter(([, v]) => v)
      .map(([k]) => k);
    if (selectedLevels.length > 0 && selectedLevels.length < 3) {
      params.level = selectedLevels.join(',');
    }
    if (state.event) {
      params.event = state.event;
    }
    if (state.q) {
      params.q = state.q;
    }
    if (state.sessionId && state.validSession) {
      params.session_id = state.sessionId;
    }
    return params;
  }

  async function fetchInitialLogs({ poll = false } = {}) {
    if (!state.validSession || state.levelError || state.ended || state.downloading) return;
    clearTimeout(state.pollTimer);
    state.queryController?.abort();
    state.queryController = new AbortController();
    const epoch = ++state.epoch;
    state.inFlight = true;
    state.loadingOlder = false;
    state.hasMoreOlder = false;
    state.snapshotReady = false;
    state.gapNotice = false;
    state.queryFailed = false;
    state.lastError = null;
    render();

    try {
      if (state.timeoutMs === null) {
        const session = parseSession(await getSession(request, { timeoutMs: BOOTSTRAP_TIMEOUT_MS }));
        if (epoch !== state.epoch || state.ended) return;
        state.csrfToken = session.csrfToken;
        state.timeoutMs = session.queryTimeoutMs;
        state.pollIntervalMs = session.pollMs;
      }

      const params = currentFilterParams();
      const res = parseLogs(await getLogs(request, params, {
        poll, timeoutMs: state.timeoutMs, signal: state.queryController.signal,
      }));
      if (epoch !== state.epoch || state.ended) return;

      state.initialLoading = false;
      state.inFlight = false;
      state.queryFailed = false;
      state.lastError = null;
      state.maxEntries = res.maxEntries;
      state.oldestRetainedSeq = res.oldestSeq;
      state.latestRetainedSeq = res.latestSeq;
      state.lastSuccessAt = new Date();
      state.hasMoreOlder = res.hasMore;

      // Items arrive in ascending seq; display in descending (newest-first)
      const sorted = [...res.items].sort((a, b) => b.seq - a.seq);
      state.logs = sorted;
      state.snapshotReady = true;

      if (res.latestSeq !== null) {
        state.lastSeenSeq = res.latestSeq;
      } else if (sorted.length > 0) {
        state.lastSeenSeq = sorted[0].seq;
      } else {
        state.lastSeenSeq = 0;
      }

      render();
      schedulePoll();
    } catch (err) {
      if (epoch !== state.epoch || state.ended) return;
      state.initialLoading = false;
      state.inFlight = false;
      if (isUnauthorized(err)) {
        leave(LOGIN_URL);
        return;
      }
      state.queryFailed = true;
      state.lastError = { status: err.status, code: err.code };
      render();
      schedulePoll();
    }
  }

  async function pollNewLogs() {
    if (
      !state.autoRefresh ||
      state.inFlight ||
      state.downloading ||
      !state.validSession ||
      state.levelError ||
      state.loadingOlder ||
      state.maxEntries === 0 ||
      state.ended
    ) {
      return;
    }
    if (!state.snapshotReady) {
      await fetchInitialLogs({ poll: true });
      return;
    }

    state.inFlight = true;
    state.queryController = new AbortController();
    const epoch = state.epoch;
    render();

    try {
      let afterSeq = state.lastSeenSeq ?? 0;
      let hasMore = true;
      const allNewItems = [];
      let lastPage = null;
      let gap = state.gapNotice;

      while (hasMore && !state.ended && epoch === state.epoch) {
        const params = {
          ...currentFilterParams(),
          after: afterSeq,
        };
        const res = parseLogs(
          await getLogs(request, params, {
            poll: true, timeoutMs: state.timeoutMs, signal: state.queryController.signal,
          }),
        );
        if (epoch !== state.epoch || state.ended) return;
        if ((res.latestSeq ?? 0) < afterSeq) {
          state.logs = [];
          state.expandedSeqs.clear();
          state.lastSeenSeq = null;
          state.inFlight = false;
          await fetchInitialLogs({ poll: true });
          return;
        }

        lastPage = res;
        // Compare each page with the cursor confirmed before that page.
        if (res.oldestSeq !== null && res.oldestSeq > afterSeq + 1) {
          gap = true;
        }

        if (res.items.length > 0) {
          allNewItems.push(...res.items);
          afterSeq = res.items[res.items.length - 1].seq;
        }


        hasMore = res.hasMore;
        if (!state.autoRefresh) break;
      }

      if (lastPage) {
        state.maxEntries = lastPage.maxEntries;
        state.oldestRetainedSeq = lastPage.oldestSeq;
        state.latestRetainedSeq = lastPage.latestSeq;
        state.lastSuccessAt = new Date();
        state.gapNotice = gap;
        state.lastSeenSeq = hasMore ? afterSeq : lastPage.latestSeq ?? 0;
      }
      if (allNewItems.length > 0) {
        // Deduplicate and prepend newest first
        const existingMap = new Map(state.logs.map((item) => [item.seq, item]));
        for (const item of allNewItems) {
          existingMap.set(item.seq, item);
        }
        state.logs = Array.from(existingMap.values()).sort((a, b) => b.seq - a.seq);
      }

      state.inFlight = false;
      state.queryFailed = false;
      state.lastError = null;
      render();
    } catch (err) {
      if (epoch !== state.epoch || state.ended) return;
      state.inFlight = false;
      if (isUnauthorized(err)) {
        leave(LOGIN_URL);
        return;
      }
      state.queryFailed = true;
      state.lastError = { status: err.status, code: err.code };
      render();
    } finally {
      schedulePoll();
    }
  }

  async function loadOlder() {
    if (state.loadingOlder || state.inFlight || state.downloading || state.levelError ||
        state.ended || !state.hasMoreOlder) return;
    const oldestLoadedSeq =
      state.logs.length > 0 ? state.logs[state.logs.length - 1].seq : null;
    if (oldestLoadedSeq === null) return;

    state.loadingOlder = true;
    state.inFlight = true;
    clearTimeout(state.pollTimer);
    state.queryController = new AbortController();
    const epoch = state.epoch;
    render();

    try {
      const params = {
        ...currentFilterParams(),
        before: oldestLoadedSeq,
      };
      const res = parseLogs(await getLogs(request, params, {
        timeoutMs: state.timeoutMs, signal: state.queryController.signal,
      }));
      if (state.ended || epoch !== state.epoch) return;

      state.maxEntries = res.maxEntries;
      state.oldestRetainedSeq = res.oldestSeq;
      state.latestRetainedSeq = res.latestSeq;
      state.lastSuccessAt = new Date();
      state.hasMoreOlder = res.hasMore;

      if (res.items.length > 0) {
        const existingMap = new Map(state.logs.map((item) => [item.seq, item]));
        for (const item of res.items) {
          existingMap.set(item.seq, item);
        }
        state.logs = Array.from(existingMap.values()).sort((a, b) => b.seq - a.seq);
      }

      state.loadingOlder = false;
      state.inFlight = false;
      state.queryFailed = false;
      state.lastError = null;
      render();
    } catch (err) {
      if (state.ended || epoch !== state.epoch) return;
      state.loadingOlder = false;
      state.inFlight = false;
      if (isUnauthorized(err)) {
        leave(LOGIN_URL);
        return;
      }
      state.queryFailed = true;
      state.lastError = { status: err.status, code: err.code };
      render();
    } finally {
      schedulePoll();
    }
  }

  async function downloadJsonLines() {
    if (state.downloading || state.inFlight || state.levelError || !state.validSession ||
        state.maxEntries === null || state.maxEntries === 0 || state.ended) return;
    state.downloading = true;
    state.downloadFailed = false;
    clearTimeout(state.pollTimer);
    render();

    const controller = new AbortController();
    state.downloadController = controller;
    const filters = currentFilterParams();

    try {
      const collectedEntries = [];
      state.downloadBuffer = collectedEntries;
      let beforeSeq = null;
      let hasMore = true;
      const startTime = new Date();

      while (hasMore && !state.ended) {
        const params = {
          ...filters,
          limit: 1000,
        };
        if (beforeSeq !== null) {
          params.before = beforeSeq;
        }

        const res = parseLogs(
          await getLogs(request, params, {
            timeoutMs: state.timeoutMs,
            signal: controller.signal,
          }),
        );
        if (state.ended || controller.signal.aborted) return;
        if (beforeSeq !== null && res.oldestSeq !== null && res.oldestSeq >= beforeSeq) {
          state.gapNotice = true;
        }

        if (res.items.length > 0) {
          // Collect entries
          for (const item of res.items) {
            collectedEntries.push(item);
          }
          // The items are in ascending seq order
          beforeSeq = res.items[0].seq;
        }
        hasMore = res.hasMore;
      }

      // Sort entries ascending by seq for download
      collectedEntries.sort((a, b) => a.seq - b.seq);
      const lines = collectedEntries.map((item) => JSON.stringify(item.entry) + '\n').join('');

      const blob = new Blob([lines], { type: 'application/x-jsonlines;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `vnc-relay-logs-${formatUtcTimestamp(startTime)}.jsonl`;
      document.body.append(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);

      state.downloading = false;
      render();
      schedulePoll();
    } catch (err) {
      if (state.ended) return;
      if (isUnauthorized(err)) {
        leave(LOGIN_URL);
        return;
      }
      state.downloading = false;
      state.downloadFailed = true;
      render();
      schedulePoll();
    } finally {
      state.downloadBuffer = [];
      state.downloadController = null;
    }
  }

  function render() {
    const copyObj = t();
    const focused = root.contains(document.activeElement) ? document.activeElement : null;
    const focusId = focused?.id;
    const focusDetail = focused?.closest('.log-row')?.id;
    const focusHref = focused?.getAttribute('href');
    const selection = focused?.id === 'logs-q'
      ? [focused.selectionStart, focused.selectionEnd] : null;
    root.replaceChildren();

    const navbar = buildNavbar({
      t: copyObj,
      lang: state.lang,
      active: 'logs',
      idPrefix: 'logs',
      onLanguageChange,
      onSignOut,
    });

    const main = el('main', 'app-main logs-main');

    // Page head
    const pageHeading = el('h1', 'page-title', copyObj.logs);
    const meta = el('p', 'log-meta');

    if (state.validSession && state.lastSuccessAt === null) {
      const retainedBadge = state.initialLoading
        ? statusBadge(copyObj, 'loading')
        : createBadge({ tone: 'neutral', label: copyObj.unavailable });
      meta.append(
        retainedBadge,
        el('span', 'num muted', fill(copyObj.lastSuccess, { time: copyObj.unavailable })),
      );
    } else if (state.validSession) {
      let retainedCount = 0;
      if (state.oldestRetainedSeq !== null && state.latestRetainedSeq !== null) {
        retainedCount = state.latestRetainedSeq - state.oldestRetainedSeq + 1;
      }
      const maxCount = state.maxEntries ?? 0;
      const retainedText = fill(copyObj.retainedCount, {
        n: formatCount(retainedCount),
        max: formatCount(maxCount),
      });
      const timeStr = state.lastSuccessAt ? formatTime(state.lastSuccessAt) : copyObj.unavailable;
      meta.append(
        el('span', 'num', retainedText),
        el('span', 'num muted', fill(copyObj.lastSuccess, { time: timeStr })),
      );
    }

    const headText = el('div', 'page-head-text');
    headText.append(pageHeading, meta);

    // Acts: auto-refresh toggle & refresh
    const acts = el('div', 'page-actions');

    const autoRefreshOn = state.autoRefresh && state.validSession && state.maxEntries !== 0;
    const toggle = el('button', 'button button-outline btn-toggle');
    toggle.id = 'logs-auto-refresh';
    toggle.type = 'button';
    toggle.setAttribute('role', 'switch');
    toggle.setAttribute('aria-checked', String(autoRefreshOn));
    if (!state.validSession || state.maxEntries === 0) {
      toggle.disabled = true;
    }
    const toggleIcon = icon(autoRefreshOn ? 'refresh' : 'pause');
    const toggleLabel = el('span', 'toggle-label', copyObj.autoRefresh);
    const toggleState = el(
      'span',
      'state',
      autoRefreshOn ? copyObj.autoRefreshOn : copyObj.autoRefreshOff,
    );
    toggleState.setAttribute('aria-hidden', 'true');
    toggle.append(toggleIcon, toggleLabel, toggleState);

    toggle.addEventListener('click', () => {
      state.autoRefresh = !state.autoRefresh;
      state.pausedNotice = !state.autoRefresh;
      render();
      if (state.autoRefresh) {
        schedulePoll();
      } else {
        clearTimeout(state.pollTimer);
      }
    });

    const refreshBtn = createButton({
      variant: 'outline',
      label: state.inFlight ? (state.initialLoading ? copyObj.loading : copyObj.refreshing) : copyObj.refresh,
      icon: 'refresh',
      disabled: !state.validSession || state.levelError || state.inFlight || state.downloading,
    });
    refreshBtn.id = 'logs-refresh';
    refreshBtn.addEventListener('click', () => {
      void fetchInitialLogs();
    });

    acts.append(toggle, refreshBtn);

    const head = el('header', 'page-head');
    head.append(headText, acts);
    main.append(head);

    // Filters form
    const filterForm = el('form', 'card');
    const filtersDisabled = state.downloading || state.maxEntries === 0;
    filterForm.setAttribute('role', 'search');
    filterForm.setAttribute('aria-labelledby', 'logs-filter-heading');

    const filterHead = el('header', 'card-head');
    const filterTitle = el('h2', 'card-heading', copyObj.logFiltersTitle);
    filterTitle.id = 'logs-filter-heading';
    filterHead.append(filterTitle);

    const filterBody = el('div', 'card-body filter-body');

    // Filter row 1: Levels & Event
    const filterRow = el('div', 'filter-row');

    const checkGroup = el('fieldset', 'check-group');
    if (state.levelError) {
      checkGroup.classList.add('has-error');
    }
    const legend = el('legend', 'text-field-label', copyObj.levelLabel);
    checkGroup.append(legend);

    const checkList = el('div', 'check-list');

    const levelDefs = [
      { key: 'info', tone: 'neutral', icon: null },
      { key: 'warn', tone: 'warning', icon: 'warning' },
      { key: 'error', tone: 'destructive', icon: 'alert' },
    ];

    for (const def of levelDefs) {
      const checkLabel = el('label', 'check');
      const input = el('input');
      input.type = 'checkbox';
      input.id = `logs-level-${def.key}`;
      input.checked = state.levels[def.key];
      input.disabled = filtersDisabled;
      input.addEventListener('change', () => {
        state.levels[def.key] = input.checked;
        const count = Object.values(state.levels).filter(Boolean).length;
        if (count === 0) {
          state.levelError = true;
          ++state.epoch;
          state.queryController?.abort();
          state.inFlight = false;
          clearTimeout(state.pollTimer);
          render();
        } else {
          state.levelError = false;
          render();
          void fetchInitialLogs();
        }
      });
      const badge = createBadge({
        tone: def.tone,
        label: def.key,
        icon: def.icon,
      });
      checkLabel.append(input, badge);
      checkList.append(checkLabel);
    }
    checkGroup.append(checkList);
    if (state.levelError) {
      const err = el('p', 'err');
      err.append(icon('alert', { size: 'sm' }), document.createTextNode(copyObj.levelRequiredError));
      checkGroup.append(err);
    }

    const eventField = el('div', 'field event-field');
    const eventLabel = el('label', 'text-field-label', copyObj.eventNameLabel);
    eventLabel.setAttribute('for', 'logs-event');
    const selectWrap = el('div', 'select');
    const select = el('select', 'text-field-input');
    select.id = 'logs-event';
    select.disabled = filtersDisabled;

    const allOpt = el('option', '', copyObj.allEvents);
    allOpt.value = '';
    if (state.event === '') allOpt.selected = true;
    select.append(allOpt);

    for (const ev of KNOWN_EVENTS) {
      const opt = el('option', '', ev);
      opt.value = ev;
      if (state.event === ev) opt.selected = true;
      select.append(opt);
    }

    select.addEventListener('change', () => {
      state.event = select.value;
      render();
      void fetchInitialLogs();
    });

    selectWrap.append(select, icon('chevron-down', { size: 'sm' }));
    eventField.append(eventLabel, selectWrap);

    filterRow.append(checkGroup, eventField);

    // Search field
    const searchField = el('div', 'field search-field');
    const searchLabel = el('label', 'text-field-label', copyObj.searchLabel);
    searchLabel.setAttribute('for', 'logs-q');

    const searchRow = el('div', 'search-row');
    const searchInput = el('input', 'text-field-input');
    searchInput.type = 'text';
    searchInput.id = 'logs-q';
    searchInput.value = state.qInput;
    searchInput.autocomplete = 'off';
    searchInput.disabled = filtersDisabled;
    searchInput.addEventListener('input', () => { state.qInput = searchInput.value; });

    const searchHelpId = 'logs-q-help';
    const searchErrorId = 'logs-q-error';

    const searchHelp = el('p', 'text-field-help', copyObj.searchHelp);
    searchHelp.id = searchHelpId;

    let searchError = null;
    if (state.qError) {
      searchInput.classList.add('has-error');
      searchInput.setAttribute('aria-invalid', 'true');
      searchInput.setAttribute('aria-describedby', `${searchHelpId} ${searchErrorId}`);
      searchError = el('p', 'err');
      searchError.append(icon('alert', { size: 'sm' }), document.createTextNode(copyObj.searchLengthError));
      searchError.id = searchErrorId;
    } else {
      searchInput.setAttribute('aria-describedby', searchHelpId);
    }

    const searchBtn = createButton({
      variant: 'primary',
      label: copyObj.search,
      icon: 'search',
      type: 'submit',
      disabled: filtersDisabled,
    });
    searchBtn.id = 'logs-search';

    searchRow.append(searchInput, searchBtn);
    searchField.append(searchLabel, searchRow, searchHelp);
    if (searchError) {
      searchField.append(searchError);
    }

    filterForm.addEventListener('submit', (e) => {
      e.preventDefault();
      const val = searchInput.value;
      const len = countScalars(val);
      if (len > 256) {
        state.qInput = searchInput.value;
        state.qError = true;
        render();
        document.getElementById('logs-q')?.focus();
        return;
      }
      state.qError = false;
      state.qInput = val;
      state.q = val;
      render();
      void fetchInitialLogs();
    });

    filterBody.append(filterRow, searchField);

    // Session chip
    if (state.sessionId !== null) {
      const chipField = el('div', 'field session-field');
      const chip = el('div', 'chip');
      chip.setAttribute('role', 'group');
      const chipLabel = fill(copyObj.sessionFilterChip, { session_id: state.sessionId });
      chip.setAttribute('aria-label', chipLabel);

      const chipText = el('span', 'chip-text');
      chipText.append(
        document.createTextNode(copyObj.sessionFilterChip.split('{')[0]),
        el('span', 'mono', state.sessionId),
      );

      const clearBtn = createButton({
        variant: 'outline',
        label: copyObj.clearFilter,
        icon: 'close',
      });
      clearBtn.id = 'logs-session-clear';
      clearBtn.disabled = state.downloading;
      clearBtn.classList.add('sm');
      clearBtn.addEventListener('click', () => {
        state.sessionId = null;
        state.validSession = true;
        (deps.replaceUrl ?? ((url) => globalThis.history.replaceState(null, '', url)))('/#/logs');
        render();
        void fetchInitialLogs();
      });

      chip.append(chipText, clearBtn);
      chipField.append(chip);
      chipField.append(el('p', 'text-field-help', copyObj.sessionFilterNote));
      filterBody.append(chipField);
    }

    filterForm.append(filterHead, filterBody);
    main.append(filterForm);

    // Status alerts
    if (!state.validSession) {
      const alertNode = createAlert({
        tone: 'destructive',
        title: copyObj.invalidSessionId,
      });
      alertNode.classList.add('logs-session-error');
      main.append(alertNode);
      root.append(navbar.navbar, main);
      return { dispose };
    }

    if (state.gapNotice) {
      const gapAlert = createAlert({
        tone: 'warning',
        title: copyObj.logsGapNotice,
      });
      gapAlert.setAttribute('role', 'status');
      main.append(gapAlert);
    }

    if (state.pausedNotice && !state.autoRefresh) {
      const pauseAlert = createAlert({
        tone: 'neutral',
        title: copyObj.logsPausedNotice,
      });
      pauseAlert.setAttribute('role', 'status');
      main.append(pauseAlert);
    }


    if (state.queryFailed) {
      let message = copyObj.failedMessage;
      if (state.lastError?.status === 400) {
        message = copyObj.logsInvalidRequest;
      } else if (state.lastError?.status === 405) {
        message = copyObj.logsMethodNotAllowed;
      } else if (state.lastSuccessAt !== null) {
        const timeStr = state.lastSuccessAt ? formatTime(state.lastSuccessAt) : copyObj.unavailable;
        message = fill(copyObj.staleMessage, { time: timeStr });
      }
      const failAlert = createAlert({
        tone: 'destructive',
        title: copyObj.failed,
        message,
        action: createButton({
          variant: 'outline',
          label: copyObj.refresh,
          icon: 'refresh',
        }),
      });
      failAlert.querySelector('button')?.addEventListener('click', () => {
        void fetchInitialLogs();
      });
      main.append(failAlert);
    }

    // List card
    const listCard = el('section', 'card log-list');
    listCard.setAttribute('aria-labelledby', 'logs-list-heading');

    const listHead = el('header', 'card-head');
    const listTitleWrap = el('div', 'card-title');
    const listTitle = el('h2', 'card-heading', copyObj.recentLogsTitle);
    listTitle.id = 'logs-list-heading';
    listTitleWrap.append(listTitle);
    if (state.inFlight && state.lastSuccessAt !== null) listTitleWrap.append(statusBadge(copyObj, 'refreshing'));

    const listActs = el('div', 'page-actions');
    const dlBtn = createButton({
      variant: 'outline',
      label: copyObj.downloadJsonLines,
      icon: 'download',
      disabled: state.downloading || state.inFlight || state.levelError || state.maxEntries === null ||
        state.maxEntries === 0 || state.initialLoading,
    });
    dlBtn.id = 'logs-download';
    dlBtn.addEventListener('click', () => {
      void downloadJsonLines();
    });
    listActs.append(dlBtn);
    listHead.append(listTitleWrap, listActs);

    const listBody = el('div', 'card-body');
    if (state.downloading) {
      listBody.append(createAlert({ tone: 'info', title: copyObj.downloadCollecting }));
    }
    if (state.downloadFailed) {
      listBody.append(createAlert({ tone: 'destructive', title: copyObj.downloadFailed }));
    }

      const table = el('table', 'data-table log-table');
      table.setAttribute('role', 'table');
      const caption = el('caption', 'sr-only', copyObj.recentLogsTitle);
      table.append(caption);

      const thead = el('thead');
      thead.setAttribute('role', 'rowgroup');
      const trHead = el('tr');
      trHead.setAttribute('role', 'row');
      for (const colName of copyObj.logColumns) {
        const th = el('th', '', colName);
        th.scope = 'col';
        th.setAttribute('role', 'columnheader');
        trHead.append(th);
      }
      thead.append(trHead);
      table.append(thead);

      const tbody = el('tbody');
      tbody.setAttribute('role', 'rowgroup');

      if (state.logs.length === 0 || state.maxEntries === 0) {
        const trEmpty = el('tr', 'data-table-empty');
        const tdEmpty = el('td', 'empty');
        tdEmpty.colSpan = 5;
        if (state.maxEntries === 0) {
          const disabledMsg = el('p', 'logs-disabled-message', copyObj.logsDisabled);
          const settingsLink = el('a', 'button button-outline');
          settingsLink.href = '/#/settings';
          settingsLink.append(icon('settings', { size: 'sm' }), document.createTextNode(copyObj.settingsTitle));
          const disabledGuide = el('div', 'logs-disabled');
          disabledGuide.append(disabledMsg, settingsLink);
          tdEmpty.append(disabledGuide);
        } else if (state.initialLoading) tdEmpty.append(statusBadge(copyObj, 'loading'));
        else if (!state.queryFailed) tdEmpty.textContent = copyObj.logsEmpty;
        else tdEmpty.append(createBadge({ tone: 'neutral', label: copyObj.unavailable }));
        trEmpty.append(tdEmpty);
        tbody.append(trEmpty);
      } else {
        for (const item of state.logs) {
          const isOpen = state.expandedSeqs.has(item.seq);
          const entry = item.entry || {};
          const rowId = `log-r-${item.seq}`;
          const detailId = `log-d-${item.seq}`;

          const trRow = el('tr', `log-row ${isOpen ? 'is-open' : ''}`);
          trRow.id = rowId;

          // Time cell
          const tdTime = el('td', 'time');
          tdTime.setAttribute('data-label', copyObj.logColumns[0]);
          const timeText = formatTime(entry.ts);
          const [datePart, ...timeParts] = timeText.split(' ');
          const timePart = timeParts.join(' ');

          const vDiv = el('div', 'v');
          vDiv.append(
            el('span', 't wd', datePart),
            document.createTextNode(' '),
            el('span', 't wt', timePart),
          );
          tdTime.append(vDiv);

          // Level cell
          const tdLevel = el('td', 'head2 data-table-head-secondary');
          tdLevel.setAttribute('data-label', copyObj.logColumns[1]);
          const lvl = entry.level || 'info';
          let tone = 'neutral';
          let lvlIcon = null;
          if (lvl === 'warn') {
            tone = 'warning';
            lvlIcon = 'warning';
          } else if (lvl === 'error') {
            tone = 'destructive';
            lvlIcon = 'alert';
          }
          tdLevel.append(createBadge({ tone, label: lvl, icon: lvlIcon }));

          // Event cell
          const tdEvent = el('td', 'head data-table-head-primary');
          tdEvent.setAttribute('data-label', copyObj.logColumns[2]);
          tdEvent.append(el('span', 'mono ev', entry.event || ''));

          // Summary cell
          const tdSum = el('td', 'sum');
          tdSum.setAttribute('data-label', copyObj.logColumns[3]);
          const sumDl = el('dl', 'summary');
          let hasSummary = false;
          for (const key of SUMMARY_FIELDS) {
            if (entry[key] !== undefined && entry[key] !== null) {
              hasSummary = true;
              const div = el('div');
              div.append(el('dt', 'mono', key), el('dd', 'mono', String(entry[key])));
              sumDl.append(div);
            }
          }
          if (hasSummary) {
            tdSum.append(sumDl);
          }

          // Detail button cell
          const tdAct = el('td', 'act');
          tdAct.setAttribute('data-label', copyObj.logColumns[4]);
          const detailBtn = el('button', 'button button-outline');
          detailBtn.type = 'button';
          detailBtn.setAttribute('aria-expanded', String(isOpen));
          detailBtn.setAttribute('aria-controls', detailId);
          detailBtn.append(
            icon(isOpen ? 'chevron-up' : 'chevron-down'),
            document.createTextNode(isOpen ? copyObj.hideDetails : copyObj.showDetails),
          );
          detailBtn.addEventListener('click', () => {
            if (state.expandedSeqs.has(item.seq)) {
              state.expandedSeqs.delete(item.seq);
            } else {
              state.expandedSeqs.add(item.seq);
            }
            render();
            // Restore focus
            document.getElementById(rowId)?.querySelector('.act button')?.focus();
          });
          tdAct.append(detailBtn);

          trRow.append(tdTime, tdLevel, tdEvent, tdSum, tdAct);

          // Detail row
          const trDetail = el('tr', 'detail-row');
          trDetail.id = detailId;
          if (!isOpen) {
            trDetail.hidden = true;
          }
          const tdDetail = el('td');
          tdDetail.colSpan = 5;

          const detailDl = el('dl', 'log-detail');
          detailDl.setAttribute('aria-label', `${copyObj.eventNameLabel}: ${entry.event || ''}`);

          for (const [k, v] of Object.entries(entry)) {
            const div = el('div');
            const dt = el('dt', 'mono', k);
            const dd = el('dd', 'mono');
            if (v !== null && typeof v === 'object') {
              const pre = el('pre', 'json mono', JSON.stringify(v, null, 2));
              dd.append(pre);
            } else {
              dd.textContent = String(v);
            }
            div.append(dt, dd);
            detailDl.append(div);
          }
          tdDetail.append(detailDl);
          trDetail.append(tdDetail);

          tbody.append(trRow, trDetail);
        }
      }
      table.append(tbody);
      listBody.append(table);

    listCard.append(listHead, listBody);

    if (state.hasMoreOlder && state.maxEntries !== 0) {
      const listFoot = el('footer', 'card-foot center');
      const olderBtn = createButton({
        variant: 'outline',
        label: copyObj.loadOlderLogs,
        icon: 'chevron-down',
        disabled: state.inFlight || state.downloading || state.levelError,
      });
      olderBtn.id = 'logs-older';
      olderBtn.addEventListener('click', () => {
        void loadOlder();
      });
      listFoot.append(olderBtn);
      listCard.append(listFoot);
    }

    main.append(listCard);
    for (const row of listCard.querySelectorAll('tbody tr')) row.setAttribute('role', 'row');
    for (const cell of listCard.querySelectorAll('td')) cell.setAttribute('role', 'cell');
    root.append(navbar.navbar, main);
    if (focusId) {
      const next = document.getElementById(focusId);
      next?.focus();
      if (selection && next?.setSelectionRange) next.setSelectionRange(...selection);
    } else if (focusDetail) {
      document.getElementById(focusDetail)?.querySelector('.act button')?.focus();
    } else if (focusHref !== null && focusHref !== undefined) {
      [...root.querySelectorAll('a[href]')].find((link) => link.getAttribute('href') === focusHref)?.focus();
    }

    return { dispose };
  }

  function dispose() {
    state.ended = true;
    lifecycle.abort();
    if (state.downloadController) {
      state.downloadController.abort();
    }
    clearTimeout(state.pollTimer);
    clearProtected();
  }

  applyDocumentLanguage();
  render();
  void fetchInitialLogs();

  return { dispose };
}
