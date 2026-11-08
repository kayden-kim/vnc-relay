// SCR-004/SCR-005: entry/manual-only config reads, explicit review/apply, and recoverable edits.
import { BOOTSTRAP_TIMEOUT_MS, getConfig, getSession, isUnauthorized, patchConfig } from './api.js';
import { createAlert } from './components/alert.js';
import { createBadge } from './components/badge.js';
import { createButton } from './components/button.js';
import { createCard } from './components/card.js';
import { createDialog } from './components/dialog.js';
import { icon } from './components/icons.js';
import { BRAND_NAME, COPY } from './copy.js';
import { fill, formatTime } from './format.js';
import { LANGUAGES, chooseLanguage, readSavedLanguage, saveLanguage } from './language.js';
import { parseSession } from './model.js';
import { el, pageHead, statusAlert, statusBadge } from './panels.js';
import { LOGIN_URL, buildNavbar, defaultStorage, signOutUrl } from './shell.js';
import {
  canEdit, diagnosticText, editableItems, fieldGroup, fieldUnit, initialInputs, inputErrorText,
  inspectInputs, mapDiagnostics, parseConfig, selectErrorLine,
} from './settings-model.js';

const GROUPS = ['sourceAllowLists', 'accessPolicies', 'connectionLimits', 'timeouts', 'keepalive'];
const FILE_COPY = {
  in_effect: 'configInEffect', pending: 'configPending', restart_required: 'configRestartRequired',
  invalid: 'configInvalid', unreadable: 'configUnreadable',
};

export function mountSettings(root, deps = {}, { edit = false, applied = null } = {}) {
  const rawRequest = deps.request ?? ((path, init) => globalThis.fetch(path, init));
  const storage = deps.storage === undefined ? defaultStorage() : deps.storage;
  const navigate = deps.navigate ?? ((url) => globalThis.location.assign(url));
  const preferred = deps.preferredLanguages ?? globalThis.navigator?.languages ?? [];
  const lifecycle = new AbortController();
  const request = (path, init) => rawRequest(path, {
    ...init, signal: AbortSignal.any([lifecycle.signal, init.signal]),
  });
  const state = {
    lang: chooseLanguage(readSavedLanguage(storage), preferred), epoch: 0, ended: false,
    config: null, inputs: new Map(), errors: [], lineMaps: new Map(), changes: {},
    csrfToken: null, timeoutMs: null, loading: true, failed: false,
    saving: false, logoutBusy: false, notice: null, dialogOpen: false,
  };
  let page = null;
  let dialog = null;
  const t = () => COPY[state.lang];

  function dispose() {
    state.ended = true;
    state.epoch += 1;
    lifecycle.abort();
    dialog?.hide();
    dialog?.element.remove();
    dialog = null;
    state.config = null;
    state.inputs.clear();
    state.changes = {};
    state.errors = [];
    state.csrfToken = null;
    page = null;
  }

  function leave(url, options) {
    dispose();
    root.replaceChildren();
    navigate(url, options);
  }

  function valueNode(value) {
    const text = t();
    if (value === null) return el('span', '', text.noValue);
    if (Array.isArray(value)) {
      const list = el('div', 'config-value-list');
      if (!value.length) list.append(el('span', '', text.emptyList));
      for (const item of value) list.append(el('div', 'config-data', item));
      return list;
    }
    return el('span', typeof value === 'string' ? 'config-data' : 'config-number', String(value));
  }

  function keyList(keys) {
    const list = el('div', 'config-value-list');
    for (const key of keys) list.append(el('code', 'config-data', key));
    return list;
  }

  function alert(message, tone = 'neutral', title) {
    return createAlert({ tone, title: title ?? message, ...(title ? { message } : {}) });
  }

  function fileNotice() {
    const config = state.config;
    const kind = config.file.state;
    const box = alert(t()[FILE_COPY[kind]] ?? kind,
      kind === 'pending' || kind === 'restart_required' ? 'warning' :
      kind === 'invalid' || kind === 'unreadable' ? 'destructive' : 'neutral', t().configFileTitle);
    box.querySelector('.alert-body').append(keyList(config.file.config_keys));
    const diagnostics = el('div', 'config-diagnostics');
    for (const diagnostic of config.file.diagnostics) {
      const row = el('div', 'config-diagnostic');
      row.append(el('code', 'config-data', diagnostic.key), el('p', '', diagnosticText(diagnostic, t())));
      diagnostics.append(row);
    }
    box.querySelector('.alert-body').append(diagnostics);
    return box;
  }

  function reloadCard() {
    const body = el('div', 'config-body');
    const reload = state.config.last_reload;
    if (!reload) body.append(el('p', '', t().noReloadHistory));
    else {
      const labels = t().reloadLabels;
      const list = el('dl', 'config-reload');
      const values = [formatTime(reload.at), t().reloadResults[reload.result] ?? reload.result,
        t().reloadTriggers[reload.trigger] ?? reload.trigger, keyList(reload.config_keys)];
      values.forEach((value, i) => {
        const row = el('div', 'config-reload-row');
        const dd = el('dd', i === 0 ? 'config-help' : '');
        dd.append(value instanceof Node ? value : document.createTextNode(value));
        row.append(el('dt', 'config-help', labels[i]), dd);
        list.append(row);
      });
      body.append(list);
    }
    return createCard({ title: t().lastReloadTitle, body });
  }

  function viewCards() {
    const groups = new Map();
    for (const item of state.config.items) {
      const table = item.key.slice(0, item.key.lastIndexOf('.'));
      if (!groups.has(table)) groups.set(table, []);
      groups.get(table).push(item);
    }
    const cards = [];
    for (const [table, items] of groups) {
      const list = el('dl', 'config-list');
      list.setAttribute('aria-label', t().configColumns.join(' / '));
      for (const item of items) {
        const row = el('div', 'config-row');
        row.dataset.configKey = item.key;
        const value = el('dd', 'config-value');
        value.append(valueNode(item.value));
        const application = el('dd', 'config-application');
        application.append(createBadge({ tone: item.reload === 'dynamic' ? 'info' : 'neutral',
          label: item.reload === 'dynamic' ? t().configDynamic : t().configRestart,
          ...(item.reload === 'dynamic' ? {} : { icon: false }) }));
        if (state.config.file.config_keys.includes(item.key)) {
          application.append(createBadge({ tone: 'warning', label: t().configDiffers }));
        }
        row.append(el('dt', 'config-data', item.key), value, application);
        list.append(row);
      }
      cards.push(createCard({ title: `[${table}]`, body: list }));
    }
    return cards;
  }

  function field(item) {
    const wrapper = el('div', 'text-field config-field');
    const id = `config-${item.key.replaceAll('.', '-')}`;
    const label = el('label', 'config-data', item.key);
    label.htmlFor = id;
    const input = el(Array.isArray(item.value) ? 'textarea' : 'input', 'text-field-input config-input');
    input.id = id;
    input.dataset.configKey = item.key;
    input.value = state.inputs.get(item.key);
    input.disabled = state.saving;
    if (input.tagName === 'TEXTAREA') {
      input.classList.add('config-list-input');
      input.rows = 3;
      input.spellcheck = false;
    } else {
      // Text plus numeric keyboard retains invalid decimal/exponent input for explicit client validation.
      input.type = 'text';
      input.inputMode = 'numeric';
      input.autocomplete = 'off';
    }
    input.addEventListener('input', () => state.inputs.set(item.key, input.value));
    const help = el('p', 'config-help', Array.isArray(item.value) ? t().configListHelp :
      fill(t().configRangeHelp, { min: item.min, max: item.max }));
    help.id = `${id}-help`;
    let proxyHelp = null;
    if (item.key.endsWith('.trusted_proxies')) {
      proxyHelp = el('p', 'config-help', t().trustedProxiesHelp);
      proxyHelp.id = `${id}-proxy-help`;
    }
    const errors = state.errors.filter((error) => error.key === item.key && error.field !== false);
    const errorNode = el('div', 'config-field-errors');
    errorNode.id = `${id}-error`;
    for (const error of errors) {
      const message = el('p', 'config-field-error');
      message.append(icon('alert'), document.createTextNode(inputErrorText(error, t())));
      errorNode.append(message);
    }
    input.setAttribute('aria-invalid', String(errors.length > 0));
    const describedBy = [help.id, proxyHelp?.id, errors.length ? errorNode.id : null].filter(Boolean).join(' ');
    input.setAttribute('aria-describedby', describedBy);
    const control = el('div', 'config-number-unit');
    control.append(input);
    if (!Array.isArray(item.value)) control.append(el('span', 'config-help', t().configUnits[fieldUnit(item.key)]));
    wrapper.append(label, control, help, ...(proxyHelp ? [proxyHelp] : []), errorNode);
    return wrapper;
  }

  function editCards() {
    const cards = [];
    const editable = editableItems(state.config);
    for (const group of GROUPS) {
      const items = editable.filter((item) => fieldGroup(item.key) === group);
      if (!items.length) continue;
      const fields = el('div', 'settings-fields');
      fields.append(...items.map(field));
      cards.push(createCard({ title: t()[group], body: fields }));
    }
    return cards;
  }

  function button(id, label, handler, variant = 'outline', disabled = false) {
    const node = createButton({ label, variant, disabled });
    node.id = id;
    node.addEventListener('click', handler);
    return node;
  }

  function render() {
    if (state.ended) return;
    const focus = document.activeElement;
    const focusId = root.contains(focus) ? focus.id : null;
    const selection = focus?.tagName === 'TEXTAREA' ? [focus.selectionStart, focus.selectionEnd] : null;
    dialog?.hide();
    dialog?.element.remove();
    dialog = null;
    document.documentElement.lang = state.lang;
    document.title = `${BRAND_NAME} ${edit ? t().settingsEditTitle : t().settingsTitle}`;
    const navbar = buildNavbar({ t: t(), lang: state.lang, active: 'settings', idPrefix: 'settings',
      onLanguageChange, onSignOut: signOut });
    const refresh = button('settings-refresh', t().refresh, () => void load(), 'outline',
      state.loading || state.saving || state.logoutBusy);
    const editable = canEdit(state.config) && !state.failed && !state.loading;
    const actions = edit ?
      [refresh, button('settings-view', t().settingsTitle, () => leave('/#/settings'))] :
      [refresh, button('settings-edit', t().edit, () => leave('/#/settings/edit'), 'primary', !editable)];
    const header = pageHead({ title: edit ? t().settingsEditTitle : t().settingsTitle, actions });
    header.meta.textContent = fill(t().settingsRetrieved, {
      time: state.config ? formatTime(state.config.as_of) : t().unavailable,
    });
    header.head.querySelector('h1').tabIndex = -1;
    header.head.querySelector('h1').id = 'settings-title';
    const main = el('main', 'app-main settings-main');
    main.append(header.head);
    if (applied !== null && !edit) main.append(alert(fill(t().settingsApplied, { n: applied }), 'success'));
    if (state.loading) main.append(statusBadge(t(), state.config ? 'refreshing' : 'loading'));
    if (state.failed) main.append(statusAlert(t(), state.config ? 'stale' : 'failed', state.config?.as_of));
    if (state.saving) main.append(alert(t().settingsSaving, 'info'));
    if (state.notice) {
      main.append(alert(t()[state.notice], ['settingsConflict', 'settingsUnknown'].includes(state.notice) ?
        'warning' : 'destructive'));
    }
    if (state.errors.length) {
      main.append(alert(fill(t().settingsErrorSummary, { n: state.errors.length }), 'destructive'));
      if (state.errors.some((error) => error.diagnostic)) main.append(alert(t().settingsValidationFailed, 'destructive'));
      for (const error of state.errors.filter((entry) => entry.field === false)) {
        const box = alert(inputErrorText(error, t()), 'destructive');
        box.append(el('code', 'config-data', error.diagnostic.key));
        main.append(box);
      }
    }
    if (state.config) {
      if (!edit || (state.config.edit_enabled && !canEdit(state.config)) || state.config.file.state === 'pending') main.append(fileNotice());
      if (!state.config.edit_enabled) main.append(alert(t().configEditDisabled));
      if (!edit) main.append(reloadCard(), ...viewCards());
      else if (canEdit(state.config)) {
        main.append(alert(t().settingsApplyTiming, 'info'), ...editCards());
      }
    }
    if (edit) {
      const controls = el('div', 'page-actions settings-actions');
      controls.append(button('settings-cancel', t().cancel, () => leave('/#/settings')));
      if (state.notice === 'settingsConflict' || state.notice === 'settingsRejected' ||
          (state.config && !canEdit(state.config))) {
        controls.append(button('settings-reload', t().reloadSettings, () => void load({ renew: true }),
          'outline', state.loading || state.saving));
      }
      if (canEdit(state.config)) controls.append(button('settings-review', t().reviewChanges, review,
        'primary', !editable || state.saving || state.logoutBusy ||
          ['settingsUnknown', 'settingsConflict', 'settingsRejected'].includes(state.notice)));
      main.append(controls);
    }
    root.replaceChildren(navbar.navbar, main);
    page = { ...navbar, main };
    if (focusId) {
      const next = document.getElementById(focusId);
      if (next && !next.disabled) {
        next.focus();
        if (selection && next.tagName === 'TEXTAREA') next.setSelectionRange(...selection);
      }
    }
    if (state.dialogOpen) showReview();
  }

  function focusFirstError() {
    const error = state.errors.find((entry) => entry.field !== false);
    if (!error) return;
    const input = [...root.querySelectorAll('input[data-config-key], textarea[data-config-key]')]
      .find((node) => node.dataset.configKey === error.key);
    if (input) selectErrorLine(input, error.line);
  }

  function review() {
    if (state.saving || state.loading || state.failed || !canEdit(state.config) ||
        ['settingsUnknown', 'settingsConflict', 'settingsRejected'].includes(state.notice)) return;
    const inspection = inspectInputs(state.config, state.inputs);
    state.errors = inspection.errors;
    state.lineMaps = inspection.lineMaps;
    state.changes = inspection.changes;
    if (state.errors.length) {
      render();
      focusFirstError();
    } else {
      state.dialogOpen = true;
      showReview();
    }
  }

  function showReview() {
    dialog?.hide();
    dialog?.element.remove();
    const reviewButton = document.getElementById('settings-review');
    dialog = createDialog({ title: t().settingsConfirmTitle, description: t().settingsApplyTiming,
      confirmLabel: t().apply, cancelLabel: t().cancel, returnFocus: reviewButton,
      onConfirm: () => { state.dialogOpen = false; void save(); },
      onCancel: () => { state.dialogOpen = false; },
    });
    dialog.element.classList.add('config-dialog');
    const body = el('div', 'config-confirm-body');
    if (!Object.keys(state.changes).length) body.append(alert(t().noSettingsChanges, 'info'));
    for (const [key, value] of Object.entries(state.changes)) {
      const previous = state.config.items.find((item) => item.key === key).value;
      const content = el('div', 'config-body');
      t().settingsComparisonLabels.forEach((label, index) => {
        content.append(el('p', 'config-help', label), valueNode(index ? value : previous));
      });
      if (Array.isArray(value)) {
        content.append(el('p', 'config-help', t().addedItems), valueNode(value.filter((entry) => !previous.includes(entry))),
          el('p', 'config-help', t().removedItems), valueNode(previous.filter((entry) => !value.includes(entry))));
        if (!value.length) {
          const warning = key.endsWith('.trusted_proxies')
            ? t().emptyTrustedProxiesWarning
            : key.endsWith('.allowed_origins')
              ? t().emptyAllowedOriginsWarning
              : t().emptyAllowWarning;
          content.append(alert(warning, 'warning'));
        }
      }
      const card = createCard({ title: key, headingLevel: 3, body: content });
      card.classList.add('config-change');
      body.append(card);
    }
    dialog.element.insertBefore(body, dialog.element.querySelector('.dialog-actions'));
    root.append(dialog.element);
    dialog.show();
    dialog.element.querySelector('.dialog-actions button').focus();
  }

  async function load({ renew = false } = {}) {
    if (state.ended || state.saving || state.logoutBusy || (state.loading && state.timeoutMs !== null)) return;
    const epoch = state.epoch;
    state.loading = true;
    state.failed = false;
    state.dialogOpen = false;
    render();
    try {
      if (renew || state.timeoutMs === null) {
        const session = parseSession(await getSession(request, { timeoutMs: BOOTSTRAP_TIMEOUT_MS }));
        if (epoch !== state.epoch) return;
        state.csrfToken = session.csrfToken;
        state.timeoutMs = session.queryTimeoutMs;
      }
      const config = parseConfig(await getConfig(request, { timeoutMs: state.timeoutMs }));
      if (epoch !== state.epoch) return;
      state.config = config;
      state.inputs = initialInputs(config);
      state.errors = [];
      state.changes = {};
      state.notice = null;
    } catch (error) {
      if (epoch !== state.epoch) return;
      if (isUnauthorized(error)) { leave(LOGIN_URL); return; }
      state.failed = true;
    } finally {
      if (epoch === state.epoch) { state.loading = false; render(); }
    }
  }

  async function save() {
    if (state.saving || state.ended) return;
    if (!Object.keys(state.changes).length) {
      leave('/#/settings', { applied: 0 });
      return;
    }
    state.saving = true;
    state.notice = null;
    state.errors = [];
    const epoch = state.epoch;
    render();
    try {
      const result = await patchConfig(request, { csrfToken: state.csrfToken, timeoutMs: state.timeoutMs,
        revision: state.config.revision, changes: state.changes });
      if (epoch !== state.epoch) return;
      if (result?.result === 'applied' && Array.isArray(result.applied)) {
        leave('/#/settings', { applied: result.applied.length });
        return;
      }
      state.notice = 'settingsUnknown';
    } catch (error) {
      if (epoch !== state.epoch) return;
      if (isUnauthorized(error)) { leave(LOGIN_URL); return; }
      const code = error.code;
      if (error.status === 400 && code === 'config_invalid') {
        state.errors = mapDiagnostics(error.details?.diagnostics ?? [], state.config, state.lineMaps);
        if (!state.errors.length) state.notice = 'settingsValidationFailed';
      } else if (error.status === 409 && code === 'config_conflict') state.notice = 'settingsConflict';
      else if (error.status === 500 && code === 'config_write_failed') state.notice = 'settingsWriteFailed';
      else if (error.status === 403 && code === 'config_edit_disabled') {
        state.config = { ...state.config, edit_enabled: false };
        state.inputs.clear();
      } else if (error.status === 409 && code === 'config_restart_pending') {
        state.config = { ...state.config, file: { state: 'restart_required',
          config_keys: error.details?.config_keys ?? [], diagnostics: [] } };
        state.inputs.clear();
      } else if (error.status === 409 && code === 'config_file_invalid') {
        const diagnostics = error.details?.diagnostics ?? [];
        state.config = { ...state.config, file: { state: diagnostics.length ? 'invalid' : 'unreadable',
          config_keys: [], diagnostics } };
        state.inputs.clear();
        if (!diagnostics.length) {
          // A PATCH with no diagnostics cannot distinguish file access from other file errors.
          try {
            const config = parseConfig(await getConfig(request, { timeoutMs: state.timeoutMs }));
            if (epoch !== state.epoch) return;
            state.config = config;
            state.inputs = initialInputs(config);
          } catch (readError) {
            if (epoch !== state.epoch) return;
            if (isUnauthorized(readError)) { leave(LOGIN_URL); return; }
            state.failed = true;
          }
        }
      } else if (error.status >= 400 && error.status < 500) state.notice = 'settingsRejected';
      else state.notice = 'settingsUnknown';
    } finally {
      if (epoch === state.epoch) {
        state.saving = false;
        render();
        focusFirstError();
      }
    }
  }

  function onLanguageChange(lang) {
    if (!LANGUAGES.includes(lang) || state.lang === lang) return;
    state.lang = lang;
    saveLanguage(storage, lang);
    render();
    page.select.focus();
  }

  async function signOut() {
    if (state.ended || state.logoutBusy) return;
    state.logoutBusy = true;
    const token = state.csrfToken;
    // Hide data and abort reads/writes before the sign-out exchange, even if its response is lost.
    dispose();
    root.replaceChildren();
    navigate(await signOutUrl(rawRequest, token));
  }

  render();
  if (applied !== null && !edit) document.getElementById('settings-title').focus();
  void load();
  return { dispose };
}
