// Builders for the operations overview panels. Each takes copy from COPY[lang] and checked data, and returns DOM nodes.
// API values enter the DOM only through textContent, attributes, or component APIs.

import { createAlert } from './components/alert.js';
import { createBadge } from './components/badge.js';
import { createCard } from './components/card.js';
import { createDataTable } from './components/data-table.js';
import { createGauge } from './components/gauge.js';
import { createStat } from './components/stat.js';
import {
  fill,
  formatAddress,
  formatBytes,
  formatCount,
  formatDuration,
  formatMode,
  formatRole,
  formatTime,
  formatTransport,
} from './format.js';

const WAITING_KEYS = ['role', 'relayId', 'connectedAt', 'waited'];
const ACTIVE_KEYS = ['session', 'mode', 'transport', 'started', 'bytes', 'peer', 'details'];
const CLOSED_KEYS = ['session', 'cause', 'endedAt', 'bytes'];

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

// Alert for a query that failed. Stale data shows the last successful time.
export function statusAlert(t, kind, lastSuccessAt) {
  if (kind === 'stale') {
    return createAlert({
      tone: 'destructive',
      title: t.failed,
      message: fill(t.staleMessage, { time: formatTime(lastSuccessAt) }),
    });
  }
  if (kind === 'failed') {
    return createAlert({ tone: 'destructive', title: t.failed, message: t.failedMessage });
  }
  return null;
}

// Summary stats. Unknown values (null) show "확인 불가". Before the first success they show the query state.
export function summaryGrid(t, summary, kind) {
  const placeholder = kind === 'loading' ? t.loading : t.unavailable;
  const count = (value) => (summary && value !== null ? formatCount(value) : placeholder);
  const state = summary ? stateText(t, summary.serverState) : { text: placeholder, tone: 'neutral' };
  const grid = el('div', 'summary-grid');
  grid.append(
    createStat({ label: t.serverState, value: state.text, tone: state.tone, badge: !summary }),
    createStat({
      label: t.uptime,
      value: summary ? formatDuration(summary.uptimeMs, t.dayUnit) : placeholder,
      tone: 'neutral',
      badge: !summary,
    }),
    createStat({
      label: t.waitingCount,
      value: summary ? formatCount(summary.waiting) : placeholder,
      tone: 'neutral',
      badge: !summary,
    }),
    createStat({
      label: t.activeCount,
      value: summary ? formatCount(summary.active) : placeholder,
      tone: 'neutral',
      badge: !summary,
    }),
    createStat({
      label: t.handshaking,
      value: count(summary?.handshaking),
      tone: 'neutral',
      badge: !summary || summary.handshaking === null,
    }),
    createStat({
      label: t.targetConnecting,
      value: count(summary?.targetConnecting),
      tone: 'neutral',
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

// Connection usage gauge with its description. `usage` is null until the first success.
export function usageGauge({ t, title, note, usage, kind }) {
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
    });
  }
  const block = el('div', 'gauge-wrap');
  block.append(gauge, el('p', 'card-note', note));
  return block;
}

function cellLine(label, value) {
  const line = el('div', 'cell-line');
  line.append(el('span', 'cell-label', label), document.createTextNode(` ${value}`));
  return line;
}

function bytesCell(t, item) {
  const cell = el('div', 'cell-bytes');
  cell.append(
    cellLine(t.directionViewerToTarget, formatBytes(item.bytesViewerToTarget)),
    cellLine(t.directionTargetToViewer, formatBytes(item.bytesTargetToViewer)),
  );
  return cell;
}

function peerCell(t, item) {
  const cell = el('div', 'cell-peer');
  cell.append(
    cellLine(t.viewerAddressLabel, formatAddress(item.viewerAddress)),
    cellLine(t.targetAddressLabel, formatAddress(item.targetAddress)),
  );
  return cell;
}

// Link to the session detail screen (SCR-002).
function detailsLink(t, sessionId) {
  const link = el('a', 'button button-outline');
  link.href = `/#/sessions/${encodeURIComponent(sessionId)}`;
  link.append(el('span', 'button-label', t.details));
  return link;
}

function columns(labels, keys) {
  return keys.map((key, index) => ({ key, label: labels[index] }));
}

function tableBlock(config) {
  const wrap = el('div', 'table-scroll');
  wrap.append(createDataTable(config));
  return wrap;
}

export function waitingTable(t, items, emptyText) {
  return tableBlock({
    columns: columns(t.waitingColumns, WAITING_KEYS),
    rows: items.map((item) => [
      formatRole(item.role),
      item.relayId,
      formatTime(item.connectedAt),
      formatDuration(item.waitedMs, t.dayUnit),
    ]),
    emptyText,
    numericColumns: ['waited'],
  });
}

export function activeTable(t, items, emptyText) {
  return tableBlock({
    columns: columns(t.activeColumns, ACTIVE_KEYS),
    rows: items.map((item) => [
      item.sessionId,
      formatMode(item.mode),
      formatTransport(item.viewerTransport),
      formatTime(item.startedAt),
      bytesCell(t, item),
      peerCell(t, item),
      detailsLink(t, item.sessionId),
    ]),
    emptyText,
    numericColumns: ['bytes'],
  });
}

export function closedTable(t, items, emptyText) {
  return tableBlock({
    columns: columns(t.closedColumns, CLOSED_KEYS),
    rows: items.map((item) => [
      item.sessionId,
      Object.hasOwn(t.closeCause, item.closeCause) ? t.closeCause[item.closeCause] : item.closeCause,
      formatTime(item.endedAt),
      bytesCell(t, item),
    ]),
    emptyText,
    numericColumns: ['bytes'],
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
export function panelCard({ title, span, chrome, footer }) {
  const slots = {
    badgeSlot: el('div', 'card-status'),
    alertSlot: el('div', 'card-alert'),
    contentSlot: el('div', 'card-content'),
    chromeKey: null,
    contentKey: null,
  };
  let body = slots.contentSlot;
  if (chrome) {
    body = el('div', 'card-stack');
    body.append(slots.alertSlot, slots.contentSlot);
  }
  const node = createCard({ title, actions: chrome ? slots.badgeSlot : undefined, body, footer });
  node.classList.add(span);
  return { node, slots };
}

// Updates the badge and alert only when the view or the last success time changes.
export function applyChrome(slots, query, view, t) {
  const key = [view, query.lastSuccessAt].join('|');
  if (slots.chromeKey === key) {
    return;
  }
  slots.chromeKey = key;
  const badge = statusBadge(t, view);
  const alert = statusAlert(t, view, query.lastSuccessAt);
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
