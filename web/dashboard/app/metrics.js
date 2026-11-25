// SCR-007 Metrics screen: key indicators, trend charts of the last hour or the last 24 hours, duration distribution,
// counts by cause, and process values.
// State, requests, and rendering live here. The checks and calculations live in metrics-model.js.
import { BRAND_NAME, COPY } from './copy.js';
import { chooseLanguage, readSavedLanguage, saveLanguage } from './language.js';
import {
  fill,
  formatBytesRate,
  formatBytesValue,
  formatClock,
  formatCount,
  formatDecimal,
  formatDuration,
  formatPercent,
  formatShortSeconds,
  formatTime,
} from './format.js';
import { parseSession } from './model.js';
import { ApiError, BOOTSTRAP_TIMEOUT_MS, getQuery, getSession, isUnauthorized } from './api.js';
import { createAlert } from './components/alert.js';
import { createBadge } from './components/badge.js';
import { createButton } from './components/button.js';
import { createCard } from './components/card.js';
import { createDataTable } from './components/data-table.js';
import { createGauge } from './components/gauge.js';
import { icon } from './components/icons.js';
import { buildNavbar, defaultStorage, signOutUrl, LOGIN_URL } from './shell.js';
import { beginQuery, clearBusyTimer, endQuery, showBusy, viewOf } from './query.js';
import { closeCauseBadge, el, statusBadge } from './panels.js';
import {
  CAUSE_TABLES,
  COUNT_VIEWBOX_WIDTH,
  DIST_VIEWBOX_HEIGHT,
  HISTORY_WINDOWS,
  PLOT_HEIGHT,
  PLOT_WIDTH,
  buildCharts,
  causeRows,
  countBarWidth,
  distributionItems,
  historyAfter,
  keyIndicators,
  mergeHistory,
  parseMetrics,
  plotX,
  plotY,
  processRows,
} from './metrics-model.js';

const METRICS_PATH = '/api/v1/metrics';
const SVG_NS = 'http://www.w3.org/2000/svg';

// Line shape of the series of one chart: solid, dashed (flat ends), dotted (round ends). The dash pattern is a
// presentation attribute so the stylesheet keeps only tokens (DESIGN.md 선 차트).
const SERIES_STYLES = [
  { cls: 's1', dash: null },
  { cls: 's2', dash: '6 4' },
  { cls: 's3', dash: '1 5' },
];

function svg(tag, attributes = {}) {
  const node = document.createElementNS(SVG_NS, tag);
  for (const [name, value] of Object.entries(attributes)) {
    node.setAttribute(name, String(value));
  }
  return node;
}

function round(value) {
  return Math.round(value * 100) / 100;
}

// CPY-244 as one sentence. The two settings words are set in the data font, as in the adopted mockup.
function noteNodes(text) {
  const tokens = ['/metrics', '[metrics] enabled = true'];
  const nodes = [];
  let rest = text;
  while (rest.length > 0) {
    const hits = tokens.map((token) => [rest.indexOf(token), token]).filter(([index]) => index >= 0);
    if (hits.length === 0) {
      nodes.push(rest);
      break;
    }
    const [index, token] = hits.reduce((best, hit) => (hit[0] < best[0] ? hit : best));
    if (index > 0) nodes.push(rest.slice(0, index));
    nodes.push(el('span', 'mono', token));
    rest = rest.slice(index + token.length);
  }
  return nodes;
}

export function mountMetrics(root, deps = {}) {
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
    // The last failure was a 400 or 405: automatic polling stays stopped until a refresh succeeds (dashboard.md 5.3).
    rejected: false,
    stopped: false,
    data: null,
    lastSuccessAt: null,
    // Retained history points of the shown window, at most its capacity (360 or 1440).
    points: [],
    // Trend windows (HISTORY_WINDOWS keys): the one the select names and the one the kept points belong to. Both start
    // at the last hour on every entry (dashboard.md SCR-007).
    selected: '1h',
    shown: '1h',
    // Counts successful responses, so the body rebuilds only when there is new data.
    dataVersion: 0,
    refocusRefresh: false,
  };
  let ui = null;

  function t() {
    return COPY[state.lang];
  }

  // Window the screen shows: the kept points' window, or the chosen one before the first success (dashboard.md SCR-007).
  function visiblePeriod() {
    return state.data ? state.shown : state.selected;
  }

  // Texts that follow the shown window: CPY-203·213·209·220 for the last hour, CPY-251·252·254·253 for the last 24 hours.
  function periodTexts(copy) {
    return visiblePeriod() === '24h'
      ? { range: copy.metricsRange24h, trends: copy.trendsTitle24h, cpuNote: copy.cpuNote24h, chartEmpty: copy.chartEmpty24h }
      : { range: copy.metricsRange, trends: copy.trendsTitle, cpuNote: copy.cpuNote, chartEmpty: copy.chartEmpty };
  }

  function renderIfLive() {
    if (!state.ended) update();
  }

  function applyDocumentLanguage() {
    document.title = `${BRAND_NAME} ${t().metrics}`;
    document.documentElement.lang = state.lang;
  }

  // The language changes the screen texts at once. No request is made and the retained data stays.
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
    state.points = [];
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

  // One metrics request for the chosen window. Entry, refresh, and a period change (`manual`) send no history_after and
  // no X-Relay-Poll, and replace the kept points. An automatic poll sends X-Relay-Poll: 1 and, while the kept points
  // belong to the chosen window, the largest kept seq; otherwise its points replace the kept ones too. Only the last
  // 24 hours adds history_window=24h. Requests never overlap. A request that ends after the period changed is dropped
  // and the chosen window is asked for at once (dashboard.md 4.1).
  async function load(manual) {
    if (state.ended || state.inFlight) return;
    clearTimeout(state.pollTimer);
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
      const after = manual || state.shown !== period ? null : historyAfter(state.points);
      const params = [period === '24h' ? 'history_window=24h' : null, after === null ? null : `history_after=${after}`];
      const query = params.filter((param) => param !== null).join('&');
      const path = query === '' ? METRICS_PATH : `${METRICS_PATH}?${query}`;
      const parsed = parseMetrics(await getQuery(request, path, { poll: !manual, timeoutMs: state.timeoutMs }), period);
      if (epoch !== state.epoch || state.ended) return;
      if (period !== state.selected) {
        endQuery(state);
        void load(true);
        return;
      }
      state.points = mergeHistory(state.points, parsed.points, after === null, HISTORY_WINDOWS[period].capacity);
      state.shown = period;
      state.data = parsed;
      state.dataVersion += 1;
      state.lastSuccessAt = new Date();
      state.failed = false;
      state.rejected = false;
      state.stopped = false;
      endQuery(state);
      update();
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
      update();
      schedulePoll();
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

  // The trend period select (DASH-080). The chosen window is asked for at once, or right after the running request,
  // which load() then drops. CPY-017 shows at once and the focus stays on the select.
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

  // Builds the fixed part of the page: navigation bar, head, notice, and the slots that update() fills.
  function build() {
    const copy = t();
    const focusId = root.contains(document.activeElement) ? document.activeElement.id : '';
    const navbar = buildNavbar({
      t: copy,
      lang: state.lang,
      active: 'metrics',
      idPrefix: 'metrics',
      onLanguageChange,
      onSignOut,
    });
    const main = el('main', 'app-main metrics-main');

    const title = el('h1', 'page-title', copy.metrics);
    const status = el('span');
    status.id = 'metrics-status';
    status.setAttribute('role', 'status');
    const time = el('span', 'num muted');
    time.id = 'metrics-time';
    const meta = el('p', 'log-meta');
    meta.append(status, time);
    const range = el('p', 'num muted');
    range.id = 'metrics-range';
    const text = el('div', 'page-head-text');
    text.append(title, meta, range);
    // The period select is built here once. A status change swaps only the refresh button, so the select keeps focus.
    const select = el('select', 'text-field-input');
    select.id = 'metrics-period';
    for (const [value, label] of [['1h', copy.periodHour], ['24h', copy.periodDay]]) {
      const option = el('option', undefined, label);
      option.value = value;
      option.selected = value === state.selected;
      select.append(option);
    }
    select.addEventListener('change', () => onPeriodChange(select.value));
    const selectWrap = el('div', 'select');
    selectWrap.append(select, icon('chevron-down', { size: 'sm' }));
    const periodLabel = el('label', 'text-field-label', copy.periodLabel);
    periodLabel.setAttribute('for', select.id);
    const period = el('div', 'field period-field');
    period.append(periodLabel, selectWrap);
    const actions = el('div', 'page-actions');
    actions.append(period);
    const head = el('header', 'page-head');
    head.append(text, actions);

    // External collection note: always the same text, never read from the server (dashboard.md SCR-007).
    // DESIGN.md "외부 수집 안내": neutral tone with the circled exclamation icon.
    const note = createAlert({ tone: 'neutral', icon: 'alert', title: copy.externalNote });
    note.removeAttribute('role');
    note.querySelector('.alert-title').replaceChildren(...noteNodes(copy.externalNote));

    const alerts = el('div', 'metrics-alerts');
    const body = el('div', 'metrics-body');
    main.append(head, note, alerts, body);
    root.replaceChildren(navbar.navbar, main);

    ui = {
      status,
      time,
      range,
      actions,
      refresh: null,
      alerts,
      body,
      statusKey: null,
      actionKey: null,
      alertKey: null,
      bodyKey: null,
    };
    if (focusId) document.getElementById(focusId)?.focus();
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
    ui.range.textContent = state.data ? fill(periodTexts(copy).range, { n: state.points.length }) : '';
    ui.range.hidden = !state.data;
    const actionKey = view;
    if (ui.actionKey !== actionKey) {
      // A focused button that turns busy (disabled) loses focus. Ask for focus again once it is enabled.
      if (ui.refresh !== null && document.activeElement === ui.refresh) state.refocusRefresh = true;
      ui.actionKey = actionKey;
      const button = refreshButton('metrics-refresh', view);
      if (ui.refresh === null) ui.actions.append(button);
      else ui.refresh.replaceWith(button);
      ui.refresh = button;
      if (state.refocusRefresh && view !== 'refreshing' && view !== 'loading') {
        state.refocusRefresh = false;
        button.focus();
      }
    }
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
    if (state.rejected) message = copy.metricsRejected;
    else if (state.data) message = copy.staleMessage;
    const action = refreshButton('metrics-alert-refresh', busy ? 'refreshing' : 'ok');
    const hadFocus = ui.alerts.contains(document.activeElement);
    ui.alerts.replaceChildren(createAlert({ tone: 'destructive', title: message, action }));
    if (hadFocus) action.focus();
  }

  function updateBody(view) {
    const key = [state.lang, visiblePeriod(), state.data ? `data:${state.dataVersion}` : view].join('|');
    if (ui.bodyKey === key) return;
    ui.bodyKey = key;
    if (!state.data && view !== 'loading') {
      ui.body.replaceChildren();
      return;
    }
    const copy = t();
    ui.body.replaceChildren(
      keySection(copy),
      trendSection(copy),
      distributionCard(copy),
      causeSection(copy),
      processCard(copy),
    );
  }

  function update() {
    if (state.ended || ui === null) return;
    const view = viewOf(state);
    updateHead(view);
    updateAlerts(view);
    updateBody(view);
  }

  // ---- Sections ----

  let sectionSeq = 0;

  function sectionOf(title, content) {
    const heading = el('h2', 'card-heading', title);
    const section = el('section', 'metrics-section');
    sectionSeq += 1;
    heading.id = `metrics-section-${sectionSeq}`;
    section.setAttribute('aria-labelledby', heading.id);
    section.append(heading, content);
    return section;
  }

  function loadingBadge(copy) {
    return statusBadge(copy, 'loading');
  }

  function unavailableBadge(copy) {
    return createBadge({ tone: 'neutral', label: copy.unavailable });
  }

  // Card whose body is one centered value or placeholder badge.
  function kpiCard(key, title, note, content) {
    const card = createCard({ title, description: note, headingLevel: 3, body: content });
    card.dataset.kpi = key;
    card.querySelector('.card-body').classList.add('kpi-body');
    return card;
  }

  function valueOrBadge(copy, value, format, loading) {
    if (loading) return loadingBadge(copy);
    if (value === null) return unavailableBadge(copy);
    return el('span', 'metric', format(value));
  }

  // Gauge of a connection limit: the limit text is CPY-079, or CPY-080 when unknown. An unknown usage is CPY-035.
  function gaugeCard(key, title, note, usage, loading) {
    const copy = t();
    const wrap = el('div', 'gauge-wrap');
    if (!loading && usage.current !== null && usage.limit === null) {
      wrap.append(el('span', 'metric', formatCount(usage.current)), el('span', 'stat-unit', copy.limitUnknown));
    } else {
      const limitText = loading || usage.limit === null ? copy.limitUnknown : fill(copy.limit, { n: formatCount(usage.limit) });
      wrap.append(
        loading || usage.current === null
          ? createGauge({
            label: title,
            value: null,
            max: loading || usage.limit === null ? 1 : usage.limit,
            limitText,
            unknownText: loading ? copy.loading : copy.unavailable,
            unknownTone: loading ? 'info' : 'neutral',
          })
          : createGauge({
            label: title,
            value: Math.min(usage.current, usage.limit),
            max: usage.limit,
            valueText: formatCount(usage.current),
            limitText,
          }),
      );
    }
    const card = createCard({ title, description: note, headingLevel: 3, body: wrap });
    card.dataset.kpi = key;
    return card;
  }

  function throughputCard(copy, kpi, loading) {
    let content;
    if (!loading && kpi.throughput.state === 'collecting') {
      content = createBadge({ tone: 'neutral', label: copy.collecting });
    } else if (!loading && kpi.throughput.state === 'unavailable') {
      content = unavailableBadge(copy);
    } else {
      const list = el('dl', 'kpi-xfer');
      const rows = [
        [copy.directionViewerToTarget, 'direction-mark-1', kpi?.throughput.viewerToTarget],
        [copy.directionTargetToViewer, 'direction-mark-2', kpi?.throughput.targetToViewer],
      ];
      for (const [name, markClass, value] of rows) {
        const row = el('div');
        const term = el('dt', 'label');
        const mark = el('span', `direction-mark ${markClass}`);
        mark.setAttribute('aria-hidden', 'true');
        term.append(mark, name);
        const detail = el('dd');
        detail.append(
          loading ? loadingBadge(copy) : el('span', 'metric', fill(copy.perSecond, { value: formatBytesRate(value) })),
        );
        row.append(term, detail);
        list.append(row);
      }
      content = list;
    }
    return kpiCard('throughput', copy.throughput, undefined, content);
  }

  function rateCard(copy, key, title, note, kpiValue, loading, format) {
    let content;
    if (!loading && kpiValue.state === 'collecting') content = createBadge({ tone: 'neutral', label: copy.collecting });
    else if (!loading && kpiValue.state === 'unavailable') content = unavailableBadge(copy);
    else content = loading ? loadingBadge(copy) : el('span', 'metric', format(kpiValue.percent));
    return kpiCard(key, title, note, content);
  }

  function keySection(copy) {
    const loading = !state.data;
    const kpi = loading ? null : keyIndicators(state.data, state.points);
    const grid = el('div', 'kpi-grid');
    grid.append(
      kpiCard('active', copy.activeCount, undefined, valueOrBadge(copy, kpi?.active ?? null, formatCount, loading)),
      kpiCard('waiting', copy.waitingCount, undefined, valueOrBadge(copy, kpi?.waiting ?? null, formatCount, loading)),
      gaugeCard('relay', copy.relayTitle, copy.relayNote, kpi?.relay, loading),
      gaugeCard('admin', copy.adminTitle, copy.adminNote, kpi?.admin, loading),
      throughputCard(copy, kpi, loading),
      kpiCard('rejected', copy.rejectedTotal, undefined, valueOrBadge(copy, kpi?.rejected ?? null, formatCount, loading)),
      rateCard(copy, 'cpu', copy.cpuUsage, periodTexts(copy).cpuNote, kpi?.cpu, loading, formatPercent),
      kpiCard('memory', copy.memoryUsage, copy.memoryNote, valueOrBadge(copy, kpi?.memory ?? null, formatBytesValue, loading)),
    );
    return sectionOf(copy.kpiTitle, grid);
  }

  // ---- Charts ----

  // Per chart: title, series names, and how a value reads in the legend and on the axis (DESIGN.md 표시 형식).
  function chartViews(copy) {
    const perSecond = (value) => fill(copy.perSecond, { value });
    const bytesTick = (value, axis) => `${formatDecimal(value / axis.divisor)} ${axis.unit}`;
    return {
      sessions: {
        title: copy.chartSessions,
        names: [copy.activeCount, copy.waitingCount],
        legend: formatCount,
        tick: formatCount,
      },
      connections: {
        title: copy.chartConnections,
        names: copy.connectionSeries,
        legend: formatDecimal,
        tick: formatDecimal,
      },
      throughput: {
        title: copy.chartThroughput,
        names: [copy.directionViewerToTarget, copy.directionTargetToViewer],
        legend: (value) => perSecond(formatBytesRate(value)),
        tick: (value, axis) => perSecond(bytesTick(value, axis)),
      },
      cpu: { title: copy.cpuUsage, names: [copy.cpuUsage], legend: formatPercent, tick: formatPercent },
      memory: {
        title: copy.memoryUsage,
        names: [copy.memoryUsage],
        legend: formatBytesValue,
        tick: bytesTick,
      },
    };
  }

  // Path data of one run. A single point becomes a zero-length segment that the round cap shows as a dot.
  function runPath(run, chart) {
    const coords = run.map((entry) => {
      const x = round(plotX(entry.atMs, chart.from, chart.to));
      return `${x} ${round(plotY(entry.value, chart.axis.top))}`;
    });
    return coords.length === 1 ? `M${coords[0]}L${coords[0]}` : `M${coords.join('L')}`;
  }

  function lineStyle(path, index) {
    const style = SERIES_STYLES[index];
    path.setAttribute('class', `chart-line ${style.cls}`);
    if (style.dash) path.setAttribute('stroke-dasharray', style.dash);
    return path;
  }

  function chartFigure(copy, chart, view) {
    const legendText = (series) => fill(copy.legendValue, {
      current: series.current === null ? copy.unavailable : view.legend(series.current),
      max: series.max === null ? copy.unavailable : view.legend(series.max),
    });
    const figure = el('figure', 'chart-fig');
    figure.setAttribute('role', 'img');
    figure.setAttribute(
      'aria-label',
      [view.title, ...chart.series.map((series, index) => `${view.names[index]}: ${legendText(series)}`)].join('. '),
    );

    const ticks = [chart.axis.top, chart.axis.top / 2, 0];
    const yAxis = el('div', 'chart-y label muted');
    for (const value of ticks) yAxis.append(el('span', undefined, view.tick(value, chart.axis)));

    const plot = svg('svg', {
      class: 'chart-plot',
      viewBox: `0 0 ${PLOT_WIDTH} ${PLOT_HEIGHT}`,
      preserveAspectRatio: 'none',
      'aria-hidden': 'true',
      focusable: 'false',
    });
    plot.append(svg('path', {
      class: 'chart-grid',
      d: `M0 0H${PLOT_WIDTH}M0 ${PLOT_HEIGHT / 2}H${PLOT_WIDTH}M0 ${PLOT_HEIGHT}H${PLOT_WIDTH}`,
      'vector-effect': 'non-scaling-stroke',
    }));
    chart.series.forEach((series, index) => {
      for (const run of series.runs) {
        const path = lineStyle(svg('path', { d: runPath(run, chart), 'vector-effect': 'non-scaling-stroke' }), index);
        if (run.length === 1) path.classList.add('chart-dot');
        plot.append(path);
      }
    });

    // Five local times that split the window span into quarters: 15 minutes or 6 hours apart.
    const quarterMs = (chart.to - chart.from) / 4;
    const xAxis = el('div', 'chart-x label muted');
    for (let step = 0; step <= 4; step += 1) xAxis.append(el('span', undefined, formatClock(chart.from + step * quarterMs)));
    const frame = el('div', 'chart');
    frame.append(yAxis, plot, xAxis);

    const legend = el('ul', 'chart-legend');
    chart.series.forEach((series, index) => {
      const swatch = svg('svg', { class: 'chart-swatch', viewBox: '0 0 24 16', 'aria-hidden': 'true', focusable: 'false' });
      swatch.append(lineStyle(svg('path', { d: 'M1 8H23' }), index));
      const item = el('li');
      item.append(swatch, el('span', undefined, view.names[index]), el('span', 'label muted', legendText(series)));
      legend.append(item);
    });
    figure.append(frame, legend);
    figure.dataset.chart = chart.id;
    return figure;
  }

  function chartBody(copy, chart, view, loading) {
    if (loading) {
      const empty = el('div', 'chart-empty');
      empty.append(loadingBadge(copy));
      return empty;
    }
    if (!chart.enough) return el('p', 'chart-empty', periodTexts(copy).chartEmpty);
    return chartFigure(copy, chart, view);
  }

  function trendSection(copy) {
    const loading = !state.data;
    const views = chartViews(copy);
    const charts = loading
      ? Object.keys(views).map((id) => ({ id, enough: false }))
      : buildCharts(state.points, state.shown);
    const grid = el('div', 'grid');
    for (const chart of charts) {
      const view = views[chart.id];
      const card = createCard({ title: view.title, headingLevel: 3, body: chartBody(copy, chart, view, loading) });
      card.classList.add('span-6');
      card.dataset.chartCard = chart.id;
      grid.append(card);
    }
    return sectionOf(periodTexts(copy).trends, grid);
  }

  // ---- Distribution ----

  // The caption names the table for assistive technology and is hidden on screen (DESIGN.md 표).
  function tableBlock(table) {
    table.querySelector('caption')?.classList.add('sr-only');
    const wrap = el('div', 'table-scroll');
    wrap.append(table);
    return wrap;
  }

  function boundText(copy, bound) {
    return fill(bound.over ? copy.p95Gt : copy.p95Le, { value: formatShortSeconds(bound.value, copy) });
  }

  function distributionBar(copy, row) {
    const items = row.buckets
      .filter((bucket) => bucket.count > 0)
      .map((bucket) => fill(copy.distBarItem, { bound: boundText(copy, bucket.bound), count: formatCount(bucket.count) }));
    const bar = svg('svg', {
      class: 'dist',
      viewBox: `0 0 ${row.viewWidth} ${DIST_VIEWBOX_HEIGHT}`,
      preserveAspectRatio: 'none',
      role: 'img',
      'aria-label': fill(copy.distBarName, { items: items.join(', ') }),
    });
    for (const bucket of row.buckets) {
      if (bucket.count === 0) continue;
      const rect = svg('rect', {
        class: 'dist-bar',
        x: bucket.x,
        y: round(DIST_VIEWBOX_HEIGHT - bucket.height),
        width: 8,
        height: round(bucket.height),
      });
      rect.dataset.count = String(bucket.count);
      bar.append(rect);
    }
    bar.append(svg('path', {
      class: 'dist-base',
      d: `M0 ${DIST_VIEWBOX_HEIGHT}H${row.viewWidth}`,
      'vector-effect': 'non-scaling-stroke',
    }));
    return bar;
  }

  function attributeText(attributes) {
    return el('span', 'mono', attributes.map(([name, value]) => `${name}=${value}`).join(', '));
  }

  function distributionCard(copy) {
    const columns = copy.distColumns.map((label, index) => ({
      key: ['item', 'attrs', 'count', 'mean', 'p95', 'dist'][index],
      label,
    }));
    let content;
    if (!state.data) {
      content = el('div', 'chart-empty');
      content.append(loadingBadge(copy));
    } else {
      const rows = [];
      const emptyAt = [];
      distributionItems(state.data.metrics).forEach((item, index) => {
        const name = copy.distNames[index];
        if (item.rows.length === 0) {
          emptyAt.push(rows.length);
          rows.push([name, '', '', '', '', '']);
          return;
        }
        for (const row of item.rows) {
          rows.push([
            name,
            attributeText(row.attributes),
            formatCount(row.count),
            row.mean === null ? unavailableBadge(copy) : formatShortSeconds(row.mean, copy),
            row.p95 === null ? unavailableBadge(copy) : boundText(copy, { value: row.p95.bound, over: row.p95.over }),
            distributionBar(copy, row),
          ]);
        }
      });
      const table = createDataTable({ caption: copy.distTitle, columns, rows, numericColumns: ['count', 'mean', 'p95'] });
      // An item without records is its name and one "no records" cell across the other columns.
      const bodyRows = [...table.tBodies[0].rows];
      for (const at of emptyAt) {
        const cells = [...bodyRows[at].cells];
        cells[1].colSpan = cells.length - 1;
        cells[1].textContent = copy.noRecords;
        cells[1].classList.add('none');
        cells[1].removeAttribute('data-label');
        for (const extra of cells.slice(2)) extra.remove();
      }
      content = tableBlock(table);
    }
    const card = createCard({ title: copy.distTitle, description: copy.distNote, headingLevel: 2, body: content });
    card.dataset.metricsCard = 'distribution';
    return card;
  }

  // ---- Counts by cause ----

  function causeColumnLabels(copy, id) {
    if (id === 'rejected' || id === 'aborted') return copy.causeConnectionColumns;
    if (id === 'closed') return copy.causeClosedColumns;
    if (id === 'admin') return copy.causeAdminColumns;
    if (id === 'config') return copy.causeConfigColumns;
    return copy.causeWebSocketColumns;
  }

  function causeItems(copy, id) {
    if (id === 'admin') return copy.adminItems;
    if (id === 'config') return copy.configItems;
    return copy.webSocketItems;
  }

  function causeCell(copy, cell) {
    if (cell === null) return '';
    if (cell.name === 'close_cause') return closeCauseBadge(copy, cell.value);
    return el('span', 'mono', cell.value);
  }

  function countCell(count, largest) {
    const cell = el('span', 'count-cell');
    const bar = svg('svg', {
      class: 'count-bar',
      viewBox: `0 0 ${COUNT_VIEWBOX_WIDTH} 8`,
      preserveAspectRatio: 'none',
      'aria-hidden': 'true',
      focusable: 'false',
    });
    bar.append(svg('rect', { class: 'count-track', width: COUNT_VIEWBOX_WIDTH, height: 8 }));
    const width = countBarWidth(count, largest);
    if (width > 0) bar.append(svg('rect', { class: 'count-fill', width: round(width), height: 8 }));
    cell.append(bar, el('span', 'num', formatCount(count)));
    return cell;
  }

  function causeCard(copy, table, index) {
    const labels = causeColumnLabels(copy, table.id);
    const columns = labels.map((label, position) => ({
      key: position === labels.length - 1 ? 'count' : `c${position}`,
      label,
    }));
    let content;
    if (!state.data) {
      content = el('div', 'chart-empty');
      content.append(loadingBadge(copy));
    } else {
      const found = causeRows(state.data.metrics, table);
      const largest = found.reduce((top, row) => Math.max(top, row.count), 0);
      const rows = found.map((row) => [
        ...(table.item ? [causeItems(copy, table.id)[row.itemIndex] ?? ''] : []),
        ...row.cells.map((cell) => causeCell(copy, cell)),
        countCell(row.count, largest),
      ]);
      content = tableBlock(
        createDataTable({ caption: copy.causeTables[index], columns, rows, emptyText: copy.noRecords, numericColumns: ['count'] }),
      );
    }
    const card = createCard({ title: copy.causeTables[index], headingLevel: 3, body: content });
    card.classList.add('span-6');
    card.dataset.cause = table.id;
    return card;
  }

  function causeSection(copy) {
    const grid = el('div', 'grid');
    grid.append(...CAUSE_TABLES.map((table, index) => causeCard(copy, table, index)));
    return sectionOf(copy.causeTitle, grid);
  }

  // ---- Process ----

  function processLabels(copy) {
    return {
      uptime: copy.uptime,
      cpuTime: copy.cpuTime,
      memory: copy.memoryUsage,
      virtual: copy.virtualMemory,
      threads: copy.threadCount,
      handles: copy.handleCount,
      descriptors: copy.descriptorCount,
      version: copy.version,
    };
  }

  // Value of one process row. Seconds read as `D일 HH:MM:SS`, CPU time in the short format, memory in KiB/MiB.
  function processValue(copy, row) {
    if (row.value === null) return unavailableBadge(copy);
    if (row.key === 'uptime') return el('span', 'num', formatDuration(row.value * 1000, copy.dayUnit));
    if (row.key === 'cpuTime') {
      const part = (seconds) => (seconds === null ? copy.unavailable : formatShortSeconds(seconds, copy));
      return el('span', 'num', `${part(row.value.user)} / ${part(row.value.system)}`);
    }
    if (row.key === 'memory' || row.key === 'virtual') return el('span', 'num', formatBytesValue(row.value));
    if (row.key === 'version') return el('span', 'mono', row.value);
    return el('span', 'num', formatCount(row.value));
  }

  function processCard(copy) {
    const labels = processLabels(copy);
    const list = el('dl', 'kv-list');
    // The platform is unknown before the first response, so the handle and descriptor rows wait for it.
    const rows = state.data
      ? processRows(state.data)
      : ['uptime', 'cpuTime', 'memory', 'virtual', 'threads', 'version'].map((key) => ({ key, value: null }));
    for (const row of rows) {
      const detail = el('dd');
      detail.append(state.data ? processValue(copy, row) : loadingBadge(copy));
      list.append(el('dt', undefined, labels[row.key]), detail);
    }
    const card = createCard({ title: copy.processTitle, headingLevel: 2, body: list });
    card.dataset.metricsCard = 'process';
    return card;
  }

  applyDocumentLanguage();
  build();
  update();
  void load(true);

  return { dispose };
}
