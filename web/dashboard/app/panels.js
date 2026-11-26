// Builders for the operations overview panels. Each takes copy from COPY[lang] and checked data, and returns DOM nodes.
// API values enter the DOM only through textContent, attributes, or component APIs.

import { createAlert } from './components/alert.js';
import { createBadge } from './components/badge.js';
import { createCard } from './components/card.js';
import { createDataTable } from './components/data-table.js';
import { createGauge } from './components/gauge.js';
import { icon } from './components/icons.js';
import { createLink } from './components/link.js';
import { createStat } from './components/stat.js';
import {
  fill,
  formatAddress,
  formatBytes,
  formatCount,
  formatDuration,
  formatMode,
  formatTime,
  formatTransport,
} from './format.js';

const WAITING_KEYS = ['role', 'relayId', 'transport', 'address', 'connectedAt', 'waited'];
const ACTIVE_KEYS = ['session', 'mode', 'transport', 'started', 'bytes', 'peer', 'details'];
const CLOSED_KEYS = ['session', 'cause', 'mode', 'transport', 'time', 'bytes', 'peer', 'logs'];

export function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) {
    node.className = className;
  }
  if (text !== undefined) {
    node.textContent = text;
  }
  return node;
}

// Badge tone and copy key for each query view. Views without a badge are absent.
const BADGES = {
  loading: ['info', 'loading'],
  refreshing: ['info', 'refreshing'],
  stale: ['warning', 'stale'],
};

// Badge for a loading, refreshing, or stale view. Other views return null.
export function statusBadge(t, kind) {
  const spec = BADGES[kind];
  return spec ? createBadge({ tone: spec[0], label: t[spec[1]] }) : null;
}

// Alert for a query that failed. The last successful time is in the page head (CPY-015), not in the alert.
export function statusAlert(t, kind) {
  if (kind === 'stale') {
    return createAlert({ tone: 'destructive', title: t.failed, message: t.staleMessage });
  }
  if (kind === 'failed') {
    return createAlert({ tone: 'destructive', title: t.failed, message: t.failedMessage });
  }
  return null;
}

// Summary stats. Unknown values (null) show "확인 불가" in a neutral badge. Before the first success they show
// "조회 중" in the info badge, the same mark as the card's status badge (DESIGN 상태 색).
export function summaryGrid(t, summary, kind) {
  const loading = kind === 'loading';
  const placeholder = loading ? t.loading : t.unavailable;
  const placeholderTone = loading ? 'info' : 'neutral';
  const count = (value) => (summary && value !== null ? formatCount(value) : placeholder);
  const state = summary ? stateText(t, summary.serverState) : { text: placeholder, tone: placeholderTone };
  const grid = el('div', 'summary-grid');
  grid.append(
    // The server state is a status badge in every state (DESIGN 391, 639).
    createStat({ label: t.serverState, value: state.text, tone: state.tone, badge: true }),
    createStat({
      label: t.uptime,
      value: summary ? formatDuration(summary.uptimeMs, t.dayUnit) : placeholder,
      tone: summary ? 'neutral' : placeholderTone,
      badge: !summary,
    }),
    createStat({
      label: t.waitingCount,
      value: summary ? formatCount(summary.waiting) : placeholder,
      tone: summary ? 'neutral' : placeholderTone,
      badge: !summary,
    }),
    createStat({
      label: t.activeCount,
      value: summary ? formatCount(summary.active) : placeholder,
      tone: summary ? 'neutral' : placeholderTone,
      badge: !summary,
    }),
    createStat({
      label: t.handshaking,
      value: count(summary?.handshaking),
      tone: summary ? 'neutral' : placeholderTone,
      badge: !summary || summary.handshaking === null,
    }),
    createStat({
      label: t.targetConnecting,
      value: count(summary?.targetConnecting),
      tone: summary ? 'neutral' : placeholderTone,
      badge: !summary || summary.targetConnecting === null,
    }),
  );
  return grid;
}

// Server state values from the contract. Unknown values are shown as received.
function stateText(t, state) {
  if (state === 'ready') {
    return { text: t.stateReady, tone: 'success' };
  }
  if (state === 'shutdown_grace') {
    return { text: t.stateShutdownGrace, tone: 'warning' };
  }
  return { text: state, tone: 'neutral' };
}

// Connection usage gauge. `usage` is null until the first success.
export function usageGauge({ t, title, usage, kind }) {
  let gauge;
  if (usage) {
    const limitText = fill(t.limit, { n: formatCount(usage.limit) });
    gauge =
      usage.current === null
        ? createGauge({ label: title, value: null, max: usage.limit, limitText, unknownText: t.unavailable })
        : createGauge({
            label: title,
            value: Math.min(usage.current, usage.limit),
            max: usage.limit,
            valueText: formatCount(usage.current),
            limitText,
          });
  } else {
    gauge = createGauge({
      label: title,
      value: null,
      max: 1,
      limitText: t.limitUnknown,
      unknownText: kind === 'loading' ? t.loading : t.unavailable,
      unknownTone: kind === 'loading' ? 'info' : 'neutral',
    });
  }
  const block = el('div', 'gauge-wrap');
  block.append(gauge);
  return block;
}

function cellLine(label, value) {
  const line = el('div', 'cell-line');
  line.append(el('span', 'cell-label', label), document.createTextNode(' '), el('span', 'cell-value', value));
  return line;
}

function directionLine(label, value, markClass) {
  const line = cellLine(label, value);
  const mark = el('span', `direction-mark ${markClass}`);
  mark.setAttribute('aria-hidden', 'true');
  line.querySelector('.cell-label').prepend(mark);
  return line;
}

function bytesCell(t, item) {
  const cell = el('div', 'cell-bytes');
  cell.append(
    directionLine(t.directionViewerToTarget, formatBytes(item.bytesViewerToTarget), 'direction-mark-1'),
    directionLine(t.directionTargetToViewer, formatBytes(item.bytesTargetToViewer), 'direction-mark-2'),
  );
  return cell;
}

function peerCell(t, item) {
  const cell = el('div', 'cell-peer');
  cell.append(
    addressLine(t.viewerAddressLabel, formatAddress(item.viewerAddress)),
    addressLine(t.targetAddressLabel, formatAddress(item.targetAddress)),
  );
  return cell;
}

// A labeled address line whose label sits above the value (Q235 waiting and closed tables, Q237 active table).
function addressLine(label, value) {
  const line = cellLine(label, value);
  line.classList.add('cell-line-stacked');
  line.querySelector('.cell-value').classList.add('fact-address');
  return line;
}

// Closed session addresses: each socket address, then its forwarded IP when there is one.
function closedAddressCell(t, item) {
  const cell = el('div', 'cell-peer');
  cell.append(addressLine(t.viewerAddressLabel, formatAddress(item.viewerAddress)));
  if (item.viewerForwardedIp !== null) {
    cell.append(addressLine(t.viewerForwardedAddressLabel, item.viewerForwardedIp));
  }
  cell.append(addressLine(t.targetAddressLabel, formatAddress(item.targetAddress)));
  if (item.targetForwardedIp !== null) {
    cell.append(addressLine(t.targetForwardedAddressLabel, item.targetForwardedIp));
  }
  return cell;
}

// Waiting connection address: the socket address, then the forwarded IP on its own labeled line when there is one.
function waitingAddressCell(t, item) {
  const cell = el('div', 'cell-peer');
  const socket = el('div', 'cell-line');
  socket.append(el('span', 'cell-value fact-address', formatAddress(item.address)));
  cell.append(socket);
  if (item.forwardedIp !== null) {
    cell.append(addressLine(t.forwardedAddressLabel, item.forwardedIp));
  }
  return cell;
}

// A formatted time as two whole parts: the date, then the clock with its zone. A space stays between them, so the
// text is still the formatted string.
function timeParts(value) {
  const split = value.indexOf(' ');
  return [
    el('span', 'data-table-nowrap', value.slice(0, split)),
    document.createTextNode(' '),
    el('span', 'data-table-nowrap', value.slice(split + 1)),
  ];
}

// A time line flows as text: label, date, then the clock with its zone. It breaks only between them, so the clock
// moves to the next line first (DESIGN: a narrow time value takes two lines; Q235 mockup `.tl`).
function timeLine(label, value) {
  const line = cellLine(label, value);
  line.classList.add('cell-line-flow');
  line.querySelector('.cell-value').replaceChildren(...timeParts(value));
  return line;
}

// Waiting connect time, active start time, and SCR-008 last event time: the date, then the clock with its zone below
// it at every width (Q236 mockup `.wd`·`.wt`).
export function timeStack(value) {
  const cell = el('div', 'cell-time-stack');
  cell.append(...timeParts(value));
  return cell;
}

// Start, end, and duration lines of a closed session. The duration is the server's duration_ms.
function closedTimeCell(t, item) {
  const cell = el('div', 'cell-time');
  cell.append(
    timeLine(t.startedLabel, formatTime(item.startedAt)),
    timeLine(t.closedAtLabel, formatTime(item.endedAt)),
    cellLine(t.durationLabel, formatDuration(item.durationMs, t.dayUnit)),
  );
  return cell;
}

// Link to the session detail screen (SCR-002).
function detailsLink(t, sessionId) {
  const link = el('a', 'button button-outline');
  link.href = `/#/sessions/${encodeURIComponent(sessionId)}`;
  link.append(icon('chevron-right'), el('span', 'button-label', t.details));
  return link;
}

// Link to the recent logs screen (SCR-006) with the session filter. SCR-001 closed rows and SCR-002 use it.
export function logsLink(t, sessionId) {
  const link = el('a', 'button button-outline');
  link.href = `/#/logs?session_id=${encodeURIComponent(sessionId)}`;
  link.append(icon('log'), el('span', 'button-label', t.viewLogs));
  return link;
}

// `wrapped` lists the columns whose long header may wrap (the adopted mockup's wrapping headers).
function columns(labels, keys, primary, secondary, wrapped = []) {
  return keys.map((key, index) => ({
    key,
    label: labels[index],
    ...(key === primary ? { mobileHead: 'primary' } : {}),
    ...(key === secondary ? { mobileHead: 'secondary' } : {}),
    ...(wrapped.includes(key) ? { wrapLabel: true } : {}),
  }));
}

function tableBlock(config) {
  const wrap = el('div', 'table-scroll');
  wrap.append(createDataTable(config));
  return wrap;
}

// Known roles use translated copy (CPY-327, CPY-328). An unknown role is shown as received.
function roleLabel(t, role) {
  if (role === 'viewer') {
    return t.roleViewer;
  }
  if (role === 'server') {
    return t.roleServer;
  }
  return role;
}

export function waitingTable(t, items, emptyText) {
  return tableBlock({
    columns: columns(t.waitingColumns, WAITING_KEYS, 'relayId', 'role'),
    rows: items.map((item) => [
      createBadge({ label: roleLabel(t, item.role) }),
      el('span', 'session-id', item.relayId),
      createBadge({ label: formatTransport(item.transport) }),
      waitingAddressCell(t, item),
      timeStack(formatTime(item.connectedAt)),
      formatDuration(item.waitedMs, t.dayUnit),
    ]),
    emptyText,
    numericColumns: ['waited'],
    labelPrimary: true,
  });
}

export function activeTable(t, items, emptyText) {
  return tableBlock({
    columns: columns(t.activeColumns, ACTIVE_KEYS, 'session', 'details'),
    rows: items.map((item) => [
      createLink({ href: `/#/sessions/${encodeURIComponent(item.sessionId)}`, label: item.sessionId }),
      createBadge({ label: formatMode(item.mode) }),
      transportCell(item),
      timeStack(formatTime(item.startedAt)),
      bytesCell(t, item),
      peerCell(t, item),
      detailsLink(t, item.sessionId),
    ]),
    emptyText,
    wide: true,
  });
}

function transportCell(item) {
  const cell = el('span', 'xport');
  cell.append(
    createBadge({ label: formatTransport(item.viewerTransport) }),
    icon('arrow-right', { size: 'sm' }),
    el('span', 'sr-only', ' → '),
    createBadge({ label: formatTransport(item.targetTransport) }),
  );
  return cell;
}

export function closeCauseBadge(t, cause) {
  const tone = cause === 'connection_error' || cause === 'internal_error'
    ? 'destructive'
    : ['dead_connection', 'close_timeout'].includes(cause) ? 'warning' : 'neutral';
  const label = Object.hasOwn(t.closeCause, cause) ? t.closeCause[cause] : cause;
  return createBadge({ tone, label });
}

export function closedTable(t, items, emptyText) {
  return tableBlock({
    columns: columns(t.closedColumns, CLOSED_KEYS, 'session', 'cause', ['transport']),
    rows: items.map((item) => [
      el('span', 'session-id', item.sessionId),
      closeCauseBadge(t, item.closeCause),
      createBadge({ label: formatMode(item.mode) }),
      transportCell(item),
      closedTimeCell(t, item),
      bytesCell(t, item),
      closedAddressCell(t, item),
      logsLink(t, item.sessionId),
    ]),
    emptyText,
    wide: true,
  });
}

// Page head for SCR-001 and SCR-002. `back` (the SCR-002 ghost link) sits above the title. `actions` sit beside the title block.
export function pageHead({ back = null, title = '', actions = [] }) {
  const heading = el('h1', 'page-title', title);
  const meta = el('p', 'page-meta');
  const text = el('div', 'page-head-text');
  text.append(...(back ? [back] : []), heading, meta);
  const actionRow = el('div', 'page-actions');
  actionRow.append(...actions);
  const head = el('header', 'page-head');
  head.append(text, actionRow);
  return { head, heading, meta, actions: actionRow };
}

// Card with a status badge slot and an alert slot (list and summary panels). Chrome is optional. Footer is an optional action row.
export function panelCard({ title, description, span, chrome, footer }) {
  const slots = {
    badgeSlot: el('div', 'card-status'),
    alertSlot: el('div', 'card-alert'),
    contentSlot: el('div', 'card-content'),
    chromeKey: null,
    contentKey: null,
    titleSlot: null,
    countBadge: null,
  };
  let body = slots.contentSlot;
  if (chrome) {
    body = el('div', 'card-stack');
    body.append(slots.alertSlot, slots.contentSlot);
  }
  const node = createCard({ title, description, actions: chrome ? slots.badgeSlot : undefined, body, footer });
  slots.titleSlot = node.querySelector('.card-title');
  node.classList.add(span);
  return { node, slots };
}

// Counts belong to the same successful list snapshot as the rows, not the summary.
export function applyListCount(slots, count, t) {
  if (count === null) {
    slots.countBadge?.remove();
    slots.countBadge = null;
    return;
  }
  const label = count === 1 ? t.listCountOne : fill(t.listCount, { n: formatCount(count) });
  if (!slots.countBadge) {
    slots.countBadge = createBadge({ tone: 'neutral', label });
    slots.titleSlot.append(slots.countBadge);
  } else if (slots.countBadge.textContent !== label) {
    slots.countBadge.textContent = label;
  }
}

// Updates the badge and alert only when the view changes.
export function applyChrome(slots, view, t) {
  if (slots.chromeKey === view) {
    return;
  }
  slots.chromeKey = view;
  const badge = statusBadge(t, view);
  const alert = statusAlert(t, view);
  slots.badgeSlot.replaceChildren(...(badge ? [badge] : []));
  slots.alertSlot.replaceChildren(...(alert ? [alert] : []));
}

// Replaces the content only when the key changes. Rebuilding only on change keeps focus steady.
export function setContent(slots, key, build) {
  if (slots.contentKey === key) {
    return;
  }
  slots.contentKey = key;
  const node = build();
  slots.contentSlot.replaceChildren(...(node ? [node] : []));
}
