# VNC Relay 운영 대시보드 프런트엔드 컨벤션

채택된 디자인 기준을 `web/dashboard/`의 HTML·CSS·JavaScript에 일관되게 적용하는 규칙이다. 제품 동작은 [대시보드 화면 스펙](../specs/dashboard.md)에서, 코드의 책임과 내부 구조는 아키텍처 문서에서 정한다. 이 문서는 Draft다. [8절](#8-미결정-사항)의 결정이 채택되기 전까지 DESIGN-TBD-005는 열려 있다.

| 항목 | 값 |
| --- | --- |
| 상태 | Draft (2026-10-22 초안. 사용자 채택 전) |
| 적용 대상 | `web/dashboard/` 아래 HTML·CSS·JavaScript, 토큰 생성·대조 코드(`xtask/`), 테스트 페이지(`web/dashboard/tests/`), 미리보기(`web/dashboard/preview/`), 시안의 토큰 블록(`docs/design/mockups/dashboard.html`) |
| 기준 버전 | Rust 1.99.1(`rust-toolchain.toml`). 대시보드 런타임 의존성 없음. 브라우저 검증에 쓴 Chromium 버전은 검증 기록에 적는다 |
| 관련 디자인 기준 | [운영 대시보드 디자인 시스템](../design/DESIGN.md) (Accepted, 2026-10-20) |
| 관련 아키텍처 문서 | [코드 지도](../architectures/code-map.md), [xtask 모듈 설계](../architectures/xtask.md) |
| 관련 계약 | [관리 API 계약 1.11절](../contracts/management-api-v1.md) (정적 경로·비인증 범위·응답 헤더) |
| 관련 ADR | 없음 |
| 적용 시작일 | 채택 후 기록 |
| 규칙 준수 확인 방법 | `cargo xtask tokens --check`, 제공 표 대조 테스트, 브라우저 테스트, [검증 how-to](../how-to/verify-dashboard-frontend.md)의 수동 검토 |

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
| 미리보기 | 실제 컴포넌트를 불러 쓰는 정적 페이지(`web/dashboard/preview/`) | [3절 컴포넌트 미리보기](#컴포넌트-미리보기) |

선택이 비어 있으면 특정 프레임워크나 라이브러리를 기본값으로 설치하지 않는다. 기존 설정과 사용자 요청을 먼저 확인한다.

### 1.1 정적 파일 구조

대시보드 파일은 `web/dashboard/` 아래에 둔다. 제공 경로는 [관리 API 계약 1.11절](../contracts/management-api-v1.md)의 경로 표와 같아야 한다.

```text
web/dashboard/
├── login/            인증 없이 제공한다 (SCR-003, 로그인 화면)
│   ├── index.html    제공 경로 /login/
│   ├── login.js
│   ├── login.css
│   └── tokens.css    생성 산출물. 직접 고치지 않는다
├── app/              관리 세션이 있어야 제공한다 (SCR-001·SCR-002)
│   ├── index.html    제공 경로 /  (이 문서의 제안: /app/index.html은 제공하지 않는다)
│   ├── main.js
│   ├── components/   공통 컴포넌트 ES 모듈
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
- `login/`과 `app/`은 파일을 공유하지 않는다. 로그인 화면은 세션이 없으므로 `/app/` 파일을 받을 수 없다. 같은 내용이 두 곳에 필요하면 생성 도구가 두 곳에 같은 내용으로 쓰고, 대조 명령이 두 파일의 같음을 확인한다.
- 파일 이름은 소문자와 하이픈(kebab-case)으로 쓴다. 허용 확장자는 `.html`, `.css`, `.js`다. 다른 형식이 필요하면 이 절을 먼저 고친다.

### 1.2 실행 파일 내장

제공 파일은 실행 파일에 들어 있어야 한다([PRD](../requirements/PRD.md), [DESIGN.md 정적 파일 원칙](../design/DESIGN.md#정적-파일-원칙)).

- 제공 파일은 `include_bytes!`로 실행 파일에 넣는다. 파일이 없으면 빌드가 실패한다.
- 제공 경로, 원본 파일, `Content-Type`을 한 Rust 파일의 표에 적는다. 표에 없는 파일은 넣지 않는다.
- `Content-Type`은 확장자로 정한다: `text/html; charset=utf-8`, `text/css; charset=utf-8`, `text/javascript; charset=utf-8`.
- 표 대조 테스트는 `login/`과 `app/`을 걸어 실제 파일 목록과 표를 비교한다. 표에 없는 파일이나 표에만 있는 항목이 있으면 실패한다. 제공 표에 `preview/`나 `tests/` 경로가 있어도 실패한다.
- 정적 응답에는 계약 1.11절의 헤더를 붙인다. `Cache-Control: no-store`는 계약이 보호된 정적 응답에 붙인다고 정했다. `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `Content-Security-Policy`는 계약에서 권장이다. 이 문서는 로그인 화면 응답에도 `Cache-Control: no-store`를 붙이기를 제안한다. CSP를 필수로 할지는 [FE-TBD-010](#8-미결정-사항)에서 정한다.
- 내장 방법(표 한 곳, build.rs, 외부 crate)은 [FE-TBD-003](#8-미결정-사항)에서 정한다.

### 1.3 HTML·CSS·JavaScript 작성 규칙

다음 규칙은 CSP `default-src 'self'`를 전제로 한다. [계약 1.11절](../contracts/management-api-v1.md)은 대시보드가 인라인 스크립트를 쓰지 않는다고 정하고, CSP 값을 권장으로 적는다. CSP를 필수로 할지는 [FE-TBD-010](#8-미결정-사항)에서 확정한다.

- HTML 루트에 `lang`을 둔다. 언어 선택에 따라 값을 바꾼다([화면 언어](../specs/dashboard.md#화면-언어)).
- 스크립트는 `<script type="module" src="…">`로만 불러온다. 인라인 스크립트, `on…` 이벤트 속성, `javascript:` 주소를 쓰지 않는다.
- 스타일은 `<link rel="stylesheet">`로 같은 출처의 파일만 불러온다. `<style>` 요소와 `style` 속성을 쓰지 않는다. 시안(`docs/design/mockups/dashboard.html`)의 `<style>`은 시안 파일 안에서만 쓴다.
- 스크립트로 값을 넣을 때는 `element.style.setProperty()`를 쓴다. 이 방식이 CSP에서 막히는지는 브라우저 테스트로 확인한다.
- 외부 자원을 쓰지 않는다. CDN, 외부 글꼴, 외부 이미지, 외부 스크립트, 외부 `@import`를 금지한다. 요청은 같은 출처의 제공 경로와 `/api/v1/`만 보낸다.
- API에서 받은 값은 `textContent`나 속성으로 넣는다. `innerHTML`에 API 값을 넣지 않는다.
- 폴링 주기와 조회 제한 시간은 코드에 상수로 쓰지 않는다. [관리 API 계약 1.12절](../contracts/management-api-v1.md#112-폴링-주기와-조회-제한-시간의-전달)의 전달 방법을 따른다. 기본값과 범위는 [설정 계약 1.2절](../contracts/config-v1.md#12-키-표)이 정한다.
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
| 참조·생성 관계 | DESIGN.md YAML → `web/dashboard/login/tokens.css`, `web/dashboard/app/tokens.css`(같은 내용) → 화면 CSS가 `var(--…)`로 참조 → 시안의 토큰 블록(표식 사이, [FE-TBD-013](#8-미결정-사항)) |
| 생성 방법 | `cargo xtask tokens`가 원본을 읽어 `:root` 블록을 쓴다. 도구 형태는 [FE-TBD-001](#8-미결정-사항)에서 정한다 |
| 대조 방법 | `cargo xtask tokens --check`가 생성 결과와 두 CSS 파일, 시안 표식 사이 줄을 비교한다. 차이가 있으면 파일과 줄 번호를 출력하고 종료 코드 1로 끝난다 |
| 생성 대상 밖의 규칙 | 중단점과 미디어 쿼리는 [DESIGN.md 중단점 절](../design/DESIGN.md#중단점과-모바일-배치)을 따른다. 모션은 없다([DESIGN.md 모션 절](../design/DESIGN.md#모션)). 대비 기준은 [DESIGN.md 대비 기준](../design/DESIGN.md#대비-기준)을 따른다. 한글 글꼴 굵기는 400·700만 쓴다 |

### 2.1 원본 읽기와 이름 규칙

- 허용 키는 `colors`, `typography`, `spacing`, `rounded`다. `version`, `name`, `description`과 `components`는 CSS로 내보내지 않는다. `components`는 원본 참조용이다([FE-TBD-011](#8-미결정-사항)).
- `typography`의 단계 아래에는 `fontFamily`, `fontSize`, `fontWeight`, `lineHeight`, `fontFeature`만 쓴다. 현재 원본에는 `letterSpacing`이 없다.
- 허용하지 않는 키, 중복된 키, 해석할 수 없는 줄이 있으면 생성과 대조를 멈추고 종료 코드 1로 끝난다.
- 값은 YAML 값을 그대로 쓰고, 바깥 따옴표만 벗긴다. 값 안의 따옴표(`'tnum'`, 글꼴 이름)는 그대로 둔다. `#`으로 시작하는 색 값은 따옴표로 감싸야 한다. YAML에서 `#`은 주석 시작이다. 현재 원본은 이 규칙을 따른다(예: `overlay: "#0d1a1580"`).
- 이름은 DESIGN.md의 [CSS 사용자 정의 속성 이름 규칙](../design/DESIGN.md#css-사용자-정의-속성-이름-규칙)을 따른다.

| 원본 | CSS 이름 |
| --- | --- |
| `colors.<이름>` | `--color-<이름>` |
| `spacing.<이름>` | `--space-<이름>` |
| `rounded.<이름>` | `--radius-<이름>` |
| `typography.<단계>.fontFamily·fontSize·fontWeight·lineHeight·fontFeature` | `--type-<단계>-family·size·weight·line·feature` |

- 출력은 `:root { … }` 블록 하나다. 한 줄에 한 속성을 쓴다. 순서는 원본 순서를 따른다. 첫 줄에 "생성 파일이다. 직접 고치지 않는다"는 주석과 원본 경로를 쓴다.

### 2.2 화면 CSS 검사

`cargo xtask tokens --check`는 토큰 파일을 뺀 화면 CSS도 검사한다. 다음 항목이 있으면 실패한다.

- 16진수 색(`#`과 3·4·6·8자리 수), `rgb(`, `hsl(`
- `url(`, `@import`
- 길이 리터럴(`px`, `em`, `rem`). 미디어 쿼리 조건에서만 쓴다. 조건의 값은 DESIGN.md 중단점 절을 따른다.
- `font-weight`에 토큰 변수나 `inherit`이 아닌 값
- 정의되지 않은 `var(--…)`

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
| 구현과 실행 위치 | `web/dashboard/preview/` 아래 정적 HTML 페이지. 실행은 로컬 전용 서버로 한다([FE-TBD-004](#8-미결정-사항)). 브라우저는 모듈 스크립트를 `file://`로 읽지 않으므로 서버가 필요하다 |
| 재사용 대상 | `web/dashboard/app/components/`, `web/dashboard/app/tokens.css`, `web/dashboard/app/styles/`. 원본은 한 곳이다 |
| 확인할 상태 | DESIGN.md 컴포넌트 표의 항목별 기본·호버·비활성·포커스 상태. 오류·빈 값·긴 한글·긴 영문·긴 숫자(`data` 단계) 상태 |
| 확인 환경 | 폭 1280·1024·1023·768·390·320px, 한국어·영어 문구, 라이트 테마 |
| 배포·공유 범위 | 실행 파일에 넣지 않는다. 제공 표에 없다(1.2절 대조 테스트가 확인한다) |
| 검증과 증거 | 테스트 페이지의 결과(`data-result`), 폭별 스크린샷(저장소에 넣지 않고 `target/`에 둔다), 검증 기록 |

기본 컴포넌트, 컴포넌트 조합, 화면 순서로 확인한다. 예를 들어 `button`과 `input`을 확인한 뒤 필드 묶음과 로그인 폼을 확인한다. 검증을 위해 제품에 없는 상태나 기능을 추가하지 않는다.

### 화면 시안과 대조

- HTML 시안은 디자인 기준과 화면 구성을 보여 주는 산출물이다([디자인 문서 작성 규칙](../design/AGENTS.md#html-시안)). 시안은 동작·상태·검증의 기준이 아니다([화면 스펙 서문](../specs/dashboard.md)).
- 시안과 스펙 문장이 다르면 스펙 문장을 따른다.
- 대조는 같은 화면 상태와 같은 폭에서 구현 스크린샷과 시안 스크린샷을 나란히 두고 한다. 차이를 검증 기록에 적는다. 자동 픽셀 비교는 [FE-TBD-008](#8-미결정-사항)에서 정한다.
- 시안의 토큰 블록은 생성 결과와 같아야 한다([FE-TBD-013](#8-미결정-사항)).

## 4. 접근성·반응형·성능 확인

접근성은 다양한 입력 방식과 보조 기술로 UI를 쓸 수 있는지를 뜻한다. 자동 검사 결과만으로 전체 기준 준수를 선언하지 않는다.

| 대상 | 적용 기준 | 확인 방법 |
| --- | --- | --- |
| 접근성 수준 | WCAG 2.2 AA([DESIGN.md 접근성 기준](../design/DESIGN.md#접근성-기준)). 동작 요구 범위는 [화면 스펙 6절](../specs/dashboard.md#6-화면-크기와-접근성) | 자동: 테스트 페이지가 역할·이름·속성을 확인한다. 수동: 키보드 전체 순회와 스크린 리더 확인([FE-TBD-007](#8-미결정-사항)) |
| 키보드와 포커스 | 포커스 표시는 토큰 `spacing.focus-ring-width`와 `spacing.focus-ring-offset`, 링 색 `colors.ring`(상단 막대 안은 `colors.navbar-ring`). 대화상자 포커스 규칙은 [화면 스펙 6절](../specs/dashboard.md#6-화면-크기와-접근성) | 자동: 포커스 순서, 대화상자 열기·닫기 뒤 포커스 위치. 수동: Tab 순회, Esc로 닫기, 기록 |
| 이름과 의미 | 시맨틱 HTML. 상단 막대 메뉴는 `nav`(이름 "주 메뉴"), 본문은 `main`. 카드 배치에서도 `table`·`row`·`cell`·`columnheader` 역할을 명시한다([DESIGN.md 중단점 절](../design/DESIGN.md#중단점과-모바일-배치)) | 자동: 역할과 이름. 수동: 접근성 트리 확인 |
| 대비와 의미 전달 | DESIGN.md [대비 기준](../design/DESIGN.md#대비-기준) 표. 상태는 글씨 레이블과 도형으로 구분한다 | 토큰 쌍은 WCAG 상대 휘도 공식으로 다시 계산하고 값을 검증 기록에 적는다. 색 외 표시는 수동 확인 |
| 반응형과 확대 | 폭 1280·1024·1023·768·390·320px. 200% 확대는 640px과 같다([DESIGN.md 중단점 절](../design/DESIGN.md#중단점과-모바일-배치)) | 자동: 폭별로 Chromium을 열어 `scrollWidth`가 `clientWidth`를 넘지 않는지 확인. 대상 크기는 수동 측정 |
| 모션 | 없다([DESIGN.md 모션 절](../design/DESIGN.md#모션)). 모션을 추가하기 전에 DESIGN.md를 먼저 고친다 | 해당 없음. 추가하면 움직임 감소 설정 처리를 이 표에 추가한다 |
| 성능 | 확인한 PRD에 대시보드 성능 지표가 없다. 지표가 정해질 때까지 성능 통과 조건을 만들지 않는다([FE-TBD-016](#8-미결정-사항)) | 측정 도구와 환경은 지표가 정해진 뒤 적는다 |

## 5. 예외와 외부 자료

- 시각 예외는 [DESIGN.md 예외 절](../design/DESIGN.md#예외)을 따른다. 현재 예외가 없다.
- 코드 예외가 생기면 `FE-EX-001`부터 번호를 매기고, 범위·근거·재검토 조건을 이 절에 적는다. 현재 예외가 없다.
- DESIGN.md는 shadcn/ui의 시각 관례만 참고했다. shadcn/ui와 Tailwind CSS 패키지는 쓰지 않는다([참고 자료](../design/DESIGN.md#참고-자료), [정적 파일 원칙](../design/DESIGN.md#정적-파일-원칙)).
- 참고 이미지는 시각 방향만 참고한다. 이미지 자산과 문구는 복제하지 않는다([DESIGN.md 참고 자료](../design/DESIGN.md#참고-자료)).

## 6. 검증 명령과 통과 기준

절차는 [대시보드 프런트엔드를 검증하는 방법](../how-to/verify-dashboard-frontend.md)(Draft)이 정한다. 결과와 증거는 [검증 기록](../AGENTS.md#검증-기록)을 따른다.

- 실행 환경: [개발 환경·검증 how-to](../how-to/set-up-and-verify.md)의 [전제 조건](../how-to/set-up-and-verify.md#2-전제-조건)이 정한 지원 환경(Windows 11 x64·PowerShell, Ubuntu 24.04 LTS x64·bash)과 고정 Rust 툴체인(1.99.1). Chromium 경로는 환경 변수 `DASHBOARD_BROWSER`로 준다(제안, [FE-TBD-006](#8-미결정-사항)). Node.js는 선택 항목인 DESIGN lint에만 필요하다.
- 실행 위치: 저장소 루트.
- 명령 확인 근거: `xtask/src/main.rs`의 현재 명령은 `scenarios`와 `layers`다. 아래의 `tokens`, `dashboard-serve`와 테스트 파일 이름은 채택 뒤 추가할 대상이다.

| 변경 범위 | 확인 명령·방법 | 통과 조건 | 절차와 증거 위치 |
| --- | --- | --- | --- |
| 디자인 토큰·문서 | `cargo xtask tokens --check`. 선택: DESIGN.md lint(`@google/design.md`, 고정 버전) | 종료 코드 0. 차이 0건, 금지 항목 0건. lint 오류 0건. 경고 수는 기록한다 | [검증 how-to 3.1절](../how-to/verify-dashboard-frontend.md) |
| 정적 파일과 내장 | 제공 표 대조 테스트(`tests/dashboard_assets.rs`) | 실제 파일과 표가 일치한다. 금지 경로가 없다 | [검증 how-to 3.2절](../how-to/verify-dashboard-frontend.md) |
| 컴포넌트·스타일 | 브라우저 테스트(`tests/dashboard_browser.rs`)와 화면 CSS 검사(2.2절) | 테스트 페이지의 `data-result`가 `pass`다 | [검증 how-to 3.3절](../how-to/verify-dashboard-frontend.md) |
| 사용자 동작 | 브라우저 테스트와 `cargo xtask scenarios` | DASH 시나리오가 통과 또는 부분으로 보고된다. 부분은 Chromium 한 곳만 검증한 경우다 | [테스트 컨벤션 7절](testing.md#7-테스트와-스펙-시나리오-연결), `target/scenario-report.md` |
| 시각 결과·접근성 | 폭별 스크린샷, 시안과 나란히 대조, 키보드·스크린 리더 수동 확인 | 스펙과 DESIGN.md 기준을 만족한다. 검토자가 기록한다 | [검증 how-to 3.4~3.7절](../how-to/verify-dashboard-frontend.md) |
| 전체 완료 | [개발 환경·검증 how-to 3.4절](../how-to/set-up-and-verify.md)의 명령과 위 명령 | 모든 필수 검사가 종료 코드 0이다 | [개발 환경·검증 how-to](../how-to/set-up-and-verify.md) |

시행하지 않은 검사는 성공으로 표시하지 않는다. 미실행 사유와 영향을 검증 기록에 적는다.

## 7. 기존 코드 적용

`web/dashboard/`는 아직 없다(2026-10-22 확인). 기존 제품 코드와 충돌하는 부분이 없다. 목업의 `<style>`은 시안 파일이며 제품 파일이 아니므로 CSP 적용 대상이 아니다. 채택 뒤 되돌릴 때는 생성 산출물을 `cargo xtask tokens`로 다시 쓰거나, 변경한 파일을 이전 커밋 상태로 되돌린다.

## 8. 미결정 사항

사용자 결정 2026-10-22(Q09): 작업 0.9를 지금 시작한다. 이 문서는 그 초안이다. DESIGN-TBD-005는 아래 결정이 채택될 때까지 열려 있다.

아래 권장안은 초안 작성자의 추천이다. 사용자 결정이 아니다. 채택 전까지 이 문서는 Draft다. 책임자는 [DESIGN.md 미결정 사항](../design/DESIGN.md#미결정-사항)의 DESIGN-TBD-005 책임자와 같다.

| ID | 항목 | 영향 범위 | 책임자 | 확정 위치 |
| --- | --- | --- | --- | --- |
| FE-TBD-001 | 토큰 생성·대조 도구 형태 | 토큰 파일, `xtask/` | 프로젝트 요구사항·디자인 결정 담당자 | 이 문서 2절, [xtask 모듈 설계](../architectures/xtask.md) |
| FE-TBD-002 | 생성 산출물을 저장소에 커밋할지 | 빌드, 토큰 파일 | 같은 담당자 | 이 문서 2절 |
| FE-TBD-003 | 실행 파일 내장 방법 | 관리 리스너, `Cargo.toml` | 같은 담당자 | 이 문서 1.2절 |
| FE-TBD-004 | 미리보기 위치와 실행 방법 | `preview/`, `xtask/` | 같은 담당자 | 이 문서 3절 |
| FE-TBD-005 | 브라우저 테스트 방식과 보고서 연결 | `tests/`, 테스트 컨벤션, `xtask/` | 같은 담당자 | 이 문서 6절, [테스트 컨벤션 7절](testing.md#7-테스트와-스펙-시나리오-연결) |
| FE-TBD-006 | Chromium 필요 여부와 없을 때 처리 | 검증 환경, 검증 how-to | 같은 담당자 | 이 문서 6절, [개발 환경 how-to 2절](../how-to/set-up-and-verify.md) |
| FE-TBD-007 | 키보드·포커스·스크린 리더 확인의 자동화 범위 | 접근성 검증 | 같은 담당자 | 이 문서 4절 |
| FE-TBD-008 | 시각 비교 방법(수동 또는 자동 픽셀 비교) | 스크린샷 검증 | 같은 담당자 | 이 문서 3절 |
| FE-TBD-009 | 자동 검증 브라우저 범위와 수동 범위 | 검증 기록 | 같은 담당자 | 이 문서 4·6절 |
| FE-TBD-010 | CSP 필수 여부와 인라인 스타일 금지 | 응답 헤더, 계약 | 같은 담당자 | 이 문서 1.2·1.3절, [관리 API 계약 1.11절](../contracts/management-api-v1.md) |
| FE-TBD-011 | DESIGN.md `components` 블록의 내보내기 | 토큰 파일 | 같은 담당자 | 이 문서 2.1절 |
| FE-TBD-012 | DESIGN.md lint 도구의 검증 절차 내 위치 | 문서 검증 | 같은 담당자 | 이 문서 6절 |
| FE-TBD-013 | 시안 토큰 블록의 표식과 대조 | `docs/design/mockups/dashboard.html`, 토큰 대조 | 같은 담당자 | 이 문서 2절, [DESIGN.md 검증 절](../design/DESIGN.md#검증) |
| FE-TBD-014 | 실기기 모바일 확인의 시점 | 릴리스 검증 | 같은 담당자 | 이 문서 4·6절 |
| FE-TBD-015 | 검증 how-to의 위치 | `docs/how-to/` | 같은 담당자 | [검증 how-to](../how-to/verify-dashboard-frontend.md) |
| FE-TBD-016 | 대시보드 성능 지표와 예산 | 성능 검증 | 같은 담당자 | [PRD](../requirements/PRD.md) |

### 8.1 선택지와 권장안

- **FE-TBD-001 도구 형태.** (가) `cargo xtask tokens`와 `--check` 하위 명령. 권장. 추가 언어나 의존성이 없고 xtask 설계의 Rust 전용 선택과 맞다. (나) Node나 Python 스크립트. 두 OS의 실행 환경이 갈린다. (다) 수동 복사. 대조가 없어 DESIGN.md의 생성 원칙을 지키지 못한다.
- **FE-TBD-002 커밋 방식.** (가) 생성 파일을 커밋하고 `--check`로 대조. 권장. 빌드 없이 검토할 수 있다. (나) 빌드 때 `build.rs`가 생성. 빌드 입력이 늘고 생성 결과를 검토하기 어렵다.
- **FE-TBD-003 내장 방법.** (가) 한 Rust 파일의 `include_bytes!` 표와 대조 테스트. 권장. 새 crate가 없다. (나) `build.rs`가 표를 생성. 빌드 스크립트 로직이 늘어난다. (다) 새 crate(예: `include_dir`, `rust-embed`). 새 crate이므로 사용자 승인이 필요하다.
- **FE-TBD-004 미리보기.** (가) `web/dashboard/preview/`에 두고 `cargo xtask dashboard-serve`로 로컬 서버를 연다. 권장. 제공 표에 없어 실행 파일에 들어가지 않는다. 새 xtask 명령이므로 xtask 설계 문서를 고친다. (나) `examples/` 바이너리. 실행 파일과 같은 빌드를 타서 경계가 흐리다. 브라우저는 모듈을 `file://`로 열 수 없으므로 어느 쪽이든 로컬 서버가 필요하다.
- **FE-TBD-005 브라우저 테스트.** (가) Rust 통합 테스트가 Chromium을 `--headless --dump-dom`으로 실행한다. 테스트 페이지가 결과를 `data-result`에 쓰고, `/// spec:` 표식은 Rust 테스트에 둔다. 권장. 새 crate와 스캐너 변경이 없다. 실제 키 입력은 이 방식으로 보낼 수 없어 제외된다. (나) xtask 스캐너가 JavaScript 파일의 표식도 읽는다. 테스트 컨벤션 7절과 xtask 설계를 함께 고쳐야 한다. (다) DevTools 프로토콜 클라이언트로 실제 키 입력을 보낸다. 새 의존성 승인이 필요하다.
- **FE-TBD-006 Chromium.** (가) 필수 전제로 두고 `DASHBOARD_BROWSER`가 없으면 테스트를 실패시킨다. 권장. 실행하지 않은 검사가 통과로 보이지 않는다. 개발 환경 전제 목록에 추가해야 한다. (나) `#[ignore]`로 표시한다. 기본 실행에서 빠지고 보고서는 미실행이 된다. `--require-all`에서 항상 실패한다.
- **FE-TBD-007 키보드·스크린 리더.** (가) 1차는 수동 체크리스트를 검증 기록에 남긴다. 권장. 해당 시나리오는 부분으로 본다. (나) FE-TBD-005의 (다)로 자동화한다. 승인이 필요하다.
- **FE-TBD-008 시각 비교.** (가) 스크린샷을 `target/`에 두고 사람이 시안과 비교한다. 권장. (나) 픽셀 비교 도구를 쓴다. 도구 승인과 설치 절차가 필요하다.
- **FE-TBD-009 브라우저 범위.** (가) 자동은 Chromium 한 곳(Chrome·Edge 엔진). Firefox와 Safari는 수동으로 확인하고 버전을 기록한다. 권장. (나) 모든 브라우저를 수동으로 확인한다. 기록 부담이 크다.
- **FE-TBD-010 CSP.** (가) 정적 응답에 `default-src 'self'; frame-ancestors 'none'`을 필수로 한다. 인라인 `<style>`과 `style` 속성을 금지한다. 권장. 계약 1.11절의 "권장"을 "필수"로 바꾸는 변경이 따로 필요하다. (나) CSP는 권장으로 두고, 인라인 스타일을 허용하려면 `style-src 'unsafe-inline'`을 추가한다. 보호가 약해진다.
- **FE-TBD-011 `components` 블록.** (가) CSS로 내보내지 않는다. 원본 참조용이다. 권장. (나) 컴포넌트마다 CSS 변수를 만든다. DESIGN.md의 이름 규칙을 바꿔야 한다.
- **FE-TBD-012 DESIGN lint.** (가) 선택 항목으로 두고 `@google/design.md`의 버전을 고정한다. 결과를 기록한다. 권장. (나) 검증 절차에서 뺀다.
- **FE-TBD-013 시안 토큰 블록.** (가) 시안의 토큰 블록을 시작·끝 표식으로 감싸고, `--check`가 생성 결과와 같은지 본다. 권장. (나) 일회성 확인을 유지한다. 원본과 시안이 어긋날 수 있다.
- **FE-TBD-014 실기기.** (가) 변경마다 필수는 아니다. 릴리스 전에 한 번 확인하고 기록한다. 권장. (나) 변경마다 확인한다. 비용이 크다.
- **FE-TBD-015 검증 how-to 위치.** (가) 새 파일 `docs/how-to/verify-dashboard-frontend.md`. 권장. 개발 환경 how-to와 주제가 다르다. (나) 개발 환경 how-to에 절을 추가한다. 다른 작업과 충돌할 수 있다.
- **FE-TBD-016 성능 지표.** 확인한 PRD에 대시보드 성능 지표가 없다. 지표와 예산을 정한다. 정하기 전까지 성능 통과 조건을 두지 않는다.

### 8.2 채택 시 함께 고칠 문서

채택되면 다음 문서를 같은 변경에서 고친다. 이 초안 변경에서는 고치지 않는다.

- [xtask 모듈 설계](../architectures/xtask.md) 1·7·8절: 새 하위 명령과 대시보드 테스트 수집 방법
- [코드 지도](../architectures/code-map.md) `web/dashboard/` 행: 파일 구조와 내장 위치
- [관리 API 계약 1.11절](../contracts/management-api-v1.md): CSP 문구(FE-TBD-010)
- [테스트 컨벤션 7절](testing.md#7-테스트와-스펙-시나리오-연결): 브라우저 테스트 표식 규칙(FE-TBD-005)
- [개발 환경·검증 how-to 2절](../how-to/set-up-and-verify.md): Chromium 전제(FE-TBD-006)
- [DESIGN.md 검증 절](../design/DESIGN.md#검증): 검증 how-to와 생성 절차 링크(DESIGN-TBD-005를 해소할 때만)
