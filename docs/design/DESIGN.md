---
version: alpha
name: "VNC Relay 운영 대시보드"
description: "참고 이미지(밝은 회녹색 바탕 위 흰 카드 격자, 청록·녹색 차트 색)와 shadcn/ui의 의미 토큰·컴포넌트 형태를 따른 운영 도구용 라이트 테마 디자인 기준(Accepted, 2026-10-20 사용자 채택). 메뉴는 녹색 기운만 남긴 검정에 가까운 상단 막대에 둔다. 화면 문구는 한국어·영어다. 글꼴은 실행파일에 넣은 Pretendard이고 굵기는 400·700만 쓴다."
colors:
  background: "#f4f6f5"
  foreground: "#0d1a15"
  card: "#ffffff"
  muted: "#eef2f0"
  muted-foreground: "#58665f"
  border: "#e1e7e4"
  input: "#7d8a84"
  ring: "#12352b"
  primary: "#12352b"
  primary-foreground: "#ffffff"
  primary-hover: "#1e4a3d"
  link: "#0f7048"
  destructive: "#b42323"
  destructive-foreground: "#ffffff"
  destructive-hover: "#931b1b"
  destructive-bg: "#fdeeee"
  destructive-border: "#f2c4c4"
  success: "#166342"
  success-bg: "#e4f4eb"
  success-border: "#b5dec6"
  info: "#0b6173"
  info-bg: "#e3f2f5"
  info-border: "#b3dbe4"
  warning: "#8a4a00"
  warning-bg: "#fdf3dc"
  warning-border: "#efd497"
  neutral: "#47544e"
  neutral-bg: "#eaefec"
  neutral-border: "#d5ddd9"
  navbar: "#0b1210"
  navbar-foreground: "#eef3f0"
  navbar-muted-foreground: "#9eaca6"
  navbar-accent: "#1c2723"
  navbar-primary: "#5fd39a"
  navbar-ring: "#ffffff"
  navbar-brand: "#ffffff"
  chart-1: "#62b6c8"
  chart-2: "#4fb488"
  gauge: "#2a8b9f"
  gauge-track: "#e2eae7"
  overlay: "#0d1a1580"
  shadow: "#0d1a1514"
typography:
  heading-page:
    fontFamily: "Pretendard, 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: 24px
    fontWeight: 700
    lineHeight: 1.5
    letterSpacing: -0.03em
  heading-card:
    fontFamily: "Pretendard, 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: 16px
    fontWeight: 700
    lineHeight: 1.5
    letterSpacing: -0.03em
  body:
    fontFamily: "Pretendard, 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: -0.03em
  body-strong:
    fontFamily: "Pretendard, 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: 14px
    fontWeight: 700
    lineHeight: 1.5
    letterSpacing: -0.03em
  body-numeric:
    fontFamily: "Pretendard, 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: -0.03em
    fontFeature: "'tnum'"
  label:
    fontFamily: "Pretendard, 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: 12px
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: -0.03em
  badge:
    fontFamily: "Pretendard, 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: 12px
    fontWeight: 400
    lineHeight: normal
    letterSpacing: -0.03em
  button:
    fontFamily: "Pretendard, 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: 14px
    fontWeight: 400
    lineHeight: normal
    letterSpacing: -0.03em
  button-strong:
    fontFamily: "Pretendard, 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: 14px
    fontWeight: 700
    lineHeight: normal
    letterSpacing: -0.03em
  input:
    fontFamily: "Pretendard, 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: 16px
    fontWeight: 400
    lineHeight: normal
    letterSpacing: -0.03em
  data:
    fontFamily: "ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, 'Liberation Mono', monospace"
    fontSize: 12px
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: 0
    fontFeature: "'tnum'"
  metric:
    fontFamily: "Pretendard, 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: 28px
    fontWeight: 700
    lineHeight: 1.5
    letterSpacing: -0.03em
    fontFeature: "'tnum'"
spacing:
  xs: 4px
  sm: 8px
  md: 12px
  lg: 16px
  xl: 24px
  2xl: 32px
  control-height: 36px
  control-height-sm: 32px
  control-height-touch: 44px
  target-min: 24px
  content-max: 1440px
  appbar-height: 56px
  brand-size: 32px
  border-width: 1px
  indicator-width: 3px
  focus-ring-width: 2px
  focus-ring-offset: 2px
  shadow-y: 1px
  shadow-blur: 3px
  mark-size: 10px
  icon-size: 16px
  icon-size-sm: 14px
  icon-stroke: 1.75px
  gauge-size: 120px
  gauge-stroke: 12px
  label-col: 128px
  kv-label-col: 160px
  alert-text-min: 192px
  index-min: 240px
  login-max: 400px
  dialog-max: 480px
rounded:
  sm: 6px
  md: 8px
  lg: 10px
  xl: 14px
  mark: 3px
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.primary-foreground}"
    typography: "{typography.button-strong}"
    rounded: "{rounded.md}"
    padding: "{spacing.lg}"
    height: "{spacing.control-height}"
  button-primary-hover:
    backgroundColor: "{colors.primary-hover}"
    textColor: "{colors.primary-foreground}"
  button-outline:
    backgroundColor: "{colors.card}"
    textColor: "{colors.foreground}"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    padding: "{spacing.lg}"
    height: "{spacing.control-height}"
  button-outline-hover:
    backgroundColor: "{colors.muted}"
    textColor: "{colors.foreground}"
  button-ghost:
    backgroundColor: "{colors.background}"
    textColor: "{colors.foreground}"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    padding: "{spacing.md}"
    height: "{spacing.control-height}"
  button-ghost-hover:
    backgroundColor: "{colors.muted}"
    textColor: "{colors.foreground}"
  button-destructive:
    backgroundColor: "{colors.destructive}"
    textColor: "{colors.destructive-foreground}"
    typography: "{typography.button-strong}"
    rounded: "{rounded.md}"
    padding: "{spacing.lg}"
    height: "{spacing.control-height}"
  button-destructive-hover:
    backgroundColor: "{colors.destructive-hover}"
    textColor: "{colors.destructive-foreground}"
  input:
    backgroundColor: "{colors.card}"
    textColor: "{colors.foreground}"
    typography: "{typography.input}"
    rounded: "{rounded.md}"
    padding: "{spacing.md}"
    height: "{spacing.control-height}"
  card:
    backgroundColor: "{colors.card}"
    textColor: "{colors.foreground}"
    typography: "{typography.body}"
    rounded: "{rounded.xl}"
    padding: "{spacing.xl}"
  stat:
    backgroundColor: "{colors.card}"
    textColor: "{colors.foreground}"
    typography: "{typography.metric}"
  navbar:
    backgroundColor: "{colors.navbar}"
    textColor: "{colors.navbar-foreground}"
    typography: "{typography.body-strong}"
    height: "{spacing.appbar-height}"
  navbar-item-active:
    backgroundColor: "{colors.navbar-accent}"
    textColor: "{colors.navbar-foreground}"
    rounded: "{rounded.md}"
  navbar-item:
    backgroundColor: "{colors.navbar}"
    textColor: "{colors.navbar-muted-foreground}"
    rounded: "{rounded.md}"
  lang-select:
    backgroundColor: "{colors.navbar}"
    textColor: "{colors.navbar-foreground}"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    height: "{spacing.control-height}"
  gauge:
    backgroundColor: "{colors.gauge-track}"
    textColor: "{colors.gauge}"
    size: "{spacing.gauge-size}"
  badge-success:
    backgroundColor: "{colors.success-bg}"
    textColor: "{colors.success}"
    typography: "{typography.badge}"
    rounded: "{rounded.sm}"
    padding: "{spacing.xs}"
  badge-info:
    backgroundColor: "{colors.info-bg}"
    textColor: "{colors.info}"
    typography: "{typography.badge}"
    rounded: "{rounded.sm}"
    padding: "{spacing.xs}"
  badge-warning:
    backgroundColor: "{colors.warning-bg}"
    textColor: "{colors.warning}"
    typography: "{typography.badge}"
    rounded: "{rounded.sm}"
    padding: "{spacing.xs}"
  badge-destructive:
    backgroundColor: "{colors.destructive-bg}"
    textColor: "{colors.destructive}"
    typography: "{typography.badge}"
    rounded: "{rounded.sm}"
    padding: "{spacing.xs}"
  badge-neutral:
    backgroundColor: "{colors.neutral-bg}"
    textColor: "{colors.neutral}"
    typography: "{typography.badge}"
    rounded: "{rounded.sm}"
    padding: "{spacing.xs}"
  alert-info:
    backgroundColor: "{colors.info-bg}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.lg}"
    padding: "{spacing.lg}"
  alert-warning:
    backgroundColor: "{colors.warning-bg}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.lg}"
    padding: "{spacing.lg}"
  alert-destructive:
    backgroundColor: "{colors.destructive-bg}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.lg}"
    padding: "{spacing.lg}"
  alert-neutral:
    backgroundColor: "{colors.neutral-bg}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.lg}"
    padding: "{spacing.lg}"
---

# VNC Relay 운영 대시보드 디자인 기준

## Overview

이 문서는 운영 대시보드의 공통 시각 기준을 정하는 디자인 시스템이다. `docs/design/DESIGN.md`에 둔다. 화면 동작은 [대시보드 화면 스펙](../specs/dashboard.md)이, 교환 형식은 [관리 API v1 계약](../contracts/management-api-v1.md)이 정한다. 이 문서는 동작과 문구를 새로 정하지 않고 색·글꼴·간격·배치만 정한다.

**이 문서는 채택된 기준(Accepted)이다.** 사용자가 2026-10-20에 이 문서와 [dashboard.html](mockups/dashboard.html) 시안을 채택했다. 대시보드 구현은 이 문서의 토큰과 컴포넌트 기준을 따라야 한다. 남은 미결정 사항은 [미결정 사항](#미결정-사항)에서 관리한다.

| 항목 | 값 |
| --- | --- |
| 상태 | Accepted |
| 적용 대상 | 운영 대시보드(SCR-001 운영 현황, SCR-002 세션 상세, SCR-003 관리자 로그인). 라이트 테마만 둔다 |
| 채택 근거 | 사용자 채택(2026-10-20). 라이트 테마만 두는 결정도 같은 날의 사용자 결정이다 |
| 적용 시작일 | 2026-10-20 |
| 관련 PRD·스펙 | [PRD](../requirements/PRD.md) FR-06·AC-31, [대시보드 화면 스펙](../specs/dashboard.md) DASH-015·DASH-021·DASH-023 |
| 관련 아키텍처 문서·ADR | 없음. 정적 HTML·JS 선택은 [PRD 확정 기준](../requirements/PRD.md#확정-기준)이 정한다 |
| 기준 시안 | [dashboard.html](mockups/dashboard.html). 이 문서의 토큰으로 만든 화면별 시안이며 사용자가 2026-10-20에 채택했다 |
| 토큰 원본 | 이 파일의 YAML 머리말 |
| 컴포넌트 미리보기 | `web/dashboard/preview/`. `cargo xtask dashboard-serve`로 띄운다. 로컬 전용이며 실행 파일에 넣지 않는다. 명세의 컴포넌트를 상태별로 보여 준다(작업 4.8). |
| 명세·도구 버전 | [DESIGN.md 명세](https://github.com/google-labs-code/design.md/blob/main/docs/spec.md) `version: alpha`(2026-10-16 확인). 검사 도구 버전은 [검증](#검증)에 기록한다 |

### 디자인 방향

- 대상 사용자와 주요 작업: 로그인한 관리자가 대기 연결·활성 세션·최근 종료 목록을 읽고, 한 세션을 골라 종료한다. 이 작업을 하는 화면은 [화면 스펙 1절](../specs/dashboard.md#1-범위와-사용자)을 따른다.
- 화면 구성: 위쪽에 검정에 가까운 짙은 색의 상단 막대를 둔다. 왼쪽에 브랜드와 메뉴, 오른쪽에 언어 선택과 [로그아웃]이 있다. 아래 본문은 밝은 회녹색 바탕 위에 페이지 머리말과 흰 카드 격자를 놓는다. 카드마다 제목 줄과 구분선이 있다. 서버 요약과 중계 연결·관리 연결 사용량은 첫 줄 카드에, 목록은 그 아래 전체 폭 카드에 둔다. 자세한 배치는 [화면 구성](#화면-구성)을 따른다.
- 정보 밀도와 강조 방식: 요약 숫자는 크게 보인다. 표는 촘촘한 행을 유지한다. 운영 도구는 한 화면에 많은 행을 보여야 하기 때문이다. 강조는 요약 숫자, 연결 사용량 고리, 상태 표시, 종료 행동에만 쓴다. 정상 값에는 배지를 붙이지 않는다.
- 시각적 특징: shadcn/ui의 형태를 따른다. 카드는 흰 바탕에 1px 경계와 얕은 그림자, 14px 모서리를 쓴다. 버튼·입력은 8px 모서리, 36px 높이이고, 포커스는 2px 링으로 표시한다. 배지는 6px 모서리의 작은 사각형이다. 주요 버튼은 짙은 녹색이고, 세션 종료 버튼만 빨간색이다. 청록·녹색은 데이터 표식과 사용량 고리에만 쓴다.
- 참고 자료: [참고 자료](#참고-자료)에 기록한다. 접근성은 [WCAG 2.2](https://www.w3.org/TR/WCAG22/) 대비·포커스·대상 크기 기준을 따른다.
- 대표 화면: [dashboard.html](mockups/dashboard.html)의 SCR-001(요약 카드·사용량 고리·긴 표 7열, 빈 목록·오류·갱신 중 상태), SCR-002(종료 확인 대화상자, 긴 주소), SCR-003(입력 폼). 확인할 시나리오는 [대시보드 화면 스펙](../specs/dashboard.md)의 DASH-001~003·DASH-009·DASH-015·DASH-018·DASH-021·DASH-023이다. 시안의 토큰 블록은 채택된 YAML에서 만든 값과 같다([검증](#검증)).

#### 참고 자료

**사용자 제공 첨부 이미지 두 장(2026-10-20), 원본 링크 없음.** 첫째는 SEO 분석 대시보드 화면("My Dashboard")이다. 둘째는 같은 제품의 위젯 편집 패널을 확대한 화면이다. 제작자·출처·사용 조건을 알 수 없다. 이 프로젝트는 이미지의 시각 방향만 참고한다. 이미지의 문구·로고·아이콘·이미지 자산은 복제하지 않는다. 이미지 파일을 저장소에 두지 않는다.

| 구분 | 요소 | 적용 방식과 이유 |
| --- | --- | --- |
| 벗어난 점 | 세로 내비게이션 대신 상단 막대. 활성 항목의 밝은 바탕과 녹색 표시 막대는 유지한다 | 사용자 요청(2026-10-20)이다. 메뉴 항목이 하나뿐이라 세로 레일은 본문 폭만 줄인다. `navbar` 토큰을 쓴다. 활성 항목은 `navbar-accent` 바탕과 아래쪽 `navbar-primary` 막대로 표시한다. 아이콘 옆에 글씨 레이블을 둔다. 아이콘만으로 뜻을 전하지 않기 위해서다 |
| 채택 | 밝은 회녹색 바탕, 큰 페이지 제목, 제목 아래 흐린 보조 줄 | 페이지 머리말은 바탕(`background`) 위에 둔다. 보조 줄에는 마지막 성공 갱신 시각을 쓴다 |
| 채택 | 흰 카드 격자, 카드 제목 줄 아래 구분선, 오른쪽 위 조작 | `card` 컴포넌트를 쓴다. SCR-001의 세 목록 제목 옆에는 개수 배지를 둔다. 문구는 [화면 스펙 CPY-084·085](../specs/dashboard.md#932-scr-001-운영-현황)를 따른다(사용자 결정 2026-11-05(Q115)) |
| 채택 | 고리형 게이지(DR·UR·Top AR)와 가운데 큰 숫자 | 중계 연결·관리 연결 사용량에 `gauge`를 쓴다. 현재 값과 상한을 글씨로 함께 쓴다 |
| 채택 | 범례의 작은 색 사각형(청록·녹색) | 방향별 바이트 앞의 표식(`chart-1`·`chart-2`)으로 쓴다 |
| 채택 | 표의 흐린 열 머리글과 얇은 행 구분선 | `muted-foreground` 머리글과 `border` 구분선을 쓴다 |
| 채택 | 둘째 이미지의 폼 행, 녹색으로 칠한 선택 상태, 짙은 녹색 주요 버튼 | 입력과 주요 버튼(`primary`)에 쓴다 |
| 벗어난 점 | 세션 종료·종료 확인 버튼은 빨간색 | 되돌릴 수 없는 행동을 주요 버튼과 구분한다. 이미지에는 위험 행동이 없다 |
| 벗어난 점 | 입력 경계는 3:1 대비의 진한 회녹색(`input`) | 입력은 경계로 식별하므로 WCAG 1.4.11의 3:1이 필요하다. 이미지와 shadcn/ui의 연한 경계는 이 기준에 못 미친다 |
| 벗어난 점 | 사용량 고리의 채움은 진한 청록(`gauge`) | 이미지의 연한 청록은 흰 바탕에서 3:1에 못 미친다 |
| 벗어난 점 | 메뉴 바탕은 이미지의 짙은 녹색 대신 녹색 기운만 남긴 검정에 가까운 색(`navbar`) | 사용자 요청(2026-10-20)이다. 채도 높은 녹색 면이 넓으면 투박해 보인다. 녹색은 활성 표시 막대(`navbar-primary`)에만 남긴다. 브랜드 표식 선은 순백색(`navbar-brand`)이다(사용자 요청, 2026-10-20) |

**[shadcn/ui](https://ui.shadcn.com/docs/theming)(Theming 문서, 2026-10-20 확인).** React·Tailwind CSS 기반 컴포넌트 모음이다. 이 프로젝트는 시각 관례만 참고한다. 코드·패키지·빌드 도구는 쓰지 않는다.

| 구분 | 요소 | 적용 방식과 이유 |
| --- | --- | --- |
| 채택 | 의미 토큰 이름(`background`·`foreground`·`card`·`muted`·`muted-foreground`·`border`·`input`·`ring`·`primary`·`primary-foreground`·`destructive`·`chart-*`) | 같은 이름을 `colors.*`에 쓴다. 바탕 토큰과 `-foreground` 글씨 토큰을 짝으로 쓴다. 메뉴 색은 shadcn/ui `sidebar-*`와 같은 구조(`navbar`·`-foreground`·`-accent`·`-primary`·`-ring`)로 두되, 상단 막대에 쓰므로 이름은 `navbar-*`다 |
| 채택 | 모서리 단계(`sm`·`md`·`lg`·`xl`) | `rounded.sm` 6px, `md` 8px, `lg` 10px, `xl` 14px다. 문서 기본값 0.625rem(10px)을 `lg`로 보고 비율을 맞췄다 |
| 채택 | Button 변형(default·outline·ghost·destructive), Card, Badge, Alert, Dialog, Table, Input의 형태 | [Components](#components)에서 변형별로 정한다 |
| 채택 | 포커스 링: 2px 링과 2px 간격 | `ring` 색을 쓴다. 상단 막대 안에서는 `navbar-ring`을 쓴다 |

### 정적 파일 원칙

대시보드는 별도 빌드 단계 없는 정적 HTML·JS다([화면 스펙 1절](../specs/dashboard.md#1-범위와-사용자)). 따라서 다음을 지켜야 한다.

- 글꼴·스타일·스크립트·아이콘을 외부에서 불러오지 않는다. CDN, 웹 글꼴 서비스, `@import url(...)`, 아이콘 글꼴, shadcn/ui·Tailwind CSS 패키지를 쓰지 않는다.
- 모든 CSS·이미지·글꼴 파일은 실행파일에 포함한 정적 파일이어야 한다. 글꼴 파일은 Pretendard 부분 글자 woff2 두 개뿐이다(사용자 결정 2026-11-07(Q139), [Typography](#typography)).
- 아이콘은 직접 그린 인라인 SVG를 쓴다. 아이콘만으로 뜻을 전하지 않는다.
- 토큰은 CSS 사용자 정의 속성(`--*`)으로 한 곳에서 정의한다. 아래 [이름 규칙](#css-사용자-정의-속성-이름-규칙)을 따른다.

## Colors

색 값의 원본은 YAML 머리말이다. 본문은 토큰 이름과 사용 조건을 쓴다. 상태 이름은 [관리 API v1 계약](../contracts/management-api-v1.md)과 [화면 스펙 4절](../specs/dashboard.md#4-화면-상태와-갱신)의 이름을 따른다.

### CSS 사용자 정의 속성 이름 규칙

| YAML 위치 | CSS 속성 이름 | 예 |
| --- | --- | --- |
| `colors.<이름>` | `--color-<이름>` | `colors.muted-foreground` → `--color-muted-foreground` |
| `spacing.<이름>` | `--space-<이름>` | `spacing.control-height` → `--space-control-height` |
| `rounded.<이름>` | `--radius-<이름>` | `rounded.xl` → `--radius-xl` |
| `typography.<단계>.fontFamily·fontSize·fontWeight·lineHeight·letterSpacing·fontFeature` | `--type-<단계>-family·size·weight·line·tracking·feature` | `typography.body.letterSpacing` → `--type-body-tracking` |

값을 CSS에서 직접 쓰지 않고 `var(--…)`로 참조한다. 이 규칙은 YAML에서 CSS를 기계적으로 만들 수 있게 한다. 생성 방법과 대조는 [프런트엔드 컨벤션 2절](../conventions/frontend.md#2-토큰-적용과-재사용)에서 정한다(사용자 결정 2026-10-22(Q32)). 폭 토큰(`spacing.label-col`·`kv-label-col`·`alert-text-min`·`index-min`·`login-max`·`dialog-max`)은 레이아웃 폭을 정한다. 미디어 쿼리 조건은 사용자 정의 속성을 쓸 수 없어 예외다([중단점과 모바일 배치](#중단점과-모바일-배치)).

### 색 토큰과 사용 조건

| 토큰 | 역할 | 사용 조건 | 함께 쓰는 토큰·표시 |
| --- | --- | --- | --- |
| `colors.background` | 페이지 바탕 | 상단 막대 아래 본문 전체 바탕. 밝은 회녹색이다. 페이지 머리말은 이 바탕 위에 둔다 | `colors.foreground`·`colors.muted-foreground` |
| `colors.foreground` | 본문·값·제목 | 모든 값과 제목 | `colors.card`·`colors.background`·`colors.muted`·상태 바탕 |
| `colors.card` | 카드·입력·대화상자·outline 버튼 바탕 | 표·요약·폼의 기본 표면 | `colors.foreground`·`colors.muted-foreground` |
| `colors.muted` | 약한 표면 | 표 행 hover, outline·ghost 버튼 hover, 비활성 버튼 바탕. 표 행 줄무늬로 쓰지 않는다 | `colors.foreground`·`colors.muted-foreground` |
| `colors.muted-foreground` | 보조 글씨 | 열 머리글, 요약 레이블, 단위, 상한 값, 보조 줄. 12px 이상에서만 쓴다 | `colors.card`·`colors.background`·`colors.muted`에서 4.5:1 이상 |
| `colors.border` | 장식 경계·구분선 | 카드 경계, 카드 제목 줄 구분선, 표 행 구분선, outline 버튼 경계. 정보를 전달하지 않는 선만 해당한다 | 3:1을 요구하지 않는다 |
| `colors.input` | 입력 상자 경계 | 입력 상자 경계. 입력은 경계로 식별하므로 3:1 이상이어야 한다 | `colors.card`·`colors.background`에서 3:1 이상 |
| `colors.ring` | 포커스 링 | 상단 막대 밖 모든 조작 요소의 `:focus-visible` 윤곽선 | `spacing.focus-ring-width`·`spacing.focus-ring-offset` |
| `colors.primary` / `colors.primary-foreground` / `colors.primary-hover` | 주요 행동 | 로그인 버튼. 한 영역에 주요 버튼은 하나만 둔다 | 버튼 글씨 |
| `colors.link` | 링크 글씨 | 세션 식별자 링크 | 밑줄을 함께 쓴다 |
| `colors.destructive` / `colors.destructive-foreground` / `colors.destructive-hover` | 위험한 행동 | 세션 종료 버튼과 종료 확인 버튼에만 쓴다. 입력 오류 경계와 오류 문장에도 쓴다 | 버튼에 "종료" 글씨를 쓴다 |
| `colors.navbar` / `colors.navbar-foreground` / `colors.navbar-muted-foreground` | 상단 막대 바탕·글씨 | 모든 폭의 상단 막대. 브랜드 이름·활성 항목·언어 선택 글씨는 `navbar-foreground`, 나머지 항목은 `navbar-muted-foreground`다. 언어 선택 경계도 `navbar-muted-foreground`다. 로그인 화면의 브랜드 표식 바탕은 `navbar`다 | 상단 막대 안의 모든 글씨 |
| `colors.navbar-accent` / `colors.navbar-primary` | 상단 막대 활성 항목 | `navbar-accent`는 활성 항목·hover 바탕과 상단 막대 브랜드 표식 바탕, `navbar-primary`는 활성 항목 아래 표시 막대다 | 글씨 레이블 |
| `colors.navbar-ring` | 상단 막대 포커스 링 | 상단 막대 안 조작 요소의 `:focus-visible` 윤곽선 | `spacing.focus-ring-width` |
| `colors.navbar-brand` | 브랜드 표식 선 | 상단 막대와 로그인 화면의 브랜드 표식 화살표 선. 순백색이다(사용자 요청, 2026-10-20). 글씨에는 쓰지 않는다 | 브랜드 이름 글씨 |
| `colors.chart-1` / `colors.chart-2` | 방향 표식 | 방향별 바이트 레이블 앞의 작은 사각형이다. Viewer->대상은 `chart-1`(청록), 대상->Viewer는 `chart-2`(녹색)다. 장식이며 글씨 레이블이 뜻을 전한다 | 방향 이름 글씨 |
| `colors.gauge` / `colors.gauge-track` | 사용량 고리 | `gauge`는 채움 호, `gauge-track`은 트랙이다. 중계 연결·관리 연결 고리에만 쓴다 | 현재·상한 값을 글씨로 함께 쓴다 |
| `colors.overlay` | 대화상자 뒤 가림막 | 모달 대화상자가 열린 동안 | 반투명 값. 대비 기준 대상이 아니다 |
| `colors.shadow` | 카드·대화상자 그림자 색 | [Elevation & Depth](#elevation--depth)의 그림자에만 쓴다 | 반투명 값. 대비 기준 대상이 아니다 |

`chart-*` 토큰은 글씨 색으로 쓰지 않는다. 흰 바탕에서 4.5:1을 만족하지 못한다([대비 기준](#대비-기준)).

### 상태 색

상태 표시는 색만으로 구분하지 않는다([화면 스펙 6절](../specs/dashboard.md#6-화면-크기와-접근성)). 모든 상태는 **글씨 레이블 + 표시 도형 + 색**으로 나타낸다. 한 상태는 같은 이름의 `<상태>`·`<상태>-bg`·`<상태>-border` 세 토큰을 쓴다. 배지는 `<상태>` 글씨와 `<상태>-bg` 바탕을 쓴다. 안내(Alert)는 `<상태>-bg` 바탕과 `<상태>-border` 경계를 쓰고, 아이콘과 제목은 `<상태>`, 설명은 `foreground` 색이다.

| 상태 색 | 토큰 | 대상 상태 | 설명 |
| --- | --- | --- | --- |
| 성공 | `colors.success`·`success-bg`·`success-border` | 서버 실행 상태 `ready`(준비됨) | 평소 값은 배지를 붙이지 않는다. 서버 실행 상태만 배지로 보인다 |
| 정보 | `colors.info`·`info-bg`·`info-border` | 최초 로딩·갱신 중·종료 요청 중 | 진행 중임을 나타낸다. 참고 이미지의 청록 계열이다 |
| 경고 | `colors.warning`·`warning-bg`·`warning-border` | 서버 실행 상태 `shutdown_grace`(종료 유예 중), 세션 상태 `closing`(종료 중), 오래된 성공 데이터, 종료 결과 불명, 종료 원인 `dead_connection`·`close_timeout` | 주의가 필요하지만 실패는 아니다 |
| 위험 | `colors.destructive`·`destructive-bg`·`destructive-border` | 조회 실패, 종료 실패, 로그인 실패 안내, 종료 원인 `connection_error`·`internal_error` | 실패를 뜻한다. 빈 목록과 구분한다. `internal_error`의 위험 톤은 이 문서의 판단이다(사용자 결정 없음) |
| 중립 | `colors.neutral`·`neutral-bg`·`neutral-border` | 활성 세션 모드·전송 경로·역할, 확인 불가 값, 대상 없음, 권한 없음, 종료 원인 `peer_eof`·`admin_terminated`·`server_shutdown`, 목록에 없는 알 수 없는 값 | 알 수 없는 값은 받은 그대로 보여주고 오류 색을 쓰지 않는다([관리 API 계약](../contracts/management-api-v1.md) 3절) |

세션 상태 `active`는 배지 없이 표시한다. 정상 상태를 모두 강조하면 예외가 보이지 않기 때문이다. 이 매핑은 시각 표현만 정한다. 상태의 의미는 스펙과 계약이 정한다.

"조회 중"(CPY-016)은 나오는 모든 자리에서 같은 정보 배지(`badge-info`, 시계 아이콘 + 글씨)로 쓴다. SCR-001의 자리는 카드 머리의 상태 배지, 서버 요약 값, 사용량 고리 가운데, 목록 표의 빈 행이다. SCR-002의 자리는 세션 정보 카드 머리, 사실 다섯 줄의 값, 전송량 두 방향의 값이다. SCR-002는 첫 조회 중에도 항목 이름과 방향 표식을 보이고 값 자리에만 배지를 둔다(사용자 결정 2026-11-07(Q143)). 같은 상태가 자리마다 다르게 보이지 않게 하기 위해서다. "확인 불가"는 중립 배지 그대로다. 이 규칙은 위 표의 정보 행(최초 로딩)을 모든 자리에 적용한 것이다. SCR-001 적용은 사용자 지적(2026-11-07)을 반영한 이 문서의 판단이며 사용자 결정은 없다.

### 테마

라이트 테마만 둔다(사용자 결정, 2026-10-20). `prefers-color-scheme`에 따른 대체 값과 수동 전환 버튼을 두지 않는다. 시안과 구현은 `color-scheme: light`를 선언한다.

### 대비 기준

기준은 WCAG 2.2 AA다. 일반 글씨는 대비 4.5:1 이상, 입력 경계·포커스 링·사용량 고리 채움은 3:1 이상이어야 한다. 아래 값은 YAML의 값에서 계산했다([검증](#검증)). 상태 쌍은 배지·안내 바탕 위의 글씨를 확인했다.

| 전경 | 배경 | 기준 | 대비 |
| --- | --- | --- | --- |
| `foreground` | `card` | 4.5 | 17.86 |
| `foreground` | `background` | 4.5 | 16.46 |
| `foreground` | `muted` | 4.5 | 15.82 |
| `muted-foreground` | `card` | 4.5 | 6.03 |
| `muted-foreground` | `background` | 4.5 | 5.56 |
| `muted-foreground` | `muted` | 4.5 | 5.34 |
| `link` | `card` | 4.5 | 6.12 |
| `link` | `muted` | 4.5 | 5.42 |
| `primary-foreground` | `primary` | 4.5 | 13.38 |
| `primary-foreground` | `primary-hover` | 4.5 | 9.99 |
| `destructive-foreground` | `destructive` | 4.5 | 6.55 |
| `destructive-foreground` | `destructive-hover` | 4.5 | 8.70 |
| `destructive`(오류 문장) | `card` | 4.5 | 6.55 |
| `success` | `success-bg` | 4.5 | 6.36 |
| `info` | `info-bg` | 4.5 | 6.15 |
| `warning` | `warning-bg` | 4.5 | 6.22 |
| `destructive` | `destructive-bg` | 4.5 | 5.82 |
| `neutral` | `neutral-bg` | 4.5 | 6.82 |
| `foreground` | `info-bg`·`warning-bg`·`destructive-bg`·`neutral-bg` | 4.5 | 최소 15.36 |
| `navbar-foreground` | `navbar` | 4.5 | 16.89 |
| `navbar-muted-foreground` | `navbar` | 4.5 | 8.04 |
| `navbar-foreground` | `navbar-accent` | 4.5 | 13.72 |
| `navbar-muted-foreground` | `navbar-accent` | 4.5 | 6.53 |
| `input` | `card` | 3 | 3.60 |
| `input` | `background` | 3 | 3.31 |
| `ring` | `card` | 3 | 13.38 |
| `ring` | `background` | 3 | 12.32 |
| `navbar-ring` | `navbar` | 3 | 18.95 |
| `navbar-ring` | `navbar-accent` | 3 | 15.40 |
| `navbar-primary`(활성 표시 막대) | `navbar` | 3 | 10.17 |
| `navbar-muted-foreground`(언어 선택 경계) | `navbar` | 3 | 8.04 |
| `input`(로그인 화면 언어 선택 경계) | `background` | 3 | 3.31 |
| `navbar-brand`(상단 막대 브랜드 표식 선) | `navbar-accent` | 3 | 15.40 |
| `navbar-brand`(로그인 화면 브랜드 표식 선) | `navbar` | 3 | 18.95 |
| `gauge` | `gauge-track` | 3 | 3.24 |
| `gauge` | `card` | 3 | 3.97 |

기준이 없는 쌍은 따로 적는다.

| 전경 | 배경 | 대비 | 판정 |
| --- | --- | --- | --- |
| `chart-1` | `card` | 2.32 | 기준 없음. 장식 표식이다 |
| `chart-2` | `card` | 2.56 | 기준 없음. 장식 표식이다 |
| `gauge-track` | `card` | 1.22 | 기준 없음. 값을 전달하지 않는 배경이다 |
| `border` | `card` | 1.25 | 기준 없음. 장식 구분선이다 |

- 방향 표식(`chart-1`·`chart-2`)은 장식이다. 방향 이름은 글씨로 쓰므로 대비 기준 대상이 아니다.
- outline 버튼과 카드의 경계는 3:1을 요구하지 않는다. 버튼은 글씨 레이블로, 카드는 제목으로 식별한다.

## Typography

글꼴 값의 원본은 YAML이다. 글꼴은 실행파일에 넣은 Pretendard이고, 값(고정 폭)만 시스템 고정 폭 글꼴이다. 아래 규칙은 사용자 지시(2026-11-07)와 그 세부 결정 Q139~Q142를 따른다.

| 역할 | 토큰 | 적용 대상 | 콘텐츠 처리 |
| --- | --- | --- | --- |
| 페이지 제목 | `typography.heading-page` | 화면 이름(운영 현황, 세션 상세의 "세션 …", 관리자 로그인 카드 제목). 24px 굵기 700, 행간 150% | 좁은 화면에서 줄바꿈한다. 말줄임으로 정보를 숨기지 않는다 |
| 카드 제목 | `typography.heading-card` | 카드 제목(서버 요약, 중계 연결, 관리 연결, 대기 연결, 활성 세션, 최근 종료 목록, 세션 정보, 전송량), 대화상자 제목. 16px 굵기 700, 행간 150% | 같음 |
| 본문 | `typography.body` | 설명, 오류·안내 문장. 14px 굵기 400, 행간 150% | 한글은 단어 단위로 줄바꿈한다(`word-break: keep-all`) |
| 강조 본문 | `typography.body-strong` | 안내 제목, 상단 막대 브랜드 이름, 시안 틀의 화면 식별자. 14px 굵기 700, 행간 150% | 같음 |
| 숫자 본문 | `typography.body-numeric` | 표 안의 개수, 시각, 전송량, 시간 길이. 14px 굵기 400, 행간 150% | 자릿수가 맞는 숫자(`tnum`). 숫자 열은 오른쪽 정렬한다 |
| 레이블·보조 글씨 | `typography.label` | 열 머리글, 요약 레이블, 모바일 카드의 레이블, 입력 레이블, 카드 설명. 12px 굵기 400, 행간 150% | 12px 아래로 줄이지 않는다. 필요 시 `colors.muted-foreground`와 함께 쓴다 |
| 배지 | `typography.badge` | 배지 글씨. 12px 굵기 400, 행간 `normal` | 한 줄이다 |
| 버튼 | `typography.button` | outline·ghost 버튼, 상단 막대 항목, 언어 선택. 14px 굵기 400, 행간 `normal` | 한 줄. 좁은 폭에서 줄바꿈이 필요하면 버튼을 전체 폭으로 만든다 |
| 강조 버튼 | `typography.button-strong` | 주요 버튼과 위험 버튼. 14px 굵기 700, 행간 `normal` | 같음 |
| 입력 | `typography.input` | 계정명·비밀번호 입력. 16px 굵기 400, 행간 `normal` | 모든 폭에서 16px 이상이다. 모바일 브라우저가 입력 때 화면을 확대하지 않게 한다 |
| 값(고정 폭) | `typography.data` | 세션 식별자, 중계 ID, 상대 주소. 12px 굵기 400, 행간 150%, 자간 0 | 긴 값은 `overflow-wrap: anywhere`로 줄바꿈한다. 값을 잘라 표시하지 않는다 |
| 요약 숫자 | `typography.metric` | 서버 요약의 개수·시간 길이, 사용량 고리 가운데 현재 값, 세션 상세의 전송량. 28px 굵기 700, 행간 150% | 자릿수가 맞는 숫자. 폭이 모자라면 줄바꿈한다 |

- 글꼴: Pretendard 1.3.9의 부분 글자 woff2 두 개(Regular 400, Bold 700)를 실행파일에 넣는다(사용자 결정 2026-11-07(Q139)). 부분 글자 파일은 KS X 1001 한글 2,350자와 라틴 문자를 담는다. 이 밖의 글자는 그 글자만 다음 글꼴로 보인다. 글꼴 스택은 `Pretendard, 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', system-ui, -apple-system, 'Segoe UI', sans-serif`다.
- 굵기: 400(Regular)과 700(Bold)만 쓴다. 파일을 두 개로 줄이고, 대체 글꼴인 맑은 고딕(Malgun Gothic)에서도 위계를 지키기 위해서다. 맑은 고딕은 400과 700만 지원하며 500(Medium)이나 600(SemiBold)은 400으로 렌더링되어 위계가 사라진다. 이 이유는 **이 문서의 판단, 사용자 결정 없음**이다.
- 크기: 모든 글꼴 크기는 짝수 px이고 12px 이상이어야 한다(사용자 지시 2026-11-07). 값 글꼴은 13px에서 12px로 바꿨다(사용자 결정 2026-11-07(Q141)).
- 자간: Pretendard를 쓰는 모든 단계는 -3%(`-0.03em`)다. 값(고정 폭) 단계는 0이다. 고정 폭 글꼴의 글자 폭과 자릿수 정렬을 지키기 위해서다(사용자 결정 2026-11-07(Q142)). 글꼴 단계를 쓰는 CSS 규칙은 `font-family`와 함께 `letter-spacing`도 그 단계 값으로 정해야 한다. 자간은 상속되므로 생략하면 부모의 값이 남는다.
- 행간: 두 줄 이상이 될 수 있는 역할은 150%(`1.5`)다. 한 줄로 고정된 역할(배지, 버튼, 강조 버튼, 입력, 버튼 글꼴을 쓰는 상단 막대 항목과 언어 선택)은 `normal`(글꼴의 기본 행간)이다. CSS는 실제 줄 수를 알 수 없으므로 역할별로 정한다(사용자 결정 2026-11-07(Q140)).
- 시각 위계: 굵기(400 대 700), 크기, 색(`foreground` 대 `muted-foreground`)의 조합으로 위계를 드러낸다. 상대 굵기(`bolder`)를 쓰지 않고 명시적 700 토큰을 쓴다.
- 글꼴 자산과 사용 권한: Pretendard는 SIL Open Font License 1.1이다. 원본 파일과 라이선스 전문은 `web/dashboard/fonts/`에 둔다. 라이선스 전문은 배포 고지 `THIRD-PARTY-NOTICES.txt`에도 넣는다. 처리 방법은 [프런트엔드 컨벤션 2절](../conventions/frontend.md#2-토큰-적용과-재사용)을 따른다.
- 글꼴 로딩과 대체: `@font-face`는 `font-display: swap`을 쓴다. 글꼴 파일을 받기 전이나 받지 못했을 때는 위 스택의 다음 글꼴로 보인다. 데이터 고정 폭 글꼴은 `ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, 'Liberation Mono', monospace`를 쓴다.
- UI 문구 규칙: 화면 문구는 [화면 스펙](../specs/dashboard.md)이 정한다. 문구 확정은 [화면 스펙 9절](../specs/dashboard.md#9-화면-문구)이 관리한다(사용자 결정 2026-10-22(Q49)). 이 문서는 문구를 정하지 않는다.
- 화면 언어: 한국어와 영어 두 가지다. 언어 선택 규칙은 [화면 스펙의 화면 언어](../specs/dashboard.md#화면-언어)를 따른다. 두 언어가 같은 글꼴 스택과 토큰을 쓴다. 문서 루트의 `lang` 속성을 고른 언어(`ko`·`en`)로 바꾼다. 영어 문구가 한국어보다 길 수 있으므로 버튼·레이블·카드 제목에 고정 폭을 두지 않는다. 언어 이름은 각 언어로 쓴다("한국어", "English").

### 표시 형식

값의 의미와 단위는 [화면 스펙 3절](../specs/dashboard.md#시각전송량-표시-형식)이 정한다. 이 절은 그 스펙이 맡긴 자릿수·전환 경계를 정한다. 화면 언어를 바꿔도 형식은 같다. 단위 글씨만 언어에 따라 바뀐다(사용자 결정, 2026-10-20).

| 값 | 형식 | 예 |
| --- | --- | --- |
| 전송량 | KiB와 MiB만 쓴다. 소수 첫째 자리까지 반올림해 표시한다. KiB 값이 반올림 뒤 1024.0 이상이면 MiB로 바꾼다. 자릿수 구분 기호는 브라우저 언어 설정을 따르며 화면 언어 선택과 관계없다. 1 GiB 이상도 MiB로 쓴다 | `0`→`0.0 KiB`, `1536`→`1.5 KiB`, `1048575`→`1.0 MiB`, `1572864`→`1.5 MiB` |
| 시각 | 브라우저 현지 시각 `YYYY-MM-DD HH:mm:ss`와 시간대 약어를 함께 쓴다. 두 언어에서 같다 | `2026-10-16 14:03:25 GMT+9` |
| 시간 길이(가동·대기) | `D일 HH:MM:SS`(영어 `Dd HH:MM:SS`) 형식이다. 일수가 0이면 일 부분을 뺀다 | `1일 02:03:04`·`1d 02:03:04`, `00:00:07` |
| 확인 불가 값 | 글씨 "확인 불가"(영어 "Unavailable")와 중립 상태 색을 쓴다. 0이나 빈 칸으로 쓰지 않는다 | 연결 수 `null` |

## Layout

### 화면 구성

보호된 화면(SCR-001·SCR-002)은 앱 틀(상단 막대 + 본문) 안에 놓인다. 로그인 화면(SCR-003)은 상단 막대 없이 바탕 가운데에 카드 하나를 두고, 카드 위 오른쪽에 언어 선택을 둔다.

```text
+--------------------------------------------------------------------------+
| [표식] VNC Relay  [운영 현황]                       [언어 v] [로그아웃]   |
+--------------------------------------------------------------------------+
| 운영 현황                                                   [새로고침]   |
| 마지막 성공 갱신: 2026-10-16 14:03:25 GMT+9                              |
| (상태 안내: 조회 중·갱신 중·조회 실패·권한 없음일 때만)                   |
| +------------------------------+ +---------------+ +---------------+   |
| | 서버 요약                    | | 중계 연결     | | 관리 연결     |   |
| | 실행 상태  가동 시간  대기   | |   ( 312 )     | |   (  6  )     |   |
| | 활성  초기 교환 중  Mode I   | |  상한 1,000   | |  상한 100     |   |
| +------------------------------+ +---------------+ +---------------+   |
| +--------------------------------------------------------------------+ |
| | 대기 연결 [3건]                                                    | |
| +--------------------------------------------------------------------+ |
| | 활성 세션 [3건]                                                    | |
| +--------------------------------------------------------------------+ |
| | 최근 종료 목록 [7건]                                               | |
| +--------------------------------------------------------------------+ |
+--------------------------------------------------------------------------+
```

| 영역 | 데스크톱 폭 배치 | 근거 |
| --- | --- | --- |
| 상단 막대 | 높이 `spacing.appbar-height`, 전체 폭, `navbar` 바탕. 왼쪽부터 브랜드 표식과 "VNC Relay", "운영 현황" 항목(SCR-001·SCR-002에서 활성), 오른쪽 끝에 언어 선택과 [로그아웃]. 가로 안쪽 여백은 본문과 같은 `spacing.xl`이라 브랜드와 페이지 제목의 왼쪽 선이 맞는다 | 화면 스펙의 로그아웃 행동(DASH-019)과 화면 언어. 내비게이션 항목은 SCR-001 하나다 |
| 페이지 머리말 | 왼쪽에 제목과 마지막 성공 갱신 시각, 오른쪽에 [새로고침]. SCR-002는 제목 위에 [목록으로] ghost 버튼 | DASH-003~005·DASH-007 |
| 상태 안내 | 머리말 아래 전체 폭 Alert. 상태가 있을 때만 | 화면 스펙 4절 |
| SCR-001 첫 줄 | 12열 격자. 서버 요약 카드 6열, 중계 연결 카드 3열, 관리 연결 카드 3열. 연결 카드는 제목 아래 한 줄 설명으로 무엇을 세는지 밝힌다 | DASH-001·DASH-021 |
| SCR-001 목록 | 대기 연결, 활성 세션, 최근 종료 목록 카드를 이 순서로 전체 폭에 쌓는다. 화면 스펙 와이어프레임의 순서와 같다 | DASH-001·DASH-002·DASH-020 |
| SCR-002 | 세션 정보 카드 8열(모드·전송 경로·시작 시각·주소, 아래 카드 바닥에 [세션 종료]), 전송량 카드 4열(방향별 바이트) | DASH-006·DASH-009 |
| SCR-003 | 바탕 가운데, 위 오른쪽에 언어 선택(로그인형), 그 아래 브랜드 표식, 아래 로그인 카드 | DASH-017·DASH-018·DASH-024·DASH-025 |

- 공통 콘텐츠 폭: 본문은 `spacing.content-max` 이하다. 좁은 화면에서는 전체 폭을 쓴다.
- 바깥 여백과 간격: 간격은 `spacing.xs`~`spacing.2xl` 척도만 쓴다. 데스크톱 폭의 본문 바깥 여백은 `spacing.2xl`(세로)·`spacing.xl`(가로)이다. 머리말·안내·카드 줄 사이와 카드 사이는 `spacing.lg`다. 카드 제목 줄의 안쪽 여백은 `spacing.lg`(세로)·`spacing.xl`(가로), 카드 본문은 `spacing.xl`이다. 표가 들어간 카드 본문은 가로 여백을 `spacing.md`로 줄이고, 표 칸 안쪽은 `spacing.sm`(세로)·`spacing.md`(가로)다. 첫 열 글씨가 카드 제목과 왼쪽 선이 맞는다. 모바일 폭의 본문 바깥 여백은 `spacing.lg`, 카드 본문은 `spacing.lg`다.
- 밀도 변형: 없음. 하나의 밀도만 쓴다. 모바일 폭에서는 조작 요소 높이만 `spacing.control-height-touch`로 키운다.
- 콘텐츠 경계 조건: 긴 중계 ID·IPv6 주소·세션 식별자는 줄바꿈해 모두 보여준다. 시각 값은 폭이 모자라면 날짜와 시각 두 줄로 나눈다. 빈 목록은 스펙의 빈 목록 안내를 표 자리에 보이고 열 머리글은 유지한다. 알 수 없는 상태 값은 받은 그대로 보여준다.

### 중단점과 모바일 배치

화면 동작은 [화면 스펙](../specs/dashboard.md)을 따른다. 이 절은 배치 기준만 정한다.

| 이름 | 조건 | 적용 |
| --- | --- | --- |
| 모바일 폭 | 뷰포트 폭 1023.98px 이하(`@media (max-width: 1023.98px)`) | 상단 막대에서 "운영 현황" 항목과 브랜드 이름 글씨를 감춘다. 카드를 한 열로 쌓는다. 표를 카드 목록으로 바꾼다. 조작 요소 높이는 `spacing.control-height-touch`다 |
| 데스크톱 폭 | 뷰포트 폭 1024px 이상 | 상단 막대 전체 항목, 12열 격자, 표를 쓴다. 조작 요소 높이는 `spacing.control-height`다 |

중단점은 하나만 둔다. CSS 미디어 쿼리에는 사용자 정의 속성을 쓸 수 없으므로 값 1024px를 CSS 한 곳에서만 쓴다. 1024px에서 본문 여백을 뺀 카드 안 표 영역은 시안에서 956px이고, 활성 세션 7열 표의 최소 내용 폭은 753px이다([검증](#검증)). 최소 지원 폭은 320px이다.

| 구성 | 데스크톱 폭 | 모바일 폭 |
| --- | --- | --- |
| 상단 막대 | 브랜드 표식과 "VNC Relay", "운영 현황", 언어 선택, [로그아웃] | 브랜드 표식, 언어 선택, [로그아웃]. "운영 현황" 항목은 숨긴다. 페이지 제목이 같은 정보를 준다. 브랜드 이름은 화면에서만 감추고 접근성 트리에는 남긴다. SCR-002의 [목록으로]는 페이지 머리말에 그대로 둔다 |
| 페이지 머리말 | 왼쪽에 제목·갱신 시각, 오른쪽에 [새로고침]을 한 줄에 둔다 | 제목, 갱신 시각, 전체 폭 [새로고침] 순서로 쌓는다 |
| 서버 요약 | 카드 안 값 격자 3열 | 실행 상태와 가동 시간은 각각 한 줄 전체 폭, 나머지 네 값은 2열. 320px에서도 같다. 순서는 데스크톱과 같다 |
| 중계 연결·관리 연결 사용량 | 카드 둘, 각 3열 | 각각 전체 폭 카드. 고리와 값은 가운데 정렬을 유지한다 |
| 대기 연결 표 | 4열 표(역할·중계 ID·접속 시각·대기 시간) | 행마다 첫 줄에 중계 ID와 역할 배지, 아래에 레이블·값 쌍 |
| 활성 세션 표 | 7열 표(세션·모드·전송 경로·시작·방향별 바이트·상대 주소·상세) | 행마다 첫 줄에 세션 식별자 링크와 [상세] 버튼, 아래에 레이블·값 쌍(모드, 전송 경로, 시작, 방향별 바이트 2줄, 상대 주소) |
| 최근 종료 표 | 4열 표(세션·종료 원인·종료 시각·방향별 바이트) | 행마다 첫 줄에 세션 식별자와 종료 원인 배지, 아래에 레이블·값 쌍 |
| 세션 상세 | 세션 정보 카드(레이블·값 두 열 목록)와 전송량 카드를 나란히 둔다. [세션 종료]는 세션 정보 카드 바닥 오른쪽 | 두 카드를 쌓는다. 레이블·값 한 열 목록. [세션 종료]는 전체 폭 |
| 종료 확인 대화상자 | 최대 폭 `spacing.dialog-max`, 가운데. 버튼은 가로로 [취소][종료 확인] | 뷰포트 폭에서 가로 여백 `spacing.md`씩 뺀 폭. 버튼은 세로로 쌓고 전체 폭이며 [취소]가 위에 온다 |
| 로그인 | 최대 폭 `spacing.login-max` 카드, 가운데. 입력과 [로그인] 버튼은 전체 폭 | 전체 폭 카드. 같음 |

표를 카드로 바꾸는 규칙:

- 모바일 폭에서도 정보를 숨기거나 말줄임하지 않는다. 카드 첫 줄의 값(세션 식별자·중계 ID·역할·종료 원인·[상세])은 레이블 없이 제목처럼 보인다. 나머지 값은 같은 열 머리글 이름을 앞에 레이블로 보인다. 레이블은 `typography.label`과 `colors.muted-foreground`다.
- 가로 스크롤 표를 쓰지 않는다. 320px 폭에서 페이지에 가로 스크롤이 생기면 안 된다(WCAG 1.4.10).
- 표의 `display`를 바꾸면 일부 브라우저가 표 의미를 잃는다. 카드 배치에서도 `table`·`row`·`cell`·`columnheader` 역할을 명시해 보조 기술이 표로 읽게 한다. 열 머리글은 화면에서만 감추고 접근성 트리에는 남긴다.
- 레이블을 CSS 가상 요소로 만들면 보조 기술이 레이블을 두 번 읽을 수 있다. 가상 요소의 대체 텍스트를 비운다.
- 카드 순서는 표의 행 순서와 같다. 정렬은 [관리 API 계약 1.1절](../contracts/management-api-v1.md)을 따르며 화면에서 바꾸지 않는다.

### 접근성 기준

적용 기준은 WCAG 2.2 AA다. 지원 브라우저는 최신 Chrome·Edge·Firefox·Safari이다([PRD](../requirements/PRD.md#확정-기준)). 동작 요구는 [화면 스펙 6절](../specs/dashboard.md#6-화면-크기와-접근성)이 정한다.

| 항목 | 검증 가능한 기준 | 근거 |
| --- | --- | --- |
| 글씨 대비 | 본문·상태 글씨 4.5:1 이상 | WCAG 1.4.3. [대비 기준](#대비-기준) 표 |
| 조작 요소 대비 | 입력 경계·포커스 링·사용량 고리 채움 3:1 이상. 글씨 레이블이 있는 버튼의 경계는 요구하지 않는다 | WCAG 1.4.11 |
| 포커스 표시 | 모든 조작 요소가 `:focus-visible`에서 `spacing.focus-ring-width` 두께의 링과 `spacing.focus-ring-offset` 간격을 가진다. 상단 막대 밖은 `colors.ring`, 상단 막대 안은 `colors.navbar-ring`이다. `outline: none`을 대체 표시 없이 쓰지 않는다 | WCAG 2.4.7 |
| 포커스 가림 | 본문 위에 겹치는 고정(sticky·fixed) 머리글·바닥글을 두지 않는다. 상단 막대는 페이지와 함께 스크롤되어 본문과 겹치지 않는다. 대화상자는 포커스 요소를 가리지 않는다 | WCAG 2.4.11 |
| 대상 크기 | 데스크톱 폭에서 조작 요소는 가로·세로 `spacing.target-min` 이상, 높이는 `spacing.control-height`(표 안 [상세]는 `spacing.control-height-sm`). 모바일 폭에서 높이 `spacing.control-height-touch` | WCAG 2.5.8. 모바일 44px는 WCAG 최소(24px)보다 큰 이 프로젝트의 기준 |
| 색 외 표시 | 상태는 글씨 레이블과 도형으로 구분한다. 링크는 밑줄을 쓴다. 사용량은 고리와 함께 현재·상한 값을 글씨로 쓴다. 상단 막대 활성 항목은 바탕·표시 막대와 `aria-current="page"`를 함께 쓴다 | WCAG 1.4.1 |
| 사용량 고리 | 고리 묶음에 `role="progressbar"`와 현재·최소·최대 값, 접근 가능한 이름을 둔다. SVG는 `aria-hidden="true"`다 | WAI-ARIA `progressbar` 역할 |
| 랜드마크 | 상단 막대는 `nav`(이름 "주 메뉴"), 본문은 `main`이다. 시안은 한 파일에 프레임이 여럿이므로 본문을 `main` 대신 일반 요소로 둔다 | WCAG 1.3.1 |
| 언어 | 문서 루트 `lang`을 고른 언어로 둔다. 언어 선택의 각 항목은 자기 언어의 `lang`을 가진다. 언어 선택의 접근 가능한 이름은 두 언어를 함께 쓴다("언어 (Language)", "Language (언어)"). 읽지 못하는 언어가 선택된 사용자도 찾을 수 있게 하기 위해서다 | WCAG 3.1.1·3.1.2 |
| 재배치 | 320px 폭에서 가로 스크롤 없이 모든 정보와 행동을 쓴다 | WCAG 1.4.10 |
| 글씨 확대 | 브라우저 확대를 200%로 키워도 정보와 행동이 가려지지 않는다. 데스크톱 폭 1280px의 200% 확대는 640px 폭과 같으므로 모바일 폭 배치가 적용된다 | WCAG 1.4.4 |
| 입력 레이블 | 모든 입력에 보이는 레이블을 둔다. 자리표시자만으로 대신하지 않는다 | WCAG 3.3.2 |

## Elevation & Depth

shadcn/ui처럼 경계선과 얕은 그림자로 층을 나눈다.

| 대상 | 토큰·스타일 기준 | 사용 조건 |
| --- | --- | --- |
| 페이지 | `colors.background` | 상단 막대 아래 가장 아래 층 |
| 상단 막대 | `colors.navbar`. 경계·그림자 없음 | 화면 맨 위, 모든 폭 |
| 카드 | `colors.card`, `spacing.border-width` `colors.border` 경계, 그림자 `0 spacing.shadow-y spacing.shadow-blur colors.shadow` | 요약·사용량·목록·세션 정보·전송량·로그인 카드. 카드 안 구분은 `colors.border` 선이다 |
| 안내(Alert) | `<상태>-bg` 바탕, `spacing.border-width` `<상태>-border` 경계. 그림자 없음 | 상태 안내 |
| 대화상자 | 카드와 같은 경계, 그림자는 카드 그림자를 네 배 키운 값(`spacing.shadow-y`·`spacing.shadow-blur`의 4배). 뒤에 `colors.overlay` | 종료 확인 대화상자 |

## Shapes

| 대상 | 토큰 | 적용 조건 |
| --- | --- | --- |
| 버튼·입력 | `rounded.md` | 모든 크기 |
| 안내 | `rounded.lg` | 상태 안내 |
| 상단 막대 항목·언어 선택·브랜드 표식 | `rounded.md` | 상단 막대 안 항목 바탕, 언어 선택, 브랜드 표식 |
| 카드·대화상자 | `rounded.xl` | 모든 카드와 대화상자 |
| 배지 | `rounded.sm` | 상태 배지, 모드·전송 경로·역할 배지, 개수 배지 |
| 방향 표식 | `rounded.mark` | 방향별 바이트 앞의 사각형 |
| 사용량 고리 | 둥근 끝(`stroke-linecap: round`) | 채움 호의 양 끝 |

테두리 두께는 `spacing.border-width`, 포커스 링 두께는 `spacing.focus-ring-width`, 상단 막대 활성 표시 막대 두께는 `spacing.indicator-width`다.

### 아이콘 규칙

직접 그린 인라인 SVG 선 아이콘만 쓴다. 외부 아이콘 라이브러리, CDN, 아이콘 글꼴, 유니코드 문자 기호(◆ ▲ ● ■ ⓘ ⚠ 등)를 쓰지 않는다. 문자 기호는 플랫폼과 글꼴에 따라 정렬, 굵기, 이모지 치환이 달라져 일관된 시각 위계를 해치고 보조 기술 처리가 모호하기 때문이다.

- 규격: 선 색은 현재 글씨 색(`stroke="currentColor"`), 선 두께 `spacing.icon-stroke`, 둥근 끝과 꺾임(`stroke-linecap="round" stroke-linejoin="round"`), 보조 기술 숨김(`aria-hidden="true"`, `focusable="false"`). 그리는 좌표계는 `viewBox="0 0 16 16"`이다.
- 크기: 버튼·상단 막대 항목·언어 선택은 `spacing.icon-size`(16px), 배지·안내·언어 선택의 펼침 표시는 `spacing.icon-size-sm`(14px)이다.
- 사용 조건: 아이콘만으로 뜻을 전하지 않으며 글씨 레이블이 항상 함께 있다.
- 정의한 아이콘:
  - 상태: 성공(원 안 체크), 정보(원 안 시계 바늘), 경고(삼각형 안 느낌표), 위험(원 안 느낌표).
  - 행동: 새로고침(원호 화살표), 로그아웃(문과 오른쪽 화살표), 목록으로(왼쪽 화살표), 상세(오른쪽 꺾쇠), 세션 종료(원 안 가로줄), 언어 선택(원 안 경선·적도의 지구), 펼침(아래 꺾쇠).
  - 내비게이션: 운영 현황(네 칸 격자).
  - 브랜드 표식: 둥근 사각형 안의 서로 반대 방향 화살표 두 개. 중계를 뜻한다. 참고 이미지의 로고를 복제하지 않는다.
- 기호 없는 중립 배지: 중립 상태 배지와 역할·모드·전송 경로 배지(Viewer, Server, Mode I, Mode II, TCP, WSS, 확인 불가 등), 개수 배지는 아이콘 없이 글씨만 쓴다. 정상 운영 데이터를 장식 기호로 어지럽히지 않기 위함이다. 중립 안내(Alert)에는 원 안 느낌표를 `neutral` 색으로 쓴다.

## Components

첫 화면에 필요한 컴포넌트만 정한다. 실제 컴포넌트와 코드가 없어 구현·API·실행 예시는 정할 수 없다. 동작은 각 행의 스펙 시나리오를 따른다. 형태는 shadcn/ui의 같은 이름 컴포넌트를 따른다.

| 컴포넌트 | 변형 | 시각 상태 | 크기와 구성 | 관련 동작 |
| --- | --- | --- | --- | --- |
| Button | `button-primary`(로그인), `button-outline`(새로고침·상세·상태 재조회·취소·안내 안 버튼), `button-ghost`(목록으로), `button-destructive`(세션 종료·종료 확인), 상단 막대 항목형(로그아웃) | 기본·hover(`*-hover` 토큰)·focus-visible·disabled(투명도를 낮추지 않고 `colors.muted-foreground` 글씨와 `colors.muted` 바탕, 경계 `colors.border`)·진행 중(레이블을 진행 중 문구로 바꾸고 disabled). 주요·위험 버튼은 `typography.button-strong`, 나머지는 `typography.button` | 높이 `spacing.control-height`(표 안 [상세]는 `spacing.control-height-sm`, 모바일 `spacing.control-height-touch`), 가로 안쪽 `spacing.lg`, 아이콘과 글씨 사이 `spacing.sm`. outline은 `colors.border` 경계와 `colors.card` 바탕이다. 한 영역의 주요 버튼은 하나다 | DASH-003·DASH-009~013·DASH-017~019 |
| Input | 텍스트·비밀번호 | 기본·focus-visible·오류 | 높이 `spacing.control-height`(모바일 `spacing.control-height-touch`), `typography.input`, `colors.input` 경계. 오류일 때 경계를 `colors.destructive`로 바꾸고 오류 문장을 입력 아래에 쓴다 | DASH-018 |
| Badge | `badge-success`·`badge-info`·`badge-warning`·`badge-destructive`·`badge-neutral` | 정적 | `typography.badge`, `rounded.sm`, 가로 안쪽 `spacing.sm`·세로 `spacing.xs`. 상태 배지는 아이콘 + 글씨, 중립·개수 배지는 글씨만 | DASH-001·DASH-003·DASH-020 |
| Alert | `alert-info`·`alert-warning`·`alert-destructive`·`alert-neutral` | 정적 | `<상태>-bg` 바탕, `<상태>-border` 경계, `rounded.lg`, 안쪽 `spacing.md`(세로)·`spacing.lg`(가로). 왼쪽 아이콘(`<상태>` 색), 제목 `typography.body-strong`(`<상태>` 색), 설명 `typography.body`(`foreground`), 오른쪽에 outline 버튼 | DASH-003·DASH-007·DASH-008·DASH-012·DASH-013 |
| Card | 없음 | 정적 | [Elevation & Depth](#elevation--depth)의 카드. 제목 줄은 `typography.heading-card`와 개수 배지, 아래 `colors.border` 구분선. 설명이 있으면 제목 아래 `typography.label`·`colors.muted-foreground`. 바닥 행동 줄은 위 `colors.border` 구분선 | DASH-002 |
| Stat | 서버 요약 값 | 정적 | 레이블 `typography.label`+`colors.muted-foreground`, 값 `typography.metric`. 실행 상태는 배지로 쓴다. 확인 불가는 중립 배지, 조회 중은 정보 배지로 쓴다([상태 색](#상태-색)) | DASH-001·DASH-021 |
| Gauge | 없음(중계 연결·관리 연결 공통) | 정적 | 지름 `spacing.gauge-size`, 선 두께 `spacing.gauge-stroke`의 고리. 트랙 `colors.gauge-track`, 채움 호 `colors.gauge`, 12시 방향에서 시계 방향으로 채운다. 채움 비율은 현재/상한이며 100%를 넘지 않는다. 현재 값이 0이면 채움을 그리지 않는다. 가운데 현재 값(`typography.metric`)과 "상한 n"(`typography.label`, `colors.muted-foreground`). 임계값에 따른 색 변화는 없다(스펙에 임계값이 없다). 값을 확인하지 못하면 고리 없이 "확인 불가" 중립 배지를 쓴다. 첫 조회 전이면 "조회 중" 정보 배지를 쓴다. 접근 가능한 이름은 카드 제목과 "사용량"이다 | DASH-021 |
| Navbar | 상단 막대(모든 폭) | 항목: 기본·hover(`navbar-accent` 바탕)·활성(`navbar-accent` 바탕, 아래 `navbar-primary` 표시 막대, `navbar-foreground` 글씨)·focus-visible(`navbar-ring`) | 높이 `spacing.appbar-height`. 항목은 아이콘 + 글씨 한 줄, 높이 `spacing.control-height`, 가로 안쪽 `spacing.md`, `rounded.md`. 활성 표시 막대는 두께 `spacing.indicator-width`이고 상단 막대 아래 끝에 붙는다. 브랜드 표식은 `spacing.brand-size` 정사각형, `navbar-accent` 바탕에 `navbar-brand`(순백색) 선 | DASH-019 |
| Language select | 상단 막대형(`lang-select`, 어두운 바탕)·로그인형(밝은 바탕) | 기본·hover·focus-visible | 브라우저 기본 `<select>`. 왼쪽에 지구 아이콘, 오른쪽에 펼침 아이콘. 상단 막대형은 `navbar` 바탕·`navbar-foreground` 글씨·`navbar-muted-foreground` 경계, 로그인형은 `card` 바탕·`foreground` 글씨·`input` 경계. 높이는 버튼과 같다. 선택한 언어 이름을 보인다 | DASH-024·DASH-025 |
| Data table | 표(데스크톱 폭)·카드 목록(모바일 폭) | 행 hover(`colors.muted`), 링크 hover·focus-visible | 머리글 `typography.label`·`colors.muted-foreground`, 바탕 없음, 아래에 `colors.border` 선. 행은 `colors.border` 선으로 나눈다. 숫자 열은 오른쪽 정렬한다. 방향별 바이트는 줄마다 방향 표식 + 흐린 방향 이름 + 값이다 | DASH-001·DASH-015 |
| Link | 세션 식별자 | 기본(밑줄)·hover·focus-visible | `colors.link`, 밑줄, `typography.data` | DASH-006 |
| Dialog | 종료 확인 | 열림·닫힘 | `<dialog>` 모달 기준. 제목 `typography.heading-card`, 설명 `typography.body`, 바닥에 오른쪽 정렬 버튼. 폭은 위 [배치](#중단점과-모바일-배치)를 따른다 | DASH-009 |

### 모션

모션을 쓰지 않는다. 전환 효과·스피너 애니메이션·스켈레톤 깜박임이 없다. 진행 중 상태는 글씨와 도형으로 표시한다. 따라서 움직임 감소 설정(`prefers-reduced-motion`)에 따른 별도 표현은 필요 없다. 모션을 추가하려면 이 절을 먼저 갱신한다.

## Do's and Don'ts

### 허용·금지 조합

| 대상 | 허용 예시 | 금지 예시 | 판정 이유 |
| --- | --- | --- | --- |
| 상태 표시 | 글씨 "갱신 중" + 정보 도형 + `info` 색 | 색만 다른 점 | 색을 구분하지 못하는 사용자가 상태를 알 수 없다 |
| 링크 | 밑줄 + `link` 색 | 색만 다른 글씨 | 색 외 표시가 없다 |
| 상단 막대 항목 | 아이콘 + 글씨 레이블 | 아이콘만 있는 항목, 툴팁으로만 뜻을 알리기 | 아이콘만으로 뜻을 전하지 않는다 |
| 모바일 표 | 카드 목록으로 바꾸고 모든 값을 보인다 | 가로 스크롤 표, 열 숨김, 말줄임 | 필수 정보가 가려진다([화면 스펙 6절](../specs/dashboard.md#6-화면-크기와-접근성)) |
| 확인하지 못한 값 | "확인 불가" 배지 | 0, 빈 칸, `-`, 빈 고리 | 0과 구분할 수 없다([화면 스펙 3절](../specs/dashboard.md#3-화면-구성과-와이어프레임)) |
| 포커스 | 토큰의 링 | `outline: none`만 적용 | 키보드 위치를 알 수 없다 |
| 위험 행동 | `button-destructive`는 세션 종료·종료 확인에만 쓴다 | 목록 행마다 빨간 종료 버튼, 주요 행동을 빨간색으로 표시 | 종료는 SCR-002의 확인 절차로만 시작한다([화면 스펙 2절](../specs/dashboard.md#2-화면-흐름)) |
| 차트 색 | 방향 표식, 사용량 고리 | 글씨 색, 상태 의미를 혼자 전하는 색, 스펙에 없는 그래프 | 흰 바탕에서 글씨 대비 4.5:1을 못 넘는다. 그래프는 새 데이터가 필요하다 |
| 사용량 고리 | 현재·상한 값을 글씨로 함께 쓴다 | 값 없이 고리만 쓰기, 비율에 따라 색 바꾸기 | 스펙에 임계값 규칙이 없다. 값을 확인하려면 글씨가 필요하다 |
| 카드 | `border` 경계와 얕은 그림자 | 굵은 테두리, 진한 그림자, 표 행 줄무늬 | shadcn/ui의 평평한 카드 형태다 |
| 글꼴·자산 | 실행파일에 넣은 Pretendard woff2와 시스템 대체 글꼴, 인라인 SVG | 웹 글꼴 서비스, CDN, 아이콘 글꼴, shadcn/ui·Tailwind 패키지 | 정적 파일만 쓴다 |
| 색 값 | `var(--color-…)` | 화면 CSS에 직접 쓴 `#rrggbb` | 토큰 원본이 둘이 된다 |
| 참고 이미지 | 카드 격자·고리·범례 표식 같은 시각 방향 | 검색·알림·위젯 편집·시계열 차트, 이미지 자산 복제 | 스펙에 없는 화면 요소는 새 동작을 만든다 |
| 아이콘 기호 | 직접 그린 인라인 SVG 선 아이콘과 글씨 레이블 함께 쓰기 | 유니코드 문자 기호(◆ ▲ ● ■ ⓘ ⚠), 외부 아이콘 글꼴, CDN | 문자 기호는 플랫폼·글꼴별 정렬·굵기·이모지 치환 차이가 크다. 외부 글꼴은 정적 파일 원칙을 위반한다 |
| 글꼴 굵기 | 400(Regular)과 700(Bold)만 쓴다 | 500 또는 600 굵기 지정, `bolder` 임시 방편 | 글꼴 파일이 두 굵기뿐이다. 대체 글꼴인 맑은 고딕에서 500·600이 400으로 렌더링되어 위계가 사라진다 |

### 예외

현재 예외 없음. 화면별 예외는 `DESIGN-EX-001`부터 이 표에 범위·이유·재검토 조건과 함께 추가한다.

### 검증

- 실행 절차: [대시보드 프런트엔드를 검증하는 방법](../how-to/verify-dashboard-frontend.md)(Draft)이 정한다. 사용자 결정 2026-10-22(Q09)로 작업 0.9를 시작했고, 사용자 결정 2026-10-22(Q32~Q38)로 DESIGN-TBD-005를 해소했다. 아래 기록은 채택(2026-10-20) 전에 한 일회성 확인이다.
- 통과 조건: lint 오류 0건. 대비는 [대비 기준](#대비-기준) 표의 기준 이상이다.
- 생성 산출물 대조: `cargo xtask tokens --check`가 이 문서의 YAML에서 만든 CSS를 저장된 CSS 파일과 시안 토큰 블록에 대조한다. 절차와 방법은 [프런트엔드 컨벤션 2절](../conventions/frontend.md#2-토큰-적용과-재사용)이 정한다(사용자 결정 2026-10-22(Q32)). 작업 4.8에서 구현했다.
- 시각 검토 범위: 대표 화면 SCR-001·SCR-002·SCR-003, 데스크톱·모바일 폭.
- 실행 결과와 증거: 아래 표.

문서 검사만으로 실제 화면의 접근성과 시각 품질이 검증되지는 않는다. 아래 시안은 실제 컴포넌트가 아니므로 구현 검증의 증거가 아니다.

| 확인 대상 | 기준 문서·범위 | 환경·명령·방법 | 실제 결과 | 증거 |
| --- | --- | --- | --- | --- |
| 문서·토큰 구조 | 이 문서 YAML | 2026-10-20. Windows, `npx -y -p @google/design.md designmd lint docs/design/DESIGN.md` | 통과: 오류 0건, 경고 18건, 정보 1건. 경고 17건은 `orphaned-tokens`다. 컴포넌트 토큰이 직접 참조하지 않고 CSS가 직접 참조하는 색이므로 허용한다(`muted-foreground`·`border`·`input`·`ring`·`link`·`*-border` 5개·`navbar-primary`·`navbar-ring`·`navbar-brand`·`chart-1`·`chart-2`·`overlay`·`shadow`). 경고 1건은 `components.gauge`의 `contrast-ratio`(3.24:1)다. 고리 채움은 글씨가 아닌 그래픽이라 3:1 기준을 적용하므로 허용한다. 토큰 수는 색 42·글꼴 단계 11·모서리 5·간격 31·컴포넌트 25 | 로컬 실행 |
| 대비(YAML 계산) | YAML 색 토큰 | 2026-10-20. Python. WCAG 상대 휘도 공식으로 [대비 기준](#대비-기준) 표의 쌍과 상태 색 5개의 `card`·`background` 대비, 기준이 없는 4쌍을 계산 | 통과: 기준 미달 0건. 4.5:1 기준 최저는 5.34(`muted-foreground`/`muted`), 3:1 기준 최저는 3.24(`gauge`/`gauge-track`)와 3.31(`input`/`background`). 상태 색 5개는 `card`·`background` 위에서 최저 6.04 | 로컬 실행 |
| 대비·글꼴(렌더링 측정) | [dashboard.html](mockups/dashboard.html)의 모든 글씨와 아이콘(23개 프레임, 대화상자 뒤 화면 제외, 비활성 버튼 포함, 화면에서만 감춘 글씨 제외) | 2026-10-20. 관리형 Chromium 헤드리스, 폭 1280·1024·1023·768·390·320px. 렌더링된 요소의 색·겹친 배경·굵기·글꼴을 스크립트로 측정 | 통과: 글씨 요소 1,114개(1024px 이상)·1,097개(1023px 이하), 4.5:1 미달 0건. 최저 5.34(비활성 버튼 "갱신 중"). 인라인 SVG 아이콘 209개 최저 5.34. 언어 선택 경계 최저 3.31(로그인 화면). 계산된 굵기는 400과 700만 있다. 렌더링 글꼴은 한글 우선 스택(`Apple SD Gothic Neo` 첫 항목)과 고정 폭(`ui-monospace`) | 로컬 실행 |
| 대표 화면 시안 | [화면 스펙 4절](../specs/dashboard.md#4-화면-상태와-갱신)의 SCR-001·SCR-002·SCR-003 화면 상태 전체와 [화면 언어](../specs/dashboard.md#화면-언어)의 영어 예시. 단일 파일에 23개 프레임(SCR-001 8개, SCR-002 10개, SCR-003 5개. 영어 프레임 2개 포함) | 2026-10-20. Windows, 관리형 Chromium 헤드리스, 시스템 글꼴. 로컬 HTTP 서버로 파일을 열었다. 폭 1280·1024·1023·768·390·320px. 영어 프레임의 한글은 정규식으로 찾았다 | 통과(시안 한정): 6개 폭에서 페이지 가로 스크롤 0건, 프레임 밖으로 나온 요소 0건, 카드·표 칸·상단 막대 안 넘침 0건(스크립트 측정). 요소 ID 중복 0건, 깨진 내부 링크 0건. 상단 막대 높이 56px. 상단 막대 항목·언어 선택·머리말 버튼 높이는 1024px 이상 36px, 1023px 이하 44px. 활성 표시 막대 아래 끝이 상단 막대 아래 끝과 같다. 1024px에서 활성 세션 표 영역 956px, 표 최소 내용 폭 753px. 영어 프레임 2개의 글씨에 한글 0건. 예외는 `lang="ko"` 항목 "한국어"와 언어 선택 접근 이름 "Language (언어)"다. 네트워크 요청은 시안 HTML뿐이다 | 로컬 실행 |
| 시안 접근성·아이콘 | 포커스 링, 카드 배치의 표 역할, 인라인 SVG 속성 | 2026-10-20. 관리형 Chromium. 1280px에서 세션 링크·상단 막대 언어 선택에 포커스를 주고 윤곽선 측정. 390px에서 활성 세션 목록의 접근성 트리 확인(표 마크업은 이번 변경에서 바뀌지 않았다) | 통과: 세션 링크는 2px 실선, 간격 2px, `ring` 색. 상단 막대 언어 선택은 2px 실선, `navbar-ring` 색. 390px 접근성 트리에 `table`·`rowgroup`·`row`·`columnheader`·`cell` 유지. SVG 전부 `aria-hidden="true"`·`focusable="false"` 보유 | 로컬 실행 |
| 시안 CSS 점검 | 시안 `<style>` 안 토큰 블록 밖 | 2026-10-20. 정규식 검색 | 통과: `#rrggbb`·`rgb()` 0건, `em`·`rem` 0건, 길이 리터럴은 미디어 쿼리 `1023.98px` 한 곳, `style` 속성·`url(`·`@import`·외부 링크·스크립트 0건, 정의되지 않은 `var()` 0건. `font-weight`는 모두 토큰 변수 또는 `inherit`다. 토큰 블록에 정의만 있고 CSS가 쓰지 않는 값은 `success-border`와 글꼴 단계 속성 2개다(성공 안내가 없는 화면이다). 토큰 블록은 이 문서 YAML에서 다시 만든 값과 같다 | 로컬 실행 |
| 토큰 블록 생성 | 시안 CSS 토큰 블록 | 2026-10-20. 임시 스크립트(저장소 밖)가 이 문서 YAML에서 [이름 규칙](#css-사용자-정의-속성-이름-규칙)대로 CSS를 만들고 시안 전체를 생성했다 | 통과(일회성). 저장소의 생성 명령은 아직 없다. `cargo xtask tokens`로 정했다(사용자 결정 2026-10-22(Q32)) | 로컬 실행 |
| 스크린샷 육안 검토 | 대표 화면 상태 | 2026-10-20. SCR-001 정상(1280px), 최초 로딩(1280px), 조회 실패(1024px), SCR-001 요약·활성 세션 카드(390px), SCR-002 종료 확인(1280px)·정상(390px)·종료 결과 불명(1024px), SCR-003 로그인 실패(390px)를 직접 확인했다. 상단 막대로 바꾼 뒤 상단 막대(1280px·320px), SCR-001 정상 영어(1280px), 중계 연결 카드(1280px), SCR-002 종료 확인(1280px), SCR-001 권한 없음(390px), SCR-003 로그인 실패(390px)·입력 영어(320px)를 다시 확인했다. 브랜드 표식 선을 `navbar-brand`로 바꾼 뒤 SCR-001 정상 브랜드(1280px)를 확인하고 모든 브랜드 표식과 활성 표시 막대의 계산된 색을 측정했다 | 통과(검토한 화면 한정). 짙은 상단 막대와 활성 항목 아래 표시 막대, 언어 선택, 바탕 위 페이지 머리말, 경계와 얕은 그림자의 흰 카드, 카드 제목 줄 구분선과 개수 배지, 연결 카드 설명과 사용량 고리, 청록·녹색 방향 표식, 모바일 카드 목록의 레이블·값 정렬, 영어 문구 길이에서 줄바꿈 확인. 브랜드 표식 선은 상단 막대·로그인 화면 모두 `rgb(255, 255, 255)`, 브랜드 이름 글씨는 `rgb(238, 243, 240)`, 활성 표시 막대는 `rgb(95, 211, 154)`다 | 스크린샷 검토. 실제 컴포넌트가 아니므로 구현 검증의 증거가 아니다 |
| 스크린 리더·Firefox·Safari·Edge·실제 모바일 기기 | 접근성·화면 호환성 | 2026-10-20 시안 단계에서는 실제 UI가 없어 시안을 Chromium 헤드리스로만 열었다. 구현 뒤 Firefox·Edge는 수동 확인했고(검증 4.14·4.17절), Safari는 Playwright WebKit 자동 확인으로 대신했다(사용자 결정 2026-11-07(Q135), 검증 4.21절). 작업 4.12 완료 판정(검증 4.26절, 2026-11-07)에서 Chrome 157.0.8103.38·Edge 156.0.4314.58·Firefox 159.0·WebKit 27.2로 실서버 흐름과 실제 키 입력을 확인했다 | Chrome·Edge·Firefox·WebKit 대체 확인 통과(작업 4.12 범위). 버전은 최신 안정판보다 낮다(사용자 결정 2026-11-07(Q146)). 스크린 리더와 실제 모바일 기기는 미실행이며 작업 4.12 완료 조건에서 뺐다(사용자 결정 2026-11-07(Q136·Q137)) | [검증 how-to 4절](../how-to/verify-dashboard-frontend.md#4-검증) |
| 추가 시안 3개. 사용자 결정 2026-10-22(Q52) | [dashboard.html](mockups/dashboard.html)에 SCR-002 잘못된 세션 식별자(CPY-067), SCR-003 로그인 요청 실패(CPY-077), SCR-003 로그아웃 결과 확인 필요(CPY-076) 프레임을 더했다. 프레임 수는 26개(SCR-001 8개, SCR-002 11개, SCR-003 7개. 영어 프레임 2개 포함)다. 새 CSS와 인라인 스타일은 없다 | 2026-10-22. Windows, 관리형 Chromium 헤드리스, 저장소 루트를 로컬 HTTP 서버(127.0.0.1)로 열었다. 폭 1280·390·320px. 새 프레임 3개의 가로 넘침과 프레임 밖 요소를 스크립트로 측정했다. 콘솔 오류·경고와 실패한 요청을 기록했다. 파일 전체의 요소 ID 중복과 내부 `#` 링크를 스크립트로 확인했다. 토큰 블록 검사는 `cargo xtask tokens --check`로 했다. 삽입은 스크립트로 했다. 구현 판단(사용자 결정 없음) | 통과(새 프레임 3개 한정): 세 폭 모두 페이지 가로 스크롤 0px, 프레임 안 가로 넘침 0px, 프레임 밖 요소 0건. 콘솔 오류·경고 0건, 실패한 요청과 4xx·5xx 응답 0건. 파일 전체 기준 요소 ID 중복 0건, 내부 `#` 링크 81건 모두 대상 있음. 새 프레임 3개는 색인에서 각 한 번 연결된다. 토큰 검사 통과. 미실행: 새 프레임의 대비·글꼴 렌더링 측정(2026-10-20 행의 방법)과 육안 검토. 스크린샷 9장은 `target/mockup-shots/`에 저장했고 아직 검토하지 않았다 | 로컬 실행. 스크린샷 파일 9장
| 추가 시안 3개의 나머지 폭. 사용자 결정 2026-10-22(Q52) | 위 행의 새 프레임 3개(`scr002-bad-id`, `scr003-request-failed`, `scr003-logout-unknown`) | 2026-10-22. Windows, 관리형 Chromium 헤드리스, 로컬 HTTP 서버(127.0.0.1). 폭 1024·1023·768px. 페이지 가로 넘침, 프레임 안에서 `overflow-x`가 `visible`이 아닌 요소의 넘침, 페이지 오류를 스크립트로 측정했다. 에이전트가 1280px `scr002-bad-id`, 390px `scr003-request-failed`, 768px `scr002-bad-id` 스크린샷을 기존 SCR-002 대상 없음 프레임과 비교해 보았다 | 통과: 세 폭 모두 페이지 가로 스크롤 0px, 페이지 오류 0건. 프레임 안 넘침은 1023·768px에서 `span.brand-name` 하나(69px)만 나왔다. 이 요소는 좁은 폭에서 화면에서만 감춘 브랜드 이름이며, 기존 `scr002-gone`·`scr002-normal` 프레임도 같은 값을 낸다. 실제 넘침이 아니다. 육안 비교에서 배치·문구 차이를 찾지 못했다. 검토자는 에이전트이며 사용자 검토가 아니다. 미실행: 대비·글꼴 렌더링 측정. 새 프레임은 기존 안내 클래스(`a-destructive`·`a-warning`)를 그대로 쓴다 | 로컬 실행. 스크린샷 9장 추가(`target/mockup-shots/<프레임>-<폭>.png`, 모두 18장) |
| Q115 목록 개수 배지 | SCR-001의 세 목록 제목, CPY-084·085 | 2026-11-05. 실제 Windows 서버·관리형 브라우저, 1280·320px, ko·en. 실제 Mode II 연결과 EOF로 개수를 바꾸고 캡처·DOM을 확인했다. 영어 시안의 배지 세 곳도 맞췄다. `177040d`의 두 OS 전체 게이트 실행 | 통과(확인 범위 한정). 0·1·여러 항목과 언어 변경이 일치했다. 아이콘 없는 중립 배지를 재사용했다. 가로 넘침 없음. 320px의 긴 영어 제목은 배지가 다음 줄로 내려가며 겹치지 않았다. 새 토큰·CSS 없음. 수동 접근성·Safari·실기기 검증은 아님 | [검증 4.20절](../how-to/verify-dashboard-frontend.md#420-목록-개수-배지-q115-2026-11-05) |
| Pretendard 글꼴·자간·행간. 사용자 지시 2026-11-07, 사용자 결정 2026-11-07(Q139~Q142) | [Typography](#typography)의 모든 단계, 미리보기·로그인·시안, 비교 페이지 26개 프레임 × 6개 폭 | 2026-11-07. Windows, Chrome 157.0.8103.38(Playwright 1.64.1). `cargo xtask tokens --check`, `document.fonts`·`getComputedStyle` 측정(1280·320px), 두 OS 전체 게이트, 비교 페이지 재촬영 | 통과(확인 범위 한정). 400·700 글꼴이 세 페이지와 비교 페이지 156쌍의 두 쪽에서 모두 로드됐다. 계산된 크기·행간·자간이 위 표와 같다. 값 글꼴은 12px·자간 0이다. 가로 넘침 0. 사람의 시안 비교, Firefox·Edge·WebKit 재확인, 대비 재측정은 미실행이다. 색 토큰은 바뀌지 않았다 | [검증 4.23절](../how-to/verify-dashboard-frontend.md#423-pretendard-글꼴과-자간행간-q139q142-2026-11-07) |
| 조회 중 표시 통일. 사용자 지적 2026-11-07 | [상태 색](#상태-색)의 조회 중 규칙. SCR-001 카드 머리·요약 값·고리·목록 빈 행 | 2026-11-07. Windows, Chrome 157.0.8103.38(Playwright 1.64.1). 브라우저 시험(DASH-005 사례의 조회 중 표시 확인), 비교 페이지 재촬영, 두 OS 전체 게이트 | 통과(확인 범위 한정). 네 자리 모두 시계 아이콘이 있는 같은 정보 배지다. "확인 불가"는 중립 배지 그대로다. 가로 넘침 0. 사람의 재비교와 다른 브라우저 확인은 미실행이다. 이때 남은 SCR-002 첫 조회 중 화면의 차이는 다음 줄(Q143)에서 처리했다 | [검증 4.24절](../how-to/verify-dashboard-frontend.md#424-조회-중-표시-통일-작업-412-사용자-지적-2026-11-07) |
| SCR-002 첫 조회 중 화면. 사용자 결정 2026-11-07(Q143) | [상태 색](#상태-색)의 조회 중 규칙. 세션 정보 사실 다섯 줄, 전송량 두 방향 | 2026-11-07. Windows, Chrome 157.0.8103.38(Playwright 1.64.1). 브라우저 시험(DASH-022 느린 읽기 사례), 비교 페이지 재촬영, 두 OS 전체 게이트 | 통과(확인 범위 한정). 항목 이름과 방향 표식을 보이고 값 자리 7곳이 카드 머리와 같은 정보 배지다. 가로 넘침 0. 시안의 전체 폭 안내는 CPY 표에 없어 넣지 않았다. 사람의 재비교와 다른 브라우저 확인은 미실행이다 | [검증 4.25절](../how-to/verify-dashboard-frontend.md#425-scr-002-첫-조회-중-화면-작업-412-q143-2026-11-07) |

### 미결정 사항

현재 이 문서에 열린 미결정 사항은 없다. DESIGN-TBD-005(프런트엔드 컨벤션과 검증 방법)는 사용자 결정 2026-10-22(Q32~Q38)로 해소했다. 결정 내용은 [프런트엔드 컨벤션 8절](../conventions/frontend.md#8-미결정-사항)에 있다. 프런트엔드 성능 지표(FE-TBD-016)는 사용자 결정 2026-11-07(Q159)로 지표 없이 닫았다.

DESIGN-TBD-002(다크 테마)는 사용자 결정(2026-10-20)으로 라이트 테마만 두기로 해 [테마](#테마)에 반영했다. DESIGN-TBD-001(시각 방향), DESIGN-TBD-003(중단점과 모바일 배치), DESIGN-TBD-004(표시 형식)는 사용자가 이 문서를 채택(2026-10-20)해 결정했다. 이 ID들은 재사용하지 않는다.

DESIGN-TBD-005도 사용자 결정 2026-10-22(Q32~Q38)로 해소했으며, 이 ID도 재사용하지 않는다.
