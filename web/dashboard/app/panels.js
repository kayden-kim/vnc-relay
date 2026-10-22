// Builders for the operations overview panels. Each takes copy from COPY[lang] and checked data, and returns DOM nodes.
// API values enter the DOM only through textContent, attributes, or component APIs.

import { createAlert } from './components/alert.js';
import { createBadge } from './components/badge.js';
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
    createStat({ label: t.serverState, value: state.text, tone: state.tone }),
    createStat({
      label: t.uptime,
      value: summary ? formatDuration(summary.uptimeMs, t.dayUnit) : placeholder,
      tone: 'neutral',
    }),
    createStat({
      label: t.waitingCount,
      value: summary ? formatCount(summary.waiting) : placeholder,
      tone: 'neutral',
    }),
    createStat({
      label: t.activeCount,
      value: summary ? formatCount(summary.active) : placeholder,
      tone: 'neutral',
    }),
    createStat({ label: t.handshaking, value: count(summary?.handshaking), tone: 'neutral' }),
    createStat({ label: t.targetConnecting, value: count(summary?.targetConnecting), tone: 'neutral' }),
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
    cellLine(t.peerViewer, formatAddress(item.viewerAddress)),
    cellLine(t.peerTarget, formatAddress(item.targetAddress)),
  );
  return cell;
}

// Link to the session detail screen (SCR-002). The target screen is not built yet.
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
