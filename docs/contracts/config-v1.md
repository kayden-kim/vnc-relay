# 설정 파일 v1 계약

운영자가 쓰는 TOML 설정 파일의 키, 자료형, 기본값, 유효 범위, 항목 문법, 재로드 구분을 정한다. `vnc-relay serve`는 이 계약에 맞지 않는 파일로 시작하지 않으며, 실행 중에도 이 계약에 맞지 않는 재로드 후보를 적용하지 않는다. 이 문서는 파일 형식만 정한다. 시작·재로드의 외부 동작은 [운영 스펙](../specs/operations.md)이 정하고, 관리자 계정 파일과 하위 명령은 [CLI·계정 파일 계약](cli-v1.md)이 정한다.

이 계약은 2026-10-16에 사용자가 채택했다(상태 Active). 승인 표의 기본값·유효 범위, 추가로 승인한 값, 이 문서에서 정한 키 이름·테이블 구조·형식·오류 코드가 모두 채택 범위다. 6절에 "구현에서 확인"·"관찰로 확인 필요"로 남긴 항목은 채택으로 해소되지 않았다. 구현과 계약 테스트가 아직 없어 5절 테스트 결과는 모두 미실행이다.

| 항목 | 값 |
| --- | --- |
| 이름과 버전 | `vnc-relay-config` 1.0.0 |
| 상태 | Active(2026-10-16 채택) |
| 소유자 | vnc-relay 프로젝트 관리자 |
| 만드는 쪽(Producer) | 운영자(설정 파일 작성자) |
| 사용하는 쪽(Consumer) | `vnc-relay serve`(시작 검증과 자동 재로드). `vnc-relay admin` 하위 명령은 `[admin]` 테이블만 읽는다. |
| 방향 | 운영자가 쓴 파일을 서버가 읽는다. 서버는 파일을 쓰지 않는다. |
| 스키마 기준 파일 | 없음. 이 문서의 Markdown 표가 형식의 기준이다. |
| 기준 스펙 | [설정·프로세스 수명·운영 보안 스펙](../specs/operations.md) |
| 관련 요구사항 | [PRD](../requirements/PRD.md) FR-02·FR-05·FR-06·FR-07, [7절 보안 및 신뢰 경계](../requirements/PRD.md#7-보안-및-신뢰-경계) |
| 호환성 범위 | 설정 파일 v1. Windows 11 x64·Ubuntu 24.04 LTS x64의 `vnc-relay` 1.x. 파일 안에 형식 버전 키는 없다. |
| 관련 테스트 | [운영 스펙 수용 테스트](../specs/operations.md#7-수용-테스트) OPS-001~006·OPS-016·OPS-023~027·OPS-031·OPS-034~042, [WS·WSS 스펙](../specs/websocket.md#7-수용-테스트) WS-015·WS-022·WS-024·WS-027, [중계 스펙](../specs/relay.md#7-수용-테스트) RLY-007·RLY-008·RLY-022~027. 계약 테스트는 5절에 연결한다. 구현·테스트 파일이 없어 모두 미실행이다. |

용어는 [VNC Relay 용어집](../../GLOSSARY.md)을 따른다. 테스트·검증 기호 L~Dp 등은 [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)의 기호이며, 이 계약이 각 기호를 키에 연결한다.

관련 문서: [CLI·계정 파일 계약](cli-v1.md)(`[admin]` 테이블 사용, 종료 코드), [관리 API 계약](management-api-v1.md)(`[dashboard]`·`[recent_closed]`·`[management]`의 값 전달), [WebSocket 중계 계약](websocket-relay-v1.md)(`allowed_origins`·WS 키 사용), [Repeater 초기 교환 계약](repeater-handshake-v1.md)(`initial_message_bytes`·`handshake_secs` 사용), [운영 정보 계약](ops-info-v1.md)(경고·재로드 사건 이름, `[log]`·`[status]`), [ADR-0001 내장 TLS](../decisions/ADR-0001-built-in-tls.md), [ADR-0002 출발지 IP 허용 목록](../decisions/ADR-0002-source-ip-allowlist.md), [ADR-0004 tokio·rustls](../decisions/ADR-0004-tokio-rustls.md).

## 1. 형식과 의미

이 절의 표가 형식의 기준이다. TOML에는 `null`이 없다. "필수 아님"은 키를 생략할 수 있다는 뜻이다.

### 1.1 파일 규칙

| 항목 | 규칙 |
| --- | --- |
| 형식 | TOML. UTF-8 텍스트 한 파일. 파일 이름·확장자는 정하지 않는다. 경로는 `--config`로 받는다. |
| 시간 | 정수 초. 키 이름이 `_secs`로 끝난다. 분·시 단위가 자연스러운 값도 초로 적는다. |
| 크기 | 정수 바이트. 키 이름이 `_bytes`로 끝난다. `4KiB` 같은 단위 문자열은 받지 않는다. |
| 자료형 | 키 표의 자료형과 TOML 자료형이 정확히 같아야 한다. 정수 키에 문자열(`"10"`)이나 실수(`10.0`)를 쓰면 `config_type_mismatch`다. |
| 경로 값 | 문자열. 빈 문자열은 받지 않는다. 상대 경로는 실행 위치가 아니라 설정 파일이 있는 디렉터리를 기준으로 푼다. Windows 경로는 TOML 리터럴 문자열(작은따옴표)이나 `/`를 쓰기를 권장한다. 큰따옴표 문자열의 `\`는 이스케이프로 해석된다. |
| 주소 값 | `listen`은 `"IP:포트"`다. IPv6는 `"[IP]:포트"`다. 호스트 이름은 받지 않는다. 포트는 1~65535다. 포트 0은 받지 않는다. `relay.tcp_viewer`·`relay.tcp_server`의 `listen`만 포트를 생략할 수 있다(1.2절 진입점 기본 포트). |
| 알 수 없는 테이블·키 | 설정 오류 `config_unknown_key`다. 무시하거나 경고만 하지 않는다. 오타가 보안 경계를 조용히 끄는 일을 막기 위해서다. |
| 중복 키·구문 오류 | TOML 구문 오류 `config_syntax_error`다. |
| 기본값 | 키를 생략하면 아래 표의 기본값을 쓴다. 보안 경계 키에는 기본 허용 값을 두지 않는다. 필수 키는 기본값이 없다. |
| 비밀 | 설정 파일에는 비밀번호·해시·개인키 내용을 넣지 않는다. 개인키는 `key_file`이 가리키는 별도 파일에 둔다. |
| 환경 변수·명령행 덮어쓰기 | 없다. 설정 값은 파일에서만 받는다. |

### 1.2 키 표

기호는 [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)의 검증 기호다. "재로드"는 1.8절의 구분이다. 표에 "재로드: 대상"이 없는 키는 모두 재시작이 필요하다. 다른 계약은 이 표의 수치를 반복하지 않고 키 이름과 이 문서 링크로 인용한다.

#### `[admin]`

| 키(기호) | 자료형 | 필수 여부 | 기본값 | 유효 범위·의미 |
| --- | --- | --- | --- | --- |
| `accounts_file` | 문자열(경로) | 필수 | 없음 | 관리자 계정 파일 경로. 형식은 [CLI·계정 파일 계약](cli-v1.md)이 정한다. `vnc-relay admin` 하위 명령과 `serve`가 같은 경로를 쓴다. |
| `argon2.memory_kib`(Argon2id 메모리) | 정수 | 아님 | 19456 | 19456~1048576. 새 해시를 만들 때 쓴다. 19456 미만은 거절한다. |
| `argon2.iterations`(Argon2id 반복) | 정수 | 아님 | 2 | 2~10. |
| `argon2.parallelism`(Argon2id 병렬) | 정수 | 아님 | 1 | 1~16. |

`[admin.argon2]`는 테이블 이름이다. 값 19456 KiB는 19 MiB다. 승인된 하한 미만은 이 설정에서도, 계정 파일에 저장된 해시에서도 거절한다([CLI·계정 파일 계약 1.4절](cli-v1.md#14-해시-문자열-형식)).

#### 진입점: `[relay.tcp_viewer]`, `[relay.tcp_server]`, `[relay.ws_viewer]`, `[relay.wss_viewer]`

네 테이블은 같은 키를 가진다. 기본 포트는 `tcp_viewer`·`tcp_server`에만 있다. 기본 포트와 주소만 쓰는 `listen` 형식은 표 아래에서 정한다. `ws_viewer`·`wss_viewer`는 포트까지 필수다.

| 키 | 자료형 | 필수 여부 | 기본값 | 유효 범위·의미 | 재로드 |
| --- | --- | --- | --- | --- | --- |
| `enabled` | 불리언 | 아님 | `false` | `true`일 때만 리스너를 연다. 네 진입점 모두 기본은 `false`다. | 재시작 |
| `listen` | 문자열(주소) | `enabled = true`이면 필수 | 없음 | `"IP:포트"`(1.1절). `tcp_viewer`·`tcp_server`는 주소만 써도 된다(아래 기본 포트). | 재시작 |
| `allow` | 문자열 배열 | `enabled = true`이면 필수 | 없음 | 출발지 IP 항목 배열(1.3절). 빈 배열 `[]`은 모든 출발지 거절. 생략은 시작 실패. | 대상 |
| `allowed_origins` | 문자열 배열 | 아님 | `[]` | `ws_viewer`·`wss_viewer`만 가진다. Origin 허용 값 배열(1.4절). 빈 배열은 Origin 헤더가 있는 요청을 모두 거절한다. Origin 헤더가 없는 요청은 이 키와 무관하게 IP 검사만 한다. | 재시작 |
| `cert_file` | 문자열(경로) | `wss_viewer`에서 `enabled = true`이면 필수 | 없음 | `wss_viewer`만 가진다. PEM 인증서 체인 파일(1.9절). | 대상(경로와 파일 내용) |
| `key_file` | 문자열(경로) | `wss_viewer`에서 `enabled = true`이면 필수 | 없음 | `wss_viewer`만 가진다. PEM 개인키 파일(1.9절). | 대상(경로와 파일 내용) |

`tcp_viewer`·`tcp_server`에 `allowed_origins`·`cert_file`·`key_file`을 쓰면 `config_unknown_key`다. `ws_viewer`에 `cert_file`·`key_file`을 써도 `config_unknown_key`다. 진입점이 꺼져 있어도 키는 형식·범위 검사를 받는다. 꺼진 진입점은 필수 키 검사를 받지 않는다.

기본 포트는 UltraVNC Repeater의 기본 포트를 따른다(사용자 승인, 2026-10-16). 공식 안내 [Repeater 안내 PDF](https://www.uvnc.eu/download/1210/Repeater.pdf)는 "Default VNC ports 5901, 5500"이라 적는다. 같은 문서에서 Server는 `-connect www.uvnc.com:5500`으로, Viewer는 "viewer port 5901"로 Repeater에 접속한다.

| 진입점 | 기본 포트 | 근거 |
| --- | --- | --- |
| `relay.tcp_viewer` | 5901 | 위 문서의 Viewer 접속 포트 |
| `relay.tcp_server` | 5500 | 위 문서의 Server `-connect` 포트 |
| `relay.ws_viewer`·`relay.wss_viewer` | 없음. 포트 필수 | UltraVNC에 대응하는 포트가 없다. |

1. 주소에는 기본값이 없다. `listen`을 생략하거나 임의의 공개 주소로 대체하지 않는다([운영 스펙 3.1절](../specs/operations.md#31-설정-검증과-시작)). 켠 진입점의 `listen`은 여전히 필수다.
2. `tcp_viewer`·`tcp_server`의 `listen`은 주소만 쓰면 기본 포트를 붙인다. IPv4는 `"203.0.113.10"`(`tcp_viewer`면 5901, `tcp_server`면 5500), IPv6는 대괄호로 묶는다(`"[2001:db8::1]"`). 대괄호가 없는 IPv6는 `config_listen_invalid`다.
3. 포트를 쓰면 쓴 포트를 쓴다.
4. `ws_viewer`·`wss_viewer`·`management.listen`·`status.listen`은 주소만 쓰면 `config_listen_invalid`다. UltraVNC에 대응하는 포트가 없어 기본 포트를 두지 않는다.
5. 리스너 주소 중복 검사(1.6절)는 기본 포트를 붙인 뒤의 값에 적용한다.
6. Repeater의 웹 GUI 기본 포트 80은 이 제품의 관리 HTTPS와 역할이 달라 적용하지 않는다.

#### `[mode1]`, `[mode2]`

| 키 | 자료형 | 필수 여부 | 기본값 | 유효 범위·의미 | 재로드 |
| --- | --- | --- | --- | --- | --- |
| `mode1.enabled` | 불리언 | 아님 | `false` | Mode I 요청을 받는다. 꺼져 있으면 대기 등록·대상 연결 전에 거절한다([RLY-R17](../specs/relay.md#21-판정-가능한-규칙)). | 재시작 |
| `mode1.allow_targets` | 문자열 배열 | `mode1.enabled = true`이면 필수 | 없음 | 대상 허용 항목 배열(1.5절). 빈 배열은 모든 대상 거절. 생략은 시작 실패. | 재시작 |
| `mode2.enabled` | 불리언 | 아님 | `false` | Mode II 요청을 받는다. | 재시작 |

#### `[limits]`

| 키(기호) | 자료형 | 필수 여부 | 기본값 | 유효 범위 | 의미 |
| --- | --- | --- | --- | --- | --- |
| `relay_connections`(L) | 정수 | 아님 | 1000 | 2~65535 | 중계 측 연결 소켓 상한. 집계 범위는 [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)의 L이다. |
| `admin_connections`(Lm) | 정수 | 아님 | 64 | 1~1024 | 관리·상태 리스너 연결 소켓 상한. L과 독립이다. |
| `waiting`(Lw) | 정수 | 아님 | 200 | 1~`relay_connections` | Mode II 대기 연결 상한. L도 함께 적용한다. |
| `initial_message_bytes`(I) | 정수(바이트) | 아님 | 4096 (4 KiB) | 256~65536 (256 B~64 KiB) | Repeater 초기 메시지 크기 상한. |
| `buffer_bytes`(B) | 정수(바이트) | 아님 | 131072 (128 KiB) | 16384~4194304 (16 KiB~4 MiB) | 연결당 중계 버퍼 상한. |
| `ws_message_bytes`(M) | 정수(바이트) | 아님 | 1048576 (1 MiB) | 65536~16777216 (64 KiB~16 MiB) | WebSocket 논리 메시지 크기 상한. 프레임 분할 전후에 같은 제한이다. |

모든 `[limits]` 키는 재시작이 필요하다.

#### `[timeouts]`

| 키(기호) | 자료형 | 필수 여부 | 기본값 | 유효 범위 | 의미 |
| --- | --- | --- | --- | --- | --- |
| `handshake_secs`(H) | 정수(초) | 아님 | 10 | 1~120 | Repeater 초기 교환 제한 시간. 초기 교환 상태에 들어간 때부터 잰다. |
| `tls_upgrade_secs`(U) | 정수(초) | 아님 | 10 | 1~120 | WSS의 TLS 협상과 WS·WSS의 HTTP 업그레이드를 합친 제한 시간. 접속 수락 때부터 잰다. |
| `wait_secs`(W) | 정수(초) | 아님 | 3600 | 10~86400 | Mode II 상대 대기 제한 시간. 유효한 대기 등록부터 매칭까지다. |
| `connect_secs`(C) | 정수(초) | 아님 | 10 | 1~120 | Mode I 대상 연결 제한 시간. 주소 해석 시작부터 TCP 연결 성립까지를 하나로 잰다. |
| `close_secs`(E) | 정수(초) | 아님 | 10 | 1~120 | EOF 이후 반대 방향 전달과 종료 교환(WebSocket close 포함)을 기다리는 제한 시간. |
| `shutdown_grace_secs`(G) | 정수(초) | 아님 | 30 | 0~600 | 정상 종료 때 활성 세션에 주는 유예 시간. 0은 유예 없이 바로 정리한다. |

모든 `[timeouts]` 키는 재시작이 필요하다.

#### `[keepalive]`

| 키(기호) | 자료형 | 필수 여부 | 기본값 | 유효 범위 | 의미 |
| --- | --- | --- | --- | --- | --- |
| `tcp_idle_secs`(K 유휴) | 정수(초) | 아님 | 60 | 10~3600 | TCP keepalive 첫 탐색 전의 유휴 시간. |
| `tcp_interval_secs`(K 탐색 간격) | 정수(초) | 아님 | 10 | 1~300 | 탐색 사이의 간격. |
| `tcp_probes`(K 탐색 횟수) | 정수 | 아님 | 6 | 1~20 | 응답 없는 탐색을 몇 번 보낸 뒤 끊긴 연결로 볼지 정한다. |
| `ws_ping_secs`(Kw ping) | 정수(초) | 아님 | 30 | 5~300 | WS·WSS 연결에 보내는 ping 주기. |
| `ws_pong_timeout_secs`(Kw pong 제한) | 정수(초) | 아님 | 30 | 5~300 | ping 뒤 pong을 기다리는 제한 시간. |

K의 감지 시간은 `tcp_idle_secs + tcp_interval_secs × tcp_probes`다. 기본값은 120초다. 중계 측 TCP 소켓 전체에 적용한다(WS·WSS 하부 TCP 포함). 모든 `[keepalive]` 키는 재시작이 필요하다.

#### `[management]`, `[management.login_limit]`, `[management.session]`

| 키(기호) | 자료형 | 필수 여부 | 기본값 | 유효 범위·의미 |
| --- | --- | --- | --- | --- |
| `management.listen` | 문자열(주소) | 필수 | 없음 | 관리 리스너 주소(1.1절). 포트까지 필수다. 루프백도 쓸 수 있다. |
| `management.cert_file` | 문자열(경로) | 아님. `key_file`과 함께 쓴다. | 없음 | PEM 인증서 체인(1.9절). 두 키를 모두 쓰면 HTTPS, 모두 생략하면 평문 HTTP다. 평문 HTTP는 루프백 주소에서만 허용한다. |
| `management.key_file` | 문자열(경로) | 아님. `cert_file`과 함께 쓴다. | 없음 | PEM 개인키(1.9절). |
| `management.login_limit.per_account`(F 계정별 횟수) | 정수 | 아님 | 5 | 1~100. 계정별로 `window_secs` 안에 센 로그인 실패 횟수 기준. |
| `management.login_limit.per_source`(F 출발지별 횟수) | 정수 | 아님 | 20 | 1~100. 출발지 IP별로 `window_secs` 안에 센 실패 횟수 기준. |
| `management.login_limit.window_secs`(F 집계 시간) | 정수(초) | 아님 | 900 (15분) | 60~86400 (1~1440분). 실패를 세는 시간 창. |
| `management.login_limit.lockout_secs`(F 제한 시간) | 정수(초) | 아님 | 900 (15분) | 60~86400 (1~1440분). 기준을 넘은 뒤 로그인을 거절하는 시간. |
| `management.session.idle_secs`(Si) | 정수(초) | 아님 | 1800 (30분) | 300~28800 (5~480분). 관리 세션 유휴 만료 시간. 사용자 행동 요청만 연장한다. |
| `management.session.max_lifetime_secs`(Sa) | 정수(초) | 아님 | 43200 (12시간) | 3600~259200 (1~72시간). 관리 세션 절대 최대 수명. |

`management.cert_file`·`key_file`이 있으면 두 경로 값과 가리키는 파일 내용은 재로드 대상이다. HTTP·HTTPS 전환(두 키를 쓰는지 여부), `management.listen`, 로그인 제한, 세션 키는 재시작이 필요하다. 로그인 제한 응답은 [관리 스펙 3.0절](../specs/management.md#30-관리자-로그인과-세션)이, 쿠키·교환 형식은 관리 API 계약이 정한다.

#### `[dashboard]`, `[recent_closed]`, `[status]`, `[log]`, `[reload]`

| 키(기호) | 자료형 | 필수 여부 | 기본값 | 유효 범위·의미 |
| --- | --- | --- | --- | --- |
| `dashboard.poll_secs`(P) | 정수(초) | 아님 | 2 | 1~60. 대시보드 폴링 주기. 화면은 서버의 값을 따른다. |
| `dashboard.query_timeout_secs` | 정수(초) | 아님 | 5 | 1~60. 폴링 조회의 제한 시간. |
| `recent_closed.max_entries`(R) | 정수 | 아님 | 100 | 0~10000. 최근 종료 목록의 최대 항목 수. 0이면 목록을 보관하지 않는다. |
| `status.listen` | 문자열(주소) | 필수 | 없음 | 상태 리스너 주소(1.1절). 포트까지 필수다. 루프백을 쓰기를 권장한다. 루프백이 아니면 경고 `status_listen_not_loopback`을 기록한다. |
| `log.output` | 문자열 | 아님 | `"stdout"` | `"stdout"`(표준 출력) 또는 로그 파일 경로. 파일이면 이어 쓴다. 파일이 없으면 만든다. 이름이 `stdout`인 파일은 `"./stdout"`으로 쓴다. 교체·보관은 운영자 책임이다. |
| `reload.settle_secs`(D) | 정수(초) | 아님 | 5 | 1~60. 마지막 변경을 감지한 뒤 적용·실패 결과를 확인할 때까지의 최대 시간. |
| `reload.poll_secs`(Dp) | 정수(초) | 아님 | 30 | 5~600. OS 파일 알림을 놓친 변경을 찾는 주기 확인 간격. |

`[dashboard]`·`[recent_closed]`·`[status]`·`[log]`·`[reload]`의 모든 키는 재시작이 필요하다. `status.listen`은 UltraVNC에 대응하는 기본 포트가 없어 포트까지 필수로 둔다. D의 이름 `settle_secs`는 "안정될 때까지 기다리는 시간"으로 읽을 수 있다. 이 계약에서 D는 [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)의 정의대로 결과 확인 제한 시간이다. 변경 안정화 대기는 D 안에서 구현이 정한다.

### 1.3 IP 항목 문법

`allow` 배열의 각 문자열은 항목 하나다. 한 문자열에 여러 항목을 넣을 수 없다. 공백은 어디에도 허용하지 않는다. 빈 문자열은 잘못된 표기다. IPv4와 IPv6를 모두 쓴다.

| 형태 | 문법 | 규칙 | 예 |
| --- | --- | --- | --- |
| 단일 IPv4 | 십진수 4개를 점으로 이은 표기 | 각 자리 0~255. 앞자리 0(`01`)은 받지 않는다. | `198.51.100.7` |
| 단일 IPv6 | RFC 4291 텍스트 표기 | 압축(`::`)과 대소문자 혼용을 받는다. 대괄호·영역 ID(`%eth0`)·점 표기 IPv4는 받지 않는다. | `2001:db8::1` |
| CIDR | `주소/접두사` | 접두사는 십진수. IPv4 0~32, IPv6 0~128. 주소의 호스트 비트가 모두 0이어야 한다. | `198.51.100.0/24`, `2001:db8::/32` |
| 범위 | `주소-주소` | 두 주소는 같은 체계다. 시작 ≤ 끝이다. 양 끝을 모두 포함한다. | `203.0.113.10-203.0.113.20` |

다음은 설정 오류다.

| 조건 | 오류 코드 |
| --- | --- |
| 표기 오류(위 문법에 맞지 않음, 공백, 빈 문자열, 와일드카드, 접두사 범위 초과, 호스트 비트가 0이 아닌 CIDR) | `config_ip_item_invalid` |
| 범위의 시작이 끝보다 큼 | `config_ip_range_reversed` |
| 범위의 두 주소가 서로 다른 체계(IPv4와 IPv6) | `config_ip_range_family_mixed` |

항목의 주소가 `::ffff:0:0/96`(IPv4-mapped IPv6) 안에 있으면 `config_ip_item_invalid`다. 서버는 이 대역의 상대 주소를 IPv4 주소로 바꿔 비교한다. 겹치거나 중복된 항목은 오류도 경고도 아니다. 상대 IP가 항목 중 하나라도 일치하면 허용이다.

IPv4 또는 IPv6 한 체계의 전체 주소를 덮는 항목은 허용하며 경고 `allow_all_source`를 시작·재로드 때 기록한다. 표기와 관계없이 항목 하나가 그 체계의 전체 주소를 덮으면 해당한다. `0.0.0.0/0`, `::/0`이 해당하고, 같은 효과를 내는 범위(`0.0.0.0-255.255.255.255`, `::-ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff`)도 해당한다. 서로 다른 여러 항목을 합쳐야 전체가 되는 경우(예: `0.0.0.0/1`과 `128.0.0.0/1`)는 판정하지 않는다. 합집합 계산을 요구하지 않기 위해서이며, 범위가 넓은 목록은 운영자가 직접 확인한다.

### 1.4 Origin 값 문법과 비교

`allowed_origins`의 각 문자열은 브라우저가 보내는 `Origin` 헤더와 비교할 값 하나다. 와일드카드는 없다.

| 부분 | 규칙 |
| --- | --- |
| 형식 | `scheme://host[:port]`. 경로·쿼리·조각·사용자 정보·끝의 `/`·공백은 받지 않는다. |
| `scheme` | `http` 또는 `https`. 대소문자를 구분하지 않고 받는다. |
| `host` | ASCII 도메인 이름, IPv4, 또는 대괄호로 묶은 IPv6(`[2001:db8::1]`). 한글 등 비ASCII 도메인은 punycode로 적는다. `*` 같은 와일드카드는 받지 않는다. |
| `port` | 생략하거나 1~65535의 십진수. |
| 오류 | 위 규칙에 맞지 않으면 `config_origin_invalid`. 값 `null`도 받지 않는다. |

비교 규칙. 이 절이 Origin 값 문법과 정규화의 유일한 기준이며 [WebSocket 중계 계약 1.3절](websocket-relay-v1.md#13-origin-비교)은 이 규칙을 인용한다. 서버는 허용 값과 요청의 `Origin` 헤더 값을 같은 방식으로 정규화한 뒤 비교한다.

1. 스킴과 호스트는 ASCII 소문자로 바꿔 비교한다. 요청 값에 비ASCII 문자가 있으면 일치하지 않는다.
2. 기본 포트(http 80, https 443)를 생략한 표기는 그 포트를 명시한 표기와 같은 값이다. 그 밖의 포트는 숫자가 같을 때만 일치한다.
3. 정규화한 스킴·호스트·포트가 모두 같을 때만 일치한다. 하위 도메인은 다른 호스트다.
4. 호스트가 IPv6이면 대괄호를 포함한 소문자 문자열로 비교한다. 주소를 풀어 쓰거나 줄여 쓴 표기를 같은 값으로 보지 않는다.
5. 요청 값의 스킴이 `http`·`https`가 아니면(`null` 포함) 일치하지 않는다. 요청 값에 경로·쿼리·후행 슬래시·사용자 정보가 있으면 일치하지 않는다.
6. 정규화한 값이 같은 항목이 둘 이상이어도 오류가 아니다.
7. 요청에 `Origin` 헤더가 없으면 이 비교를 하지 않고 IP 검사만 한다([WS 스펙 2절](../specs/websocket.md#2-상태와-리스너-책임)). 요청에 `Origin` 헤더가 둘 이상이면 비교하지 않고 요청 오류다(HTTP 상태는 WebSocket 중계 계약이 정한다).

### 1.5 Mode I 대상 허용 항목 문법

`mode1.allow_targets`의 각 문자열은 `대상:포트` 항목이다. 이 정책은 출발지 IP 허용 목록과 별개다.

| 부분 | 문법 | 규칙 |
| --- | --- | --- |
| 대상 | IP 항목(1.3절) 또는 DNS 이름 | IPv6 항목(단일·CIDR·범위)은 대괄호로 묶는다. |
| DNS 이름 | 점으로 구분한 라벨. ASCII 영문·숫자·하이픈 | 라벨 1~63자, 전체 253자 이하. 하이픈으로 시작·끝나지 않는다. 끝의 점은 지운다. 대소문자를 구분하지 않는다. 와일드카드·비ASCII는 받지 않는다. |
| 포트 | `포트` 또는 `시작-끝` | 각 포트 1~65535. 시작 ≤ 끝. 생략·`*`는 받지 않는다. |

구분은 다음 순서로 한다.

1. 문자열이 `[`로 시작하면 짝이 되는 `]`까지가 대상이고, 바로 뒤에 `:포트`가 와야 한다.
2. `[`로 시작하지 않으면 `:`는 정확히 하나여야 한다. 앞이 대상, 뒤가 포트다.
3. 대상에 `/`가 있거나, 숫자·점·하이픈만으로 이루어졌거나, `:`가 있으면 IP 항목이다. 그 밖은 DNS 이름이다. 숫자·점만으로 된 이름 모양이 IPv4 문법에 맞지 않으면 `config_target_item_invalid`다.

예: `192.0.2.20:5900`, `192.0.2.0/24:5900-5905`, `[2001:db8::/32]:5900`, `[2001:db8::1-2001:db8::9]:5900-5901`, `pc1.example:5900`.

표기 오류, 시작이 끝보다 큰 포트 범위, IP 항목의 오류(역순 범위·체계 혼합)는 모두 `config_target_item_invalid`다. 메시지에 세부 원인을 적는다.

판정은 최종 연결 IP에도 적용한다([PRD FR-02](../requirements/PRD.md#fr-02-mode-i--대상-지정-중계), [중계 스펙 3.2절](../specs/relay.md#32-mode-i)). 판정 순서는 다음과 같다.

| Viewer가 지정한 대상 | 허용 조건 |
| --- | --- |
| IP | 그 IP와 포트가 IP 항목 중 하나와 일치한다. |
| DNS 이름 | (1) 이름과 포트가 이름 항목 중 하나와 일치하고, (2) 주소 해석 결과 중 실제로 연결할 IP와 포트가 IP 항목 중 하나와 일치한다. |

이름 항목은 이름만 허용한다. 이름이 어떤 IP로 해석되든 IP 항목에 없으면 연결하지 않는다. 해석 결과 중 IP 항목에 맞지 않는 주소는 시도하지 않고, 맞는 주소가 없으면 거절한다. 검사한 IP가 아닌 주소로는 연결하지 않는다. 이름 항목이 없으면 이름으로 지정한 대상은 모두 거절한다. 빈 배열은 모든 대상을 거절한다.

대상의 IP 항목이 포트와 관계없이 한 체계의 전체 주소를 덮으면(판정은 1.3절 규칙과 같다) 허용하되 경고 `allow_all_target`을 시작 때 기록한다. 임의 대상에 접속하는 공개 프록시가 되지 않도록 운영자가 항목을 좁혀야 한다.

### 1.6 값 관계 검사

시작과 재로드 후보 검증에서 모두 검사한다. 위반은 `config_relation_violated` 또는 지정한 코드다.

| 관계 | 조건 | 위반 코드 |
| --- | --- | --- |
| Lw ≤ L | `limits.waiting` ≤ `limits.relay_connections` | `config_value_out_of_range`(범위의 상한이 L이다) |
| Si ≤ Sa | `management.session.idle_secs` ≤ `management.session.max_lifetime_secs` | `config_relation_violated` |
| I ≤ B | `limits.initial_message_bytes` ≤ `limits.buffer_bytes` | `config_relation_violated` |
| 평문 관리 리스너는 루프백만 | `management.cert_file`·`key_file`이 모두 없으면 `management.listen`의 IP가 루프백(`127.0.0.0/8` 또는 `::1`)이어야 한다. `0.0.0.0`·`::`는 루프백이 아니다. | `config_plaintext_management_not_loopback` |
| 관리 TLS 파일 쌍 | `management.cert_file`과 `management.key_file`은 둘 다 쓰거나 둘 다 생략한다. | `config_tls_pair_incomplete` |
| WSS 파일 필수 | `relay.wss_viewer.enabled = true`이면 `cert_file`·`key_file`이 모두 있어야 한다. | `config_missing_key` |
| 리스너 주소 중복 금지 | 켠 진입점, `management.listen`, `status.listen` 중 두 곳의 `IP:포트`가 완전히 같으면 안 된다. `tcp_viewer`·`tcp_server`는 기본 포트를 붙인 값으로 비교한다. 와일드카드와 구체 주소가 겹치는 경우는 OS가 바인드 때 알려 준다. | `config_listen_conflict` |
| Argon2id 하한 | `admin.argon2` 값이 하한(19456·2·1) 미만이면 안 된다. | `config_value_out_of_range` |

### 1.7 시작 때 필수 키

| 조건 | 필수 키 | 누락 때 결과 |
| --- | --- | --- |
| 항상 | `admin.accounts_file`, `management.listen`, `status.listen` | `config_missing_key`, 시작 실패 |
| 진입점 `enabled = true` | 그 테이블의 `listen`, `allow` | `config_missing_key`, 시작 실패 |
| `relay.wss_viewer.enabled = true` | `cert_file`, `key_file` | `config_missing_key`, 시작 실패 |
| `mode1.enabled = true` | `mode1.allow_targets` | `config_missing_key`, 시작 실패 |

`config_missing_key`의 오류는 키의 전체 경로를 알려 준다. 준비된 관리자 계정이 없는 경우는 설정 오류가 아니라 계정 파일 오류다([CLI·계정 파일 계약 2.2절](cli-v1.md#22-오류-코드)). 켠 진입점의 허용 목록 생략을 공개 허용으로 바꾸지 않는다.

### 1.8 재로드 구분

[운영 스펙 3.3절](../specs/operations.md#33-파일-변경-자동-감지와-부분-재로드)을 키에 적용한 표다. 목록에 없는 키는 모두 "재시작 필요"다.

| 재로드 대상 | 적용 시점 |
| --- | --- |
| `relay.tcp_viewer.allow` | 새 TCP Viewer 접속부터 |
| `relay.tcp_server.allow` | 새 TCP Server 접속부터 |
| `relay.ws_viewer.allow` | 새 WS 접속부터(수락 직후와 업그레이드 직전 검사) |
| `relay.wss_viewer.allow` | 새 WSS 접속부터(수락 직후와 업그레이드 직전 검사) |
| `relay.wss_viewer.cert_file`·`key_file`의 경로 값과 가리키는 파일 내용 | 새 TLS 핸드셰이크부터. 기존 TLS 연결은 유지한다. |
| `management.cert_file`·`key_file`의 경로 값과 가리키는 파일 내용 | 새 TLS 핸드셰이크부터. 기존 TLS 연결은 유지한다. |

재시작이 필요한 변경에는 다음이 포함된다: 진입점·모드의 `enabled`, 모든 `listen`, `allowed_origins`, `mode1.allow_targets`, `admin.accounts_file`, `admin.argon2`, `[limits]`, `[timeouts]`, `[keepalive]`, `[management]`의 나머지 키(HTTP·HTTPS 전환 포함), `[dashboard]`, `[recent_closed]`, `[status]`, `[log]`, `[reload]`.

규칙:

1. 감시 대상은 설정 파일과 `cert_file`·`key_file`이 가리키는 파일이다. 계정 파일은 감시하지 않는다([CLI·계정 파일 계약 1.7절](cli-v1.md#17-서버-실행-중-계정-파일-변경)).
2. 후보는 파일을 읽어 기본값을 채운 유효 설정이다. 설정 값이 같으면 주석·공백·키 순서·기본값 명시 여부가 달라도 차이가 없다.
3. 후보 검증은 이 문서 1.1~1.7절을 모두 포함한다. 알 수 없는 키와 모든 필수 키 검사도 같다. 인증서·개인키 쌍은 개인키가 인증서에 맞아야 한다. 한쪽 파일만 바뀌어 맞지 않으면 후보는 유효하지 않다.
4. 재로드 대상이 아닌 값이 실행 중 값과 다르면 후보 전체를 `reload_restart_required`로 거절한다. 그 차이가 파일에 남아 있는 동안 이후 IP·인증서 변경도 모두 거절한다.
5. 후보가 유효하고 차이가 재로드 대상뿐이면 재로드 대상 값을 한 번에 교체한다. 일부만 적용하지 않는다.
6. 검증 실패는 `reload_rejected`다. 실패 원인 코드(2절)를 함께 알린다. 기존 설정·연결을 유지한다.
7. OS 파일 알림을 놓친 변경은 `reload.poll_secs`(Dp) 주기 확인으로 감지한다. 감시 자체가 실패하면 `reload_watch_failed`를 알리고 이전 유효 설정으로 계속 운영한다.

### 1.9 인증서·개인키 파일 형식

PEM을 쓴다. 파일 내용 검증은 서버가 한다.

| 키 | 형식 |
| --- | --- |
| `cert_file` | PEM 인코딩 X.509 인증서 한 개 이상. 첫 번째가 서버(리프) 인증서이고 뒤에 중간 인증서를 이어 붙인다. |
| `key_file` | 위 서버 인증서의 개인키 한 개를 담은 PEM. PKCS#8(`BEGIN PRIVATE KEY`)을 권장한다. 지원하는 키 종류(RSA·ECDSA 등)는 구현에서 확인한 범위로 확정한다. |

TLS 1.2 미만은 거절한다. 허용 암호 스위트는 선택한 rustls 암호 모듈(CryptoProvider)이 제공하는 rustls 기본 목록이다. 설정 키가 없으므로 운영자가 바꿀 수 없다. 스위트 이름은 모듈에 따라 달라지므로 이 계약이 나열하지 않는다. 암호 모듈 선택은 [OPS-TBD-009](../specs/operations.md#8-미결정-사항)에서 정한다. 개인키 파일은 서버를 실행하는 OS 계정만 읽도록 권한을 제한하기를 권장한다. 인증서·개인키 내용은 로그·오류 메시지에 쓰지 않는다.

## 2. 오류

### 2.1 설정 오류와 준비 실패

모든 진단은 `code`, `key`(점으로 이은 전체 경로. 배열은 0부터 센 `[i]` 첨자), `message`(사람이 읽는 문장)를 가진다. `code`와 `key`는 안정적인 값이다. `message`는 바뀔 수 있다. 진단에 비밀·파일 내용을 넣지 않는다. 시작에서는 가능한 모든 설정 오류를 모아 보고한다. 출력 형식은 [CLI·계정 파일 계약 1.5절](cli-v1.md#15-입력과-출력-규칙)이, 종료 코드는 [2.1절](cli-v1.md#21-종료-코드)이 정한다.

| 오류 코드 | 의미·형식 | 시작 | 재로드 후보 | 관련 스펙 시나리오 |
| --- | --- | --- | --- | --- |
| `config_file_unreadable` | 설정 파일이 없거나 읽을 수 없거나 UTF-8이 아니다. | 시작 실패 | 일시 소멸 포함, 후보 거절 | OPS-001, OPS-026 |
| `config_syntax_error` | TOML 구문 오류, 중복 키. | 시작 실패 | 후보 거절 | OPS-001, OPS-026 |
| `config_unknown_key` | 알 수 없는 테이블·키. | 시작 실패 | 후보 거절 | OPS-001 |
| `config_missing_key` | 필수 키 누락(1.7절). 켠 진입점의 `allow` 생략을 포함한다. | 시작 실패 | 후보 거절 | OPS-001, OPS-038 |
| `config_type_mismatch` | 키의 자료형이 다르다. | 시작 실패 | 후보 거절 | OPS-001 |
| `config_value_out_of_range` | 값이 유효 범위 밖이다. `waiting`이 L 초과인 경우 포함. | 시작 실패 | 후보 거절 | OPS-001, OPS-004~006 |
| `config_relation_violated` | 키 사이 관계(1.6절)를 어겼다. | 시작 실패 | 후보 거절 | OPS-001 |
| `config_listen_invalid` | `listen`이 1.1절의 주소 형식이 아니다. 주소만 쓸 수 없는 키에서 포트를 생략한 경우와 대괄호 없는 IPv6를 포함한다(1.2절). | 시작 실패 | 후보 거절 | OPS-001 |
| `config_listen_conflict` | 두 리스너의 주소·포트가 같다. | 시작 실패 | 후보 거절 | OPS-001 |
| `config_ip_item_invalid` | IP 항목 표기 오류(1.3절). | 시작 실패 | 후보 전체 거절 | OPS-040 |
| `config_ip_range_reversed` | 범위의 시작이 끝보다 크다. | 시작 실패 | 후보 전체 거절 | OPS-040 |
| `config_ip_range_family_mixed` | 범위의 두 주소가 IPv4와 IPv6로 다르다. | 시작 실패 | 후보 전체 거절 | OPS-040 |
| `config_origin_invalid` | Origin 값 표기 오류(1.4절). | 시작 실패 | 후보 거절 | WS-027 |
| `config_target_item_invalid` | Mode I 대상 항목 표기 오류(1.5절). | 시작 실패 | 후보 거절 | RLY-007, RLY-008 |
| `config_plaintext_management_not_loopback` | 평문 HTTP 관리 리스너가 루프백이 아닌 주소다. | 시작 실패 | 후보 거절 | OPS-014, OPS-034 |
| `config_tls_pair_incomplete` | `management.cert_file`·`key_file` 중 하나만 있다. | 시작 실패 | 후보 거절 | OPS-034 |
| `tls_file_invalid` | 인증서·개인키 경로·형식 오류. 파일이 없거나 읽을 수 없거나 PEM이 아니거나 서로 맞지 않는다. 시작 종료 코드는 설정 오류와 같은 3이다([CLI·계정 파일 계약 2.1절](cli-v1.md#21-종료-코드)). | 시작 실패(설정 오류와 같은 종료 코드) | 후보 거절, 기존 인증서 유지 | OPS-002, OPS-026 |

후보에 설정 오류가 있으면 어느 키의 오류든 후보 전체를 `reload_rejected`로 거절한다. 오류가 없고 재로드 대상이 아닌 값이 실행 중 값과 다르면 `reload_restart_required`다(1.8절).

### 2.2 경고

경고는 시작·재로드를 막지 않는다. 운영 로그의 경고로 기록한다([PRD FR-08](../requirements/PRD.md#fr-08-운영-정보)). 로그 사건 이름은 아래 경고 코드를 그대로 쓴다. 필드는 [운영 정보 계약 1.5절](ops-info-v1.md#15-사건-목록)이 정한다.

| 경고 코드 | 조건 | 관련 스펙 시나리오 |
| --- | --- | --- |
| `allow_all_source` | `allow`에 한 체계 전체를 덮는 항목(1.3절)이 있다. 시작·재로드 때. | OPS-038 |
| `plaintext_ws_enabled` | `relay.ws_viewer.enabled = true`. 시작 때. | OPS-016, WS-024 |
| `allow_all_target` | `mode1.allow_targets`에 한 체계 전체를 덮는 IP 항목이 있다. 시작 때. | RLY-007 |
| `status_listen_not_loopback` | `status.listen`이 루프백이 아니다. 시작 때. | OPS-032 |

### 2.3 재로드 결과

| 결과 코드 | 의미 | 관련 스펙 시나리오 |
| --- | --- | --- |
| `reload_applied` | 후보가 유효하고 재로드 대상 값을 적용했다. | OPS-024, OPS-025, OPS-041 |
| `reload_rejected` | 후보가 유효하지 않다. 원인 코드(2.1절)를 함께 알린다. 기존 설정·연결 유지. | OPS-026, OPS-027 |
| `reload_restart_required` | 재로드 대상이 아닌 값이 실행 중 값과 다르다. 후보 전체 미적용. | OPS-003, OPS-035 |
| `reload_watch_failed` | 변경 감지에 실패했다. 이전 유효 설정으로 계속 운영한다. 시작 때 감시를 준비할 수 없으면 시작 실패다. | OPS-028 |

## 3. 호환성 규칙

- 서버는 알 수 없는 테이블·키를 `config_unknown_key` 오류로 거절해야 한다. 허용 형식 밖의 값(예: 허용하지 않는 Origin 스킴)은 해당 값의 오류 코드(`config_origin_invalid` 등)로 거절해야 한다. 무시하거나 경고만 하지 않는다.
- 호환 변경(minor 올림): 기본값이 있고 생략해도 기존 동작이 같은 새 선택 키 추가, 유효 범위 넓힘, 오류 메시지 문구 변경. 새 키를 쓴 파일은 그 키를 모르는 이전 서버가 거절한다. 그래서 새 키를 쓰려면 서버를 먼저 올린다.
- 호환성을 깨는 변경(새 major 버전): 키 이름·자료형·단위 변경이나 삭제, 필수 키 추가, 기본값 변경, 유효 범위 좁힘, 재로드 구분 변경, 항목 문법 변경, 보안 검사 완화. 새 major는 `vnc-relay-config` 2.0.0으로 올리고 마이그레이션(v1 파일을 v2로 바꾸는 방법)과 롤백(v2 파일을 v1로 되돌리는 방법) 계획을 함께 작성한다. 지금은 v1만 있어 계획이 없다.
- 모든 값의 의미는 키 표에 있다. 키 순서와 주석은 의미가 없다.

## 4. 버전과 지원 단계

| 상태 | 조건 | 지원 종료·대체 |
| --- | --- | --- |
| Draft | 채택 전 상태. 이 계약은 2026-10-16에 채택해 Draft를 지났다. | 해당 없음 |
| Active | 현재. 사용자 채택(2026-10-16). 구현과 양 OS의 계약 테스트 통과·검증 기록은 5절 표가 관리하며, 통과하기 전에는 지원 완료로 표시하지 않는다. | v2가 Active가 된 뒤 Deprecated로 바꾼다. |
| Deprecated | 대체 계약이 Active이다. | 지원 종료 날짜는 대체 계약에서 정한다. |
| Retired | 지원 종료. | 새 구현의 근거로 쓰지 않는다. |

## 5. 예제와 계약 테스트

예제의 포트는 테스트용 임의값이며 기본값이 아니다. IP는 문서용 대역이다. 비밀은 없다. 개인키 내용은 파일 경로로만 가리킨다. 기본 포트를 쓰는 형태는 `listen = "203.0.113.10"`(`tcp_viewer`는 5901, `tcp_server`는 5500)이다.

최소 예제(TCP Viewer·TCP Server·Mode II, 평문 루프백 관리 리스너):

```toml
[admin]
accounts_file = "admin-accounts.toml"

[relay.tcp_viewer]
enabled = true
listen = "0.0.0.0:15901"
allow = ["198.51.100.0/24"]

[relay.tcp_server]
enabled = true
listen = "0.0.0.0:15902"
allow = ["203.0.113.10-203.0.113.20", "2001:db8::/32"]

[mode2]
enabled = true

[management]
listen = "127.0.0.1:18080"

[status]
listen = "127.0.0.1:18081"
```

WSS·Mode I·관리 HTTPS를 더한 예제(생략한 키는 기본값):

```toml
[admin]
accounts_file = "admin-accounts.toml"

[relay.wss_viewer]
enabled = true
listen = "0.0.0.0:15903"
allow = ["198.51.100.0/24"]
allowed_origins = ["https://viewer.example", "https://viewer.example:8443"]
cert_file = "certs/relay.pem"
key_file = "certs/relay.key"

[mode1]
enabled = true
allow_targets = ["192.0.2.0/24:5900-5905", "[2001:db8::/32]:5900", "pc1.example:5900"]

[mode2]
enabled = true

[management]
listen = "0.0.0.0:18443"
cert_file = "certs/admin.pem"
key_file = "certs/admin.key"

[status]
listen = "127.0.0.1:18081"

[log]
output = "stdout"
```

두 번째 예제의 `192.0.2.0/24:5900-5905`와 `[2001:db8::/32]:5900`는 최종 연결 IP 항목이다. `pc1.example:5900`은 이름 항목이다. `pc1.example`이 이 두 IP 항목 밖의 주소로 해석되면 연결하지 않는다.

결과와 증거는 [검증 기록](../AGENTS.md#검증-기록)을 따른다. 스펙과 같은 테스트는 해당 시나리오의 기록을 연결한다. 설정 로더의 테스트는 `crates/adapters/tests/config_contract.rs`(아래 표에서 "계약 테스트 파일")와 domain 단위 테스트에 있다. 실행 기록은 표 아래 [계약 테스트 실행 기록](#계약-테스트-실행-기록)에 있다. 아직 기능이 없는 규칙은 미실행이다.

| 주체·테스트 위치 | 확인할 계약 규칙 | 결과 | 실행 기록 |
| --- | --- | --- | --- |
| Consumer·계약 테스트 파일 `reports_every_problem_in_one_pass`, `rejects_syntax_errors_and_duplicate_keys_with_one_diagnostic`, `rejects_missing_unreadable_and_non_utf8_files`, `rejects_non_table_values_in_place_of_tables` | 구문 오류, 알 수 없는 키, 자료형 오류, 필수 키 누락(1.1·1.7절)을 모두 모아 진단으로 돌려준다. [OPS-001](../specs/operations.md#7-수용-테스트)의 설정 검증 부분 | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·`serve` 시스템 테스트(미생성, 작업 2.2) | 위 진단이 시작 실패와 비밀 없는 원인 표시로 이어진다. [OPS-001](../specs/operations.md#7-수용-테스트) | 미실행 | `serve` 미구현 |
| Consumer·계약 테스트 파일 `accepts_minimal_example_and_fills_every_default`, `checks_every_numeric_key_at_its_bounds`, `checks_relations_between_values` | 승인 수치의 기본값, 최소·최대와 그 바깥(1.2절). Lw ≤ L, Si ≤ Sa, I ≤ B(1.6절). 수치가 각 기능에 주는 효과는 [OPS-004~006](../specs/operations.md#7-수용-테스트)·[OPS-023](../specs/operations.md#7-수용-테스트)를 다루는 작업에서 확인한다. | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·계약 테스트 파일 `applies_default_ports_only_to_tcp_entry_points`, `rejects_listen_conflicts_after_default_ports` | 기본 포트: 주소만 쓴 `tcp_viewer`는 5901, `tcp_server`는 5500, IPv6는 대괄호 필수. `ws_viewer`·`wss_viewer`·관리·상태 `listen`의 포트 생략은 `config_listen_invalid`. 충돌 검사는 기본 포트를 붙인 뒤 적용(1.2·1.6절). [OPS-001](../specs/operations.md#7-수용-테스트) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·`crates/domain/src/allow_list.rs` 단위 테스트, 계약 테스트 파일 `accepts_single_cidr_and_range_items_of_both_families`, `rejects_bad_ip_items_with_their_own_codes`, `requires_allow_list_only_for_enabled_entry_points`, `accepts_an_empty_allow_list`, `accepts_items_covering_a_whole_family_with_warnings` | IP 항목: 단일·CIDR·범위 해석, 역순 범위·체계 혼합·표기 오류의 코드, 목록 누락은 오류, 빈 목록은 받음, 전체 허용 항목의 경고 대상 계산(1.3·2절). [OPS-038](../specs/operations.md#7-수용-테스트), [OPS-040](../specs/operations.md#7-수용-테스트)의 설정 검증 부분 | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·TCP 진입점 통합 테스트(미생성, 작업 2.4) | 경계 주소의 허용·거절, 빈 목록의 모든 출발지 거절, 진입점별 목록, 전체 허용 경고 기록(1.3·2절). [OPS-037](../specs/operations.md#7-수용-테스트), [OPS-038](../specs/operations.md#7-수용-테스트), [OPS-040](../specs/operations.md#7-수용-테스트) | 미실행 | TCP 진입점 미구현 |
| Consumer·`crates/domain/src/origin.rs` 단위 테스트, 계약 테스트 파일 `checks_origin_values_and_entry_point_specific_keys` | Origin 문법과 정규화·정확 일치(1.4절). 진입점별로 쓸 수 있는 키 | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·WebSocket 통합 테스트(미생성, 작업 3.2) | 업그레이드 요청의 Origin 비교. [WS-027](../specs/websocket.md#7-수용-테스트), [WS-015](../specs/websocket.md#7-수용-테스트) | 미실행 | WebSocket 진입점 미구현 |
| Consumer·`crates/domain/src/target_policy.rs` 단위 테스트, 계약 테스트 파일 `checks_mode1_targets`, `accepts_tls_example_and_resolves_relative_paths` | Mode I 대상 항목 문법과 세부 원인(1.5절), `mode1.enabled = true`일 때 필수 | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·중계 통합 테스트(미생성, 작업 2.8) | 최종 IP 판정(1.5절). 꺼진 모드 거절. [RLY-007](../specs/relay.md#7-수용-테스트), [RLY-008](../specs/relay.md#7-수용-테스트), [RLY-022](../specs/relay.md#7-수용-테스트) | 미실행 | Mode I 미구현 |
| Consumer·계약 테스트 파일 `allows_plaintext_management_only_on_loopback`, `warns_for_plaintext_ws_and_non_loopback_status` | 평문 관리 리스너는 루프백만, 관리 TLS 파일 쌍, 평문 WS·비루프백 상태 리스너의 경고 대상 계산(1.6·2.2절) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·`serve`·WebSocket 통합 테스트(미생성, 작업 2.2·3.2·4.2) | 비루프백 평문 관리 설정의 시작 실패, 평문 WS는 명시적으로 켠 경우만 열림, 경고 기록. [OPS-014](../specs/operations.md#7-수용-테스트), [OPS-016](../specs/operations.md#7-수용-테스트), [OPS-034](../specs/operations.md#7-수용-테스트), [WS-024](../specs/websocket.md#7-수용-테스트) | 미실행 | `serve`·WebSocket 진입점 미구현 |
| Consumer·계약 테스트 파일 `rejects_empty_paths_and_reads_log_output`, `load_resolves_relative_paths_against_the_config_directory` | 빈 경로 거절, 상대 경로는 설정 파일 디렉터리 기준, `log.output`의 `"stdout"`과 경로 구분(1.1·1.2절) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·재로드 통합 테스트(미생성) | 재로드 대상과 재시작 필요 구분, 비재로드 차이가 남은 후보 거절, 후보 실패 시 기존 유지, 주기 확인(1.8·2.3절). [OPS-003](../specs/operations.md#7-수용-테스트), [OPS-024~028](../specs/operations.md#7-수용-테스트), [OPS-035](../specs/operations.md#7-수용-테스트), [OPS-041](../specs/operations.md#7-수용-테스트), [WS-022](../specs/websocket.md#7-수용-테스트) | 미실행 | 구현·테스트 파일 없음 |
| Consumer·설정 로더·각 기능 통합 테스트(미생성) | K·Kw·G·F·Si·Sa·P·R 값이 각 기능에 반영. [RLY-025~027](../specs/relay.md#7-수용-테스트), [WS-026](../specs/websocket.md#7-수용-테스트), [OPS-008](../specs/operations.md#7-수용-테스트)·[OPS-009](../specs/operations.md#7-수용-테스트)·[OPS-042](../specs/operations.md#7-수용-테스트), [MGT-022](../specs/management.md#7-수용-테스트), [MGT-024](../specs/management.md#7-수용-테스트), [MGT-025](../specs/management.md#7-수용-테스트), [DASH-022](../specs/dashboard.md#7-수용-테스트) | 미실행 | 구현·테스트 파일 없음 |
| Producer·배포 검토(미생성) | 배포하는 설정 예시가 이 계약을 통과하고 비밀·기본 비밀번호가 없다. [OPS-031](../specs/operations.md#7-수용-테스트) | 미실행 | 예시 파일·구현 없음 |

### 계약 테스트 실행 기록

| 대상 | 환경 | 명령 | 결과 |
| --- | --- | --- | --- |
| 작업 2.1 변경(기준 커밋 `80267b2`에 `crates/domain`·`crates/app`·`crates/adapters`와 작업 공간 설정 추가) | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행) | 통과. `config_contract` 21건, adapters 단위 2건, domain 단위 10건 모두 통과. 시나리오 보고: 통과 3(OPS-001, OPS-038, OPS-040의 설정 검증 부분), 미연결 145, 오류 0, 판정 통과 |
| 같음 | Windows 11 | 같음 | 미실행. Windows MSVC 링커가 없어 빌드할 수 없다(작업 0.1). Windows 경로 분기(`cfg!(windows)`)는 실행하지 않았다. |

## 6. 초안을 확정하기 전에 해결할 사항

- 이 계약의 이름·코드 선택(키 이름, 오류·경고 코드, 테이블 구조)은 2026-10-16에 사용자가 채택했다. 키 이름은 공유 인터페이스 이름을 따른다. 인증서 파일 오류(`tls_file_invalid`)의 시작 종료 코드는 3으로 확정했다. 리스너 바인드 실패는 4다.
- 허용 암호 스위트는 선택한 rustls 암호 모듈의 rustls 기본 목록으로 확정했다(1.9절). 암호 모듈(aws-lc-rs 또는 ring) 선택은 구현 단계에서 두 OS 빌드를 확인한 뒤 정한다([OPS-TBD-009](../specs/operations.md#8-미결정-사항)). 개인키 종류의 지원 범위(RSA·ECDSA 등)는 구현에서 확인한 뒤 1.9절에 적는다.
- `buffer_bytes`(B)와 `ws_message_bytes`(M)의 관계를 정하지 않았다. M이 B보다 클 때 메시지 처리 방식은 구현에서 확인한다([WS-TBD-004](../specs/websocket.md#8-미결정-사항)).
- 알림을 놓친 변경의 최대 지연은 Dp + D다([운영 스펙 3.3절](../specs/operations.md#33-파일-변경-자동-감지와-부분-재로드), [OPS-041](../specs/operations.md#7-수용-테스트)). 변경 안정화·교체 저장 감지·동시성의 실제 방식은 [OPS-TBD-007](../specs/operations.md#8-미결정-사항)에서 구체화한다.
- Origin 정규화(1.4절)는 채택했으며 WebSocket 계약 1.3절이 이 규칙을 인용한다. 두 계약이 같은 값을 쓴다.
- Windows 파일 권한 설정과 경로 처리, 로그 파일을 교체할 때 서버의 재오픈 여부는 관찰로 확인 필요하다.
