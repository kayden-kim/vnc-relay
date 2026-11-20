// SCR-008 block and unblock of one address (dashboard.md SCR-008 "주소 차단과 해제", 5.5). No DOM here.
// The deny list changes through the config API: read the current list and the revision, then send the whole list.
import { getConfig, isUnauthorized, patchConfig } from './api.js';
import { DENY_KEY, canEdit, parseConfig } from './settings-model.js';
import { listForBlock, listForUnblock } from './sources-model.js';

function notice(key, tone, extra = {}) {
  return { kind: 'notice', key, tone, ...extra };
}

// The failed PATCH as a notice (dashboard.md 5.5). A rejection that leaves the list as it was is not a success and never
// resent; a result that cannot be told asks for one more sources query.
function patchNotice(error) {
  if (isUnauthorized(error)) return { kind: 'unauthorized' };
  const { status = null, code = null } = error ?? {};
  if (error?.kind !== 'failed' || status === null || status >= 500) return notice('denyUnknown', 'warning', { requery: true });
  if (status === 403 && code === 'config_edit_disabled') return notice('configEditDisabled', 'neutral', { requery: true });
  if (status === 409 && (code === 'config_restart_pending' || code === 'config_file_invalid')) {
    return notice('denyFileState', 'warning');
  }
  if (status === 409 && code === 'config_conflict') return notice('denyConflict', 'warning');
  if (status === 400 && code === 'config_invalid') {
    const diagnostics = Array.isArray(error.details?.diagnostics) ? error.details.diagnostics : [];
    return notice('denyInvalid', 'destructive', { diagnostics, fallback: error.details?.message ?? null });
  }
  return notice('denyRequestFailed', 'destructive');
}

/**
 * One block or unblock. `action` is 'block' (address `address`) or 'unblock' (the row's `entries`).
 * Resolves to `{kind: 'done'}`, `{kind: 'nothing'}` (unblock: none of the entries is in the current list, no PATCH),
 * `{kind: 'unauthorized'}`, or `{kind: 'notice', key, tone, requery, diagnostics?}`. Never rejects and never resends.
 */
export async function changeDenyList(request, { csrfToken, timeoutMs, action, address, entries }) {
  let config;
  try {
    config = parseConfig(await getConfig(request, { timeoutMs }));
  } catch (error) {
    if (isUnauthorized(error)) return { kind: 'unauthorized' };
    return notice('denyConfigReadFailed', 'destructive');
  }
  if (!config.edit_enabled) return notice('configEditDisabled', 'neutral', { requery: true });
  const item = config.items.find((entry) => entry.key === DENY_KEY);
  if (!item || !Array.isArray(item.value) || !item.value.every((entry) => typeof entry === 'string')) {
    return notice('denyConfigReadFailed', 'destructive');
  }
  if (!canEdit(config) || !item.editable) return notice('denyFileState', 'warning');

  const next = action === 'block' ? listForBlock(item.value, address) : listForUnblock(item.value, entries);
  if (next === null) return action === 'block' ? { kind: 'done' } : { kind: 'nothing' };
  try {
    const result = await patchConfig(request, {
      csrfToken, timeoutMs, revision: config.revision, changes: { [DENY_KEY]: next },
    });
    if (result?.result === 'applied' && Array.isArray(result.applied)) return { kind: 'done' };
    return notice('denyUnknown', 'warning', { requery: true });
  } catch (error) {
    return patchNotice(error);
  }
}
