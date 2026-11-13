# VNC Relay 코드 지도

VNC Relay는 TCP·WS·WSS Viewer를 Mode I 대상이나 TCP·WS·WSS Mode II Server와 이어 바이트 스트림을 중계한다. 중계 소켓은 TCP Viewer·TCP Server·WS·WSS 네 개다. WS·WSS는 경로로 역할을 나눈다. 이 지도는 코드 위치와 책임, 수락에서 초기 교환·대상 연결 또는 매칭·정리·운영 로그까지의 흐름을 설명한다.

| 항목 | 값 |
| --- | --- |
| 구조 시점 | Current |
| 문서 상태 | Draft |
| 구현 대조 | 설정 보기·편집과 실행 값 재로드는 [2026-11-08 소스 대조](#2026-11-08-설정-보기편집과-실행-값-재로드-소스-대조). 이번 변경의 자동 검사(`tests/code_map.rs` 포함)는 커밋 `eae442f`의 두 OS 게이트에서 통과했다([작업 6.9 판정표](../how-to/set-up-and-verify.md#42-작업-69-판정표)). 이전 실행 증거는 [2026-11-04 대조 기록](#2026-11-04-구조-대조)에 있다. WSS Server·프록시 TLS 종료·최근 로그 범위의 구현은 커밋 `3df0693`이다. 시험은 커밋 `51bd747`의 두 OS 게이트에서 통과했다([WSS Server·프록시·최근 로그 판정표](../how-to/set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)). |
| 관련 PRD | [PRD](../requirements/PRD.md), [유지보수성](../requirements/PRD.md#8-비기능-요구사항), [구현 기술](../requirements/PRD.md#확정-기준) |
| 관련 ADR | [ADR-0001 내장 TLS](../decisions/ADR-0001-built-in-tls.md), [ADR-0003 메모리 상태](../decisions/ADR-0003-single-instance-memory-state.md), [ADR-0004 tokio·rustls](../decisions/ADR-0004-tokio-rustls.md), [ADR-0005 WS 코덱](../decisions/ADR-0005-in-house-ws-streaming-frame-codec.md), [ADR-0006 파일을 통한 설정 편집](../decisions/ADR-0006-config-edit-through-file.md) |

제품은 domain·app·adapters·실행파일 조립의 네 crate로 나뉜다. 개발 도구는 별도 `xtask` crate다. 정적 대시보드는 `web/dashboard/`에 있다. 코드가 있다는 사실과 스펙 시나리오 전체가 검증되었다는 판단은 다르다. 시나리오별 실행 결과는 [테스트 컨벤션](../conventions/testing.md#7-테스트와-스펙-시나리오-연결)의 보고서가 맡는다.

### R1·R2 적용 범위

후속: 2026-11-08 R1은 [설정 v4](../contracts/config-v4.md)·[ops-info 2.0.0](../contracts/ops-info-v2.md)의 리스너·역할 분리, WS·WSS Server, 양쪽 전송 메타데이터, 재로드 33개·편집 29개를 반영했다.

후속: 2026-11-08 R2는 WS·WSS 리스너의 공식 역방향 프록시 TLS 종료 및 `X-Forwarded-For` 기반 전달 주소 판정을 반영한다(사용자 결정 2026-11-08(Q182·Q188), [ADR-0007](../decisions/ADR-0007-proxy-tls-termination.md)). `trusted_proxies`에 있는 소켓 상대는 accept의 역할 allow 검사를 건너뛰고, upgrade에서 전달 주소(`forwarded_ip`)를 도출해 역할 allow 검사에 적용한다. 잘못된 헤더나 신뢰 프록시로만 이뤄진 체인은 400 `forwarded_for_invalid`로 거절된다. 전체 주소 신뢰 프록시 항목은 시작·재로드 시 `trust_all_proxies` 경고를 낸다. 관리 API 세션 객체(활성 목록 및 상세)에 `viewer_forwarded_ip`·`target_forwarded_ip`를 제공한다.

구조 시점은 R2 소스 반영 범위의 Current다. 아래 과거 대조·통과 기록은 R2 실행 증거가 아니다. 이번 문서 작업에서는 빌드·시험·포매터를 실행하지 않았다. 후속: 구현은 커밋 `3df0693`이며, 시험은 커밋 `51bd747`의 두 OS 게이트에서 통과했다([WSS Server·프록시·최근 로그 판정표](../how-to/set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)).

## 1. 전체 구조

다음 그림은 현재 실행 경로다. 모든 실선 화살표는 호출 또는 값 전달이다. `포트` 표시가 있는 화살표는 app이 정의한 trait를 통한 호출이다. 조립 코드가 adapters 구현체를 주입하므로 app은 adapters 타입을 알지 못한다. 오류 변환과 모듈 안의 보조 함수는 생략했다.

```mermaid
flowchart LR
    CLI["src/main·cli·serve·admin"]
    CFG["adapters/config_file·accounts_file·tls"]
    NET["adapters/net·websocket"]
    RELAY["app/relay<br/>초기 교환·대상 연결·매칭·중계"]
    TARGET["adapters/target"]
    HTTP["adapters/management_http"]
    MGMT["app/management<br/>로그인·조회·종료·설정 편집"]
    STATUS["adapters/status_http"]
    LIFE["app/lifecycle<br/>준비 상태"]
    WATCH["adapters/file_watch"]
    RELOAD["app/reload"]
    LOG["adapters/ops_log"]
    DOM["domain 업무 규칙"]
    CLI -->|"설정·계정·TLS 준비"| CFG
    CLI -->|"리스너 시작"| NET
    CLI -->|"관리 서비스 주입·시작"| HTTP
    CLI -->|"상태 리스너 시작"| STATUS
    CLI -->|"감지 시작"| WATCH
    CLI -->|"준비·종료 상태 갱신"| LIFE
    CLI -->|"종료 요청·G 감독"| RELAY
    NET -->|"스트림·문맥·L"| RELAY
    RELAY -->|"포트: DNS·단일 IP 연결"| TARGET
    HTTP -->|"인증·조회·종료·설정 요청"| MGMT
    MGMT -->|"포트: 스냅샷·지정 세션 종료"| RELAY
    MGMT -->|"공유 잠금: 분류·후보 준비·commit"| RELOAD
    MGMT -->|"포트: 원문 읽기·보존 편집·원자 교체"| CFG
    STATUS -->|"준비 상태 조회"| LIFE
    WATCH -->|"해석한 후보·세대 확인 함수"| RELOAD
    RELOAD -->|"포트: 파일·TLS 후보 읽기"| CFG
    RELAY -->|"포트: 사건"| LOG
    MGMT -->|"포트: 사건"| LOG
    RELOAD -->|"포트: 사건"| LOG
    RELAY -->|"규칙 호출"| DOM
    MGMT -->|"규칙 호출"| DOM
    RELOAD -->|"설정의 정책 값"| DOM
```

### 레이어와 의존 방향

레이어 규칙은 [아키텍처 작성 규칙](AGENTS.md#작성-규칙)을 따른다. 아래 화살표 `A → B`는 컴파일 의존이다. 실행 중 포트 호출과 구분한다.

| 레이어 | 코드 위치 | 책임 | 의존할 수 있는 레이어 | 의존 위반 확인 방법 |
| --- | --- | --- | --- | --- |
| domain | `crates/domain` (`vnc-relay-domain`) | 초기 교환·매칭·세션·주소·예산·계정 규칙. 입출력·시계·난수 결과는 값으로 받는다. | 없음. 외부 crate도 쓰지 않는다. | 컴파일과 `cargo xtask layers` |
| app | `crates/app` (`vnc-relay-app`) | 중계·관리·재로드·준비 상태·계정 준비 사용 사례. 외부 기능은 포트로 정의한다. | domain. tokio의 동기화·시간·입출력 trait | 컴파일, `cargo xtask layers`, `crates/app/clippy.toml`의 입출력 금지 목록과 `cargo clippy` |
| adapters | `crates/adapters` (`vnc-relay-adapters`) | TCP·TLS·WS·HTTP·파일·운영 로그·난수·터미널. app 포트를 구현한다. | domain, app, [PRD 승인 라이브러리](../requirements/PRD.md#확정-기준) | `cargo xtask layers`, [라이선스 검사](../how-to/check-dependency-licenses.md) |
| 실행파일 조립 | 루트 crate `vnc-relay`의 `src/` | CLI·종료 코드·런타임·OS 신호·객체 연결. 업무 규칙은 두지 않는다. | domain, app, adapters, clap, tokio. rustls는 개발 의존성이다. serde_json·tokio-tungstenite는 실서버 대시보드 시험의 CDP 통신에 쓴다(Q112). aws-lc-rs는 개발 의존성이며 실서버 접속 ticket 시험의 ES256 서명에만 쓴다. adapters가 이미 쓰는 [PRD 승인 라이브러리](../requirements/PRD.md#확정-기준)다(이 문서의 판단, 사용자 결정 없음). windows-sys는 Windows 전용 개발 의존성이며 ConPTY 시험만 쓴다(Q103, [Rust 컨벤션 2절](../conventions/rust.md#2-규칙) "unsafe" 행). | 코드 리뷰, 컴파일, `cargo xtask layers` |
| 개발 도구 | `xtask/` | 시나리오 보고·레이어 검사·배포 고지 생성과 대조·토큰 생성과 대조·로컬 미리보기·시험용 접속 ticket 발급 | 제품 crate에 의존하지 않는다. | `cargo xtask layers` |
| 정적 대시보드 | `web/dashboard/` | HTML·CSS·JS 화면·공통 컴포넌트·생성 복사본·미리보기·브라우저 시험 페이지 | 관리 API만 HTTP로 호출한다. JS 외부 의존성이 없다. | [프런트엔드 컨벤션](../conventions/frontend.md), [대시보드 검증](../how-to/verify-dashboard-frontend.md), `cargo xtask tokens --check` |

```mermaid
flowchart LR
    BIN["실행파일 조립"] -->|"의존"| ADP["adapters"]
    BIN -->|"의존"| APP["app"]
    BIN -->|"의존"| DOM["domain"]
    ADP -->|"의존"| APP
    ADP -->|"의존"| DOM
    APP -->|"의존"| DOM
```

- domain·app은 바깥 레이어 타입을 알지 못한다. 레이어 경계에서는 domain·app 값과 tokio 입출력 trait를 넘긴다.
- axum 요청·tungstenite 메시지·TOML 값은 app에 넘기지 않는다.
- 대기·활성·최근 종료·관리 세션 상태는 메모리에만 둔다([ADR-0003](../decisions/ADR-0003-single-instance-memory-state.md)).
- 이 구조에서 벗어나려면 [ADR](../decisions/AGENTS.md)을 작성한다.

## 2. 코드 위치와 구성요소

모든 행은 현재 파일이다. 작은 보조 모듈은 함께 묶었다. `코드 위치` 칸에서는 경로 뒤에 그 경로가 정의하는 심볼을 쓴다. `tests/code_map.rs`는 §2의 경로·심볼, §3의 모든 진입점 호출 체인, §4의 모든 인접 단계 사이에 있는 파일 단위 사용 근거를 검사한다. 주석·문자열과 정의 자체는 호출 근거가 아니다. 책임 문장의 의미와 실제 소유권은 사람이 대조한다. 이 표 구성과 검사 방법은 **이 문서의 판단, 사용자 결정 없음**이다.

### 2.1 domain

| 코드 위치 | 구성요소 | 주된 책임 | 맡지 않는 책임 | 의존 대상 | 모듈 설계 |
| --- | --- | --- | --- | --- | --- |
| `crates/domain/src/lib.rs` | 모듈 공개 | domain 모듈 경계를 공개한다. | 조립·입출력 | 하위 모듈 | 이 지도 |
| `crates/domain/src/handshake.rs` `HandshakeParser`; `crates/domain/src/relay_id.rs` `RelayId` | 초기 교환 | 250바이트 요청·모드·대상·정확한 ID를 해석한다. 후속 데이터는 소비하지 않는다. | 소켓·H·HMAC | 주소·대상 문법 | [초기 교환](initial-exchange.md) |
| `crates/domain/src/matching.rs` `WaitingTable` `RegistrationPolicy` | Mode II 점유 | Waiting·Claimed·Active·Closing 전이와 중복·만료·식별자 일치를 판정한다. 등록마다 Lw·W를 받고 기존 대기의 deadline은 보존한다. | 소켓·실제 permit | RelayId·ConnId·SessionId | [중계 수명](relay-session.md) |
| `crates/domain/src/session.rs` `SessionId` `Transport` `Session` `Session::finish_after_owner_loss`; `crates/domain/src/recent_closed.rs` `RecentClosedList` | 세션·최근 종료 | 양쪽 전송, 최초 종료 원인·부분 쓰기 카운터·EOF·E·최종 요약과 R 상한을 관리한다. Mode I 대상은 TCP다. | 바이트 전달·시각 읽기 | std 값 | [중계 수명](relay-session.md), [관리자 종료](management-terminate.md) |
| `crates/domain/src/target_policy.rs` `TargetPolicy::permits_name` `TargetPolicy::permits_ip` | 대상 정책 | 설정 항목을 해석하고 이름·포트와 최종 IP·포트를 각각 검사한다. | DNS·연결 | allow_list | [설정](config-loading.md), [중계 수명](relay-session.md) |
| `crates/domain/src/entry_point.rs` `Listener` `EntryPoint` `Role`; `crates/domain/src/conn_id.rs` `ConnId` | 리스너·진입점·연결 ID | 중계 리스너 네 개, 리스너·역할 진입점 여섯 개, TCP 고정 역할과 프로세스 안 연결 식별자를 제공한다. | 소켓 수락·로그 출력 | std | [TCP 진입점](tcp-entry-point.md), [WS 진입점](websocket-entry-point.md) |
| `crates/domain/src/allow_list.rs` `AllowList`; `crates/domain/src/origin.rs` `Origin`; `crates/domain/src/peer_ip.rs` `PeerIp`; `crates/domain/src/forwarded_for.rs` `client_ip` `ForwardedForError` | 접근 값 | IP 항목 해석·허용 판정·Origin 정규화·상대 IP 표기와 신뢰 프록시 전달 주소 도출 | HTTP·소켓 읽기 | std | [설정](config-loading.md), [WS 진입점](websocket-entry-point.md) |
| `crates/domain/src/ticket.rs` `TicketClaims` `TicketPolicy` `TicketKey` `TicketKeyList` `TicketUseTable` `split_compact` | 접속 ticket 규칙 | JWS 압축 직렬화를 나누고 클레임 대조·시각 판정을 계약 순서로 한다. 공개 키 목록의 `kid` 중복과 두 역할의 같은 키를 판정한다. 역할별 `jti` 사용 기록을 원자적으로 쓰고 만료 기록을 정리한다. | 서명 계산·JSON 해석·파일 읽기·시각 읽기 | entry_point·relay_id·std | [접속 ticket](relay-access-ticket.md) |
| `crates/domain/src/budget.rs` `ConnectionBudget` `BudgetPermit` | 연결 예산 | 사용량만 저장한다. `try_acquire`가 받은 현재 L·Lm·Lw로 자리를 확보하고 permit 해제로 반환한다. 상한 하향은 기존 permit을 취소하지 않는다. | 설정 읽기·연결 수락 | std 원자 값 | [연결 상한](connection-limits.md) |
| `crates/domain/src/account.rs` `Password`; `crates/domain/src/admin_session.rs` `AdminSessionTable`; `crates/domain/src/login_limit.rs` `LoginLimiter` | 관리 규칙 | 계정·비밀번호·해시 형식, 세션·CSRF 토큰·만료, 로그인 실패 제한 | 해시 계산·파일·시각 읽기 | std | [계정 준비](admin-accounts.md), [관리 세션](management-session.md) |

### 2.2 app

| 코드 위치 | 구성요소 | 주된 책임 | 맡지 않는 책임 | 의존 대상 | 모듈 설계 |
| --- | --- | --- | --- | --- | --- |
| `crates/app/src/lib.rs` | 모듈 공개 | 사용 사례 모듈을 공개한다. | 구현체 주입 | 하위 모듈 | 이 지도 |
| `crates/app/src/ports.rs` `TargetConnector` `RelayIdSource` `SessionIdSource` `RelayStream` `RelayStateSource` `RelaySessionControl` | 외부 기능 경계 | DNS·단일 IP 연결, ID 계산, 전송·종료·maintenance, 상태 조회·관리자 종료를 정의한다. 계정·입력·해시·시계·토큰·사건 포트도 정의한다. | 포트 구현·HTTP | domain·tokio trait | [중계 수명](relay-session.md), [관리 조회](management-query.md), [관리자 종료](management-terminate.md) |
| `crates/app/src/settings.rs` `Settings` `RelaySettings` `EntryPointSettings` `WsListenerSettings` `WsRoleSettings` `TicketSettings` `TicketRoleSettings` `RecentLogsSettings` `ListenerPlan`; `crates/app/src/events.rs` `Event` `ShutdownTrigger` | 설정·사건 | v4의 TCP 설정·공유 WS 리스너·역할 설정·역할별 접속 ticket 설정과 공개 키·최근 로그 상한과 리스너 계획, ops-info 2.1.0의 선택 역할·Server 전송·ticket 사건 값을 정의한다. | TOML·JSON 출력·로그 보관 | domain | [설정](config-loading.md), [운영 로그](ops-log.md), [접속 ticket](relay-access-ticket.md) |
| `crates/app/src/relay.rs` `RelayService::handle_connection` `RelayService::dispatch` `PreparedConnection` `TicketBinding` `RelayRuntime` `RelayState` `run_pair` | 중계 소유권 | 초기 교환을 준비하고 Mode I·II로 분기한다. ticket으로 받은 연결의 중계 ID 대조와 세션 만료 시각을 넘긴다. 공유 상태·활성 등록·정리 guard·최종 사건을 소유한다. B·L은 복제하지 않고 이동한다. | 수락·DNS 구현·프레임·ticket 서명 확인 | domain·ports·tokio | [초기 교환](initial-exchange.md), [중계 수명](relay-session.md), [접속 ticket](relay-access-ticket.md) |
| `crates/app/src/relay.rs` `RelayRuntime::snapshot` `RelaySnapshot` `ActivePeer` `HandshakingGuard` | 관리 상태·제어 | 상태 잠금 한 번으로 메타데이터를 복사한다. 실제 peer 주소·초기 교환 수를 보존한다. RelaySessionControl 구현은 세션 소유자에게 종료를 전달한다. | HTTP·스트림 읽기 | Clock·domain·제어 알림 | [관리 조회](management-query.md), [관리자 종료](management-terminate.md) |
| `crates/app/src/relay/handshake.rs` `exchange`; `crates/app/src/relay/target.rs` `dispatch`; `crates/app/src/relay/matching.rs` `dispatch`; `crates/app/src/relay/session.rs` `run_session_controlled` | 처리 단계 | 각각 시작 바이트·H, 정책·C·대상 L, 대기·W·Lw·원자적 매칭, 두 방향 중계·E·관리자 종료·Kw 진행을 맡는다. | OS 소켓·신호·작업 생성 | RelayRuntime·RelayStream·domain | [중계 수명](relay-session.md), [WS 스트림](websocket-stream.md) |
| `crates/app/src/relay.rs` `RelayService::request_shutdown` `RelayService::request_fatal_shutdown` `RelayService::drive_shutdown`; `crates/app/src/lifecycle.rs` `LifecycleService` `Readiness` | 수명 | RelayService는 최초 종료 원인을 유지하며 Running→Grace→Force와 G 감독을 맡는다. LifecycleService는 시작·준비·종료 중·필수 리스너 실패 상태를 제공한다. | OS 신호 수신·소켓 오류 분류 | 공유 상태·시계·watch | [중계 수명](relay-session.md), [상태 리스너](status-listener.md) |
| `crates/app/src/management.rs` `AdminAuth`; `crates/app/src/management/login.rs`; `crates/app/src/management/session.rs` | 관리 인증 | 로그인·실패 제한·세션 검사·CSRF·로그아웃·감사 사건 | HTTP·쿠키 형식 | domain·ports | [관리 세션](management-session.md) |
| `crates/app/src/management/query.rs` `ManagementQueryService`; `crates/app/src/management/terminate.rs` `ManagementTerminateService` | 관리 사용 사례 | 조회 뷰 조립과 활성 상세 검사. 지정 세션 종료 수락·완료 대기·합류 요청 감사 | HTTP·JSON·스트림 소유 | RelayStateSource·RelaySessionControl·Clock | [관리 조회](management-query.md), [관리자 종료](management-terminate.md) |
| `crates/app/src/management/config.rs` `ConfigEditService` `ConfigManagement` `ConfigFileStore` `ConfigDocument` `ConfigView` `ConfigPatch` `ConfigEditResult` `ConfigGetError` `ConfigEditError`; `crates/app/src/management/config/items.rs` `ConfigItem` `ConfigReloadKind` `config_items` | 설정 보기·편집 | 실행 값·파일 분류·이력을 조회하고 공유 재로드 잠금 안에서 원문 검증→보존 편집→후보 준비→파일 교체→commit을 진행한다. descriptor가 계약 순서·형식·편집 가능 여부를 제공한다. | TOML·SHA·HTTP·실제 파일 쓰기 | SharedReload·Clock·EventSink·파일/문서 포트 | [설정 보기·편집](config-edit.md) |
| `crates/app/src/reload.rs` `SharedReload` `ConfigCandidate` `ConfigPresence` `FileClassification` `LastReload` `ReloadTrigger` `ChangeSource` `WatchFailure`; `crates/app/src/reload/ports.rs` `ReloadFileStore` `ReloadTlsStore` `ReloadTicketKeyStore` `ReloadClock` `RuntimeSettingsSource` | 재로드 경계 | 후보·명시 키·파일 상태·결과 이력, 공유 트랜잭션 잠금과 파일·TLS·ticket 공개 키·시계·실행 값 읽기 포트를 정의한다. | OS 알림·HTTP | domain·settings | [파일 감지](file-watch.md), [설정 보기·편집](config-edit.md) |
| `crates/app/src/reload/service.rs` `ReloadService` `PreparedReload` `dynamic_changed_keys` `restart_required_keys`; `crates/app/src/reload/snapshot.rs` `ReloadCell` `ReloadSnapshot` | 재로드 적용 | 부작용 없는 후보 준비·파일 분류와 실패 없는 commit을 나눈다. Settings·presence·TLS를 한 snapshot으로 발행하고 실제 적용 키·trigger·마지막 결과를 기록한다. | 파일 감지·TLS 라이브러리 | 재로드 포트·EventSink | [파일 감지](file-watch.md) |
| `crates/app/src/reload/ticket_keys.rs` `load` `file_key` | ticket 공개 키 적재 | 켠 역할의 공개 키 파일을 읽어 Settings에 넣고 끈 역할의 키를 비운다. 파일 오류와 두 역할의 같은 키를 `ticket_keys_invalid` 진단으로 바꾼다. 시작과 재로드 후보 준비가 같은 함수를 쓴다. | JWKS 해석·파일 감지 | ReloadFileStore·ReloadTicketKeyStore·domain | [접속 ticket](relay-access-ticket.md), [설정](config-loading.md) |
| `crates/app/src/accounts.rs` `AdminService` `list` | 계정 준비 | 계정 진단·변경 절차·목록 | 숨김 입력·잠금·파일 교체 | domain·ports | [계정 준비](admin-accounts.md) |

### 2.3 adapters와 실행파일 조립

| 코드 위치 | 구성요소 | 주된 책임 | 맡지 않는 책임 | 의존 대상 | 모듈 설계 |
| --- | --- | --- | --- | --- | --- |
| `crates/adapters/src/lib.rs` | 모듈 공개 | 외부 연결 구현을 공개하고 HTTP 보조 모듈은 비공개로 둔다. | 업무 규칙 | 하위 모듈 | 이 지도 |
| `crates/adapters/src/net.rs` `TcpEntryPoint` `TcpRelayStream` `accept_connections` `AllowSource` `AcceptedSettings`; `crates/adapters/src/target.rs` `TcpTargetConnector` | TCP | 바인드·공유 수락·현재 IP/L 판정과 CAS, 수락 B/H/U/K를 `AcceptedSettings`로 보존, 공통 초기 교환→dispatch 호출, Linux 오류 분류, DNS와 새 대상 소켓의 현재 K 적용 | 매칭·대상 정책 | app·domain·tokio·socket2 | [TCP 진입점](tcp-entry-point.md), [중계 수명](relay-session.md) |
| `crates/adapters/src/listener.rs` `Listener` `SupervisedListener` `ListenerFailure` `is_fatal_accept_error` | 리스너 감독 | 취소 가능한 수락 포트, OS별 복구 불가 오류 분류, 최초 장애의 감독 채널 전달 | 재시도 횟수·G·프로세스 종료 판정 | tokio·std 소켓 오류 | [상태 리스너](status-listener.md), [TCP 진입점](tcp-entry-point.md) |
| `crates/adapters/src/websocket.rs` `WsEntryPoint` `check_upgrade` `exchange_handshake`; `crates/adapters/src/upgrade.rs` `read_head` `check` | WS·WSS 진입점 | 공유 리스너 수락·WSS 협상, 경로별 역할과 해당 Origin/IP·M/Kw 검사, 101 뒤 H 읽기와 suffix·선택한 진입점의 공통 서비스 인계 | Repeater 해석·전달 헤더 판정 | app·TLS·헤더 도구 | [WS 진입점](websocket-entry-point.md) |
| `crates/adapters/src/ticket.rs` `TicketJwksStore`; `crates/adapters/src/ticket/jwks.rs`; `crates/adapters/src/ticket/jws.rs` `verify` | 접속 ticket 어댑터 | 공개 키 파일(JWKS)의 크기·키 수·P-256 좌표 해석, JWS 헤더·중복 멤버·ES256 R‖S 서명·클레임 자료형 확인 뒤 domain 정책 호출. 서명 통과 뒤에만 `jti`를 거절에 담는다 | 사용 기록·101 뒤 대조·파일 감시 | aws-lc-rs·serde_json·domain·app 포트 | [접속 ticket](relay-access-ticket.md) |
| `crates/adapters/src/websocket_stream.rs` `WsRelayStream`; `crates/adapters/src/websocket_stream/frame.rs` `FrameDecoder`; `crates/adapters/src/websocket_stream/keepalive.rs` `Keepalive` | WS 스트림 | 페이로드 청크·마스크·분할·논리 M·같은 방향 B·제어 프레임·Close와 고정 Ping·Pong 기한 FIFO | H·C·W·E·G 판정·사건 | RelayStream·tokio·헤더 도구 | [WS 스트림](websocket-stream.md) |
| `crates/adapters/src/websocket_stream/tests.rs` | WS 내부 시험 | 메모리 스트림·부분 쓰기·M/B·제어·Close·Kw 회귀 시험 | 실제 외부 Viewer | 테스트 전용 tokio | [WS 스트림 7절](websocket-stream.md#7-테스트-계획) |
| `crates/adapters/src/tls.rs` `load_acceptor` `RustlsTlsStore` `TlsSource`; `crates/adapters/src/tls_listener.rs` `TlsListener` | TLS | 파일 검사·후보 acceptor·실행 중 스냅샷 선택·접속별 협상 | 관리 인증 | rustls·tokio-rustls·app | [TLS](tls.md), [관리 리스너](management-listener.md) |
| `crates/adapters/src/management_http.rs` `ManagementListener` `ManagementServices` `DashboardHints` `answer` `static_answer`; `crates/adapters/src/dashboard_assets.rs` `ASSETS` `find_asset` | 관리 HTTP·정적 파일 | 관리 리스너와 API 분기, 보호 경로 세션 검사, 정확한 내장 파일 표, `DashboardHints`의 동적 E+5 힌트 계산, 응답 보안 헤더 | 관리 업무 규칙 | app·axum·TLS | [관리 리스너](management-listener.md) |
| `crates/adapters/src/management_http/session_api.rs` `answer`; `crates/adapters/src/management_http/query_api.rs` `answer`; `crates/adapters/src/management_http/terminate_api.rs` `answer`; `crates/adapters/src/management_http/config_api.rs` `answer`; `crates/adapters/src/management_http/logs_api.rs` `answer` | 관리 API 어댑터 | 인증→PATCH 포함 CSRF→경로·메서드·입력 검증, 서비스 호출, 계약 JSON·쿠키. 설정 GET/PATCH는 spawn_blocking에서 실행하고 PATCH 본문은 64KiB로 제한한다. 최근 로그는 링 버퍼에서 조건 검색·페이지네이션한다. 종료 수락 뒤 완료 대기에만 U를 멈춘다. | 상태·스트림·파일 소유 | app 관리 서비스·axum·serde_json | [관리 세션](management-session.md), [조회](management-query.md), [종료](management-terminate.md), [설정 편집](config-edit.md) |
| `crates/adapters/src/status_http.rs` `StatusListener`; `crates/adapters/src/admission.rs` `AdmittingListener` `AdmittedAddr`; `crates/adapters/src/request_deadline.rs` `DeadlineListener` `IdleControl` | 상태·수락·기한 | /livez·/readyz, 공유 Lm의 현재 상한 읽기·CAS, `AdmittedAddr`로 수락 U 메타데이터 전달, 관리 요청 U와 종료 대기 일시 정지 | 준비 상태 전이 | app·domain·axum·tokio | [상태 리스너](status-listener.md), [관리 리스너](management-listener.md) |
| `crates/adapters/src/config_file.rs` `load` `load_candidate` `parse_candidate` `load_admin` `read_tcp_listener` `read_websocket_listener` `read_websocket_role` `read_recent_logs`; `crates/adapters/src/config_file/reader.rs`; `crates/adapters/src/config_file/listen.rs`; `crates/adapters/src/toml_text.rs` | 설정 읽기 | v4의 부모 listen/TLS/trusted_proxies와 역할별 enabled/allow/Origin, 최근 로그 상한을 검증한다. 시작과 후보는 같은 바이트에서 Settings·presence를 만든다. | 재로드 판정·XFF 해석·로그 보관 | app·domain·toml | [설정](config-loading.md) |
| `crates/adapters/src/config_edit.rs` `FsConfigFileStore` `TomlConfigDocument` `edit_change` | 설정 파일 편집 | SHA-256 revision, 중첩 역할 테이블·dotted/inline 키의 source span 보존 편집, 임시 파일 동기화→원본 revision 재확인→rename 교체 | 후보 수용·인증·HTTP | app 포트·toml_edit·aws-lc-rs·std 파일 | [설정 보기·편집](config-edit.md) |
| `crates/adapters/src/accounts_file.rs` `load`; `crates/adapters/src/accounts_file/store.rs` `AccountsFile`; `crates/adapters/src/accounts_file/hasher.rs` `Argon2Hasher` `Argon2Verifier` | 계정 파일 | 읽기·검증·잠금·원자적 교체·Argon2id 생성과 검증 | 계정 규칙 | app 포트·toml·argon2 | [계정 준비](admin-accounts.md), [관리 세션](management-session.md) |
| `crates/adapters/src/file_watch.rs` `ReloadWatcher` `FsReloadFileStore` `SystemReloadClock`; `crates/adapters/src/file_watch/feed.rs` `ChangeQueue` `NotifyFeed` | 변경 감지 | OS 알림·유실 재검사·주기 확인·안정화·세대 관리·후보 읽기 | 후보 수용 판정 | notify·tokio·app 재로드 | [파일 감지](file-watch.md) |
| `crates/adapters/src/ops_log.rs` `JsonLinesEventSink` `LogEntry` `RecentLogs` `format_line`; `crates/adapters/src/clock.rs` `SystemClock` `timestamp`; `crates/adapters/src/crypto.rs` `RelayIdKey` `SessionIdKey` `SystemTokenSource` | 운영 기반 | 사건 JSON Lines와 순서 있는 필드 목록, 최근 로그 메모리 링 버퍼, UTC와 단조 시각, ID HMAC·세션 ID 순열·토큰 난수. 같은 출력 잠금 안에서 보관 순번과 줄 순서를 정한다. | 사건 의미·매칭 정책 | app 포트·serde_json·aws-lc-rs | [운영 로그](ops-log.md), [중계 수명](relay-session.md) |
| `crates/adapters/src/terminal.rs`; `crates/adapters/src/terminal/windows_console.rs` | 터미널 | 숨김 입력·확인·Ctrl+C, Windows 콘솔 모드 저장·복구 | 비밀번호 규칙 | rpassword·tokio signal·windows-sys | [계정 준비](admin-accounts.md) |
| `src/main.rs` `main`; `src/cli.rs` `parse`; `src/admin.rs` `run` | CLI 조립 | 명령·옵션·진단·종료 코드, admin 포트 주입 | 계정 규칙 | clap·app·adapters | [시작](serve-startup.md), [계정 준비](admin-accounts.md) |
| `src/serve.rs` `run` `listen_until_shutdown` `config_management` `generate_startup_keys` `ShutdownRequest` `wait_for_shutdown` `begin_process_shutdown` `begin_relay_shutdown` `finish_relay_shutdown` | 서버 조립 | 설정·로그·계정·키·TLS·감지·리스너를 준비한다. 같은 reload cell/source와 SharedReload를 실행 reader·watcher·설정 서비스에 주입한다. 종료 시작의 현재 G, 첫 원인·두 번째 신호·Windows 콘솔 닫기·작업 회수·종료 코드를 연결한다. | 매칭·바이트 전달·OS 오류 분류 | 제품 crate·tokio | [시작](serve-startup.md), [파일 감지](file-watch.md), [설정 편집](config-edit.md), [중계 수명](relay-session.md) |
| `crates/adapters/src/file_watch.rs` `reopen_log`; `crates/adapters/src/ops_log.rs` `JsonLinesEventSink::reopen` `file_path` | 로그 파일 교체 | 재로드와 분리한 로그 알림·Dp 재열기, 같은 줄 쓰기 잠금의 핸들 교체, 실패 연속 구간 첫 경고·사건 | 로그 보관·압축·삭제 | notify·tokio·app 사건 | [운영 로그](ops-log.md), [파일 감지](file-watch.md) |

### 2.4 개발 도구와 정적 대시보드

| 코드 위치 | 구성요소 | 주된 책임 | 맡지 않는 책임 | 의존 대상 | 모듈 설계 |
| --- | --- | --- | --- | --- | --- |
| `xtask/src/main.rs` `main`; `xtask/src/cargo.rs` `metadata`; `xtask/src/layers.rs` `check` | 도구 진입·의존 검사 | 하위 명령 선택, Cargo 메타데이터, 허용 crate·버전 요구 검사 | 제품 실행 | std·serde_json | [xtask](xtask.md) |
| `xtask/src/scenarios.rs` `run` `locate`; `xtask/src/scenarios/spec.rs` `parse`; `xtask/src/scenarios/markers.rs` `scan`; `xtask/src/scenarios/test_run.rs` `run`; `xtask/src/scenarios/report.rs` `Report` | 시나리오 보고 | 수용 표·표식·Cargo 테스트 결과 연결과 부분·미연결·실패 판정 | 책임 문장 의미 판정 | Cargo 출력·std | [xtask](xtask.md) |
| `xtask/src/tokens.rs` `run`; `xtask/src/tokens/front_matter.rs` `parse_document`; `xtask/src/tokens/screen_css.rs` `check`; `xtask/src/tokens/shared.rs` `write` `check`; `xtask/src/tokens/fonts.rs` `write` `check`; `xtask/src/dashboard_serve.rs` `run` | 대시보드 도구 | 토큰·`@font-face` 생성·`web/dashboard/`와 `web/help/`의 CSS 검사·로그인 공통 복사본과 글꼴 복사본 대조·루프백 미리보기 | 제품 HTTP 제공 | std·로컬 정적 서버 | [xtask](xtask.md) |
| `xtask/src/notices.rs` `run` `render` `collect_texts` | 배포 의존성 고지 | 두 대상의 일반·빌드 의존성 합집합과 고지 전문, 내장 글꼴 라이선스 전문 생성·대조 | 허용 라이선스 판정·프로젝트 MIT 고지 확정 | Cargo 출력·std·serde_json | [xtask](xtask.md), [라이선스 검사](../how-to/check-dependency-licenses.md) |
| `xtask/src/package.rs` `run` `check_version` `sha256` | 배포 묶음 | 실행한 OS의 release 빌드·고지 대조·묶음 압축·SHA-256 파일, 버전 형식 검사 | Release 게시(GitHub Actions `release` 작업) | Cargo·OS `tar`·std | [xtask](xtask.md), [릴리스 how-to](../how-to/release.md) |
| `xtask/src/static_server.rs` `Site` `bind` `run` `percent_decode` | 로컬 정적 서버 | 두 미리보기 명령의 루프백 바인드, 요청 해석, 마운트 아래 경로 검사, 이동·404·응답 헤더 | 제품 HTTP 제공, 동시 요청 처리 | std | [xtask](xtask.md) |
| `xtask/src/help_site.rs` `run` `LANGUAGES`; `xtask/src/help_site/checks.rs` `check` `chapters` `check_book` `check_references`; `xtask/src/help_site/html.rs` `start_tags` `Tag`; `xtask/src/help_serve.rs` `run` | 도움말 사이트 도구 | mdBook 0.5.4 확인, 책 모으기와 빌드, 사이트 루트·토큰·글꼴 복사, 장 목록·제목·링크·그림·외부 자원 검사, `/vnc-relay/` 미리보기 | Pages 게시(GitHub Actions `help` 워크플로), 브라우저 동작 시험 | mdBook·std·토큰 생성·배포 묶음 복사·로컬 정적 서버 | [xtask](xtask.md), [도움말 게시](../how-to/publish-help.md) |
| `xtask/src/ticket.rs` `parse` `run` `write_keys` `load_key` `issue` | 시험용 접속 ticket | 역할별 P-256 키 쌍과 공개 키 JWKS·개인 JWK 파일 생성, ES256 ticket 서명(사용자 결정 2026-11-13(Q233)) | 운영 발급·운영 키 보관, ticket 확인 | aws-lc-rs·serde_json·std | [xtask](xtask.md), [수동 운영 3.10.1절](../how-to/operate-relay.md#3101-시험용-키와-ticket으로-확인하기) |
| `docs/help/ko/`; `docs/help/en/`; `web/help/theme/`; `web/help/site/` | 도움말 원본 | 언어별 책 원문·그림·책 설정, 두 책이 함께 쓰는 테마(머리·상단 막대·CSS·스크립트), 사이트 루트(언어 선택·없는 페이지·공통 자원) | 기준 문서, 대시보드 화면 | mdBook·브라우저 | [도움말 사이트 스펙](../specs/help-site.md), [도움말 작성 규칙](../help/AGENTS.md) |
| `web/dashboard/app/index.html`; `web/dashboard/app/main.js`; `web/dashboard/app/router.js`; `web/dashboard/app/shell.js`; `web/dashboard/app/operations.js`; `web/dashboard/app/session.js`; `web/dashboard/app/settings.js`; `web/dashboard/app/logs.js` | 화면·라우팅 | 부팅·세 항목 셸·운영 현황·세션 상세·설정 보기·동적 설정 편집·최근 로그·화면 전환 | 서버 규칙 | 공통 JS·관리 API | [대시보드 컴포넌트](dashboard-components.md) |
| `web/dashboard/app/api.js`; `web/dashboard/app/query.js`; `web/dashboard/app/model.js`; `web/dashboard/app/settings-model.js`; `web/dashboard/app/panels.js`; `web/dashboard/app/format.js`; `web/dashboard/app/copy.js`; `web/dashboard/app/language.js` | 화면 공통 상태 | API 통신·조회 모델·v4 편집 구역 순서·줄 진단 대응·양쪽 전송 경로·전달 주소·목록별 성공 개수·문구·언어. 최근 로그는 필터·증분·이전 커서와 다운로드 요청을 분리한다 | 중계 상태 소유 | 관리 API·브라우저 | [대시보드 컴포넌트](dashboard-components.md), [최근 로그 흐름](dashboard-operations-overview.md#16-최근-로그-scr-006) |
| `web/dashboard/app/components/`; `web/dashboard/app/styles/`; `web/dashboard/app/app.css`; `web/dashboard/app/tokens.css`; `web/dashboard/app/fonts/` | 공통 표시 | 컴포넌트·공통 스타일·화면 스타일·생성 토큰·글꼴 복사본 | 서버 권한 | 브라우저 | [프런트엔드 컨벤션](../conventions/frontend.md) |
| `web/dashboard/login/index.html`; `web/dashboard/login/main.js`; `web/dashboard/login/login.js`; `web/dashboard/login/login.css`; `web/dashboard/login/components/`; `web/dashboard/login/styles/`; `web/dashboard/login/language.js`; `web/dashboard/login/tokens.css`; `web/dashboard/login/fonts/` | 로그인 | 세션 없는 로그인 화면과 tokens가 만드는 공통·글꼴 복사본 | 보호된 app 파일 직접 참조 | 관리 세션 API | [대시보드 컴포넌트](dashboard-components.md) |
| `web/dashboard/fonts/` | 글꼴 원본 | Pretendard woff2 원본과 OFL 전문. 제공하지 않는다 | 제공 경로 | 없음 | [프런트엔드 컨벤션 2.4절](../conventions/frontend.md#24-글꼴-파일) |
| `web/dashboard/preview/`; `web/dashboard/tests/` | 개발용 페이지 | 공통 미리보기·가짜 API 브라우저 시험. `logs-scenarios.js`의 `shot-*`는 SCR-006 16개 상태, `settings-scenarios.js`의 `shot-scr004-*`·`shot-scr005-*`는 v4 설정 프레임을 실제 화면 함수로 재현한다 | 제품 번들 제공 | 브라우저·Rust 실행기 | [대시보드 검증](../how-to/verify-dashboard-frontend.md) |
| `web/dashboard/tests/help-link-checks.js` | 도움말 링크 시험 조각 | 다섯 시나리오 페이지가 부르는 checkHelpLink·checkHelpLanguageChange 함수로 SCR-001~006의 링크 속성·이름 구성·배치·언어 갱신을 확인한다 | 외부 도움말 사이트 이동·조회 | 화면 시험 DOM·브라우저 | [대시보드 컴포넌트](dashboard-components.md#도움말-링크-조립) |

### 2.5 통합·시스템 시험

아래 영역은 실제 `/// spec:` 표식에서 모았다. `부분` 여부와 이번 실행 결과는 이 목록에서 판정하지 않는다. domain·app·adapters의 단위 시험은 위 소스 파일 안에도 있다. 표식이 없는 시험은 내부 규칙을 확인한다.

| 코드 위치 | 표식의 스펙·시나리오 영역 |
| --- | --- |
| `tests/cli.rs`; `tests/dashboard_assets.rs`; `crates/adapters/tests/accounts_contract.rs` | 표식 없음. CLI·정적 파일 표·계정 계약 내부 검사 |
| `tests/code_map.rs` `code_map_paths_and_symbols_resolve_to_current_code` | operations.md: OPS-022. §2 경로·정의와 §3·§4 파일 단위 연결 근거 |
| `tests/dependency_notices.rs` `distribution_notices_match_locked_dependencies_and_license_texts` | operations.md: OPS-021 partial. 오프라인 고지 대조만 확인하며 프로젝트 LICENSE는 확인하지 않는다. |
| `tests/admin.rs` | operations.md: OPS-029·030 |
| `tests/serve.rs` | operations.md: 시작·종료·L/H·상태·관리 보안·IP·중복 시작. `every_relay_entry_point_accepts_only_its_own_source_ip`는 여섯 역할×여섯 IP의 36쌍과 공유 수락 뒤 역할별 403을 다룬다. `server_only_websocket_roles_open_each_shared_listener`는 Server만 켠 공유 소켓·role 없는 수락 사건을 다룬다. `enabled_websocket_roles_require_their_policy_and_shared_tls_at_startup`는 역할 allow·부모 TLS 필수를 다룬다. 기한 시험은 U/H와 경로 판정 전후 role을 구분한다. R1 실행 결과는 이 표에서 판정하지 않는다. |
| `tests/unix_shutdown.rs`; `tests/windows_console.rs`; `tests/common/shutdown.rs` `ServerProcess` | operations.md: OPS-007~010·042·043(양 OS), OPS-044(Windows). Windows는 MGT-027·OPS-029·OPS-030과 Ctrl+C·Ctrl+Break도 확인한다. OPS-030 시험은 ConPTY 출력에서 제어 순서를 뺀 화면 문자로 비교한다(`tests/common/log_reopen.rs` `plain`). 공통 본문에는 표식이 없다. |
| `tests/dashboard_browser.rs` | dashboard.md: 컴포넌트·목 응답을 쓰는 DASH-001~022·024~064 부분 증거. 설정 화면(DASH-028~053)은 `web/dashboard/tests/settings.html`, 최근 로그 화면(DASH-054~061·063)은 `web/dashboard/tests/logs.html`의 계약 기반 가짜 API를 쓴다. |
| `tests/dashboard_live.rs`; `tests/dashboard_live/login.rs`; `tests/dashboard_live/overview.rs`; `tests/dashboard_live/detail.rs`; `tests/dashboard_live/cross.rs`; `tests/dashboard_live/settings.rs`; `tests/dashboard_live/settings_view.rs`; `tests/dashboard_live/settings_edit.rs`; `tests/dashboard_live/settings_recovery.rs`; `tests/dashboard_live/help_screenshots.rs` `capture` | dashboard.md: 실제 로그인·중계·관리 조회·상세·종료·언어·폴링·Si 만료의 DASH-001~014·016~022·024~027, 실제 설정 파일·감시기·`GET`·`PATCH /api/v1/config`를 쓰는 설정 보기·편집의 DASH-028~053. management.md: MGT-040·042·043. `settings.rs`는 세 설정 모듈의 공통 도우미다. 표식은 최상위 Rust 테스트 함수에만 둔다. `help_screenshots.rs`는 표식 없는 Linux 전용 `#[ignore]` 캡처 도구(`capture_help_screenshots`)다. 실서버 화면 12장을 도움말 그림으로 다시 만든다. |
| `tests/dashboard_live/logs.rs`; `tests/dashboard_live/logs_download.rs`; `tests/dashboard_live/logs_states.rs`; `tests/dashboard_live/logs_accessibility.rs`; `tests/dashboard_live/transports.rs` | dashboard.md: DASH-054~063. 실제 사건·로그 GET과 화면 필드·필터·검색·커서·증분을 대조한다. 다운로드는 CDP가 정한 폴더의 파일과 완료 사건을 읽는다. 실패·실제 300초 유휴 만료, 두 폭·두 언어의 키 입력, TCP·WS·WSS 양쪽 조합과 전달 주소를 확인한다. 실행 함수와 전체 표식은 `tests/dashboard_live.rs`에만 있다. |
| `tests/help_site.rs` | help-site.md: HELP-001~018. `cargo xtask help-site`로 만든 사이트를 시험 안의 정적 서버(`/vnc-relay/` 마운트, 없는 경로에 `404.html`)에 올리고 CDP 헤드리스 브라우저로 연다. mdBook 0.5.4가 필요하다(`MDBOOK` 또는 `PATH`). |
| `tests/common/dashboard_live.rs` `Server` `Pair`; `tests/common/cdp.rs` `Browser`; `tests/common/dashboard_cdp.rs`; `tests/common/dashboard_proxy.rs` `Proxy` `Fault` `Traffic` | 실서버·중계 소켓·CDP 브라우저·전달 프록시의 시험 자원 소유자. `cdp.rs`는 브라우저 실행·CDP 명령·탐색·조건 대기와 네트워크·다운로드 사건 수집을 맡는다. `dashboard_cdp.rs`는 같은 `Browser`에 대시보드 입력·로그인 동작을 더한다. Enter는 CDP 문자 값 `\r`도 보내 실제 폼 기본 동작을 실행한다. 일반 데이터는 실서버에서 받고, 프록시가 주입한 지연·실패·유실은 별도로 기록한다. 공통 모듈에는 표식이 없다. |
| `tests/websocket_browser.rs` `browser_websocket_relays_mode_i_and_ii_over_ws_and_wss_without_novnc_or_websockify` `novnc_repeater_id_completes_rfb_through_ws_and_wss_in_both_server_orders` | websocket.md: WS-020(Chromium 내장 WebSocket과 실제 실행파일의 WS·WSS Mode I·II), WS-025(고정 noVNC 1.7.0 `repeaterID`와 시험 쪽 최소 RFB 서버, `#[ignore]`). 시험 쪽 정적 HTTP 서버가 빈 페이지와 noVNC 파일을 제공한다. |
| `src/serve/listener_tests.rs` `listener_failures_retry_then_drive_fatal_grace_cleanup_and_preserve_requested_races`; `crates/adapters/src/listener.rs` | operations.md OPS-046: 실제 감독 함수·TCP 중계·상태 HTTP에 수락 포트를 연결한 시험, 일시 오류 재시도·치명 오류·최초 종료 원인·G 정리. 어댑터 내부 시험은 실제 OS 수락 오류와 분류를 확인한다. |
| `tests/management_query.rs` | management.md: MGT-001~007·013·015·016·021·023·024·026. operations.md: OPS-033 |
| `tests/management_session.rs` | management.md: MGT-017·020·027. operations.md: OPS-029, OPS-003 부분(계정 파일 내용 변경의 재시작 뒤 반영) |
| `tests/management_terminate.rs` | management.md: MGT-008~014·023. relay.md: RLY-020. operations.md: OPS-011 |
| `tests/relay_entry_scenarios.rs` | relay.md: RLY-001~005·022·023. operations.md: OPS-039 |
| `tests/relay_target_scenarios.rs` | relay.md: RLY-007~009. operations.md: OPS-023 |
| `tests/relay_matching_scenarios.rs` | relay.md: RLY-012~016. operations.md: OPS-005 |
| `tests/relay_log_scenarios.rs`; `tests/relay_dead_peer_linux.rs` | 로그: operations.md OPS-011·012, relay.md RLY-027. 단절: relay.md RLY-025·026 |
| `tests/reload.rs` | operations.md: OPS-003·024~026·035·038·040·070~072. websocket.md: WS-022 |
| `tests/ticket_system.rs`; `tests/common/ticket_server.rs` `Server` `Ws` | 실제 실행파일의 접속 ticket. websocket.md: WS-038·046. relay.md: RLY-029~032. management.md: MGT-052. operations.md: OPS-073. `common/ticket_server.rs`는 ticket을 켠 실서버·WS 클라이언트·관리 API 도우미와 `sx` 만료를 단조 시계로 재는 `sx_deadline`·`assert_closed_between`을 두며 표식이 없다. 서명은 `crates/adapters/tests/common/ticket.rs`를 경로로 가져와 쓴다 |
| `crates/adapters/tests/config_contract.rs` | operations.md: OPS-001·034·038·040·070 |
| `crates/adapters/tests/management.rs` | operations.md: OPS-006·029·033·034·036·072. management.md: MGT-012~014·017~020·022·025·027·053. dashboard.md: DASH-014 |
| `crates/adapters/tests/ops_info_contract.rs` | relay.md: 초기 교환·대상·대기·세션 사건. operations.md: 상한·정리·로그·상태·종료 원인. management.md: MGT-009·023. websocket.md: WS-008·010·019·038~043·045 |
| `crates/adapters/tests/websocket_ticket.rs`; `crates/adapters/tests/relay_ticket.rs`; `crates/adapters/tests/common/ticket.rs` `Signer` | websocket.md: WS-037~046. relay.md: RLY-029~032. management.md: MGT-052. `common/ticket.rs`는 시험용 ES256 서명과 JWKS를 만들며 표식이 없다 |
| `crates/adapters/tests/relay_delayed_cleanup_scenarios.rs`; `crates/adapters/tests/relay_stream_scenarios.rs`; `crates/adapters/tests/tcp_target.rs` | 각각 relay.md RLY-016, RLY-017~019·021, RLY-009·019 |
| `crates/adapters/tests/tls.rs`; `crates/adapters/tests/websocket.rs` | TLS: operations.md OPS-036, websocket.md WS-013. WS 진입점: websocket.md 업그레이드·Origin·IP·TLS, operations.md U·종료·TLS·허용 목록, relay.md RLY-005 |
| `crates/adapters/tests/websocket_relay.rs`; `crates/adapters/tests/websocket_scenarios.rs` | websocket.md: 실제 중계·M/B·제어·Close·Kw·인증 경계. relay.md: 초기 교환·중계·half-close·E·모드. operations.md: OPS-006·023 |
| `tests/log_reopen.rs`; `tests/common/log_reopen.rs`; `tests/windows_console.rs`; `crates/adapters/src/file_watch/reopen_tests.rs` | operations.md OPS-045: 실제 바이너리의 회전 전후 줄 보존·첫 실패 진단·복구·정상 종료, 주입한 알림·시계의 Dp·실패 연속 구간 |
| `examples/relay.toml`; `tests/distribution.rs`; `tests/common/distribution.rs`; `tests/windows_console.rs` | operations.md OPS-019·OPS-031: 실제 설정 로더의 비밀 없는 예시, 예시의 모든 키·기본값 주석과 `config_items` 목록 대조, Linux PTY·Windows ConPTY의 계정 준비→시작→중계→재로드→종료 |
| `tests/performance.rs` | operations.md OPS-017·OPS-018(`#[ignore]`, 작업 5.4): 실제 실행파일의 TCP·WS·WSS 활성 100세션 측정, 느린 수신자 부하와 새 서버의 예열·접속·종료 수명 반복의 자원 판정. 기준은 운영 스펙 6절(Q98·Q118~Q127·Q148)이다. Windows 전용 `handle_snapshot` 모듈은 중계 프로세스의 핸들 목록에서 Event 핸들을 따로 센다(`unsafe` 예외, Q149). |

## 3. 진입점

| 진입점 종류 | 코드 위치(파일·심볼) | 시작 조건 | 다음으로 호출하는 곳 |
| --- | --- | --- | --- |
| CLI | `src/main.rs` `main` | 프로세스 실행 | `cli::parse` → `serve::run` 또는 `cli::parse` → `admin::run` |
| TCP | `crates/adapters/src/net.rs` `TcpEntryPoint::run` | Viewer·Server 수락 | `accept_connections` → `RelayService::handle_connection` → `RelayService::dispatch` |
| WS·WSS | `crates/adapters/src/websocket.rs` `WsEntryPoint::run` | 공유 리스너 수락·TLS·경로별 역할/정책 검사와 101 flush 성공 | `WsRelayStream::new` → `RelayService::handle_connection` → `RelayService::dispatch` |
| 관리 HTTP | `crates/adapters/src/management_http.rs` `answer` | 관리 요청 | `session_api` → `query_api` 또는 `session_api` → `terminate_api` 또는 `session_api` → `config_api` 또는 `session_api` → `logs_api` 또는 `static_answer` |
| 상태 HTTP | `crates/adapters/src/status_http.rs` `StatusListener::run` | 상태 요청 | `LifecycleService::readiness` |
| 파일 변경 | `crates/adapters/src/file_watch.rs` `ReloadWatcher::run` | OS 알림·주기 확인 | `ReloadService::handle_config` 또는 `ReloadService::handle_unreadable` |
| 종료 신호 | `src/serve.rs` `ShutdownSignal::wait` | Ctrl+C·SIGTERM(Ubuntu)·Ctrl+Break·콘솔 창 닫기(Windows) | `wait_for_shutdown` → `begin_process_shutdown` → `begin_relay_shutdown` → `finish_relay_shutdown` |
| 필수 리스너 장애 | `crates/adapters/src/listener.rs` `SupervisedListener`; `src/serve.rs` `wait_for_shutdown` | 복구 불가 `accept` 오류의 최초 보고 | `wait_for_shutdown` → `begin_process_shutdown` → `LifecycleService::listener_failed` 또는 `wait_for_shutdown` → `begin_process_shutdown` → `RelayService::request_fatal_shutdown` |
| H 기한 | `crates/app/src/relay.rs` `RelayService::handle_connection` | 초기 교환 | `exchange` |
| C 기한 | `crates/app/src/relay/target.rs` `dispatch` | Mode I 대상 연결 | `connect` |
| W 기한 | `crates/app/src/relay/matching.rs` `dispatch` | Mode II 대기 등록 | `wait` |
| E 기한 | `crates/app/src/relay.rs` `run_pair` | 활성 중계·정리 | `run_session_controlled` |
| G 기한 | `crates/app/src/relay.rs` `RelayService::drive_shutdown` | Grace 기한 만료 | `RelayRuntime::force_locked` |
| Ping·Pong 기한 | `crates/adapters/src/websocket_stream.rs` `poll_maintenance` | Kw 진행 | `poll_keepalive` → `Keepalive::poll` |

## 4. 핵심 흐름

다음은 현재 Mode II 흐름이다. Server가 `ID:0123`으로 먼저 기다리고 Viewer가 같은 ID로 접속한다([RLY-010](../specs/relay.md#7-수용-테스트)). 요청 형식은 [초기 교환 계약](../contracts/repeater-handshake-v1.md), 내부 처리는 [중계 수명 설계](relay-session.md)를 따른다.

| 순서 | 코드 위치(파일·심볼) | 하는 일 | 넘기는 값·바뀌는 상태 |
| --- | --- | --- | --- |
| 1 | `crates/adapters/src/net.rs` `accept_connections` | IP·L·K를 검사하고 수락 사건을 기록한다. | 스트림·ConnectionContext·L permit |
| 2 | `crates/app/src/relay.rs` `handle_connection`; `crates/app/src/relay/handshake.rs` `exchange` | 절대 H 안에서 250바이트를 모은다. Viewer에만 시작 바이트를 쓴다. | HandshakeParser의 요청 결과와 후속 데이터 |
| 3 | `crates/app/src/relay.rs` `dispatch`; `crates/app/src/relay/matching.rs` `dispatch` | Server의 Mode II 요청을 등록한다. | 같은 잠금 안에서 점유·스트림·Lw·W 기한을 연결한다. |
| 4 | `crates/app/src/relay/matching.rs` `dispatch`; `crates/domain/src/matching.rs` `WaitingTable` | 같은 ID Viewer가 도착하면 관찰된 EOF·오류·W 만료를 먼저 확인하고 Claim으로 옮긴다. | Lw 반환, L 두 자리 유지, 한 소유자에게 두 연결 이동 |
| 5 | `crates/app/src/relay.rs` `run_pair`; `crates/app/src/relay/session.rs` `run_session_controlled` | 활성 등록 뒤 후속 데이터를 먼저 전달한다. 두 방향 부분 쓰기를 각각 센다. | 새 session_id, session_started, 방향별 B와 성공 쓰기 카운터 |
| 6 | `crates/app/src/relay/session.rs`; `crates/domain/src/session.rs` `Session` | EOF·E·오류·관리자 종료·Force에 맞춰 한 세션을 정리한다. 최초 원인을 보존한다. | closing 동안 ID 점유. 스트림→L→최종 모델→활성·점유 제거·최근 종료 |
| 7 | `crates/adapters/src/ops_log.rs` `JsonLinesEventSink::record` | 최종 사건을 JSON Lines로 기록한다. | session_closed 한 번 |

Mode I는 3~4단계 대신 `relay/target.rs`가 이름 정책→DNS→최종 IP 정책→같은 L의 대상 자리→단일 주소 연결을 진행한다. 전체가 같은 C 안에 있다. 성공하면 같은 `run_pair`로 간다.

WS·WSS는 1단계의 수락을 공유 리스너에서 한다. 소켓 상대가 `trusted_proxies`에 있거나 켜진 역할 allow 합집합을 통과하면 연결을 수락하고, HTTP `/relay/viewer` 또는 `/relay/server`로 역할을 정한다. 소켓 상대가 신뢰 프록시이면 `X-Forwarded-For`로부터 전달 주소를 도출하고(실패 시 400 `forwarded_for_invalid`), 선택한 역할의 Origin과 클라이언트 주소 IP를 다시 검사한다. 101을 flush한 뒤 `WsRelayStream`과 `EntryPoint::from_parts(listener, role)`, 도출된 `forwarded_ip`의 문맥을 넘긴다. H는 `exchange_at`부터 잰다. Server에는 시작 바이트를 보내지 않는다. `frame.rs`는 프레임·메시지 경계를 스트림 경계로 쓰지 않는다.

`run_pair`는 양쪽 스트림의 `Transport`를 세션과 조회 스냅샷에 보존한다. Mode I 대상은 `Tcp`, Mode II 대상은 실제 Server 전송이다. 운영 로그의 `session_started.server_transport`는 Mode II에만 있고 관리 목록·상세·최근 종료의 `target_transport`는 두 모드 모두에 있다. Kw와 종료는 각 연결의 스트림 유지 처리를 공통 소유 루프에서 진행한다.

### 관리 조회 흐름(Current)

`management_http::answer`는 API 요청을 `session_api::answer`로 보낸다. 로그인 외 요청은 경로 검사 전에 세션을 인증한다. `query_api::answer`는 메서드·폴링 헤더·본문을 검사한다. `ManagementQueryService`는 `RelayStateSource::snapshot`으로 메타데이터를 받고 조회 뷰를 만든다. HTTP 어댑터가 이를 계약 JSON으로 바꾼다([관리 조회 설계](management-query.md)).

`RelayRuntime::snapshot`은 상태 잠금 안에서 단조 시각을 읽고 대기·활성 경과 시간을 계산한다. 그 뒤 서비스가 `as_of`를 읽는다. L·Lm 예산은 상태 잠금 밖에서 각각 읽는다. 모든 값을 한 순간에 포착한 응답은 아니다. 조회는 스트림을 소비하지 않는다.

### 관리자 종료 흐름(Current)

`session_api`의 인증·CSRF 뒤 `terminate_api`가 경로·메서드·폴링 헤더·ID·실제 본문을 검사한다. `ManagementTerminateService::request_terminate`는 `RelaySessionControl`을 호출한다. `RelayRuntime`은 활성 표의 제어 핸들로 기존 소유자를 깨운다. 완료 대기 동안 해당 요청의 U만 멈춘다.

`run_session_controlled`는 같은 절대 E 또는 더 이른 Force까지 정리한다. `run_pair`가 스트림·L·점유를 회수하고 최종 요약과 감사를 남긴다. 서비스의 `complete` 뒤 HTTP가 결과를 직렬화한다. 합류 요청은 기존 정리를 다시 시작하지 않는다([관리자 종료 설계](management-terminate.md)).
### 설정 GET/PATCH 흐름(Current)

`management_http::answer` → `session_api::answer` → `config_api::answer`가 `/api/v1/config`를 처리한다. 세션 인증은 경로·메서드 판정보다 먼저다. PATCH는 공통 CSRF 검사도 통과해야 한다. `config_api`는 본문을 64KiB까지 읽고 `ConfigManagement::get` 또는 `patch`를 `spawn_blocking`에서 호출한다. 응답 연결이 끊겨도 시작된 blocking 트랜잭션은 계속된다.

GET의 `ConfigEditService::get`은 `SharedReload` 잠금 안에서 파일 원시 바이트와 revision을 읽는다. `ConfigDocument::parse`가 만든 후보를 `ReloadService::classify_file`로 분류한다. 응답 items는 파일 후보가 아니라 `running()`·`running_presence()`로 만든다. 조회는 snapshot·사건·이력을 바꾸지 않는다.

PATCH의 `ConfigEditService::patch`는 같은 잠금으로 편집 허용→원본 읽기·revision 일치→원본 전체 검증·재시작 차이 검사→`ConfigDocument::edit`→후보 `prepare_candidate`→바이트 no-op 검사까지 진행한다. `FsConfigFileStore::replace`는 같은 디렉터리 임시 파일을 쓰고 동기화한 뒤 원본 revision을 다시 확인해 원자 교체한다. 성공하면 준비한 후보 그대로 `commit_prepared(ConfigEdit)`를 호출하고 `ConfigEdited`를 기록한다. 원본 바이트가 그대로인 요청은 파일·실행 값·이력을 바꾸지 않는다. 뒤 watcher의 같은 후보도 다시 적용하지 않는다([설정 보기·편집 설계](config-edit.md), [파일 감지 §10](file-watch.md#10-target--설정-v3-실행-값과-공유-트랜잭션)).

설정 descriptor와 PATCH 검사는 29개 편집 키에 같은 조건을 쓴다. 역할 allow/Origin은 해당 역할이 켜져야 하고 부모 trusted_proxies는 어느 역할이든 켜져야 한다. TCP allow와 Mode I 정책도 각각 해당 기능이 켜져야 한다. TLS 네 경로는 재로드 33개에는 포함되지만 편집 불가다. 역할 enabled·부모 listen·recent_logs.max_entries는 재시작 키다. 문서 편집은 부모 설정을 역할 테이블로 잘못 옮기지 않고 중첩 경로의 leaf만 고친다.

### 최근 로그 조회 흐름(Current)

`management_http::answer` → `session_api::answer` → `logs_api::answer`가 `/api/v1/logs`를 처리한다. 세션 인증이 선행되며 CSRF 검증은 필요하지 않다. `X-Relay-Poll: 1` 자동 갱신 요청은 유휴 만료를 연장하지 않는다. `logs_api`는 쿼리 매개변수를 검증하고 `RecentLogs::query`를 부른다. 버퍼 잠금 안에서 커서 범위를 좁히고 그 범위의 `Arc<LogEntry>`를 복제한다. 세션 필터는 범위 밖의 `session_started` 참조도 확보해 연결 식별자 연계를 보존한다. 일치 검사와 페이지 선택은 잠금 밖에서 한다. `src/serve.rs::run`은 기록 싱크와 관리 서비스에 같은 `Arc<RecentLogs>`를 직접 전달한다.



## 5. 시작과 연결

- `src/serve.rs` `run`은 설정·로그·계정을 준비한다. 비동기 런타임과 종료 신호 처리기를 준비한 뒤 `listen_until_shutdown`이 두 시작 키·TLS·재로드 스냅샷·감지기를 준비한다. 감지 준비 실패는 리스너 바인드 전에 끝난다.
- 상태 리스너, 관리 리스너, 나머지 리스너 순서로 바인드한다. 상태 리스너는 시작 중에도 응답한다. 준비 완료 뒤 `server_ready`를 쓰고 관리·TCP·WS·WSS 작업을 실행한다.
- 같은 count-only L을 수락과 Mode I 대상에 넣는다. 별도 Lw는 대기 등록에, Lm은 상태·관리에 공유한다. 현재 상한은 같은 `RuntimeSettingsSource` callback 안에서 읽고 permit CAS까지 마친다. 같은 RelayRuntime을 관리 조회·관리자 종료 포트에 넣는다.
- 첫 종료 원인을 고른 뒤 현재 snapshot의 G를 한 번 읽어 Grace deadline을 고정한다. 준비 상태를 종료 중으로 바꾸고 watcher와 새 중계 수락을 멈춘다. 초기 교환·대기·대상 연결을 정리하며 기존 활성은 Grace 안에서 계속 진행한다. 관리 GET/PATCH는 유예 동안 유지한다. 유예 중 G 변경은 시작한 deadline을 바꾸지 않으며 두 번째 신호는 Force를 요청한다.
- 필수 리스너의 `SupervisedListener`는 첫 복구 불가 수락 오류를 `ListenerFailure`로 알린다. `wait_for_shutdown`이 종료 신호와 이 사건을 함께 기다린다. `begin_process_shutdown`은 리스너 장애에 `LifecycleService::listener_failed`와 `RelayService::request_fatal_shutdown`을 적용한다. 같은 G·자원 정리 경로를 거친 뒤 종료 코드 1을 반환한다. 먼저 선택된 정상 종료 원인은 나중 리스너 오류로 바꾸지 않는다.
- `finish_relay_shutdown`은 모든 진입점 소유자가 끝날 때까지 감독 작업을 유지한다. 그 뒤 관리·상태·감지 작업을 거두고 `server_stopped`를 쓴다.
- 설정은 `config_file`이 읽는다. 객체 연결은 `src/serve.rs`와 `src/admin.rs`의 인수·Arc로 한다. DI 프레임워크는 없다.
- 파일 로그의 원래 경로는 재로드 후보와 분리해 감시한다. 생성·삭제·이름 변경 알림과 독립 Dp 작업이 `JsonLinesEventSink::reopen`으로 핸들을 교체한다. 같은 출력 잠금이 줄 경계를 보존한다. 실패하면 이전 핸들에 계속 쓰고 실패 연속 구간의 첫 실패만 알린다.

## 6. 처음 읽을 코드

1. `src/serve.rs` — 포트 구현체·예산·리스너·종료 감독의 연결을 본다.
2. `crates/app/src/relay.rs`와 `crates/app/src/relay/` — 준비된 연결이 대상·대기·활성으로 이동하는 순서를 본다.
3. `crates/domain/src/matching.rs`와 `crates/domain/src/session.rs` — 매칭 전이와 최초 종료 원인을 본다.
4. `crates/adapters/src/management_http/session_api.rs` — 인증 뒤 조회·종료로 나누는 경계를 본다.

## 7. 확인하지 못한 부분

| 항목 | 확인하지 못한 이유 | 확인 방법 |
| --- | --- | --- |
| 책임 문장의 의미 전체 | 파일 단위 식별자 사용은 호출 근거다. 다만 업무 책임·실행 순서·소유권의 의미를 완전히 판정하지는 못한다. | 자동 위치·연결 검사와 아래 수동 대조를 함께 유지한다. OPS-022는 비부분 표식이다. |
| 모듈 설계 본문의 남은 계획·이전 설명 | 이번 변경은 설계 머리말과 차이 절만 갱신한다. 초기 교환·중계 수명·WS 스트림 본문에 실제 API와 다른 설명이 남는다. | 각 문서의 현재 코드와의 차이 절을 따른다. Target을 유지하며 Current 지도와 구분한다. |
| 외부 VNC 상호운용·OS별 단절·전체 수용 완료 | 이번 구조 대조는 제품 동작 전체를 실행하지 않는다. | [테스트 컨벤션](../conventions/testing.md)과 [개발 환경 검증](../how-to/set-up-and-verify.md)의 별도 증거를 따른다. |

## 8. 계획과 문서 정리(Target)

이 절은 현재 구조 그림에 포함하지 않는다. [TODO 5.3](https://github.com/kayden-kim/vnc-relay/blob/556fab23d15e77125ca2ad5c575089857b312049/docs/TODO.md)의 완료 판정은 별도다. 현재 코드에는 이미 RelayService의 G·두 번째 종료 요청과 serve의 감독 조립이 있다. 이를 미구현 경로로 나열하지 않는다.

`LifecycleService::request_shutdown`이라는 API는 현재 없다. 준비 상태 전이는 `LifecycleService::begin_shutdown`이, 중계 수명 전이는 `RelayService::request_shutdown`이 맡는다. 이 이름을 전제로 한 설계·TODO 문구는 실제 책임 분리와 대조해야 한다. 이 지도는 향후 API 이동을 결정하지 않는다. **이 문서의 판단, 사용자 결정 없음**이다.

## 대조 기록

### R1 리스너·역할 전환 후속(2026-11-08)

- 범위: `entry_point.rs`의 Listener·EntryPoint, `session.rs`의 Transport·양쪽 전송, settings의 리스너·역할 타입과 계획, config_file의 v4 reader, config_edit의 중첩 source span 편집, net의 enabled-role 합집합, upgrade의 경로·역할 보존, 공통 relay·관리 query·ops 사건의 전송 메타데이터다. 기존 파일에서 바꾸며 R2 전달 주소 모듈이나 R3 로그 API 파일을 현재 코드로 추가하지 않는다.
- `Listener` 타입은 두 경계에 있다. domain의 enum은 소켓 종류의 값이고 adapters `listener.rs`의 trait는 수락 I/O 포트다. 두 타입의 이름이 같아도 책임과 의존 방향은 다르다.
- 현재 흐름 설명은 소스 읽기와 작업별 공유 인터페이스를 근거로 갱신했다. 테스트 소스의 존재와 실행 결과를 구분한다. R1 빌드·시험·포매터는 이 문서 작업에서 미실행이며 통합 담당자의 실행 기록으로 판정한다. 이전 기록은 보존한다. 후속: 구현은 커밋 `3df0693`이며, 시험은 커밋 `51bd747`의 두 OS 게이트에서 통과했다([WSS Server·프록시·최근 로그 판정표](../how-to/set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)).


### 2026-11-08 설정 보기·편집과 실행 값 재로드 소스 대조

- 범위: 작업 트리의 `management/config.rs`·`items.rs`, `config_edit.rs`·`config_api.rs`·`config_file.rs`, reload service/snapshot/ports, watcher 최종 read, serve의 공유 객체·종료 G 조립과 runtime reader다.
- §2에 설정 서비스·포트·descriptor·파일 어댑터·HTTP 경계와 실제 reload 타입을 반영했다. 중복 `RelayAllow`는 제거된 코드에 맞춰 지웠다. §3은 관리 HTTP에 `session_api` → `config_api` 대안을 추가했다. §4의 설정 흐름은 문단으로 분리했다. 숫자 단계 표는 기존 Mode II 한 흐름을 유지한다.
- `tests/code_map.rs`의 정의·진입점·인접 단계 해석 규칙을 읽고 실제 정의·사용 경로에 맞춰 적었다. 이번 개정에서는 테스트·빌드·포매터를 실행하지 않았다. 과거 실행 기록은 이번 작업 트리의 자동 검사 통과를 뜻하지 않는다. 후속: 커밋 `eae442f`의 두 OS 게이트에서 `tests/code_map.rs`를 포함한 자동 검사가 통과했다([작업 6.9 판정표](../how-to/set-up-and-verify.md#42-작업-69-판정표)).

### 2026-11-05 리스너 감독과 실서버 대시보드 시험 대조

- 대상: `b5dc9b5`의 `Listener`·`SupervisedListener`와 serve 조립, `tests/dashboard_live.rs`·공통 CDP·전달 프록시·화면별 하위 본문이다. `9cdebc6`에서 Windows 수락 소켓의 차단 모드와 지연 요청 회귀 시험을 보강했다.
- §2의 새 코드 위치와 루트 개발 의존성, §3의 리스너 오류→종료 진입점, §4·§5의 감독·상태·중계 정리 연결을 실제 구현과 대조했다. 리스너 오류 분류와 최초 종료 원인의 보존은 Q114를 따른다. CDP와 시험용 장애 주입은 Q112·Q113을 따른다.
- `9cdebc6`의 Windows·WSL 전체 게이트에서 `tests/code_map.rs` 4건이 모두 통과했다. OPS-022·OPS-046과 실서버 대시보드 26건도 두 OS에서 통과했다. 전체 집계와 증거 범위는 [대시보드 검증 4.19절](../how-to/verify-dashboard-frontend.md#419-실서버-대시보드-시험과-수동-스모크-2026-11-05), [운영 정보 계약 실행 기록](../contracts/ops-info-v2.md#계약-테스트-실행-기록)에 있다.
- WSL 보고서는 `9cdebc6 + 미커밋 변경`으로 표시됐다. 게이트 실행 중 코드·문서를 수정하지 않았다. 이 실행에서 접미사 원인은 별도로 확인하지 않았다. 구조 대조는 외부 VNC 호환성과 사람의 화면 검토 완료를 뜻하지 않는다.
- 후속 Q115는 기존 `operations.js` → `panels.js` 경로에 목록별 성공 응답의 개수 갱신을 연결했다. 새 파일·공통 컴포넌트 API는 없다. `177040d`의 두 OS 전체 게이트에서 코드 지도 4건과 브라우저·실서버 시험이 통과했다([검증 4.20절](../how-to/verify-dashboard-frontend.md#420-목록-개수-배지-q115-2026-11-05)).

### 2026-11-04 구조 대조

| 항목 | 기록 |
| --- | --- |
| 날짜·대상 | 2026-11-04, HEAD `ad5cfb2`와 이 작업의 미커밋 변경. 지도 전체 정리와 `tests/code_map.rs` 추가, 모듈 설계의 머리말·차이 절 변경 |
| 방법 | §2의 경로·Rust 정의, §3 모든 진입점 체인, §4 모든 인접 단계의 파일 단위 사용 근거를 자동 검사한다. Type::method는 method 정의를 찾는다. `a → b`는 진입점 파일이나 a 정의 파일에 b의 사용 토큰이 있어야 한다. 첫 호출은 진입점 파일에 있어야 한다. 핵심 흐름의 다음 단계 심볼 중 하나는 이전 단계 파일이나 흐름 진입점 파일에 사용되어야 한다. 주석·문자열·정의 자체는 사용 근거에서 뺀다. 책임 문장의 의미와 소유권은 수동 리뷰한다. **이 문서의 판단, 사용자 결정 없음** |
| 수동 대조 | net·websocket → handle_connection → dispatch → target/matching → run_pair/session을 읽었다. WaitingTable은 메타데이터, RelayState는 스트림·permit·활성 제어를 소유한다. management의 인증→조회/종료→런타임 포트, serve의 실제 주입, file_watch→reload의 후보 적용, 첫 신호·G 감독·두 번째 신호 조립을 대조했다. |
| 결과 | 구조 대조 완료. stale Target 행과 혼합 그림을 현재 코드로 바꿨다. 빠졌던 모듈·테스트·개발 도구·대시보드 파일을 추가했다. 구조 Verified는 문서 채택이나 전체 동작 통과를 뜻하지 않는다. |
| 표식 연결 | `tests/code_map.rs`의 `/// spec: operations.md:OPS-022`. testing.md §7과 scenarios.rs는 루트 패키지 tests를 수집한다. xtask 자신은 기존 제외를 유지한다. |
| 검사 선택 | 루트 std-only 통합 시험으로 둔다. 기존 scenarios가 읽는 위치이므로 새 xtask 명령이나 예외는 없다. layers는 crate 의존, 이 시험은 문서 경로·정의·파일 단위 연결을 검사한다. 책임 문장의 의미는 사람이 확인한다. **이 문서의 판단, 사용자 결정 없음** |
| 실행 증거 | 아래 실행 기록에 기록한다. |
| 남은 차이 | 초기 교환·중계 수명·WS 스트림 본문의 옛 API·계획 표현. 경로·정의 존재 밖의 책임 의미와 전체 동작은 자동 검사 범위가 아니다. 5.3 완료 여부와 외부 상호운용은 이 기록으로 판정하지 않는다. |

### 실행 기록

대상은 위 HEAD와 미커밋 문서·`tests/code_map.rs` 변경이다. Windows는 `AWS_LC_SYS_PREBUILT_NASM=1`, `TEMP=TMP=D:/vr-tools/tmp`, `CARGO_TARGET_DIR=D:/Workspace/vnc-relay/target`을 썼다. WSL Ubuntu는 `CARGO_TARGET_DIR=$HOME/vnc-relay-target`을 썼다.

| 환경·방법 | 명령 또는 확인 | 결과 |
| --- | --- | --- |
| Windows | `rustfmt --edition 2024 --config skip_children=true tests/code_map.rs` | 통과. 수정한 Rust 파일만 포맷했다. |
| WSL Ubuntu | `cargo test -p xtask` | 통과. 12 passed, 0 failed |
| WSL Ubuntu, 연결 검사 확장 뒤 | `cargo test --test code_map` | 통과. 4 passed, 0 failed. 진입점 13행과 핵심 흐름 7단계의 정의·연결 및 검사기 회귀 시험을 확인했다. |
| WSL Ubuntu | `cargo clippy -p xtask --all-targets -- -D warnings`와 `cargo clippy -p vnc-relay --all-targets -- -D warnings` | 통과. 두 명령 모두 종료 코드 0, 경고 없음 |
| Windows PowerShell | `cargo test -p xtask`와 연결 확장 뒤 `cargo test --test code_map` | 통과. 각각 12 passed와 4 passed, 0 failed |
| 드리프트 주입·복구 | §2의 lib.rs 행에 없는 심볼 `NonexistentCodeMapProbe`를 임시 추가하고 WSL에서 해당 테스트만 실행했다. | 기대한 실패. 종료 코드 101. `line 92: crates/domain/src/lib.rs has no definition for NonexistentCodeMapProbe`를 보고했다. 원래 행 복구 뒤 code_map 시험 3개가 다시 통과했다. |
| Windows 문서 검사 | `python D:/vr-tools/tmp/doc_check.py` | `errors_total=4 known=4 other=0`. 기존 오류만 남았다: dashboard-operations-overview.md:73, tls.md:126, verify-dashboard-frontend.md:179·873. |
| 진입점·핵심 흐름 드리프트 주입·복구 | §3의 첫 호출을 `cli::nonexistent_entry_callee`로, §4 마지막 심볼을 `JsonLinesEventSink::nonexistent_flow_callee`로 임시 바꿨다. 같은 OPS-022 시험을 실행했다. | 기대한 실패, 종료 코드 101. `line 183: unresolved callee`, `line 183: missing entry link`, `line 209: unresolved flow symbol`, `line 208 → 209: missing flow link`를 모두 보고했다. 두 행 복구 뒤 WSL·Windows의 시험 4개가 통과했다. |
| 연결 검사 확장 뒤 lint | WSL `cargo clippy -p vnc-relay --all-targets -- -D warnings` | 통과. 종료 코드 0, 경고 없음 |

진입점 체인은 `또는`으로 대안을 나눈다. `→`와 `·`·쉼표는 순차 연결이다. 식별자가 아닌 설명은 건너뛴다. 각 식별자는 정의 파일이나 같은 이름의 모듈 파일로 해석한다. 파일 단위 사용 근거는 호출 위치의 존재를 확인하지만 함수 안의 실행 순서까지 증명하지는 않는다. 그 의미는 수동 리뷰에 남긴다. **이 문서의 판단, 사용자 결정 없음**이다.
