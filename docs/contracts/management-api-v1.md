# 관리 API v1 계약

관리 리스너가 제공하는 JSON over HTTP 관리 API의 경로·메서드·요청·응답·오류와 로그인 교환 형식을 정한다. 내장 대시보드(정적 HTML·JS)가 이 API로 서버 요약·대기 연결·활성 세션·최근 종료 목록·최근 로그·메트릭과 설정을 조회하고, 동적 설정을 편집하고, 활성 세션을 종료하고, 관리자 로그인·로그아웃을 한다. 이 문서는 [관리 스펙](../specs/management.md)과 [화면 스펙](../specs/dashboard.md)의 동작을 바꾸지 않는다. 스펙과 다르면 스펙을 따른다.

| 항목 | 값 |
| --- | --- |
| 이름과 버전 | `management-api` 1.3.0 |
| 상태 | Active |
| 버전 차이 | 1.1.0은 1.11절 CSP 헤더가 필수다. Consumer 동작은 같으며 Producer는 CSP 헤더를 보내야 한다. 1.2.0은 1.16절 메트릭 경로를 더한 호환 추가다. 1.3.0은 1.16절 `history_window` 선택 매개변수를 더한 호환 추가다. |
| 소유자 | vnc-relay 프로젝트 관리자 |
| 만드는 쪽(Producer) | `vnc-relay` 관리 리스너(서버) |
| 사용하는 쪽(Consumer) | 같은 실행파일에 내장된 대시보드(정적 HTML·JS). 같은 규칙으로 호출하는 외부 HTTP 클라이언트도 Consumer로 본다. |
| 방향 | 요청/응답. 브라우저 → 관리 리스너. HTTPS가 원칙이며 평문 HTTP는 루프백 관리 리스너에서만 쓴다. |
| 스키마 기준 파일 | 없음. 스키마 파일을 만들지 않는다. 1절의 표가 형식의 기준이다. |
| 기준 스펙 | [관리 조회와 세션 종료](../specs/management.md), [운영 대시보드 화면](../specs/dashboard.md). L·Lm·E·P 등 매개변수 정의는 [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)을 따른다. |
| 관련 요구사항 | [PRD](../requirements/PRD.md) FR-06·FR-07·FR-08(비밀 비노출), 7절 보안 경계, AC-09·AC-10·AC-19~AC-21·AC-52 |
| 호환성 범위 | 경로 접두사 `/api/v1/`. 같은 릴리스의 서버와 내장 대시보드. HTTPS 관리 리스너와 평문 HTTP 루프백 관리 리스너. |
| 관련 테스트 | [관리 스펙 7절](../specs/management.md#7-수용-테스트) MGT-001~MGT-055, [화면 스펙 7절](../specs/dashboard.md#7-수용-테스트), [운영 스펙 7절](../specs/operations.md#7-수용-테스트) OPS-043. 계약 테스트와 현재 결과 확인 방법은 5절을 따른다. |
| 접속 ticket 관련 테스트 | [관리 스펙 7절](../specs/management.md#7-수용-테스트) MGT-052~053. |
| 목록 필드 관련 테스트 | [관리 스펙 7절](../specs/management.md#7-수용-테스트) MGT-054~055, [화면 스펙 7절](../specs/dashboard.md#7-수용-테스트) DASH-065~066. 시험 위치와 현재 결과 확인 방법은 5절을 따른다. |
| 메트릭 관련 테스트 | [관리 스펙 7절](../specs/management.md#7-수용-테스트) MGT-056~061, [화면 스펙 7절](../specs/dashboard.md#7-수용-테스트) DASH-068~081. |

관련 문서: [설정 계약](config-v4.md)(`[dashboard]`·`[limits]`·`[recent_closed]`·`[recent_logs]`·`[timeouts]`·`[management]` 키와 기본값·범위), [CLI·계정 파일 계약](cli-v1.md)(계정명 규칙), [운영 정보 계약](ops-info-v2.md)(`session_id`·종료 사건 로그, 상태 리스너), [메트릭 계약](metrics-v1.md)(메트릭 응답 본문), [WebSocket 중계 계약](websocket-relay-v2.md)(관리자 종료의 Close 4005), [ADR-0001 내장 TLS](../decisions/ADR-0001-built-in-tls.md), [ADR-0003 단일 인스턴스·메모리 상태](../decisions/ADR-0003-single-instance-memory-state.md).

이 계약의 경로·메서드·필드·오류 코드·헤더·쿠키·화면 URL은 확정된 형식이다. 기본 규칙은 다음과 같다.

- 관리 API는 JSON over HTTP이고 경로 접두사는 `/api/v1/`이다.
- 시각은 UTC ISO 8601 문자열이고, 전송량은 바이트 정수다. 화면이 현지 시각과 KiB/MiB로 바꿔 표시한다.
- 관리 세션 쿠키는 HttpOnly·SameSite=Strict이고, HTTPS에서는 Secure를 붙인다. 평문 HTTP 루프백 관리 리스너에서만 Secure를 뺀다. CSRF 방어를 별도로 유지한다.
- 관리 세션은 서버 메모리에 두며, 재시작하면 모두 무효다.
- 종료 요청은 정리가 끝난 뒤 응답한다. 별도의 "처리 중" 응답은 없다.
- 폴링 주기 P(`dashboard.poll_secs`)와 폴링 조회 제한 시간(`dashboard.query_timeout_secs`)의 기본값·유효 범위는 [설정 계약 1.2절](config-v4.md#12-키-표)이 정한다. 이 문서는 수치를 반복하지 않는다.

## 1. 형식과 의미

### 1.1 공통 규칙

| 항목 | 규칙 |
| --- | --- |
| 전송 | 관리 리스너 하나에서 정적 대시보드(`/`)와 API(`/api/v1/`)를 함께 제공한다. 상태 리스너(`/livez`·`/readyz`·`/metrics`)는 이 계약의 대상이 아니다. |
| 본문 형식 | 요청·응답 본문은 UTF-8 JSON이다. 응답은 `Content-Type: application/json; charset=utf-8`이다. 본문이 있는 요청은 `Content-Type: application/json`을 보내야 한다. 다르면 `415`와 `invalid_request`로 거절한다. 단, 1.9절의 본문 금지처럼 경로별 규칙이 있으면 그 규칙을 먼저 적용한다. |
| 응답 헤더 | API 응답에 `Cache-Control: no-store`를 붙인다. CORS 헤더(`Access-Control-*`)를 보내지 않는다. 같은 출처의 대시보드만 API를 호출한다. |
| 필드 이름 | 소문자 `snake_case`다. |
| 시각(`timestamp`) | 문자열. UTC ISO 8601. 형식은 `YYYY-MM-DDThh:mm:ss.sssZ`다. 밀리초를 항상 3자리로 쓰고, 오프셋은 항상 `Z`다. 현지 시각 변환은 Consumer가 한다. |
| 전송량(`bytes`) | 정수. 0 이상. 단위는 바이트다. 단위를 바꾸거나 반올림한 값을 넣지 않는다. 의미는 [관리 스펙 2.1절](../specs/management.md#21-전송량의-의미)을 따른다. |
| 경과 시간(`duration_ms`) | 정수. 0 이상. 단위는 밀리초다. 서버가 단조 시계로 계산한다. 시스템 시각을 보정해도 음수가 되지 않는다. |
| 기준 시점 | 모든 `GET` 성공 응답은 최상위 `as_of`(`timestamp`)를 가진다. 한 응답 안의 집계와 목록은 `as_of` 시점에 일관된다. 서로 다른 요청의 결과가 같은 시점이라고 보장하지 않는다. 서버 요약의 `usage`(L·Lm 사용량)는 이 일관성의 대상이 아니며, 연결이 바뀌는 동안 같은 응답의 `counts`와 잠깐 어긋날 수 있다([관리 스펙 2.2절](../specs/management.md#22-주소와-스냅샷)). |
| 목록 | 목록은 항상 `items` 배열이다. 정상적으로 조회한 빈 목록은 `200`과 `"items": []`다. 조회 실패나 인증 실패로 대신하지 않는다. |
| 정렬 | `waiting`·`sessions`는 오래된 항목이 먼저다(`connected_at`·`started_at` 오름차순). `closed-sessions`는 최근 종료가 먼저다(`ended_at` 내림차순). 같은 시각 항목의 순서는 정하지 않는다. |
| null과 확인 불가 | 값을 읽지 못하면 0으로 대신하지 않는다. 그 필드를 `null`로 주고, 같은 객체의 `<필드>_unavailable_reason`에 사유 문자열을 준다. 값이 있으면 사유는 `null`이다. 이 규칙은 1.6절 표가 "확인 불가 가능"으로 표시한 필드에만 적용한다. 설정 보기의 `revision`·`last_reload`·`items[].value`는 1.13절, 전달 주소는 1.8절, 최근 로그의 순번 경계는 1.15절의 별도 `null` 규칙을 따른다. 그 밖의 필드는 항상 값을 가진다. |
| 확인 불가 사유 | 현재 값은 `read_failed`(값을 읽는 데 실패)다. Consumer는 모르는 사유 문자열을 일반 "확인 불가"로 표시한다. |
| 상대 주소(`address`) | 객체 `{"ip": 문자열, "port": 정수}`. 실제 소켓 상대 주소다. 전달 헤더(`Forwarded`·`X-Forwarded-For` 등)로 바꾸지 않는다. `ip`는 `IpAddr::to_canonical()`을 적용한 표기다. IPv4 매핑 IPv6(`::ffff:a.b.c.d`)는 `a.b.c.d`로 쓴다. 그 밖의 IPv6는 괄호 없는 소문자 압축 표기다. `PeerIp`와 운영 로그도 같은 규칙을 쓴다. |
| 노출 범위 | 아래 1.1.1절을 따른다. |

전송량이 JavaScript 정수의 안전 범위(2^53 − 1 바이트, 약 8 PiB)를 넘더라도 Producer는 정확한 정수를 그대로 낸다. 대시보드의 MiB 소수 첫째 자리 표시는 2^53 − 1 바이트까지 정확하다. 2^53 이상 2^70 바이트 미만에서는 반올림 경계 근처의 표시가 한 칸(0.1 MiB) 어긋날 수 있다. 2^70 바이트 이상은 시험하지 않는다. 표시 형식은 [디자인 기준](../design/DESIGN.md#표시-형식)을 따른다.

#### 1.1.1 운영 정보 노출 범위

노출 범위는 [관리 스펙 2절](../specs/management.md#2-조회-대상과-상태)과 MGT-R08을 따른다. 이 계약이 범위를 넓히지 않는다.

| 정보 | 나오는 곳 | 나오지 않는 곳 |
| --- | --- | --- |
| 중계 ID(`relay_id`) | `GET /api/v1/waiting`의 항목만 | 활성 세션 목록·상세, 최근 종료 목록, 오류 본문, 쿠키 |
| 세션 식별자(`session_id`) | 활성 세션 목록·상세, 최근 종료 목록, 종료 응답, 최근 로그의 해당 사건 | 대기 연결 목록(세션이 아직 없음) |
| 상대 주소(`viewer_address`·`target_address`·`address`)와 전달 주소 | 활성 세션 목록·상세, 대기 연결 목록, 최근 종료 목록, 종료 응답의 `closed_session`. 최근 로그에는 운영 정보 계약이 정한 주소 필드가 나옴 | 오류 본문 |
| 관리자 비밀번호·비밀번호 해시·관리 세션 쿠키 값 | 어디에도 없음. 비밀번호는 로그인 요청 본문에만 있다. | 모든 응답·오류·로그 |
| CSRF 토큰 | 로그인 응답과 `GET /api/v1/session` 응답만 | 그 밖의 모든 응답 |
| 원시 중계 데이터(RFB 스트림) | 어디에도 없음 | 모든 응답 |
| 설정 키·값과 설정 경로 | `GET /api/v1/config`의 실행 중 유효 설정(1.13절). 최근 로그에는 운영 정보 계약이 정한 설정 키 등 사건 메타 정보만 포함 | 경로가 가리키는 파일 내용, 비밀번호·해시·개인키 내용, 오류의 입력값 인용 |
| 최근 로그 사건 | `GET /api/v1/logs`의 `entry`. 운영 정보 계약의 비밀 비노출·`relay_id_ref` 규칙을 그대로 따름 | 원시 중계 ID·인증 비밀·원시 스트림은 로그에 없음 |
| 메트릭 값 | `GET /api/v1/metrics`. 속성은 [메트릭 계약 1.4절](metrics-v1.md#14-속성-값)의 고정 값만 쓴다 | 중계 ID·세션 식별자·주소·계정명·설정 값(L·Lm 상한 제외)은 메트릭에 없음 |

오류 본문의 `message`에는 요청 입력값(계정명·비밀번호·세션 식별자·중계 ID·설정값)을 되풀이하지 않는다. 설정 진단의 `message`에도 값을 인용하지 않는다([설정 계약 2.1절](config-v4.md#21-설정-오류와-준비-실패)).

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
| `GET /api/v1/logs` | 필요 | 불필요 | `200` 최근 로그(1.15절) | 최근 로그 조회([관리 2.4절](../specs/management.md#24-최근-로그), MGT-046~050) |
| `GET /api/v1/metrics` | 필요 | 불필요 | `200` 메트릭(1.16절) | 메트릭과 최근 1시간 또는 최근 24시간 추이([관리 2.5절](../specs/management.md#25-메트릭), MGT-056~061) |
| `GET /api/v1/config` | 필요 | 불필요 | `200` 설정 보기(1.13절) | 실행 중 유효 설정과 파일 상태([관리 3.3절](../specs/management.md#33-설정-보기와-동적-설정-편집), MGT-R19) |
| `PATCH /api/v1/config` | 필요 | 필요 | `200` 편집 결과(1.14절) | 설정 파일 기록과 동적 설정 변경([관리 3.3절](../specs/management.md#33-설정-보기와-동적-설정-편집), MGT-R20~R27) |

`GET /api/v1/session`은 화면 진입·새로고침 때 유효한 관리 세션을 확인하고 CSRF 토큰을 복구하는 경로다. 화면이 새로고침하면 메모리의 CSRF 토큰이 사라지므로 같은 관리 세션의 토큰을 다시 준다.

위 표에 없는 `/api/v1/` 경로는 `404 not_found`다. 경로는 있으나 메서드가 다르면 `405 invalid_request`이고 `Allow` 헤더를 붙인다.

### 1.3 요청 처리 순서

서버는 `/api/v1/` 요청을 다음 순서로 판정한다. 앞 단계에서 거절하면 뒤 단계는 하지 않는다.

1. `POST /api/v1/session`이면 5단계의 로그인 처리로 간다. 그 밖의 요청은 2단계로 간다.
2. 쿠키로 관리 세션을 찾는다. 유효하지 않으면 `401 unauthenticated`다. 경로·메서드가 틀려도 같다. 인증 없는 호출자가 경로 존재 여부를 알 수 없게 한다.
3. 상태를 바꾸는 요청(`POST`·`PATCH`·`DELETE`)이면 `X-CSRF-Token`을 검증한다. 틀리면 `403 csrf_invalid`다.
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
| 사용자 행동으로 보내는 요청(화면 진입, 새로고침, 상세 열기, 종료, 설정 보기·편집, 로그아웃) | 없음 | 연장한다. 새 `idle_expires_at`은 `min(지금 + Si, absolute_expires_at)`이다. |

- `X-Relay-Poll`은 `GET` 요청에만 쓴다. 값은 `1`뿐이다. 다른 값이거나 `POST`·`PATCH`·`DELETE`에 붙이면 `400 invalid_request`다.
- 절대 최대 수명 Sa는 헤더와 활동에 관계없이 `absolute_expires_at`에 무효가 된다.
- 연장은 인증과 CSRF 검증을 통과한 요청만 일으킨다. 거절된 요청은 연장하지 않는다. 정적 파일 요청도 연장하지 않는다.
- Consumer는 자동 폴링에만 헤더를 붙인다. 어떤 화면 행동이 사용자 행동인지는 Consumer가 정한다.
- 서버는 유휴 만료·절대 만료·로그아웃·재시작 뒤 쿠키를 모두 같은 `401 unauthenticated`로 거절한다. 원인을 구분해 알려주지 않는다.

#### 1.4.3 CSRF 방어

- 로그인과 `GET /api/v1/session` 응답의 `csrf_token`은 그 관리 세션이 끝날 때까지 같은 값이다.
- Consumer는 로그인 이외의 `POST`·`PATCH`·`DELETE` 요청마다 헤더 `X-CSRF-Token: <csrf_token>`을 보낸다.
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
| `dashboard.poll_secs` | 정수 | 필수, null 불가 | 폴링 주기 P(초). 설정 `[dashboard] poll_secs`의 값이다. 기본값·범위는 [설정 계약](config-v4.md#12-키-표). |
| `dashboard.query_timeout_secs` | 정수 | 필수, null 불가 | 조회 제한 시간과 설정 편집 응답 대기 한도(초). 설정 `[dashboard] query_timeout_secs`의 값이다. 기본값·범위는 [설정 계약](config-v4.md#12-키-표). |
| `dashboard.terminate_timeout_secs` | 정수 | 필수, null 불가 | 종료 요청의 응답 대기 한도(초). 응답 때의 실행 중 `[timeouts] close_secs`(E)에 5초를 더한 값이다. E는 재로드 대상이다. |

폴링 주기 P와 `query_timeout_secs`는 재시작이 필요한 설정이므로 관리 세션이 살아 있는 동안 바뀌지 않는다. 화면은 로그인 직후와 새로고침 직후 세션 정보로 이 값을 받아 쓴다. 서버 요약에는 P를 넣지 않는다. E는 재로드 대상이므로 이후 세션 정보 응답의 `terminate_timeout_secs`는 새 E + 5초가 된다. 이미 시작한 종료 교환의 기한은 이전 E를 유지한다([설정 계약 1.8절](config-v4.md#18-재로드-구분)).

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
| `transport` | 문자열 | 필수, null 불가 | 대기 연결이 들어온 전송 경로. `tcp`, `ws`, `wss`. |
| `address` | `address` | 필수, null 불가 | 대기 연결의 실제 소켓 상대 주소 |
| `forwarded_ip` | 문자열 | 필수, null 허용 | 대기 연결이 신뢰 프록시를 거친 WS·WSS 연결이면 전달 주소다. TCP 연결이거나 전달 주소가 없으면 `null`이다. |

- 주소와 전달 주소의 의미·IP 표기·`null` 규칙은 1.8절의 `viewer_address`·`viewer_forwarded_ip`와 같다.

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
| `target_transport` | 문자열 | 필수, null 불가 | Server/Mode I 대상 전송 경로. `tcp`, `ws`, `wss`. Mode I은 항상 `tcp`이고, Mode II는 Server의 전송 경로다. |
| `started_at` | `timestamp` | 필수, null 불가 | 중계 시작 시각 |
| `bytes_viewer_to_target` | `bytes` | 필수, null 불가 | Viewer → Server/Mode I 대상 방향의 누적 바이트 |
| `bytes_target_to_viewer` | `bytes` | 필수, null 불가 | Server/Mode I 대상 → Viewer 방향의 누적 바이트 |
| `viewer_address` | `address` | 필수, null 불가 | Viewer 쪽 상대 주소 |
| `target_address` | `address` | 필수, null 불가 | Server(Mode II) 또는 Mode I 대상 쪽 상대 주소 |
| `viewer_forwarded_ip` | 문자열 | 필수, null 허용 | Viewer가 신뢰 프록시를 거친 경우의 전달 주소. 없으면 `null`이다. |
| `target_forwarded_ip` | 문자열 | 필수, null 허용 | Mode II Server가 신뢰 프록시를 거친 경우의 전달 주소. Mode I 또는 전달 주소가 없으면 `null`이다. |

- 누적 바이트는 한 세션 안에서 줄거나 순환하지 않는다. 재접속 세션은 새 `session_id`와 0에서 시작한다(MGT-007).
- `session_id`가 위 형식을 만족하지 않으면 `400 invalid_request`다(잘못된 대상).
- 형식은 맞지만 지금 활성 세션이 아니면 `404 not_found`다(대상 없음). 이미 정리가 끝난 세션과 처음부터 없던 세션을 구분하지 않는다. 같은 중계 ID의 다른 세션으로 대체하지 않는다.
- 상세는 `state`가 `closing`인 세션도 돌려준다.
- `viewer_address`·`target_address`는 프록시를 거쳐도 실제 소켓 상대 주소다. 전달 주소는 별도 필드에만 담는다. IP 표기는 1.1절의 정규화 규칙을 따른다. 전달 주소가 없는 경우는 확인 불가가 아니므로 `*_unavailable_reason`을 추가하지 않는다. 주소 결정은 [WebSocket 계약 1.8절](websocket-relay-v2.md#18-신뢰-프록시와-x-forwarded-for)을 따른다.

### 1.9 관리자 종료 `POST /api/v1/sessions/{session_id}/terminate`

요청: 관리 세션 쿠키와 `X-CSRF-Token`이 필요하다. 본문은 없다. 본문이 있으면 `Content-Type`과 관계없이 `400 invalid_request`다. 이 경로별 규칙은 1.1절의 415 규칙보다 우선한다. 서버는 지정한 `session_id`의 세션에만 영향을 준다.

서버는 그 세션의 신규 데이터 수락을 멈추고 양 끝점을 종료한다. 자원 정리가 끝난 뒤에 응답한다. 응답 시간은 최대 정리 제한 시간 E까지 길어질 수 있다. 검증·수락된 종료 요청이 완료를 기다리는 동안만 관리 연결의 U를 멈춰 연결을 유지한다. 응답 쓰기부터 U를 다시 잰다([설정 계약 1.2절](config-v4.md#12-키-표)의 관리 연결 U 예외). 정리 중에는 그 세션이 `counts.active`와 `GET /api/v1/sessions`에 `closing`으로 남는다.

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
- 서버는 종료 사건 로그에 수행한 관리자 계정명을 기록한다([운영 스펙](../specs/operations.md)의 OPS-011, 사건 형식은 [운영 정보 계약](ops-info-v2.md)의 `admin_terminate`). 계정명은 이 응답에 넣지 않는다.

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
| `items[].target_transport` | 문자열 | 필수, null 불가 | `tcp`, `ws`, `wss`. Mode I은 `tcp`, Mode II는 Server의 전송 경로다. |
| `items[].started_at` | `timestamp` | 필수, null 불가 | 중계 시작 시각 |
| `items[].ended_at` | `timestamp` | 필수, null 불가 | 정리 완료 시각. `started_at` 이상이다. |
| `items[].duration_ms` | `duration_ms` | 필수, null 불가 | 중계 시작부터 정리 완료까지의 경과 시간. 같은 세션의 운영 로그 `session_closed`의 `duration_ms`와 같다. 단조 시계 값이므로 `ended_at`과 `started_at`의 차이와 다를 수 있다. |
| `items[].bytes_viewer_to_target` | `bytes` | 필수, null 불가 | Viewer → Server/Mode I 대상 방향의 최종 누적 바이트 |
| `items[].bytes_target_to_viewer` | `bytes` | 필수, null 불가 | Server/Mode I 대상 → Viewer 방향의 최종 누적 바이트 |
| `items[].viewer_address` | `address` | 필수, null 불가 | 종료 전 Viewer 쪽 상대 주소. 의미는 1.8절과 같다. |
| `items[].target_address` | `address` | 필수, null 불가 | 종료 전 Server(Mode II) 또는 Mode I 대상 쪽 상대 주소. 의미는 1.8절과 같다. |
| `items[].viewer_forwarded_ip` | 문자열 | 필수, null 허용 | 종료 전 Viewer의 전달 주소. 없으면 `null`이다. 의미는 1.8절과 같다. |
| `items[].target_forwarded_ip` | 문자열 | 필수, null 허용 | 종료 전 Mode II Server의 전달 주소. Mode I 또는 전달 주소가 없으면 `null`이다. 의미는 1.8절과 같다. |
| `items[].close_cause` | 문자열 | 필수, null 불가 | 종료 원인. 아래 표를 따른다. |

종료 원인 `close_cause`의 값 목록은 [관리 스펙 2.3절](../specs/management.md#23-최근-종료-목록)을 따른다. 운영 로그·WebSocket close 사유 설명과 같은 이름을 쓴다. 한 세션의 `close_cause`는 정리를 시작하게 한 첫 원인이다. 강제 정리 시점에 이미 정리를 시작한 세션은 최초 원인을 유지한다. 기존 원인 없이 소유 작업이 끝나면 서버가 Running일 때는 `internal_error`, 정상 종료 중(Grace·Force)의 취소일 때는 `server_shutdown`을 쓴다.

| 값 | 의미 |
| --- | --- |
| `peer_eof` | 정상 EOF. 양쪽이 정상적으로 끝났다. |
| `connection_error` | 연결 오류. 중계 I/O 오류로 세션이 끝났다. |
| `admin_terminated` | 관리자 종료. 관리자 종료 요청으로 끝났다. |
| `dead_connection` | 끊긴 연결 감지. 응답하지 않는 상대를 TCP keepalive나 WebSocket ping으로 판정해 종료했다. |
| `close_timeout` | 정리 제한 시간 E 초과. E 안에 정리가 끝나지 않아 강제로 정리했다. |
| `server_shutdown` | 정상 종료 유예 G가 지나거나 유예 중 두 번째 종료 요청을 받아 강제로 정리했다. 정상 종료 중(Grace·Force)에 세션 소유 작업이 취소된 경우도 포함한다. 기존 종료 원인이 없는 세션에만 기록한다. Running 중 소유 작업이 버그로 끝난 경우는 포함하지 않는다. |
| `internal_error` | 내부 오류. 서버가 Running일 때 세션 소유 작업이 panic 등 버그로 끝났고 기존 종료 원인이 없었다. 기존 원인은 이 값으로 바꾸지 않는다. |
| `session_expired` | 접속 ticket의 세션 만료 시각(`sx`)이 되어 서버가 세션을 끝냈다([WebSocket 계약 1.9.5절](websocket-relay-v2.md#195-101-뒤의-중계-id-대조와-세션-만료)). |

- Consumer는 목록에 없는 값을 받으면 받은 그대로 표시하고 오류로 처리하지 않는다.
- 목록은 보관 개수 R을 넘으면 가장 오래된 항목부터 제거한다(MGT-024).
- 이 목록은 목록 정보만 제공한다. 상세 이력을 제공하지 않는다.

### 1.11 정적 대시보드 경로와 비인증 접근 범위

대시보드는 빌드 단계 없는 정적 HTML·JS이며 실행파일에 들어 있다. 관리 리스너가 제공한다. 인증 없이 접근할 수 있는 것은 로그인 화면(SCR-003)에 필요한 정적 파일뿐이다. 운영 데이터를 담은 응답은 없다.

| 메서드·경로 | 내용 | 관리 세션 | 없을 때 |
| --- | --- | --- | --- |
| `GET /login/` | 로그인 화면 HTML(SCR-003) | 불필요 | 해당 없음 |
| `GET /login/<파일>` | 로그인 화면이 쓰는 정적 파일 | 불필요 | 해당 없음 |
| `GET /` | 운영 화면 HTML(SCR-001·SCR-002·SCR-004·SCR-005·SCR-006) | 필요 | `303 See Other`, `Location: /login/` |
| `GET /app/<경로>` | 운영 화면이 쓰는 정적 파일. `components/`·`styles/` 등 하위 디렉터리 포함 | 필요 | `303 See Other`, `Location: /login/` |
| `/api/v1/` 아래 | 1.2절의 API | 경로별 | `401 unauthenticated` (로그인 제외) |
| 그 밖의 경로 | 없음 | 있으면 `404`, 없으면 `303`으로 `/login/` | 존재 여부를 노출하지 않는다 |

- 정적 경로는 `GET`(과 `HEAD`)만 받는다. 그 밖의 메서드는 `405`다.
- 보호된 정적 파일의 `303`은 JSON 오류 본문이 없다. API가 아니기 때문이다.
- 정적 경로의 `404`는 2.1절 형식의 JSON 본문과 `not_found`를 유지한다. 정적 `303`·`404`를 API와 나누는 예외는 경로 범위에 관한 것이다. `404`의 JSON 본문을 없애는 뜻이 아니다.
- 로그인 화면 파일에는 운영 데이터·계정 정보·설정 값을 넣지 않는다.
- 로그인 성공 뒤 화면은 `/`로 이동한다.
- 화면 URL: SCR-003은 `/login/`, SCR-001은 `/`, SCR-002는 `/#/sessions/{session_id}`, SCR-004는 `/#/settings`, SCR-005는 `/#/settings/edit`, SCR-006은 `/#/logs`, SCR-007은 `/#/metrics`다. URL의 조각(`#` 뒤)은 서버로 전송되지 않고 서버 접근 로그에 남지 않는다. 서버는 이 조각을 해석하지 않는다.
- 화면은 URL의 `session_id`가 1.8절 형식을 만족하는지 먼저 확인한다. 형식이 틀리면 요청을 보내지 않고 "잘못된 세션 식별자"를 표시한다. SCR-002에서 형식이 맞으면 상세를 조회하고, `404`이면 "대상 없음"을 표시한다. SCR-006의 세션 필터 URL은 `/#/logs?session_id={session_id}`다. 이 형식도 요청 전에 같은 형식 검사를 한다. 형식이 맞지만 일치하는 로그가 없으면 `200`의 빈 결과다.
- 보호된 정적 응답과 API 응답에 `Cache-Control: no-store`를 붙인다. 로그인 화면 정적 응답에도 같은 헤더를 붙인다. 서버를 갱신한 뒤 브라우저가 이전 버전의 화면 코드를 다시 쓰지 않게 하기 위해서다.
- 정적 응답에 `X-Content-Type-Options: nosniff`와 `Referrer-Policy: no-referrer`를 권장한다. `Content-Security-Policy: default-src 'self'; frame-ancestors 'none'`은 필수다. 대시보드는 인라인 스크립트와 인라인 스타일(`<style>`, `style` 속성)을 쓰지 않는다.

### 1.12 폴링 주기와 조회 제한 시간의 전달

기본값과 유효 범위는 [설정 계약 1.2절](config-v4.md#12-키-표)이 정한다. 이 표는 전달 방법만 정한다.

| 값 | 설정 키(설정 계약) | 전달 방법 |
| --- | --- | --- |
| P | `[dashboard] poll_secs`(재시작 필요) | 세션 정보의 `dashboard.poll_secs` |
| 조회 제한 시간·설정 편집 응답 대기 한도 | `[dashboard] query_timeout_secs`(재시작 필요) | 세션 정보의 `dashboard.query_timeout_secs` |
| 종료 응답 대기 한도 | 설정 키 없음. 재로드 대상 `[timeouts] close_secs`(E)에서 계산 | 세션 정보의 `dashboard.terminate_timeout_secs`. 응답 때의 E + 5초다. |

- 폴링 조회는 `GET` 일곱 경로(`summary`·`waiting`·`sessions`·`sessions/{session_id}`·`closed-sessions`·`logs`·`metrics`)에 붙인 자동 요청이다. 응답이 `query_timeout_secs` 안에 오지 않으면 Consumer는 요청을 중단하고 조회 실패로 처리한다. 실패를 빈 목록으로 표시하지 않는다.
- 같은 조회가 진행 중이면 다음 주기 요청을 겹쳐 보내지 않는다([화면 4.1절](../specs/dashboard.md#41-폴링)). 조회 제한 시간이 P보다 길어도 된다.
- 종료 요청에는 폴링 조회 제한 시간을 쓰지 않는다. 정리가 최대 E까지 걸릴 수 있기 때문이다. `terminate_timeout_secs`를 쓴다.
- E를 교체한 뒤 세션 정보를 응답하면 새 E + 5초를 전달한다. 이미 시작한 종료 교환의 기한은 바꾸지 않는다. P와 `query_timeout_secs`는 재시작 필요로 남는다.
- `GET /api/v1/config`는 사용자 행동 조회이며 자동 폴링하지 않는다. `PATCH /api/v1/config`도 사용자 행동 요청이다. 두 요청은 성공한 사용자 행동의 유휴 만료 연장 규칙을 따른다. 응답 대기 한도는 2.3절을 따른다.
- 서버는 일반 조회를 제한 시간 안에 응답하도록 만든다. 서버 내부 처리 제한은 이 계약이 정하지 않는다.

### 1.13 설정 보기 `GET /api/v1/config`

유효한 관리 세션이 필요하다. CSRF 토큰은 필요하지 않다. `management.config_edit`가 꺼져 있어도 조회할 수 있다. 서버는 요청 때 `serve`가 읽는 설정 파일을 다시 읽어 실행 중 유효 설정과 비교한다. 파일을 읽지 못하거나 검증에 실패해도 실행 중 값은 조회할 수 있으므로 `200`으로 파일 상태를 반환한다.

| 필드·메시지 | 자료형 | 필수 여부·null 허용 여부 | 의미·제약 |
| --- | --- | --- | --- |
| `as_of` | `timestamp` | 필수, null 불가 | 조회 기준 시점 |
| `revision` | 문자열 | 필수, null 허용 | 지금 설정 파일 바이트의 SHA-256. `sha256:` 뒤 소문자 16진수 64자다. 바이트 읽기 실패일 때만 `null`이다. UTF-8이 아니어도 바이트를 읽었으면 그 바이트의 리비전을 반환한다. |
| `edit_enabled` | 불리언 | 필수, null 불가 | 실행 중 `management.config_edit` 값 |
| `file` | 객체 | 필수, null 불가 | 요청 때 다시 읽은 파일의 상태 |
| `file.state` | 문자열 | 필수, null 불가 | 아래 표의 5가지 상태 |
| `file.config_keys` | 문자열 배열 | 필수, null 불가 | `pending`이면 다른 재로드 대상 키, `restart_required`면 다른 재시작 필요 키. 그 밖에는 빈 배열이다. 값은 넣지 않는다. |
| `file.diagnostics` | 진단 객체 배열 | 필수, null 불가 | `invalid`일 때 설정 계약 2.1절의 `code`·`key`·`message`. 그 밖에는 빈 배열이다. |
| `last_reload` | 객체 | 필수, null 허용 | 이 프로세스 시작 뒤 마지막 재로드 결과. 파일 변경 감지의 `applied`·`rejected`·`restart_required`와 파일·실행 중 값 교체를 마친 PATCH의 `applied`만 기록한다. 없으면 `null`이다. |
| `last_reload.at` | `timestamp` | 객체가 있으면 필수, null 불가 | 마지막 재로드 결과 시각 |
| `last_reload.result` | 문자열 | 객체가 있으면 필수, null 불가 | `applied`·`rejected`·`restart_required` |
| `last_reload.trigger` | 문자열 | 객체가 있으면 필수, null 불가 | `file_watch`·`config_edit` |
| `last_reload.config_keys` | 문자열 배열 | 객체가 있으면 필수, null 불가 | `applied`면 바뀐 키, `restart_required`면 다른 키, `rejected`면 빈 배열 |
| `items` | 객체 배열 | 필수, null 불가 | 기본값을 포함한 실행 중 유효 설정의 모든 키. 순서는 [설정 계약 1.2절](config-v4.md#12-키-표)의 표 순서다. |
| `items[].key` | 문자열 | 필수, null 불가 | 테이블과 키를 점으로 이은 설정 키 |
| `items[].value` | 불리언·정수·문자열·문자열 배열 | 필수, null 허용 | 설정 자료형 그대로다. 꺼진 진입점의 생략된 키는 `null`이다. 경로는 설정 파일 기준으로 푼 값이다. |
| `items[].reload` | 문자열 | 필수, null 불가 | `dynamic`(재로드 대상)·`restart`(재시작 필요). [설정 계약 1.8절](config-v4.md#18-재로드-구분)을 따른다. |
| `items[].editable` | 불리언 | 필수, null 불가 | 편집 대상이고 해당 진입점·모드가 켜져 있으면 `true`. `edit_enabled`와는 별개다. |
| `items[].min`·`items[].max` | 정수 | 정수 키에 필수, null 불가. 그 밖의 키에는 없음 | 설정 계약의 정적 범위. Lw의 `max`는 65535이며 Lw ≤ L 관계 검사는 따로 한다. |

`items`에는 접속 ticket 키 10개도 포함한다. Viewer는 `relay.ticket.viewer.enabled`·`relay.ticket.viewer.issuer`·`relay.ticket.viewer.relay_host`·`relay.ticket.viewer.public_keys_file`·`relay.ticket.viewer.max_lifetime_secs`다. Server는 `relay.ticket.server.enabled`·`relay.ticket.server.issuer`·`relay.ticket.server.relay_host`·`relay.ticket.server.public_keys_file`·`relay.ticket.server.max_lifetime_secs`다. 모두 `reload = dynamic`이다. `public_keys_file` 2개는 항상 `editable = false`다. 나머지 8개는 그 역할이 WS 또는 WSS 리스너에서 하나라도 켜져 있을 때만 `editable = true`다. `enabled`의 기본값은 `false`다. `max_lifetime_secs`의 기본값은 60이며 `min = 10`, `max = 600`이다([설정 계약 1.2절](config-v4.md#12-키-표)).

| `file.state` | 뜻 |
| --- | --- |
| `in_effect` | 파일과 실행 중 유효 설정에 차이가 없다. |
| `pending` | 재로드 대상 키만 다르다. 파일 변경 감지가 적용할 후보가 남아 있다. |
| `restart_required` | 재시작 필요 키에 차이가 남아 있다. |
| `invalid` | 파일을 읽었으나 설정 검증에 실패했다. |
| `unreadable` | 파일 바이트를 읽지 못했거나 UTF-8이 아니다. `diagnostics`는 빈 배열이다. 바이트 읽기에 성공한 비UTF-8 파일의 `revision`은 `null`이 아니다. |

`last_reload`는 GET, 같은 후보를 다시 읽은 파일 변경 감지, 변경 없음·폐기된 파일 변경 감지 결과로 갱신하지 않는다. PATCH의 인증·본문·편집 꺼짐·충돌·파일 상태·후보 검증·쓰기 실패와 no-op도 이력을 갱신하지 않는다. `trigger = config_edit`와 `result = rejected`·`restart_required`의 조합은 스키마에서 허용하지만 현재 서버는 생성하지 않는다.

설정 파일에는 비밀이 없다([설정 계약 1.1절](config-v4.md#11-파일-규칙)). 설정 경로는 보여 주되 경로가 가리키는 파일 내용은 넣지 않는다. 인증서·개인키·계정 파일의 내용도 노출하지 않는다. 설정 보기에서 파일의 미적용 값은 반환하지 않는다.

### 1.14 동적 설정 편집 `PATCH /api/v1/config`

유효한 관리 세션과 CSRF 토큰이 필요하다. 로그인한 모든 관리자가 같은 권한으로 편집한다. `management.config_edit`는 기본 `false`이며 이를 켜려면 설정 파일을 고치고 재시작해야 한다.

요청 본문:

| 필드·메시지 | 자료형 | 필수 여부·null 허용 여부 | 의미·제약 |
| --- | --- | --- | --- |
| 헤더 `Content-Type` | 문자열 | 필수 | `application/json` |
| 헤더 `X-CSRF-Token` | 문자열 | 필수 | 1.4.3절의 관리 세션 토큰 |
| `revision` | 문자열 | 필수, null 불가 | 조회한 파일의 리비전. `sha256:` 뒤 소문자 16진수 64자다. |
| `changes` | 객체 | 필수, null 불가 | 바꿀 설정 키를 속성 이름으로, 새 값을 속성 값으로 담는다. 빈 객체는 거절한다. |
| `changes.<설정 키>` | 불리언·정수·문자열·문자열 배열 | 하나 이상 필수, null 불가 | 해당 설정 자료형과 맞아야 한다. JSON 자료형 오류는 `invalid_request`, 설정 범위·항목 문법·관계 오류는 `config_invalid`다. |

PATCH 본문은 64 KiB(65536바이트) 이하여야 한다. 초과하면 `400 invalid_request`이며 파일·실행 중 값을 바꾸거나 편집 사건을 기록하지 않는다. 로그인의 기존 16 KiB 상한은 그대로다.

JSON 정수는 `i64`나 `u64`로 읽을 수 있으면 정수로 받아 7단계에서 설정 범위를 검사한다. 범위를 벗어나면 `config_invalid`다. 이보다 큰 정수 표기는 JSON 수 해석 한계 때문에 지수 표기 실수와 구분하지 않으므로 1단계에서 `invalid_request`로 거절한다.

편집 대상은 출발지 IP 허용 목록 6개, WS·WSS 역할별 Origin 허용 목록 4개, 리스너별 `trusted_proxies` 2개, `mode1.allow_targets`, `[limits]` 5개, `[timeouts]` 6개, `[keepalive]` 5개와 접속 ticket 8개인 총 37개 키다. ticket 편집 대상은 `relay.ticket.viewer.enabled`·`relay.ticket.viewer.issuer`·`relay.ticket.viewer.relay_host`·`relay.ticket.viewer.max_lifetime_secs`·`relay.ticket.server.enabled`·`relay.ticket.server.issuer`·`relay.ticket.server.relay_host`·`relay.ticket.server.max_lifetime_secs`다. 목록과 적용 시점은 [설정 계약 1.8절](config-v4.md#18-재로드-구분)이 기준이다. 인증서·개인키 경로 4개는 편집 대상이 아니다. 접속 ticket 공개 키 파일 경로 2개도 편집할 수 없다. 이 경로 6개를 포함한 재로드 대상은 총 43개다. 공개 키 교체는 같은 경로의 파일 내용을 바꾸는 방식을 따른다([설정 계약 1.11절](config-v4.md#111-접속-ticket-공개-키-파일-형식)). 역할별 키는 그 역할이 켜져 있을 때만, `trusted_proxies`는 해당 리스너의 역할 중 하나 이상이 켜져 있을 때만 편집할 수 있다. ticket 키는 그 역할이 WS 또는 WSS 리스너에서 하나라도 켜져 있을 때만 편집할 수 있다. `mode1.allow_targets`는 `mode1.enabled = true`일 때만 편집할 수 있다. `management.config_edit`와 `recent_logs.max_entries`를 비롯한 재시작 필요 키는 편집할 수 없다.

서버는 재로드 서비스와 직렬화하여 다음 10단계를 순서대로 처리한다. 앞 단계에서 거절하면 뒤 단계는 하지 않는다.

1. 인증·CSRF·본문 형식을 공통 순서(1.3절)로 검사한다. 빈 `changes`, 편집 대상이 아니거나 지금 `editable`이 아닌 키, 맞지 않는 JSON 자료형은 `400 invalid_request`다. `PATCH`에 `X-Relay-Poll`이 있으면 `400 invalid_request`다.
2. 실행 중 `management.config_edit`가 `false`이면 `403 config_edit_disabled`로 거절한다.
3. 설정 파일 바이트를 읽고 UTF-8인지 확인한다. 바이트를 읽지 못했거나 UTF-8이 아니면 `409 config_file_invalid`다. `diagnostics`는 빈 배열이며 조회의 `file.state = unreadable`에 대응한다. 비UTF-8도 이 단계에서 거절하므로 리비전 비교보다 먼저다.
4. 요청 `revision`이 지금 파일 바이트의 리비전과 다르면 `409 config_conflict`로 거절한다.
5. 지금 파일이 검증에 실패하면 `409 config_file_invalid`와 `diagnostics`를 반환한다. 재시작 필요 키의 차이가 남아 있으면 `409 config_restart_pending`과 그 키의 `config_keys`를 반환한다.
6. `toml_edit`로 요청 `changes` 중 파일값이 바뀐 키의 값만 고친다. 나머지 주석·공백·키 순서·다른 값은 보존한다. 없는 키는 기존 테이블에 넣는다. 테이블도 없으면 파일 끝에 새 테이블로 더한다. 새 줄은 파일의 기존 줄바꿈 방식을 따른다. 지금 파일이 `pending`이면 그 미적용 변경에 이번 편집을 얹은 후보 전체를 만든다.
7. 고친 후보 전체를 시작·재로드와 같은 규칙으로 검증한다. 실패하면 `400 config_invalid`와 `diagnostics`를 반환한다. 진단의 `key`는 `relay.tcp_viewer.allow[2]`처럼 첨자를 포함할 수 있다. ticket의 `enabled = true`로 바꾸는 후보에 그 역할의 `issuer`·`relay_host`·`public_keys_file`이 빠져 있으면 `400 config_invalid`와 `config_missing_key` 진단을 반환한다([설정 계약 1.7절](config-v4.md#17-시작-때-필수-키)). 파일은 바꾸지 않는다.
8. 편집 전후 파일 바이트가 같으면 쓰기·교체·실행 중 값 적용 없이 `200`으로 응답한다. `applied`는 빈 배열이며 `revision`은 그대로다. `reload_applied`·`config_edited`를 기록하거나 `last_reload`를 갱신하지 않는다. 파일에 이미 남아 있던 `pending` 변경은 파일 변경 감지가 적용한다.
9. 공유 잠금 안에서 같은 디렉터리의 임시 파일을 쓰고 flush한다. 원자 교체 바로 전에 현재 파일을 다시 읽어 `revision`을 비교한다. 다르면 임시 파일을 지우고 `409 config_conflict`로 거절하며 이번 요청은 파일·실행 중 값을 바꾸지 않는다. 같으면 원자 교체한 뒤 같은 후보를 적용한다. 마지막 재확인과 교체 사이의 아주 짧은 틈은 보장 범위가 아니다. 재확인 읽기 실패도 임시 파일을 지우고 `409 config_conflict`로 거절하며 `config_edit_rejected.reason = config_conflict`를 기록한다. 쓰기·교체 자체에 실패하면 `500 config_write_failed`이며 파일과 실행 중 값은 이전 그대로다.
10. 같은 후보의 재로드 대상 값을 한 번에 교체한다. `pending`에 있던 변경도 함께 적용한다. 응답과 `reload_applied`의 `applied`는 실제 실행 중 값이 바뀐 모든 키다. `config_edited.config_keys`는 요청 `changes` 중 파일값이 바뀐 키만이다. 실행 중 값이 바뀌면 `reload_applied`(`trigger = config_edit`)를 기록하고 `last_reload`를 갱신한다. 그 뒤 `config_edited`를 기록한다([운영 정보 계약 1.5절](ops-info-v2.md#15-사건-목록)). 실행 중 값의 차이가 없으면 `reload_applied`와 이력 갱신 없이 `config_edited`만 기록한다. 이후 파일 변경 감지가 같은 내용을 읽으면 다시 교체하거나 성공 사건·이력을 추가하지 않는다. `reload_change_detected`는 기록될 수 있다.

성공 응답 `200`:

| 필드·메시지 | 자료형 | 필수 여부·null 허용 여부 | 의미·제약 |
| --- | --- | --- | --- |
| `as_of` | `timestamp` | 필수, null 불가 | 처리 결과 시점 |
| `result` | 문자열 | 필수, null 불가 | `applied` |
| `revision` | 문자열 | 필수, null 불가 | 처리 뒤 파일 바이트의 리비전. `sha256:` 뒤 소문자 16진수 64자다. |
| `applied` | 문자열 배열 | 필수, null 불가 | 후보 전체에서 실제 실행 중 값이 바뀐 모든 키. 파일에 이미 남아 있던 `pending` 변경도 포함한다. 요청 `changes` 중 파일값이 바뀐 키만 담는 `config_edited.config_keys`와 구분한다. 편집 전후 파일 바이트가 같으면 빈 배열이다. |

편집은 파일에 남아 재시작 뒤에도 유지된다. 이미 맺은 연결·시작한 타이머·설정한 소켓 옵션은 이전 값을 유지한다. 바뀐 값은 [설정 계약 1.8절](config-v4.md#18-재로드-구분)의 시점부터 적용한다. 정상 종료 유예 중에도 다른 관리 요청처럼 처리한다. 특별히 거절하지 않는다.

ticket 설정 변경은 새 업그레이드 요청부터 적용한다. 이미 101을 보낸 연결의 중계 ID 대조와 `sx` 타이머, 사용 기록은 유지한다([WebSocket 계약 1.9.6절](websocket-relay-v2.md#196-설정과-적용-시점)).

인증·CSRF를 통과한 편집 거절은 `invalid_request`를 제외하고 `config_edit_rejected`로 기록한다. PATCH 거절은 `last_reload`를 갱신하지 않는다. 편집 사건의 `admin_account`는 필수다. 키와 리비전 등 사건별 메타 정보는 기록하되 설정값(IP·Origin·대상 항목·수치)은 기록하지 않는다. 필드·사건 형식은 [운영 정보 계약 1.5절](ops-info-v2.md#15-사건-목록)과 [1.8절](ops-info-v2.md#18-관리자-계정명-기록-규칙)을 따른다.

### 1.15 최근 로그 `GET /api/v1/logs`

유효한 관리 세션이 필요하다. CSRF 토큰은 필요하지 않다. `X-Relay-Poll: 1`을 허용하며, 이 헤더가 있는 자동 갱신은 관리 세션의 유휴 만료를 연장하지 않는다. 헤더가 없으면 사용자 행동 요청이다(1.4.2절).

서버는 메모리에 보관한 최근 운영 로그를 조회한다. 보관 수명과 출력 대상과의 관계는 [운영 스펙](../specs/operations.md)이 정한다. 각 `entry`는 [운영 정보 계약](ops-info-v2.md)의 JSON 로그 한 줄을 해석한 객체와 같아야 한다. 비밀을 담지 않으며 중계 ID는 원문 대신 `relay_id_ref`로만 나온다.

모든 쿼리 매개변수는 선택이다. 여러 종류의 필터는 AND로 결합한다. `level`·`event` 목록 안에서는 OR로 결합한다.

| 매개변수 | 형식·범위 | 의미 |
| --- | --- | --- |
| `level` | `info,warn,error` 중 하나 이상의 값을 쉼표로 구분 | 지정한 레벨만 조회한다. 모르는 값이나 빈 항목은 `400 invalid_request`다. 생략하면 모든 레벨이다. |
| `event` | `[a-z0-9_]{1,64}`를 만족하는 사건 이름 하나 이상을 쉼표로 구분 | 대소문자를 구분해 정확히 일치시킨다. 형식에 맞는 모르는 이름은 오류가 아니며 아무 사건과도 일치하지 않는다. 빈 항목은 `400 invalid_request`다. 생략하면 모든 사건이다. |
| `q` | 문자열, 1~256 유니코드 스칼라 값 | `entry` 안의 모든 필드 값에서 부분 문자열을 찾는다. UTF-8 바이트 수로 길이를 세지 않는다. ASCII 영문만 대소문자를 구분하지 않고 그 밖의 문자는 그대로 비교한다. 검색용 텍스트 변환은 아래 규칙을 따른다. |
| `session_id` | 1.8절의 세션 식별자 형식 | 사건의 `session_id`가 이 값과 같은 항목을 찾는다. 해당 세션의 `session_started`가 아직 보관되어 있으면, 그 사건의 `viewer_conn_id`·`server_conn_id`와 `conn_id`가 같은 사건도 포함한다. `session_started`가 없으면 연결 식별자 관계를 복구하지 않는다. |
| `before` | 부호 없는 10진 숫자. `u64` 범위의 정수, 1 이상 | 이 순번보다 작은 항목을 조회한다. `after`와 함께 쓰면 `400 invalid_request`다. |
| `after` | 부호 없는 10진 숫자. `u64` 범위의 정수, 0 이상 | 이 순번보다 큰 항목을 조회한다. `before`와 함께 쓰면 `400 invalid_request`다. |
| `limit` | 정수, 1~1000. 기본 200 | 반환할 최대 항목 수다. |

문법·자료형·범위가 틀린 매개변수와 빈 `q`는 `400 invalid_request`다. 필터 조건은 보관된 사건에만 적용한다. 형식에 맞는 세션 식별자가 현재 활성 세션이 아니어도 오류가 아니다. 일치하는 사건이 없으면 성공한 빈 결과다.

검색과 커서 표현의 세부 규칙은 다음과 같다.

- `q`는 `entry`의 문자열 값을 그대로 비교한다. 수는 JSON 로그 줄에 쓰인 10진 표기로 비교한다. 불리언은 `true`·`false`, `null`은 `null`로 비교한다. 배열 원소와 중첩 객체의 값도 같은 규칙을 따른다. 필드 이름과 응답의 조회용 `seq`는 검색하지 않는다.
- `before`·`after`는 `[0-9]+` 표기만 받는다. 부호·공백·소수점·지수 표기는 거절한다. 64비트 부호 없는 정수의 최댓값을 넘거나 각 하한을 벗어나면 `400 invalid_request`다. 별도의 업무 상한은 없다.
- 쿼리 문자열은 `application/x-www-form-urlencoded` 규칙으로 해석한다. `+`는 공백이고, `%XX`는 UTF-8 바이트다. 잘못된 퍼센트 인코딩과 UTF-8이 아닌 값은 `400 invalid_request`다. `=`가 없는 항목은 값이 빈 문자열인 매개변수로 본다. 그래서 위 표의 매개변수이면 빈 값 규칙에 따라 `400 invalid_request`다.
- `level`·`event`를 여러 번 보내면 모든 값을 한 목록으로 합친다. `q`·`session_id`·`before`·`after`·`limit`를 두 번 이상 보내면 `400 invalid_request`다. 어느 값을 쓸지 서버가 추측하지 않기 위해서다.
- 위 표에 없는 매개변수 이름은 무시한다. [3절](#3-호환성-규칙)의 Producer 규칙(요청의 알 수 없는 필드 무시)을 따른다. 그래서 이후 호환 버전이 선택 매개변수를 더해도 이전 서버가 요청을 거절하지 않는다.

페이지 선택과 정렬:

| 커서 | 선택 | `has_more`가 참인 조건 |
| --- | --- | --- |
| 없음 | 일치하는 항목 중 가장 최근 `limit`개 | 선택한 항목보다 오래된 일치 항목이 더 있다. |
| `before=B` | `seq < B`인 일치 항목 중 가장 최근 `limit`개 | 이번 페이지보다 오래되고 `seq < B`인 일치 항목이 더 있다. |
| `after=A` | `seq > A`인 일치 항목 중 가장 오래된 `limit`개 | 이번 페이지보다 새롭고 `seq > A`인 일치 항목이 더 있다. |

`items`는 항상 `seq` 오름차순이다. 커서가 현재 보관 범위 밖이어도 위 부등식만 적용한다. 응답 하나의 항목·경계·`has_more`는 같은 `as_of`의 보관 목록에서 계산한다. 서로 다른 페이지 요청 사이의 추가·제거를 막거나 이미 제거한 항목을 복구하지 않는다.

`200` 응답 본문:

| 필드 | 자료형 | 필수 여부·null 허용 여부 | 의미 |
| --- | --- | --- | --- |
| `as_of` | `timestamp` | 필수, null 불가 | 조회 기준 시점. 1.1절 형식이다. |
| `max_entries` | 정수 | 필수, null 불가 | 실행 중 `recent_logs.max_entries`. 기본 1000, 범위 0~10000이다. |
| `oldest_seq` | 정수 | 필수, null 허용 | 필터·커서를 적용하기 전 보관 목록의 가장 오래된 순번. 보관 목록이 비었으면 `null`이다. |
| `latest_seq` | 정수 | 필수, null 허용 | 필터·커서를 적용하기 전 보관 목록의 가장 최근 순번. 보관 목록이 비었으면 `null`이다. |
| `has_more` | 불리언 | 필수, null 불가 | 위 표의 요청 방향으로 일치하는 항목이 더 있는지 나타낸다. |
| `items` | 객체 배열 | 필수, null 불가 | 선택한 항목. 일치 항목이 없으면 빈 배열이다. |
| `items[].seq` | 정수 | 항목마다 필수, null 불가 | 1 이상인 기록 순번. 아래 규칙을 따른다. |
| `items[].entry` | 객체 | 항목마다 필수, null 불가 | 운영 로그 한 줄의 객체. 필드·값을 바꾸거나 검색용 요약으로 대신하지 않는다. |

`seq`는 프로세스의 첫 보관 사건에서 1로 시작하고 기록할 때마다 1씩 증가한다. 가장 오래된 항목을 제거해도 순번을 다시 쓰지 않는다. 재시작하면 보관 목록을 비우고 순번을 다시 시작한다. `seq`는 사건 객체 밖의 조회용 값이며 운영 로그 한 줄에 추가하지 않는다. `max_entries = 0`이면 아무 사건도 보관하지 않는다. 이때 `items`는 빈 배열, `oldest_seq`·`latest_seq`는 `null`, `has_more`는 `false`다. 필터 결과만 비었을 때는 보관 목록의 경계가 `null`로 바뀌지 않는다.

오류는 `400 invalid_request`(매개변수·폴링 헤더 오류), `401 unauthenticated`, `405 invalid_request`(`Allow: GET`), `500 internal`이다. 인증·메서드·본문·Content-Type의 공통 규칙(1.1·1.3절)도 적용하므로 본문이 있고 Content-Type이 틀리면 `415 invalid_request`다. 조회 실패를 빈 성공 목록으로 바꾸지 않는다.

JSON Lines 다운로드는 Consumer가 만든다. 같은 필터로 커서 없이 `limit=1000`을 조회한 뒤 `before`와 `limit=1000`으로 과거 페이지를 수집한다. 각 다음 요청의 `before`는 직전 페이지의 가장 작은 `seq`다. `has_more = false`이면 끝낸다. 각 `entry`를 UTF-8·LF의 JSON 한 줄로 기록한다. 조회용 `seq`와 응답 포장 필드는 파일에 넣지 않는다. 파일 이름은 `vnc-relay-logs-<UTC yyyymmddThhmmssZ>.jsonl`이다. 다운로드 전용 API 경로는 추가하지 않는다. 첫 페이지 이후 생긴 새 사건은 포함하지 않는다. 페이지 요청 사이에 제거된 사건까지 완전한 이력으로 보장하지 않는다.

### 1.16 메트릭 `GET /api/v1/metrics`

유효한 관리 세션이 필요하다. CSRF 토큰은 필요하지 않다. `X-Relay-Poll: 1`을 허용하며, 이 헤더가 있는 자동 갱신은 관리 세션의 유휴 만료를 연장하지 않는다. 헤더가 없으면 사용자 행동 요청이다(1.4.2절). `[metrics] enabled`와 관계없이 응답한다. 이 설정은 상태 리스너의 `/metrics`만 켜고 끈다.

`200` 응답 본문은 [메트릭 계약 1.7절](metrics-v1.md#17-json-표현)의 JSON이다. `history`는 [메트릭 계약 1.8절](metrics-v1.md#18-추이)을 따른다. 값은 같은 시점의 상태 리스너 `/metrics`와 같다. 이 응답에는 `as_of` 외에 1.6절 서버 요약의 필드를 넣지 않는다.

| 매개변수 | 형식·범위 | 의미 |
| --- | --- | --- |
| `history_window` | `1h` 또는 `24h`. 대소문자를 구분한다 | 받을 추이 기간. `1h`는 최근 1시간(10초 간격, 최대 360개), `24h`는 최근 24시간(1분 간격, 최대 1440개)이다([메트릭 계약 1.8절](metrics-v1.md#18-추이)). 생략하면 `1h`다. |
| `history_after` | 부호 없는 10진 숫자. `u64` 범위의 정수, 0 이상 | `history.points`에 고른 기간에서 `seq`가 이 값보다 큰 점만 넣는다. 생략하면 그 기간에 보관 중인 모든 점을 넣는다. `metrics`와 `history`의 다른 필드는 이 값과 관계없다. |

- `history_window`는 표의 두 값만 받는다. 다른 값과 빈 값은 `400 invalid_request`다.
- `history_after`는 1.15절의 `after`와 같은 표기 규칙을 따른다. `[0-9]+` 표기만 받는다. 부호·공백·소수점·지수 표기, `u64` 최댓값을 넘는 값, 빈 값은 `400 invalid_request`다.
- `history_window`나 `history_after`를 두 번 이상 보내면 `400 invalid_request`다. 쿼리 문자열 해석과 표에 없는 매개변수 무시는 1.15절 규칙과 같다.
- `history_after`는 고른 기간의 `seq`와 비교한다. 두 기간은 순번을 따로 센다. 그래서 기간을 바꾼 Consumer는 `history_after` 없이 처음부터 조회한다.
- `history_after`가 `latest_seq` 이상이면 `history.points`는 빈 배열이다. 오류가 아니다. 보관 범위보다 오래된 값이면 보관 중인 모든 점을 준다.
- 재시작하면 두 기간의 추이 순번이 0부터 다시 시작한다. 재시작 전 관리 세션은 모두 무효이므로 Consumer는 다시 로그인한 뒤 `history_after` 없이 처음부터 조회한다.

오류는 `400 invalid_request`(매개변수·폴링 헤더 오류), `401 unauthenticated`, `405 invalid_request`(`Allow: GET`), `500 internal`이다. 인증·메서드·본문·Content-Type의 공통 규칙(1.1·1.3절)도 적용하므로 본문이 있고 Content-Type이 틀리면 `415 invalid_request`다. 프로세스 값을 읽지 못한 것은 오류가 아니다. `200`의 `available = false`로 구분한다.

## 2. 오류

### 2.1 오류 본문과 상태 코드

모든 `/api/v1/` 오류 응답 본문은 다음 형식이다. 정적 파일의 `303`·`404`는 API 경로 범위 밖의 예외다(1.11절). `303`에는 JSON 오류 본문이 없다. `404`는 아래 형식의 JSON 본문과 `not_found`를 유지한다.

```json
{"error": {"code": "<snake_case 코드>", "message": "<사람이 읽는 문장>"}}
```

- Consumer는 `code`와 HTTP 상태 코드로 판정한다. `message`로 판정하지 않는다. `message`는 화면에 그대로 보여줄 수 있는 한국어 한 문장이며 비밀과 요청 입력값을 담지 않는다.
- 새 오류 코드는 호환 변경이다. Consumer는 모르는 코드를 HTTP 상태 코드 범주로 처리한다(3절).
- 모든 `401` 응답(`unauthenticated`·`login_failed`)에 헤더 `WWW-Authenticate: Session realm="vnc-relay"`를 붙인다. RFC 9110이 `401` 응답에 이 헤더를 요구하기 때문이다. `Session`은 관리 세션 쿠키 인증을 뜻하는 이 계약의 고유 스킴 이름이다. 표준 인증 방식(`Basic`·`Digest` 등)이 아니다. 브라우저가 이 헤더 때문에 기본 인증 창을 띄우지 않는지는 5절의 브라우저 관찰과 6절의 미확인 범위로 구분한다.

`error` 객체의 필드:

| 필드·메시지 | 자료형 | 필수 여부·null 허용 여부 | 의미·제약 |
| --- | --- | --- | --- |
| `code` | 문자열 | 필수, null 불가 | 아래 표의 오류 코드 |
| `message` | 문자열 | 필수, null 불가 | 입력값·비밀을 담지 않는 한국어 한 문장. Consumer 판정에는 쓰지 않는다. |
| `diagnostics` | 객체 배열 | 선택, 있으면 null 불가 | `config_invalid`·`config_file_invalid`에 필수. 읽기 실패면 빈 배열이다. |
| `diagnostics[].code` | 문자열 | 진단마다 필수, null 불가 | [설정 계약 2.1절](config-v4.md#21-설정-오류와-준비-실패)의 안정적인 진단 코드 |
| `diagnostics[].key` | 문자열 | 진단마다 필수, null 불가 | 점으로 이은 키. 배열은 0부터 센 `[i]` 첨자를 포함할 수 있다. |
| `diagnostics[].message` | 문자열 | 진단마다 필수, null 불가 | 값을 인용하지 않는 진단 설명. 비밀·파일 내용도 넣지 않는다. |
| `config_keys` | 문자열 배열 | 선택, 있으면 null 불가 | `config_restart_pending`에 필수. 재시작 필요 차이가 남은 키. 값은 넣지 않는다. |

| 오류 코드 | 의미·형식 | 관련 스펙 시나리오 |
| --- | --- | --- |
| `unauthenticated` | `401`. 유효한 관리 세션이 없다. 쿠키 없음·잘못된 쿠키·유휴 만료·절대 만료·로그아웃됨·재시작 전 발급을 구분하지 않는다. 쿠키가 있었으면 지우는 `Set-Cookie`를 보낸다. 상태를 바꾸지 않는다. 경로·메서드가 틀려도 인증이 먼저다. | [관리: MGT-008·MGT-019·MGT-020·MGT-021·MGT-025·MGT-027](../specs/management.md#7-수용-테스트), [화면: DASH-008·DASH-014](../specs/dashboard.md#7-수용-테스트), [관리 4절](../specs/management.md#4-오류와-복구) "관리 세션 없음"·"관리 세션 유휴·절대 수명 만료" |
| `csrf_invalid` | `403`. 관리 세션은 유효하지만 상태 변경 메서드 `POST`·`PATCH`·`DELETE`의 `X-CSRF-Token`이 없거나 틀리다(로그인 제외). 관리 세션을 유지하고 대상의 상태를 바꾸지 않는다. | [관리: MGT-014·MGT-032](../specs/management.md#7-수용-테스트), [관리 4절](../specs/management.md#4-오류와-복구) "관리 세션·CSRF 검증 실패" |
| `login_failed` | `401`. 로그인 실패. 계정 없음·비밀번호 틀림·로그인 제한 중이 모두 같은 상태·코드·본문이다. 세션을 발급하지 않는다. 계정 존재 여부를 알 수 있는 정보를 담지 않는다. | [관리: MGT-018·MGT-022](../specs/management.md#7-수용-테스트), [화면: DASH-018](../specs/dashboard.md#7-수용-테스트) |
| `not_found` | `404`. 대상 없음. 형식은 맞지만 지금 활성 세션이 아닌 `session_id`(정리 완료·최근 종료 목록에 있음·처음부터 없음), 또는 알 수 없는 `/api/v1/` 경로. 다른 세션으로 대체하지 않는다. | [관리: MGT-010·MGT-011·MGT-023](../specs/management.md#7-수용-테스트), [화면: DASH-007·DASH-011](../specs/dashboard.md#7-수용-테스트), [관리 4절](../specs/management.md#4-오류와-복구) "상세 대상 소멸"·"종료 중 대상 소멸" |
| `invalid_request` | `400`(본문·필드·`session_id` 형식·`X-Relay-Poll` 값 오류. `POST`·`PATCH`·`DELETE`의 `X-Relay-Poll`, 빈 `changes`·편집 불가 키·잘못된 JSON 자료형 포함. PATCH 본문 64 KiB(65536바이트) 초과도 포함하며 파일·실행 중 값 변경과 편집 사건이 없다. 로그인 상한은 16 KiB다), `405`(경로는 있으나 메서드가 다름, `Allow` 헤더 포함), `415`(본문이 있고 `Content-Type`이 `application/json`이 아님. 단, 1.9절의 terminate 본문은 항상 400). 잘못된 대상을 대상 없음(`not_found`)과 구분한다. 로그인 실패 횟수에 세지 않는다. | [화면 5절](../specs/dashboard.md#5-입력-검증과-종료-오류), [관리: MGT-025·MGT-013·MGT-037](../specs/management.md#7-수용-테스트) |
| `terminate_failed` | `500`. 종료 요청은 유효했지만 양 끝점 정리에 실패해 종료를 확인하지 못했다. 완료로 표시하지 않는다. 재조회로 실제 상태를 확인한다. | [관리: MGT-012](../specs/management.md#7-수용-테스트), [화면: DASH-012](../specs/dashboard.md#7-수용-테스트), [관리 4절](../specs/management.md#4-오류와-복구) "종료 처리 실패" |
| `internal` | `500`. 조회 처리 중 서버 내부 오류. 빈 성공 목록과 구분한다. 중계 영향은 없다. | [관리: MGT-004](../specs/management.md#7-수용-테스트), [화면: DASH-003](../specs/dashboard.md#7-수용-테스트), [관리 4절](../specs/management.md#4-오류와-복구) "조회 처리·통신 실패" |
| `config_invalid` | `400`. 고친 후보가 설정 검증에 실패했다. `diagnostics`를 포함한다. ticket을 켜는 후보의 필수 키 누락은 `config_missing_key` 진단으로 구분한다. 파일·실행 중 값을 바꾸지 않는다. | [관리: MGT-036·MGT-053](../specs/management.md#7-수용-테스트) |
| `config_edit_disabled` | `403`. 실행 중 `management.config_edit = false`다. | [관리: MGT-030](../specs/management.md#7-수용-테스트) |
| `config_conflict` | `409`. 요청의 `revision`이 지금 파일과 다르다. | [관리: MGT-033·MGT-043](../specs/management.md#7-수용-테스트) |
| `config_restart_pending` | `409`. 파일에 재시작 필요 차이가 남아 있다. `config_keys`를 포함한다. | [관리: MGT-034·MGT-044](../specs/management.md#7-수용-테스트) |
| `config_file_invalid` | `409`. 지금 파일을 읽지 못했거나 검증에 실패했다. `diagnostics`를 포함하며 읽기 실패면 빈 배열이다. | [관리: MGT-035·MGT-044](../specs/management.md#7-수용-테스트) |
| `config_write_failed` | `500`. 임시 파일 쓰기·원자적 교체 실패. 파일과 실행 중 값은 이전 그대로다. | [관리: MGT-038·MGT-045](../specs/management.md#7-수용-테스트) |

본문·필드 형식 오류, 알 수 없는 경로와 허용되지 않는 메서드의 오류는 독립된 스펙 시나리오가 없다. DASH-018·MGT-013·MGT-025와 [화면 스펙 5절](../specs/dashboard.md#5-입력-검증과-종료-오류)에 연결하며, 이 절의 응답 정의를 따른다.

경로별 상태 코드:

| 경로 | 성공 | 가능한 오류 |
| --- | --- | --- |
| `POST /api/v1/session` | `200` | `400`·`415`(`invalid_request`), `401`(`login_failed`), `405`, `500`(`internal`) |
| `GET /api/v1/session` | `200` | `400`(`X-Relay-Poll` 값 오류), `401`, `405`, `415`, `500` |
| `DELETE /api/v1/session` | `204` | `400`, `401`, `403`, `405`, `415`, `500` |
| `GET /api/v1/summary`·`waiting`·`sessions`·`closed-sessions` | `200` | `400`, `401`, `405`, `415`, `500`(`internal`) |
| `GET /api/v1/sessions/{session_id}` | `200` | `400`, `401`, `404`, `405`, `415`, `500` |
| `POST /api/v1/sessions/{session_id}/terminate` | `200` | `400`, `401`, `403`, `404`, `405`, `500`(`terminate_failed`·`internal`) |
| `GET /api/v1/config` | `200` | `400`, `401`, `405`, `415`, `500`(`internal`). 파일 검증·읽기 실패 자체는 `200`의 `file.state`로 구분한다. |
| `PATCH /api/v1/config` | `200` | `400`(`invalid_request`·`config_invalid`), `401`(`unauthenticated`), `403`(`csrf_invalid`·`config_edit_disabled`), `405`, `409`(`config_conflict`·`config_restart_pending`·`config_file_invalid`), `415`, `500`(`config_write_failed`·`internal`) |
| `GET /api/v1/logs` | `200` | `400`(`invalid_request`), `401`(`unauthenticated`), `405`(`invalid_request`), `415`(공통 본문 형식 규칙), `500`(`internal`) |
| `GET /api/v1/metrics` | `200` | `400`(`invalid_request`), `401`(`unauthenticated`), `405`(`invalid_request`), `415`(공통 본문 형식 규칙), `500`(`internal`) |

### 2.2 HTTP 응답이 없는 실패

다음은 오류 코드가 없다. 조회는 조회 실패, 종료·설정 편집은 결과 불명으로 처리한다. 로그인·로그아웃의 별도 동작은 2.3절을 따른다.

| 조건 | 설명 | 관련 스펙 시나리오 |
| --- | --- | --- |
| 연결 거절 | Lm 상한 도달로 새 관리·상태 연결을 거절했다. 중계 접속은 영향이 없다. | [운영: OPS-033](../specs/operations.md#7-수용-테스트), [운영 5절](../specs/operations.md#5-오류와-복구) "Lm 상한 도달" |
| TLS 협상 실패 | TLS 1.2 미만 클라이언트의 협상을 거절한다. 평문으로 전환하지 않는다. | [운영: OPS-036](../specs/operations.md#7-수용-테스트) |
| 응답 시간 초과·연결 끊김 | `query_timeout_secs`(조회·설정 편집) 또는 `terminate_timeout_secs`(종료) 안에 응답이 없다. | [화면: DASH-005·DASH-013](../specs/dashboard.md#7-수용-테스트), [관리: MGT-040·MGT-042](../specs/management.md#7-수용-테스트), [관리 4절](../specs/management.md#4-오류와-복구) |

### 2.3 Consumer의 판정 규칙

| 받은 것 | Consumer 동작 |
| --- | --- |
| `200`과 빈 `items` | 성공한 빈 결과. "대기 없음"·"활성 없음"·"최근 종료 없음" 또는 최근 로그의 빈 상태를 표시한다. 최근 로그의 `max_entries = 0`은 보관 꺼짐으로 구분한다. |
| `401 unauthenticated` | 보호된 이전 데이터와 갱신을 제거하고 `/login/`으로 이동한다. |
| `401 login_failed` | 공통 실패 안내를 표시하고 비밀번호를 지운다. 계정명은 남긴다. |
| `403 csrf_invalid` | 관리 세션은 유효하다. `GET /api/v1/session`으로 토큰을 다시 받아 사용자가 새로 확인한 뒤 요청한다. 자동 재전송하지 않는다. 로그아웃 요청의 `403`은 이 행이 아니라 아래 로그아웃 요청 행을 따른다. |
| `404 not_found` | "대상 없음"을 표시하고 종료를 비활성으로 한다. 목록 이동을 제공한다. |
| `400 invalid_request` | 잘못된 대상·요청으로 표시한다. 대상 없음과 구분한다. |
| `409 config_conflict` | 다시 불러오기 행동을 제공한다. 입력을 버린다는 안내 뒤 새 리비전·값으로 다시 편집한다. |
| `400 config_invalid` | `diagnostics`의 `key`·배열 첨자로 입력란과 목록 줄을 찾아 진단을 표시한다. |
| `403 config_edit_disabled`·`409 config_restart_pending`·`409 config_file_invalid` | 편집 불가 상태로 바꾸고 이유를 표시한다. 각각 기능 꺼짐·재시작 필요 차이·현재 파일 오류를 구분한다. |
| `500 config_write_failed` | 파일·실행 중 값은 이전 그대로다. 입력을 유지하고 명시적 재시도를 제공한다. |
| `PATCH /api/v1/config`의 응답 없음·시간 초과·그 밖의 `5xx` | 결과 불명이다. 자동 재전송하지 않는다. `GET /api/v1/config`로 `revision`과 실행 중 값을 다시 확인한다. 확인 전 성공·실패로 단정하지 않는다. |
| `GET /api/v1/config`의 `500`·응답 없음·시간 초과 | 조회 실패다. 이전 데이터는 마지막 성공 시각과 함께 오래된 값으로 표시하고 빈 설정으로 대신하지 않는다. |
| 로그인 요청(`POST /api/v1/session`)의 `500`, 응답 없음, 시간 초과 | 로그인 요청 실패. 인증 실패(`401 login_failed`)의 공통 안내와 다른 안내를 표시한다. 관리 세션이 없는 상태로 SCR-003에 머문다. 문구는 [화면 스펙](../specs/dashboard.md#4-화면-상태와-갱신) SCR-003 "로그인 요청 실패" 행이 정한다. |
| 로그아웃 요청(`DELETE /api/v1/session`)의 `403 csrf_invalid`, `500`, 응답 없음, 시간 초과 | 로그아웃을 완료로 표시하지 않는다. 보호된 데이터를 숨기고 SCR-003에 로그아웃 결과 확인 필요 안내를 보인다. 상태와 문구는 [화면 스펙](../specs/dashboard.md#4-화면-상태와-갱신) SCR-003 "로그아웃 결과 확인 필요" 행이 정한다. `403`도 같이 처리한다. 이때 서버는 관리 세션을 유지하므로(1.4.3절), 그 관리 세션은 유휴 만료 Si나 절대 만료 Sa까지 남는다. |
| 그 밖의 조회 요청의 `500`, 응답 없음, 시간 초과 | 조회 실패. 이전 데이터는 마지막 성공 시각과 함께 오래된 값으로 표시한다. 빈 결과로 표시하지 않는다. |
| 모르는 `code` | HTTP 상태 코드 범주로 처리한다(`401`은 관리 세션 무효, `4xx`는 요청 오류, `5xx`는 조회 실패). 단, 종료·설정 편집의 `5xx`는 결과 불명으로 재조회한다. `409`도 `4xx` 요청 오류 범주다. |

설정 보기와 편집의 응답 대기 한도는 기존 세션 정보의 `dashboard.query_timeout_secs`를 쓴다. 파일 읽기·검증·로컬 원자적 교체는 종료 교환의 E 완료를 기다리지 않는다. 따라서 종료 전용 `terminate_timeout_secs`나 새 설정 키를 추가하지 않는다. 한도를 넘으면 GET은 조회 실패, PATCH는 결과 불명으로 처리한다. 클라이언트가 기다리기를 멈췄다는 사실만으로 서버의 편집 처리가 취소되었다고 판단하지 않는다. 재조회도 실패하면 결과 불명 상태를 유지한다.

## 3. 호환성 규칙

- 데이터를 사용하는 쪽(Consumer)은 응답의 알 수 없는 필드를 무시해야 한다. 허용 값 목록에 없는 문자열 값(`server.state`·`state`·`mode`·`transport`·`viewer_transport`·`target_transport`·`close_cause`·`role`·`*_unavailable_reason`)은 받은 그대로 표시하고 오류로 처리하지 않아야 한다. 모르는 `result` 값은 결과 불명으로 보고 재조회해야 한다. 모르는 오류 코드는 2.3절대로 HTTP 상태 코드 범주로 처리해야 한다.
- 데이터를 만드는 쪽(Producer)은 요청의 알 수 없는 필드를 무시해야 한다. 헤더·쿠키·경로는 이 계약이 정한 것만 해석한다.
- 호환 변경(v1.x): 응답에 필드를 추가한다. 허용 값 목록에 값을 추가한다. 새 경로를 추가한다. 요청에 선택 필드를 추가한다. 새 오류 코드를 기존 상태 코드 범주 안에 추가한다. 확인 불가 사유 값을 추가한다. 성능·내부 동작을 바꾼다.
- 설정 보기·편집 경로·메서드·응답 필드·오류 코드는 1.1.0의 호환 추가 범위다. 상태 코드 `409`는 2.3절의 `4xx` 요청 오류 범주에 속한다.
- 최근 로그 경로·선택 매개변수·응답 필드와 세션 필드는 1.1.0의 호환 추가 범위다. 세션 필터 URL은 1.11절을 따른다.
- 접속 ticket의 `session_expired`와 설정 보기 항목은 1.1.0의 허용 값·필드 추가 범위다. ticket 편집은 편집 대상 8개에 적용한다.
- 대기·최근 종료 목록 필드는 1.1.0의 응답 필드 추가 범위다. 기존 필드의 이름·자료형·의미는 바꾸지 않는다.
- 메트릭 경로·선택 매개변수·응답 본문은 1.2.0의 새 경로 추가 범위다. 응답 본문의 호환 규칙은 [메트릭 계약 3절](metrics-v1.md#3-호환성-규칙)을 따른다.
- `history_window` 선택 매개변수와 최근 24시간 추이는 1.3.0의 요청 선택 필드 추가 범위다. 이 매개변수를 보내지 않는 Consumer는 1.2.0과 같은 최근 1시간 추이를 받는다.
- 호환성을 깨는 변경(Breaking change): 필드나 경로를 삭제·이름 변경한다. 자료형·단위·시각 형식·`null` 의미를 바꾼다. 기존 조건의 HTTP 상태 코드나 오류 코드를 바꾼다. 요청 필드를 필수로 바꾼다. 쿠키 이름이나 헤더 이름·의미를 바꾼다. 쿠키 보안 속성을 약하게 바꾼다. 인증·CSRF 요구를 약하게 바꾼다. 위 변경은 새 major 버전(`/api/v2/`)으로 낸다.
- 마이그레이션: 새 major 버전은 새 접두사로 제공하고, 같은 릴리스에서 대시보드를 새 버전으로 옮긴다. 대시보드는 같은 실행파일에 들어 있으므로 서버와 대시보드의 버전이 어긋나지 않는다. 외부 클라이언트가 있으면 이전 major를 한동안 함께 제공한다. 기간은 새 major 채택 때 정한다.
- 롤백: 이전 릴리스 실행파일로 되돌리면 이전 major와 대시보드가 함께 돌아온다. 관리 세션은 서버 메모리에만 있으므로 되돌리는 시점에 모두 무효가 되고, 관리자는 다시 로그인한다. 최근 종료 목록도 비워진다.
- 정적 응답이 `no-store`이므로 서버 갱신 뒤 브라우저가 이전 버전의 화면 코드를 재사용하지 않는다.

## 4. 버전과 지원 단계

| 상태 | 조건 | 지원 종료·대체 |
| --- | --- | --- |
| Draft | 채택 전 상태. | 해당 없음 |
| Active | 현재. Producer·Consumer 계약 테스트의 현재 결과는 5절의 방법으로 확인한다. 통과 전에는 지원 완료로 표시하지 않는다. | 새 major가 Active가 될 때까지 유지한다. |
| Deprecated | 새 major(`/api/v2/`)가 Active가 됐다. | 이전 major를 함께 제공하는 기간과 종료일은 새 major 채택 때 정한다. |
| Retired | Deprecated 기간이 끝났다. | 새 major가 대체한다. 이전 경로는 `404`를 반환한다. |

## 5. 예제와 계약 테스트

예제의 값은 문서용 값이다. 실제 비밀값·비밀번호·해시·쿠키 값이 아니다. IP는 문서용 대역(192.0.2.0/24, 198.51.100.0/24, 2001:db8::/32)을 쓴다.

설정 보기 `GET /api/v1/config`의 `200` 예제(읽기 편하게 `items`에서 세 항목만 발췌했다. 실제 응답에는 모든 키가 설정 계약 표 순서로 나온다):

```json
{
  "as_of": "2026-11-08T05:00:00.000Z",
  "revision": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
  "edit_enabled": true,
  "file": {"state": "in_effect", "config_keys": [], "diagnostics": []},
  "last_reload": {"at": "2026-11-08T04:59:00.000Z", "result": "applied", "trigger": "file_watch", "config_keys": ["relay.tcp_viewer.allow"]},
  "items": [
    {"key": "relay.tcp_viewer.enabled", "value": true, "reload": "restart", "editable": false},
    {"key": "relay.tcp_viewer.allow", "value": ["198.51.100.0/24"], "reload": "dynamic", "editable": true},
    {"key": "limits.relay_connections", "value": 1000, "reload": "dynamic", "editable": true, "min": 2, "max": 65535}
  ]
}
```

설정 편집 요청과 `200` 응답(리비전은 형식을 보이는 문서용 값이다):

```http
PATCH /api/v1/config HTTP/1.1
Content-Type: application/json
X-CSRF-Token: <csrf-token>

{"revision": "sha256:0000000000000000000000000000000000000000000000000000000000000000", "changes": {"limits.relay_connections": 2000, "relay.tcp_viewer.allow": ["198.51.100.0/24", "192.0.2.7"]}}
```

```json
{"as_of": "2026-11-08T05:00:01.000Z", "result": "applied", "revision": "sha256:1111111111111111111111111111111111111111111111111111111111111111", "applied": ["limits.relay_connections", "relay.tcp_viewer.allow"]}
```

후보 검증 실패 `400 config_invalid`:

```json
{"error": {"code": "config_invalid", "message": "설정 후보 검증에 실패했습니다.", "diagnostics": [{"code": "config_relation_violated", "key": "limits.waiting", "message": "대기 상한은 중계 연결 상한 이하여야 합니다."}]}}
```

리비전 충돌 `409 config_conflict`:

```json
{"error": {"code": "config_conflict", "message": "설정 파일이 바뀌었습니다. 다시 불러오세요."}}
```

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
    "target_transport": "ws",
    "started_at": "2026-10-16T08:40:10.120Z",
    "bytes_viewer_to_target": 1048576,
    "bytes_target_to_viewer": 73400320,
    "viewer_address": {"ip": "198.51.100.7", "port": 51234},
    "target_address": {"ip": "192.0.2.20", "port": 40022},
    "viewer_forwarded_ip": null,
    "target_forwarded_ip": "192.0.2.42"
  }
}
```

대기 연결 목록(두 번째 항목은 신뢰 프록시를 거친 WSS 연결이다):

```json
{
  "as_of": "2026-10-16T09:00:02.000Z",
  "items": [
    {"role": "viewer", "relay_id": "48213", "connected_at": "2026-10-16T08:59:28.000Z", "waited_ms": 34000, "transport": "tcp", "address": {"ip": "198.51.100.7", "port": 51234}, "forwarded_ip": null},
    {"role": "server", "relay_id": "0123", "connected_at": "2026-10-16T08:59:50.500Z", "waited_ms": 11500, "transport": "wss", "address": {"ip": "192.0.2.10", "port": 443}, "forwarded_ip": "192.0.2.42"}
  ]
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
    "target_transport": "ws",
    "started_at": "2026-10-16T08:40:10.120Z",
    "ended_at": "2026-10-16T09:00:05.380Z",
    "duration_ms": 1195260,
    "bytes_viewer_to_target": 1049600,
    "bytes_target_to_viewer": 73500000,
    "viewer_address": {"ip": "198.51.100.7", "port": 51234},
    "target_address": {"ip": "192.0.2.20", "port": 40022},
    "viewer_forwarded_ip": null,
    "target_forwarded_ip": "192.0.2.42",
    "close_cause": "admin_terminated"
  }
}
```

오류 응답(대상 없음):

```json
{"error": {"code": "not_found", "message": "지정한 세션을 찾을 수 없습니다."}}
```

최근 로그 요청과 응답(쿠키는 문서용 표시다):

```http
GET /api/v1/logs?level=info&event=connection_accepted&after=0&limit=200 HTTP/1.1
Cookie: relay_admin_session=<불투명 식별자>
X-Relay-Poll: 1
```

```json
{
  "as_of": "2026-11-08T05:00:02.000Z",
  "max_entries": 1000,
  "oldest_seq": 1,
  "latest_seq": 3,
  "has_more": false,
  "items": [
    {"seq": 2, "entry": {"ts": "2026-11-08T05:00:01.000Z", "level": "info", "event": "connection_accepted", "conn_id": "con_example", "listener": "ws", "peer_ip": "192.0.2.20"}}
  ]
}
```

계약 테스트는 아래 위치에서 관리한다. 현재 시나리오별 결과는 [테스트 컨벤션](../conventions/testing.md)에 따라 `cargo xtask scenarios`로 생성한 `scenario-report.md`와 CI 결과에서 확인한다.

| 범위 | 테스트 위치 |
| --- | --- |
| Producer의 경로·인증·조회·종료·설정·최근 로그 | `crates/adapters/tests/management.rs`, `tests/management_session.rs`, `tests/management_query.rs`, `tests/management_terminate.rs` |
| 실행파일의 전송 경로·주소·대기·최근 종료 필드 | `tests/serve.rs`의 `refuses_public_plaintext_and_answers_login_less_queries_with_401_on_loopback`·`real_mode_i_and_proxied_ws_wss_sessions_expose_transport_and_forwarded_addresses`·`real_waiting_lists_preserve_all_entry_transports_and_proxy_addresses`·`real_closed_lists_and_termination_preserve_addresses_and_logged_monotonic_duration` |
| 접속 ticket의 종료 원인·설정 | `tests/ticket_system.rs`, `crates/adapters/tests/management.rs` |
| Consumer의 정적 파일·화면·실서버 교환 | `tests/dashboard_assets.rs`, `tests/dashboard_browser.rs`, `tests/dashboard_live.rs` |
| 단위 시험 | `crates/app/src/management/login.rs`·`session.rs`·`query.rs`·`terminate.rs`·`config/tests.rs`, `crates/adapters/src/management_http/query_api.rs`·`terminate_api.rs` |

Consumer 실서버 시험은 일반 로그인·조회·종료·설정 데이터를 실제 실행파일과 설정 파일에서 얻는다. 확인 불가 상태는 실제 조회 전달 실패로 만든다. 미래 enum 값은 현재 Producer가 만들 수 없는 별도 Consumer 계약 입력이다. 지연·응답 유실·500·연결 종료는 시험 쪽에서 주입한다. DASH-040의 `config_missing_key`·`config_type_mismatch`·`config_relation_violated`와 모르는 코드는 계약 형식의 진단이다. 편집 가능한 키의 PATCH로는 서버가 이 코드를 내지 않는다. DASH-043의 편집 불가 오류는 실제 원시 PATCH 응답을 다시 보낸다. 이런 주입을 Producer 오류의 증거로 세지 않는다.

로그인 응답 시간의 계정 존재 비노출은 시간 측정 대신 `CountingVerifier` 구조 시험으로 판정한다. 모든 로그인 시도가 비밀번호 검증을 정확히 한 번 하는지 센다. 계정 해시와 더미 해시의 Argon2 매개변수가 같은지는 확인하지 않았다.

브라우저 관찰 기록은 다음과 같다. 자동화 환경에서 인증 창 자체는 보이지 않으므로 `Session`과 `Basic` 401의 반응 차이로 판정한다.

| 날짜·대상 | 환경 | 확인 방법 | 실제 결과 |
| --- | --- | --- | --- |
| 2026-11-04, 관리 로그인·401 응답 | Chrome 156.0.8078.76·Edge 156.0.4314.58(Windows), Firefox 159.0(WSL) | `WWW-Authenticate: Session realm="vnc-relay"`과 `Basic` 401을 대조한다. 다른 사이트 `http://localhost:<port>`의 JSON fetch·no-cors text/plain·폼 POST로 평문 루프백 로그인 CSRF를 관찰한다. 정상 로그인 뒤 다른 사이트 no-cors 요청도 관찰한다. | Session 401은 JSON 본문을 보이고 Basic 401은 인증 요구로 반응했다. JSON fetch는 사전 요청에서 막혔고 no-cors는 불투명 응답만 받았다. 폼 POST는 415이며 Set-Cookie가 없었다. 공격 뒤 쿠키가 없었고 로그인 성공 로그는 정상 로그인뿐이었다. 정상 로그인 뒤 다른 사이트 요청에는 쿠키가 실리지 않았다. |
| 2026-11-07, Safari 대체 관찰 | Windows용 WebKit 27.2(Playwright) | 같은 401 대조와 로그인 CSRF 세 가지, 정상 로그인 뒤 다른 사이트 no-cors GET·종료 요청을 관찰한다. | Basic도 본문을 보여 인증 창 대조는 판정하지 못했다. 로그인 CSRF 세 가지는 위와 같았다. SameSite=Strict 쿠키를 SameSite=None으로 저장해 다른 사이트 GET에 쿠키가 실렸다. 종료 요청은 X-CSRF-Token이 없어 403이었다. 실제 Safari의 SameSite 동작은 판정하지 못했다. |

## 6. 초안을 확정하기 전에 해결할 사항

- 미확인: 실제 Safari의 `WWW-Authenticate: Session` 401 반응과 로그인 CSRF·SameSite 동작. 책임자는 프로젝트 관리자다. macOS 환경의 실제 Safari 관찰이 필요하다. 5절의 Windows용 WebKit 결과로 대신 판정하지 않는다.

