// Preview page: every sample is built by calling the shared component functions.
// No component markup or CSS is copied here.
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
import { COPY } from "../app/copy.js";

// Counts CSP violations. Registered at module top so no early violation is missed.
let cspViolations = 0;
document.addEventListener("securitypolicyviolation", () => {
  cspViolations += 1;
});

// ?lang=en selects English. Any other value, or none, selects Korean.
const lang = new URLSearchParams(location.search).get("lang") === "en" ? "en" : "ko";
document.documentElement.lang = lang;

const STRINGS = {
  ko: {
    title: "컴포넌트 미리보기",
    intro: "공통 컴포넌트를 상태별로 보여 준다. 모든 예시는 app/components/ 의 함수로 만든다.",
    navLabel: "주 메뉴",
    button: "버튼",
    primary: "주요",
    outline: "윤곽선",
    ghost: "투명",
    destructive: "위험",
    default: "기본",
    hover: "호버",
    focus: "포커스",
    disabled: "비활성",
    busy: "진행 중",
    busyLabel: "처리 중",
    signIn: "로그인",
    refresh: "새로고침",
    back: "목록으로",
    endSession: "세션 종료",
    logout: "로그아웃",
    navbarAction: "상단 막대 항목",
    textField: "텍스트 입력",
    fieldLabel: "관리자 ID",
    fieldHelp: "로그인에 쓰는 ID를 입력하세요.",
    fieldError: "ID가 맞지 않습니다.",
    help: "도움말",
    error: "오류",
    badge: "배지",
    alert: "경고",
    alertMessage: "중계 연결 상태를 다시 확인하세요.",
    tone: { success: "정상", info: "안내", warning: "확인 필요", destructive: "조회 실패", neutral: "대기" },
    card: "카드",
    cardTitle: "활성 세션",
    empty: "활성 세션이 없습니다.",
    emptyState: "빈 상태",
    stat: "지표",
    statActive: "활성 세션",
    statBytes: "전송 바이트",
    unitCount: "건",
    unitBytes: "B",
    gauge: "사용량",
    gaugeLimit: "상한",
    gaugeUnknown: "측정 없음",
    navbar: "상단 메뉴",
    navItem: "운영 현황",
    navSettings: "설정",
    languageSelect: "언어 선택",
    languageLabel: "언어 (Language)",
    form: "입력 양식",
    dataTable: "데이터 표",
    tableCaption: "활성 세션 예시",
    colSession: "세션",
    colMode: "모드",
    colBytes: "바이트",
    link: "링크",
    dialog: "대화상자",
    closed: "닫힘",
    open: "열림",
    openConfirm: "확인 창 열기",
    confirmTitle: "설정을 저장할까요?",
    confirmDescription: "바꾼 값이 적용됩니다.",
    endTitle: "세션을 종료할까요?",
    endDescription: "종료한 세션은 되돌릴 수 없습니다.",
    confirm: "확인",
    cancel: "취소",
    longKo: "긴 한국어",
    longEn: "긴 영어",
    numbers: "긴 숫자",
  },
  en: {
    title: "Component preview",
    intro: "Shows the shared components by state. Every sample is built by functions in app/components/.",
    navLabel: "Main menu",
    button: "Button",
    primary: "Primary",
    outline: "Outline",
    ghost: "Ghost",
    destructive: "Destructive",
    default: "Default",
    hover: "Hover",
    focus: "Focus",
    disabled: "Disabled",
    busy: "Busy",
    busyLabel: "Processing",
    signIn: "Sign in",
    refresh: "Refresh",
    back: "Back to list",
    endSession: "End session",
    logout: "Sign out",
    navbarAction: "Top bar item",
    textField: "Text field",
    fieldLabel: "Admin ID",
    fieldHelp: "Enter the ID you use to sign in.",
    fieldError: "The ID does not match.",
    help: "Help text",
    error: "Error",
    badge: "Badge",
    alert: "Alert",
    alertMessage: "Check the relay connection state again.",
    tone: { success: "Healthy", info: "Notice", warning: "Check needed", destructive: "Failed", neutral: "Waiting" },
    card: "Card",
    cardTitle: "Active sessions",
    empty: "No active sessions.",
    emptyState: "Empty state",
    stat: "Stat",
    statActive: "Active sessions",
    statBytes: "Transferred bytes",
    unitCount: "items",
    unitBytes: "B",
    gauge: "Usage",
    gaugeLimit: "Limit",
    gaugeUnknown: "No reading",
    navbar: "Navbar",
    navItem: "Operations",
    navSettings: "Settings",
    languageSelect: "Language select",
    languageLabel: "Language (언어)",
    form: "Form",
    dataTable: "Data table",
    tableCaption: "Example active sessions",
    colSession: "Session",
    colMode: "Mode",
    colBytes: "Bytes",
    link: "Link",
    dialog: "Dialog",
    closed: "Closed",
    open: "Open",
    openConfirm: "Open confirmation",
    confirmTitle: "Save settings?",
    confirmDescription: "Changed values will apply.",
    endTitle: "End this session?",
    endDescription: "An ended session cannot be restored.",
    confirm: "Confirm",
    cancel: "Cancel",
    longKo: "Long Korean",
    longEn: "Long English",
    numbers: "Long numbers",
  },
};
const t = STRINGS[lang];

// Long strings are fixed in each language so the overflow check always sees them.
const LONG_KO = "중계 연결 수가 상한에 가까워지면 새 연결이 거절될 수 있습니다. 활성 세션을 종료하기 전에 대기 연결과 최근 종료 목록을 함께 확인하세요.";
const LONG_EN = "When relay connections approach the limit, new connections can be refused. Check pending connections and recently ended sessions together before ending any active session.";
const LONG_KO_LABEL = "선택한 세션을 종료하고 중계 자원을 정리";
const LONG_EN_LABEL = "End the selected session and release relay resources";
const LONG_ID_EN = "relay-session-0123456789abcdef0123456789abcdef0123456789";
const LONG_NUMBER = "18,446,744,073,709,551,615";

const TONES = ["success", "info", "warning", "destructive", "neutral"];
const BUTTON_VARIANTS = {
  primary: { name: t.primary, label: t.signIn },
  outline: { name: t.outline, label: t.refresh, icon: "refresh" },
  ghost: { name: t.ghost, label: t.back, icon: "back" },
  destructive: { name: t.destructive, label: t.endSession, icon: "cancel" },
  navbar: { name: t.navbarAction, label: t.logout, icon: "logout" },
};
const BUTTON_STATES = ["default", "hover", "focus", "disabled", "busy"];
const LANGUAGE_OPTIONS = [
  { value: "ko", label: "한국어" },
  { value: "en", label: "English" },
];

// Every sample id the page must render. The module checks each one in the DOM.
const REQUIRED_SAMPLES = [
  "button-primary-default", "button-primary-hover", "button-primary-focus", "button-primary-disabled", "button-primary-busy",
  "button-outline-default", "button-outline-hover", "button-outline-focus", "button-outline-disabled", "button-outline-busy",
  "button-ghost-default", "button-ghost-hover", "button-ghost-focus", "button-ghost-disabled", "button-ghost-busy",
  "button-destructive-default", "button-destructive-hover", "button-destructive-focus", "button-destructive-disabled", "button-destructive-busy",
  "button-navbar-default", "button-navbar-hover", "button-navbar-focus", "button-navbar-disabled", "button-navbar-busy",
  "button-long-ko", "button-long-en",
  "text-field-default", "text-field-error", "text-field-focus", "text-field-help-long-ko", "text-field-help-long-en",
  "badge-success", "badge-info", "badge-warning", "badge-destructive", "badge-neutral", "badge-long-ko", "badge-long-en",
  "alert-success", "alert-info", "alert-warning", "alert-destructive", "alert-neutral", "alert-long-ko", "alert-long-en",
  "card-default", "card-empty", "card-long-ko", "card-long-en",
  "stat-default", "stat-long-numbers", "stat-long-ko", "stat-long-en",
  "gauge-0", "gauge-312", "gauge-1000", "gauge-unknown", "gauge-long-ko", "gauge-long-en",
  "navbar-default", "navbar-long-ko", "navbar-long-en",
  "language-select-navbar", "language-select-form", "language-select-long-ko", "language-select-long-en",
  "data-table-default", "data-table-empty", "data-table-long-ko", "data-table-long-en", "data-table-long-numbers",
  "link-default", "link-hover", "link-focus", "link-long-ko", "link-long-en",
  "link-help-login",
  "dialog-closed", "dialog-default", "dialog-destructive", "dialog-long-ko", "dialog-long-en",
];

// Failed check ids. Written to data-failures as a space-separated list.
const failures = [];
function fail(id) {
  if (!failures.includes(id)) failures.push(id);
}

function cap(...parts) {
  return parts.join(" · ");
}

function sample(id, caption, build) {
  return { id, caption, build };
}

function group(title, rows, fill = false) {
  return { title, rows, fill };
}

function withClass(element, name) {
  element.classList.add(name);
  return element;
}

function paragraph(text) {
  const p = document.createElement("p");
  p.textContent = text;
  return p;
}

function onDark(element) {
  const wrap = document.createElement("div");
  wrap.className = "sample-dark";
  wrap.append(element);
  return wrap;
}

function sessionRows() {
  return [
    ["S-1001", "Mode I", "12,288"],
    ["S-1002", "Mode II", "4,096"],
    ["S-1003", "Mode I", "0"],
  ];
}

function sessionTable(rows, emptyText = t.empty) {
  return createDataTable({
    caption: t.tableCaption,
    columns: [
      { key: "session", label: t.colSession },
      { key: "mode", label: t.colMode },
      { key: "bytes", label: t.colBytes },
    ],
    rows,
    emptyText,
    numericColumns: ["bytes"],
  });
}

function buttonSample(variant, state) {
  const { name, label, icon } = BUTTON_VARIANTS[variant];
  return sample(`button-${variant}-${state}`, cap(t.button, name, t[state]), () => {
    const button = createButton({
      variant,
      label: state === "busy" ? t.busyLabel : label,
      disabled: state === "disabled",
      busy: state === "busy",
      ...(icon ? { icon } : {}),
    });
    // Preview-only state classes. App code never adds these.
    if (state === "hover") button.classList.add("is-hover");
    if (state === "focus") button.classList.add("is-focus");
    // The top bar item sits on the dark bar, so it gets the same backdrop as the navbar sample.
    return variant === "navbar" ? onDark(button) : button;
  });
}

function textField(sid, extra = {}) {
  return createTextField({
    id: `${sid}-field`,
    label: t.fieldLabel,
    type: "text",
    value: "",
    required: true,
    autocomplete: "username",
    helpText: t.fieldHelp,
    ...extra,
  });
}

function focusInput(field) {
  // Preview-only focus state. App code never adds this class.
  field.querySelector("input").classList.add("is-focus");
  return field;
}

function alertSample(tone) {
  return sample(`alert-${tone}`, cap(t.alert, t.tone[tone]), () =>
    createAlert({ tone, title: t.tone[tone], message: t.alertMessage }),
  );
}

// Gauge samples share one fixed limit, so the 상한 line is the same in every sample.
const GAUGE_MAX = 1000;

function formatCount(value) {
  return value.toLocaleString("en-US");
}

function gaugeLimitText() {
  return `${t.gaugeLimit} ${formatCount(GAUGE_MAX)}`;
}

function gaugeSample(value) {
  const unknown = value === null;
  const caption = unknown ? t.gaugeUnknown : formatCount(value);
  return sample(unknown ? "gauge-unknown" : `gauge-${value}`, cap(t.gauge, caption), () =>
    createGauge({
      label: t.gauge,
      value,
      max: GAUGE_MAX,
      valueText: unknown ? undefined : formatCount(value),
      limitText: gaugeLimitText(),
      unknownText: t.gaugeUnknown,
    }),
  );
}

function languageSelect(sid, variant, label) {
  return createLanguageSelect({
    id: `${sid}-select`,
    label,
    value: lang,
    options: LANGUAGE_OPTIONS,
    variant,
  });
}

function helpLink(className) {
  const help = createLink({
    href: `https://kayden-kim.github.io/vnc-relay/${lang}/`,
    label: COPY[lang].help,
    icon: "help",
    iconSize: "md",
    newTabText: COPY[lang].opensInNewTab,
  });
  help.className = className;
  return help;
}

function navbar(sid, itemLabel) {
  return createNavbar({
    brand: "VNC Relay",
    items: [
      { label: itemLabel, href: `#${sid}`, current: true, icon: "grid" },
      { label: t.navSettings, href: `#${sid}-settings`, icon: "settings" },
    ],
    help: helpLink("navbar-item help-link"),
    language: languageSelect(`${sid}-language`, "navbar", t.languageLabel),
    action: createButton({ variant: "navbar", label: t.logout, icon: "logout" }),
    label: t.navLabel,
  });
}

function link(sid, label) {
  return createLink({ href: `#${sid}`, label });
}

// Builds a dialog whose buttons close it. Each sample decides how the dialog is shown.
function makeDialog(options) {
  const dialog = createDialog({
    title: options.title,
    description: options.description,
    confirmLabel: t.confirm,
    cancelLabel: t.cancel,
    destructive: options.destructive,
    onConfirm: () => dialog.hide(),
    onCancel: () => dialog.hide(),
  });
  return dialog;
}

// Closed sample: the trigger opens the dialog as a real modal.
function dialogSample(options) {
  const dialog = makeDialog(options);
  const trigger = createButton({
    variant: options.destructive ? "destructive" : "outline",
    label: options.trigger,
  });
  trigger.addEventListener("click", () => dialog.show());
  const wrap = document.createElement("div");
  wrap.append(trigger, dialog.element);
  return wrap;
}

// Open sample: a modal would leave the page frame, so the dialog is set open in place
// (non-modal) and a preview-only scrim stands in for the backdrop.
function dialogOpenSample(options) {
  const dialog = makeDialog(options);
  dialog.element.open = true;
  const scrim = document.createElement("div");
  scrim.className = "dialog-frame-scrim";
  scrim.setAttribute("aria-hidden", "true");
  const frame = document.createElement("div");
  frame.className = "dialog-frame";
  frame.append(scrim, dialog.element);
  return frame;
}

// Rows share the app's 12-column grid. Five samples wrap after the third.
function buildGroups() {
  return [
    group(t.button, [
      ...Object.keys(BUTTON_VARIANTS).map((variant) => BUTTON_STATES.map((state) => buttonSample(variant, state))),
      [
        sample("button-long-ko", cap(t.button, t.longKo), () =>
          createButton({ variant: "primary", label: LONG_KO_LABEL }),
        ),
        sample("button-long-en", cap(t.button, t.longEn), () =>
          createButton({ variant: "outline", label: LONG_EN_LABEL, icon: "refresh" }),
        ),
      ],
    ]),
    group(t.textField, [
      [
        sample("text-field-default", cap(t.textField, t.default), (sid) => textField(sid)),
        sample("text-field-error", cap(t.textField, t.error), (sid) => textField(sid, { error: t.fieldError })),
        sample("text-field-focus", cap(t.textField, t.focus), (sid) => focusInput(textField(sid))),
      ],
      [
        sample("text-field-help-long-ko", cap(t.textField, t.help, t.longKo), (sid) =>
          textField(sid, { helpText: LONG_KO }),
        ),
        sample("text-field-help-long-en", cap(t.textField, t.help, t.longEn), (sid) =>
          textField(sid, { helpText: LONG_EN }),
        ),
      ],
    ], true),
    group(t.badge, [
      TONES.map((tone) =>
        sample(`badge-${tone}`, cap(t.badge, t.tone[tone]), () => createBadge({ tone, label: t.tone[tone] })),
      ),
      [
        sample("badge-long-ko", cap(t.badge, t.longKo), () => createBadge({ tone: "info", label: LONG_KO_LABEL })),
        sample("badge-long-en", cap(t.badge, t.longEn), () => createBadge({ tone: "info", label: LONG_EN_LABEL })),
      ],
    ]),
    group(t.alert, [
      [alertSample("success"), alertSample("info")],
      [alertSample("warning"), alertSample("destructive")],
      [alertSample("neutral")],
      [
        sample("alert-long-ko", cap(t.alert, t.longKo), () =>
          createAlert({ tone: "warning", title: LONG_KO_LABEL, message: LONG_KO }),
        ),
        sample("alert-long-en", cap(t.alert, t.longEn), () =>
          createAlert({ tone: "info", title: LONG_EN_LABEL, message: LONG_EN }),
        ),
      ],
    ], true),
    group(t.card, [
      [
        sample("card-default", cap(t.card, t.default), () =>
          createCard({
            title: t.cardTitle,
            count: 2,
            actions: createButton({ variant: "outline", label: t.refresh, icon: "refresh" }),
            body: sessionTable(sessionRows()),
          }),
        ),
        sample("card-empty", cap(t.card, t.emptyState), () =>
          createCard({ title: t.cardTitle, count: 0, body: sessionTable([]) }),
        ),
      ],
      [
        sample("card-long-ko", cap(t.card, t.longKo), () =>
          createCard({ title: LONG_KO_LABEL, count: 1, body: paragraph(LONG_KO) }),
        ),
        sample("card-long-en", cap(t.card, t.longEn), () =>
          createCard({ title: LONG_EN_LABEL, count: 1, body: paragraph(LONG_EN) }),
        ),
      ],
    ], true),
    group(t.stat, [
      [
        sample("stat-default", cap(t.stat, t.default), () =>
          createStat({ label: t.statActive, value: "3", unit: t.unitCount }),
        ),
        sample("stat-long-numbers", cap(t.stat, t.numbers), () =>
          createStat({ label: t.statBytes, value: LONG_NUMBER, unit: t.unitBytes }),
        ),
      ],
      [
        sample("stat-long-ko", cap(t.stat, t.longKo), () =>
          createStat({ label: LONG_KO_LABEL, value: "12", unit: t.unitCount }),
        ),
        sample("stat-long-en", cap(t.stat, t.longEn), () =>
          createStat({ label: LONG_EN_LABEL, value: "12", unit: t.unitCount }),
        ),
      ],
    ], true),
    group(t.gauge, [
      [gaugeSample(0), gaugeSample(312), gaugeSample(GAUGE_MAX)],
      [
        gaugeSample(null),
        sample("gauge-long-ko", cap(t.gauge, t.longKo), () =>
          createCard({
            title: LONG_KO_LABEL,
            body: createGauge({
              label: t.gauge, value: 312, max: GAUGE_MAX, valueText: formatCount(312), limitText: gaugeLimitText(),
            }),
          }),
        ),
        sample("gauge-long-en", cap(t.gauge, t.longEn), () =>
          createCard({
            title: LONG_EN_LABEL,
            body: createGauge({
              label: t.gauge, value: 312, max: GAUGE_MAX, valueText: formatCount(312), limitText: gaugeLimitText(),
            }),
          }),
        ),
      ],
    ]),
    group(t.navbar, [
      [sample("navbar-default", cap(t.navbar, t.default), (sid) => navbar(sid, t.navItem))],
      [sample("navbar-long-ko", cap(t.navbar, t.longKo), (sid) => navbar(sid, LONG_KO_LABEL))],
      [sample("navbar-long-en", cap(t.navbar, t.longEn), (sid) => navbar(sid, LONG_EN_LABEL))],
    ], true),
    group(t.languageSelect, [
      [
        sample("language-select-navbar", cap(t.languageSelect, t.navbar), (sid) =>
          onDark(languageSelect(sid, "navbar", t.languageLabel)),
        ),
        sample("language-select-form", cap(t.languageSelect, t.form), (sid) =>
          languageSelect(sid, "form", t.languageLabel),
        ),
      ],
      [
        sample("language-select-long-ko", cap(t.languageSelect, t.longKo), (sid) =>
          languageSelect(sid, "form", LONG_KO_LABEL),
        ),
        sample("language-select-long-en", cap(t.languageSelect, t.longEn), (sid) =>
          languageSelect(sid, "form", LONG_EN_LABEL),
        ),
      ],
    ]),
    group(t.dataTable, [
      [
        sample("data-table-default", cap(t.dataTable, t.default), () => sessionTable(sessionRows())),
        sample("data-table-empty", cap(t.dataTable, t.emptyState), () => sessionTable([])),
      ],
      [
        sample("data-table-long-ko", cap(t.dataTable, t.longKo), () =>
          sessionTable([["S-1004", LONG_KO, "512"]]),
        ),
        sample("data-table-long-en", cap(t.dataTable, t.longEn), () =>
          sessionTable([[LONG_ID_EN, LONG_EN, "2,048"]]),
        ),
      ],
      [
        sample("data-table-long-numbers", cap(t.dataTable, t.numbers), () =>
          sessionTable([
            ["S-1005", "Mode II", LONG_NUMBER],
            ["S-1006", "Mode I", "1,234,567,890,123"],
          ]),
        ),
      ],
    ], true),
    group(t.link, [
      [
        sample("link-default", cap(t.link, t.default), (sid) => link(sid, "S-1001")),
        sample("link-hover", cap(t.link, t.hover), (sid) => withClass(link(sid, "S-1001"), "is-hover")),
        sample("link-focus", cap(t.link, t.focus), (sid) => withClass(link(sid, "S-1001"), "is-focus")),
      ],
      [
        sample("link-long-ko", cap(t.link, t.longKo), (sid) => link(sid, LONG_KO_LABEL)),
        sample("link-long-en", cap(t.link, t.longEn), (sid) => link(sid, LONG_ID_EN)),
      ],
      [sample("link-help-login", COPY[lang].help, () => helpLink("help-link-login"))],
    ]),
    group(t.dialog, [
      [
        sample("dialog-closed", cap(t.dialog, t.closed), () =>
          dialogSample({
            title: t.confirmTitle,
            description: t.confirmDescription,
            destructive: false,
            trigger: t.openConfirm,
          }),
        ),
        sample("dialog-default", cap(t.dialog, t.open, t.default), () =>
          dialogOpenSample({
            title: t.confirmTitle,
            description: t.confirmDescription,
            destructive: false,
          }),
        ),
        sample("dialog-destructive", cap(t.dialog, t.open, t.destructive), () =>
          dialogOpenSample({
            title: t.endTitle,
            description: t.endDescription,
            destructive: true,
          }),
        ),
      ],
      [
        sample("dialog-long-ko", cap(t.dialog, t.open, t.longKo), () =>
          dialogOpenSample({ title: LONG_KO_LABEL, description: LONG_KO, destructive: false }),
        ),
        sample("dialog-long-en", cap(t.dialog, t.open, t.longEn), () =>
          dialogOpenSample({ title: LONG_EN_LABEL, description: LONG_EN, destructive: false }),
        ),
      ],
    ]),
  ];
}

function renderSample(item, fill, span) {
  const section = document.createElement("section");
  section.className = `sample ${span}${fill ? " sample-fill" : ""}`;
  section.dataset.sample = item.id;
  const caption = document.createElement("h3");
  caption.id = `caption-${item.id}`;
  caption.className = "sample-caption";
  caption.textContent = item.caption;
  section.setAttribute("aria-labelledby", caption.id);
  section.append(caption, item.build(item.id));
  return section;
}

function renderGroup(group, index) {
  const section = document.createElement("section");
  section.className = "sample-group";
  const heading = document.createElement("h2");
  heading.id = `group-${index}`;
  heading.className = "sample-group-title";
  heading.textContent = group.title;
  section.setAttribute("aria-labelledby", heading.id);
  section.append(heading);
  for (const samples of group.rows) {
    const row = document.createElement("div");
    row.className = "grid";
    const span = samples.length === 1 ? "span-12" : samples.length === 2 ? "span-6" : "span-4";
    for (const item of samples) {
      try {
        row.append(renderSample(item, group.fill, span));
      } catch {
        // The failed sample is missing from the DOM, so the missing-sample check also fails.
        fail("render-error");
      }
    }
    section.append(row);
  }
  return section;
}

function run() {
  document.title = t.title;
  document.getElementById("page-title").textContent = t.title;
  document.getElementById("page-intro").textContent = t.intro;
  const container = document.getElementById("samples");
  buildGroups().forEach((group, index) => container.append(renderGroup(group, index)));
  const main = document.getElementById("result");
  const missing = REQUIRED_SAMPLES.filter((id) => main.querySelector(`section[data-sample="${id}"]`) === null);
  if (missing.length > 0) fail("missing-sample");
}

function finish() {
  const main = document.getElementById("result");
  const overflowX = document.documentElement.scrollWidth > document.documentElement.clientWidth;
  if (cspViolations > 0) fail("csp-violation");
  if (overflowX) fail("overflow-x");
  // A closed dialog must stay hidden. A visible one would show as a stray panel.
  for (const dialog of document.querySelectorAll("dialog:not([open])")) {
    if (getComputedStyle(dialog).display !== "none") fail("closed-dialog-visible");
  }
  main.dataset.lang = lang;
  main.dataset.cspViolations = String(cspViolations);
  main.dataset.overflowX = overflowX ? "1" : "0";
  main.dataset.failures = failures.join(" ");
  main.dataset.result = failures.length === 0 ? "pass" : "fail";
}

try {
  run();
} catch {
  fail("render-error");
} finally {
  finish();
}
