// Config input conversion preserves source line numbers and compares only editable values.
import { fill } from './format.js';

export function parseConfig(body) {
  if (!body || typeof body.as_of !== 'string' || !Array.isArray(body.items) ||
      typeof body.edit_enabled !== 'boolean' || !body.file || !Array.isArray(body.file.config_keys) ||
      !Array.isArray(body.file.diagnostics)) throw new TypeError('invalid config response');
  for (const item of body.items) {
    if (!item || typeof item.key !== 'string' || typeof item.reload !== 'string' ||
        typeof item.editable !== 'boolean' || !('value' in item)) throw new TypeError('invalid config item');
  }
  return body;
}

export function canEdit(config) {
  return config?.edit_enabled && typeof config.revision === 'string' &&
    ['in_effect', 'pending'].includes(config.file.state);
}

export function fieldGroup(key) {
  if (key.startsWith('relay.ticket.')) return 'accessTickets';
  if (/^relay\..+\.allow$/.test(key)) return 'sourceAllowLists';
  if (key.startsWith('relay.') || key.startsWith('mode1.')) return 'accessPolicies';
  if (key.startsWith('limits.')) return 'connectionLimits';
  if (key.startsWith('timeouts.')) return 'timeouts';
  return 'keepalive';
}

// DASH-035: six sections; access tickets follow keepalive in contract order.
export function editableItems(config) {
  const rank = (key) => {
    const group = fieldGroup(key);
    if (group === 'sourceAllowLists') return 0;
    if (group === 'accessPolicies') return key.endsWith('.trusted_proxies') ? 1
      : key.endsWith('.allowed_origins') ? 2 : 3;
    return { connectionLimits: 4, timeouts: 5, keepalive: 6, accessTickets: 7 }[group];
  };
  return config.items.filter((item) => item.editable).sort((a, b) => rank(a.key) - rank(b.key));
}

export function fieldUnit(key) {
  if (key.endsWith('_secs')) return 'seconds';
  if (key.endsWith('_bytes')) return 'bytes';
  return 'count';
}

export function initialInputs(config) {
  return new Map(editableItems(config).map((item) =>
    [item.key, Array.isArray(item.value) ? item.value.join('\n') :
      typeof item.value === 'boolean' ? item.value : String(item.value ?? '')]));
}

export function diagnosticText(diagnostic, t) {
  return t.configDiagnostics[diagnostic.code] ?? `${diagnostic.message} (${diagnostic.code})`;
}

export function listLines(text) {
  const values = [];
  const lines = [];
  text.split('\n').forEach((value, index) => {
    if (value !== '') {
      values.push(value);
      lines.push(index + 1);
    }
  });
  return { values, lines };
}

// Error entries retain message identifiers/data, not translated text, so language changes preserve state.
export function inspectInputs(config, inputs) {
  const values = {};
  const changes = {};
  const errors = [];
  const lineMaps = new Map();
  for (const item of editableItems(config)) {
    const text = inputs.get(item.key);
    if (Array.isArray(item.value)) {
      const list = listLines(text);
      values[item.key] = list.values;
      lineMaps.set(item.key, list.lines);
      list.values.forEach((value, index) => {
        if (/\s/u.test(value)) errors.push({ key: item.key, line: list.lines[index], copy: 'listWhitespaceError' });
      });
    } else if (typeof item.value === 'boolean') {
      values[item.key] = text;
    } else if (typeof item.value === 'string' || item.value === null) {
      if (item.value === null && text === '') continue;
      values[item.key] = text;
      const bytes = new TextEncoder().encode(text).length;
      if (bytes < 1 || bytes > 256) errors.push({ key: item.key, copy: 'stringLengthError' });
    } else if (!/^-?\d+$/.test(text)) {
      errors.push({ key: item.key, copy: 'integerRequired' });
    } else {
      const value = Number(text);
      values[item.key] = value;
      if (!Number.isSafeInteger(value) || value < item.min || value > item.max) {
        errors.push({ key: item.key, copy: 'integerRange', min: item.min, max: item.max });
      }
    }
    if (Object.hasOwn(values, item.key) && JSON.stringify(values[item.key]) !== JSON.stringify(item.value)) {
      changes[item.key] = values[item.key];
    }
  }
  if (values['limits.waiting'] > values['limits.relay_connections']) {
    errors.push({ key: 'limits.waiting', copy: 'waitingLimitError' });
  }
  const order = new Map(editableItems(config).map((item, index) => [item.key, index]));
  errors.sort((a, b) => order.get(a.key) - order.get(b.key));
  return { changes, values, errors, lineMaps };
}

export function mapDiagnostics(diagnostics, config, lineMaps) {
  const keys = editableItems(config).map((item) => item.key);
  return diagnostics.map((diagnostic) => {
    const match = /^(.*)\[(\d+)\]$/.exec(diagnostic.key);
    const key = match?.[1] ?? diagnostic.key;
    return {
      key, diagnostic,
      line: match ? lineMaps.get(key)?.[Number(match[2])] : undefined,
      field: keys.includes(key),
    };
  }).sort((a, b) => {
    const ai = keys.indexOf(a.key);
    const bi = keys.indexOf(b.key);
    return (ai < 0 ? Infinity : ai) - (bi < 0 ? Infinity : bi);
  });
}

export function inputErrorText(error, t) {
  if (error.diagnostic) {
    const diagnostic = diagnosticText(error.diagnostic, t);
    return error.line ? fill(t.configLineDiagnostic, { line: error.line, diagnostic }) : diagnostic;
  }
  return fill(t[error.copy], error);
}

export function selectErrorLine(input, line) {
  input.focus();
  if (!line || input.tagName !== 'TEXTAREA') return;
  const lines = input.value.split('\n');
  const start = lines.slice(0, line - 1).reduce((sum, value) => sum + value.length + 1, 0);
  input.setSelectionRange(start, start + lines[line - 1].length);
}
