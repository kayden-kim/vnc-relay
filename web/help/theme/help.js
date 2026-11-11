// VNC Relay help theme script for mdBook 0.5.4 pages (docs/specs/help-site.md, HLP-001).
// mdBook loads this file as a classic script after book.js at the end of <body>, so it
// cannot be an ES module. It shows mdBook's English interface text in the page language
// and adds the pager, the code copy buttons, the note boxes, and the search panel.
// Every visible string is an HCP text from docs/specs/help-site.md section 9.
'use strict';

(function helpTheme() {
  /** Same storage key and values as the dashboard (web/dashboard/app/language.js). */
  const STORAGE_KEY = 'vnc-relay.language';
  const SVG_NS = 'http://www.w3.org/2000/svg';
  /** Characters kept on each side of the first match in a search excerpt. */
  const SNIPPET_RADIUS = 40;
  /** Shortest query, in characters after trimming, that runs a search. */
  const MIN_QUERY_LENGTH = 2;

  const TEXT = {
    ko: {
      toc: '목차',
      searchTitle: '도움말 검색',
      searchHelp: '두 글자 이상 입력하면 결과가 바로 바뀝니다.',
      results: (count) => `검색 결과 ${count}건`,
      noResults: (query) => `‘${query}’ 검색 결과가 없습니다.`,
      loading: '검색 자료를 불러오는 중입니다.',
      failed: '검색 자료를 불러오지 못했습니다. 검색을 닫았다가 다시 여십시오.',
      close: '닫기',
      previous: '이전',
      next: '다음',
      copy: '복사',
      copied: '복사했습니다.',
      copyFailed: '복사하지 못했습니다. 코드를 직접 선택해 복사하십시오.',
      pager: '장 이동',
      keysTitle: '단축키',
      keys: [
        [['←', '→'], '·', '이전 장·다음 장'],
        [['/'], '', '검색 열기'],
        [['?'], '', '이 도움말 보기'],
        [['Esc'], '', '검색·단축키 도움말 닫기'],
      ],
    },
    en: {
      toc: 'Contents',
      searchTitle: 'Search help',
      searchHelp: 'Type at least two characters to see results.',
      results: (count) => (count === 1 ? '1 result' : `${count} results`),
      noResults: (query) => `No results for “${query}”.`,
      loading: 'Loading search data.',
      failed: 'Could not load search data. Close search and open it again.',
      close: 'Close',
      previous: 'Previous',
      next: 'Next',
      copy: 'Copy',
      copied: 'Copied.',
      copyFailed: 'Could not copy. Select the code and copy it manually.',
      pager: 'Chapter navigation',
      keysTitle: 'Keyboard shortcuts',
      keys: [
        [['←', '→'], '', 'Previous or next chapter'],
        [['/'], '', 'Open search'],
        [['?'], '', 'Show this help'],
        [['Esc'], '', 'Close search or this help'],
      ],
    },
  };

  /** Bold labels that start a note box, by box kind (docs/help/AGENTS.md). */
  const NOTE_KINDS = { '참고:': 'info', 'Note:': 'info', '주의:': 'warning', 'Warning:': 'warning' };

  /** Icon parts: the same drawings as the dashboard icons (DESIGN.md 아이콘 규칙). */
  const ICONS = {
    prev: [['path', { d: 'M9.75 4l-4 4 4 4' }]],
    next: [['path', { d: 'M6.25 4l4 4-4 4' }]],
    copy: [
      ['rect', { x: '5.25', y: '5.25', width: '8', height: '8', rx: '1.25' }],
      ['path', { d: 'M10.75 5.25V3.75a1 1 0 0 0-1-1h-6a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h1.5' }],
    ],
    close: [['path', { d: 'M4 4l8 8M12 4l-8 8' }]],
    info: [
      ['circle', { cx: '8', cy: '8', r: '6.25' }],
      ['path', { d: 'M8 4.75V8l2.25 1.5' }],
    ],
    warning: [
      ['path', { d: 'M8 2.25l6.25 11H1.75z' }],
      ['path', { d: 'M8 6.5v3M8 11.25h.01' }],
    ],
    danger: [
      ['circle', { cx: '8', cy: '8', r: '6.25' }],
      ['path', { d: 'M8 5v3.5M8 10.9h.01' }],
    ],
    success: [
      ['circle', { cx: '8', cy: '8', r: '5.5' }],
      ['path', { d: 'm5 8 2 2 4-4' }],
    ],
  };

  const html = document.documentElement;
  const text = TEXT[html.lang === 'ko' ? 'ko' : 'en'];
  // mdBook defines `path_to_root` in an inline script in <head>.
  const bookRoot = new URL(
    (typeof path_to_root === 'string' && path_to_root) || './',
    window.location.href,
  );

  function element(tag, className, content) {
    const node = document.createElement(tag);
    if (className) {
      node.className = className;
    }
    if (content !== undefined) {
      node.textContent = content;
    }
    return node;
  }

  function icon(name) {
    const svg = document.createElementNS(SVG_NS, 'svg');
    svg.setAttribute('class', 'ico');
    svg.setAttribute('viewBox', '0 0 16 16');
    svg.setAttribute('aria-hidden', 'true');
    svg.setAttribute('focusable', 'false');
    for (const [tag, attributes] of ICONS[name]) {
      const part = document.createElementNS(SVG_NS, tag);
      for (const [key, value] of Object.entries(attributes)) {
        part.setAttribute(key, value);
      }
      svg.append(part);
    }
    return svg;
  }

  function alertBox(kind, iconName, message) {
    const box = element('div', `alert a-${kind}`);
    const body = element('div', 'alert-body');
    body.append(element('p', '', message));
    box.append(icon(iconName), body);
    return box;
  }

  function saveLanguage(language) {
    try {
      window.localStorage.setItem(STORAGE_KEY, language);
    } catch {
      // Storage is blocked. The link still opens the other language.
    }
  }

  /** The chapter links of the contents, in contents order, without section links. */
  function chapterLinks(sidebar) {
    const seen = new Set();
    const links = [];
    for (const link of sidebar.querySelectorAll('.chapter a[href]')) {
      const url = new URL(link.href);
      url.hash = '';
      if (url.origin !== bookRoot.origin || !url.pathname.startsWith(bookRoot.pathname)) {
        continue;
      }
      if (seen.has(url.href)) {
        continue;
      }
      seen.add(url.href);
      links.push({ url: url.href, title: link.textContent.trim() });
    }
    return links;
  }

  // Language link: the same chapter in the other book, and remember the choice (2.1).
  function setupLanguageLink() {
    const link = document.querySelector('.help-lang');
    if (!link) {
      return;
    }
    const language = link.dataset.lang;
    const path = window.location.pathname;
    const page = path.startsWith(bookRoot.pathname) ? path.slice(bookRoot.pathname.length) : '';
    link.href = new URL(`../${language}/${page || 'index.html'}`, bookRoot).href;
    link.addEventListener('click', () => saveLanguage(language));
  }

  // Contents: page-language name, current chapter, and the top bar button state.
  function setupContents(sidebar) {
    sidebar.setAttribute('aria-label', text.toc);
    const active = sidebar.querySelector('.chapter a.active');
    if (active) {
      active.setAttribute('aria-current', 'page');
    }
    for (const spacer of sidebar.querySelectorAll('li.spacer')) {
      spacer.setAttribute('aria-hidden', 'true');
    }
    const button = document.querySelector('.help-toc-toggle');
    const label = document.getElementById('mdbook-sidebar-toggle');
    if (!button || !label) {
      return;
    }
    // book.js reacts to its own toggle label, so the button clicks that label.
    button.addEventListener('click', () => label.click());
    const sync = () => {
      button.setAttribute('aria-expanded', String(html.classList.contains('sidebar-visible')));
    };
    new MutationObserver(sync).observe(html, { attributes: true, attributeFilter: ['class'] });
    sync();
  }

  function setupCodeBlocks(main) {
    for (const pre of main.querySelectorAll('pre')) {
      const box = element('div', 'help-code');
      const bar = element('div', 'help-code-bar');
      const live = element('span', 'help-copy-live');
      live.setAttribute('role', 'status');
      const button = element('button', 'btn btn-outline sm');
      button.type = 'button';
      button.append(icon('copy'), element('span', '', text.copy));
      bar.append(live, button);
      pre.replaceWith(box);
      box.append(bar, pre);
      pre.tabIndex = 0;
      button.addEventListener('click', () => copyCode(pre, live));
    }
  }

  async function copyCode(pre, live) {
    live.replaceChildren();
    const code = pre.querySelector('code') ?? pre;
    let copied = false;
    try {
      await navigator.clipboard.writeText(code.textContent);
      copied = true;
    } catch {
      copied = false;
    }
    const status = element('span', `help-copy-status ${copied ? 'ok' : 'fail'}`);
    status.append(
      icon(copied ? 'success' : 'danger'),
      element('span', '', copied ? text.copied : text.copyFailed),
    );
    live.replaceChildren(status);
  }

  function setupTables(main) {
    for (const wrapper of main.querySelectorAll('.table-wrapper')) {
      wrapper.classList.add('help-table');
      wrapper.tabIndex = 0;
      for (const header of wrapper.querySelectorAll('thead th')) {
        header.scope = 'col';
      }
    }
  }

  // `> **참고:** …` and `> **Note:** …` become note boxes; `주의`/`Warning` become warnings.
  function setupCallouts(main) {
    for (const quote of main.querySelectorAll('blockquote')) {
      const first = quote.firstElementChild;
      const label = first && first.tagName === 'P' ? first.firstElementChild : null;
      let kind = 'neutral';
      if (label && label.tagName === 'STRONG' && first.firstChild === label) {
        kind = NOTE_KINDS[label.textContent.trim()] ?? 'neutral';
        if (kind !== 'neutral') {
          label.classList.add('help-callout-label');
        }
      }
      const body = element('div', 'alert-body');
      body.append(...quote.childNodes);
      quote.classList.add('alert', `a-${kind}`, 'help-callout');
      quote.append(icon(kind === 'warning' ? 'warning' : 'info'), body);
    }
  }

  // Previous and next chapter, from mdBook's hidden chapter arrows. book.js keeps
  // reading those arrows for the `←` and `→` keys.
  function setupPager(content, links) {
    const arrows = document.querySelector('.nav-wide-wrapper');
    const previous = arrows?.querySelector('a.nav-chapters.previous');
    const next = arrows?.querySelector('a.nav-chapters.next');
    if (!previous && !next) {
      return;
    }
    const titleOf = (href) => {
      const url = new URL(href);
      url.hash = '';
      return links.find((link) => link.url === url.href)?.title ?? url.pathname.split('/').pop();
    };
    const nav = element('nav', 'help-pager');
    nav.setAttribute('aria-label', text.pager);
    if (previous) {
      nav.append(pagerLink(previous.href, 'prev', text.previous, titleOf(previous.href)));
    }
    if (next) {
      nav.append(pagerLink(next.href, 'next', text.next, titleOf(next.href)));
    }
    content.append(nav);
  }

  function pagerLink(href, kind, direction, title) {
    const link = element('a', `help-pager-link ${kind}`);
    link.href = href;
    link.rel = kind;
    const label = element('span', 'help-pager-dir label');
    const word = element('span', '', direction);
    if (kind === 'prev') {
      label.append(icon('prev'), word);
    } else {
      label.append(word, icon('next'));
    }
    link.append(label, element('span', 'help-pager-title', title));
    return link;
  }

  // Shortcut help: mdBook's popup with the page-language text (HCP-023~HCP-027).
  function setupShortcuts() {
    const container = document.getElementById('mdbook-help-container');
    const popup = document.getElementById('mdbook-help-popup');
    if (!container || !popup) {
      return;
    }
    const title = element('h2', 't-card', text.keysTitle);
    title.id = 'help-keys-title';
    const list = element('ul', 'help-keys-list');
    for (const [keys, separator, description] of text.keys) {
      const item = element('li');
      const group = element('span', 'help-keys');
      keys.forEach((key, index) => {
        if (index > 0 && separator) {
          group.append(separator);
        }
        group.append(element('kbd', '', key));
      });
      item.append(group, element('span', '', description));
      list.append(item);
    }
    popup.replaceChildren(title, list);
    popup.setAttribute('role', 'dialog');
    popup.setAttribute('aria-labelledby', title.id);
    popup.tabIndex = -1;
    // book.js shows and hides the popup with an inline display value. Focus goes into the
    // popup when it opens and back to the earlier element when it closes.
    let returnFocus = null;
    new MutationObserver(() => {
      const open = container.style.display === 'flex';
      if (open && !returnFocus) {
        returnFocus = document.activeElement ?? document.body;
        popup.focus();
      } else if (!open && returnFocus) {
        const target = returnFocus;
        returnFocus = null;
        if (target !== document.body && target.isConnected && typeof target.focus === 'function') {
          target.focus();
        }
      }
    }).observe(container, { attributes: true, attributeFilter: ['style'] });
  }

  /** Unicode NFC and lower case, so matching ignores case and composition. */
  function fold(value) {
    return value.normalize('NFC').toLowerCase();
  }

  function squeeze(value) {
    return value.replace(/\s+/g, ' ').trim();
  }

  /** Text of a block with a space between cells, items, and paragraphs. */
  function blockText(node) {
    for (const part of node.querySelectorAll('p, li, th, td, tr, dt, dd, br, pre')) {
      part.append(' ');
    }
    return squeeze(node.textContent);
  }

  /** Splits one chapter page into sections at `h1`~`h3` (search rule: range). */
  function chapterSections(page, source) {
    const doc = new DOMParser().parseFromString(source, 'text/html');
    const main = doc.querySelector('#mdbook-content main') ?? doc.querySelector('main');
    const sections = [];
    if (!main) {
      return sections;
    }
    let current = null;
    for (const node of main.children) {
      if (/^H[1-3]$/.test(node.tagName)) {
        const heading = squeeze(node.textContent).normalize('NFC');
        current = {
          label: node.tagName === 'H1' ? page.title : `${page.title} › ${heading}`,
          url: node.tagName === 'H1' || !node.id ? page.url : `${page.url}#${node.id}`,
          heading,
          parts: [],
        };
        sections.push(current);
      } else if (current) {
        current.parts.push(blockText(node));
      }
    }
    return sections.map((section) => {
      const body = section.parts.filter(Boolean).join(' ').normalize('NFC');
      return {
        label: section.label,
        url: section.url,
        headingKey: fold(section.heading),
        body,
        bodyKey: fold(body),
      };
    });
  }

  /** Appends `shown` from `start` to `end` to `parent`, with every match of `words`
      marked. `key` is the folded `shown` and has the same length. */
  function appendMarked(parent, shown, key, words, start, end) {
    const marks = [];
    for (const word of words) {
      let from = key.indexOf(word, start);
      while (from >= 0 && from < end) {
        marks.push([from, Math.min(end, from + word.length)]);
        from = key.indexOf(word, from + word.length);
      }
    }
    marks.sort((a, b) => a[0] - b[0]);
    let cursor = start;
    for (const [from, to] of marks) {
      if (to <= cursor) {
        continue;
      }
      const begin = Math.max(from, cursor);
      if (begin > cursor) {
        parent.append(shown.slice(cursor, begin));
      }
      parent.append(element('mark', '', shown.slice(begin, to)));
      cursor = to;
    }
    if (cursor < end) {
      parent.append(shown.slice(cursor, end));
    }
  }

  /** `text` for display next to its folded form. Lower case can change the length of
      rare characters; then the folded text is shown, so marks stay in place. */
  function foldedPair(text) {
    const key = fold(text);
    return [key.length === text.length ? text : key, key];
  }

  /** Excerpt around the first word found in the body, with every match marked.
      `null` when the section has no body text (a heading right before a subsection). */
  function excerpt(section, words) {
    const key = section.bodyKey;
    const shown = key.length === section.body.length ? section.body : key;
    if (!shown) {
      return null;
    }
    const paragraph = element('p');
    let at = -1;
    let length = 0;
    for (const word of words) {
      at = key.indexOf(word);
      if (at >= 0) {
        length = word.length;
        break;
      }
    }
    let start = at < 0 ? 0 : Math.max(0, at - SNIPPET_RADIUS);
    let end = Math.min(shown.length, (at < 0 ? 0 : at + length) + SNIPPET_RADIUS * (at < 0 ? 2 : 1));
    // Do not cut a surrogate pair, and do not start or end the excerpt on a space.
    if (start > 0 && /[\udc00-\udfff]/.test(shown[start])) {
      start += 1;
    }
    if (end < shown.length && /[\udc00-\udfff]/.test(shown[end])) {
      end += 1;
    }
    while (start > 0 && start < end && shown[start] === ' ') {
      start += 1;
    }
    while (end < shown.length && end > start && shown[end - 1] === ' ') {
      end -= 1;
    }
    if (start > 0) {
      paragraph.append('… ');
    }
    appendMarked(paragraph, shown, key, words, start, end);
    if (end < shown.length) {
      paragraph.append(' …');
    }
    return paragraph;
  }

  // Search panel (search rules in docs/specs/help-site.md HLP-001). mdBook's own
  // search is off because it does not index Korean words.
  function setupSearch(content, links) {
    const toggle = document.querySelector('.help-search-toggle');
    if (!toggle) {
      return;
    }
    let panel = null;
    let input = null;
    let status = null;
    let results = null;
    let sections = null;
    let loading = null;
    let failed = false;

    function build() {
      panel = element('section', 'card help-search');
      panel.id = 'help-search';
      panel.hidden = true;
      panel.setAttribute('role', 'search');
      panel.setAttribute('aria-labelledby', 'help-search-title');
      const head = element('div', 'help-search-head');
      const title = element('label', 't-card', text.searchTitle);
      title.id = 'help-search-title';
      title.htmlFor = 'help-search-input';
      const close = element('button', 'btn btn-outline sm');
      close.type = 'button';
      close.append(icon('close'), element('span', '', text.close));
      head.append(title, close);
      input = element('input', 'input');
      input.id = 'help-search-input';
      input.type = 'search';
      input.autocomplete = 'off';
      input.spellcheck = false;
      input.setAttribute('aria-describedby', 'help-search-help help-search-status');
      const help = element('p', 'label muted', text.searchHelp);
      help.id = 'help-search-help';
      status = element('div', 'help-status');
      status.id = 'help-search-status';
      status.setAttribute('role', 'status');
      results = element('ol', 'help-results');
      panel.append(head, input, help, status, results);
      content.prepend(panel);
      toggle.setAttribute('aria-controls', panel.id);
      close.addEventListener('click', closeSearch);
      input.addEventListener('input', render);
    }

    function load() {
      failed = false;
      sections = null;
      const request = Promise.all(
        links.map(async (page) => {
          const response = await fetch(page.url, { credentials: 'same-origin' });
          if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
          }
          return chapterSections(page, await response.text());
        }),
      );
      loading = request;
      request
        .then(
          (pages) => {
            if (loading === request) {
              sections = pages.flat();
            }
          },
          () => {
            if (loading === request) {
              failed = true;
            }
          },
        )
        .finally(() => {
          if (loading === request) {
            loading = null;
            render();
          }
        });
    }

    function render() {
      if (!panel) {
        return;
      }
      const query = input.value.trim();
      results.replaceChildren();
      if (failed) {
        status.replaceChildren(alertBox('destructive', 'danger', text.failed));
        return;
      }
      if ([...query].length < MIN_QUERY_LENGTH) {
        status.replaceChildren();
        return;
      }
      if (!sections) {
        status.replaceChildren(alertBox('info', 'info', text.loading));
        return;
      }
      const words = fold(query).split(/\s+/).filter(Boolean);
      const inHeading = [];
      const inBody = [];
      for (const section of sections) {
        const haystack = `${section.headingKey}\n${section.bodyKey}`;
        if (!words.every((word) => haystack.includes(word))) {
          continue;
        }
        const group = words.every((word) => section.headingKey.includes(word)) ? inHeading : inBody;
        group.push(section);
      }
      const found = inHeading.concat(inBody);
      status.replaceChildren(
        element('p', '', found.length > 0 ? text.results(found.length) : text.noResults(query)),
      );
      for (const section of found) {
        const item = element('li');
        // A match in a heading is marked in the chapter › section label.
        const link = element('a');
        const [label, labelKey] = foldedPair(section.label);
        appendMarked(link, label, labelKey, words, 0, labelKey.length);
        link.href = section.url;
        item.append(link);
        const lines = excerpt(section, words);
        if (lines) {
          item.append(lines);
        }
        results.append(item);
      }
    }

    function openSearch() {
      if (!panel) {
        build();
      }
      const reopened = panel.hidden;
      panel.hidden = false;
      toggle.setAttribute('aria-expanded', 'true');
      if (reopened && (failed || (!sections && !loading))) {
        load();
      }
      render();
      input.focus();
    }

    function closeSearch() {
      if (!panel || panel.hidden) {
        return;
      }
      panel.hidden = true;
      toggle.setAttribute('aria-expanded', 'false');
      toggle.focus();
    }

    toggle.addEventListener('click', () => {
      if (panel && !panel.hidden) {
        closeSearch();
      } else {
        openSearch();
      }
    });

    document.addEventListener('keydown', (event) => {
      if (event.altKey || event.ctrlKey || event.metaKey || event.defaultPrevented) {
        return;
      }
      const target = event.target;
      const typing =
        target instanceof Element &&
        target.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"])');
      if (event.key === '/' && !typing) {
        event.preventDefault();
        openSearch();
      } else if (event.key === 'Escape' && panel && !panel.hidden) {
        event.preventDefault();
        closeSearch();
      }
    });
  }

  // mdBook's template puts the contents before the page, so the top bar would come after
  // the contents in the tab order although it is above them on the screen. Moving it to
  // the start of the grid keeps the layout, because each part has its own grid area.
  const top = document.querySelector('.help-top');
  const container = document.getElementById('mdbook-body-container');
  if (top && container) {
    container.prepend(top);
  }
  setupLanguageLink();
  const sidebar = document.getElementById('mdbook-sidebar');
  const links = sidebar ? chapterLinks(sidebar) : [];
  if (sidebar) {
    setupContents(sidebar);
  }
  const content = document.getElementById('mdbook-content');
  const main = content?.querySelector('main');
  if (content && main) {
    setupCodeBlocks(main);
    setupTables(main);
    setupCallouts(main);
    setupPager(content, links);
    setupSearch(content, links);
  }
  setupShortcuts();
})();
