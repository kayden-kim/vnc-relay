# 운영 대시보드 앱 구조 (SCR-001·002·004·005·006·007·008)

| 항목 | 값 |
| --- | --- |
| 구조 시점 | Current |
| 문서 상태 | Draft |
| 구현 대조 | Partial — [대조 기록](#대조-기록) |
| 코드 위치 | `web/dashboard/app/`의 `main.js`, `router.js`(`mountRouter`, `routeOf`), `operations.js`(`mountOperations`, `endSession`, `signOut`), `session.js`(`mountSession`), `metrics.js`(`mountMetrics`), `metrics-model.js`, `sources.js`(`mountSources`), `sources-model.js`, `shell.js`(`buildNavbar`, `signOutUrl`), `query.js`(`newQuery`, `beginQuery`, `showBusy`, `endQuery`, `viewOf`), `model.js`, `api.js`(`getQuery`, `getSession`, `deleteSession`, `terminateSession`), `panels.js`(`statusAlert`, `statusBadge`, `usageGauge`, `waitingTable`, `closedTable`, `logsLink`, `panelCard`, `applyChrome`, `setContent`, `timeStack`), `format.js`, `copy.js`, `language.js`, `app.css`. `crates/adapters/src/dashboard_assets.rs`(`ASSETS`), `crates/adapters/src/management_http.rs`(`static_answer`), `xtask/src/tokens/shared.rs`(`SHARED_FILES`) |
| 코드 지도 | [코드 지도](code-map.md) (`web/dashboard/` 행, `dashboard_assets.rs` 행, xtask 행), [컴포넌트 구조](dashboard-components.md) |
| 관련 스펙 | [화면 스펙](../specs/dashboard.md) 3·4·5·6·9절; [디자인 기준](../design/DESIGN.md) 화면 구성(522~534행), 중단점, 표시 형식(489~492행), [메트릭 화면 확장](../design/DESIGN.md#메트릭-화면-확장), [접속 출처 화면 확장](../design/DESIGN.md#접속-출처-화면-확장); [프런트엔드 규약](../conventions/frontend.md) 1.1·1.2·1.3·3·4절; [시험 규약](../conventions/testing.md) 상태 규칙(23·99·100행) |
| 관련 계약 | [관리 API 계약 v1](../contracts/management-api-v1.md) 1.3·1.4·1.5·1.6~1.12·1.16·1.17·2.2·2.3절 |
| 관련 ADR | 없음 |

이 문서는 `web/dashboard/app/`의 파일 배치, 모듈 책임, 조회·갱신 흐름, 실패 처리, 로그아웃, 표시 형식을 정한다. 화면 문구는 스펙 9절이, 필드 의미는 계약이 정한다. 미확인 차이는 [열린 항목](#15-열린-항목)에 둔다.

## 1. 범위

- SCR-001 운영 현황과 앱 셸은 진입점·관리 세션 확인·언어 선택·로그아웃을 제공한다.
- SCR-002는 본문·상세 조회·종료 흐름과 해시 라우터를 쓴다. SCR-001 목록 링크는 계약 1.11절의 `/#/sessions/{session_id}`다.
- 해시(`#/sessions/…`)는 서버로 가지 않는다. 서버는 `/`와 같은 응답을 준다: 세션이 있으면 200 HTML, 없으면 303 `/login/`. 404 JSON이 아니다.
- `router.js`는 세션 상세·설정 보기·설정 편집·최근 로그·메트릭·접속 출처 해시를 각 화면으로 보낸다. 그 밖의 해시는 운영 현황으로 보낸다. `#/logs?session_id=…`의 값은 `logs.js`가 API 요청 전에 검사한다.
- 이 문서가 정하지 않는 것: 관리 API 필드 의미(계약), 화면 문구(스펙 9절), 색과 간격 토큰(DESIGN.md).

## 2. 파일 배치

```text
web/dashboard/app/
├── index.html         제공 경로 `/`. `/app/index.html`은 제공하지 않는다
├── main.js            진입점. mountRouter(document.getElementById('app-root')) 호출
├── router.js          운영 현황·세션 상세·설정 보기·편집·로그 라우팅. 전환 전 기존 화면 dispose
├── operations.js      SCR-001 조립, 세션 확인, 조회 루프, 세대 번호, 로그아웃, 언어 상태
├── session.js         SCR-002 조립, 상세 조회, 종료 확인·결과, 대상 소멸, 세대 번호
├── logs.js            SCR-006 필터, 증분 커서, 이전 페이지, 상세 펼침, 다운로드 수집·정리
├── metrics.js         SCR-007 상태·조회 루프·화면 조립·언어 전환·해제
├── metrics-model.js   메트릭 응답 검증, 추이 합치기, 카드·차트·표·프로세스 계산 (DOM 없음)
├── sources.js         SCR-008 상태·조회 루프·집계 기간·화면 조립·언어 전환·해제
├── sources-model.js   접속 출처 응답 검증, 요약 값, 표 개수, 출처 표시, 국가 이름 (DOM 없음)
├── settings.js        SCR-004·005 설정 보기·편집, 요청·입력·진단·저장 결과
├── settings-model.js  설정 v4 응답 검증과 편집 대상 입력·진단
├── shell.js           보호 화면의 상단 막대, 언어 선택, 로그아웃 결과 판정
├── query.js           DOM 없는 조회 상태와 갱신 타이머
├── model.js           응답 검증. 맞지 않으면 invalid
├── panels.js          서버 요약·연결·목록 카드, 상태 안내, 공유 카드 틀과 갱신
├── api.js             fetch 헤더, 시간 제한, 오류 분류
├── format.js          시각, 기간, 바이트, 숫자, 알 수 없는 값
├── copy.js            한국어·English 문구. 키는 CPY ID
├── language.js        언어 선택·저장. login/language.js의 생성 원본
├── components/        공통 조각 13개. login 복사본의 원본
├── tokens.css         토큰 생성 파일
├── app.css            보호 화면 전용 CSS
└── styles/
    ├── base.css       공통 원본
    └── components.css 공통 조각·배치·중단점 원본
```

규칙:
- `index.html`은 절대 경로(`/app/...`)로 파일을 가리킨다. 인라인 script, 인라인 `<style>`, `style` 속성, `on…` 속성, `javascript:` URL을 쓰지 않는다 (frontend.md 1.3절).
- `app/`과 `login/`은 파일을 공유하지 않는다 (스펙 L15, frontend.md 1.1절 L78). 같은 내용의 복사본만 `cargo xtask tokens`가 만든다. 복사 방향은 `app/`(원본)에서 `login/`(복사본)이다 (`xtask/src/tokens/shared.rs`).
- `xtask/src/tokens/shared.rs`의 `COPY_DIRS`(`components`, `styles`)는 login 쪽 복사 폴더만 검사한다. app 전용 파일은 `app/` 아래 어디에 두어도 `--check`에 걸리지 않는다.

## 3. 모듈 책임

| 파일 | 책임 | 테스트 주입 (deps) |
| --- | --- | --- |
| `main.js` | `mountRouter(document.getElementById('app-root'))`만 호출 | 없음 |
| `operations.js` | `mountOperations(root, deps)`: 세션 확인(`GET /api/v1/session`), 네 조회 루프, 세대 번호, 로그아웃, 언어 상태 | `request`(fetch 대체), `storage`, `preferredLanguages`, `navigate`(페이지 이동 대체) |
| `model.js` | 응답 검증: `parseSession`·`parseSummary`·`parseWaiting`·`parseSessions`·`parseClosed`. 맞지 않으면 `invalid` 오류 | 없음 |
| `api.js` | 요청 헤더, 시간 제한(AbortController), 응답을 `ApiError`의 kind로 분류. 로그아웃은 204만 성공 | `request` 인자로 받음 |
| `panels.js` | SCR-001 카드 조립: 서버 요약, 연결 카드, 목록 카드 세 개와 목록 상태 안내 | 없음 |
| `format.js` | 시각·기간·바이트·숫자 표시, 역할·모드·전송 경로 표시 이름 | 없음 (`Intl`과 `Date`만 사용) |
| `copy.js` | `ko`·`en` 문구 객체. 각 키에 CPY ID 주석 | 없음 |
| `language.js` | `LANGUAGES`, `chooseLanguage`, `readSavedLanguage`, `saveLanguage` (login과 같은 저장 키). login/language.js는 이 파일의 복사본 | `storage`, `preferredLanguages` |
| `logs.js` | `mountLogs(root, deps, { sessionId })`: 최근 로그 상태와 조회를 소유한다. 다른 화면으로 이동하면 요청·타이머·보호 데이터를 제거한다. | `request`, `storage`, `preferredLanguages`, `navigate`, `replaceUrl` |
| `metrics.js` | `mountMetrics(root, deps)`: 메트릭 상태와 조회, 추이 기간 선택을 소유한다. 다른 화면으로 이동하면 요청·타이머·보호 데이터와 고른 기간을 제거한다. | `request`, `storage`, `preferredLanguages`, `navigate` |
| `metrics-model.js` | `HISTORY_WINDOWS`, `parseMetrics`, `mergeHistory`, `keyIndicators`, `buildCharts`, `distributionItems`, `causeRows`, `processRows`. DOM과 시계를 쓰지 않는다. 맞지 않는 응답과 추이 간격·개수가 요청한 기간과 다른 응답은 `invalid` 오류 | 없음 |
| `sources.js` | `mountSources(root, deps)`: 접속 출처 상태와 조회, 집계 기간 선택을 소유한다. 다른 화면으로 이동하면 요청·타이머·보호 데이터와 고른 기간을 제거한다. | `request`, `storage`, `preferredLanguages`, `navigate` |
| `sources-model.js` | `SUMMARY_KEYS`, `DATABASE_ROWS`, `parseSources`, `storageLimited`, `summaryValues`, `tableCount`, `attributedProviders`, `countryLabel`. DOM과 시계를 쓰지 않는다. 맞지 않는 응답과 `window`가 요청한 기간과 다른 응답은 `invalid` 오류 | 없음 |

화면은 `createNavbar`, `createLanguageSelect`(variant `navbar`), `createButton`, `createCard`, `createStat`, `createGauge`, `createDataTable`, `createBadge`, `createAlert`, `createLink`, `icon`을 쓴다. SCR-002의 종료 확인은 `createDialog`를 쓴다. 공개 API는 [컴포넌트 구조](dashboard-components.md#31-공개-함수)를 따른다.

### 설정 보기·편집

- SCR-004는 v4 계약의 테이블 순서를 유지하며 ticket 키도 `[relay.ticket.viewer]`·`[relay.ticket.server]`로 묶는다. SCR-005의 `editableItems`는 allow 6개 → trusted_proxies 2개 → allowed_origins 4개 → Mode I 목록 → 상한·제한 시간·연결 유지 → 접속 ticket 순서다. 마지막 구역은 Viewer 다음 Server이며 각 역할의 `enabled`·`issuer`·`relay_host`·`max_lifetime_secs` 순서다. 비활성 진입점의 편집 불가 키와 공개 키 파일 경로는 입력 목록에서 빠진다. 확인 대화상자는 허용 목록·프록시 목록·Origin 목록의 빈 값 경고(CPY-105·183·192)를 각각 고른다. ticket `enabled`의 false→true 경고(CPY-194)는 그 키의 변경 항목 안에 둔다.

## 4. 화면 골격 (SCR-001)

- 상단 막대는 `shell.js` `buildNavbar`를 쓴다. 브랜드, 운영 현황·메트릭·접속 출처·로그·설정, 언어 선택과 로그아웃을 조합한다. 현재 화면은 `aria-current`로 표시한다. 모바일에서도 다섯 항목을 둘째 줄에 보이며 필요한 경우 줄바꿈한다. `nav` 이름은 CPY-078이다. 운영 현황 항목의 라벨은 CPY-294(한국어 "운영 현황", English "Overview")이고 페이지 제목은 CPY-004 그대로다. 메트릭 항목의 아이콘은 `metrics`이고 라벨은 CPY-202다. 접속 출처 항목의 아이콘은 `pin`이고 라벨은 CPY-255다.
- 랜드마크: 상단 막대는 `nav`, 본문은 `main`이다. `nav` 이름은 CPY-078 한국어 "주 메뉴", English "Main menu"다.
- 페이지 머리말: 제목 "운영 현황"(CPY-004), 줄 "마지막 성공 갱신: {시각}"(CPY-015), [새로고침] 버튼(CPY-008).
- 상태 안내는 각 목록 카드 맨 위에 둔다(`operations.js`의 `applyChrome`). 머리말 아래 전체 폭 안내는 두지 않는다.
- 서버 요약 카드(6열): 실행 상태, 가동 시간, 대기 연결 수, 활성 세션 수, 초기 교환 중, Mode I 대상 연결 중. 모바일은 실행 상태·가동 시간을 한 줄 전체 폭, 나머지는 2열이다.
- 연결 카드 두 개(각 3열): 중계 연결, 관리 연결. `panelCard`는 CPY-032·034를 `createCard({description})`에 넘긴다. 설명은 `.card-head` 제목 아래 `p.card-description`이다. `usageGauge({t, title, usage, kind})`는 고리만 만들며 `note`를 받지 않는다. `createGauge`로 현재 수와 `상한 {n}`(CPY-079)을 보인다. 상한이 없으면 CPY-080 `상한 확인 불가`다.
- 목록 카드 세 개(전체 폭, 이 순서): 대기 연결(CPY-036), 활성 세션(CPY-039), 최근 종료 목록(CPY-042).
- 목록 제목 옆 개수는 CPY-084·085다. `operations.js`의 `renderList`는 마지막 성공 응답 `query.data.items.length`를 `applyListCount(slots, count, t)`에 넘긴다. 성공 데이터가 없으면 `count=null`로 배지를 숨긴다. 요약 수를 가져오지 않으므로 독립 갱신·오래된 목록에서도 개수와 표가 같은 응답을 따른다.
- `panelCard`는 `.card-title`의 `titleSlot`과 `countBadge`를 보관한다. `applyListCount`는 아이콘 없는 neutral `createBadge`를 쓴다. 수가 바뀌면 같은 배지를 갱신하고 알 수 없으면 제거한다. `copy.js`의 `listCount`는 CPY-084, `listCountOne`은 CPY-085다. 언어 변경은 같은 목록 데이터로 문구를 다시 만든다.

## 5. 조회 상태 모델

조회마다 상태를 둔다.

```text
Query = { data,          // 마지막 성공 응답의 값 (없을 수 있음)
          lastSuccessAt, // 이 값을 적용한 클라이언트 시각
          inFlight,      // 요청 진행 중 여부
          busyShown,     // 진행 중 표시(조회 중·갱신 중)를 보이는지 여부
          busyTimer,     // 자동 폴링의 진행 중 표시를 늦추는 타이머
          failed }       // 마지막 조회가 실패했는지 여부
```

- 진행 중 표시는 `inFlight`와 `busyShown`이 모두 참일 때만 보인다(`query.js`의 `viewOf`). 요청 시작은 `beginQuery`, 끝은 `endQuery`가 맡는다.
- 사용자가 고른 조회는 `busyShown`을 바로 참으로 둔다. 자동 폴링은 `BUSY_DELAY_MS`(1000ms) 뒤에야 참으로 바꾸고 화면을 다시 그린다. 그 전에 끝나면 직전 상태(정상·오래된 데이터·실패)가 그대로 남는다. 주기 갱신마다 배지가 깜빡이지 않게 하려는 것이다([스펙 4.1절](../specs/dashboard.md#41-폴링), DASH-067).

목록 하나의 표시 규칙:

| 상태 | 데이터 | 표시 |
| --- | --- | --- |
| 첫 조회 중 | 없음 | 표 머리글 아래 빈 행(`data-table-empty`)에 CPY-016 "조회 중" 정보 배지(`statusBadge(t, 'loading')`, 카드 머리 배지와 같은 모양) (스펙 4절 최초 로딩 행, 구현 판단. 배지 모양은 DESIGN 상태 색) |
| 성공 | 빈 목록 | 표 머리글 유지, 빈 목록 문구 (CPY-038 / CPY-041 / CPY-044) |
| 성공 | 항목 있음 | 표 |
| 갱신 중 | 있음 | CPY-017 "갱신 중" 배지 + 이전 표 |
| 실패 | 없음 | CPY-018 + CPY-021 안내. 표 없음 (실패를 빈 목록으로 보이지 않음) |
| 실패 | 있음 | CPY-019 + CPY-020(`마지막 성공 시각 {lastSuccessAt}`) + 이전 표 |

- 오래된 데이터와 조회 실패 안내는 `createAlert`, 조회 중·갱신 중 표시는 `createBadge`로 그린다 (`panels.js`의 `statusAlert`·`statusBadge`).
- 조회 중·갱신 중은 info, 오래된 데이터는 warning, 조회 실패는 destructive다.
- "조회 중"은 카드 머리, 서버 요약 값(`summaryGrid`), 사용량 고리 가운데, 목록 빈 행 모두 같은 info 배지다. `usageGauge`는 `createGauge`에 `unknownTone: 'info'`를 넘긴다. "확인 불가"는 neutral 배지다.
- SCR-002 첫 조회 중에는 `loadingFactsBlock`이 모드·양쪽 전송 경로·시작 시각·양쪽 소켓 주소의 여섯 줄을 조회 중 배지로 그린다. 전달 주소는 응답 전에는 유무를 모르므로 행을 만들지 않는다. `bytesBlock(t, null)`은 두 방향 이름·표식·정보 배지를 그린다. 첫 조회 실패와 사라진 세션은 값을 그리지 않는다.
- 페이지 머리말의 "마지막 성공 갱신"은 네 조회의 `lastSuccessAt` 중 가장 최근 값이다 (구현 판단).
- 개수 배지도 같은 `data`를 따른다. 첫 성공 전에는 숨긴다. 성공한 빈 목록은 0건/0 items다. 갱신 중·실패 때 이전 표를 유지하면 그 개수도 유지한다. 첫 조회 실패를 0건으로 바꾸지 않는다.

## 6. 갱신 흐름

- 시작: 세션 확인(`GET /api/v1/session`)이 200이면 네 조회의 루프를 시작한다. 조회는 `summary`, `waiting`, `sessions`, `closed-sessions` 네 개다.
- 주기: 조회 하나의 응답(성공이든 실패든)을 받은 뒤 `poll_secs`(세션 응답의 `dashboard.poll_secs`) 뒤에 다음 요청을 보낸다. 타이머는 `setTimeout`으로 예약하며 `setInterval`은 쓰지 않는다. 응답을 기다리는 동안에는 같은 조회를 다시 보내지 않으므로 중복 전송이 없다 (계약 1.12절).
- P 값은 코드 상수로 두지 않는다 (frontend.md 1.3절, 계약 1.12절).
- 시간 제한: 조회 요청은 `query_timeout_secs`(세션 응답) 초과 시 중단하고 실패(kind `timeout`)로 처리한다 (계약 1.12절). 로그아웃은 조회가 아니므로 이 값을 쓰지 않는다 (7절).
- 자동 조회에만 `X-Relay-Poll: 1` 헤더를 붙인다. 계약 1.4절에 따라 이 헤더는 GET에만 쓰며, 자동 조회만 유휴 만료를 연장하지 않는다. 사용자 행동(화면 진입, 새로고침, 로그아웃)은 유휴 만료를 연장한다.
- 새로고침(CPY-008)은 진행 중이 아닌 조회만 즉시 다시 보낸다. 보류 타이머는 취소하고 다시 예약한다. 진행 중인 자동 조회가 배지를 보이지 않으면 `showBusy`로 즉시 갱신 중을 보인다(DASH-067). 버튼은 로그아웃 중에만 `disabled`다. 조회 중 버튼의 `busy` 표시는 없다.
- 세대 번호는 세션 상실(`endSession`)·로그아웃(`signOut`)·화면 해제(`dispose`) 때 올린다. 라우터는 전환 전에 이전 화면을 해제한다. 응답의 세대가 다르면 버린다(DASH-005).
- 세션 확인 실패(401이 아닌 오류): 페이지에 CPY-018·CPY-021 안내와 [새로고침]을 보여 주고, 새로고침으로 세션 확인을 다시 한다 (구현 판단).

## 7. 실패 처리

| 응답 | 조회 (SCR-001) | 로그아웃 | 세션 확인 |
| --- | --- | --- | --- |
| 200 | 적용, `lastSuccessAt` 갱신 | CPY-076 경로 (204만 완료, 아래 참고) | 적용 |
| 401 `unauthenticated` | 보호 데이터·루프 제거, `/login/`으로 이동 | 세션이 없으므로 `/login/`으로 정상 이동 | `/login/`으로 이동 |
| 403 `csrf_invalid` | 해당 없음(GET) | CPY-076 경로 | 해당 없음 |
| 404 `not_found` | 조회 실패로 처리 (구현 판단) | CPY-076 경로 (구현 판단) | 해당 없음 |
| 400 `invalid_request` | 조회 실패로 처리 (구현 판단) | CPY-076 경로 (구현 판단) | 실패 안내 |
| 500, 응답 없음, 시간 초과 | 실패. 이전 데이터 유지, 오래된 데이터 표시 (CPY-019~021) | CPY-076 경로 (아래 참고) | 실패 안내 |
| 알 수 없는 `code` | HTTP 등급(4xx·5xx)으로 판단 (계약 2.3절) | CPY-076 경로 (204 외) | 같음 |
| 2xx지만 JSON이 맞지 않음 | 실패로 처리, 이전 데이터 유지 (구현 판단) | CPY-076 경로 (204 외) | 실패 안내 |

- 이 표는 SCR-001을 다룬다. 로그인은 `web/dashboard/login/`이 맡는다.
- 로그아웃은 204이면 완료이고 401이면 이미 세션이 없어 정상 이동한다. 그 밖의 응답·응답 없음·시간 초과는 CPY-076 경로다. 403에서도 서버의 관리 세션은 유지되며 Si나 Sa까지 남을 수 있다(계약 1.4.3·2.3절).
- 로그아웃 시간 제한은 `app/api.js`의 별도 10초 상수다. `login/login.js`의 `DEFAULT_TIMEOUT_MS`(10000)와 값은 같지만 공유하지 않는다. 세션 응답의 `query_timeout_secs`·`terminate_timeout_secs`는 조회·종료에만 적용한다.

## 8. 로그아웃 (SCR-001)

1. [로그아웃] 클릭: 버튼을 `disabled`로 막아 중복 클릭을 막는다 (`busy` 표시는 없다).
2. 토큰이 메모리에 없으면 세션 확인(`getSession`)으로 먼저 받는다 (구현 판단). `DELETE /api/v1/session`을 `X-CSRF-Token` 헤더와 함께 보낸다. 자동 조회 헤더는 붙이지 않는다. 시간 제한은 10초다 (7절).
3. 결과에 따라 7절 표를 따른다.
4. 완료(204) 또는 세션 없음(401): 루프를 멈추고 `app-root` 안의 보호 DOM을 비운 뒤 `/login/`으로 이동한다.
5. 확인하지 못함(204 외 응답: 200·400·403·404·500 등, 응답 없음, 시간 초과): 루프를 멈추고 보호 DOM을 비운 뒤 `/login/?notice=logout-unconfirmed`로 이동한다. 로그인 화면은 이 쿼리를 읽어 CPY-076 안내를 보인 뒤 `history.replaceState`로 쿼리를 지운다. 알 수 없는 쿼리 값은 CPY-075 안내로 처리한다. 릴레이 세션은 끊지 않는다 (스펙 DASH-019).

CSRF 토큰은 메모리에만 둔다. 저장소(localStorage, sessionStorage)나 DOM 속성에 쓰지 않는다.

## 9. 표시 형식 (`format.js`)

- 시각은 `YYYY-MM-DD HH:mm:ss`와 시간대 약칭이다(예: `2026-10-16 14:03:25 GMT+9`). `Date`와 0 채움으로 조립하고 약칭만 `Intl.DateTimeFormat.formatToParts`(`en-US`)로 구한다. 두 언어의 순서·구분자는 같다.
- 잘못된 시각 문자열은 2xx JSON 불일치로 실패 처리한다. `null`에만 CPY-035를 쓴다.
- 바이트는 KiB·MiB 소수 한 자리다. KiB 반올림 값이 1024.0 이상이면 MiB다. 예: `0` → `0.0 KiB`, `256` → `0.3 KiB`, `1536` → `1.5 KiB`, `1048575` → `1.0 MiB`, `1572864` → `1.5 MiB`. 중간값은 0에서 먼 쪽으로 반올림한다(`halfExpand`). 숫자 자릿수 구분자는 화면 언어가 아닌 브라우저 언어 설정을 따른다.
- 바이트는 JS `Number`로 다룬다. `JSON.parse`는 2^53 이상의 정수를 가장 가까운 binary64 값으로 근사한다. 2^53−1까지 정확하며 `formatBytes`는 받은 Number를 계산한다. 구간 `[2^k, 2^(k+1))`의 최대 절대 오차는 `2^(k−53)` 바이트다. 2^70 미만이면 최대 65536바이트로 표시 한 칸인 0.1 MiB(104857.6바이트)보다 작다. 반올림 경계 근처에서 최대 한 칸 어긋날 수 있다. 2^70 이상은 한 칸을 넘을 수 있으며 시험하지 않는다.
- 기간: `D일 HH:MM:SS` (English `Dd HH:MM:SS`). 일 수가 0이면 일 부분을 뺀다.
- `null`은 CPY-035 "확인 불가"/"Unavailable"와 중립 색으로 보이며 0으로 바꾸지 않는다. `*_unavailable_reason`은 읽지 않는다. 알 수 없는 사유를 일반 확인 불가로 보이는 계약 소비자 규칙과 결과가 같다.
- 세션 식별자·중계 ID·주소는 계약 값을 보인다. 역할·모드·전송 경로는 표시 이름으로 바꾼다(`mode1`→`Mode I`, `tcp`→`TCP`, `viewer`→`Viewer`). 목록 밖 값은 원문이다.

## 10. 계약 필드 매핑

| 계약 필드 | 화면 | 표시 규칙 |
| --- | --- | --- |
| `summary.server.state` | 실행 상태 (CPY-023) | `ready` → CPY-024, `shutdown_grace` → CPY-025, 그 밖의 값은 받은 그대로 |
| `summary.server.uptime_ms` | 가동 시간 (CPY-026) | 기간 |
| `summary.counts.waiting` | 대기 연결 수 (CPY-027) | 숫자 |
| `summary.counts.active` | 활성 세션 수 (CPY-028) | 숫자 |
| `summary.counts.handshaking` + `_unavailable_reason` | 초기 교환 중(CPY-029) | 숫자, `null`이면 확인 불가. 사유 필드는 읽지 않음 |
| `summary.counts.target_connecting` + `_unavailable_reason` | Mode I 대상 연결 중(CPY-030) | 숫자, `null`이면 확인 불가. 사유 필드는 읽지 않음 |
| `summary.usage.relay_connections.current` / `limit` | 중계 연결(CPY-031·032) | 설명은 CPY-032. 게이지는 현재 수와 CPY-079. current가 null이면 확인 불가, limit가 null이면 CPY-080 |
| `summary.usage.admin_connections.current` / `limit` | 관리 연결 (CPY-033·034) | `panelCard.description`은 CPY-034. 게이지 규칙은 중계 연결과 같다 |
| `waiting.items[]`: `role`, `relay_id`, `transport`, `address`, `forwarded_ip`, `connected_at`, `waited_ms` | 대기 연결(CPY-036·037·197·199) | 머리글은 CPY-197이다. `relay_id`는 `.session-id`와 `mobileHead: 'primary'`, 역할은 아이콘 없는 중립 배지와 `'secondary'`다. 전송도 중립 배지다. `waitingAddressCell`은 레이블 없는 실제 소켓 주소와 값이 있을 때만 CPY-199 전달 주소 줄을 만든다. `addressLine`은 레이블을 값 위에 두며 두 주소 모두 `.fact-address`다. `timeStack`은 `timeParts`의 날짜와 시각·시간대를 폭과 관계없이 두 줄로 쌓는다. 글씨는 `formatTime` 결과다. 빈 목록은 CPY-038이다. |
| `sessions.items[]`: `session_id`, `mode`, `viewer_transport`, `target_transport`, `started_at`, 방향별 바이트·소켓 주소 | 활성 세션(CPY-039·187) | 링크는 `/#/sessions/${encodeURIComponent(sessionId)}`다. 전송은 Viewer → 대상 순서의 두 중립 배지다. 주소·바이트 방향을 유지한다. 시작 시각은 `timeStack`이다. `peerCell`은 CPY-056·057과 `addressLine`으로 레이블을 값 위에 두며 두 주소 모두 `.fact-address`다. 빈 목록은 CPY-041이다. |
| `sessions.items[].state` | 없음 | `closing` 배지를 표시하지 않는다. 머리글은 CPY-187이다. |
| `closed-sessions.items[]`: `session_id`, `close_cause`, `mode`, `viewer_transport`, `target_transport`, `started_at`, `ended_at`, `duration_ms`, 방향별 바이트, 양 끝 주소·전달 주소 | 최근 종료 목록(CPY-042·043·198·200·201) | 머리글은 CPY-198이며 전송·바이트는 `wrapLabel`로 낱말 사이에서 줄바꿈한다. ID는 `.session-id`·`mobileHead: 'primary'`, 원인 배지는 `'secondary'`다. 원인은 CPY-045~050·083으로 매핑하고 목록 밖 값은 원문이다. `connection_error`·`internal_error`는 destructive, `dead_connection`·`close_timeout`은 warning, 나머지는 neutral이다. 모드·전송은 중립 배지다. `closedTimeCell`은 CPY-055 시작, CPY-200 종료, CPY-201 지속 시간 세 줄이다. `timeLine`의 `timeParts`는 날짜·시각을 `data-table-nowrap` 인라인 블록으로 나눈다. 폭이 되면 날짜는 레이블 옆에, 부족하면 시각부터 다음 줄에 놓인다(`cell-line-flow`). 글씨는 `formatTime` 결과다. 지속 시간은 서버 `duration_ms`이며 시각 차이로 계산하지 않는다. `closedAddressCell`은 Viewer 주소·전달 주소·대상 주소·전달 주소 순서다. null 전달 주소 줄은 없다. 레이블은 `addressLine`으로 값 위에 둔다. `logsLink`의 CPY-182 링크는 `/#/logs?session_id=${encodeURIComponent(sessionId)}`다. 빈 목록은 CPY-044이며 `max_entries`는 표시하지 않는다. |

- 응답의 `as_of`는 화면에 쓰지 않는다. "마지막 성공 갱신"은 클라이언트가 응답을 적용한 시각이다 (구현 판단).
- 방향별 바이트는 두 줄이다. 각 줄은 `.direction-mark.direction-mark-1` 또는 `.direction-mark.direction-mark-2`, 흐린 방향 이름, 값 순서다. 표식은 `aria-hidden="true"`인 장식이다. Viewer→대상은 `chart-1`, 대상→Viewer는 `chart-2`다(DESIGN 378·643행).
- 활성 세션의 열 이름은 CPY-187이다. [상세] 링크는 `.button.button-outline`과 오른쪽 꺾쇠를 쓴다. 표 안 최소 높이는 데스크톱 `spacing.control-height-sm`, 모바일 `spacing.control-height-touch`다.
- 역할·모드·전송 경로와 중립 종료 원인 배지는 아이콘이 없다. warning·destructive 종료 원인 배지는 상태 아이콘을 함께 쓴다(DESIGN 608·622~626·636행).

## 11. 정적 제공과 서버 변경

- `crates/adapters/src/dashboard_assets.rs`의 `ASSETS`는 정확한 제공 경로를 지정한다. `/`는 `app/index.html`이며 앱 모듈·CSS·공통 조각·글꼴을 `/app/` 아래에 제공한다. `/app/index.html`은 제공하지 않는다.
- `management_http.rs`의 `static_answer`는 `/`에 세션이 있으면 200 HTML, 없으면 303 `/login/`을 준다.
- `tests/dashboard_assets.rs`의 `asset_table_lists_exactly_the_dashboard_files`가 표와 배포 파일 목록을 대조한다.
- `xtask/src/tokens/shared.rs`의 `SHARED_FILES` 8개에는 `language.js`가 있다. 원본은 `app/language.js`이고 `login/language.js`는 생성 머리글 한 줄을 붙인 같은 내용의 복사본이다.
- 정적 응답은 `Content-Security-Policy: default-src 'self'; frame-ancestors 'none'`, `Cache-Control: no-store`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`를 쓴다. `protected_files_are_served_with_a_session`이 `/`의 헤더를 확인한다.

## 12. 시험 설계

### 실서버 브라우저 시험

- 시험 전용 CDP(Chrome DevTools Protocol) 클라이언트가 `--remote-debugging-port`로 Chrome·Edge를 조작한다. 루트 개발 의존성 `tokio-tungstenite`·`serde_json`을 쓴다. 외부 드라이버나 새 crate는 필요 없다.
- `tests/common/dashboard_live.rs`는 실제 계정 해시·설정·`vnc-relay serve`·TCP 중계 연결을 준비한다. 시험 서버의 루프백 포트 다섯 개는 예약 소켓으로 고른다. 전달 프록시를 바인드한 뒤 예약을 놓고 `serve`를 시작한다. 놓은 포트를 병렬 시험의 다른 소켓이 먼저 가져가면 `serve`가 `listener_bind_failed`(종료 코드 4)로 끝난다. 이때만 같은 디렉터리에서 그 다섯 포트를 새 포트로 바꿔 최대 두 번 다시 시작한다. 새 포트는 설정 파일에 아직 없는 포트만 쓴다. 실패한 시작이 놓은 포트를 다시 받으면, 포트를 차례로 바꾸는 중에 두 리스너가 같은 포트로 바뀔 수 있기 때문이다. 다른 종료는 시험 실패다. `tests/common/cdp.rs`는 브라우저 실행·CDP 명령·탐색·DOM 조건 대기를 맡는 공통 클라이언트다. WebSocket 브라우저 시험(`tests/websocket_browser.rs`)도 같은 파일을 쓴다. `tests/common/dashboard_cdp.rs`는 같은 `Browser`에 입력·클릭·새로고침·화면 폭·로그인 동작을 더한다. `tests/common/dashboard_proxy.rs`는 전달과 장애 주입을 맡는다. `tests/dashboard_live.rs`의 25개 함수가 각각 한 행 전체의 표식을 소유한다. 화면별 본문은 `tests/dashboard_live/{login,overview,detail,cross}.rs`에 둔다. 공통 모듈과 하위 본문에는 표식을 붙이지 않는다.
- 시험 TCP 프록시는 실제 관리 리스너로 전달한다. 데이터와 쿠키는 서버 원본이다. 지연·응답 유실·연결 종료·계약 모양 500만 시험 쪽에서 주입한다. 요청 시각·동시 처리 수·실제 전달 수·주입 수를 구분하며 주입 오류를 서버 500의 증거로 쓰지 않는다.
- DASH-008은 `Si=300s`를 실제로 기다린다. 독립 쿠키의 운영 현황·상세 브라우저를 동시에 관찰한다. 각 쿠키의 마지막 수동 요청 시각부터 300초 이전에는 실제 401이 없어야 한다. 297초 뒤에도 정상 자동 폴링과 보호된 DOM을 확인한다. 그 뒤 첫 실제 401, 보호 데이터 제거, 로그인 이동과 폴링 중단을 확인한다. 단순히 301초 잠든 뒤 401이 있는지만 검사하지 않는다. CDP 명령마다 제한 시간을 두되 브라우저 전체에 60초 기한을 두지 않는다.
- `P` 기본값·설정값은 서로 다른 실제 프로세스에서 관찰한다. 긴 응답과 늦은 응답은 프록시의 실제 전송 지연으로 만든다. 시험 페이지의 fetch 교체는 실서버 증거로 세지 않는다.
- 초기 로딩 관찰은 `Fault::Hold`로 실제 응답을 보관한다. 세 응답이 보관 경계에 도달한 뒤 로딩·개수 배지 없음 검사를 마치고 하나의 플래그로 해제한다. 고정 지연 만료와 검사 사이의 경합을 피하기 위한 시험 구성이다. 보관 대기도 기존 LIMIT와 프록시 종료 신호를 따른다. 제품의 제한 시간·응답은 바꾸지 않는다.
- dump-dom 시험은 빠른 컴포넌트·형식 부분 증거다. DASH-015의 수동 접근성과 DASH-023의 네 브라우저 호환성은 별도로 확인한다.
- DASH-001·021의 확인 불가 값은 실제 조회 전달 실패로 만든다. 정상 서버의 정수 필드를 null로 위조하지 않는다. DASH-025의 미래 enum 값은 생산자가 낼 수 없는 입력이다. 일반 데이터·탐색은 실제 서버로 확인하고, 미래 값만 실제 브라우저에서 제품 파서·표 컴포넌트에 별도 Consumer 계약 입력으로 준다. 두 언어의 원문 값 보존과 번역된 머리글·행동을 확인한다. 이 입력은 API 응답이나 프록시 본문이 아니다.
- 한 시험 바이너리 안의 행은 직렬로 실행한다. 서로 다른 행의 브라우저 부하가 폴링 시각 관찰을 흔들지 않게 하기 위해서다. DASH-008의 운영 현황·상세 브라우저 두 개는 같은 행에서 동시에 실행한다.
- 원시 조회의 관찰 계정은 `bob`, 브라우저 계정은 `alice`다. 역할을 분리하기 위한 시험 구성이다. 같은 계정의 독립 세션을 금지하는 제품 정책은 아니다. 전달 프록시는 `Content-Length`·chunked·본문 없는 상태의 HTTP 프레이밍으로 응답 완료를 판정한다. 완전한 응답을 읽은 뒤 TCP FIN을 추가로 기다리거나, 후속 연결 reset 때문에 그 응답을 폐기하지 않는다.
- 프록시 accept 루프만 비차단이다. 수락 소켓은 제한 시간이 있는 차단 모드로 바꾼다. Windows는 비차단 모드를 상속하므로 그대로 쓰면 브라우저 사전 연결에서 요청 도착 전 `WouldBlock`(10035)로 소켓을 닫을 수 있다.
- 실패 진단은 프록시 전송 단계·상태·소요 시간·OS 오류, CDP 네트워크 실패·응답 상태, 서버 사건의 열거 값만 남긴다. 요청·응답 본문, 쿠키·비밀번호·토큰·헤더 값은 출력하지 않는다.

### 가짜 응답 시험


- 브라우저 시험 페이지: `web/dashboard/tests/operations.html`과 `operations-scenarios.js`. 로그인 시험과 같은 방식으로 `?case=` 값에 따라 가짜 응답을 준다. 결과는 `data-result`에 쓴다.
- 세션 상세 시험 페이지: `web/dashboard/tests/session.html`과 `session-scenarios.js`. 가짜 응답으로 사례를 돌리고, 결과는 `data-result`에 쓴다.
- 라우터 시험 페이지는 `web/dashboard/tests/router.html`·`router-scenarios.js`다. 제품 `mountRouter`와 목록 링크 `href`를 확인하고 같은 해시로 상세와 목록 복귀를 확인한다(DASH-006).
- SCR-001 부분 사례는 dash-001·002·003·004·005·008·016·019·020·021·022·024·025다. 로그아웃은 성공·확인 못함·403을 포함한다.
- SCR-002 부분 사례는 dash-006·007·009·010·011·012·013·014·016·019·022·024·025·026이다. DASH-006은 상세 값과 라우터를 나누어 확인한다. DASH-024·025의 SCR-003 부분은 로그인 시험이 맡는다. 가짜 응답 결과는 실서버 증거가 아니다.
- 형식 시험은 `0`, `256`, `1536`, `1048575`, `1572864`와 2^53−1을 BigInt 기대값과 비교한다. `JSON.parse('9007199254740993')`가 2^53이며 2^53 이상 2^70 미만의 표시 오차가 한 칸 이내인지 확인한다. 2^70 이상은 시험하지 않는다.
- `tests/dashboard_browser.rs`: 사례마다 `/// spec: dashboard.md:DASH-0xx` 마커를 붙인다. 마커 표기와 ` partial` 접미사는 `docs/conventions/testing.md` 7절을 따른다.
- 로그인 쿼리 시험은 `notice=logout-unconfirmed`의 CPY-076, `replaceState` 뒤 쿼리 제거, 다시 로드한 CPY-075, 알 수 없는 쿼리 값의 CPY-075와 제거를 확인한다.
- 상세 링크는 실제로 누르면 시험 페이지를 벗어나므로 `href`와 같은 해시를 적용한다. 상세 본문·종료 흐름은 `session-scenarios.js`가 맡는다.
- 정적 시험은 세션 없는 `/`·`/app/…`의 303과 세션 있는 `/`의 200 HTML·헤더를 확인한다.
- 시안 대조·키보드·스크린 리더·다른 브라우저·실기기는 [프런트엔드 검증 절차](../how-to/verify-dashboard-frontend.md)를 따른다.
- 자동 시험 결과는 [테스트 컨벤션](../conventions/testing.md)의 `cargo xtask scenarios`, `scenario-report.md`, CI에서 확인한다.

## 13. 반응형 (SCR-001)

- 중단점: `max-width: 1023.98px`에서 모바일 배치. 데스크톱은 `1024px` 이상의 12열 격자 (DESIGN 527행, 543~544행).
- 목록 표는 하나의 DOM(`<table>`)을 쓴다. 네이티브 `table`에 `role="table"`, `thead`·`tbody`에 `role="rowgroup"`, 모든 `tr`에 `role="row"`, `th scope="col"`에 `role="columnheader"`, `td`에 `role="cell"`을 명시한다. 빈 행·빈 셀도 같다. 이는 구현 판단이 아니라 DESIGN 565행의 기준이다. 머리글 행은 화면에서만 숨기고 접근성 트리에는 남긴다.
- `createDataTable`의 열 옵션 `mobileHead`가 첫 줄을 정한다. `primary`·`secondary` 셀은 `.data-table-head-primary`·`.data-table-head-secondary`로 모바일 격자의 첫 행(`grid-row: 1`)을 공유한다. 대기는 중계 ID·역할, 활성은 세션 링크·[상세], 종료는 세션 식별자·종료 원인 순서다(DESIGN 554~563행).
- 첫 줄에는 `::before` 레이블을 붙이지 않는다. 나머지 셀은 `data-label`을 CSS 가상 요소로 보이는 레이블·값 쌍으로 만든다. 가상 요소의 대체 텍스트는 비워 중복 읽기를 막는다(DESIGN 563·566행). 셀 값으로 DOM `Node`를 받으며 모바일 전용 마크업은 만들지 않는다. API와 검증은 [공통 컴포넌트 3.1절](dashboard-components.md#31-공개-함수)을 따른다.
- 최소 지원 폭은 320px이며 가로 스크롤이 생기지 않아야 한다. 폭별 결과는 [시각 대조 기록](../how-to/verify-dashboard-frontend.md#41-시각-대조-기록)을 따른다.

## 세션 상세 (SCR-002)

- 머리말에는 `목록으로`와 CPY-008 `새로고침`을 둔다. 알림 조치의 재조회는 CPY-014다.
- 정보 카드 8열과 전송량 카드 4열을 나란히 둔다. 1023.98px 이하에서는 카드와 `.fact` 행이 한 열로 쌓인다. 종료 버튼은 정보 카드 바닥(`card-foot`)이다.
- 정보는 `dl.fact-list`·`dt`·`dd`로 만들며 주소는 `.fact-address`다. 모드·전송은 아이콘 없는 중립 배지다. `state` 행은 표시하지 않는다.
- 전송량은 `directionStat`과 `createStat`으로 두 방향을 구분한다. chart-1은 Viewer→대상, chart-2는 대상→Viewer다. 표식은 10px 장식 정사각형이다.
- 종료 버튼은 `cancel` 아이콘을 유지하며 `.button-label`만 갱신한다. 조회 실패 중에는 이전 데이터가 있어도 `canTerminate`가 거짓이다.
- 대상 없음은 CPY-059 한 문장과 CPY-010 `목록으로` 조치다. 정보 카드 제목은 남고 본문은 비운다. 종료는 비활성이다. 전송량 카드·머리말 새로고침은 숨기며 포커스는 알림의 목록 링크로 옮긴다. 같은 중계 ID의 다른 세션으로 대체하지 않는다.
- 세션 식별자는 `.session-id`의 데이터 글꼴을 쓴다. 카드 제목은 CPY-081·082다.
- 문서 제목은 `VNC Relay`와 CPY-004 운영 현황 또는 CPY-051 세션 제목을 잇는다. 목록 바이트 문구는 CPY-040, 상세 카드 제목은 CPY-081·082를 따른다.

## 16. 최근 로그 (SCR-006)

동작 기준은 [화면 스펙 4·5.2절](../specs/dashboard.md#52-로그-필터다운로드와-조회-오류), 데이터 형식은 [관리 API 계약 1.15절](../contracts/management-api-v1.md#115-최근-로그-get-apiv1logs)이다.

대표 입력은 `/#/logs?session_id=ses_example`이다. `router.js`의 `routeOf`가 값을 해석해 `mountLogs`에 넘긴다. `isSessionId`가 형식을 확인한다. 형식 오류면 로그 조회·폴링·다운로드를 시작하지 않는다.

화살표는 실행 중 호출과 응답 전달을 뜻한다.

```text
router.js mountRouter → logs.js mountLogs
  → api.js getSession → model.js parseSession → P와 조회 제한 시간
  → api.js getLogs → model.js parseLogs → 필터·커서 세대가 같은 응답만 반영
  → 공통 Card·Badge·Alert·Button·상단 막대와 로그 표
```

### 상태와 요청 소유

- `logs.js`의 `state`는 필터, 정렬한 항목, 펼친 seq 집합, 보관 범위, 마지막 성공 시각과 마지막 확인 seq를 소유한다. `api.js`는 헤더·제한 시간·오류 분류만 맡는다. `parseLogs`는 필수 응답 필드를 검사한다.
- 첫 성공 전에는 수를 만들지 않는다. 보관 상태는 조회 중 또는 확인 불가 배지다. 시각은 CPY-035다. 실패는 기존 목록과 성공 시각을 유지하며 빈 결과로 바꾸지 않는다.
- 첫 조회·수동 갱신·필터 변경은 커서 없는 요청이다. `snapshotReady`는 현재 필터의 첫 조회가 성공했는지 나타낸다. 필터 변경은 이 값을 끄고 이전 요청을 중단하며 `epoch`를 올린다. 실패하면 이전 목록·시각은 오래된 값으로 유지한다. 첫 조회가 성공하기 전의 자동 복구도 커서 없이 다시 읽는다. 성공한 새 스냅숏으로 목록을 교체한 뒤에만 증분을 합친다. 세션 필터 해제는 URL에서 그 값만 지우고 수준·사건·적용한 검색을 유지한다.
- `schedulePoll`은 서버가 준 `pollMs` 뒤에 한 번 실행한다. `fetchInitialLogs({ poll })`가 커서 없는 요청의 출처를 받는다. 자동 재시도·seq 재시작 복구는 `poll: true`로 호출한다. 증분 요청과 이어지는 페이지도 `X-Relay-Poll: 1`을 쓴다. `has_more`가 끝나면 응답의 `latest_seq`까지 확인한 것으로 기록한다. 재시작으로 seq가 작아지면 이전 목록과 커서를 버리고 폴링 표시가 있는 커서 없는 요청으로 다시 읽는다.
- 이전 페이지는 가장 오래된 표시 seq를 `before`로 보낸다. 수집 결과는 seq로 중복을 제거하고 최신 순으로 합친다. 이전 페이지·다운로드 중에는 자동 조회가 겹치지 않는다.
- `400`·`405` 뒤에는 같은 자동 요청을 반복하지 않는다. `500`·응답 없음은 켜짐 상태에서 다음 주기에 다시 조회한다. `401`·로그아웃·화면 이탈은 `dispose`로 요청·타이머·로그·펼친 상세·다운로드 버퍼와 보호 DOM을 제거한다.
- 언어 변경·갱신 렌더는 조작 요소 ID·상세 행 ID·링크 목적지로 초점을 복원한다. 검색 버튼에는 안정된 ID를 준다. 입력 선택 범위·작성 중인 검색어·펼친 상세도 유지한다. 표 본문은 라이브 영역이 아니다. 표·행·열·셀 역할과 상세의 `aria-expanded`·`aria-controls`를 명시한다.

### 다운로드

`downloadJsonLines`는 시작 필터를 고정하고 `limit=1000`으로 첫 페이지를 읽는다. 다음 페이지는 가장 오래된 수집 seq의 `before`다. 끝까지 성공했을 때만 seq 오름차순의 `entry`를 UTF-8·LF로 합쳐 Blob을 만든다. 파일명 시각은 시작 UTC다. 새 사건은 첫 페이지 이후 파일에 섞이지 않는다. 오류·이탈·401이면 부분 파일을 저장하지 않는다.

### 관련 화면과 시험

- SCR-001의 `panels.js` `transportCell`은 Viewer·대상 중립 배지와 화살표를 조합한다. SCR-002는 `target_transport`와 값이 있는 전달 주소를 따로 표시하고 세션 필터 로그 링크를 만든다. 필수 필드 누락은 응답 형식 오류이며 TCP로 대체하지 않는다.
- SCR-004는 v4 계약의 테이블 순서를 유지하며 ticket 키도 `[relay.ticket.viewer]`·`[relay.ticket.server]`로 묶는다. SCR-005의 `editableItems`는 allow 6개 → trusted_proxies 2개 → allowed_origins 4개 → Mode I 목록 → 상한·제한 시간·연결 유지 → 접속 ticket 순서다. 마지막 구역은 Viewer 다음 Server이며 각 역할의 `enabled`·`issuer`·`relay_host`·`max_lifetime_secs` 순서다. 비활성 진입점의 편집 불가 키와 공개 키 파일 경로는 입력 목록에서 빠진다. 확인 대화상자는 허용 목록·프록시 목록·Origin 목록의 빈 값 경고(CPY-105·183·192)를 각각 고른다. ticket `enabled`의 false→true 경고(CPY-194)는 그 키의 변경 항목 안에 둔다.
- `web/dashboard/tests/logs.html`의 `dash-054`~`dash-061`·`dash-063`은 가짜 API 부분 증거다. Rust 실행 함수의 표식은 `partial`이다. 실제 로그 보관·세션 연관 검색·유휴 만료·중계 전송 조합의 전체 증거는 실서버 시험이 맡는다.
- SCR-006 실서버 실행기는 `tests/dashboard_live.rs`의 DASH-054~063 함수다. `logs.rs`는 실제 Repeater 거절·세션 사건과 `GET /api/v1/logs`를 화면과 대조한다. 설정 파일을 잠시 디렉터리로 바꿔 실제 `reload_watch_failed(level=error)`를 만든 뒤 원래 파일을 복구한다. 일반 로그 응답은 만들거나 바꾸지 않는다.
- `logs_download.rs`는 1000건을 넘는 필터 결과의 모든 페이지를 모은 실제 브라우저 다운로드를 읽는다. CDP `Browser.downloadProgress(state=completed)`까지 기다려 파일 이동·닫힘과 시험의 파일 제거가 경합하지 않게 한다. `logs_states.rs`는 별도 서버의 보관 0·빈 필터 결과·실패와 복구, 실제 300초 유휴 만료를 맡는다.
- `logs_accessibility.rs`는 1280·390px × ko·en에서 실제 CDP 키 입력으로 조작한다. `transports.rs`는 실제 TCP·WS·WSS와 시험 TLS 인증서로 Mode II 양쪽 9조합과 전달 주소 0·1·2개를 만든다. 활성 목록·상세 대조는 `overview.rs`·`detail.rs`가 제공한다. 자동 결과는 시나리오 보고서, 수동 다른 엔진의 결과와 한계는 [키보드·다른 엔진 확인 기록](../how-to/verify-dashboard-frontend.md#42-키보드다른-엔진-확인-기록)을 따른다.
- F-C 재현은 같은 페이지의 `shot-<상태>&lang=ko|en`을 쓴다. `loading`, `normal`, `empty`, `disabled`, `failed`, `failed-first`, `invalid-session`, `gap`, `paused`, `session`, `download`, `download-failed`, `refreshing`, `input-error`, `request-rejected`, `expired`를 제공한다. 실제 화면 함수와 공통 CSS를 쓰며 제품에 넣지 않는다.

## 17. 메트릭 (SCR-007)

동작 기준은 [화면 스펙 4.1·5.3절과 SCR-007](../specs/dashboard.md#41-폴링), 데이터 형식은 [관리 API 계약 1.16절](../contracts/management-api-v1.md#116-메트릭-get-apiv1metrics)이다. 시안은 `docs/design/mockups/dashboard.html`의 SCR-007 프레임이다.

### 파일과 호출

```text
router.js mountRouter → metrics.js mountMetrics
  → api.js getSession → model.js parseSession → P와 조회 제한 시간
  → api.js getQuery(/api/v1/metrics[?history_window=24h][&history_after=N]) → metrics-model.js parseMetrics(응답, 기간)
  → metrics-model.js mergeHistory(보관한 점, 응답의 점, 바꿈 여부, 기간) → 보관한 점
  → metrics-model.js keyIndicators·buildCharts(보관한 점, 기간)·distributionItems·causeRows·processRows
  → 공통 Card·Badge·Alert·Button·Gauge·DataTable·상단 막대, 추이 기간 select
```

- 화면 값 계산은 `metrics-model.js`에만 둔다. `metrics.js`는 그 결과를 DOM으로 옮긴다.
- 새 공통 조각은 없다. `components/icons.js`에 `metrics` 아이콘 하나를 더했다. login 복사본도 같은 줄을 가진다.

### 상태와 요청 소유

- `metrics.js`의 `state`는 `query.js`의 필드(`inFlight`, `busyShown`, `busyTimer`, `failed`, `data`, `lastSuccessAt`)에 언어, 보관한 점, `rejected`, `stopped`, `epoch`, 고른 기간(`selected`), 보관한 점의 기간(`shown`)을 더한다. `data`는 마지막 성공 응답을 검사한 값이다. 두 기간은 처음에 `1h`다. 화면을 떠나면 사라지므로 다시 들어오면 `1h`다.
- 화면 진입과 [새로고침]은 사용자 행동이다. 이 요청은 `history_after`와 `X-Relay-Poll`을 보내지 않고 보관한 점을 모두 바꾼다. 자동 조회는 `X-Relay-Poll: 1`을 붙이고, `shown`이 `selected`와 같고 보관한 점이 있으면 가장 큰 `seq`를 `history_after`로 보낸다. 모든 요청은 `selected`가 `24h`일 때만 `history_window=24h`를 붙인다.
- 자동 조회는 응답이 끝난 뒤 `setTimeout`으로 P 뒤에 한 번 예약한다. 요청은 겹치지 않는다. 자동 조회 중 [새로고침]은 새 요청 없이 `showBusy`로 진행 표시만 바로 보인다(DASH-077).
- 추이 기간 변경도 사용자 행동이다(DASH-080). `selected`를 바꾸고 예약된 자동 조회를 취소한 뒤 CPY-017 진행 표시를 바로 보인다. 진행 중인 요청이 없으면 바로 고른 기간을 요청한다. 요청은 새로고침과 같은 모양이다. 진행 중인 요청이 있으면 그 요청이 끝난 뒤 요청한다. 각 요청은 시작할 때의 기간을 기억한다. 끝났을 때 그 기간이 `selected`와 다르면 응답을 반영하지 않고 고른 기간을 바로 다시 요청한다.
- 기간 요청이 성공하면 `shown`을 `selected`로 바꾸고 보관한 점을 응답의 점으로 바꾼다. 실패하면 `shown`·보관한 점·이전 값을 그대로 두고 아래 표대로 실패를 보인다. 자동 조회가 계속되는 실패이면 다음 자동 조회는 `shown`이 `selected`와 다르므로 `history_after` 없이 고른 기간을 요청한다. 400·405이면 자동 조회는 멈춘 채로 있다.
- 세션 확인(`GET /api/v1/session`)은 진입 때 한 번 한다. 이 응답이 P·조회 제한 시간·CSRF 토큰을 준다. 이 확인이 실패하면 P를 모르므로 자동 재시도가 없다. 사용자가 [새로고침]을 눌러야 한다.
- 실패 처리는 아래 표를 따른다. 모든 실패에서 이전 값·보관한 점·마지막 성공 시각은 그대로 둔다.

| 응답 | 표시 | 자동 조회 |
| --- | --- | --- |
| 401 | `dispose` 뒤 `/login/`으로 이동. 값·추이·진행 중 요청을 지운다 | 멈춤 |
| 400·405 | CPY-018 배지와 CPY-245 | 멈춤. 새로고침이나 추이 기간 변경의 요청이 성공하면 다시 시작. 그 요청이 실패해도 멈춘 채로 둔다 |
| 500·응답 없음·시간 초과·검사 실패 | 첫 실패는 CPY-018 배지와 CPY-021. 성공 뒤 실패는 CPY-019 배지와 CPY-020 | P마다 계속 |

- 성공하면 `failed`·`rejected`·`stopped`를 지우고 `dataVersion`을 올린다. 본문은 `dataVersion`이 바뀔 때만 다시 만든다.
- 세대 번호(`epoch`)와 `state.ended`로 화면을 떠난 뒤 도착한 응답을 버린다. `dispose`는 요청 중단, 타이머 해제, 값·점·CSRF 토큰 제거, 보호 DOM 제거를 한다(DASH-068).
- 언어 변경은 언어를 저장하고 문서 언어·제목을 바꾼 뒤 화면을 다시 만든다. 요청은 보내지 않고 점·값·두 기간은 그대로다(DASH-079).

### 추이를 합치는 규칙

- 기간 값은 `metrics-model.js`의 `HISTORY_WINDOWS`가 정한다. `1h`는 10초·360개·3600초 폭·묶음 쌍 6개이고, `24h`는 60초·1440개·86400초 폭·묶음 쌍 1개다. `parseMetrics`는 응답의 `interval_secs`·`capacity`가 요청한 기간의 값과 다르면 응답 형식 오류로 본다.
- 보관한 점은 최대 그 기간의 `capacity`개다. 자동 조회 응답의 점은 보관한 가장 큰 `seq`보다 큰 것만 더한다. 그 뒤 `가장 큰 seq − capacity` 이하인 점을 버린다.
- 진입·새로고침·기간 변경은 응답의 점으로 모두 바꾼다. 응답의 `history.points`가 비어도 정상이다.
- 점마다 `seq`, `at`, `values`를 가진다. `at`은 `YYYY-MM-DDTHH:mm:ss.SSSZ` 형식이어야 한다. 아니면 응답 형식 오류다.

### 계산

- 활성·대기·거절은 같은 이름의 모든 점(속성 행)의 합이다. 값이 없으면 확인 불가다.
- 처리량과 CPU 사용률은 보이는 기간의 가장 최근 이웃한 두 점(`seq` 차이 1)의 차이를 `at` 차이(초)로 나눈다. 최근 24시간이면 두 점의 간격은 1분이다. 시각 차이가 0 이하이면 그 점은 확인 불가다. 방향 하나라도 확인 불가이면 처리량 카드는 확인 불가다. 이웃한 점이 없으면 "수집 중"(CPY-212)이다.
- 연결 차트는 `seq`를 묶음 쌍 수(최근 1시간 6, 최근 24시간 1)씩 묶는다. (묶음 마지막 값 − 첫 값) ÷ `at` 차이(초) × 60을 묶음 마지막 점 시각에 그린다.
- 차트 x축은 가장 최근 점이 오른쪽 끝이고 폭은 보이는 기간(3600초 또는 86400초)이다. 눈금은 그 폭을 4등분한 다섯 개의 현지 시각(15분 또는 6시간 간격)이다. 창 밖 점은 그리지 않는다. 선은 `null`과 `seq`가 비는 자리에서 끊긴다. 한 점만 남은 선은 길이 0인 선분이며 둥근 끝(`chart-dot`)으로 점처럼 보인다.
- y축 위쪽 끝은 `niceCeil`이 구한다. {2, 4, 6, 8, 10} × 10^k 가운데 보이는 최댓값 이상인 가장 작은 값이다(DESIGN.md 선 차트). 보이는 값이 없거나 모두 0이면 2다. 10 미만에서는 최댓값 자신의 10의 거듭제곱부터 찾아서 개수 축의 위쪽 끝이 1이 되지 않는다. 바이트는 KiB 기준 위쪽 끝을 먼저 구하고 그 값이 1024 KiB 이상이면 MiB 기준으로 다시 구한다. 아니면 KiB 눈금이다.
- 차트 SVG는 `viewBox="0 0 360 100"`, `preserveAspectRatio="none"`이고 모든 선에 `vector-effect="non-scaling-stroke"`를 준다. 가로 폭은 컨테이너를 따른다.
- 차트별 최소 점 수는 순서대로 2·(묶음 쌍 수 × 2 + 1)·3·3·2다. 연결 차트는 최근 1시간 13개, 최근 24시간 3개다. 모자란 차트 자리는 CPY-220(최근 24시간은 CPY-253)이다. 점 수가 충분해도 선이 없으면(모든 값이 `null`) 범례의 현재·최대는 CPY-035이다.
- 보이는 기간에 따라 추이 범위(CPY-203·251), 추이 구역 제목(CPY-213·252), CPU 사용률 설명(CPY-209·254)을 고른다.

### 최소 막대 크기

DESIGN.md가 정한 대로 0이 아닌 값의 막대는 `spacing.chart-stroke`(2px)보다 작게 그리지 않는다. 아주 작은 값은 일부러 크게 보이게 한다. 0인 값은 막대를 그리지 않는다.

- 분포 막대는 높이 32px에 viewBox 높이 100이다. 최소 높이는 `2/32 × 100 = 6.25`다(`MIN_DIST_BAR_HEIGHT`).
- 수 막대는 폭 64px에 viewBox 폭 100이다. 최소 폭은 `2/64 × 100 = 3.125`다(`MIN_COUNT_BAR_WIDTH`).
- 가장 큰 값의 막대는 비례 크기를 유지한다.

### 표

- 시간 분포 표는 `createDataTable` 하나다. `caption`은 `sr-only`다. 항목마다 행이 여러 개일 수 있다. 속성은 `이름=값`을 쉼표로 이어 `span.mono`에 둔다. 평균·p95는 `formatShortSeconds`로 쓴다. p95 상한은 CPY-224 `≤ x` 또는 `> 마지막 경계`다. 개수가 0이면 평균·p95는 CPY-035다.
- p95 구간은 누적 개수 × 100이 전체 × 95 이상이 되는 첫 구간이다.
- 기록이 없는 항목은 항목 이름과 CPY-236 한 칸(나머지 열에 걸침)이다.
- 분포 막대 `svg.dist`는 `role="img"`이고 이름은 CPY-247에 0이 아닌 구간만 `≤ 5 ms 3개`처럼 쉼표로 이어 넣는다.
- 원인별 표 6개는 수가 큰 순서이고 같으면 응답 순서다. `close_cause` 값은 SCR-001과 같은 번역(CPY-045~050·083·190)을 쓰고 목록 밖 값은 중립 배지로 원문을 보인다. 다른 속성 값은 번역하지 않는다. 행이 없으면 CPY-236이다.
- 수 칸은 `span.count-cell`이다. `svg.count-bar`(`aria-hidden`)와 숫자를 담는다. 막대 길이는 그 표의 가장 큰 수에 비례한다.

### 프로세스

- 행은 가동 시간, CPU 시간(사용자 / 시스템), 메모리 사용량, 가상 메모리, 스레드 수, 열린 핸들 수 또는 열린 파일 기술자 수, 버전이다. 핸들·기술자 행은 응답에 그 메트릭이 있을 때만 만든다.
- 첫 응답 전에는 플랫폼을 모르므로 핸들·기술자 행을 만들지 않고 조회 중 배지만 둔다. 시안은 이때 핸들 행을 보인다.

### 화면 골격과 접근성

- 머리말: `h1.page-title`(CPY-202) → 상태 배지와 "마지막 성공 갱신"(CPY-015) → 추이 범위(CPY-203·251, 데이터가 있을 때만) → 추이 기간 선택 → [새로고침]. 추이 기간 선택은 `div.field.period-field` 안의 `label`(CPY-248)과 `select#metrics-period`(CPY-249·250)다. `build()`에서 한 번 만들고, 상태가 바뀌면 [새로고침] 자리만 다시 만든다. 그래서 갱신이 선택의 포커스를 빼앗지 않는다. 선택은 비활성으로 두지 않는다.
- 순서: 외부 수집 안내(CPY-244, 중립 안내에 `createAlert`의 `icon: 'alert'`로 원 안 느낌표 아이콘, 라이브 영역 아님) → 조회 실패 안내 → 핵심 지표 → 추이 → 시간 분포 → 원인별 수 → 프로세스.
- 구역 제목은 `h2`, 카드 제목은 `h3`이고, 시간 분포·프로세스 카드 제목은 `h2`다. 시안은 `h5`·`h6`을 쓴다. `base.css`의 제목 여백 초기화가 h1~h5만 덮으므로 h6을 쓰지 않는다.
- 상태 배지 칸만 `role="status"`다. 조회 때마다 칸의 내용은 상태가 바뀔 때만 바꾼다. 차트·표·값에는 라이브 영역이 없다.
- 차트는 `figure[role="img"]`이고 이름은 `제목. 계열 이름: 현재 X · 최대 Y. …`다. 마우스 도구 설명이나 포커스 대상이 없다. 선 모양(실선·긴 점선·점선)과 색을 함께 쓴다.
- 포커스는 `id`로 복원한다: `metrics-period`, `metrics-refresh`, `metrics-alert-refresh`, `metrics-language`, `metrics-logout`. 새로고침이 끝나면 [새로고침]으로 돌려준다. 기간 변경 뒤에는 포커스를 옮기지 않는다.
- 모바일(1023.98px 이하)에서 카드와 표는 한 열이다. 표는 `data-label`을 가진 열별 쌍이다. 기록이 없는 칸(`td.none`)은 레이블이 없다.

### 시험

- `web/dashboard/tests/metrics.html`·`metrics-scenarios.js`가 가짜 API로 dash-028·064·068~075·077~081과 최소 막대 크기(`bars`), y축 위쪽 끝(`axis`) 사례를 돌린다. 결과는 `data-result`다. `?case=…&lang=ko|en`을 쓴다. 가짜 응답 결과는 실서버 증거가 아니다. dash-077은 시간이 길어 `dash-077`과 `dash-077-more` 두 사례로 나눈다. `dump-dom`이 가상 시간 10초에서 멈추기 때문이다.
- `tests/dashboard_browser.rs`가 이 사례를 실행하고 마커를 가진다. 가짜 응답이 증명하지 못하는 부분은 마커에 `partial`을 붙인다.
- `tests/dashboard_live/metrics.rs`는 실제 서버와 프록시로 DASH-068(SCR-005 입력 버림, 진입 요청 모양, 떠난 뒤 자동 조회 중단, 붙잡아 둔 늦은 응답)과 DASH-076(401, 자동 조회의 유휴 만료 비연장)을 확인한다. 두 마커는 `partial`이 아니다.
- F-C 재현은 `metrics.html?case=shot-<상태>&lang=ko|en`이다. `loading`, `collecting`, `normal`, `partial`, `failed`, `failed-first`, `refreshing`, `rejected`를 제공한다. `partial`은 메모리 메트릭이 `available = false`이고 추이 값 일부가 `null`인 상태다. 제품에 넣지 않는다.

## 18. 접속 출처 (SCR-008)

동작 기준은 [화면 스펙 4.1·5.4절과 SCR-008](../specs/dashboard.md#scr-008-접속-출처), 데이터 형식은 [관리 API 계약 1.17절](../contracts/management-api-v1.md#117-접속-출처-get-apiv1sources)이다. 시안은 `docs/design/mockups/dashboard.html`의 `scr008-*` 프레임이다.

### 파일과 호출

```text
router.js mountRouter → sources.js mountSources
  → api.js getSession → model.js parseSession → P와 조회 제한 시간
  → api.js getQuery(/api/v1/sources?window=1h|24h) → sources-model.js parseSources(응답, 기간)
  → sources-model.js summaryValues·storageLimited·tableCount·attributedProviders·countryLabel
  → 공통 Card·Badge·Alert·Button·DataTable·Link·상단 막대, 집계 기간 select
```

- 화면 값 계산은 `sources-model.js`에만 둔다. `sources.js`는 그 결과를 DOM으로 옮긴다.
- 새 공통 조각은 없다. `components/icons.js`에 `pin` 아이콘 하나를 더했다. login 복사본도 같은 줄을 가진다. 출처 표시 문구(CPY-276~278)는 번역하지 않으므로 `copy.js`의 `PROVIDER_ATTRIBUTION` 하나에 둔다.
- 마지막 사건 칸은 SCR-001과 같은 두 줄 시각이다. 그래서 `panels.js`의 `timeStack`을 내보낸다.

### 상태와 요청 소유

- `sources.js`의 `state`는 `query.js`의 필드에 언어, `rejected`, `stopped`, `epoch`, 고른 기간(`selected`)을 더한다. 기간은 진입할 때마다 `1h`다.
- 진입·[새로고침]·기간 변경은 사용자 행동이라 `X-Relay-Poll`을 보내지 않는다. 자동 조회는 `X-Relay-Poll: 1`을 붙인다. 응답은 증분이 아니므로 성공할 때마다 모든 값을 바꾼다.
- 요청은 겹치지 않는다. 진행 중에 기간을 바꾸면 그 요청이 끝난 뒤 고른 기간을 바로 요청하고, 끝난 요청의 응답은 반영하지 않는다. 실패해도 이전 값과 마지막 성공 시각을 유지한다.
- 실패 처리와 해제는 17절 메트릭 표와 같다. 400·405는 CPY-018 배지와 CPY-293이며 자동 조회를 멈춘다. 401은 `/login/`으로 이동한다.
- 언어 변경은 같은 응답으로 화면을 다시 만든다. 요청은 보내지 않는다. 국가 이름도 이때 바뀐다.

### 표시 규칙

- 요약 카드 7개는 `totals` 네 값과 세 표의 `total`이다. 조회 전에는 0 대신 조회 중 배지를 둔다.
- IP 정보 DB 카드는 국가·ASN 행마다 상태 배지(CPY-264~267, 모르는 상태는 받은 그대로), 값이 있는 종류·빌드 시각·읽은 시각, 안내 문장(CPY-269~271)을 둔다. `error`가 `null`이면 원인 문장을 뺀다. 출처 표시는 쓰는 DB의 제공자마다 한 번이다. 문구에 주소가 들어 있으면(CPY-278) 주소만 링크다.
- 세 표는 응답 순서대로 모든 `items`를 보인다. 표 제목 옆 개수는 `total`이 `items` 수보다 크면 CPY-289, 같으면 CPY-084·085다.
- `local = false`인 주소와 ASN은 `https://ipinfo.io/<값>` 링크다. 새 탭으로 열고 `rel="noopener noreferrer"`이며 이름은 CPY-291과 CPY-196이다. 로컬 주소는 링크 없이 CPY-282 배지를 붙인다. `null` 국가·ASN은 CPY-288이다.
- 국가 이름은 `Intl.DisplayNames(fallback: 'none')`의 이름과 코드(`대한민국 (KR)`)다. 브라우저가 이름을 모르면 코드만 보인다.
- 보관 상한 안내(CPY-258)는 `dropped_before`가 `window_start` 이상일 때만 보인다. 경고 모양이지만 라이브 영역이 아니다.
- 상태 배지 칸만 `role="status"`다. 포커스는 `id`(`sources-period`, `sources-refresh`, `sources-alert-refresh`)로 복원한다. 제목은 `h1` → 구역 `h2` → DB 이름 `h3`이다. 시안은 `h4`~`h6`을 쓴다.
- 모바일(1023.98px 이하)에서 DB 행은 한 열이고, 표는 `data-label`을 가진 열별 쌍이다. 빈 칸(이름 없는 ASN)은 숨긴다.

### 시험

- `web/dashboard/tests/sources.html`·`sources-scenarios.js`·`sources-fixture.js`가 가짜 API로 dash-028·064·082~088을 돌린다. dash-086은 시간이 길어 `dash-086`·`dash-086-more`·`dash-086-rejected`·`dash-086-busy`로 나눈다. `tests/dashboard_browser.rs`의 표식은 `partial`이다.
- `tests/dashboard_live/sources.rs`는 실제 서버로 DASH-082 전체(메뉴 순서, SCR-005 입력 버림, 진입 요청 모양, 떠난 뒤 자동 조회 중단, 늦은 응답, 돌아오면 최근 1시간, 세션 없는 직접 진입)를 확인한다. 생성한 MMDB와 신뢰 프록시 뒤 WebSocket 클라이언트로 DASH-083~085 일부도 확인한다.
- F-C 재현은 `sources.html?case=shot-<상태>&lang=ko|en`이다. `loading`, `normal`, `empty`, `db-mixed`, `db-failed`, `storage-limit`, `refreshing`, `failed`, `failed-first`, `rejected`를 제공한다. 주소·ASN 표는 시안처럼 7행이다. 제품에 넣지 않는다.

## 15. 열린 항목

| 항목 | 현재 동작·영향 | 책임자 |
| --- | --- | --- |
| 역할·모드·전송 경로의 표시 이름과 스펙의 원문 표기 범위 | 알려진 값은 9절의 표시 이름, 목록 밖 값은 원문이다. | 화면 스펙 검토 담당자 |
| 조회 진행 중 새로고침 버튼의 busy 표현 | 현재는 로그아웃 중 disabled만 쓴다. 조회 상태는 카드 배지로 보인다. | 화면 스펙 검토 담당자 |
| 잘못된 시각 문자열의 처리 범위 | 현재는 확인 불가가 아닌 응답 실패다. | 화면 스펙·API 소비자 검토 담당자 |

## 대조 기록

| 날짜 | 대상 | 환경 | 방법 | 실제 결과 |
| --- | --- | --- | --- | --- |
| 2026-11-17 | `9d43e3e`의 대기·활성·최근 종료 표 | Windows의 에이전트 조작 Chromium 탭. 버전은 원 기록에 없다. | `cargo xtask dashboard-serve`의 제품 화면과 파일 시안을 1280·1100·1024·1023·768·390·320px × ko·en으로 대조했다. | 가로 넘침 0건. 대기·활성 시각은 날짜와 시각·시간대 두 줄이며 주소 레이블은 값 위에 있다. 범위별 실제 결과와 허용 차이는 [시각 대조 기록](../how-to/verify-dashboard-frontend.md#41-시각-대조-기록)을 따른다. |
| 2026-11-19 | `metrics.js`·`metrics-model.js`·`app.css`의 SCR-007 구현 | Windows 10.0.26200 x64, Chrome 157.0.8103.79, 에이전트 조작 탭 | 이 문서 17절의 규칙과 시안 SCR-007 프레임의 마크업·CSS를 코드와 비교했다. `cargo xtask dashboard-serve`의 시험 페이지와 시안 값의 가짜 API로 화면을 시안과 대조했다. | 남은 차이는 제목 단계(h2·h3), 조회 중의 핸들 행 생략, 클래스 이름(시안 `xfer`·`kv`는 `kpi-xfer`·`kv-list`)이다. 외부 수집 안내 아이콘은 구현을, 빈 원인 표 정렬과 모바일 배지 폭은 시안을 고쳤다. 폭별 결과는 [시각 대조 기록](../how-to/verify-dashboard-frontend.md#41-시각-대조-기록)을 따른다. |
| 2026-11-20 | `sources.js`·`sources-model.js`·`app.css`의 SCR-008 구현 | Windows 10.0.26200 x64, Chrome, CDP 기기 크기 지정 | `sources.html?case=shot-<상태>`를 1280·1024·390·320px × ko·en(80장)으로 찍어 채택 시안 그림(`target/scr008-shots/`)과 대조했다. | 가로 넘침 0건, 콘솔 오류 0건. 남은 차이는 제목 단계(h1·h2·h3), 시험 값, 표 개수 배지의 표시 행 수(시안은 7행에 "상위 100개")다. |

전체 화면 시안 대조·개수 배지·도움말 링크의 최신 기록도 같은 시각 대조 표에서 관리한다. 키보드·다른 엔진의 수동 결과는 [별도 기록](../how-to/verify-dashboard-frontend.md#42-키보드다른-엔진-확인-기록)을 따른다.

