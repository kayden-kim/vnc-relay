# 설정 파일 v3 계약

운영자가 쓰는 TOML 설정 파일의 키, 자료형, 기본값, 유효 범위, 항목 문법, 재로드 구분을 정한다. `vnc-relay serve`는 이 계약에 맞지 않는 파일로 시작하지 않으며, 실행 중에도 이 계약에 맞지 않는 재로드 후보를 적용하지 않는다. 이 문서는 파일 형식만 정한다. 시작·재로드의 외부 동작은 [운영 스펙](../specs/operations.md)이 정하고, 관리자 계정 파일과 하위 명령은 [CLI·계정 파일 계약](cli-v1.md)이 정한다.

이 계약은 2026-10-16에 사용자가 채택했다(상태 Active). 승인 표의 기본값·유효 범위, 추가로 승인한 값, 이 문서에서 정한 키 이름·테이블 구조·형식·오류 코드가 모두 채택 범위다. 2026-11-01에 `limits.initial_message_bytes`(I)를 지워 2.0.0으로 올렸다(사용자 결정 2026-11-01(Q71)). 2026-11-08에 재로드 대상을 넓혀 3.0.0으로 올렸다(사용자 결정 2026-11-08(Q168·Q171), 3절의 재로드 구분 변경 규칙). v1 파일은 [3.1절](#31-v1-파일을-v2로-옮기기), v2 파일은 [3.2절](#32-v2-파일을-v3로-옮기기)을 따른다. 새 설정 키 `management.config_edit`는 사용자 결정 2026-11-08(Q176)으로 채택했다. 6절에 남긴 구현·관찰 확인은 이 채택으로 해소되지 않는다. 계약 테스트 결과는 5절에서 관리한다. 3.0.0의 확대 재로드·설정 편집 규칙은 커밋 `eae442f`의 두 OS 게이트에서 통과했다([작업 6.9 판정표](../how-to/set-up-and-verify.md#42-작업-69-판정표)).

| 항목 | 값 |
| --- | --- |
| 이름과 버전 | `vnc-relay-config` 3.0.0. 19개 키의 재로드 구분을 바꿔 대상을 8개에서 27개로 넓혔다(사용자 결정 2026-11-08(Q168·Q171), 3절의 호환성을 깨는 변경). `management.config_edit` 선택 키와 관리 API의 파일 쓰기 규칙도 더했다(사용자 결정 2026-11-08(Q167·Q170)). 이전 2.0.0은 `limits.initial_message_bytes`(I)와 관계 I ≤ B를 지웠다(사용자 결정 2026-11-01(Q71)). B와 M 사이에는 대소 제약이 없다(사용자 결정 2026-11-01(Q78)). 이전 1.1.0은 `log_file_unavailable`, 1.2.0은 관리 TLS·첫 요청 머리 읽기의 U 적용을 더했다. 1.8절 규칙 1의 감시 대상 명확화는 사용자 결정 2026-10-22(Q50)를 따른다. |
| 상태 | Active(2026-10-16 채택) |
| 소유자 | vnc-relay 프로젝트 관리자 |
| 만드는 쪽(Producer) | 운영자(설정 파일 작성자), 편집 기능을 켰을 때의 관리 API |
| 사용하는 쪽(Consumer) | `vnc-relay serve`(시작 검증·자동 재로드·관리 API 편집 적용). `vnc-relay admin` 하위 명령은 `[admin]` 테이블만 읽는다. |
| 방향 | 운영자가 쓴 파일을 서버가 읽는다. `management.config_edit = true`이면 관리 API도 같은 파일을 쓴다(1.10절). |
| 스키마 기준 파일 | 없음. 이 문서의 Markdown 표가 형식의 기준이다. |
| 기준 스펙 | [설정·프로세스 수명·운영 보안 스펙](../specs/operations.md) |
| 관련 요구사항 | [PRD](../requirements/PRD.md) FR-02·FR-05·FR-06·FR-07·FR-08, [7절 보안 및 신뢰 경계](../requirements/PRD.md#7-보안-및-신뢰-경계). 설정 편집과 확대 재로드의 근거는 사용자 결정 2026-11-08(Q167~Q172)다. |
| 호환성 범위 | 설정 파일 v3(`vnc-relay-config` 3.0.0). Windows 11 x64·Ubuntu 24.04 LTS x64의 `vnc-relay` 1.x. 파일 안에 형식 버전 키는 없다. v1→v2는 3.1절, v2→v3는 3.2절을 따른다. |
| 관련 테스트 | [운영 스펙 수용 테스트](../specs/operations.md#7-수용-테스트) OPS-001~006·OPS-016·OPS-023~027·OPS-031·OPS-034~042·OPS-047~064, [WS·WSS 스펙](../specs/websocket.md#7-수용-테스트) WS-015·WS-022·WS-024·WS-027, [중계 스펙](../specs/relay.md#7-수용-테스트) RLY-007·RLY-008·RLY-022~027. 계약 테스트는 5절에 연결한다. 기존 2.0.0 구현의 `ee0314f` 두 OS 전체 게이트 결과와 [작업 5.9 판정표](../how-to/set-up-and-verify.md#41-작업-59-판정표)는 확대 재로드·설정 편집의 증거가 아니다. 새 규칙은 커밋 `eae442f`의 두 OS 전체 게이트와 [작업 6.9 판정표](../how-to/set-up-and-verify.md#42-작업-69-판정표)로 확인했다. |

용어는 [VNC Relay 용어집](../../GLOSSARY.md)을 따른다. 테스트·검증 기호 L~Dp 등은 [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)의 기호이며, 이 계약이 각 기호를 키에 연결한다.

관련 문서: [CLI·계정 파일 계약](cli-v1.md)(`[admin]` 테이블 사용, 종료 코드), [관리 API 계약](management-api-v1.md)(`[dashboard]`·`[recent_closed]`·`[management]`의 값 전달), [WebSocket 중계 계약](websocket-relay-v1.md)(`allowed_origins`·WS 키 사용), [Repeater 초기 교환 계약](repeater-handshake-v1.md)(`handshake_secs`·`[mode1]`·`[mode2]` 사용), [운영 정보 계약](ops-info-v1.md)(경고·재로드 사건 이름, `[log]`·`[status]`), [ADR-0001 내장 TLS](../decisions/ADR-0001-built-in-tls.md), [ADR-0002 출발지 IP 허용 목록](../decisions/ADR-0002-source-ip-allowlist.md), [ADR-0004 tokio·rustls](../decisions/ADR-0004-tokio-rustls.md).

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

기호는 [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)의 검증 기호다. "재로드"는 1.8절의 구분이다. 재로드 열 또는 표 아래 문장에 대상으로 적지 않은 키는 모두 재시작이 필요하다. 다른 계약은 이 표의 수치를 반복하지 않고 키 이름과 이 문서 링크로 인용한다.

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
| `allowed_origins` | 문자열 배열 | 아님 | `[]` | `ws_viewer`·`wss_viewer`만 가진다. Origin 허용 값 배열(1.4절). 빈 배열은 Origin 헤더가 있는 요청을 모두 거절한다. Origin 헤더가 없는 요청은 이 키와 무관하게 IP 검사만 한다. | 대상 |
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
| `mode1.allow_targets` | 문자열 배열 | `mode1.enabled = true`이면 필수 | 없음 | 대상 허용 항목 배열(1.5절). 빈 배열은 모든 대상 거절. 생략은 시작 실패. | 대상 |
| `mode2.enabled` | 불리언 | 아님 | `false` | Mode II 요청을 받는다. | 재시작 |

#### `[limits]`

| 키(기호) | 자료형 | 필수 여부 | 기본값 | 유효 범위 | 의미 |
| --- | --- | --- | --- | --- | --- |
| `relay_connections`(L) | 정수 | 아님 | 1000 | 2~65535 | 중계 측 연결 소켓 상한. 집계 범위는 [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)의 L이다. |
| `admin_connections`(Lm) | 정수 | 아님 | 64 | 1~1024 | 관리·상태 리스너 연결 소켓 상한. L과 독립이다. |
| `waiting`(Lw) | 정수 | 아님 | 200 | 1~`relay_connections` | Mode II 대기 연결 상한. L도 함께 적용한다. |
| `buffer_bytes`(B) | 정수(바이트) | 아님 | 131072 (128 KiB) | 16384~4194304 (16 KiB~4 MiB) | 연결당 중계 버퍼 상한. |
| `ws_message_bytes`(M) | 정수(바이트) | 아님 | 1048576 (1 MiB) | 65536~16777216 (64 KiB~16 MiB) | WebSocket 논리 메시지 크기 상한. 프레임 분할 전후에 같은 제한이다. B와 M 사이에 크기 제약은 없다(사용자 결정 2026-11-01(Q78)). |

모든 `[limits]` 키는 재로드 대상이다. 적용 시점은 1.8절을 따른다.

#### `[timeouts]`

| 키(기호) | 자료형 | 필수 여부 | 기본값 | 유효 범위 | 의미 |
| --- | --- | --- | --- | --- | --- |
| `handshake_secs`(H) | 정수(초) | 아님 | 10 | 1~120 | Repeater 초기 교환 제한 시간. 초기 교환 상태에 들어간 때부터 잰다. WS·WSS에서는 초기 교환 상태가 `101` 응답을 보낸 때 들어가므로, 그때부터 잰다(사용자 결정 2026-10-22(Q22)). |
| `tls_upgrade_secs`(U) | 정수(초) | 아님 | 10 | 1~120 | WSS의 TLS 협상과 WS·WSS의 HTTP 업그레이드를 합친 제한 시간. 접속 수락 때부터 잰다. 관리 리스너에서는 TLS 협상과 첫 요청 머리 읽기에 쓴다. 관리 리스너는 첫 요청 머리가 끝나기 전에는 수락 때부터 잰 절대 기한이고, 머리가 끝난 뒤에는 읽기·쓰기가 없는 동안만 잰다(사용자 결정 2026-10-22(Q25), 관리 리스너 설계 9절 11번). 예외: 검증·수락된 관리자 종료 요청의 정리 완료 대기 중에만 U를 멈추고, 응답 쓰기부터 다시 잰다(사용자 결정 2026-11-02(Q92)). |
| `wait_secs`(W) | 정수(초) | 아님 | 3600 | 10~86400 | Mode II 상대 대기 제한 시간. 유효한 대기 등록부터 매칭까지다. |
| `connect_secs`(C) | 정수(초) | 아님 | 10 | 1~120 | Mode I 대상 연결 제한 시간. 주소 해석 시작부터 TCP 연결 성립까지를 하나로 잰다. |
| `close_secs`(E) | 정수(초) | 아님 | 10 | 1~120 | EOF 이후 반대 방향 전달과 종료 교환(WebSocket close 포함)을 기다리는 제한 시간. |
| `shutdown_grace_secs`(G) | 정수(초) | 아님 | 30 | 0~600 | 정상 종료 때 활성 세션에 주는 유예 시간. 0은 유예 없이 바로 정리한다. |

모든 `[timeouts]` 키는 재로드 대상이다. 적용 시점은 1.8절을 따른다.

#### `[keepalive]`

| 키(기호) | 자료형 | 필수 여부 | 기본값 | 유효 범위 | 의미 |
| --- | --- | --- | --- | --- | --- |
| `tcp_idle_secs`(K 유휴) | 정수(초) | 아님 | 60 | 10~3600 | TCP keepalive 첫 탐색 전의 유휴 시간. |
| `tcp_interval_secs`(K 탐색 간격) | 정수(초) | 아님 | 10 | 1~300 | 탐색 사이의 간격. |
| `tcp_probes`(K 탐색 횟수) | 정수 | 아님 | 6 | 1~20 | 응답 없는 탐색을 몇 번 보낸 뒤 끊긴 연결로 볼지 정한다. |
| `ws_ping_secs`(Kw ping) | 정수(초) | 아님 | 30 | 5~300 | WS·WSS 연결에 보내는 ping 주기. |
| `ws_pong_timeout_secs`(Kw pong 제한) | 정수(초) | 아님 | 30 | 5~300 | ping 뒤 pong을 기다리는 제한 시간. |

K의 감지 시간은 `tcp_idle_secs + tcp_interval_secs × tcp_probes`다. 기본값은 120초다. 중계 측 TCP 소켓 전체에 적용한다(WS·WSS 하부 TCP 포함). 모든 `[keepalive]` 키는 재로드 대상이다. 적용 시점은 1.8절을 따른다.

#### `[management]`, `[management.login_limit]`, `[management.session]`

| 키(기호) | 자료형 | 필수 여부 | 기본값 | 유효 범위·의미 |
| --- | --- | --- | --- | --- |
| `management.listen` | 문자열(주소) | 필수 | 없음 | 관리 리스너 주소(1.1절). 포트까지 필수다. 루프백도 쓸 수 있다. |
| `management.cert_file` | 문자열(경로) | 아님. `key_file`과 함께 쓴다. | 없음 | PEM 인증서 체인(1.9절). 두 키를 모두 쓰면 HTTPS, 모두 생략하면 평문 HTTP다. 평문 HTTP는 루프백 주소에서만 허용한다. |
| `management.key_file` | 문자열(경로) | 아님. `cert_file`과 함께 쓴다. | 없음 | PEM 개인키(1.9절). |
| `management.config_edit` | 불리언 | 아님 | `false` | `true`이면 관리 API·대시보드에서 편집 대상 키를 바꿀 수 있다(1.8·1.10절). 재시작 필요. 편집 대상 아님(사용자 결정 2026-11-08(Q170)). |
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
| `log.output` | 문자열 | 아님 | `"stdout"` | `"stdout"`(표준 출력) 또는 로그 파일 경로. 파일이면 이어 쓴다. 파일이 없으면 만든다. 이름이 `stdout`인 파일은 `"./stdout"`으로 쓴다. 운영자가 파일을 교체하면 같은 경로를 다시 연다([운영 정보 계약 1.1절](ops-info-v1.md#11-로그-일반-규칙) "파일 다시 열기", 사용자 결정 2026-11-04(Q104)). 회전 시점·보관은 운영자 책임이다. |
| `reload.settle_secs`(D) | 정수(초) | 아님 | 5 | 1~60. 마지막 변경을 감지한 뒤 적용·실패 결과를 확인할 때까지의 최대 시간. |
| `reload.poll_secs`(Dp) | 정수(초) | 아님 | 30 | 5~600. OS 파일 알림을 놓친 변경을 찾는 주기 확인 간격. 로그 파일을 다시 여는 주기도 이 값이다(사용자 결정 2026-11-04(Q109)). |

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

대상의 IP 항목이 포트와 관계없이 한 체계의 전체 주소를 덮으면(판정은 1.3절 규칙과 같다) 허용하되 경고 `allow_all_target`을 시작·재로드 때 기록한다. 임의 대상에 접속하는 공개 프록시가 되지 않도록 운영자가 항목을 좁혀야 한다.

### 1.6 값 관계 검사

시작과 재로드 후보 검증에서 모두 검사한다. 위반은 `config_relation_violated` 또는 지정한 코드다.

| 관계 | 조건 | 위반 코드 |
| --- | --- | --- |
| Lw ≤ L | `limits.waiting` ≤ `limits.relay_connections` | `config_value_out_of_range`(범위의 상한이 L이다) |
| Si ≤ Sa | `management.session.idle_secs` ≤ `management.session.max_lifetime_secs` | `config_relation_violated` |
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

[운영 스펙 3.3절](../specs/operations.md#33-파일-변경-자동-감지와-부분-재로드)을 키에 적용한 표다. 재로드 대상은 기존 8개와 새 19개를 합친 27개다(사용자 결정 2026-11-08(Q168·Q171)). 목록에 없는 키는 모두 "재시작 필요"다. 이미 맺은 연결·이미 시작한 타이머·이미 설정한 소켓 옵션은 이전 값을 유지한다.

| 재로드 대상 | 적용 시점 |
| --- | --- |
| `relay.tcp_viewer.allow` | 새 TCP Viewer 접속부터 |
| `relay.tcp_server.allow` | 새 TCP Server 접속부터 |
| `relay.ws_viewer.allow` | 새 WS 접속부터(수락 직후와 업그레이드 직전 검사) |
| `relay.wss_viewer.allow` | 새 WSS 접속부터(수락 직후와 업그레이드 직전 검사) |
| `relay.wss_viewer.cert_file`·`key_file`의 경로 값과 가리키는 파일 내용 | 새 TLS 핸드셰이크부터. 기존 TLS 연결은 유지한다. |
| `management.cert_file`·`key_file`의 경로 값과 가리키는 파일 내용 | 새 TLS 핸드셰이크부터. 기존 TLS 연결은 유지한다. |
| `relay.ws_viewer.allowed_origins`, `relay.wss_viewer.allowed_origins` | 교체 뒤 Origin 검사를 받는 새 업그레이드 요청부터 |
| `mode1.allow_targets` | 교체 뒤 시작하는 Mode I 대상 연결의 허용 판정부터. 이미 성립한 세션 유지 |
| `limits.relay_connections`(L), `limits.admin_connections`(Lm) | 교체 뒤 새 연결 수락 판정부터. 올리면 바로 적용한다. 낮춘 뒤 사용량이 새 상한 이상이면 기존 연결은 유지하고, 사용량이 새 상한 아래로 내려갈 때까지 새 연결을 거절한다. 서버 요약의 상한도 새 값 |
| `limits.waiting`(Lw) | 교체 뒤 새 대기 등록부터. 새 상한을 넘는 기존 대기도 유지 |
| `limits.buffer_bytes`(B) | 교체 뒤 수락하는 연결부터 |
| `limits.ws_message_bytes`(M) | 교체 뒤 업그레이드하는 WS·WSS 연결부터 |
| `timeouts.handshake_secs`(H) | 교체 뒤 시작하는 초기 교환부터 |
| `timeouts.tls_upgrade_secs`(U) | 교체 뒤 수락하는 WS·WSS·관리 연결부터 |
| `timeouts.wait_secs`(W) | 교체 뒤 등록하는 대기 연결부터. 기존 대기 기한 유지 |
| `timeouts.connect_secs`(C) | 교체 뒤 시작하는 대상 연결 시도부터 |
| `timeouts.close_secs`(E) | 교체 뒤 시작하는 종료 교환부터. 세션 정보 응답의 `dashboard.terminate_timeout_secs`도 그 뒤 응답부터 E + 5초 |
| `timeouts.shutdown_grace_secs`(G) | 교체 뒤 시작하는 정상 종료부터. 이미 시작한 유예 유지 |
| `keepalive.tcp_idle_secs`, `keepalive.tcp_interval_secs`, `keepalive.tcp_probes` | 교체 뒤 수락·연결하는 TCP 소켓부터 |
| `keepalive.ws_ping_secs`, `keepalive.ws_pong_timeout_secs` | 교체 뒤 업그레이드하는 WS·WSS 연결부터 |

재시작이 필요한 변경에는 다음이 포함된다: 진입점·모드의 `enabled`, 모든 `listen`, `admin.accounts_file`, `admin.argon2`, `[management]`의 인증서·개인키 이외 키(HTTP·HTTPS 전환과 `management.config_edit` 포함), `[dashboard]`, `[recent_closed]`, `[status]`, `[log]`, `[reload]`.

규칙:

1. 감시 대상은 설정 파일과, 켜진 WSS 진입점·관리 리스너가 쓰는 `cert_file`·`key_file`이 가리키는 파일이다(사용자 결정 2026-10-22(Q50)). 꺼진 진입점의 파일은 감시하지 않는다. 계정 파일은 감시하지 않는다([CLI·계정 파일 계약 1.7절](cli-v1.md#17-서버-실행-중-계정-파일-변경)).
2. 후보는 파일을 읽어 기본값을 채운 유효 설정이다. 설정 값이 같으면 주석·공백·키 순서·기본값 명시 여부가 달라도 차이가 없다.
3. 후보 검증은 이 문서 1.1~1.7절을 모두 포함한다. 알 수 없는 키와 모든 필수 키 검사도 같다. 인증서·개인키 쌍은 개인키가 인증서에 맞아야 한다. 한쪽 파일만 바뀌어 맞지 않으면 후보는 유효하지 않다.
4. 재로드 대상이 아닌 값이 실행 중 값과 다르면 후보 전체를 `reload_restart_required`로 거절한다. 그 차이가 파일에 남아 있는 동안 이후 재로드 대상 변경(IP·인증서·관리 API 편집 포함)도 모두 거절한다.
5. 후보가 유효하고 차이가 재로드 대상뿐이면 재로드 대상 값을 한 번에 교체한다. 일부만 적용하지 않는다.
6. 검증 실패는 `reload_rejected`다. 실패 원인 코드(2절)를 함께 알린다. 기존 설정·연결을 유지한다.
7. OS 파일 알림을 놓친 변경은 `reload.poll_secs`(Dp) 주기 확인으로 감지한다. 감시 자체가 실패하면 `reload_watch_failed`를 알리고 이전 유효 설정으로 계속 운영한다.
8. 편집 대상은 재로드 대상에서 인증서·개인키 경로 4개(`relay.wss_viewer.cert_file`·`key_file`, `management.cert_file`·`key_file`)를 뺀 23개다(사용자 결정 2026-11-08(Q172)). 이 4개 경로는 설정 보기에만 나온다. 인증서 갱신은 같은 경로의 파일을 교체하는 기존 방식으로 한다.
9. 진입점별 `allow`·`allowed_origins`는 그 진입점이 켜져 있을 때만 편집할 수 있다. `mode1.allow_targets`는 `mode1.enabled = true`일 때만 편집할 수 있다. 꺼진 진입점·모드의 키는 보기에만 나온다. 값이 없으면 API에서 `null`로 보인다.
10. 파일 변경 감지와 관리 API 편집은 같은 후보 검증·전체 교체 규칙을 쓴다. 관리 API는 파일 쓰기 성공 뒤 같은 후보를 즉시 한 번 교체한다. `reload_applied.trigger`는 파일 변경 감지이면 `file_watch`, 관리 API 편집이면 `config_edit`다. 뒤따르는 파일 감지는 차이가 없으므로 다시 교체하거나 `reload_applied`를 기록하지 않는다.

### 1.9 인증서·개인키 파일 형식

PEM을 쓴다. 파일 내용 검증은 서버가 한다.

| 키 | 형식 |
| --- | --- |
| `cert_file` | PEM 인코딩 X.509 인증서 한 개 이상. 첫 번째가 서버(리프) 인증서이고 뒤에 중간 인증서를 이어 붙인다. |
| `key_file` | 위 서버 인증서의 개인키 한 개를 담은 PEM. PKCS#8(`BEGIN PRIVATE KEY`)을 권장한다. 지원하는 키는 RSA(PKCS#1·PKCS#8), ECDSA P-256·P-384·P-521(SEC1·PKCS#8), Ed25519(PKCS#8)다. 그 밖의 키(X25519 등)와 손상된 키는 `tls_file_invalid`로 거절한다. |

TLS 1.2 미만은 거절한다. 허용 암호 스위트는 선택한 rustls 암호 모듈(CryptoProvider)이 제공하는 rustls 기본 목록이다. 설정 키가 없으므로 운영자가 바꿀 수 없다. 스위트 이름은 모듈에 따라 달라지므로 이 계약이 나열하지 않는다. 암호 모듈은 aws-lc-rs다(사용자 결정 2026-10-20). 확정 기록은 [운영 스펙 8절](../specs/operations.md#8-미결정-사항)의 OPS-TBD-009다. 개인키 파일은 서버를 실행하는 OS 계정만 읽도록 권한을 제한하기를 권장한다. 인증서·개인키 내용은 로그·오류 메시지에 쓰지 않는다.

### 1.10 관리 API 편집의 파일 쓰기

관리 API는 `serve`가 읽는 같은 TOML 파일에 편집 값을 기록한다(사용자 결정 2026-11-08(Q167)). 결과 파일은 같은 로더가 읽는 v3 파일이다. 별도 동적 설정 파일이나 메모리 전용 적용은 없다. 요청·오류 형식은 [관리 API 계약](management-api-v1.md)이 정한다.

| 항목 | 규칙 |
| --- | --- |
| 바꾸는 내용 | `toml_edit`로 실제 바뀐 키의 값만 교체한다(사용자 결정 2026-11-08(Q167·Q173)). 다른 값·주석·공백·키 순서를 보존한다. |
| 없는 키·테이블 | 키가 없으면 기존 테이블에 넣는다. 테이블도 없으면 파일 끝에 새 테이블로 더한다. |
| 인코딩·줄바꿈 | UTF-8을 쓴다. 새 줄은 파일에서 처음 나오는 줄바꿈 방식을 따른다. 줄바꿈이 없으면 LF를 쓴다. 고치지 않은 바이트는 그대로 유지하며 혼합된 줄바꿈을 통일하지 않는다(이 문서의 판단, 사용자 결정 없음). |
| 검증 | 고친 내용을 시작·재로드와 같은 로더로 검증한다. 오류이면 파일과 실행 값을 바꾸지 않는다. 편집 전후 파일 바이트가 같으면 쓰기·교체·실행 값 적용을 하지 않는다(이 문서의 판단, 사용자 결정 없음). |
| 쓰기·교체 | 공유 잠금 안에서 같은 디렉터리의 임시 파일을 쓰고 flush한다. 원자 교체 바로 전에 현재 파일을 다시 읽어 `revision`을 비교한다. 다르면 임시 파일을 지우고 `409 config_conflict`로 거절한다. 이번 요청은 파일·실행 값을 바꾸지 않는다. 같으면 원자 교체 뒤 같은 후보를 한 번에 적용한다(1.8절 규칙 10). 마지막 재확인과 교체 사이의 아주 짧은 틈은 보장 범위가 아니다(사용자 결정 2026-11-08(Q178)). 재확인에서 읽기에 실패해도 임시 파일을 지우고 `409 config_conflict`로 거절한다. 이때도 교체·적용하지 않고 `config_edit_rejected.reason = config_conflict`를 기록한다(이 문서의 판단, 사용자 결정 없음). 그 밖의 쓰기·교체 실패이면 원본 파일과 실행 값을 유지한다. |
| `revision` | 현재 설정 파일 바이트의 SHA-256이다. `sha256:` 뒤에 소문자 16진수 64자를 붙인다. 의미가 같아도 바이트가 바뀌면 달라진다. UTF-8이 아니어도 바이트를 읽었으면 그 바이트의 SHA-256을 반환한다. 보기 응답의 `null`은 바이트를 읽지 못했을 때만 쓴다(이 문서의 판단, 사용자 결정 없음). |
| OS 쓰기 권한 | `management.config_edit = true`일 때만 서비스 실행 계정에 설정 파일·같은 디렉터리의 임시 파일 생성·원자적 교체에 필요한 쓰기 권한이 필요하다. 편집을 끄면 읽기 권한만으로 운영할 수 있다. |


## 2. 오류

### 2.1 설정 오류와 준비 실패

모든 진단은 `code`, `key`(점으로 이은 전체 경로. 배열은 0부터 센 `[i]` 첨자), `message`(사람이 읽는 문장)를 가진다. `code`와 `key`는 안정적인 값이다. `message`는 바뀔 수 있다. 진단에 비밀·파일 내용을 넣지 않는다. 시작에서는 가능한 모든 설정 오류를 모아 보고한다. 출력 형식은 [CLI·계정 파일 계약 1.5절](cli-v1.md#15-입력과-출력-규칙)이, 종료 코드는 [2.1절](cli-v1.md#21-종료-코드)이 정한다.

UTF-8이 아닌 파일도 `config_file_unreadable` 진단에 해당한다. 관리 API에서는 `file.state = unreadable`, `diagnostics = []`로 표시한다. 바이트 읽기에 성공했으면 `revision`은 그 바이트의 SHA-256이며, 바이트를 읽지 못했을 때만 `null`이다. 편집 요청은 관리 API 처리 3단계에서 `409 config_file_invalid`와 빈 진단 배열로 거절한다(이 문서의 판단, 사용자 결정 없음). 요청 처리 순서는 [관리 API 계약](management-api-v1.md)을 따른다.

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
| `tls_file_invalid` | 인증서·개인키 경로·형식 오류. 파일이 없거나 읽을 수 없거나 PEM이 아니거나 지원하지 않는 키 종류이거나 서로 맞지 않는다. 시작 종료 코드는 설정 오류와 같은 3이다([CLI·계정 파일 계약 2.1절](cli-v1.md#21-종료-코드)). | 시작 실패(설정 오류와 같은 종료 코드) | 후보 거절, 기존 인증서 유지 | OPS-002, OPS-026 |
| `log_file_unavailable` | `log.output`이 가리키는 로그 파일을 이어 쓰기로 열 수 없다. 디렉터리가 없거나 권한이 없거나 디렉터리이거나 입출력 오류다. `key`는 `log.output`이다. 설정 검증이 끝난 뒤 시작 때만 검사한다. 시작 종료 코드는 설정 오류와 같은 3이다([CLI·계정 파일 계약 2.1절](cli-v1.md#21-종료-코드)). | 시작 실패(설정 오류와 같은 종료 코드) | 검사하지 않는다. `log.output`은 재시작이 필요한 키다(1.8절). | 없음 |

후보에 설정 오류가 있으면 어느 키의 오류든 후보 전체를 `reload_rejected`로 거절한다. 오류가 없고 재로드 대상이 아닌 값이 실행 중 값과 다르면 `reload_restart_required`다(1.8절).

### 2.2 경고

경고는 시작·재로드를 막지 않는다. 운영 로그의 경고로 기록한다([PRD FR-08](../requirements/PRD.md#fr-08-운영-정보)). 로그 사건 이름은 아래 경고 코드를 그대로 쓴다. 필드는 [운영 정보 계약 1.5절](ops-info-v1.md#15-사건-목록)이 정한다.

| 경고 코드 | 조건 | 관련 스펙 시나리오 |
| --- | --- | --- |
| `allow_all_source` | `allow`에 한 체계 전체를 덮는 항목(1.3절)이 있다. 시작·재로드 때. | OPS-038 |
| `plaintext_ws_enabled` | `relay.ws_viewer.enabled = true`. 시작 때. | OPS-016, WS-024 |
| `allow_all_target` | `mode1.allow_targets`에 한 체계 전체를 덮는 IP 항목이 있다. 시작·재로드 때(`phase` = `startup` 또는 `reload`). | RLY-007, OPS-058 |
| `status_listen_not_loopback` | `status.listen`이 루프백이 아니다. 시작 때. | OPS-032 |

### 2.3 재로드 결과

관리 API 편집도 1.8절의 같은 재로드 결과 코드를 쓴다. HTTP 요청의 거절·쓰기 실패 코드는 [관리 API 계약](management-api-v1.md)이 따로 정한다.

| 결과 코드 | 의미 | 관련 스펙 시나리오 |
| --- | --- | --- |
| `reload_applied` | 후보가 유효하고 재로드 대상 값을 적용했다. | OPS-024, OPS-025, OPS-041 |
| `reload_rejected` | 후보가 유효하지 않다. 원인 코드(2.1절)를 함께 알린다. 기존 설정·연결 유지. | OPS-026, OPS-027 |
| `reload_restart_required` | 재로드 대상이 아닌 값이 실행 중 값과 다르다. 후보 전체 미적용. | OPS-003, OPS-035 |
| `reload_watch_failed` | 변경 감지에 실패했다. 이전 유효 설정으로 계속 운영한다. 시작 때 감시를 준비할 수 없으면 시작 실패다. | OPS-028 |

## 3. 호환성 규칙

- 서버는 알 수 없는 테이블·키를 `config_unknown_key` 오류로 거절해야 한다. 허용 형식 밖의 값(예: 허용하지 않는 Origin 스킴)은 해당 값의 오류 코드(`config_origin_invalid` 등)로 거절해야 한다. 무시하거나 경고만 하지 않는다.
- 호환 변경(minor 올림): 기본값이 있고 생략해도 기존 동작이 같은 새 선택 키 추가, 유효 범위 넓힘, 오류 메시지 문구 변경. 새 키를 쓴 파일은 그 키를 모르는 이전 서버가 거절한다. 그래서 새 키를 쓰려면 서버를 먼저 올린다.
- 호환성을 깨는 변경(새 major 버전): 키 이름·자료형·단위 변경이나 삭제, 필수 키 추가, 기본값 변경, 유효 범위 좁힘, 재로드 구분 변경, 항목 문법 변경, 보안 검사 완화. major를 올리고 마이그레이션(이전 파일을 새 형식으로 바꾸는 방법)과 롤백(새 형식 파일을 이전 서버로 되돌리는 방법) 계획을 함께 작성한다. 2.0.0의 계획은 3.1절, 3.0.0의 계획은 3.2절에 있다.
- 모든 값의 의미는 키 표에 있다. 키 순서와 주석은 의미가 없다.

### 3.1 v1 파일을 v2로 옮기기

| 항목 | 내용 |
| --- | --- |
| 바뀐 점 | `limits.initial_message_bytes`(I) 키와 관계 I ≤ B를 지웠다. 다른 키·기본값·유효 범위·문법·재로드 구분은 1.2.0과 같다. |
| v2 서버가 v1 파일을 읽을 때 | `initial_message_bytes`가 없는 파일은 그대로 읽는다. 있으면 `config_unknown_key`로 시작하지 않는다. 실행 중 재로드 후보이면 `reload_rejected`로 거절한다. |
| 마이그레이션 | 1. `[limits]`에서 `initial_message_bytes` 줄을 지운다. 2. 서버를 새 버전으로 재시작한다. 고친 파일은 v1 서버도 읽으므로 이 순서에서는 시작 실패가 없다. 지운 값이 4096이 아니었다면, 실행 중인 v1 서버는 그 변경을 `reload_restart_required`로 거절한다. 다음 재시작에서 해소된다. |
| 롤백 | v2 파일은 v1 서버(1.2.0)도 그대로 읽는다. v1에서 I는 선택 키이므로 생략하면 기본값 4096을 쓴다. 서버만 이전 버전으로 되돌린다. |
| 지원 기간 | 두지 않는다. Q71에서 키를 지금 지우기로 했으므로 v2 서버는 v1에만 있는 키를 받지 않고, 이 문서가 v1을 대체한다. v1을 따로 지원하는 기간은 사용자 결정이 아니라 이 문서를 쓰며 정한 판단이다. 판단 근거: 결정 당시 저장소에는 릴리스 태그가 없었고, 중계 기능(초기 교환 해석·매칭·중계)도 구현되지 않았다([중계 스펙](../specs/relay.md) 머리 표). 지원 기간이 필요하면 프로젝트 소유자가 정한다. |

### 3.2 v2 파일을 v3로 옮기기

| 항목 | 내용 |
| --- | --- |
| 바뀐 점 | 재로드 대상 19개를 더해 총 27개로 넓혔다(사용자 결정 2026-11-08(Q168·Q171)). `management.config_edit` 선택 키를 더했다(기본 `false`, 재시작 필요). 다른 기존 키의 뜻·기본값·유효 범위·문법은 같다. |
| v3 서버가 v2 파일을 읽을 때 | 모든 v2 파일을 고치지 않고 같은 뜻으로 읽는다. 파일에 버전 키를 더하지 않는다. |
| 마이그레이션 | 파일 수정 없이 v3 서버로 재시작한다. 다만 19개 키를 다음 재시작용으로 미리 고쳐 두면 이제 자동 적용된다. 이를 원하지 않는 운영자는 사본에서 고친 뒤 재시작할 때 파일을 바꿔 넣는다. |
| 롤백 | `management.config_edit`를 쓰지 않은 v3 파일은 v2 서버(릴리스 v2026.11.0)가 그대로 읽는다. 이 키를 썼으면 해당 줄을 지운 뒤 v2 서버로 되돌린다. 줄을 남기면 v2 서버는 `config_unknown_key`로 시작하지 않는다. |
| 지원 기간 | 두지 않는다. v3 서버가 모든 v2 파일을 같은 뜻으로 읽기 때문이다. 별도 지원 기간을 두지 않는 것은 이 계약의 기준이며 사용자 결정으로 보지 않는다. |


## 4. 버전과 지원 단계

| 상태 | 조건 | 지원 종료·대체 |
| --- | --- | --- |
| Draft | 채택 전 상태. 이 계약은 2026-10-16에 채택해 Draft를 지났다. | 해당 없음 |
| Active | 현재. 사용자 채택(2026-10-16). 3.0.0의 확대 재로드 근거는 사용자 결정 2026-11-08(Q168·Q171)와 3절이다. 구현·양 OS 계약 테스트 증거는 5절에서 관리한다. 새 규칙은 커밋 `eae442f`의 두 OS 게이트에서 통과했다. | 새 major가 Active가 되면 Deprecated로 바꾼다. v1은 3.1절, v2는 3.2절의 지원 기간 규칙대로 대체한다. |
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
config_edit = false # true로 바꾸고 재시작하면 관리 API·대시보드 편집을 켠다.

[status]
listen = "127.0.0.1:18081"

[log]
output = "stdout"
```

두 번째 예제의 `192.0.2.0/24:5900-5905`와 `[2001:db8::/32]:5900`는 최종 연결 IP 항목이다. `pc1.example:5900`은 이름 항목이다. `pc1.example`이 이 두 IP 항목 밖의 주소로 해석되면 연결하지 않는다.

결과와 증거는 [검증 기록](../AGENTS.md#검증-기록)을 따른다. 스펙과 같은 테스트는 해당 시나리오의 기록을 연결한다. 설정 로더의 테스트는 `crates/adapters/tests/config_contract.rs`(아래 표에서 "계약 테스트 파일")와 domain 단위 테스트에 있다. 실행 기록은 표 아래 [계약 테스트 실행 기록](#계약-테스트-실행-기록)에 있다. 각 시험의 범위는 아래 표에 구분한다. 현재 지원 판정은 `ee0314f`의 두 OS 전체 게이트와 [작업 5.9 판정표](../how-to/set-up-and-verify.md#41-작업-59-판정표)를 따른다.

| 주체·테스트 위치 | 확인할 계약 규칙 | 결과 | 실행 기록 |
| --- | --- | --- | --- |
| Consumer·계약 테스트 파일 `reports_every_problem_in_one_pass`, `rejects_syntax_errors_and_duplicate_keys_with_one_diagnostic`, `rejects_missing_unreadable_and_non_utf8_files`, `rejects_non_table_values_in_place_of_tables` | 구문 오류, 알 수 없는 키, 자료형 오류, 필수 키 누락(1.1·1.7절)을 모두 모아 진단으로 돌려준다. [OPS-001](../specs/operations.md#7-수용-테스트)의 설정 검증 부분 | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·`tests/serve.rs` `rejects_invalid_config_with_exit_code_3_and_every_cause`, `rejects_toml_syntax_error_and_missing_config_file_with_exit_code_3` | 위 진단이 시작 실패(종료 코드 3)와 비밀 없는 원인 표시(`error[<code>] key=<key>: <message>`)로 이어진다. stdout에는 진단마다 운영 로그 `startup_failed` 줄이 있고 설정 값은 없다. [OPS-001](../specs/operations.md#7-수용-테스트) | 통과 | [운영 정보 계약 실행 기록](ops-info-v1.md#계약-테스트-실행-기록) |
| Consumer·계약 테스트 파일 `accepts_minimal_example_and_fills_every_default`, `checks_every_numeric_key_at_its_bounds`, `checks_relations_between_values` | 승인 수치의 기본값, 최소·최대와 그 바깥(1.2절). Lw ≤ L, Si ≤ Sa(1.6절). 지운 키 `initial_message_bytes`는 `config_unknown_key`(3.1절). 수치가 각 기능에 주는 효과는 [OPS-004~006](../specs/operations.md#7-수용-테스트)·[OPS-023](../specs/operations.md#7-수용-테스트)를 다루는 작업에서 확인한다. | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·`tests/serve.rs` `initial_tls_and_upgrade_deadlines_have_distinct_causes_and_release_capacity` | H=1초·U=2초 설정을 실제 실행파일에 적용한다. TCP Viewer·TCP Server·WS·WSS 초기 교환은 H를, WS 업그레이드와 WSS TLS·업그레이드는 U를 적용한다. 원인·단계·통신 응답을 구분하고 L=2의 두 자리가 다시 수락되는지 확인한다. [OPS-006](../specs/operations.md#7-수용-테스트)(비부분) | 통과 | [운영 정보 계약 실행 기록](ops-info-v1.md#계약-테스트-실행-기록) |
| Consumer·계약 테스트 파일 `applies_default_ports_only_to_tcp_entry_points`, `rejects_listen_conflicts_after_default_ports` | 기본 포트: 주소만 쓴 `tcp_viewer`는 5901, `tcp_server`는 5500, IPv6는 대괄호 필수. `ws_viewer`·`wss_viewer`·관리·상태 `listen`의 포트 생략은 `config_listen_invalid`. 충돌 검사는 기본 포트를 붙인 뒤 적용(1.2·1.6절). [OPS-001](../specs/operations.md#7-수용-테스트) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·`crates/domain/src/allow_list.rs` 단위 테스트, 계약 테스트 파일 `accepts_single_cidr_and_range_items_of_both_families`, `rejects_bad_ip_items_with_their_own_codes`, `requires_allow_list_only_for_enabled_entry_points`, `accepts_an_empty_allow_list`, `accepts_items_covering_a_whole_family_with_warnings` | IP 항목: 단일·CIDR·범위 해석, 역순 범위·체계 혼합·표기 오류의 코드, 목록 누락은 오류, 빈 목록은 받음, 전체 허용 항목의 경고 대상 계산(1.3·2절). [OPS-038](../specs/operations.md#7-수용-테스트), [OPS-040](../specs/operations.md#7-수용-테스트)의 설정 검증 부분 | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·`tests/serve.rs` `every_relay_entry_point_accepts_only_its_own_source_ip`, `applies_item_boundaries_and_an_empty_list_at_accept`, `tests/reload.rs` `allow_list_modes_hold_at_start_and_at_reload` | 경계 주소의 허용·거절, 빈 목록의 모든 출발지 거절, 진입점별 목록, 전체 허용 경고 기록(1.3·2절). 첫 시험은 TCP Viewer·TCP Server·WS Viewer·WSS Viewer의 서로 다른 출발지 IP 4개로 16쌍을 검사한다. [OPS-037](../specs/operations.md#7-수용-테스트)(비부분), [OPS-038](../specs/operations.md#7-수용-테스트), [OPS-040](../specs/operations.md#7-수용-테스트) | 부분 | 커밋 `534d4f7`의 두 OS 게이트에서 OPS-037·OPS-038은 통과했다. OPS-040은 IPv6 출발지를 이 환경에서 만들 수 없어 부분이다(461행과 같다). [운영 정보 계약 실행 기록](ops-info-v1.md#계약-테스트-실행-기록) |
| Consumer·`crates/domain/src/origin.rs` 단위 테스트, 계약 테스트 파일 `checks_origin_values_and_entry_point_specific_keys` | Origin 문법과 정규화·정확 일치(1.4절). 진입점별로 쓸 수 있는 키 | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·`crates/adapters/tests/websocket.rs` `origin_must_equal_the_allowed_value`, `origin_must_equal_the_allowed_value_on_wss`, `allowed_source_with_disallowed_origin_is_refused_with_403`, `allowed_source_with_disallowed_origin_is_refused_with_403_on_wss` | 업그레이드 요청의 Origin 비교. [WS-027](../specs/websocket.md#7-수용-테스트), [WS-015](../specs/websocket.md#7-수용-테스트) | 통과 | 커밋 `534d4f7`의 두 OS 게이트에서 WS-015·WS-027 통과. 시험 내용은 [WebSocket 중계 계약 5절](websocket-relay-v1.md#5-예제와-계약-테스트)에도 있다 |
| Consumer·`crates/domain/src/target_policy.rs` 단위 테스트, 계약 테스트 파일 `checks_mode1_targets`, `accepts_tls_example_and_resolves_relative_paths` | Mode I 대상 항목 문법과 세부 원인(1.5절), `mode1.enabled = true`일 때 필수 | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·`tests/relay_target_scenarios.rs` `forbidden_ip_and_forbidden_port_never_create_target_connections`·`allowed_hostname_cannot_connect_to_a_forbidden_resolved_ip`, `tests/relay_entry_scenarios.rs` `disabled_modes_reject_before_waiting_registration_or_target_connect` | 최종 IP 판정(1.5절). 꺼진 모드 거절. [RLY-007](../specs/relay.md#7-수용-테스트), [RLY-008](../specs/relay.md#7-수용-테스트), [RLY-022](../specs/relay.md#7-수용-테스트) | 통과 | `ee0314f`의 두 OS 전체 게이트. 금지 IP·포트와 금지된 최종 IP로 대상 연결을 만들지 않는지 확인한다. 꺼진 모드는 대기 등록·대상 연결 전에 거절한다. [작업 5.9 판정표](../how-to/set-up-and-verify.md#41-작업-59-판정표) |
| Consumer·계약 테스트 파일 `allows_plaintext_management_only_on_loopback`, `warns_for_plaintext_ws_and_non_loopback_status` | 평문 관리 리스너는 루프백만, 관리 TLS 파일 쌍, 평문 WS·비루프백 상태 리스너의 경고 대상 계산(1.6·2.2절) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·`tests/serve.rs` `refuses_plaintext_management_on_a_public_address_with_exit_code_3` | 비루프백 평문 관리 설정은 시작 실패다. 종료 코드 3, `error[config_plaintext_management_not_loopback] key=management.listen` 한 줄, stdout에 `startup_failed`(`config_key`는 `management.listen`), 포트를 잡지 않음(1.6·2.1절). 루프백 평문 시작 뒤 로그인 없는 401은 `answers_api_queries_with_401_without_login_on_loopback_plaintext`가 확인한다([관리 API 계약 5절](management-api-v1.md#5-예제와-계약-테스트)). 두 조건은 `refuses_public_plaintext_and_answers_login_less_queries_with_401_on_loopback`(OPS-034, 비부분)이 함께 실행한다. [OPS-014](../specs/operations.md#7-수용-테스트) 통과(`public_management_never_starts_on_plaintext_or_without_tls_or_an_account`, 비부분: 세 시작 거절과 공개 HTTPS의 평문 무응답·로그인 없는 401을 확인한다), [OPS-034](../specs/operations.md#7-수용-테스트) 통과 | 통과 | [관리 리스너 대조 기록](../architectures/management-listener.md#대조-기록) |
| Consumer·`tests/serve.rs` `plaintext_ws_listener_is_off_by_default_and_warns_at_start_when_enabled` | 평문 WS는 명시적으로 켠 경우만 열림, 경고 기록. 켜지 않으면 다른 진입점(WSS·관리 HTTPS·TCP)이 평문 업그레이드를 받지 않는다. [OPS-016](../specs/operations.md#7-수용-테스트), [WS-024](../specs/websocket.md#7-수용-테스트) | 통과 | 커밋 `ad5cfb2`. [운영 정보 계약 실행 기록](ops-info-v1.md#계약-테스트-실행-기록) |
| Consumer·계약 테스트 파일 `rejects_empty_paths_and_reads_log_output`, `load_resolves_relative_paths_against_the_config_directory` | 빈 경로 거절, 상대 경로는 설정 파일 디렉터리 기준, `log.output`의 `"stdout"`과 경로 구분(1.1·1.2절) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·`tests/serve.rs` `fails_with_exit_code_3_when_the_log_file_cannot_be_opened`, `writes_the_log_to_the_configured_file_from_start_to_stop` | `log.output` 파일을 열 수 없으면 `log_file_unavailable`(`key=log.output`)로 시작 실패, 종료 코드 3. 열 수 있으면 기존 내용 뒤에 이어 쓴다(1.2·2.1절) | 통과 | [운영 정보 계약 실행 기록](ops-info-v1.md#계약-테스트-실행-기록) |
| Consumer·재로드 통합 테스트 `tests/reload.rs` 10건과 단위 시험(`file_watch.rs`, `reload/service.rs`, `src/serve.rs`) | 재로드 대상과 재시작 필요 구분, 비재로드 차이가 남은 후보 거절, 후보 실패 시 기존 유지, 주기 확인(1.8·2.3절), 시작 때 감시 등록 실패(`watch_unavailable`). [OPS-003](../specs/operations.md#7-수용-테스트), [OPS-024~028](../specs/operations.md#7-수용-테스트), [OPS-035](../specs/operations.md#7-수용-테스트), [OPS-038](../specs/operations.md#7-수용-테스트), [OPS-040](../specs/operations.md#7-수용-테스트), [OPS-041](../specs/operations.md#7-수용-테스트), [WS-022](../specs/websocket.md#7-수용-테스트). 완전: OPS-003, OPS-024, OPS-026, OPS-035, OPS-038. 완전(단위 시험): OPS-027, OPS-028, OPS-041. 부분: OPS-025, OPS-040(IPv6 출발지는 환경 한계), WS-022(업그레이드 시점만) | 부분 | [운영 정보 계약 실행 기록](ops-info-v1.md#계약-테스트-실행-기록) |
| Consumer·`crates/adapters/src/net.rs` `sets_keepalive_from_the_settings_and_turns_nagle_off`; `tests/relay_dead_peer_linux.rs` `removes_unresponsive_waiting_server_and_reuses_id_and_l`·`removes_unresponsive_active_peers_without_harming_other_sessions`; `tests/relay_log_scenarios.rs` `idle_active_sessions_survive_longer_than_k_and_transfer_data`; `crates/adapters/tests/websocket_scenarios.rs` `ws_and_wss_kw_keeps_idle_peers_and_drops_unresponsive_peers`; `tests/unix_shutdown.rs`·`tests/windows_console.rs`의 종료 유예 시험; `crates/adapters/tests/management.rs` `limited_logins_refuse_alike_and_the_right_password_works_after_the_locks`·`idle_polling_and_absolute_activity_expiry_leave_existing_relays_alive`; `tests/management_query.rs` `recent_closed_fifo_overflow_and_restart_clears_list`; `tests/dashboard_live.rs` `dashboard_022_real_configured_and_default_polling_on_both_screens` | K·Kw·G·F·Si·Sa·P·R 값이 각 기능에 반영. [RLY-025~027](../specs/relay.md#7-수용-테스트), [WS-026](../specs/websocket.md#7-수용-테스트), [OPS-008](../specs/operations.md#7-수용-테스트)·[OPS-009](../specs/operations.md#7-수용-테스트)·[OPS-042](../specs/operations.md#7-수용-테스트), [MGT-022](../specs/management.md#7-수용-테스트), [MGT-024](../specs/management.md#7-수용-테스트), [MGT-025](../specs/management.md#7-수용-테스트), [DASH-022](../specs/dashboard.md#7-수용-테스트) | 통과(지원 판정) | `ee0314f`의 두 OS 전체 게이트와 [작업 5.9 판정표](../how-to/set-up-and-verify.md#41-작업-59-판정표). RLY-025·026의 실제 끊긴 네트워크 시험은 Linux 전용이다. Windows 부분 증거의 인정 범위는 Q86·Q99를 따른다. |
| Producer·`tests/distribution.rs` `repository_example_loads_without_secrets`, `example_cli_account_relay_reload_and_graceful_stop`(Linux), `tests/windows_console.rs`의 같은 실행파일 시험(Windows) | [`examples/relay.toml`](../../examples/relay.toml)을 실제 로더로 읽는다. 상대 경로·안전한 진입점·비밀과 기본 비밀번호 없음도 확인한다. 예시 복사본으로 CLI 계정 준비·시작·중계·IP 재로드·종료를 확인한다. [OPS-031](../specs/operations.md#7-수용-테스트) 부분. 전체 판정에는 두 OS의 [수동 안내](../how-to/operate-relay.md) 실행 기록도 필요하다. | 통과 | 커밋 `1c7c7a9`의 두 OS 게이트. 두 OS 수동 실행은 예시를 고치지 않고 묶어 시작했다(2026-11-05, [CLI 계약 실행 기록](cli-v1.md#계약-테스트-실행-기록)) |
| Producer·Consumer / 확대 재로드·설정 편집 계약 시험 | 27개 재로드 대상·23개 편집 대상과 꺼진 진입점·모드 제외(1.8절), `config_edit` 기본값·재시작 구분(1.2절), 주석·서식·줄바꿈 보존·없는 키 추가·원자적 교체·SHA-256 revision(1.10절), v2 마이그레이션·롤백(3.2절). [OPS-047~064](../specs/operations.md#7-수용-테스트) | 통과 | 2026-11-08 작업 트리: `crates/adapters/tests/config_contract.rs`의 `config_edit_is_opt_in_and_requires_a_boolean`·`candidate_presence_distinguishes_disabled_omissions_and_explicit_empty_lists`, `config_edit_document.rs`의 SHA·서식 보존·배열 주석·inline/dotted table·없는 키/테이블·실파일 교체·실패 시험을 연결했다. 대표 함수는 `raw_revision_uses_sha256_and_includes_comments_bom_and_line_endings`, `scalar_edits_preserve_bom_comments_decor_and_mixed_newlines_exactly`, `array_shrink_empty_and_grow_keep_all_comments_and_parse_as_requested`, `absent_keys_use_existing_tables_and_absent_tables_append_with_first_newline`, `real_file_replacement_saves_exact_bytes_preserves_permissions_and_leaves_no_temp`다. `config_edit.rs` 단위 시험은 Q178의 마지막 재확인 직전 외부 저장·읽기 실패와 임시 쓰기·교체 실패를 넣는다. 실행 결과는 아래 작업 6 설정 편집 기록을 따른다. |

### 계약 테스트 실행 기록

| 대상 | 환경 | 명령 | 결과 |
| --- | --- | --- | --- |
| 커밋 `4e594f4`에 작업 2.2를 더한 작업 트리(`[admin.argon2]` 범위를 domain 상수로 옮김 포함) | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행) | 통과. `config_contract` 21건, adapters 단위 2건, domain 단위 15건 모두 통과. 시나리오 보고는 [CLI 계약 실행 기록](cli-v1.md#계약-테스트-실행-기록)에 있다. |
| 커밋 `f25983a`에 작업 2.3을 더한 작업 트리 | 같음 | 같음 | 통과. `config_contract` 21건과 `tests/serve.rs` 10건 모두 통과. 시나리오 보고는 [운영 정보 계약 실행 기록](ops-info-v1.md#계약-테스트-실행-기록)에 있다. |
| 같음 | Windows 11 | 같음 | 미실행. Visual Studio Build Tools 설치가 UAC 미승인으로 멈춰(종료 코드 1602) MSVC 링커가 없다([개발 환경 how-to](../how-to/set-up-and-verify.md)). 사용자가 Windows 검증을 뒤로 미뤘다(2026-10-20). Windows 경로 분기(`cfg!(windows)`)는 실행하지 않았다. |
| 커밋 `e1cfc4f`에 Windows 경고 수정(`crates/adapters/src/net.rs`·`tests/serve.rs`의 `cfg`)을 더한 작업 트리 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 17.14.37806.7, `AWS_LC_SYS_PREBUILT_NASM=1` | 같음 | 통과. `config_contract` 21건과 adapters 단위 5건 모두 통과. 테스트의 Windows 경로 분기(`cfg!(windows)`: `C:\relay`, `D:\logs\relay.log`)를 실행했다. 시나리오 보고는 [운영 정보 계약 실행 기록](ops-info-v1.md#계약-테스트-실행-기록)에 있다. |
| 커밋 `e701997`(작업 3.1 검증. 이 커밋들은 설정 계약 시험을 바꾸지 않았다). 작업 트리 깨끗함 | Ubuntu 24.04.4(WSL2), Rust 1.99.1 · Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`) | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` · `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | Ubuntu `ALL PASSED`, Windows `ALL PASSED (all)`. 두 실행 모두 전체 `cargo test --workspace`를 포함하며 통과했다. 시나리오 보고는 [운영 정보 계약 실행 기록](ops-info-v1.md#계약-테스트-실행-기록)에 있다. |
| 커밋 `307cfd8`(작업 5.2 최종 게이트. 코드는 `3eea38f`와 같고 뒤에 문서 커밋만 더해졌다). 게이트 실행 때 추적 파일 변경 없음 | Ubuntu 24.04.4(WSL2), Rust 1.99.1 · Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`) | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` · `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | Ubuntu `ALL PASSED`, Windows `ALL PASSED (all)`. 두 실행 모두 전체 `cargo test --workspace`를 포함하며 통과했다. 시나리오 보고는 [운영 정보 계약 실행 기록](ops-info-v1.md#계약-테스트-실행-기록)에 있다. 대조는 [file-watch 작업 5.2 게이트 실행 기록](../architectures/file-watch.md#작업-52-게이트-실행-기록)에 있다. |
| 커밋 `553b549`에 작업 1.5 변경(2.0.0: `limits.initial_message_bytes`와 관계 I ≤ B 제거, 파일 이름 `config-v2.md`)을 더한 작업 트리 | Ubuntu 24.04.4(WSL2), Rust 1.99.1 · Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`) | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` · `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | Ubuntu `ALL PASSED`, Windows `ALL PASSED (all)`. 두 실행 모두 `config_contract` 21건이 통과했다. `checks_relations_between_values`는 `initial_message_bytes`를 쓴 파일이 `config_unknown_key`로 거절되는 것을 확인한다(3.1절). 시나리오 보고 판정은 두 OS 모두 `통과`(150건, 실패 0)다. |
| 커밋 `fd8c097`(`tests/reload.rs` OPS-024 시험의 D를 단조 시계로 잼. 운영 코드 변경 없음) | Ubuntu 24.04 LTS(WSL2), Rust 1.99.1 · Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`) | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` · `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | Ubuntu `ALL PASSED`(보고 2026-11-02T15:43:31Z, 통과 83, 부분 55, 실패 0, 미연결 12), Windows `ALL PASSED (all)`(보고 2026-11-02T15:50:17Z, 통과 77, 부분 61, 실패 0, 미연결 12). `tests/reload.rs` 10건이 두 OS에서 통과했다. 대조는 [파일 변경 감지 설계의 OPS-024 시험 시간 측정 수정](../architectures/file-watch.md#ops-024-시험의-시간-측정-수정)에 있다. |
| 커밋 `1c7c7a9`(작업 5.6 설정 예시와 배포 시험). 작업 트리 깨끗함(Ubuntu 보고의 `+ 미커밋 변경`은 CRLF와 추적되지 않는 게이트 로그 파일) | Ubuntu 24.04.4(WSL2), Rust 1.99.1 · Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`) | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` · `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | Ubuntu `ALL PASSED`(보고 2026-11-04T15:43:30Z, 통과 109, 부분 36, 실패 0, 미실행 1, 미연결 6), Windows `ALL PASSED (all)`(보고 2026-11-04T15:35:46Z, 통과 105, 부분 41, 실패 0, 미실행 0, 미연결 6). `repository_example_loads_without_secrets`는 두 OS에서, 예시 복사본 실행파일 시험은 각 OS 파일에서 통과했다. release 묶음의 수동 실행은 [CLI 계약 실행 기록](cli-v1.md#계약-테스트-실행-기록)에 있다 |
| 커밋 `534d4f7`(작업 3.2 OPS-006·OPS-037 실행파일 시험). 작업 트리 깨끗함(Ubuntu 보고의 `+ 미커밋 변경`은 CRLF 차이) | Ubuntu 24.04.4(WSL2), Rust 1.99.1 · Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`) | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` · `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | Ubuntu `ALL PASSED`(보고 2026-11-04T16:22:56Z, 통과 111, 부분 34, 실패 0, 미실행 1, 미연결 6), Windows `ALL PASSED (all)`(보고 2026-11-04T16:15:07Z, 통과 107, 부분 39, 실패 0, 미실행 0, 미연결 6). H=1초·U=2초를 쓴 OPS-006 시험과 진입점별 허용 목록 OPS-037 시험이 두 OS에서 통과했다. OPS-040은 부분 그대로다 |
| 커밋 `eae442f`(작업 6 설정 편집 기록. 구현 `7ce077f`, 시험 수정 `0bc9a47`, 수락 쪽 K 시험 `eae442f`). Windows 작업 트리 깨끗함, Ubuntu 보고의 `+ 미커밋 변경`은 줄 끝 차이 | Ubuntu 24.04.4(WSL2), Rust 1.99.1 · Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`) | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` · `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | Ubuntu `ALL PASSED`(보고 2026-11-08T09:59:46Z, 215건: 통과 201, 부분 8, 실패 0, 미실행 4, 미연결 2), Windows `ALL PASSED (all)`(보고 2026-11-08T09:56:59Z, 215건: 통과 198, 부분 12, 실패 0, 미실행 3, 미연결 2). 두 실행 모두 전체 `cargo test --workspace`를 포함하며, 위 확대 재로드·설정 편집 계약 시험이 통과했다. OPS-047~064 가운데 OPS-050을 뺀 17건은 두 OS에서 통과다. OPS-050은 두 OS에서 부분이며 [작업 6.9 판정표](../how-to/set-up-and-verify.md#42-작업-69-판정표)의 조항별 시험으로 통과로 판정했다. 같은 구현의 첫 게이트(`7ce077f`)는 시험 결함 두 개로 실패했고 `0bc9a47`이 고쳤다. |

## 6. 초안을 확정하기 전에 해결할 사항

- (해결) 설정 보기·편집의 설정 키 `management.config_edit`는 사용자 결정 2026-11-08(Q176)으로 채택해 1.2절에 반영했다. 관련 API·사건 이름은 [관리 API 계약](management-api-v1.md)·[운영 정보 계약](ops-info-v1.md)에 같은 결정으로 반영했다. 구현·계약 테스트는 미실행(구현 전)이다. 후속: 커밋 `eae442f`의 두 OS 게이트에서 구현과 계약 테스트가 통과했다(5절).

- 이 계약의 이름·코드 선택(키 이름, 오류·경고 코드, 테이블 구조)은 2026-10-16에 사용자가 채택했다. 키 이름은 공유 인터페이스 이름을 따른다. 인증서 파일 오류(`tls_file_invalid`)의 시작 종료 코드는 3으로 확정했다. 리스너 바인드 실패는 4다.
- 허용 암호 스위트는 선택한 rustls 암호 모듈의 rustls 기본 목록으로 확정했다(1.9절). 암호 모듈은 aws-lc-rs로 정했다(사용자 결정 2026-10-20). 두 OS 빌드 확인은 [운영 스펙 8절](../specs/operations.md#8-미결정-사항)의 OPS-TBD-009에 기록했다. 개인키 종류의 지원 범위는 작업 3.1에서 `crates/adapters/tests/tls.rs`로 확인해 1.9절에 적었다.
- (해결) `buffer_bytes`(B)와 `ws_message_bytes`(M)의 관계: B와 M 사이에 크기 제약은 없다(사용자 결정 2026-11-01(Q78)). 프레임 안에서 스트리밍하며 방향당 미전달 보관 바이트는 B 이하다. 분할 프레임을 합친 논리 메시지 크기가 M을 넘으면 1009로 닫고 추가 전달을 멈춘다. B보다 큰 유효 프레임도 수락해 스트리밍하며 전체를 저장하지 않는다. [WS-TBD-004](../specs/websocket.md#8-미결정-사항)를 해소했다.
- 알림을 놓친 변경의 최대 지연은 Dp + D다([운영 스펙 3.3절](../specs/operations.md#33-파일-변경-자동-감지와-부분-재로드), [OPS-041](../specs/operations.md#7-수용-테스트)). 변경 안정화·교체 저장 감지·동시성의 실제 방식은 [파일 변경 감지 모듈 설계](../architectures/file-watch.md)에 적는다.
- Origin 정규화(1.4절)는 채택했으며 WebSocket 계약 1.3절이 이 규칙을 인용한다. 두 계약이 같은 값을 쓴다.
- Windows 파일 권한 설정은 관찰로 확인 필요하다. Windows 경로 처리는 계약 테스트로 확인했다([실행 기록](#계약-테스트-실행-기록)). 로그 파일 교체 뒤 다시 열기는 사용자 결정 2026-11-04(Q104·Q109·Q110)로 정했다. 2026-11-04 Windows 11에서 제품과 같은 열기 방식(`OpenOptions::append(true).create(true)`)의 작은 프로그램으로 관찰했다. 쓰는 중에도 이름 바꾸기와 삭제가 됐고, 이름을 바꾼 뒤에는 옛 파일에 계속 썼다.
- 배포 설정 예시는 [`examples/relay.toml`](../../examples/relay.toml)에 둔다(사용자 결정 2026-11-04(Q106)). 루프백 TCP Mode II와 루프백 관리 HTTP로 시작하며 평문 WS는 끈다. 공개 배치의 주소·IP 정책·TLS 파일은 운영자가 준비한다. 이 예시 구성은 이 문서의 판단, 사용자 결정 없음이다. 실제 로더·비밀 검사는 5절의 배포 시험으로 확인한다.
