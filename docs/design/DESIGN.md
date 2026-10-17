---
version: alpha
name: "VNC Relay 운영 대시보드"
description: "정보 밀도를 우선하는 운영 도구용 디자인 기준 초안(Draft). 사용자 승인 전 제안이다."
colors:
  bg: "#f4f5f7"
  surface: "#ffffff"
  surface-muted: "#eceef2"
  text: "#1b1f24"
  text-muted: "#4a5260"
  border: "#d0d5dd"
  border-strong: "#7d8696"
  primary: "#1d4ed8"
  on-primary: "#ffffff"
  primary-hover: "#1e40af"
  danger: "#b42318"
  on-danger: "#ffffff"
  danger-hover: "#912018"
  focus: "#1d4ed8"
  success: "#0f6b2f"
  success-bg: "#e3f4e8"
  info: "#1d4ed8"
  info-bg: "#e6edff"
  warning: "#8a4600"
  warning-bg: "#fff1d6"
  danger-bg: "#fdebe9"
  neutral: "#4a5260"
  neutral-bg: "#eceef2"
  scrim: "#0f131880"
  # 다크 테마(제안, DESIGN-TBD-002). 접두사 dark-를 뗀 이름의 값을 다크에서 대체한다.
  dark-bg: "#0f1318"
  dark-surface: "#171c23"
  dark-surface-muted: "#202731"
  dark-text: "#e8ebf0"
  dark-text-muted: "#a6aebb"
  dark-border: "#303844"
  dark-border-strong: "#7f8999"
  dark-primary: "#8fb0ff"
  dark-on-primary: "#0b1020"
  dark-primary-hover: "#b0c7ff"
  dark-danger: "#ff9a90"
  dark-on-danger: "#2a0a07"
  dark-danger-hover: "#ffb8b0"
  dark-focus: "#9db9ff"
  dark-success: "#6fd58f"
  dark-success-bg: "#12301d"
  dark-info: "#9db9ff"
  dark-info-bg: "#16264d"
  dark-warning: "#f2b45c"
  dark-warning-bg: "#3a2a0b"
  dark-danger-bg: "#3d1512"
  dark-neutral: "#a6aebb"
  dark-neutral-bg: "#202731"
typography:
  heading-page:
    fontFamily: "system-ui, -apple-system, 'Segoe UI', 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', sans-serif"
    fontSize: 20px
    fontWeight: 600
    lineHeight: 1.3
  heading-section:
    fontFamily: "system-ui, -apple-system, 'Segoe UI', 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', sans-serif"
    fontSize: 16px
    fontWeight: 600
    lineHeight: 1.4
  body:
    fontFamily: "system-ui, -apple-system, 'Segoe UI', 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', sans-serif"
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.5
  body-numeric:
    fontFamily: "system-ui, -apple-system, 'Segoe UI', 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', sans-serif"
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.5
    fontFeature: "'tnum'"
  label:
    fontFamily: "system-ui, -apple-system, 'Segoe UI', 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', sans-serif"
    fontSize: 12px
    fontWeight: 500
    lineHeight: 1.4
  button:
    fontFamily: "system-ui, -apple-system, 'Segoe UI', 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', sans-serif"
    fontSize: 14px
    fontWeight: 500
    lineHeight: 1.2
  input:
    fontFamily: "system-ui, -apple-system, 'Segoe UI', 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', sans-serif"
    fontSize: 16px
    fontWeight: 400
    lineHeight: 1.5
  data:
    fontFamily: "ui-monospace, 'SF Mono', 'Cascadia Mono', Consolas, 'Liberation Mono', monospace"
    fontSize: 13px
    fontWeight: 400
    lineHeight: 1.4
    fontFeature: "'tnum'"
  metric:
    fontFamily: "system-ui, -apple-system, 'Segoe UI', 'Apple SD Gothic Neo', 'Malgun Gothic', 'Noto Sans KR', sans-serif"
    fontSize: 18px
    fontWeight: 600
    lineHeight: 1.2
    fontFeature: "'tnum'"
spacing:
  xs: 4px
  sm: 8px
  md: 12px
  lg: 16px
  xl: 24px
  2xl: 32px
  control-height: 32px
  control-height-touch: 44px
  target-min: 24px
  content-max: 1440px
  border-width: 1px
  focus-ring-width: 2px
  focus-ring-offset: 2px
rounded:
  control: 4px
  surface: 6px
  badge: 999px
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    typography: "{typography.button}"
    rounded: "{rounded.control}"
    padding: "{spacing.md}"
    height: "{spacing.control-height}"
  button-primary-hover:
    backgroundColor: "{colors.primary-hover}"
    textColor: "{colors.on-primary}"
  button-secondary:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    typography: "{typography.button}"
    rounded: "{rounded.control}"
    padding: "{spacing.md}"
    height: "{spacing.control-height}"
  button-secondary-hover:
    backgroundColor: "{colors.surface-muted}"
    textColor: "{colors.text}"
  button-danger:
    backgroundColor: "{colors.danger}"
    textColor: "{colors.on-danger}"
    typography: "{typography.button}"
    rounded: "{rounded.control}"
    padding: "{spacing.md}"
    height: "{spacing.control-height}"
  button-danger-hover:
    backgroundColor: "{colors.danger-hover}"
    textColor: "{colors.on-danger}"
  input:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    typography: "{typography.input}"
    rounded: "{rounded.control}"
    padding: "{spacing.sm}"
    height: "{spacing.control-height}"
  badge-success:
    backgroundColor: "{colors.success-bg}"
    textColor: "{colors.success}"
    typography: "{typography.label}"
    rounded: "{rounded.badge}"
    padding: "{spacing.xs}"
  badge-info:
    backgroundColor: "{colors.info-bg}"
    textColor: "{colors.info}"
    typography: "{typography.label}"
    rounded: "{rounded.badge}"
    padding: "{spacing.xs}"
  badge-warning:
    backgroundColor: "{colors.warning-bg}"
    textColor: "{colors.warning}"
    typography: "{typography.label}"
    rounded: "{rounded.badge}"
    padding: "{spacing.xs}"
  badge-danger:
    backgroundColor: "{colors.danger-bg}"
    textColor: "{colors.danger}"
    typography: "{typography.label}"
    rounded: "{rounded.badge}"
    padding: "{spacing.xs}"
  badge-neutral:
    backgroundColor: "{colors.neutral-bg}"
    textColor: "{colors.neutral}"
    typography: "{typography.label}"
    rounded: "{rounded.badge}"
    padding: "{spacing.xs}"
---

# VNC Relay 운영 대시보드 디자인 기준

## Overview

이 문서는 운영 대시보드의 공통 시각 기준을 정하는 디자인 시스템이다. `docs/design/DESIGN.md`에 둔다. 화면 동작은 [대시보드 화면 스펙](../specs/dashboard.md)이, 교환 형식은 [관리 API v1 계약](../contracts/management-api-v1.md)이 정한다. 이 문서는 동작과 문구를 새로 정하지 않고 색·글꼴·간격·배치만 정한다.

**이 문서는 초안(Draft)이다. 사용자 승인 전이며 모든 값은 제안이다.** 승인 전에는 구현의 기준으로 쓰지 않는다. 사용자가 2026-10-16에 디자인 기준 초안 작성을 요청했다. 승인 범위는 [미결정 사항](#미결정-사항)에서 관리한다.

| 항목 | 값 |
| --- | --- |
| 상태 | Draft |
| 적용 대상 | 운영 대시보드(SCR-001 운영 현황, SCR-002 세션 상세, SCR-003 관리자 로그인). 제안 범위는 라이트 테마와 다크 테마 |
| 채택 근거 | 없음. 사용자의 초안 작성 요청만 있고 승인된 값은 없다 |
| 적용 시작일 | 미정(승인 뒤 기록) |
| 관련 PRD·스펙 | [PRD](../requirements/PRD.md) FR-06·AC-31, [대시보드 화면 스펙](../specs/dashboard.md) DASH-015·DASH-023·DASH-TBD-004 |
| 관련 아키텍처 문서·ADR | 없음. 정적 HTML·JS 선택은 [PRD 확정 기준](../requirements/PRD.md#확정-기준)이 정한다 |
| 기준 시안 | [dashboard.html](mockups/dashboard.html). 초안 토큰으로 대표 화면을 확인한 시안이며 2026-10-17에 확인했다. 채택 뒤에는 채택된 토큰으로 다시 확인한다 |
| 토큰 원본 | 이 파일의 YAML 머리말 |
| 컴포넌트 미리보기 | 없음. 실제 컴포넌트가 없다 |
| 명세·도구 버전 | [DESIGN.md 명세](https://github.com/google-labs-code/design.md/blob/main/docs/spec.md) `version: alpha`(2026-10-16 확인). 검사 도구 버전은 [검증](#검증)에 기록한다 |

### 디자인 방향

- 대상 사용자와 주요 작업: 로그인한 관리자가 대기 연결·활성 세션·최근 종료 목록을 읽고, 한 세션을 골라 종료한다. 이 작업을 하는 화면은 [화면 스펙 1절](../specs/dashboard.md#1-범위와-사용자)을 따른다.
- 정보 밀도와 강조 방식: 운영 도구이므로 정보 밀도를 우선한다. 표와 값을 한 화면에 많이 보여주고, 장식을 줄인다. 강조는 상태 표시와 종료 행동에만 쓴다. 정상 값은 강조하지 않는다.
- 시각적 특징: 무채색 바탕과 하나의 파란색 행동 색을 쓴다. 그림자와 그라데이션을 쓰지 않는다. 표면은 1px 테두리로 구분한다. 이유는 장시간 모니터링에서 눈의 피로를 줄이고, 상태 색이 눈에 띄게 하려는 것이다.
- 참고 자료: [WCAG 2.2](https://www.w3.org/TR/WCAG22/) 대비·포커스·대상 크기 기준을 따른다. 외부 디자인 시스템의 시각 요소는 가져오지 않는다.
- 대표 화면: [dashboard.html](mockups/dashboard.html)의 SCR-001(긴 표 7열, 빈 목록·오류·갱신 중 상태), SCR-002(종료 확인 대화상자, 긴 주소), SCR-003(입력 폼). 초안 토큰을 검증하는 화면이다. 확인할 시나리오는 [대시보드 화면 스펙](../specs/dashboard.md)의 DASH-001~003·DASH-009·DASH-015·DASH-018·DASH-023이다. 채택 뒤에는 채택된 토큰으로 다시 확인한 시안을 연결한다.

### 정적 파일 원칙

대시보드는 별도 빌드 단계 없는 정적 HTML·JS다([화면 스펙 1절](../specs/dashboard.md#1-범위와-사용자)). 따라서 다음을 지켜야 한다.

- 글꼴·스타일·스크립트·아이콘을 외부에서 불러오지 않는다. CDN, 웹 글꼴 서비스, `@import url(...)`, 아이콘 글꼴을 쓰지 않는다.
- 모든 CSS와 이미지는 실행파일에 포함한 정적 파일이어야 한다.
- 아이콘이 필요하면 텍스트 레이블 옆에 CSS 도형이나 인라인 SVG를 쓴다. 아이콘만으로 뜻을 전하지 않는다.
- 토큰은 CSS 사용자 정의 속성(`--*`)으로 한 곳에서 정의한다. 아래 [이름 규칙](#css-사용자-정의-속성-이름-규칙)을 따른다.

## Colors

색 값의 원본은 YAML 머리말이다. 본문은 토큰 이름과 사용 조건을 쓴다. 상태 이름은 [관리 API v1 계약](../contracts/management-api-v1.md)과 [화면 스펙 4절](../specs/dashboard.md#4-화면-상태와-갱신)의 이름을 따른다.

### CSS 사용자 정의 속성 이름 규칙

| YAML 위치 | CSS 속성 이름 | 예 |
| --- | --- | --- |
| `colors.<이름>` | `--color-<이름>` | `colors.surface-muted` → `--color-surface-muted` |
| `colors.dark-<이름>` | 다크 규칙 안의 `--color-<이름>` | `colors.dark-surface` → 다크에서 `--color-surface` 대체 |
| `spacing.<이름>` | `--space-<이름>` | `spacing.control-height` → `--space-control-height` |
| `rounded.<이름>` | `--radius-<이름>` | `rounded.control` → `--radius-control` |
| `typography.<단계>.fontFamily·fontSize·fontWeight·lineHeight·fontFeature` | `--type-<단계>-family·size·weight·line·feature` | `typography.body.fontSize` → `--type-body-size` |

값을 CSS에서 직접 쓰지 않고 `var(--…)`로 참조한다. 이 규칙은 YAML에서 CSS를 기계적으로 만들 수 있게 한다. 생성 방법과 대조는 프런트엔드 컨벤션에서 정한다(DESIGN-TBD-005).

### 색 토큰과 사용 조건

| 토큰 | 역할 | 사용 조건 | 함께 쓰는 토큰·표시 |
| --- | --- | --- | --- |
| `colors.bg` | 페이지 바탕 | 화면 전체 바탕 | `colors.surface` 패널이 위에 놓인다 |
| `colors.surface` | 패널·입력·대화상자 바탕 | 표·카드·폼의 기본 표면 | `colors.text`·`colors.text-muted` |
| `colors.surface-muted` | 표 머리글·보조 영역 바탕 | 표 머리글, 요약 값 띠. 본문 행의 줄무늬로 쓰지 않는다 | `colors.text`·`colors.text-muted` |
| `colors.text` | 본문·값 | 모든 값과 제목 | `colors.surface`·`colors.bg`·`colors.surface-muted` |
| `colors.text-muted` | 보조 글씨 | 열 머리글, 단위, 보조 설명. 12px 이상에서만 쓴다 | `colors.surface`·`colors.bg`·`colors.surface-muted`에서 4.5:1 이상 |
| `colors.border` | 장식 구분선 | 패널 테두리, 표 행 구분선. 정보를 전달하지 않는 선만 해당한다 | 3:1을 요구하지 않는다 |
| `colors.border-strong` | 조작 요소 경계 | 입력 상자, 보조 버튼의 테두리 | `colors.surface`·`colors.bg`에서 3:1 이상 |
| `colors.primary` / `colors.on-primary` / `colors.primary-hover` | 주요 행동 | 로그인 버튼(주요 버튼 변형)과 세션 식별자 링크(글씨 색). 한 영역에 주요 버튼은 하나만 둔다 | 링크는 밑줄을 함께 쓴다 |
| `colors.danger` / `colors.on-danger` / `colors.danger-hover` | 위험한 행동 | 세션 종료 버튼과 종료 확인 버튼에만 쓴다 | 버튼에 "종료" 글씨를 쓴다 |
| `colors.focus` | 포커스 표시 | 모든 조작 요소의 `:focus-visible` 윤곽선 | `spacing.focus-ring-width`·`spacing.focus-ring-offset` |
| `colors.scrim` | 대화상자 뒤 가림막 | 모달 대화상자가 열린 동안 | 반투명 값. 대비 기준 대상이 아니다 |

### 상태 색

상태 표시는 색만으로 구분하지 않는다([화면 스펙 6절](../specs/dashboard.md#6-화면-크기와-접근성)). 모든 상태는 **글씨 레이블 + 표시 도형 + 색**으로 나타낸다. 도형은 성공·정보는 원, 경고는 삼각형, 위험은 마름모, 중립은 가로선이다. 한 상태의 글씨와 배경은 같은 이름의 `<상태>`·`<상태>-bg` 한 쌍을 쓴다.

| 상태 색 | 토큰 쌍 | 대상 상태 | 설명 |
| --- | --- | --- | --- |
| 성공 | `colors.success` / `colors.success-bg` | 서버 실행 상태 `ready`(준비됨) | 평소 값은 배지를 붙이지 않는다. 서버 실행 상태만 배지로 보인다 |
| 정보 | `colors.info` / `colors.info-bg` | 최초 로딩·갱신 중·종료 요청 중 | 진행 중임을 나타낸다. 행동을 막지 않는 경우 작은 배지로 쓴다 |
| 경고 | `colors.warning` / `colors.warning-bg` | 서버 실행 상태 `shutdown_grace`(종료 유예 중), 세션 상태 `closing`(종료 중), 오래된 성공 데이터, 종료 결과 불명, 종료 원인 `dead_connection`·`close_timeout` | 주의가 필요하지만 실패는 아니다 |
| 위험 | `colors.danger` / `colors.danger-bg` | 조회 실패, 종료 실패, 로그인 실패 안내, 종료 원인 `connection_error` | 실패를 뜻한다. 빈 목록과 구분한다 |
| 중립 | `colors.neutral` / `colors.neutral-bg` | 활성 세션 모드·전송 경로·역할, 확인 불가 값, 대상 없음, 종료 원인 `peer_eof`·`admin_terminated`·`server_shutdown`, 목록에 없는 알 수 없는 값 | 알 수 없는 값은 받은 그대로 보여주고 오류 색을 쓰지 않는다([관리 API 계약](../contracts/management-api-v1.md) 3절) |

세션 상태 `active`는 배지 없이 표시한다. 정상 상태를 모두 강조하면 예외가 보이지 않기 때문이다. 이 매핑은 시각 표현 제안이며 상태의 의미는 스펙과 계약이 정한다.

### 테마

- 기본은 라이트 테마다. 라이트 값은 `colors.<이름>`이다.
- 다크 테마는 제안이다(DESIGN-TBD-002). 채택하면 `prefers-color-scheme: dark`에서 `colors.dark-<이름>` 값으로 같은 CSS 속성을 대체한다. 수동 전환 버튼은 두지 않는다. 버튼은 설정 저장이라는 새 동작이 필요하다.
- 다크를 채택하지 않으면 `colors.dark-*` 토큰을 지운다.

### 대비 기준

기준은 WCAG 2.2 AA다. 일반 글씨는 대비 4.5:1 이상, 조작 요소의 경계와 포커스 표시는 3:1 이상이어야 한다. 아래 값은 2026-10-16에 YAML의 값에서 계산했다([검증](#검증)). 상태 쌍은 배지 배경과 `colors.surface`·`colors.bg` 위의 글씨를 함께 확인했다.

| 전경 | 배경 | 기준 | 라이트 | 다크(제안) |
| --- | --- | --- | --- | --- |
| `text` | `surface` | 4.5 | 16.56 | 14.32 |
| `text` | `bg` | 4.5 | 15.18 | 15.60 |
| `text` | `surface-muted` | 4.5 | 14.26 | 12.59 |
| `text-muted` | `surface` | 4.5 | 7.87 | 7.66 |
| `text-muted` | `bg` | 4.5 | 7.22 | 8.34 |
| `text-muted` | `surface-muted` | 4.5 | 6.78 | 6.73 |
| `primary`(링크) | `surface` | 4.5 | 6.70 | 8.01 |
| `primary`(링크) | `bg` | 4.5 | 6.14 | 8.72 |
| `primary`(링크) | `surface-muted` | 4.5 | 5.77 | 7.04 |
| `on-primary` | `primary` | 4.5 | 6.70 | 8.86 |
| `on-primary` | `primary-hover` | 4.5 | 8.72 | 11.23 |
| `on-danger` | `danger` | 4.5 | 6.57 | 8.98 |
| `on-danger` | `danger-hover` | 4.5 | 8.66 | 11.12 |
| `border-strong` | `surface` | 3 | 3.67 | 4.84 |
| `border-strong` | `bg` | 3 | 3.36 | 5.27 |
| `focus` | `surface` | 3 | 6.70 | 8.82 |
| `focus` | `bg` | 3 | 6.14 | 9.60 |
| `focus` | `surface-muted` | 3 | 5.77 | 7.75 |
| `success` | `success-bg` | 4.5 | 5.81 | 7.90 |
| `info` | `info-bg` | 4.5 | 5.72 | 7.63 |
| `warning` | `warning-bg` | 4.5 | 6.36 | 7.56 |
| `danger` | `danger-bg` | 4.5 | 5.71 | 7.82 |
| `neutral` | `neutral-bg` | 4.5 | 6.78 | 6.73 |

상태 색 5개(`success`·`info`·`warning`·`danger`·`neutral`)는 `surface`와 `bg` 위에서도 4.5:1 이상이다(최소 라이트 6.03, 다크 7.66). 새 색을 추가하면 이 표에 행을 추가하고 다시 계산해야 한다.

## Typography

글꼴 값의 원본은 YAML이다. 본문 글꼴은 사용자 기기에 있는 시스템 글꼴이다. 외부 글꼴은 쓰지 않는다.

| 역할 | 토큰 | 적용 대상 | 콘텐츠 처리 |
| --- | --- | --- | --- |
| 화면 제목 | `typography.heading-page` | 화면 이름(운영 현황, 세션 상세, 관리자 로그인) | 좁은 화면에서 줄바꿈한다. 말줄임으로 정보를 숨기지 않는다 |
| 절 제목 | `typography.heading-section` | 대기 연결·활성 세션·최근 종료 목록의 패널 제목 | 같음 |
| 본문 | `typography.body` | 설명, 오류·안내 문장 | 한글은 단어 단위로 줄바꿈한다(`word-break: keep-all`) |
| 숫자 본문 | `typography.body-numeric` | 개수, 시각, 전송량, 시간 길이 | 자릿수가 맞는 숫자(`tnum`). 숫자 열은 오른쪽 정렬한다 |
| 레이블 | `typography.label` | 열 머리글, 입력 레이블, 요약 항목 이름, 배지 | 12px 아래로 줄이지 않는다 |
| 버튼 | `typography.button` | 모든 버튼 | 한 줄. 좁은 폭에서 줄바꿈이 필요하면 버튼을 전체 폭으로 만든다 |
| 입력 | `typography.input` | 계정명·비밀번호 입력 | 모든 폭에서 16px 이상이다. 모바일 브라우저가 입력 때 화면을 확대하지 않게 한다 |
| 값(고정 폭) | `typography.data` | 세션 식별자, 중계 ID, 상대 주소 | 긴 값은 `overflow-wrap: anywhere`로 줄바꿈한다. 값을 잘라 표시하지 않는다 |
| 요약 숫자 | `typography.metric` | 요약 띠의 개수·사용량 | 자릿수가 맞는 숫자 |

- 글꼴 자산과 사용 권한: 별도 파일이 없다. 사용자 기기의 시스템 글꼴을 쓴다.
- 글꼴 로딩과 대체: 로딩 단계가 없다. `typography.*.fontFamily`의 순서대로 대체하며, 마지막은 일반 계열(`sans-serif`·`monospace`)이다. 한글은 `Apple SD Gothic Neo`·`Malgun Gothic`·`Noto Sans KR` 순으로 대체한다.
- UI 문구 규칙: 화면 문구는 [화면 스펙](../specs/dashboard.md)이 정한다. 문구 확정은 DASH-TBD-003이 관리한다. 이 문서는 문구를 정하지 않는다.

### 표시 형식(제안)

값의 의미와 단위는 [화면 스펙 3절](../specs/dashboard.md#시각전송량-표시-형식)이 정한다. 아래는 그 스펙이 남긴 자릿수·전환 경계의 제안이다(DESIGN-TBD-004). 승인 전에는 확정이 아니다.

| 값 | 제안 형식 | 예 |
| --- | --- | --- |
| 전송량 | KiB와 MiB만 쓴다. 소수 첫째 자리까지 반올림해 표시한다. KiB 값이 반올림 뒤 1024.0 이상이면 MiB로 바꾼다. 자릿수 구분 기호는 브라우저 언어 설정을 따른다. 1 GiB 이상도 MiB로 쓴다 | `0`→`0.0 KiB`, `1536`→`1.5 KiB`, `1048575`→`1.0 MiB`, `1572864`→`1.5 MiB` |
| 시각 | 브라우저 현지 시각 `YYYY-MM-DD HH:mm:ss`와 시간대 약어를 함께 쓴다 | `2026-10-16 14:03:25 GMT+9` |
| 시간 길이(가동·대기) | `D일 HH:MM:SS` 형식이다. 일수가 0이면 일 부분을 뺀다 | `1일 02:03:04`, `00:00:07` |
| 확인 불가 값 | 글씨 "확인 불가"와 중립 상태 색을 쓴다. 0이나 빈 칸으로 쓰지 않는다 | 연결 수 `null` |

## Layout

- 공통 콘텐츠 폭: 화면 내용은 `spacing.content-max` 이하로 가운데에 둔다. 좁은 화면에서는 전체 폭을 쓴다.
- 바깥 여백과 간격: 간격은 `spacing.xs`~`spacing.2xl` 척도만 쓴다. 데스크톱 폭의 페이지 바깥 여백은 `spacing.xl`, 패널 사이는 `spacing.lg`, 표 칸 안쪽은 세로 `spacing.sm`·가로 `spacing.md`다. 모바일 폭의 페이지 바깥 여백은 `spacing.md`다.
- 밀도 변형: 없음. 하나의 밀도만 쓴다. 모바일 폭에서는 조작 요소 높이만 `spacing.control-height-touch`로 키운다.
- 콘텐츠 경계 조건: 긴 중계 ID·IPv6 주소·세션 식별자는 줄바꿈해 모두 보여준다. 빈 목록은 스펙의 빈 목록 안내를 표 자리에 보이고 열 머리글은 유지한다. 알 수 없는 상태 값은 받은 그대로 보여준다.

### 중단점과 모바일 배치

화면 동작은 [화면 스펙](../specs/dashboard.md)을 따른다. 이 절은 배치 기준만 정한다(제안, DESIGN-TBD-003).

| 이름 | 조건 | 적용 |
| --- | --- | --- |
| 모바일 폭 | 뷰포트 폭 1023.98px 이하(`@media (max-width: 1023.98px)`) | 표를 카드 목록으로 바꾼다. 조작 요소 높이는 `spacing.control-height-touch`다 |
| 데스크톱 폭 | 뷰포트 폭 1024px 이상 | 표를 유지한다. 조작 요소 높이는 `spacing.control-height`다 |

중단점은 하나만 둔다. 값 1024px는 시안 확인에서 정했다. 활성 세션 7열 표는 가장 긴 주소(IPv6와 포트)와 3,072.0 MiB를 넣었을 때 내용 폭 약 845px가 필요했다. 768px에서는 상대 주소·상세 열이 패널 밖으로 잘려 보이지 않았다. CSS 미디어 쿼리에는 사용자 정의 속성을 쓸 수 없으므로 값 1024px를 CSS 한 곳에서만 쓴다. 최소 지원 폭은 320px이다.

| 구성 | 데스크톱 폭 | 모바일 폭 |
| --- | --- | --- |
| 머리 줄(제목·로그아웃·마지막 성공 갱신·새로고침) | 한 줄. 제목 왼쪽, 갱신 정보·버튼 오른쪽 | 줄바꿈한다. 제목 줄 아래에 갱신 시각, 그 아래에 버튼 줄을 둔다. 조작 요소 사이는 `spacing.sm` |
| 요약 값(실행 상태·가동 시간·대기·활성·초기 교환 중·Mode I 대상 연결 중·L·Lm) | 격자. 한 칸 최소 폭 140px, 남는 폭은 균등 분배. 두 폭 공통이다 | 같음. 320px에서 2열이 된다. 칸 안의 레이블은 줄바꿈한다 |
| 대기 연결 표 | 4열 표(역할·중계 ID·접속 시각·대기 시간) | 행마다 카드 하나. 카드 첫 줄에 중계 ID와 역할 배지, 아래에 레이블·값 쌍 |
| 활성 세션 표 | 7열 표(세션·모드·전송 경로·시작·방향별 바이트·상대 주소·상세) | 행마다 카드 하나. 첫 줄에 세션 식별자 링크와 [상세] 버튼, 아래에 레이블·값 쌍(모드, 전송 경로, 시작, 방향별 바이트 2줄, 상대 주소). [상세]의 높이는 `spacing.control-height-touch` |
| 최근 종료 표 | 4열 표(세션·종료 원인·종료 시각·방향별 바이트) | 행마다 카드 하나. 첫 줄에 세션 식별자와 종료 원인 배지, 아래에 레이블·값 쌍 |
| 세션 상세 | 레이블·값 두 열 목록. [세션 종료]는 오른쪽 아래 | 레이블·값 한 열 목록. [세션 종료]는 전체 폭 버튼으로 맨 아래 |
| 종료 확인 대화상자 | 최대 폭 480px, 가운데. 버튼은 가로로 [취소][종료 확인] | 뷰포트 폭에서 가로 여백 `spacing.md`씩 뺀 폭. 버튼은 세로로 쌓고 전체 폭이며 [취소]가 위에 온다 |
| 로그인 | 최대 폭 400px 카드, 가운데 | 전체 폭 카드. 입력과 [로그인] 버튼은 전체 폭 |

표를 카드로 바꾸는 규칙:

- 모바일 폭에서도 정보를 숨기거나 말줄임하지 않는다. 카드 첫 줄의 값(세션 식별자·중계 ID·역할·종료 원인·[상세])은 레이블 없이 제목처럼 보인다. 나머지 값은 같은 열 머리글 이름을 앞에 레이블로 보인다.
- 가로 스크롤 표를 쓰지 않는다. 320px 폭에서 페이지에 가로 스크롤이 생기면 안 된다(WCAG 1.4.10).
- 표의 `display`를 바꾸면 일부 브라우저가 표 의미를 잃는다. 카드 배치에서도 `table`·`row`·`cell`·`columnheader` 역할을 명시해 보조 기술이 표로 읽게 한다. 열 머리글은 화면에서만 감추고 접근성 트리에는 남긴다.
- 레이블을 CSS 가상 요소로 만들면 보조 기술이 레이블을 두 번 읽을 수 있다. 가상 요소의 대체 텍스트를 비운다.
- 카드 순서는 표의 행 순서와 같다. 정렬은 [관리 API 계약 1.1절](../contracts/management-api-v1.md)을 따르며 화면에서 바꾸지 않는다.

### 접근성 기준

적용 기준은 WCAG 2.2 AA다. 지원 브라우저는 최신 Chrome·Edge·Firefox·Safari이다([PRD](../requirements/PRD.md#확정-기준)). 동작 요구는 [화면 스펙 6절](../specs/dashboard.md#6-화면-크기와-접근성)이 정한다.

| 항목 | 검증 가능한 기준 | 근거 |
| --- | --- | --- |
| 글씨 대비 | 본문·상태 글씨 4.5:1 이상 | WCAG 1.4.3. [대비 기준](#대비-기준) 표 |
| 조작 요소 대비 | 입력 경계·포커스 표시 3:1 이상 | WCAG 1.4.11 |
| 포커스 표시 | 모든 조작 요소가 `:focus-visible`에서 `spacing.focus-ring-width` 두께의 `colors.focus` 윤곽선과 `spacing.focus-ring-offset` 간격을 가진다. `outline: none`을 대체 표시 없이 쓰지 않는다 | WCAG 2.4.7 |
| 포커스 가림 | 고정(sticky·fixed) 머리글·바닥글을 두지 않는다. 대화상자는 포커스 요소를 가리지 않는다 | WCAG 2.4.11 |
| 대상 크기 | 데스크톱 폭에서 조작 요소는 가로·세로 `spacing.target-min` 이상, 높이는 `spacing.control-height`. 모바일 폭에서 높이 `spacing.control-height-touch` | WCAG 2.5.8. 44px는 이 프로젝트의 제안 |
| 색 외 표시 | 상태는 글씨 레이블과 도형으로 구분한다. 링크는 밑줄을 쓴다 | WCAG 1.4.1 |
| 재배치 | 320px 폭에서 가로 스크롤 없이 모든 정보와 행동을 쓴다 | WCAG 1.4.10 |
| 글씨 확대 | 브라우저 확대를 200%로 키워도 정보와 행동이 가려지지 않는다. 데스크톱 폭 1280px의 200% 확대는 640px 폭과 같으므로 모바일 폭 배치가 적용된다 | WCAG 1.4.4 |
| 입력 레이블 | 모든 입력에 보이는 레이블을 둔다. 자리표시자만으로 대신하지 않는다 | WCAG 3.3.2 |

## Elevation & Depth

그림자를 쓰지 않는 평면 디자인이다. 층은 바탕색과 1px 테두리로 구분한다.

| 대상 | 토큰·스타일 기준 | 사용 조건 |
| --- | --- | --- |
| 페이지 | `colors.bg` | 가장 아래 층 |
| 패널·카드 | `colors.surface` + `spacing.border-width` `colors.border` 테두리 | 표, 카드, 폼, 요약 띠 |
| 대화상자 | `colors.surface` + `spacing.border-width` `colors.border-strong` 테두리 + 뒤에 `colors.scrim` | 종료 확인 대화상자 |

## Shapes

| 대상 | 토큰 | 적용 조건 |
| --- | --- | --- |
| 입력과 버튼 | `rounded.control` | 모든 크기 |
| 패널·카드·대화상자 | `rounded.surface` | 표를 감싸는 패널과 모바일 카드 |
| 배지 | `rounded.badge` | 상태 배지, 모드·전송 경로 배지 |

테두리 두께는 `spacing.border-width`, 포커스 윤곽선 두께는 `spacing.focus-ring-width`다.

## Components

첫 화면에 필요한 컴포넌트만 정한다. 실제 컴포넌트와 코드가 없어 구현·API·실행 예시는 정할 수 없다. 동작은 각 행의 스펙 시나리오를 따른다.

| 컴포넌트 | 변형 | 시각 상태 | 크기와 구성 | 관련 동작 |
| --- | --- | --- | --- | --- |
| Button | `button-primary`, `button-secondary`, `button-danger` | 기본·hover(`*-hover` 토큰)·focus-visible·disabled(투명도를 낮추지 않고 `colors.text-muted` 글씨와 `colors.surface-muted` 바탕)·진행 중(레이블을 진행 중 문구로 바꾸고 disabled) | 높이 `spacing.control-height`(모바일 `spacing.control-height-touch`), 가로 안쪽 `spacing.md`, 최소 폭은 레이블 폭. 보조 버튼은 `colors.border-strong` 테두리 | DASH-009~013·DASH-017~019 |
| Input | 텍스트·비밀번호 | 기본·focus-visible·오류 | 높이 `spacing.control-height`(모바일 `spacing.control-height-touch`), `typography.input`, `colors.border-strong` 테두리. 오류일 때 테두리를 `colors.danger`로 바꾸고 오류 문장을 입력 아래에 쓴다 | DASH-018 |
| Badge | `badge-success`·`badge-info`·`badge-warning`·`badge-danger`·`badge-neutral` | 정적(상호작용 없음) | `typography.label`, 가로 안쪽 `spacing.sm`·세로 `spacing.xs`. 도형 + 글씨 | DASH-001·DASH-003·DASH-020 |
| Banner | 정보·경고·위험·중립 | 정적 | `<상태>-bg` 바탕, `<상태>` 글씨, `<상태>` 색 `spacing.border-width` 테두리, 도형 + 글씨 | DASH-003·DASH-007·DASH-012·DASH-013 |
| Panel | 없음 | 정적 | `colors.surface`, 테두리, `rounded.surface`, 안쪽 `spacing.lg`. 제목은 `typography.heading-section` | DASH-002 |
| Data table | 표(데스크톱 폭)·카드 목록(모바일 폭) | 정적. 링크 hover·focus-visible | 머리글 `colors.surface-muted`·`typography.label`·`colors.text-muted`. 숫자 열은 오른쪽 정렬 | DASH-001·DASH-015 |
| Metric | 없음 | 정적 | 레이블 `typography.label`, 값 `typography.metric`. 확인 불가 값은 "확인 불가" 배지 | DASH-021 |
| Link | 세션 식별자 | 기본(밑줄)·hover·focus-visible | `colors.primary`, 밑줄, `typography.data` | DASH-006 |
| Dialog | 종료 확인 | 열림·닫힘 | `<dialog>` 모달 기준. 폭은 위 [배치](#중단점과-모바일-배치)를 따른다 | DASH-009 |

### 모션

모션을 쓰지 않는다. 전환 효과·스피너 애니메이션이 없다. 진행 중 상태는 글씨와 도형으로 표시한다. 따라서 움직임 감소 설정(`prefers-reduced-motion`)에 따른 별도 표현은 필요 없다. 모션을 추가하려면 이 표를 먼저 갱신한다.

## Do's and Don'ts

### 허용·금지 조합

| 대상 | 허용 예시 | 금지 예시 | 판정 이유 |
| --- | --- | --- | --- |
| 상태 표시 | 글씨 "갱신 중" + 정보 도형 + `info` 색 | 색만 다른 점 | 색을 구분하지 못하는 사용자가 상태를 알 수 없다 |
| 링크 | 밑줄 + `primary` 색 | 색만 다른 글씨 | 색 외 표시가 없다 |
| 모바일 표 | 카드 목록으로 바꾸고 모든 값을 보인다 | 가로 스크롤 표, 열 숨김, 말줄임 | 필수 정보가 가려진다([화면 스펙 6절](../specs/dashboard.md#6-화면-크기와-접근성)) |
| 확인하지 못한 값 | "확인 불가" 배지 | 0, 빈 칸, `-` | 0과 구분할 수 없다([화면 스펙 3절](../specs/dashboard.md#3-화면-구성과-와이어프레임)) |
| 포커스 | 토큰의 윤곽선 | `outline: none`만 적용 | 키보드 위치를 알 수 없다 |
| 위험 행동 | `button-danger`는 세션 종료·종료 확인에만 쓴다 | 목록 행마다 빨간 종료 버튼 | 종료는 SCR-002의 확인 절차로만 시작한다([화면 스펙 2절](../specs/dashboard.md#2-화면-흐름)) |
| 글꼴·자산 | 시스템 글꼴, 인라인 SVG | 웹 글꼴 서비스, CDN, 아이콘 글꼴 | 정적 파일만 쓴다 |
| 색 값 | `var(--color-…)` | 화면 CSS에 직접 쓴 `#rrggbb` | 토큰 원본이 둘이 된다 |

### 예외

현재 예외 없음. 화면별 예외는 `DESIGN-EX-001`부터 이 표에 범위·이유·재검토 조건과 함께 추가한다.

### 검증

- 실행 절차: 검증 how-to가 없다(DESIGN-TBD-005). 아래 기록은 이 초안 작성 때의 일회성 확인이다.
- 통과 조건: lint 오류 0건. 대비는 [대비 기준](#대비-기준) 표의 기준 이상이다.
- 생성 산출물 대조: 생성 절차가 없다. 프런트엔드 컨벤션 작성 때 정한다(DESIGN-TBD-005).
- 시각 검토 범위: 대표 화면 SCR-001·SCR-002·SCR-003, 데스크톱·모바일 폭, 라이트·다크.
- 실행 결과와 증거: 아래 표.

문서 검사만으로 실제 화면의 접근성과 시각 품질이 검증되지는 않는다. 아래 시안은 실제 컴포넌트가 아니므로 구현 검증의 증거가 아니다.

| 확인 대상 | 기준 문서·범위 | 환경·명령·방법 | 실제 결과 | 증거 |
| --- | --- | --- | --- | --- |
| 문서·토큰 구조 | 이 문서 YAML | Windows, Node 26.7.0, `npx -p @google/design.md designmd lint docs/design/DESIGN.md`(패키지 0.4.0) | 통과: 오류 0건, 경고 29건, 정보 1건. 경고는 모두 `orphaned-tokens`다. 컴포넌트 토큰이 직접 참조하지 않는 색(바탕·테두리·포커스·가림막·`dark-*`)이며 CSS가 직접 참조하므로 허용한다. 토큰 수는 색 47·글꼴 단계 9·모서리 3·간격 13·컴포넌트 12 | 로컬 실행 |
| 대비 | YAML 색 토큰 | Python 3.14.6. WCAG 상대 휘도 공식으로 [대비 기준](#대비-기준) 표의 23쌍과 상태 색 5개의 `surface`·`bg` 대비를 라이트·다크 모두 계산 | 통과: 기준 미달 0건. 최저값은 라이트 3.36(`border-strong`/`bg`, 기준 3), 본문 글씨 계열 최저 5.71 | 로컬 실행 |
| 대표 화면 시안 | [화면 스펙 4절](../specs/dashboard.md#4-화면-상태와-갱신)의 SCR-001·SCR-002·SCR-003 화면 상태 전체. 단일 파일 [dashboard.html](mockups/dashboard.html)에 21개 프레임(스펙 4절 표의 모든 화면 상태, 기본 상세, 종료 확인 대화상자, 최초 조회 실패 구분 포함) | 2026-10-17. Windows, Chromium 155 헤드리스, 시스템 글꼴. 로컬 HTTP 서버로 파일을 열었다. 폭 1280·1024·1023·768·390·320px, 라이트·다크(`prefers-color-scheme` 에뮬레이션) 12개 조합. 시안 CSS의 토큰 블록은 이 문서 YAML에서 이 문서의 [이름 규칙](#css-사용자-정의-속성-이름-규칙)대로 생성했다 | 통과(시안 한정): 12개 조합에서 페이지 가로 스크롤 0건, 프레임 밖으로 나온 요소 0건, 패널 안에서 잘린 표 0건(스크립트 측정). 버튼 높이는 1024px 이상 32px, 1023px 이하 44px, 입력 글씨 16px이다. Tab 포커스 윤곽선(2px 실선, 간격 2px, `focus` 색)은 한 곳, 카드 배치의 표 역할(접근성 트리 `table`·`row`·`cell`·`columnheader`)은 390px의 활성 세션 목록 한 곳에서 확인했다. CSS에 토큰 밖 색 값(`#rrggbb`·`rgb()`)과 정의되지 않은 `var()`가 0건이다. 길이 리터럴은 레이아웃 폭(140·400·480·1023.98px, 열·레이블 폭 `em` 몇 개)만 남았다. 외부 요청은 문서 자체 외에 없다(로컬 서버 기록, 브라우저의 자동 아이콘 요청 제외). 스크린샷 육안 검토는 일부 화면만 했다(1280px 라이트 3개·다크 2개, 390px 라이트 4개·다크 2개) | 스크린샷 검토. 실제 컴포넌트가 아니므로 구현 검증의 증거가 아니다 |
| 스크린 리더·Firefox·Safari·Edge·실제 모바일 기기 | 접근성·화면 호환성 | 해당 없음 | 미실행. 실제 UI가 없다. 시안은 Chromium만 열었다 | 없음 |

### 미결정 사항

| ID | 항목 | 영향 범위 | 책임자 | 확정 위치 |
| --- | --- | --- | --- | --- |
| DESIGN-TBD-001 | 색 방향 승인: 무채색 바탕·파란 `primary`·상태 색 매핑([상태 색](#상태-색)) | 전 화면 | 프로젝트 요구사항·디자인 결정 담당자 | 이 문서 |
| DESIGN-TBD-002 | 다크 테마 채택 여부와 방식(시스템 설정만 따를지) | 전 화면 | 같음 | 이 문서 |
| DESIGN-TBD-003 | 중단점 1024px와 표→카드 배치 승인([중단점과 모바일 배치](#중단점과-모바일-배치)) | SCR-001·SCR-002·SCR-003, DASH-TBD-004 | 같음 | 이 문서, [화면 스펙 8절](../specs/dashboard.md#8-미결정-사항) |
| DESIGN-TBD-004 | 전송량 자릿수·단위 전환 경계, 시각·시간 길이 표시 형식([표시 형식](#표시-형식제안)) | SCR-001·SCR-002, DASH-TBD-004 | 같음 | 이 문서, [화면 스펙 3절](../specs/dashboard.md#시각전송량-표시-형식) |
| DESIGN-TBD-005 | 프런트엔드 컨벤션과 검증 how-to(토큰 CSS 생성·대조, 미리보기 위치) 작성 | 구현 전체 | 같음 | [컨벤션 규칙](AGENTS.md), 검증 how-to |
