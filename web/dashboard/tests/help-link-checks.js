// DASH-064 DOM and layout checks. Never activate the external help link.
export function checkHelpLink(root, lang, check, { login = false } = {}) {
  const label = `DASH-064 ${login ? 'login' : 'navbar'} ${lang}`;
  const text = lang === 'ko' ? '도움말' : 'Help';
  const notice = lang === 'ko' ? '새 탭에서 열림' : 'opens in a new tab';
  const group = root.querySelector(login ? '.login-top' : '.navbar-tools');
  const link = group?.querySelector(login ? '.help-link-login' : '.help-link');
  check(`${label}: one help link`, root.querySelectorAll('.help-link, .help-link-login').length === 1);
  check(`${label}: help is the first tool before language`, !!link && group.firstElementChild === link
    && link.nextElementSibling?.classList.contains('language-select'));
  check(`${label}: current-language URL`, link?.getAttribute('href') === `https://kayden-kim.github.io/vnc-relay/${lang}/`);
  check(`${label}: safe new tab`, link?.target === '_blank' && link?.getAttribute('rel') === 'noopener noreferrer');
  check(`${label}: visible label`, link?.querySelector('span:not(.sr-only)')?.textContent === text);
  check(`${label}: accessible new-tab notice`, link?.querySelector('.sr-only')?.textContent === ` (${notice})`
    && link?.textContent === `${text} (${notice})` && !link.hasAttribute('aria-label'));
  const icons = link?.querySelectorAll('svg') ?? [];
  check(`${label}: decorative help and new-tab icons`, icons.length === 2
    && [...icons].every((svg) => svg.getAttribute('aria-hidden') === 'true' && svg.getAttribute('focusable') === 'false')
    && icons[0].classList.contains('icon-md') && icons[1].classList.contains('icon-sm'));
  if (link) {
    const box = link.getBoundingClientRect();
    check(`${label}: visible and inside viewport`, getComputedStyle(link).display !== 'none'
      && box.width > 0 && box.height > 0 && box.left >= 0 && box.right <= innerWidth + 1);
    if (innerWidth < 1024) {
      check(`${label}: touch height`, box.height >= 44);
      if (!login) {
        const style = getComputedStyle(group);
        const brand = root.querySelector('.navbar-brand').getBoundingClientRect();
        check(`${label}: right-aligned tool wrapping`, style.flexWrap === 'wrap' && style.justifyContent === 'flex-end');
        check(`${label}: brand stays on the first tools row`, Math.abs(brand.top - group.getBoundingClientRect().top) < 1 && brand.height >= 44);
        const nav = root.querySelector('.navbar-nav').getBoundingClientRect();
        check(`${label}: navigation follows all tool rows`, nav.top >= group.getBoundingClientRect().bottom);
      }
    }
  }
  check(`${label}: no page overflow`, document.documentElement.scrollWidth <= document.documentElement.clientWidth);
  check(`${label}: no off-origin resource requests`, performance.getEntriesByType('resource')
    .every((entry) => new URL(entry.name, location.href).origin === location.origin));
}

export function checkHelpLanguageChange(root, lang, check, options) {
  checkHelpLink(root, lang, check, options);
  for (const next of [lang === 'ko' ? 'en' : 'ko', lang]) {
    const select = root.querySelector(options?.login ? '#login-language' : '.navbar-tools select');
    select.value = next;
    select.dispatchEvent(new Event('change'));
    checkHelpLink(root, next, check, options);
    check(`DASH-064 ${next}: document language updates immediately`, document.documentElement.lang === next);
  }
}
