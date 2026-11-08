# 대상 연결·매칭·중계 수명 모듈 설계

## 작업 6.2 실행 값 경계 (Current / Active)

`RelayService`와 `RelayRuntime`은 `Arc<dyn RuntimeSettingsSource>`를 공유한다. [소비 경계 표](file-watch.md#101-발행-단위와-reader)와 [동적 상한](connection-limits.md#10-target--동적-상한과-시간-제한)에 따라 `relay/target.rs`가 대상 허용 정책과 C를 시도 시작에 읽고, `relay/matching.rs`가 Lw·W를 등록 때 읽는다. `WaitingTable::arrive`는 `RegistrationPolicy`를 값으로 받는다. 세션 엔진은 첫 EOF·오류의 종료 전이 때 E를 한 번 읽는다. 관리자 종료는 `request_terminate`가 수락 시각과 E를 명령에 함께 담는다. 이미 시작한 종료에는 새 E를 적용하지 않는다. B는 `ConnectionContext.buffer_bytes`에서 각 방향으로 전달하며 Mode II의 두 값이 달라도 유지한다. `RelayRuntimeConfig`에는 재시작 대상인 `recent_closed`만 남는다. G는 정상 종료 시작 때 받은 절대 기한으로 유지한다([설정 §1.8](../contracts/config-v3.md#18-재로드-구분), OPS-047~057). 이 변경의 시험은 통합 검증 전이며 기존 Verified 기록은 과거 범위에만 해당한다.

이 문서는 유효한 `PreparedConnection<S>`를 받아 Mode I 대상에 연결하거나 Mode II 상대를 찾고, 양방향 중계와 세션 정리까지 이어 가는 현재 구조를 설명한다. 예를 들어 Server가 `ID:0123`으로 기다릴 때 Viewer가 같은 ID로 접속하면 두 연결을 한 세션으로 옮긴다. 초기 요청 뒤의 데이터부터 상대에게 한 번씩 전달한다.

| 항목 | 값 |
| --- | --- |
| 구조 시점 | Current |
| 문서 상태 | Draft |
| 구현 대조 | 작업 6.2 변경은 Current / 실행 미확인. 이전 범위는 [2026-11-04 대조 기록](#8-현재-코드와의-차이와-연결-순서)을 따른다. |
| 코드 위치 | `crates/domain/src/{matching,session,recent_closed,target_policy}.rs`, `crates/app/src/{ports,events,relay}.rs`와 `relay/{handshake,target,matching,session}.rs`, `crates/adapters/src/{target,crypto,ops_log}.rs`와 `net.rs`의 `TcpRelayStream`, `crates/adapters/tests/{tcp_target,ops_info_contract}.rs` |
| 코드 지도 | [코드 지도](code-map.md#2-코드-위치와-구성요소) |
| 관련 스펙 | [중계](../specs/relay.md): RLY-003·006~021·023·025~027. [운영](../specs/operations.md): OPS-005·007~009·023·039·042~043. [관리](../specs/management.md#2-조회-대상과-상태): MGT-005 일부 |
| 관련 계약 | [Repeater 초기 교환](../contracts/repeater-handshake-v1.md), [설정 1.2·1.5절](../contracts/config-v3.md#12-키-표), [운영 정보 1.5~1.9절](../contracts/ops-info-v1.md#15-사건-목록), [관리 API 1.6~1.10절](../contracts/management-api-v1.md#16-서버-요약-get-apiv1summary), [WebSocket 1.4~1.6절](../contracts/websocket-relay-v1.md#14-데이터-메시지와-제어-프레임) |
| 관련 ADR | [ADR-0003 단일 인스턴스·메모리 상태](../decisions/ADR-0003-single-instance-memory-state.md), [ADR-0004 tokio·rustls](../decisions/ADR-0004-tokio-rustls.md) |

작업 2.8·2.9·2.10을 한 문서로 묶는다. 세 작업이 같은 연결 소유권, L 예산, 활성 등록과 정리 경계를 공유하기 때문이다. 각 작업의 구현 범위는 절과 파일로 나눈다. 이 배치와 아래 내부 API·동시 실행 방식은 모두 **이 문서의 판단, 사용자 결정 없음**이다. 계약이 정한 외부 동작은 바꾸지 않는다. 계약에 없는 오류 대응은 9절의 권고이며, 채택 전 구현 기준이 아니다.

업무 용어는 [용어집](../../GLOSSARY.md#중계-방식과-세션)을 따른다. [초기 교환 설계](initial-exchange.md)의 출력 인터페이스를 재사용한다. TCP·WS·WSS는 유효 요청을 임시로 보류하거나 닫지 않고 `dispatch`에 넘긴다. 초기 교환과 후속 단계를 함께 연결하는 배치는 **이 문서의 판단, 사용자 결정 없음**이다.

## 1. 책임과 경계

| 레이어 | 맡는 책임 | 맡지 않는 책임 |
| --- | --- | --- |
| domain | 최종 IP·포트 허용 판정, 정확한 중계 ID 비교, 대기·점유 상태 전이, 세션 상태·종료 원인·바이트 수, 최대 R개의 최근 종료 목록 | 소켓·DNS·시계·난수·tokio. 시각과 난수 결과를 값으로 받는다. |
| app | `PreparedConnection` 분기, C·W·E, L·Lw 소유권, 원자적 전이, 중계 읽기·쓰기, 사건 생성, 조회용 메모리 상태, 종료 신호에 따른 정리 | `tokio::net`, `tokio::fs`, 프로세스·OS 신호, TLS·WebSocket 프레임 |
| adapters | DNS 해석, 검사한 단일 IP로 TCP 연결, TCP keepalive K, 전송 방식별 종료·단절 감지, 난수·HMAC, JSON Lines | 대상 정책·중복 정책·종료 원인의 재정의 |
| `src` | 구현체 주입, 시작 실패 처리, 리스너·작업의 실행과 회수, OS 종료 신호를 app 수명 관리로 전달 | 매칭 규칙, 바이트 전달, 소켓별 종료 원인 판정 |

app은 tokio의 `time`, `sync`, `io-util`만 제품 코드에서 쓴다. `std::future::poll_fn`으로 여러 진행 조건을 기다린다. app에서 작업을 `spawn`하지 않는다. adapters 또는 `src`의 작업이 app future를 실행한다. `macros`·`rt`·`test-util`은 app 개발 의존에만 둔다. [초기 교환 설계 2·5절](initial-exchange.md#2-내부-구조)의 금지 목록 확장을 함께 적용한다. [Rust 컨벤션](../conventions/rust.md#2-규칙)의 모듈·오류·이름 규칙을 따른다.

공개 입구는 `RelayService::dispatch(prepared, runtime)`이다. 대상 연결 또는 실제 대기·매칭·중계 수명을 끝까지 실행한다. 초기 교환은 `handle_connection(...) -> Result<PreparedConnection<S>, RelayError>`를 반환한다. 해석 완료를 세션 시작으로 취급하지 않는다.

정상 종료 입력은 `RelayService::request_shutdown`이다. 첫 입력은 G를 시작하고 다음 입력은 Force다. 필수 리스너 장애 입력은 `request_fatal_shutdown`이다. Running에서만 `ShutdownTrigger::FatalError`로 G를 시작한다. 이미 Grace·Force이면 원인과 기한을 유지한다. 리스너 장애가 두 번째 OS 요청처럼 유예를 줄이지 않도록 두 입력을 구분한다. 실제 상태 HTTP의 장애 원인은 `LifecycleService`가 유지한다([상태 리스너 설계](status-listener.md), 사용자 결정 2026-11-05(Q114)).

## 2. 내부 구조

다음 그림은 현재 구조다. 실선 화살표는 실행 중 호출·소유권 전달 또는 상태 접근이다. 점선은 구현체를 포트에 주입하는 관계다. 컴파일 의존은 `src` → adapters → app → domain이다.

```mermaid
flowchart LR
    SERVE["src/serve.rs 조립"] --> ENTRY["net.rs·websocket.rs 연결 작업"]
    ENTRY --> HS["RelayService::handle_connection"]
    HS -->|"PreparedConnection 이동"| DISPATCH["RelayService::dispatch"]
    DISPATCH -->|"Mode1"| TARGET["relay/target.rs"]
    DISPATCH -->|"Mode2"| MATCH["relay/matching.rs"]
    TARGET --> PORT["ports.rs TargetConnector"]
    SERVE -.->|"주입"| ADAPTER["target.rs TcpTargetConnector"]
    ADAPTER -.->|"구현"| PORT
    TARGET -->|"연결 쌍"| PAIR["RelayService::run_pair"]
    MATCH -->|"매칭된 연결 쌍"| PAIR
    PAIR --> SESSION["relay/session.rs run_session_controlled"]
    MATCH --> STATE["RelayRuntime의 RelayState"]
    PAIR --> STATE
    PAIR --> LOG["EventSink"]
```

| 요소·파일 | 역할 | 생성 시점·소유자 | 의존 대상 |
| --- | --- | --- | --- |
| `crates/domain/src/target_policy.rs` `TargetPolicy` | `permits_name(name, port)`와 `permits_ip(ip, port)`로 정책 검사. 기존 `TargetItem`, `IpItem::contains` 재사용 | 실행 설정이 원본을 소유하고 대상 시도는 시작 세대의 정책을 보관 | 기존 주소·포트 값 |
| `crates/domain/src/matching.rs` `WaitingTable` | `RelayId`별 `Waiting`·`Claimed`·`Active`·`Closing` 점유 판정. 연결 식별자·세션 식별자를 비교해 전이 | app의 `RelayState`가 소유 | `RelayId`, `Role`, `ConnId`, `SessionId` |
| `crates/domain/src/session.rs` `SessionId`, `Session`, `CloseCause` | 식별자 형식, `active`·`closing`, 첫 종료 원인, 방향별 카운터 | 세션 활성화 때 생성 | std 값 |
| `crates/domain/src/recent_closed.rs` `RecentClosedList` | 정리된 세션의 요약, R 초과 시 가장 오래된 항목 제거 | 프로세스마다 하나 | `session` |
| `crates/app/src/relay.rs` `RelayService`, `RelayRuntime<C>`, `RelayState` | 서비스는 초기 교환과 단계 연결을 맡는다. runtime은 연결 포트·상태 잠금·예산·식별자 포트를 소유한다. | `serve`가 서비스와 공유 runtime을 생성 | domain, `ports`, 허용된 tokio |
| `crates/app/src/relay/target.rs` | Mode I C·정책·L와 대상 연결 결과 | Mode I 연결 작업 | `TargetConnector`, `Clock`, 예산 |
| `crates/app/src/relay/matching.rs` | 대기 I/O·W·Lw, 연결 쌍 인계 | 각 대기 연결 작업 | `WaitingTable`, 스트림 인터페이스 |
| `crates/app/src/relay/session.rs` `run_session_controlled` | 두 방향 중계·EOF·E·전송 종료, 관리자 종료 명령 poll. 최종 등록·예산·사건 정리는 `RelayService::run_pair`가 맡는다. | 매칭 또는 대상 연결을 완료한 작업 | `Session`, 스트림 인터페이스 |
| `crates/app/src/ports.rs` | `TargetConnector`, `RelayIdSource`, `SessionIdSource`, `RelaySessionControl`, `RelayStateSource`와 전송 수명 인터페이스 | trait 정의 | domain·tokio 입출력 trait |
| `crates/adapters/src/target.rs` `TcpTargetConnector` | DNS와 단일 `SocketAddr` TCP 연결, K 적용 | `serve`가 생성 | tokio·socket2·app |
| `crates/adapters/src/net.rs` TCP 스트림 래퍼 | TCP 단절 알림·FIN·입출력 구현. 수락과 연결 쪽에서 같은 래퍼 사용 | 소켓마다 하나 | tokio·app |
| `crates/adapters/src/crypto.rs` | `RelayIdKey`가 `RelayIdSource`를, `SessionIdKey`가 `SessionIdSource`를 구현 | 프로세스마다 하나씩 | aws-lc-rs·app |
| `crates/app/src/events.rs`, `crates/adapters/src/ops_log.rs` | 6절 사건과 JSON 필드 | 사건마다 | 기존 `EventSink` 경로 |

`matching.rs`의 domain 표는 소켓을 갖지 않는다. app의 대기 항목이 실제 스트림과 permit을 소유한다. 표의 연결 키와 app 항목은 같은 잠금 아래 함께 넣고 뺀다. 별도 DB나 외부 상태 저장소는 없다.

## 3. 데이터 모델과 인계 계약

### 3.1 초기 교환에서 받는 값

`PreparedConnection<S>`는 초기 교환의 스트림, `HandshakeRequest`, 후속 데이터, `ConnectionContext`, L의 `BudgetPermit` 하나를 이동받는다. 진입점은 실제 상대 IP·소켓 주소 `peer_addr`, 표시용 수락 시각 `accepted_wall`, 수락·초기 교환 진입 단조 시각, 수락 B와 초기 교환 시작 H를 채운다. 파싱한 요청·IP나 현재 벽시계로 상대 주소·수락 시각을 재구성하지 않는다.

H 감시는 유효 요청 반환으로 끝난다. C·W·E에서 H를 다시 쓰지 않는다. `PreparedConnection`을 복제하지 않는다. L permit을 다시 얻거나 단계 전환 때 반환하지 않는다. 후속 데이터 `n`바이트는 이미 해당 연결의 B를 쓴 상태다.

### 3.2 전송 수명 인터페이스

`RelayStream: AsyncRead + AsyncWrite + Unpin + Send`를 초기 교환부터 정리까지 쓴다. WS Close와 TCP half-close를 종료 action으로 구분한다. 초기 실패도 action을 먼저 실행한다. `WsRelayStream`은 같은 포트를 구현한다.

- `RelayStream`은 위 입출력 trait에 더해 전송 종류, `pending_write_len`, `poll_maintenance`, `poll_close(action)`을 제공한다. TCP·WS 라이브러리 타입을 반환하지 않는다. 별도 `poll_terminal`은 없다. EOF는 읽기 결과가 0일 때, 오류는 읽기·쓰기 오류를 받았을 때 관찰한다.
- `poll_close`의 action은 TCP 쓰기 방향 종료, 계약 사유에 따른 전체 종료, 상대 정상 종료, 서버 종료를 구분한다. TCP 쓰기 종료는 `AsyncWrite::poll_shutdown`에 대응한다. 전체 종료 완료 뒤 래퍼를 drop해 소켓을 회수한다.
- `AsyncRead`의 0바이트는 TCP 읽기 EOF 또는 WS 어댑터가 알려 준 Close다. 빈 WS 바이너리 메시지를 EOF로 바꾸면 안 된다. `RelayStreamError::new(kind).into_io_error()`가 표준 I/O 오류 안에 계약의 종류를 보존한다. app은 `RelayStreamError::classify(&error)`로 읽는다. 명시된 종류가 없는 소켓 오류만 `io_error`다. `ws_protocol_error`, `ws_text_message`, `ws_message_too_large`는 그대로 사건과 `StreamError(kind)` action에 전달한다.
- `AsyncWrite`는 쓰기에 성공한 **페이로드** 길이만 반환한다. WS 어댑터는 무제한 내부 큐에 복사한 길이를 성공으로 알리면 안 된다. 아직 전송 계층에 쓰지 못한 데이터의 소유권은 방향 버퍼에 남긴다.
- WS 어댑터가 저장한 미전달 페이로드도 같은 B에 포함한다. 제어 프레임·TLS 프레이밍은 스트림과 카운터에 섞지 않는다. 자체 프레임 코덱은 사용자 결정 2026-11-01(Q77)을 따른다. M>B 프레임의 스트리밍은 사용자 결정 2026-11-01(Q78)을 따른다. Kw는 `websocket_stream/keepalive.rs`와 공통 소유 루프에 연결되어 있다.

초기 교환 실패는 `HandshakeInvalid`(WS 4008), `HandshakeTimeout`(4009), `ModeDisabled`(1008), `ServerShutdown`(1001) action으로 구별한다. 형식 있는 스트림 오류는 `StreamError(kind)`로 전달한다. WS 어댑터는 종류에 따라 1002·1003·1009를 선택한다. TCP는 추가 오류 바이트 없이 FIN·소켓 정리만 수행한다. `handle_connection`은 실패 판정 뒤 절대 `close_started + E`까지 action을 진행한다. 이후 stream → L permit → 사건 순서로 정리한다. H는 요청 판정의 기한이다. Close 대기 때문에 H를 다시 시작하지 않는다. 초기 실패의 종료 교환에도 기존 E를 쓴다. Close 오류·E 만료는 원래 초기 실패의 사건 사유를 바꾸지 않는다.

초기 실패와 활성화 전 두 연결의 Close 대기는 action 완료·절대 E·Grace의 절대 G·Force 중 먼저 온 경계에서 끝낸다. Grace 수신 때 E나 G를 다시 시작하지 않는다. 대기 중 수명 변경도 wake 원인이다. 경계 뒤에는 stream → permit → 사건 순서를 그대로 지킨다.

중계 엔진은 Viewer와 반대쪽 스트림 타입을 다르게 받을 수 있다. 공유 대기 표에는 `Box<dyn RelayStream>`을 보관한다. `Unpin` 조건으로 poll할 때만 `Pin<&mut dyn RelayStream>`을 만든다. 이 연결당 한 번의 타입 소거로 TCP Server와 TCP·WS·WSS Viewer를 같은 표에서 매칭한다. 중계 청크마다 박싱하거나 복사하지 않는다.

[WS 스트림 설계 5.4절](websocket-stream.md#54-필요한-공통-인터페이스-변경)의 현재 포트와 소유 루프를 따른다. 별도 WS 전용 스트림 trait나 소켓 소유 작업은 없다. 아래 진행 방식은 **이 문서의 판단, 사용자 결정 없음**이다. 시작한 출력 프레임의 drain 범위는 사용자 결정 2026-11-01(Q83)을 따른다.

| 현재 경계 | 구현과 의미 | 소유자 |
| --- | --- | --- |
| 활성 WS의 제어 출력 | 세션 소유자가 페이로드 방향 poll과 별도로 `viewer.poll_flush`를 poll한다. 이미 큐에 있는 Pong과 부분 제어 출력을 진행한다. 입력을 읽거나 B 뒤를 미리 읽지 않는다. `Pending`이어도 두 페이로드 방향을 계속 poll한다. | `relay/session.rs`의 `run_session_controlled`. 오류는 `RelayStreamError::classify`로 분류한 Viewer 출력 오류다. |
| closing의 종료 출력·수신 정리 | `poll_close(action)`을 반복 poll한다. 같은 절대 E·G·Force 경계를 적용한다. | 기존 세션 소유자. 별도 제어 작업을 만들지 않는다. |
| 미완료 WS 출력 프레임 | `pending_write_len(&self) -> usize`가 이미 시작한 프레임의 잔여 페이로드 길이를 반환한다. TCP는 0이다. app 버퍼의 같은 미전달 범위를 E 안에서 마친다. | `RelayStream`과 `WsRelayStream`. 병렬 쓰기 API나 별도 메시지 큐는 없다. |

`poll_maintenance`는 입력을 소비하지 않고 Kw의 출력·기한을 진행한다. C·W·활성 소유 루프가 이를 poll한다. H에서는 `WsRelayStream`의 읽기·쓰기·flush가 같은 유지 처리를 진행한다. `poll_flush`와 유지 처리는 B 뒤의 새 제어 입력을 읽지 않는다. B가 찬 페이로드 뒤의 Pong도 절대 기한을 따른다. [WS 스트림 설계 9.5절](websocket-stream.md#95-kw의-여러-ping과-b-backpressure)의 Q84를 따른다.

### 3.3 공유 메모리 상태

| 이름 | 형태·저장 위치 | 항상 지켜야 할 조건 |
| --- | --- | --- |
| `RelayState` | app 공유 잠금 안의 대기 표·점유 표·활성 표·최근 종료 목록·128비트 발급 카운터·수명 단계 | 한 전이에서 관련 항목을 함께 변경한다. 발급 ID 집합은 없다. 잠금 안에서 비동기 대기·DNS·소켓 쓰기를 하지 않는다. |
| 대기 항목 | `RelayId`, 역할, `conn_id`, 수락 시각, `registered_at`, `deadline = registered_at + W`, `relay_id_ref`, 스트림·후속 데이터·L permit·Lw permit | ID 원문은 비교·인증된 관리 조회에만 쓴다. 로그는 ref만 쓴다. |
| `Claimed` 점유 | ID, 역할 순서의 두 연결 식별자, 선행 대기 메타데이터, 매칭 시각 | 두 연결 식별자가 인계의 소유권을 구별한다. 활성화 때 세션 식별자를 이동한다. 인계 중에도 ID를 점유하며 조회의 대기·활성 목록에는 노출하지 않는다. |
| 활성 항목 | `ActiveSessionSnapshot`의 `Session` 메타데이터, `viewer: ConnectionContext`, `peer: ActivePeer`, 연결 소유자, 종료 요청·완료 알림 | `peer`는 Mode II의 `ActivePeer::Server(ConnectionContext)` 또는 Mode I의 `ActivePeer::Target(SocketAddr)` 중 하나다. `active`와 `closing` 모두 활성 수에 포함한다. Mode II의 양 역할을 함께 점유한다. |
| 방향 버퍼 | 원본 연결의 후속 데이터와 읽기 작업 공간, 미전달 범위의 시작·끝 인덱스 | 각 방향은 해당 원본 연결을 수락할 때의 B 이내다. Mode II의 두 B가 다르면 합계 상한은 두 값의 합이다. Mode I의 대상 방향은 Viewer 수락 B를 이어받는다. |
| `SessionId` | 계약의 `ses_` + 소문자 영문·숫자 26자 | 중계 ID·연결 식별자와 별도다. 프로세스 안에서 한 번 발급한 값을 다시 쓰지 않는다. |
| 최근 종료 항목 | 계약 1.10절의 시작·종료 시각, 최종 두 카운터, 종료 원인과 식별자·모드·전송 | 정리된 활성 세션만 넣는다. 대기 만료·연결 실패는 넣지 않는다. R=0이면 보관하지 않는다. |

세션 식별자는 시작 때 만든 비밀 키와 128비트 카운터로 만든다. 카운터의 big-endian 16바이트에 키가 있는 의사 난수 순열을 적용한다. 순열은 입력과 출력이 일대일인 변환이다. 그 결과를 `SessionId::from_block([u8; 16])`로 인코딩한다. 출력은 `ses_`와 소문자 base32 26자이며 알파벳은 `a-z2-7`이다. 마지막 문자의 쓰지 않는 하위 2비트는 0이다. `SessionId::parse`도 이 정규 형식을 검사한다.

발급 경로에는 OS 난수 호출·충돌 재시도·발급 ID 집합이 없다. 순열과 정규 인코딩이 모두 일대일이므로 카운터가 순환하지 않는 한 재사용은 구조적으로 불가능하다. 키가 없으면 다음 값을 예측할 수 없으므로 관리 API 1.8절의 불투명·무작위 형식을 만족한다고 판단한다. 카운터는 checked 증가하며 128비트 전 범위를 소진하면 재사용하지 않고 발급을 중지한다. 이는 현실적인 운영 수명보다 큰 수학적 경계이며 새 런타임 난수 실패 사건을 만들지 않는다. 이 선택과 이유는 **이 문서의 판단, 사용자 결정 없음**이다.

외부 API도 확인했다. aws-lc-rs 1.18.2는 공개 `cipher::EncryptingKey::ecb`와 `AES_128`을 제공한다. 그러나 [`encrypt`와 `less_safe_encrypt`](https://docs.rs/aws-lc-rs/1.18.2/aws_lc_rs/cipher/struct.EncryptingKey.html)는 모두 `Result<DecryptionContext, Unspecified>`를 반환한다. [원시 AES·block 모듈](https://docs.rs/aws-lc-rs/1.18.2/src/aws_lc_rs/cipher.rs.html)은 `pub(crate)`다. 공개된 무실패 단일 블록 AES API는 없다. 따라서 런타임 실패 없는 포트에는 기존 [`hmac::sign`](https://docs.rs/aws-lc-rs/1.18.2/aws_lc_rs/hmac/fn.sign.html)을 쓰는 고정 6라운드 balanced Feistel 순열을 선택한다(**이 문서의 판단, 사용자 결정 없음**). 일반 데이터 암호화가 아니라 식별자용 128비트 순열이다.

구성은 고정한다. 입력 카운터를 big-endian 64비트 `L`, `R`로 나눈다. 각 라운드 `i = 0..5`에서 `F = HMAC-SHA-256(key, b"vnc-relay/session-id/v1" || [i] || R.to_be_bytes())`의 앞 8바이트를 big-endian 정수로 읽는다. `(L, R) = (R, L XOR F)`로 바꾼다. 최종 `L || R`을 인코딩한다. 각 라운드는 역변환이 있으므로 충돌이 없다. 키·카운터·라운드 입력은 로그에 넣지 않는다. 키는 시작 때만 생성한다. `crates/adapters/src/crypto.rs`의 시험이 고정 벡터·역변환·서로 다른 카운터와 키 Debug 비노출을 확인한다. domain에는 암호 코드나 키를 넣지 않는다.

조회용 `snapshot()`은 같은 잠금 안에서 기준 시각을 한 번 잡고 대기·활성·최근 종료와 집계를 복사한다. HTTP 인증·직렬화는 작업 4.4의 [관리 조회 API](management-query.md)가 맡는다(`ec05fd1`). `connected_at`은 수락 시각이고, `waited_ms`는 등록 단조 시각부터 잰다. `ActivePeer::target_address()`는 Mode II의 실제 Server 상대 주소 또는 Mode I에서 실제 연결한 대상 주소를 반환한다. 표시 시각은 벽시계, 경과 시간은 기존 `Clock::monotonic()`을 쓴다. 시스템 시각이 뒤로 가면 `ended_at`은 `started_at`보다 작지 않게 한다. 예산 현재 사용량은 `ConnectionBudget::current()`로 같은 예산에서 읽는다. 별도 추정 카운터를 기준값으로 만들지 않는다.

조회 스냅샷은 대기를 `connected_at`, 활성을 `started_at` 오름차순으로 정렬한다. 최근 종료는 `ended_at` 내림차순이다. 같은 시각의 순서는 정하지 않는다. ID 원문은 대기 항목에만, 상대 주소는 활성 항목에만 내보낸다. 내부 점유 표를 API 응답으로 직접 직렬화하지 않는다.

현재 초기 교환 중 수는 `crates/app/src/relay.rs`의 `RelayRuntime::track_handshaking()`과 `HandshakingGuard`로 센다. 이 메서드는 `RelayState.handshaking`을 늘리고 가드를 반환한다. 가드는 버려질 때 수를 줄인다. TCP의 `crates/adapters/src/net.rs` `TcpEntryPoint::run`은 `handle_connection` 직전에 가드를 만든다. WS·WSS의 `crates/adapters/src/websocket.rs` `connection`은 101 응답을 flush한 뒤 같은 가드를 만든다. 두 경로 모두 `handle_connection`이 끝나면 성공·실패와 관계없이 가드를 버린다. 성공 경로도 `dispatch` 전에 초기 교환 집계에서 빠진다. 연결 작업이 취소되어 가드가 버려질 때도 수를 줄인다. `RelayRuntime::snapshot`은 같은 상태 잠금 아래 이 수를 `RelaySnapshot.handshaking`에 복사한다.

## 4. 처리 순서

### 4.1 작업 2.8: Mode I 대상 연결

`TargetConnector`는 app에 있고 adapters가 구현한다. 정적 제네릭 포트의 `resolve(name) -> Future<Result<Vec<IpAddr>, TargetError>>`와 `connect(SocketAddr) -> Future<Result<Self::Stream, TargetError>>`를 분리한다. `Self::Stream: RelayStream + 'static`이다. 연결 future는 단일 IP·포트만 받는다. `TargetError`는 해석 실패, 연결 거절, 기타 연결 오류, K 설정 실패를 구분한다. C 초과는 app이 만든다. 연결 future를 취소하면 시도 중인 소켓도 닫힌다. OS DNS 작업이 취소 뒤 완료돼도 결과를 버리고 연결을 열지 않는다.

| 순서 | 위치·처리 | 상태·결과 |
| --- | --- | --- |
| 1 | 진입점이 `HandshakingGuard`를 버린 뒤 `dispatch`가 `Mode1(TargetAddress)`를 선택한다. 종료 중인지 확인한다. | 초기 교환 중 집계는 이미 끝났다. 대상 연결 중 집계를 시작한다. 아직 세션 식별자 없음 |
| 2 | `relay/target.rs`가 같은 실행 설정 읽기에서 정책과 C를 보관하고 `started = clock.monotonic()`, 절대 기한 `started + C`를 한 번 만든다. | IP 리터럴이면 해석을 생략하되 같은 C를 사용. DNS 대기 중 재로드도 시작 정책과 기한을 바꾸지 않음 |
| 3 | DNS 요청이면 `TargetPolicy::permits_name`으로 정규화된 이름·포트를 검사한다. 거절이면 DNS와 연결을 하지 않는다. 허용이면 `resolve`를 기다린다. | DNS 실패·빈 결과는 `resolve_failed`. 결과는 IPv4·IPv6 모두 포함 |
| 4 | 각 IP를 canonical IPv4로 바꾼다. `permits_ip`가 단일 IP·포트를 검사한다. | 금지 주소는 시도하지 않는다. 허용 주소가 없으면 `target_not_allowed`. 이름 허용만으로 IP를 허용하지 않음 |
| 5 | 첫 대상 소켓 생성 직전에 실행 설정 읽기 callback 안에서 현재 L을 읽고 수락 측과 **같은** 예산의 `try_acquire(L)`까지 실행한다. | 없으면 `relay_connection_limit`, `stage = target_connect`, `limit = 판정에 쓴 L`. 대상 소켓 없음(OPS-023) |
| 6 | 허용한 `SocketAddr`를 `connect`에 준다. 연결 성립 후 K를 적용한다. `TcpStream::connect((name, port))`처럼 이름을 다시 해석하는 API는 쓰지 않는다. | 연결 중에도 대상 L permit 유지. 성공 소켓과 실제 IP·포트 반환 |
| 7 | 연결 성공 직후 C·서버 종료를 다시 확인한다. 두 소켓과 permit을 세션 시작 함수로 이동한다. | 활성 등록과 `session_started`, 이어서 `run_session_controlled` |

C는 이름 해석과 모든 허용 후보의 연결 시도를 합친 절대 기한이다. 각 await 뒤에도 `now >= deadline`을 확인한다. 단계·후보마다 다시 C를 주지 않는다. K 설정은 활성 등록 전에 끝낸다. K 설정 실패면 활성화하지 않는다. 대상 소켓을 닫고 대상 L 자리를 돌려준 뒤 Viewer도 실패 경로로 정리한다. `target_connect_failed`의 `reason=connect_error`를 기록하고 `error_kind`는 넣지 않는다. RLY-R18이 대상에도 K를 요구하므로 적용되지 않은 세션을 성공으로 만들지 않는다. 기존 catch-all 원인을 재사용하므로 새 계약 값은 필요 없다. 이는 **이 문서의 판단, 사용자 결정 없음**이다.

포트 내부의 `TargetError::KeepaliveFailed`는 일반 `ConnectError`와 구별한다. K 실패 뒤 다음 DNS 후보를 시도해 세션을 만들면 이 실패 경로의 “세션 없음” 조건을 놓칠 수 있기 때문이다. 같은 함수(`set_keepalive`)가 함께 켜는 `TCP_NODELAY`의 실패도 이 값이다(2026-11-06). 이 값이면 app은 후보 순회를 끝내고 대상 permit을 반환한다. 외부 사건과 `Display` 값은 기존 `connect_error`이며 새 wire 코드를 만들지 않는다. 일반 연결 오류는 나머지 후보를 계속 시도한 뒤 집계한다. 이 내부 구별은 **이 문서의 판단, 사용자 결정 없음**이다. `target_k_failure_is_terminal_but_other_candidate_errors_are_aggregated`가 두 경로와 L 반환·마지막 IP를 비교한다.

DNS 후보는 반환 순서를 유지해 허용된 주소만 순차 시도한다. 한 시점에 대상 소켓은 하나다. 실패한 소켓을 닫은 뒤 다음 후보를 시도하며 같은 대상 L permit과 C를 유지한다. 재해석·자동 재접속은 하지 않는다. 기한 만료는 `connect_timeout`, 시도한 후보가 모두 연결 거절이면 `connect_refused`, 그 밖의 전부 실패는 `connect_error`다. 실패 사건의 `target_ip`는 마지막으로 시도한 IP다. 아직 시도하지 않았다면 생략한다. 금지 후보만 있으면 연결 실패가 아니라 정책 거절이다. 순차 시도는 자원 상한과 실패 원인을 단순하게 추적하기 위한 선택이다(**이 문서의 판단, 사용자 결정 없음**).

대상 L이 부족하면 app은 사유를 이름에 담은 `RelayCloseAction::TargetConnectionLimit`를 사용한다(사용자 결정 2026-11-01(Q79)). 이 값이 이전의 사유 없는 `Reject`를 대체한다. TCP는 기존처럼 추가 바이트 없이 FIN·소켓 정리하며 동작을 바꾸지 않는다. WS·WSS는 101 성공 후 Mode I 대상 연결용 L이 부족할 때 close 4011(`relay_connection_limit`)로 닫는다. 운영 로그는 TCP와 같이 `connection_rejected(stage=target_connect, reason=relay_connection_limit, limit=L)`를 기록한다.

### 4.2 작업 2.9: Mode II 등록과 원자적 매칭

원자적 전이는 동시에 온 작업 중 하나만 상태를 바꾼다는 뜻이다. app은 `RelayState` 잠금 아래 domain 표의 판정과 스트림 이동을 함께 한다. 잠금은 짧게 잡는다. I/O 결과로 이미 관찰한 EOF·오류만 값으로 반영한다.

1. 새 요청의 정확한 `RelayId`로 ref를 계산한다. 수명 상태가 종료 중이면 등록하지 않는다.
2. 잠금을 잡고 현재 시각을 읽는다. 같은 ID 대기의 `now >= deadline`, 관찰한 EOF·오류를 먼저 판정한다. 종료 대상은 매칭 후보에서 제거한다. 제거된 연결의 정리는 잠금 밖에서 끝낸다.
3. 유효한 같은 역할 대기가 있거나 `Claimed`·`Active`·`Closing`이 ID를 점유했으면 후발만 `duplicate_id`로 거절한다. 기존 대기와 세션은 건드리지 않는다. 후발은 Lw를 얻지 않고 대기 사건도 만들지 않는다. TCP는 오류 바이트 없이 FIN, WS·WSS는 Close 4004다(Q74).
4. 유효한 반대 역할 대기가 있으면 Lw 고갈 여부보다 먼저 매칭한다. 추가 대기 항목을 만들지 않으므로 후발용 Lw가 필요 없다. 두 연결의 종료 상태·W·서버 종료를 다시 검사하고 둘을 한 번에 `Claimed`로 옮긴다. 선행 항목의 Lw permit을 이 전이에서 반환한다. L permit 두 개는 유지한다.
5. 반대 역할이 없으면 Lw `try_acquire`를 잠금 안에서 호출한다. 없으면 `waiting_full`로 후발만 거절한다. 있으면 `registered_at`·W 기한과 app 항목을 함께 삽입한다. `waiting_registered`를 기록한다. 처음 온 역할이 Viewer인지 Server인지에 따른 별도 경로를 두지 않는다.
6. 매칭을 확정한 작업이 두 스트림을 소유한다. 다른 연결 작업에는 소유권 인계 완료를 알린다. 빈손이 된 작업은 끝나며 permit을 다시 반환하지 않는다. 잠금 안에서 `Claimed`의 두 연결 식별자·세대를 확인해 활성 항목으로 바꾼다. 세션 시작 함수를 한 번만 부른다.
7. 활성 등록·`session_started` 뒤 두 후속 버퍼를 `run_session_controlled`로 이동한다. `waited_ms`는 먼저 등록된 연결의 등록부터 매칭까지다. 매칭 때문에 `waiting_removed`를 추가하지 않는다. 계약에 그런 reason이 없다.

매칭을 확정할 때 동기화된 카운터에서 다음 값 하나를 가져와 순열·인코딩을 적용한다. 반환형에는 `RandomError`가 없고 발급 집합에 넣는 단계도 없다. 인계 직전 W·관찰한 종료·서버 수명은 그대로 검사한다. 카운터 후보를 계산하는 동안 W를 멈추지 않는다. 시작 키와 순열을 사용하는 이유는 3.3절과 같다(**이 문서의 판단, 사용자 결정 없음**).

대기 작업은 W 타이머, 서버 종료 알림, 매칭 인계 알림, I/O를 함께 기다린다. B가 남으면 그 길이까지만 읽어 후속 데이터 뒤에 붙인다. B가 차면 읽기를 멈춘다. 별도 OS 종료 준비 신호로 FIN을 탐색하지 않는다. EOF는 `read=0`, 오류는 실제 I/O 오류가 돌아올 때 관찰한다. 따라서 매칭 전 B 이상을 보낸 상대가 FIN도 보내면, 저장 바이트를 매칭 뒤 소진하고 다시 읽은 다음에야 종료를 관찰할 수 있다. 그 전에는 매칭할 수 있다. 정상 VNC 끝점의 대기 데이터는 최대 12바이트이므로 이 한계는 대기 중 B 이상을 보내는 상대에 해당한다. 대기 항목의 점유는 W와 Lw로 제한한다.

이 선택은 이식 가능한 읽기 API만 쓰고, TCP half-close(RLY-019)와 관찰된 전이 경합 규칙을 일관되게 적용하기 위한 것이다(**이 문서의 판단, 사용자 결정 없음**). 데이터를 버리거나 B+1 버퍼로 EOF를 탐색하지 않는다. 매칭은 기존 허용 판정을 재사용한다. 재로드된 출발지 허용 목록으로 대기를 다시 거절하지 않는다.

**RLY-014 경합 규칙(이 문서의 판단, 사용자 결정 없음):** W 정확 경계는 만료다. 잠금 아래 관찰된 EOF·오류·만료 또는 매칭 중 먼저 확정된 전이 하나만 실행한다. `Claimed` 인계 때에도 이미 관찰한 종료를 활성화하지 않는다. 활성화 뒤 새로 관찰한 EOF는 세션 종료 경로다. 아직 읽기 결과로 관찰하지 않은 원격의 과거 종료를 소급해 판정하지 않는다.

대기 만료 작업은 `(relay_id, conn_id)`가 모두 같은 항목만 제거한다. 활성 정리 작업은 `(relay_id, session_id)`가 같은 점유만 해제한다. 이전 연결의 늦은 작업이 같은 ID의 새 대기·세션을 지우지 못한다(RLY-016).

구현한 domain API는 `WaitingTable::new(Lw, W)`, `arrive`, `expire`, `observe_eof`, `remove_waiting`, `activate`, `cancel_claim`, `begin_closing`, `release`다. `arrive`는 `Waiting`, `Claimed`, `Expired`, `Rejected` 중 하나를 준다. `Expired(old)`이면 app이 같은 잠금 아래 옛 항목과 Lw permit을 회수하고 같은 새 도착을 다시 처리한다. 새 연결은 아직 등록·거절되지 않았다. `Waiting`을 반환한 전이는 app이 같은 잠금 아래 스트림과 실제 Lw permit을 붙인 뒤 공개한다. domain의 대기 수는 메타데이터이며 별도의 permit 예산이 아니다.

표는 현재 점유 중인 `ConnId`만 집합으로 보관한다. 같은 연결을 다른 ID로 두 번 인계하면 `ConnectionOccupied`를 반환하며 기존 연결은 닫지 않는다. 이 집합은 현재 연결 수에 비례하고 해제 때 지운다. 삭제하지 않는 발급 세션 식별자 집합과 다르다. `DuplicateId`·`WaitingFull`만 계약 거절 코드를 가진다. `InvalidClaim`·`ConnectionOccupied`는 호출 순서 오류이며 새 wire 사유로 기록하지 않는다. `waiting()`·`iter()`·`get()`은 복사 없는 메타데이터 조회다.

### 4.3 작업 2.10: 두 방향 중계

`run_session_controlled`는 두 방향을 같은 세션 소유자에서 번갈아 poll한다. 한 방향의 읽기·쓰기가 막혀도 다른 방향과 종료 신호, 관리자 종료 명령을 계속 poll한다. app에 소켓별 작업 생성이나 무제한 메시지 채널을 추가하지 않는다.

각 방향은 `BufferedDirection`의 `storage`, 미전달 범위 `start..end`, `eof`, `complete`로 진행 상태를 보관한다. 별도 `Buffered`·`Reading`·`Writing` enum은 없다. 초기 후속 데이터가 있으면 같은 저장 공간의 미전달 범위에서 시작한다.

1. 후속 데이터의 미전달 범위를 상대에 먼저 쓴다. 새 읽기보다 항상 앞선다. 성공한 부분 쓰기 길이만큼 인덱스와 해당 방향 카운터를 늘린다.
2. 미전달 데이터가 남아 있으면 다시 읽지 않는다. 모두 소진하면 같은 저장 공간을 재사용한다. `read`에 주는 슬라이스는 남은 B 이하이고, 별도 B 크기 버퍼를 중복 보유하지 않는다.
3. 새로 읽은 바이트도 같은 쓰기 경로를 쓴다. `write_all` 한 번의 최종 성공만 세면 부분 성공 뒤 오류를 놓치므로 직접 부분 쓰기를 센다. 0바이트 쓰기는 진행으로 보지 않고 쓰기 오류로 처리한다.
4. 한쪽 TCP 읽기 EOF를 관찰하면 첫 EOF 시각과 쪽을 저장하고 즉시 절대 `eof_at + E`를 만든다. 이미 읽은 데이터를 모두 쓴 뒤 상대의 쓰기 방향만 닫는다. 해당 drain·shutdown도 같은 E 안에서 끝내야 한다.
5. 반대 방향은 계속 읽고 전달한다. 반대 EOF와 모든 미전달 데이터·쓰기 종료가 E 전에 끝나면 `peer_eof`로 정리한다. E가 지나면 남은 데이터를 보장하지 않고 `close_timeout`으로 정리한다. 읽기·쓰기·shutdown 직후에도 E를 확인한다. EOF마다 E를 새로 시작하지 않는다.
6. I/O 오류는 그 세션만 정리한다. 다른 대기와 활성 항목을 순회해 닫지 않는다. 원인을 일으킨 쪽은 읽기이면 입력 끝점, 쓰기이면 출력 끝점이다. 정상 EOF와 달리 미전달 바이트의 완전한 도착은 보장하지 않는다.
7. 활성 WS는 페이로드 진행과 별도로 `viewer.poll_flush`를 poll한다. `Pending`은 페이로드 진행을 막지 않는다. 오류는 Viewer 출력 쪽 `connection_error`로 처리한다. closing에서는 `poll_close`가 종료 진행을 맡는다.

TCP half-close는 읽기 쪽 전체 소켓 drop이 아니다. 예를 들어 Viewer가 요청 뒤 쓰기를 닫아도 Server가 보낸 응답은 Viewer로 계속 전달한다(RLY-019). 데이터가 없다는 이유의 세션 유휴 타이머는 없다. K를 넘는 정상 유휴도 유지한다.

각 카운터는 상대 전송 계층에 쓰기에 성공한 스트림 바이트만 센다. 초기 요청 250바이트, Viewer 시작 바이트 12바이트, TLS·WS 프레이밍과 제어 데이터는 제외한다. 후속 데이터와 RFB 자체의 초기 교환·인증·화면 데이터는 포함한다. 새 세션은 0부터 시작한다. `u64`의 포화 덧셈으로 감소·순환을 막는 안을 쓴다(**이 문서의 판단, 사용자 결정 없음**). 조회 스냅샷에는 같은 잠금 아래 확정한 두 카운터를 담는다. EOF·오류 시 마지막 부분 쓰기도 반영한다. 실제 애플리케이션 수신 완료로 설명하지 않는다.

구현한 `Session`은 `new`, `record_write`, `observe_eof`, `complete_direction`, `io_error`, `dead_connection`, `begin_admin_close`, `force_server_shutdown`, `expire_close`, `finish`를 공개한다. 시각은 호출자가 값으로 준다. `observe_eof`는 방향별 입력 종료만 표시한다. app이 버퍼 소진과 상대 쓰기 종료까지 마친 뒤 `complete_direction`을 부른다. `finish`는 실제 소켓·permit 정리 뒤 한 번만 호출할 수 있고 `ClosedSession`을 반환한다. `ClosedSession`은 외부 생성자와 `Clone`이 없으므로 같은 정리 결과를 중복 삽입하지 않는다. `SessionId`는 고정 30바이트 저장 공간을 쓰며 인코딩 때 할당하지 않는다. `SessionId::parse`는 Producer 형식용이다. 관리 URL의 더 넓은 Consumer 문법을 이 함수로 거절하지 않는다.

`RecentClosedList::new(R)`, `push(ClosedSession)`, `iter`, `len`, `is_empty`, `max_entries`는 정리된 세션만 다룬다. `push`는 최대 R개를 유지하고 `ended_at` 내림차순으로 보관한다. 뒤늦게 전달된 오래된 종료 시각도 순서와 상한을 바꾸지 않는다. R=0이면 요약을 보관하지 않는다. 반복자는 항목을 빌리므로 조회마다 이력을 복제하지 않는다.

<a id="44-ws를-나중에-연결하는-경계"></a>

### 4.4 WS·WSS와 공통 중계의 경계

WS·WSS는 `WsRelayStream`을 통해 같은 초기 교환→`dispatch`에 연결되어 있다. 자체 프레임 코덱은 사용자 결정 2026-11-01(Q77)을 따른다. M>B 스트리밍과 같은 방향 B 상한은 사용자 결정 2026-11-01(Q78)을 따른다. 클라이언트 Ping에 대한 Pong, 요청하지 않은 Pong 무시, 제어 페이로드 제외, Kw 주기의 서버 Ping과 Pong 기한을 구현했다.

- `Session::first_eof()`의 첫 쪽으로 Close 순서를 고정한다. Viewer가 먼저 Close를 보내면 `PeerEof`를 선택한다. 유효한 수신 코드만 응답한다. 코드가 없으면 1000이다. Close 앞의 데이터를 TCP로 소진한 뒤 TCP 쓰기 방향을 닫는다.
- Viewer가 먼저 Close를 보냈다면 이미 시작한 출력 프레임의 잔여만 같은 E 안에서 마친다. 새 메시지는 시작하지 않는다. 이후 TCP 데이터는 버리고 전송량에 세지 않는다. TCP EOF도 같은 E까지 기다린다. 사용자 결정 2026-11-01(Q83)을 따른다.
- TCP가 먼저 EOF를 내면 받은 데이터를 WS로 소진한 뒤 `WriteHalf`로 Close 1000을 보낸다. `WriteHalf`가 `Pending`인 동안 늦은 Viewer Close를 받아도 `PeerEof` 응답으로 바꾸지 않는다. 상대 Close 또는 같은 E까지 WS 데이터를 TCP로 전달할 수 있다. Close를 보낸 뒤 새 WS 데이터 메시지는 시작하지 않는다.
- TCP 오류 4007, 반대 TCP 끊긴 연결 4006, 관리자 종료 4005, 서버 종료 1001의 action을 구분한다. Mode I 대상 L 부족은 `TargetConnectionLimit`로 4011을 보낸다(사용자 결정 2026-11-01(Q79)).
- `Force` 또는 G 만료이면 먼저 `Session::force_server_shutdown()`을 적용한다. 이후 보존된 원인으로 action을 선택한다. typed 오류는 `StreamError(kind)`를 유지한다. Viewer 첫 EOF는 `PeerEof`를 유지한다. TCP 첫 EOF는 `WriteHalf`를 유지한다. 기존 정리 원인이 없을 때만 `ServerShutdown`(1001)을 선택한다. 사용자 결정 2026-11-01(Q82)을 따른다.
- 강제 경계에서는 Close 교환이나 막힌 shutdown을 기다리지 않는다. action을 한 번 poll한 뒤 소유자에게 반환한다. 이미 보낸 Close는 다시 보내지 않는다. 반환하는 `error_kind`는 최종 원인이 `ConnectionError`일 때만 유지한다.
- `run_session_controlled`는 Force에서도 최종 원인이 `ConnectionError`이면 기존 `error_kind`를 반환한다. `RelayService::run_pair`는 두 스트림과 L permit을 반환한 뒤 `SessionClosed`에 이 값을 넣는다. 다른 최종 원인에는 오류 종류를 붙이지 않는다. 최초 원인은 사용자 결정 2026-11-01(Q82)을 따른다. 필드는 [운영 정보 계약 1.6·1.7절](../contracts/ops-info-v1.md#16-원인-코드)을 따른다.
- terminal disposal은 유효한 프레임과 거절한 프레임을 구별한다. 유효한 프레임은 decode하며 페이로드를 폐기한다. 상대 Close 또는 transport EOF 뒤 종료한다. 1002·1003·1009로 거절한 프레임은 원시 입력을 폐기한다. 그 안의 Close 모양 바이트를 응답으로 해석하지 않는다. 두 경로 모두 app의 절대 E·G·Force로 제한한다. 구체적인 내부 처리는 **이 문서의 판단, 사용자 결정 없음**이다. [WS 스트림 설계 5절](websocket-stream.md#5-기존-relaystream과의-대응)을 따른다.

## 5. 상태와 초기화

### 5.1 프로세스 시작과 키

기존 [운영 로그 설계](ops-log.md#5-상태와-초기화)의 ref 계산을 재사용한다. 시작 4단계에서 비동기 런타임과 종료 신호 처리기를 준비한 직후, TLS 파일 검사 전에 `RelayIdKey::generate()`와 별도 세션 식별자 키 생성을 한 번씩 실행한다. 키 두 개는 서로 독립된 OS 난수 32바이트다. [CLI 1.8절](../contracts/cli-v1.md#18-수동-시작과-종료)의 시작 순서를 유지하며, 리스너는 아직 없다. 키를 파일에 저장하거나 `Debug`·로그에 내보내지 않는다. `RelayIdSource::reference(&RelayId) -> RelayIdRef` 포트만 ref 계산에 주입한다. 정확한 ID 바이트를 HMAC 입력으로 쓰고 ref를 매칭 키로 쓰지 않는다.

어느 키든 생성하지 못하면 준비됨으로 가지 않는다. 리스너를 바인드하기 전에 멈추고 확보한 자원을 회수한 뒤 `runtime_unavailable`(종료 코드 1)로 비정상 종료한다(사용자 결정 2026-11-01(Q76)). 고정 키·키 없는 해시로 대체하지 않는다.

`RelayService::new(modes, H, B, E, clock, events)`는 초기 교환 설정과 공통 시계·사건 포트를 받는다. 별도 `RelayRuntime::new`는 대상 연결 포트, 두 식별자 포트, 같은 L 예산과 별도 Lw 예산, 대상 정책·W·C·E·R 설정을 받는다. `dispatch`가 runtime을 명시적으로 받는다. 런타임 난수 포트와 발급 ID 집합은 없다. Lm은 관리·상태 예산에 남는다. Lw는 등록된 대기 항목만 점유한다. 두 모드의 활성 세션은 모두 L 두 자리와 Lw 0자리다.

### 5.2 정상 종료와 G

첫 종료 요청은 리스너 수락 중지와 app 수명 전이를 먼저 확정한다. 같은 상태 잠금 아래 `ready → shutdown_grace`로 바꾸고 이후 등록·매칭·세션 활성화를 막는다. 대기·초기 교환·대상 연결 중인 작업은 즉시 취소·정리한다. 종료 직전에 연결 또는 매칭이 준비돼도 상태 잠금의 활성화 시점이 종료 뒤이면 세션을 만들지 않는다.

이미 활성인 세션은 `shutdown_started`부터 한 절대 G까지 정상 중계를 계속한다. 자발적으로 끝나면 실제 원인으로 정리한다. G=0, G 만료 또는 두 번째 종료 요청이면 신규 읽기를 멈춘다. 아직 정리를 시작하지 않은 세션만 `server_shutdown`으로 정리한다. EOF·오류·단절·관리자 종료를 먼저 관찰한 세션은 기존 원인을 유지한다(사용자 결정 2026-11-01(Q82)). `shutdown_forced`는 남은 세션이 있을 때만 그 `session_closed`보다 먼저 기록한다.

app 수명은 `Running`, `Grace { deadline }`, `Force`다. `src/serve.rs`가 OS 종료 요청을 받아 `request_shutdown`을 호출한다. 별도 `supervisor`가 `drive_shutdown`으로 G를 진행한다. `finish_relay_shutdown`은 모든 진입점 소유자가 끝날 때까지 이 감독 작업을 유지한다. 두 번째 요청은 `Force`를 요청한다. 모든 진입점을 join한 뒤에만 감독 작업을 abort하고 join한다. 소유 작업이 먼저 끝나면 G를 기다리지 않는다. 이 조립 방식은 **이 문서의 판단, 사용자 결정 없음**이다. [시작 설계 4절](serve-startup.md#4-처리-순서)을 따른다.

유예 중 관리 조회·로그인·관리자 종료와 상태 조회는 계속 실행한다. 모든 중계 정리가 끝난 뒤 관리·상태·감지 작업을 거두고 `server_stopped`를 마지막에 쓴다. 강제 정리의 최근 종료 항목도 Q82의 최초 원인을 유지한다. 프로세스 종료 전에 실제 HTTP 조회 기회가 생긴다고 보장하지 않는다.

Windows의 `ShutdownSignal::wait`는 Ctrl+C·Ctrl+Break를 `ShutdownRequest::Requested`로, 콘솔 창 닫기를 `ConsoleClosed`로 반환한다. Unix는 SIGINT·SIGTERM을 `Requested`로 반환한다. `begin_relay_shutdown`은 모든 요청에 설정한 G로 첫 `request_shutdown`을 호출한다. `ConsoleClosed`이면 같은 호출 안에서 `Duration::ZERO`로 한 번 더 요청한다. 그러므로 Grace의 `shutdown_started`를 기록한 뒤 G 없이 Force로 진행한다. 남은 활성 세션이 있을 때만 `shutdown_forced`를 기록한다. 이 신호 분류와 조립 함수 분리는 **이 문서의 판단, 사용자 결정 없음**이다. 외부 동작의 근거는 사용자 결정 2026-11-04(Q102)와 [운영 스펙 3.4절](../specs/operations.md#34-정상-종료와-재시작), OPS-044다. 정리와 `server_stopped` 기록은 OS의 기본 5초 제한 안에 마쳐야 한다. 로그오프·시스템 종료는 대화형 콘솔 프로그램이 신호를 받지 못하므로 처리하지 않는다.


## 6. 사건·오류·정리

공통 `ts`·`level`·`event`와 선택 필드 생략 규칙은 [운영 정보 계약](../contracts/ops-info-v1.md#12-공통-필드)이 기준이다. 아래는 만드는 위치와 필드 대응이다. 새로운 wire 사건 이름은 만들지 않는다.

Mode II가 초기 교환을 마쳤더라도 대기 등록 전에 서버 종료로 중단되면 `connection_aborted(stage=initial_exchange, reason=server_shutdown)`을 쓴다. 매칭 후 활성화 전에 종료되면 이미 등록된 쪽은 `waiting_removed(reason=server_shutdown)`로 기록한다. 도착하여 등록 중이던 쪽은 같은 초기 교환 중단 사건으로 기록한다. 이는 Q81의 등록 전 중단 구분을 실제 `Stage::InitialExchange` 값으로 표현한 것이다. Mode I의 대응 단계는 `target_connect`다. `stage=register`는 아래 중복·Lw 거절 사건의 값이다.

| 발생 조건 | 사건·level | 계약 필드 |
| --- | --- | --- |
| Mode I 정책 거절 | `connection_rejected`, warn | `conn_id`, `listener`, `role=viewer`, `mode=mode1`, `peer_ip`, `stage=target_connect`, `reason=target_not_allowed`, `target_host`, `target_port`, 검사한 경우 `target_ip` |
| 대상 L 부족 | `connection_rejected`, warn | 위 연결 문맥, `stage=target_connect`, `reason=relay_connection_limit`, `limit` |
| 해석·연결 실패 | `target_connect_failed`, warn | `conn_id`, `listener`, `viewer_transport`, `peer_ip`, `target_host`, `target_port`, C 시작부터 `elapsed_ms`, 시도 IP가 있으면 `target_ip`, `reason=resolve_failed/connect_refused/connect_timeout/connect_error` |
| 대상 연결 중 서버 종료 | `connection_aborted`, info | `conn_id`, `listener`, `stage=target_connect`, `reason=server_shutdown`, `peer_ip`, 수락부터 `elapsed_ms`, `role`, `mode` |
| 대기 등록 | `waiting_registered`, info | `conn_id`, `listener`, `role`, `mode=mode2`, `relay_id_ref`, `peer_ip` |
| 후발 중복·Lw 부족 | `connection_rejected`, warn | 후발 `conn_id`, `listener`, `role`, `mode=mode2`, `peer_ip`, `stage=register`, `reason=duplicate_id`와 `relay_id_ref`, 또는 `reason=waiting_full`과 `limit=Lw` |
| W 만료 | `waiting_expired`, info | `conn_id`, `role`, `relay_id_ref`, `waited_ms`. 별도의 `wait_timeout` 중단 사건을 중복 기록하지 않음 |
| EOF·오류·단절·서버 종료로 대기 제거 | `waiting_removed`, info. 오류·단절은 warn | `conn_id`, `role`, `relay_id_ref`, `waited_ms`, `reason=peer_eof/connection_error/dead_connection/server_shutdown`, 연결 오류면 `error_kind` |
| 활성화 성공 | `session_started`, info | `session_id`, `mode`, `viewer_transport`, `viewer_conn_id`, `viewer_peer_ip`. Mode II: `server_conn_id`, `server_peer_ip`, `relay_id_ref`, 선행 `waited_ms`. Mode I: `target_ip`, `target_port`, 이름 요청이면 `target_host` |
| 활성 세션 최종 정리 | `session_closed`, 원인별 | `session_id`, `mode`, `viewer_transport`, UTC `started_at`, `duration_ms`, `bytes_viewer_to_target`, `bytes_target_to_viewer`, `close_cause`. Mode II `relay_id_ref`. 필요한 `side`, `error_kind`, `admin_account` |

대상 IP 요청의 `target_host`는 요청에서 얻은 단일 IP의 정규 표기다. DNS 요청이면 정규화한 이름이다. 원시 요청을 잘라 쓰지 않는다. formatter는 기존 `cut`으로 `target_host` 256바이트 제한을 적용한다. 세션 생성 전에는 `session_id`를 사건에 넣지 않는다. 페이로드·초기 요청·ID 원문·키를 사건에 받지 않는다.

| 종료 원인 | level·추가 필드 | 최종 정리 의미 |
| --- | --- | --- |
| `peer_eof` | info, 먼저 EOF를 낸 `side` | 일반 경로는 양방향 소진·쓰기 종료 완료. Force 경계에서는 먼저 관찰한 EOF를 보존하며 drain 완료를 보장하지 않음(Q82) |
| `connection_error` | warn, 오류 쪽 `side`, `error_kind=io_error` 등 | 해당 연결 쌍만 종료 |
| `dead_connection` | warn, 응답 없는 `side` | Linux 소켓 `ETIMEDOUT`과 앞선 ICMP 도달 불능에 따른 포기 오류 `EHOSTUNREACH`·`ENETUNREACH`는 keepalive 또는 재전송 포기로 판정한 응답 없는 상대이므로 `dead_connection`으로 바꾼다(ICMP 사례는 커널 소스 분석 근거, 실행 관찰 아님, 이 문서의 구현 판단, 사용자 결정 아님). `TcpRelayStream`과 `WsRelayStream`의 하부 I/O가 `net::classify_socket_error`를 쓰며 H·C·W의 app 타이머는 제외한다. Windows의 단절 오류는 관찰하지 않아 기존 부분 범위를 유지한다(Q86). 유휴 자체를 이 원인으로 만들지 않는다. |
| `admin_terminated` | info, `admin_account`, side 없음 | [관리자 세션 종료](management-terminate.md)가 지정 세션만 종료. 완료 알림 뒤 API 응답 |
| `close_timeout` | warn, EOF가 있으면 먼저 EOF를 낸 `side` | EOF 뒤 E 초과. 관리자 종료 E 초과도 같은 원인 |
| `server_shutdown` | warn, side 없음 | G 만료·두 번째 종료 요청으로 강제 정리하거나 정상 종료 중(Grace·Force)에 소유 작업을 취소한 경우. 기존 원인이 없는 세션에만 쓴다. Running 중 버그 종료는 제외한다(사용자 결정 2026-11-01(Q82), 2026-11-02(Q94)) |
| `internal_error` | warn(운영 정보 계약 1.3절), side·추가 필드 없음 | Running 중 세션 소유 작업이 panic 등 버그로 끝났고 기존 원인이 없는 경우(사용자 결정 2026-11-02(Q94)). panic 되감기가 스트림을 버리므로 WebSocket Close를 보내지 못한다(구현 한계, 이 문서의 판단, 사용자 결정 없음) |

종료 원인은 [관리 API 1.10절](../contracts/management-api-v1.md#110-최근-종료-목록-get-apiv1closed-sessions)과 [운영 정보 계약 1.6절](../contracts/ops-info-v1.md#16-원인-코드)을 따른다. 한 세션의 `close_cause`는 정리를 시작하게 한 첫 원인이다. 일반 경로의 `peer_eof`는 양방향 소진·쓰기 종료 뒤 확정한다. 한쪽 EOF는 먼저 E와 `closing`을 시작한다. 이후 반대 방향 I/O가 실패하면 `connection_error`를 쓴다. E가 먼저 만료하면 `close_timeout`이다. 다만 G 만료나 두 번째 요청으로 즉시 정리할 때는 `force_server_shutdown()`이 먼저 관찰한 EOF를 `peer_eof`로 보존한다. 이는 양방향 drain 완료를 보장한다는 뜻이 아니다. 이미 정한 오류·단절·관리자 종료도 보존한다(사용자 결정 2026-11-01(Q82)). 원인이 전혀 없을 때만 `server_shutdown`을 선택한다. 양방향 전송이 끝난 뒤 소켓 정리 오류는 원인을 바꾸지 않는다. 관리자 종료의 E 만료는 관리 API 1.9절의 `close_timeout` 규칙을 따른다.

세션 소유 작업의 비정상 종료도 서버 수명 상태로 구분한다. 기존 원인이 없으면 Running에서는 `internal_error`, 정상 종료 중(Grace·Force)의 취소에서는 `server_shutdown`을 쓴다. `ActiveGuard`는 기존 원인을 바꾸지 않는다(사용자 결정 2026-11-02(Q94)).

### 정리 순서와 정확히 한 번 반환

1. 해당 연결·세션에 종료 소유자를 하나만 정한다. 활성 세션은 `closing`으로 남기고 Mode II ID 점유도 유지한다. 새 읽기·매칭은 중지한다. 정상 TCP EOF의 허용된 반대 방향 전달은 4.3절을 따른다.
2. 대기 등록을 제거할 때는 연결 식별자까지 비교한다. 같은 잠금 아래 Lw permit을 반환하고 실제 스트림을 정리하는 소유자에게 옮긴다. 중복 제거 요청은 아무 permit도 갖지 못한다.
3. 스트림의 종료 action을 실행하고 소켓·방향 버퍼를 버린다. 모든 split·래퍼 참조가 사라져 실제 소켓이 닫힌 뒤에만 L permit을 drop한다. 완료되지 않는 종료 교환은 E 또는 강제 종료 경계에서 끊는다.
4. 대기 제거·매칭 전이에서 이미 Lw를 반환했으므로 비동기 소켓 정리에서는 다시 반환하지 않는다. `BudgetPermit`은 복제하지 않는다. 수동 수치 차감도 하지 않는다.
5. 활성 정리 완료는 잠금 아래 활성 항목 제거·같은 `session_id`의 ID 점유 해제·최근 종료 추가를 함께 한다. 최종 카운터와 시각을 확정한다. `session_closed`를 한 번 기록하고 완료 알림을 보낸다. 관리자 API는 이 알림 뒤에만 응답할 수 있다.
6. 대기 만료·중단은 자원을 돌려준 뒤 각각 `waiting_expired`·`waiting_removed`를 한 번 기록한다. Mode I 연결 실패도 대상 소켓·추가 L·Viewer 스트림·기존 L·대상 연결 중 등록을 정리한 뒤 `target_connect_failed`를 쓴다. 정책·등록 거절도 Close·stream·permit 정리를 마친 뒤 거절 사건을 기록한다.

같은 연결·세션의 사건 순서를 유지하기 위해 등록·활성화·최종 제거의 사건은 해당 상태 전이를 직렬화하는 구간에서 기록한다. EventSink의 느린 출력이 전이 잠금을 잡을 수 있지만 소켓 I/O를 그 안에서 기다리지는 않는다. 다른 연결의 같은-ID 사건에 전역 순서를 추가로 보장하지 않는다. 로그 쓰기 실패는 기존 sink 정책대로 줄만 버리고 정리와 permit 반환을 계속한다. 이 동시 실행 선택은 **이 문서의 판단, 사용자 결정 없음**이다.

<a id="7-설계-선택과-테스트-계획"></a>

## 7. 설계 선택과 현재 시험

| 선택 | 이유 | 다른 방법 | 근거 |
| --- | --- | --- | --- |
| 공통 서비스·공유 잠금·domain 전이 | 양 역할·만료·종료가 같은 상태를 동시에 바꾸는 지점을 한 곳으로 모음 | ID별 독립 작업과 다중 잠금. 등록·활성 집계 일관성 확인이 더 어려움 | [ADR-0003](../decisions/ADR-0003-single-instance-memory-state.md), RLY-013·014 |
| 단계 간 연결·버퍼·permit 이동 | 중복 L 반환과 후속 바이트 재전송을 타입 소유권으로 막음 | 단계마다 재확보·복사 | [초기 교환 설계 5·6절](initial-exchange.md#5-후속-바이트와-b), OPS-023 |
| 방향마다 쓰기를 소진한 뒤 다음 읽기 | B와 바이트 순서를 간단히 확인할 수 있음 | 무제한 채널·기본 버퍼 크기의 복사 함수 | RLY-003·017·018, MGT-005 |
| 포트가 IP 하나로만 연결 | DNS 정책 검사 뒤 재해석 우회를 막음 | hostname을 TCP connect에 전달 | [설정 계약 1.5절](../contracts/config-v3.md#15-mode-i-대상-허용-항목-문법) |

이 표의 내부 선택은 모두 **이 문서의 판단, 사용자 결정 없음**이다. 외부 시나리오 자체는 연결한 스펙·계약을 따른다.

### 현재 시험

아래는 현재 소스의 시험 위치와 표식 범위다. 실행 결과가 아니다. 단위 시험은 제어 가능한 Clock·부분 읽기·부분 쓰기 스트림과 tokio 정지 시각을 쓴다. [테스트 컨벤션](../conventions/testing.md#2-규칙)을 따른다.

| 범위·현재 파일 | 표식 범위 | 직접 비교하는 결과 |
| --- | --- | --- |
| `tests/relay_target_scenarios.rs` | RLY-007·008·009, OPS-023. RLY-009·OPS-023의 전체 시험은 Linux 전용이고 이식 가능한 부분 시험도 있음 | 금지 IP·포트·DNS 결과의 연결 없음, 해석 실패·거절·C, 연결 중·연결된 대상의 같은 L 점유와 관리 예산 분리 |
| `tests/relay_matching_scenarios.rs` | RLY-012~015 전체, RLY-016 부분, OPS-005 전체 | 정확 ID·역할, 동시 도착·단일 매칭, W·EOF 경합, TCP·WS·WSS 중복, Lw 경계 |
| `crates/adapters/tests/relay_delayed_cleanup_scenarios.rs` | RLY-016 전체 | 옛 소유자의 지연 정리가 재사용한 ID의 새 세션을 지우지 않음 |
| `tests/relay_entry_scenarios.rs` | RLY-001~005·022·023, OPS-039 전체 | 실제 TCP 인계·후속 데이터·거절·두 번째 프로세스 실패 때 기존 대기와 활성 유지 |
| `crates/adapters/tests/relay_stream_scenarios.rs` | RLY-017·018·019·021 전체 | 동시 양방향 전달, 실제 방향별 B 상한과 수신 재개, half-close, RST 때 다른 세션 격리 |
| `tests/management_query.rs`, `tests/management_terminate.rs` | 관리 조회·종료 시나리오와 중계 카운터·선택 종료의 부분 범위 | 실제 HTTP 인증·조회·종료가 공유 runtime에 연결됨. 표식의 전체·부분 판정은 각 시험 소스를 기준으로 함 |
| `crates/app/src/relay.rs`, `crates/app/src/relay/session.rs` | relay·operations·management 부분 표식과 표식 없는 내부 시험 | 단일 전이·취소 guard·부분 쓰기·E·G·Force·원인 보존·정확한 자원 반환 |
| `crates/adapters/src/crypto.rs`, `crates/adapters/tests/tcp_target.rs`, `crates/adapters/tests/ops_info_contract.rs` | 내부 규칙과 부분 표식 | 식별자 순열·키 비노출, 실제 TCP 연결·K·half-close, 사건 필드·level·원문 비노출 |
| `tests/relay_dead_peer_linux.rs` `removes_unresponsive_waiting_server_and_reuses_id_and_l`·`removes_unresponsive_active_peers_without_harming_other_sessions`(작업 2.11, Current) | 각각 `/// spec: relay.md:RLY-025`, `/// spec: relay.md:RLY-026` | Linux의 권한 없는 `unshare --user --map-root-user --net` 안에서 `lo`를 켠다. `nft` input/output DROP으로 응답 없는 대기 Server, 활성 Mode II Viewer·Server와 Mode I Viewer·대상을 각각 만든다. K=11초와 Linux 타이머·스케줄링 오차 최대 5초(상한 16초) 안에 정리 사건이 나오는지 확인한다. 시간은 DROP 직전과 사건 줄을 읽은 순간의 단조 시계(`Instant`) 차이로 잰다. 로그 `ts`는 벽시계라서 WSL2의 벽시계 보정(40초 동안 +2.933초 관찰)에 흔들리므로 시간 판정에 쓰지 않는다(이 문서의 판단, 사용자 결정 없음). 반대쪽 EOF·L 반환·동일 대기 ID 재등록·다른 세션 양방향 전송도 확인한다. 대기 시험은 설정 최솟값 L=2에서 정상 대기 Server 하나로 자리를 채운다. 그래서 교체 Server는 끊긴 연결이 돌려준 자리에만 들어갈 수 있다. 실패시킬 연결에는 초기 교환 뒤 데이터를 보내지 않아 미확인 데이터의 재전송과 keepalive를 섞지 않는다. 하한은 그 연결의 마지막 초기 교환·대상 연결 시각으로 비교한다. 규칙의 패킷 카운터로 실제 DROP을 확인한다. 필수 도구·사용자 네임스페이스가 없으면 실패한다. 커밋 `bf27d30`의 WSL 게이트에서 통과했다. Windows는 기존 부분 시험을 유지한다(사용자 결정 2026-11-02(Q86·Q93), [준비 절차](../how-to/set-up-and-verify.md#wsl-끊긴-연결-감지-시험-준비)). |
| `tests/relay_log_scenarios.rs` `idle_active_sessions_survive_longer_than_k_and_transfer_data` | `/// spec: relay.md:RLY-027` | Mode I·II에서 살아 있는 유휴 연결을 K보다 오래 유지하고 양방향 데이터를 보낸다. 두 OS에서 통과했다(`1a6f93e`). |

추가 내부 시험은 시나리오가 없으면 표식을 붙이지 않는다. 시작 키 생성 실패·키 비노출·프로세스별 ref, 세션 ID 정규 인코딩·순열 역변환·카운터 비순환, 취소 중 permit 반환 순서, R=0·R 초과 제거, 벽시계 역행과 단조 경과, 종료 원인 우선순위를 확인한다. `ops_info_contract.rs`는 사건의 필수·선택 필드와 level을 비교한다. 사건 종단 간 수집은 `tests/relay_log_scenarios.rs`가 맡는다. 이 구조 문서 자체는 시나리오 실행 결과를 대신하지 않는다.

### WS 스트림 시험과의 연결

다음은 현재 소스의 시험과 표식이다. 이 문서 개정에서 실행하지 않았다. [WS 스트림 설계 7절](websocket-stream.md#7-테스트-계획)과 같은 구현 경계를 가리킨다.

| 기존 시험 위치·함수 | 실제 표식 | 이 문서와 만나는 경계 |
| --- | --- | --- |
| `crates/adapters/src/websocket_stream/tests.rs`의 `preserves_initial_exchange_trailing_bytes_once_within_b`, `streams_a_single_frame_larger_than_b_without_a_message_buffer` | `websocket.md:WS-018 partial`·`relay.md:RLY-003 partial`, `websocket.md:WS-011 partial`·`relay.md:RLY-018 partial` | 후속 데이터와 같은 B, M>B 프레임 스트리밍 |
| 같은 파일의 `rejects_text_and_invalid_frames_with_typed_errors`, `finishes_started_frames_before_replying_to_close_within_e` | `websocket.md:WS-008 partial`·`websocket.md:WS-019 partial`, `websocket.md:WS-017 partial` | typed 오류와 시작한 출력 프레임의 잔여 |
| 같은 파일의 `bounds_outbound_messages_and_counts_only_written_payload` | `websocket.md:WS-007 partial` | 실제 부분 페이로드 쓰기만 계산 |
| `crates/adapters/tests/websocket_relay.rs`의 `ws_and_wss_forward_initial_trailing_data_once` | `websocket.md:WS-018`, `websocket.md:WS-003`~`websocket.md:WS-006`, `relay.md:RLY-003 partial` | WS·WSS 후속 데이터→실제 TCP 전달 |
| 같은 파일의 `client_close_drains_to_tcp_and_echoes_only_the_code`, `tcp_eof_drains_then_forwards_client_data_after_close_1000` | 각각 `websocket.md:WS-016 partial`. 후자는 `relay.md:RLY-019 partial`도 있음 | 두 Close 순서·코드·drain·카운터 |
| 같은 파일의 `mode_i_target_limit_after_101_closes_4011_and_preserves_existing_session` | `operations.md:OPS-023 partial` | 101 뒤 대상 L 부족과 기존 세션 보존(Q79) |
| 같은 파일의 `single_and_fragmented_messages_accept_m_and_close_1009_at_m_plus_one` | `websocket.md:WS-010` | WS·WSS의 분할·비분할 M 수락. M+1 헤더와 본문을 함께 보내고 1009·추가 미전달·typed 사건·자원 반환 확인 |

### 기존 수명 회귀 시험

아래 함수는 현재 소스에 있다. 표식 없음은 시나리오 표식이 없는 내부 회귀 시험이다. 실행 결과가 아니다.

| 위치·함수 | 실제 표식 | 비교하는 경계 |
| --- | --- | --- |
| `crates/app/src/relay/session.rs`의 `drives_queued_and_partial_pongs_while_payload_writes_are_backpressured` | 없음 | B가 찬 상태의 큐·부분 Pong을 출력 wake만으로 진행. 추가 입력 읽기 없음 |
| 같은 파일의 `classifies_control_flush_errors_as_viewer_output_errors` | 없음 | flush 실패의 typed 종류·Viewer 쪽·0 페이로드 카운터 |
| 같은 파일의 `g_expiry_preserves_eof_and_never_waits_for_transport_close` | `operations.md:OPS-009 partial` | G=0·G 만료에서 첫 EOF 원인·쪽 보존 |
| 같은 파일의 `force_preserves_an_error_whose_terminal_close_is_still_pending` | `operations.md:OPS-009 partial`, `operations.md:OPS-042 partial` | G·두 번째 요청이 막힌 종료를 끝내도 오류 종류·원인 보존 |
| 같은 파일의 `force_before_first_close_poll_preserves_the_selected_websocket_action` | `websocket.md:WS-016 partial`, `operations.md:OPS-042 partial` | 첫 Close poll 전 Force의 `StreamError`·`PeerEof`·`WriteHalf`·`ServerShutdown` 선택 |
| 같은 파일의 `force_published_by_expired_cleanup_keeps_the_typed_error_action` | `websocket.md:WS-008 partial`, `operations.md:OPS-042 partial` | E 경계 정리 중 새 Force가 생겨도 typed action 보존 |
| 같은 파일의 `force_after_either_close_order_never_sends_a_second_websocket_close` | `websocket.md:WS-017 partial`, `operations.md:OPS-009 partial` | 두 Close 순서 뒤 Force의 중복 Close 방지 |
| 같은 파일의 `late_client_close_cannot_replace_tcp_first_pending_write_half_with_an_echo` | `websocket.md:WS-016 partial` | Mode I·II의 TCP 첫 EOF를 늦은 Viewer Close가 바꾸지 않음 |
| `crates/app/src/relay.rs`의 `forced_dispatch_records_retained_error_kind_only_after_streams_and_permits_are_released` | `operations.md:OPS-009 partial`, `operations.md:OPS-042 partial` | Force·G 뒤 `connection_error`의 종류·쪽 보존. 스트림과 permit 반환 뒤 사건 기록 |
| `crates/adapters/src/websocket_stream/tests.rs`의 `terminal_close_completes_on_peer_close_without_transport_eof` | 없음 | 유효한 데이터 폐기 뒤 상대 Close로 종료 완료 |
| 같은 파일의 `valid_terminal_disposal_yields_during_a_data_flood` | 없음 | 유효한 데이터 연속 수신 중 제한된 poll 뒤 yield |
| 같은 파일의 `rejected_frame_disposal_waits_for_transport_eof_not_apparent_close_bytes` | 없음 | 거절한 페이로드 안의 Close 모양 바이트 무시 |

### 현재 Kw 시험

`crates/adapters/src/websocket_stream/keepalive.rs`의 세 내부 시험과 `websocket_stream/tests.rs`의 `kw_` 시험 네 개가 고정 주기·FIFO 상한·기한·B 뒤 Pong·부분 출력 순서를 확인한다. `crates/adapters/tests/websocket_scenarios.rs`의 `ws_and_wss_kw_keeps_idle_peers_and_drops_unresponsive_peers`는 WS-026 전체 표식이다. 실제 WS·WSS의 응답 없는 활성·대기 Viewer 제거, 살아 있는 유휴 연결 유지와 자원 반환을 확인한다. [WS 스트림 설계 9.5절](websocket-stream.md#95-kw의-여러-ping과-b-backpressure)은 구현한 Q84 정책이다.

## 8. 현재 코드와의 차이와 연결 순서

### 2026-11-04 대조 기록

- 기준: HEAD `e9221c6`과 이 문서 개정 작업 트리다.
- 방법: domain 전이, `RelayService`·`RelayRuntime`·방향 엔진, TCP·WS·WSS 호출자, 관리 포트와 serve 종료 감독을 읽었다. [코드 지도](code-map.md#2-코드-위치와-구성요소)의 경로·심볼·호출 연결 검사와 사람의 책임 검토를 함께 쓴다.
- 결과: 초기 교환 서비스와 공유 runtime의 소유권을 분리해 적었다. `SessionIdKey`·`poll_maintenance`·관리 조회/종료·Kw·G 감독은 현재 코드다. 현재 시험 파일로 7절을 바꿨다.
- 남은 차이: Windows 콘솔 close·logoff는 위 Target 절의 TODO 5.3 범위다. 소유자 panic 때 스트림 drop으로 WS Close를 보내지 못하는 한계는 6절대로다. 이 대조에서 외부 VNC 화면·입력 시험은 실행하지 않았다.
- 구조 판정: `Current`·`Verified`. **이 문서의 판단, 사용자 결정 없음**. 아래 과거 실행 기록은 당시 기준과 결과를 유지한다.
- 이번 검증: WSL `cargo test --test code_map`은 `4 passed; 0 failed`다. 코드 지도의 참조·호출 연결 검사이며 이 문서의 책임 설명은 위 수동 대조로 확인했다. `python D:/vr-tools/tmp/doc_check.py`는 기존 오류 4건만 보고했다(`other=0`). 제품 동작 시험은 다시 실행하지 않았다.

### TCP 연결 경로와 WS·WSS 연결 경로

TCP·WS·WSS는 공통 초기 교환·dispatch에 연결되어 있다. 시작 키 실패 코드는 `runtime_unavailable`이다(사용자 결정 2026-11-01(Q76)). 7절의 현재 시험 범위와 외부 화면·입력 수용 결과는 구분한다. 이 문서의 구조 대조만으로 TODO 완료를 판정하지 않는다.

TCP 경로(HEAD): `listen_until_shutdown`이 시작 키를 만든다(`generate_startup_keys`, 실패 시 `runtime_unavailable`, 종료 코드 1). `TcpEntryPoint::run`의 `accept_connections` serve closure가 `ConnectionContext`와 `runtime.lifecycle()`로 `RelayService::handle_connection`을 await한다. `Ok(prepared)`이면 같은 연결 작업에서 `dispatch(prepared, &runtime).await`를 부른다. `Err`이면 초기 교환에서 끝낸 정리를 다시 하지 않는다.
`accept_connections`의 IP → L → K → conn_id → `connection_accepted` 순서는 유지한다. 수락 반복은 리스너 종료 뒤 연결 작업들을 끝까지 거둔다.
TCP `hold`·`closed_before_data`는 제거했다. WS의 101 뒤 `hold_upgraded` 보류도 `638cf09`에서 제거했다. WS 연결 경로의 세부는 [WebSocket 진입점 모듈 설계](websocket-entry-point.md)가 다룬다.

WS·WSS 진입점 연결은 `638cf09`에서 끝냈다. TCP·WS·WSS 경로의 코드 위치는 `src/serve.rs`, `crates/adapters/src/net.rs`, `crates/adapters/src/websocket.rs`, `crates/app/src/relay.rs`다.

### 구현 형태와 설계 차이

- `RelayService::new(modes, H, B, E, clock, events)`는 초기 교환에 필요한 값만 가진다. `RelayRuntime::new`에는 필수 연결 포트·두 식별자 포트·L·Lw·W/C/E/R/대상 정책을 준다. `dispatch`가 runtime을 명시적으로 받는다. 2.7을 독립적으로 시험하면서 비어 있는 dispatch 설정이나 가짜 handoff를 만들지 않기 위한 분리다. **이 문서의 판단, 사용자 결정 없음**이다. 시작 조립은 두 곳의 E에 같은 설정을 넣어야 한다.
- Lw는 등록된 대기 수만 센다. 매칭과 만료·오류·취소의 대기 표 제거 전이에서 바로 반환한다. 비동기 Close가 늦어져도 표에서 제거한 항목 때문에 새 등록을 막지 않기 위한 선택이다. L은 실제 stream drop 뒤 반환한다. 기존 6절의 Lw 정리 순서를 이 기준으로 구체화한다. **이 문서의 판단, 사용자 결정 없음**이다.
- 정책·등록 거절도 스트림과 permit 정리 뒤 사건을 기록한다. 사건을 받은 관찰자가 즉시 반환된 자원을 볼 수 있게 초기 교환과 같은 순서로 맞췄다. wire 필드·거절 사유는 바꾸지 않았다. **이 문서의 판단, 사용자 결정 없음**이다.
- 공통 fixture가 필요한 대상·등록·수명 단위 시험은 `relay.rs`의 `tests`에 모았다. 방향 엔진 시험은 `relay/session.rs`, 실제 대상 소켓 시험은 `crates/adapters/tests/tcp_target.rs`다. 7절의 전체 시나리오 시험을 이 단위 시험으로 대체하지 않는다. **이 문서의 판단, 사용자 결정 없음**이다.
- 외부에서 dispatch future 자체를 취소해도 guard가 정확한 점유와 permit을 회수한다. 이미 버린 future에서 비동기 Close를 완료했다거나 정상 종료 요약을 확정했다고 기록하지 않는다. 정상 서버 종료는 future 강제 취소 대신 공유 수명 입력으로 진행한다.
- 각 wake에서 G 만료·공유 Force 전이·`shutdown_forced`를 엔진 poll보다 먼저 처리한다. 엔진이 Ready를 반환한 뒤에는 그 세션의 정리가 완료된 것으로 표시하며 원인을 바꾸지 않는다. 뒤늦은 Force의 `remaining_sessions`에서 이 세션을 제외하지만 stream·permit drop 전까지 ID 점유는 유지한다.
- `RecentClosedList::push`가 보관하지 않은 요약 하나를 반환한다. 최근 종료 ref 목록은 그 식별자 하나만 제거한다. R=0과 너무 오래된 새 요약도 반환 대상이다. 전체 ref마다 최근 목록을 다시 검색하는 O(R²) 정리를 하지 않는다.

### 이번 구현에서 확인한 시험

| 시험 위치·대표 함수 | 표식 범위 | 확인한 경계 |
| --- | --- | --- |
| `relay/handshake.rs` `rejection_and_h_timeout_close_before_release_but_never_hold_l_beyond_e` | RLY-004·005, OPS-006 `partial` | 거절·H 만료의 Close action이 L 반환보다 먼저 실행됨. E 뒤에는 반환·로그가 진행됨 |
| `relay.rs`의 매칭·대상·수명 시험 | RLY-003·006~016·018·020·021·025, OPS-005·007~009·023, MGT-005 `partial` | 양 도착 순서·정확 ID·중복·W 경합·B가 찬 뒤 FIN·취소·순열 카운터 경계·C·최종 IP·공유 L·Grace→Force |
| `relay/session.rs`의 방향 엔진 시험 | RLY-003·017~021·023·026·027, OPS-008·009, MGT-005 `partial` | 후속 데이터·부분 쓰기·B·역방향 진행·half-close·절대 E·종료 원인 우선순위·격리·살아 있는 유휴 유지 |
| `ports.rs`·초기 교환·방향 엔진·`ops_info_contract.rs` typed 오류 시험 | WS-008·010·019 `partial` | 표준 I/O 오류 채널에서 원래 종류가 Close action과 사건까지 보존됨. WS 프레임 처리는 시험하지 않음 |
| `crypto.rs` 고정 벡터·역변환 시험 | 내부 암호 규칙, 시나리오 표식 없음 | 고정 키 결정성·서로 다른 카운터·6라운드 역변환·키 Debug 비노출 |
| `tcp_target.rs` 6개 Windows 시험 | RLY-009·019 `partial`, 나머지는 독립 어댑터 규칙 | 실제 루프백 주소 고정·해석 순서·거절·K 활성·TCP half-close·종료 action |
| `ops_info_contract.rs` 신규 7개 시험 | 해당 relay·operations·management·WS 표식 `partial` | 필수·선택 필드·원인별 level·ID 원문 비노출·UTF-8 안전 256바이트 제한 |

### 문서 검증 기록

첫 설계·domain 작업의 기록과 이번 app·adapters 작업을 구별한다. 아래 앞 세 문단은 이전 작업의 실행 결과다. 이번 작업의 결과는 뒤의 명령 표에 기록한다. [검증 기록](../AGENTS.md#검증-기록)을 따른다.

실행 기록: Python inline 검사(`functions.eval`, `pathlib`·정규식)로 상대 링크 존재, 제목에서 만든 앵커, fenced code를 제외한 표 열 수를 검사했다. 새 문서는 링크 43개·앵커 36개·표 11개 모두 통과했다. 저장소 범위 첫 검사는 기존 링크 오류 2개(`dashboard-operations-overview.md:73`, `verify-dashboard-frontend.md:873`), 앵커 오류 0개를 보고했다. 표의 기존 오류는 `tls.md:126`, `verify-dashboard-frontend.md:179`다. inline code 안의 파이프를 열로 센 기존 두 행은 파서 오탐으로 분류했다. 사전 고지된 `DESIGN.md:698`은 이번 fenced-code 제외 검사 대상에서 빠졌다. 기존 오류는 수정하지 않았다. Cargo와 전체 게이트는 실행하지 않았다. 문서-only how-to에는 별도 링크 검사 실행파일 이름이 없으므로 위 inline 검사를 사용했다.

이번 domain 구현 검증: Windows PowerShell에서 `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`를 적용했다. `cargo test -p vnc-relay-domain`은 단위 시험 84개·문서 시험 0개, 실패 0개로 통과했다. 신규 시험은 matching 15개, session 14개, recent_closed 5개, 대상 정책 3개다. 이어 `cargo clippy -p vnc-relay-domain --all-targets -- -D warnings`가 종료 코드 0·경고 없음으로 통과했다. 전체 게이트와 다른 crate 시험은 실행하지 않았다. app·실제 소켓·암호 어댑터의 시험 결과로 확대하지 않는다. 새 시나리오 표식은 모두 `partial`이며 스펙 전체 완료를 뜻하지 않는다.

개정 문서의 최종 inline 검사도 통과했다. 상대 링크 36개, 제목 앵커 29개, Markdown 표 12개에서 오류가 없었다. 외부 API 확인은 aws-lc-rs 1.18.2의 공개 rustdoc와 cipher 모듈 원문에 한정한다. Context7 도구는 제공되지 않아 버전이 고정된 1차 문서를 직접 읽었다. 암호 어댑터를 실행하거나 UltraVNC·websockify·noVNC 원문을 읽지 않았다.

이번 app·adapters 검증은 Windows PowerShell에서 `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`와 `$env:AWS_LC_SYS_PREBUILT_NASM = "1"`을 적용해 실행했다.

| 명령 | 결과 | 확인 범위 |
| --- | --- | --- |
| `cargo test -p vnc-relay-domain -p vnc-relay-app -p vnc-relay-adapters` | 통과. 총 358개, 실패·무시 0개. domain 88, app 93, adapters 단위 52와 통합 125. 문서 시험 0개 | 운영 정보 계약 21개·새 TCP 대상 시험 6개 포함. Linux 전용 K 실패 시험은 Windows의 `cfg`에서 제외됨 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 통과, 종료 코드 0·경고 없음 | 작업 공간 전체 타입·lint 검사 |
| `cargo xtask layers` | 통과, 패키지 5개 | 내부·외부 의존 레이어 규칙 |

첫 실행의 초기 Close 시험 fixture는 임시 배열 참조 수명 오류(E0716)로 컴파일하지 못했다. 배열을 지역 값에 보존해 수정했다. 다음 검사에서 enum 크기 lint·중첩 조건 lint와 시험 helper의 `unwrap` lint를 고쳤다. 등록 전이는 추가 heap 할당을 피하려고 좁은 `expect`와 이유를 붙였고, 조건은 let-chain으로 바꿨다. 이어 K 실패가 일반 후보 재시도로 바뀌지 않도록 내부 오류 종류와 회귀 시험을 추가했다. 마지막 제품 코드 뒤에 표의 세 명령을 모두 실행해 통과했다. 전체 게이트·실제 프로세스 연결·WS 스트림·외부 VNC 상호 운용·네트워크 차단 감지·Ubuntu 실행은 하지 않았다. 현재 TCP 진입점이 실제 중계 기능을 제공한다고 확대하지 않는다.

이번 개정의 문서 inline 검사도 통과했다(`functions.eval`, Python). 이 문서는 상대 링크 36개·제목 앵커 29개·표 14개, `ops-info-v1.md`는 링크 131개·앵커 115개·표 24개를 확인했고 오류는 없었다. 전체 저장소의 기존 문서 오류를 다시 검사하거나 고치지 않았다.

WS 스트림 설계 연결 개정에서는 이 파일만 문서 검사했다(`functions.eval`, Python inline). 상대 링크 48개·제목 앵커 41개·표 16개를 확인해 오류 0개로 통과했다. 첫 검사에서 새로 쓴 내부 결정 항목 앵커 1개가 실제 제목과 달라 수정한 뒤 다시 통과했다. Rust 코드·다른 문서는 변경하지 않았고 Cargo·전체 게이트·커밋은 실행하지 않았다.

독립 리뷰(`review-2-7-10`) 지적 사항 반영 검증:
1. Close 대기에서 Force/Grace 만료를 반영하도록 `close_bounded` 통합 헬퍼를 도입하고, 초기 실패 및 활성화 전 `close_pair`에 Grace→G 만료 및 2차 shutdown 종료 시험을 추가했다.
2. 엔진 poll 직전에 `force_if_due`를 실행하고, 엔진 Ready 즉시 `engine_completed`로 표시해 이후 Force 사건의 `remaining_sessions`에서 제외하되 소켓 해제 전까지 ID 점유를 유지하도록 했다. 정밀 시계 회귀 시험 2건을 추가했다.
3. Mode II 미등록 연결 종료 시 계약에 없는 `Stage::Register` 대신 `Stage::InitialExchange`를 쓰고, 매칭 후 활성화 전 종료된 선착 대기자에는 `waiting_removed(reason=server_shutdown)`를 남기도록 수정했다. 역할별 시험을 추가했다.
4. `RecentClosedList::push`가 축출된 요약(또는 용량 초과로 미보관된 요약)을 반환하게 하여 $O(R^2)$ retain 대신 $O(1)$ remove를 수행하도록 개선하고 용량 및 R=0 일관성 시험을 추가했다.
위 표의 세 명령(cargo test 358건, clippy, xtask layers)을 모두 통과했다.

### 전체 게이트 실행 기록 (커밋 0cf18b6)

독립 리뷰 반영 후 커밋 `0cf18b6`(rustfmt 서식 보정 포함)에서 두 OS의 전체 게이트를 실행했다.

| 환경 | 명령 | 결과 | 비고 |
| --- | --- | --- | --- |
| Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 150건: 통과 23, 부분 87, 실패 0, 미실행 0, 미연결 40. 오류 0건, 판정 통과 | build, fmt, clippy, layers, tokens, tests, scenarios, licenses, deny 전체 통과 |
| Ubuntu 24.04.4(WSL2), Rust 1.99.1 | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 시나리오 150건: 통과 24, 부분 86, 실패 0, 미실행 0, 미연결 40. 오류 0건, 판정 통과 | Linux 전용 소켓·계정 권한 시험 포함 전체 통과 |

### 전체 게이트 실행 기록 (커밋 4e4a271)

커밋 `4e4a271`(2026-11-02)에서 [개발 환경 how-to](../how-to/set-up-and-verify.md) §3.4의 여섯 명령을 실행했다. 이번 라운드의 커밋은 `68a427b`(TCP 진입점 연결), `4391b14`(WS 코덱), `638cf09`(WS·WSS 연결), `f43c44a`(리뷰 반영), `0376e08`(설계 문서), `4a76049`(중계 수락 시험), `b2c0cac`(WS 수락 시험), `4e4a271`(`tests/serve.rs` 기대 수정)이다.

| 환경 | 명령 | 결과 | 비고 |
| --- | --- | --- | --- |
| Windows 11 x64 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 모든 단계 종료 코드 0. 시나리오 150건: 통과 60, 부분 62, 실패 0, 미실행 0, 미연결 28, 오류 0. 판정 통과. | RLY-009·OPS-023은 Linux 전용 시험이라 이 OS에서 부분이다. |
| Ubuntu 24.04 LTS x64, WSL2 | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 모든 단계 종료 코드 0. 시나리오 150건: 통과 63, 부분 59, 실패 0, 미실행 0, 미연결 28, 오류 0. 판정 통과. | 보고서의 `+ 미커밋 변경`은 CRLF 차이 때문이다(§3.4 7단계). WSL에서만 통과한 것은 RLY-009·OPS-023(Linux 전용 전체 시험)과 OPS-030(Unix 전용 `admin` 시험)이다. |

이전 WSL 게이트는 `b2c0cac`에서 실패했다. `tests/serve.rs`의 세 시험이 신호 종료 때 `shutdown_started`가 없어야 한다고 기대했다. `68a427b` 이후 제품은 운영 정보 계약 1.5절에 따라 이 사건을 기록한다. `4e4a271`은 시험 기대만 고쳤고 제품 코드는 바꾸지 않았다.

### 전체 게이트 실행 기록 (커밋 1a6f93e)

커밋 `1a6f93e`에서 두 OS의 전체 게이트를 실행했다. 이번 라운드의 커밋은 `59991f2`(재로드 중 연결 유지 시험, OPS-025·WS-022 전체 표식)와 `1a6f93e`(중계 로그 시험과 유휴 세션 시험)다. 추가된 시험은 RLY-027(K를 넘는 유휴 세션 유지, `tests/relay_log_scenarios.rs`)과 OPS-011·012의 중계 사건 로그·비밀 비노출 시험(같은 파일)이다. RLY-025·026은 여전히 부분이다.

| 환경 | 명령 | 결과 | 비고 |
| --- | --- | --- | --- |
| Windows 11 Pro 10.0.26200 x64 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 보고 2026-11-01T16:31:19Z: 통과 63, 부분 60, 실패 0, 미실행 0, 미연결 27, 오류 0 | RLY-027 통과 |
| Ubuntu 24.04 LTS x64(WSL2) | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 시나리오 보고 16:36:35Z: 통과 66, 부분 57, 실패 0, 미실행 0, 미연결 27, 오류 0 | RLY-027 통과. `+ 미커밋 변경`은 CRLF 때문에 나타남 |

### 전체 게이트 실행 기록 (커밋 59fc535)

커밋 `59fc535`에서 두 OS의 전체 게이트를 실행했다. 이번 라운드의 커밋은 `768d1af`(Q94 `internal_error` 기준 문서), `7c706c6`(관리자 종료·Kw·`internal_error` 구현), `cd60aad`(대시보드 문구), `538a4ec`(설계 문서), `59fc535`(Linux 소켓 단절 오류의 `dead_connection` 분류와 netns 시험)다. 추가된 시험은 RLY-025·026의 Linux 전용 netns 시험(`tests/relay_dead_peer_linux.rs`)과 소유 작업 panic 경로의 회귀 시험(`relay::tests::panic_after_eof_in_one_engine_poll_keeps_log_and_history_identical`)이다.

| 환경 | 명령 | 결과 | 비고 |
| --- | --- | --- | --- |
| Windows 11 Pro 10.0.26200 x64 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 보고 2026-11-01T23:41:53Z: 통과 77, 부분 61, 실패 0, 미실행 0, 미연결 12, 오류 0 | netns 시험은 Linux 전용이라 0건이다. RLY-025·026은 부분(사용자 결정 2026-11-02(Q86)), RLY-027 통과 |
| Ubuntu 24.04 LTS x64(WSL2) | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 실패(`FAILED: scenarios`). 시나리오 보고 23:51:07Z: 통과 81, 부분 55, 실패 2, 미실행 0, 미연결 12, 오류 0 | 실패 2건은 RLY-025·026이다. WSL에 `nft`가 없어 netns 시험 2건이 누락 도구를 밝히고 실패했다(사용자 결정 2026-11-02(Q93)). 실패한 시험은 이 2건뿐이며, 부분·미연결 시나리오는 왼쪽 수치와 같다. `+ 미커밋 변경`은 CRLF 때문에 나타남 |

### 전체 게이트 실행 기록 (커밋 bf27d30)

커밋 `bf27d30`에서 두 OS의 전체 게이트를 실행했다. WSL에는 사용자 요청(2026-11-02)으로 `nftables v1.0.9`를 설치했다. 이 커밋은 netns 시험의 결함 두 가지를 고쳤고 제품 코드는 바꾸지 않았다.

- 대기 시험이 설정 최솟값보다 작은 L=1로 시작해 설정 검사(`config_value_out_of_range`)에서 실패했다. L=2와 정상 대기 Server 하나로 바꿨다.
- 시간 판정에 벽시계인 로그 `ts`를 썼다. 그래서 반복 실행 약 12회 중 1회가 상한 16초를 넘었다. 같은 keepalive 값과 DROP 규칙으로 커널만 측정한 Python 탐침은 매번 11.0~11.2초에 연결 시간 초과로 끝났다. 벽시계로 14.077초가 나온 사례를 단조 시계로 재면 11.141초였다. 단조 시계 판정으로 바꾼 뒤 연속 3회 통과했다.

| 환경 | 명령 | 결과 | 비고 |
| --- | --- | --- | --- |
| Windows 11 Pro 10.0.26200 x64 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 보고 2026-11-02T15:30:51Z: 통과 77, 부분 61, 실패 0, 미실행 0, 미연결 12, 오류 0 | netns 시험은 Linux 전용이라 0건이다. RLY-025·026은 부분(사용자 결정 2026-11-02(Q86)), RLY-027 통과. 첫 실행은 D: 드라이브 공간 부족(`no space on device`)으로 `build` 단계에서 실패했다. 사용자 승인으로 시험 임시 폴더와 별도 빌드 폴더를 지운 뒤 다시 실행했다. `+ 미커밋 변경`은 실행 중 고친 문서 때문이며 코드는 `bf27d30`과 같다 |
| Ubuntu 24.04 LTS x64(WSL2) | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 시나리오 보고 2026-11-02T07:42:23Z: 통과 83, 부분 55, 실패 0, 미실행 0, 미연결 12, 오류 0 | RLY-025·026·027 통과. `+ 미커밋 변경`은 CRLF 때문에 나타남 |

### 작업 5.3 완료 대조(2026-11-04)

결론: 작업 5.3("정상 종료와 재시작")은 완료했다. 커밋 `de693ad`의 두 OS 게이트에서 작업 5.3의 완료 조건 시나리오(OPS-007~010, OPS-013, OPS-042~044)가 모두 통과했다.

- 구현: Windows 콘솔 창 닫기는 `ShutdownSignal::wait`가 `ShutdownRequest::ConsoleClosed`로 돌려준다. `begin_relay_shutdown`이 G로 유예를 시작해 `shutdown_started`를 쓴 뒤, 같은 차례에 `Duration::ZERO`로 강제 정리를 요청한다(커밋 `976550f`, 사용자 결정 2026-11-04(Q102)).
- 공통 시험 본문: `tests/common/shutdown.rs`를 구현하여 실제 프로세스 수명, 신호 전달, 종료 순서를 공유한다. `assert_final_log`가 `server_starting` 시작, `server_stopped` 끝, `shutdown_started < shutdown_forced`, 그리고 `shutdown_forced` 뒤에 `server_shutdown` 원인의 `session_closed`가 기록되는 순서를 단언한다.
- OS별 바이너리 시험: WSL은 `tests/unix_shutdown.rs`(SIGTERM), Windows는 `tests/windows_console.rs`(의사 콘솔 ConPTY, `#[expect(unsafe_code)]` 허용, 사용자 결정 2026-11-04(Q103))로 구축했다.
- 게이트 결과:
  - WSL `vr-full.sh` (2026-11-04T13:31:32Z 보고서): `ALL PASSED`. 152건 중 통과 107, 부분 35, 실패 0, 미실행 1(OPS-044), 미연결 9, 오류 0. `operations.md` 44건 중 통과 31, 부분 6, 미실행 1, 미연결 6.
  - Windows `vr-win.ps1 -Only all` (2026-11-04T13:39:13Z 보고서): `ALL PASSED (all)`. 152건 중 통과 103, 부분 40, 실패 0, 미실행 0, 미연결 9, 오류 0. `operations.md` 44건 중 통과 30, 부분 8, 미연결 6.

| 시나리오 | 판정 | WSL(Linux) 시험 | Windows 시험 |
| --- | --- | --- | --- |
| OPS-007 | 통과(두 OS) | `tests/unix_shutdown.rs`<br>`stop_unfinished_work` | `tests/windows_console.rs`<br>`stop_unfinished_work` |
| OPS-008 | 통과(두 OS) | `tests/unix_shutdown.rs`<br>`active_session_finishes_in_grace` | `tests/windows_console.rs`<br>`active_session_finishes_in_grace` |
| OPS-009 | 통과(두 OS) | `tests/unix_shutdown.rs`<br>`grace_expiry_cleans_session` (G=3 및 G=0) | `tests/windows_console.rs`<br>`grace_expiry_cleans_session` (G=3 및 G=0) |
| OPS-010 | 통과(두 OS) | `tests/unix_shutdown.rs`<br>`restart_has_fresh_state` | `tests/windows_console.rs`<br>`restart_has_fresh_state` |
| OPS-013 | 통과(두 OS) | `crates/adapters/tests/ops_info_contract.rs`<br>`answers_liveness_and_readiness_in_each_state_with_only_the_contract_fields`<br>(보강: `tests/unix_shutdown.rs` `management_works_during_grace`) | `crates/adapters/tests/ops_info_contract.rs`<br>`answers_liveness_and_readiness_in_each_state_with_only_the_contract_fields`<br>(보강: `tests/windows_console.rs` `management_works_during_grace`) |
| OPS-042 | 통과(두 OS) | `tests/unix_shutdown.rs`<br>`second_request_forces_cleanup` | `tests/windows_console.rs`<br>`second_request_forces_cleanup` |
| OPS-043 | 통과(두 OS) | `tests/unix_shutdown.rs`<br>`management_works_during_grace` | `tests/windows_console.rs`<br>`management_works_during_grace` |
| OPS-044 | 통과(Windows 전용) | 해당 없음(Windows 전용 스펙) | `tests/windows_console.rs`<br>`console_close_forces_cleanup` (5초 이내 프로세스 종료) |

- 추가 검증: Windows 정상 종료 신호인 Ctrl+C(`ctrl_c_stops_gracefully_with_exit_code_zero`)와 Ctrl+Break(`ctrl_break_stops_gracefully_with_exit_code_zero`)의 종료 코드 0 정상 종료도 `tests/windows_console.rs`에서 통과했다.

## 9. 미결정 사항

<a id="1-시작-키-생성-실패의-공개-오류-코드"></a>

### 1. (해결) 시작 키 생성 실패의 공개 오류 코드

사용자 결정 2026-11-01(Q76)로 선택 B(`runtime_unavailable`, 종료 코드 1)를 확정했다. RelayIdKey와 세션 식별자 키 생성 실패 모두에 이 코드를 적용한다. 키 생성 시점은 5.1절의 시작 4단계이며, 런타임·종료 신호 준비 뒤 TLS 검사 전이다. 리스너를 바인드하기 전에 멈추고 확보한 자원을 회수한다. 고정 키나 키 없는 해시로 대체하지 않는다.

### 해결한 설계 항목

아래 2~5번은 **이 문서의 판단, 사용자 결정 없음**이다. 6번은 사용자 결정 2026-11-01(Q82)이다. 판단의 이유와 적용 위치를 기록한다.

| 이전 항목 | 판단·이유 | 적용 위치·근거 |
| --- | --- | --- |
| 2. 세션 식별자 | 시작 키와 128비트 카운터의 키 있는 순열을 사용한다. 재사용을 구조적으로 막고 메모리를 일정하게 유지한다. 런타임 RNG·발급 집합·실패 사건은 없다. 공개 AES 블록 API의 반환형을 확인해 무실패 HMAC Feistel 구성을 선택했다. | 3.3·4.2·5.1절, [관리 API 1.8절](../contracts/management-api-v1.md#18-활성-세션-get-apiv1sessions-get-apiv1sessionssession_id) |
| 3. 대상 K 실패 | 활성화하지 않고 대상 소켓·L을 회수한다. 기존 `target_connect_failed/connect_error`를 쓰고 `error_kind`는 생략한다. RLY-R18의 모든 중계 TCP 소켓 K 요구를 지킨다. `connection-limits.md` 6절의 수락 측 규칙은 이 판단과 다르며, 이 설계는 그 규칙을 바꾸지 않는다. | 4.1·6절, [RLY-R18](../specs/relay.md#21-판정-가능한-규칙), [기존 수락 측 규칙](connection-limits.md#6-오류-처리) |
| 4. B가 찬 뒤 FIN | 선택 B. 읽기가 0 또는 오류를 반환해야 관찰한 종료다. B가 차면 읽기를 쉬므로 매칭 뒤 버퍼 소진 후 감지할 수 있다. OS별 준비 신호 없이 이식성과 TCP half-close 의미를 유지한다. W·Lw는 계속 적용한다. | 3.2·4.2절, [RLY-014·019](../specs/relay.md#7-수용-테스트) |
| 5. DNS 후보 | 선택 A. 반환 순서의 허용 후보를 순차 시도하고 C·대상 소켓 한 개를 유지한다. 실패 집계와 마지막 `target_ip`로 원인을 추적한다. | 4.1·6절, [RLY-009](../specs/relay.md#7-수용-테스트) |
| 6. 겹친 종료 원인 | ops-info §1.6 규칙을 문자 그대로 따른다(사용자 결정 2026-11-01(Q82)). 한 세션의 `close_cause`는 정리를 시작하게 한 첫 원인이다. 강제 정리 시점에 이미 정리를 시작한 세션은 최초 원인을 유지하고, 아직 정리를 시작하지 않은 세션만 `server_shutdown`을 기록한다. | 6절, [ops-info-v1 1.6절](../contracts/ops-info-v1.md#16-원인-코드), [관리 API 1.10절](../contracts/management-api-v1.md#110-최근-종료-목록-get-apiv1closed-sessions), [CLI 1.8절](../contracts/cli-v1.md#18-수동-시작과-종료) |

중복 정책(Q74), 중계 ID 정확 일치, C의 해석+연결 범위, W·Lw, TCP half-close, 강제 정리 때의 종료 원인(Q82), 세션 ID의 외부 형식은 다시 질문하지 않는다. WS의 M>B는 Q78로 정했다. Kw의 여러 Ping과 backpressure 중 Pong 기한은 사용자 결정 2026-11-02(Q84)로 정했고 [TODO 3.4](../TODO.md#3-wswss-진입점)에서 구현했다([WebSocket 스트림 설계 9.5절](websocket-stream.md#95-kw의-여러-ping과-b-backpressure)).
