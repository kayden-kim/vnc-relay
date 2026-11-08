# VNC Relay 운영 대시보드 프런트엔드 컨벤션

채택된 디자인 기준을 `web/dashboard/`의 HTML·CSS·JavaScript에 일관되게 적용하는 규칙이다. 제품 동작은 [대시보드 화면 스펙](../specs/dashboard.md)에서, 코드의 책임과 내부 구조는 아키텍처 문서에서 정한다. 작업 0.9는 사용자 결정 2026-10-22(Q09)로 시작했고, 사용자 결정 2026-10-22(Q32~Q39)로 DESIGN-TBD-005(컨벤션과 검증 방법)를 해소했다. FE-TBD-016(성능 지표)은 사용자 결정 2026-11-07(Q159)로 지표 없이 닫았다. 결정 내용은 [8절](#8-미결정-사항)에 있다. 열린 항목은 없다.

| 항목 | 값 |
| --- | --- |
| 상태 | Active (2026-10-22. 사용자 결정 2026-10-22(Q09·Q32~Q39) 반영. FE-TBD-016은 사용자 결정 2026-11-07(Q159)로 닫았다) |
| 적용 대상 | `web/dashboard/` 아래 HTML·CSS·JavaScript, 토큰 생성·대조 코드(`xtask/`), 테스트 페이지(`web/dashboard/tests/`), 미리보기(`web/dashboard/preview/`), 시안의 토큰 블록(`docs/design/mockups/dashboard.html`) |
| 기준 버전 | Rust 1.99.1(`rust-toolchain.toml`). 대시보드 런타임 의존성 없음. 브라우저 검증에 쓴 Chromium 버전은 검증 기록에 적는다 |
| 관련 디자인 기준 | [운영 대시보드 디자인 시스템](../design/DESIGN.md) (Accepted, 2026-10-20) |
| 관련 아키텍처 문서 | [코드 지도](../architectures/code-map.md), [xtask 모듈 설계](../architectures/xtask.md) |
| 관련 계약 | [관리 API 계약 1.11절](../contracts/management-api-v1.md) (정적 경로·비인증 범위·응답 헤더) |
| 관련 ADR | 없음 |
| 적용 시작일 | 2026-10-22 |
| 규칙 준수 확인 방법 | `cargo xtask tokens --check`, 제공 표 대조 테스트(`tests/dashboard_assets.rs`, 작업 4.6), 브라우저 테스트, [검증 how-to](../how-to/verify-dashboard-frontend.md)의 수동 검토(6절) |

이 문서가 다루는 작업 0.9 항목은 다음과 같다.

| 작업 0.9 항목 | 이 문서의 절 |
| --- | --- |
| 토큰 CSS 생성과 대조 | [2절](#2-토큰-적용과-재사용) |
| 정적 파일 구조 | [1.1절](#11-정적-파일-구조) |
| 실행 파일 내장 | [1.2절](#12-실행-파일-내장) |
| 컴포넌트 미리보기 위치, 시안 대조 | [3절](#3-컴포넌트와-콘텐츠) |
| 접근성 검사 | [4절](#4-접근성반응형성능-확인) |
| 브라우저 검증과 검증 방법 | [6절](#6-검증-명령과-통과-기준) |

## 1. 기준과 선택

코드 규칙은 [컨벤션 작성 규칙의 우선순위](AGENTS.md#규칙-우선순위)를 따른다. 디자인·동작 기준의 충돌은 [기준 문서 적용과 충돌 처리](../AGENTS.md#기준-문서-적용과-충돌-처리)를 따른다.

| 선택 항목 | 채택 값·버전 | 근거·기준 위치 |
| --- | --- | --- |
| 프레임워크와 빌드 도구 | 없다. 빌드 단계, 번들러, 트랜스파일러를 쓰지 않는다. 브라우저가 읽는 HTML·CSS·JavaScript를 그대로 배포한다 | [PRD](../requirements/PRD.md) 대시보드 구현, [화면 스펙 1절](../specs/dashboard.md#1-범위와-사용자) |
| 스타일 적용 방식 | 외부 CSS 파일. 토큰 CSS는 DESIGN.md에서 생성하고, 화면 CSS는 `var(--…)`로만 토큰을 쓴다 | [DESIGN.md 정적 파일 원칙](../design/DESIGN.md#정적-파일-원칙), [2절](#2-토큰-적용과-재사용) |
| 컴포넌트 기반 | 외부 UI 라이브러리를 쓰지 않는다. ES 모듈 함수로 공통 컴포넌트를 만들고, 반복 조각은 `<template>` 요소로 둔다 | [PRD](../requirements/PRD.md) (JS 프레임워크 없음), [3절](#3-컴포넌트와-콘텐츠) |
| 아이콘 | 직접 그린 인라인 SVG다. 규격은 [DESIGN.md 아이콘 규칙](../design/DESIGN.md#아이콘-규칙)(크기 16px·14px, 선 두께 `spacing.icon-stroke`)을 따른다. 장식용 SVG는 `aria-hidden="true"`, `focusable="false"`를 붙인다 | [DESIGN.md 아이콘 규칙](../design/DESIGN.md#아이콘-규칙) |
| 미리보기 | 실제 컴포넌트를 불러 쓰는 정적 페이지(`web/dashboard/preview/`, 사용자 결정 2026-10-22(Q34)) | [3절 컴포넌트 미리보기](#컴포넌트-미리보기) |

선택이 비어 있으면 특정 프레임워크나 라이브러리를 기본값으로 설치하지 않는다. 기존 설정과 사용자 요청을 먼저 확인한다.

### 1.1 정적 파일 구조

대시보드 파일은 `web/dashboard/` 아래에 둔다. 제공 경로는 [관리 API 계약 1.11절](../contracts/management-api-v1.md)의 경로 표와 같아야 한다.

```text
web/dashboard/
├── fonts/            글꼴 원본(Pretendard 부분 글자 woff2 두 개와 라이선스 전문 LICENSE.txt). 제공하지 않는다
├── login/            인증 없이 제공한다 (SCR-003, 로그인 화면)
│   ├── index.html    제공 경로 /login/
│   ├── main.js
│   ├── login.js
│   ├── language.js   app/language.js의 복사본(로그인이 쓰는 것). 생성 산출물. 직접 고치지 않는다
│   ├── login.css
│   ├── components/   app/components/의 복사본(로그인이 쓰는 것). 생성 산출물. 직접 고치지 않는다
│   ├── fonts/        fonts/의 woff2 복사본. 생성 산출물. 직접 고치지 않는다
│   ├── styles/       app/styles/의 base.css·components.css 복사본. 생성 산출물. 직접 고치지 않는다
│   └── tokens.css    생성 산출물. 직접 고치지 않는다
├── app/              관리 세션이 있어야 제공한다 (SCR-001·SCR-002)
│   ├── index.html    제공 경로 /  (이 문서의 기본값, 사용자 결정 아님: /app/index.html은 제공하지 않는다)
│   ├── main.js
│   ├── language.js
│   ├── components/   공통 컴포넌트 ES 모듈
│   ├── fonts/        fonts/의 woff2 복사본. 생성 산출물. 직접 고치지 않는다
│   ├── styles/       화면 CSS
│   └── tokens.css    생성 산출물. 직접 고치지 않는다
├── preview/          개발용 미리보기. 실행 파일에 넣지 않는다
└── tests/            브라우저 테스트 페이지. 실행 파일에 넣지 않는다
```

위 목록은 예시다. 실제 파일 이름은 구현 때 정하되, 아래 규칙을 지킨다.

| 제공 경로 | 원본 파일 | 관리 세션 | 세션이 없을 때 |
| --- | --- | --- | --- |
| `GET /login/` | `login/index.html` | 불필요 | 해당 없음 |
| `GET /login/<파일>` | `login/<파일>` | 불필요 | 해당 없음 |
| `GET /` | `app/index.html` | 필요 | `303 See Other`, `/login/`으로 이동 |
| `GET /app/<파일>` | `app/<파일>` | 필요 | `303 See Other`, `/login/`으로 이동 |

- 제공 표에 없는 파일은 제공하지 않는다. `preview/`와 `tests/`는 제공 표에 넣지 않는다.
- `login/`에는 운영 데이터, 계정 정보, 설정 값을 넣지 않는다([계약 1.11절](../contracts/management-api-v1.md)).
- `login/`과 `app/`은 파일을 공유하지 않는다. 로그인 화면은 세션이 없으므로 `/app/` 파일을 받을 수 없다. 같은 내용이 두 곳에 필요하면 생성 도구가 두 곳에 같은 내용으로 쓰고, 대조 명령이 두 파일의 같음을 확인한다. 로그인 화면이 쓰는 공통 컴포넌트와 스타일도 같은 방식이다(2.3절).
- 파일 이름은 소문자와 하이픈(kebab-case)으로 쓴다. 허용 확장자는 `.html`, `.css`, `.js`다. 다른 형식이 필요하면 이 절을 먼저 고친다.

### 1.2 실행 파일 내장

제공 파일은 실행 파일에 들어 있어야 한다([PRD](../requirements/PRD.md), [DESIGN.md 정적 파일 원칙](../design/DESIGN.md#정적-파일-원칙)).

- 제공 파일은 `include_bytes!`로 실행 파일에 넣는다. 파일이 없으면 빌드가 실패한다.
- 제공 경로, 원본 파일, `Content-Type`을 한 Rust 파일의 표에 적는다. 표에 없는 파일은 넣지 않는다.
- `Content-Type`은 확장자로 정한다: `text/html; charset=utf-8`, `text/css; charset=utf-8`, `text/javascript; charset=utf-8`, `font/woff2`.
- 표 대조 테스트는 `login/`과 `app/`을 걸어 실제 파일 목록과 표를 비교한다. 표에 없는 파일이나 표에만 있는 항목이 있으면 실패한다. 제공 표에 `preview/`나 `tests/` 경로가 있어도 실패한다.
- 정적 응답에는 계약 1.11절의 헤더를 붙인다. `Cache-Control: no-store`는 보호된 정적 응답과 로그인 화면 응답 모두에 붙인다(사용자 결정 2026-11-07(Q155)). `Content-Security-Policy: default-src 'self'; frame-ancestors 'none'`은 필수다(사용자 결정 2026-10-22(Q37)). `X-Content-Type-Options: nosniff`와 `Referrer-Policy: no-referrer`는 계약에서 권장이다.
- 내장 방법은 `include_bytes!` 표와 대조 테스트다(사용자 결정 2026-10-22(Q33)). `build.rs`와 외부 crate는 쓰지 않는다.

### 1.3 HTML·CSS·JavaScript 작성 규칙

다음 규칙은 CSP `default-src 'self'`를 전제로 한다. CSP는 필수다([계약 1.11절](../contracts/management-api-v1.md), 사용자 결정 2026-10-22(Q37)). 계약 1.11절은 대시보드가 인라인 스크립트를 쓰지 않는다고 정한다.

- HTML 루트에 `lang`을 둔다. 언어 선택에 따라 값을 바꾼다([화면 언어](../specs/dashboard.md#화면-언어)).
- 스크립트는 `<script type="module" src="…">`로만 불러온다. 인라인 스크립트, `on…` 이벤트 속성, `javascript:` 주소를 쓰지 않는다.
- 스타일은 `<link rel="stylesheet">`로 같은 출처의 파일만 불러온다. `<style>` 요소와 `style` 속성을 쓰지 않는다(사용자 결정 2026-10-22(Q37)). 시안(`docs/design/mockups/dashboard.html`)의 `<style>`은 시안 파일 안에서만 쓴다.
- 스크립트로 값을 넣을 때는 `element.style.setProperty()`를 쓴다. 이 방식이 CSP에서 막히는지는 브라우저 테스트로 확인한다.
- 외부 자원을 쓰지 않는다. CDN, 외부 글꼴, 외부 이미지, 외부 스크립트, 외부 `@import`를 금지한다. 요청은 같은 출처의 제공 경로와 `/api/v1/`만 보낸다.
- API에서 받은 값은 `textContent`나 속성으로 넣는다. `innerHTML`에 API 값을 넣지 않는다.
- 폴링 주기와 조회 제한 시간은 코드에 상수로 쓰지 않는다. [관리 API 계약 1.12절](../contracts/management-api-v1.md#112-폴링-주기와-조회-제한-시간의-전달)의 전달 방법을 따른다. 기본값과 범위는 [설정 계약 1.2절](../contracts/config-v3.md#12-키-표)이 정한다.
- JavaScript는 ES 모듈만 쓴다. 번들러, 트랜스파일러, npm 패키지를 쓰지 않는다. 문법 오류는 브라우저 테스트가 찾는다.

## 2. 토큰 적용과 재사용

| 대상 | 해야 할 것 | 피해야 할 것 | 판정 방법 |
| --- | --- | --- | --- |
| 공통 시각 값 | DESIGN.md의 YAML 머리말을 원본으로 쓰고, 생성된 토큰 CSS의 변수로 참조한다. | 같은 색·간격 값을 화면 CSS마다 따로 적는다. | 화면 CSS 검사(2.2절)와 `cargo xtask tokens --check` |
| 생성된 스타일 | 원본을 고치고 `cargo xtask tokens`로 다시 쓴다. | 생성 파일을 손으로 고쳐 원본과 다르게 만든다. | `cargo xtask tokens --check`의 차이 출력 |
| 공통 컴포넌트 | `app/components/`의 기존 컴포넌트와 변형을 먼저 쓴다. | 같은 용도의 컴포넌트를 화면마다 복제한다. | 호출 위치 검토와 미리보기 |
| 새 공통 스타일 | DESIGN.md에 사용 조건을 적은 뒤 변형을 추가한다. | 기존 변형을 설명 없이 덮어쓴다. | 문서와 미리보기 대조 |
| 화면별 예외 | DESIGN.md의 [예외 절](../design/DESIGN.md#예외)에 범위와 근거를 연결한다. | 추적할 수 없는 개별 스타일을 쌓는다. | 예외 ID와 적용 범위 검토 |

이 규칙은 공통 기준으로 관리하는 값에 적용한다. 화면 배치의 근거는 관련 스펙에 연결한다.

| 토큰 관리 항목 | 프로젝트 기준 |
| --- | --- |
| 원본 참조 | [DESIGN.md](../design/DESIGN.md) 첫 `---`부터 둘째 `---`까지의 YAML 머리말. 본문의 표는 원본이 아니다 |
| 참조·생성 관계 | DESIGN.md YAML → `web/dashboard/login/tokens.css`, `web/dashboard/app/tokens.css`(같은 내용) → 화면 CSS가 `var(--…)`로 참조 → 시안의 토큰 블록(표식 사이, 사용자 결정 2026-10-22(Q32)). 글꼴 원본 `web/dashboard/fonts/` → `app/fonts/`, `login/fonts/`(같은 바이트) |
| 생성 방법 | `cargo xtask tokens`가 원본을 읽어 `:root` 블록과 Pretendard `@font-face` 규칙을 쓰고 글꼴 파일을 복사한다. 도구 형태와 커밋 방식은 사용자 결정 2026-10-22(Q32)다. 생성 파일은 저장소에 커밋한다 |
| 대조 방법 | `cargo xtask tokens --check`가 생성 결과와 두 CSS 파일, 시안 표식 사이 줄, 글꼴 복사본의 바이트를 비교한다. 차이가 있으면 파일과 줄 번호를 출력하고 종료 코드 1로 끝난다 |
| 생성 대상 밖의 규칙 | 중단점과 미디어 쿼리는 [DESIGN.md 중단점 절](../design/DESIGN.md#중단점과-모바일-배치)을 따른다. 화면 배치 미디어 쿼리는 `web/dashboard/app/styles/components.css`에만 둔다(`app.css`에는 `@media`가 없다). 로그인 복사본은 2.3절의 복사 규칙을 따른다. 모션은 없다([DESIGN.md 모션 절](../design/DESIGN.md#모션)). 대비 기준은 [DESIGN.md 대비 기준](../design/DESIGN.md#대비-기준)을 따른다. 글꼴 굵기는 400·700만 쓴다. 글꼴 단계를 쓰는 규칙은 `font-family`와 `letter-spacing`을 함께 그 단계 변수로 정한다([DESIGN.md Typography](../design/DESIGN.md#typography)) |

### 2.1 원본 읽기와 이름 규칙

- 허용 키는 `colors`, `typography`, `spacing`, `rounded`다. `version`, `name`, `description`과 `components`는 CSS로 내보내지 않는다. `components`는 원본 참조용이다(사용자 결정 2026-10-22(Q32)).
- `typography`의 단계 아래에는 `fontFamily`, `fontSize`, `fontWeight`, `lineHeight`, `letterSpacing`, `fontFeature`만 쓴다. `fontFeature` 말고는 모든 단계에 있어야 한다. `letterSpacing`은 사용자 지시(2026-11-07)로 더했다.
- 허용하지 않는 키, 중복된 키, 해석할 수 없는 줄이 있으면 생성과 대조를 멈추고 종료 코드 1로 끝난다.
- 값은 YAML 값을 그대로 쓰고, 바깥 따옴표만 벗긴다. 값 안의 따옴표(`'tnum'`, 글꼴 이름)는 그대로 둔다. `#`으로 시작하는 색 값은 따옴표로 감싸야 한다. YAML에서 `#`은 주석 시작이다. 현재 원본은 이 규칙을 따른다(예: `overlay: "#0d1a1580"`).
- 이름은 DESIGN.md의 [CSS 사용자 정의 속성 이름 규칙](../design/DESIGN.md#css-사용자-정의-속성-이름-규칙)을 따른다.

| 원본 | CSS 이름 |
| --- | --- |
| `colors.<이름>` | `--color-<이름>` |
| `spacing.<이름>` | `--space-<이름>` |
| `rounded.<이름>` | `--radius-<이름>` |
| `typography.<단계>.fontFamily·fontSize·fontWeight·lineHeight·letterSpacing·fontFeature` | `--type-<단계>-family·size·weight·line·tracking·feature` |

- 출력은 `:root { … }` 블록 하나와 그 뒤의 `@font-face` 규칙들이다. 한 줄에 한 속성을 쓴다. 속성 순서는 원본 순서를 따른다. 첫 줄에 `Generated file. Do not edit. Source: <경로>` 형식의 주석을 쓴다.
- `@font-face`는 2.4절의 글꼴 파일마다 하나다. 글꼴 이름 `Pretendard`, 굵기, `font-display: swap`, `src: url("<기준>/<파일>") format("woff2")`를 쓴다. 기준은 두 토큰 파일에서 `fonts`(토큰 파일 옆의 `fonts/`)이고, 시안에서 `../../../web/dashboard/fonts`(글꼴 원본)다. 그래서 시안 블록은 이 기준만 토큰 파일과 다르다.

### 2.2 화면 CSS 검사

`cargo xtask tokens --check`는 토큰 파일을 뺀 화면 CSS도 검사한다. 다음 항목이 있으면 실패한다.

- 16진수 색(`#`과 3·4·6·8자리 수), `rgb(`, `hsl(`
- `url(`, `@import`
- 길이 리터럴(`px`, `em`, `rem`). 미디어 쿼리 조건에서만 쓴다. 조건의 값은 DESIGN.md 중단점 절을 따른다.
- `font-weight`에 토큰 변수나 `inherit`이 아닌 값
- 정의되지 않은 `var(--…)`

`url(`이 필요한 `@font-face`는 생성 토큰 파일에만 있다. 화면 CSS에는 계속 `url(`을 쓰지 않는다.

### 2.3 로그인 화면의 공통 복사본

로그인 화면은 `app/`의 공통 컴포넌트와 스타일을 복사해 쓴다. 복사본은 손으로 만들지 않는다. 복사 방식과 복사 목록은 구현 판단이다(사용자 결정 없음).

- 복사 목록은 `xtask/src/tokens/shared.rs`의 `SHARED_FILES`다. 현재 목록은 `components/icons.js`, `components/alert.js`, `components/button.js`, `components/text-field.js`, `components/language-select.js`, `styles/base.css`, `styles/components.css`, `language.js`다.
- `cargo xtask tokens`가 `app/`의 원본을 `login/components/`, `login/styles/`, `login/language.js`에 같은 내용으로 쓴다. 첫 줄 주석은 `Generated file. Do not edit. Source: <경로>`다. CSS는 `/* … */`, JavaScript는 `// …` 형식이다.
- `cargo xtask tokens --check`는 복사본마다 생성 결과와 실제 파일을 비교한다. 다르면 차이를 알린다. 없거나 읽을 수 없으면 파일 경로와 함께 오류로 멈춘다. 복사 목록에 없는 파일이 `login/components/`나 `login/styles/`에 있으면 차이로 집계한다.
- 복사본도 화면 CSS 검사(2.2절)와 제공 표 대조(1.2절)의 대상이다. 복사본은 원본과 같으므로 두 검사를 그대로 통과한다.
- 원본을 고치면 `cargo xtask tokens`로 복사본을 다시 쓴다. 로그인 화면에만 다른 모양이 필요하면 복사본을 고치지 않고, 원본 컴포넌트에 옵션을 추가한 뒤 [xtask 모듈 설계](../architectures/xtask.md)와 컴포넌트 모듈 설계에 적는다.

### 2.4 글꼴 파일

글꼴 원본은 `web/dashboard/fonts/`의 `Pretendard-Regular.subset.woff2`(400), `Pretendard-Bold.subset.woff2`(700), `LICENSE.txt`(SIL Open Font License 1.1 전문)다. Pretendard 1.3.9 배포본(`dist/web/static/woff2-subset/`, `dist/LICENSE.txt`)을 바꾸지 않고 넣었다(사용자 결정 2026-11-07(Q139)). 파일 목록과 굵기는 `xtask/src/tokens/fonts.rs`의 `FONT_FACES`다.

- `cargo xtask tokens`는 woff2 원본을 `app/fonts/`와 `login/fonts/`에 바이트 그대로 복사한다. 머리 주석은 붙이지 않는다. 로그인 화면은 인증 없이 받아야 하므로 `app/`과 따로 둔다(1.1절).
- `cargo xtask tokens --check`는 복사본의 바이트가 원본과 다르거나, 두 `fonts/` 폴더에 목록 밖 파일이 있으면 차이로 집계한다.
- `web/dashboard/fonts/`는 제공하지 않는다. 시안은 이 원본을 상대 경로로 읽는다.
- `cargo xtask notices`는 `LICENSE.txt` 전문을 배포 고지 `THIRD-PARTY-NOTICES.txt`에 넣는다([라이선스 검사 how-to](../how-to/check-dependency-licenses.md)).
- 글꼴을 바꾸려면 원본과 `FONT_FACES`를 같은 변경에서 고치고 `cargo xtask tokens`와 `cargo xtask notices`를 다시 실행한다.

## 3. 컴포넌트와 콘텐츠

- 변형·크기·시각 상태는 DESIGN.md의 [컴포넌트 표](../design/DESIGN.md#components)를 따른다.
- props, 책임과 내부 상태 관리는 모듈 설계를 따른다.
- 사용자 행동과 오류·로딩·빈 결과·권한 상태는 관련 스펙을 따른다.
- 필요한 변형과 상태는 실행 가능한 컴포넌트 예시로 연결한다.
- 실제 서비스 문구와 비슷한 한글·숫자·긴 문자열로 표시를 확인한다.

좋음: 기존 버튼 컴포넌트의 문서화된 변형으로 주요 행동을 표시한다.

나쁨: 화면마다 버튼의 색과 패딩을 새로 정한다.

이유: 공통 값과 변형을 재사용하면 디자인 기준을 바꿀 때 영향 범위를 확인할 수 있다. 컴포넌트 이름은 예시이며 실제 코드의 이름을 사용한다.

### 컴포넌트 미리보기

미리보기는 `web/dashboard/preview/`에 둔다. 미리보기 페이지는 `app/components/`, `app/tokens.css`, `app/styles/`를 그대로 가져와 쓴다. 미리보기용으로 컴포넌트나 스타일을 복제하지 않는다.

| 미리보기 관리 항목 | 프로젝트 기준 |
| --- | --- |
| 구현과 실행 위치 | `web/dashboard/preview/` 아래 정적 HTML 페이지. 실행은 `cargo xtask dashboard-serve`로 하는 로컬 전용 서버로 한다(사용자 결정 2026-10-22(Q34)). 브라우저는 모듈 스크립트를 `file://`로 읽지 않으므로 서버가 필요하다 |
| 재사용 대상 | `web/dashboard/app/components/`, `web/dashboard/app/tokens.css`, `web/dashboard/app/styles/`. 원본은 한 곳이다 |
| 확인할 상태 | DESIGN.md 컴포넌트 표의 항목별 기본·호버·비활성·포커스 상태. 오류·빈 값·긴 한글·긴 영문·긴 숫자(`data` 단계) 상태 |
| 확인 환경 | 폭 1280·1024·1023·768·390·320px, 한국어·영어 문구, 라이트 테마 |
| 배포·공유 범위 | 실행 파일에 넣지 않는다. 제공 표에 없다(1.2절 대조 테스트가 확인한다) |
| 검증과 증거 | 테스트 페이지의 결과(`data-result`), 폭별 스크린샷(저장소에 넣지 않고 `target/`에 둔다), 검증 기록 |

기본 컴포넌트, 컴포넌트 조합, 화면 순서로 확인한다. 예를 들어 `button`과 `input`을 확인한 뒤 필드 묶음과 로그인 폼을 확인한다. 검증을 위해 제품에 없는 상태나 기능을 추가하지 않는다.

### 화면 시안과 대조

- HTML 시안은 디자인 기준과 화면 구성을 보여 주는 산출물이다([디자인 문서 작성 규칙](../design/AGENTS.md#html-시안)). 시안은 동작·상태·검증의 기준이 아니다([화면 스펙 서문](../specs/dashboard.md)).
- 시안과 스펙 문장이 다르면 스펙 문장을 따른다.
- 대조는 같은 화면 상태와 같은 폭에서 구현 스크린샷과 시안 스크린샷을 나란히 두고 한다. 차이를 검증 기록에 적는다. 자동 픽셀 비교는 쓰지 않는다. 스크린샷은 `target/`에 두고 사람이 비교한다(사용자 결정 2026-10-22(Q36)).
- 시안의 토큰 블록은 시작·끝 표식으로 감싸고, 생성 결과와 같아야 한다. `cargo xtask tokens --check`가 이 표식 사이를 대조한다(사용자 결정 2026-10-22(Q32)).

## 4. 접근성·반응형·성능 확인

접근성은 다양한 입력 방식과 보조 기술로 UI를 쓸 수 있는지를 뜻한다. 자동 검사 결과만으로 전체 기준 준수를 선언하지 않는다.

| 대상 | 적용 기준 | 확인 방법 |
| --- | --- | --- |
| 접근성 수준 | WCAG 2.2 AA([DESIGN.md 접근성 기준](../design/DESIGN.md#접근성-기준)). 동작 요구 범위는 [화면 스펙 6절](../specs/dashboard.md#6-화면-크기와-접근성) | 자동: 테스트 페이지가 역할·이름·속성을 확인한다. 수동 체크리스트(1차, 사용자 결정 2026-10-22(Q36)): 키보드 전체 순회와 스크린 리더 확인. 해당 시나리오는 부분으로 본다 |
| 키보드와 포커스 | 포커스 표시는 토큰 `spacing.focus-ring-width`와 `spacing.focus-ring-offset`, 링 색 `colors.ring`(상단 막대 안은 `colors.navbar-ring`). 대화상자 포커스 규칙은 [화면 스펙 6절](../specs/dashboard.md#6-화면-크기와-접근성) | 자동: 포커스 순서, 대화상자 열기·닫기 뒤 포커스 위치. 수동: Tab 순회, Esc로 닫기, 기록 |
| 이름과 의미 | 시맨틱 HTML. 상단 막대 메뉴는 `nav`(이름 "주 메뉴"), 본문은 `main`. 카드 배치에서도 `table`·`row`·`cell`·`columnheader` 역할을 명시한다([DESIGN.md 중단점 절](../design/DESIGN.md#중단점과-모바일-배치)) | 자동: 역할과 이름. 수동: 접근성 트리 확인 |
| 대비와 의미 전달 | DESIGN.md [대비 기준](../design/DESIGN.md#대비-기준) 표. 상태는 글씨 레이블과 도형으로 구분한다 | 토큰 쌍은 WCAG 상대 휘도 공식으로 다시 계산하고 값을 검증 기록에 적는다. 색 외 표시는 수동 확인 |
| 반응형과 확대 | 폭 1280·1024·1023·768·390·320px. 200% 확대는 640px과 같다([DESIGN.md 중단점 절](../design/DESIGN.md#중단점과-모바일-배치)) | 자동: 폭별로 Chromium을 열어 `scrollWidth`가 `clientWidth`를 넘지 않는지 확인. 대상 크기는 수동 측정 |
| 모션 | 없다([DESIGN.md 모션 절](../design/DESIGN.md#모션)). 모션을 추가하기 전에 DESIGN.md를 먼저 고친다 | 해당 없음. 추가하면 움직임 감소 설정 처리를 이 표에 추가한다 |
| 성능 | 성능 지표와 예산을 두지 않는다. 확인한 PRD에 대시보드 성능 요구가 없고, 대시보드는 루프백·소수 관리자용이다. 성능 통과 조건은 없다(사용자 결정 2026-11-07(Q159), [FE-TBD-016](#8-미결정-사항)) | 해당 없음 |

## 5. 예외와 외부 자료

- 시각 예외는 [DESIGN.md 예외 절](../design/DESIGN.md#예외)을 따른다. 현재 예외가 없다.
- 코드 예외가 생기면 `FE-EX-001`부터 번호를 매기고, 범위·근거·재검토 조건을 이 절에 적는다. 현재 예외가 없다.
- DESIGN.md는 shadcn/ui의 시각 관례만 참고했다. shadcn/ui와 Tailwind CSS 패키지는 쓰지 않는다([참고 자료](../design/DESIGN.md#참고-자료), [정적 파일 원칙](../design/DESIGN.md#정적-파일-원칙)).
- 참고 이미지는 시각 방향만 참고한다. 이미지 자산과 문구는 복제하지 않는다([DESIGN.md 참고 자료](../design/DESIGN.md#참고-자료)).

## 6. 검증 명령과 통과 기준

절차는 [대시보드 프런트엔드를 검증하는 방법](../how-to/verify-dashboard-frontend.md)(Draft)이 정한다. 결과와 증거는 [검증 기록](../AGENTS.md#검증-기록)을 따른다.

- 실행 환경: [개발 환경·검증 how-to](../how-to/set-up-and-verify.md)의 [전제 조건](../how-to/set-up-and-verify.md#2-전제-조건)이 정한 지원 환경(Windows 11 x64·PowerShell, Ubuntu 24.04 LTS x64·bash)과 고정 Rust 툴체인(1.99.1). Chromium(Chrome 또는 Edge)은 두 OS 모두에 필수다. 없거나 실행되지 않으면 브라우저 시험은 실패한다(사용자 결정 2026-10-22(Q35)). Chromium 실행 파일 경로는 환경 변수 `DASHBOARD_BROWSER`로 준다(이 문서의 기본값, 사용자 결정 아님). Node.js는 선택 항목인 DESIGN lint에만 필요하다(사용자 결정 2026-10-22(Q38)).
- 실행 위치: 저장소 루트.
- 명령 확인 근거: `xtask/src/main.rs`에 `tokens [--check]`와 `dashboard-serve`가 있다. `tests/dashboard_browser.rs`는 컴포넌트·목 응답 시험을 맡는다. 실서버 화면 시험은 CDP(Chromium의 원격 조작 프로토콜)로 헤드리스 브라우저를 조작한다(사용자 결정 2026-11-05(Q112)). 승인된 `tokio-tungstenite`를 개발 의존성으로 재사용하며 새 crate나 외부 드라이버는 추가하지 않는다. 제공 표 대조는 `tests/dashboard_assets.rs`가 맡는다.

| 변경 범위 | 확인 명령·방법 | 통과 조건 | 절차와 증거 위치 |
| --- | --- | --- | --- |
| 디자인 토큰·문서 | `cargo xtask tokens --check`. 선택: DESIGN.md lint(`@google/design.md`, 고정 버전. 사용자 결정 2026-10-22(Q38)) | 종료 코드 0. 차이 0건, 금지 항목 0건. lint 오류 0건. 경고 수는 기록한다 | [검증 how-to 3.1절](../how-to/verify-dashboard-frontend.md) |
| 정적 파일과 내장 | 제공 표 대조 테스트(`tests/dashboard_assets.rs`) | 실제 파일과 표가 일치한다. 금지 경로가 없다 | [검증 how-to 3.2절](../how-to/verify-dashboard-frontend.md) |
| 컴포넌트·스타일 | 브라우저 테스트(`tests/dashboard_browser.rs`, 사용자 결정 2026-10-22(Q35))와 화면 CSS 검사(2.2절) | 테스트 페이지의 `data-result`가 `pass`다 | [검증 how-to 3.3절](../how-to/verify-dashboard-frontend.md) |
| 사용자 동작 | 브라우저 테스트와 `cargo xtask scenarios` | 시나리오의 모든 기대 결과를 확인해야 비부분 표식을 붙인다. 실서버 시험은 실제 로그인·조회·중계·종료 경로를 연결한다. 실패·지연·응답 유실은 시험용 전달 프록시로 만든다(사용자 결정 2026-11-05(Q113)). 일반 데이터는 실서버 응답을 쓰며, 주입한 오류 응답을 제품에서 발생한 오류로 보고하지 않는다. 브라우저별 호환성과 사람의 접근성 확인은 DASH-015·DASH-023의 별도 확인 범위다 | [테스트 컨벤션 7절](testing.md#7-테스트와-스펙-시나리오-연결), `target/scenario-report.md` |
| 시각 결과·접근성 | 폭별 스크린샷, 시안과 나란히 대조, 키보드·스크린 리더 수동 확인 | 스펙과 DESIGN.md 기준을 만족한다. 검토자가 기록한다 | [검증 how-to 3.4~3.7절](../how-to/verify-dashboard-frontend.md) |
| 전체 완료 | [개발 환경·검증 how-to 3.4절](../how-to/set-up-and-verify.md)의 명령과 위 명령 | 모든 필수 검사가 종료 코드 0이다 | [개발 환경·검증 how-to](../how-to/set-up-and-verify.md) |

시행하지 않은 검사는 성공으로 표시하지 않는다. 미실행 사유와 영향을 검증 기록에 적는다.

## 7. 기존 코드 적용

`web/dashboard/`는 작업 4.8(구성 요소)과 작업 4.9(로그인 화면)에서 만들었다. 기존 제품 코드와 충돌하는 부분이 없다. 목업의 `<style>`은 시안 파일이며 제품 파일이 아니므로 CSP 적용 대상이 아니다. 채택 뒤 되돌릴 때는 생성 산출물을 `cargo xtask tokens`로 다시 쓰거나, 변경한 파일을 이전 커밋 상태로 되돌린다.

## 8. 미결정 사항

작업 0.9는 사용자 결정 2026-10-22(Q09)로 시작했다. FE-TBD-001~015는 사용자 결정 2026-10-22(Q32)~(Q38)로 정했다. FE-TBD-016은 사용자 결정 2026-10-22(Q39)로 보류했다가 사용자 결정 2026-11-07(Q159)로 지표 없이 닫았다. 열린 항목은 없다.

| ID | 항목 | 영향 범위 | 결정 | 확정 위치 |
| --- | --- | --- | --- | --- |
| FE-TBD-001 | 토큰 생성·대조 도구 형태 | 토큰 파일, `xtask/` | 사용자 결정 2026-10-22(Q32): `cargo xtask tokens`와 `--check` 하위 명령 | 이 문서 2절, [xtask 모듈 설계](../architectures/xtask.md) |
| FE-TBD-002 | 생성 산출물을 저장소에 커밋할지 | 빌드, 토큰 파일 | 사용자 결정 2026-10-22(Q32): 커밋한다 | 이 문서 2절 |
| FE-TBD-003 | 실행 파일 내장 방법 | 관리 리스너, `Cargo.toml` | 사용자 결정 2026-10-22(Q33): `include_bytes!` 표와 대조 테스트 | 이 문서 1.2절 |
| FE-TBD-004 | 미리보기 위치와 실행 방법 | `preview/`, `xtask/` | 사용자 결정 2026-10-22(Q34): `web/dashboard/preview/`와 `cargo xtask dashboard-serve`(로컬 전용) | 이 문서 3절, [xtask 모듈 설계](../architectures/xtask.md) |
| FE-TBD-005 | 브라우저 테스트 방식과 보고서 연결 | `tests/`, 개발 의존성, 시나리오 보고 | 컴포넌트·목 응답 시험은 `--headless --dump-dom`과 `data-result`를 유지한다(Q35). 실서버 시험은 직접 만든 CDP 클라이언트로 조작한다(사용자 결정 2026-11-05(Q112)). 시험용 전달 프록시로 실패·지연·유실을 만들고 Si 만료는 실제 300초를 기다린다(Q113). `/// spec:` 표식은 Rust 시험에 두며 스캐너는 바꾸지 않는다 | 이 문서 6절, [테스트 컨벤션 7절](testing.md#7-테스트와-스펙-시나리오-연결) |
| FE-TBD-006 | Chromium 필요 여부와 없을 때 처리 | 검증 환경, 검증 how-to | 사용자 결정 2026-10-22(Q35): Chrome 또는 Edge를 두 OS 모두 필수로 둔다. 없으면 시험이 실패한다 | 이 문서 6절, [개발 환경 how-to 2절](../how-to/set-up-and-verify.md) |
| FE-TBD-007 | 키보드·포커스·스크린 리더 확인의 자동화 범위 | 접근성 검증 | 사용자 결정 2026-10-22(Q36): 1차는 수동 체크리스트다. 해당 시나리오는 부분으로 본다. 스크린 리더 확인은 작업 4.12 완료 조건에서 뺐다(사용자 결정 2026-11-07(Q137)) | 이 문서 4절 |
| FE-TBD-008 | 시각 비교 방법 | 스크린샷 검증 | 사용자 결정 2026-10-22(Q36): 스크린샷을 `target/`에 두고 사람이 시안과 비교한다. 자동 픽셀 비교는 쓰지 않는다. 작업 4.12는 시안과 구현을 나란히 놓은 비교 페이지를 사용자가 검토한다(사용자 결정 2026-11-07(Q138)) | 이 문서 3절, [검증 how-to 3.4절](../how-to/verify-dashboard-frontend.md#34-폭별-스크린샷과-시안-대조-수동) |
| FE-TBD-009 | 자동 검증 브라우저 범위와 수동 범위 | 검증 기록 | 사용자 결정 2026-10-22(Q36): 자동은 Chromium만 쓴다. Firefox는 수동으로 확인하고 버전을 기록한다. Safari 확인은 Playwright WebKit 자동 확인으로 대신한다(사용자 결정 2026-11-07(Q135)). 실제 Safari가 아니며 저장소 밖 검증 도구다 | 이 문서 4·6절, [검증 how-to 3.7절](../how-to/verify-dashboard-frontend.md#37-다른-브라우저와-실기기-확인-수동) |
| FE-TBD-010 | CSP 필수 여부와 인라인 스타일 금지 | 응답 헤더, 계약 | 사용자 결정 2026-10-22(Q37): CSP를 필수로 둔다. 인라인 `<style>`과 `style` 속성을 금지한다 | 이 문서 1.2·1.3절, [관리 API 계약 1.11절](../contracts/management-api-v1.md) |
| FE-TBD-011 | DESIGN.md `components` 블록의 내보내기 | 토큰 파일 | 사용자 결정 2026-10-22(Q32): CSS로 내보내지 않는다. 원본 참조용이다 | 이 문서 2.1절 |
| FE-TBD-012 | DESIGN lint 도구의 검증 절차 내 위치 | 문서 검증 | 사용자 결정 2026-10-22(Q38): 선택 항목이다. `@google/design.md` 버전을 고정하고 결과를 기록한다 | 이 문서 6절 |
| FE-TBD-013 | 시안 토큰 블록의 표식과 대조 | 시안, 토큰 대조 | 사용자 결정 2026-10-22(Q32): 시작·끝 표식으로 감싸고 `--check`가 생성 결과와 같은지 본다 | 이 문서 2절, [DESIGN.md 검증 절](../design/DESIGN.md#검증) |
| FE-TBD-014 | 실기기 모바일 확인의 시점 | 릴리스 검증 | 사용자 결정 2026-10-22(Q36): 변경마다가 아니라 릴리스 전에 한 번 확인하고 기록한다. 작업 4.12 완료 조건에서는 뺐다(사용자 결정 2026-11-07(Q136)) | 이 문서 4·6절 |
| FE-TBD-015 | 검증 how-to의 위치 | `docs/how-to/` | 사용자 결정 2026-10-22(Q38): `docs/how-to/verify-dashboard-frontend.md`를 새로 둔다 | [검증 how-to](../how-to/verify-dashboard-frontend.md) |
| FE-TBD-016 | 대시보드 성능 지표와 예산 | 성능 검증 | 사용자 결정 2026-11-07(Q159): 지표와 예산을 두지 않는다. 이전에는 사용자 결정 2026-10-22(Q39)로 화면 구현 뒤까지 보류했다 | 4절 성능 행 |

### 8.1 결정 내용과 채택하지 않은 선택지

각 항목은 채택한 방법을 먼저 쓰고, 채택하지 않은 선택지와 이유를 뒤에 쓴다.

- **FE-TBD-001 도구 형태** (사용자 결정 2026-10-22(Q32)). 채택: (가) `cargo xtask tokens`와 `--check` 하위 명령. 추가 언어나 의존성이 없고 xtask 설계의 Rust 전용 선택과 맞다. 채택하지 않음: (나) Node나 Python 스크립트. 두 OS의 실행 환경이 갈린다. (다) 수동 복사. 대조가 없어 DESIGN.md의 생성 원칙을 지키지 못한다.
- **FE-TBD-002 커밋 방식** (사용자 결정 2026-10-22(Q32)). 채택: (가) 생성 파일을 커밋하고 `--check`로 대조한다. 빌드 없이 검토할 수 있다. 채택하지 않음: (나) 빌드 때 `build.rs`가 생성한다. 빌드 입력이 늘고 생성 결과를 검토하기 어렵다.
- **FE-TBD-003 내장 방법** (사용자 결정 2026-10-22(Q33)). 채택: (가) 한 Rust 파일의 `include_bytes!` 표와 대조 테스트. 새 crate가 없다. 채택하지 않음: (나) `build.rs`가 표를 생성한다. 빌드 스크립트 로직이 늘어난다. (다) 새 crate(예: `include_dir`, `rust-embed`). 새 crate는 사용자 승인이 필요하다.
- **FE-TBD-004 미리보기** (사용자 결정 2026-10-22(Q34)). 채택: (가) `web/dashboard/preview/`와 `cargo xtask dashboard-serve`(로컬 전용). 제공 표에 없어 실행 파일에 들어가지 않는다. 채택하지 않음: (나) `examples/` 바이너리. 실행 파일과 같은 빌드를 타서 경계가 흐리다. 브라우저는 모듈을 `file://`로 열 수 없으므로 어느 쪽이든 로컬 서버가 필요하다.
- **FE-TBD-005 브라우저 테스트** (사용자 결정 2026-10-22(Q35)). 채택: (가) Rust 통합 시험이 Chromium을 `--headless --dump-dom`으로 실행하고, 테스트 페이지가 결과를 `data-result`에 쓴다. `/// spec:` 표식은 Rust 시험에 둔다. 새 crate와 스캐너 변경이 없다. 실제 키 입력은 이 방식으로 보낼 수 없으므로 수동 체크리스트(FE-TBD-007)로 확인한다. 채택하지 않음: (나) xtask 스캐너가 JavaScript 파일의 표식도 읽는다. 테스트 컨벤션 7절과 xtask 설계를 함께 고쳐야 한다. (다) DevTools 프로토콜 클라이언트로 실제 키 입력을 보낸다. 새 의존성 승인이 필요하다.
- **FE-TBD-006 Chromium** (사용자 결정 2026-10-22(Q35)). 채택: (가) Chrome 또는 Edge를 Windows와 Ubuntu 모두에 필수 전제로 둔다. 브라우저를 찾지 못하면 시험을 실패시킨다. 채택하지 않음: (나) `#[ignore]`로 표시한다. 기본 실행에서 빠지고 보고서는 미실행이 된다. `--require-all`에서 항상 실패한다.
- **FE-TBD-007 키보드·스크린 리더** (사용자 결정 2026-10-22(Q36)). 채택: (가) 1차는 수동 체크리스트를 검증 기록에 남긴다. 해당 시나리오는 부분으로 본다. 채택하지 않음: (나) FE-TBD-005의 (다)로 자동화한다. 승인이 필요하다. Q137 변경: 스크린 리더 확인을 작업 4.12 완료 조건에서 뺐다. 확인하지 않은 동안 검증 기록에는 미실행으로 남긴다. 접근성 트리 자동 확인으로 대신하는 안과 사용자가 직접 듣는 안은 채택하지 않았다.
- **FE-TBD-008 시각 비교** (사용자 결정 2026-10-22(Q36)). 채택: (가) 스크린샷을 `target/`에 두고 사람이 시안과 비교한다. 채택하지 않음: (나) 픽셀 비교 도구를 쓴다. 도구 승인과 설치 절차가 필요하다. Q138 적용: 작업 4.12에서는 저장소 밖 도구가 시안 프레임과 같은 상태의 구현 화면을 같은 폭으로 찍어 비교 페이지를 만든다. 사용자가 그 페이지를 검토하고 차이를 알려 준다. 에이전트의 비교로 사람 비교를 대신하는 안은 채택하지 않았다.
- **FE-TBD-009 브라우저 범위** (사용자 결정 2026-10-22(Q36), 2026-11-07(Q135)). 채택: (가) 자동은 Chromium 한 곳(Chrome·Edge 엔진)만 쓴다. Firefox는 수동으로 확인하고 버전을 기록한다. 채택하지 않음: (나) 모든 브라우저를 수동으로 확인한다. 기록 부담이 크다. Q135 변경: macOS가 없어 Safari를 확인할 수 없으므로, Safari 확인을 Playwright WebKit 자동 확인으로 대신한다. Playwright는 저장소 밖 검증 도구이며 프로젝트 의존성에 넣지 않는다. 이 엔진은 실제 Safari와 같은 빌드가 아니다. 그래서 결과는 "WebKit 대체 확인"으로 적고 Safari 통과로 적지 않는다. Q135는 Q36의 스크린 리더·실기기·사람 시안 비교를 바꾸지 않았다. 그 뒤의 Q136~Q138은 FE-TBD-007·008·014 항목에 적었다.
- **FE-TBD-010 CSP** (사용자 결정 2026-10-22(Q37)). 채택: (가) 정적 응답에 `default-src 'self'; frame-ancestors 'none'`을 필수로 한다. 인라인 `<style>`과 `style` 속성을 금지한다. 계약 1.11절의 "권장"을 "필수"로 고친다. 채택하지 않음: (나) CSP를 권장으로 두고 인라인 스타일을 허용하려고 `style-src 'unsafe-inline'`을 추가한다. 보호가 약해진다.
- **FE-TBD-011 `components` 블록** (사용자 결정 2026-10-22(Q32)). 채택: (가) CSS로 내보내지 않는다. 원본 참조용이다. 채택하지 않음: (나) 컴포넌트마다 CSS 변수를 만든다. DESIGN.md의 이름 규칙을 바꿔야 한다.
- **FE-TBD-012 DESIGN lint** (사용자 결정 2026-10-22(Q38)). 채택: (가) 선택 항목으로 두고 `@google/design.md`의 버전을 고정한다. 결과를 기록한다. 채택하지 않음: (나) 검증 절차에서 뺀다.
- **FE-TBD-013 시안 토큰 블록** (사용자 결정 2026-10-22(Q32)). 채택: (가) 시안의 토큰 블록을 시작·끝 표식으로 감싸고, `--check`가 생성 결과와 같은지 본다. 채택하지 않음: (나) 일회성 확인을 유지한다. 원본과 시안이 어긋날 수 있다.
- **FE-TBD-014 실기기** (사용자 결정 2026-10-22(Q36)). 채택: (가) 변경마다 필수는 아니다. 릴리스 전에 한 번 확인하고 기록한다. 채택하지 않음: (나) 변경마다 확인한다. 비용이 크다. Q136 변경: 실기기 확인을 작업 4.12 완료 조건에서 뺐다. 에뮬레이션으로 대신하는 안과 사용자가 실기기로 확인하는 안은 채택하지 않았다. 릴리스 전 확인 규칙은 바꾸지 않았다. 이 문서의 판단, 사용자 결정 없음: Q136은 작업 4.12의 범위만 물었다.
- **FE-TBD-015 검증 how-to 위치** (사용자 결정 2026-10-22(Q38)). 채택: (가) 새 파일 `docs/how-to/verify-dashboard-frontend.md`. 채택하지 않음: (나) 개발 환경 how-to에 절을 추가한다. 다른 작업과 충돌할 수 있다.
- **FE-TBD-016 성능 지표** (사용자 결정 2026-10-22(Q39) 보류, 사용자 결정 2026-11-07(Q159) 종결). 채택: 지표 없이 닫는다. 확인한 PRD에 대시보드 성능 요구가 없고, 대시보드는 루프백·소수 관리자용이다. 채택하지 않음: (나) 정적 자원 총량 예산과 초과 시 실패하는 시험. (다) 브라우저 첫 표시 시간 예산. 환경 편차로 시험이 불안정할 수 있다.

### 8.2 같은 변경에서 고친 문서

다음 문서를 이 결정과 같은 변경에서 고쳤다.

- [xtask 모듈 설계](../architectures/xtask.md) 1·7·8절: 목표 명령 `tokens [--check]`·`dashboard-serve`(미구현 표시), 시험 수집 방법(Q35), 미결정 행 해소.
- [관리 API 계약 1.11절](../contracts/management-api-v1.md): CSP 필수(FE-TBD-010). 계약 버전 1.1.0.
- [테스트 컨벤션 2·7절](testing.md#7-테스트와-스펙-시나리오-연결): 브라우저 시험은 `#[ignore]` 예외이고, 표식은 Rust 시험에 둔다(FE-TBD-005·006).
- [개발 환경·검증 how-to 2절](../how-to/set-up-and-verify.md): Chromium 전제(FE-TBD-006).
- [DESIGN.md 검증 절](../design/DESIGN.md#검증): 검증 how-to 링크와 생성 절차 연결. DESIGN-TBD-005 해소.
- [코드 지도](../architectures/code-map.md) `web/dashboard/` 행: 파일 구조와 내장 위치 연결.
