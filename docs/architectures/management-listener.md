# 관리 리스너 모듈 설계

관리 리스너(`[management] listen`)가 접속을 받고, 관리 HTTPS(내장 TLS)와 루프백 평문 HTTP를 처리하는 구조를 다룬다. 접속은 상태 리스너와 같은 관리·상태 연결 상한 Lm을 쓴다. 관리 리스너의 접속은 L을 쓰지 않는다. 이 문서는 작업 4.2에서 썼다. 로그인과 세션 검사는 작업 4.3이 더했다. 조회(4.4)·종료(4.5)·정적 파일(4.6)은 맡지 않는다.

| 항목 | 값 |
| --- | --- |
| 구조 시점 | Current |
| 문서 상태 | Draft |
| 구현 대조 | Verified — 작업 4.3 시점, 커밋 `59f3f62` + 작업 트리. 4.3 항목은 [대조 기록](#대조-기록)에 있다 |
| 코드 위치 | `crates/adapters/src/admission.rs`(`AdmittingListener`, `Admitted`), `crates/adapters/src/tls_listener.rs`(`TlsListener`, `TlsIo`, `Handshake`, `Recorded`, `offers_no_tls12`), `crates/adapters/src/request_deadline.rs`(`DeadlineListener`, `Deadlined`), `crates/adapters/src/management_http.rs`(`ManagementListener`, `PeerAddr`, `answer`), `crates/adapters/src/management_http/session_api.rs`(`answer`), `crates/adapters/src/status_http.rs`(`StatusListener`), `crates/adapters/src/config_file.rs`(`read_management`), `crates/app/src/events.rs`(`Stage::Tls`, `RejectReason::TlsVersionUnsupported`), `crates/app/src/settings.rs`(`Settings::management_listener`, `Settings::other_listeners`), `src/serve.rs`(`listen_until_shutdown`, `stop_task`) |
| 코드 지도 | [코드 지도 2절](code-map.md#2-코드-위치와-구성요소), [3절](code-map.md#3-진입점) |
| 관련 스펙 | [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)의 OPS-014, OPS-034, OPS-033(일부: 관리 연결), OPS-004(일부: 관리 연결을 L에 세지 않음), OPS-036, OPS-006(일부: 관리 TLS 협상과 첫 요청 머리의 U, 사용자 결정 2026-10-21 Q12), [관리 스펙 7절](../specs/management.md#7-수용-테스트)의 MGT-015(4.4 완료 조건으로 옮김, 사용자 결정 2026-10-21 Q14) |
| 관련 계약 | [관리 API 계약 1.1·1.3·1.4.1·1.11절](../contracts/management-api-v1.md#11-공통-규칙)(라우팅 순서, 쿠키, 정적 경로), [2.1·2.2절](../contracts/management-api-v1.md#21-오류-본문과-상태-코드)(401 본문과 헤더), [운영 정보 계약 1.5·1.6절](../contracts/ops-info-v1.md#15-사건-목록)(`connection_rejected`, `tls_version_unsupported`), [설정 계약 1.2절](../contracts/config-v1.md#12-키-표)(`management.listen`, `management.cert_file`·`key_file`) |
| 관련 ADR | [ADR-0001 내장 TLS](../decisions/ADR-0001-built-in-tls.md), [ADR-0004 tokio·rustls](../decisions/ADR-0004-tokio-rustls.md) |

## 1. 책임과 경계

- 맡는 책임:
  - 접속 수락: 관리 리스너는 바인드한 뒤 접속을 받는다. 수락 직후 Lm 예산의 자리를 잡는다. 자리가 없으면 `connection_rejected`(`listener` = `management`, `stage` = `accept`, `reason` = `admin_connection_limit`, `limit`)를 기록하고 HTTP 응답 없이 닫는다.
  - 관리 HTTPS: `[management] cert_file`·`key_file`이 있으면 수락한 접속마다 내장 TLS 핸드셰이크를 한다(`TlsAcceptor`, 작업 3.1). TLS 1.2 미만만 제안한 클라이언트는 `connection_rejected`(`listener` = `management`, `stage` = `tls`, `reason` = `tls_version_unsupported`)를 기록하고 평문 응답 없이 닫는다(OPS-036).
  - 평문 HTTP: 두 파일이 모두 없으면 평문 HTTP를 쓴다. 설정 검사가 주소를 루프백으로 제한한다(`config_file.rs` `read_management`, 설정 계약 1.2절). 관리 리스너는 이 제한을 다시 검사하지 않는다.
  - 요청 처리 순서(3·4절): `POST /api/v1/session`은 로그인이다. 세션을 보지 않고 로그인 처리로 넘긴다(관리 로그인과 세션 설계 4절). 나머지 `/api/v1/` 요청은 관리 세션을 먼저 본다. 세션이 없으면 경로·메서드와 관계없이 401을 준다(관리 API 계약 1.3절 2단계). 정적 경로는 1.11절 표를 따른다.
  - 응답 헤더: 모든 관리 응답에 `Cache-Control: no-store`를 붙인다(1.1절). 모든 401에 `WWW-Authenticate: Session realm="vnc-relay"`를 붙인다(2.1절). 쿠키가 있었던 401은 `relay_admin_session` 쿠키를 지운다(1.4.1절). HTTPS 리스너가 보낸 지우기 쿠키에는 `Secure`를 붙인다.
  - 상대 주소: 관리 응답에 들어가는 주소는 소켓의 실제 상대 주소다. 전달 헤더(`Forwarded`·`X-Forwarded-For`)는 읽지 않는다(1.1절, 관리 스펙 MGT-015).
  - 요청 시간 제한: 접속마다 U(`[timeouts] tls_upgrade_secs`)를 건다. TLS 핸드셰이크와 첫 요청 머리가 끝날 때까지는 수락 때부터 잰 절대 기한이다. 그 뒤에는 읽기·쓰기가 있을 때마다 U를 다시 잰다(구현 판단, 9절 11번). 기한이 지나면 평문 응답 없이 닫는다(4·6절).
- 맡지 않는 책임:
  - 로그인 처리와 관리 세션 발급(작업 4.3, [관리 로그인과 세션 설계](management-session.md)).
  - 관리 세션 검사·CSRF 검사·유휴 만료(작업 4.3, [관리 로그인과 세션 설계](management-session.md)).
  - 조회 API 처리(작업 4.4), 종료 API 처리(작업 4.5), 정적 파일 제공(작업 4.6).
  - 관리 리스너의 연결 기록 `connection_accepted`. ops-info-v1 §1.5가 관리 리스너에 이 사건을 정하지 않는다.
  - TLS 핸드셰이크 실패 가운데 버전 거절이 아닌 것(인증서 오류, 클라이언트 중단). 9절 6번(Q13).
- 다른 모듈에 공개하는 기능:
  - `vnc_relay_adapters::management_http::ManagementListener`
  - `vnc_relay_adapters::tls_listener::TlsListener`(관리 HTTPS가 쓴다. WSS 진입점은 `TlsIo`를 소켓마다 직접 쓴다)
  - `vnc_relay_adapters::admission::AdmittingListener`(상태·관리 리스너가 같이 쓴다)

## 2. 내부 구조

화살표는 실행 중 호출이다. 현재 코드와 목표 구조를 한 그림에 섞지 않는다. 이 그림은 현재 구조다.

```mermaid
flowchart LR
    SERVE["src/serve.rs<br/>listen_until_shutdown"] -->|"1 관리 리스너 바인드<br/>(bind 계획 management)"| BIND["adapters net.rs<br/>bind"]
    SERVE -->|"2 spawn (준비됨 뒤)"| MGMT["adapters management_http.rs<br/>ManagementListener::run"]
    MGMT -->|"3 수락 직후 Lm 자리"| ADM["adapters admission.rs<br/>AdmittingListener"]
    ADM -->|"Lm 거절 Event"| SINK["app ports.rs<br/>EventSink::record"]
    ADM -->|"4 HTTPS일 때 감쌈"| TLSL["adapters tls_listener.rs<br/>TlsListener"]
    TLSL -->|"5 읽기·쓰기 때 핸드셰이크<br/>(TlsAcceptor, 3.1)"| TLS["adapters tls.rs<br/>load_acceptor 결과"]
    TLSL -->|"TLS 1.2 미만 거절 Event"| SINK
    MGMT -->|"6 HTTP 요청 처리<br/>fallback 라우터"| ROUTE["adapters management_http.rs<br/>answer"]
    SERVE -->|"관리 TLS acceptor 전달"| MGMT
    MGMT -->|"U 기한 래퍼가 가장 바깥"| DL["adapters request_deadline.rs<br/>DeadlineListener"]
    DL -->|"감싼 리스너(HTTPS는 TlsListener, 평문은 AdmittingListener)"| TLSL
```

| 요소(파일·심볼) | 역할 | 만들어지는 시점과 소유자 | 의존 대상 |
| --- | --- | --- | --- |
| `crates/adapters/src/admission.rs` `AdmittingListener` | 수락 직후 Lm 자리를 잡는다. 자리가 없으면 거절 Event를 기록하고 닫는다. 리스너 이름(`management`·`status`)을 받는다. | 관리·상태 리스너의 `run` 안에서 만든다 | tokio, app, domain |
| `crates/adapters/src/admission.rs` `Admitted` | Lm 자리를 든 접속. 자리는 접속이 끝나면 돌아간다. TCP 스트림만 감싼다. | `AdmittingListener::accept`가 만든다 | tokio, domain |
| `crates/adapters/src/tls_listener.rs` `TlsListener` | `axum::serve::Listener`를 감싼다. 수락 때는 핸드셰이크를 기다리지 않는다. 접속의 `Io`가 읽기·쓰기 때 핸드셰이크를 진행한다. | HTTPS 관리 리스너가 `run` 안에서 만든다 | tokio-rustls, app, domain |
| `crates/adapters/src/request_deadline.rs` `DeadlineListener`, `Deadlined` | `axum::serve::Listener`를 감싼다. 접속마다 U 타이머를 든다. 첫 요청 머리(`\r\n\r\n`)가 끝나기 전에는 수락 때부터 잰 절대 기한이다. 끝난 뒤에는 읽기·쓰기가 있을 때마다 U를 다시 건다. 기한이 지나면 `TimedOut` 오류를 돌려준다. | `ManagementListener::run`이 HTTPS·평문 모두에서 `axum::serve`에 넘기는 가장 바깥 리스너로 만든다 | tokio, axum |
| `crates/adapters/src/management_http.rs` `ManagementListener` | 관리 리스너 하나. 관리 HTTPS 또는 루프백 평문 HTTP를 처리한다. | `serve`가 관리 리스너를 바인드한 뒤 띄운다. 종료 때 `serve`가 멈춘다. | axum, tokio, tokio-rustls, adapters 내부 모듈 |
| `crates/adapters/src/management_http.rs` `answer` | fallback 라우터. 3·4절 순서로 응답을 고른다. `/api/v1/` 요청은 `management_http/session_api.rs`의 `answer`로 넘긴다. | `ManagementListener::run`이 만든다 | axum, serde_json |
| `crates/adapters/src/management_http.rs` `PeerAddr` | 접속의 상대 소켓 주소를 `PeerIp`로 바꾼다(IPv4 매핑 IPv6는 IPv4로). 로그인 제한 키와 로그 `peer_ip`가 같은 값을 쓴다. `axum::extract::connect_info::Connected`를 구현한다. | `ManagementListener::run`이 `into_make_service_with_connect_info::<PeerAddr>()`로 만든다 | domain(`PeerIp`), axum |
| `crates/adapters/src/management_http/session_api.rs` `answer` | `/api/v1/` 요청을 맡는다. `POST /api/v1/session`은 로그인(`login`)이다. 그 밖은 세션 검사 → CSRF → 경로 → 메서드 순으로 검사한다. 4.4·4.5 경로는 아직 없어 인증 뒤 `404 not_found`다(관리 로그인과 세션 설계 9절 카). | `management_http.rs` `answer`가 `/api/v1/`일 때 부른다 | axum, app(`AdminAuth`) |
| `crates/adapters/src/status_http.rs` `StatusListener` | 상태 리스너. 이 문서가 옮긴 `AdmittingListener`를 `status` 이름으로 쓴다. | `serve`가 상태 리스너를 바인드한 직후 띄운다. | `admission` 모듈 |
| `crates/app/src/events.rs` `Stage::Tls`, `RejectReason::TlsVersionUnsupported` | `connection_rejected`의 `stage` 값 `tls`와 `reason` 값 `tls_version_unsupported` | 사건이 생길 때 | 없음 |
| `crates/adapters/src/config_file.rs` `read_management` | 평문 HTTP 주소가 루프백이 아니면 `config_plaintext_management_not_loopback` 설정 오류를 낸다. 관리 리스너는 이 검사를 다시 하지 않는다. | 설정 읽기 때 | 없음 |
| `crates/app/src/settings.rs` `Settings::management_listener`, `Settings::other_listeners` | `management_listener`는 관리 리스너 바인드 계획(이름 `management`, 키 `management.listen`)을 만든다. `other_listeners`는 관리 리스너를 넣지 않는다. | `serve`가 바인드 전에 부른다 | 없음 |
| `src/serve.rs` `listen_until_shutdown` | 관리 리스너를 바인드하고, 준비됨 뒤 띄우고, 종료 때 진입점 정리 뒤에 멈춘다. 관리 TLS acceptor를 `ManagementListener`에 넘긴다. WSS용 `wss_tls`는 `take()`로 꺼내 `WsEntryPoint`에 넘긴다(작업 3.2). | serve 실행마다 | adapters, app, domain, tokio |

## 3. 데이터 모델

| 이름 | 필드·형태 | 저장 위치 | 제약과 항상 지켜야 할 조건 |
| --- | --- | --- | --- |
| `ManagementListener` | `listener: TcpListener`, `tls: Option<TlsAcceptor>`, `admin_budget: Arc<ConnectionBudget>`, `tls_upgrade: Duration`(U) | 메모리, `run`이 소비 | `tls`가 `Some`이면 HTTPS, `None`이면 평문. 두 값 중 하나만 쓴다. `tls_upgrade`는 설정 `[timeouts] tls_upgrade_secs`다. |
| `TlsListener<L>` | `inner: L`, `acceptor: TlsAcceptor`, `log: Arc<dyn EventSink>`, `name: &'static str` | 메모리 | `L::Addr`는 `SocketAddr`여야 한다. 거절 기록에 상대 IP를 쓴다. |
| `TlsIo<IO>` | `Handshaking(Handshake<IO>)`, `Established(TlsStream<Recorded<IO>>)`, `Failed(HandshakeFailure)` | 접속 하나의 `Io`값 | 상태는 앞으로만 간다: `Handshaking` → `Established` 또는 `Failed`. `Failed`에서는 되돌아가지 않는다. |
| `Handshake<IO>` | `accept: Accept<Recorded<IO>>`, `capture: Capture`, `peer: PeerIp`, `log: Arc<dyn EventSink>`, `name: &'static str` | `TlsIo::Handshaking` 안 | 핸드셰이크가 끝나면 `Accept`를 버린다. 버전 거절을 기록할 때 `peer`를 쓴다. |
| `Recorded<IO>` | `io: IO`, `capture: Capture`(`Arc<Mutex<Option<Vec<u8>>>>`) | 접속 하나. `Accept`가 읽는 `Io`를 감싼다 | 핸드셰이크 동안 읽은 바이트를 `capture`에 복사한다(최대 `CAPTURE_LIMIT` = 5 + 16 384 바이트). 핸드셰이크가 끝나면 `capture`를 `None`으로 바꾼다. |
| `Admitted` | `stream: TcpStream`, `_permit: BudgetPermit` | 접속 하나 | `_permit`은 `Admitted`가 사라질 때 Lm으로 돌아간다. 핸드셰이크 중에도 자리를 든다. |
| `Deadlined<IO>` | `io: IO`, `limit: Duration`(U), `timer: Pin<Box<Sleep>>`, `head_done: bool`, `window: u32`(마지막 4바이트) | 접속 하나 | `head_done`은 `\r\n\r\n`을 읽으면 참이 되고 되돌아가지 않는다. `window`는 머리 끝을 찾는 데만 쓴다. 기한이 지나면 읽기·쓰기가 `TimedOut`을 돌려준다. |
| 관리 응답 본문 | `{"error":{"code":"unauthenticated","message":…}}`, `{"error":{"code":"not_found","message":…}}` | HTTP 응답 | 2.1절 형식만 쓴다. `message`는 요청 입력값을 담지 않는다. |
| 지우기 쿠키 | `relay_admin_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict` (HTTPS면 `; Secure` 추가) | `Set-Cookie` 헤더 | 1.4.1절 속성과 맞춘다. 값은 비운다. |

## 4. 처리 순서

| 순서 | 코드 위치 | 조건·계산 | 바뀌는 상태 | 결과를 쓰는 곳 |
| --- | --- | --- | --- | --- |
| 1 | `admission.rs` `AdmittingListener::accept` | TCP 수락 뒤 Lm 자리를 잡는다. 수락 오류는 `ACCEPT_RETRY_DELAY` 뒤 다시 수락한다. | Lm 사용량 +1 | 자리 없음: `connection_rejected`(`stage` = `accept`, `admin_connection_limit`), 연결 닫음 |
| 2 | `tls_listener.rs` `TlsListener::accept` | 평문이면 이 단계를 건너뛴다(`tls` = `None`일 때 `serve`가 `TlsListener`를 만들지 않는다). HTTPS면 `TlsIo::Handshaking`을 만든다. 핸드셰이크는 하지 않는다. | `TlsIo` 상태 `Handshaking` | 없음 |
| 3 | `tls_listener.rs` `TlsIo::poll_handshake` | `poll_read`·`poll_write`·`poll_flush`가 먼저 부른다. 핸드셰이크를 진행한다. 성공하면 `Established`로 바꾸고 캡처를 `None`으로 바꾼다. 실패하면 캡처한 ClientHello 바이트(레코드 한 개 분)로 버전을 판정한다. `supported_versions`가 있으면 그 목록에 TLS 1.2 이상이 없을 때, 없으면 `legacy_version`이 TLS 1.2 미만일 때 버전 거절이다. 버전 거절이면 기록을 먼저 쓴다. 그다음 `Failed`로 바꾸고 `io::Error`를 돌려준다. | `TlsIo` 상태 `Established` 또는 `Failed` | 버전 거절: `connection_rejected`(`listener` = `management`, `stage` = `tls`, `reason` = `tls_version_unsupported`). 그 밖의 실패: 기록 없음(사용자 결정 Q13). |
| 4 | `request_deadline.rs` `Deadlined::poll_read`·`poll_write`·`poll_flush` | 읽기·쓰기 전에 타이머를 먼저 본다. 기한이 지났으면 `TimedOut`을 돌려준다. 첫 요청 머리가 끝나기 전에는 절대 기한(수락 + U)이다. 끝난 뒤 바이트가 오가면 U를 다시 건다. 2·3단계의 TLS 핸드셰이크도 이 안에서 진행된다. | 타이머, `head_done`, `window` | 기한이 지나면 axum이 접속을 닫는다(6절). 머리가 끝나면 axum이 요청을 5단계(`answer`)로 넘긴다. |
| 5 | `management_http.rs` `answer` 1단계 | 경로가 `/api/v1/`로 시작하면 `management_http/session_api.rs` `answer`로 넘긴다. 그 안에서 `POST /api/v1/session`은 세션을 보지 않고 로그인한다. 그 밖의 `/api/v1/` 요청은 세션을 먼저 본다. 세션이 없으면 경로·메서드와 관계없이 `401`이고, 요청에 `relay_admin_session` 쿠키가 있었으면 지우기 `Set-Cookie`를 붙인다. 세션이 있으면 CSRF(`POST`·`DELETE`) → 경로 → 메서드 순으로 검사한다(관리 로그인과 세션 설계 4절, 관리 API 계약 1.3절). | 없음 | 응답(`WWW-Authenticate`, `Cache-Control: no-store`). 로그인과 세션의 응답 본문·상태 코드는 관리 로그인과 세션 설계 4·6절 |
| 6 | `management_http.rs` `answer` 2단계 | 경로가 `/`이거나 `/app/`로 시작하면 메서드가 `GET`·`HEAD`가 아닐 때 `405`(`Allow: GET, HEAD`, `invalid_request` 본문). 그 밖에는 `303`, `Location: /login/`, 본문 없음. | 없음 | 응답 |
| 7 | `management_http.rs` `answer` 3단계 | 경로가 `/login/`로 시작하면 메서드가 `GET`·`HEAD`가 아닐 때 `405`(`Allow: GET, HEAD`, `invalid_request` 본문). 그 밖에는 `404` `not_found` 본문(열린 질문 2). | 없음 | 응답 |
| 8 | `management_http.rs` `answer` 4단계 | 나머지 경로는 메서드와 관계없이 `303`, `Location: /login/`. | 없음 | 응답 |

관리 응답은 모두 `Cache-Control: no-store`를 가진다. 정적 보안 헤더(`X-Content-Type-Options`·`Referrer-Policy`·`Content-Security-Policy`, 1.11절 권장)는 정적 파일 응답에만 붙이며 4.2에는 정적 파일 응답이 없다.

## 5. 상태와 초기화

- 관리 리스너는 바인드 뒤 `serve`가 준비됨 전환과 `server_ready` 기록을 마친 뒤 접속을 받는다. 상태 리스너처럼 준비 전 응답은 없다.
- Lm 사용량은 접속마다 하나씩 늘고 접속이 끝나면 줄어든다. 관리 연결과 상태 연결이 같은 예산을 쓴다.
- 관리 TLS acceptor는 `serve` 시작 단계에서 검사하고 만든다(`tls.md` 7절). 핸드셰이크 중 인증서를 다시 읽지 않는다. 설정 변경은 재시작으로 반영한다(설정 계약 1.2절).
- 종료 요청을 받으면 `serve`가 진입점 연결을 정리한 뒤 관리 리스너 작업을 멈춘다. 종료 유예 G와 `shutdown_started`는 작업 5.3이 맡는다.

## 6. 오류 처리

| 실패 지점 | 처리 방법 | 남는 상태 | 관련 스펙 시나리오 |
| --- | --- | --- | --- |
| Lm 자리 없음 | `connection_rejected`(`stage` = `accept`, `admin_connection_limit`, `limit`). HTTP 응답 없이 닫는다. | 접속 없음. L은 바뀌지 않는다. | OPS-033(일부), OPS-004(일부) |
| TLS 1.2 미만만 제안 | 평문 응답 없이 닫는다. 같은 접속에서 평문으로 바꾸지 않는다. `connection_rejected`(`stage` = `tls`, `tls_version_unsupported`). 버전 판정은 접속의 첫 TLS 레코드(5 + 16384 = 16389바이트)만 본다. ClientHello가 그 레코드를 넘으면 판정하지 못해 기록하지 않는다. rustls는 그래도 접속을 거절한다(사용자 결정 2026-10-21, Q17). | 접속 없음 | OPS-036 |
| 그 밖의 TLS 핸드셰이크 실패(인증서 오류, 클라이언트가 중간에 끊음) | 평문 응답 없이 닫는다. 기록하지 않는다(사용자 결정 2026-10-21, Q13). | 접속 없음. 자리는 돌아간다. | OPS-036(일부) |
| TLS 핸드셰이크가 U 안에 끝나지 않음 | 수락 때부터 U가 지나면 읽기·쓰기가 `TimedOut`으로 끝나고 axum이 접속을 닫는다. 평문 응답을 보내지 않는다. 기록하지 않는다(구현 판단: `connection_aborted`는 `conn_id`가 필요하다). | 접속 없음. Lm 자리는 돌아간다. | OPS-006(일부), OPS-033(일부) |
| 첫 요청 머리가 U 안에 끝나지 않음 | 같은 방법으로 닫는다. 기록하지 않는다(구현 판단). | 접속 없음. Lm 자리는 돌아간다. | OPS-006(일부), OPS-033(일부) |
| 첫 요청 뒤 읽기·쓰기가 U 동안 없음(keep-alive 유휴) | 같은 방법으로 닫는다. 기록하지 않는다(구현 판단, 9절 11번). | 접속 없음. Lm 자리는 돌아간다. | OPS-006(일부) |
| 수락 오류 | `ACCEPT_RETRY_DELAY` 뒤 다시 수락한다. 기록하지 않는다(ops-info-v1은 수락 실패 사건이 없다). | 없음 | 없음 |
| 관리 요청의 읽기 오류 | axum이 접속을 닫는다. 기록하지 않는다(`connection_aborted`는 `conn_id`가 필요하다). | 접속 없음 | 구현 판단(Q13은 TLS 실패만 다룬다) |

## 7. 설계 선택

| 선택 | 이유 | 검토한 다른 방법 | 근거 |
| --- | --- | --- | --- |
| TLS 핸드셰이크를 `TlsIo`의 읽기·쓰기 때 진행한다 | 한 클라이언트가 핸드셰이크를 멈춰도 다른 접속의 수락이 막히지 않는다. axum이 접속마다 작업을 만든다. | 수락 루프에서 `TlsAcceptor::accept`를 기다린다(수락이 한 접속에 묶임). 수락마다 백그라운드 작업과 채널을 만든다(구조가 커짐). | ADR-0004 tokio·rustls. 설계의 같은 접속 수락 원칙(tcp-entry-point 설계). |
| Lm 자리를 TLS 핸드셰이크 전에 잡는다 | 핸드셰이크 작업도 Lm 안에 들어가 자원을 제한한다. 관리 TLS는 Lm을 넘지 않는다. | 핸드셰이크 뒤에 자리를 잡는다(핸드셰이크 작업이 Lm 밖에서 늘어남). | OPS-033(Lm은 관리·상태 연결 상한). |
| `TlsListener<L>`을 일반 `Listener`로 만든다 | 관리 HTTPS가 이 wrapper를 쓴다. WSS 진입점은 `TlsIo`를 소켓마다 직접 쓴다(`Wire`, 작업 3.2). | 관리 리스너 안에 TLS 코드를 넣는다(WSS 중복). | tls.md 1절(연결 수락은 진입점 모듈) |
| `Admitted`는 `TcpStream`만 감싼다 | 관리·상태 리스너 모두 평문 TCP 위에 Lm을 세운다. TLS는 그 위에 쌓는다. | `Admitted<S>`로 일반화한다(사용처가 없다). | 이 문서 2절 |
| 관리 라우팅은 fallback 한 곳에서 한다 | 1.3절 순서(세션 확인이 경로·메서드 판정보다 먼저)를 한 함수에 둔다. 상태 리스너와 같은 방식이다. | 경로마다 axum 라우트를 둔다(순서가 흩어지고 HEAD·405 기본 응답이 계약과 다르다). | 관리 API 계약 1.3절, `status_http.rs` 주석 |
| 지우기 쿠키의 `Secure`는 리스너 종류로 정한다 | 1.4.1절 표가 HTTPS 리스너에만 `Secure`를 정한다. | 요청 헤더로 판정한다(전달 헤더를 쓰지 않는다는 1.1절과 충돌). | 관리 API 계약 1.4.1절, MGT-017 |
| 관리 요청 읽기와 TLS 핸드셰이크에 U를 쓴다. 첫 요청 머리가 끝날 때까지 접속 수락 때부터 잰다. 그 뒤에는 읽기·쓰기가 한 번 있을 때마다 U를 다시 잰다(구현 판단, 9절 11번). | 멈춘 클라이언트가 Lm 자리를 무기한 잡지 못하게 한다. U는 WS·WSS 업그레이드와 같은 설정 값이다(사용자 결정 2026-10-21, Q12). | 새 설정 키를 만든다(계약 변경). 제한 없이 둔다(멈춘 클라이언트가 자리를 무기한 잡는다). | 설정 계약 1.2절 U 행, 운영 스펙 1절 U, OPS-006 |
| 시간 제한은 `serve::Listener` 래퍼(`request_deadline.rs`)로 건다 | `axum::serve`에는 읽기 제한 설정이 없다. 래퍼는 HTTPS와 평문에 같은 규칙을 준다. | hyper 연결 설정을 직접 쓴다(hyper를 직접 크레이트로 쓰는 결정이 없다). | axum 0.8.9 `serve` |
| ALPN을 쓰지 않는다 | `tls.rs`가 ALPN을 설정하지 않는다. 관리 API는 HTTP/1.1만 쓴다. | `http/1.1`을 광고한다(사용자 결정 2026-10-21, Q15로 쓰지 않기로 했다). | tls.md 7절, 9절 |
| 버전 거절은 ClientHello 바이트로 판정한다(RFC 8446 4.2.1). | rustls는 버전을 보기 전에 `signature_algorithms` 확장을 검사한다(`server/hs.rs` `process_client_hello`). rustls 오류 변형으로 판정하면 TLS 1.0·1.1 클라이언트를 버전 거절로 못 보고, `signature_algorithms`가 없는 TLS 1.2 클라이언트를 잘못 거절한다. | rustls 오류 변형으로 판정(기각). | rustls 0.23.46 `server/hs.rs`, RFC 8446 4.2.1 |

## 8. 현재 코드와의 차이

- 4.2의 대조 결과는 4.3 이후 현재 구현과 맞지 않는다. 작업 4.3이 '맡지 않는 책임' 목록과 4절 5단계를 바꿨다. 아래 대조 기록의 4.2 항목은 그 시점 기록으로 남기고, 4.3 항목이 현재 기준이다.

## 대조 기록

### 작업 4.3

- 대조 범위: 커밋 `59f3f62` + 작업 트리(이 문서, `management-session.md`, `management-api-v1.md`, `ops-info-v1.md`, `code-map.md`, `architectures/README.md`, `docs/TODO.md`의 문서 변경).
- 대조 방법: 2절 요소 표의 파일·심볼을 `grep`으로 찾았다. `PeerAddr`(`management_http.rs` 118행)와 `session_api.rs` `answer`를 2절 표에 더했다. `management_http.rs` `answer`의 `/api/v1/` 분기와 `session_api.rs`의 검사 순서(세션 → CSRF → 경로 → 메서드)를 4절 5단계와 비교해 5단계를 고쳤다. 1절 응답 헤더 규칙은 `unauthorized`(206행), `CLEAR_SESSION_COOKIE`(42행)·`CLEAR_SESSION_COOKIE_SECURE`(44행)와 맞는다. 4절 6–8단계(정적 경로)는 이번 작업의 변경 범위 밖이라 다시 읽지 않았다.
- 대조 결과: 1절 맡는 책임·맡지 않는 책임(4.3 항목 포함), 2절 요소 표, 4절 5단계가 구현과 맞다. 4.2 시점 설명(`/api/v1/`를 항상 401로 답함)은 4절 5단계와 8절에서 고쳤다.
- 작업 4.3 시점 상태: OPS-034 부분(운영 조회 API 4.4 전), OPS-014 부분(운영 조회·종료 API 4.4·4.5 전), OPS-006 부분(로그인 본문 읽기는 9절 11번과 같이 읽기마다 U를 다시 잰다), OPS-033 부분, MGT-015 미연결(작업 4.4).
- 환경: Windows 11 Pro 10.0.26200 x64, Visual Studio Build Tools 2022 17.14.37806.7, `AWS_LC_SYS_PREBUILT_NASM=1`; WSL2 Ubuntu 24.04.4, Rust 1.99.1.
- 검증 기록:

| 명령 | 환경 | 결과 |
| --- | --- | --- |
| `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | WSL2 Ubuntu 24.04.4 | 통과(`ALL PASSED`). 시나리오 148건: 통과 11, 부분 27, 실패 0, 미실행 0, 미연결 110. 오류 0건. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | Windows 11 Pro 10.0.26200 x64 | 통과(`ALL PASSED (all)`). 시나리오 148건: 통과 10, 부분 27, 실패 0, 미실행 1, 미연결 110. 오류 0건. Windows 미실행 1은 OPS-007(Unix 전용)이다. |

- 판정: 구조 시점 `Current`, 구현 대조 `Verified`(커밋 `59f3f62` + 작업 트리). 문서 상태 `Draft`는 그대로다. 9절 5번(표기)과 11번(keep-alive 절대 제한)은 남는다.

### 작업 4.2

- 대조 범위: 커밋 `ea024fc`(작업 4.2, 관리 TLS 핸드셰이크와 요청 머리 읽기에 U 적용) + 작업 트리 변경(이 문서, `docs/TODO.md`, `docs/contracts/config-v1.md`, `docs/architectures/code-map.md`).
- 대조 방법: 2절 요소 표의 파일·심볼을 grep으로 찾았다. `request_deadline.rs`의 `DeadlineListener`·`Deadlined`를 읽어 U 적용 순서를 4절 표와 맞췄다. 접속 수락부터 첫 요청 머리 끝까지는 절대 제한이고, 그 뒤에는 읽기·쓰기마다 다시 잰다. `management_http.rs` `answer`(3·4절 순서), `tls_listener.rs` `TlsIo::poll_handshake`·`offers_no_tls12`(버전 판정), `src/serve.rs` `listen_until_shutdown`·`stop_task`(바인드·준비됨·종료 순서)를 읽어 4절 표와 맞췄다. U 규칙은 `crates/adapters/tests/management.rs`의 `stalled_tls_handshake_is_closed_after_u_without_an_answer`, `stalled_request_frees_its_lm_place_after_u`, `idle_connection_after_a_reply_is_closed_after_u`로 시험했다. 세 시험은 U를 300 ms로 줄인 설정(`SHORT_UPGRADE`)을 쓴다.
- 환경: Windows 11 Pro 10.0.26200 x64, WSL2 Ubuntu 24.04(`VERSION_ID="24.04"`), rustc·cargo 1.99.1, `Cargo.lock`의 axum 0.8.9, rustls 0.23.46, tokio-rustls 0.26.6.

| 명령 | 환경 | 결과 |
| --- | --- | --- |
| `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | WSL2 Ubuntu 24.04 | 통과(`ALL PASSED`). 시나리오 148건: 통과 4, 부분 19, 실패 0, 미실행 0, 미연결 125. 오류 0건. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | Windows 11 Pro 10.0.26200 x64 | 통과(`ALL PASSED (all)`). 시나리오 148건: 통과 3, 부분 19, 실패 0, 미실행 1, 미연결 125. 오류 0건. |
| `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` (커밋 `e701997`, 테스트 추가 뒤) | WSL2 Ubuntu 24.04 | Main이 보고한 실행 결과(이 작업에서 다시 실행하지 않음): 시나리오 148건: 통과 11, 부분 20, 실패 0, 미실행 0, 미연결 117. |

- 대상 시나리오 상태(`ea024fc`, 두 환경 같음): OPS-006 부분(이번 작업: 관리 TLS 핸드셰이크·첫 요청 머리·유휴 U 시험), OPS-033 부분(이번 작업: 멈춘 요청의 Lm 자리 반환 시험 추가), OPS-014 부분, OPS-034 부분(운영 조회 API 작업 4.4 전까지 전체가 아니다), OPS-004 부분, OPS-036 부분(아래 `e701997` 행에서 통과로 바뀜), MGT-017 부분, MGT-015 미연결(작업 4.4 완료 조건).
- `e701997` 변화: OPS-036 통과. 시험은 `crates/adapters/tests/websocket.rs`의 `tls10_and_tls11_are_refused_and_tls12_and_tls13_succeed_on_wss_and_management`(`/// spec: operations.md:OPS-036`, 부분 표식 없음)다. TLS 1.0·1.1 거절과 TLS 1.2·1.3 성립을 WSS와 관리 HTTPS에서 함께 본다. Ubuntu 실행 결과만 있다(위 표). Windows 실행 결과는 이 문서에 없다. 이 문서의 나머지 상태는 `e701997`에서 다시 대조하지 않았다(미관찰).
- Windows 보고서의 미실행 1건은 OPS-007이다(`tests/serve.rs` `closes_and_logs_every_held_connection_when_stopping`이 이번 실행에 없음).
- 판정: 구조 대조를 마쳤다. 구조 시점은 `Current`, 구현 대조는 `Verified`(작업 트리 범위)다. 문서 상태는 `Draft`로 둔다(열린 질문 5·11이 남아 있다. 11은 사용자 결정이 필요하다).
- TODO 4.2는 `[ ]`로 둔다. MGT-015는 사용자 결정 2026-10-21(Q14)로 작업 4.4 완료 조건에 옮겼다. OPS-014 부분 사유: 운영 조회·종료 API가 아직 없다(작업 4.4·4.5). 공개 평문 주소 시작 거절(`tests/serve.rs` `refuses_plaintext_management_on_a_public_address_with_exit_code_3`), 계정 없음 시작 거절(`serve`), 인증서 누락 시작 거절(3.1 `serve`), 미인증 401(`management.rs`와 `answers_api_queries_with_401_without_login_on_loopback_plaintext`)은 확인했다. OPS-034 부분 사유: 운영 조회 엔드포인트(작업 4.4)가 아직 없어, 로그인 없이 `/api/v1/` 요청이 401로 거절되는 것만 확인했다. 비루프백 평문 시작 실패와 루프백 평문 시작은 serve 테스트로 확인했다.

## 9. 미결정 사항

| 항목 | 영향 | 확정 위치 |
| --- | --- | --- |
| 1. (해결) `POST /api/v1/session`(4.2) 응답은 4.2 시점의 401 `unauthenticated`였다. 작업 4.3이 로그인을 구현해 응답이 바뀌었다(관리 로그인과 세션 설계 4.1절). 사용자 결정 2026-10-21(Q15)로 승인됐던 값이다. | 로그인 전까지 응답 본문 | 사용자 결정 2026-10-21(Q15) |
| 2. (해결) `GET /login/`·`/login/<파일>`(4.2)의 404 `not_found`는 사용자 결정 2026-10-21(Q15)로 승인됐다. 정적 파일은 4.6에서 준다. | 로그인 화면이 4.6 전까지 없다 | 사용자 결정 2026-10-21(Q15) |
| 3. (해결) 핸드셰이크와 요청 읽기 제한 시간은 U(`[timeouts] tls_upgrade_secs`)를 쓴다. 사용자 결정 2026-10-21(Q12). 설정 계약 1.2절 U 행을 넓혔다. | 멈춘 핸드셰이크가 Lm 자리를 무기한 잡지 않는다 | 사용자 결정 2026-10-21(Q12), 설정 계약 1.2절 |
| 4. (해결) 관리 HTTPS의 ALPN은 쓰지 않는다. 사용자 결정 2026-10-21(Q15)로 승인됐다. | 클라이언트 협상 방식(HTTP/1.1만) | 사용자 결정 2026-10-21(Q15) |
| 5. IPv4 매핑 IPv6 주소(`::ffff:a.b.c.d`) 표기. 가정은 `PeerIp`(`peer_ip.rs`)가 IPv4로 바꾼 값을 쓴다. 로그인 제한과 로그가 같은 값을 쓴다. 계약 6절의 표기 항목은 아직 닫히지 않았다. | 관리 응답의 `address.ip` 표기 | 관리 API 계약 6절 열린 항목 |
| 6. (해결) 버전 거절이 아닌 TLS 실패(인증서 오류, 클라이언트 중단)는 기록하지 않는다. 사용자 결정 2026-10-21(Q13). 운영 정보 계약은 바뀌지 않는다. | 관리 TLS 실패의 감사 흔적이 없다 | 사용자 결정 2026-10-21(Q13) |
| 7. (해결) U의 관리 TLS 적용은 3번과 같은 사용자 결정 2026-10-21(Q12)이다. | 3번과 같음 | 사용자 결정 2026-10-21(Q12) |
| 8. (해결) MGT-015(상대 주소)는 TODO 4.4 완료 조건으로 옮긴다. TODO 4.2 완료 조건에서는 뺀다. 사용자 결정 2026-10-21(Q14). | TODO 4.2·4.4 완료 판정 | 사용자 결정 2026-10-21(Q14), `docs/TODO.md` |
| 9. (해결) 비루프백 평문 시작 실패(OPS-014·OPS-034)의 검증 위치. 서버 수준 테스트 2건을 추가했다. `tests/serve.rs`의 `refuses_plaintext_management_on_a_public_address_with_exit_code_3`(시작 실패, OPS-014 부분·OPS-034 부분)와 `answers_api_queries_with_401_without_login_on_loopback_plaintext`(루프백 평문 시작 뒤 401, OPS-034 부분). 설정 규칙 테스트 `allows_plaintext_management_only_on_loopback`(OPS-034 부분)는 그대로 둔다. | 표식 위치만 바뀜(해결됨) | 구현 판단(서버 수준 테스트 추가, 해결됨) |
| 10. (해결) 첫 TLS 레코드(16 389바이트)만으로 버전을 판정하는 한계를 받아들인다. 사용자 결정 2026-10-21(Q17). 현재 코드가 이 옵션이다. | 관리 TLS 버전 거절의 감사 흔적. OPS-036 시험(`crates/adapters/tests/websocket.rs`의 `tls10_and_tls11_are_refused_and_tls12_and_tls13_succeed_on_wss_and_management`)은 첫 레코드 안의 거절 기록을 확인한다. 첫 레코드를 넘는 ClientHello의 기록은 시험하지 않는다. | 사용자 결정 2026-10-21(Q17) |
| 11. 첫 요청 머리 뒤 keep-alive 요청 머리에는 절대 제한이 없다. 구현 판단(사용자 결정 없음): 읽기·쓰기가 없는 동안 U만 잰다. 옵션은 (a) 요청마다 절대 제한을 다시 건다, (b) 관리 응답에 `Connection: close`를 붙인다(계약 변경), (c) 접속 수명 상한을 둔다. | 느린 keep-alive 요청이 U 간격마다 Lm 자리를 연장할 수 있다(OPS-006 부분) | 사용자 결정 필요 |
