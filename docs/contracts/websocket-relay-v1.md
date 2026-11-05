# WebSocket 중계 v1 계약

외부 WebSocket 클라이언트(noVNC 포함)가 `vnc-relay`의 WS·WSS 진입점에 접속해 중계 스트림을 주고받을 때의 교환 형식을 정한다. 이 계약은 업그레이드 요청과 거절, 서브프로토콜, Origin 비교, 데이터 메시지와 제어 프레임, 종료 코드와 종료 의미를 다룬다. 초기 교환(Repeater) 바이트는 [Repeater 초기 교환 계약](repeater-handshake-v1.md)이 정한다. 이 문서는 그 바이트를 다시 정의하지 않는다.

이 계약은 2026-10-16에 사용자가 채택했다(상태 Active). HTTP 상태, close 코드 번호, 이름, 서브프로토콜 처리 세부, 검사 순서, 종료 의미가 모두 채택 범위다. 스펙이 이 계약에 남겨 두었던 WS-TBD-002(HTTP 상태·종료 코드)와 WS-TBD-003(Origin 정규화)은 이 채택으로 해소했다. 6절에 관찰·구현에서 확인한다고 남긴 항목은 채택으로 해소되지 않았다.

| 항목 | 값 |
| --- | --- |
| 이름과 버전 | `websocket-relay` 1.1.0. 1.1.0은 새 close 코드 4011 `relay_connection_limit`을 더하고(사용자 결정 2026-11-01(Q79)), 클라이언트 Close 수신 시 전송 중 프레임 마무리 규칙(1.6절 1행, 사용자 결정 2026-11-01(Q83))과 강제 정리 시 1001 발송 및 종료 원인 보존 규칙(1.5·1.6·2.2절, 사용자 결정 2026-11-01(Q82))을 명시했다. 3절에 따른 호환 변경이다. 1.0.0 이력: H 시작점과 검사 순서의 명확화(사용자 결정 2026-10-22(Q22), 2026-10-22(Q24)), Repeater 초기 교환 계약 채택에 따른 4008 문장·시작 바이트 반영(사용자 결정 2026-11-01(Q71~Q73)), noVNC 1.7.0 관찰 결과 반영(2026-11-01). |
| 상태 | Active(2026-10-16 채택) |
| 소유자 | vnc-relay 프로젝트 관리자 |
| 만드는 쪽(Producer) | 외부 WebSocket 클라이언트(요청·바이너리 메시지·Close 송신), `vnc-relay` WS·WSS 리스너(응답·바이너리 메시지·ping·Close 송신). 방향마다 만드는 쪽이 다르며, 각 절에 방향을 적는다. |
| 사용하는 쪽(Consumer) | `vnc-relay` WS·WSS 리스너(요청 수신), 외부 WebSocket 클라이언트(응답 수신) |
| 방향 | 양방향. HTTP 업그레이드는 요청·응답이고, 이후는 WebSocket 메시지 교환이다. |
| 스키마 기준 파일 | 없음. 아래 Markdown 표가 형식의 기준이다. WebSocket 프레임 형식은 [RFC 6455](https://www.rfc-editor.org/rfc/rfc6455)를 따른다. |
| 기준 스펙 | [WS·WSS 전송](../specs/websocket.md), [TCP 중계와 연결 매칭](../specs/relay.md) |
| 관련 요구사항 | [PRD](../requirements/PRD.md) FR-05, 7절 보안 경계, AC-05~08, AC-16~17 |
| 호환성 범위 | WS(`ws://`)와 WSS(`wss://`) Viewer 진입점. 서버 쪽은 Mode I·II 모두. 클라이언트는 RFC 6455 클라이언트이며, 검증 대상은 고정 버전 noVNC의 `repeaterID` 옵션이다([WS-025](../specs/websocket.md#7-수용-테스트)). WS·WSS Server 역접속은 범위 밖이다. |
| 관련 테스트 | [WS-001~027](../specs/websocket.md#7-수용-테스트). 연결은 2절과 5절에 있다. |

관련 문서: [Repeater 초기 교환 계약](repeater-handshake-v1.md), [설정 계약](config-v2.md)(키 이름·값 범위, Origin 값 문법과 정규화), [관리 API 계약](management-api-v1.md)(관리자 종료·종료 원인 `close_cause`), [운영 정보 계약](ops-info-v1.md)(종료 원인·로그), [ADR-0001 내장 TLS](../decisions/ADR-0001-built-in-tls.md), [ADR-0002 출발지 IP 허용 목록](../decisions/ADR-0002-source-ip-allowlist.md).

## 1. 형식과 의미

### 1.1 업그레이드 요청

클라이언트가 `vnc-relay`에 보내는 HTTP 요청이다. WSS는 TLS 1.2 이상이 성립한 뒤에 보낸다([ADR-0001](../decisions/ADR-0001-built-in-tls.md), [OPS-R17](../specs/operations.md#41-판정-가능한-규칙)).

| 요소 | 값·규칙 | 어긋날 때 |
| --- | --- | --- |
| 업그레이드 경로 | 요청 대상의 경로가 정확히 `/relay`여야 한다. 대소문자를 구분한다. 끝 슬래시(`/relay/`)·퍼센트 인코딩·점 세그먼트(`/./relay`)를 같은 경로로 보지 않는다. 쿼리 문자열은 해석하지 않으며 대상이나 ID 선택에 쓰지 않는다. | HTTP 404 |
| `/api/` 요청 | 관리 API 경로(`/api/` 이하)는 WS·WSS 리스너에서 처리하지 않는다. 다른 경로와 같이 404다. 관리 리스너는 별도다([WS-R08](../specs/websocket.md#3-정상-흐름과-규칙), [WS-012](../specs/websocket.md#7-수용-테스트)). | HTTP 404 |
| 메서드 | `GET`만 허용한다. | HTTP 405, `Allow: GET` |
| HTTP 버전 | HTTP/1.1 이상. | HTTP 400 |
| `Host` | 필수. 값은 라우팅에 쓰지 않는다. | HTTP 400 |
| `Upgrade` | 필수. 값 `websocket`(대소문자 무시). | HTTP 400 |
| `Connection` | 필수. 토큰 `Upgrade`를 포함한다(대소문자 무시). | HTTP 400 |
| `Sec-WebSocket-Key` | 필수. base64 값이며 디코드하면 16바이트여야 한다([RFC 6455 4.2.1절](https://www.rfc-editor.org/rfc/rfc6455#section-4.2.1)). | HTTP 400 |
| `Sec-WebSocket-Version` | 필수. 값 `13`. | HTTP 426, 응답 헤더 `Sec-WebSocket-Version: 13` ([RFC 6455 4.2.2절](https://www.rfc-editor.org/rfc/rfc6455#section-4.2.2)) |
| `Origin` | 선택. 비교와 처리는 1.3절이 정한다. | 불일치 HTTP 403, 중복 HTTP 400 |
| `Sec-WebSocket-Protocol` | 선택. 1.2절 규칙을 따른다. | HTTP 400 |
| `Sec-WebSocket-Extensions` | 선택. 서버는 어떤 확장도 수락하지 않는다. 요청에 있어도 거절하지 않고, 응답에 이 헤더를 넣지 않는다. | 해당 없음 |
| 그 밖의 헤더 | 무시한다. `Cookie`, `Authorization`, `X-Forwarded-For`, `Forwarded`는 접근 판정에 쓰지 않는다. 출발지 IP는 항상 실제 소켓 상대 주소다([ADR-0002](../decisions/ADR-0002-source-ip-allowlist.md), [WS-014](../specs/websocket.md#7-수용-테스트)). | 해당 없음 |

검사 순서. 한 요청이 여러 조건을 어길 때 먼저 걸리는 상태를 하나만 돌려준다.

1. 접속 수락 직후: 출발지 IP 허용 목록. 실패하면 TLS·HTTP 처리 전에 연결을 닫는다. HTTP 응답은 없다.
2. WSS: TLS 협상. 실패하면 연결이 성립하지 않는다.
3. 요청 구문, 경로, 메서드, 업그레이드 헤더와 버전, 키. 이 단계 안의 순서는 다음과 같다(사용자 결정 2026-10-22(Q24)).
   - 가. 요청 구문. HTTP 400이다.
   - 나. 경로(404), 메서드(405), HTTP 버전(400).
   - 다. `Host`, `Upgrade`, `Connection` 헤더(400).
   - 라. `Sec-WebSocket-Version`(426).
   - 마. `Sec-WebSocket-Key`(400).
4. `Origin`.
5. 서브프로토콜.
6. 업그레이드 직전 출발지 IP 재검사. 그 순간의 유효한 목록을 쓴다.
7. 모두 통과하면 `101`을 보낸다.

TLS 협상과 업그레이드 단계에는 제한 시간 U(`tls_upgrade_secs`)가 적용된다. `101` 전에 U가 지나면 HTTP 응답 없이 연결을 닫는다. 출발지 IP 허용 목록 값의 형식은 [설정 계약](config-v2.md)이 정한다.

H(`handshake_secs`)는 초기 교환 제한 시간이다. WS·WSS에서는 `101` 응답을 보낸 뒤부터 잰다. 101 전의 시간은 U가 맡는다(사용자 결정 2026-10-22(Q22)).

### 1.2 업그레이드 응답과 서브프로토콜

성공 응답은 `HTTP/1.1 101 Switching Protocols`이며 `Upgrade: websocket`, `Connection: Upgrade`, `Sec-WebSocket-Accept`를 담는다([RFC 6455 4.2.2절](https://www.rfc-editor.org/rfc/rfc6455#section-4.2.2)). 응답은 `Set-Cookie`를 담지 않는다.

서브프로토콜은 클라이언트가 제시한 값 목록(여러 헤더면 합친 목록)으로 정한다. 값은 정확히 일치하는지만 본다(대소문자 구분).

| 클라이언트가 제시한 값 | 서버 처리 |
| --- | --- |
| `binary`를 포함 | 응답에 `Sec-WebSocket-Protocol: binary`를 넣고 수락한다. |
| 헤더 없음 | 응답에 `Sec-WebSocket-Protocol`을 넣지 않고 수락한다. |
| 값이 있으나 `binary`가 없음 | 업그레이드하지 않고 HTTP 400으로 거절한다. |

[RFC 6455 4.2.2절](https://www.rfc-editor.org/rfc/rfc6455#section-4.2.2)은 서버가 응답에 클라이언트가 제시하지 않은 서브프로토콜을 넣지 못하게 한다. 위 규칙은 그에 맞다.

noVNC 호환: 고정 버전 noVNC 1.7.0([호환성 검증 기록](repeater-compat-record.md) 2.4절)은 WS와 WSS 모두 요청에 `Sec-WebSocket-Protocol` 헤더를 넣지 않았다(같은 기록 3.4절 W-02·W-03b, 2026-11-01 관찰). 위 표의 "헤더 없음" 행에 해당하므로 서버는 응답에 이 헤더를 넣지 않고 수락한다. 이 결과는 noVNC 1.7.0의 `vnc.html`을 `wsProtocols` 설정 없이 쓴 경우에 한정한다. 다른 버전이나 다른 설정의 noVNC는 관찰하지 않았다.

응답 본문은 중계 ID, 허용 목록, 내부 정보를 담지 않는다.

### 1.3 Origin 비교

허용 값(`allowed_origins`)의 문법과 `Origin` 값의 정규화·비교 규칙은 [설정 계약 1.4절](config-v2.md#14-origin-값-문법과-비교)이 정한다. 이 계약은 비교 결과에 따른 업그레이드 처리만 정한다. 정규화 규칙은 설정 계약과 함께 채택했다. Origin은 사용자 인증 수단이 아니다.

| 비교 결과 | 처리 |
| --- | --- |
| `Origin` 헤더 없음 | Origin 검사를 건너뛰고 출발지 IP 검사만 한다. |
| 하나이며 허용 값과 일치 | 다음 검사로 넘어간다. |
| 하나이며 허용 값과 불일치(`allowed_origins`가 빈 배열이어서 모든 값이 불일치인 경우 포함) | HTTP 403(2.1절). |
| 둘 이상 | HTTP 400(2.1절). |

예(허용 값 `https://a.example:8443`): `https://a.example:8443`은 통과, `http://a.example:8443`(스킴 다름)·`https://a.example:9443`(포트 다름)·`https://x.a.example:8443`(하위 도메인)은 거절한다([WS-027](../specs/websocket.md#7-수용-테스트)).

### 1.4 데이터 메시지와 제어 프레임

`101` 이후 모든 교환은 WebSocket 프레임이다. 프레임 형식·마스킹·분할은 [RFC 6455](https://www.rfc-editor.org/rfc/rfc6455)를 따른다.

| 요소 | 방향 | 규칙 |
| --- | --- | --- |
| 바이너리 메시지 | 양방향 | 페이로드를 연속된 바이트 스트림으로 취급한다. 여러 메시지의 내용과 순서를 보존한다. 메시지·프레임 경계는 RFB 또는 Repeater 메시지의 경계가 아니다([WS-R02](../specs/websocket.md#3-정상-흐름과-규칙), [WS-R03](../specs/websocket.md#3-정상-흐름과-규칙), [WS-007](../specs/websocket.md#7-수용-테스트), [WS-018](../specs/websocket.md#7-수용-테스트)). 분할(연속 프레임)을 허용한다. 길이 0인 메시지는 허용하며 스트림에 바이트를 더하지 않는다. |
| 시작 바이트 | 서버 → 클라이언트 | `101` 응답 직후 [Repeater 초기 교환 계약 1.1절](repeater-handshake-v1.md#11-메시지)의 시작 바이트 12바이트(`RFB 000.000\n`)를 바이너리 메시지 하나로 보낸다. 서버가 보내는 첫 데이터 메시지다. 클라이언트는 이것을 받은 뒤 초기 요청을 보낸다. |
| 텍스트 메시지 | 클라이언트 → 서버 | 지원하지 않는다. 첫 프레임이 텍스트이면 즉시 종료 코드 1003으로 닫는다. 텍스트는 TCP에 전달하지 않는다([WS-R04](../specs/websocket.md#3-정상-흐름과-규칙), [WS-008](../specs/websocket.md#7-수용-테스트)). 서버도 텍스트 메시지를 보내지 않는다. |
| 메시지 크기 M | 클라이언트 → 서버 | 프레임이 아니라 분할을 합친 논리 메시지 하나에 적용한다. 설정 키 `limits.ws_message_bytes`([설정 계약](config-v2.md)). 크기 M은 허용하고 M+1부터 거절한다. 초과를 확인하면 종료 코드 1009로 닫고 추가 전달을 멈춘다. 스트리밍하므로 초과를 확인하기 전에 앞부분이 TCP에 이미 전달될 수 있다. 실패한 메시지를 원자적으로 취소하지 않는다([WS-R06](../specs/websocket.md#3-정상-흐름과-규칙), [WS-010](../specs/websocket.md#7-수용-테스트)). |
| 서버가 보내는 메시지 크기 | 서버 → 클라이언트 | 한 메시지는 M 이하로 나눠 보낸다. 입력 TCP 읽기 경계와 같을 필요는 없다. |
| ping | 서버 → 클라이언트 | 연결 성립 후 `keepalive.ws_ping_secs`마다 보낸다([설정 계약](config-v2.md)). 페이로드는 비워 두며 중계 ID나 세션 정보를 담지 않는다. |
| pong | 클라이언트 → 서버 | ping 하나마다 `keepalive.ws_pong_timeout_secs` 안에 pong이 와야 한다. 이 시간 안에 pong이 없으면 끊긴 연결로 처리한다. 데이터 메시지는 pong을 대신하지 않는다. 데이터가 없어도 pong에 응답하는 연결은 유지한다([WS-R13](../specs/websocket.md#3-정상-흐름과-규칙), [WS-026](../specs/websocket.md#7-수용-테스트)). |
| 클라이언트의 ping | 클라이언트 → 서버 | 서버는 pong으로 응답한다([RFC 6455 5.5.2절](https://www.rfc-editor.org/rfc/rfc6455#section-5.5.2)). 이미 Close를 받은 뒤에는 응답하지 않는다. |
| 요청하지 않은 pong | 클라이언트 → 서버 | 허용하며 무시한다. |
| ping·pong·Close 페이로드 | 양방향 | 어떤 제어 프레임의 페이로드도 TCP 중계 스트림에 넣지 않는다([WS-R05](../specs/websocket.md#3-정상-흐름과-규칙), [WS-009](../specs/websocket.md#7-수용-테스트)). |
| 잘못된 프레임·순서 | 클라이언트 → 서버 | 마스크 없는 클라이언트 프레임, 협상하지 않은 확장 비트, 알 수 없는 opcode, 연속 프레임 순서 위반, 분할되거나 125바이트를 넘는 제어 프레임, 유효하지 않은 Close 본문. 종료 코드 1002로 닫는다([WS-019](../specs/websocket.md#7-수용-테스트)). |

WS·WSS 하부 TCP 소켓에는 TCP keepalive(K)도 적용한다. 값은 이 계약이 아니라 [설정 계약](config-v2.md)과 [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)이 정한다.

### 1.5 close 코드

서버가 Close 프레임에 넣는 코드다. 표준 코드는 [RFC 6455 7.4.1절](https://www.rfc-editor.org/rfc/rfc6455#section-7.4.1)의 뜻을 따르고, 4000번대는 사설 범위로 이 계약이 정한 중계 사유다([RFC 6455 7.4.2절](https://www.rfc-editor.org/rfc/rfc6455#section-7.4.2)). 번호와 이름은 2026-10-16에 채택했다. 각 코드가 스펙의 어떤 오류와 시나리오에 대응하는지는 [2.2절](#22-close-코드와-스펙-오류의-대응)에 있다.

| 코드 | 이름 | reason 문자열 | 서버가 보내는 조건 |
| --- | --- | --- | --- |
| 1000 | 정상 종료 | 없음 | 상대 TCP가 정상 EOF를 보냈고 받은 데이터를 모두 전달한 뒤. 또는 클라이언트의 Close에 대한 응답. |
| 1001 | 서버 종료 | `server_shutdown` | 정상 종료 요청으로 서버가 내려가는 중이다. 초기 교환·대기 중인 연결과, 유예 만료 뒤(또는 유예 중 두 번째 종료 요청 뒤) 아직 정리를 시작하지 않은 활성 세션에 보낸다(사용자 결정 2026-11-01(Q82)). 이미 Close를 교환했거나 보낸 세션에는 다시 보내지 않는다. |
| 1002 | 프로토콜 오류 | 없음 | 1.4절의 잘못된 프레임·순서. |
| 1003 | 지원하지 않는 데이터 | 없음 | 텍스트 메시지 수신. |
| 1008 | 정책 위반 | `mode_disabled` | 설정으로 끈 모드의 초기 교환 요청. |
| 1009 | 메시지 너무 큼 | 없음 | 논리 메시지가 M을 초과. |
| 1011 | 서버 내부 오류 | `internal_error` | 서버의 예기치 않은 오류로 계속할 수 없음. |
| 4001 | 대기 시간 초과 | `wait_timeout` | Mode II 상대가 W 안에 도착하지 않음. |
| 4002 | 대상 연결 실패 | `target_connect_failed` | Mode I 대상의 주소 해석 실패, 연결 거절, 연결 시간 초과(C). |
| 4003 | 대상 비허용 | `target_not_allowed` | Mode I 대상의 최종 IP·포트가 허용 정책 위반. |
| 4004 | 중복 ID 거절 | `duplicate_id` | Mode II에서 점유된 ID에 같은 역할이 추가됨. 후발 연결에 보낸다. |
| 4005 | 관리자 종료 | `admin_terminated` | 관리자가 세션을 종료함. |
| 4006 | 끊긴 연결 감지 | `dead_connection` | 세션의 반대쪽(상대 TCP)이 끊긴 연결로 판정됨. 살아 있는 WS 쪽에 보낸다. |
| 4007 | 상대 연결 오류 | `peer_closed` | 상대 TCP가 오류(재설정 등)로 끝났거나, EOF 뒤 남은 데이터를 보낼 수 없음. |
| 4008 | 초기 교환 오류 | `handshake_invalid` | 초기 요청의 형식 오류([Repeater 초기 교환 계약 2절](repeater-handshake-v1.md#2-오류)). |
| 4009 | 초기 교환 시간 초과 | `handshake_timeout` | 초기 교환이 H 안에 끝나지 않음. |
| 4010 | 대기 상한 도달 | `waiting_full` | 대기 연결 상한 Lw 때문에 새 대기 등록을 거절. |
| 4011 | 중계 연결 상한 도달 | `relay_connection_limit` | Mode I에서 101 성공 후 대상 연결을 위한 중계 연결 상한 L 슬롯이 없음(사용자 결정 2026-11-01(Q79)). |

reason 문자열은 UTF-8의 고정 식별자만 쓴다. Close 본문은 코드 2바이트와 reason을 합쳐 125바이트 이하다([RFC 6455 5.5절](https://www.rfc-editor.org/rfc/rfc6455#section-5.5)). reason은 중계 ID, 주소, 내부 오류 문장을 담지 않는다. 클라이언트는 reason을 사용자에게 보이지 않아야 한다([RFC 6455 5.5.1절](https://www.rfc-editor.org/rfc/rfc6455#section-5.5.1)). 예약 코드(1005·1006·1015)는 서버가 보내지 않는다.

### 1.6 종료 의미

WebSocket에는 TCP 같은 독립적인 양방향 half-close가 없다. 그래서 WS·WSS가 포함된 세션은 한쪽 EOF 뒤 반대 방향을 정리 제한 시간 E(`timeouts.close_secs`, [설정 계약](config-v2.md)) 이상 유지하지 않는다([websocket.md 4절](../specs/websocket.md#4-오류와-종료)). 아래는 이 의미를 사건별로 고정한 것이다. 이 세부는 스펙이 계약에서 정하도록 남긴 항목이다.

| 사건 | 서버 동작 | 보장 범위 |
| --- | --- | --- |
| 클라이언트의 Close 수신 | 1. Close 앞의 모든 데이터 메시지를 TCP로 전달한다. Close 수신 전에 이미 헤더를 전송하기 시작한 전송 중 프레임의 나머지 페이로드만 E 안에서 끝까지 전송하고, 새 메시지는 시작하지 않는다. Close 수신 이후 TCP에서 읽은 데이터는 전달하지 않는다. 2. 상대 TCP의 쓰기 방향을 닫아 EOF를 전달한다. 3. 같은 코드로 Close에 응답한다. 코드가 없으면 1000으로 응답한다. 4. 상대 TCP의 EOF를 E까지 기다린 뒤 TCP를 닫는다(사용자 결정 2026-11-01(Q83)). | Close 뒤 TCP에서 오는 데이터는 WS로 보내지 못하며 버린다. 완전한 전달을 보장하지 않는다. E가 지나면 멈춘다. |
| 상대 TCP의 정상 EOF | 1. 이미 받은 TCP 데이터를 모두 바이너리 메시지로 전달한다. 2. Close 1000을 보낸다. 3. 클라이언트의 Close를 E까지 기다리는 동안 클라이언트가 보낸 데이터 메시지는 계속 TCP로 전달한다. 4. 클라이언트의 Close를 받거나 E가 지나면 TCP를 닫고 정리한다. | 받은 데이터를 소진하지 못하면 Close 4007로 중단하고 원인을 기록한다. |
| 상대 TCP 오류 | 즉시 Close 4007을 보낸다. 남은 데이터 전달을 보장하지 않는다. | 최선 노력. |
| 관리자 종료 | 새 데이터 수락을 멈추고 Close 4005를 보낸다. 상대 TCP를 즉시 닫는다. 클라이언트의 Close를 E까지 기다린 뒤 정리를 마치고 관리 API 응답을 한다. | 남은 데이터 전달을 보장하지 않는다. |
| 서버 종료 | 유예 중 자발적으로 끝나는 세션은 위 정상 흐름을 따른다. 초기 교환·대기 중인 연결과, 유예 만료 뒤(또는 유예 중 두 번째 종료 요청으로 남은 유예를 건너뛴 뒤) 아직 정리를 시작하지 않은 남은 세션에는 Close 1001을 보내고 정리한다(이미 Close를 교환했거나 보낸 세션에는 1001을 다시 보내지 않는다, 사용자 결정 2026-11-01(Q82)). | 유예 만료·즉시 강제 정리 때 남은 데이터 전달을 보장하지 않는다. |
| 상대 TCP가 끊긴 연결로 판정됨 | Close 4006을 보내고 정리한다. 반대쪽도 같은 세션에서 종료한다. | 최선 노력. |
| pong 제한 초과 | 클라이언트가 끊긴 연결이다. Close 프레임의 응답을 기다리지 않고 TCP 소켓을 닫고, 상대 TCP와 등록 자원을 회수한다. 클라이언트가 보는 종료는 코드 없는 비정상 종료다(RFC 6455의 1006은 클라이언트 쪽에서 붙이는 값이며 서버가 보내지 않는다). | 비정상 종료. |
| Close 교환 중 E 초과 | 응답이 없어도 TCP 소켓을 닫고 자원을 회수한다. 다른 세션에 영향을 주지 않는다. | 비정상 종료([WS-017](../specs/websocket.md#7-수용-테스트)). |

Close를 보낸 뒤에는 데이터 메시지를 보내지 않는다([RFC 6455 5.5.1절](https://www.rfc-editor.org/rfc/rfc6455#section-5.5.1)). Mode I·II 모두 같은 규칙이다. 서버는 세션을 자동으로 재접속·재매칭하지 않는다. 새 접속은 새 연결 수명이다.

## 2. 오류

### 2.1 HTTP 단계의 거절

`101` 전에 거절하는 경우다. `101` 뒤의 오류는 [2.2절](#22-close-코드와-스펙-오류의-대응)을 따른다. 응답 본문은 비우거나 고정 문장만 쓴다.

| 오류 코드 | 의미·형식 | 관련 스펙 시나리오 |
| --- | --- | --- |
| 연결 종료(HTTP 응답 없음) | 접속 수락 시 출발지 IP가 허용 목록에 없음. WSS는 TLS 협상 전에 닫는다. 쿠키·전달 헤더로 우회할 수 없다. | [WS-014](../specs/websocket.md#7-수용-테스트), [WS-023](../specs/websocket.md#7-수용-테스트), [websocket.md 4절](../specs/websocket.md#4-오류와-종료) 넷째 행 |
| 연결 종료(HTTP 응답 없음) | 중계 연결 상한 L 도달 시 새 접속. TLS·업그레이드 제한 시간 U 초과. | [OPS-004](../specs/operations.md#7-수용-테스트), [OPS-006](../specs/operations.md#7-수용-테스트) |
| TLS 협상 실패(HTTP 응답 없음) | 인증서 검증 실패, TLS 1.2 미만. 평문 WS로 전환하지 않는다. | [WS-013](../specs/websocket.md#7-수용-테스트), [OPS-036](../specs/operations.md#7-수용-테스트), [websocket.md 4절](../specs/websocket.md#4-오류와-종료) 첫·둘째 행 |
| `HTTP 400` | 요청 구문 오류, HTTP/1.0, 필수 헤더 누락·잘못된 값, `Origin` 중복, 허용되지 않는 서브프로토콜. 중계 등록·대상 연결 없음. | [WS-012](../specs/websocket.md#7-수용-테스트), [websocket.md 4절](../specs/websocket.md#4-오류와-종료) 셋째 행 |
| `HTTP 404` | 경로가 `/relay`가 아님. `/api/` 요청 포함. | [WS-012](../specs/websocket.md#7-수용-테스트) |
| `HTTP 405` | `GET`이 아닌 메서드. `Allow: GET`. | [WS-012](../specs/websocket.md#7-수용-테스트) |
| `HTTP 426` | `Sec-WebSocket-Version`이 13이 아님. `Sec-WebSocket-Version: 13`을 함께 보낸다. | [WS-012](../specs/websocket.md#7-수용-테스트) |
| `HTTP 403` | `Origin`이 허용 값과 불일치, 또는 업그레이드 직전 출발지 IP 재검사 실패. 신규 중계 세션 없음. | [WS-015](../specs/websocket.md#7-수용-테스트), [WS-022](../specs/websocket.md#7-수용-테스트), [WS-027](../specs/websocket.md#7-수용-테스트), [websocket.md 4절](../specs/websocket.md#4-오류와-종료) 다섯째 행 |
| `HTTP 503` | 서버가 종료 유예 중이라 새 업그레이드를 받지 않음. | [OPS-007](../specs/operations.md#7-수용-테스트) |
| `HTTP 500` | 서버 내부 오류. | [operations.md 5절](../specs/operations.md#5-오류와-복구) 단일 세션 오류 행 |

### 2.2 close 코드와 스펙 오류의 대응

종료 원인 열은 [관리 API 계약 1.10절](management-api-v1.md#110-최근-종료-목록-get-apiv1closed-sessions)의 `close_cause` 값이다. 운영 로그의 같은 필드는 [운영 정보 계약](ops-info-v1.md)이 정한다. "해당 없음"은 활성 세션이 되기 전에 끝나 최근 종료 목록에 오르지 않는 경우다([management.md 2.3절](../specs/management.md#23-최근-종료-목록)).

| 오류 코드 | 의미·형식 | 관련 스펙 시나리오 | 종료 원인 |
| --- | --- | --- | --- |
| 1000 | 정상 종료. Close 1000. | [websocket.md 4절](../specs/websocket.md#4-오류와-종료) 'WebSocket close 수신'·'TCP 정상 EOF' 행, [WS-016](../specs/websocket.md#7-수용-테스트), [RLY-019](../specs/relay.md#7-수용-테스트), [RLY-020](../specs/relay.md#7-수용-테스트) | `peer_eof` |
| 1001 | 서버 종료. | [OPS-007](../specs/operations.md#7-수용-테스트), [OPS-008](../specs/operations.md#7-수용-테스트), [OPS-009](../specs/operations.md#7-수용-테스트), [OPS-042](../specs/operations.md#7-수용-테스트), [websocket.md 4절](../specs/websocket.md#4-오류와-종료) '관리자 종료·프로세스 유예 만료' 행 | 유예 만료나 두 번째 종료 요청으로 강제 정리한 활성 세션 중 아직 정리를 시작하지 않았던 세션은 `server_shutdown`(이미 정리를 시작한 세션은 최초 원인 유지, 사용자 결정 2026-11-01(Q82)). 초기 교환·대기 중 종료는 해당 없음 |
| 1002 | 잘못된 프레임·순서·제어 프레임. | [websocket.md 4절](../specs/websocket.md#4-오류와-종료) '잘못된 프레임' 행, [WS-019](../specs/websocket.md#7-수용-테스트) | 활성 세션 중이면 `connection_error`(`error_kind`는 `ws_protocol_error`). 아니면 해당 없음 |
| 1003 | 텍스트 메시지. | [websocket.md 4절](../specs/websocket.md#4-오류와-종료) '텍스트 메시지' 행, [WS-008](../specs/websocket.md#7-수용-테스트) | 활성 세션 중이면 `connection_error`(`error_kind`는 `ws_text_message`). 아니면 해당 없음 |
| 1008 | 비활성 모드 요청. | [relay.md 4절](../specs/relay.md#4-오류와-복구) 첫 행, [RLY-022](../specs/relay.md#7-수용-테스트) | 해당 없음 |
| 1009 | 논리 메시지 M 초과. | [websocket.md 4절](../specs/websocket.md#4-오류와-종료) '프레임·논리 메시지 M 초과' 행, [WS-010](../specs/websocket.md#7-수용-테스트) | 활성 세션 중이면 `connection_error`(`error_kind`는 `ws_message_too_large`). 아니면 해당 없음 |
| 1011 | 서버 내부 오류. | [operations.md 5절](../specs/operations.md#5-오류와-복구) '단일 세션 오류' 행, [RLY-021](../specs/relay.md#7-수용-테스트) | `connection_error` |
| 4001 | 대기 시간 초과. | [relay.md 4절](../specs/relay.md#4-오류와-복구) '대기 단절·만료' 행, [RLY-014](../specs/relay.md#7-수용-테스트) | 해당 없음 |
| 4002 | 대상 연결 실패. 원인(해석 실패·거절·시간 초과)은 서버 로그에서만 구분한다. | [relay.md 4절](../specs/relay.md#4-오류와-복구) '주소 해석 실패·연결 거절·시간 초과' 행, [RLY-009](../specs/relay.md#7-수용-테스트) | 해당 없음 |
| 4003 | 대상 비허용. | [relay.md 4절](../specs/relay.md#4-오류와-복구) '금지 대상·최종 IP 정책 위반' 행, [RLY-007](../specs/relay.md#7-수용-테스트), [RLY-008](../specs/relay.md#7-수용-테스트) | 해당 없음 |
| 4004 | 중복 ID 거절. | [relay.md 4절](../specs/relay.md#4-오류와-복구) 'Mode II 같은 역할 중복' 행, [RLY-013](../specs/relay.md#7-수용-테스트), [RLY-015](../specs/relay.md#7-수용-테스트) | 해당 없음 |
| 4005 | 관리자 종료. | [websocket.md 4절](../specs/websocket.md#4-오류와-종료) '관리자 종료·프로세스 유예 만료' 행, [MGT-009](../specs/management.md#7-수용-테스트), [MGT-023](../specs/management.md#7-수용-테스트) | `admin_terminated` |
| 4006 | 끊긴 연결 감지. | [relay.md 4절](../specs/relay.md#4-오류와-복구) '끊긴 연결 감지' 행, [RLY-025](../specs/relay.md#7-수용-테스트), [RLY-026](../specs/relay.md#7-수용-테스트), [WS-026](../specs/websocket.md#7-수용-테스트) | `dead_connection` |
| 4007 | 상대 연결 오류, 또는 EOF 뒤 데이터 소진 불가. | [relay.md 4절](../specs/relay.md#4-오류와-복구) '중계 I/O 오류' 행, [websocket.md 4절](../specs/websocket.md#4-오류와-종료) 'TCP 정상 EOF' 행, [RLY-020](../specs/relay.md#7-수용-테스트), [WS-016](../specs/websocket.md#7-수용-테스트) | `connection_error` |
| 4008 | 초기 교환 오류. | [websocket.md 4절](../specs/websocket.md#4-오류와-종료) 'Repeater 초기 교환 오류·시간 초과' 행, [relay.md 4절](../specs/relay.md#4-오류와-복구) 첫 행, [RLY-004](../specs/relay.md#7-수용-테스트) | 해당 없음 |
| 4009 | 초기 교환 시간 초과. | [relay.md 4절](../specs/relay.md#4-오류와-복구) '초기 교환 미완료·시간 초과' 행, [RLY-005](../specs/relay.md#7-수용-테스트), [OPS-006](../specs/operations.md#7-수용-테스트) | 해당 없음. 운영 로그는 `connection_aborted`(`handshake_timeout`, `stage` `initial_exchange`)다. 101 뒤 H가 지나 이 코드를 보내는 연결은 활성 세션이 아니다. |
| 4010 | 대기 상한 도달. | [operations.md 5절](../specs/operations.md#5-오류와-복구) 'Lw 상한 도달' 행, [OPS-005](../specs/operations.md#7-수용-테스트) | 해당 없음 |
| 4011 | 중계 연결 상한 도달. | [operations.md 5절](../specs/operations.md#5-오류와-복구) 'L 상한 도달' 행, [OPS-023](../specs/operations.md#7-수용-테스트) | 해당 없음 |

코드를 받을 수 없는 종료(클라이언트의 pong 제한 초과, 네트워크 단절)는 서버가 코드를 보내지 않는다. 이 경우의 종료 원인은 `dead_connection`이며 테스트는 [WS-017](../specs/websocket.md#7-수용-테스트), [WS-026](../specs/websocket.md#7-수용-테스트)이다. Close 교환 중 E 초과의 종료 원인은 `close_timeout`이다.

서버가 Running일 때 세션 소유 작업이 panic 등 버그로 끝났고 기존 종료 원인이 없으면 운영 로그·관리 API의 `close_cause`는 `internal_error`다(사용자 결정 2026-11-02(Q94)). 이 경로에서는 panic 되감기(unwind)가 소유 작업 안의 WS 스트림을 버린다. 되감기 중에는 비동기 쓰기를 할 수 없고, TLS로 감싼 스트림에는 막히지 않는 쓰기 수단도 없다. 그래서 서버는 Close 프레임을 보내지 못하고, 클라이언트는 Close 없는 TCP 종료를 본다. 1.5절의 1011은 서버가 Close를 보낼 수 있는 경로(2.2절)에만 쓴다(이 문서의 판단, 사용자 결정 없음).

## 3. 호환성 규칙

- 서버는 알 수 없는 요청 헤더와 확장 요청을 무시하며 업그레이드를 막지 않는다. 필수 헤더의 잘못된 값은 거절한다.
- 클라이언트는 알 수 없는 4000번대 코드를 받으면 중계 사유를 알 수 없는 종료로 처리해야 한다. reason 문자열은 사람이 읽는 문장이 아니므로 화면 문구로 쓰지 않는다.
- 호환 변경: 새 4000번대 코드·reason 추가, 새 선택 요청 헤더 처리, 서버 메시지 분할 크기 조정. 이때 `1.x.0`으로 올린다. 기존 코드의 번호나 뜻은 바꾸지 않는다.
- 호환성을 깨는 변경(Breaking change): 업그레이드 경로 변경, 서브프로토콜 규칙 변경, 기존 close 코드의 번호·뜻 변경, Origin 비교 규칙 변경, 종료 의미(1.6절) 변경. 새 major 버전(`websocket-relay` 2.0.0)과 마이그레이션·롤백 계획이 필요하다. 지금은 v1만 있어 계획 문서는 없다.

## 4. 버전과 지원 단계

| 상태 | 조건 | 지원 종료·대체 |
| --- | --- | --- |
| Draft | 채택 전 상태. 이 계약은 2026-10-16에 채택해 Draft를 지났다. | 해당 없음 |
| Active | 현재. 사용자 채택(2026-10-16). 계약 테스트 통과 기록은 5절 표가 관리하며, 통과하기 전에는 지원 완료로 표시하지 않는다. | 종료·대체 정책은 새 major를 채택할 때 정한다. |

## 5. 예제와 계약 테스트

예(문서용 값). 허용된 출발지 `192.0.2.10`의 클라이언트가 WS 리스너 `example` 호스트에 보내는 업그레이드 요청과 응답이다. `Sec-WebSocket-Key` 예는 [RFC 6455 1.3절](https://www.rfc-editor.org/rfc/rfc6455#section-1.3)의 예시 값이다. 이 값은 비밀이 아니다.

```text
GET /relay HTTP/1.1
Host: relay.example:8080
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Version: 13
Origin: https://a.example:8443
Sec-WebSocket-Protocol: binary

HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=
Sec-WebSocket-Protocol: binary
```

Close 예: 대기 시간이 W를 넘으면 서버는 코드 `4001`과 reason `wait_timeout`을 담은 Close 프레임을 보낸다.

결과와 증거는 [검증 기록](../AGENTS.md#검증-기록)을 따른다. 스펙과 같은 테스트는 해당 시나리오의 기록을 연결한다.

| 주체·테스트 위치 | 확인할 계약 규칙 | 결과 | 실행 기록 |
| --- | --- | --- | --- |
| Consumer: `vnc-relay` WS·WSS 리스너 / `crates/adapters/src/upgrade.rs` 단위 테스트 `header_names_must_be_tokens`·`control_characters_in_the_head_are_refused`·`syntax_is_checked_before_the_path`·`wrong_version_is_refused_before_a_bad_key`, `crates/adapters/tests/websocket.rs` `management_paths_and_malformed_upgrades_are_refused_without_upgrade` | 1.1절 경로·메서드·헤더 검사 순서, 2.1절 HTTP 상태: [WS-012](../specs/websocket.md#7-수용-테스트)(일부) | 부분 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer: `vnc-relay` WS·WSS 리스너 / `crates/adapters/tests/websocket.rs` `disallowed_source_is_closed_before_tls_or_http_even_with_session_hints`, `disallowed_source_on_wss_is_closed_before_tls_and_holds_no_relay_place`, `stalled_upgrade_request_is_closed_at_u_without_a_reply`, `stalled_tls_handshake_on_wss_is_closed_at_u_without_a_reply`, `head_completed_after_u_gets_no_answer`; `tests/serve.rs` `initial_tls_and_upgrade_deadlines_have_distinct_causes_and_release_capacity`, `every_relay_entry_point_accepts_only_its_own_source_ip` | 1.1절 출발지 IP 검사 순서와 제한 시간 U 뒤 연결 종료: [WS-014](../specs/websocket.md#7-수용-테스트), [WS-023](../specs/websocket.md#7-수용-테스트)(WSS), [WS-022](../specs/websocket.md#7-수용-테스트)(`tests/reload.rs` `removed_source_is_refused_at_upgrade_on_connections_opened_before_the_removal`). 실행파일 시험은 [OPS-006](../specs/operations.md#7-수용-테스트)(비부분)의 H·TLS·업그레이드 시간 제한, Close 4009와 원인·자원 회수를 확인한다. 다른 실행파일 시험은 네 진입점의 독립된 허용 목록을 모두 확인한다([OPS-037](../specs/operations.md#7-수용-테스트), 비부분). | 통과 | 커밋 `534d4f7`의 두 OS 게이트에서 WS-014·WS-022·WS-023·OPS-006·OPS-037이 모두 통과했다. [실행 기록](#계약-테스트-실행-기록) |
| Consumer: `vnc-relay` WSS 리스너 / `crates/adapters/tests/websocket.rs` `expired_untrusted_and_mismatched_certificates_fail_the_client_on_wss`, `tls10_and_tls11_are_refused_and_tls12_and_tls13_succeed_on_wss_and_management` | 2.1절 TLS 협상 실패 행: 만료·신뢰되지 않음·호스트 불일치 인증서는 검증 클라이언트가 협상을 거절한다. TLS 1.0·1.1 접속은 HTTP 응답 없이 닫히고, TLS 1.2 이상은 협상이 성립한다. 평문으로 전환하지 않는다. [WS-013](../specs/websocket.md#7-수용-테스트), [OPS-036](../specs/operations.md#7-수용-테스트)(WSS) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer: `vnc-relay` WS·WSS 리스너 / `crates/adapters/tests/websocket.rs` `origin_must_equal_the_allowed_value`, `origin_must_equal_the_allowed_value_on_wss`, `allowed_source_with_disallowed_origin_is_refused_with_403`, `allowed_source_with_disallowed_origin_is_refused_with_403_on_wss`, `allowed_client_without_origin_or_session_gets_switching_protocols` | 1.3절 Origin 비교: [WS-015](../specs/websocket.md#7-수용-테스트), [WS-027](../specs/websocket.md#7-수용-테스트)(WS·WSS), [WS-021](../specs/websocket.md#7-수용-테스트)(일부) | 부분 | [실행 기록](#계약-테스트-실행-기록) |
| Producer: 고정 버전 noVNC 1.7.0 / Consumer: `vnc-relay` / `tests/websocket_browser.rs` `novnc_repeater_id_completes_rfb_through_ws_and_wss_in_both_server_orders`(`#[ignore]`, 실행 방법은 [noVNC 상호운용 시험 준비](../how-to/set-up-and-verify.md#novnc-상호운용-시험-준비)) | 1.2절 서브프로토콜(noVNC 기본값 포함), 초기 교환 연동: [WS-025](../specs/websocket.md#7-수용-테스트). TCP Server는 시험 코드의 최소 RFB 3.8 서버다. 실제 UltraVNC Server와의 조합은 TODO 5.5에서 확인한다 | 미실행 | noVNC 1.7.0 고정([호환성 검증 기록](repeater-compat-record.md) 2.4절) |
| Producer: Chromium 내장 WebSocket / Consumer: `vnc-relay` 실행파일 / `tests/websocket_browser.rs` `browser_websocket_relays_mode_i_and_ii_over_ws_and_wss_without_novnc_or_websockify` | noVNC·websockify 없는 배포에서 외부 독립 클라이언트로 WS·WSS Mode I·II 중계: [WS-020](../specs/websocket.md#7-수용-테스트). 제품은 `tokio-tungstenite`로 업그레이드를 처리하므로 같은 라이브러리의 시험 클라이언트는 독립 도구로 세지 않는다(이 문서의 판단, 사용자 결정 없음) | 미실행 | [실행 기록](#계약-테스트-실행-기록) |
| Producer: 테스트용 WebSocket 클라이언트 / Consumer: `vnc-relay` / `crates/adapters/tests/websocket_scenarios.rs` `message_and_fragment_boundaries_preserve_the_same_stream_on_ws_and_wss`, `fragmented_binary_with_ping_pong_and_close_never_leaks_control_bytes_to_tcp`, `invalid_frame_sequences_and_controls_close_1002_and_release_registrations`, `ws_and_wss_kw_keeps_idle_peers_and_drops_unresponsive_peers`; `crates/adapters/src/websocket_stream/tests.rs` `bounds_outbound_messages_and_counts_only_written_payload`; `crates/adapters/tests/websocket_relay.rs` `text_frame_closes_1003_without_forwarding_text_and_logs_typed_error`, `single_and_fragmented_messages_accept_m_and_close_1009_at_m_plus_one`, `ws_and_wss_forward_initial_trailing_data_once` | 1.4절 바이너리·분할·ping·pong·텍스트·M 경계: [WS-007](../specs/websocket.md#7-수용-테스트), [WS-008](../specs/websocket.md#7-수용-테스트), [WS-009](../specs/websocket.md#7-수용-테스트), [WS-010](../specs/websocket.md#7-수용-테스트), [WS-018](../specs/websocket.md#7-수용-테스트), [WS-019](../specs/websocket.md#7-수용-테스트), [WS-026](../specs/websocket.md#7-수용-테스트) | 통과(WS-007~010·018·019, 4e4a271. WS-026, 59fc535) | [실행 기록](#계약-테스트-실행-기록). WS-026은 실제 WS·WSS에서 Pong 없는 활성·대기 Viewer의 제거와 `dead_connection`, Pong에 응답하는 유휴 Viewer의 유지를 확인한다(사용자 결정 2026-11-02(Q84)의 기한 FIFO). |
| Producer: `vnc-relay` / Consumer: 테스트용 WebSocket 클라이언트 / `crates/adapters/src/websocket_stream/tests.rs` `rejects_text_and_invalid_frames_with_typed_errors`와 종료 코드 매핑 시험; `crates/adapters/tests/websocket_relay.rs` `text_frame_closes_1003_without_forwarding_text_and_logs_typed_error`, `invalid_initial_requests_close_4008_and_log_the_rejection`, `incomplete_initial_exchange_closes_4009_and_logs_h_timeout`, `disabled_modes_close_1008_before_registration_or_target_connect`, `single_and_fragmented_messages_accept_m_and_close_1009_at_m_plus_one`; `crates/adapters/tests/websocket_scenarios.rs` `invalid_frame_sequences_and_controls_close_1002_and_release_registrations`; `tests/management_terminate.rs` `completion_mgt009_selective_tcp_ws_wss_mode1_mode2_cleanup_and_final_summary` | 1.5절 close 코드와 reason, 2.2절 시나리오: 1000·1002·1003·1008·1009·4001~4005·4008~4011 확인. 4005는 WS·WSS 관리자 종료에서 받은 Close 코드로 확인했다(59fc535). 1001·1011·4006·4007은 이 검색 범위에서 찾지 못함 | 부분 | [실행 기록](#계약-테스트-실행-기록) |
| Producer·Consumer: 테스트용 클라이언트와 TCP 상대 / `crates/adapters/tests/websocket_scenarios.rs` `close_from_either_end_drains_accepted_data_and_records_the_first_cause`, `disconnect_during_transfer_and_missing_close_responses_release_only_the_failed_session`; `crates/adapters/tests/websocket_relay.rs` `client_close_drains_to_tcp_and_echoes_only_the_code`, `tcp_eof_drains_then_forwards_client_data_after_close_1000`, `close_timeout_in_either_order_releases_only_its_session`; `crates/adapters/src/websocket_stream/tests.rs` `finishes_started_frames_before_replying_to_close_within_e` | 1.6절 종료 의미: [WS-016](../specs/websocket.md#7-수용-테스트), [WS-017](../specs/websocket.md#7-수용-테스트), [OPS-042](../specs/operations.md#7-수용-테스트) | 부분(WS-016·017 통과, 4e4a271. OPS-042는 부분) | [실행 기록](#계약-테스트-실행-기록)

### 계약 테스트 실행 기록

| 대상 | 환경 | 명령 | 결과 |
| --- | --- | --- | --- |
| 커밋 `c1c24d2`(작업 3.2 업그레이드 단계, 검토 결함 5건 수정 뒤). 작업 트리에 `docs/contracts/config-v1.md`의 커밋 안 된 변경 1개가 있었으나 이 작업과 무관하다 | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). `tests/websocket.rs` 14건과 `src/upgrade.rs` 단위 테스트 4건(`header_names_must_be_tokens`, `control_characters_in_the_head_are_refused`, `syntax_is_checked_before_the_path`, `wrong_version_is_refused_before_a_bad_key`) 모두 통과. 시나리오 보고: 통과 9, 부분 22, 미실행 0, 미연결 117, 오류 0, 판정 통과 |
| 같은 커밋 `c1c24d2`, 같은 작업 트리 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 17.14.37806.7, `AWS_LC_SYS_PREBUILT_NASM=1` | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). `tests/websocket.rs` 14건과 `src/upgrade.rs` 단위 테스트 4건 모두 통과. 시나리오 보고: 통과 8, 부분 22, 미실행 1(OPS-007: Unix 전용 `closes_and_logs_every_held_connection_when_stopping`), 미연결 117, 오류 0, 판정 통과 |
| 커밋 `e701997`(작업 3.1 검증. 코드 커밋 `89a1001`·`e701997`, 작업 트리 깨끗함) | Ubuntu 24.04.4(WSL2), Rust 1.99.1 | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). `tests/websocket.rs` 16건 모두 통과(새 시험 2건 포함). 시나리오 보고: 통과 11, 부분 20, 실패 0, 미실행 0, 미연결 117, 오류 0, 판정 통과 |
| 같은 커밋 `e701997`, 작업 트리 깨끗함 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), 이전 Windows 기록과 같은 환경 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 보고: 통과 10, 부분 20, 실패 0, 미실행 1(OPS-007: Unix 전용 `closes_and_logs_every_held_connection_when_stopping`), 미연결 117, 오류 0, 판정 통과 |
| 커밋 `4e4a271`(작업 2.7~3.3 초기 교환·중계 시험 기록). 작업 트리 깨끗함 | Windows 11 Pro 10.0.26200 x64 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 보고(150개): 통과 60, 부분 62, 실패 0, 미실행 0, 미연결 28, 오류 0, 판정 통과 |
| 같은 커밋 `4e4a271`, 같은 작업 트리 | Ubuntu 24.04 LTS x64(WSL2) | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 시나리오 보고(150개): 통과 63, 부분 59, 실패 0, 미실행 0, 미연결 28, 오류 0, 판정 통과. `+ 미커밋 변경`은 CRLF 때문에 나타남 |
| 커밋 `59fc535`(작업 3.4 Kw, 4.5 관리자 종료 4005, 2.11 소켓 단절 분류). 작업 트리 깨끗함 | Windows 11 Pro 10.0.26200 x64 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 보고 2026-11-01T23:41:53Z(150개): 통과 77, 부분 61, 실패 0, 미실행 0, 미연결 12, 오류 0, 판정 통과. `websocket.md` 27건은 통과 25, 미연결 2(WS-020·025)다. WS-016·017·026 통과 |
| 같은 커밋 `59fc535` | Ubuntu 24.04 LTS x64(WSL2) | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 실패(`FAILED: scenarios`). 시나리오 보고 23:51:07Z(150개): 통과 81, 부분 55, 실패 2, 미실행 0, 미연결 12, 오류 0. 실패 2건은 RLY-025·026이며 `nft`가 없어 생긴 의도된 실패다(사용자 결정 2026-11-02(Q93)). `websocket.md` 27건은 통과 25, 미연결 2로 Windows와 같다. WS-016·017·026 통과. `+ 미커밋 변경`은 CRLF 때문에 나타남 |
| 커밋 `534d4f7`(작업 3.2 OPS-006·OPS-037 실행파일 시험). 작업 트리 깨끗함 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`) | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 보고 2026-11-04T16:15:07Z(152개): 통과 107, 부분 39, 실패 0, 미실행 0, 미연결 6, 오류 0, 판정 통과. operations.md 44건 중 통과 34, 부분 7, 미연결 3. `tests/serve.rs` 25건 통과. `initial_tls_and_upgrade_deadlines_have_distinct_causes_and_release_capacity`(OPS-006)와 `every_relay_entry_point_accepts_only_its_own_source_ip`(OPS-037, 출발지 `127.0.0.2`~`127.0.0.5`)가 통과해 두 시나리오가 통과가 됐다. |
| 같은 커밋 `534d4f7` | Ubuntu 24.04.4 LTS x64(WSL2), Rust 1.99.1 | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 시나리오 보고 2026-11-04T16:22:56Z(152개): 통과 111, 부분 34, 실패 0, 미실행 1(OPS-044, Windows 전용), 미연결 6, 오류 0, 판정 통과. operations.md 44건 중 통과 35, 부분 5, 미실행 1, 미연결 3. `tests/serve.rs` 31건 통과, OPS-006·OPS-037 통과. `+ 미커밋 변경`은 CRLF 차이다. |

## 6. 초안을 확정하기 전에 해결할 사항

- 이 계약의 이름·코드 선택은 2026-10-16에 사용자가 채택했다. 해당하는 것: 업그레이드 경로 처리의 세부(쿼리 무시, 끝 슬래시 불일치), HTTP 상태(특히 서브프로토콜 거절 400, 종료 유예 503, 상한 L 도달 시 연결 종료), 검사 순서, 확장 비수락, close 코드 번호·이름·reason, 1.6절 종료 의미, 서버 메시지 크기 M 이하.
- 채택으로 WS-TBD-002(HTTP 상태·종료 코드)와 WS-TBD-003(Origin 정규화)을 해소했다. WS-TBD-003의 기준은 [설정 계약 1.4절](config-v2.md#14-origin-값-문법과-비교)이며 이 계약은 그 규칙을 인용하므로 따로 맞출 값이 없다.
- (해결) WS-TBD-004: M·U·E의 기본값과 유효 범위는 사용자가 승인한 값이며 [설정 계약](config-v2.md)이 기준이다. B와 M 사이에 크기 제약은 없다(사용자 결정 2026-11-01(Q78)). 프레임 안에서 스트리밍하며 방향당 미전달 보관 바이트는 B 이하다. 분할 프레임을 합친 논리 메시지 크기가 M을 넘으면 1009로 닫고 추가 전달을 멈춘다. B보다 큰 유효 프레임도 수락해 스트리밍하며 전체를 저장하지 않는다. 프레임 분할 우회와 종료 중 자원 점유는 WS-010·WS-017로 검증한다.
- (해결) WS-TBD-005와 noVNC 기본 서브프로토콜: noVNC 1.7.0을 [호환성 검증 기록](repeater-compat-record.md) 2.4절에 고정했다(버전·해시·받은 주소·날짜). 2026-11-01 관찰에서 이 버전은 WS·WSS 모두 `Sec-WebSocket-Protocol` 헤더를 보내지 않았다(같은 기록 3.4절 W-02·W-03b, 3.5절 F행). 1.2절 규칙과 같아 규칙은 그대로 두고, 1.2절 noVNC 문단을 관찰 사실로 고쳤다(TODO 1.7).
- 1.4절 잘못된 프레임의 종료는 WebSocket 구현이 자동으로 1002를 보내는 경우가 많다. 구현 선택 시 위 규칙과 같게 맞출 수 있는지 확인한다.
