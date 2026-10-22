# 대시보드 공통 컴포넌트 모듈 설계

대시보드 화면이 함께 쓰는 공통 조각(버튼, 입력, 배지, 경고, 카드, 지표, 게이지, 상단 메뉴, 언어 선택, 데이터 표, 링크, 대화상자)과 아이콘을 만든다. 각 조각은 DOM 요소를 돌려주는 ES 모듈 함수다. 상태 표현은 속성과 클래스로 하고, 색·크기·간격은 토큰 CSS의 `var(--…)`로 그린다. 이 모듈은 관리 API를 부르지 않고 상태를 저장하지 않는다.

| 항목 | 값 |
| --- | --- |
| 구조 시점 | Current |
| 문서 상태 | Draft |
| 구현 대조 | 대조함, [대조 기록](#대조-기록) |
| 코드 위치 | `web/dashboard/app/components/`, `web/dashboard/app/styles/`, `web/dashboard/preview/`, `web/dashboard/tests/` |
| 코드 지도 | [코드 지도 2절](code-map.md#2-코드-위치와-구성요소) |
| 관련 스펙 | [대시보드 화면 스펙](../specs/dashboard.md)의 화면 상태 표. 이 모듈은 화면이 아니라서 시나리오 ID를 가지지 않는다. |
| 관련 계약 | 없음 |
| 관련 ADR | 없음 |

## 1. 책임과 경계

- 맡는 책임:
  - 공통 조각을 만든다: 버튼, 텍스트 입력, 배지, 경고, 카드, 지표, 게이지, 상단 메뉴, 언어 선택, 데이터 표, 링크, 대화상자.
  - 아이콘 SVG를 만든다([프런트엔드 컨벤션 5절](../conventions/frontend.md)의 아이콘 규칙).
  - 상태를 속성(`disabled`, `aria-*`, `data-tone`)과 클래스(`is-hover`, `is-focus`)로 나타낸다.
  - 미리보기와 테스트 페이지가 같은 조각을 쓴다(복제하지 않는다).
- 맡지 않는 책임:
  - 관리 API 호출, 폴링, 세션 상태, 오류 분류: 화면 모듈(작업 4.10·4.11)이 맡는다.
  - 로그인 화면 조립: 작업 4.9가 맡았다. 조립은 `web/dashboard/login/`에 있고, 구성 요소는 `cargo xtask tokens`가 만든 복사본(`login/components/`, `login/styles/`)을 가져온다. 복사본은 직접 고치지 않는다(`login/`과 `app/`은 파일을 공유하지 않는다, 프런트엔드 컨벤션 1.1절).
  - 토큰 값 정의: [DESIGN.md](../design/DESIGN.md) YAML과 `cargo xtask tokens`가 맡는다.
  - 정적 파일 제공: 작업 4.6이 맡는다. `cargo xtask dashboard-serve`는 로컬 미리보기 전용이다.
  - 언어 선택 저장과 문구 번역: 화면 모듈이 맡는다.
- 다른 모듈에 공개하는 기능: `web/dashboard/app/components/` 아래 모듈의 함수. 아래 2절 표.

## 2. 내부 구조

화살표의 의미: 실행 중 호출(`import`와 함수 호출), 클래스·속성 연결(요소가 CSS 선택자와 맞는다), 토큰 참조(CSS가 `var(--…)`를 쓴다).

```mermaid
flowchart LR
    PRV["preview/preview.js<br/>미리보기 페이지"] -->|"호출"| CMP["app/components/*.js<br/>create 함수"]
    TST["tests/components.js<br/>테스트 페이지"] -->|"호출"| CMP
    LGN["login/login.js<br/>로그인 화면"] -->|"호출"| LCP["login/components/*.js<br/>app 복사본(생성 파일)"]
    CMP -->|"호출"| ICO["app/components/icons.js<br/>icon"]
    CMP -->|"클래스·속성 연결"| CSS["app/styles/base.css<br/>app/styles/components.css"]
    CSS -->|"토큰 참조"| TOK["app/tokens.css<br/>생성 파일"]
```

| 요소(파일·심볼) | 역할 | 의존 대상 |
| --- | --- | --- |
| `app/components/icons.js` `icon` | 이름으로 아이콘 SVG 하나를 만든다. 정적 SVG 문자열을 모듈 안 `<template>`에 한 번 넣고 `cloneNode`로 복제한다. | 없음 |
| `app/components/button.js` `createButton` | 버튼 요소를 만든다. | `icons.js` |
| `app/components/text-field.js` `createTextField`, `setTextFieldError` | 레이블, 입력, 오류 문구를 묶은 `div`를 만든다. `setTextFieldError`는 만든 필드의 오류 문구를 바꾼다. 오류 아이콘은 `icons.js`로 만든다. | `icons.js` |
| `app/components/badge.js` `createBadge` | 상태 배지를 만든다. | `icons.js` |
| `app/components/alert.js` `createAlert` | 상태 안내 상자를 만든다. | `icons.js` |
| `app/components/card.js` `createCard` | 제목이 있는 카드 `section`을 만든다. 개수는 배지로 보인다. | `badge.js` |
| `app/components/stat.js` `createStat` | 값 하나와 이름을 묶은 지표를 만든다. 값은 `metric` 글꼴을 쓴다. | 없음 |
| `app/components/gauge.js` `createGauge` | 원형 게이지를 `progressbar` 역할로 만든다. | 없음 |
| `app/components/navbar.js` `createNavbar` | 상단 막대와 호출자가 준 레이블의 `nav`를 만든다. | `link.js`, `icons.js` |
| `app/components/language-select.js` `createLanguageSelect` | 레이블이 있는 네이티브 `select`를 만든다. 지구본과 화살표 아이콘을 붙인다. | `icons.js` |
| `app/components/data-table.js` `createDataTable` | 표 요소를 만든다. 좁은 폭의 카드 배치는 CSS가 맡는다. | 없음 |
| `app/components/link.js` `createLink` | 링크 `a`를 만든다. | `icons.js` |
| `app/components/dialog.js` `createDialog` | 네이티브 `dialog`로 확인 대화상자를 만든다. | `button.js` |
| `app/styles/base.css` | `color-scheme`, 본문 글꼴, 포커스 링, 화면 밖 글 클래스 | `tokens.css` |
| `app/styles/components.css` | 조각별 클래스와 상태 선택자 | `tokens.css` |

## 3. 데이터 모델

모듈은 전역 상태가 없다. 입력은 옵션 객체, 출력은 DOM 요소다.

| 이름 | 값 | 제약 |
| --- | --- | --- |
| `tone` | `success`, `info`, `warning`, `destructive`, `neutral` | 다른 값이면 `TypeError`. 요소에 `data-tone`으로 쓴다. |
| `variant`(버튼) | `primary`, `outline`, `ghost`, `destructive`, `navbar` | 다른 값이면 `TypeError`. `navbar`는 상단 막대의 항목형(로그아웃) 버튼이다. |
| `variant`(언어 선택) | `navbar`, `form` | 다른 값이면 `TypeError` |
| 아이콘 이름 | `check`, `clock`, `warning`, `alert`, `refresh`, `logout`, `back`, `chevron-right`, `chevron-down`, `cancel`, `globe`, `grid`, `brand` | 다른 값이면 `TypeError` |
| 아이콘 크기 | `md`(`--space-icon-size`), `sm`(`--space-icon-size-sm`) | 기본값 `md` |
| 상태 클래스 | `is-hover`, `is-focus` | 미리보기가 상태를 고정할 때만 붙인다. 화면 코드는 붙이지 않는다. |
| 상태 속성 | `disabled`, `aria-busy`, `aria-invalid`, `aria-current`, `aria-describedby` | 상태 표현은 이 속성으로 한다. |

### 3.1 공개 함수

| 함수 | 옵션 | 반환 |
| --- | --- | --- |
| `icon(name, {size})` | 아이콘 이름, 크기 | `SVGSVGElement`. `aria-hidden="true"`, `focusable="false"`. |
| `createButton({variant, label, icon, disabled, busy, type})` | `variant` 기본 `primary`. `navbar`는 상단 막대 항목형 버튼이다(DESIGN.md 버튼 변형). `icon`은 아이콘 이름. `busy`는 진행 중 상태다. | `HTMLButtonElement`. `busy`이면 `disabled`와 `aria-busy="true"`를 붙인다. 문구는 호출자가 바꾼다. |
| `createTextField({id, label, type, value, required, autocomplete, helpText, error})` | `type`은 `text` 또는 `password`. `error`가 있으면 오류 상태다. 비활성 옵션은 없다(DESIGN.md 입력 상태는 기본·포커스·오류). 포커스는 CSS의 `:focus-visible`과 미리보기 클래스 `is-focus`로 나타난다. | `HTMLDivElement`. 오류가 있으면 `aria-invalid="true"`, `aria-describedby`에 오류 문구 id를 쓴다. |
| `setTextFieldError(field, error)` | `field`는 `createTextField`가 만든 `div`다. `error`가 있으면 오류 문구를 붙이고 `aria-invalid="true"`, `aria-describedby`를 오류 문구 id로 바꾼다. `error`가 빈 값이면 오류 문구를 지우고 `aria-invalid="false"`로 바꾼다. 도움말이 있으면 `aria-describedby`를 도움말 id로 되돌리고, 없으면 지운다. 로그인 화면이 필드를 다시 만들지 않고 오류만 바꾸려고 추가했다(4.9 재작업). | `undefined`. 요소를 새로 만들지 않고 같은 요소를 바꾼다. |
| `createBadge({tone, label, icon})` | `icon` 기본값은 `tone`별 아이콘(`neutral`은 없음). `icon: false`로 끈다. | `HTMLSpanElement` |
| `createAlert({tone, title, message})` | `warning`·`destructive`는 `role="alert"`, 나머지는 `role="status"` | `HTMLDivElement` |
| `createCard({title, headingLevel, count, actions, body, footer})` | `title` 필수. `headingLevel` 기본 2(1~6의 정수). `count`는 `createBadge`로 붙인다. | `HTMLElement`(`section`). `aria-labelledby`로 제목과 묶는다. |
| `createStat({label, value, unit, tone})` | `label` 필수. 값은 문자열 또는 숫자이며 `metric` 글꼴과 `tnum`을 쓴다. | `HTMLDivElement` |
| `createGauge({label, value, max, valueText, limitText, unknownText})` | `value`는 `0`~`max`. `label`과 `limitText`는 필수다. 알려진 값은 가운데에 `valueText`(없으면 값)를 크게 보이고, 아래 줄에 `limitText`(상한 줄)를 보인다. `value`가 `null`이면 알 수 없음이며 링을 그리지 않고 `unknownText`(필수)를 쓴다. 채움 길이는 `style.setProperty`로 쓴다. | `HTMLDivElement`. `role="progressbar"`, `aria-label`, `aria-valuemin`, `aria-valuemax`, `aria-valuenow`, `aria-valuetext`. 알려진 값의 `aria-valuetext`는 `valueText, limitText`다. |
| `createNavbar({brand, items, language, action, label})` | `label`은 필수다(기본값 없음, 호출자가 준다). `items`는 `{label, href, current, desktopOnly}`. `desktopOnly` 항목은 1024px 미만에서 숨긴다. | `HTMLElement`(`header`). 안에 `nav`를 둔다. 현재 항목은 `aria-current="page"`. |
| `createLanguageSelect({id, label, value, options, variant})` | `id`, `label`, `options`(`{value, label}` 배열, 빈 배열이면 `TypeError`)는 필수. `variant` 기본 `form`(`navbar`·`form`). | `HTMLDivElement`. 안에 `label`과 `select`. |
| `createDataTable({caption, columns, rows, emptyText, numericColumns})` | `columns`는 `{key, label}`. `rows`는 값 배열의 배열. | `HTMLTableElement`. 열 머리글은 `scope="col"`. 행이 없으면 `emptyText`를 한 행으로 쓴다. |
| `createLink({href, label, icon})` | `icon`은 아이콘 이름 | `HTMLAnchorElement` |
| `createDialog({title, description, confirmLabel, cancelLabel, destructive, onConfirm, onCancel})` | `title`, `confirmLabel`, `cancelLabel`은 필수. 확인 버튼은 `destructive`면 위험 스타일. 닫힌 대화상자는 CSS가 숨긴다(`display`는 `.dialog[open]`에만 준다). 미리보기는 열린 견본을 비모달 `open` 속성으로 프레임 안에 둔다(미리보기 전용). | `{element, show(), hide()}`. `show`는 `showModal()`을 부르고, 닫기와 취소는 네이티브 포커스 복귀를 쓴다. |

모든 함수는 API 값을 `textContent`나 속성 값으로만 넣는다. `innerHTML`에는 정적 문자열만 넣는다.

## 4. 처리 순서

1. 호출자가 함수를 `import`하고 옵션 객체를 넘긴다.
2. 함수가 옵션을 검사한다. 허용하지 않는 `tone`·`variant`·아이콘 이름, 빠진 필수 `label`이면 `TypeError`를 던진다. 이 오류는 개발 중 실수를 알리며 화면 오류로 바꾸지 않는다.
3. 함수가 요소를 만든다. 아이콘은 `icon`의 캐시 템플릿에서 복제한다.
4. 상태 속성과 클래스를 붙인다. 호출자는 반환된 요소를 DOM에 넣는다.
5. 상태가 바뀌면 호출자가 같은 요소의 속성을 바꾼다. 모듈은 다시 만들지 않는다. 대화상자는 `show()`와 `hide()`로 열고 닫는다.

## 5. 상태와 초기화

- 전역 상태: 아이콘 템플릿 캐시(`Map`, 처음 쓸 때 채운다)만 있다.
- 상태 표현: 호버와 포커스는 CSS 가상 클래스(`:hover`, `:focus-visible`)와 미리보기 클래스(`is-hover`, `is-focus`)를 함께 쓴다. 비활성은 `disabled`, 진행 중은 `aria-busy`, 오류는 `aria-invalid`다.
- 테마: 밝은 테마만 있다. `base.css`가 `color-scheme: light`를 준다.
- 움직임: 없다. 애니메이션, 전환 효과, 스피너를 쓰지 않는다([DESIGN.md 모션 절](../design/DESIGN.md#components)).

## 6. 오류 처리

- 개발 실수(허용하지 않는 옵션, 빠진 필수 값): `TypeError`를 던진다. 호출한 모듈이 처리하지 않는다. 테스트 페이지(`tests/components.js`)가 `navbar`의 빠진 `label`로 이 경로를 확인한다.
- API 값과 긴 값: 텍스트로만 넣는다. 한글은 `word-break: keep-all`, 긴 영어와 숫자는 `overflow-wrap: anywhere`로 줄인다.
- 네트워크 오류, 권한 오류는 이 모듈에 없다. 화면 모듈이 `createAlert`로 표시한다.

## 7. 설계 선택

| 선택 | 채택 | 이유 | 채택하지 않은 선택지 |
| --- | --- | --- | --- |
| 조각의 형태 | 요소를 돌려주는 함수 | 빌드와 프레임워크가 없다. 요소를 직접 검사하기 쉽다. | 사용자 정의 요소(`customElements`). 생명주기와 공개 표면이 늘어난다. |
| 스타일 위치 | `app/styles/` CSS 파일에 클래스로 둔다 | 미리보기와 화면이 같은 CSS를 직접 가져온다. | 조각마다 스타일 문자열을 넣는다. CSP가 막는다. |
| 상태 미리보기 | `is-hover`·`is-focus` 클래스 | 호버와 포커스를 재현할 수 있고 복제 CSS가 없다. | 미리보기 전용 CSS 복사본. 두 곳을 맞춰야 한다. |
| 좁은 폭의 표 | CSS로 카드처럼 배치하고 표 역할은 유지 | 역할과 읽기 순서를 그대로 쓴다. | 폭마다 다른 마크업. |
| 대화상자 | 네이티브 `dialog`와 `showModal()` | 포커스 가두기와 복귀를 브라우저가 한다. | 직접 만든 포커스 가두기. |
| 언어 선택 | 네이티브 `select` | 키보드 동작과 스크린 리더 이름을 브라우저가 맡는다. | 직접 만든 목록 상자. |
| 아이콘 | 인라인 SVG와 `<template>` 복제 | 외부 파일 요청이 없고 CSP를 지킨다. | 외부 스프라이트 `<use href>`. 외부 파일 요청이 생긴다. |

## 8. 미결정 사항

- (해결) 브라우저 시험(`tests/dashboard_browser.rs`)은 4.8에 넣었다. Windows와 WSL Ubuntu 모두에서 실행한다. Ubuntu에는 Google Chrome 안정판 .deb를 설치한다(사용자 결정 2026-10-22(Q41)). 브라우저가 없으면 시험은 실패한다(건너뛰지 않는다, 사용자 결정 2026-10-22(Q35)). 설치와 실행 기록은 [개발 환경 how-to](../how-to/set-up-and-verify.md#4-검증) 실행 기록 4에 있다.
- 성능 예산(FE-TBD-016)은 화면이 생긴 뒤 정한다(사용자 결정 2026-10-22(Q39)).
- 로그인(4.9)은 구성 요소의 생성 복사본을 가져온다. 로그인이 쓰려고 `setTextFieldError`를 추가했고 §3.1 표에 적었다. 운영 화면(4.10·4.11)이 조각을 더 요구하면 이 문서에 API를 추가한다.
- 로그인 열린 질문: 새로고침 후 계정명 복원, 로그인 요청 제한 시간 10초, 시안과의 차이. 사용자 결정이 없어 구현 판단으로 남긴다(검증 기록 4.2절).

## 대조 기록

대조일 2026-10-22, 환경 Windows 10.0.26200. 경로, 함수 이름, 상태 속성은 소스와 일치한다. 아래 표의 "차이"는 설계 문서를 코드에 맞춘 것이다.

| 대상 | 확인 방법 | 결과 |
| --- | --- | --- |
| 파일 경로(2절 표) | 13개 조각 파일, `app/styles/base.css`·`components.css`, `preview/`, `tests/components.js` 존재 확인 | 통과 |
| 공개 함수 이름과 옵션(3.1 표) | 각 `export function`의 옵션 구조 분해와 `TypeError` 검사를 대조 | 차이 7건을 고쳤다: card(`badge.js` 의존, 개수 배지), stat(`label` 필수), gauge(`limitText`·`unknownText` 추가), navbar(`label` 필수, "주 메뉴" 기본값 없음, `desktopOnly`), language-select(`icons.js` 의존, `variant` 기본 `form`), text-field(`icons.js` 의존, 2절 표), dialog(`title`·`confirmLabel`·`cancelLabel` 필수) |
| 아이콘 이름 13개와 크기 md·sm | `PATHS` 키, `SIZE_CLASS`, CSS `.icon-md`·`.icon-sm` | 통과 |
| 상태 속성 | `aria-busy`, `aria-current`, `aria-invalid`, `aria-describedby`, `aria-labelledby`, `role`(alert·status·progressbar) 검색 | 통과 |
| 상태 클래스 | `is-hover`·`is-focus`를 미리보기 `preview.js`만 붙이고 `app/components/`는 붙이지 않는지 검색 | 통과 |
| 색과 움직임 | `color-scheme: light`(base.css), 전환·애니메이션 검색 | 통과. 전환과 애니메이션은 없다 |
| 줄바꿈 | `word-break: keep-all`(base.css), `overflow-wrap: anywhere`(components.css) | 통과 |
| 미리보기와 테스트 호출 | `preview.js`와 `tests/components.js`가 `app/components/`의 함수를 import한다 | 통과 |
| 토큰 CSS 연결 | `cargo xtask tokens --check` | 통과(종료 코드 0) |
| 브라우저 시험 | `cargo test --test dashboard_browser` | 통과(시험 2개 ok). 미리보기와 테스트 페이지를 6폭 × ko/en 12회 열어 `data-result=pass`, CSP 위반 0, 가로 스크롤 0 |
| 리뷰 수정 A–D(Q42, 2026-10-22) | 3.1 표와 CSS를 다시 대조: 게이지 `aria-valuetext`(gauge.js L71), 버튼 `navbar` 변형(button.js), 입력 상태(옵션 없음), 대화상자 `display`(components.css `.dialog[open]`) | 차이 4건을 고쳤다. A 게이지: 가운데에 현재 값, 아래 "상한" 줄. DESIGN.md 게이지 절을 따랐고 리뷰의 "비율" 표기는 쓰지 않았다(결정 요청 참고). B 상단 막대 로그아웃: `navbar` 변형, 항목 1개(`bc6248e`, `fc7c325`). C 입력: 비활성 견본을 포커스 견본으로 바꿈(`e1ba857`). D 대화상자: 닫힌 상태는 숨기고, 열린 견본은 프레임 안에 둠(`0741c3c`, 시험 `4e65789`). |
| 로그인 복사본(4.9 재작업) | `cargo xtask tokens --check`, 복사본 변조·원본 변조·복사 목록 밖 파일 시험 | 통과. 깨끗한 상태는 통과. 복사본 변조, 원본 변조, 복사 목록 밖 파일 각각 `생성 결과와 다르다` 또는 `복사 목록에 없는 파일이다`로 실패하고 종료 코드 1이었다. 되돌린 뒤 다시 통과했다. |

3.1 표에 없는 문자열 필수 옵션(`button`의 `label`, `badge`의 `label`, `link`의 `href`·`label`, `text-field`의 `id`·`label`, `alert`의 `title`)도 빈 값이면 `TypeError`를 던진다.
