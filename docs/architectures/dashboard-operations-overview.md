# 운영 현황 앱 구조 (SCR-001·SCR-002 자리)

| 항목 | 값 |
| --- | --- |
| 구조 시점 | Current |
| 문서 상태 | Draft |
| 구현 대조 | Partial — [대조 기록](#대조-기록) |
| 코드 위치 | `web/dashboard/app/`의 `main.js`, `router.js`(`mountRouter`, `routeOf`), `operations.js`(`mountOperations`, `endSession`, `signOut`), `session.js`(`mountSession`), `shell.js`(`buildNavbar`, `signOutUrl`), `query.js`(`newQuery`), `model.js`, `api.js`(`getQuery`, `getSession`, `deleteSession`, `terminateSession`), `panels.js`(`statusAlert`, `statusBadge`, `usageGauge`, `closedTable`, `panelCard`, `applyChrome`, `setContent`), `format.js`, `copy.js`, `language.js`, `app.css`. `crates/adapters/src/dashboard_assets.rs`(`ASSETS`), `crates/adapters/src/management_http.rs`(`static_answer`), `xtask/src/tokens/shared.rs`(`SHARED_FILES`) |
| 코드 지도 | [코드 지도](code-map.md) (`web/dashboard/` 행, `dashboard_assets.rs` 행, xtask 행), [컴포넌트 구조](dashboard-components.md) |
| 관련 스펙 | [화면 스펙](../specs/dashboard.md) 3·4·5·6·9절; [디자인 기준](../design/DESIGN.md) 화면 구성(522~534행), 중단점, 표시 형식(489~492행); [프런트엔드 규약](../conventions/frontend.md) 1.1·1.2·1.3·3·4절; [시험 규약](../conventions/testing.md) 상태 규칙(23·99·100행) |
| 관련 계약 | [관리 API 계약 v1](../contracts/management-api-v1.md) 1.3·1.4·1.5·1.6~1.12·2.2·2.3절 |
| 관련 ADR | 없음 |

이 문서는 `web/dashboard/app/`의 파일 배치, 모듈 책임, 조회·갱신 흐름, 실패 처리, 로그아웃, 표시 형식을 정한다. 화면 문구는 스펙 9절이, 필드 의미는 계약이 정한다. 이 문서의 "구현 판단"은 사용자 결정이 아니다. 사용자 결정과 확인이 필요한 항목은 14절에 모은다.

## 1. 범위

- 4.10 구현 범위: SCR-001 전체, 앱 셸(진입점, 세션 확인, 언어 선택, 로그아웃). 해시 라우터는 4.10 코드에 없다 (대조 기록).
- SCR-002는 TODO 배정대로 4.11에서 전체를 만든다(본문, 상세 조회, 종료 흐름). 4.10의 SCR-001 목록 링크는 계약 1.11절의 화면 URL `/#/sessions/{session_id}`를 그대로 쓴다.
- 해시(`#/sessions/…`)는 서버로 가지 않는다. 서버는 `/`와 같은 응답을 준다: 세션이 있으면 200 HTML, 없으면 303 `/login/`. 404 JSON이 아니다.
- 4.11에서 `router.js`가 `#/sessions/{session_id}`를 SCR-002(`session.js`)로, 그 밖의 해시를 SCR-001로 보낸다 (구현 판단(사용자 결정 없음)). 4.10의 임시 동작(해시가 무엇이든 SCR-001)은 없어졌다. 링크 처리 방식은 14절 D4에 적었다.
- 이 문서가 정하지 않는 것: 관리 API 필드 의미(계약), 화면 문구(스펙 9절), 색과 간격 토큰(DESIGN.md).

## 2. 파일 배치

```text
web/dashboard/app/
├── index.html        신규. 제공 경로 `/` (frontend.md 1.1절). `/app/index.html`은 제공하지 않는다.
├── main.js           신규. 진입점. mountRouter(document.getElementById('app-root')) 호출. 해시에 따라 화면을 고른다
├── router.js        신규. 해시 라우터. `#/sessions/{id}`는 session.js, 그 밖의 해시는 operations.js. 화면을 바꿀 때 이전 화면을 `dispose`한다
├── operations.js     신규. SCR-001 조립과 흐름: 세션 확인, 조회 루프, 세대(generation) 번호, 로그아웃, 언어 상태
├── session.js        신규. SCR-002 조립과 흐름: 세션 확인, 상세 조회, 종료 확인, 종료 결과 분류, 대상 소멸, 세대(generation) 번호
├── shell.js          신규. 두 화면이 같이 쓰는 상단 막대(언어 선택, 로그아웃)와 로그아웃 결과 판정
├── query.js          신규. 조회 상태(진행 중, 세대, 데이터, 오류)와 갱신 타이머. DOM을 쓰지 않는다
├── model.js          신규. 응답 검증(계약 1.5~1.10절 필드). 맞지 않으면 `invalid`
├── panels.js         신규. 서버 요약 카드, 연결 카드, 목록 카드와 목록 상태 안내. SCR-002와 같이 쓰는 카드 틀(`panelCard`)과 조회 칸 갱신(`applyChrome`, `setContent`)
├── api.js            신규. fetch 감싸기: 헤더, 시간 제한, 오류 분류
├── format.js         신규. 시각, 기간, 바이트, 숫자, 알 수 없는 값
├── copy.js           신규. 한국어·English 문구. 키는 CPY ID
├── language.js       이동. login/language.js 원본을 옮긴다. login 쪽은 생성 복사본이 된다 (구현 판단, 14절)
├── components/       기존 원본 13개 (login 복사본의 원본). data-table.js는 셀 값으로 DOM Node를 받고 셀마다 data-label을 붙이도록 확장 (3.1절 대상)
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
| `operations.js` | `mountOperations(root, deps)`: 세션 확인(`GET /api/v1/session`), 네 조회 루프, 세대 번호, 로그아웃, 언어 상태 | `request`(fetch 대체), `storage`, `preferredLanguages`, `navigate`(페이지 이동 대체) |
| `model.js` | 응답 검증: `parseSession`·`parseSummary`·`parseWaiting`·`parseSessions`·`parseClosed`. 맞지 않으면 `invalid` 오류 | 없음 |
| `api.js` | 요청 헤더, 시간 제한(AbortController), 응답을 `ApiError`의 kind로 분류. 로그아웃은 204만 성공 | `request` 인자로 받음 |
| `panels.js` | SCR-001 카드 조립: 서버 요약, 연결 카드, 목록 카드 세 개와 목록 상태 안내 | 없음 |
| `format.js` | 시각·기간·바이트·숫자 표시, 역할·모드·전송 경로 표시 이름 | 없음 (`Intl`과 `Date`만 사용) |
| `copy.js` | `ko`·`en` 문구 객체. 각 키에 CPY ID 주석 | 없음 |
| `language.js` | `LANGUAGES`, `chooseLanguage`, `readSavedLanguage`, `saveLanguage` (login과 같은 저장 키). login/language.js는 이 파일의 복사본 | `storage`, `preferredLanguages` |

사용할 기존 컴포넌트 (3.1절 함수, `app/components/`에 이미 있음): `createNavbar`, `createLanguageSelect`(variant `navbar`), `createButton`, `createCard`, `createStat`, `createGauge`, `createDataTable`, `createBadge`, `createAlert`, `createLink`, `icon`. SCR-002 단계(4.11)에서 `createDialog`.

## 4. 화면 골격 (SCR-001)

- 상단 막대: `createNavbar`. 브랜드 "VNC Relay", 항목 "운영 현황"(현재 항목), 오른쪽에 언어 선택과 [로그아웃]. `createNavbar`는 `label`을 필수로 받는다. label은 CPY-078 한국어 "주 메뉴", English "Main menu"다 (사용자 결정 2026-10-23(Q55), 14절 navbar). 모바일 폭(1023.98px 이하)에서는 브랜드 글자와 "운영 현황" 항목을 CSS로 숨기고 DOM에는 남긴다 (DESIGN 543행).
- 랜드마크: 상단 막대는 `nav`, 본문은 `main`이다 (DESIGN 582행). `nav`의 이름은 CPY-078 한국어 "주 메뉴", English "Main menu"다 (14절 navbar).
- 페이지 머리말: 제목 "운영 현황"(CPY-004), 줄 "마지막 성공 갱신: {시각}"(CPY-015), [새로고침](CPY-008).
- 상태 안내: 목록 상태 안내(조회 중·갱신 중·실패·오래된 데이터)는 각 목록 카드 맨 위에 둔다 (`operations.js`의 `applyChrome`). 머리말 아래 전체 폭 안내는 두지 않는다. 배치가 시안과 같은지는 스크린샷 비교(검증 기록 4.6)에서 확인한다 (구현 판단).
- 서버 요약 카드(6열): 실행 상태, 가동 시간, 대기 연결 수, 활성 세션 수, 초기 교환 중, Mode I 대상 연결 중. 모바일은 실행 상태·가동 시간을 한 줄 전체 폭, 나머지는 2열.
- 연결 카드 두 개(각 3열): 중계 연결, 관리 연결. `panelCard`가 CPY-032·CPY-034를 `createCard({description})`에 넘긴다. 설명은 `.card-head`의 제목 아래 `p.card-description`에 둔다(DESIGN 527·638행). `usageGauge({t, title, usage, kind})`는 고리만 만들며 `note`를 받거나 본문에 붙이지 않는다. 값은 `createGauge`로 현재 수와 `상한 {n}`(CPY-079)을 보인다. 상한이 없으면 `상한 확인 불가`(CPY-080)다 (사용자 결정 2026-10-23(Q55)).
- 목록 카드 세 개(전체 폭, 이 순서): 대기 연결(CPY-036), 활성 세션(CPY-039), 최근 종료 목록(CPY-042).

## 5. 조회 상태 모델

조회마다 상태를 둔다.

```text
Query = { data,          // 마지막 성공 응답의 값 (없을 수 있음)
          lastSuccessAt, // 이 값을 적용한 클라이언트 시각
          inFlight,      // 요청 진행 중 여부
          failed }       // 마지막 조회가 실패했는지 여부
```

목록 하나의 표시 규칙:

| 상태 | 데이터 | 표시 |
| --- | --- | --- |
| 첫 조회 중 | 없음 | 표 머리글 아래 빈 행(`data-table-empty`)에 CPY-016 "조회 중" 안내 (스펙 4절 최초 로딩 행, 구현 판단) |
| 성공 | 빈 목록 | 표 머리글 유지, 빈 목록 문구 (CPY-038 / CPY-041 / CPY-044) |
| 성공 | 항목 있음 | 표 |
| 갱신 중 | 있음 | CPY-017 "갱신 중" 배지 + 이전 표 |
| 실패 | 없음 | CPY-018 + CPY-021 안내. 표 없음 (실패를 빈 목록으로 보이지 않음) |
| 실패 | 있음 | CPY-019 + CPY-020(`마지막 성공 시각 {lastSuccessAt}`) + 이전 표 |

- 오래된 데이터와 조회 실패 안내는 `createAlert`, 조회 중·갱신 중 표시는 `createBadge`로 그린다 (`panels.js`의 `statusAlert`·`statusBadge`).
- 톤: 조회 중·갱신 중은 info, 오래된 데이터는 warning, 조회 실패는 destructive로 한다는 설계다. 실제 톤은 스크린샷 비교에서 확인한다 (검증 기록 4.6).
- 페이지 머리말의 "마지막 성공 갱신"은 네 조회의 `lastSuccessAt` 중 가장 최근 값이다 (구현 판단).

## 6. 갱신 흐름

- 시작: 세션 확인(`GET /api/v1/session`)이 200이면 네 조회의 루프를 시작한다. 조회는 `summary`, `waiting`, `sessions`, `closed-sessions` 네 개다.
- 주기: 조회 하나의 응답(성공이든 실패든)을 받은 뒤 `poll_secs`(세션 응답의 `dashboard.poll_secs`) 뒤에 다음 요청을 보낸다. 타이머는 `setTimeout`으로 예약하며 `setInterval`은 쓰지 않는다. 응답을 기다리는 동안에는 같은 조회를 다시 보내지 않으므로 중복 전송이 없다 (계약 1.12절).
- P 값은 코드 상수로 두지 않는다 (frontend.md 1.3절, 계약 1.12절).
- 시간 제한: 조회 요청은 `query_timeout_secs`(세션 응답) 초과 시 중단하고 실패(kind `timeout`)로 처리한다 (계약 1.12절). 로그아웃은 조회가 아니므로 이 값을 쓰지 않는다 (7절).
- 자동 조회에만 `X-Relay-Poll: 1` 헤더를 붙인다. 계약 1.4절에 따라 이 헤더는 GET에만 쓰며, 자동 조회만 유휴 만료를 연장하지 않는다. 사용자 행동(화면 진입, 새로고침, 로그아웃)은 유휴 만료를 연장한다.
- 새로고침(CPY-008): 진행 중이 아닌 조회만 즉시 다시 보낸다. 보류 중인 타이머는 취소하고 다시 예약한다. 진행 중인 조회는 건드리지 않는다. 버튼은 로그아웃이 진행 중일 때만 `disabled`로 막는다. 조회가 진행 중일 때 `busy`로 표시하는 동작은 아직 없다 (차이, 대조 기록).
- 세대 번호: 세션 상실(`endSession`)과 로그아웃(`signOut`) 때 올린다. 화면이 바뀔 때는 라우터가 이전 화면의 `dispose`를 부르고, `dispose`도 올린다 (4.11, `router.js`). 응답이 돌아왔을 때 세대가 다르면 버린다. 늦은 응답이 최신 값을 덮지 않는다 (DASH-005).
- 세션 확인 실패(401이 아닌 오류): 페이지에 CPY-018·CPY-021 안내와 [새로고침]을 보여 주고, 새로고침으로 세션 확인을 다시 한다 (구현 판단).

## 7. 실패 처리

| 응답 | 조회 (SCR-001) | 로그아웃 | 세션 확인 |
| --- | --- | --- | --- |
| 200 | 적용, `lastSuccessAt` 갱신 | CPY-076 경로 (204만 완료, 아래 참고) | 적용 |
| 401 `unauthenticated` | 보호 데이터·루프 제거, `/login/`으로 이동 | 세션이 없으므로 `/login/`으로 정상 이동 | `/login/`으로 이동 |
| 403 `csrf_invalid` | 해당 없음 (GET) | CPY-076 경로 (14절 D2, 사용자 결정 2026-10-23(Q54)) | 해당 없음 |
| 404 `not_found` | 조회 실패로 처리 (구현 판단) | CPY-076 경로 (구현 판단) | 해당 없음 |
| 400 `invalid_request` | 조회 실패로 처리 (구현 판단) | CPY-076 경로 (구현 판단) | 실패 안내 |
| 500, 응답 없음, 시간 초과 | 실패. 이전 데이터 유지, 오래된 데이터 표시 (CPY-019~021) | CPY-076 경로 (아래 참고) | 실패 안내 |
| 알 수 없는 `code` | HTTP 등급(4xx·5xx)으로 판단 (계약 2.3절) | CPY-076 경로 (204 외) | 같음 |
| 2xx지만 JSON이 맞지 않음 | 실패로 처리, 이전 데이터 유지 (구현 판단) | CPY-076 경로 (204 외) | 실패 안내 |

- 로그인 요청은 4.9에서 이미 구현했다. 이 표는 SCR-001 범위만 다룬다.
- 로그아웃 결과 판정: 204이면 완료다. 401이면 세션이 이미 없으므로 정상 이동이다. 500·응답 없음·시간 초과와 그 밖의 204 외 응답(200·400·404 등)은 확인하지 못한 상태로 보고 CPY-076을 보인다 (계약 2.3절 로그아웃 행, 구현 판단). 403도 같은 경로를 쓴다 (계약 2.3절 로그아웃 행, 14절 D2, 사용자 결정 2026-10-23(Q54)). 403에서는 관리 세션이 서버에 그대로 남는다 (계약 1.4.3절).
- 로그아웃 시간 제한: 계약 1.5절(세션 정보)의 `query_timeout_secs`·`terminate_timeout_secs`는 조회와 종료 요청에만 쓴다. 로그아웃에는 시간 제한 값이 없다. 10초로 정하고 `app/api.js`의 상수로 둔다. 같은 값이 `login/login.js`의 `DEFAULT_TIMEOUT_MS`(10000)에 있지만 별도 상수다 (구현 판단, 사용자 결정 없음).

## 8. 로그아웃 (SCR-001)

1. [로그아웃] 클릭: 버튼을 `disabled`로 막아 중복 클릭을 막는다 (`busy` 표시는 없다).
2. 토큰이 메모리에 없으면 세션 확인(`getSession`)으로 먼저 받는다 (구현 판단). `DELETE /api/v1/session`을 `X-CSRF-Token` 헤더와 함께 보낸다. 자동 조회 헤더는 붙이지 않는다. 시간 제한은 10초다 (7절).
3. 결과에 따라 7절 표를 따른다.
4. 완료(204) 또는 세션 없음(401): 루프를 멈추고 `app-root` 안의 보호 DOM을 비운 뒤 `/login/`으로 이동한다.
5. 확인하지 못함(204 외 응답: 200·400·403·404·500 등, 응답 없음, 시간 초과): 루프를 멈추고 보호 DOM을 비운 뒤 `/login/?notice=logout-unconfirmed`로 이동한다. 로그인 화면은 이 쿼리를 읽어 CPY-076 안내를 보인 뒤 `history.replaceState`로 쿼리를 지운다. 알 수 없는 쿼리 값은 CPY-075 안내로 처리한다. 릴레이 세션은 끊지 않는다 (스펙 DASH-019).

CSRF 토큰은 메모리에만 둔다. 저장소(localStorage, sessionStorage)나 DOM 속성에 쓰지 않는다.

## 9. 표시 형식 (`format.js`)

- 시각: `YYYY-MM-DD HH:mm:ss` + 시간대 약칭 (예: `2026-10-16 14:03:25 GMT+9`). 날짜·시각은 `Date` 메서드와 0 채움으로 조립하고, 시간대 약칭만 `Intl.DateTimeFormat.formatToParts`(`en-US`)로 얻는다. 두 언어에서 같은 순서와 구분자를 쓴다. 실제 출력은 stage 2에서 검증한다 (구현 판단).
- 시각 문자열이 맞지 않으면 응답 자체를 2xx JSON 불일치로 보고 실패로 처리한다 (7절). 값이 `null`일 때만 CPY-035(확인 불가)를 쓴다 (구현 판단).
- 바이트: 디자인 기준(DESIGN 489~492행)을 따른다. KiB를 소수 한 자리로 반올림하고, 반올림 값이 1024.0 이상이면 MiB로 바꾼다. 예: `0` → `0.0 KiB`, `256` → `0.3 KiB`, `1536` → `1.5 KiB`, `1048575` → `1.0 MiB`, `1572864` → `1.5 MiB`. 정확히 중간인 값(예: `256`, 0.25 KiB)은 0에서 먼 쪽으로 반올림한다 (`Intl.NumberFormat` 기본 `halfExpand`). 숫자 자릿수 구분자는 브라우저 언어 설정을 따른다 (화면 언어와 무관). 구현 판단, stage 2에서 확인.
- 바이트 정밀도 (14절 D1, 사용자 결정 2026-10-23(Q53)): `JSON.parse`는 2^53 이상의 정수를 가장 가까운 binary64 값으로 근사하고, `formatBytes`는 받은 Number를 정확히 계산한다. 2^53−1까지는 정확하다. 구간 `[2^k, 2^(k+1))`의 최대 절대 오차는 `2^(k−53)` 바이트다. 2^70 바이트 미만이면 최대 오차가 65536바이트 이하다. 표시 한 칸은 0.1 MiB(104857.6바이트)이므로 표시는 최대 한 칸 어긋나고, 그 차이는 반올림 경계 근처 값에서만 생긴다. 2^70 바이트 이상은 한 칸을 넘을 수 있으며 시험하지 않는다. 사용자가 이 한계를 받아들였고 계약 6절의 전송량 안전 범위 항목을 닫았다.
- 기간: `D일 HH:MM:SS` (English `Dd HH:MM:SS`). 일 수가 0이면 일 부분을 뺀다.
- 알 수 없는 값(`null`): CPY-035 "확인 불가" / "Unavailable", 중립 색. 0으로 바꾸지 않는다. `*_unavailable_reason` 필드는 읽지 않는다. 계약 소비자 규칙은 알 수 없는 사유를 일반 "확인 불가"로 보이라고 하므로 결과는 같다 (차이, 14절 구현 판단).
- 세션 식별자, 중계 ID, 주소는 계약 값을 그대로 보인다. 역할·모드·전송 경로는 스펙이 쓰는 표기로 바꾼다 (예: `mode1`→`Mode I`, `tcp`→`TCP`, `viewer`→`Viewer`). 목록에 없는 값은 원문이다. 스펙 3절 L163·9.2절 L274의 "번역하지 않음"과 맞는지는 구현 판단이며 14절 확인 항목이다.

## 10. 계약 필드 매핑

| 계약 필드 | 화면 | 표시 규칙 |
| --- | --- | --- |
| `summary.server.state` | 실행 상태 (CPY-023) | `ready` → CPY-024, `shutdown_grace` → CPY-025, 그 밖의 값은 받은 그대로 |
| `summary.server.uptime_ms` | 가동 시간 (CPY-026) | 기간 |
| `summary.counts.waiting` | 대기 연결 수 (CPY-027) | 숫자 |
| `summary.counts.active` | 활성 세션 수 (CPY-028) | 숫자 |
| `summary.counts.handshaking` + `_unavailable_reason` | 초기 교환 중 (CPY-029) | 숫자. `null`이면 확인 불가 (사유 필드는 읽지 않음, 14절 구현 판단) |
| `summary.counts.target_connecting` + `_unavailable_reason` | Mode I 대상 연결 중 (CPY-030) | 숫자. `null`이면 확인 불가 (사유 필드는 읽지 않음, 14절 구현 판단) |
| `summary.usage.relay_connections.current` / `limit` | 중계 연결 (CPY-031·032) | `panelCard.description`은 CPY-032. 게이지는 현재 수와 `상한 {n}`(CPY-079). `current`가 null이면 확인 불가, `limit`이 null이면 `상한 확인 불가`(CPY-080) (사용자 결정 2026-10-23(Q55)) |
| `summary.usage.admin_connections.current` / `limit` | 관리 연결 (CPY-033·034) | `panelCard.description`은 CPY-034. 게이지 규칙은 중계 연결과 같다 |
| `waiting.items[]`: `role`, `relay_id`, `connected_at`, `waited_ms` | 대기 연결 (CPY-036·037) | `relay_id`는 원문 `.session-id`와 `mobileHead: 'primary'`. 역할은 아이콘 없는 중립 배지와 `'secondary'`. 시각·기간 형식을 적용한다. 빈 목록 CPY-038 (DESIGN 395·473·554·608·626·636행) |
| `sessions.items[]`: `session_id`, `mode`, `viewer_transport`, `started_at`, `bytes_viewer_to_target`, `bytes_target_to_viewer`, `viewer_address`, `target_address` | 활성 세션 (CPY-039·040) | 식별자는 `createLink`로 `/#/sessions/${encodeURIComponent(sessionId)}`에 연결한다. `.session-id`와 `mobileHead: 'primary'`를 쓴다. [상세]는 `'secondary'`다. 모드·전송 경로는 아이콘 없는 중립 배지다. 주소는 Viewer·대상 주소 줄의 `.fact-address` 값으로 보인다. 빈 목록 CPY-041 (DESIGN 395·473·555·608·626·636·644행) |
| `sessions.items[].state` | 없음 | D6에 따라 `closing` 배지를 포함해 표시하지 않는다. 스펙 9.3.2절 CPY-040 표 머리글과 DESIGN 555행(7열)을 따른다 |
| `closed-sessions.items[]`: `session_id`, `close_cause`, `ended_at`, `bytes_viewer_to_target`, `bytes_target_to_viewer` | 최근 종료 목록 (CPY-042·043) | 식별자는 원문 `.session-id`와 `mobileHead: 'primary'`. 종료 원인 배지는 `'secondary'`다. 원인은 CPY-045~050·083으로 매핑하며 목록 밖 값은 원문이다. `connection_error`·`internal_error`는 destructive, `dead_connection`·`close_timeout`은 warning, 나머지는 neutral이다. `internal_error`의 문구는 화면 스펙 CPY-083(Q94), 톤은 DESIGN의 판단을 따른다. 빈 목록 CPY-044. `max_entries`는 표시하지 않는다 (구현 판단, 스펙에 문구 없음; DESIGN 393~395·473·556·608·622·626·636행) |

- 응답의 `as_of`는 화면에 쓰지 않는다. "마지막 성공 갱신"은 클라이언트가 응답을 적용한 시각이다 (구현 판단).
- 방향별 바이트는 두 줄이다. 각 줄은 `.direction-mark.direction-mark-1` 또는 `.direction-mark.direction-mark-2`, 흐린 방향 이름, 값 순서다. 표식은 `aria-hidden="true"`인 장식이다. Viewer→대상은 `chart-1`, 대상→Viewer는 `chart-2`다(DESIGN 378·643행).
- 활성 세션의 바이트 열 이름은 CPY-040을 쓴다. 시안 문구로 바꾸지 않는다. [상세] 링크는 `.button.button-outline`에 오른쪽 꺾쇠 아이콘을 붙인다. 표 안 최소 높이는 데스크톱 `spacing.control-height-sm`, 모바일 `spacing.control-height-touch`다(DESIGN 579·623·634행).
- 역할·모드·전송 경로와 중립 종료 원인 배지는 아이콘이 없다. warning·destructive 종료 원인 배지는 상태 아이콘을 함께 쓴다(DESIGN 608·622~626·636행).

## 11. 정적 제공과 서버 변경

- `crates/adapters/src/dashboard_assets.rs`의 `ASSETS` 표(정확한 경로 일치):
  - 이미 있음: `/app/tokens.css`, `/app/styles/base.css`, `/app/styles/components.css`, `/app/components/*.js` 13개, `/login/language.js`. 4.10에서 이 항목들의 경로는 바꾸지 않는다.
  - 추가 (구현 완료): `/` → `app/index.html`, `/app/app.css`, `/app/main.js`, `/app/operations.js`, `/app/panels.js`, `/app/api.js`, `/app/model.js`, `/app/format.js`, `/app/copy.js`, `/app/language.js`.
  - 추가하지 않음: `/app/index.html` (frontend.md 1.1절).
- `crates/adapters/src/management_http.rs` `static_answer`: 로직은 바꾸지 않는다. `/`가 표에 있으면 세션이 있을 때 200을 준다. 세션이 없으면 303 `/login/` (계약 1.11절).
- `tests/dashboard_assets.rs`의 `asset_table_lists_exactly_the_dashboard_files`가 빠진 파일을 잡는다.
- `xtask/src/tokens/shared.rs`: `language.js`를 `SHARED_FILES`에 더했다 (7 → 8). 원본은 `app/language.js`이고 `login/language.js`는 `cargo xtask tokens`가 만든 복사본이다 (머리글 한 줄 추가, 내용 동일).
- 응답 헤더: `Content-Security-Policy: default-src 'self'; frame-ancestors 'none'`, `Cache-Control: no-store`(보호 응답), `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer` (계약 1.11절, frontend.md 1.2절). `/` 응답 헤더는 `management.rs`의 `protected_files_are_served_with_a_session`이 세션이 있을 때 `check_file_headers`로 확인한다 (계약 5절). 실제 브라우저 흐름은 12절의 별도 실서버 시험으로 확인한다.

## 12. 시험 설계

### 실서버 브라우저 시험 (2026-11-05)

- 사용자 결정 2026-11-05(Q112): 시험 전용 작은 CDP(Chrome DevTools Protocol) 클라이언트가 `--remote-debugging-port`로 Chrome·Edge를 조작한다. 승인된 `tokio-tungstenite`와 `serde_json`을 루트 개발 의존성으로 재사용한다. 외부 브라우저 드라이버와 새 crate는 추가하지 않는다.
- `tests/common/dashboard_live.rs`는 실제 계정 해시·설정·`vnc-relay serve`·TCP 중계 연결을 준비한다. `tests/common/dashboard_cdp.rs`는 탐색·입력·클릭·DOM 관찰을 맡는다. `tests/common/dashboard_proxy.rs`는 전달과 장애 주입을 맡는다. `tests/dashboard_live.rs`의 25개 함수가 각각 한 행 전체의 표식을 소유한다. 화면별 본문은 `tests/dashboard_live/{login,overview,detail,cross}.rs`에 둔다. 공통 모듈과 하위 본문에는 표식을 붙이지 않는다.
- 사용자 결정 2026-11-05(Q113): 시험 쪽 TCP 프록시가 실제 관리 리스너로 전달한다. 일반 데이터와 쿠키는 서버 원본이다. 지연·응답 유실·연결 종료·계약 모양 500만 시험 쪽에서 주입한다. 요청 시각·동시 처리 수·실제 전달 수·주입 수를 따로 기록한다. 주입 오류를 실제 서버의 500 증거로 쓰지 않는다.
- DASH-008은 `Si=300s`를 실제로 기다린다. 독립 쿠키의 운영 현황·상세 브라우저를 동시에 관찰한다. 각 쿠키의 마지막 수동 요청 시각부터 300초 이전에는 실제 401이 없어야 한다. 297초 뒤에도 정상 자동 폴링과 보호된 DOM을 확인한다. 그 뒤 첫 실제 401, 보호 데이터 제거, 로그인 이동과 폴링 중단을 확인한다. 단순히 301초 잠든 뒤 401이 있는지만 검사하지 않는다. CDP 명령마다 제한 시간을 두되 브라우저 전체에 60초 기한을 두지 않는다.
- `P` 기본값·설정값은 서로 다른 실제 프로세스에서 관찰한다. 긴 응답과 늦은 응답은 프록시의 실제 전송 지연으로 만든다. 시험 페이지의 fetch 교체는 실서버 증거로 세지 않는다.
- 기존 dump-dom 시험은 빠른 컴포넌트·형식 부분 증거로 유지한다. DASH-015의 수동 접근성·DASH-023의 네 브라우저 호환성은 별도다. 새 시험은 실행 전까지 통과로 기록하지 않는다.
- DASH-001·021의 확인 불가 값은 실제 조회 전달 실패로 만든다. 정상 서버의 정수 필드를 null로 위조하지 않는다. DASH-025의 미래 enum 값은 생산자가 낼 수 없는 입력이다. 일반 데이터·탐색은 실제 서버로 확인하고, 미래 값만 실제 브라우저에서 제품 파서·표 컴포넌트에 별도 Consumer 계약 입력으로 준다. 두 언어의 원문 값 보존과 번역된 머리글·행동을 확인한다. 이 입력은 API 응답이나 프록시 본문이 아니다.
- 한 시험 바이너리 안의 행은 직렬로 실행한다. 서로 다른 행의 브라우저 부하가 폴링 시각 관찰을 흔들지 않게 하기 위해서다. DASH-008의 운영 현황·상세 브라우저 두 개는 같은 행에서 동시에 실행한다.
- 원시 조회의 관찰 계정은 `bob`, 브라우저 계정은 `alice`다. 역할을 분리하기 위한 시험 구성이다. 같은 계정의 독립 세션을 금지하는 제품 정책은 아니다. 전달 프록시는 `Content-Length`·chunked·본문 없는 상태의 HTTP 프레이밍으로 응답 완료를 판정한다. 완전한 응답을 읽은 뒤 TCP FIN을 추가로 기다리거나, 후속 연결 reset 때문에 그 응답을 폐기하지 않는다.
- 프록시 accept 루프만 비차단이다. 수락한 소켓은 제한 시간이 있는 차단 모드로 명시적으로 바꾼다. Windows는 accept된 소켓에 비차단 모드를 상속한다. 이를 그대로 두면 브라우저 사전 연결에서 요청 도착 전 `WouldBlock`(10035)을 읽고 소켓을 닫는다. 실제 조사에서 POST가 프록시·서버에 도달하지 않고 Chromium이 `ERR_SOCKET_NOT_CONNECTED`를 기록했다. 이 수정은 재시도나 시간 제한 증가가 아니다.
- 실패 진단은 프록시 전송 단계·상태·소요 시간·OS 오류, CDP 네트워크 실패·응답 상태, 서버 사건의 열거 값만 남긴다. 요청·응답 본문, 쿠키·비밀번호·토큰·헤더 값은 출력하지 않는다.


- 브라우저 시험 페이지: `web/dashboard/tests/operations.html`과 `operations-scenarios.js`. 로그인 시험과 같은 방식으로 `?case=` 값에 따라 가짜 응답을 준다. 결과는 `data-result`에 쓴다.
- 세션 상세 시험 페이지: `web/dashboard/tests/session.html`과 `session-scenarios.js`. 가짜 응답으로 사례를 돌리고, 결과는 `data-result`에 쓴다.
- 라우터 시험 페이지: `web/dashboard/tests/router.html`과 `router-scenarios.js`. `app/router.js`의 `mountRouter`를 그대로 쓴다. 목록 페이지에서 상세 링크의 `href`를 확인하고, 같은 해시를 적용해 상세(SCR-002)가 열리고 목록으로 돌아오는지 본다(DASH-006, 14절 D4).
- 시험 사례 (SCR-001 부분): dash-001, 002, 003, 004, 005, 008, 016, 019(로그아웃 성공, 확인 못함 CPY-076, 403 포함), 020, 021, 022, 024, 025.
- 시험 사례 (SCR-002 부분, 4.11): dash-006, 007, 009, 010, 011, 012, 013, 014, 016, 019, 022, 024, 025, 026. 모두 `partial`이다. DASH-006은 `session.html`(상세 값)과 `router.html`(목록에서 상세, 목록 복귀) 두 페이지로 시험한다. DASH-024·025는 SCR-002 부분만 여기서 시험한다. SCR-003 부분은 로그인 시험(TODO 4.9)이다. 실서버 확인은 하지 않았다.
- 형식 시험 (`format.js`, 시험 페이지 사례): `0`, `256`(→ `0.3 KiB`), `1536`, `1048575`, `1572864`는 기대값과 같다. 2^53−1은 BigInt로 계산한 기대값과 같다. `JSON.parse('9007199254740993')`의 결과가 2^53임을 확인한다. 2^53 이상 2^70 바이트 미만 값은 BigInt 기대값과 표시 차이가 한 칸(0.1 MiB) 이내다 (9절). 2^70 이상은 시험하지 않는다 (구현 판단(사용자 결정 없음)).
- `tests/dashboard_browser.rs`: 사례마다 `/// spec: dashboard.md:DASH-0xx` 마커를 붙인다. 마커 표기와 ` partial` 접미사는 `docs/conventions/testing.md` 7절을 따른다.
- 로그인 쿼리 시험 (`tests/dashboard_browser.rs`, 14절 D3): `/login/?notice=logout-unconfirmed`이면 CPY-076 안내가 보이고, 이후 주소에서 쿼리가 사라지며(`replaceState`), 다시 불러오면 CPY-075 안내가 보인다. 알 수 없는 쿼리 값은 CPY-075 안내로 처리되고 쿼리가 지워진다.
- 상세 링크 (14절 D4): 4.11에서 `router.js`가 `#/sessions/{session_id}`를 SCR-002로 보낸다. `router-scenarios.js`가 링크 `href`를 확인하고 같은 해시를 적용해 상세와 목록 복귀를 본다. 링크를 실제로 누르면 시험 페이지를 벗어나므로 쓰지 않는다. SCR-002 본문과 종료 흐름은 `session-scenarios.js` 사례로 시험한다.
- 정적 시험 (`management.rs`): 세션이 없으면 `/`와 `/app/…` 경로는 303 `/login/`이다 (`static_paths_redirect_to_login_without_a_session`, `protected_files_redirect_to_login_without_a_session`). 세션이 있으면 `/`는 200 HTML과 정적 헤더다 (`protected_files_are_served_with_a_session`). Q43 결정에 따른다.
- 수동 확인 (verify-dashboard-frontend.md 3.4~3.7, Q36): 폭별 스크린샷과 시안 대조(3.4), 키보드(3.5), 스크린 리더(3.6), Firefox·Safari와 390px 실기기(3.7).
- 실서버 값·Si 만료·`P` 설정 확인은 위 CDP 시험이 맡는다. 기존 목 시험의 부분 표식은 유지한다. 새 비부분 표식의 실행 여부는 실제 게이트 결과와 분리한다.

## 13. 반응형 (SCR-001)

- 중단점: `max-width: 1023.98px`에서 모바일 배치. 데스크톱은 `1024px` 이상의 12열 격자 (DESIGN 527행, 543~544행).
- 목록 표는 하나의 DOM(`<table>`)을 쓴다. 네이티브 `table`에 `role="table"`, `thead`·`tbody`에 `role="rowgroup"`, 모든 `tr`에 `role="row"`, `th scope="col"`에 `role="columnheader"`, `td`에 `role="cell"`을 명시한다. 빈 행·빈 셀도 같다. 이는 구현 판단이 아니라 DESIGN 565행의 기준이다. 머리글 행은 화면에서만 숨기고 접근성 트리에는 남긴다.
- `createDataTable`의 열 옵션 `mobileHead`가 첫 줄을 정한다. `primary`·`secondary` 셀은 `.data-table-head-primary`·`.data-table-head-secondary`로 모바일 격자의 첫 행(`grid-row: 1`)을 공유한다. 대기는 중계 ID·역할, 활성은 세션 링크·[상세], 종료는 세션 식별자·종료 원인 순서다(DESIGN 554~563행).
- 첫 줄에는 `::before` 레이블을 붙이지 않는다. 나머지 셀은 `data-label`을 CSS 가상 요소로 보이는 레이블·값 쌍으로 만든다. 가상 요소의 대체 텍스트는 비워 중복 읽기를 막는다(DESIGN 563·566행). 셀 값으로 DOM `Node`를 받으며 모바일 전용 마크업은 만들지 않는다. API와 검증은 [공통 컴포넌트 3.1절](dashboard-components.md#31-공개-함수)을 따른다.
- 최소 지원 폭 320px에서 가로 스크롤이 생기지 않는다 (DESIGN). 운영 화면의 320px 확인은 스크린샷 비교(검증 기록 4.6)에서 한다.

## 14. 결정 사항과 구현 판단

### 주요 결정

- **D1. 2^53−1 초과 바이트 — (a) JS `Number` 유지. 한계는 사용자 결정 2026-10-23(Q53)으로 받아들였다.**
  - 근거: DESIGN 489행은 바이트를 KiB·MiB 소수 한 자리로 표시한다고 정한다. 스펙 3절 L92는 방향별 바이트를 KiB·MiB로 표시한다고 정한다. 바이트 정수를 그대로 보이라는 요구는 없다.
  - 한계 (9절 참고): 2^53 이상 2^70 바이트 미만 값은 표시가 한 칸(0.1 MiB) 이내로 어긋날 수 있다. 2^70 바이트 이상은 한 칸을 넘을 수 있다. 이 구간은 시험하지 않는다.
  - 계약 6절의 전송량 안전 범위 항목은 이 결정으로 닫았다 (계약 변경 행, 2026-10-23 명확화).
  - 시험: 2^53−1 정확, 2^53 이상 2^70 바이트 미만은 한 칸 이내, `JSON.parse` 결과 확인 (12절).
  - TODO 4.10 L246의 "정확한지 확인한다" 문구는 그대로 둔다. 확인 결과: 2^53−1까지 정확하다. 그 위에서는 반올림 경계 근처 값만 한 칸 이내로 어긋난다 (2^70 미만).

- **D2. 로그아웃 403 `csrf_invalid` — (a) CPY-076 경로. 사용자 결정 2026-10-23(Q54).**
  - 근거: 계약 1.4.3절은 403에서 관리 세션을 유지한다. 스펙 9.3절에는 로그아웃 403만을 위한 문구가 없다. CPY-068은 종료 요청 문구다 (스펙 4절 L180, 종료 전용).
  - 결정 내용: 로그아웃이 확인되지 않았으므로 관리 데이터를 숨기고 `/login/?notice=logout-unconfirmed`로 이동해 CPY-076을 보인다. 기존 승인 문구만 쓴다. 계약 2.3절 로그아웃 행과 스펙 4절 SCR-003 '로그아웃 결과 확인 필요' 행에 403을 더했다.
  - 감수한 점: 403에서 관리 세션은 서버에 그대로 남는다. 화면은 이동하지만 세션은 유휴 만료 Si나 절대 만료 Sa까지 살아 있다.
  - 고르지 않은 안: (b) 새 문구, (c) 토큰만 다시 받고 사용자가 [로그아웃]을 다시 누르게 하는 방식.

- **D3. CPY-076을 SCR-003에 보이는 방법 — 구현 판단(사용자 결정 없음): URL 쿼리와 `replaceState`.**
  - 채택 내용: 로그아웃 확인 실패 시 `/login/?notice=logout-unconfirmed`로 이동한다. 로그인 화면이 `location.search`를 읽고 `history.replaceState`로 쿼리를 지운다. 저장소를 쓰지 않는다. 로그인 화면에는 운영 데이터가 없다.
  - 구현 세부(쿼리 이름, 안내 종류 이름, 톤, 알 수 없는 값 처리)는 구현 판단이다 (14절 구현 판단 목록).

- **D4. SCR-002 작업 배정 — 구현 판단: TODO 배정 유지(SCR-002 전체 4.11). 4.10 상세 링크는 계약 화면 URL `/#/sessions/{session_id}`로 두고 숨기지 않는다 (2026-10-22 Main 결정).**
  - 충돌: 해시는 서버로 가지 않는다. 서버 응답은 `/`와 같다 (1절). 4.10 코드에는 해시 라우터가 없어 SCR-002를 모른다. 링크를 누르면 SCR-001이 보이고 목록은 그대로 남는다. 상세 화면은 4.11까지 없다.
  - 임시 동작(알 수 없는 해시는 SCR-001)은 구현 판단이다. 링크는 숨기지 않고 텍스트로만 두지도 않는다. 계약 URL 연결(해시 라우터와 상세 화면)은 4.11로 미룬다.
  - 시험: 12절 상세 링크 임시 동작 시험.
  - 4.11 반영: `#/sessions/{session_id}`는 SCR-002로 간다. 그 밖의 해시는 SCR-001로 간다. 4.10의 임시 동작 설명은 이 문장으로 대체된다. 라우터 시험은 12절의 DASH-006 사례(`router.html`)다.

- **D5, D6 — 결정 목록에서 뺐다.**
  - D5: `docs/conventions/testing.md` 7절을 따른다 (12절).
  - D6: 활성 세션의 `closing`은 표시하지 않는다. 스펙 9.3.2절 CPY-040 표 머리글과 DESIGN 555행(7열)을 따른다 (10절).

### navbar 이름 (사용자 결정 2026-10-23(Q55))

- CPY-078 한국어 "주 메뉴", English "Main menu". 한국어는 디자인 기준(DESIGN 582행)과 프런트엔드 컨벤션 4절(206행)을 따르고, English는 시안 English 프레임의 `aria-label`을 따른다. 스펙 9.3.1절에 문구 짝으로 더했다.

### 구현 판단 (사용자 결정 없음)

- 마지막 성공 갱신 시각은 클라이언트가 응답을 적용한 시각이다. 머리말은 네 조회 중 가장 최근 성공이다.
- 목록 조회 실패(데이터 없음)에서는 표를 그리지 않고 안내만 보인다.
- 404·400·JSON 불일치를 조회 실패로 처리한다.
- 로그아웃 요청 시간 제한은 10초 상수다 (`login.js`의 `DEFAULT_TIMEOUT_MS`와 같은 값, 별도 상수). 계약 1.12절·2.2절에 로그아웃 값이 없다.
- 로그인 쿼리 `notice=logout-unconfirmed`, 안내 종류 `logout-unconfirmed`(톤 warning), 알 수 없는 쿼리 값은 CPY-075로 처리하고 쿼리를 지운다. 로그인 원본 코드는 이 안내만 더한다 (D3 채택의 세부, 구현 판단).
- 해시 처리: `router.js`가 `#/sessions/{session_id}`는 SCR-002로, 그 밖의 해시는 SCR-001로 보낸다 (구현 판단(사용자 결정 없음), 1절).
- 바이트: 정확히 중간인 값은 0에서 먼 쪽으로 반올림한다 (`halfExpand`).
- 시각 조립 방식(`Date` 메서드, 시간대 약칭만 `formatToParts`)과 잘못된 시각 문자열의 실패 처리 (9절).
- 역할, 모드, 전송 경로는 표시 이름으로 바꾸고 목록에 없는 값은 원문으로 둔다 (9절). 스펙 원문 표기와 같은지 확인 필요 (차이 후보). `max_entries`는 표시하지 않는다.
- 게이지 상한 표기는 `상한 {n}`(CPY-079)과 `상한 확인 불가`(CPY-080)를 쓴다. 사용자 결정 2026-10-23(Q55)로 스펙 9.3.2절에 더했다.
- 사유 필드 `*_unavailable_reason`은 읽지 않는다. `null`만 확인 불가로 보인다. 계약 소비자 규칙과 결과가 같다 (구현 판단, 사용자 결정 없음).
- `language.js`를 `app/`으로 옮기고 `SHARED_FILES`에 더했다 (7 → 8). `login/language.js`는 생성 복사본이다. 내용은 같고 머리글 한 줄만 붙는다. 로그인 코드 동작은 바뀌지 않는다. `frontend.md` 1.1절 트리와 2.3절 복사 목록, `xtask.md`, `code-map.md`를 같은 변경으로 고쳤다.
- `createDataTable`의 `mobileHead` 옵션 이름과 두 값은 공통 API로 정했다. 첫 줄 배치·레이블 생략·명시적 표 역할은 DESIGN 554~566행을 따른다. 역할 생략은 구현 판단에서 제외한다(13절).

## 15. 열린 항목

- 문서 상태는 Draft다 (구현 완료, 대조 부분 완료: 대조 기록 참고).
- 해결한 사용자 질문 (2026-10-23):
  - Q1 → 사용자 결정 2026-10-23(Q53): D1 (a)의 한계를 받아들였다. 계약 6절의 전송량 안전 범위 항목을 닫았다.
  - Q2 → 사용자 결정 2026-10-23(Q54): 로그아웃 403은 CPY-076 경로다. 403 뒤 관리 세션 잔존을 감수한다.
  - Q3 → 사용자 결정 2026-10-23(Q55): 스펙 9.3절에 CPY-078(주 메뉴/Main menu), CPY-079(상한 {n}/Limit {n}), CPY-080(상한 확인 불가/Limit unavailable), CPY-081(세션 정보/Session information), CPY-082(전송량/Transfer)를 더했다. 게이지 `12 / 200` 형식으로 바꾸지 않는다.
  - Q3 보충(4.11 복사 검토): 문서 제목 `VNC Relay 운영 현황`·`VNC Relay 세션 {id}`, 역할 표시 `Viewer`·`대상`, 상태 라벨 `상태`/`State`는 질문에서 뺐다. 문서 제목은 브랜드와 CPY-004 `운영 현황`·CPY-051 `세션 {session_id}`를 잇는다. SCR-001 목록의 피어 칸 역할 이름은 CPY-056·057 주소 이름으로 바꿨다. 상태 행은 삭제했다. 기간 단위 `일`/`d`는 DESIGN 491행 근거라 질문이 아니다.
- 남은 확인 필요 항목: 역할·모드·전송 경로 표기와 스펙 "번역하지 않음"의 관계(14절 구현 판단, 대조 기록). 사용자 질문으로 아직 올리지 않았다.
- SCR-001 목록 카드의 개수 배지는 CPY 승인 대기다. DESIGN 608·626·636·638행의 시각 기준만으로 문구를 새로 만들지 않는다. 승인 전에는 표시하지 않는다. D6의 `closing` 생략과 CPY-040 바이트 머리글은 유지한다.
- 반영 완료(2026-10-22 Main 결정): D4 상세 링크는 `/#/sessions/{session_id}`로 두고 숨기지 않는다(14절). 검증 기록 4.6의 수정 필요 6건은 코드 수정을 마쳤다(4.6 기록).
- 반영 완료(2026-10-22): `code-map.md`, `dashboard-components.md` 2절·3.1절, `architectures/README.md`, `frontend.md` 1.1·2.3절, `xtask.md`, `management-api-v1.md` 5절과 6절(`tests/dashboard_browser.rs` 추가). 5절 결과 열은 검증 기록 규칙(`docs/AGENTS.md` 검증 기록)에 따라 부분 행을 `미실행`으로 두고, 부분 내용은 실행 기록 열에 적는다.
- 실서버 시험 설계·본문은 12절에 추가했다(2026-11-05). 실행 결과와 DASH-015·023의 수동 결과는 별도로 기록한다.

## 대조 기록

| 대조 범위 | 환경 | 확인 방법 | 결과 |
| --- | --- | --- | --- |
| 2~14절 구조·책임·흐름·표시 규칙과 소스 | Windows 10.0.26200 x64, 코드 `3944dc2`, 문서 기준 `3fbbead` | 읽기 전용 감사. 절마다 파일과 심볼을 읽어 대조했다. 사람 검토가 아니다. | 코드와 다르던 설명을 고쳤다 (해시 라우터, 첫 조회 빈 행, 로그아웃 버튼 상태, 세대 번호 지점, 시각 실패 처리, 표 role, 게이지 문구). 판단이 필요한 차이는 아래 표에 남겼다. |
| 시험 DASH-001·002·003·004·005·008·016·019·020·021·022·024·025 (SCR-001) | WSL `vr-full.sh`, Windows `vr-win.ps1 -Only all`, 기준 `3944dc2` | 시나리오 보고서와 `tests/dashboard_browser.rs` 마커 대조 | 13개 사례 모두 부분. 남은 조건은 12절과 검증 기록 4.1~4.5를 따른다. |
| 스크린샷 비교 SCR-001 (1280·390·320px, ko·en, 상태 8종 48장) | Windows 10.0.26200 x64, Chrome 155.0.8059.84 헤드리스, 기준 `bd1032b` + 미커밋 수정 | `target/dashboard-shots/` (커밋하지 않음), 검증 기록 4.6 | 재촬영(2026-10-22, 수정 뒤): 가로 스크롤 0건, CSP 위반 0건. 수정 필요 6건은 수정을 마쳤다(검증 기록 4.6). 1280은 정상 상태와 빈 목록 문구만 판정했다. 사람 비교는 4.12 |
| 시험 DASH-006·007·009~014·016·019·022·024·025·026 (SCR-002) | Windows 10.0.26200 x64, 작업 트리 | `tests/dashboard_browser.rs`의 `session_` 시험 14개와 `router_opens_session_detail_from_the_list_in_browser`(`session.html`·`router.html`, 목 응답) | 모두 부분. DASH-015는 표식만 달았다(4.12). 실서버 확인은 4.4·4.5 뒤. 검증 기록 4.7, 4.9 |
| 스크린샷 비교 SCR-002 (상태 53장: 정상·종료 확인 × 1280·390·320 × ko·en 12장, 나머지 상태 41장) | Windows 10.0.26200 x64, Chrome 헤드리스 | `target/dashboard-shots/scr/`(커밋하지 않음, 기록 `scr/scr002-capture.json`·`scr/scr002-capture-stale-failed.json`·`scr/scr002-capture-gone.json`·`scr/scr002-capture-normal.json`), 검증 기록 4.7 | 53장 모두 가로 넘침 0, CSP 위반 0. `98b2fc1`에서 대상 없음 5장과 정상 6장을 다시 찍었다(모두 `pass`). 상태 판정은 4.7절 표. 사람 비교는 4.12 |
| 수동 확인 3.4(시안 대조)·3.5(키보드)·3.6(스크린 리더)·3.7(Firefox·Safari·390px 실기기) | — | 검증 기록 3절 | 미실행 |
| 실서버 확인(MGT-026 값 일치, 세션 만료, `P` 설정 반영) | — | 4.4 뒤 | 미실행 |

### 차이

| 항목 | 이전 설계 | 코드 | 판단 | 조치 |
| --- | --- | --- | --- | --- |
| 목록 상태 안내 위치 | 머리말 아래 전체 폭 | 각 목록 카드 맨 위 (`applyChrome`) | 구현 판단. 스크린샷 비교에서 시안의 화면 배너와 다른 것으로 확인 (허용 차이, 검증 기록 4.6) | 4절 맞춤 |
| 첫 조회 중 목록 | 표 자리 없음 | 표 안 빈 행에 CPY-016 | 구현 판단 | 5절 맞춤 |
| 새로고침 `busy` | 조회 진행 중 `busy` | 로그아웃 진행 중 `disabled`만 | 차이 | 6절에 현재 동작 기록. 스펙 4절 대조 후 결정 |
| 세대 번호 올림 지점 | 화면 전환·언마운트 포함 | 세션 상실(`endSession`)·로그아웃(`signOut`)만 확인 | 차이 (해시 라우터 없음) | 6절 맞춤. 언마운트 시 올림은 미확인 |
| 로그아웃 결과 (204 외) | 완료로 보지 않음 (처리 경로 미정) | CPY-076 경로 (204만 완료) | 구현 판단 | 7절 표 맞춤 |
| 로그아웃 토큰 없음 | 기록 없음 | `getSession`으로 먼저 받음 | 구현 판단 | 8절 추가 |
| 잘못된 시각 문자열 | CPY-035 표시 | 응답 불일치로 실패 처리 | 구현 판단. 스펙 9.2절 적용 범위와 대조 필요 | 9절 맞춤 |
| 사유 필드 `*_unavailable_reason` | 사유가 있으면 확인 불가로 표시 | 사유를 읽지 않고 `null`만 확인 불가 | 차이 (계약 소비자 규칙과 결과 같음) | 14절 구현 판단 |
| 역할·모드·전송 경로 표기 | 번역하지 않음 (스펙 3절 L163) | 스펙 표기로 바꿈. 목록 밖 값은 원문 | 허용 차이 후보 | 14절 확인 항목 |
| 게이지 상한 문구 | 숫자만 (`12 / 200`) | `상한 {n}`, `상한 확인 불가` | 스펙 9.3.2절 CPY-079·CPY-080 (사용자 결정 2026-10-23(Q55)) | 해소 |
| 표 role | 명시적 role 속성 | 네이티브 요소, role 속성 없음 | 구현 판단 | 13절 맞춤. 3.6절에서 AT 확인 |
| 상단 막대 (390·320) | 목업: `운영 현황` 숨김, 56px 한 행, 활성 인디케이터 top 53 (DESIGN.md 550·641행) | 수정 뒤(2026-10-22): 390·320 ko·en 막대 56px 한 행, 언어 선택 top 6·높이 44, `운영 현황` 항목 미표시(인디케이터 없음). 1280은 인디케이터 top 53, 간격 7, 두 행 없음 | 수정 완료 (구현). 목업과 같은 배치 | 검증 기록 4.6 수정 필요 1–5 해소 |
| 서버 요약 `Unavailable` 값 | 한 줄 표시 (스펙 6절, 필수 정보가 읽힘) | 수정 뒤: 1280·390·320 en 정상 상태에서 `Unavailable` 한 줄(320 en 스크린샷 확인, 완전 문자열 텍스트 확인). 원인은 `overflow-wrap: anywhere`가 값 줄의 최소 폭을 줄인 것이다 | 수정 완료 (구현). [INFERENCE] 스펙 6절 기준 | 검증 기록 4.6 수정 필요 6 해소 |
| SCR-002 배치 | 스펙 99–109행 ASCII: `목록으로`는 왼쪽 위, `세션 종료`는 본문 아래 줄. 목업: 전송량을 정보 카드 옆 별도 카드로 둠 | `목록으로`·`새로고침`은 머리말 줄. 알림 조치 `상태 재조회`는 알림 안. 세션 정보 카드(8칸)와 전송량 카드(4칸)를 나란히 둠. `세션 종료`는 정보 카드 바닥(`card-foot`). 1023.98px 이하에서 카드가 한 줄씩 쌓이고 `.fact` 행도 한 열이 된다(단일 중단점) | 수정 완료 (구현). 목업의 두 카드 배치와 같다. 스펙 ASCII의 `본문 아래 줄`은 정보 카드 바닥으로 읽었다 [INFERENCE] | 검증 기록 4.7절 |
| SCR-002 모드·전송 경로 표시 | 목업: 배지 칩 | `createBadge` 중립 배지 (아이콘 없음, DESIGN 626행) | 수정 완료 (구현) | 검증 기록 4.7절 |
| SCR-002 주소·전송량 표시 | 목업: monospace 주소, dt/dd 목록 | 정보 카드는 `dl.fact-list`(dt/dd). 주소 값은 `.fact-address`(monospace). 전송량 카드는 `createStat`에 방향 표식(`directionStat`)을 붙인다. DESIGN 378행: `chart-1` Viewer->대상, `chart-2` 대상->Viewer, 10px 정사각형, 장식 | 수정 완료 (구현). 카드 제목 `세션 정보`·`전송량`은 CPY-081·CPY-082 (사용자 결정 2026-10-23(Q55)) | 검증 기록 4.7절 |
| SCR-002 상태 행 | 목업: 상태 행 없음 | 상태 행 삭제, `stateLabel` 제거. DASH-006이 사실 행 다섯 개와 상태 행 없음을 확인 | 수정 완료 (구현). 15절 Q3 보충에서 뺌 | 검증 기록 4.7절 |
| SCR-002 세션 종료 아이콘 | 목업·DESIGN 623행: 원 안 가로줄 아이콘 | 첫 렌더 뒤 아이콘이 사라졌다(`textContent` 덮어쓰기). `cancel` 아이콘을 붙이고 라벨은 `.button-label`만 갱신 | 수정 완료 (구현). 정상·종료 확인 캡처 12장에서 아이콘 확인 | 검증 기록 4.7절 |
| SCR-002 새로고침 이름 | 스펙 CPY-008 `새로고침`(머리말), CPY-014 `상태 재조회`(알림 조치) | 머리말 `새로고침`(`session-refresh`, outline), 알림 조치 `상태 재조회`(`recheck`, 알림 종류가 바뀔 때만 다시 그림) | 수정 완료 (사용자 지시). 시안 대조 해소 | 검증 기록 4.7절 |
| SCR-002 세션 id 글꼴 | 목업: monospace | 데이터 글꼴 토큰(`--type-data-*`, `.session-id`). DESIGN 473행 | 구현 판단 (DESIGN 우선). 시안과 다름 | 검증 기록 4.7절 남은 차이 1. 사람 판단 4.12 |
| SCR-002 조회 실패 중 세션 종료 | 스펙 §4 l.175: 조회 실패의 가능한 행동은 명시적 재조회뿐 | `canTerminate`에 `!query.failed` 추가. 이전 데이터가 있어도 종료 비활성. 수정 전에는 종료가 켜져 있었다 | 수정 완료 (스펙 기준) | 검증 기록 4.7절, 캡처 `stale-failed` |
| SCR-002 대상 없음(gone) 카드 본문 | 스펙 §4 l.177: 종료 비활성, 목록으로 | 중립 알림 `현재 세션을 찾을 수 없음`(CPY-059)과 알림 조치 `목록으로`(CPY-010, 목록 링크). 정보 카드는 제목만 남고 본문이 비어 있다. 세션 종료 비활성. 전송량 카드와 머리말 새로고침 숨김. 포커스는 알림의 `목록으로` | 수정 완료 (스펙 §4 l.177·§6 l.211 기준). 빈 카드 본문은 시안 프레임과 같아 유지 | 검증 기록 4.7절 |
| SCR-002 대상 없음 머리말 새로고침 | 시안 `scr002-gone`: 머리말 `새로고침` 있음 | 숨김. 스펙 §4 l.177이 대상 없음의 가능한 행동을 목록으로 이동뿐이라고 정한다 | 스펙 기준. 시안과 다름 | 검증 기록 4.7절 남은 차이 6 |
| SCR-002 대상 없음 알림 제목·본문 | 시안: 제목 `대상 없음`, 본문 `현재 세션을 찾을 수 없다. 같은 중계 ID의 다른 세션으로 바꾸지 않는다.` | CPY-059 `현재 세션을 찾을 수 없음` 한 문장만 씀. 스펙 §9.3에 두 문구의 키가 없어 추가하지 않았다. 같은 중계 ID 규칙은 스펙 §5 l.200에 따라 화면 동작으로 지킨다 | 구현 판단. 시안과 다름 | 검증 기록 4.7절 남은 차이 7. 문구를 넣으려면 스펙 §9.3 키 추가가 먼저 필요하다 |
| SCR-001 표 역할 (2026-10-23, 4.12) | 위 `표 role` 행의 당시 기록: 역할 속성 생략 | 네이티브 요소에 table·rowgroup·row·columnheader·cell을 명시한다. 빈 행·빈 셀도 포함한다 | DESIGN 565행으로 해소. 역할 생략은 구현 판단이 아니다 | 13절·공통 컴포넌트 3.1절 반영. 자동 역할 확인 통과. AT 검증은 미실행(검증 기록 4.15절) |
| SCR-001 모바일 첫 줄 (2026-10-23, 4.12) | 모든 셀이 레이블·값 쌍 | `mobileHead`로 대기 중계 ID·역할, 활성 세션 링크·상세, 종료 세션·원인을 첫 줄에 둔다. 첫 줄 레이블은 없다 | DESIGN 554~566행으로 해소 | 1023·390·320 ko와 320 en 캡처·DOM 위치 확인 통과(검증 기록 4.15절) |
| SCR-001 데이터·링크 (2026-10-23, 4.12) | 원문 일반 글씨, 상세 링크만 제공 | 대기·종료 ID는 `.session-id`, 주소는 `.fact-address`다. 활성 식별자는 `createLink`와 인코딩한 세션 URL을 쓴다 | DESIGN 473·644행으로 해소 | 10절 반영. 데이터 글꼴·주소 인코딩 시험과 Chrome 캡처 확인(검증 기록 4.15절) |
| SCR-001 배지·방향 (2026-10-23, 4.12) | 역할·모드·전송 경로·종료 원인은 일반 글씨, 바이트에 방향 표식 없음 | 중립 분류 배지, 종료 원인별 warning·destructive·neutral 배지, 장식 방향 표식을 쓴다 | DESIGN 378·393~395·608·622~626·636·643행으로 해소 | 10절 반영. 톤·아이콘·장식 속성 시험과 Chrome 캡처 확인. 모바일 배지 늘어남도 수정(검증 기록 4.15절) |
| SCR-001 상세 조작 (2026-10-23, 4.12) | 상세 링크에 아이콘 없음 | 오른쪽 꺾쇠와 표 안 최소 높이 sm·모바일 touch를 적용한다 | DESIGN 579·623·634행으로 해소 | 10절 반영. 아이콘·높이 시험과 Chrome 캡처 확인(검증 기록 4.15절) |
| SCR-001 연결 카드 설명 (2026-10-23, 4.12) | `usageGauge`가 본문 아래 `note`를 붙임 | `panelCard.description`을 `createCard`에 넘긴다. 제목 아래 `p.card-description`으로 옮기고 `usageGauge.note`를 없앤다 | DESIGN 527·638행으로 해소 | 4·10절·공통 컴포넌트 3.1절 반영. DOM 순서 시험과 Chrome 캡처 확인(검증 기록 4.15절) |
| SCR-001 시안과 스펙 경계 (2026-10-23, 4.12) | 시안의 개수 배지·closing·바이트 머리글과 스펙 문구의 차이 | 개수 배지는 CPY 승인 전 미표시. `closing`은 D6으로 생략. 바이트 머리글은 CPY-040 유지 | 개수 배지는 승인 대기. 나머지는 스펙 우선으로 해소 | 10·15절 반영. 남은 차이는 검증 기록 4.15절에 있다 |
