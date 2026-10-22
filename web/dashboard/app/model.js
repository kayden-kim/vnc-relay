// Shape checks for management API responses. A response that fails a check becomes an 'invalid' query failure.
// Unknown fields are ignored. Unknown string values are kept as received (display shows them raw).

import { ApiError } from './api.js';

const TIMESTAMP = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/;
const SESSION_ID = /^[A-Za-z0-9_-]{1,64}$/;

// Session identifier format of contract 1.8. Checked before any request that uses the id.
export function isSessionId(value) {
  return typeof value === 'string' && SESSION_ID.test(value);
}

function invalid() {
  return new ApiError('invalid');
}

function object(value) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw invalid();
  }
  return value;
}

function array(value) {
  if (!Array.isArray(value)) {
    throw invalid();
  }
  return value;
}

function integer(value, min = 0) {
  if (!Number.isInteger(value) || value < min) {
    throw invalid();
  }
  return value;
}

function nullableInteger(value) {
  return value === null ? null : integer(value);
}

function text(value) {
  if (typeof value !== 'string') {
    throw invalid();
  }
  return value;
}

function nonEmptyText(value) {
  const checked = text(value);
  if (checked.length === 0) {
    throw invalid();
  }
  return checked;
}

function timestamp(value) {
  if (typeof value !== 'string' || !TIMESTAMP.test(value) || Number.isNaN(Date.parse(value))) {
    throw invalid();
  }
  return value;
}

function sessionId(value) {
  if (typeof value !== 'string' || !SESSION_ID.test(value)) {
    throw invalid();
  }
  return value;
}

function address(value) {
  const checked = object(value);
  const port = integer(checked.port);
  if (port > 65535) {
    throw invalid();
  }
  return { ip: nonEmptyText(checked.ip), port };
}

export function parseSession(body) {
  const session = object(body);
  const dashboard = object(session.dashboard);
  return {
    csrfToken: nonEmptyText(session.csrf_token),
    pollMs: integer(dashboard.poll_secs, 1) * 1000,
    queryTimeoutMs: integer(dashboard.query_timeout_secs, 1) * 1000,
    terminateTimeoutMs: integer(dashboard.terminate_timeout_secs, 1) * 1000,
  };
}

export function parseSummary(body) {
  const summary = object(body);
  timestamp(summary.as_of);
  const server = object(summary.server);
  const counts = object(summary.counts);
  const usage = object(summary.usage);
  const relay = object(usage.relay_connections);
  const admin = object(usage.admin_connections);
  return {
    serverState: text(server.state),
    uptimeMs: integer(server.uptime_ms),
    waiting: integer(counts.waiting),
    active: integer(counts.active),
    handshaking: nullableInteger(counts.handshaking),
    targetConnecting: nullableInteger(counts.target_connecting),
    relay: { current: nullableInteger(relay.current), limit: integer(relay.limit, 1) },
    admin: { current: nullableInteger(admin.current), limit: integer(admin.limit, 1) },
  };
}

export function parseWaiting(body) {
  const waiting = object(body);
  timestamp(waiting.as_of);
  return {
    items: array(waiting.items).map((raw) => {
      const item = object(raw);
      return {
        role: text(item.role),
        relayId: text(item.relay_id),
        connectedAt: timestamp(item.connected_at),
        waitedMs: integer(item.waited_ms),
      };
    }),
  };
}

// One session object of the active list and of the detail response (contract 1.8).
function sessionItem(raw) {
  const item = object(raw);
  return {
    sessionId: sessionId(item.session_id),
    state: text(item.state),
    mode: text(item.mode),
    viewerTransport: text(item.viewer_transport),
    startedAt: timestamp(item.started_at),
    bytesViewerToTarget: integer(item.bytes_viewer_to_target),
    bytesTargetToViewer: integer(item.bytes_target_to_viewer),
    viewerAddress: address(item.viewer_address),
    targetAddress: address(item.target_address),
  };
}

// One closed-session object of the closed list and of the terminate response (contract 1.10).
function closedItem(raw) {
  const item = object(raw);
  const startedAt = timestamp(item.started_at);
  const endedAt = timestamp(item.ended_at);
  if (Date.parse(endedAt) < Date.parse(startedAt)) {
    throw invalid();
  }
  return {
    sessionId: sessionId(item.session_id),
    mode: text(item.mode),
    viewerTransport: text(item.viewer_transport),
    startedAt,
    endedAt,
    bytesViewerToTarget: integer(item.bytes_viewer_to_target),
    bytesTargetToViewer: integer(item.bytes_target_to_viewer),
    closeCause: text(item.close_cause),
  };
}

export function parseSessions(body) {
  const list = object(body);
  timestamp(list.as_of);
  return { items: array(list.items).map(sessionItem) };
}

export function parseClosed(body) {
  const list = object(body);
  timestamp(list.as_of);
  integer(list.max_entries);
  return { items: array(list.items).map(closedItem) };
}

// Detail response (contract 1.8). Its session object is the same as one item of the active list.
export function parseDetail(body) {
  const detail = object(body);
  timestamp(detail.as_of);
  return { session: sessionItem(detail.session) };
}

// Terminate response (contract 1.9). An unknown result value is invalid, so the caller treats the outcome as unknown.
export function parseTerminate(body) {
  const outcome = object(body);
  timestamp(outcome.as_of);
  const result = text(outcome.result);
  if (result !== 'terminated' && result !== 'already_closed') {
    throw invalid();
  }
  closedItem(outcome.closed_session);
  return { result };
}
