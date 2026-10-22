# 운영 현황 앱 구조 (SCR-001·SCR-002 자리)

상태: Active (작업 4.10 구현 완료). code-map과 dashboard-components.md 3.1절 반영은 15절에 남긴다

관련 문서:
- [화면 스펙](../specs/dashboard.md) 3·4·5·6·9절
- [관리 API 계약 v1](../contracts/management-api-v1.md) 1.3·1.4·1.6~1.12·2.2·2.3절
- [디자인 기준](../design/DESIGN.md) 화면 구성(522~534행), 중단점, 표시 형식(489~492행)
- [프런트엔드 규약](../conventions/frontend.md) 1.1·1.2·1.3·3·4절
- [시험 규약](../conventions/testing.md) 상태 규칙(23·99·100행)
- [컴포넌트 구조](dashboard-components.md) (현재 구조, 3.1절 함수 목록)

이 문서는 `web/dashboard/app/`의 파일 배치, 모듈 책임, 조회·갱신 흐름, 실패 처리, 로그아웃, 표시 형식을 정한다. 화면 문구는 스펙 9절이, 필드 의미는 계약이 정한다. 이 문서의 "구현 판단"은 사용자 결정이 아니다. 사용자 결정과 확인이 필요한 항목은 14절에 모은다.

## 1. 범위

- 4.10 구현 범위: SCR-001 전체, 앱 셸(진입점, 세션 확인, 해시 라우터, 언어 선택, 로그아웃).
- SCR-002는 TODO 배정대로 4.11에서 전체를 만든다(본문, 상세 조회, 종료 흐름). 4.10의 SCR-001 목록 링크는 계약 1.11절의 화면 URL `/#/sessions/{session_id}`를 그대로 쓴다.
- 해시(`#/sessions/…`)는 서버로 가지 않는다. 서버는 `/`와 같은 응답을 준다: 세션이 있으면 200 HTML, 없으면 303 `/login/`. 404 JSON이 아니다.
- 4.10 라우터는 SCR-002 경로를 처리하지 않는다. 임시 동작: 알 수 없는 해시는 SCR-001을 그대로 보인다. 임시 동작과 링크 처리 방식은 14절 Main 확인 항목이다.
- 이 문서가 정하지 않는 것: 관리 API 필드 의미(계약), 화면 문구(스펙 9절), 색과 간격 토큰(DESIGN.md).

## 2. 파일 배치

```text
web/dashboard/app/
├── index.html        신규. 제공 경로 `/` (frontend.md 1.1절). `/app/index.html`은 제공하지 않는다.
├── main.js           신규. 진입점. mountOperations(document.getElementById('app-root')) 호출
├── operations.js     신규. SCR-001 조립과 흐름: 세션 확인, 조회 루프, 세대(generation) 번호, 로그아웃, 언어 상태
├── model.js          신규. 응답 검증(계약 1.5~1.10절 필드). 맞지 않으면 `invalid`
├── panels.js         신규. 서버 요약 카드, 연결 카드, 목록 카드와 목록 상태 안내
├── api.js            신규. fetch 감싸기: 헤더, 시간 제한, 오류 분류
├── format.js         신규. 시각, 기간, 바이트, 숫자, 알 수 없는 값
├── copy.js           신규. 한국어·English 문구. 키는 CPY ID
├── language.js       이동. login/language.js 원본을 옮긴다. login 쪽은 생성 복사본이 된다 (구현 판단, 14절)
├── components/       기존 원본 13개 (login 복사본의 원본). data-table.js는 셀 값으로 DOM Node를 받도록 확장 (3.1절 대상)
├── tokens.css        기존. 이 작업에서 바꾸지 않는다
├── app.css           신규. SCR-001 화면 CSS (app 전용). 설계 초안의 ops.css 이름을 바꿨다
└── styles/
    ├── base.css      기존 원본
    └── components.css 기존 원본. 카드형 표 규칙을 추가하면 login 복사본을 재생성한다
```

규칙:
- `index.html`은 절대 경로(`/app/...`)로 파일을 가리킨다. 인라인 script, 인라인 `<style>`, `style` 속성, `on…` 속성, `javascript:` URL을 쓰지 않는다 (frontend.md 1.3절).
- `app/`과 `login/`은 파일을 공유하지 않는다 (스펙 L15, frontend.md 1.1절 L78). 같은 내용의 복사본만 `cargo xtask tokens`가 만든다. 복사 방향은 `app/`(원본)에서 `login/`(복사본)이다 (`xtask/src/tokens/shared.rs`).
- `xtask/src/tokens/shared.rs`의 `COPY_DIRS`(`components`, `styles`)는 login 쪽 복사 폴더만 검사한다. app 전용 파일은 `app/` 아래 어디에 두어도 `--check`에 걸리지 않는다.

## 3. 모듈 책임

| 파일 | 책임 | 테스트 주입 (deps) |
| --- | --- | --- |
| `main.js` | `mountOperations(document.getElementById('app-root'))`만 호출 | 없음 |
| `operations.js` | `mountOperations(root, deps)`: 세션 확인(`GET /api/v1/session`), 네 조회 루프, 세대 번호, 로그아웃, 언어 상태 | `request`(fetch 대체), `storage`, `preferredLanguages` |
| `model.js` | 응답 검증: `parseSession`·`parseSummary`·`parseWaiting`·`parseSessions`·`parseClosed`. 맞지 않으면 `invalid` 오류 | 없음 |
| `api.js` | 요청 헤더, 시간 제한(AbortController), 응답을 `ApiError`의 kind로 분류. 로그아웃은 204만 성공 | `request` 인자로 받음 |
| `panels.js` | SCR-001 카드 조립: 서버 요약, 연결 카드, 목록 카드 세 개와 목록 상태 안내 | 없음 |
| `format.js` | 시각·기간·바이트·숫자 표시 | 없음 (`Intl`만 사용) |
| `copy.js` | `ko`·`en` 문구 객체. 각 키에 CPY ID 주석 | 없음 |
| `language.js` | `LANGUAGES`, `chooseLanguage`, `readSavedLanguage`, `saveLanguage` (login과 같은 저장 키). login/language.js는 이 파일의 복사본 | `storage`, `preferredLanguages` |

사용할 기존 컴포넌트 (3.1절 함수, `app/components/`에 이미 있음): `createNavbar`, `createLanguageSelect`(variant `navbar`), `createButton`, `createCard`, `createStat`, `createGauge`, `createDataTable`, `createBadge`, `createAlert`, `createLink`, `icon`. SCR-002 단계(4.11)에서 `createDialog`.

## 4. 화면 골격 (SCR-001)

- 상단 막대: `createNavbar`. 브랜드 "VNC Relay", 항목 "운영 현황"(현재 항목), 오른쪽에 언어 선택과 [로그아웃]. `createNavbar`는 `label`을 필수로 받는다. label은 한국어 "주 메뉴", English "Main menu"다 (한국어: DESIGN 582행·프런트엔드 컨벤션 4절 206행, English: 시안 English 프레임). English 이름은 스펙 9.3.1절 문구 짝에 아직 없으므로 잠정이다 (14절 navbar, 15절 Q3). 모바일 폭(1023.98px 이하)에서는 브랜드 글자와 "운영 현황" 항목을 CSS로 숨기고 DOM에는 남긴다 (DESIGN 543행).
- 랜드마크: 상단 막대는 `nav`, 본문은 `main`이다 (DESIGN 582행). `nav`의 이름은 한국어 "주 메뉴", English "Main menu"다 (14절 navbar, 15절 Q3).
- 페이지 머리말: 제목 "운영 현황"(CPY-004), 줄 "마지막 성공 갱신: {시각}"(CPY-015), [새로고침](CPY-008).
- 상태 안내: 머리말 아래 전체 폭 `createAlert`. 상태가 있을 때만 그린다.
- 서버 요약 카드(6열): 실행 상태, 가동 시간, 대기 연결 수, 활성 세션 수, 초기 교환 중, Mode I 대상 연결 중. 모바일은 실행 상태·가동 시간을 한 줄 전체 폭, 나머지는 2열.
- 연결 카드 두 개(각 3열): 중계 연결, 관리 연결. 제목 아래 한 줄 설명(CPY-032, CPY-034). 값은 `createGauge`로 `현재 / 상한`.
- 목록 카드 세 개(전체 폭, 이 순서): 대기 연결(CPY-036), 활성 세션(CPY-039), 최근 종료 목록(CPY-042).

## 5. 조회 상태 모델

조회마다 상태를 둔다.

```text
Query = { status: 'loading' | 'ok' | 'failed',
          data,            // 마지막 성공 응답의 값 (없을 수 있음)
          successAt,       // 이 조회가 마지막으로 성공 적용된 클라이언트 시각
          refreshing,      // 요청 진행 중 여부
          failure }        // 마지막 실패의 kind
```

목록 하나의 표시 규칙:

| 상태 | 데이터 | 표시 |
| --- | --- | --- |
| 첫 조회 중 | 없음 | CPY-016 "조회 중" 안내. 표 자리는 두지 않음 |
| 성공 | 빈 목록 | 표 머리글 유지, 빈 목록 문구 (CPY-038 / CPY-041 / CPY-044) |
| 성공 | 항목 있음 | 표 |
| 갱신 중 | 있음 | CPY-017 "갱신 중" 안내 + 이전 표 |
| 실패 | 없음 | CPY-018 + CPY-021 안내. 표 없음 (실패를 빈 목록으로 보이지 않음) |
| 실패 | 있음 | CPY-019 + CPY-020(`마지막 성공 시각 {successAt}`) + 이전 표 |

- 목록 안내는 `createAlert`로 그린다. 같은 종류가 이어지면 노드를 바꾸지 않는다 (login.js의 안내 방식과 같음).
- 톤: 조회 중·갱신 중은 info, 오래된 데이터는 warning, 조회 실패는 destructive.
- 페이지 머리말의 "마지막 성공 갱신"은 네 조회의 `successAt` 중 가장 최근 값이다 (구현 판단).

## 6. 갱신 흐름

- 시작: 세션 확인(`GET /api/v1/session`)이 200이면 네 조회의 루프를 시작한다. 조회는 `summary`, `waiting`, `sessions`, `closed-sessions` 네 개다.
- 주기: 조회 하나의 응답(성공이든 실패든)을 받은 뒤 `poll_secs`(세션 응답의 `dashboard.poll_secs`) 뒤에 다음 요청을 보낸다. `setInterval`은 쓰지 않는다. 응답을 기다리는 동안에는 같은 조회를 다시 보내지 않으므로 중복 전송이 없다 (계약 1.12절).
- P 값은 코드 상수로 두지 않는다 (frontend.md 1.3절, 계약 1.12절).
- 시간 제한: 조회 요청은 `query_timeout_secs`(세션 응답) 초과 시 중단하고 실패(kind `timeout`)로 처리한다 (계약 1.12절). 로그아웃은 조회가 아니므로 이 값을 쓰지 않는다 (7절).
- 자동 조회에만 `X-Relay-Poll: 1` 헤더를 붙인다. 계약 1.4절에 따라 이 헤더는 GET에만 쓰며, 자동 조회만 유휴 만료를 연장하지 않는다. 사용자 행동(화면 진입, 새로고침, 로그아웃)은 유휴 만료를 연장한다.
- 새로고침(CPY-008): 진행 중이 아닌 조회만 즉시 다시 보낸다. 보류 중인 타이머는 취소하고 다시 예약한다. 진행 중인 조회는 건드리지 않는다. 버튼은 조회가 하나라도 진행 중이면 `busy`로 표시한다.
- 세대 번호: 화면 전환, 로그아웃, 세션 상실, 언마운트 때 세대 번호를 올린다. 응답이 돌아왔을 때 세대가 다르면 버린다. 늦은 응답이 최신 값을 덮지 않는다 (DASH-005).
- 세션 확인 실패(401이 아닌 오류): 페이지에 CPY-018·CPY-021 안내와 [새로고침]을 보여 주고, 새로고침으로 세션 확인을 다시 한다 (구현 판단).

## 7. 실패 처리

| 응답 | 조회 (SCR-001) | 로그아웃 | 세션 확인 |
| --- | --- | --- | --- |
| 200 | 적용, `successAt` 갱신 | 204가 완료다 (아래 참고) | 적용 |
| 401 `unauthenticated` | 보호 데이터·루프 제거, `/login/`으로 이동 | 세션이 없으므로 `/login/`으로 정상 이동 | `/login/`으로 이동 |
| 403 `csrf_invalid` | 해당 없음 (GET) | 잠정: CPY-076 경로 (14절 D2, 사용자 확인 대기) | 해당 없음 |
| 404 `not_found` | 조회 실패로 처리 (구현 판단) | 완료로 보지 않음 (구현 판단) | 해당 없음 |
| 400 `invalid_request` | 조회 실패로 처리 (구현 판단) | 완료로 보지 않음 (구현 판단) | 실패 안내 |
| 500, 응답 없음, 시간 초과 | 실패. 이전 데이터 유지, 오래된 데이터 표시 (CPY-019~021) | CPY-076 경로 (아래 참고) | 실패 안내 |
| 알 수 없는 `code` | HTTP 등급(4xx·5xx)으로 판단 (계약 2.3절) | 같은 HTTP 등급 규칙 | 같음 |
| 2xx지만 JSON이 맞지 않음 | 실패로 처리, 이전 데이터 유지 (구현 판단) | 완료로 보지 않음 | 실패 안내 |

- 로그인 요청은 4.9에서 이미 구현했다. 이 표는 SCR-001 범위만 다룬다.
- 로그아웃 결과 판정: 204이면 완료다. 401이면 세션이 이미 없으므로 정상 이동이다. 500·응답 없음·시간 초과는 확인하지 못한 상태로 보고 CPY-076을 보인다 (계약 2.3절 로그아웃 행). 403은 잠정으로 같은 경로를 쓴다 (14절 D2, 사용자 확인 대기). 403에서는 관리 세션이 서버에 그대로 남는다 (계약 1.4.3절).
- 로그아웃 시간 제한: 계약 1.5절(세션 정보)의 `query_timeout_secs`·`terminate_timeout_secs`는 조회와 종료 요청에만 쓴다. 로그아웃에는 시간 제한 값이 없다. 10초로 정하고 `app/api.js`의 상수로 둔다. 같은 값이 `login/login.js`의 `DEFAULT_TIMEOUT_MS`(10000)에 있지만 별도 상수다 (구현 판단, 사용자 결정 없음).

## 8. 로그아웃 (SCR-001)

1. [로그아웃] 클릭: 버튼을 `busy`로 바꾸고 중복 클릭을 막는다.
2. `DELETE /api/v1/session`을 `X-CSRF-Token` 헤더와 함께 보낸다. 자동 조회 헤더는 붙이지 않는다. 시간 제한은 10초다 (7절).
3. 결과에 따라 7절 표를 따른다.
4. 완료: 루프를 멈추고 `app-root` 안의 보호 DOM을 비운 뒤 `/login/`으로 이동한다.
5. 확인하지 못함(500·응답 없음·시간 초과, 잠정으로 403 포함): 루프를 멈추고 보호 DOM을 비운 뒤 `/login/?notice=logout-unconfirmed`로 이동한다. 로그인 화면은 이 쿼리를 읽어 CPY-076 안내를 보인 뒤 `history.replaceState`로 쿼리를 지운다. 알 수 없는 쿼리 값은 무시하고 CPY-075 안내를 보인다. 릴레이 세션은 끊지 않는다 (스펙 DASH-019).

CSRF 토큰은 메모리에만 둔다. 저장소(localStorage, sessionStorage)나 DOM 속성에 쓰지 않는다.

## 9. 표시 형식 (`format.js`)

- 시각: `YYYY-MM-DD HH:mm:ss` + 시간대 약칭 (예: `2026-10-16 14:03:25 GMT+9`). 날짜·시각 부분은 `Intl.DateTimeFormat.formatToParts`로 조립해 두 언어에서 같은 순서와 구분자를 쓴다. 시간대 약칭은 영어 로캘(`en-US`)으로 얻는다. 실제 출력은 stage 2에서 검증한다 (구현 판단).
- 시각 문자열이 맞지 않으면 CPY-035(확인 불가)로 표시한다 (구현 판단).
- 바이트: 디자인 기준(DESIGN 489~492행)을 따른다. KiB를 소수 한 자리로 반올림하고, 반올림 값이 1024.0 이상이면 MiB로 바꾼다. 예: `0` → `0.0 KiB`, `256` → `0.3 KiB`, `1536` → `1.5 KiB`, `1048575` → `1.0 MiB`, `1572864` → `1.5 MiB`. 정확히 중간인 값(예: `256`, 0.25 KiB)은 0에서 먼 쪽으로 반올림한다 (`Intl.NumberFormat` 기본 `halfExpand`). 숫자 자릿수 구분자는 브라우저 언어 설정을 따른다 (화면 언어와 무관). 구현 판단, stage 2에서 확인.
- 바이트 정밀도 (구현 판단(사용자 결정 없음), 14절 D1): 바이트 값은 `JSON.parse`가 준 Number로 쓴다. 정수는 2^53−1까지 정확하다. 2^53 이상에서는 가장 가까운 binary64 값으로 반올림되며, 구간 `[2^k, 2^(k+1))`의 최대 절대 오차는 `2^(k−53)` 바이트다. 2^70 바이트 미만이면 최대 오차가 65536바이트 이하다. 표시 한 칸은 0.1 MiB(104857.6바이트)이므로 표시는 최대 한 칸 어긋나고, 그 차이는 반올림 경계 근처 값에서만 생긴다. 2^70 바이트 이상은 한 칸을 넘을 수 있으며 시험하지 않는다. 계약 6절 L562는 열린 상태로 둔다.
- 기간: `D일 HH:MM:SS` (English `Dd HH:MM:SS`). 일 수가 0이면 일 부분을 뺀다.
- 알 수 없는 값(`null`, `current_unavailable_reason`이 있는 값): CPY-035 "확인 불가" / "Unavailable", 중립 색. 0으로 바꾸지 않는다.
- 계약이 받은 그대로 주는 값(세션 식별자, 중계 ID, 주소, 역할, 모드, 전송 경로)은 번역하지 않는다 (스펙 3절 L163, 9.2절 L274).

## 10. 계약 필드 매핑

| 계약 필드 | 화면 | 표시 규칙 |
| --- | --- | --- |
| `summary.server.state` | 실행 상태 (CPY-023) | `ready` → CPY-024, `shutdown_grace` → CPY-025, 그 밖의 값은 받은 그대로 |
| `summary.server.uptime_ms` | 가동 시간 (CPY-026) | 기간 |
| `summary.counts.waiting` | 대기 연결 수 (CPY-027) | 숫자 |
| `summary.counts.active` | 활성 세션 수 (CPY-028) | 숫자 |
| `summary.counts.handshaking` + `_unavailable_reason` | 초기 교환 중 (CPY-029) | 숫자, 없으면 확인 불가 |
| `summary.counts.target_connecting` + `_unavailable_reason` | Mode I 대상 연결 중 (CPY-030) | 숫자, 없으면 확인 불가 |
| `summary.usage.relay_connections.current` / `limit` | 중계 연결 (CPY-031·032) | 게이지 `현재 / 상한`. `current`가 null이면 확인 불가 |
| `summary.usage.admin_connections.current` / `limit` | 관리 연결 (CPY-033·034) | 같음 |
| `waiting.items[]`: `role`, `relay_id`, `connected_at`, `waited_ms` | 대기 연결 (CPY-036·037) | 역할·중계 ID 원문, 시각·기간 형식. 빈 목록 CPY-038 |
| `sessions.items[]`: `session_id`, `mode`, `viewer_transport`, `started_at`, `bytes_viewer_to_target`, `bytes_target_to_viewer`, `viewer_address`, `target_address` | 활성 세션 (CPY-039·040) | 식별자는 `/#/sessions/{id}` 링크로 연결한다 (4.10은 링크만, 14절 D4). 상대 주소는 Viewer 주소 줄과 대상 주소 줄을 쌓아 원문으로. 빈 목록 CPY-041 |
| `sessions.items[].state` | 없음 | 표시하지 않는다. 스펙 9.3.2절 CPY-040 표 머리글과 DESIGN 555행(7열)을 따른다 |
| `closed-sessions.items[]`: `session_id`, `close_cause`, `ended_at`, `bytes_viewer_to_target`, `bytes_target_to_viewer` | 최근 종료 목록 (CPY-042·043) | `close_cause` 매핑 (CPY-045~050), 목록에 없는 값은 원문. 빈 목록 CPY-044. `max_entries`는 표시하지 않는다 (구현 판단, 스펙에 문구 없음) |

- 응답의 `as_of`는 화면에 쓰지 않는다. "마지막 성공 갱신"은 클라이언트가 응답을 적용한 시각이다 (구현 판단).

## 11. 정적 제공과 서버 변경

- `crates/adapters/src/dashboard_assets.rs`의 `ASSETS` 표(정확한 경로 일치):
  - 이미 있음: `/app/tokens.css`, `/app/styles/base.css`, `/app/styles/components.css`, `/app/components/*.js` 13개, `/login/language.js`. 4.10에서 이 항목들의 경로는 바꾸지 않는다.
  - 추가 (구현 완료): `/` → `app/index.html`, `/app/app.css`, `/app/main.js`, `/app/operations.js`, `/app/panels.js`, `/app/api.js`, `/app/model.js`, `/app/format.js`, `/app/copy.js`, `/app/language.js`.
  - 추가하지 않음: `/app/index.html` (frontend.md 1.1절).
- `crates/adapters/src/management_http.rs` `static_answer`: 로직은 바꾸지 않는다. `/`가 표에 있으면 세션이 있을 때 200을 준다. 세션이 없으면 303 `/login/` (계약 1.11절).
- `tests/dashboard_assets.rs`의 `asset_table_lists_exactly_the_dashboard_files`가 빠진 파일을 잡는다.
- `xtask/src/tokens/shared.rs`: `language.js`를 `SHARED_FILES`에 더했다 (7 → 8). 원본은 `app/language.js`이고 `login/language.js`는 `cargo xtask tokens`가 만든 복사본이다 (머리글 한 줄 추가, 내용 동일).
- 응답 헤더: `Content-Security-Policy: default-src 'self'; frame-ancestors 'none'`, `Cache-Control: no-store`(보호 응답), `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer` (계약 1.11절, frontend.md 1.2절). `/` 응답에도 같은 헤더가 붙는지 stage 2에서 확인한다.

## 12. 시험 설계

- 브라우저 시험 페이지: `web/dashboard/tests/operations.html`과 `operations-scenarios.js`. 로그인 시험과 같은 방식으로 `?case=` 값에 따라 가짜 응답을 준다. 결과는 `data-result`에 쓴다.
- 시험 사례 (SCR-001 부분): dash-001, 002, 003, 004, 005, 008, 016, 019(로그아웃 성공, 확인 못함 CPY-076, 잠정 403 포함), 020, 021, 022, 024, 025.
- 형식 시험 (`format.js`, 시험 페이지 사례): `0`, `256`(→ `0.3 KiB`), `1536`, `1048575`, `1572864`는 기대값과 같다. 2^53−1은 BigInt로 계산한 기대값과 같다. `JSON.parse('9007199254740993')`의 결과가 2^53임을 확인한다. 2^53 이상 2^70 바이트 미만 값은 BigInt 기대값과 표시 차이가 한 칸(0.1 MiB) 이내다 (9절). 2^70 이상은 시험하지 않는다 (구현 판단(사용자 결정 없음)).
- `tests/dashboard_browser.rs`: 사례마다 `/// spec: dashboard.md:DASH-0xx` 마커를 붙인다. 마커 표기와 ` partial` 접미사는 `docs/conventions/testing.md` 7절을 따른다.
- 로그인 쿼리 시험 (`tests/dashboard_browser.rs`, 14절 D3): `/login/?notice=logout-unconfirmed`이면 CPY-076 안내가 보이고, 이후 주소에서 쿼리가 사라지며(`replaceState`), 다시 불러오면 CPY-075 안내가 보인다. 알 수 없는 쿼리 값은 CPY-075 안내로 처리되고 쿼리가 지워진다.
- 상세 링크 임시 동작 시험 (14절 D4): `/#/sessions/{id}`로 들어가면 SCR-001이 보이고, 4.10 범위에서는 상세 요청을 보내지 않는다.
- 정적 시험 (`management.rs` 정적 경로 시험 위치): `/`가 세션 있을 때 200 HTML과 CSP 헤더, 세션 없을 때 303 `/login/`. Q43 결정에 따른다.
- 수동 확인 (verify-dashboard-frontend.md 3.4~3.7, Q36): 폭별 스크린샷과 시안 대조(3.4), 키보드(3.5), 스크린 리더(3.6), Firefox·Safari와 390px 실기기(3.7).
- 실서버 확인(MGT-026 값 일치, 세션 만료 Si/Sa 시점, `P` 설정 반영)은 4.4 완료 뒤로 미룬다. 해당 시나리오는 부분으로 둔다.

## 13. 반응형 (SCR-001)

- 중단점: `max-width: 1023.98px`에서 모바일 배치. 데스크톱은 `1024px` 이상의 12열 격자 (DESIGN 527행, 543~544행).
- 목록 표는 하나의 DOM(`<table>`)을 두고 명시적 `role`(table, row, columnheader, cell)과 셀마다 `data-label`을 붙인다. 모바일에서는 CSS로 카드처럼 배치하고 `::before { content: attr(data-label) }`로 머리글을 보인다. 머리글 원문은 AT를 위해 남긴다 (구현 판단).
- `createDataTable`은 셀 값으로 DOM `Node`(세션 식별자 링크)를 받는다. 열 옵션 `role`은 만들지 않았다. 카드 배치는 셀마다 붙인 `data-label`과 CSS로 한다. 카드형 변형 컴포넌트는 새로 만들지 않는다 (구현 판단).
- 최소 지원 폭 320px에서 가로 스크롤이 생기지 않는다 (DESIGN).

## 14. 결정 사항과 구현 판단

### 주요 결정 (구현 판단)

- **D1. 2^53−1 초과 바이트 — 구현 판단(사용자 결정 없음): (a) JS `Number` 유지.**
  - 근거: DESIGN 489행은 바이트를 KiB·MiB 소수 한 자리로 표시한다고 정한다. 스펙 3절 L92는 방향별 바이트를 KiB·MiB로 표시한다고 정한다. 바이트 정수를 그대로 보이라는 요구는 없다.
  - 한계 (9절 참고): 2^53 이상 2^70 바이트 미만 값은 표시가 한 칸(0.1 MiB) 이내로 어긋날 수 있다. 2^70 바이트 이상은 한 칸을 넘을 수 있다. 이 구간은 시험하지 않는다.
  - 계약 6절 L562는 이 결정으로 닫히지 않는다. 닫으려면 사용자가 계약을 개정한다.
  - 질문 Q1 (15절): 2^53 이상 2^70 바이트 미만에서 표시가 한 칸(0.1 MiB) 이내로 어긋나는 것을 받아들이는가.
  - 시험: 2^53−1 정확, 2^53 이상 2^70 바이트 미만은 한 칸 이내, `JSON.parse` 결과 확인 (12절).
  - TODO 4.10 L246의 "정확한지 확인한다" 문구는 그대로 둔다. 확인 결과: 2^53−1까지 정확하다. 그 위에서는 반올림 경계 근처 값만 한 칸 이내로 어긋난다 (2^70 미만).

- **D2. 로그아웃 403 `csrf_invalid` — 잠정 (a), 구현 판단(사용자 결정 없음), 사용자 확인 대기 (Q2).**
  - 근거: 계약 1.4.3절·2.3절은 403에서 관리 세션을 유지하고 토큰을 다시 받아 사용자가 다시 요청하도록 한다. 스펙 9.3절에는 로그아웃 403에 맞는 문구가 없다. CPY-068은 종료 요청 문구다 (스펙 4절 L180, 종료 전용).
  - (a) CPY-076 경로로 보낸다. 로그아웃이 확인되지 않았으므로 관리 데이터를 숨기고 로그인 화면으로 이동한다. 기존 승인 문구만 쓴다. 단점: 403에서 관리 세션은 서버에 그대로 남는다. 화면은 이동하지만 세션은 유휴·절대 만료까지 살아 있다.
  - (b) 새 문구를 만든다 (예: "로그아웃 요청을 처리하지 못했습니다. 다시 시도하십시오"). 스펙 9절 개정과 사용자 승인이 필요하다.
  - (c) 토큰만 다시 받고 사용자가 [로그아웃]을 다시 누르게 한다. 화면에 안내가 없다. 확인되지 않은 상태가 드러나지 않는다.
  - 질문 Q2 (15절): (a) 잠정 적용과 세션 잔존을 받아들이는가, 아니면 (b) 또는 (c)인가.

- **D3. CPY-076을 SCR-003에 보이는 방법 — 구현 판단(사용자 결정 없음): URL 쿼리와 `replaceState`.**
  - 채택 내용: 로그아웃 확인 실패 시 `/login/?notice=logout-unconfirmed`로 이동한다. 로그인 화면이 `location.search`를 읽고 `history.replaceState`로 쿼리를 지운다. 저장소를 쓰지 않는다. 로그인 화면에는 운영 데이터가 없다.
  - 구현 세부(쿼리 이름, 안내 종류 이름, 톤, 알 수 없는 값 처리)는 Main 구현 판단이다 (14절 구현 판단 목록).

- **D4. SCR-002 작업 배정 — 구현 판단(사용자 결정 없음): TODO 배정 유지(SCR-002 전체 4.11). 4.10 상세 링크는 계약 화면 URL `/#/sessions/{session_id}`.**
  - 충돌(Main 확인): 해시는 서버로 가지 않는다. 서버 응답은 `/`와 같다 (1절). 4.10 라우터는 SCR-002를 모른다. 링크를 누르면 SCR-001이 보이고 목록은 그대로 남는다. 상세 화면은 4.11까지 없다.
  - 임시 동작(알 수 없는 해시는 SCR-001)은 사용자가 받아들였다. Main 확인 항목: 4.10에서 링크를 숨기거나 텍스트로만 두는가 (계약 URL 연결은 4.11로 미룸).
  - 시험: 12절 상세 링크 임시 동작 시험.

- **D5, D6 — 결정 목록에서 뺐다.**
  - D5: `docs/conventions/testing.md` 7절을 따른다 (12절).
  - D6: 활성 세션의 `closing`은 표시하지 않는다. 스펙 9.3.2절 CPY-040 표 머리글과 DESIGN 555행(7열)을 따른다 (10절).

### navbar 이름 (잠정, 사용자 승인 대기)

- 한국어 "주 메뉴": 디자인 기준(DESIGN 582행)과 프런트엔드 컨벤션 4절(206행)을 따른다. 구현 판단(사용자 결정 없음).
- English "Main menu": 시안 English 프레임의 `aria-label`을 따른다. 스펙 9.3.1절 문구 표에 없고, 스펙 9.5절 7항이 승인한 영어 용어에도 없으므로 잠정이다. 스펙 9.3.1절에 짝 문구를 더하는 것은 스펙 개정이므로 사용자 승인이 필요하다 (15절 Q3).

### 구현 판단 (사용자 결정 없음)

- 마지막 성공 갱신 시각은 클라이언트가 응답을 적용한 시각이다. 머리말은 네 조회 중 가장 최근 성공이다.
- 목록 조회 실패(데이터 없음)에서는 표를 그리지 않고 안내만 보인다.
- 404·400·JSON 불일치를 조회 실패로 처리한다.
- 로그아웃 요청 시간 제한은 10초 상수다 (`login.js`의 `DEFAULT_TIMEOUT_MS`와 같은 값, 별도 상수). 계약 1.12절·2.2절에 로그아웃 값이 없다.
- 로그인 쿼리 `notice=logout-unconfirmed`, 안내 종류 `logout-unconfirmed`(톤 warning), 알 수 없는 쿼리 값은 CPY-075로 처리하고 쿼리를 지운다. 로그인 원본 코드는 이 안내만 더한다 (D3 채택의 세부, Main 구현 판단).
- 해시 라우터: 알 수 없는 해시는 SCR-001을 보인다 (임시, 사용자 수용).
- 바이트: 정확히 중간인 값은 0에서 먼 쪽으로 반올림한다 (`halfExpand`).
- 시각 조립 방식과 잘못된 시각 문자열의 CPY-035 처리.
- 역할, 모드, 전송 경로는 원문. `max_entries`는 표시하지 않는다.
- 게이지 상한 표기는 숫자만 쓴다 (`12 / 200`). 새 문구가 없다.
- `language.js`를 `app/`으로 옮기고 `SHARED_FILES`에 더한다(7 → 8). `login/language.js`는 생성 복사본이 된다. 내용은 같고 머리글 한 줄만 붙는다. 로그인 코드 동작은 바뀌지 않는다. 2단계에서 같은 변경으로 `frontend.md` 1.1절 트리(`login/language.js`가 원본으로 적힌 부분)와 2.3절 복사 목록(L156)을 고친다.
- `createDataTable` 확장(열 옵션 `role`, 셀 `data-label`), 카드 배치는 CSS로 한다 (13절).

## 15. 열린 항목

- 상태는 Active다 (4.10 구현 완료). 아래 질문과 Main 확인 항목은 사용자 답을 기다린다.
- 사용자 질문 (답 필요):
  - Q1: D1 (a)의 한계 수용. 2^53 이상 2^70 바이트 미만에서 표시가 한 칸(0.1 MiB) 이내로 어긋나는 것을 받아들이는가. 계약 6절 L562를 닫을지도 함께 정한다.
  - Q2: D2 잠정 (a) 승인과 403 뒤 관리 세션 잔존 감수. 아니면 (b) 또는 (c).
  - Q3: 상단 막대 이름 "주 메뉴"(한국어)와 "Main menu"(English)를 스펙 9.3.1절 문구 짝에 더하는 것을 승인하는가 (스펙 개정 필요, 14절 navbar 항목).
- Main 확인 (답 필요): D4 상세 링크 처리(14절: 4.10에서 숨기거나 텍스트로만 둠).
- 현재 구조(`dashboard-components.md`)는 구현 뒤 3.1절과 `code-map.md`에 반영한다 (stage 2).
- 실서버 확인은 4.4 뒤에 한다. 그 전까지 관련 시나리오는 부분이다.
