// Component test page. Each check calls the shared component API and records pass or fail.
// All work is synchronous. The result is written to data-result in finally, so it is never left as running.
import { icon } from "../app/components/icons.js";
import { createButton } from "../app/components/button.js";
import { createTextField } from "../app/components/text-field.js";
import { createBadge } from "../app/components/badge.js";
import { createAlert } from "../app/components/alert.js";
import { createCard } from "../app/components/card.js";
import { createStat } from "../app/components/stat.js";
import { createGauge } from "../app/components/gauge.js";
import { createNavbar } from "../app/components/navbar.js";
import { createLanguageSelect } from "../app/components/language-select.js";
import { createDataTable } from "../app/components/data-table.js";
import { createLink } from "../app/components/link.js";
import { createDialog } from "../app/components/dialog.js";

// Count CSP violations. Registered at module top so no violation is missed.
let cspViolations = 0;
document.addEventListener("securitypolicyviolation", () => {
  cspViolations += 1;
});

const lang = new URLSearchParams(location.search).get("lang") === "en" ? "en" : "ko";
document.documentElement.lang = lang;

const STRINGS = {
  ko: {
    title: "컴포넌트 테스트",
    signIn: "로그인",
    busy: "처리 중",
    refresh: "새로고침",
    logout: "로그아웃",
    ok: "정상",
    fieldLabel: "관리자 ID",
    help: "로그인에 쓰는 ID입니다.",
    fieldError: "ID가 맞지 않습니다.",
    alertTitle: "조회 실패",
    alertMessage: "잠시 후 다시 시도하세요.",
    cardTitle: "활성 세션",
    cardBody: "세션 목록을 확인하세요.",
    cardFooter: "마지막 갱신 10:42",
    statLabel: "활성 세션",
    unit: "건",
    gauge: "사용량",
    gaugeLimit: "상한 100",
    gaugeUnknown: "측정 없음",
    navLabel: "주 메뉴",
    navItemA: "운영 현황",
    navItemB: "설정",
    languageLabel: "언어 (Language)",
    tableCaption: "활성 세션 목록",
    colSession: "세션",
    colMode: "모드",
    colBytes: "바이트",
    empty: "활성 세션이 없습니다.",
    linkLabel: "세부 정보",
    dialogTitle: "세션을 종료할까요?",
    dialogDescription: "종료한 세션은 되돌릴 수 없습니다.",
    confirm: "확인",
    cancel: "취소",
  },
  en: {
    title: "Component test",
    signIn: "Sign in",
    busy: "Working",
    refresh: "Refresh",
    logout: "Sign out",
    ok: "OK",
    fieldLabel: "Admin ID",
    help: "The ID used to sign in.",
    fieldError: "The ID does not match.",
    alertTitle: "Query failed",
    alertMessage: "Try again later.",
    cardTitle: "Active sessions",
    cardBody: "Check the session list.",
    cardFooter: "Last updated 10:42",
    statLabel: "Active sessions",
    unit: "items",
    gauge: "Usage",
    gaugeLimit: "Limit 100",
    gaugeUnknown: "No reading",
    navLabel: "Main menu",
    navItemA: "Operations",
    navItemB: "Settings",
    languageLabel: "Language (언어)",
    tableCaption: "Active session list",
    colSession: "Session",
    colMode: "Mode",
    colBytes: "Bytes",
    empty: "No active sessions.",
    linkLabel: "Details",
    dialogTitle: "End the session?",
    dialogDescription: "An ended session cannot be restored.",
    confirm: "Confirm",
    cancel: "Cancel",
  },
};
const t = STRINGS[lang];

const TONES = ["success", "info", "warning", "destructive", "neutral"];
const BUTTON_VARIANTS = ["primary", "outline", "ghost", "destructive", "navbar"];
const ICON_NAMES = [
  "check", "clock", "warning", "alert", "refresh", "logout", "back",
  "chevron-right", "chevron-down", "cancel", "globe", "grid", "settings", "brand",
  "help", "external-link",
];
const LANGUAGE_OPTIONS = [
  { value: "ko", label: "한국어" },
  { value: "en", label: "English" },
];
const TABLE_COLUMNS = [
  { key: "session", label: t.colSession, mobileHead: "primary" },
  { key: "mode", label: t.colMode, mobileHead: "secondary" },
  { key: "bytes", label: t.colBytes },
];
const TABLE_ROWS = [
  ["S-1001", "Mode I", "12,288"],
  ["S-1002", "Mode II", "4,096"],
];
const LONG_KO_LABEL = "선택한 세션을 종료하고 중계 자원을 모두 정리합니다";
const LONG_EN_LABEL = "End the selected session and release every relay resource";
const LONG_ID = "relay-session-0123456789abcdef0123456789abcdef0123456789";
const LONG_NUMBER = "18,446,744,073,709,551,615";

const main = document.getElementById("result");
const cases = document.getElementById("cases");
const results = [];
const failures = [];

function fail(id) {
  if (!failures.includes(id)) failures.push(id);
}

// Runs one check. A throw from the check marks only that check as failed.
function check(id, fn) {
  try {
    fn();
    results.push({ id, ok: true });
  } catch {
    results.push({ id, ok: false });
    fail(id);
  }
}

function expect(condition) {
  if (!condition) throw new Error("expectation failed");
}

function throwsTypeError(fn) {
  try {
    fn();
  } catch (error) {
    return error instanceof TypeError;
  }
  return false;
}

// Adds a rendered element to the page so the check can see it.
function mount(element) {
  cases.append(element);
  return element;
}

function paragraph(text) {
  const p = document.createElement("p");
  p.textContent = text;
  return p;
}

function makeNavbar(sid, extra) {
  return createNavbar({
    brand: "VNC Relay",
    items: [
      { label: t.navItemA, href: `#${sid}-a`, current: true, icon: "grid" },
      { label: t.navItemB, href: `#${sid}-b`, icon: "settings" },
    ],
    language: createLanguageSelect({
      id: `${sid}-language`,
      label: t.languageLabel,
      value: lang,
      options: LANGUAGE_OPTIONS,
      variant: "navbar",
    }),
    action: createButton({ variant: "navbar", label: t.logout, icon: "logout" }),
    label: t.navLabel,
    ...extra,
  });
}

function runChecks() {
  check("create-button", () => {
    for (const variant of BUTTON_VARIANTS) {
      const button = mount(createButton({ variant, label: t.signIn }));
      expect(button instanceof HTMLButtonElement);
      expect(button.textContent.includes(t.signIn));
    }
    const iconButton = mount(createButton({ variant: "outline", label: t.refresh, icon: "refresh" }));
    expect(iconButton.querySelector("svg") !== null);
  });

  check("button-busy", () => {
    const button = mount(createButton({ variant: "primary", label: t.busy, busy: true }));
    expect(button.disabled === true);
    expect(button.getAttribute("aria-busy") === "true");
  });

  check("invalid-button-variant", () => {
    expect(throwsTypeError(() => createButton({ variant: "unknown", label: t.signIn })));
  });

  check("create-text-field", () => {
    const field = mount(createTextField({
      id: "test-field",
      label: t.fieldLabel,
      type: "text",
      value: "abc",
      required: true,
      autocomplete: "username",
      helpText: t.help,
    }));
    const input = field.querySelector("input");
    const label = field.querySelector("label");
    expect(field instanceof HTMLDivElement);
    expect(input !== null && input.id === "test-field" && input.value === "abc");
    expect(input.required === true && input.getAttribute("autocomplete") === "username");
    expect(label !== null && label.htmlFor === input.id);
    const password = mount(createTextField({ id: "test-password", label: t.fieldLabel, type: "password" }));
    expect(password.querySelector("input").type === "password");
  });

  check("text-field-error", () => {
    const field = mount(createTextField({ id: "test-error", label: t.fieldLabel, error: t.fieldError }));
    const input = field.querySelector("input");
    expect(input.getAttribute("aria-invalid") === "true");
    const describedBy = (input.getAttribute("aria-describedby") || "").split(" ");
    expect(describedBy.some((id) => {
      const el = document.getElementById(id);
      return el !== null && el.textContent.includes(t.fieldError);
    }));
  });

  check("create-badge", () => {
    for (const tone of TONES) {
      const badge = mount(createBadge({ tone, label: t.ok }));
      expect(badge instanceof HTMLSpanElement);
      expect(badge.dataset.tone === tone);
    }
  });

  check("invalid-tone", () => {
    expect(throwsTypeError(() => createBadge({ tone: "unknown", label: t.ok })));
    expect(throwsTypeError(() => createAlert({ tone: "unknown", title: t.alertTitle, message: t.alertMessage })));
  });

  check("create-alert", () => {
    for (const tone of TONES) {
      const alert = mount(createAlert({ tone, title: t.alertTitle, message: t.alertMessage }));
      expect(alert instanceof HTMLDivElement && alert.dataset.tone === tone);
      expect(alert.textContent.includes(t.alertTitle) && alert.textContent.includes(t.alertMessage));
    }
  });

  check("alert-role", () => {
    for (const tone of ["warning", "destructive"]) {
      const alert = createAlert({ tone, title: t.alertTitle, message: t.alertMessage });
      expect(alert.getAttribute("role") === "alert");
    }
    for (const tone of ["success", "info", "neutral"]) {
      const alert = createAlert({ tone, title: t.alertTitle, message: t.alertMessage });
      expect(alert.getAttribute("role") === "status");
    }
  });

  check("create-card", () => {
    const card = mount(createCard({
      title: t.cardTitle,
      description: t.help,
      count: 2,
      actions: createButton({ variant: "outline", label: t.refresh, icon: "refresh" }),
      body: paragraph(t.cardBody),
      footer: paragraph(t.cardFooter),
    }));
    expect(card instanceof HTMLElement && card.tagName === "SECTION");
    const heading = document.getElementById(card.getAttribute("aria-labelledby") || "");
    expect(heading !== null && heading.tagName === "H2" && heading.textContent.includes(t.cardTitle));
    const description = card.querySelector(".card-head .card-description");
    expect(description instanceof HTMLParagraphElement && description.textContent === t.help);
    expect(!!(heading.compareDocumentPosition(description) & Node.DOCUMENT_POSITION_FOLLOWING));
    expect(throwsTypeError(() => createCard({ title: t.cardTitle, description: 1 })));
    expect(createCard({ title: t.cardTitle }).querySelector(".card-description") === null);
    expect(card.textContent.includes(t.refresh) && card.textContent.includes(t.cardBody));
    expect(card.textContent.includes(t.cardFooter));
  });

  check("create-stat", () => {
    const stat = mount(createStat({ label: t.statLabel, value: "1,234", unit: t.unit, tone: "info" }));
    expect(stat instanceof HTMLDivElement);
    expect(stat.textContent.includes("1,234") && stat.textContent.includes(t.statLabel));
  });

  check("create-gauge", () => {
    const gauge = mount(createGauge({
      label: t.gauge, value: 50, max: 100, valueText: "50%", limitText: t.gaugeLimit,
    }));
    expect(gauge instanceof HTMLDivElement && gauge.getAttribute("role") === "progressbar");
    expect(gauge.textContent.includes(t.gaugeLimit));
  });

  check("gauge-aria", () => {
    const gauge = createGauge({
      label: t.gauge, value: 50, max: 100, valueText: "50%", limitText: t.gaugeLimit,
    });
    expect(gauge.getAttribute("aria-label") === t.gauge);
    expect(gauge.getAttribute("aria-valuemin") === "0");
    expect(gauge.getAttribute("aria-valuemax") === "100");
    expect(gauge.getAttribute("aria-valuenow") === "50");
    expect(gauge.getAttribute("aria-valuetext") === `50%, ${t.gaugeLimit}`);
  });

  check("gauge-unknown", () => {
    const gauge = mount(createGauge({
      label: t.gauge, value: null, max: 100, limitText: t.gaugeLimit, unknownText: t.gaugeUnknown,
    }));
    expect(gauge.getAttribute("role") === "progressbar");
    expect(gauge.hasAttribute("aria-valuenow") === false);
    expect(gauge.getAttribute("aria-valuetext") === t.gaugeUnknown);
    expect(gauge.textContent.includes(t.gaugeUnknown));
    const badge = gauge.querySelector(".gauge-center > .badge");
    expect(badge?.dataset.tone === "neutral" && badge.querySelector("svg") === null);
    // While the first query runs the caller passes the info tone: the badge gets the clock icon like status badges.
    const loading = createGauge({
      label: t.gauge, value: null, max: 100, limitText: t.gaugeLimit, unknownText: t.gaugeUnknown, unknownTone: "info",
    });
    const loadingBadge = loading.querySelector(".gauge-center > .badge");
    expect(loadingBadge?.dataset.tone === "info" && loadingBadge.querySelector("svg") !== null);
    expect(loading.getAttribute("aria-valuetext") === t.gaugeUnknown);
  });

  check("invalid-gauge-options", () => {
    // limitText is always required.
    expect(throwsTypeError(() => createGauge({ label: t.gauge, value: 50, max: 100 })));
    // unknownText is required when value is null.
    expect(throwsTypeError(() => createGauge({ label: t.gauge, value: null, max: 100, limitText: t.gaugeLimit })));
    expect(throwsTypeError(() => createGauge({
      label: t.gauge, value: null, max: 100, limitText: t.gaugeLimit, unknownText: t.gaugeUnknown, unknownTone: "loud",
    })));
  });

  check("create-navbar", () => {
    const navbar = mount(makeNavbar("test-nav", { label: t.navLabel }));
    expect(navbar instanceof HTMLElement && navbar.tagName === "HEADER");
    expect(navbar.querySelector("nav") !== null);
    expect(navbar.textContent.includes("VNC Relay"));
  });

  check("navbar-label", () => {
    // The label has no default. createNavbar must reject a missing label.
    expect(throwsTypeError(() => createNavbar({ brand: "VNC Relay", items: [] })));
    const en = makeNavbar("test-nav-en", { label: STRINGS.en.navLabel });
    expect(en.querySelector("nav").getAttribute("aria-label") === "Main menu");
  });

  check("navbar-current", () => {
    const navbar = makeNavbar("test-nav-current", {});
    const current = navbar.querySelectorAll('[aria-current="page"]');
    expect(current.length === 1 && current[0].textContent.includes(t.navItemA));
  });

  check("navbar-items-visible", () => {
    const navbar = mount(makeNavbar("test-nav-visible", {}));
    const items = [...navbar.querySelectorAll(".navbar-item")];
    expect(items.length === 2 && items[0].textContent.includes(t.navItemA) && items[1].textContent.includes(t.navItemB));
    expect(items.every((item) => getComputedStyle(item).display !== "none" && item.getBoundingClientRect().height > 0));
    expect(items.every((item) => item.querySelector('svg[aria-hidden="true"]') !== null));
    if (window.innerWidth < 1024) {
      expect(navbar.querySelector(".navbar-nav").getBoundingClientRect().top >= navbar.querySelector(".navbar-tools").getBoundingClientRect().bottom);
    }
  });

  check("navbar-help-slot", () => {
    const help = createLink({ href: "#test-help", label: "Help", icon: "help", iconSize: "md", newTabText: "opens in a new tab" });
    const navbar = makeNavbar("test-help-slot", { help });
    const tools = navbar.querySelector(".navbar-tools");
    expect(tools.firstElementChild === help);
    expect(help.nextElementSibling.classList.contains("language-select"));
    expect(tools.lastElementChild.tagName === "BUTTON");
  });

  check("create-language-select", () => {
    const form = mount(createLanguageSelect({
      id: "test-language-form", label: t.languageLabel, value: "ko", options: LANGUAGE_OPTIONS, variant: "form",
    }));
    const control = form.querySelector("select");
    expect(form instanceof HTMLDivElement && control !== null);
    expect(control.value === "ko" && control.options.length === LANGUAGE_OPTIONS.length);
    mount(createLanguageSelect({
      id: "test-language-navbar", label: t.languageLabel, value: "en", options: LANGUAGE_OPTIONS, variant: "navbar",
    }));
  });

  check("language-select-label", () => {
    const select = createLanguageSelect({
      id: "test-language-label", label: t.languageLabel, value: "en", options: LANGUAGE_OPTIONS, variant: "form",
    });
    const control = select.querySelector("select");
    const label = select.querySelector("label");
    expect(control.id === "test-language-label" && control.value === "en");
    expect(label !== null && label.htmlFor === control.id && label.textContent.includes(t.languageLabel));
  });

  check("invalid-language-variant", () => {
    expect(throwsTypeError(() => createLanguageSelect({
      id: "test-bad", label: t.languageLabel, value: "ko", options: LANGUAGE_OPTIONS, variant: "bogus",
    })));
  });

  check("create-data-table", () => {
    const table = mount(createDataTable({
      caption: t.tableCaption, columns: TABLE_COLUMNS, rows: TABLE_ROWS, emptyText: t.empty, numericColumns: ["bytes"],
    }));
    expect(table instanceof HTMLTableElement);
    expect(table.getAttribute("role") === "table");
    expect([...table.querySelectorAll("thead, tbody")].every((group) => group.getAttribute("role") === "rowgroup"));
    expect([...table.querySelectorAll("tr")].every((row) => row.getAttribute("role") === "row"));
    expect([...table.querySelectorAll("td")].every((cell) => cell.getAttribute("role") === "cell"));
    for (const row of table.querySelectorAll("tbody tr")) {
      const [primary, secondary, ordinary] = row.cells;
      expect(primary.classList.contains("data-table-head-primary"));
      expect(secondary.classList.contains("data-table-head-secondary"));
      expect(!ordinary.classList.contains("data-table-head-primary") && !ordinary.classList.contains("data-table-head-secondary"));
      if (window.innerWidth <= 1023) {
        expect(getComputedStyle(row).display === "grid");
        expect(getComputedStyle(primary).gridRowStart === "1" && getComputedStyle(secondary).gridRowStart === "1");
        expect(Math.abs(primary.getBoundingClientRect().top - secondary.getBoundingClientRect().top) < 1);
        for (const head of [primary, secondary]) {
          expect(["none", "normal", '""'].includes(getComputedStyle(head, "::before").content));
        }
        expect(!["none", "normal", '""'].includes(getComputedStyle(ordinary, "::before").content));
      }
    }
    expect(table.caption !== null && table.caption.textContent.includes(t.tableCaption));
    expect(table.querySelectorAll("tbody tr").length === TABLE_ROWS.length);
  });

  check("data-table-header", () => {
    const table = createDataTable({
      caption: t.tableCaption, columns: TABLE_COLUMNS, rows: TABLE_ROWS, emptyText: t.empty, numericColumns: ["bytes"],
    });
    const headers = Array.from(table.querySelectorAll("thead th"));
    expect(headers.length === TABLE_COLUMNS.length);
    expect(headers.every((th, index) =>
      th.getAttribute("scope") === "col" && th.getAttribute("role") === "columnheader"
        && th.textContent.includes(TABLE_COLUMNS[index].label)));
  });

  check("data-table-empty", () => {
    const table = mount(createDataTable({
      caption: t.tableCaption, columns: TABLE_COLUMNS, rows: [], emptyText: t.empty,
    }));
    const rows = table.querySelectorAll("tbody tr");
    expect(rows.length === 1 && rows[0].textContent.includes(t.empty));
    expect(table.getAttribute("role") === "table");
    expect([...table.querySelectorAll("thead, tbody")].every((group) => group.getAttribute("role") === "rowgroup"));
    expect([...table.querySelectorAll("tr")].every((row) => row.getAttribute("role") === "row"));
    expect([...table.querySelectorAll("th")].every((cell) => cell.getAttribute("role") === "columnheader"));
    expect(rows[0].cells[0].getAttribute("role") === "cell");
    expect(rows[0].cells[0].colSpan === TABLE_COLUMNS.length);
    // A Node such as a status badge goes into the empty cell as is.
    const mark = document.createElement("span");
    mark.textContent = t.empty;
    const withNode = createDataTable({ caption: t.tableCaption, columns: TABLE_COLUMNS, rows: [], emptyText: mark });
    expect(withNode.querySelector("tbody tr.data-table-empty td")?.firstChild === mark);
  });

  check("invalid-data-table-mobile-head", () => {
    for (const mobileHead of ["tertiary", "", null, false, 1]) {
      expect(throwsTypeError(() => createDataTable({
        columns: [{ key: "session", label: t.colSession, mobileHead }],
      })));
    }
  });

  check("create-link", () => {
    const link = mount(createLink({ href: "#test-link", label: t.linkLabel, icon: "chevron-right" }));
    expect(link instanceof HTMLAnchorElement && link.getAttribute("href") === "#test-link");
    expect(link.textContent.includes(t.linkLabel));
    expect(!link.hasAttribute("target") && !link.hasAttribute("rel"));
  });

  check("create-new-tab-link", () => {
    const link = mount(createLink({
      href: "#test-help", label: "Help", icon: "help", iconSize: "md", newTabText: "opens in a new tab",
    }));
    expect(link.target === "_blank" && link.rel === "noopener noreferrer");
    expect(link.textContent === "Help (opens in a new tab)");
    expect(link.querySelector(".sr-only").textContent === " (opens in a new tab)");
    const icons = link.querySelectorAll("svg");
    expect(icons.length === 2 && icons[0].classList.contains("icon-md") && icons[1].classList.contains("icon-sm"));
  });

  check("create-icon", () => {
    const row = document.createElement("div");
    for (const name of ICON_NAMES) {
      const svg = icon(name, { size: "md" });
      expect(svg instanceof SVGSVGElement && svg.getAttribute("aria-hidden") === "true");
      expect(svg.getAttribute("focusable") === "false" && svg.getAttribute("viewBox") === "0 0 16 16");
      expect(svg.classList.contains("icon-md"));
      row.append(svg);
    }
    expect(icon("check", { size: "sm" }).classList.contains("icon-sm"));
    mount(row);
  });

  check("invalid-icon-name", () => {
    expect(throwsTypeError(() => icon("not-an-icon", { size: "md" })));
  });

  check("create-dialog", () => {
    const dialog = createDialog({
      title: t.dialogTitle,
      description: t.dialogDescription,
      confirmLabel: t.confirm,
      cancelLabel: t.cancel,
      destructive: true,
      onConfirm: () => {},
      onCancel: () => {},
    });
    expect(dialog.element instanceof HTMLDialogElement);
    expect(typeof dialog.show === "function" && typeof dialog.hide === "function");
  });

  check("dialog-show-hide", () => {
    let confirmed = false;
    const dialog = createDialog({
      title: t.dialogTitle,
      description: t.dialogDescription,
      confirmLabel: t.confirm,
      cancelLabel: t.cancel,
      destructive: true,
      onConfirm: () => {
        confirmed = true;
      },
    });
    // showModal() needs the dialog to be connected to the document.
    mount(dialog.element);
    try {
      expect(dialog.element.open === false);
      dialog.show();
      expect(dialog.element.open === true);
      const confirmButton = Array.from(dialog.element.querySelectorAll("button"))
        .find((button) => button.textContent.includes(t.confirm));
      confirmButton.click();
      expect(confirmed);
    } finally {
      dialog.hide();
    }
    expect(dialog.element.open === false);
  });

  check("dialog-closed-hidden", () => {
    const dialog = createDialog({
      title: t.dialogTitle,
      description: t.dialogDescription,
      confirmLabel: t.confirm,
      cancelLabel: t.cancel,
      destructive: false,
      onConfirm: () => {},
      onCancel: () => {},
    });
    mount(dialog.element);
    try {
      expect(getComputedStyle(dialog.element).display === "none");
      dialog.show();
      expect(getComputedStyle(dialog.element).display !== "none");
    } finally {
      dialog.hide();
    }
  });

  check("long-text", () => {
    const button = mount(createButton({ variant: "primary", label: LONG_KO_LABEL }));
    const badge = mount(createBadge({ tone: "info", label: LONG_EN_LABEL }));
    const table = mount(createDataTable({
      caption: t.tableCaption,
      columns: TABLE_COLUMNS,
      rows: [[LONG_ID, LONG_KO_LABEL, LONG_NUMBER]],
      emptyText: t.empty,
      numericColumns: ["bytes"],
    }));
    const link = mount(createLink({ href: "#test-long", label: LONG_ID }));
    expect(button.textContent.includes(LONG_KO_LABEL) && badge.textContent.includes(LONG_EN_LABEL));
    expect(table.textContent.includes(LONG_ID) && link.textContent.includes(LONG_ID));
  });

  check("inline-style", () => {
    expect(document.querySelectorAll("style").length === 0);
    // Only the gauge arc may carry an inline style attribute. It must sit inside a progressbar.
    for (const el of document.querySelectorAll("[style]")) {
      expect(el.closest('[role="progressbar"]') !== null);
    }
  });

  check("inline-handler", () => {
    expect(document.querySelectorAll("script:not([src])").length === 0);
    for (const el of document.querySelectorAll("*")) {
      for (const attr of el.attributes) {
        expect(!attr.name.startsWith("on"));
      }
      const href = (el.getAttribute("href") || "").trim().toLowerCase();
      expect(!href.startsWith("javascript:"));
    }
  });
}

// Writes the result. data-result is set last, after every other attribute.
function finish() {
  if (cspViolations > 0) fail("csp-violation");
  const list = document.getElementById("check-list");
  for (const { id, ok } of results) {
    const item = document.createElement("li");
    item.textContent = `${id}: ${ok ? "pass" : "fail"}`;
    list.append(item);
  }
  const overflowX = document.documentElement.scrollWidth > document.documentElement.clientWidth;
  if (overflowX) fail("overflow-x");
  main.dataset.lang = lang;
  main.dataset.cspViolations = String(cspViolations);
  main.dataset.overflowX = overflowX ? "1" : "0";
  main.dataset.failures = failures.join(" ");
  main.dataset.result = failures.length === 0 ? "pass" : "fail";
}

try {
  document.getElementById("page-title").textContent = t.title;
  runChecks();
} catch {
  fail("render-error");
} finally {
  finish();
}
