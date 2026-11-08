// Management API calls for the operations overview. Each call resolves to parsed JSON, or throws an ApiError.

export const SESSION_PATH = '/api/v1/session';

// Detail path of one session. The caller checks the id format first.
export function sessionPath(id) {
  return `/api/v1/sessions/${id}`;
}

export const QUERY_PATHS = {
  summary: '/api/v1/summary',
  waiting: '/api/v1/waiting',
  sessions: '/api/v1/sessions',
  closed: '/api/v1/closed-sessions',
};

// Implementation choice: no response within these limits counts as a failure.
export const BOOTSTRAP_TIMEOUT_MS = 10000;
export const LOGOUT_TIMEOUT_MS = 10000;

// kind: 'unauthorized' (HTTP 401), 'failed' (other HTTP status), 'timeout', 'network', or 'invalid' (body does not parse).
// code: the contract error code of a failed response, or null when the body has none.
export class ApiError extends Error {
  constructor(kind, status = null, code = null, details = null) {
    super(kind);
    this.name = 'ApiError';
    this.kind = kind;
    this.status = status;
    this.code = code;
    this.details = details;
  }
}

export function isUnauthorized(error) {
  return error instanceof ApiError && error.kind === 'unauthorized';
}

// Runs one request under a timeout. `handle` reads the response and may throw an ApiError.
async function exchange(request, path, init, timeoutMs, handle) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await request(path, {
      ...init,
      credentials: 'same-origin',
      cache: 'no-store',
      signal: controller.signal,
    });
    if (response.status === 401) {
      throw new ApiError('unauthorized', 401);
    }
    return await handle(response);
  } catch (error) {
    if (error instanceof ApiError) {
      throw error;
    }
    throw new ApiError(controller.signal.aborted ? 'timeout' : 'network');
  } finally {
    clearTimeout(timer);
  }
}

async function readJson(response) {
  if (!response.ok) {
    let details = null;
    try {
      details = JSON.parse(await response.text())?.error ?? null;
    } catch {
      // A non-JSON rejection still belongs to its HTTP status category.
    }
    throw new ApiError('failed', response.status, details?.code ?? null, details);
  }
  const text = await response.text();
  try {
    return JSON.parse(text);
  } catch {
    throw new ApiError('invalid', response.status);
  }
}

export function getSession(request, { timeoutMs = BOOTSTRAP_TIMEOUT_MS } = {}) {
  return exchange(request, SESSION_PATH, { method: 'GET' }, timeoutMs, readJson);
}

// Sign-out succeeds only on 204. Any other status is a failure.
export function deleteSession(request, { csrfToken, timeoutMs = LOGOUT_TIMEOUT_MS }) {
  const init = { method: 'DELETE', headers: { 'X-CSRF-Token': csrfToken } };
  return exchange(request, SESSION_PATH, init, timeoutMs, async (response) => {
    if (response.status !== 204) {
      throw new ApiError('failed', response.status);
    }
  });
}

// Automatic polls send X-Relay-Poll so the server does not extend the idle deadline.
export function getQuery(request, path, { poll, timeoutMs }) {
  const headers = poll ? { 'X-Relay-Poll': '1' } : {};
  return exchange(request, path, { method: 'GET', headers }, timeoutMs, readJson);
}

// Terminate succeeds only on 200. Other outcomes reject with their status and error code. Never resent here.
export function terminateSession(request, id, { csrfToken, timeoutMs }) {
  const init = { method: 'POST', headers: { 'X-CSRF-Token': csrfToken } };
  return exchange(request, `${sessionPath(id)}/terminate`, init, timeoutMs, readJson);
}

export function getConfig(request, { timeoutMs }) {
  return getQuery(request, '/api/v1/config', { poll: false, timeoutMs });
}

export function patchConfig(request, { csrfToken, timeoutMs, revision, changes }) {
  const init = {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ revision, changes }),
  };
  return exchange(request, '/api/v1/config', init, timeoutMs, readJson);
}
