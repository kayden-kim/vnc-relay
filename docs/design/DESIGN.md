---
version: alpha
name: "VNC Relay 운영 대시보드"
description: "참고 이미지(밝은 회녹색 바탕 위 흰 카드 격자, 청록·녹색 차트 색)와 shadcn/ui의 의미 토큰·컴포넌트 형태를 따른 운영 도구용 라이트 테마 디자인 기준(Accepted). 메뉴는 녹색 기운만 남긴 검정에 가까운 상단 막대에 둔다. 화면 문구는 한국어·영어다. 글꼴은 실행파일에 넣은 Pretendard이고 굵기는 400·700만 쓴다."
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
  # 설정 목록 입력용 고정폭 단계.
  input-data:
    fontFamily: "ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, 'Liberation Mono', monospace"
    fontSize: 16px
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: 0
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
  # 도움말 사이트의 본문·절 제목·코드 단계.
  prose:
    fontFamily: "Pretendard, 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: 16px
    fontWeight: 400
    lineHeight: 1.75
    letterSpacing: -0.01em
  heading-section:
    fontFamily: "Pretendard, 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', system-ui, -apple-system, 'Segoe UI', sans-serif"
    fontSize: 20px
    fontWeight: 700
    lineHeight: 1.5
    letterSpacing: -0.03em
  code:
    fontFamily: "ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, 'Liberation Mono', monospace"
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.6
    letterSpacing: 0
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
  # 도움말 사이트의 목차 폭과 본문 최대 폭.
  sidebar-width: 280px
  prose-max: 760px
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

**이 문서는 채택된 기준(Accepted)이다.** 대시보드 구현은 이 문서의 토큰과 컴포넌트 기준을 따라야 한다. 설정 화면·로그 화면·도움말 사이트와 대시보드 도움말 링크에도 적용한다. 남은 미결정 사항은 [미결정 사항](#미결정-사항)에서 관리한다.


| 항목 | 값 |
| --- | --- |
| 상태 | Accepted |
| 적용 대상 | 운영 대시보드(SCR-001 운영 현황, SCR-002 세션 상세, SCR-003 관리자 로그인, SCR-004 설정 보기, SCR-005 동적 설정 편집, SCR-006 로그), 도움말 사이트. 라이트 테마만 둔다. Windows 트레이 앱의 알림 영역 아이콘도 [아이콘 규칙](#아이콘-규칙)의 브랜드 표식을 따른다 |
| 관련 PRD·스펙 | [PRD](../requirements/PRD.md) FR-06·AC-31, [대시보드 화면 스펙](../specs/dashboard.md) DASH-015·DASH-021·DASH-023. 트레이 아이콘은 PRD FR-10과 [Windows 트레이 앱 스펙](../specs/windows-tray.md)의 공통 시각 기준 |
| 관련 아키텍처 문서·ADR | [공통 컴포넌트 설계](../architectures/dashboard-components.md), [xtask 모듈 설계](../architectures/xtask.md). 관련 ADR은 없다. 정적 HTML·JS 선택은 [PRD 확정 기준](../requirements/PRD.md#확정-기준)이 정한다 |
| 기준 시안 | [dashboard.html](mockups/dashboard.html), 도움말 사이트의 [help-site.html](mockups/help-site.html) |
| 토큰 원본 | 이 파일의 YAML 머리말 |
| 컴포넌트 미리보기 | `web/dashboard/preview/`. `cargo xtask dashboard-serve`로 띄운다. 로컬 전용이며 실행 파일에 넣지 않는다. 명세의 컴포넌트를 상태별로 보여 준다 |
| 명세·도구 버전 | [DESIGN.md 명세](https://github.com/google-labs-code/design.md/blob/main/docs/spec.md) `version: alpha`. 검사 도구 버전은 검증 기록에 적는다 |

### 디자인 방향

- 대상 사용자와 주요 작업: 로그인한 관리자가 대기 연결·활성 세션·최근 종료 목록을 읽고, 한 세션을 골라 종료한다. 이 작업을 하는 화면은 [화면 스펙 1절](../specs/dashboard.md#1-범위와-사용자)을 따른다.
- 화면 구성: 위쪽에 검정에 가까운 짙은 색의 상단 막대를 둔다. 왼쪽에 브랜드와 메뉴, 오른쪽에 언어 선택과 [로그아웃]이 있다. 아래 본문은 밝은 회녹색 바탕 위에 페이지 머리말과 흰 카드 격자를 놓는다. 카드마다 제목 줄과 구분선이 있다. 서버 요약과 중계 연결·관리 연결 사용량은 첫 줄 카드에, 목록은 그 아래 전체 폭 카드에 둔다. 자세한 배치는 [화면 구성](#화면-구성)을 따른다.
- 정보 밀도와 강조 방식: 요약 숫자는 크게 보인다. 표는 촘촘한 행을 유지한다. 운영 도구는 한 화면에 많은 행을 보여야 하기 때문이다. 강조는 요약 숫자, 연결 사용량 고리, 상태 표시, 종료 행동에만 쓴다. 정상 값에는 배지를 붙이지 않는다.
- 시각적 특징: shadcn/ui의 형태를 따른다. 카드는 흰 바탕에 1px 경계와 얕은 그림자, 14px 모서리를 쓴다. 버튼·입력은 8px 모서리, 36px 높이이고, 포커스는 2px 링으로 표시한다. 배지는 6px 모서리의 작은 사각형이다. 주요 버튼은 짙은 녹색이고, 세션 종료 버튼만 빨간색이다. 청록·녹색은 데이터 표식과 사용량 고리에만 쓴다.
- 참고 자료: [참고 자료](#참고-자료)에 기록한다. 접근성은 [WCAG 2.2](https://www.w3.org/TR/WCAG22/) 대비·포커스·대상 크기 기준을 따른다.
- 대표 화면: [dashboard.html](mockups/dashboard.html)의 SCR-001(요약 카드·사용량 고리·긴 표 7열, 빈 목록·오류·갱신 중 상태), SCR-002(종료 확인 대화상자, 긴 주소), SCR-003(입력 폼). 확인할 시나리오는 [대시보드 화면 스펙](../specs/dashboard.md)의 DASH-001~003·DASH-009·DASH-015·DASH-018·DASH-021·DASH-023이다. 시안의 토큰 블록은 채택된 YAML에서 만든 값과 같다([검증](#검증)).

#### 참고 자료

**사용자 제공 첨부 이미지 두 장, 원본 링크 없음.** 첫째는 SEO 분석 대시보드 화면("My Dashboard")이다. 둘째는 같은 제품의 위젯 편집 패널을 확대한 화면이다. 제작자·출처·사용 조건을 알 수 없다. 이 프로젝트는 이미지의 시각 방향만 참고한다. 이미지의 문구·로고·아이콘·이미지 자산은 복제하지 않는다. 이미지 파일을 저장소에 두지 않는다.

| 구분 | 요소 | 적용 방식과 이유 |
| --- | --- | --- |
| 벗어난 점 | 세로 내비게이션 대신 상단 막대. 활성 항목의 밝은 바탕과 녹색 표시 막대는 유지한다 | 세로 레일로 본문 폭을 줄이지 않는다. `navbar` 토큰을 쓴다. 활성 항목은 `navbar-accent` 바탕과 아래쪽 `navbar-primary` 막대로 표시한다. 아이콘 옆에 글씨 레이블을 둔다. 설정 항목도 같은 표시를 재사용한다 |
| 채택 | 밝은 회녹색 바탕, 큰 페이지 제목, 제목 아래 흐린 보조 줄 | 페이지 머리말은 바탕(`background`) 위에 둔다. 보조 줄에는 마지막 성공 갱신 시각을 쓴다 |
| 채택 | 흰 카드 격자, 카드 제목 줄 아래 구분선, 오른쪽 위 조작 | `card` 컴포넌트를 쓴다. SCR-001의 세 목록 제목 옆에는 개수 배지를 둔다. 문구는 [화면 스펙 CPY-084·085](../specs/dashboard.md#932-scr-001-운영-현황)를 따른다 |
| 채택 | 고리형 게이지(DR·UR·Top AR)와 가운데 큰 숫자 | 중계 연결·관리 연결 사용량에 `gauge`를 쓴다. 현재 값과 상한을 글씨로 함께 쓴다 |
| 채택 | 범례의 작은 색 사각형(청록·녹색) | 방향별 바이트 앞의 표식(`chart-1`·`chart-2`)으로 쓴다 |
| 채택 | 표의 흐린 열 머리글과 얇은 행 구분선 | `muted-foreground` 머리글과 `border` 구분선을 쓴다 |
| 채택 | 둘째 이미지의 폼 행, 녹색으로 칠한 선택 상태, 짙은 녹색 주요 버튼 | 입력과 주요 버튼(`primary`)에 쓴다 |
| 벗어난 점 | 세션 종료·종료 확인 버튼은 빨간색 | 되돌릴 수 없는 행동을 주요 버튼과 구분한다. 이미지에는 위험 행동이 없다 |
| 벗어난 점 | 입력 경계는 3:1 대비의 진한 회녹색(`input`) | 입력은 경계로 식별하므로 WCAG 1.4.11의 3:1이 필요하다. 이미지와 shadcn/ui의 연한 경계는 이 기준에 못 미친다 |
| 벗어난 점 | 사용량 고리의 채움은 진한 청록(`gauge`) | 이미지의 연한 청록은 흰 바탕에서 3:1에 못 미친다 |
| 벗어난 점 | 메뉴 바탕은 이미지의 짙은 녹색 대신 녹색 기운만 남긴 검정에 가까운 색(`navbar`) | 채도 높은 녹색 면이 넓으면 투박해 보인다. 녹색은 활성 표시 막대(`navbar-primary`)에만 둔다. 브랜드 표식 선은 순백색(`navbar-brand`)이다 |

**[shadcn/ui](https://ui.shadcn.com/docs/theming)(Theming 문서).** React·Tailwind CSS 기반 컴포넌트 모음이다. 이 프로젝트는 시각 관례만 참고한다. 코드·패키지·빌드 도구는 쓰지 않는다.

| 구분 | 요소 | 적용 방식과 이유 |
| --- | --- | --- |
| 채택 | 의미 토큰 이름(`background`·`foreground`·`card`·`muted`·`muted-foreground`·`border`·`input`·`ring`·`primary`·`primary-foreground`·`destructive`·`chart-*`) | 같은 이름을 `colors.*`에 쓴다. 바탕 토큰과 `-foreground` 글씨 토큰을 짝으로 쓴다. 메뉴 색은 shadcn/ui `sidebar-*`와 같은 구조(`navbar`·`-foreground`·`-accent`·`-primary`·`-ring`)로 두되, 상단 막대에 쓰므로 이름은 `navbar-*`다 |
| 채택 | 모서리 단계(`sm`·`md`·`lg`·`xl`) | `rounded.sm` 6px, `md` 8px, `lg` 10px, `xl` 14px다. 문서 기본값 0.625rem(10px)을 `lg`로 보고 비율을 맞춘다 |
| 채택 | Button 변형(default·outline·ghost·destructive), Card, Badge, Alert, Dialog, Table, Input의 형태 | [Components](#components)에서 변형별로 정한다 |
| 채택 | 포커스 링: 2px 링과 2px 간격 | `ring` 색을 쓴다. 상단 막대 안에서는 `navbar-ring`을 쓴다 |

### 정적 파일 원칙

대시보드는 별도 빌드 단계 없는 정적 HTML·JS다([화면 스펙 1절](../specs/dashboard.md#1-범위와-사용자)). 따라서 다음을 지켜야 한다.

- 글꼴·스타일·스크립트·아이콘을 외부에서 불러오지 않는다. CDN, 웹 글꼴 서비스, `@import url(...)`, 아이콘 글꼴, shadcn/ui·Tailwind CSS 패키지를 쓰지 않는다.
- 모든 CSS·이미지·글꼴 파일은 실행파일에 포함한 정적 파일이어야 한다. 글꼴 파일은 Pretendard 부분 글자 woff2 두 개뿐이다([Typography](#typography)).
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

값을 CSS에서 직접 쓰지 않고 `var(--…)`로 참조한다. 이 규칙은 YAML에서 CSS를 기계적으로 만들 수 있게 한다. 생성 방법과 대조는 [프런트엔드 컨벤션 2절](../conventions/frontend.md#2-토큰-적용과-재사용)에서 정한다. 폭 토큰(`spacing.label-col`·`kv-label-col`·`alert-text-min`·`index-min`·`login-max`·`dialog-max`)은 레이아웃 폭을 정한다. 미디어 쿼리 조건은 사용자 정의 속성을 쓸 수 없어 예외다([중단점과 모바일 배치](#중단점과-모바일-배치)).

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
| `colors.primary` / `colors.primary-foreground` / `colors.primary-hover` | 주요 행동 | 로그인 버튼. 설정 보기의 편집·변경 확인·적용에도 재사용한다. 한 영역에 주요 버튼은 하나만 둔다 | 버튼 글씨 |
| `colors.link` | 링크 글씨 | 세션 식별자 링크 | 밑줄을 함께 쓴다 |
| `colors.destructive` / `colors.destructive-foreground` / `colors.destructive-hover` | 위험한 행동 | 세션 종료 버튼과 종료 확인 버튼에만 쓴다. 입력 오류 경계와 오류 문장에도 쓴다 | 버튼에 "종료" 글씨를 쓴다 |
| `colors.navbar` / `colors.navbar-foreground` / `colors.navbar-muted-foreground` | 상단 막대 바탕·글씨 | 모든 폭의 상단 막대. 브랜드 이름·활성 항목·언어 선택 글씨는 `navbar-foreground`, 나머지 항목은 `navbar-muted-foreground`다. 언어 선택 경계도 `navbar-muted-foreground`다. 로그인 화면의 브랜드 표식 바탕은 `navbar`다 | 상단 막대 안의 모든 글씨 |
| `colors.navbar-accent` / `colors.navbar-primary` | 상단 막대 활성 항목 | `navbar-accent`는 활성 항목·hover 바탕과 상단 막대 브랜드 표식 바탕, `navbar-primary`는 활성 항목 아래 표시 막대다 | 글씨 레이블 |
| `colors.navbar-ring` | 상단 막대 포커스 링 | 상단 막대 안 조작 요소의 `:focus-visible` 윤곽선 | `spacing.focus-ring-width` |
| `colors.navbar-brand` | 브랜드 표식 선 | 상단 막대와 로그인 화면의 브랜드 표식 화살표 선. 순백색이다. 글씨에는 쓰지 않는다 | 브랜드 이름 글씨 |
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
| 위험 | `colors.destructive`·`destructive-bg`·`destructive-border` | 조회 실패, 종료 실패, 로그인 실패 안내, 종료 원인 `connection_error`·`internal_error` | 실패를 뜻한다. 빈 목록과 구분한다 |
| 중립 | `colors.neutral`·`neutral-bg`·`neutral-border` | 활성 세션 모드·전송 경로·역할, 확인 불가 값, 대상 없음, 권한 없음, 종료 원인 `peer_eof`·`admin_terminated`·`server_shutdown`, 목록에 없는 알 수 없는 값 | 알 수 없는 값은 받은 그대로 보여주고 오류 색을 쓰지 않는다([관리 API 계약](../contracts/management-api-v1.md) 3절) |

세션 상태 `active`는 배지 없이 표시한다. 정상 상태를 모두 강조하면 예외가 보이지 않기 때문이다. 이 매핑은 시각 표현만 정한다. 상태의 의미는 스펙과 계약이 정한다.

"조회 중"(CPY-016)은 나오는 모든 자리에서 같은 정보 배지(`badge-info`, 시계 아이콘 + 글씨)로 쓴다. SCR-001의 자리는 카드 머리의 상태 배지, 서버 요약 값, 사용량 고리 가운데, 목록 표의 빈 행이다. SCR-002의 자리는 세션 정보 카드 머리, 사실 다섯 줄의 값, 전송량 두 방향의 값이다. SCR-002는 첫 조회 중에도 항목 이름과 방향 표식을 보이고 값 자리에만 배지를 둔다. 같은 상태가 자리마다 다르게 보이지 않게 하기 위해서다. "확인 불가"는 중립 배지다. 이 규칙은 위 표의 정보 행(최초 로딩)을 모든 자리에 적용한다.

### 테마

라이트 테마만 둔다. `prefers-color-scheme`에 따른 대체 값과 수동 전환 버튼을 두지 않는다. 시안과 구현은 `color-scheme: light`를 선언한다.

### 대비 기준

기준은 WCAG 2.2 AA다. 일반 글씨는 대비 4.5:1 이상, 입력 경계·포커스 링·사용량 고리 채움은 3:1 이상이어야 한다. 아래 값은 YAML의 값에 WCAG 상대 휘도 공식을 적용한 대비다. 상태 쌍은 배지·안내 바탕 위의 글씨를 기준으로 한다.

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

글꼴 값의 원본은 YAML이다. 글꼴은 실행파일에 넣은 Pretendard이고, 값(고정 폭)만 시스템 고정 폭 글꼴이다.

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
| 입력 | `typography.input` | 계정명·비밀번호·설정 숫자 입력. 16px 굵기 400, 행간 `normal` | 모든 입력은 모든 폭에서 16px 이상이다. 모바일 브라우저가 입력 때 화면을 확대하지 않게 한다 |
| 값(고정 폭) | `typography.data` | 세션 식별자, 중계 ID, 상대 주소. 설정 키·문자열 값도 같은 단계다. 12px 굵기 400, 행간 150%, 자간 0 | 긴 값은 `overflow-wrap: anywhere`로 줄바꿈한다. 값을 잘라 표시하지 않는다 |
| 입력 값(고정 폭) | `typography.input-data` | SCR-005의 여러 줄 목록 입력. `data`와 같은 고정폭 계열·400 굵기·자간 0, 16px·행간 150% | 키 이름과 보기 값은 기존 `data`로 유지한다. 입력 값만 16px 이상을 지킨다. 긴 항목은 줄바꿈하며 항목의 원래 줄 번호를 유지한다 |
| 요약 숫자 | `typography.metric` | 서버 요약의 개수·시간 길이, 사용량 고리 가운데 현재 값, 세션 상세의 전송량. 28px 굵기 700, 행간 150% | 자릿수가 맞는 숫자. 폭이 모자라면 줄바꿈한다 |

- 글꼴: Pretendard 1.3.9의 부분 글자 woff2 두 개(Regular 400, Bold 700)를 실행파일에 넣는다. 부분 글자 파일은 KS X 1001 한글 2,350자와 라틴 문자를 담는다. 이 밖의 글자는 그 글자만 다음 글꼴로 보인다. 글꼴 스택은 `Pretendard, 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', system-ui, -apple-system, 'Segoe UI', sans-serif`다.
- 굵기: 400(Regular)과 700(Bold)만 쓴다. 파일을 두 개로 줄이고, 대체 글꼴인 맑은 고딕(Malgun Gothic)에서도 위계를 지키기 위해서다. 맑은 고딕은 400과 700만 지원하며 500(Medium)이나 600(SemiBold)은 400으로 렌더링되어 위계가 사라진다.
- 크기: 모든 글꼴 크기는 짝수 px이고 12px 이상이어야 한다. 값 글꼴은 12px이다.
- 자간: Pretendard를 쓰는 모든 대시보드 단계는 -3%(`-0.03em`)다. 값(고정 폭) 단계는 0이다. 고정 폭 글꼴의 글자 폭과 자릿수 정렬을 지키기 위해서다. 글꼴 단계를 쓰는 CSS 규칙은 `font-family`와 함께 `letter-spacing`도 그 단계 값으로 정해야 한다. 자간은 상속되므로 생략하면 부모의 값이 남는다. 도움말 본문은 [도움말 사이트 확장](#도움말-사이트-확장)의 `typography.prose`를 따른다.
- 행간: 두 줄 이상이 될 수 있는 대시보드 역할은 150%(`1.5`)다. 한 줄로 고정된 역할(배지, 버튼, 강조 버튼, 입력, 버튼 글꼴을 쓰는 상단 막대 항목과 언어 선택)은 `normal`(글꼴의 기본 행간)이다. CSS는 실제 줄 수를 알 수 없으므로 역할별로 정한다. 도움말 본문·코드는 [도움말 사이트 확장](#도움말-사이트-확장)의 단계를 따른다.
- 시각 위계: 굵기(400 대 700), 크기, 색(`foreground` 대 `muted-foreground`)의 조합으로 위계를 드러낸다. 상대 굵기(`bolder`)를 쓰지 않고 명시적 700 토큰을 쓴다.
- 글꼴 자산과 사용 권한: Pretendard는 SIL Open Font License 1.1이다. 원본 파일과 라이선스 전문은 `web/dashboard/fonts/`에 둔다. 라이선스 전문은 배포 고지 `THIRD-PARTY-NOTICES.txt`에도 넣는다. 처리 방법은 [프런트엔드 컨벤션 2절](../conventions/frontend.md#2-토큰-적용과-재사용)을 따른다.
- 글꼴 로딩과 대체: `@font-face`는 `font-display: swap`을 쓴다. 글꼴 파일을 받기 전이나 받지 못했을 때는 위 스택의 다음 글꼴로 보인다. 데이터 고정 폭 글꼴은 `ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, 'Liberation Mono', monospace`를 쓴다.
- UI 문구 규칙: 화면 문구는 [화면 스펙](../specs/dashboard.md)이 정한다. 문구 확정은 [화면 스펙 9절](../specs/dashboard.md#9-화면-문구)이 관리한다. 이 문서는 문구를 정하지 않는다.
- 화면 언어: 한국어와 영어 두 가지다. 언어 선택 규칙은 [화면 스펙의 화면 언어](../specs/dashboard.md#화면-언어)를 따른다. 두 언어가 같은 글꼴 스택과 토큰을 쓴다. 문서 루트의 `lang` 속성을 고른 언어(`ko`·`en`)로 바꾼다. 영어 문구가 한국어보다 길 수 있으므로 버튼·레이블·카드 제목에 고정 폭을 두지 않는다. 언어 이름은 각 언어로 쓴다("한국어", "English").

### 표시 형식

값의 의미와 단위는 [화면 스펙 3절](../specs/dashboard.md#시각전송량-표시-형식)이 정한다. 이 절은 그 스펙이 맡긴 자릿수·전환 경계를 정한다. 화면 언어를 바꿔도 형식은 같다. 단위 글씨만 언어에 따라 바뀐다.

| 값 | 형식 | 예 |
| --- | --- | --- |
| 전송량 | KiB와 MiB만 쓴다. 소수 첫째 자리까지 반올림해 표시한다. KiB 값이 반올림 뒤 1024.0 이상이면 MiB로 바꾼다. 자릿수 구분 기호는 브라우저 언어 설정을 따르며 화면 언어 선택과 관계없다. 1 GiB 이상도 MiB로 쓴다 | `0`→`0.0 KiB`, `1536`→`1.5 KiB`, `1048575`→`1.0 MiB`, `1572864`→`1.5 MiB` |
| 시각 | 브라우저 현지 시각 `YYYY-MM-DD HH:mm:ss`와 시간대 약어를 함께 쓴다. 두 언어에서 같다 | `2026-10-16 14:03:25 GMT+9` |
| 시간 길이(가동·대기·지속) | `D일 HH:MM:SS`(영어 `Dd HH:MM:SS`) 형식이다. 일수가 0이면 일 부분을 뺀다. 최근 종료 항목의 지속 시간도 같다 | `1일 02:03:04`·`1d 02:03:04`, `00:00:07` |
| 확인 불가 값 | 글씨 "확인 불가"(영어 "Unavailable")와 중립 상태 색을 쓴다. 0이나 빈 칸으로 쓰지 않는다 | 연결 수 `null` |

## Layout

### 화면 구성

보호된 화면(SCR-001·SCR-002·SCR-004·SCR-005·SCR-006)은 앱 틀(상단 막대 + 본문) 안에 놓인다. 로그인 화면(SCR-003)은 상단 막대 없이 바탕 가운데에 카드 하나를 두고, 카드 위 오른쪽에 언어 선택을 둔다.

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
| 상단 막대 | 높이 `spacing.appbar-height`, 전체 폭, `navbar` 바탕. 왼쪽부터 브랜드 표식과 "VNC Relay", 운영 현황·로그·설정 세 항목, 오른쪽 끝에 도움말 링크·언어 선택·[로그아웃]. 가로 안쪽 여백은 본문과 같은 `spacing.xl` | 항목·브랜드·언어의 토큰을 재사용. 세 항목 순서와 활성 범위는 [로그 화면 확장](#로그-화면-확장), 도움말 링크는 [도움말 사이트 확장](#도움말-사이트-확장)을 따른다 |
| 페이지 머리말 | 왼쪽에 제목과 마지막 성공 갱신 시각, 오른쪽에 [새로고침]. SCR-002는 제목 위에 [목록으로] ghost 버튼 | DASH-003~005·DASH-007 |
| 상태 안내 | 머리말 아래 전체 폭 Alert. 상태가 있을 때만 | 화면 스펙 4절 |
| SCR-001 첫 줄 | 12열 격자. 서버 요약 카드 6열, 중계 연결 카드 3열, 관리 연결 카드 3열. 연결 카드는 제목 아래 한 줄 설명으로 무엇을 세는지 밝힌다 | DASH-001·DASH-021 |
| SCR-001 목록 | 대기 연결, 활성 세션, 최근 종료 목록 카드를 이 순서로 전체 폭에 쌓는다. 화면 스펙 와이어프레임의 순서와 같다 | DASH-001·DASH-002·DASH-020 |
| SCR-002 | 세션 정보 카드 8열(모드·전송 경로·시작 시각·주소, 아래 카드 바닥에 [세션 종료]), 전송량 카드 4열(방향별 바이트) | DASH-006·DASH-009 |
| SCR-003 | 바탕 가운데, 위 오른쪽에 언어 선택(로그인형), 그 아래 브랜드 표식, 아래 로그인 카드 | DASH-017·DASH-018·DASH-024·DASH-025 |

- 공통 콘텐츠 폭: 본문은 `spacing.content-max` 이하다. 좁은 화면에서는 전체 폭을 쓴다.
- 바깥 여백과 간격: 간격은 `spacing.xs`~`spacing.2xl` 척도만 쓴다. 데스크톱 폭의 본문 바깥 여백은 `spacing.2xl`(세로)·`spacing.xl`(가로)이다. 머리말·안내·카드 줄 사이와 카드 사이는 `spacing.lg`다. 카드 제목 줄의 안쪽 여백은 `spacing.lg`(세로)·`spacing.xl`(가로), 카드 본문은 `spacing.xl`이다. 표가 들어간 카드 본문은 가로 여백을 `spacing.md`로 줄이고, 표 칸 안쪽은 `spacing.sm`(세로)·`spacing.md`(가로)다. 첫 열 글씨가 카드 제목과 왼쪽 선이 맞는다. 모바일 폭의 본문 바깥 여백은 `spacing.lg`, 카드 본문은 `spacing.lg`다.
- 밀도 변형: 없음. 하나의 밀도만 쓴다. 모바일 폭에서는 조작 요소 높이만 `spacing.control-height-touch`로 키운다.
- 콘텐츠 경계 조건: 긴 중계 ID·IPv6 주소·세션 식별자는 줄바꿈해 모두 보여준다. 영어 1024px 최근 종료 표에서 가장 긴 종료 원인(CPY-047)이 주소 열을 좁히면 IPv4 주소도 값 안에서 줄바꿈한다. 이 줄바꿈은 허용한다. 시각 값은 폭이 모자라면 날짜와 시각 두 줄로 나눈다. 대기 연결 표의 접속 시각과 활성 세션 표의 시작 시각은 폭과 관계없이 날짜와 시각 두 줄이다. 빈 목록은 스펙의 빈 목록 안내를 표 자리에 보이고 열 머리글은 유지한다. 알 수 없는 상태 값은 받은 그대로 보여준다.

### 중단점과 모바일 배치

화면 동작은 [화면 스펙](../specs/dashboard.md)을 따른다. 이 절은 배치 기준만 정한다.

| 이름 | 조건 | 적용 |
| --- | --- | --- |
| 모바일 폭 | 뷰포트 폭 1023.98px 이하(`@media (max-width: 1023.98px)`) | 브랜드 이름 글씨를 감춘다. 세 내비게이션 항목은 둘째 줄에 표시한다. 카드를 한 열로 쌓는다. 표를 카드 목록으로 바꾼다. 조작 요소 높이는 `spacing.control-height-touch`다 |
| 데스크톱 폭 | 뷰포트 폭 1024px 이상 | 상단 막대 전체 항목, 12열 격자, 표를 쓴다. 조작 요소 높이는 `spacing.control-height`다 |

중단점은 하나만 둔다. CSS 미디어 쿼리에는 사용자 정의 속성을 쓸 수 없으므로 값 1024px를 CSS 한 곳에서만 쓴다. 1024px에서 본문 여백을 뺀 카드 안 표 영역은 시안에서 956px이고, 활성 세션 7열 표의 최소 내용 폭은 753px이다. 최근 종료 8열 표는 시안에서 최소 내용 폭이 한국어 817px, 영어 885px이며 956px 안에 들어간다. 최소 지원 폭은 320px이다.

| 구성 | 데스크톱 폭 | 모바일 폭 |
| --- | --- | --- |
| 상단 막대 | 브랜드 표식과 "VNC Relay", 운영 현황·로그·설정, 언어 선택, [로그아웃] | 브랜드 표식·언어 선택·[로그아웃] 아래 둘째 줄에 운영 현황·로그·설정을 모두 표시한다. 브랜드 이름은 화면에서만 감추고 접근성 트리에는 남긴다. SCR-002의 [목록으로]는 페이지 머리말에 그대로 둔다 |
| 페이지 머리말 | 왼쪽에 제목·갱신 시각, 오른쪽에 [새로고침]을 한 줄에 둔다 | 제목, 갱신 시각, 전체 폭 [새로고침] 순서로 쌓는다 |
| 서버 요약 | 카드 안 값 격자 3열 | 실행 상태와 가동 시간은 각각 한 줄 전체 폭, 나머지 네 값은 2열. 320px에서도 같다. 순서는 데스크톱과 같다 |
| 중계 연결·관리 연결 사용량 | 카드 둘, 각 3열 | 각각 전체 폭 카드. 고리와 값은 가운데 정렬을 유지한다 |
| 대기 연결 표 | 6열 표(역할·중계 ID·전송 경로·상대 주소·접속 시각·대기 시간). 상대 주소 칸은 소켓 주소 아래에 있는 전달 주소만 레이블과 함께 보인다. 레이블은 값 위 줄에 둔다. 접속 시각 칸은 날짜 아래 줄에 시각(시간대 포함)을 둔다(시안의 `.wd`·`.wt`) | 행마다 첫 줄에 중계 ID와 역할 배지, 아래에 레이블·값 쌍(전송 경로, 상대 주소, 접속 시각, 대기 시간) |
| 활성 세션 표 | 7열 표(세션·모드·전송 경로·시작·방향별 바이트·상대 주소·상세). 시작 칸은 날짜 아래 줄에 시각(시간대 포함)을 둔다(시안의 `.wd`·`.wt`). 상대 주소 칸의 레이블은 값 위 줄에 둔다(시안의 `.addr .k`) | 행마다 첫 줄에 세션 식별자 링크와 [상세] 버튼, 아래에 레이블·값 쌍(모드, 전송 경로, 시작, 방향별 바이트 2줄, 상대 주소) |
| 최근 종료 표 | 8열 표(세션·종료 원인·모드·전송 경로·시각·방향별 바이트·상대 주소·로그). 시각 칸은 시작 시각·종료 시각·지속 시간 세 줄이고, 로그 칸은 [로그 보기] outline 버튼이다. 시각 줄은 레이블, 날짜, 시각(시간대 포함)을 이어 쓴다. 폭이 모자라면 시각부터 다음 줄로 넘기고, 날짜와 시각은 각각 통째로 넘긴다. 1280px에서는 날짜가 레이블 옆에 온다. 상대 주소 칸의 레이블은 값 위 줄에 둔다. 전송 경로와 방향별 바이트 머리글은 낱말 사이에서 줄바꿈을 허용한다(시안의 `.tl`·`.addr .k`·`th.wrap`) | 행마다 첫 줄에 세션 식별자와 종료 원인 배지, 아래에 레이블·값 쌍(모드, 전송 경로, 시각 3줄, 방향별 바이트 2줄, 상대 주소, [로그 보기]) |
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
  - 행동: 새로고침(원호 화살표), 로그아웃(문과 오른쪽 화살표), 목록으로(왼쪽 화살표), 상세(오른쪽 꺾쇠), 세션 종료(원 안 가로줄), 언어 선택(원 안 경선·적도의 지구), 펼침·접기(아래·위 꺾쇠), 검색(돋보기), 다운로드(아래 화살표와 받침선), 닫기(×), 일시정지(막대 둘), 경로 화살표(→).
  - 내비게이션: 운영 현황(네 칸 격자), 로그(문서와 줄 세 개), 설정(가로선 세 개와 조절 막대).
  - 브랜드 표식: 둥근 사각형 안의 서로 반대 방향 화살표 두 개. 중계를 뜻한다. 참고 이미지의 로고를 복제하지 않는다.
    - Windows 트레이 아이콘: 상단 막대의 브랜드 표식과 같다. `spacing.brand-size` 정사각형, `rounded.md` 모서리, `colors.navbar-accent` 바탕, `colors.navbar-brand` 선이다. 화살표는 `spacing.icon-size`의 1.25배 크기로 가운데에 두고 선 두께도 같은 비율로 키운다. 트레이 API는 SVG를 받지 않으므로 같은 경로를 32×32 RGBA로 그린다. 서비스 상태에 따라 그림을 바꾸지 않고, 상태는 툴팁과 메뉴의 상태 줄이 글씨로 보여 준다.
- 기호 없는 중립 배지: 중립 상태 배지와 역할·모드·전송 경로 배지(Viewer, Server, Mode I, Mode II, TCP, WSS, 확인 불가 등), 개수 배지는 아이콘 없이 글씨만 쓴다. 정상 운영 데이터를 장식 기호로 어지럽히지 않기 위함이다. 중립 안내(Alert)에는 원 안 느낌표를 `neutral` 색으로 쓴다.

## Components

대시보드의 공통 컴포넌트 기준이다. 구현과 API는 [공통 컴포넌트 설계](../architectures/dashboard-components.md)가 정한다. 동작은 각 행의 스펙 시나리오를 따른다. 형태는 shadcn/ui의 같은 이름 컴포넌트를 따른다.

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

### 설정 화면 확장

설정 화면은 공통 토큰과 설정 목록 입력용 `typography.input-data`를 쓴다. Input·Alert·Badge·Navbar·Card·Dialog의 설정 화면 구성은 다음과 같다. 함수·props는 모듈 설계에서 정한다.

| 공통 요소 | 토큰·구성 | 폭·상태·사용 조건 | 상태 |
| --- | --- | --- | --- |
| 여러 항목 내비게이션 | `navbar-item`·`navbar-item-active`, `navbar-accent`·`navbar-primary`, `indicator-width` 재사용. 항목 사이 `spacing.xs`, 아이콘·글씨 사이 `spacing.sm` | 운영 현황·로그·설정 순서로 모두 표시. SCR-001·002는 운영 현황, SCR-006은 로그, SCR-004·005는 설정 활성. `aria-current="page"`. 모바일에서는 브랜드·언어·로그아웃 아래 둘째 줄을 전체 폭으로 쓰고 항목을 숨기지 않음. 상단 막대 높이는 이때 내용만큼 늘어남 | Accepted |
| 여러 줄 목록 입력 | Input과 같은 바탕·경계·모서리·포커스. 채택한 단계 `typography.input-data`(고정폭 16px·행간 150%). 안쪽은 세로 `spacing.xs`·가로 `spacing.sm`, 최소 높이 `spacing.control-height`의 3배 | 세로 크기 조절 가능. 모든 폭에서 너비 100%, 가로로 늘리지 않음. 한 항목 한 줄. 원문 줄을 유지하며 긴 문자열은 시각적으로 줄바꿈. 오류는 destructive 경계와 아래 글씨 | Accepted |
| 단위 있는 숫자 입력 | 기존 `typography.input`(16px). 옆 단위는 `typography.label`·`muted-foreground`. 범위는 도움말. 안쪽은 여러 줄 입력과 같음 | 입력과 단위를 같은 행에 놓고 입력은 남는 폭 사용. 도움말은 다음 줄. 초·바이트·개수. 모바일 입력 높이 `control-height-touch` 유지 | Accepted |
| 설정 불리언 입력 | 수준 체크 그룹의 체크박스 상자를 재사용: `card` 바탕, `input` 경계, `radius-md`, 최소 높이 `control-height`(모바일 `control-height-touch`), 체크박스는 `icon-size`·`primary`. 상자 안 label은 설정 키 이름(`typography.data`)이며 수준 배지는 없음 | 키 하나가 상자 하나다(`relay.ticket.<role>.enabled`). 상자는 격자 칸 전체 폭이다. 데스크톱에서는 옆 칸 입력과 높이를 맞추려고 키 이름 한 줄 높이(`typography.data`)와 `spacing.xs`만큼 내려 놓고, 모바일 1열에서는 내리지 않음. 저장 중·결과 불명처럼 입력을 잠그는 상태에서는 체크박스도 `disabled` | Accepted |
| 입력 도움말·오류 | 설정 키 라벨은 `typography.data`. 도움말은 `typography.label`·`muted-foreground`. 오류는 `typography.body`·`destructive`와 위험 아이콘. 간격 `spacing.xs` | 보이는 label에 키 포함. 보기 목록·입력 라벨·확인 대화상자의 키 이름은 같은 고정폭 단계. 도움말·오류 모두 입력과 연결. 오류는 색 외에 문장·줄 번호 포함. 첫 오류 초점은 스펙 6절 | Accepted |
| 적용 구분 표시 | dynamic은 `badge-info`, restart는 `badge-neutral`. 글씨와 정보 아이콘/재시작 필요 글씨로 구분. 기존 배지 형태 | 값 자체에 상태 배지를 붙이지 않음. 적용 구분은 별도 자리. 긴 영어 구분 문구는 배지 전체를 다음 줄로 보냄 | Accepted |
| 키·값 목록 | Card 안 항목별 `colors.border` 구분선, 안쪽 `spacing.lg`, 항목 사이 `spacing.md`. 키·문자열 값은 `typography.data`, 정수·불리언 값은 `typography.body-numeric`, 구분은 Badge | 데스크톱은 같은 너비 3열(키·값·적용 구분). 모바일은 키·값·구분 순서 1열. 배열은 항목별 줄, 빈 목록·null은 글씨로 구분. 긴 키·경로는 `overflow-wrap:anywhere`. 잘라 숨기지 않음 | Accepted |
| 화면 상단 안내 | 기존 Alert의 info·warning·destructive·neutral 재사용. 성공은 `colors.success-bg`·`success-border`·`success`, 설명은 `foreground`, 아이콘은 원 안 체크. 간격·모서리는 기존 Alert와 같음 | 머리말 아래 전체 폭. 저장 성공은 success, pending·충돌·결과 불명은 warning, 검증·쓰기 실패는 destructive, 꺼짐은 neutral. 파일 상태는 제목·본문·키·진단으로 표시 | Accepted |
| 변경 확인 | 기존 Dialog·Card·목록·Alert와 `dialog-max` 재사용. 키별 이전/새 값과 목록 추가/삭제를 세로로 쌓음. 항목 간 `spacing.lg` | 내용이 길면 대화상자 자체를 뷰포트 안에서 세로 스크롤. 최대 높이는 뷰포트에서 `spacing.md` 양쪽 여백을 뺀 값. 모바일 버튼은 전체 폭, 취소 먼저. 확인·적용은 primary, 취소는 outline | Accepted |
| 설정 구역 배치 | SCR-004는 테이블별 전체 폭 Card, SCR-005는 여섯 구역 전체 폭 Card. 여섯째 구역은 마지막의 "접속 ticket"(CPY-191)이다. 입력란 격자는 데스크톱 2열, 모바일 1열, 간격 `spacing.lg` | 읽기 순서는 스펙 구역 순서. 접속 ticket 구역은 Viewer 키 넷, Server 키 넷 순서이며 키는 계약 순서(`enabled`, `issuer`, `relay_host`, `max_lifetime_secs`)를 따름. 한 줄 문자열은 기존 Input(`typography.input`), 정수는 "단위 있는 숫자 입력"을 재사용. 페이지 행동은 데스크톱 오른쪽, 모바일은 전체 폭으로 쌓음. 기존 content-max·카드 여백을 유지 | Accepted |
| 재로드 정보 글꼴 | 번역한 결과·계기는 `typography.body`, 키 이름은 `typography.data`. 시각은 화면 머리의 조회 시각과 같은 `typography.label`·`muted-foreground` | 시각 값은 두 자리 날짜·시각과 시간대를 포함하는 기존 표시 형식. 번역 문구에 고정폭 글꼴을 쓰지 않음 | Accepted |

설정 항목 아이콘은 같은 `viewBox`와 선 규칙으로 그린 가로선 세 개와 조절 막대다. 네트워크·외부 아이콘은 쓰지 않는다. 글씨 레이블을 항상 함께 둔다.

세 내비게이션 항목은 모든 보호된 화면에 적용하는 공통 기준이다. 입력은 모두 16px 하한을 지킨다.

### 로그 화면 확장

**Accepted.** SCR-006 로그 화면과 SCR-001·002·004·005의 관련 구성 기준이다. 상단 막대 순서는 운영 현황 · 로그 · 설정이다. 시각 시안은 [dashboard.html](mockups/dashboard.html)의 SCR-006 프레임 32개(한국어·English 각 16개), SCR-002 `scr002-forwarded`·`scr002-forwarded-en`, SCR-001·004·005 프레임이다. 공통 색·간격·폭·글꼴 토큰을 쓴다. 모든 행은 Card·Data table·Badge·Alert·Button·Input·Navbar의 조합이나 변형이다. "관계·이유"는 공통 요소와의 관계와 해당 형태의 이유를 설명한다. 함수·props는 모듈 설계에서 정한다.

| 공통 요소 | 토큰·구성 | 폭·상태·사용 조건 | 관계·이유 | 상태 |
| --- | --- | --- | --- | --- |
| 세 항목 내비게이션 | 기존 `navbar-item`·`navbar-item-active`·`indicator-width` 재사용. 운영 현황·로그·설정 순서. 로그 아이콘은 줄 세 개가 있는 문서 모양 | SCR-006은 로그 활성. `aria-current="page"`. 모바일 둘째 줄에서 항목이 한 줄에 다 들어가지 않으면(320px English) 다음 줄로 넘기고 숨기지 않음 | "여러 항목 내비게이션"과 같은 구성이다. 세 항목을 모든 보호된 화면에서 같은 순서로 표시한다 | Accepted |
| 로그 필터 카드 | Card(`card`)와 카드 제목 `typography.heading-card`. 안쪽 `spacing.lg` 세로 간격. 폼 `role="search"`, 이름은 CPY-172 | 수준 그룹과 사건 선택을 한 줄(데스크톱)에, 검색, 세션 칩 순서. 모바일은 한 열 | 새 컨테이너가 아니라 Card. 필터를 목록과 같은 카드에 섞지 않아 목록이 카드 하나로 읽힌다 | Accepted |
| 수준 체크 그룹 | `fieldset`+`legend`(`typography.label`, 그룹 이름 CPY-151). 항목은 체크박스(`icon-size`, `primary`)와 수준 배지를 한 label로 묶은 상자: `card` 바탕, `input` 경계, `radius-md`, 최소 높이 `control-height`(모바일 `control-height-touch`). 항목 사이 `spacing.sm` | 세 수준 모두 켜짐이 기본. 하나도 없으면 destructive 경계와 위험 아이콘·문장(CPY-173)을 그룹 아래에 둠. 배지 글씨가 label 이름 | 체크박스는 브라우저 기본을 쓰되 Input의 경계·높이·모서리·포커스 링과 같은 상자 안에 둬 조작 대상 크기를 맞춘다. 수준 배지를 그대로 써 목록과 같은 표식이 필터에도 보인다 | Accepted |
| 사건 선택 | Input과 같은 바탕·경계·모서리·높이·16px(`typography.input`). 오른쪽 아래 꺾쇠(`icon-size-sm`). 언어 선택(`lang-select`)과 같은 구조의 밝은 바탕 | 폭은 `index-min` 이상 두 배 이하(데스크톱), 모바일은 전체 폭. 첫 항목은 CPY-153. visible label은 CPY-152 | 언어 선택기와 같은 네이티브 select 변형. 항목이 많아(ops-info 사건 32개) 칩·라디오보다 select가 맞다 | Accepted |
| 검색 입력·검색 버튼 | Input 한 줄(`typography.input`)과 `button-primary`(CPY-155)를 한 행에 놓음. 도움말(CPY-156)은 `typography.label`·`muted-foreground`. 오류(CPY-165)는 `aria-invalid`·destructive 경계와 위험 아이콘·문장 | 입력은 남는 폭, 버튼은 입력 높이. 모바일에서도 같은 행을 유지. 도움말·오류는 `aria-describedby`로 연결. 이 구역의 유일한 primary 버튼 | "입력 도움말·오류"와 같은 규칙을 검색에 적용. 검색 제출은 이 필터 구역의 주 행동이라 primary로 둠. 페이지의 다른 버튼은 outline | Accepted |
| 세션 필터 칩 | `neutral-bg`·`neutral-border`, `radius-md`, 안쪽 `spacing.xs`·`spacing.md`. 글씨 `typography.body`. 세션 식별자는 `typography.data`. 오른쪽 `button-outline` `sm`의 닫기 아이콘과 "필터 해제"(CPY-158). 아래에 CPY-186 설명 | 세션 필터가 있을 때만. 모바일은 칩 전체 폭과 전체 폭 해제 버튼. 잘못된 식별자는 이 칩에 원문을 보이고 요청하지 않음 | neutral Alert 색과 Button outline `sm`의 조합이다. 칩은 닫기 아이콘만이 아니라 글씨 버튼을 둬 접근 이름과 클릭 대상이 분명하다 | Accepted |
| 자동 갱신 토글 | `button-outline`과 같은 크기. 켜짐은 `info-bg` 바탕·`info-border` 경계, 새로고침 아이콘과 상태 글씨(`info`, 700). 꺼짐은 기본 outline, 일시정지 아이콘, 상태 글씨 `muted-foreground`. 이름 CPY-159, 상태 CPY-170·171 | `role="switch"`·`aria-checked`. 상태 글씨는 화면에만 보이고 접근 이름은 CPY-159다. 아이콘+글씨로 상태를 구분하며 색만으로 전하지 않음. 보관 꺼짐·잘못된 세션에서는 비활성 | Button outline의 "눌림" 상태다. 같은 `info` 상태 색을 쓰되 "조회 중"과 구분하려고 아이콘이 다르다(새로고침 고리 대 시계) | Accepted |
| 로그 표 | 기존 Data table 확장. 머리글 `typography.label`·`muted-foreground`. 시각 `typography.body-numeric`, 사건 이름·요약·상세 값은 `typography.data`. 열 폭은 `label-col` 배수(시각 1+`md`, 수준 3/4, 사건 2−`md`, 자세히 1+`xl`)로 고정하고 요약이 남는 폭을 씀. 표 이름은 CPY-178 caption(화면에서만 감춤) | 한 사건은 요약 행 하나와 상세 행 하나(상세는 접힘이면 `hidden`). 긴 값은 `overflow-wrap:anywhere`로 줄바꿈하며 자르지 않음. 요약할 필드가 없는 사건은 요약 칸이 빔(모바일에서는 그 항목을 생략). 모바일은 사건 카드: 제목=사건 이름, 오른쪽=수준 배지, 항목 이름 시각·요약, 마지막에 전체 폭 자세히 버튼 | 기존 표·모바일 카드 규칙을 따른다. 행마다 펼치는 상세 행과 `table-layout:fixed`를 쓴다. 사건 이름이 길고 값이 가변이라 열 폭을 고정해야 프레임마다 열이 흔들리지 않는다 | Accepted |
| 수준 배지 | info는 아이콘 없는 `badge-neutral`, warn은 삼각형 아이콘 `badge-warning`, error는 원 느낌표 아이콘 `badge-destructive`. 글씨는 원문 `info`·`warn`·`error`이며 번역하지 않음 | 목록과 필터 체크박스에 같은 모양. 색 외에 글씨와 모양이 있다 | 기존 상태 배지 3종 재사용. `badge-info`는 "조회 중·진행 중"에 쓰고 있어 정상 로그 수준에 쓰면 두 뜻이 섞인다. 정상 데이터는 강조하지 않는 중립 배지로 둠 | Accepted |
| 자세히·접기와 상세 패널 | `button-outline` `sm`과 아래·위 꺾쇠 아이콘. 상세 패널은 `background` 바탕, `border` 경계, `radius-md`, 안쪽 `spacing.md`. 키는 `typography.data`·`muted-foreground`, 값은 `typography.data`·`foreground`. 키 열은 `kv-label-col`. 중첩 값은 들여쓴 JSON 문자열을 `pre-wrap`으로 | `aria-expanded`·`aria-controls`로 상세 행과 연결. 원문 필드와 값을 변형 없이 모두 표시(`ts`·`level`·`event` 포함). 모바일은 키·값 한 열 | 키·값 목록의 모양을 재사용. 접기·펼치기는 SCR-004의 `details`와 달리 표 행마다 독립이라 버튼으로 구현한다 | Accepted |
| 이전 로그 더 보기 | 목록 카드의 `card-foot` 가운데에 `button-outline`(CPY-160) | `has_more`일 때만 표시. 모바일은 전체 폭 | 기존 Card 푸터 재사용. 가운데 정렬은 오른쪽 정렬 푸터와 다른 점이다 | Accepted |
| 로그 상태 안내 | 기존 Alert 재사용. 누락 경고(CPY-168)는 warning, 일시정지(CPY-169)는 neutral, 다운로드 진행(CPY-174)은 info, 다운로드 실패(CPY-175)는 destructive, 조회 실패(CPY-018·020·021)와 요청 거절(CPY-176·177)은 destructive+[새로고침] | 필터 카드와 목록 사이 전체 폭. 다운로드 안내는 목록 카드 머리 아래(다운로드 버튼 가까이). 진행·누락·일시정지는 `role="status"`, 실패는 `role="alert"`. 목록 본문은 라이브 영역이 아님 | 새 변형 없음. 목록 상태와 다운로드 상태를 다른 위치로 분리해 서로 덮지 않는다 | Accepted |
| 표 자리 안내 | 기존 `td.empty`. 빈 결과(CPY-166)·보관 꺼짐(CPY-167)·조회 중·확인 불가를 열 머리글 아래 한 칸에 보임 | 보관 꺼짐은 안내 오른쪽에 "설정 보기"(`button-outline` `sm`). 열 머리글은 유지 | SCR-001의 빈 목록 규칙과 같다 | Accepted |
| SCR-001 전송 경로 열 | 두 중립 배지 사이 오른쪽 화살표 아이콘(`icon-size-sm`, `muted-foreground`). 머리글 CPY-187 | 셀은 `viewer → target` 순서. 알 수 없는 값은 번역하지 않고 중립 배지에 원문. 머리글은 줄바꿈을 허용 | 기존 중립 배지 둘과 화살표. 열 수는 그대로 | Accepted |
| SCR-002 세션 정보 | 키·값 목록에 Server/대상 전송 경로(CPY-179), Viewer 전달 주소(CPY-180), Server/대상 전달 주소(CPY-181)를 표시. 설명 줄 CPY-188(`typography.label`·`muted-foreground`, 위 간격 `spacing.md`). 카드 푸터에 `button-outline`(링크) "로그 보기"(CPY-182)와 destructive 종료 | 전달 주소 행은 값이 있을 때만 표시하고 null이면 행을 두지 않음. 로그 보기는 SCR-006 세션 필터로 이동. 푸터 순서는 로그 보기, 상태 재조회(있을 때), 세션 종료 | 키·값 목록과 버튼을 재사용한다. 로그 보기를 outline으로 둬 위험 행동과 섞이지 않게 함 | Accepted |
| SCR-004·005 설정 계약 v4 | 공통 컴포넌트 재사용. 읽기 카드 6개(`[relay.ws]`·`[relay.ws.viewer]`·`[relay.ws.server]`·`[relay.wss]`·`[relay.wss.viewer]`·`[relay.wss.server]`)와 `[recent_logs]`. 편집 입력은 목록 13·숫자 16. trusted_proxies 입력은 CPY-185 도움말 줄을 표시 | 변경 확인 대화상자: 빈 trusted_proxies는 CPY-183, 빈 allowed_origins는 CPY-184, 빈 허용 목록은 CPY-105를 각각 별도 warning Alert로. 안내 문구는 CPY-189 | "키·값 목록"·"여러 줄 목록 입력"·"변경 확인"을 키만 바꿔 재사용 | Accepted |

로그 화면 아이콘은 모두 공통 규칙(`viewBox="0 0 16 16"`, `currentColor`, 선 굵기 `spacing.icon-stroke`, 둥근 끝·꺾임, `aria-hidden="true"`·`focusable="false"`)으로 직접 그린다: 로그(문서와 줄 세 개), 검색(돋보기), 다운로드(아래 화살표와 받침선), 닫기(×), 일시정지(막대 둘), 위 꺾쇠(아래 꺾쇠의 반전), 경로 화살표(→). 글씨 레이블을 항상 함께 두며 유니코드 기호 문자는 쓰지 않는다(단, 표 머리글 문구 CPY-187의 "→"는 문구의 일부다).

상태 색 연결: 로그 수준은 info=중립(글씨), warn=warning, error=destructive를 쓴다. 보관 꺼짐·일시정지는 neutral, 누락은 warning, 다운로드 진행은 info, 실패는 destructive로 기존 상태 색 표의 뜻을 따른다.

접근성 요약: 수준 체크박스는 보이는 그룹 이름을 가진 `fieldset`이다. 사건 선택·검색·자동 갱신은 보이는 label이 있다. 검색 도움말·오류는 `aria-describedby`로 연결한다. 목록 본문은 라이브 영역이 아니며 상태 변화만 알린다. 포커스 링은 기존 `ring` 2px을 그대로 쓴다. 입력은 모두 16px 이상이다. SCR-006의 첫 성공 전 시각 자리에는 화면 스펙 4절에 따라 CPY-035를 쓴다. 시안의 "마지막 성공 갱신: 아직 없음 / None yet"은 기준 문구가 아니다.

### 도움말 사이트 확장

**Accepted.** [도움말 사이트 스펙](../specs/help-site.md)의 HLP-001~003과 대시보드 도움말 링크(DASH-064)의 시각 기준이다. 시안은 [help-site.html](mockups/help-site.html)과 [dashboard.html](mockups/dashboard.html)의 도움말 링크 프레임이다. 도움말 사이트는 mdBook 0.5.4로 만든다. mdBook 기본 테마의 색·글꼴·간격 대신 이 문서의 토큰을 쓴다. 아래 다섯 토큰은 도움말 전용이며, 나머지는 공통 토큰과 컴포넌트를 재사용한다.

| 도움말 전용 토큰 | 값 | 쓰임 | 이유 |
| --- | --- | --- | --- |
| `typography.prose` | Pretendard 16px, 400, 행간 1.75, 자간 -0.01em | 도움말 본문의 문단·목록·인용 | 도움말은 긴 글을 읽는 화면이다. 대시보드 본문(`typography.body` 14px, 자간 -0.03em)은 짧은 레이블·값에 맞춘 단계다 |
| `typography.heading-section` | Pretendard 20px, 700, 행간 1.5, 자간 -0.03em | 장 안의 절 제목(`h2`) | 장 제목(`typography.heading-page` 24px)과 소절 제목(`typography.heading-card` 16px) 사이 단계가 없다 |
| `typography.code` | 고정폭 14px, 400, 행간 1.6, 자간 0 | 코드 블록과 본문 안 코드 | `typography.data`(12px)는 표 값용이라 긴 명령·설정 예시에는 작다 |
| `spacing.sidebar-width` | 280px | 데스크톱 폭의 목차 폭 | 가장 긴 장 이름(ko·en)을 한 줄에 담고 본문 폭을 `spacing.prose-max`로 남긴다 |
| `spacing.prose-max` | 760px | 본문 글 상자의 최대 폭 | 한 줄 길이를 한국어 약 45자, 영어 약 90자로 묶어 읽기 쉽게 한다 |

| 구성 | 토큰·구성 | 폭·상태·사용 조건 | 관계·이유 | 상태 |
| --- | --- | --- | --- | --- |
| 페이지 틀 | 바탕 `colors.background`. 위에 상단 막대(전체 폭), 아래 왼쪽 목차와 오른쪽 본문 열 | 데스크톱 폭은 목차 열 `spacing.sidebar-width`와 본문 열. 목차를 접으면 본문 열이 전체 폭을 쓴다. 모바일 폭은 상단 막대·목차·본문을 한 열로 쌓는다 | 대시보드 앱 틀(상단 막대 + 바탕 위 카드)과 같은 층 구조다 | Accepted |
| 상단 막대 | Navbar 재사용: `navbar` 바탕, 높이 `spacing.appbar-height`, 가로 안쪽 `spacing.xl`(모바일 `spacing.lg`). 왼쪽부터 목차 단추(`navbar-item`, 목차 아이콘 + HCP-003), 브랜드 표식과 사이트 이름(HCP-001, `typography.body-strong`, 첫 페이지 링크). 오른쪽에 검색 단추(`navbar-item`, 검색 아이콘 + HCP-004)와 언어 링크(`navbar-item`, 언어 아이콘 + HCP-012) | 페이지와 함께 스크롤하며 고정하지 않는다. 목차 단추는 목차가 펼쳐져 있으면 `navbar-item-active` 바탕이다. 모바일 폭에서는 사이트 이름을 화면에서만 감추고 항목 높이를 `spacing.control-height-touch`로 키운다. 오른쪽 묶음이 한 줄에 들어가지 않으면 다음 줄로 넘기고 숨기지 않는다 | 대시보드 상단 막대 규칙을 그대로 쓴다. 목차 단추를 활성 바탕으로 둬 상태를 색 외에 `aria-expanded`와 함께 알린다 | Accepted |
| 목차 | `card` 바탕, 오른쪽 `spacing.border-width` `colors.border` 선. 안쪽 `spacing.lg`(세로)·`spacing.md`(가로). 항목은 `typography.body`, `foreground`, 안쪽 `spacing.sm`·`spacing.md`, `rounded.md`. hover는 `muted` 바탕. 현재 장은 `muted` 바탕, `typography.body-strong`, 왼쪽 `spacing.indicator-width` `primary` 표시 막대, `aria-current="page"`. 고지 앞 구분선은 `colors.border` | 데스크톱 폭에서는 목차 열의 `card` 바탕과 오른쪽 선이 본문 열 높이만큼 이어진다. 열 안의 항목 목록은 세로로 스크롤해도 화면 안에 남고(sticky), 화면보다 길면 목록 안에서 스크롤한다. 모바일 폭에서는 상단 막대 바로 아래 전체 폭으로 펼치고 아래 `colors.border` 선을 둔다. 닫으면 보이지 않는다 | 상단 막대 항목형 표시(바탕 + 표시 막대)를 밝은 바탕에 맞게 쓴다. 목차는 본문과 다른 열이라 본문·초점을 가리지 않는다([예외](#예외)) | Accepted |
| 본문 글 상자 | Card 재사용(`card`, `colors.border` 경계, 카드 그림자, `rounded.xl`). 최대 폭 `spacing.prose-max`, 안쪽 `spacing.2xl`(모바일 `spacing.lg`). 본문 열 안 바깥 여백은 대시보드 본문과 같다 | 장 제목 `h1`은 `typography.heading-page`. `h2`는 `typography.heading-section`, 위 간격 `spacing.2xl`, 아래 `colors.border` 선과 `spacing.sm` 간격. `h3`는 `typography.heading-card`. 문단·목록은 `typography.prose`, 문단 사이 `spacing.lg`. 링크는 `colors.link` 밑줄. 제목의 절 링크는 `foreground`이며 hover 때만 밑줄 | 대시보드의 Card·Link 규칙을 긴 글에도 적용한다. 공통 색을 재사용한다 | Accepted |
| 본문 안 코드 | `typography.code`, `muted` 바탕, `rounded.sm`, 가로 안쪽 `spacing.xs` | 줄바꿈을 허용한다(`overflow-wrap:anywhere`) | 값 표시(`typography.data`)와 같은 고정폭 계열 | Accepted |
| 코드 블록과 복사 | `muted` 바탕, `colors.border` 경계, `rounded.lg`. 위 막대(`colors.border` 아래 선, 안쪽 `spacing.xs`·`spacing.sm`)에 복사 단추(`button-outline`, 높이 `spacing.control-height-sm`, 복사 아이콘 + HCP-015)와 그 왼쪽 결과 글씨(`typography.label`. 성공은 `success` 색과 성공 아이콘 + HCP-016, 실패는 `destructive` 색과 위험 아이콘 + HCP-017). 코드는 `typography.code`, 안쪽 `spacing.md`·`spacing.lg` | 코드 줄은 줄바꿈하지 않고 블록 안에서만 가로 스크롤한다([예외](#예외)). 결과 글씨는 `role="status"`로 알린다. 모바일에서 복사 단추 높이는 `spacing.control-height-touch` | Button outline `sm`과 상태 색 글씨의 조합이다 | Accepted |
| 인용(참고·주의) | Alert 재사용. "참고"·"Note"는 `alert-info`와 정보 아이콘, "주의"·"Warning"은 `alert-warning`과 경고 아이콘. 그 밖의 인용은 `alert-neutral`. 글씨는 `typography.prose`. 원문 첫 낱말(`**참고:**` 등 굵은 글씨)은 Alert 제목처럼 상태 색(`info`·`warning`)이고 나머지는 `foreground`다 | 원문 첫 낱말을 그대로 보여 주며 색 외에 글씨로 구분한다 | 새 변형 없이 Alert 색 쌍과 제목 색 규칙을 쓴다 | Accepted |
| 문서 표 | Data table 재사용: 바깥 틀은 `colors.border` 경계와 `rounded.lg`. 머리글 `typography.label`·`muted-foreground`, 행 사이 `colors.border` 선, 칸 안쪽 `spacing.sm`·`spacing.md`, 칸 최소 폭 `spacing.label-col`의 1.5배, 글씨 `typography.body` | 모든 폭에서 표 모양을 유지하고, 넓으면 표 틀 안에서만 가로 스크롤한다([예외](#예외)). 칸 안 코드는 낱말 중간에서 줄을 바꾸지 않고 항목 사이에서만 바꾼다 | 대시보드 표와 같은 모양. 바깥 틀은 스크롤 영역의 경계를 보인다 | Accepted |
| 그림 | 최대 폭 100%, `colors.border` 경계, `rounded.lg`, 위아래 `spacing.lg` | 대체 텍스트 필수. 화면 캡처는 실제 대시보드의 데스크톱 폭 화면이다. 구성도는 손으로 쓴 SVG다. 흰 바탕(`card`)에 밝은 테마 색 값(`muted`·`border`·`foreground`·`muted-foreground`·`primary` 계열·`link`·`info` 계열)과 본문 글꼴 목록만 쓴다. 규칙은 [도움말 작성 규칙](../help/AGENTS.md#구성도)을 따른다 | 카드 안 이미지에 Card 경계 규칙을 적용 | Accepted |
| 이전·다음 | 링크 두 개. `card` 바탕, `colors.border` 경계, `rounded.md`, 안쪽 `spacing.md`·`spacing.lg`. 첫 줄은 꺾쇠 아이콘과 HCP-013·HCP-014(`typography.label`·`muted-foreground`), 둘째 줄은 장 이름(`typography.body-strong`, `colors.link`, 밑줄) | 본문 글 상자 아래. 데스크톱은 이전 왼쪽·다음 오른쪽, 각 절반 폭. 모바일은 전체 폭으로 이전 위·다음 아래. 첫 장에는 이전, 마지막 장에는 다음이 없다 | Button outline 모양의 링크. 글씨 레이블이 있어 아이콘만으로 뜻을 전하지 않는다 | Accepted |
| 검색 패널 | Card 재사용. 본문 열 맨 위, 최대 폭 `spacing.prose-max`. 제목 줄은 `typography.heading-card` 제목(HCP-005, 입력 `label`)과 오른쪽 닫기(`button-outline` `sm`, 닫기 아이콘 + HCP-011). 입력은 Input(`typography.input`, 전체 폭). 도움말 HCP-006은 `typography.label`·`muted-foreground`. 상태(HCP-007~HCP-010)는 `typography.body`. 결과는 항목마다 위 `colors.border` 선, 안쪽 `spacing.md`(세로). 첫 줄 장 이름 › 절 이름(`typography.body-strong`, `colors.link`, 밑줄), 둘째 줄 발췌(`typography.body`, `muted-foreground`). 일치 부분은 `<mark>`, `warning-bg` 바탕과 `foreground` 글씨 | 결과 수가 많아도 패널 안에서 줄이지 않고 페이지가 스크롤된다. 실패 상태는 `alert-destructive`, 불러오는 중은 `alert-info`. 모바일에서 입력·단추 높이 `spacing.control-height-touch` | 로그 화면의 검색(Input + 도움말 + 상태 Alert)과 같은 조합. 강조색은 상태 색 가운데 뜻이 겹치지 않는 경고 바탕만 빌린다 | Accepted |
| 단축키 도움말 | Dialog 모양: `card`, `rounded.xl`, 대화상자 그림자, 뒤에 `colors.overlay`. 제목 `typography.heading-card`, 줄마다 키(`kbd`: `typography.code`, `muted` 바탕, `colors.border` 경계, `rounded.sm`, 안쪽 `spacing.xs`)와 설명(`typography.body`) | 폭은 `spacing.dialog-max` 이하, 가운데 | 대시보드 Dialog 모양 재사용 | Accepted |
| 언어 선택(HLP-002)·없는 페이지(HLP-003) | 로그인 화면 배치 재사용: 바탕 가운데, 위에 브랜드 표식, 아래 최대 폭 `spacing.login-max` 카드. 제목 `typography.heading-page`, 안내 `typography.body`. 두 언어 글씨는 각자 `lang`을 가진 줄로 쌓는다. 언어 링크는 `button-outline` 모양(높이 `spacing.control-height`, 모바일 `spacing.control-height-touch`) | 링크 두 개를 데스크톱에서는 한 줄 같은 폭, 모바일에서는 전체 폭으로 쌓는다 | 로그인 카드와 같은 첫 화면 모양. 두 선택이 같은 무게라 primary를 쓰지 않는다 | Accepted |
| 대시보드 도움말 링크 | 상단 막대형: 상단 막대 항목형 링크(`navbar-item`), 도움말 아이콘 + CPY-195 + 새 탭 아이콘(`spacing.icon-size-sm`, 글씨와 간격 `spacing.xs`). 로그인형: Link 재사용(`colors.link`, 밑줄, `typography.body`), 같은 두 아이콘(간격 `spacing.xs`), 높이 `spacing.control-height`(모바일 `spacing.control-height-touch`), 언어 선택과 간격 `spacing.md` | 상단 막대에서는 오른쪽 도구 맨 앞(언어 선택 앞), SCR-003에서는 언어 선택 앞. 모바일 상단 막대 첫 줄에 들어가지 않으면 도구 묶음 안에서 오른쪽 정렬로 다음 줄에 넘기고 숨기지 않는다. 이때 브랜드 표식은 첫 줄 높이(`spacing.control-height-touch`)에 맞춘다. 새 탭 아이콘은 장식이며 CPY-196이 접근 가능한 이름에 들어간다 | 로그아웃과 같은 항목형이다. 로그인 화면은 밝은 바탕이라 Link 규칙을 쓴다 | Accepted |

도움말 아이콘은 공통 규칙(`viewBox="0 0 16 16"`, `currentColor`, 선 굵기 `spacing.icon-stroke`, 둥근 끝·꺾임, `aria-hidden="true"`·`focusable="false"`)으로 직접 그린다: 도움말(원 안 물음표), 새 탭(오른쪽 위 화살표와 열린 상자), 목차(점 셋과 가로줄 셋), 복사(겹친 사각형 둘), 이전·다음(왼쪽·오른쪽 꺾쇠). 이전·다음은 상세 아이콘(오른쪽 꺾쇠)과 그 반전이다. 글씨 레이블을 항상 함께 둔다. mdBook이 싣는 Font Awesome 아이콘은 이 사이트의 조작 요소에 보이지 않게 한다. 이 프로젝트의 조작 요소는 위 아이콘만 쓴다.

도움말 사이트는 라이트 테마만 쓴다. mdBook의 테마 선택·검색·인쇄·목차 폭 조절 손잡이·Font Awesome 메뉴 막대는 쓰지 않거나 보이지 않게 한다. mdBook의 전환 효과는 끈다([모션](#모션)). 포커스 링은 상단 막대 안에서 `navbar-ring`, 밖에서 `ring`이다.


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

화면별 예외는 범위·이유·재검토 조건을 함께 적는다. 아래 세 항목은 [도움말 사이트 확장](#도움말-사이트-확장)의 예외다. 대시보드 화면 전용 예외는 없다.

| 범위 | 예외 | 이유 | 재검토 조건 |
| --- | --- | --- | --- |
| 도움말 사이트(HLP-001~003) | 중단점이 1080px(`@media (max-width: 1079.98px)`)다. 대시보드는 1024px다 | mdBook 0.5.4 템플릿의 인라인 스크립트가 폭 1080px로 목차의 처음 상태를 정한다. 그 파일은 수정하지 않고 싣는다. 다른 값을 쓰면 1024~1079px에서 목차 상태와 배치가 어긋난다 | mdBook 버전을 바꾸거나 템플릿을 직접 관리하게 되면 1024px로 맞출지 다시 정한다 |
| 도움말 사이트 본문의 표·코드 블록 | 표를 카드 목록으로 바꾸지 않고 코드 줄을 줄바꿈하지 않는다. 둘 다 자기 영역 안에서만 가로로 스크롤한다. 페이지 가로 스크롤은 여전히 없다 | 문서 표는 Markdown에서 만들어져 카드 변환 규칙에 쓸 열 구조 정보가 없다. 코드 줄을 바꾸면 명령·설정이 바뀐 것처럼 보인다 | 넓은 표가 읽기 어렵다는 확인이 나오면 표를 나누는 작성 규칙을 [도움말 작성 규칙](../help/AGENTS.md)에 더한다 |
| 도움말 사이트 데스크톱 목차 | 목차 열의 항목 목록이 화면 안에 남는다(sticky) | 목차는 본문과 다른 열이라 본문과 키보드 초점을 가리지 않는다. [접근성 기준](#접근성-기준)의 포커스 가림 항목은 본문 위에 겹치는 머리글·바닥글을 막는 규칙이다 | 목차가 본문 위에 겹치는 배치로 바뀌면 다시 정한다 |

### 검증

- 실행 절차: [대시보드 프런트엔드를 검증하는 방법](../how-to/verify-dashboard-frontend.md)(Draft)이 정한다.
- 통과 조건: lint 오류 0건. 대비는 [대비 기준](#대비-기준) 표의 기준 이상이다.
- 생성 산출물 대조: `cargo xtask tokens --check`가 이 문서의 YAML에서 만든 CSS를 저장된 CSS 파일과 시안 토큰 블록에 대조한다. 절차와 방법은 [프런트엔드 컨벤션 2절](../conventions/frontend.md#2-토큰-적용과-재사용)이 정한다.
- 자동 검증 결과: `cargo xtask scenarios`가 생성하는 `scenario-report.md`와 CI 실행 결과에서 확인한다([테스트 컨벤션](../conventions/testing.md)).
- 시각 검토 범위: SCR-001~006, 도움말 사이트, 대시보드 도움말 링크, 트레이 아이콘. 데스크톱·모바일 폭에서 관련 기준을 대조한다.

문서 검사만으로 실제 화면의 접근성과 시각 품질이 검증되지는 않는다. 시안만 확인한 결과는 구현 검증의 증거가 아니다. 아래 표는 수동 대조와 별도 브라우저 확인의 최신 기록이다. 에이전트 대조는 사람의 비교 판정을 대신하지 않는다.

| 확인 대상 | 대상 버전·범위 | 날짜·환경 | 방법 | 실제 결과·증거 |
| --- | --- | --- | --- | --- |
| 설정 화면 브라우저 호환성 | 커밋 `7ce077f`과 같은 소스. 로그인·설정 보기·편집·변경 확인·적용·서버 진단·로그아웃 | 2026-11-08. Windows. Chrome 157.0.8103.38·Edge 156.0.4314.58·Firefox 157.0·WebKit 27.2 | 1280·390px × ko·en, 실제 키 입력으로 별도 실행 | 통과(16회). 단계마다 가로 넘침 0. WebKit은 Safari 대체다. 실제 Safari·실기기·스크린 리더·사람 비교는 미실행이다. [검증 how-to](../how-to/verify-dashboard-frontend.md#4-검증) |
| 로그 화면 구현과 시안 대조 | 기준 `f001385` 위 작업 트리. 구현은 커밋 `3df0693`에 포함. SCR-001·002·004·005·006 시안 115개 프레임과 세 항목 상단 막대 | 2026-11-08. Windows, Chrome 157.0.8103.38 | 실제 제품 모듈·CSS와 계약 모양 가짜 API로 1280·390px에서 230쌍·양쪽 PNG 460장 대조 | 통과(확인 범위 한정). 차이 34건 중 구현 결함 15건 수정. 스펙·디자인 우선 차이 17건과 예시 데이터 차이 2건 유지. 구현 가로 넘침·CSP 위반·페이지 오류 0건. 콘솔 오류는 favicon 404 한 건. 에이전트 대조이며 사람 비교는 미실행이다. [검증 how-to](../how-to/verify-dashboard-frontend.md#4-검증) |
| 로그 화면 브라우저 호환성 | 로그 화면과 SCR-001·002·004·005 관련 변경의 실행 작업 트리 | 2026-11-09. Windows Chrome 157.0.8103.38·Edge 156.0.4314.58·WebKit 27.2, WSL Ubuntu Firefox 159.0 | 1280·390px × ko·en, 실제 키 입력으로 별도 실행 | 통과(16회). 240개 관찰점·720개 요소에서 가림·화면 밖·크기 0은 0건. 가로 넘침·페이지 오류 0건. WebKit 네이티브 select·캡처 도구 CSP 보고와 종료 확인 GET의 예상된 404는 별도 기록. WebKit은 Safari 대체다. 실제 Safari·실기기·스크린 리더·사람 비교·키보드 판정은 미실행이다. [검증 how-to](../how-to/verify-dashboard-frontend.md#4-검증) |
| 도움말 사이트 구현과 시안 대조 | 도움말 사이트 구현 작업 트리. [help-site.html](mockups/help-site.html) 21개 프레임(HLP-001~003) | 2026-11-12. Windows, 헤드리스 Chrome(CDP) | `cargo xtask help-serve`로 띄운 사이트를 같은 상태로 만들어 1080·400px(모바일 목차 펼침은 390px)에서 캡처·대조 | 통과(확인 범위 한정). 차이 5건 중 검색 단추 강조 1건 수정. 브라우저 기본 지우기 단추·본문 없는 절의 검색 결과·포인터 흔적·실제 내용 4건 유지. 에이전트 대조이며 사람 비교·다른 브라우저·실기기·스크린 리더·대비 재측정은 미실행이다. [도움말 게시 검증](../how-to/publish-help.md#4-검증) |
| 대시보드 도움말 링크 구현과 시안 대조(DASH-064) | 도움말 링크 구현 작업 트리. SCR-001 정상·SCR-003 프레임, ko·en | 2026-11-12. Windows, 헤드리스 Chrome | 1280·1024·1023·768·390·320px에서 24쌍 캡처·대조 | 통과(확인 범위 한정). 가로 넘침 0건. 모든 링크가 뷰포트 안이다. 차이는 로그인 제목과 예시 데이터다. 사람 비교는 미실행이다. [검증 how-to](../how-to/verify-dashboard-frontend.md#4-검증) |
| 최근 종료 카드 구현과 시안 대조(DASH-066) | 목록 정보 확대 구현 작업 트리. SCR-001 정상 프레임(ko·en)의 최근 종료 카드 | 2026-11-17. Windows, 에이전트가 조작한 Chromium 탭 | 가짜 API 시험 페이지와 시안을 1280·1024·390·320px에서 캡처·대조 | 통과(확인 범위 한정). 배지 줄바꿈·시각 열 폭·좁은 폭 날짜 끊김·머리글 줄바꿈·주소 레이블 위치 수정. en 1024px 주소 열 줄바꿈은 허용 기준과 일치한다. 예시 데이터 차이는 유지. 사람 비교는 미실행이다. [검증 how-to](../how-to/verify-dashboard-frontend.md#4-검증) |
| 대기·활성 카드 구현과 시안 대조(DASH-001·065) | 대기·활성 표 통일 작업 트리. SCR-001 정상 프레임(ko·en)의 대기 연결·활성 세션 카드 | 2026-11-17. Windows, 에이전트가 조작한 Chromium 탭 | 가짜 API 시험 페이지의 대기·활성 카드 16장과 시안 8장을 1280·1024·390·320px에서 캡처·대조 | 통과(확인 범위 한정). 대기 접속 시각·활성 시작 시각은 날짜 아래 시각 한 줄. 활성 주소 레이블은 값 위 줄. 스펙을 따른 활성 주소 레이블 문구와 시안 차이는 유지. 사람 비교는 미실행이다. [검증 how-to](../how-to/verify-dashboard-frontend.md#4-검증) |
| 트레이 아이콘의 브랜드 표식 대조 | 트레이 아이콘 구현 작업 트리. `crates/tray/src/ui.rs`의 `brand_mark_rgba`, 32×32 RGBA | 2026-11-18. Windows | 임시 단위 시험으로 RGBA 추출. 8배 확대한 PNG와 상단 막대 `.navbar-mark` CSS 값(32px·반지름 8px·`#1c2723`·20px `brand` 경로·선 1.75)으로 만든 SVG 렌더링을 육안 비교. 2×2 평균으로 줄인 16px도 확인 | 통과(확인 범위 한정). 모서리 투명, 바탕 `#1c2723`, 화살표 흰색. 화살표 두 개의 위치·방향·두께가 기준 SVG와 일치. 16px에서도 구분된다. 실제 알림 영역 표시와 Windows 자체 축소는 범위 밖이다. 임시 시험과 그림은 저장소에 남기지 않았다 |
| 트레이 아이콘 Windows 그리기 | Release `v2026.11.6`의 브랜드 표식 32×32 RGBA | 2026-11-18. Windows 11, tray-icon 0.26.1과 같은 변환 방식 | RGBA를 BGRA로 바꾸고 픽셀마다 AND 마스크 바이트 하나를 만들어 `CreateIcon`에 전달. `DrawIconEx`로 밝은 바탕·어두운 바탕(`#202020`)에 16·20·24·32px로 그림. 설치 뒤 작업 표시줄 오른쪽 끝을 클릭 없이 캡처 | 통과(확인 범위 한정). 밝은 가장자리 테두리 없음. 16px에서도 두 화살표 구분. 어두운 바탕에서 사각형 테두리는 거의 구분되지 않고 흰 화살표는 또렷하다. 실제 알림 영역 표시는 보지 못했다. 아이콘은 숨김 목록 안에 있고 Windows 저장 스냅숏은 확인 시점에 이전 그림이었다. 그림과 스크립트는 저장소에 남기지 않았다. [수동 운영 검증](../how-to/operate-relay.md#4-검증) |

### 미결정 사항

열린 항목은 없다.
