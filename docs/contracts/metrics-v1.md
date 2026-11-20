# 메트릭 v1 계약

VNC Relay가 내보내는 메트릭의 이름·종류·단위·속성과 두 표현 형식을 정한다. 하나는 상태 리스너 `GET /metrics`의 Prometheus 텍스트이고, 다른 하나는 관리 API `GET /api/v1/metrics`의 JSON이다. 외부 수집 도구와 내장 대시보드가 이 문서의 표를 형식의 기준으로 쓴다. 외부 동작은 [운영 스펙 3.8절](../specs/operations.md#38-메트릭)과 [관리 스펙 2.5절](../specs/management.md#25-메트릭)이 소유하며, 이 문서는 그 동작을 바꾸지 않는다.

| 항목 | 값 |
| --- | --- |
| 이름과 버전 | `vnc-relay-metrics` 1.2.0 |
| 상태 | Active |
| 버전 차이 | 1.1.0은 1.8절에 최근 24시간 추이를 더한 호환 추가다. 기간을 고르지 않는 Consumer는 1.0.0과 같은 최근 1시간 추이를 받는다. 1.2.0은 1.4절 거절 원인·종료 원인 속성 값과 대기 제거 결과에 출발지 IP 차단 `ip_denied`를 더한 호환 추가다. |
| 소유자 | vnc-relay 프로젝트 관리자 |
| 만드는 쪽(Producer) | `vnc-relay` (`vnc-relay serve` 프로세스와 Windows 서비스로 실행한 `vnc-relay service run` 프로세스) |
| 사용하는 쪽(Consumer) | 운영자의 메트릭 수집 도구(Prometheus, OpenTelemetry Collector의 Prometheus 수신기), 내장 대시보드 SCR-007([화면 스펙](../specs/dashboard.md#scr-007-메트릭)) |
| 방향 | 요청/응답. 수집 도구가 상태 리스너에 `GET /metrics`를 보낸다. 대시보드가 관리 리스너에 `GET /api/v1/metrics`를 보낸다. |
| 스키마 기준 파일 | 없음. 1절의 Markdown 표가 형식의 기준이다. |
| 기준 스펙 | [운영 스펙 3.8절](../specs/operations.md#38-메트릭), [관리 스펙 2.5절](../specs/management.md#25-메트릭), [화면 스펙 SCR-007](../specs/dashboard.md#scr-007-메트릭) |
| 관련 요구사항 | [PRD](../requirements/PRD.md) 확정 기준의 메트릭 행, FR-06·FR-07·FR-08, AC-60~62 |
| 호환성 범위 | 이 계약의 1.x 형식을 따르는 `vnc-relay` 구현과 소비자. 지원 판정은 Producer 시험을 따른다. 저장소에 없는 외부 수집 도구의 시험은 대상이 아니다. |
| 관련 테스트 | [운영 스펙 7절](../specs/operations.md#7-수용-테스트) OPS-082~094·OPS-101, [관리 스펙 7절](../specs/management.md#7-수용-테스트) MGT-056~061, [화면 스펙 7절](../specs/dashboard.md#7-수용-테스트) DASH-068~081. 시험 위치는 5절을 따른다. |

관련 문서: [운영 정보 계약](ops-info-v2.md)(속성 값의 출처인 `listener`·`stage`·`reason`·`close_cause`·`result`, 상태 리스너), [관리 API 계약](management-api-v1.md)(인증·폴링 표시·오류 본문), [설정 계약](config-v4.md)(`[metrics] enabled`), [ADR-0011](../decisions/ADR-0011-pull-metrics-on-status-listener.md)(선택 이유).

## 1. 형식과 의미

이 계약의 메트릭 이름·종류·단위·속성 이름과 값·히스토그램 경계·두 표현 형식은 확정된 형식이다. 이름·단위·속성은 OpenTelemetry 의미 규칙(semantic conventions)의 작성 규칙을 따른다. `process.*`는 OpenTelemetry 프로세스 의미 규칙의 같은 이름을 쓴다.

### 1.1 공통 규칙

| 항목 | 규칙 |
| --- | --- |
| 계측기 종류 | `counter`(Counter): 줄지 않는 누적 합계다. `updowncounter`(UpDownCounter): 늘거나 주는 현재 수다. `gauge`(Gauge): 더할 수 없는 현재 값이다. `histogram`(Histogram): 관찰한 값의 개수·합계·구간별 개수다. |
| 시작점과 누적 | 모든 `counter`와 `histogram`은 프로세스가 시작할 때 0에서 시작하는 누적 값이다(OpenTelemetry의 cumulative temporality). 값은 메모리에만 있다. 재시작하면 0부터 다시 센다. 중간에 0으로 되돌리지 않는다. |
| 기록 시점 | 운영 로그 사건에서 세는 값은 그 사건을 운영 로그 기록부에 넘길 때 함께 센다. 로그 출력의 성공 여부와 관계없다. 그래서 같은 프로세스의 로그 사건 수와 값이 같다. 사건이 없는 값의 기록 시점은 1.3절 표가 정한다. |
| 현재 값 | `updowncounter`와 `gauge`는 응답을 만드는 시점의 값이다. 한 응답 안의 중계 상태 값(대기·세션·진행 중 연결)은 한 스냅샷에서 읽는다. L·Lm 사용량은 [관리 API 계약 1.1절](management-api-v1.md#11-공통-규칙)의 `usage`처럼 같은 응답의 다른 값과 잠깐 어긋날 수 있다. |
| 속성 조합 | `counter`·`histogram`은 한 번이라도 기록한 속성 조합만 나온다. 나온 조합은 프로세스가 끝날 때까지 남는다. 속성 있는 `updowncounter`는 1.4절이 정한 모든 조합을 값 0이어도 항상 낸다. |
| 속성 값 | 1.4절의 고정 값 목록만 쓴다. 중계 ID·`relay_id_ref`·세션 식별자·연결 식별자·주소·포트·호스트 이름·계정명·Origin·ticket 클레임·설정 값·파일 경로는 속성에도 값에도 넣지 않는다. |
| 읽지 못한 값 | 값을 읽지 못하면 0으로 대신하지 않는다. Prometheus 텍스트에서는 그 계열의 표본을 쓰지 않는다. JSON에서는 `available = false`와 빈 `points`를 준다. 현재 읽기에 실패할 수 있는 값은 `process.*`뿐이다. |
| 플랫폼별 메트릭 | `process.unix.file_descriptor.count`는 Linux에서만, `process.windows.handle.count`는 Windows에서만 낸다. 다른 플랫폼의 응답에는 그 메트릭이 없다. |

### 1.2 리소스

리소스(resource)는 메트릭을 낸 프로세스를 나타내는 속성이다.

| 속성 | 값 |
| --- | --- |
| `service.name` | 항상 `vnc-relay` |
| `service.version` | 실행파일 버전. `vnc-relay --version`이 출력하는 버전 문자열과 같다(예: `2026.11.6`). |

### 1.3 메트릭 목록

표의 순서가 두 표현의 출력 순서다. "로그 사건"은 [운영 정보 계약 1.5절](ops-info-v2.md#15-사건-목록)의 사건 이름이다.

| 이름 | 종류 | 단위 | 속성 | 의미·기록 시점 |
| --- | --- | --- | --- | --- |
| `vnc_relay.connection.accepted` | `counter` | `{connection}` | `listener` | 중계 리스너가 수락한 연결 수. 로그 사건 `connection_accepted`마다 1. |
| `vnc_relay.connection.rejected` | `counter` | `{connection}` | `listener`, `stage`, `reason` | 서버가 정책·제한·형식 때문에 거절한 연결 수. 로그 사건 `connection_rejected`마다 1. 관리·상태 리스너의 거절도 센다. |
| `vnc_relay.connection.aborted` | `counter` | `{connection}` | `listener`, `stage`, `reason` | 대기 등록이나 세션 시작 전에 끝난 연결 수. 로그 사건 `connection_aborted`마다 1. |
| `vnc_relay.connection.in_progress` | `updowncounter` | `{connection}` | `stage` | 지금 초기 교환 중인 연결 수와 Mode I 대상 연결 중인 연결 수. [관리 API 계약 1.6절](management-api-v1.md#16-서버-요약-get-apiv1summary)의 `counts.handshaking`·`counts.target_connecting`과 같은 값이다. |
| `vnc_relay.relay_connection.usage` | `updowncounter` | `{connection}` | 없음 | 중계 연결 상한 L의 현재 사용량. 서버 요약의 `usage.relay_connections.current`와 같다. |
| `vnc_relay.relay_connection.limit` | `updowncounter` | `{connection}` | 없음 | 실행 중 L(`limits.relay_connections`). 재로드하면 새 값이다. |
| `vnc_relay.admin_connection.usage` | `updowncounter` | `{connection}` | 없음 | 관리·상태 연결 상한 Lm의 현재 사용량. 이 응답을 받는 연결도 포함한다. |
| `vnc_relay.admin_connection.limit` | `updowncounter` | `{connection}` | 없음 | 실행 중 Lm(`limits.admin_connections`). |
| `vnc_relay.handshake.duration` | `histogram` | `s` | `listener`, `mode` | Repeater 초기 교환 시간. 초기 교환 제한 시간 H를 시작한 때부터 유효한 초기 요청을 판정한 때까지다. 성공한 초기 교환만 기록한다. 시간 초과·형식 오류는 `vnc_relay.connection.aborted`·`rejected`로 센다. |
| `vnc_relay.waiting.count` | `updowncounter` | `{connection}` | `role`, `transport` | 지금 대기 중인 Mode II 연결 수. 서버 요약의 `counts.waiting`을 역할·전송 경로로 나눈 값이다. |
| `vnc_relay.waiting.duration` | `histogram` | `s` | `result` | 대기 연결이 대기를 끝낸 시간. 매칭은 로그 사건 `session_started`(Mode II)의 `waited_ms`, 만료는 `waiting_expired`의 `waited_ms`, 그 밖의 제거는 `waiting_removed`의 `waited_ms`를 기록한다. |
| `vnc_relay.session.count` | `updowncounter` | `{session}` | `mode`, `viewer_transport`, `target_transport` | 지금 활성 세션 수. 정리 중인 세션도 정리가 끝날 때까지 포함한다. 서버 요약의 `counts.active`를 나눈 값이다. |
| `vnc_relay.session.started` | `counter` | `{session}` | `mode`, `viewer_transport`, `target_transport` | 시작한 활성 세션 수. 로그 사건 `session_started`마다 1. |
| `vnc_relay.session.closed` | `counter` | `{session}` | `mode`, `close_cause` | 정리가 끝난 활성 세션 수. 로그 사건 `session_closed`마다 1. |
| `vnc_relay.session.duration` | `histogram` | `s` | `mode` | 활성 세션의 지속 시간. 로그 사건 `session_closed`의 `duration_ms`를 기록한다. |
| `vnc_relay.session.io` | `counter` | `By` | `direction` | 활성 세션이 상대 전송 계층에 쓰기에 성공한 바이트 수. 쓰기에 성공할 때마다 더한다. 그래서 세션이 끝나기 전에도 늘어난다. 뜻은 [관리 스펙 2.1절](../specs/management.md#21-전송량의-의미)과 같다. |
| `vnc_relay.target_connect.duration` | `histogram` | `s` | `result` | Mode I 대상 연결 시도 시간. 연결 시간 제한 C를 시작한 때부터 연결 성공 또는 실패를 판정한 때까지다. 로그 사건 `target_connect_failed`의 `elapsed_ms`와 같은 구간이다. 허용 정책·L 거절과 종료·중단으로 끝난 시도는 기록하지 않는다. |
| `vnc_relay.ticket.accepted` | `counter` | `{ticket}` | `listener`, `role` | 받아들인 접속 ticket 수. 로그 사건 `ticket_accepted`마다 1. ticket 거절은 `vnc_relay.connection.rejected`의 `ticket_*` 원인으로 센다. |
| `vnc_relay.websocket.ping_timeouts` | `counter` | `{connection}` | `listener` | WebSocket pong 대기 시간이 지나 끊긴 연결로 판정한 수. 판정할 때마다 1. TCP keepalive로 판정한 끊긴 연결은 세지 않는다. |
| `vnc_relay.admin.logins` | `counter` | `{attempt}` | `result` | 관리자 로그인 시도 결과 수. 로그 사건 `admin_login_succeeded`, `admin_login_failed`마다 1. |
| `vnc_relay.admin.lockouts` | `counter` | `{lockout}` | `scope` | 로그인 제한을 시작한 수. 로그 사건 `admin_login_limited`마다 1. |
| `vnc_relay.admin.terminations` | `counter` | `{request}` | `result` | 처리한 관리자 종료 요청 수. 로그 사건 `admin_terminate`마다 1. |
| `vnc_relay.admin_session.count` | `updowncounter` | `{session}` | 없음 | 지금 유효한 관리 세션 수. 로그아웃·유휴 만료·절대 만료한 관리 세션은 세지 않는다. |
| `vnc_relay.config.reloads` | `counter` | `{reload}` | `result`, `trigger`, `reason` | 재로드 결과 수. 로그 사건 `reload_applied`, `reload_rejected`, `reload_restart_required`마다 1. |
| `vnc_relay.config.edits` | `counter` | `{request}` | `result`, `reason` | 설정 편집 결과 수. 로그 사건 `config_edited`, `config_edit_rejected`마다 1. |
| `process.uptime` | `gauge` | `s` | 없음 | 프로세스가 시작한 뒤 지난 시간. 단조 시계로 잰다. 소수 초다. |
| `process.cpu.time` | `counter` | `s` | `cpu.mode` | 프로세스가 쓴 CPU 시간. 소수 초다. |
| `process.memory.usage` | `updowncounter` | `By` | 없음 | 물리 메모리 사용량. Linux는 `/proc/self/status`의 `VmRSS`, Windows는 작업 집합(Working Set)이다. |
| `process.memory.virtual` | `updowncounter` | `By` | 없음 | 가상 메모리 크기. Linux는 `/proc/self/status`의 `VmSize`, Windows는 커밋한 전용 메모리(Private Bytes)다. |
| `process.thread.count` | `updowncounter` | `{thread}` | 없음 | 프로세스의 스레드 수. |
| `process.unix.file_descriptor.count` | `updowncounter` | `{file_descriptor}` | 없음 | Linux만. 열린 파일 기술자 수. `/proc/self/fd`의 항목 수다. |
| `process.windows.handle.count` | `updowncounter` | `{handle}` | 없음 | Windows만. 열린 핸들 수. |

`vnc_relay.config.reloads`의 `trigger`는 `result = applied`에만, `reason`은 `result = rejected`에만 붙는다. `vnc_relay.config.edits`의 `reason`은 `result = rejected`에만 붙는다. 다른 결과에는 그 속성이 없다.

### 1.4 속성 값

| 속성 | 쓰는 메트릭 | 값 | 출처 |
| --- | --- | --- | --- |
| `listener` | `connection.accepted` | `tcp_viewer`, `tcp_server`, `ws`, `wss` | [운영 정보 계약 1.4절](ops-info-v2.md#14-필드-사전) |
| `listener` | `connection.rejected` | `tcp_viewer`, `tcp_server`, `ws`, `wss`, `management`, `status` | 같은 절 |
| `listener` | `connection.aborted`, `handshake.duration` | `tcp_viewer`, `tcp_server`, `ws`, `wss` | 같은 절 |
| `listener` | `ticket.accepted`, `websocket.ping_timeouts` | `ws`, `wss` | 같은 절 |
| `stage`, `reason` | `connection.rejected`, `connection.aborted` | 해당 로그 사건의 값 그대로. `connection.rejected`의 `reason`에는 `ip_denied`(`stage`는 `accept`·`upgrade`·`register`)가 있다. | [운영 정보 계약 1.6절](ops-info-v2.md#16-원인-코드) |
| `stage` | `connection.in_progress` | `initial_exchange`, `target_connect` | 위 값의 부분 집합 |
| `mode` | `handshake.duration`, `session.*` | `mode1`, `mode2` | 운영 정보 계약 1.4절 |
| `role` | `waiting.count`, `ticket.accepted` | `viewer`, `server` | 같은 절 |
| `transport` | `waiting.count` | `tcp`, `ws`, `wss` | [관리 API 계약 1.7절](management-api-v1.md#17-대기-연결-목록-get-apiv1waiting) |
| `viewer_transport`, `target_transport` | `session.count`, `session.started` | `tcp`, `ws`, `wss`. Mode I의 `target_transport`는 항상 `tcp`다. | [관리 API 계약 1.8절](management-api-v1.md#18-활성-세션-get-apiv1sessions-get-apiv1sessionssession_id) |
| `close_cause` | `session.closed` | 로그 사건 `session_closed`의 값 그대로. 출발지 IP 차단으로 끊은 세션은 `ip_denied`다. | [관리 API 계약 1.10절](management-api-v1.md#110-최근-종료-목록-get-apiv1closed-sessions) |
| `direction` | `session.io` | `viewer_to_target`, `target_to_viewer` | 관리 API의 `bytes_viewer_to_target`·`bytes_target_to_viewer`와 같은 방향 |
| `result` | `waiting.duration` | `matched`(매칭), `expired`(대기 시간 W 만료), `removed`(그 밖의 제거) | 이 계약. 차단 목록 교체로 대기에서 뺀 연결(`waiting_removed`의 `reason = ip_denied`)도 `removed`다. 제거 원인은 속성에 쓰지 않는다. |
| `result` | `target_connect.duration` | `connected`, `resolve_failed`, `connect_refused`, `connect_timeout`, `connect_error` | `connected` 외에는 로그 사건 `target_connect_failed`의 `reason` |
| `result` | `admin.logins` | `succeeded`, `invalid_credentials`, `limited` | `admin_login_succeeded`는 `succeeded`. `admin_login_failed`는 그 `reason` |
| `scope` | `admin.lockouts` | `account`, `source` | 로그 사건 `admin_login_limited`의 `scope`. 어느 기준의 제한인지만 나타낸다. 계정명이나 주소가 아니다. |
| `result` | `admin.terminations` | `terminated`, `already_closed`, `not_found`, `failed` | 로그 사건 `admin_terminate`의 `result` |
| `result` | `config.reloads` | `applied`, `rejected`, `restart_required` | 각각 `reload_applied`, `reload_rejected`, `reload_restart_required` |
| `trigger` | `config.reloads` | `file_watch`, `config_edit` | `reload_applied`의 `trigger` |
| `reason` | `config.reloads` | `reload_rejected`의 `reason` 값 그대로 | [운영 정보 계약 1.6절](ops-info-v2.md#16-원인-코드) |
| `result` | `config.edits` | `edited`, `rejected` | 각각 `config_edited`, `config_edit_rejected` |
| `reason` | `config.edits` | `config_edit_rejected`의 `reason` 값 그대로 | [관리 API 계약 2.1절](management-api-v1.md#21-오류-본문과-상태-코드)의 오류 코드 |
| `cpu.mode` | `process.cpu.time` | `user`, `system` | OpenTelemetry 프로세스 의미 규칙 |

속성 있는 `updowncounter`가 항상 내는 조합은 다음과 같다.

- `vnc_relay.connection.in_progress`: `stage`의 두 값. 2개.
- `vnc_relay.waiting.count`: `role` 2개 × `transport` 3개. 6개.
- `vnc_relay.session.count`: Mode I은 `viewer_transport` 3개 × `target_transport = tcp`, Mode II는 `viewer_transport` 3개 × `target_transport` 3개. 12개.

### 1.5 히스토그램 경계

경계는 초 단위 상한이다. 값 v는 `v ≤ 경계`인 첫 구간에 들어간다. 마지막 경계보다 크면 마지막 구간(+Inf)에 들어간다.

| 메트릭 | 경계(초) |
| --- | --- |
| `vnc_relay.handshake.duration` | 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10, 30, 60, 120 |
| `vnc_relay.target_connect.duration` | 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10, 30, 60, 120 |
| `vnc_relay.waiting.duration` | 1, 5, 10, 30, 60, 300, 600, 1800, 3600, 7200, 21600, 43200, 86400 |
| `vnc_relay.session.duration` | 1, 10, 60, 300, 600, 1800, 3600, 7200, 14400, 28800, 86400, 259200, 604800 |

밀리초로 받은 값(`waited_ms`, `duration_ms`)은 1000으로 나눈 초로 기록한다. 합계(`sum`)는 기록한 초 값의 합이다.

### 1.6 Prometheus 텍스트 표현

`[metrics] enabled = true`이면 상태 리스너가 `GET /metrics`에 이 형식으로 응답한다. 꺼져 있으면 경로가 없다(2절). 응답 규칙은 [운영 스펙 3.8절](../specs/operations.md#38-메트릭)이 정한다.

| 항목 | 규칙 |
| --- | --- |
| 형식 | Prometheus 텍스트 표현 형식 0.0.4. UTF-8, 줄 끝은 LF다. 마지막 줄도 LF로 끝난다. |
| 응답 헤더 | `Content-Type: text/plain; version=0.0.4; charset=utf-8`, `Cache-Control: no-store` |
| 이름 변환 | OpenTelemetry 이름의 `.`을 `_`로 바꾼다. 단위가 `s`이면 `_seconds`, `By`이면 `_bytes`를 붙인다. 중괄호 단위(`{connection}` 등)는 이름에 붙이지 않는다. `counter`는 마지막에 `_total`을 붙인다. 아래 표가 결과다. |
| 속성 변환 | 속성 이름의 `.`을 `_`로 바꿔 라벨 이름으로 쓴다(`cpu.mode` → `cpu_mode`). 속성 값은 그대로 라벨 값이다. |
| 종류 변환 | `counter` → `counter`, `updowncounter`·`gauge` → `gauge`, `histogram` → `histogram` |
| 계열 순서 | `target_info` 다음에 1.3절 표의 순서로 쓴다. 계열마다 `# HELP`와 `# TYPE` 줄을 먼저 쓴다. 표본이 없는 계열도 두 줄은 쓴다. |
| `# HELP` | 영어 한 줄 설명이다. 문구는 이 계약의 대상이 아니며 Consumer는 판정에 쓰지 않는다. |
| 표본 순서 | 한 계열 안에서 라벨 값을 1.3절 속성 순서대로 비교한 사전 순이다. |
| 라벨 쓰기 | 라벨은 1.3절의 속성 순서대로 쓴다. 값의 `\`는 `\\`, `"`는 `\"`, 줄바꿈은 `\n`으로 쓴다. 라벨이 없으면 중괄호를 쓰지 않는다. |
| 히스토그램 | 속성 조합마다 `_bucket{le="<경계>"}` 줄을 경계 순서대로 쓰고 `le="+Inf"`로 끝낸다. 버킷 값은 누적 개수다. 이어서 `_sum`과 `_count`를 쓴다. `le="+Inf"`의 값은 `_count`와 같다. 경계는 1.5절의 수를 소수 표기(`0.005`, `1`, `86400`)로 쓴다. |
| 수 표기 | 정수 값은 10진 정수로 쓴다. 소수 값은 같은 값으로 되읽히는 가장 짧은 10진 소수 표기다. 지수 표기·`NaN`을 쓰지 않는다. |
| 시각 | 표본에 시각(timestamp)을 붙이지 않는다. 수집 도구가 받은 시각을 쓴다. |
| 리소스 | `target_info` 계열(`gauge`) 하나에 리소스를 라벨로 싣는다. `target_info{service_name="vnc-relay",service_version="<버전>"} 1`이다. |

| OpenTelemetry 이름 | Prometheus 계열 이름 | `# TYPE` |
| --- | --- | --- |
| (리소스) | `target_info` | `gauge` |
| `vnc_relay.connection.accepted` | `vnc_relay_connection_accepted_total` | `counter` |
| `vnc_relay.connection.rejected` | `vnc_relay_connection_rejected_total` | `counter` |
| `vnc_relay.connection.aborted` | `vnc_relay_connection_aborted_total` | `counter` |
| `vnc_relay.connection.in_progress` | `vnc_relay_connection_in_progress` | `gauge` |
| `vnc_relay.relay_connection.usage` | `vnc_relay_relay_connection_usage` | `gauge` |
| `vnc_relay.relay_connection.limit` | `vnc_relay_relay_connection_limit` | `gauge` |
| `vnc_relay.admin_connection.usage` | `vnc_relay_admin_connection_usage` | `gauge` |
| `vnc_relay.admin_connection.limit` | `vnc_relay_admin_connection_limit` | `gauge` |
| `vnc_relay.handshake.duration` | `vnc_relay_handshake_duration_seconds` | `histogram` |
| `vnc_relay.waiting.count` | `vnc_relay_waiting_count` | `gauge` |
| `vnc_relay.waiting.duration` | `vnc_relay_waiting_duration_seconds` | `histogram` |
| `vnc_relay.session.count` | `vnc_relay_session_count` | `gauge` |
| `vnc_relay.session.started` | `vnc_relay_session_started_total` | `counter` |
| `vnc_relay.session.closed` | `vnc_relay_session_closed_total` | `counter` |
| `vnc_relay.session.duration` | `vnc_relay_session_duration_seconds` | `histogram` |
| `vnc_relay.session.io` | `vnc_relay_session_io_bytes_total` | `counter` |
| `vnc_relay.target_connect.duration` | `vnc_relay_target_connect_duration_seconds` | `histogram` |
| `vnc_relay.ticket.accepted` | `vnc_relay_ticket_accepted_total` | `counter` |
| `vnc_relay.websocket.ping_timeouts` | `vnc_relay_websocket_ping_timeouts_total` | `counter` |
| `vnc_relay.admin.logins` | `vnc_relay_admin_logins_total` | `counter` |
| `vnc_relay.admin.lockouts` | `vnc_relay_admin_lockouts_total` | `counter` |
| `vnc_relay.admin.terminations` | `vnc_relay_admin_terminations_total` | `counter` |
| `vnc_relay.admin_session.count` | `vnc_relay_admin_session_count` | `gauge` |
| `vnc_relay.config.reloads` | `vnc_relay_config_reloads_total` | `counter` |
| `vnc_relay.config.edits` | `vnc_relay_config_edits_total` | `counter` |
| `process.uptime` | `process_uptime_seconds` | `gauge` |
| `process.cpu.time` | `process_cpu_time_seconds_total` | `counter` |
| `process.memory.usage` | `process_memory_usage_bytes` | `gauge` |
| `process.memory.virtual` | `process_memory_virtual_bytes` | `gauge` |
| `process.thread.count` | `process_thread_count` | `gauge` |
| `process.unix.file_descriptor.count` | `process_unix_file_descriptor_count` | `gauge` |
| `process.windows.handle.count` | `process_windows_handle_count` | `gauge` |

### 1.7 JSON 표현

관리 API `GET /api/v1/metrics`의 `200` 응답 본문이다. 경로·인증·질의 매개변수·오류는 [관리 API 계약 1.16절](management-api-v1.md#116-메트릭-get-apiv1metrics)이 정한다. 값은 같은 시점의 Prometheus 텍스트 표현과 같다. 시각·필드 이름의 공통 규칙은 [관리 API 계약 1.1절](management-api-v1.md#11-공통-규칙)을 따른다.

| 필드 | 자료형 | 필수 여부·null 허용 여부 | 의미 |
| --- | --- | --- | --- |
| `as_of` | `timestamp` | 필수, null 불가 | 응답을 만든 시점 |
| `resource` | 객체 | 필수, null 불가 | 1.2절의 리소스. 키는 `service.name`, `service.version`이고 값은 문자열이다. |
| `metrics` | 객체 배열 | 필수, null 불가 | 1.3절 표의 순서. 이 플랫폼에서 내는 메트릭이 모두 들어 있다. |
| `metrics[].name` | 문자열 | 필수, null 불가 | 1.3절의 OpenTelemetry 이름 |
| `metrics[].type` | 문자열 | 필수, null 불가 | `counter`, `updowncounter`, `gauge`, `histogram` |
| `metrics[].unit` | 문자열 | 필수, null 불가 | 1.3절의 단위(`s`, `By`, `{connection}` 등) |
| `metrics[].available` | 불리언 | 필수, null 불가 | 값을 읽었으면 `true`. 읽지 못했으면 `false`이고 `points`는 빈 배열이다. |
| `metrics[].bounds` | 수 배열 | 히스토그램에만 필수, null 불가 | 1.5절의 경계. 다른 종류에는 이 필드가 없다. |
| `metrics[].points` | 객체 배열 | 필수, null 불가 | 속성 조합마다 하나. 순서는 1.6절의 표본 순서와 같다. |
| `points[].attributes` | 객체 | 필수, null 불가 | 속성 이름(1.3절의 원래 이름, 예: `cpu.mode`)과 문자열 값. 속성이 없으면 빈 객체다. |
| `points[].value` | 수 | 히스토그램이 아니면 필수, null 불가 | 정수 값은 정수로, `s` 단위 값은 소수 초로 쓴다. |
| `points[].count` | 정수 | 히스토그램에만 필수, null 불가 | 기록한 값의 개수 |
| `points[].sum` | 수 | 히스토그램에만 필수, null 불가 | 기록한 초 값의 합 |
| `points[].bucket_counts` | 정수 배열 | 히스토그램에만 필수, null 불가 | 구간별 개수. 길이는 `bounds` 길이 + 1이다. 누적이 아니다. i번째 값은 `bounds[i-1] < v ≤ bounds[i]`인 개수이며 마지막 값은 마지막 경계보다 큰 값의 개수다. 합은 `count`와 같다. |
| `history` | 객체 | 필수, null 불가 | 1.8절의 추이. 요청이 고른 기간 하나다. |

### 1.8 추이

추이(history)는 서버가 고정 간격으로 찍은 일부 값의 기록이다. 대시보드가 최근 1시간이나 최근 24시간의 차트를 그리는 데 쓴다. 외부 수집 도구용 장기 기록이 아니다. 관리 API는 요청이 고른 기간의 추이 하나를 준다([관리 API 계약 1.16절](management-api-v1.md#116-메트릭-get-apiv1metrics)).

| 항목 | 규칙 |
| --- | --- |
| 기간 | 최근 1시간(`1h`)과 최근 24시간(`24h`) 두 가지다. 기간마다 점을 따로 보관하고 순번을 따로 센다. |
| 간격과 개수 | `1h`는 10초마다 한 점을 찍고 최근 360개만 메모리에 둔다. `24h`는 `1h` 점 6개마다 한 점, 곧 1분마다 한 점을 찍고 최근 1440개만 메모리에 둔다. 가득 차면 가장 오래된 점부터 지운다. 설정 키는 없다. |
| 두 기간의 관계 | `24h`의 점은 `seq`가 6의 배수인 `1h` 점과 같은 시각에 찍으며 값도 같다. 여러 점의 평균이나 합을 내지 않는다. 기록이 늦어 `1h`의 간격이 길어지면 `24h`의 간격도 함께 길어진다. |
| 첫 점 | 서버가 준비됨이 된 때 두 기간의 첫 점을 함께 찍는다. 그 전에는 점이 없다. 정상 종료 유예 중에도 계속 찍는다. |
| 순번 | 각 점의 `seq`는 기간마다 0부터 시작해 점마다 1씩 늘어난다. 지운 점의 순번을 다시 쓰지 않는다. 재시작하면 두 추이를 비우고 0부터 다시 센다. |
| 값 | 한 점의 값은 그 시점에 1.7절 응답을 만들 때와 같은 방법으로 읽는다. 읽지 못한 값은 `null`이다. |

`history` 객체의 필드:

| 필드 | 자료형 | 필수 여부·null 허용 여부 | 의미 |
| --- | --- | --- | --- |
| `interval_secs` | 정수 | 필수, null 불가 | 기간의 기록 간격(초). `1h`이면 `10`, `24h`이면 `60`이다. |
| `capacity` | 정수 | 필수, null 불가 | 기간의 최대 보관 개수. `1h`이면 `360`, `24h`이면 `1440`이다. |
| `latest_seq` | 정수 | 필수, null 허용 | 그 기간에서 보관 중인 가장 최근 점의 `seq`. 점이 없으면 `null`이다. |
| `points` | 객체 배열 | 필수, null 불가 | 요청 조건에 맞는 점. `seq` 오름차순이다. |
| `points[].seq` | 정수 | 필수, null 불가 | 위 순번 |
| `points[].at` | `timestamp` | 필수, null 불가 | 점을 찍은 시각 |
| `points[].values` | 객체 | 필수, null 불가 | 아래 계열 키를 모두 가진다. 값은 수 또는 `null`이다. |

| 계열 키 | 값 | 종류 |
| --- | --- | --- |
| `session_count` | `vnc_relay.session.count`의 모든 점의 합 | 현재 값 |
| `waiting_count` | `vnc_relay.waiting.count`의 모든 점의 합 | 현재 값 |
| `connection_accepted` | `vnc_relay.connection.accepted`의 모든 점의 합 | 누적 값 |
| `connection_rejected` | `vnc_relay.connection.rejected`의 모든 점의 합 | 누적 값 |
| `connection_aborted` | `vnc_relay.connection.aborted`의 모든 점의 합 | 누적 값 |
| `session_io_viewer_to_target` | `vnc_relay.session.io{direction=viewer_to_target}` | 누적 값(바이트) |
| `session_io_target_to_viewer` | `vnc_relay.session.io{direction=target_to_viewer}` | 누적 값(바이트) |
| `process_cpu_time` | `process.cpu.time`의 두 점의 합(소수 초) | 누적 값 |
| `process_memory_usage` | `process.memory.usage`(바이트) | 현재 값 |

누적 값 계열의 구간 증가량과 초당 비율은 Consumer가 이웃한 두 점의 차이로 계산한다.

## 2. 오류

`/metrics`의 응답 코드다. 관리 API 경로의 오류는 [관리 API 계약 2절](management-api-v1.md#2-오류)이 정한다.

| 오류 코드 | 의미·형식 | 관련 스펙 시나리오 |
| --- | --- | --- |
| `not_found` | `[metrics] enabled = false`일 때 `/metrics`. 404와 [운영 정보 계약 2절](ops-info-v2.md#2-오류)의 JSON 오류 본문이다. 정의하지 않은 경로와 같은 응답이다. | OPS-082 |
| `method_not_allowed` | 켜져 있을 때 `GET`이 아닌 메서드. 405와 `Allow: GET` 헤더, 운영 정보 계약 2절의 JSON 오류 본문이다. | OPS-084 |

Lm 상한에서 거절한 연결은 HTTP 응답을 받지 못한다([운영 정보 계약 2절](ops-info-v2.md#2-오류)과 같다).

## 3. 호환성 규칙

- 수집 도구는 모르는 계열·라벨·라벨 값을 오류로 처리하지 않아야 한다. JSON Consumer는 모르는 필드·메트릭·속성 값과 `type` 값을 무시하거나 받은 그대로 보여 줘야 한다.
- 수집 도구는 `# HELP` 문구로 분기하지 않아야 한다.
- 호환 변경: 메트릭 추가, 1.4절 속성 값 추가(운영 정보 계약에 원인 값이 추가된 경우 포함), JSON 응답에 필드 추가, 추이 계열 키 추가, 추이 기간 추가, `# HELP` 문구 변경. 1.2.0의 `ip_denied` 값 추가는 이 범위다.
- 호환성을 깨는 변경(Breaking change): 메트릭·속성의 이름 변경이나 삭제, 종류·단위·의미 변경, 히스토그램 경계 변경, 기록 시점의 의미 변경, 기존 추이 기간의 간격·개수와 계열 키의 의미 변경, 텍스트 형식 버전 변경. 이런 변경은 새 major 버전(`vnc-relay-metrics` 2.0.0)과 함께 한다. 수집 도구의 질의·경보 규칙을 새 이름으로 먼저 옮긴 뒤 Producer를 올린다. 롤백은 이전 실행파일로 되돌리는 것이다. 값은 메모리에만 있으므로([ADR-0003](../decisions/ADR-0003-single-instance-memory-state.md)) 데이터 되돌림은 필요 없다.
- 운영 정보 계약의 원인 값 이름이 바뀌는 major 변경은 이 계약의 속성 값도 바꾸므로 이 계약의 major 변경과 함께 한다.

## 4. 버전과 지원 단계

| 상태 | 조건 | 지원 종료·대체 |
| --- | --- | --- |
| Draft | 채택 전 상태. | 해당 없음 |
| Active | 현재 `vnc-relay-metrics` 1.2.0. Producer 계약 테스트의 현재 결과는 5절의 방법으로 확인한다. 통과 전에는 지원 완료로 표시하지 않는다. 저장소에 외부 수집 도구가 없어 Consumer 시험은 대시보드 시험만 해당한다. | 새 major가 Active가 될 때까지 유지한다. |
| Deprecated | 새 major가 있고 이전 major를 쓰는 소비자가 남아 있다. | 신규 적용을 피하고 대체 문서를 확인한다. |
| Retired | 지원 종료. | 새 구현의 근거로 쓰지 않는다. |

## 5. 예제와 계약 테스트

예제의 값은 문서용 값이다. 실행 결과가 아니다. 다음은 켜진 `/metrics` 응답의 일부다. 줄 사이의 `...`는 생략 표시이며 응답에 나오지 않는다.

```text
# HELP target_info Target metadata
# TYPE target_info gauge
target_info{service_name="vnc-relay",service_version="2026.11.6"} 1
# HELP vnc_relay_connection_accepted_total Connections accepted by relay listeners.
# TYPE vnc_relay_connection_accepted_total counter
vnc_relay_connection_accepted_total{listener="tcp_server"} 4
vnc_relay_connection_accepted_total{listener="wss"} 7
# HELP vnc_relay_connection_rejected_total Connections rejected by policy, limit or format.
# TYPE vnc_relay_connection_rejected_total counter
vnc_relay_connection_rejected_total{listener="tcp_viewer",stage="accept",reason="ip_not_allowed"} 2
vnc_relay_connection_rejected_total{listener="wss",stage="upgrade",reason="ip_denied"} 1
vnc_relay_connection_rejected_total{listener="wss",stage="upgrade",reason="ticket_expired"} 1
...
# HELP vnc_relay_handshake_duration_seconds Repeater initial exchange duration.
# TYPE vnc_relay_handshake_duration_seconds histogram
vnc_relay_handshake_duration_seconds_bucket{listener="tcp_server",mode="mode2",le="0.005"} 3
vnc_relay_handshake_duration_seconds_bucket{listener="tcp_server",mode="mode2",le="0.01"} 4
...
vnc_relay_handshake_duration_seconds_bucket{listener="tcp_server",mode="mode2",le="120"} 4
vnc_relay_handshake_duration_seconds_bucket{listener="tcp_server",mode="mode2",le="+Inf"} 4
vnc_relay_handshake_duration_seconds_sum{listener="tcp_server",mode="mode2"} 0.0121
vnc_relay_handshake_duration_seconds_count{listener="tcp_server",mode="mode2"} 4
...
# HELP vnc_relay_session_io_bytes_total Bytes written to the opposite transport by active sessions.
# TYPE vnc_relay_session_io_bytes_total counter
vnc_relay_session_io_bytes_total{direction="target_to_viewer"} 4194304
vnc_relay_session_io_bytes_total{direction="viewer_to_target"} 10240
...
# HELP process_cpu_time_seconds_total CPU time used by the process.
# TYPE process_cpu_time_seconds_total counter
process_cpu_time_seconds_total{cpu_mode="system"} 0.42
process_cpu_time_seconds_total{cpu_mode="user"} 1.37
```

다음은 `GET /api/v1/metrics?history_after=40` 응답의 일부다. 기간을 고르지 않았으므로 최근 1시간 추이다. `history_window=24h`로 조회하면 `interval_secs`는 `60`, `capacity`는 `1440`이고 점은 1분 간격이다.

```json
{
  "as_of": "2026-11-19T01:02:03.456Z",
  "resource": {"service.name": "vnc-relay", "service.version": "2026.11.6"},
  "metrics": [
    {"name": "vnc_relay.connection.accepted", "type": "counter", "unit": "{connection}", "available": true,
     "points": [{"attributes": {"listener": "tcp_server"}, "value": 4}, {"attributes": {"listener": "wss"}, "value": 7}]},
    {"name": "vnc_relay.handshake.duration", "type": "histogram", "unit": "s", "available": true,
     "bounds": [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10, 30, 60, 120],
     "points": [{"attributes": {"listener": "tcp_server", "mode": "mode2"}, "count": 4, "sum": 0.0121,
                 "bucket_counts": [3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]}]},
    {"name": "process.memory.usage", "type": "updowncounter", "unit": "By", "available": false, "points": []}
  ],
  "history": {
    "interval_secs": 10,
    "capacity": 360,
    "latest_seq": 41,
    "points": [
      {"seq": 41, "at": "2026-11-19T01:02:00.000Z",
       "values": {"session_count": 1, "waiting_count": 2, "connection_accepted": 11, "connection_rejected": 3,
                  "connection_aborted": 0, "session_io_viewer_to_target": 10240, "session_io_target_to_viewer": 4194304,
                  "process_cpu_time": 1.79, "process_memory_usage": null}}
    ]
  }
}
```

| 범위 | 테스트 위치 |
| --- | --- |
| 텍스트 표현(이름 변환·순서·라벨 쓰기·히스토그램 누적·수 표기) | `crates/adapters/src/metrics_text.rs`의 단위 시험 |
| 사건에서 세는 값·직접 기록 값·추이 | `crates/app/src/metrics.rs`와 하위 모듈의 단위 시험 |
| 상태 리스너 `/metrics`의 켜짐·꺼짐·메서드·헤더 | `crates/adapters/tests/ops_info_contract.rs` |
| 실제 프로세스의 사건 수·노출 경계·프로세스 값 | `tests/metrics.rs` |
| 관리 API JSON·추이 증분 조회 | `tests/management_query.rs`, `crates/adapters/tests/management.rs` |
| 대시보드 SCR-007 | `tests/dashboard_browser.rs`의 `web/dashboard/tests/metrics-scenarios.js` |

결과와 증거는 [검증 기록](../AGENTS.md#검증-기록)을 따른다. 현재 시나리오별 결과는 `cargo xtask scenarios`로 만든 `scenario-report.md`에서 확인한다.

## 6. 초안을 확정하기 전에 해결할 사항

미결정 사항은 없다. 이 계약이 정하지 않는 범위는 다음과 같다.

- OTLP 내보내기(push), 표본 시각, exemplar, 네이티브 히스토그램은 제공하지 않는다.
- OpenMetrics 1.0 텍스트 형식(`application/openmetrics-text`) 협상은 하지 않는다. `Accept` 헤더와 관계없이 0.0.4로 응답한다.
- 외부 수집 도구의 수집 주기는 정하지 않는다. 운영 안내의 예는 15초다.
