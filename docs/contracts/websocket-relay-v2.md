# WebSocket 중계 v2 계약

외부 WebSocket 클라이언트(noVNC 포함)가 `vnc-relay`의 WS·WSS 진입점에 접속해 중계 스트림을 주고받을 때의 교환 형식을 정한다. 이 계약은 업그레이드 요청과 거절, 서브프로토콜, Origin 비교, 데이터 메시지와 제어 프레임, 종료 코드와 종료 의미를 다룬다. 초기 교환(Repeater) 바이트는 [Repeater 초기 교환 계약](repeater-handshake-v1.md)이 정한다. 이 문서는 그 바이트를 다시 정의하지 않는다.

이 계약은 2026-10-16에 사용자가 채택했다(상태 Active). HTTP 상태, close 코드 번호, 이름, 서브프로토콜 처리 세부, 검사 순서, 종료 의미가 모두 채택 범위다. 스펙이 이 계약에 남겨 두었던 WS-TBD-002(HTTP 상태·종료 코드)와 WS-TBD-003(Origin 정규화)은 이 채택으로 해소했다. 6절에 관찰·구현에서 확인한다고 남긴 항목은 채택으로 해소되지 않았다.

2026-11-10에 Viewer 경로를 `/relay`에서 `/relay/viewer`로 바꿔 2.0.0으로 올렸다(사용자 결정 2026-11-10). 3절의 호환성을 깨는 변경이다. 1.x 클라이언트의 이전과 롤백은 [3.1절](#31-1x-클라이언트를-200으로-옮기기)을 따른다.

2.1.0은 설정으로 켠 역할의 접속 ticket 확인을 더한다(사용자 결정 2026-11-11(Q192·Q195)). 기본은 꺼짐이다. 구현·계약 테스트는 미실행(구현 전)이다.

| 항목 | 값 |
| --- | --- |
| 이전 버전 이력 | `websocket-relay` 2.0.0(2026-11-10). Viewer 경로를 `/relay`에서 `/relay/viewer`로 바꿨다(사용자 결정 2026-11-10). 3절의 호환성을 깨는 변경이다. 2.0.0 서버는 `/relay`를 그 밖의 경로와 같이 404로 거절한다. Server 경로와 나머지 규칙은 1.2.0과 같다. 1.2.0(2026-11-08)은 공유 WS·WSS 리스너의 경로별 Server 역할 `/relay/server`와 신뢰 프록시의 `X-Forwarded-For` 처리를 더했다(사용자 결정 2026-11-08(Q181), 사용자 결정 2026-11-08(Q182), 사용자 결정 2026-11-08(Q188)). Viewer 경로 `/relay`를 유지한 3절의 호환 추가였다. 구현은 커밋 `3df0693`이며, 커밋 `51bd747`의 두 OS 게이트에서 통과했다([WSS Server·프록시·최근 로그 판정표](../how-to/set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)). 1.1.0은 새 close 코드 4011 `relay_connection_limit`을 더하고(사용자 결정 2026-11-01(Q79)), 클라이언트 Close 수신 시 전송 중 프레임 마무리 규칙(1.6절 1행, 사용자 결정 2026-11-01(Q83))과 강제 정리 시 1001 발송 및 종료 원인 보존 규칙(1.5·1.6·2.2절, 사용자 결정 2026-11-01(Q82))을 명시했다. 3절에 따른 호환 변경이다. 명확화(2026-11-07, 1.1.0 유지): 1011을 예약 코드로 적었다. 현재 구현은 1011을 보내지 않는다(사용자 결정 2026-11-07(Q152)). 요청 머리의 16 KiB 상한과 HTTP 400 경계를 명시했다(사용자 결정 2026-11-07(Q156)). 종료 시 새 접속은 TCP에서 거절하고 101 전 연결은 HTTP 응답 없이 닫는다(사용자 결정 2026-11-07(Q160)). 기존 구현은 503을 보내지 않았으므로 클라이언트가 보는 동작은 바뀌지 않는다. 1.0.0 이력: H 시작점과 검사 순서의 명확화(사용자 결정 2026-10-22(Q22), 2026-10-22(Q24)), Repeater 초기 교환 계약 채택에 따른 4008 문장·시작 바이트 반영(사용자 결정 2026-11-01(Q71~Q73)), noVNC 1.7.0 관찰 결과 반영(2026-11-01). |
| 이름과 버전 | `websocket-relay` 2.1.0(2026-11-11). 명시적으로 켠 역할의 접속 ticket 확인을 더하는 3절의 호환 변경이다(사용자 결정 2026-11-11(Q192·Q195)). |
| 상태 | Active(2026-10-16 채택. 2.1.0은 사용자 결정 2026-11-11(Q192·Q195)). 접속 ticket 구현·계약 테스트는 미실행(구현 전)이다. |
| 소유자 | vnc-relay 프로젝트 관리자 |
| 만드는 쪽(Producer) | 외부 WebSocket 클라이언트(요청·바이너리 메시지·Close 송신), `vnc-relay` WS·WSS 리스너(응답·바이너리 메시지·ping·Close 송신). 방향마다 만드는 쪽이 다르며, 각 절에 방향을 적는다. |
| 사용하는 쪽(Consumer) | `vnc-relay` WS·WSS 리스너(요청 수신), 외부 WebSocket 클라이언트(응답 수신) |
| 방향 | 양방향. HTTP 업그레이드는 요청·응답이고, 이후는 WebSocket 메시지 교환이다. |
| 스키마 기준 파일 | 없음. 아래 Markdown 표가 형식의 기준이다. WebSocket 프레임 형식은 [RFC 6455](https://www.rfc-editor.org/rfc/rfc6455)를 따른다. |
| 기준 스펙 | [WS·WSS 전송](../specs/websocket.md), [TCP 중계와 연결 매칭](../specs/relay.md) |
| 관련 요구사항 | [PRD](../requirements/PRD.md) FR-05, 7절 보안 경계, AC-05~08, AC-16~17 |
| 호환성 범위 | WS(`ws://`)와 WSS(`wss://`)의 Viewer·Server 진입점. ticket을 켜지 않은 Viewer는 Mode I·II, Server는 Mode II만 지원한다. ticket을 켠 Viewer의 Mode I 거절은 1.9.5절을 따른다. 공유 리스너의 요청 경로가 역할을 정한다(1.7절). 1.x의 Viewer 클라이언트는 3.1절에 따라 경로를 옮겨야 한다. 외부 RFC 6455 클라이언트가 대상이며, 고정 버전 noVNC의 `repeaterID` 옵션은 Viewer 검증 대상이다([WS-025](../specs/websocket.md#7-수용-테스트)). |
| 관련 테스트 | [WS-001~027](../specs/websocket.md#7-수용-테스트). 연결은 2절과 5절에 있다. `ee0314f`의 두 OS 전체 게이트가 통과했다. 고정 버전 noVNC 시험은 두 OS에서 `--include-ignored`로 별도 통과했다(커밋 `d4c5678`). 현재 지원 판정은 [작업 5.9 판정표](../how-to/set-up-and-verify.md#41-작업-59-판정표)를 따른다. 1011은 예약 코드라 시험 대상이 아니다(사용자 결정 2026-11-07(Q152)). 2.1.0의 WS-037~046·RLY-029~032는 미연결이다. 접속 ticket 구현·계약 테스트는 미실행(구현 전)이다. |
| 후속 테스트 범위 | WS-028~036과 RLY-028, 변경된 WS-012·014·022·023·024는 커밋 `51bd747`의 두 OS 게이트에서 통과했다([WSS Server·프록시·최근 로그 판정표](../how-to/set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)). 2.0.0의 Viewer 경로와 이전 경로 `/relay`의 404는 WS-012·WS-029로 확인한다. 두 시나리오는 커밋 `41dda1f`의 두 OS 게이트에서 통과했다([Viewer 경로 이전 판정표](../how-to/set-up-and-verify.md#44-viewer-경로-이전-판정표)). 2.1.0의 WS-037~046·RLY-029~032는 미연결이다. 접속 ticket 구현·계약 테스트는 미실행(구현 전)이다. 계약 테스트는 5절을 따른다. |

관련 문서: [Repeater 초기 교환 계약](repeater-handshake-v1.md), [설정 계약](config-v4.md)(키 이름·값 범위, Origin 값 문법과 정규화), [관리 API 계약](management-api-v1.md)(관리자 종료·종료 원인 `close_cause`), [운영 정보 계약](ops-info-v2.md)(종료 원인·로그), [ADR-0001 내장 TLS](../decisions/ADR-0001-built-in-tls.md), [ADR-0002 출발지 IP 허용 목록](../decisions/ADR-0002-source-ip-allowlist.md), [ADR-0009 선택형 접속 ticket](../decisions/ADR-0009-optional-relay-access-ticket.md).

## 1. 형식과 의미

### 1.1 업그레이드 요청

클라이언트가 `vnc-relay`에 보내는 HTTP 요청이다. 내장 WSS는 TLS 1.2 이상이 성립한 뒤에 보낸다([ADR-0001](../decisions/ADR-0001-built-in-tls.md), [OPS-R17](../specs/operations.md#41-판정-가능한-규칙)). 역방향 프록시가 TLS를 종료하면 릴레이 WS 리스너로 평문 업그레이드를 전달할 수 있다. 신뢰 프록시의 주소 처리는 1.8절을 따른다.

| 요소 | 값·규칙 | 어긋날 때 |
| --- | --- | --- |
| 요청 머리 크기 | 요청 줄·헤더·끝 빈 줄을 합쳐 최대 16 KiB(16,384바이트)다. 머리 끝 위치가 정확히 16,384바이트이면 허용한다. 그 안에서 끝나지 않으면 요청 구문 단계(가)에서 거절한다(사용자 결정 2026-11-07(Q156)). | HTTP 400 |
| 업그레이드 경로 | 요청 대상의 경로가 정확히 `/relay/viewer`이면 Viewer, `/relay/server`이면 Server다(1.7절). 해당 리스너에서 역할이 꺼졌거나 다른 경로이면 404다. 1.x의 Viewer 경로 `/relay`도 다른 경로다. 대소문자를 구분한다. 끝 슬래시·퍼센트 인코딩·점 세그먼트를 같은 경로로 보지 않는다. 쿼리 문자열은 해석하지 않으며 대상이나 ID 선택에 쓰지 않는다. | HTTP 404 |
| `/api/` 요청 | 관리 API 경로(`/api/` 이하)는 WS·WSS 리스너에서 처리하지 않는다. 다른 경로와 같이 404다. 관리 리스너는 별도다([WS-R08](../specs/websocket.md#3-정상-흐름과-규칙), [WS-012](../specs/websocket.md#7-수용-테스트)). | HTTP 404 |
| 메서드 | `GET`만 허용한다. | HTTP 405, `Allow: GET` |
| HTTP 버전 | HTTP/1.1 이상. | HTTP 400 |
| `Host` | 필수. 값은 라우팅에 쓰지 않는다. | HTTP 400 |
| `Upgrade` | 필수. 값 `websocket`(대소문자 무시). | HTTP 400 |
| `Connection` | 필수. 토큰 `Upgrade`를 포함한다(대소문자 무시). | HTTP 400 |
| `Sec-WebSocket-Key` | 필수. base64 값이며 디코드하면 16바이트여야 한다([RFC 6455 4.2.1절](https://www.rfc-editor.org/rfc/rfc6455#section-4.2.1)). | HTTP 400 |
| `Sec-WebSocket-Version` | 필수. 값 `13`. | HTTP 426, 응답 헤더 `Sec-WebSocket-Version: 13` ([RFC 6455 4.2.2절](https://www.rfc-editor.org/rfc/rfc6455#section-4.2.2)) |
| `Origin` | 선택. 비교와 처리는 1.3절이 정한다. | 불일치 HTTP 403, 중복 HTTP 400 |
| `Sec-WebSocket-Protocol` | 선택. 1.2절 규칙을 따른다. ticket을 켠 역할에서는 1.9절의 값이 필수다. | HTTP 400. ticket을 켠 역할의 ticket 실패는 HTTP 401(1.9절) |
| `Sec-WebSocket-Extensions` | 선택. 서버는 어떤 확장도 수락하지 않는다. 요청에 있어도 거절하지 않고, 응답에 이 헤더를 넣지 않는다. | 해당 없음 |
| 그 밖의 헤더 | 신뢰 프록시의 `X-Forwarded-For`만 1.8절에 따라 읽는다. 신뢰하지 않는 상대의 `X-Forwarded-For`는 무시한다([WS-014](../specs/websocket.md#7-수용-테스트)). `Cookie`, `Authorization`, `Forwarded`, `X-Real-IP`와 나머지 헤더는 접근 판정에 쓰지 않는다. | 해당 없음 |

검사 순서. 한 요청이 여러 조건을 어길 때 먼저 걸리는 상태를 하나만 돌려준다.

1. 접속 수락 직후: 소켓 상대 IP가 당시 유효한 해당 리스너의 `trusted_proxies`에 있거나, 켠 역할들의 `allow` 합집합에 있어야 한다. 둘 다 아니면 TLS·HTTP 처리 전에 연결을 닫는다. HTTP 응답은 없다.
2. WSS: TLS 협상. 실패하면 연결이 성립하지 않는다.
3. 요청 머리 검사. 이 단계 안의 순서는 다음과 같다(사용자 결정 2026-10-22(Q24)의 순서를 유지하며 경로에 역할 판정을 더함, 사용자 결정 2026-11-08(Q188)).
   - 가. 요청 줄 → 헤더 구문과 요청 머리 크기. 머리가 16,384바이트 안에서 끝나지 않으면 HTTP 400이다(사용자 결정 2026-11-07(Q156)). 다른 구문 오류도 HTTP 400이다.
   - 나. 경로·역할(404) → `GET` 메서드(405) → HTTP/1.1 이상(400).
   - 다. `Host` → `Upgrade` → `Connection` 헤더(400).
   - 라. `Sec-WebSocket-Version` 13(426).
   - 마. `Sec-WebSocket-Key`(400).
4. 클라이언트 주소 판정. 소켓 상대가 당시 유효한 `trusted_proxies`에 있으면 1.8절로 전달 주소를 얻는다. 실패하면 HTTP 400과 `connection_rejected(stage=upgrade, reason=forwarded_for_invalid)`를 기록한다. 신뢰하지 않는 상대이면 클라이언트 주소는 소켓 상대 IP다.
5. `Origin`과 해당 역할의 `allowed_origins` 비교. 불일치면 403이다.
6. 서브프로토콜(1.2절). 위반이면 400이다.
7. ticket 확인(1.9.3절). ticket을 켠 역할만 한다. 실패하면 401이다.
8. 업그레이드 직전 두 번째 IP 검사. 클라이언트 주소를 해당 역할의 그 순간 유효한 `allow`와 비교한다. 실패하면 HTTP 403과 `ip_not_allowed`를 기록한다.
9. 사용 기록 쓰기(1.9.4절). ticket을 켠 역할만 한다. 이미 있으면 401(`ticket_reused`)이다. 성공하면 `ticket_accepted`를 기록한다.
10. 모두 통과하면 `101`을 보낸다.

사용 기록은 실패할 수 있는 검사를 모두 마친 뒤에 쓴다. 앞 검사에서 거절된 요청은 ticket을 소비하지 않는다.

TLS 협상과 업그레이드 단계에는 제한 시간 U(`tls_upgrade_secs`)가 적용된다. `101` 전에 U가 지나면 HTTP 응답 없이 연결을 닫는다. 출발지 IP 허용 목록과 `trusted_proxies` 값의 형식은 [설정 계약](config-v4.md)이 정한다.

H(`handshake_secs`)는 초기 교환 제한 시간이다. WS·WSS에서는 `101` 응답을 보낸 뒤부터 잰다. 101 전의 시간은 U가 맡는다(사용자 결정 2026-10-22(Q22)).

### 1.2 업그레이드 응답과 서브프로토콜

성공 응답은 `HTTP/1.1 101 Switching Protocols`이며 `Upgrade: websocket`, `Connection: Upgrade`, `Sec-WebSocket-Accept`를 담는다([RFC 6455 4.2.2절](https://www.rfc-editor.org/rfc/rfc6455#section-4.2.2)). 응답은 `Set-Cookie`를 담지 않는다.

서브프로토콜은 클라이언트가 제시한 값 목록(여러 헤더면 합친 목록)으로 정한다. 값은 정확히 일치하는지만 본다(대소문자 구분).

아래 표는 ticket을 켜지 않은 역할의 규칙이다. `binary`와 ticket 값을 함께 보내도 `binary`를 포함하므로 수락한다. ticket 값은 확인·기록하지 않는다. ticket을 켠 역할은 `binary`가 없는데 값이 있으면 먼저 HTTP 400(`subprotocol_not_supported`)으로 거절한다. 헤더가 없거나 `ticket.`으로 시작하는 값이 없으면 HTTP 401 `ticket_missing`이다. `ticket.`으로 시작하는 값이 둘 이상이면 HTTP 401 `ticket_invalid`다. 그 밖의 값은 무시한다. 수락 응답은 항상 `Sec-WebSocket-Protocol: binary`다([1.9.1절](#191-서브프로토콜에-싣는-값), 사용자 결정 2026-11-11(Q196·Q202·Q213)).

| 클라이언트가 제시한 값 | 서버 처리 |
| --- | --- |
| `binary`를 포함 | 응답에 `Sec-WebSocket-Protocol: binary`를 넣고 수락한다. |
| 헤더 없음 | 응답에 `Sec-WebSocket-Protocol`을 넣지 않고 수락한다. |
| 값이 있으나 `binary`가 없음 | 업그레이드하지 않고 HTTP 400으로 거절한다. |

[RFC 6455 4.2.2절](https://www.rfc-editor.org/rfc/rfc6455#section-4.2.2)은 서버가 응답에 클라이언트가 제시하지 않은 서브프로토콜을 넣지 못하게 한다. 위 규칙은 그에 맞다.

noVNC 호환: 고정 버전 noVNC 1.7.0([호환성 검증 기록](repeater-compat-record.md) 2.4절)은 WS와 WSS 모두 요청에 `Sec-WebSocket-Protocol` 헤더를 넣지 않았다(같은 기록 3.4절 W-02·W-03b, 2026-11-01 관찰). ticket을 켜지 않은 역할에서는 위 표의 "헤더 없음" 행에 해당하므로 서버는 응답에 이 헤더를 넣지 않고 수락한다. 이 결과는 noVNC 1.7.0의 `vnc.html`을 `wsProtocols` 설정 없이 쓴 경우에 한정한다. 다른 버전이나 다른 설정의 noVNC는 관찰하지 않았다.

응답 본문은 중계 ID, 허용 목록, 내부 정보를 담지 않는다.

### 1.3 Origin 비교

허용 값(`allowed_origins`)의 문법과 `Origin` 값의 정규화·비교 규칙은 [설정 계약 1.4절](config-v4.md#14-origin-값-문법과-비교)이 정한다. 선택한 역할의 허용 값을 쓴다. 이 계약은 비교 결과에 따른 업그레이드 처리만 정한다. 정규화 규칙은 설정 계약과 함께 채택했다. Origin은 사용자 인증 수단이 아니다.

| 비교 결과 | 처리 |
| --- | --- |
| `Origin` 헤더 없음 | Origin 검사를 건너뛰고 다음 업그레이드 검사로 넘어간다. |
| 하나이며 허용 값과 일치 | 다음 검사로 넘어간다. |
| 하나이며 허용 값과 불일치(`allowed_origins`가 빈 배열이어서 모든 값이 불일치인 경우 포함) | HTTP 403(2.1절). |
| 둘 이상 | HTTP 400(2.1절). |

예(허용 값 `https://a.example:8443`): `https://a.example:8443`은 통과, `http://a.example:8443`(스킴 다름)·`https://a.example:9443`(포트 다름)·`https://x.a.example:8443`(하위 도메인)은 거절한다([WS-027](../specs/websocket.md#7-수용-테스트)).

### 1.4 데이터 메시지와 제어 프레임

`101` 이후 모든 교환은 WebSocket 프레임이다. 프레임 형식·마스킹·분할은 [RFC 6455](https://www.rfc-editor.org/rfc/rfc6455)를 따른다.

| 요소 | 방향 | 규칙 |
| --- | --- | --- |
| 바이너리 메시지 | 양방향 | 페이로드를 연속된 바이트 스트림으로 취급한다. 여러 메시지의 내용과 순서를 보존한다. 메시지·프레임 경계는 RFB 또는 Repeater 메시지의 경계가 아니다([WS-R02](../specs/websocket.md#3-정상-흐름과-규칙), [WS-R03](../specs/websocket.md#3-정상-흐름과-규칙), [WS-007](../specs/websocket.md#7-수용-테스트), [WS-018](../specs/websocket.md#7-수용-테스트)). 분할(연속 프레임)을 허용한다. 길이 0인 메시지는 허용하며 스트림에 바이트를 더하지 않는다. |
| 시작 바이트 | 서버 → Viewer | Viewer 경로의 `101` 응답 직후 [Repeater 초기 교환 계약 1.1절](repeater-handshake-v1.md#11-메시지)의 시작 바이트 12바이트(`RFB 000.000\n`)를 바이너리 메시지 하나로 보낸다. Viewer가 받는 첫 데이터 메시지다. Viewer는 이것을 받은 뒤 초기 요청을 보낸다. Server 경로에는 시작 바이트를 보내지 않는다. |
| 텍스트 메시지 | 클라이언트 → 서버 | 지원하지 않는다. 첫 프레임이 텍스트이면 즉시 종료 코드 1003으로 닫는다. 텍스트는 TCP에 전달하지 않는다([WS-R04](../specs/websocket.md#3-정상-흐름과-규칙), [WS-008](../specs/websocket.md#7-수용-테스트)). 서버도 텍스트 메시지를 보내지 않는다. |
| 메시지 크기 M | 클라이언트 → 서버 | 프레임이 아니라 분할을 합친 논리 메시지 하나에 적용한다. 설정 키 `limits.ws_message_bytes`([설정 계약](config-v4.md)). 크기 M은 허용하고 M+1부터 거절한다. 초과를 확인하면 종료 코드 1009로 닫고 추가 전달을 멈춘다. 스트리밍하므로 초과를 확인하기 전에 앞부분이 상대 스트림에 이미 전달될 수 있다. 실패한 메시지를 원자적으로 취소하지 않는다([WS-R06](../specs/websocket.md#3-정상-흐름과-규칙), [WS-010](../specs/websocket.md#7-수용-테스트)). |
| 서버가 보내는 메시지 크기 | 서버 → 클라이언트 | 한 메시지는 M 이하로 나눠 보낸다. 입력 TCP 읽기 경계와 같을 필요는 없다. |
| ping | 서버 → 클라이언트 | 연결 성립 후 `keepalive.ws_ping_secs`마다 보낸다([설정 계약](config-v4.md)). 페이로드는 비워 두며 중계 ID나 세션 정보를 담지 않는다. |
| pong | 클라이언트 → 서버 | ping 하나마다 `keepalive.ws_pong_timeout_secs` 안에 pong이 와야 한다. 이 시간 안에 pong이 없으면 끊긴 연결로 처리한다. 데이터 메시지는 pong을 대신하지 않는다. 데이터가 없어도 pong에 응답하는 연결은 유지한다([WS-R13](../specs/websocket.md#3-정상-흐름과-규칙), [WS-026](../specs/websocket.md#7-수용-테스트)). |
| 클라이언트의 ping | 클라이언트 → 서버 | 서버는 pong으로 응답한다([RFC 6455 5.5.2절](https://www.rfc-editor.org/rfc/rfc6455#section-5.5.2)). 이미 Close를 받은 뒤에는 응답하지 않는다. |
| 요청하지 않은 pong | 클라이언트 → 서버 | 허용하며 무시한다. |
| ping·pong·Close 페이로드 | 양방향 | 어떤 제어 프레임의 페이로드도 TCP 중계 스트림에 넣지 않는다([WS-R05](../specs/websocket.md#3-정상-흐름과-규칙), [WS-009](../specs/websocket.md#7-수용-테스트)). |
| 잘못된 프레임·순서 | 클라이언트 → 서버 | 마스크 없는 클라이언트 프레임, 협상하지 않은 확장 비트, 알 수 없는 opcode, 연속 프레임 순서 위반, 분할되거나 125바이트를 넘는 제어 프레임, 유효하지 않은 Close 본문. 종료 코드 1002로 닫는다([WS-019](../specs/websocket.md#7-수용-테스트)). |

WS·WSS 하부 TCP 소켓에는 TCP keepalive(K)도 적용한다. 값은 이 계약이 아니라 [설정 계약](config-v4.md)과 [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)이 정한다.

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
| 1011 | 서버 내부 오류 | `internal_error` | 예약 코드다. 현재 구현은 보내지 않는다. 서버가 Close를 보낼 수 있는 상태에서 예기치 않은 오류로 계속할 수 없을 때를 위해 번호와 이름을 남긴다. 세션 소유 작업의 panic은 Close 없이 TCP를 닫는다(2.2절, 사용자 결정 2026-11-07(Q152)). |
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
| 4012 | ticket 중계 ID 불일치 | `ticket_relay_mismatch` | ticket으로 업그레이드한 연결의 초기 요청이 ticket의 `rid`와 다르거나 Viewer Mode I 요청임(1.9.5절, 사용자 결정 2026-11-11(Q203·Q213)). |
| 4013 | 세션 만료 | `session_expired` | ticket의 세션 만료 시각 `sx`에 도달함(1.9.5절, 사용자 결정 2026-11-11(Q213)). |

reason 문자열은 UTF-8의 고정 식별자만 쓴다. Close 본문은 코드 2바이트와 reason을 합쳐 125바이트 이하다([RFC 6455 5.5절](https://www.rfc-editor.org/rfc/rfc6455#section-5.5)). reason은 중계 ID, 주소, 내부 오류 문장을 담지 않는다. 클라이언트는 reason을 사용자에게 보이지 않아야 한다([RFC 6455 5.5.1절](https://www.rfc-editor.org/rfc/rfc6455#section-5.5.1)). 예약 코드(1005·1006·1015)는 서버가 보내지 않는다.

### 1.6 종료 의미

WebSocket에는 TCP 같은 독립적인 양방향 half-close가 없다. 그래서 WS·WSS가 포함된 세션은 한쪽 EOF 뒤 반대 방향을 정리 제한 시간 E(`timeouts.close_secs`, [설정 계약](config-v4.md)) 이상 유지하지 않는다([websocket.md 4절](../specs/websocket.md#4-오류와-종료)). 아래는 이 의미를 사건별로 고정한 것이다. 이 세부는 스펙이 계약에서 정하도록 남긴 항목이다.

반대쪽이 WS·WSS인 Mode II에도 같은 스트림 전달·정리 규칙을 적용한다. 각 WS·WSS 끝점에는 이 절의 Close·E 규칙을 적용한다. 아래 “상대 TCP”는 반대쪽 중계 스트림이 WS·WSS인 경우도 포함한다.

| 사건 | 서버 동작 | 보장 범위 |
| --- | --- | --- |
| 클라이언트의 Close 수신 | 1. Close 앞의 모든 데이터 메시지를 TCP로 전달한다. Close 수신 전에 이미 헤더를 전송하기 시작한 전송 중 프레임의 나머지 페이로드만 E 안에서 끝까지 전송하고, 새 메시지는 시작하지 않는다. Close 수신 이후 TCP에서 읽은 데이터는 전달하지 않는다. 2. 상대 TCP의 쓰기 방향을 닫아 EOF를 전달한다. 3. 같은 코드로 Close에 응답한다. 코드가 없으면 1000으로 응답한다. 4. 상대 TCP의 EOF를 E까지 기다린 뒤 TCP를 닫는다(사용자 결정 2026-11-01(Q83)). | Close 뒤 TCP에서 오는 데이터는 WS로 보내지 못하며 버린다. 완전한 전달을 보장하지 않는다. E가 지나면 멈춘다. |
| 상대 TCP의 정상 EOF | 1. 이미 받은 TCP 데이터를 모두 바이너리 메시지로 전달한다. 2. Close 1000을 보낸다. 3. 클라이언트의 Close를 E까지 기다리는 동안 클라이언트가 보낸 데이터 메시지는 계속 TCP로 전달한다. 4. 클라이언트의 Close를 받거나 E가 지나면 TCP를 닫고 정리한다. | 받은 데이터를 소진하지 못하면 Close 4007로 중단하고 원인을 기록한다. |
| 상대 TCP 오류 | 즉시 Close 4007을 보낸다. 남은 데이터 전달을 보장하지 않는다. | 최선 노력. |
| 관리자 종료 | 새 데이터 수락을 멈추고 Close 4005를 보낸다. 상대 TCP를 즉시 닫는다. 클라이언트의 Close를 E까지 기다린 뒤 정리를 마치고 관리 API 응답을 한다. | 남은 데이터 전달을 보장하지 않는다. |
| 세션 만료(`sx`) | 새 데이터 수락을 멈추고 Close 4013을 보낸다. 상대 TCP를 즉시 닫는다. 클라이언트의 Close를 E까지 기다린 뒤 정리한다. | 남은 데이터 전달을 보장하지 않는다. |
| 서버 종료 | 유예 중 자발적으로 끝나는 세션은 위 정상 흐름을 따른다. 초기 교환·대기 중인 연결과, 유예 만료 뒤(또는 유예 중 두 번째 종료 요청으로 남은 유예를 건너뛴 뒤) 아직 정리를 시작하지 않은 남은 세션에는 Close 1001을 보내고 정리한다(이미 Close를 교환했거나 보낸 세션에는 1001을 다시 보내지 않는다, 사용자 결정 2026-11-01(Q82)). | 유예 만료·즉시 강제 정리 때 남은 데이터 전달을 보장하지 않는다. |
| 상대 TCP가 끊긴 연결로 판정됨 | Close 4006을 보내고 정리한다. 반대쪽도 같은 세션에서 종료한다. | 최선 노력. |
| pong 제한 초과 | 클라이언트가 끊긴 연결이다. Close 프레임의 응답을 기다리지 않고 TCP 소켓을 닫고, 상대 TCP와 등록 자원을 회수한다. 클라이언트가 보는 종료는 코드 없는 비정상 종료다(RFC 6455의 1006은 클라이언트 쪽에서 붙이는 값이며 서버가 보내지 않는다). | 비정상 종료. |
| Close 교환 중 E 초과 | 응답이 없어도 TCP 소켓을 닫고 자원을 회수한다. 다른 세션에 영향을 주지 않는다. | 비정상 종료([WS-017](../specs/websocket.md#7-수용-테스트)). |

Close를 보낸 뒤에는 데이터 메시지를 보내지 않는다([RFC 6455 5.5.1절](https://www.rfc-editor.org/rfc/rfc6455#section-5.5.1)). Mode I·II 모두 같은 규칙이다. 서버는 세션을 자동으로 재접속·재매칭하지 않는다. 새 접속은 새 연결 수명이다.

### 1.7 경로와 역할

공유 WS·WSS 리스너에서 경로가 역할을 정한다(사용자 결정 2026-11-08(Q181), 사용자 결정 2026-11-08(Q188)). Viewer 경로는 2.0.0에서 `/relay/viewer`로 바꿨다(사용자 결정 2026-11-10).

| 정확한 경로 | 역할·처리 |
| --- | --- |
| `/relay/viewer` | Viewer. Mode I·II 초기 교환을 받는다. |
| `/relay/server` | Server. TCP Server와 같이 Mode II만 받는다. Server는 `ID:` 초기 요청을 먼저 보내야 한다. `vnc-relay`는 Server에 시작 바이트를 보내지 않는다. |
| 꺼진 역할의 경로 | 알 수 없는 경로와 같이 HTTP 404다. `invalid_upgrade`를 기록하고 등록하지 않는다. |
| 그 밖의 경로 | HTTP 404다. `/api/`와 1.x의 Viewer 경로 `/relay`도 포함한다. |

쿼리 문자열은 무시한다. 끝 슬래시가 붙으면 다른 경로다. Repeater 초기 교환 바이트 형식은 바뀌지 않는다.

### 1.8 신뢰 프록시와 `X-Forwarded-For`

릴레이 WS·WSS 리스너 앞의 역방향 프록시가 TLS를 종료할 수 있다(사용자 지시 2026-11-08). 클라이언트 주소는 다음 순서로 정해야 한다(사용자 결정 2026-11-08(Q182), 사용자 결정 2026-11-08(Q188)).

1. 소켓 상대 IP가 해당 리스너의 당시 유효한 `trusted_proxies`에 있을 때만 `X-Forwarded-For`를 읽어야 한다. 신뢰하지 않는 상대의 이 헤더는 무시하며, 클라이언트 주소는 소켓 상대 IP다.
2. 모든 `X-Forwarded-For` 필드 줄을 받은 순서대로 `,`로 합쳐야 한다. `,`로 나눈 각 요소의 앞뒤 SP·HTAB을 제거해야 한다.
3. 요소는 점으로 구분한 IPv4 또는 대괄호가 있거나 없는 IPv6 리터럴이어야 한다. 포트, 영역 ID, `unknown`, 난독화 토큰과 빈 요소는 잘못된 값이다.
4. 오른쪽 끝 요소부터 왼쪽으로 검사해야 한다. 잘못된 요소를 만나면 `forwarded_for_invalid`로 거절해야 한다. 정규 IP가 `trusted_proxies`에 없는 첫 요소를 클라이언트 주소로 정해야 한다. 그 왼쪽 요소는 검사하지 않는다.
5. 모든 요소가 `trusted_proxies`에 있으면 `forwarded_for_invalid`로 거절해야 한다. 프록시 주소가 해당 역할의 `allow`에 있어도 클라이언트 주소로 대신 써서는 안 된다.
6. `X-Forwarded-For` 필드가 없거나 빈 값만 있으면 `forwarded_for_invalid`로 거절해야 한다.
7. IP 비교와 주소 판정에는 `PeerIp`와 같이 IPv4-mapped IPv6를 IPv4로 정규화해야 한다.
8. `Forwarded`(RFC 7239), `X-Real-IP`, `Cookie`, `Authorization`은 접근 판정에 읽어서는 안 된다.

전달 주소 판정에 실패하면 HTTP 400과 `connection_rejected(stage=upgrade, reason=forwarded_for_invalid)`를 기록한다. 등록과 `forwarded_ip` 기록은 없다. 판정에 성공하면 클라이언트 주소를 해당 역할의 `allow` 검사에 쓴다. `peer_ip`는 소켓 상대 IP로 유지하고, 전달 주소는 [운영 정보 계약](ops-info-v2.md)의 `forwarded_ip`로 기록한다. 관리 리스너의 전달 헤더 정책은 바꾸지 않는다.

`trusted_proxies`에는 프록시 주소만 넣어야 한다. 클라이언트 범위를 넣어서는 안 된다. 신뢰 프록시는 accept 단계의 역할 허용 목록 검사를 건너뛰므로, 프록시 주소로 클라이언트를 대신하면 접근 정책을 우회할 수 있다. 신뢰 프록시 호스트나 그 범위 안의 클라이언트에서 시작한 요청은 프록시를 거치면 거절될 수 있다. 같은 출발지에서 이 리스너에 직접 접속해도 신뢰 프록시 연결로 취급해 이 절의 전달 헤더 검사를 적용한다. 그래서 유효한 전달 헤더가 없으면 거절되고, 있으면 같은 전달 주소 판정과 역할 허용 목록 검사를 받는다. 전달 헤더 없이 직접 접속하려면 출발지가 이 리스너의 `trusted_proxies` 밖에 있고 해당 역할의 허용 목록에 들어야 한다. 모든 요소가 신뢰 프록시인 체인의 회귀 시나리오는 WS-036이다.

### 1.9 접속 ticket

접속 ticket 확인은 역할마다 설정으로 켠다. 기본은 꺼짐이다(사용자 결정 2026-11-11(Q192·Q197)). 켠 역할은 유효한 ticket 하나를 요구한다. ticket이 있을 때만 확인하는 방식은 없다(사용자 결정 2026-11-11(Q196)). 형식과 클레임은 이 계약이 정한다(사용자 결정 2026-11-11(Q204·Q213)). 구현·계약 테스트는 미실행(구현 전)이다.

#### 1.9.1 서브프로토콜에 싣는 값

클라이언트는 `Sec-WebSocket-Protocol`에 `binary`와 `ticket.` 뒤에 JWS 압축 형식을 붙인 값 하나를 함께 보낸다. 접두어는 정확히 소문자 `ticket.`이다. 나머지가 JWS다. JWS의 base64url 문자와 `.`은 [RFC 6455 4.1절](https://www.rfc-editor.org/rfc/rfc6455#section-4.1)의 서브프로토콜 값 문자 규칙에 맞는다. 길이 제한은 요청 머리 16 KiB 상한을 따른다.

ticket을 켠 역할의 값 개수와 `binary` 유무 판정은 1.2절을 따른다. 응답은 `binary`만 고른다. ticket 값은 응답에 넣지 않는다. ticket을 켜지 않은 역할은 1.2절 표를 그대로 따르며 ticket 값을 확인·기록하지 않는다(사용자 결정 2026-11-11(Q192·Q213)).

#### 1.9.2 ticket 형식과 클레임

형식은 [JWS 압축 형식(RFC 7515 7.1절)](https://www.rfc-editor.org/rfc/rfc7515#section-7.1)인 `BASE64URL(헤더).BASE64URL(내용).BASE64URL(서명)`이다. base64url에는 패딩이 없다([RFC 7515 2절](https://www.rfc-editor.org/rfc/rfc7515#section-2)).

헤더는 JSON 객체다. `alg`는 `"ES256"`만 받는다. `kid`는 1~128바이트 문자열이며 필수다. `crit`가 있으면 거절한다. `typ`과 그 밖의 멤버는 무시한다. `jku`·`jwk`·`x5u`·`x5c`로 키를 가져오지 않는다.

서명은 [ES256(RFC 7518 3.4절)](https://www.rfc-editor.org/rfc/rfc7518#section-3.4)의 P-256·SHA-256을 쓴다. 서명 값은 R‖S 64바이트다. DER 서명은 거절한다. 공개 키 파일은 [JWKS(RFC 7517 5절)](https://www.rfc-editor.org/rfc/rfc7517#section-5)이며 세부 형식은 [설정 계약 1.11절](config-v4.md#111-접속-ticket-공개-키-파일-형식)을 따른다. 헤더와 내용 JSON에서 멤버 이름이 중복되면 거절한다(사용자 결정 2026-11-11(Q213)).

| 클레임 | 필수 | 자료형·제약 | vnc-relay가 대조하는 값 |
| --- | --- | --- | --- |
| `iss` | 필수 | 문자열 | 역할의 `issuer`와 바이트 단위로 같아야 한다. |
| `aud` | 필수 | 문자열 하나. 배열은 거절 | 경로로 고른 역할의 고정 값. Viewer `vnc-relay/viewer`, Server `vnc-relay/server`(사용자 결정 2026-11-10). |
| `rh` | 필수 | 문자열 | 역할의 `relay_host`와 같아야 한다. 요청의 `Host`는 쓰지 않는다. |
| `rid` | 필수 | 문자열. 중계 ID 문법(ASCII 숫자 1~247자리, [Repeater 초기 교환 계약 1.2절](repeater-handshake-v1.md#12-초기-요청-250바이트)) | 101 뒤 초기 요청의 중계 ID(1.9.5절) |
| `jti` | 필수 | 1~128바이트 문자열 | 사용 기록의 키(1.9.4절) |
| `iat` | 필수 | 0 이상 정수(Unix 초) | 시각 검사, 기동 기준 |
| `exp` | 필수 | 0 이상 정수(Unix 초). `iat < exp`, `exp − iat ≤ max_lifetime_secs` | 시각 검사 |
| `sx` | 선택 | 0 이상 정수(Unix 초). 세션 만료 시각 | 업그레이드 때 현재 시각(오차 없음), 101 뒤 종료 시각 |
| `sub` | 선택 | 1~128바이트 문자열. 발급 쪽의 세션 식별자 | 대조하지 않는다. 운영 로그 `ticket_sub`에 남긴다. |

알 수 없는 클레임은 무시한다. 숫자 클레임에 소수·문자열을 쓰면 거절한다. 최대 수명은 역할의 `max_lifetime_secs`로 정한다(기본 60, 범위 10~600, 사용자 결정 2026-11-11(Q212·Q213)).

#### 1.9.3 확인 순서와 거절

시각은 vnc-relay 호스트의 시스템 시각을 Unix 초로 내림한 값(현재 시각)이다. 시계 오차 허용은 5초로 고정이다. 기동 시각은 `serve` 프로세스가 시작한 시각이다. 재로드로 바뀌지 않는다.

| 순서 | 검사 | 실패 `reason` |
| --- | --- | --- |
| 1 | ticket 값 개수: 0개 / 2개 이상 | `ticket_missing` / `ticket_invalid` |
| 2 | JWS 구문: 세 부분, base64url, JSON 객체, 중복 멤버 없음 | `ticket_invalid` |
| 3 | 헤더: `alg = ES256`, `kid` 있음, `crit` 없음 | `ticket_invalid` |
| 4 | 역할 공개 키 집합에서 `kid`가 같은 키 | `ticket_invalid` |
| 5 | 서명 | `ticket_invalid` |
| 6 | 필수 클레임, 자료형, `rid`·`jti`·`sub` 문법 | `ticket_invalid` |
| 7 | `iss`·`aud`·`rh` 대조 | `ticket_invalid` |
| 8 | `iat < exp`, `exp − iat ≤ max_lifetime_secs` | `ticket_invalid` |
| 9 | `iat ≤ 현재 + 5` | `ticket_invalid` |
| 10 | `현재 < exp + 5`, `sx`가 있으면 `현재 < sx` | `ticket_expired` |
| 11 | `iat > 기동 시각 + 5`(사용자 결정 2026-11-11(Q201)) | `ticket_issued_before_start` |

모든 실패는 HTTP 401이며 `WWW-Authenticate: VncRelayTicket`을 보낸다(사용자 결정 2026-11-11(Q202·Q213)). 응답 본문은 비우거나 고정 문장만 쓴다. `connection_rejected(stage=upgrade, reason=…)`를 기록한다. 서명 확인(5번)을 통과한 뒤의 거절이면 `ticket_jti`도 기록한다. 대기 등록·대상 연결·사용 기록은 없다.

#### 1.9.4 사용 기록

사용 기록의 키는 (역할, `jti`)다. 기록은 프로세스 메모리에만 둔다([ADR-0003](../decisions/ADR-0003-single-instance-memory-state.md)).

1.1절 9단계에서 기록을 쓴다. 이미 있으면 HTTP 401 `ticket_reused`로 거절한다. `WWW-Authenticate: VncRelayTicket`과 `connection_rejected(stage=upgrade, reason=ticket_reused)`를 남긴다. 서명을 확인했으므로 `ticket_jti`도 기록한다. 성공하면 101 전에 `ticket_accepted`(`info`)를 기록한다. 필드는 `conn_id`, `listener`(`ws`·`wss`), `role`, `peer_ip`, `forwarded_ip`(있으면), `ticket_jti`, `ticket_sub`(있으면), `session_expires_at`(있으면)이다. `session_expires_at`은 `sx`를 UTC ISO 8601 문자열로 나타낸 값이다([운영 정보 계약](ops-info-v2.md), 사용자 결정 2026-11-11(Q213)).

같은 (역할, `jti`)로 동시에 온 요청도 하나만 기록을 쓴다. 기록이 있는지 확인하는 일과 기록을 쓰는 일 사이에 다른 요청이 끼어들 수 없어야 한다. 기록을 쓰지 못한 요청은 위와 같이 HTTP 401 `ticket_reused`로 거절한다(이 문서의 판단, 사용자 결정 없음).

기록은 `exp + 5`초에 지운다. 한 기록은 길어야 `max_lifetime_secs` + 10초 남는다. 기록은 서명을 확인한 ticket만 담는다. 개수 상한은 두지 않는다.

재로드, `enabled` 끄고 켜기, 공개 키 교체는 기록을 지우지 않는다. 재시작하면 기록이 사라진다. 1.9.3절 11번이 기동 전에 발급한 ticket을 거절해 이를 메운다(사용자 결정 2026-11-11(Q201)).

기록 쓰기와 101 전송은 하나의 원자적 작업이 아니다. 기록 뒤 101 전송이 실패해도 그 ticket은 쓰인 것으로 남는다. 클라이언트는 새 ticket으로 다시 시작해야 한다.

#### 1.9.5 101 뒤의 중계 ID 대조와 세션 만료

ticket으로 업그레이드한 연결은 기존 초기 요청 검사를 모두 통과한 뒤, 대기 등록·대상 연결 전에 중계 ID를 대조한다. 기존 검사는 [Repeater 초기 교환 계약 1.2절](repeater-handshake-v1.md)의 순서를 유지한다. Server의 `ID:` 접두어 → 모드 판정 → 꺼진 모드 `mode_disabled`(1008) → 값 문법 `handshake_invalid`(4008) 순서다. 시간 초과(4009)도 지금과 같다(이 문서의 판단, 사용자 결정 없음).

Viewer Mode I 요청에서 `mode1.enabled = false`이면 기존 `mode_disabled`(1008)가 먼저다. `mode1.enabled = true`이면 기존 문법 검사를 통과한 뒤 Close 4012로 거절한다. 두 경우 모두 대상 연결을 시도하지 않는다. 이 우선순위는 기존 초기 요청 검사와 Q203의 Mode I 거절을 함께 유지한다(이 문서의 판단, 사용자 결정 없음).

- Viewer는 Mode II 초기 요청의 중계 ID가 `rid`와 바이트 단위로 같아야 한다. `0123`과 `123`은 다르다. Mode I 요청은 대조할 중계 ID가 없으므로 거절한다(사용자 결정 2026-11-11(Q203)).
- Server는 `ID:` 초기 요청의 중계 ID가 `rid`와 같아야 한다.
- 어긋나거나 Viewer Mode I 요청이면 Close 4012(`ticket_relay_mismatch`)로 닫는다. 대기 등록·대상 연결은 없다. 같은 중계 ID로 기다리던 상대 연결은 영향을 받지 않는다. `connection_rejected(stage=initial_exchange, reason=ticket_relay_mismatch)`를 기록한다. 필드는 `ticket_jti`, `mode`, Mode II이면 `relay_id_ref`다.

`sx`가 있는 ticket으로 연 연결은 `sx`에 끝낸다. 타이머는 101 때 정하며 재로드로 바뀌지 않는다(사용자 결정 2026-11-11(Q200·Q213)).

| 만료 시 상태 | 종료와 기록 |
| --- | --- |
| 초기 교환 중 | Close 4013. `connection_aborted(stage=initial_exchange, reason=session_expired)`를 `info`로 기록한다. |
| 대기 중 | Close 4013. `waiting_removed(reason=session_expired)`를 `info`로 기록한다. |
| 활성 세션 | 관리자 종료(1.6절)와 같은 정리 순서를 쓴다. WS·WSS 쪽에는 Close 4013을 보내고 TCP 쪽은 소켓을 닫는다. 종료 원인은 `session_expired`다. `session_closed`는 `info`이며 `side`가 없다. 최근 종료 목록에 오른다. |

양쪽 연결이 모두 `sx`를 가지면 이른 시각에 끝낸다. `sx`가 없는 ticket은 종료 시각을 정하지 않는다. ticket 없이 받은 상대와도 짝을 이룰 수 있다.

#### 1.9.6 설정과 적용 시점

`[relay.ticket.viewer]`·`[relay.ticket.server]`는 그 역할의 WS·WSS 업그레이드에 함께 적용한다([설정 계약 1.2절](config-v4.md#12-키-표), 사용자 결정 2026-11-11(Q197)). 역할마다 `enabled`, `issuer`, `relay_host`, `public_keys_file`, `max_lifetime_secs`를 둔다. `enabled` 기본값은 `false`다. 시계 오차는 5초 고정이며 설정 키가 없다.

TCP 진입점은 ticket을 받지 못한다. ticket을 켠 역할의 TCP 진입점이 켜져 있으면 시작·재로드 때 `ticket_tcp_bypass`를 경고한다. 경고로 시작·재로드를 거절하지 않는다(사용자 결정 2026-11-11(Q198)).

평문 WS에서는 ticket 원문이 암호화되지 않는다. 켤 수 있지만 그 구간은 믿을 수 있는 망 안에 있어야 한다. ticket을 켠 역할이 `[relay.ws]`에서 켜져 있으면 시작·재로드 때 `ticket_plaintext_ws`를 경고한다(사용자 결정 2026-11-11(Q199)).

10개 키 모두 재로드 대상이다. 관리 API에서는 `public_keys_file` 2개를 뺀 8개를 편집할 수 있다. 교체는 새 업그레이드 요청부터 적용한다. 이미 101을 보낸 연결의 중계 ID 대조와 `sx` 타이머, 사용 기록은 유지한다(사용자 결정 2026-11-11(Q200)). 공개 키 파일 형식·검증·교체는 [설정 계약 1.11절](config-v4.md#111-접속-ticket-공개-키-파일-형식)을 따른다.

#### 1.9.7 발급 쪽과 클라이언트의 의무

발급 쪽은 역할마다 다른 P-256 서명 키로 서명해야 한다. vnc-relay는 두 역할을 켰을 때 두 공개 키 파일에 같은 공개 키가 있으면 거절한다(사용자 결정 2026-11-11(Q207)). 발급 쪽은 `jti`를 ticket마다 128비트 이상 난수로 만들어야 한다. `exp − iat`는 그 역할의 `max_lifetime_secs` 이하여야 한다(사용자 결정 2026-11-11(Q212)). `sub`에 비밀이나 개인정보를 넣으면 안 된다. 발급 쪽과 vnc-relay의 시계는 5초 안으로 맞춰야 한다.

클라이언트는 재시도를 포함한 연결 시도마다 새 ticket을 써야 한다. `binary`를 함께 제시해야 한다. 401을 받으면 같은 ticket을 다시 보내지 않고 새 ticket을 받아야 한다. 평문 WS는 믿을 수 있는 망 안에서만 써야 한다. 브라우저 noVNC는 `wsProtocols` 옵션으로 두 값을 보낸다.

ticket 원문(JWS 전체·서명 부분)과 공개 키 파일 내용은 운영 로그·진단·상태 응답·관리 API 응답에 넣지 않는다([운영 정보 계약](ops-info-v2.md), 사용자 결정 2026-11-11(Q213)). ticket 값은 업그레이드 응답에도 넣지 않는다.

## 2. 오류

### 2.1 HTTP 단계의 거절

`101` 전에 거절하는 경우다. `101` 뒤의 오류는 [2.2절](#22-close-코드와-스펙-오류의-대응)을 따른다. 응답 본문은 비우거나 고정 문장만 쓴다.

| 오류 코드 | 의미·형식 | 관련 스펙 시나리오 |
| --- | --- | --- |
| 연결 종료(HTTP 응답 없음) | 접속 수락 시 소켓 상대 IP가 켠 역할들의 `allow` 합집합에도, 해당 리스너의 `trusted_proxies`에도 없음. WSS는 TLS 협상 전에 닫는다. | [WS-014](../specs/websocket.md#7-수용-테스트), [WS-023](../specs/websocket.md#7-수용-테스트), [WS-030](../specs/websocket.md#7-수용-테스트) |
| 연결 종료(HTTP 응답 없음) | 중계 연결 상한 L 도달 시 새 접속. TLS·업그레이드 제한 시간 U 초과. | [OPS-004](../specs/operations.md#7-수용-테스트), [OPS-006](../specs/operations.md#7-수용-테스트) |
| TLS 협상 실패(HTTP 응답 없음) | 인증서 검증 실패, TLS 1.2 미만. 평문 WS로 전환하지 않는다. | [WS-013](../specs/websocket.md#7-수용-테스트), [OPS-036](../specs/operations.md#7-수용-테스트), [websocket.md 4절](../specs/websocket.md#4-오류와-종료) 첫·둘째 행 |
| `HTTP 400` | 요청 구문 오류, 요청 머리가 16,384바이트 안에서 끝나지 않음, HTTP/1.0, 필수 헤더 누락·잘못된 값, `Origin` 중복, 허용되지 않는 서브프로토콜. 중계 등록·대상 연결 없음. | [WS-012](../specs/websocket.md#7-수용-테스트), [websocket.md 4절](../specs/websocket.md#4-오류와-종료) |
| `HTTP 400` | 신뢰 프록시의 `X-Forwarded-For`가 없거나 비어 있음, 검사 대상 요소가 잘못됨, 또는 모든 요소가 신뢰 프록시 주소임. `forwarded_for_invalid`, 중계 등록·대상 연결·`forwarded_ip` 없음. | [WS-033](../specs/websocket.md#7-수용-테스트), [WS-036](../specs/websocket.md#7-수용-테스트) |
| `HTTP 401` | ticket을 켠 역할에서 없음·무효·만료·기동 전 발급·재사용을 거절한다. `WWW-Authenticate: VncRelayTicket`을 보낸다. reason은 `ticket_missing`, `ticket_invalid`, `ticket_expired`, `ticket_issued_before_start`, `ticket_reused`다. 대기 등록·대상 연결 없음(1.9절). | [WS-039~045](../specs/websocket.md#7-수용-테스트) |
| `HTTP 404` | 알 수 없는 경로 또는 해당 리스너에서 꺼진 역할의 경로. `/api/` 요청과 1.x의 Viewer 경로 `/relay`를 포함한다. `invalid_upgrade`를 기록한다. | [WS-012](../specs/websocket.md#7-수용-테스트), [WS-029](../specs/websocket.md#7-수용-테스트) |
| `HTTP 405` | `GET`이 아닌 메서드. `Allow: GET`. | [WS-012](../specs/websocket.md#7-수용-테스트) |
| `HTTP 426` | `Sec-WebSocket-Version`이 13이 아님. `Sec-WebSocket-Version: 13`을 함께 보낸다. | [WS-012](../specs/websocket.md#7-수용-테스트) |
| `HTTP 403` | `Origin`이 해당 역할의 허용 값과 불일치, 또는 업그레이드 직전 클라이언트 주소가 해당 역할의 `allow`에 없음(`ip_not_allowed`). 신규 중계 세션 없음. | [WS-015](../specs/websocket.md#7-수용-테스트), [WS-022](../specs/websocket.md#7-수용-테스트), [WS-027](../specs/websocket.md#7-수용-테스트), [WS-030~032](../specs/websocket.md#7-수용-테스트) |
| 종료 시 TCP 접속 거절 또는 연결 종료(HTTP 응답 없음) | 종료 신호를 받으면 WS·WSS 리스너를 닫는다. 새 접속은 TCP에서 거절되며 HTTP 503은 보내지 않는다. 수락된 연결이 101 전에 종료 신호를 받으면 응답 없이 닫고 `connection_aborted`(`reason` = `server_shutdown`)를 기록한다. `stage`는 WS에서 `upgrade`다. WSS는 TLS 협상 완료 전이면 `tls`, 완료 뒤면 `upgrade`다(사용자 결정 2026-11-07(Q160)). | [OPS-007](../specs/operations.md#7-수용-테스트) |
| `HTTP 500` | 서버 내부 오류. | [operations.md 5절](../specs/operations.md#5-오류와-복구) 단일 세션 오류 행 |

### 2.2 close 코드와 스펙 오류의 대응

종료 원인 열은 [관리 API 계약 1.10절](management-api-v1.md#110-최근-종료-목록-get-apiv1closed-sessions)의 `close_cause` 값이다. 운영 로그의 같은 필드는 [운영 정보 계약](ops-info-v2.md)이 정한다. "해당 없음"은 활성 세션이 되기 전에 끝나 최근 종료 목록에 오르지 않는 경우다([management.md 2.3절](../specs/management.md#23-최근-종료-목록)).

| 오류 코드 | 의미·형식 | 관련 스펙 시나리오 | 종료 원인 |
| --- | --- | --- | --- |
| 1000 | 정상 종료. Close 1000. | [websocket.md 4절](../specs/websocket.md#4-오류와-종료) 'WebSocket close 수신'·'TCP 정상 EOF' 행, [WS-016](../specs/websocket.md#7-수용-테스트), [RLY-019](../specs/relay.md#7-수용-테스트), [RLY-020](../specs/relay.md#7-수용-테스트) | `peer_eof` |
| 1001 | 서버 종료. | [OPS-007](../specs/operations.md#7-수용-테스트), [OPS-008](../specs/operations.md#7-수용-테스트), [OPS-009](../specs/operations.md#7-수용-테스트), [OPS-042](../specs/operations.md#7-수용-테스트), [websocket.md 4절](../specs/websocket.md#4-오류와-종료) '관리자 종료·프로세스 유예 만료' 행 | 유예 만료나 두 번째 종료 요청으로 강제 정리한 활성 세션 중 아직 정리를 시작하지 않았던 세션은 `server_shutdown`(이미 정리를 시작한 세션은 최초 원인 유지, 사용자 결정 2026-11-01(Q82)). 초기 교환·대기 중 종료는 해당 없음 |
| 1002 | 잘못된 프레임·순서·제어 프레임. | [websocket.md 4절](../specs/websocket.md#4-오류와-종료) '잘못된 프레임' 행, [WS-019](../specs/websocket.md#7-수용-테스트) | 활성 세션 중이면 `connection_error`(`error_kind`는 `ws_protocol_error`). 아니면 해당 없음 |
| 1003 | 텍스트 메시지. | [websocket.md 4절](../specs/websocket.md#4-오류와-종료) '텍스트 메시지' 행, [WS-008](../specs/websocket.md#7-수용-테스트) | 활성 세션 중이면 `connection_error`(`error_kind`는 `ws_text_message`). 아니면 해당 없음 |
| 1008 | 비활성 모드 요청. | [relay.md 4절](../specs/relay.md#4-오류와-복구) 첫 행, [RLY-022](../specs/relay.md#7-수용-테스트) | 해당 없음 |
| 1009 | 논리 메시지 M 초과. | [websocket.md 4절](../specs/websocket.md#4-오류와-종료) '프레임·논리 메시지 M 초과' 행, [WS-010](../specs/websocket.md#7-수용-테스트) | 활성 세션 중이면 `connection_error`(`error_kind`는 `ws_message_too_large`). 아니면 해당 없음 |
| 1011 | 서버 내부 오류. 예약 코드이며 현재 구현은 보내지 않는다(사용자 결정 2026-11-07(Q152)). | [operations.md 5절](../specs/operations.md#5-오류와-복구) '단일 세션 오류' 행, [RLY-021](../specs/relay.md#7-수용-테스트) | 해당 없음(보내는 경로 없음). panic으로 끝난 세션의 종료 원인은 아래 `internal_error` 문단을 따른다 |
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
| 4012 | ticket 중계 ID 불일치. Viewer Mode I 요청도 포함한다. | [RLY-029](../specs/relay.md#7-수용-테스트), [RLY-030](../specs/relay.md#7-수용-테스트), [RLY-032](../specs/relay.md#7-수용-테스트) | 해당 없음 |
| 4013 | ticket의 세션 만료 시각 `sx` 도달. | [RLY-031](../specs/relay.md#7-수용-테스트) | 활성 세션이면 `session_expired`. 아니면 해당 없음 |

코드를 받을 수 없는 종료(클라이언트의 pong 제한 초과, 네트워크 단절)는 서버가 코드를 보내지 않는다. 이 경우의 종료 원인은 `dead_connection`이며 테스트는 [WS-017](../specs/websocket.md#7-수용-테스트), [WS-026](../specs/websocket.md#7-수용-테스트)이다. Close 교환 중 E 초과의 종료 원인은 `close_timeout`이다.

서버가 Running일 때 세션 소유 작업이 panic 등 버그로 끝났고 기존 종료 원인이 없으면 운영 로그·관리 API의 `close_cause`는 `internal_error`다(사용자 결정 2026-11-02(Q94)). 이 경로에서는 panic 되감기(unwind)가 소유 작업 안의 WS 스트림을 버린다. 되감기 중에는 비동기 쓰기를 할 수 없고, TLS로 감싼 스트림에는 막히지 않는 쓰기 수단도 없다. 그래서 서버는 Close 프레임을 보내지 못하고, 클라이언트는 Close 없는 TCP 종료를 본다. 1.5절의 1011은 서버가 Close를 보낼 수 있는 경로(2.2절)에만 쓴다(이 문서의 판단, 사용자 결정 없음).

## 3. 호환성 규칙

- 서버는 알 수 없는 요청 헤더와 확장 요청을 무시하며 업그레이드를 막지 않는다. 필수 헤더의 잘못된 값은 거절한다.
- 클라이언트는 알 수 없는 4000번대 코드를 받으면 중계 사유를 알 수 없는 종료로 처리해야 한다. reason 문자열은 사람이 읽는 문장이 아니므로 화면 문구로 쓰지 않는다.
- 호환 변경: 새 4000번대 코드·reason 추가, 새 선택 요청 헤더 처리, 기존 경로를 유지하는 새 역할 경로 추가, 명시적으로 설정한 신뢰 프록시의 전달 헤더 처리, 서버 메시지 분할 크기 조정, 명시적으로 켠 역할의 접속 ticket 확인(그 역할의 서브프로토콜 규칙, HTTP 401과 `WWW-Authenticate`, close 4012·4013을 포함한다). 이때 minor 버전(`2.x.0`)을 올린다. 기존 코드의 번호나 뜻은 바꾸지 않는다. 1.2.0의 `/relay/server`와 `trusted_proxies`는 이 범위였다. 기본 `trusted_proxies = []`에서는 기존 Viewer 클라이언트의 판정이 바뀌지 않았다. 2.1.0의 접속 ticket 확인은 이 범위다(사용자 결정 2026-11-11(Q195)). 기본 `enabled = false`에서는 기존 클라이언트의 판정이 바뀌지 않는다.
- 호환성을 깨는 변경(Breaking change): 업그레이드 경로 변경, 서브프로토콜 규칙 변경(위 호환 항목의 ticket 확인은 제외), 기존 close 코드의 번호·뜻 변경, Origin 비교 규칙 변경, 종료 의미(1.6절) 변경. 새 major 버전과 마이그레이션·롤백 계획이 필요하다. 2.0.0의 Viewer 경로 변경이 이 범위이며, 계획은 3.1절이다.

### 3.1 1.x 클라이언트를 2.0.0으로 옮기기

| 항목 | 규칙 |
| --- | --- |
| 바뀌는 것 | Viewer 업그레이드 경로만 `/relay`에서 `/relay/viewer`로 바뀐다. Server 경로 `/relay/server`, 서브프로토콜, Origin 비교, close 코드, 종료 의미, Repeater 초기 교환 바이트는 같다. 설정 파일 형식도 같다. |
| 2.0.0 서버가 `/relay` 요청을 받을 때 | 그 밖의 경로와 같이 HTTP 404이며 `invalid_upgrade`를 기록한다. 대기 등록과 대상 연결은 없다([WS-029](../specs/websocket.md#7-수용-테스트)). |
| 1.x 서버가 `/relay/viewer` 요청을 받을 때 | 알 수 없는 경로이므로 HTTP 404다. |
| 마이그레이션 | 서버 교체와 Viewer 클라이언트의 접속 URL 변경을 함께 한다. URL에서는 경로만 `/relay/viewer`로 바꾼다. 서버 설정 파일은 바꾸지 않는다. 절차는 [운영 how-to 3.2.3절](../how-to/operate-relay.md#323-viewer-경로를-relayviewer로-옮기기)을 따른다. |
| 롤백 | 이전 서버(릴리스 `2026.11.2` 이하)로 되돌릴 때 Viewer 클라이언트의 URL도 `/relay`로 되돌린다. 설정 파일은 그대로 쓴다. |
| 지원 기간 | 두지 않는다. 2.0.0 서버는 `/relay`를 Viewer로 받지 않는다. 지원 기간은 사용자 결정이 아니라 이 문서를 쓰며 정한 판단이다. 판단 근거: 사용자가 고른 변경은 경로 이름 바꾸기이며 이전 경로의 별칭을 정하지 않았다. Viewer 경로는 클라이언트 접속 URL의 일부라서 서버 교체와 같은 때에 바꿀 수 있다. |

### 3.2 ticket 확인을 켜고 끄기

2.0.0 서버도 `binary`와 ticket 값을 함께 받으면 수락하고 ticket을 무시한다. ticket 확인을 켤 때는 다음 순서를 따른다.

1. 서버를 2.1.0으로 올린다. ticket 확인은 꺼 둔다.
2. 발급 쪽과 클라이언트가 요청에 ticket과 `binary`를 싣게 한다.
3. 역할의 `enabled`를 켜고 재로드한다.

끄려면 `enabled = false`로 바꾸고 재로드한다. 이미 맺은 연결은 이전 값을 유지한다(사용자 결정 2026-11-11(Q200)). 실행파일을 2.0.0(설정 4.0.0)으로 되돌릴 때는 먼저 `[relay.ticket.*]` 표를 지운다. 지원 기간은 해당 없음이다. 3절의 호환 변경이기 때문이다(사용자 결정 2026-11-11(Q195)).

## 4. 버전과 지원 단계

| 상태 | 조건 | 지원 종료·대체 |
| --- | --- | --- |
| Draft | 채택 전 상태. 이 계약은 2026-10-16에 채택해 Draft를 지났다. | 해당 없음 |
| Active | 현재 2.1.0. 사용자 채택(2026-10-16), 2.1.0은 사용자 결정 2026-11-11(Q192·Q195). 접속 ticket 구현·계약 테스트는 미실행(구현 전)이다. 계약 테스트 통과 기록은 5절 표가 관리하며, 통과하기 전에는 지원 완료로 표시하지 않는다. | 1.x는 2.0.0이 대체했다. 지원 기간은 두지 않는다(3.1절). 2.1.0의 ticket 확인은 호환 변경이다(3.2절). |

## 5. 예제와 계약 테스트

예(문서용 값). 허용된 출발지 `192.0.2.10`의 클라이언트가 WS 리스너 `example` 호스트에 보내는 업그레이드 요청과 응답이다. `Sec-WebSocket-Key` 예는 [RFC 6455 1.3절](https://www.rfc-editor.org/rfc/rfc6455#section-1.3)의 예시 값이다. 이 값은 비밀이 아니다.

```text
GET /relay/viewer HTTP/1.1
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

Server 요청 예. 해당 리스너의 Server 역할과 IP 정책이 허용한 경우다. 응답은 위와 같은 101이다. 이후 Server가 `ID:1234`를 NUL로 채운 250바이트 초기 요청을 먼저 보낸다. `vnc-relay`는 Server에 시작 바이트를 보내지 않는다.

```text
GET /relay/server HTTP/1.1
Host: relay.example:8080
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Version: 13
```

프록시 요청 예. 프록시가 클라이언트의 TLS를 종료하고 WS로 전달한다. 소켓 상대 `192.0.2.20`과 중간 프록시 `192.0.2.21`이 `trusted_proxies`에 있다고 가정한다. 오른쪽의 신뢰 프록시를 건너뛴 `198.51.100.10`이 클라이언트 주소다. 그 왼쪽의 `unknown`은 검사하지 않는다.

```text
GET /relay/viewer HTTP/1.1
Host: relay.example:8080
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Version: 13
X-Forwarded-For: unknown, 198.51.100.10
X-Forwarded-For: 192.0.2.21
```

ticket 요청 예. 아래 token은 서명이 맞지 않는 문서용 값이다. 헤더는 `{"alg":"ES256","kid":"example"}`이며 내용은 `{"example":true}`다. 필수 클레임을 갖춘 유효한 ticket이 아니므로 성공 예로 쓰지 않는다.

```text
GET /relay/viewer HTTP/1.1
Host: relay.example:8080
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Version: 13
Sec-WebSocket-Protocol: binary, ticket.eyJhbGciOiJFUzI1NiIsImtpZCI6ImV4YW1wbGUifQ.eyJleGFtcGxlIjp0cnVlfQ.c2lnbmF0dXJl
```

401 응답 예. ticket을 켠 역할이 ticket 없는 요청이나 유효하지 않은 ticket을 거절할 때의 형식이다. reason은 응답 본문이 아니라 운영 로그에 남긴다.

```text
HTTP/1.1 401 Unauthorized
WWW-Authenticate: VncRelayTicket
Content-Length: 0

```

2.1.0의 계약 테스트 계획은 아래와 같다. 구현·계약 테스트는 미실행(구현 전)이다. 시나리오는 미연결이며 기존 버전의 통과 기록은 이 규칙의 증거가 아니다.

| 주체·테스트 위치 | 확인할 계약 규칙 | 결과 | 실행 기록 |
| --- | --- | --- | --- |
| Producer: WS·WSS Viewer·Server 시험 클라이언트 / Consumer: WS·WSS 리스너. 테스트 미연결 | 1.2·1.9.1절 기본 꺼짐, 유효한 역할별 ticket, `binary`만 선택, ticket 원문 비기록. WS-037·038·045 | 미실행(구현 전) | 없음 |
| Producer: ticket 오류·경계 시험 클라이언트 / Consumer: WS·WSS 리스너. 테스트 미연결 | 1.1·1.9.2·1.9.3절 형식·서명·클레임·시각·기동 기준과 401, 검사 우선순위. WS-039~041·043~045 | 미실행(구현 전) | 없음 |
| Producer: 재사용·재로드 시험 클라이언트와 설정 편집 / Consumer: WS·WSS 리스너. 테스트 미연결 | 1.9.4·1.9.6절 검사 뒤 사용 기록 쓰기, 403에서 미소비, 보관·재사용·동시 재사용 거절, 교체 뒤 기존 연결·타이머·기록 유지. WS-042·046 | 미실행(구현 전) | 없음 |
| Producer: ticket 연결의 초기 요청·세션 상대 / Consumer: 공통 중계. 테스트 미연결 | 1.9.5절 `rid` 대조, 기존 초기 검사 우선, Mode I 거절, Close 4012, 상대 대기 연결 유지. RLY-029·030·032 | 미실행(구현 전) | 없음 |
| Producer·Consumer: `sx`가 있는 ticket 연결과 TCP·WS·WSS 상대. 테스트 미연결 | 1.6·1.9.5절 초기 교환·대기·활성 상태의 `sx`, 이른 만료 시각, Close 4013, `session_expired` 로그·최근 종료 목록. RLY-031 | 미실행(구현 전) | 없음 |

아래는 1.2.0에서 더한 계약 테스트다. 1.2.0 규칙은 커밋 `51bd747`의 두 OS 게이트에서 통과했다. 2.0.0의 Viewer 경로 변경은 첫 행의 시험을 고쳐 확인한다. `/relay/viewer`는 Viewer를 고르고 이전 경로 `/relay`는 404다. 이전 버전의 날짜별 실행 기록은 그대로 유지하며 새 규칙의 증거로 쓰지 않는다.

| 주체·테스트 위치 | 확인할 계약 규칙 | 결과 | 실행 기록 |
| --- | --- | --- | --- |
| Producer: WS·WSS Server 시험 클라이언트 / Consumer: 공유 WS·WSS 리스너 | 1.7절 경로·역할, Server 시작 바이트 없음, Mode II 매칭과 양방향 전달. WS-028·WS-029, RLY-028 | 통과 | `crates/adapters/tests/websocket_scenarios.rs`의 `websocket_servers_pair_with_every_viewer_transport_in_both_orders`·`exact_enabled_paths_select_roles_and_rejections_never_register`·`websocket_server_rules_cover_mode_i_and_waiting_and_active_duplicates`. 실행 결과는 아래 실행 기록을 따른다. |
| Producer: 역할별 IP·Origin 시험 클라이언트 / Consumer: 공유 WS·WSS 리스너 | 1.1절 검사 순서, accept 합집합과 upgrade 역할별 목록. WS-030·WS-031, 변경된 WS-012·022·023·024 | 통과 | `crates/adapters/tests/websocket_scenarios.rs`의 `accept_union_does_not_replace_server_role_ip_check`·`server_path_uses_only_server_origin_policy_on_ws_and_wss`·`management_paths_and_invalid_upgrades_cannot_register_or_connect_a_target`, `tests/reload.rs`의 `trusted_proxies_reload_and_edit_apply_to_new_checks_and_keep_open_connections`, `tests/serve.rs`의 `server_only_websocket_roles_open_each_shared_listener`. 실행 결과는 아래 실행 기록을 따른다. |
| Producer: 신뢰·비신뢰 프록시 시험 클라이언트 / Consumer: WS·WSS 리스너 | 1.8절 1~8단계. 복수 필드·SP/HTAB, 양 IPv6 표기·mapped 정규화, 오른쪽 비신뢰 주소, 그 왼쪽 위조 값 무시. 검사 대상의 포트·영역 ID·unknown·난독화·빈 요소와 헤더 누락은 400. 모두 신뢰인 체인은 역할 허용 목록과 관계없이 400이며 `forwarded_ip` 없음. WS-032~034·WS-036, 변경된 WS-014 | 통과 | `crates/adapters/tests/websocket_scenarios.rs`의 `trusted_proxy_uses_rightmost_untrusted_canonical_address_on_both_paths`·`derived_address_survives_origin_rejection_abort_and_target_connect_failure`·`invalid_or_all_trusted_forwarded_chains_fail_before_origin_and_registration`·`untrusted_peer_headers_neither_grant_access_nor_supply_forwarded_metadata`. 실행 결과는 아래 실행 기록을 따른다. |
| Producer: TLS 종료 시험 프록시 / Consumer: WS 리스너 | 1.8절 TLS 종료 배포, Viewer·Server Mode II와 전달 주소·운영 로그·관리 조회. WS-035 | 통과 | `crates/adapters/tests/websocket_scenarios.rs`의 `tls_terminating_proxy_adds_each_client_address_and_pipes_mode_two_both_ways`. 시험 코드의 rustls TLS 종료 프록시가 평문 WS에 `X-Forwarded-For`를 더해 보낸다. Nginx 예시는 추정이며 실제 배치는 미실행이다. 실행 결과는 아래 실행 기록을 따른다. |

Close 예: 대기 시간이 W를 넘으면 서버는 코드 `4001`과 reason `wait_timeout`을 담은 Close 프레임을 보낸다.

결과와 증거는 [검증 기록](../AGENTS.md#검증-기록)을 따른다. 스펙과 같은 테스트는 해당 시나리오의 기록을 연결한다.

| 주체·테스트 위치 | 확인할 계약 규칙 | 결과 | 실행 기록 |
| --- | --- | --- | --- |
| Consumer: `vnc-relay` WS·WSS 리스너 / `crates/adapters/src/upgrade.rs` 단위 테스트 `header_names_must_be_tokens`·`control_characters_in_the_head_are_refused`·`syntax_is_checked_before_the_path`·`wrong_version_is_refused_before_a_bad_key`, `crates/adapters/tests/websocket.rs` `management_paths_and_malformed_upgrades_are_refused_without_upgrade` | 1.1절 경로·메서드·헤더 검사 순서, 2.1절 HTTP 상태: [WS-012](../specs/websocket.md#7-수용-테스트)(일부) | 부분 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer: `vnc-relay` WS 리스너 / `crates/adapters/tests/websocket.rs` `request_head_accepts_16384_bytes_and_refuses_16385_bytes` | 1.1절 요청 머리 상한: 끝 빈 줄을 포함해 16,384바이트이면 101, 16,385바이트이면 400 뒤 연결 종료. 업그레이드하지 않음. [WS-012](../specs/websocket.md#7-수용-테스트)(일부) | 통과(경계 규칙, WS-012 일부) | 2026-11-07, Q153~Q158·Q160 반영 작업 트리. Windows 11 x64: `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/tmp/perf-diag/cargo.ps1 test -p vnc-relay-adapters --test websocket request_head_accepts_16384_bytes_and_refuses_16385_bytes`, 1 passed, 0 failed, 18 filtered out, 시험 0.04초. WSL Ubuntu: 저장소 루트에서 `CARGO_TARGET_DIR=~/vnc-relay-target cargo test -p vnc-relay-adapters --test websocket request_head_accepts_16384_bytes_and_refuses_16385_bytes`, 1 passed, 0 failed, 18 filtered out, 시험 0.04초. 두 명령 모두 종료 코드 0. 커밋 `84f4086`의 두 OS 전체 게이트에서도 통과했다. |
| Consumer: `vnc-relay` WS·WSS 리스너 / `crates/adapters/tests/websocket.rs` `disallowed_source_is_closed_before_tls_or_http_even_with_session_hints`, `disallowed_source_on_wss_is_closed_before_tls_and_holds_no_relay_place`, `stalled_upgrade_request_is_closed_at_u_without_a_reply`, `stalled_tls_handshake_on_wss_is_closed_at_u_without_a_reply`, `head_completed_after_u_gets_no_answer`; `tests/serve.rs` `initial_tls_and_upgrade_deadlines_have_distinct_causes_and_release_capacity`, `every_relay_entry_point_accepts_only_its_own_source_ip` | 1.1절 출발지 IP 검사 순서와 제한 시간 U 뒤 연결 종료: [WS-014](../specs/websocket.md#7-수용-테스트), [WS-023](../specs/websocket.md#7-수용-테스트)(WSS), [WS-022](../specs/websocket.md#7-수용-테스트)(`tests/reload.rs` `removed_source_is_refused_at_upgrade_on_connections_opened_before_the_removal`). 실행파일 시험은 [OPS-006](../specs/operations.md#7-수용-테스트)(비부분)의 H·TLS·업그레이드 시간 제한, Close 4009와 원인·자원 회수를 확인한다. 다른 실행파일 시험은 네 진입점의 독립된 허용 목록을 모두 확인한다([OPS-037](../specs/operations.md#7-수용-테스트), 비부분). | 통과 | 커밋 `534d4f7`의 두 OS 게이트에서 WS-014·WS-022·WS-023·OPS-006·OPS-037이 모두 통과했다. [실행 기록](#계약-테스트-실행-기록) |
| Consumer: `vnc-relay` WSS 리스너 / `crates/adapters/tests/websocket.rs` `expired_untrusted_and_mismatched_certificates_fail_the_client_on_wss`, `tls10_and_tls11_are_refused_and_tls12_and_tls13_succeed_on_wss_and_management` | 2.1절 TLS 협상 실패 행: 만료·신뢰되지 않음·호스트 불일치 인증서는 검증 클라이언트가 협상을 거절한다. TLS 1.0·1.1 접속은 HTTP 응답 없이 닫히고, TLS 1.2 이상은 협상이 성립한다. 평문으로 전환하지 않는다. [WS-013](../specs/websocket.md#7-수용-테스트), [OPS-036](../specs/operations.md#7-수용-테스트)(WSS) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer: `vnc-relay` WS·WSS 리스너 / `crates/adapters/tests/websocket.rs` `origin_must_equal_the_allowed_value`, `origin_must_equal_the_allowed_value_on_wss`, `allowed_source_with_disallowed_origin_is_refused_with_403`, `allowed_source_with_disallowed_origin_is_refused_with_403_on_wss`, `allowed_client_without_origin_or_session_gets_switching_protocols` | 1.3절 Origin 비교: [WS-015](../specs/websocket.md#7-수용-테스트), [WS-027](../specs/websocket.md#7-수용-테스트)(WS·WSS), [WS-021](../specs/websocket.md#7-수용-테스트)(일부) | 부분 | [실행 기록](#계약-테스트-실행-기록) |
| Producer: 고정 버전 noVNC 1.7.0 / Consumer: `vnc-relay` / `tests/websocket_browser.rs` `novnc_repeater_id_completes_rfb_through_ws_and_wss_in_both_server_orders`(`#[ignore]`, 실행 방법은 [noVNC 상호운용 시험 준비](../how-to/set-up-and-verify.md#novnc-상호운용-시험-준비)) | 1.2절 서브프로토콜(noVNC 기본값 포함), 초기 교환 연동: [WS-025](../specs/websocket.md#7-수용-테스트). TCP Server는 시험 코드의 최소 RFB 3.8 서버다. 실제 UltraVNC Server 1.8.3.0과의 조합은 작업 5.5에서 WS·WSS × Server 먼저·noVNC 먼저 모두 확인했다([호환성 검증 기록](repeater-compat-record.md) 6절) | 통과 | noVNC 1.7.0 고정([호환성 검증 기록](repeater-compat-record.md) 2.4절). 두 OS에서 `--include-ignored`로 실행했다([실행 기록](#계약-테스트-실행-기록)). 전체 게이트에서는 무시되어 `미실행`으로 보고된다 |
| Producer: Chromium 내장 WebSocket / Consumer: `vnc-relay` 실행파일 / `tests/websocket_browser.rs` `browser_websocket_relays_mode_i_and_ii_over_ws_and_wss_without_novnc_or_websockify` | noVNC·websockify 없는 배포에서 외부 독립 클라이언트로 WS·WSS Mode I·II 중계: [WS-020](../specs/websocket.md#7-수용-테스트). 제품은 `tokio-tungstenite`로 업그레이드를 처리하므로 같은 라이브러리의 시험 클라이언트는 독립 도구로 세지 않는다(이 문서의 판단, 사용자 결정 없음) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Producer: 테스트용 WebSocket 클라이언트 / Consumer: `vnc-relay` / `crates/adapters/tests/websocket_scenarios.rs` `message_and_fragment_boundaries_preserve_the_same_stream_on_ws_and_wss`, `fragmented_binary_with_ping_pong_and_close_never_leaks_control_bytes_to_tcp`, `invalid_frame_sequences_and_controls_close_1002_and_release_registrations`, `ws_and_wss_kw_keeps_idle_peers_and_drops_unresponsive_peers`; `crates/adapters/src/websocket_stream/tests.rs` `bounds_outbound_messages_and_counts_only_written_payload`; `crates/adapters/tests/websocket_relay.rs` `text_frame_closes_1003_without_forwarding_text_and_logs_typed_error`, `single_and_fragmented_messages_accept_m_and_close_1009_at_m_plus_one`, `ws_and_wss_forward_initial_trailing_data_once` | 1.4절 바이너리·분할·ping·pong·텍스트·M 경계: [WS-007](../specs/websocket.md#7-수용-테스트), [WS-008](../specs/websocket.md#7-수용-테스트), [WS-009](../specs/websocket.md#7-수용-테스트), [WS-010](../specs/websocket.md#7-수용-테스트), [WS-018](../specs/websocket.md#7-수용-테스트), [WS-019](../specs/websocket.md#7-수용-테스트), [WS-026](../specs/websocket.md#7-수용-테스트) | 통과(WS-007~010·018·019, 4e4a271. WS-026, 59fc535) | [실행 기록](#계약-테스트-실행-기록). WS-026은 실제 WS·WSS에서 Pong 없는 활성·대기 Viewer의 제거와 `dead_connection`, Pong에 응답하는 유휴 Viewer의 유지를 확인한다(사용자 결정 2026-11-02(Q84)의 기한 FIFO). |
| Producer: `vnc-relay` / Consumer: 테스트용 WebSocket 클라이언트 / `crates/adapters/src/websocket_stream/tests.rs` `rejects_text_and_invalid_frames_with_typed_errors`·`close_mapping_for_every_relay_close_action`·`kw_expiry_is_codeless_but_other_endpoint_dead_connection_sends_4006`; `crates/adapters/tests/websocket_relay.rs` `text_frame_closes_1003_without_forwarding_text_and_logs_typed_error`, `invalid_initial_requests_close_4008_and_log_the_rejection`, `incomplete_initial_exchange_closes_4009_and_logs_h_timeout`, `disabled_modes_close_1008_before_registration_or_target_connect`, `single_and_fragmented_messages_accept_m_and_close_1009_at_m_plus_one`; `crates/adapters/tests/websocket_scenarios.rs` `invalid_frame_sequences_and_controls_close_1002_and_release_registrations`; `tests/management_terminate.rs` `completion_mgt009_selective_tcp_ws_wss_mode1_mode2_cleanup_and_final_summary` | 1.5절 close 코드와 reason, 2.2절 시나리오: 1000·1001·1002·1003·1008·1009·4001~4011을 확인한다. 1001·4006·4007의 코드·reason은 `close_mapping_for_every_relay_close_action`이 실제 프레임 바이트로 확인한다. Kw 만료의 코드 없는 종료와 상대 TCP 단절의 4006도 구분한다. | 1011 밖의 코드: 통과 | `ee0314f`·`0c0ba5e`의 두 OS 전체 게이트. [작업 5.9 판정표](../how-to/set-up-and-verify.md#41-작업-59-판정표) |
| 시험 없음 | 1.5절 1011 `internal_error` 송신 | 해당 없음 | 예약 코드이며 현재 구현은 보내지 않는다(사용자 결정 2026-11-07(Q152)). 현재 코드에는 1011을 보내는 경로가 없다(`crates/app/src/relay/session.rs`의 `CloseCause::InternalError`는 Close를 만들지 않는다). |
| Producer·Consumer: 테스트용 클라이언트와 TCP 상대 / `crates/adapters/tests/websocket_scenarios.rs` `close_from_either_end_drains_accepted_data_and_records_the_first_cause`, `disconnect_during_transfer_and_missing_close_responses_release_only_the_failed_session`; `crates/adapters/tests/websocket_relay.rs` `client_close_drains_to_tcp_and_echoes_only_the_code`, `tcp_eof_drains_then_forwards_client_data_after_close_1000`, `close_timeout_in_either_order_releases_only_its_session`; `crates/adapters/src/websocket_stream/tests.rs` `finishes_started_frames_before_replying_to_close_within_e`; `tests/unix_shutdown.rs`·`tests/windows_console.rs` `second_request_forces_cleanup` | 1.6절 종료 의미: [WS-016](../specs/websocket.md#7-수용-테스트), [WS-017](../specs/websocket.md#7-수용-테스트), [OPS-042](../specs/operations.md#7-수용-테스트) | 통과(각 시험 범위) | WS-016·017은 커밋 `4e4a271`, OPS-042는 커밋 `de693ad`의 두 OS 실행 기록에서 통과했다. 두 번째 종료 요청은 실행파일의 TCP 세션 강제 정리·최초 종료 원인 보존을 확인한다. WS의 Close 1001 매핑과 전송 중 프레임 처리는 위 단위 시험이 확인한다. `ee0314f`의 두 OS 전체 게이트도 통과했다([작업 5.9 판정표](../how-to/set-up-and-verify.md#41-작업-59-판정표)). |

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
| 커밋 `f62f11d`(WS-020·WS-025 브라우저 시험), 깨끗한 작업 트리 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1, Chrome(`DASHBOARD_BROWSER` 없음, 설치 경로 탐색) | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 153건: 통과 134, 부분 14, 실패 0, 미실행 1(WS-025, 무시됨), 미연결 4. WS-020 통과. websocket.md 27건 중 통과 26, 미실행 1 |
| 같은 커밋 `f62f11d`. 보고서 머리글은 `+ 미커밋 변경`이지만 `git diff --ignore-cr-at-eol --stat` 출력이 없어 줄 끝(CRLF) 차이뿐이다 | Ubuntu 24.04.4 LTS(WSL2), Rust 1.99.1, `/usr/bin/google-chrome` | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 시나리오 153건: 통과 138, 부분 9, 실패 0, 미실행 2(OPS-044 Windows 전용, WS-025 무시됨), 미연결 4. WS-020 통과 |
| `f62f11d`와 같은 시험 소스 | Windows 11 Pro 10.0.26200 x64, `VNC_RELAY_NOVNC_ZIP=D:\vr-lab\installers\novnc-v1.7.0.zip` | `cargo test --test websocket_browser -- --include-ignored` | 통과(2 passed, 42.61초). WS-025는 WS·WSS × Server 먼저·noVNC 먼저 4조합에서 noVNC가 RFB 3.8·보안 유형 None·`SetPixelFormat`·`SetEncodings`를 보내고 첫 화면 갱신을 읽은 뒤 다음 갱신을 요청했다. 탭 제목이 서버 이름으로 바뀌었고, 운영 로그의 `session_started`가 `mode2`와 해당 전송 방식을 기록했다. 다른 압축 파일(`repeater_v160.zip`)을 주면 SHA-256 검사에서 실패함도 확인했다 |
| 같음 | Ubuntu 24.04.4 LTS(WSL2), `VNC_RELAY_NOVNC_ZIP=/mnt/d/vr-lab/installers/novnc-v1.7.0.zip` | `cargo test --test websocket_browser -- --include-ignored` | 통과(2 passed, 43.63초). 확인 내용은 Windows와 같다 |
| 커밋 `70ff541`, 깨끗한 작업 트리 | Windows 11 Pro 10.0.26200 x64, `VNC_RELAY_NOVNC_ZIP=D:\vr-lab\installers\novnc-v1.7.0.zip`(SHA-256 고정값과 일치) | `cargo test --test websocket_browser -- --include-ignored` | 통과(2 passed, 42.57초). WS-020과 WS-025 모두 통과 |
| 같은 커밋 `70ff541` | Ubuntu 24.04 LTS(WSL2), `CARGO_TARGET_DIR=$HOME/vnc-relay-target`, `VNC_RELAY_NOVNC_ZIP=/mnt/d/vr-lab/installers/novnc-v1.7.0.zip`(SHA-256 고정값과 일치) | `cargo test --test websocket_browser -- --include-ignored` | 통과(2 passed, 42.07초). WS-020과 WS-025 모두 통과. [호환성 검증 기록](repeater-compat-record.md) 6.6절의 Ubuntu 증거다(사용자 결정 2026-11-06(Q129)) |
| 커밋 `70ff541`의 Windows release 실행파일(SHA-256 `d94e4dd3…b5fc`) | Hyper-V VM 3대, UltraVNC Server·Viewer 1.8.3.0, noVNC 1.7.0, Edge 152.0.4191.66 | 에이전트가 VM 안의 Viewer·브라우저를 UI 자동 조작으로 실행(사용자 결정 2026-11-05(Q116)의 방식, [호환성 검증 기록](repeater-compat-record.md) 6절) | 통과. 실제 Server를 상대로 WS·WSS Mode I(UltraVNC Viewer와 시험용 바이트 중계 스크립트)과 WS·WSS Mode II(noVNC, Server 먼저·noVNC 먼저)의 화면·입력을 확인했다. 프로젝트 소유자가 캡처를 검토했다(Q130·Q131) |
| 커밋 `d4c5678`(작업 5.9 판정 준비, `70ff541` 뒤 제품 변경은 대시보드 자산뿐), 깨끗한 작업 트리 | Windows 11 Pro 10.0.26200 x64, `VNC_RELAY_NOVNC_ZIP=D:\vr-lab\installers\novnc-v1.7.0.zip` | `cargo test --test websocket_browser -- --include-ignored` | 통과(2 passed, 42.43초). WS-020과 WS-025 모두 통과 |
| 같은 커밋 `d4c5678` | Ubuntu 24.04 LTS(WSL2), `CARGO_TARGET_DIR=$HOME/vnc-relay-target`, `VNC_RELAY_NOVNC_ZIP=/mnt/d/vr-lab/installers/novnc-v1.7.0.zip` | `cargo test --test websocket_browser -- --include-ignored` | 통과(2 passed, 42.31초). WS-020과 WS-025 모두 통과 |
| 커밋 `51bd747`(WSS Server·프록시 TLS 종료·최근 로그. 구현 `3df0693`, 시험 수정 `f772408`·`51bd747`) | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 246건: 통과 229, 부분 12, 실패 0, 미실행 3, 미연결 2, 오류 0, 판정 통과(보고 2026-11-09T00:32:47Z). 작업 트리는 깨끗했다. WS-028~036과 RLY-028의 판정은 [WSS Server·프록시·최근 로그 판정표](../how-to/set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)를 따른다. |
| 같은 커밋 `51bd747` | Ubuntu 24.04.4 LTS x64(WSL2), Rust 1.99.1 | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 시나리오 246건: 통과 232, 부분 8, 실패 0, 미실행 4, 미연결 2, 오류 0, 판정 통과(보고 2026-11-09T01:05:57Z). `+ 미커밋 변경`은 줄 끝 차이다. 같은 구현의 앞선 게이트(`3df0693`, `f772408`)는 Ubuntu에서 시험 결함으로 실패했고 `f772408`·`51bd747`이 고쳤다. WS-028~036과 RLY-028의 판정은 [WSS Server·프록시·최근 로그 판정표](../how-to/set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)를 따른다. |
| 커밋 `41dda1f`(2.0.0 Viewer 경로 `/relay/viewer`), 작업 트리 깨끗함 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 246건: 통과 229, 부분 12, 실패 0, 미실행 3, 미연결 2, 오류 0, 판정 통과(보고 2026-11-10T12:05:09Z). WS-012·WS-029 통과. 판정은 [Viewer 경로 이전 판정표](../how-to/set-up-and-verify.md#44-viewer-경로-이전-판정표)를 따른다. |
| 같은 커밋 `41dda1f` | Ubuntu 24.04.4 LTS x64(WSL2), Rust 1.99.1 | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 시나리오 246건: 통과 232, 부분 8, 실패 0, 미실행 4, 미연결 2, 오류 0, 판정 통과(보고 2026-11-10T12:39:25Z). `+ 미커밋 변경`은 줄 끝 차이다. WS-012·WS-029 통과. |
| `41dda1f`와 같은 시험 소스(커밋 직전 작업 트리) | Windows 11 Pro 10.0.26200 x64, `VNC_RELAY_NOVNC_ZIP=D:\vr-lab\installers\novnc-v1.7.0.zip` | `cargo test --test websocket_browser -- --include-ignored` | 통과(2 passed, 43.54초). WS-020과 WS-025 모두 통과. noVNC `vnc.html`의 `path=relay/viewer`로 WS·WSS × Server 먼저·noVNC 먼저 4조합에서 Viewer 등록과 RFB 교환을 확인했다 |
| 커밋 `41dda1f` | Ubuntu 24.04.4 LTS(WSL2), `CARGO_TARGET_DIR=$HOME/vnc-relay-target`, `VNC_RELAY_NOVNC_ZIP=/mnt/d/vr-lab/installers/novnc-v1.7.0.zip` | `cargo test --test websocket_browser -- --include-ignored` | 통과(2 passed, 42.02초). 확인 내용은 Windows와 같다 |

## 6. 초안을 확정하기 전에 해결할 사항

- 이 계약의 이름·코드 선택은 2026-10-16에 사용자가 채택했다. 해당하는 것: 업그레이드 경로 처리의 세부(쿼리 무시, 끝 슬래시 불일치), HTTP 상태(특히 서브프로토콜 거절 400, 종료 유예 503, 상한 L 도달 시 연결 종료), 검사 순서, 확장 비수락, close 코드 번호·이름·reason, 1.6절 종료 의미, 서버 메시지 크기 M 이하.
- 채택으로 WS-TBD-002(HTTP 상태·종료 코드)와 WS-TBD-003(Origin 정규화)을 해소했다. WS-TBD-003의 기준은 [설정 계약 1.4절](config-v4.md#14-origin-값-문법과-비교)이며 이 계약은 그 규칙을 인용하므로 따로 맞출 값이 없다.
- (해결) WS-TBD-004: M·U·E의 기본값과 유효 범위는 사용자가 승인한 값이며 [설정 계약](config-v4.md)이 기준이다. B와 M 사이에 크기 제약은 없다(사용자 결정 2026-11-01(Q78)). 프레임 안에서 스트리밍하며 방향당 미전달 보관 바이트는 B 이하다. 분할 프레임을 합친 논리 메시지 크기가 M을 넘으면 1009로 닫고 추가 전달을 멈춘다. B보다 큰 유효 프레임도 수락해 스트리밍하며 전체를 저장하지 않는다. 프레임 분할 우회와 종료 중 자원 점유는 WS-010·WS-017로 검증한다.
- (해결) WS-TBD-005와 noVNC 기본 서브프로토콜: noVNC 1.7.0을 [호환성 검증 기록](repeater-compat-record.md) 2.4절에 고정했다(버전·해시·받은 주소·날짜). 2026-11-01 관찰에서 이 버전은 WS·WSS 모두 `Sec-WebSocket-Protocol` 헤더를 보내지 않았다(같은 기록 3.4절 W-02·W-03b, 3.5절 F행). 1.2절 규칙과 같아 규칙은 그대로 두고, 1.2절 noVNC 문단을 관찰 사실로 고쳤다(TODO 1.7).
- 1.4절 잘못된 프레임의 Close 1002와 자원 회수는 `crates/adapters/tests/websocket_scenarios.rs`의 `invalid_frame_sequences_and_controls_close_1002_and_release_registrations`가 WS·WSS에서 확인했다. 두 OS 전체 게이트에서 통과했다(5절).
- 후속: 1.2.0(사용자 결정 2026-11-08(Q181·Q182·Q188))의 1.7·1.8절과 변경된 검사 순서는 커밋 `3df0693`에서 구현했다. 시험은 커밋 `51bd747`의 두 OS 게이트에서 통과했다([WSS Server·프록시·최근 로그 판정표](../how-to/set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)). 위 날짜별 관찰·실행 기록은 그 당시 범위의 증거로 유지한다.
- 후속: 2.0.0(사용자 결정 2026-11-10)은 Viewer 경로만 `/relay/viewer`로 바꿨다. 구현은 커밋 `41dda1f`다. 시험은 같은 커밋의 두 OS 게이트에서 통과했다([Viewer 경로 이전 판정표](../how-to/set-up-and-verify.md#44-viewer-경로-이전-판정표)). 이전과 롤백은 3.1절을 따른다.
- 후속: 2026-11-07(Q160) 이후 종료 시 503을 보내지 않는다(2.1절). 위 2026-10-16 채택 기록의 “종료 유예 503”은 과거 기록이다.
- (해결) 사용자 결정 2026-11-08(Q191)으로 채택했다: `trusted_proxies` 전체 허용 경고 코드·사건명 `trust_all_proxies`. 이름과 필드의 채택은 [운영 정보 계약](ops-info-v2.md)의 결정 기록에서 관리한다.
- 2.1.0의 접속 ticket은 사용자 결정 2026-11-11(Q192~Q204·Q207·Q212·Q213)으로 채택했다. 구현·계약 테스트는 미실행(구현 전)이다. WS-037~046·RLY-029~032는 미연결이며 5절의 계약 테스트 계획으로 확인해야 한다.
- 접속 ticket 설정 입력과 `session_expired` 표시 문구는 [대시보드 스펙의 DASH-TBD-006](../specs/dashboard.md#8-미결정-사항)에 남긴다. 구현 작업에서 시안·문구(CPY)를 함께 정한다(사용자 결정 2026-11-11(Q214)).
