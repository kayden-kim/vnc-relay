# 관리 API v1 계약

관리 리스너가 제공하는 JSON over HTTP 관리 API의 경로·메서드·요청·응답·오류와 로그인 교환 형식을 정한다. 내장 대시보드(정적 HTML·JS)가 이 API로 서버 요약·대기 연결·활성 세션·최근 종료 목록을 조회하고, 활성 세션을 종료하고, 관리자 로그인·로그아웃을 한다. 이 문서는 [관리 스펙](../specs/management.md)과 [화면 스펙](../specs/dashboard.md)의 동작을 바꾸지 않는다. 스펙과 다르면 스펙을 따른다.

| 항목 | 값 |
| --- | --- |
| 이름과 버전 | `management-api` 1.1.0 |
| 상태 | Active(2026-10-16 채택) |
| 변경 | 1.1.0(2026-10-22): 1.11절 CSP 헤더를 권장에서 필수로 바꿨다(사용자 결정 2026-10-22(Q37)). 소비하는 쪽의 동작은 바뀌지 않는다. 만드는 쪽은 CSP 헤더를 보내야 한다. 명확화(2026-10-22, 버전 유지): 2.3절에서 로그인 요청(`POST /api/v1/session`)과 로그아웃 요청(`DELETE /api/v1/session`)의 `500`·응답 없음·시간 초과를 조회 실패와 나눴다(사용자 결정 2026-10-22(Q47·Q48)). 경로·필드·상태 코드·오류 코드는 그대로이며, 화면 스펙이 정한 대시보드 동작을 계약 표에 맞춘 것이다. 명확화(2026-10-23, 버전 유지): 2.3절에서 로그아웃 요청의 `403 csrf_invalid`를 로그아웃 결과 확인 필요로 처리한다(사용자 결정 2026-10-23(Q54)). 6절의 전송량 안전 범위 항목을 닫았다(사용자 결정 2026-10-23(Q53)). 경로·필드·상태 코드·오류 코드와 만드는 쪽의 동작은 바뀌지 않는다. 명확화(2026-11-01, 버전 유지): 1.7절 `relay_id`의 값을 [Repeater 초기 교환 계약 1.2절](repeater-handshake-v1.md#12-초기-요청-250바이트)의 중계 ID(ASCII 숫자 1~247자)로 적고, 6절의 중계 ID 표현 항목을 닫았다(TODO 1.7). 자료형은 문자열 그대로이고 만드는 쪽이 낼 수 있는 값의 범위만 적었으므로 3절의 호환성을 깨는 변경이 아니다(이 문서의 판단, 사용자 결정 없음). 명확화(2026-11-01, 버전 유지): 1.10절 `server_shutdown`의 적용 조건을 명확히 했다(사용자 결정 2026-11-01(Q82)). 강제 정리 시점에 이미 정리를 시작한 세션은 최초 원인을 유지하고 아직 정리를 시작하지 않은 세션에만 `server_shutdown`을 쓰도록 명시했다. 필드·값 목록이 바뀌지 않고 내부 판정 기준의 명확화이므로 3절에 따라 버전을 유지한다(이 문서의 판단, 사용자 결정 없음). |
| 결정 반영 | 명확화(2026-11-02, 1.1.0 유지): `address.ip`의 IPv4 매핑 IPv6 정규화(사용자 결정 2026-11-02(Q88)), terminate의 본문 금지 규칙 우선(사용자 결정 2026-11-02(Q89)), 검증·수락된 종료 완료 대기 중 U 일시 중지(사용자 결정 2026-11-02(Q92))를 반영했다. 2.1절 경로 표에서 빠진 415(GET 경로와 `DELETE /api/v1/session`)를 1.1절 공통 규칙과 `a143539`의 기존 구현에 맞춰 보완했다(T415). 동작은 바뀌지 않는다. |
| 호환 값 추가 | 2026-11-02, 1.1.0 유지: `close_cause`에 `internal_error`를 더했다(사용자 결정 2026-11-02(Q94)). 기존 값과 필드는 바뀌지 않는다. 1.10·3절에 따라 Consumer는 알 수 없는 값을 받은 그대로 표시하므로 호환 변경이다. |
| 소유자 | vnc-relay 프로젝트 관리자 |
| 만드는 쪽(Producer) | `vnc-relay` 관리 리스너(서버) |
| 사용하는 쪽(Consumer) | 같은 실행파일에 내장된 대시보드(정적 HTML·JS). 같은 규칙으로 호출하는 외부 HTTP 클라이언트도 Consumer로 본다. |
| 방향 | 요청/응답. 브라우저 → 관리 리스너. HTTPS가 원칙이며 평문 HTTP는 루프백 관리 리스너에서만 쓴다. |
| 스키마 기준 파일 | 없음. 스키마 파일을 만들지 않는다. 1절의 표가 형식의 기준이다. |
| 기준 스펙 | [관리 조회와 세션 종료](../specs/management.md), [운영 대시보드 화면](../specs/dashboard.md). L·Lm·E·P 등 매개변수 정의는 [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)을 따른다. |
| 관련 요구사항 | [PRD](../requirements/PRD.md) FR-06, FR-08(비밀 비노출), 7절 보안 경계, AC-09·AC-10·AC-19~AC-21 |
| 호환성 범위 | 경로 접두사 `/api/v1/`. 같은 릴리스의 서버와 내장 대시보드. HTTPS 관리 리스너와 평문 HTTP 루프백 관리 리스너. |
| 관련 테스트 | [관리 스펙 7절](../specs/management.md#7-수용-테스트) MGT-001~MGT-027, [화면 스펙 7절](../specs/dashboard.md#7-수용-테스트) DASH-001~DASH-023, [운영 스펙 7절](../specs/operations.md#7-수용-테스트) OPS-043. 계약 테스트는 작업 4.2·4.3의 `crates/adapters/tests/management.rs`와 작업 4.3의 `tests/management_session.rs`다(5절). |

관련 문서: [설정 계약](config-v2.md)(`[dashboard]`·`[limits]`·`[recent_closed]`·`[timeouts]`·`[management]` 키와 기본값·범위), [CLI·계정 파일 계약](cli-v1.md)(계정명 규칙), [운영 정보 계약](ops-info-v1.md)(`session_id`·종료 사건 로그, 상태 리스너), [WebSocket 중계 계약](websocket-relay-v1.md)(관리자 종료의 Close 4005), [ADR-0001 내장 TLS](../decisions/ADR-0001-built-in-tls.md), [ADR-0003 단일 인스턴스·메모리 상태](../decisions/ADR-0003-single-instance-memory-state.md).

이 계약은 2026-10-16에 사용자가 채택했다(상태 Active). `/api/v1/` 아래 경로 이름과 메서드, 필드 이름, 오류 코드 일곱 개, `X-Relay-Poll`·`X-CSRF-Token` 값 규칙, 종료 원인 값 이름, 화면 URL, `WWW-Authenticate` 헤더 값이 모두 채택 범위다. 6절에 "관찰로 확인 필요"·"구현에서 확인"으로 남긴 항목은 채택으로 해소되지 않았다. 앞서 사용자가 확정한 값은 다음과 같다.

- 관리 API는 JSON over HTTP이고 경로 접두사는 `/api/v1/`이다.
- 시각은 UTC ISO 8601 문자열이고, 전송량은 바이트 정수다. 화면이 현지 시각과 KiB/MiB로 바꿔 표시한다.
- 관리 세션 쿠키는 HttpOnly·SameSite=Strict이고, HTTPS에서는 Secure를 붙인다. 평문 HTTP 루프백 관리 리스너에서만 Secure를 뺀다. CSRF 방어를 별도로 유지한다.
- 관리 세션은 서버 메모리에 두며, 재시작하면 모두 무효다.
- 종료 요청은 정리가 끝난 뒤 응답한다. 별도의 "처리 중" 응답은 없다.
- 폴링 주기 P(`dashboard.poll_secs`)와 폴링 조회 제한 시간(`dashboard.query_timeout_secs`)의 기본값·유효 범위는 [설정 계약 1.2절](config-v2.md#12-키-표)이 정한다. 이 문서는 수치를 반복하지 않는다.

## 1. 형식과 의미

### 1.1 공통 규칙

| 항목 | 규칙 |
| --- | --- |
| 전송 | 관리 리스너 하나에서 정적 대시보드(`/`)와 API(`/api/v1/`)를 함께 제공한다. 상태 리스너(`/livez`·`/readyz`)는 이 계약의 대상이 아니다. |
| 본문 형식 | 요청·응답 본문은 UTF-8 JSON이다. 응답은 `Content-Type: application/json; charset=utf-8`이다. 본문이 있는 요청은 `Content-Type: application/json`을 보내야 한다. 다르면 `415`와 `invalid_request`로 거절한다. 단, 1.9절의 본문 금지처럼 경로별 규칙이 있으면 그 규칙을 먼저 적용한다(사용자 결정 2026-11-02(Q89)). |
| 응답 헤더 | API 응답에 `Cache-Control: no-store`를 붙인다. CORS 헤더(`Access-Control-*`)를 보내지 않는다. 같은 출처의 대시보드만 API를 호출한다. |
| 필드 이름 | 소문자 `snake_case`다. |
| 시각(`timestamp`) | 문자열. UTC ISO 8601. 형식은 `YYYY-MM-DDThh:mm:ss.sssZ`다. 밀리초를 항상 3자리로 쓰고, 오프셋은 항상 `Z`다. 현지 시각 변환은 Consumer가 한다. |
| 전송량(`bytes`) | 정수. 0 이상. 단위는 바이트다. 단위를 바꾸거나 반올림한 값을 넣지 않는다. 의미는 [관리 스펙 2.1절](../specs/management.md#21-전송량의-의미)을 따른다. |
| 경과 시간(`duration_ms`) | 정수. 0 이상. 단위는 밀리초다. 서버가 단조 시계로 계산한다. 시스템 시각을 보정해도 음수가 되지 않는다. |
| 기준 시점 | 모든 `GET` 성공 응답은 최상위 `as_of`(`timestamp`)를 가진다. 한 응답 안의 집계와 목록은 `as_of` 시점에 일관된다. 서로 다른 요청의 결과가 같은 시점이라고 보장하지 않는다. 서버 요약의 `usage`(L·Lm 사용량)는 이 일관성의 대상이 아니며, 연결이 바뀌는 동안 같은 응답의 `counts`와 잠깐 어긋날 수 있다([관리 스펙 2.2절](../specs/management.md#22-주소와-스냅샷), 사용자 결정 2026-11-04(Q100)). |
| 목록 | 목록은 항상 `items` 배열이다. 정상적으로 조회한 빈 목록은 `200`과 `"items": []`다. 조회 실패나 인증 실패로 대신하지 않는다. |
| 정렬 | `waiting`·`sessions`는 오래된 항목이 먼저다(`connected_at`·`started_at` 오름차순). `closed-sessions`는 최근 종료가 먼저다(`ended_at` 내림차순). 같은 시각 항목의 순서는 정하지 않는다. |
| null과 확인 불가 | 값을 읽지 못하면 0으로 대신하지 않는다. 그 필드를 `null`로 주고, 같은 객체의 `<필드>_unavailable_reason`에 사유 문자열을 준다. 값이 있으면 사유는 `null`이다. 이 규칙은 1.6절 표가 "확인 불가 가능"으로 표시한 필드에만 적용한다. 그 밖의 필드는 항상 값을 가진다. |
| 확인 불가 사유 | 현재 값은 `read_failed`(값을 읽는 데 실패)다. Consumer는 모르는 사유 문자열을 일반 "확인 불가"로 표시한다. |
| 상대 주소(`address`) | 객체 `{"ip": 문자열, "port": 정수}`. 실제 소켓 상대 주소다. 전달 헤더(`Forwarded`·`X-Forwarded-For` 등)로 바꾸지 않는다. `ip`는 `IpAddr::to_canonical()`을 적용한 표기다. IPv4 매핑 IPv6(`::ffff:a.b.c.d`)는 `a.b.c.d`로 쓴다. 그 밖의 IPv6는 괄호 없는 소문자 압축 표기다. `PeerIp`와 운영 로그도 같은 규칙을 쓴다(사용자 결정 2026-11-02(Q88)). |
| 노출 범위 | 아래 1.1.1절을 따른다. |

#### 1.1.1 운영 정보 노출 범위

노출 범위는 [관리 스펙 2절](../specs/management.md#2-조회-대상과-상태)과 MGT-R08을 따른다. 이 계약이 범위를 넓히지 않는다.

| 정보 | 나오는 곳 | 나오지 않는 곳 |
| --- | --- | --- |
| 중계 ID(`relay_id`) | `GET /api/v1/waiting`의 항목만 | 활성 세션 목록·상세, 최근 종료 목록, 오류 본문, 쿠키 |
| 세션 식별자(`session_id`) | 활성 세션 목록·상세, 최근 종료 목록, 종료 응답 | 대기 연결 목록(세션이 아직 없음) |
| 상대 주소(`viewer_address`·`target_address`) | 활성 세션 목록·상세 | 대기 연결 목록, 최근 종료 목록, 오류 본문 |
| 관리자 비밀번호·비밀번호 해시·관리 세션 쿠키 값 | 어디에도 없음. 비밀번호는 로그인 요청 본문에만 있다. | 모든 응답·오류·로그 |
| CSRF 토큰 | 로그인 응답과 `GET /api/v1/session` 응답만 | 그 밖의 모든 응답 |
| 원시 중계 데이터(RFB 스트림) | 어디에도 없음 | 모든 응답 |

오류 본문의 `message`에는 요청 입력값(계정명·비밀번호·세션 식별자·중계 ID)을 되풀이하지 않는다.

### 1.2 경로 목록과 스펙 대응

모든 경로는 관리 리스너에 있다. "관리 세션"은 유효한 관리 세션 쿠키를 뜻한다. "CSRF"는 `X-CSRF-Token` 헤더 검증을 뜻한다.

| 메서드·경로 | 관리 세션 | CSRF | 성공 응답 | 대응하는 스펙 동작 |
| --- | --- | --- | --- | --- |
| `POST /api/v1/session` | 불필요 | 불필요 | `200` 세션 정보(1.5절) + `Set-Cookie` | 관리자 로그인([관리 3.0절](../specs/management.md#30-관리자-로그인과-세션), MGT-R12·R14) |
| `GET /api/v1/session` | 필요 | 불필요 | `200` 세션 정보(1.5절) | 화면 진입·새로고침 때 유효한 관리 세션 확인과 CSRF 토큰 복구(SCR-001 진입 조건, [화면 2절](../specs/dashboard.md#2-화면-흐름)) |
| `DELETE /api/v1/session` | 필요 | 필요 | `204` 본문 없음 + 쿠키 삭제 | 로그아웃(MGT-R13·R09, DASH-019) |
| `GET /api/v1/summary` | 필요 | 불필요 | `200` 서버 요약(1.6절) | 서버 요약(MGT-R01·R17) |
| `GET /api/v1/waiting` | 필요 | 불필요 | `200` 대기 연결 목록(1.7절) | 대기 연결 목록(MGT-R01) |
| `GET /api/v1/sessions` | 필요 | 불필요 | `200` 활성 세션 목록(1.8절) | 활성 세션 목록(MGT-R01) |
| `GET /api/v1/sessions/{session_id}` | 필요 | 불필요 | `200` 활성 세션 상세(1.8절) | 세션 상세(MGT-R02) |
| `POST /api/v1/sessions/{session_id}/terminate` | 필요 | 필요 | `200` 종료 결과(1.9절) | 관리자 종료(MGT-R05·R06·R07, [관리 3.2절](../specs/management.md#32-관리자-종료)) |
| `GET /api/v1/closed-sessions` | 필요 | 불필요 | `200` 최근 종료 목록(1.10절) | 최근 종료 목록(MGT-R15, [관리 2.3절](../specs/management.md#23-최근-종료-목록)) |

`GET /api/v1/session`은 관리 스펙이 직접 이름 붙인 동작이 아니다. 화면 스펙이 "유효한 관리 세션으로 진입"을 요구하기 때문에 추가한 경로다. 화면이 새로고침하면 메모리의 CSRF 토큰이 사라진다. 이 경로가 같은 관리 세션의 토큰을 다시 준다. 이 경로도 채택 범위다.

위 표에 없는 `/api/v1/` 경로는 `404 not_found`다. 경로는 있으나 메서드가 다르면 `405 invalid_request`이고 `Allow` 헤더를 붙인다.

### 1.3 요청 처리 순서

서버는 `/api/v1/` 요청을 다음 순서로 판정한다. 앞 단계에서 거절하면 뒤 단계는 하지 않는다.

1. `POST /api/v1/session`이면 5단계의 로그인 처리로 간다. 그 밖의 요청은 2단계로 간다.
2. 쿠키로 관리 세션을 찾는다. 유효하지 않으면 `401 unauthenticated`다. 경로·메서드가 틀려도 같다. 인증 없는 호출자가 경로 존재 여부를 알 수 없게 한다.
3. 상태를 바꾸는 요청(`POST`·`DELETE`)이면 `X-CSRF-Token`을 검증한다. 틀리면 `403 csrf_invalid`다.
4. 경로·메서드·`X-Relay-Poll` 헤더·`session_id` 형식·본문을 검증한다. 오류는 `404 not_found`·`405`·`400`·`415`다.
5. 요청을 처리한다. 사용자 행동 요청이면 유휴 만료를 연장한다(1.4.2절).

### 1.4 관리 세션

#### 1.4.1 쿠키

| 속성 | HTTPS 관리 리스너 | 평문 HTTP 루프백 관리 리스너 |
| --- | --- | --- |
| 이름 | `relay_admin_session` | 같음 |
| 값 | 불투명 식별자. 서버 메모리의 관리 세션을 가리킨다. 128비트 이상 무작위 값이다. 계정명·비밀번호·시각을 담지 않는다. | 같음 |
| `HttpOnly` | 있음 | 있음 |
| `SameSite` | `Strict` | `Strict` |
| `Secure` | 있음 | 없음(이 경우에만 뺀다) |
| `Path` | `/` | `/` |
| `Domain`·`Expires`·`Max-Age` | 없음(호스트 전용 세션 쿠키). 만료는 서버가 Si·Sa로 판정한다. | 같음 |

- 서버는 쿠키에 비밀번호를 넣지 않고, URL에도 넣지 않는다.
- 로그아웃과 `401` 응답(쿠키가 있었던 경우)은 같은 이름·`Path`로 `Max-Age=0`인 `Set-Cookie`를 보내 쿠키를 지운다.
- 유효한 관리 세션은 다음 조건을 모두 만족한다. 이 프로세스가 발급했다. 로그아웃되지 않았다. 유휴 만료 Si 전이다. 절대 최대 수명 Sa 전이다. 프로세스를 재시작하면 이전 관리 세션은 모두 무효다(MGT-R18).
- 로그인이 성공하면 항상 새 관리 세션을 만든다. 요청에 유효한 이전 쿠키가 있으면 그 관리 세션을 무효로 한다.
- 로그아웃과 관리 세션 만료는 활성 세션(중계)을 종료하지 않는다.

#### 1.4.2 자동 폴링 표시와 유휴 만료

자동 폴링 요청은 요청 헤더 `X-Relay-Poll: 1`로 표시한다. 이 헤더가 없는 인증된 `/api/v1/` 요청은 사용자 행동 요청이다.

| 요청 | 헤더 | 유휴 만료 |
| --- | --- | --- |
| 타이머로 보내는 폴링 조회 | `X-Relay-Poll: 1` | 연장하지 않는다. |
| 사용자 행동으로 보내는 요청(화면 진입, 새로고침, 상세 열기, 종료, 로그아웃) | 없음 | 연장한다. 새 `idle_expires_at`은 `min(지금 + Si, absolute_expires_at)`이다. |

- `X-Relay-Poll`은 `GET` 요청에만 쓴다. 값은 `1`뿐이다. 다른 값이거나 `POST`·`DELETE`에 붙이면 `400 invalid_request`다.
- 절대 최대 수명 Sa는 헤더와 활동에 관계없이 `absolute_expires_at`에 무효가 된다.
- 연장은 인증과 CSRF 검증을 통과한 요청만 일으킨다. 거절된 요청은 연장하지 않는다. 정적 파일 요청도 연장하지 않는다.
- Consumer는 자동 폴링에만 헤더를 붙인다. 어떤 화면 행동이 사용자 행동인지는 Consumer가 정한다.
- 서버는 유휴 만료·절대 만료·로그아웃·재시작 뒤 쿠키를 모두 같은 `401 unauthenticated`로 거절한다. 원인을 구분해 알려주지 않는다.

#### 1.4.3 CSRF 방어

- 로그인과 `GET /api/v1/session` 응답의 `csrf_token`은 그 관리 세션이 끝날 때까지 같은 값이다.
- Consumer는 `POST`·`DELETE` 요청마다 헤더 `X-CSRF-Token: <csrf_token>`을 보낸다.
- 서버는 헤더가 없거나 값이 다르면 `403 csrf_invalid`로 거절한다. 관리 세션은 유지하고 대상 상태를 바꾸지 않는다.
- 로그인 요청은 관리 세션이 없으므로 CSRF 토큰이 없다. 대신 `Content-Type: application/json`을 요구한다. 서버는 CORS 헤더를 보내지 않는다. 쿠키는 `SameSite=Strict`다.
- `csrf_token`은 URL이나 로그에 쓰지 않는다. 쿠키로 보내지 않는다.

### 1.5 로그인과 세션 정보

#### `POST /api/v1/session` 요청

| 필드·메시지 | 자료형 | 필수 여부·null 허용 여부 | 의미·제약 |
| --- | --- | --- | --- |
| 헤더 `Content-Type` | 문자열 | 필수 | `application/json` |
| `account` | 문자열 | 필수, null 불가 | 관리자 계정명. 비어 있으면 안 된다. 허용 문자와 길이는 [CLI 계약 1.2절](cli-v1.md#12-계정-이름과-비밀번호-규칙)의 계정명 규칙을 따른다(6절). |
| `password` | 문자열 | 필수, null 불가 | 비밀번호. 비어 있으면 안 된다. 응답·로그·오류에 되풀이하지 않는다. |

- 본문이 JSON이 아니거나, 필수 필드가 없거나 비어 있거나, 문자열이 아니거나, 본문이 16 KiB(16384바이트)를 넘으면 `400 invalid_request`다. 이 경우는 로그인 실패 횟수 F에 세지 않는다.
- 알 수 없는 필드는 무시한다.
- 계정이 없거나, 비밀번호가 틀리거나, 제한 중이면 모두 같은 `401 login_failed`다. 본문·헤더·상태 코드가 같다. 계정 존재 여부를 알 수 있는 차이를 두지 않는다.
- 계정이 없을 때도 있는 계정과 같은 비용의 비밀번호 검증을 수행하는 것을 권장한다. 응답 시간으로 존재 여부를 알 수 없어야 한다.
- 실패는 계정명과 실제 소켓 출발지 IP로 센다. 없는 계정명도 센다. 기준 F(`per_account`·`per_source`·`window_secs`·`lockout_secs`)를 넘으면 제한 시간 동안 로그인 시도를 `login_failed`로 거절한다. 제한 중에는 올바른 비밀번호도 거절한다. 같은 출발지의 다른 계정 시도도 제한을 받는다(MGT-022).
- 서버는 `Retry-After`를 보내지 않는다. 제한 중인지 실패인지 알려주지 않는다.

#### 세션 정보(`POST /api/v1/session`과 `GET /api/v1/session`의 200 응답 본문)

| 필드·메시지 | 자료형 | 필수 여부·null 허용 여부 | 의미·제약 |
| --- | --- | --- | --- |
| 헤더 `Set-Cookie` | 문자열 | 로그인 응답에만 있음 | 1.4.1절의 `relay_admin_session` 쿠키 |
| `account` | 문자열 | 필수, null 불가 | 로그인한 계정명 |
| `csrf_token` | 문자열 | 필수, null 불가 | 상태 변경 요청의 `X-CSRF-Token` 값. 256비트 무작위 값이며 URL 안전 문자만 쓴다. |
| `idle_expires_at` | `timestamp` | 필수, null 불가 | 응답 시점의 유휴 만료 시각. `absolute_expires_at`을 넘지 않는다. 자동 폴링은 이 값을 바꾸지 않는다. |
| `absolute_expires_at` | `timestamp` | 필수, null 불가 | 절대 최대 수명 Sa가 끝나는 시각. 로그인 이후 변하지 않는다. |
| `dashboard.poll_secs` | 정수 | 필수, null 불가 | 폴링 주기 P(초). 설정 `[dashboard] poll_secs`의 값이다. 기본값·범위는 [설정 계약](config-v2.md#12-키-표). |
| `dashboard.query_timeout_secs` | 정수 | 필수, null 불가 | 폴링 조회 제한 시간(초). 설정 `[dashboard] query_timeout_secs`의 값이다. 기본값·범위는 [설정 계약](config-v2.md#12-키-표). |
| `dashboard.terminate_timeout_secs` | 정수 | 필수, null 불가 | 종료 요청의 응답 대기 한도(초). 서버가 설정 `[timeouts] close_secs`(E)에 5초를 더해 계산한다. |

폴링 주기 P는 재시작이 필요한 설정이므로 관리 세션이 살아 있는 동안 바뀌지 않는다. 그래서 세션 정보로 전달한다. 화면은 로그인 직후와 새로고침 직후 이 값을 받아 쓴다. 서버 요약에는 P를 넣지 않는다.

### 1.6 서버 요약 `GET /api/v1/summary`

| 필드·메시지 | 자료형 | 필수 여부·null 허용 여부 | 의미·제약 |
| --- | --- | --- | --- |
| `as_of` | `timestamp` | 필수 | 1.1절 |
| `server.state` | 문자열 | 필수, null 불가 | 실행 상태. `ready`(준비됨), `shutdown_grace`(종료 유예 중). 유예 중에도 관리 조회·로그인·관리자 종료 요청을 처리한다([운영 스펙 3.4절](../specs/operations.md#34-정상-종료와-재시작)). 알 수 없는 값은 받은 그대로 표시한다. |
| `server.uptime_ms` | `duration_ms` | 필수, null 불가 | 현재 프로세스의 가동 시간. 재시작하면 0부터 다시 센다. |
| `counts.waiting` | 정수 | 필수, null 불가 | 대기 연결 수(Mode II 상대 대기 중인 연결만) |
| `counts.active` | 정수 | 필수, null 불가 | 활성 세션 수. 연결 쌍 하나가 1이다. 종료 중인 세션도 정리가 끝날 때까지 포함한다. |
| `counts.handshaking` | 정수 | 확인 불가 가능, `null` 허용 | 초기 교환을 아직 마치지 않은 연결 수. 대기 연결 수에 포함하지 않는다. |
| `counts.handshaking_unavailable_reason` | 문자열 | 필수, `counts.handshaking`이 `null`일 때만 값이 있고 아니면 `null` | 1.1절의 확인 불가 사유 |
| `counts.target_connecting` | 정수 | 확인 불가 가능, `null` 허용 | Mode I에서 VNC Relay가 대상에 연결을 시도하고 있는 연결 수. 대기 연결 수에 포함하지 않는다. |
| `counts.target_connecting_unavailable_reason` | 문자열 | 필수, 값이 `null`일 때만 값이 있고 아니면 `null` | 1.1절의 확인 불가 사유 |
| `usage.relay_connections.current` | 정수 | 확인 불가 가능, `null` 허용 | L 사용량. 중계 측 연결 소켓의 현재 수. 집계 범위는 [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)의 L을 따른다. |
| `usage.relay_connections.current_unavailable_reason` | 문자열 | 필수, 값이 `null`일 때만 값이 있고 아니면 `null` | 1.1절의 확인 불가 사유 |
| `usage.relay_connections.limit` | 정수 | 필수, null 불가 | L 상한. 설정 `[limits] relay_connections`의 값. 상한에 도달하면 `current`는 `limit`과 같다. |
| `usage.admin_connections.current` | 정수 | 확인 불가 가능, `null` 허용 | Lm 사용량. 관리·상태 리스너 연결 소켓의 현재 수. L과 독립해 센다. |
| `usage.admin_connections.current_unavailable_reason` | 문자열 | 필수, 값이 `null`일 때만 값이 있고 아니면 `null` | 1.1절의 확인 불가 사유 |
| `usage.admin_connections.limit` | 정수 | 필수, null 불가 | Lm 상한. 설정 `[limits] admin_connections`의 값. |

- 연결 수 의미의 기준은 [관리 스펙 2절](../specs/management.md#2-조회-대상과-상태)이다. 이 표는 그 의미를 다시 정의하지 않는다.
- 읽지 못한 값을 0으로 주지 않는다. Consumer도 `null`을 0으로 표시하지 않는다(DASH-021).
- `counts.waiting`·`counts.active`는 같은 `as_of`에서 `waiting`·`sessions` 목록의 길이와 같다. 서로 다른 요청의 값은 다를 수 있다.

### 1.7 대기 연결 목록 `GET /api/v1/waiting`

최상위 필드는 `as_of`와 `items`다. `items`의 각 항목은 다음과 같다.

| 필드·메시지 | 자료형 | 필수 여부·null 허용 여부 | 의미·제약 |
| --- | --- | --- | --- |
| `role` | 문자열 | 필수, null 불가 | 대기 연결의 역할. `viewer` 또는 `server`. |
| `relay_id` | 문자열 | 필수, null 불가 | 중계 ID. 운영 정보이며 인증 비밀이 아니다. 값은 [Repeater 초기 교환 계약 1.2절](repeater-handshake-v1.md#12-초기-요청-250바이트)의 중계 ID 바이트열을 그대로 담는다. ASCII 숫자(`0`~`9`) 1~247자이므로 JSON 이스케이프가 필요 없다. 앞자리 0을 지우거나 수로 바꾸지 않는다(`0123`과 `123`은 다른 값). Consumer도 수로 바꾸지 않고 받은 문자열 그대로 표시한다. 긴 ID는 JavaScript 수로 정확히 나타낼 수 없다. |
| `connected_at` | `timestamp` | 필수, null 불가 | 접속 시각 |
| `waited_ms` | `duration_ms` | 필수, null 불가 | `as_of` 시점까지의 대기 시간 |

### 1.8 활성 세션 `GET /api/v1/sessions`, `GET /api/v1/sessions/{session_id}`

목록은 최상위 `as_of`와 `items`이고, 상세는 최상위 `as_of`와 `session`이다. 두 응답은 같은 세션 객체를 쓴다.

세션 식별자 `session_id`의 외부 형식:

- 정규식 `^[A-Za-z0-9_-]{1,64}$`를 만족하는 대소문자 구분 문자열이다. Consumer는 이 범위를 URL 경로에 그대로 쓸 수 있다.
- 불투명 값이다. Consumer는 안의 내용을 해석하지 않는다.
- Producer는 한 프로세스 수명 안에서 같은 값을 다시 발급하지 않는다. 같은 중계 ID의 재접속은 새 `session_id`를 받는다.
- Producer가 발급하는 값은 `ses_` 뒤에 소문자 영문·숫자 26자(무작위)다.

| 필드·메시지 | 자료형 | 필수 여부·null 허용 여부 | 의미·제약 |
| --- | --- | --- | --- |
| `session_id` | 문자열 | 필수, null 불가 | 세션 식별자. 중계 ID와 다른 값이다. |
| `state` | 문자열 | 필수, null 불가 | `active`(중계 중), `closing`(종료 중, 정리가 끝나기 전). 종료 중인 세션도 활성 수에 포함한다. 알 수 없는 값은 받은 그대로 표시한다. |
| `mode` | 문자열 | 필수, null 불가 | `mode1`(Mode I) 또는 `mode2`(Mode II) |
| `viewer_transport` | 문자열 | 필수, null 불가 | Viewer 전송 경로. `tcp`, `ws`, `wss`. |
| `started_at` | `timestamp` | 필수, null 불가 | 중계 시작 시각 |
| `bytes_viewer_to_target` | `bytes` | 필수, null 불가 | Viewer → Server/Mode I 대상 방향의 누적 바이트 |
| `bytes_target_to_viewer` | `bytes` | 필수, null 불가 | Server/Mode I 대상 → Viewer 방향의 누적 바이트 |
| `viewer_address` | `address` | 필수, null 불가 | Viewer 쪽 상대 주소 |
| `target_address` | `address` | 필수, null 불가 | Server(Mode II) 또는 Mode I 대상 쪽 상대 주소 |

- 누적 바이트는 한 세션 안에서 줄거나 순환하지 않는다. 재접속 세션은 새 `session_id`와 0에서 시작한다(MGT-007).
- `session_id`가 위 형식을 만족하지 않으면 `400 invalid_request`다(잘못된 대상).
- 형식은 맞지만 지금 활성 세션이 아니면 `404 not_found`다(대상 없음). 이미 정리가 끝난 세션과 처음부터 없던 세션을 구분하지 않는다. 같은 중계 ID의 다른 세션으로 대체하지 않는다.
- 상세는 `state`가 `closing`인 세션도 돌려준다.

### 1.9 관리자 종료 `POST /api/v1/sessions/{session_id}/terminate`

요청: 관리 세션 쿠키와 `X-CSRF-Token`이 필요하다. 본문은 없다. 본문이 있으면 `Content-Type`과 관계없이 `400 invalid_request`다. 이 경로별 규칙은 1.1절의 415 규칙보다 우선한다(사용자 결정 2026-11-02(Q89)). 서버는 지정한 `session_id`의 세션에만 영향을 준다.

서버는 그 세션의 신규 데이터 수락을 멈추고 양 끝점을 종료한다. 자원 정리가 끝난 뒤에 응답한다. 응답 시간은 최대 정리 제한 시간 E까지 길어질 수 있다. 검증·수락된 종료 요청이 완료를 기다리는 동안만 관리 연결의 U를 멈춰 연결을 유지한다. 응답 쓰기부터 U를 다시 잰다([설정 계약 1.2절](config-v2.md#12-키-표)의 Q25 예외, 사용자 결정 2026-11-02(Q92)). 정리 중에는 그 세션이 `counts.active`와 `GET /api/v1/sessions`에 `closing`으로 남는다.

성공 응답 `200`의 본문:

| 필드·메시지 | 자료형 | 필수 여부·null 허용 여부 | 의미·제약 |
| --- | --- | --- | --- |
| `as_of` | `timestamp` | 필수 | 응답 시점 |
| `result` | 문자열 | 필수, null 불가 | `terminated` 또는 `already_closed`. 아래 표를 따른다. 알 수 없는 값은 결과 불명으로 보고 재조회한다. |
| `closed_session` | 객체 | 필수, null 불가 | 정리가 끝난 세션의 요약. 1.10절의 항목과 같은 필드를 가진다. 최근 종료 목록에 보관되는지와 관계없이 항상 포함한다. |

종료 결과 구분:

| 상황 | 응답 | 의미 |
| --- | --- | --- |
| 세션이 `active`이고 이 요청이 종료를 시작했다 | `200`, `result`가 `terminated`, `closed_session.close_cause`는 `admin_terminated` | 이 요청이 세션을 종료했다. 정리가 E를 넘어 강제로 끝났으면 `close_cause`는 `close_timeout`이다. 이때도 `result`는 `terminated`다. |
| 이 요청이 도착했을 때 세션이 이미 `closing`이었다(자연 종료·다른 관리자의 종료 요청·끊긴 연결 감지 등) | 그 정리가 끝날 때까지 기다린 뒤 `200`, `result`가 `already_closed`, `closed_session.close_cause`는 그 세션의 실제 종료 원인 | 종료는 이 요청 이전에 시작됐다. 이 요청은 새 종료를 일으키지 않았다. |
| 요청이 도착했을 때 활성 세션이 아니다(정리가 끝났거나, 처음부터 없었다) | 즉시 `404 not_found` | 대상 없음. 최근 종료 목록에 있는 `session_id`도 같다(MGT-023). |
| `session_id` 형식이 틀렸다 | 즉시 `400 invalid_request` | 잘못된 대상 |
| 양 끝점 정리에 실패했다 | `500 terminate_failed` | 세션이 종료됐다고 확인하지 못했다. 성공으로 표시하지 않는다. 재조회로 실제 상태를 확인한다. |

- 요청마다 `session_id`로 활성 세션을 찾는다. 같은 중계 ID를 다시 찾아 종료하지 않는다. 오래된 요청이 같은 중계 ID의 새 세션을 종료하지 못한다(MGT-010).
- 서버는 종료 사건 로그에 수행한 관리자 계정명을 기록한다([운영 스펙](../specs/operations.md)의 OPS-011, 사건 형식은 [운영 정보 계약](ops-info-v1.md)의 `admin_terminate`). 계정명은 이 응답에 넣지 않는다.

응답 유실 때 Consumer 규칙:

1. 다음은 모두 결과 불명이다. 응답 없이 연결이 끊김, `terminate_timeout_secs` 초과, `terminate_failed`가 아닌 `5xx`. 완료로 표시하지 않는다.
2. 종료 요청을 자동으로 다시 보내지 않는다.
3. 먼저 `GET /api/v1/sessions/{session_id}`로 다시 조회한다. `200`이면 세션이 아직 있다. `state`를 보여주고, 관리자가 확인한 뒤에만 새 종료 요청을 보낸다. `404`이면 세션이 더 이상 없다.
4. `404`일 때 원인이 필요하면 `GET /api/v1/closed-sessions`에서 같은 `session_id`를 찾는다. 목록에 없으면 원인을 확인할 수 없다(R 초과·`max_entries`가 0·재시작). 이때도 "이 요청이 종료했다"고 추정하지 않는다.
5. 재조회가 `401`이면 관리 세션이 무효다. 로그인 화면으로 이동한다.
6. `terminate_failed`를 받았을 때도 3번처럼 재조회한다.

### 1.10 최근 종료 목록 `GET /api/v1/closed-sessions`

최상위 필드는 `as_of`, `max_entries`, `items`다. 정리가 끝난 활성 세션만 담는다. 대기 만료 등 활성 세션이 되지 못한 연결은 담지 않는다. 메모리에만 두며 재시작하면 비어 있다.

| 필드·메시지 | 자료형 | 필수 여부·null 허용 여부 | 의미·제약 |
| --- | --- | --- | --- |
| `max_entries` | 정수 | 필수, null 불가 | 보관 최대 개수 R. 설정 `[recent_closed] max_entries`의 값. 0이면 `items`는 항상 비어 있다. |
| `items[].session_id` | 문자열 | 필수, null 불가 | 세션 식별자. 이 값으로 상세·종료를 요청하면 `404 not_found`다. |
| `items[].mode` | 문자열 | 필수, null 불가 | `mode1` 또는 `mode2` |
| `items[].viewer_transport` | 문자열 | 필수, null 불가 | `tcp`, `ws`, `wss` |
| `items[].started_at` | `timestamp` | 필수, null 불가 | 중계 시작 시각 |
| `items[].ended_at` | `timestamp` | 필수, null 불가 | 정리 완료 시각. `started_at` 이상이다. |
| `items[].bytes_viewer_to_target` | `bytes` | 필수, null 불가 | Viewer → Server/Mode I 대상 방향의 최종 누적 바이트 |
| `items[].bytes_target_to_viewer` | `bytes` | 필수, null 불가 | Server/Mode I 대상 → Viewer 방향의 최종 누적 바이트 |
| `items[].close_cause` | 문자열 | 필수, null 불가 | 종료 원인. 아래 표를 따른다. |

종료 원인 `close_cause`의 값 목록은 [관리 스펙 2.3절](../specs/management.md#23-최근-종료-목록)을 따른다. 운영 로그·WebSocket close 사유 설명과 같은 이름을 쓴다. 한 세션의 `close_cause`는 정리를 시작하게 한 첫 원인이다. 강제 정리 시점에 이미 정리를 시작한 세션은 최초 원인을 유지한다(사용자 결정 2026-11-01(Q82)). 기존 원인 없이 소유 작업이 끝나면 서버가 Running일 때는 `internal_error`, 정상 종료 중(Grace·Force)의 취소일 때는 `server_shutdown`을 쓴다(사용자 결정 2026-11-02(Q94)).

| 값 | 의미 |
| --- | --- |
| `peer_eof` | 정상 EOF. 양쪽이 정상적으로 끝났다. |
| `connection_error` | 연결 오류. 중계 I/O 오류로 세션이 끝났다. |
| `admin_terminated` | 관리자 종료. 관리자 종료 요청으로 끝났다. |
| `dead_connection` | 끊긴 연결 감지. 응답하지 않는 상대를 TCP keepalive나 WebSocket ping으로 판정해 종료했다. |
| `close_timeout` | 정리 제한 시간 E 초과. E 안에 정리가 끝나지 않아 강제로 정리했다. |
| `server_shutdown` | 정상 종료 유예 G가 지나거나 유예 중 두 번째 종료 요청을 받아 강제로 정리했다. 정상 종료 중(Grace·Force)에 세션 소유 작업이 취소된 경우도 포함한다. 기존 종료 원인이 없는 세션에만 기록한다. Running 중 소유 작업이 버그로 끝난 경우는 포함하지 않는다(사용자 결정 2026-11-01(Q82), 2026-11-02(Q94)). |
| `internal_error` | 내부 오류. 서버가 Running일 때 세션 소유 작업이 panic 등 버그로 끝났고 기존 종료 원인이 없었다. 기존 원인은 이 값으로 바꾸지 않는다(사용자 결정 2026-11-02(Q94)). |

- Consumer는 목록에 없는 값을 받으면 받은 그대로 표시하고 오류로 처리하지 않는다.
- 목록은 보관 개수 R을 넘으면 가장 오래된 항목부터 제거한다(MGT-024).
- 이 목록은 목록 정보만 제공한다. 상세 이력을 제공하지 않는다.

### 1.11 정적 대시보드 경로와 비인증 접근 범위

대시보드는 빌드 단계 없는 정적 HTML·JS이며 실행파일에 들어 있다. 관리 리스너가 제공한다. 인증 없이 접근할 수 있는 것은 로그인 화면(SCR-003)에 필요한 정적 파일뿐이다. 운영 데이터를 담은 응답은 없다.

| 메서드·경로 | 내용 | 관리 세션 | 없을 때 |
| --- | --- | --- | --- |
| `GET /login/` | 로그인 화면 HTML(SCR-003) | 불필요 | 해당 없음 |
| `GET /login/<파일>` | 로그인 화면이 쓰는 정적 파일 | 불필요 | 해당 없음 |
| `GET /` | 운영 화면 HTML(SCR-001·SCR-002) | 필요 | `303 See Other`, `Location: /login/` |
| `GET /app/<파일>` | 운영 화면이 쓰는 정적 파일 | 필요 | `303 See Other`, `Location: /login/` |
| `/api/v1/` 아래 | 1.2절의 API | 경로별 | `401 unauthenticated` (로그인 제외) |
| 그 밖의 경로 | 없음 | 있으면 `404`, 없으면 `303`으로 `/login/` | 존재 여부를 노출하지 않는다 |

- 정적 경로는 `GET`(과 `HEAD`)만 받는다. 그 밖의 메서드는 `405`다.
- 보호된 정적 파일의 `303`은 JSON 오류 본문이 없다. API가 아니기 때문이다.
- 로그인 화면 파일에는 운영 데이터·계정 정보·설정 값을 넣지 않는다.
- 로그인 성공 뒤 화면은 `/`로 이동한다.
- 화면 URL: SCR-003은 `/login/`, SCR-001은 `/`, SCR-002는 `/#/sessions/{session_id}`다. 세션 식별자는 URL의 조각(`#` 뒤)에 두므로 서버로 전송되지 않고 서버 접근 로그에 남지 않는다. 서버는 이 조각을 해석하지 않는다.
- 화면은 URL의 `session_id`가 1.8절 형식을 만족하는지 먼저 확인한다. 형식이 틀리면 요청을 보내지 않고 "잘못된 세션 식별자"를 표시한다. 형식이 맞으면 상세를 조회하고, `404`이면 "대상 없음"을 표시한다. 두 상황을 서로 다른 안내로 구분한다.
- 보호된 정적 응답과 API 응답에 `Cache-Control: no-store`를 붙인다. 서버를 갱신한 뒤 브라우저가 이전 버전의 화면 코드를 다시 쓰지 않게 하기 위해서다.
- 정적 응답에 `X-Content-Type-Options: nosniff`와 `Referrer-Policy: no-referrer`를 권장한다. `Content-Security-Policy: default-src 'self'; frame-ancestors 'none'`은 필수다(사용자 결정 2026-10-22(Q37)). 대시보드는 인라인 스크립트와 인라인 스타일(`<style>`, `style` 속성)을 쓰지 않는다.

### 1.12 폴링 주기와 조회 제한 시간의 전달

기본값과 유효 범위는 [설정 계약 1.2절](config-v2.md#12-키-표)이 정한다. 이 표는 전달 방법만 정한다.

| 값 | 설정 키(설정 계약) | 전달 방법 |
| --- | --- | --- |
| P | `[dashboard] poll_secs` | 세션 정보의 `dashboard.poll_secs` |
| 폴링 조회 제한 시간 | `[dashboard] query_timeout_secs` | 세션 정보의 `dashboard.query_timeout_secs` |
| 종료 응답 대기 한도 | 설정 키 없음. `[timeouts] close_secs`(E)에서 계산 | 세션 정보의 `dashboard.terminate_timeout_secs`. 값은 E + 5초다. |

- 폴링 조회는 `GET` 다섯 경로(`summary`·`waiting`·`sessions`·`sessions/{session_id}`·`closed-sessions`)에 붙인 자동 요청이다. 응답이 `query_timeout_secs` 안에 오지 않으면 Consumer는 요청을 중단하고 조회 실패로 처리한다. 실패를 빈 목록으로 표시하지 않는다.
- 같은 조회가 진행 중이면 다음 주기 요청을 겹쳐 보내지 않는다([화면 4.1절](../specs/dashboard.md#41-폴링)). 조회 제한 시간이 P보다 길어도 된다.
- 종료 요청에는 폴링 조회 제한 시간을 쓰지 않는다. 정리가 최대 E까지 걸릴 수 있기 때문이다. `terminate_timeout_secs`를 쓴다.
- 서버는 일반 조회를 제한 시간 안에 응답하도록 만든다. 서버 내부 처리 제한은 이 계약이 정하지 않는다.

## 2. 오류

### 2.1 오류 본문과 상태 코드

모든 `/api/v1/` 오류 응답 본문은 다음 형식이다. 정적 파일의 `303`·`404`는 예외다(1.11절).

```json
{"error": {"code": "<snake_case 코드>", "message": "<사람이 읽는 문장>"}}
```

- Consumer는 `code`와 HTTP 상태 코드로 판정한다. `message`로 판정하지 않는다. `message`는 화면에 그대로 보여줄 수 있는 한국어 한 문장이며 비밀과 요청 입력값을 담지 않는다.
- 새 오류 코드는 호환 변경이다. Consumer는 모르는 코드를 HTTP 상태 코드 범주로 처리한다(3절).
- 모든 `401` 응답(`unauthenticated`·`login_failed`)에 헤더 `WWW-Authenticate: Session realm="vnc-relay"`를 붙인다. 이 값은 사용자가 승인했다(2026-10-16). RFC 9110이 `401` 응답에 이 헤더를 요구하기 때문이다. `Session`은 관리 세션 쿠키 인증을 뜻하는 이 계약의 고유 스킴 이름이다. 표준 인증 방식(`Basic`·`Digest` 등)이 아니다. 브라우저가 이 헤더 때문에 기본 인증 창을 띄우지 않는지는 구현 때 확인한다(6절).

| 오류 코드 | 의미·형식 | 관련 스펙 시나리오 |
| --- | --- | --- |
| `unauthenticated` | `401`. 유효한 관리 세션이 없다. 쿠키 없음·잘못된 쿠키·유휴 만료·절대 만료·로그아웃됨·재시작 전 발급을 구분하지 않는다. 쿠키가 있었으면 지우는 `Set-Cookie`를 보낸다. 상태를 바꾸지 않는다. 경로·메서드가 틀려도 인증이 먼저다. | [관리: MGT-008·MGT-019·MGT-020·MGT-021·MGT-025·MGT-027](../specs/management.md#7-수용-테스트), [화면: DASH-008·DASH-014](../specs/dashboard.md#7-수용-테스트), [관리 4절](../specs/management.md#4-오류와-복구) "관리 세션 없음"·"관리 세션 유휴·절대 수명 만료" |
| `csrf_invalid` | `403`. 관리 세션은 유효하지만 `X-CSRF-Token`이 없거나 틀리다. 관리 세션을 유지하고 대상의 상태를 바꾸지 않는다. | [관리: MGT-014](../specs/management.md#7-수용-테스트), [관리 4절](../specs/management.md#4-오류와-복구) "관리 세션·CSRF 검증 실패" |
| `login_failed` | `401`. 로그인 실패. 계정 없음·비밀번호 틀림·로그인 제한 중이 모두 같은 상태·코드·본문이다. 세션을 발급하지 않는다. 계정 존재 여부를 알 수 있는 정보를 담지 않는다. | [관리: MGT-018·MGT-022](../specs/management.md#7-수용-테스트), [화면: DASH-018](../specs/dashboard.md#7-수용-테스트) |
| `not_found` | `404`. 대상 없음. 형식은 맞지만 지금 활성 세션이 아닌 `session_id`(정리 완료·최근 종료 목록에 있음·처음부터 없음), 또는 알 수 없는 `/api/v1/` 경로. 다른 세션으로 대체하지 않는다. | [관리: MGT-010·MGT-011·MGT-023](../specs/management.md#7-수용-테스트), [화면: DASH-007·DASH-011](../specs/dashboard.md#7-수용-테스트), [관리 4절](../specs/management.md#4-오류와-복구) "상세 대상 소멸"·"종료 중 대상 소멸" |
| `invalid_request` | `400`(본문·필드·`session_id` 형식·`X-Relay-Poll` 값 오류), `405`(경로는 있으나 메서드가 다름, `Allow` 헤더 포함), `415`(본문이 있고 `Content-Type`이 `application/json`이 아님. 단, 1.9절의 terminate 본문은 항상 400). 잘못된 대상을 대상 없음(`not_found`)과 구분한다. 로그인 실패 횟수에 세지 않는다. | [화면 5절](../specs/dashboard.md#5-입력-검증과-종료-오류) "잘못된 대상과 대상 소멸의 구분", [화면: DASH-018](../specs/dashboard.md#7-수용-테스트), [관리: MGT-025·MGT-013](../specs/management.md#7-수용-테스트) |
| `terminate_failed` | `500`. 종료 요청은 유효했지만 양 끝점 정리에 실패해 종료를 확인하지 못했다. 완료로 표시하지 않는다. 재조회로 실제 상태를 확인한다. | [관리: MGT-012](../specs/management.md#7-수용-테스트), [화면: DASH-012](../specs/dashboard.md#7-수용-테스트), [관리 4절](../specs/management.md#4-오류와-복구) "종료 처리 실패" |
| `internal` | `500`. 조회 처리 중 서버 내부 오류. 빈 성공 목록과 구분한다. 중계 영향은 없다. | [관리: MGT-004](../specs/management.md#7-수용-테스트), [화면: DASH-003](../specs/dashboard.md#7-수용-테스트), [관리 4절](../specs/management.md#4-오류와-복구) "조회 처리·통신 실패" |

경로별 상태 코드:

| 경로 | 성공 | 가능한 오류 |
| --- | --- | --- |
| `POST /api/v1/session` | `200` | `400`·`415`(`invalid_request`), `401`(`login_failed`), `405`, `500`(`internal`) |
| `GET /api/v1/session` | `200` | `400`(`X-Relay-Poll` 값 오류), `401`, `405`, `415`, `500` |
| `DELETE /api/v1/session` | `204` | `400`, `401`, `403`, `405`, `415`, `500` |
| `GET /api/v1/summary`·`waiting`·`sessions`·`closed-sessions` | `200` | `400`, `401`, `405`, `415`, `500`(`internal`) |
| `GET /api/v1/sessions/{session_id}` | `200` | `400`, `401`, `404`, `405`, `415`, `500` |
| `POST /api/v1/sessions/{session_id}/terminate` | `200` | `400`, `401`, `403`, `404`, `405`, `500`(`terminate_failed`·`internal`) |

### 2.2 HTTP 응답이 없는 실패

다음은 오류 코드가 없다. Consumer는 모두 조회 실패(종료 요청이면 결과 불명)로 처리한다.

| 조건 | 설명 | 관련 스펙 시나리오 |
| --- | --- | --- |
| 연결 거절 | Lm 상한 도달로 새 관리·상태 연결을 거절했다. 중계 접속은 영향이 없다. | [운영: OPS-033](../specs/operations.md#7-수용-테스트), [운영 5절](../specs/operations.md#5-오류와-복구) "Lm 상한 도달" |
| TLS 협상 실패 | TLS 1.2 미만 클라이언트의 협상을 거절한다. 평문으로 전환하지 않는다. | [운영: OPS-036](../specs/operations.md#7-수용-테스트) |
| 응답 시간 초과·연결 끊김 | `query_timeout_secs`(폴링 조회) 또는 `terminate_timeout_secs`(종료) 안에 응답이 없다. | [화면: DASH-005·DASH-013](../specs/dashboard.md#7-수용-테스트), [관리 4절](../specs/management.md#4-오류와-복구) "종료 요청 뒤 응답 유실" |

### 2.3 Consumer의 판정 규칙

| 받은 것 | Consumer 동작 |
| --- | --- |
| `200`과 빈 `items` | 성공한 빈 결과. "대기 없음"·"활성 없음"·"최근 종료 없음"을 표시한다. |
| `401 unauthenticated` | 보호된 이전 데이터와 갱신을 제거하고 `/login/`으로 이동한다. |
| `401 login_failed` | 공통 실패 안내를 표시하고 비밀번호를 지운다. 계정명은 남긴다. |
| `403 csrf_invalid` | 관리 세션은 유효하다. `GET /api/v1/session`으로 토큰을 다시 받아 사용자가 새로 확인한 뒤 요청한다. 자동 재전송하지 않는다. 로그아웃 요청의 `403`은 이 행이 아니라 아래 로그아웃 요청 행을 따른다(사용자 결정 2026-10-23(Q54)). |
| `404 not_found` | "대상 없음"을 표시하고 종료를 비활성으로 한다. 목록 이동을 제공한다. |
| `400 invalid_request` | 잘못된 대상·요청으로 표시한다. 대상 없음과 구분한다. |
| 로그인 요청(`POST /api/v1/session`)의 `500`, 응답 없음, 시간 초과 | 로그인 요청 실패. 인증 실패(`401 login_failed`)의 공통 안내와 다른 안내를 표시한다. 관리 세션이 없는 상태로 SCR-003에 머문다. 문구는 [화면 스펙](../specs/dashboard.md#4-화면-상태와-갱신) SCR-003 "로그인 요청 실패" 행이 정한다(사용자 결정 2026-10-22(Q48)). |
| 로그아웃 요청(`DELETE /api/v1/session`)의 `403 csrf_invalid`, `500`, 응답 없음, 시간 초과 | 로그아웃을 완료로 표시하지 않는다. 보호된 데이터를 숨기고 SCR-003에 로그아웃 결과 확인 필요 안내를 보인다. 상태와 문구는 [화면 스펙](../specs/dashboard.md#4-화면-상태와-갱신) SCR-003 "로그아웃 결과 확인 필요" 행이 정한다(사용자 결정 2026-10-22(Q47)). `403`도 같이 처리한다(사용자 결정 2026-10-23(Q54)). 이때 서버는 관리 세션을 유지하므로(1.4.3절), 그 관리 세션은 유휴 만료 Si나 절대 만료 Sa까지 남는다. |
| 그 밖의 요청의 `500`, 응답 없음, 시간 초과 | 조회 실패. 이전 데이터는 마지막 성공 시각과 함께 오래된 값으로 표시한다. 빈 결과로 표시하지 않는다. |
| 모르는 `code` | HTTP 상태 코드 범주로 처리한다(`401`은 관리 세션 무효, `4xx`는 요청 오류, `5xx`는 조회 실패). |

## 3. 호환성 규칙

- 데이터를 사용하는 쪽(Consumer)은 응답의 알 수 없는 필드를 무시해야 한다. 허용 값 목록에 없는 문자열 값(`server.state`·`state`·`mode`·`viewer_transport`·`close_cause`·`role`·`*_unavailable_reason`)은 받은 그대로 표시하고 오류로 처리하지 않아야 한다. 모르는 `result` 값은 결과 불명으로 보고 재조회해야 한다. 모르는 오류 코드는 2.3절대로 HTTP 상태 코드 범주로 처리해야 한다.
- 데이터를 만드는 쪽(Producer)은 요청의 알 수 없는 필드를 무시해야 한다. 헤더·쿠키·경로는 이 계약이 정한 것만 해석한다.
- 호환 변경(v1.x): 응답에 필드를 추가한다. 허용 값 목록에 값을 추가한다. 새 경로를 추가한다. 요청에 선택 필드를 추가한다. 새 오류 코드를 기존 상태 코드 범주 안에 추가한다. 확인 불가 사유 값을 추가한다. 성능·내부 동작을 바꾼다.
- 호환성을 깨는 변경(Breaking change): 필드나 경로를 삭제·이름 변경한다. 자료형·단위·시각 형식·`null` 의미를 바꾼다. 기존 조건의 HTTP 상태 코드나 오류 코드를 바꾼다. 요청 필드를 필수로 바꾼다. 쿠키 이름이나 헤더 이름·의미를 바꾼다. 쿠키 보안 속성을 약하게 바꾼다. 인증·CSRF 요구를 약하게 바꾼다. 위 변경은 새 major 버전(`/api/v2/`)으로 낸다.
- 마이그레이션: 새 major 버전은 새 접두사로 제공하고, 같은 릴리스에서 대시보드를 새 버전으로 옮긴다. 대시보드는 같은 실행파일에 들어 있으므로 서버와 대시보드의 버전이 어긋나지 않는다. 외부 클라이언트가 있으면 이전 major를 한동안 함께 제공한다. 기간은 새 major 채택 때 정한다.
- 롤백: 이전 릴리스 실행파일로 되돌리면 이전 major와 대시보드가 함께 돌아온다. 관리 세션은 서버 메모리에만 있으므로 되돌리는 시점에 모두 무효가 되고, 관리자는 다시 로그인한다. 최근 종료 목록도 비워진다.
- 정적 응답이 `no-store`이므로 서버 갱신 뒤 브라우저가 이전 버전의 화면 코드를 재사용하지 않는다.

## 4. 버전과 지원 단계

| 상태 | 조건 | 지원 종료·대체 |
| --- | --- | --- |
| Draft | 채택 전 상태. 이 계약은 2026-10-16에 채택해 Draft를 지났다. | 해당 없음 |
| Active | 현재. 사용자 채택(2026-10-16). 구현과 5절의 계약 테스트가 Producer·Consumer 양쪽에서 통과한 기록은 5절 표가 관리하며, 통과하기 전에는 지원 완료로 표시하지 않는다. | 새 major가 Active가 될 때까지 유지한다. |
| Deprecated | 새 major(`/api/v2/`)가 Active가 됐다. | 이전 major를 함께 제공하는 기간과 종료일은 새 major 채택 때 정한다. |
| Retired | Deprecated 기간이 끝났다. | 새 major가 대체한다. 이전 경로는 `404`를 반환한다. |

## 5. 예제와 계약 테스트

예제의 값은 문서용 값이다. 실제 비밀값·비밀번호·해시·쿠키 값이 아니다. IP는 문서용 대역(192.0.2.0/24, 198.51.100.0/24, 2001:db8::/32)을 쓴다.

로그인 요청과 응답:

```http
POST /api/v1/session HTTP/1.1
Content-Type: application/json

{"account": "admin-example", "password": "<비밀번호>"}
```

```http
HTTP/1.1 200 OK
Content-Type: application/json; charset=utf-8
Cache-Control: no-store
Set-Cookie: relay_admin_session=<불투명 식별자>; Path=/; HttpOnly; SameSite=Strict; Secure
```

```json
{
  "account": "admin-example",
  "csrf_token": "<csrf-token>",
  "idle_expires_at": "2026-10-16T09:30:00.000Z",
  "absolute_expires_at": "2026-10-16T21:00:00.000Z",
  "dashboard": {"poll_secs": 2, "query_timeout_secs": 5, "terminate_timeout_secs": 15}
}
```

자동 폴링 요청과 서버 요약(L 사용량을 읽지 못한 경우):

```http
GET /api/v1/summary HTTP/1.1
X-Relay-Poll: 1
```

```json
{
  "as_of": "2026-10-16T09:00:01.250Z",
  "server": {"state": "ready", "uptime_ms": 86400000},
  "counts": {
    "waiting": 2,
    "active": 12,
    "handshaking": 1,
    "handshaking_unavailable_reason": null,
    "target_connecting": 0,
    "target_connecting_unavailable_reason": null
  },
  "usage": {
    "relay_connections": {"current": null, "current_unavailable_reason": "read_failed", "limit": 1000},
    "admin_connections": {"current": 2, "current_unavailable_reason": null, "limit": 64}
  }
}
```

활성 세션 상세:

```json
{
  "as_of": "2026-10-16T09:00:02.000Z",
  "session": {
    "session_id": "ses_3k9d2m7q4x8v1b6n5c0z2h4w7y",
    "state": "active",
    "mode": "mode2",
    "viewer_transport": "wss",
    "started_at": "2026-10-16T08:40:10.120Z",
    "bytes_viewer_to_target": 1048576,
    "bytes_target_to_viewer": 73400320,
    "viewer_address": {"ip": "198.51.100.7", "port": 51234},
    "target_address": {"ip": "192.0.2.20", "port": 40022}
  }
}
```

관리자 종료 요청과 응답:

```http
POST /api/v1/sessions/ses_3k9d2m7q4x8v1b6n5c0z2h4w7y/terminate HTTP/1.1
X-CSRF-Token: <csrf-token>
```

```json
{
  "as_of": "2026-10-16T09:00:05.400Z",
  "result": "terminated",
  "closed_session": {
    "session_id": "ses_3k9d2m7q4x8v1b6n5c0z2h4w7y",
    "mode": "mode2",
    "viewer_transport": "wss",
    "started_at": "2026-10-16T08:40:10.120Z",
    "ended_at": "2026-10-16T09:00:05.380Z",
    "bytes_viewer_to_target": 1049600,
    "bytes_target_to_viewer": 73500000,
    "close_cause": "admin_terminated"
  }
}
```

오류 응답(대상 없음):

```json
{"error": {"code": "not_found", "message": "지정한 세션을 찾을 수 없습니다."}}
```

계약 테스트 위치는 `crates/adapters/tests/management.rs`, `tests/management_session.rs`, `tests/management_query.rs`, `tests/management_terminate.rs`, `tests/dashboard_assets.rs`, `tests/serve.rs`, `tests/dashboard_browser.rs`와 각 행에 적은 단위 시험이다. 위치가 비어 있는 행은 해당 작업에서 채운다. 아래 표는 Producer와 Consumer가 만들 계약 테스트가 확인할 규칙을 기존 스펙 시나리오에 연결한다. 이 표는 테스트가 있다는 증거가 아니다. 결과와 증거는 [검증 기록](../AGENTS.md#검증-기록)을 따른다. 스펙과 같은 테스트는 해당 시나리오의 기록을 연결한다.

| 주체·테스트 위치 | 확인할 계약 규칙 | 결과 | 실행 기록 |
| --- | --- | --- | --- |
| Producer / 통합(생성) | 1.2절의 경로·메서드·상태 코드로 서버 요약·대기·활성 목록·상세를 조회하고 집계와 목록이 일치한다. [MGT-001](../specs/management.md#7-수용-테스트)·[MGT-002](../specs/management.md#7-수용-테스트)·[MGT-003](../specs/management.md#7-수용-테스트)·[MGT-016](../specs/management.md#7-수용-테스트) | 통과 | MGT-001~003은 아래 작업 4.4 행의 `tests/management_query.rs` 비부분 시험이다. MGT-016은 `crates/adapters/tests/management.rs`의 `completion_mgt016_counts_lists_and_as_of_follow_one_capture_while_usage_is_sampled`(비부분, 커밋 `fb8a362`)다. 집계·목록·`as_of`는 한 상태 복사본을 따르고, L·Lm 사용량은 따로 읽는다(사용자 결정 2026-11-04(Q100)). |
| Producer / 단위·통합(생성) | 시각은 UTC ISO 8601(`Z`, 밀리초 3자리), 전송량은 바이트 정수, 방향 고정, 새 세션 카운터 분리, 경과 시간이 음수가 아니다. [MGT-003](../specs/management.md#7-수용-테스트)·[MGT-005](../specs/management.md#7-수용-테스트)·[MGT-006](../specs/management.md#7-수용-테스트)·[MGT-007](../specs/management.md#7-수용-테스트) | 통과 | MGT-003·006은 아래 작업 4.4 행의 `tests/management_query.rs` 비부분 시험이다. MGT-005는 `management.rs`의 `completion_mgt005_partial_success_and_write_failure_count_only_delivered_bytes`, MGT-007은 `completion_mgt007_reconnect_fresh_runtime_and_wall_steps_keep_counters_separate`다(모두 비부분, 커밋 `fb8a362`). MGT-005는 공개 `RelayStream` 포트로 부분 쓰기와 쓰기 실패를 넣는다(사용자 결정 2026-11-04(Q95)). MGT-007의 재시작은 같은 시험 안의 새 런타임·리스너로 대신한다(사용자 결정 2026-11-04(Q101)). |
| Producer / 통합(생성) | 빈 성공(`200`·`items: []`)과 조회 오류(실제 조회 경로의 `400 invalid_request`·`404 not_found`)를 구분한다. 조회 처리에는 `500` 경로가 없다(사용자 결정 2026-11-04(Q95)). [MGT-004](../specs/management.md#7-수용-테스트) | 통과 | `tests/management_query.rs`의 `empty_success_vs_failure_distinction`(비부분, 커밋 `bb44c4f`). [관리 조회 설계 6절 9행](../architectures/management-query.md#6-오류-처리) |
| Producer / 보안 통합(생성) | 로그인 성공 뒤 쿠키 속성(HTTPS는 Secure·HttpOnly·SameSite=Strict, 평문 루프백은 Secure 없음), 틀린 로그인은 `login_failed`이고 세션을 발급하지 않는다. [MGT-017](../specs/management.md#7-수용-테스트)·[MGT-018](../specs/management.md#7-수용-테스트) | 통과 | MGT-017 통과(비부분): `management.rs`의 `https_and_plain_loopback_logins_set_transport_cookies_and_allow_queries`(커밋 `c0d3b59`)가 HTTPS와 평문 루프백 로그인의 쿠키 속성과 로그인 뒤 조회 경로의 성공 응답을 확인한다. 같은 범위의 부분 시험 `stale_session_cookie_is_cleared_without_secure_over_plain_http`(4.2)와 `tests/management_session.rs`의 `plain_loopback_login_opens_the_session_and_logs_success`(로그인 줄만 확인)는 남아 있다. MGT-018 통과(비부분): `management.rs`의 `wrong_password_and_unknown_account_get_the_same_refusal`(세션 조회와 4.4 조회 경로 4종이 `401 unauthenticated`), 단위 `login.rs`의 `wrong_password_and_unknown_account_give_the_same_refusal`. |
| Producer / 보안 통합(생성) | 로그인 제한 중 올바른 비밀번호도 `login_failed`이고, 계정이 있을 때와 없을 때 응답이 같다. [MGT-022](../specs/management.md#7-수용-테스트) | 통과 | `management.rs`의 `limited_logins_refuse_alike_and_the_right_password_works_after_the_locks`(4.3, 비부분, `FakeClock`·`CountingVerifier`); 단위 `login.rs`의 `locked_account_refuses_the_correct_password_until_the_lock_ends`, `same_source_is_limited_for_other_accounts_too`, `every_attempt_runs_one_password_check_including_unknown_and_limited`. 응답 시간 동등성은 검증 횟수를 세는 구조 시험으로 판정한다(사용자 결정 2026-10-22(Q31)). 제한 줄 형식은 `ops_log.rs`의 `admin_login_lines_carry_only_their_own_fields`가 확인한다. |
| Producer / 보안 통합(생성) | 쿠키 없음·만료·로그아웃 뒤 쿠키는 모두 `401 unauthenticated`이고 중계는 유지된다. [MGT-008](../specs/management.md#7-수용-테스트)·[MGT-019](../specs/management.md#7-수용-테스트)·[MGT-020](../specs/management.md#7-수용-테스트)·[MGT-021](../specs/management.md#7-수용-테스트) | 통과 | 쿠키 없는 요청의 401은 `management.rs`의 `plaintext_api_requests_are_refused_without_a_session`(어댑터 수준, OPS-034 부분)과 `tests/serve.rs`의 `refuses_public_plaintext_and_answers_login_less_queries_with_401_on_loopback`(실행 파일 수준, OPS-034 통과, 비부분)가 평문 루프백에서 확인했다. MGT-019 통과(비부분): `management.rs`의 `expired_admin_cookie_cannot_query_or_terminate_and_the_relay_survives`(실제 `RelayRuntime`, Si 300 ms). MGT-020 통과(비부분): `tests/management_session.rs`의 `logout_refuses_the_old_cookie_but_keeps_other_admin_and_relay_sessions`. 남은 부분 시험: `management.rs`의 `logout_ends_only_its_own_session`; 단위 `session.rs`의 `missing_malformed_unknown_or_expired_cookie_is_unauthenticated`, `logout_ends_only_its_own_session`. MGT-008은 `tests/management_terminate.rs`의 `completion_mgt008_unauthorized_termination_preserves_both_sessions`(비부분)로 커밋 `59fc535`에서 두 OS 통과했다. MGT-021은 작업 4.4 행에서 통과다. |
| Producer / 보안 통합(생성) | 재시작 뒤 이전 쿠키는 `401 unauthenticated`이고 새 로그인은 된다. [MGT-027](../specs/management.md#7-수용-테스트) | 통과 | Unix는 `tests/management_session.rs`의 `restart_ends_the_admin_session_and_login_works_again`(SIGTERM), Windows는 `tests/windows_console.rs`의 `graceful_restart_ends_the_admin_session_and_login_works_again`(ConPTY Ctrl+C)다. 모두 정상 종료 뒤 같은 설정으로 새 프로세스를 시작해 이전 쿠키 거절(`401`)과 새 로그인을 확인한다. 커밋 `de693ad` 게이트에서 두 OS 모두 통과했다. |
| Producer / 보안 통합(생성) | CSRF 토큰 없는 종료·로그아웃은 `403 csrf_invalid`이고 상태가 그대로다. [MGT-014](../specs/management.md#7-수용-테스트) | 통과 | `tests/management_terminate.rs`의 `completion_q89_auth_csrf_validation_order_and_chunked_body_are_state_preserving`(MGT-014 비부분)이 종료 요청의 CSRF 거절과 상태 유지를 확인했다. 로그아웃은 `management.rs`의 `logout_without_the_csrf_token_is_refused_and_keeps_the_session`, 단위 `session.rs`의 `csrf_missing_or_wrong_is_refused_and_keeps_the_session`이 확인한다. 커밋 `59fc535`에서 두 OS 통과. |
| Producer / 보안 통합(생성) | `X-Relay-Poll: 1` 요청은 유휴 만료를 연장하지 않고, 헤더 없는 사용자 행동 요청은 연장하며, Sa는 활동과 관계없이 무효로 만든다. [MGT-025](../specs/management.md#7-수용-테스트) | 통과 | 통과(비부분): `management.rs`의 `idle_polling_and_absolute_activity_expiry_leave_existing_relays_alive`(커밋 `c0d3b59`, `34cefe5`). Si 400 ms·Sa 1.2 s로 세션 정보와 조회 경로 4종(`summary`·`waiting`·`sessions`·`closed-sessions`)을 번갈아 부른다. 자동 폴링은 유휴 기한을 늘리지 않고, 사용자 행동은 늘리되 Sa를 넘기지 않으며, 만료 뒤에도 기존 중계는 유지된다. 단위 `session.rs`의 `idle_deadline_ends_the_session_without_user_actions`, `user_actions_extend_idle_but_not_past_the_absolute_deadline`도 남아 있다. |
| Producer / 통합(생성) | 종료 유예 중 로그인·`summary` 조회·세션 종료 요청이 모두 처리되고 `server.state`가 `shutdown_grace`다. [OPS-043](../specs/operations.md#7-수용-테스트) | 통과 | `tests/unix_shutdown.rs`와 `tests/windows_console.rs`의 `management_works_during_grace`. 공통 본문은 `tests/common/shutdown.rs`다. 종료 유예 중 관리 로그인, `summary` 조회(`state="shutdown_grace"`), 활성 세션 종료 요청 및 유예 만료 후 강제 정리가 두 OS에서 모두 통과했다(커밋 `de693ad`). |
| Producer / 통합(생성) | 종료 결과 구분(`terminated`·`already_closed`·`404 not_found`), 지정한 세션만 종료, 이전 `session_id`는 새 세션에 영향이 없다. [MGT-009](../specs/management.md#7-수용-테스트)·[MGT-010](../specs/management.md#7-수용-테스트)·[MGT-011](../specs/management.md#7-수용-테스트) | 통과 | `tests/management_terminate.rs`의 `completion_mgt009_selective_tcp_ws_wss_mode1_mode2_cleanup_and_final_summary`, `completion_mgt010_stale_session_id_cannot_terminate_reused_mode2_id`, `completion_mgt011_natural_close_and_concurrent_join_keep_real_cause_and_absolute_e`(모두 비부분). 커밋 `59fc535`에서 두 OS 통과. |
| Producer / 단위·통합(생성) | 정리 실패의 `500 terminate_failed`, 응답 유실 뒤 정리·감사 유지. [MGT-012](../specs/management.md#7-수용-테스트) | 통과 | `crates/adapters/tests/management.rs`의 `completion_mgt012_failed_control_and_lost_response_never_invent_completion`(비부분, 커밋 `bb44c4f`). 실패하는 가짜 중계 제어 포트를 실제 관리 HTTP·인증·직렬화·`RelayRuntime`에 연결해 `500 terminate_failed`(`no-store`, `closed_session`·`result` 없음)와 `admin_terminate` `failed`(level `error`)를 확인한다. 같은 함수가 실제 런타임으로 응답을 읽지 않고 끊어도 정리·감사가 끝나는지 확인한다(사용자 결정 2026-11-04(Q95)). 실행파일 수준의 응답 유실은 `tests/management_terminate.rs`의 `completion_mgt012_lost_http_response_does_not_cancel_cleanup_or_initiator_audit`(부분)이, 단위 수준은 `terminate_api.rs`·`terminate.rs`·`relay.rs`의 부분 시험이 확인한다. |
| Producer / 단위·통합(생성) | 최근 종료 목록의 필드·종료 원인 일곱 값·`max_entries` 초과 시 제거·재시작 뒤 빈 목록, 종료된 `session_id`의 상세·종료는 `404`다. [MGT-023](../specs/management.md#7-수용-테스트)·[MGT-024](../specs/management.md#7-수용-테스트) | 통과 | MGT-023은 `tests/management_terminate.rs`의 `completion_mgt023_eof_connection_error_and_admin_history_have_final_fields_and_old_ids_404`(비부분)가 정상 EOF·연결 오류·관리자 종료의 목록 항목과 이전 ID 404를 확인했다. MGT-024는 작업 4.4 행과 같다. 일곱 값의 문자열과 목록 보관은 단위 `session.rs`의 `exposes_exact_contract_values`, `recent_closed.rs`의 `retains_final_causes_metadata_times_and_directional_counters`가 확인한다. `close_timeout`은 MGT-011 시험이 종료 응답의 `closed_session`과 운영 로그에서 확인한다. `server_shutdown`은 `crates/adapters/tests/management.rs`의 `completion_mgt023_shutdown_history_is_queryable_and_preserves_prior_eof_cause`(부분, 커밋 `fb8a362`)가 실제 Grace → Force 뒤 최근 종료 목록 API(`GET /api/v1/closed-sessions`)로 확인한다. `close_timeout`·`internal_error`·`dead_connection`을 그 API로 조회하는 시스템 시험은 없다. 커밋 `59fc535`에서 두 OS 통과. |
| Producer / 통합(생성) | 서버 요약의 초기 교환 중·대상 연결 중 연결 수와 L·Lm 사용량·상한이 실제와 일치하고, 읽지 못한 값은 `null`과 사유로 준다. [MGT-026](../specs/management.md#7-수용-테스트) | 통과 | `tests/management_query.rs`의 `counts_and_usage_match_at_limit_with_pending_connections_linux`(Linux)와 `counts_and_usage_match_at_limit_with_pending_connections_windows`(Windows, 커밋 `0196cab`). 둘 다 비부분이며 L이 가득 찬 상태의 집계와 사용량을 한 응답에서 확인한다. 공통 부분 시험 `counts_and_usage_match_at_limit_portable`도 남아 있다. |
| Producer / 보안 통합(생성) | 오류 본문과 응답에 비밀·원시 스트림이 없고, 상대 주소는 실제 소켓 주소다. [MGT-013](../specs/management.md#7-수용-테스트)·[MGT-015](../specs/management.md#7-수용-테스트) | 통과 | MGT-015는 작업 4.4 행에서 통과다. MGT-013은 `crates/adapters/tests/management.rs`의 `completion_mgt013_query_auth_and_failed_termination_responses_keep_secrets_private`(비부분, 커밋 `bb44c4f`)가 조회 경로 5종, 401·403·400·404·405 오류, 가짜 중계 제어 포트로 만든 `500 terminate_failed`의 본문과 헤더를 검사한다(사용자 결정 2026-11-04(Q95)). `tests/management_query.rs`의 `no_secrets_or_raw_payload_in_query_or_error_bodies`와 `tests/management_terminate.rs`의 `completion_q89_auth_csrf_validation_order_and_chunked_body_are_state_preserving`은 부분 시험으로 남는다. |
| Producer / 정적 제공 통합 | 1.11절 정적 경로의 인증·메서드·헤더, 실행파일 제공 표와 원본 파일 일치 | 통과 | 커밋 `9cdebc6`의 Windows·WSL 전체 게이트에서 `management.rs`의 정적 경로 시험과 `tests/dashboard_assets.rs` 2개가 통과했다. 실제 무인증 상세 직접 진입은 `tests/dashboard_live.rs`의 `dashboard_014_unauthenticated_direct_detail_exposes_no_data`(DASH-014 비부분)가 확인한다. 과거 어댑터의 DASH-014 부분 표식은 그대로 둔다. |
| Consumer / 실서버 화면 E2E | 응답 필드를 현지 시각·KiB/MiB로 표시하고, 확인 불가 값을 0으로 표시하지 않는다. DASH-001·006·020·021 | 통과 | `tests/dashboard_live.rs`의 `dashboard_001_real_overview_data_and_unavailable_values`, `dashboard_006_selected_real_session_facts_and_return`, `dashboard_020_real_eof_error_admin_and_empty_history`, `dashboard_021_real_pending_counts_and_resource_usage`. 실제 중계·원시 관리 조회와 DOM을 대조한다. EOF·WS 연결 오류·관리자 종료를 실제로 만든다. 확인 불가 값은 프록시 조회 실패로 확인하며 정상 서버의 null 응답을 위조하지 않는다. 모두 비부분이며 `9cdebc6`의 두 OS 전체 게이트에서 통과했다. |
| Consumer / 실서버 화면 E2E | 빈 성공과 조회 실패를 구분하고, 실패 뒤 재조회로 복구한다. DASH-002·003·004 | 통과 | `tests/dashboard_live.rs`의 `dashboard_002_real_empty_lists`, `dashboard_003_first_and_stale_failures_on_both_screens`, `dashboard_004_explicit_refresh_and_success_times_on_both_screens`. 실제 빈 목록, 두 화면의 최초·갱신 실패, 오래된 데이터, 명시적 복구와 성공 시각을 확인한다. `500 internal`은 명시적 프록시 주입이며 Producer의 실제 500 증거가 아니다. 모두 비부분이며 `9cdebc6`의 두 OS 전체 게이트에서 통과했다. |
| Consumer / 실서버 화면 E2E | 서버의 설정 P·기본 2초로 폴링하고, 같은 조회를 겹치지 않으며 늦은 응답을 버린다. 상태 변경은 다음 정상 폴링에 표시한다. DASH-005·016·022 | 통과 | `tests/dashboard_live.rs`의 `dashboard_005_real_delays_do_not_overlap_or_overwrite_routes`, `dashboard_016_real_state_changes_appear_on_next_poll`, `dashboard_022_real_configured_and_default_polling_on_both_screens`. 별도 실제 프로세스의 설정 P·기본 P, 실제 전송 지연·동시 요청 수·화면 전환·최초 DOM 반영 시각을 두 화면에서 확인한다. 모두 비부분이며 `9cdebc6`의 두 OS 전체 게이트에서 통과했다. |
| Consumer / 실서버 화면 E2E | `401`에서 보호 데이터를 제거하고 로그인으로 이동한다. 실제 로그인·입력 검증·실패 제한·로그아웃 흐름을 따른다. DASH-008·014·017·018·019 | 통과 | `tests/dashboard_live.rs`의 `dashboard_008_real_300_second_idle_expiry_on_both_screens`, `dashboard_014_unauthenticated_direct_detail_exposes_no_data`, `dashboard_017_real_login_opens_overview_and_queries`, `dashboard_018_empty_wrong_and_rate_limited_real_login`, `dashboard_019_logout_success_and_lost_response_on_both_screens`. 독립 쿠키별 실제 Si=300초, 297초 이후의 보호 DOM·정상 폴링, 300초 이전 401 없음, 첫 401 이후 데이터 제거·폴링 중단을 확인한다. 실제 실패 제한은 F 초과로 시작한다. 로그아웃 성공·실제 응답 폐기를 두 화면에서 확인하고 중계를 유지한다. 모두 비부분이며 `9cdebc6`의 두 OS 전체 게이트에서 통과했다. |
| Consumer / 실서버 화면 E2E | 취소·닫기는 요청 없음, 완료·자연 소멸·실패·응답 유실 구분, 유실 뒤 자동 재전송 없이 상태 조회. DASH-007·009·010·011·012·013 | 통과 | `tests/dashboard_live.rs`의 `dashboard_007_real_disappearance_before_and_during_detail`, `dashboard_009_cancel_and_close_restore_focus_without_termination`, `dashboard_010_real_termination_blocks_duplicates_and_refreshes_list`, `dashboard_011_natural_close_around_confirmation_preserves_other_session`, `dashboard_012_injected_failure_is_not_termination_completion`, `dashboard_013_lost_real_termination_response_reads_before_resending`. 실제 중계 소멸·종료와 요청/응답 순서를 확인한다. `terminate_failed` 500만 프록시에서 명시적으로 주입한다. 응답 유실은 실제 종료 응답을 폐기한다. 모두 비부분이며 `9cdebc6`의 두 OS 전체 게이트에서 통과했다. |
| Consumer / 실서버 화면 E2E | 잘못된 상세 식별자는 API 요청 없이 잘못된 ID 안내를 표시한다. 종료 행동이 없고 실제 404와 구분한다. DASH-026 | 통과 | `tests/dashboard_live.rs`의 `dashboard_026_malformed_ids_send_no_query_and_differ_from_not_found`. 잘못된 ID 여섯 형태의 요청 없음과 유효하지만 없는 ID의 실제 `404 not_found`를 대조한다. 비부분이며 `9cdebc6`의 두 OS 전체 게이트에서 통과했다. |
| Consumer / 실서버 화면 E2E | 로그인 서버 오류·응답 없음은 인증 실패와 다른 안내를 표시하고 중복 전송을 막는다. DASH-027 | 통과 | `tests/dashboard_live.rs`의 `dashboard_027_injected_login_failures_block_duplicates_and_allow_retry`. 명시적 프록시 `500 internal`, 연결 종료, 실제 성공 응답 폐기와 11초 지연에 따른 브라우저 제한 시간을 확인한다. 전달·주입·완료 카운터로 출처를 구분한다. 비부분이며 `9cdebc6`의 두 OS 전체 게이트에서 통과했다. Producer가 실제 로그인 500을 냈다는 증거가 아니다. |
| Producer / 조회 API 단위·시스템(작업 4.4, 생성) | 1.6~1.8·1.10절의 서버 요약·대기·활성 목록·상세·최근 종료 조회. MGT-001·002·003·006·015·021·024 | 통과 | `tests/management_query.rs`, 구현 `ec05fd1`과 MGT-002 보강 `b20b8dc`에서 Windows·WSL 전체 게이트를 통과했다. MGT-002는 E = 5초의 반닫기로 목록·상세의 closing·활성 집계 포함, 정리 뒤 상세 404·최근 종료 포함을 확인했다. MGT-026은 Linux 시험(WSL)과 Windows 시험(커밋 `0196cab`)으로 두 OS에서 비부분 통과다. 아래 작업 4.4 실행 기록을 따른다. |
| Producer / 조회 API 단위·시스템(작업 4.4, 완료 보강) | MGT-004·005·007·013·016, OPS-033, Windows의 MGT-026 | 통과 | 2026-11-04에 비부분 시험으로 채웠다(커밋 `bb44c4f`·`e9221c6`·`fb8a362`·`0196cab`). 시나리오별 시험은 위 행과 [관리 조회 API 대조 기록](../architectures/management-query.md#작업-44-완료-대조2026-11-04)에 있다. |
| Consumer / 실서버 CDP E2E 종합 | DASH-001~014·016~022·024~027의 전체 기대 결과. 실제 Si=300초 만료, 세 화면 언어 포함 | 통과 | `9cdebc6`의 `tests/dashboard_live.rs`: Windows 26개 통과(502.32초), WSL 26개 통과(491.04초). 25개는 비부분 DASH 행이고 1개는 사전 연결 프록시 회귀 시험이다. 두 OS의 DASH 보고서는 모두 통과 25·부분 1(DASH-015)·미연결 1(DASH-023)이다. Q112/Q113의 주입 오류·미래 enum 입력 범위는 아래 출처 구분을 따른다. 네 브라우저 호환성·수동 접근성은 통과 범위가 아니다. |

2026-11-05 Consumer 시험의 출처 구분: `tests/dashboard_live.rs`의 25개 행은 일반 로그인·조회·종료 데이터를 실제 실행파일에서 얻는다. DASH-001·021의 확인 불가 상태는 실제 조회 전달 실패로 만든다. DASH-025의 미래 enum 값만 제품 파서와 표 컴포넌트에 직접 주는 별도 Consumer 계약 입력이다. 현재의 타입화된 Producer는 그 값을 낼 수 없다. 프록시가 정상 응답을 위조하지 않으며 이 입력을 실제 서버 데이터라고 주장하지 않는다. 기존 부분 시험과 아래 과거 실행 기록은 그대로 보존한다.

작업 4.9~4.11 최종 실서버 실행 기록: Windows `vr-win.ps1 -Only all`은 `ALL PASSED (all)`, WSL `vr-full.sh`는 `ALL PASSED`이며 각 단계 종료 코드는 모두 0이다. Windows 보고서는 생성 시각 `2026-11-04T21:30:23Z`, 대상 `9cdebc6`다. WSL 보고서는 `2026-11-04T21:47:19Z`, 대상 표기가 `9cdebc6 + 미커밋 변경`이다. 두 게이트 실행 중 저장소를 편집하지 않았다. WSL 미커밋 표기의 원인은 이번에 감사하지 않았으며 깨끗한 작업 트리였다고 주장하지 않는다. 전체 153건 중 Windows는 통과 133·부분 14·실패 0·미실행 0·미연결 6·오류 0, WSL은 통과 137·부분 9·실패 0·미실행 1(Windows 콘솔 창 닫기 OPS-044)·미연결 6·오류 0이다. 기존 `dashboard_browser` 37개도 Windows 187.56초, WSL 111.99초로 모두 통과했다. 과거 작업 4.9~4.11의 목 응답 시험은 당시 부분 증거다. 그 실행 기록이나 부분 표식을 비부분으로 바꾸지 않는다. 현재 비부분 결과는 위 실서버 행과 [검증 절차·기록](../how-to/verify-dashboard-frontend.md)이 설명한다.

작업 4.3 실행 기록(커밋 `59f3f62` + 문서 변경 작업 트리): WSL `vr-full.sh` ALL PASSED(통과 11, 부분 27, 미연결 110). Windows `vr-win.ps1 -Only all` ALL PASSED (all)(통과 10, 부분 27, 미실행 1 = OPS-007, 미연결 110). 위 표의 결과 열은 검증 기록 규칙에 따라 부분 행을 `미실행`으로 둔다. 빠진 부분은 작업 4.4·4.5에서 채운다. 대조 기록: [관리 로그인과 세션 설계](../architectures/management-session.md#대조-기록).

작업 4.3 설계 수정 구현 실행 기록(커밋 `a143539`, 검증 뒤 코드 변경 없음): WSL `vr-full.sh` ALL PASSED(통과 11, 부분 27, 미연결 110). Windows `vr-win.ps1 -Only all` ALL PASSED (all)(통과 10, 부분 27, 미실행 1 = OPS-007, 미연결 110). 이 커밋은 계약 1.1절의 GET·DELETE 본문 415와 GET의 `as_of`를 구현했다. 계정 이름 문법 위반의 400 응답은 구현 판단이다(설계 문서 9절 라). 표 행은 바꾸지 않았다. 대조 기록: [관리 로그인과 세션 설계](../architectures/management-session.md#대조-기록).

작업 4.3 결정 반영 실행 기록(커밋 `8c87e69` + 문서 변경 작업 트리): WSL `vr-full.sh` ALL PASSED(통과 14, 부분 24, 실패 0, 미실행 0, 미연결 110). Windows `vr-win.ps1 -Only all` ALL PASSED (all)(통과 13, 부분 24, 실패 0, 미실행 1 = OPS-007, 미연결 110). 앞선 기록보다 통과가 3건 늘었다. MGT-018·MGT-022(`8c87e69`)와 OPS-002(`b505c60`, 다른 작업의 `tests/serve.rs`)다. 환경과 명령은 [관리 로그인과 세션 설계](../architectures/management-session.md#대조-기록)의 결정 반영 기록을 따른다.

작업 4.10 실행 기록(대상 HEAD `3b3a179`, 코드 `2f3b2a3`): WSL `vr-full.sh` ALL PASSED(통과 24, 부분 40, 실패 0, 미실행 0, 미연결 86). Windows `vr-win.ps1 -Only all` ALL PASSED (all)(통과 23, 부분 40, 실패 0, 미실행 1 = OPS-007, 미연결 86). 위 4.10 표 행의 결과 열은 부분 행을 `미실행`으로 둔다. 빠진 부분은 작업 4.4·4.11에서 채운다. 대조 기록: [대시보드 운영 현황 개요](../architectures/dashboard-operations-overview.md).

작업 4.4 실행 기록(설계 `bc7e762`, 구현·시험 `ec05fd1`, 사용자 확인 결과): Windows `vr-win.ps1 -Only all`의 2026-11-01T18:19:56Z 보고서는 `ALL PASSED (all)`이다. 150건 중 통과 69, 부분 64, 실패 0, 미실행 0, 미연결 17, 오류 0이다. `management.md` 27건은 통과 8, 부분 15, 실패 0, 미실행 0, 미연결 4다. WSL `vr-full.sh`의 18:33:21Z 재실행 보고서는 `ALL PASSED`이며 통과 73, 부분 60, 실패 0, 미실행 0, 미연결 17, 오류 0이다. 앞선 조회 API의 미생성·미실행 서술은 당시 기록이며, 현재 조회 범위는 작업 4.4 행과 [관리 조회 API 대조 기록](../architectures/management-query.md#대조-기록)을 따른다. 부분 행의 결과 열은 기존 기록 규칙대로 `미실행`으로 둔다.

WSL 첫 전체 게이트(2026-11-01T18:25:46Z)는 `tests/reload.rs`의 `new_tls_pair_is_used_by_new_handshakes_and_keeps_open_connections`(OPS-024) 1건이 실패했다. 감지 뒤 `reload_applied`까지 3357 ms로 D = 1000 ms를 넘었다. 단독 5회와 위 전체 재실행은 모두 통과했다. 전체 게이트 부하에서 한 번 관찰된 간헐적 D 초과이며 원인은 확정하지 않았다. `ec05fd1`은 재로드·TLS 교체 경로를 바꾸지 않는다. 이 문서 작업에서는 게이트를 다시 실행하지 않았다.

후속(커밋 `fd8c097`): 이 실패의 원인은 WSL2 벽시계 보정으로 추정한다([INFERENCE]). 시험은 로그 `ts`(벽시계) 차이로 D를 쟀다. 검증 환경에서는 40초 동안 +2.933초 보정이 관찰됐다. 시험을 단조 시계 측정으로 바꿨고, 두 OS 전체 게이트에서 OPS-024가 통과했다. 근거와 실행 기록은 [파일 변경 감지 설계의 OPS-024 시험 시간 측정 수정](../architectures/file-watch.md#ops-024-시험의-시간-측정-수정)에 있다.

작업 4.4 MGT-002 보강 실행 기록(커밋 `b20b8dc`, 시험 작업자 보고서에서 사용자가 확인한 결과): Windows `vr-win.ps1 -Only all`의 2026-11-01T19:04:30Z 보고서는 `ALL PASSED (all)`이다. 150건 중 통과 70, 부분 63, 실패 0, 미실행 0, 미연결 17, 오류 0이다. `management.md` 27건은 통과 9, 부분 14, 실패 0, 미실행 0, 미연결 4다. WSL `vr-full.sh`의 19:09:51Z 첫 실행 보고서는 `ALL PASSED`이며 통과 74, 부분 59, 실패 0, 미실행 0, 미연결 17, 오류 0이다. `management.md` 27건은 통과 10, 부분 13, 실패 0, 미실행 0, 미연결 4다. WSL 머리의 `+ 미커밋 변경`은 기존과 같은 55개 경로의 줄바꿈 차이이며 `git diff --ignore-cr-at-eol --stat` 출력은 비어 있었다. MGT-002는 두 환경에서 통과로 바뀌었고 나머지 4.4 상태는 `ec05fd1` 기록과 같다. 이 문서 작업에서는 게이트를 다시 실행하지 않았다.

작업 4.5·3.4·2.11 실행 기록(커밋 `59fc535`. 앞선 커밋 `768d1af`·`7c706c6`·`cd60aad`·`538a4ec`를 포함한다): Windows `vr-win.ps1 -Only all`의 2026-11-01T23:41:53Z 보고서는 `ALL PASSED (all)`이다. 150건 중 통과 77, 부분 61, 실패 0, 미실행 0, 미연결 12, 오류 0이다. `management.md` 27건은 통과 15, 부분 12, 미연결 0이다. WSL `vr-full.sh`의 23:51:07Z 보고서는 `FAILED: scenarios`이며 통과 81, 부분 55, 실패 2, 미실행 0, 미연결 12, 오류 0이다. 실패 2건은 RLY-025·026이다. `tests/relay_dead_peer_linux.rs`가 WSL에 `nft`가 없어 Q93대로 실패했다. 다른 시험은 모두 통과했다. `management.md` 27건은 통과 16, 부분 11, 미연결 0이다. MGT-008·009·010·011·014·023은 두 OS에서 통과다. MGT-012·013은 부분이다. WSL 머리의 `+ 미커밋 변경`은 기존과 같은 줄바꿈 차이다. 대조 기록: [관리자 종료 설계 8절](../architectures/management-terminate.md#8-현재-구조와-검증-범위).

작업 4.3·4.4·4.5 증거 보강 실행 기록(2026-11-04, 커밋 `b0c25f9`. 앞선 커밋 `c0d3b59`·`34cefe5`·`bb44c4f`·`b5c1a78`·`e9221c6`·`fb8a362`를 포함한다): WSL `vr-full.sh`의 12:01:00Z 보고서는 `ALL PASSED`다. 151건 중 통과 100, 부분 40, 실패 0, 미실행 0, 미연결 11, 오류 0이며 `management.md` 27건이 모두 통과다. Windows `vr-win.ps1 -Only all`의 12:08:11Z 보고서는 `ALL PASSED (all)`이다. 151건 중 통과 92, 부분 48, 실패 0, 미실행 0, 미연결 11, 오류 0이며 `management.md`는 통과 25, 부분 2(MGT-026·MGT-027)다. 같은 날 앞선 WSL 게이트(커밋 `082f5f1`)는 로그 시험의 벽시계 `ts` 비교 단언 때문에 실패했고, `b0c25f9`가 줄 순서 판정으로 고쳤다. MGT-026의 Windows 비부분 시험은 커밋 `0196cab`에서 추가했다. MGT-027의 Windows 정상 종료 시험은 작업 5.3에서 만든다. 대조 기록: [관리 조회 API](../architectures/management-query.md#작업-44-완료-대조2026-11-04), [관리 로그인과 세션](../architectures/management-session.md#작업-43-증거-보강과-브라우저-관찰2026-11-04), [관리자 종료 8절](../architectures/management-terminate.md#8-현재-구조와-검증-범위).

작업 4.4 MGT-026 Windows 실행 기록(커밋 `a70df88`. 시험 커밋 `0196cab` 포함, 작업 트리 깨끗함): Windows `vr-win.ps1 -Only all`의 2026-11-04T12:25:35Z 보고서는 `ALL PASSED (all)`이다. 151건 중 통과 93, 부분 47, 실패 0, 미실행 0, 미연결 11, 오류 0이다. `management.md` 27건은 통과 26, 부분 1(MGT-027)이다. MGT-026은 `counts_and_usage_match_at_limit_with_pending_connections_windows`(비부분)로 통과했다. WSL은 이 시험을 컴파일하지 않으므로(`#[cfg(windows)]`) `b0c25f9` 게이트 결과를 그대로 쓴다.

작업 5.3 및 4.3 완료 실행 기록(커밋 `de693ad`. 앞선 커밋 `976550f`·`2ef5a05` 포함, 작업 트리 깨끗함): WSL `vr-full.sh`의 2026-11-04T13:31:32Z 보고서는 `ALL PASSED`다. 152건 중 통과 107, 부분 35, 실패 0, 미실행 1(OPS-044, Windows 콘솔 창 닫기 전용), 미연결 9, 오류 0이며 `management.md` 27건이 모두 통과다(27/27). Windows `vr-win.ps1 -Only all`의 2026-11-04T13:39:13Z 보고서는 `ALL PASSED (all)`이다. 152건 중 통과 103, 부분 40, 실패 0, 미실행 0, 미연결 9, 오류 0이며 `management.md` 27건이 모두 통과다(27/27). 앞서 커밋 `976550f`의 첫 WSL 게이트는 `vnc-relay → windows-sys (dev)` 의존성이 코드 지도에 미반영되어 `cargo xtask layers` 검사에서 실패했고, 커밋 `2ef5a05`가 코드 지도 계층 행과 `xtask/src/layers.rs`를 갱신해 고쳤다. 커밋 `2ef5a05`의 WSL 게이트는 `Cargo.lock` 변경에 따른 `THIRD-PARTY-NOTICES.txt` 미갱신으로 `tests/dependency_notices.rs` 1건이 실패했고, 커밋 `de693ad`가 고지 파일을 다시 생성해 해결했다. MGT-027(`graceful_restart_ends_the_admin_session_and_login_works_again`)과 OPS-043(`management_works_during_grace`)이 두 OS에서 모두 통과했다. 대조 기록: [관리 로그인과 세션 설계](../architectures/management-session.md#작업-43-완료-대조2026-11-04).


## 6. 초안을 확정하기 전에 해결할 사항

- 이 계약의 이름·코드 선택은 2026-10-16에 사용자가 채택했다. 채택 범위는 `/api/v1/` 아래 경로 이름과 메서드, 필드 이름, 오류 코드 일곱 개, `X-Relay-Poll`·`X-CSRF-Token` 값 규칙, 종료 원인 값 이름, 화면 URL이다. 쿠키 이름 `relay_admin_session`과 헤더 이름은 공유 인터페이스 이름이다. 사용자가 앞서 확정한 값은 이 문서 앞부분에 따로 적었다.
- 이 계약의 채택으로 관리 스펙의 MGT-TBD-001(경로·메서드·오류 본문·로그인 교환 형식, 자동·사용자 요청 구분 방식)과 화면 스펙의 DASH-TBD-002(화면 URL·세션 식별자 입력 형식)를 해소했다. 기준은 1.2·1.3·1.4·1.5절과 2절(MGT-TBD-001), 1.8절(형식)과 1.11절(화면 URL, 잘못된 대상과 대상 없음의 구분)(DASH-TBD-002)이다. DASH-TBD-003은 로그인·로그아웃 URL과 세션 오류 코드(1.2절 `POST`·`DELETE /api/v1/session`, 2절)가 해소됐다. 화면 메시지 문구는 [화면 스펙 9절](../specs/dashboard.md#9-화면-문구)이 정한다(사용자 결정 2026-10-22(Q49)). 이로써 DASH-TBD-003이 모두 해소됐다.
- 공유 인터페이스에 없던 `GET /api/v1/session`(세션 정보·CSRF 토큰 복구)과 `terminate_timeout_secs`(E + 5초), `max_entries`·`as_of` 필드, 세션 객체의 `state`도 채택 범위다.
- `WWW-Authenticate: Session realm="vnc-relay"`는 확정했다. (해결, Safari 제외) 2026-11-04 관찰: Windows의 Chrome 156.0.8078.76·Edge 156.0.4314.58과 WSL의 Firefox 159.0은 이 헤더의 401에 기본 인증 창을 띄우지 않고 JSON 본문을 보였다. 같은 브라우저가 `Basic` 401에는 인증 요구로 반응해 대조가 성립했다. 자동화 환경에서는 인증 창 자체가 화면에 보이지 않으므로, 판정 근거는 `Session`과 `Basic`의 반응 차이다. Safari는 macOS가 없어 관찰하지 않았으며 작업 4.12의 Safari 확인에서 함께 본다. 2026-11-07 Safari 대체 WebKit 27.2(Playwright, 사용자 결정 2026-11-07(Q135))는 `Basic` 401에도 본문을 보여 대조가 성립하지 않았다. 판정하지 못했다([검증 기록 4.21절](../how-to/verify-dashboard-frontend.md#421-webkit-대체-확인-작업-412-q135-2026-11-07)).
- (해결) 중계 ID의 문자열 표현: [Repeater 초기 교환 계약 1.2절](repeater-handshake-v1.md#12-초기-요청-250바이트)이 중계 ID를 ASCII 숫자 1~247자리로 정했다(사용자 결정 2026-11-01(Q68·Q69)). 숫자가 아닌 바이트가 있는 요청은 `handshake_invalid`로 거절하므로 대기 목록에 오지 않는다. 그래서 JSON 문자열로 표현할 수 없는 바이트는 생기지 않는다. 대상 버전 Repeater는 숫자가 아닌 ID(`12ab`)도 받았다([호환성 검증 기록](repeater-compat-record.md) 3.5절). 이 차이는 초기 교환 계약 3.1절이 다룬다. 1.7절 `relay_id`에 반영했다(2026-11-01, TODO 1.7).
- (해결) 전송량이 JavaScript 정수의 안전 범위(2^53 − 1 바이트, 약 8 PiB)를 넘으면 대시보드가 정확한 정수를 다루지 못한다. Producer는 정확한 정수를 그대로 낸다. 대시보드 표시 형식은 [디자인 기준](../design/DESIGN.md#표시-형식)이 MiB 소수 첫째 자리로 정했다. 사용자 결정 2026-10-23(Q53)으로 한계를 받아들이고 이 항목을 닫았다. 2^53 − 1 바이트까지는 표시가 정확하다. 2^53 이상 2^70 바이트 미만에서는 반올림 경계 근처 값의 표시가 한 칸(0.1 MiB) 어긋날 수 있다. 2^70 바이트 이상은 시험하지 않는다. 근거와 시험은 [운영 현황 앱 구조](../architectures/dashboard-operations-overview.md) 9절과 14절 D1에 있다.
- (해결) IPv4 매핑 IPv6 주소(`::ffff:a.b.c.d`)의 `ip` 표기는 `IpAddr::to_canonical()`을 적용한 `a.b.c.d`다. `PeerIp`와 운영 로그의 규칙에 맞췄다. 기준은 1.1절 `address` 행이다(사용자 결정 2026-11-02(Q88)).
- (해결) terminate의 모든 본문은 `Content-Type`과 관계없이 400이며 1.1절의 415 규칙보다 우선한다. 기준은 1.9절이다(사용자 결정 2026-11-02(Q89)).
- (해결) 검증·수락된 terminate 요청의 정리 완료 대기 중에만 U를 멈춘다. 응답 쓰기부터 다시 잰다. 기준은 1.9절과 [설정 계약 1.2절](config-v2.md#12-키-표)이다(사용자 결정 2026-11-02(Q92)).
- 로그인 CSRF는 `Content-Type: application/json` 요구, CORS 헤더 미송신, `SameSite=Strict`로 막는다. (해결, Safari 제외) 2026-11-04 관찰: Chrome·Edge(Windows)와 Firefox(WSL)에서 다른 사이트(`http://localhost:<port>`)의 공격 페이지가 평문 루프백 관리 리스너에 로그인을 시도했다. JSON `fetch`는 사전 요청에서 막혔고, `no-cors` `text/plain` 요청은 불투명 응답만 받았으며, `text/plain` 폼 POST는 `415 invalid_request`와 `Set-Cookie` 없음으로 끝났다. 공격 뒤 쿠키는 생기지 않았고 운영 로그의 `admin_login_succeeded`는 정상 로그인의 것뿐이었다. 정상 로그인 뒤 다른 사이트의 `no-cors` 요청에는 쿠키가 실리지 않았다. Safari는 관찰하지 않았다(작업 4.12). 2026-11-07 Safari 대체 WebKit 27.2(Playwright, Q135)에서도 로그인 시도 세 가지는 같은 결과였다. 다만 이 Windows용 빌드는 `SameSite=Strict` 쿠키를 `SameSite=None`으로 저장해 다른 사이트의 `no-cors` GET에 쿠키를 실었다. 같은 조건의 다른 사이트 종료 요청은 `X-CSRF-Token`이 없어 403이었다. 실제 Safari의 SameSite 동작은 판정하지 못했다([검증 기록 4.21절](../how-to/verify-dashboard-frontend.md#421-webkit-대체-확인-작업-412-q135-2026-11-07)).
- (해결) 로그인 응답 시간으로 계정 존재 여부를 알 수 없는지는 구조 시험으로 판정한다. 사용자 결정 2026-10-22(Q31)로 해결했다. 모든 로그인 시도가 비밀번호 검증을 정확히 한 번 하는지를 `CountingVerifier`로 센다. 자동 시간 측정은 하지 않는다. 계정 해시와 더미 해시의 Argon2 매개변수가 같은지는 확인하지 않았다 [INFERENCE: 다를 수 있다].
- 계정명의 허용 문자와 길이는 [CLI 계약 1.2절](cli-v1.md#12-계정-이름과-비밀번호-규칙)이 정한다. 로그인 본문 한도 16 KiB와 종료 대기 여유 5초도 채택 범위다.
- 스펙 시나리오가 직접 다루지 않는 오류가 있다. 형식이 잘못된 요청(`invalid_request` 중 본문·필드 오류), 알 수 없는 경로와 허용되지 않는 메서드(`404`·`405`)다. 이 계약은 가장 가까운 시나리오(DASH-018·MGT-013·MGT-025)와 화면 스펙 5절에 연결했다. 대응 시나리오는 스펙에 없으며 사용자가 추가하지 않기로 했다(2026-10-16). 계약 본문의 해당 응답 정의는 유지한다.
- 종료 원인 `close_cause`의 값은 일곱 개다. 사용자 결정 2026-11-02(Q94)으로 추가한 `internal_error`를 포함하며 [관리 스펙 2.3절](../specs/management.md#23-최근-종료-목록)·PRD FR-06의 종료 원인 목록과 같다. 프로세스가 끝나면 목록도 사라지므로 `server_shutdown` 항목이 실제로 조회되는지는 구현에서 확인해야 한다.
- 계약 테스트는 `crates/adapters/tests/management.rs`(작업 4.2·4.3·4.6), `tests/dashboard_assets.rs`(작업 4.6), `tests/dashboard_browser.rs`(작업 4.9·4.10·4.11), `tests/serve.rs`의 `refuses_public_plaintext_and_answers_login_less_queries_with_401_on_loopback`, 단위 시험(`crates/app/src/management/login.rs`·`session.rs`·`query.rs`·`terminate.rs`, `crates/adapters/src/management_http/terminate_api.rs`), `tests/management_session.rs`(작업 4.3), `tests/management_query.rs`(작업 4.4), `tests/management_terminate.rs`(작업 4.5)다. CI 실행 기록은 없다. 나머지 테스트 파일이 생기면 5절 표의 테스트 위치와 결과를 채운다.
