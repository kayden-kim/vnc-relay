# 운영 로그와 상태 엔드포인트 v2 계약

VNC Relay가 운영자에게 내보내는 두 가지 정보의 교환 형식을 정한다. 하나는 JSON Lines 운영 로그이고, 다른 하나는 상태 리스너의 `/livez`·`/readyz` 응답이다. 로그 수집 도구와 상태 확인 도구가 이 문서의 표를 형식의 기준으로 쓴다. 외부 동작은 [운영 스펙](../specs/operations.md#35-운영-정보)이 소유하며, 이 문서는 그 동작을 바꾸지 않는다. 같은 상태 리스너의 `/metrics`는 [메트릭 계약](metrics-v1.md)이 정한다.

현재 계약은 `ops-info` 2.3.0이다. 파일 이름의 v2는 major 버전을 뜻한다.

| 항목 | 값 |
| --- | --- |
| 현재 이름과 버전 | `ops-info` 2.3.0. 출발지 IP 차단 목록의 거절·끊김 원인 값 `ip_denied`와 설정 오류 코드 `config_deny_overlaps_proxy`는 3절의 호환 추가 범위다. 2.2.0의 IP 정보 DB 사건(`ip_db_loaded`·경고 `ip_db_unavailable`)과 그 필드·원인 값, 2.1.0의 접속 ticket 사건·필드·원인 값·설정 경고도 같은 범위다. |
| 2.0.0과 1.x의 차이 | `listener`는 소켓 단위이며 WS·WSS의 경로 판정 전 `role`은 생략한다. 필드 의미와 필수 여부가 다른 major 변경이다. 전달 주소·Mode II `server_transport`·새 거절 원인·v4 설정 키 이름을 포함한다. |
| 1.x 버전 차이 | 1.1.0은 `startup_failed.reason`에 `runtime_unavailable`을 추가한다. 1.2.0은 `log_file_unavailable`, 로그 쓰기 실패 처리와 진단마다 한 줄인 `startup_failed`를 포함한다. 1.3.0은 `connection_aborted.stage=register`와 강제 정리 중 기존 `close_cause` 보존 규칙을 포함한다. 모두 호환 변경이다. |
| 1.3.0의 호환 추가 범위 | 수락 소켓의 keepalive·`TCP_NODELAY` 설정 실패 기록, `admin_terminate.result=already_closed`, 관리자 E 만료의 `side`·계정 표기, `internal_error`, Windows 콘솔 창 닫기 조건, `log_reopen_failed`, 필수 리스너 장애 종료, `config_edited`·`config_edit_rejected`·`revision`·`reload_applied.trigger`, 재로드 대상 27개와 `allow_all_target.phase=reload`가 포함된다. |
| Windows 서비스 종료·출력 | 서비스 중지 요청은 `shutdown_started(trigger=requested)`이며 G를 적용한다. 서비스 실행 중 시스템 종료 알림은 `requested`이며 G 없이 남은 세션을 강제 정리한다. 로그 출력은 파일만이다. 새 사건·필드·값은 없다. |
| 상태 | Active |
| 소유자 | vnc-relay 프로젝트 관리자 |
| 만드는 쪽(Producer) | `vnc-relay` (`vnc-relay serve` 프로세스와 Windows 서비스로 실행한 `vnc-relay service run` 프로세스) |
| 사용하는 쪽(Consumer) | 운영자의 로그 수집 도구, 운영자의 상태 확인 도구(상태 점검·감시 도구), Windows 트레이 앱(`/readyz`만, [Windows 트레이 앱 스펙](../specs/windows-tray.md)) |
| 방향 | 로그: 발행(Producer가 표준 출력 또는 지정한 파일에 한 줄씩 씀). 상태: 요청/응답(Consumer가 상태 리스너에 `GET`) |
| 스키마 기준 파일 | 없음. 1절의 Markdown 표가 형식의 기준이다. |
| 기준 스펙 | [운영 스펙 3.5절](../specs/operations.md#35-운영-정보), [4절](../specs/operations.md#4-보안과-신뢰-경계), [5절](../specs/operations.md#5-오류와-복구), [중계 스펙](../specs/relay.md), [WS·WSS 스펙](../specs/websocket.md), [관리 스펙](../specs/management.md) |
| 관련 요구사항 | [PRD](../requirements/PRD.md) FR-08, FR-07, 7절 보안, AC-23, AC-25 |
| 호환성 범위 | 이 계약의 2.x 형식을 따르는 `vnc-relay` 구현과 소비자. 지원 판정은 Producer 시험을 따른다. 저장소에 없는 외부 소비 도구의 시험은 대상이 아니다. |
| 관련 테스트 | [운영 스펙 7절](../specs/operations.md#7-수용-테스트)의 OPS-011·OPS-012·OPS-013·OPS-032를 중심으로 5절 표에 연결한다. 계약 테스트는 `crates/adapters/tests/ops_info_contract.rs`와 `tests/serve.rs`에 있다. 접속 ticket의 WS-038~045·RLY-029~032·OPS-070~073·MGT-052와 IP 정보 DB의 OPS-096~097, 출발지 IP 차단 목록의 OPS-100·OPS-101·RLY-033·RLY-034·WS-047·WS-048은 5절의 계약 시험에 연결한다. |

관련 문서: [설정 계약](config-v4.md)(`[log]`·`[status]`·`[ip_info]`·`[relay.deny]` 키, 경고 코드, 오류 코드, 재로드 결과 코드, IP 정보 DB 원인 코드), [관리 API 계약](management-api-v1.md)(`session_id`·`close_cause`·바이트 필드 이름), [CLI·계정 파일 계약](cli-v1.md)(시작 실패 오류 코드), [WebSocket 중계 계약](websocket-relay-v2.md)(close 코드와 종료 원인 대응), [Repeater 초기 교환 계약](repeater-handshake-v1.md)(초기 교환 오류 코드), [메트릭 계약](metrics-v1.md)(상태 리스너의 `/metrics`와 이 계약의 사건에서 세는 값), [ADR-0013 허용 목록과 공통 차단 목록](../decisions/ADR-0013-relay-source-deny-list.md), [ADR-0003 단일 인스턴스·메모리 상태](../decisions/ADR-0003-single-instance-memory-state.md), [ADR-0009 접속 ticket](../decisions/ADR-0009-optional-relay-access-ticket.md), [ADR-0012 IP 정보 DB](../decisions/ADR-0012-local-ip-info-databases.md).

## 1. 형식과 의미

이 계약의 필드·사건 이름·level·원인 코드·중계 ID 참조 방식·상대 주소·상태 응답 형식은 확정된 형식이다. 접속 ticket 확인·사용 기록·세션 만료 조건은 [WebSocket 계약 1.9절](websocket-relay-v2.md#19-접속-ticket)이 정한다. 이 문서는 로그 형식만 정한다.

- 로그는 JSON Lines이고 한 줄에 사건 하나를 쓴다. 출력 위치는 표준 출력 또는 지정한 파일이며 VNC Relay는 출력만 한다. 관리자 종료 사건에는 계정명을 기록한다. 평문 WS 활성화와 전체 허용 항목은 경고로 기록한다. 상태 리스너는 관리 리스너와 분리하며 인증 없이 생존·준비 결과만 응답한다. 상태 응답에 세션 목록·중계 ID·상대 주소·인증 비밀을 넣지 않는다.

### 1.1 로그 일반 규칙

| 규칙 | 내용 |
| --- | --- |
| 줄 | 한 줄에 JSON 객체 하나를 쓴다. 객체는 줄 안에서 끝난다. 값 안의 줄바꿈은 JSON 이스케이프로 쓴다. |
| 줄 끝 | LF(`\n`)이다. Windows 11 x64에서도 같다. |
| 인코딩 | UTF-8이다. |
| 줄 섞임 | 서로 다른 사건의 내용이 한 줄 안에서 섞이지 않는다. |
| 필드 이름 | `snake_case`다. 사건 이름과 원인 코드도 `snake_case`다. |
| 선택 필드 | 값이 없으면 필드를 생략한다. `null`을 쓰지 않는다. |
| 순서 | 한 줄 안의 필드 순서는 의미가 없다. 같은 연결·세션의 사건은 발생 순서대로 기록한다. |
| 문자열 길이 | `detail`은 512바이트, `origin`·`target_host`는 256바이트까지 쓴다. 넘는 부분은 잘라낸다. |
| 출력 위치 | 설정(`[log] output`)이 정한다. 설정을 읽기 전에 생긴 사건(설정 파일 오류로 인한 `startup_failed`)과 로그 파일을 열지 못한 사건(`log_file_unavailable`의 `startup_failed`)은 표준 출력에 쓴다. |
| 출력 실패 | 줄을 쓰지 못하면 그 줄을 버리고 서비스를 계속한다. 처음 실패할 때만 stderr에 `warning[log_write_failed] key=log.output: <message>`를 한 줄 쓴다([CLI 계약 1.5절](cli-v1.md#15-입력과-출력-규칙)의 형식). 이후 실패는 알리지 않는다. |
| 파일 다시 열기 | 출력 위치가 파일이면 같은 경로를 다시 열어 그 경로의 파일에 이어 쓴다. 파일이 없으면 만든다. 경로의 이름 변경·삭제·생성 알림을 받으면 곧바로, 그리고 `reload.poll_secs`(Dp)마다 항상 다시 연다. 다시 열기 전에 쓴 줄은 이전 파일에, 다시 연 뒤의 줄은 새 파일에 있다. 한 줄이 두 파일에 나뉘거나 겹치지 않는다. 다시 열기에 성공하면 사건을 쓰지 않는다. 다시 열기에 실패하면 이전 파일에 계속 쓰고, 다음 알림·주기에 다시 시도한다. 실패가 시작될 때만 stderr에 `warning[log_reopen_failed] key=log.output: <message>`를 한 줄 쓰고 `log_reopen_failed` 사건을 이전 파일에 쓴다. 다시 열기에 성공한 뒤 다시 실패하면 새 실패로 보고 한 번 더 알린다. 표준 출력이면 다시 열지 않는다. |

### 1.2 공통 필드

모든 줄이 가진다.

| 필드 | 자료형 | 필수 여부 | 의미·제약 |
| --- | --- | --- | --- |
| `ts` | 문자열 | 필수 | 사건 발생 시각. UTC ISO 8601, 밀리초 정밀도, 끝에 `Z`를 붙인다. 예: `2026-10-16T01:02:03.456Z`. |
| `level` | 문자열 | 필수 | 1.3절의 값 중 하나. |
| `event` | 문자열 | 필수 | 1.5절의 사건 이름. |

### 1.3 `level` 값

| 값 | 쓰는 경우 |
| --- | --- |
| `info` | 정상 운영 사건. 정책에 따른 정상 종료도 포함한다. |
| `warn` | 서비스는 계속하지만 운영자가 볼 만한 사건. 거절, 제한 도달, 비정상 종료 원인, 재로드 실패·재시작 필요, 설정 경고가 해당한다. |
| `error` | 시작 실패, 계속 운영할 수 없는 실패, 변경 감지 장애, 관리자 종료 실패. |

`debug` 같은 다른 값은 2.1.0에 없다. 사건마다 `level`은 1.5절 표가 정한다. 종료 원인에 따라 달라지는 사건은 1.6절 표를 따른다.

### 1.4 필드 사전

사건 표에서 쓰는 필드의 뜻이다. 필드 이름 뒤 `?`는 선택 필드다. 같은 이름은 모든 사건에서 같은 뜻과 자료형을 가진다.

| 필드 | 자료형 | 의미 |
| --- | --- | --- |
| `conn_id` | 문자열 | 연결 식별자. 중계 리스너가 접속을 수락할 때 붙인다. 세션 식별자와 다른 값이며 프로세스 수명 안에서만 유일하다. 재시작 뒤 같은 값이 다시 나올 수 있다. |
| `session_id` | 문자열 | 세션 식별자. 관리 API 계약의 `session_id`와 같은 값이다. 형식은 관리 API 계약이 정한다([관리 스펙](../specs/management.md)은 외부 형식을 API 계약에 맡긴다). 세션이 생기기 전 사건에는 쓰지 않는다. |
| `listener` | 문자열 | 소켓 단위의 리스너. `tcp_viewer`, `tcp_server`, `ws`, `wss`, `management`, `status` 중 하나다. 중계 진입점은 `listener`와 `role`의 쌍이다. |
| `role` | 문자열 | `viewer` 또는 `server`. TCP 중계 리스너의 역할은 고정이다. `ws`·`wss`에서는 경로가 역할을 정한 뒤에만 쓴다. 수락·TLS·경로 판정 전 업그레이드 단계에서는 생략한다. |
| `mode` | 문자열 | `mode1` 또는 `mode2`. 초기 교환으로 모드를 알게 된 뒤의 사건에만 쓴다. |
| `viewer_transport` | 문자열 | 뷰어 전송 경로. `tcp`, `ws`, `wss`. 관리 API 계약의 `viewer_transport`와 같은 이름·값이다. |
| `server_transport` | 문자열 | Mode II 서버 전송 경로. `tcp`, `ws`, `wss` 중 하나다. Mode I에서는 생략한다. |
| `peer_ip` | 문자열 | 실제 소켓 상대 IP. 표기는 1.10절을 따른다. |
| `forwarded_ip` | 문자열 | 신뢰 프록시의 전달 주소. WS·WSS 연결에서 주소 도출에 성공한 뒤 `peer_ip`를 가진 모든 사건에 쓴다. 그 전이나 프록시를 신뢰하지 않으면 생략한다. 표기는 1.10절을 따른다. |
| `viewer_forwarded_ip`, `server_forwarded_ip` | 문자열 | 각각 뷰어·Mode II 서버의 전달 주소. 해당 쪽의 `viewer_peer_ip`·`server_peer_ip`가 있는 사건에 주소를 도출한 경우에만 함께 쓴다. Mode I에서는 `server_forwarded_ip`를 생략한다. |
| `relay_id_ref` | 문자열 | 중계 ID의 참조 값. 중계 ID 원문이 아니다. 1.9절을 따른다. |
| `ticket_jti` | 문자열 | 접속 ticket의 `jti`. `ticket_accepted`에서 필수다. 거절 사건의 기록 조건은 1.5·1.6절을 따른다. |
| `ticket_sub` | 문자열 | 접속 ticket의 선택 클레임 `sub`. 값이 없으면 생략한다. 발급 쪽은 비밀이나 개인정보를 넣으면 안 된다([WebSocket 계약 1.9.7절](websocket-relay-v2.md#197-발급-쪽과-클라이언트의-의무)). |
| `session_expires_at` | 문자열 | 접속 ticket의 선택 클레임 `sx`가 정한 세션 만료 시각. `ts`와 같은 UTC ISO 8601 형식(밀리초, `Z`)이다. `sx`가 없으면 생략한다. `sx`가 `253402300799`(9999-12-31T23:59:59Z)보다 크면 네 자리 연도로 나타낼 수 없으므로 생략한다. 이 경우에도 ticket 확인과 세션 만료 타이머는 `sx`를 그대로 쓴다. |
| `reason` | 문자열 | 사건이 일어난 이유의 코드. 값 목록은 사건마다 1.6절 이후 표가 정한다. |
| `error_kind` | 문자열 | 연결 오류의 세부 종류. 1.7절의 값. |
| `side` | 문자열 | 원인을 처음 낸 쪽. `viewer`, `server`, `target`(Mode I 대상) 중 하나. |
| `admin_account` | 문자열 | 관리자 계정명. 1.8절의 규칙을 따른다. |
| `config_key` | 문자열 | 문제가 있는 설정 키. [설정 계약 2.1절](config-v4.md#21-설정-오류와-준비-실패)의 진단 `key`와 같은 값이다(점으로 이은 전체 경로, 목록 항목은 0부터 센 `[i]` 첨자. 예: `relay.tcp_viewer.allow[2]`). 키 이름만 쓰고 값은 쓰지 않는다. |
| `detail` | 문자열 | 사람이 읽는 짧은 설명. 비밀과 설정 값을 넣지 않는다. 수집 도구는 이 필드로 분기하지 않는다. |
| `elapsed_ms`, `waited_ms`, `duration_ms` | 정수 | 밀리초. 단조 시계로 잰 경과 시간이다. 음수가 되지 않는다. |
| `limit` | 정수 | 거절 원인이 된 상한의 설정값. |
| `phase` | 문자열 | `startup` 또는 `reload`. |
| `trigger` | 문자열 | 사건을 시작한 경로. `shutdown_started`는 `requested`·`fatal_error`, `reload_applied`는 `file_watch`·`config_edit`다. |
| `revision` | 문자열 | 설정 파일 바이트의 SHA-256. `sha256:` 접두사 뒤 소문자 16진수 64자다. `config_edited`에서는 교체한 새 파일의 값이다([설정 계약 1.10절](config-v4.md#110-관리-api-편집의-파일-쓰기)). |
| `config_keys` | 문자열 배열 | 값이 다른 키 또는 편집 요청·결과의 키 경로. 사건 표가 정한 범위만 쓰며, 키의 값은 넣지 않는다. |
| `database_type` | 문자열 | IP 정보 DB 파일 메타데이터의 `database_type`. 128자를 넘으면 앞 128자만 쓴다. |
| `build_time` | 문자열 | IP 정보 DB 파일 메타데이터의 빌드 시각. `ts`와 같은 UTC ISO 8601 형식(밀리초, `Z`)이다. |

### 1.5 사건 목록

각 사건은 아래 표에 있다. "스펙 근거"는 사건을 기록하라고 요구하거나 사건의 조건을 정한 스펙 시나리오 ID다. 시나리오 ID는 [운영](../specs/operations.md#7-수용-테스트)(OPS), [중계](../specs/relay.md#7-수용-테스트)(RLY), [WS·WSS](../specs/websocket.md#7-수용-테스트)(WS), [관리](../specs/management.md#7-수용-테스트)(MGT) 스펙의 수용 테스트 표에 있다.

#### 시작·준비·종료 단계

단계 이름은 [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)의 상태에 대응한다.

| 사건 | `level` | 발생 시점 | 필드 | 스펙 근거 |
| --- | --- | --- | --- | --- |
| `server_starting` | `info` | 설정이 유효하고 로그 출력을 연 뒤 시작 중 상태로 들어갔을 때 | `log_contract`: 문자열. 2.3.0 Producer의 값은 `ops-info/2.3.0`이다. | OPS-001, OPS-010, OPS-069 |
| `server_ready` | `info` | 필수 리스너와 접근 경계가 모두 준비돼 준비됨 상태로 들어갔을 때 | `listeners`: 객체 배열. 각 객체는 `name`(1.4절의 `listener` 값)과 `listen`(바인딩 주소 문자열)을 가진다. 공유 WS·WSS 리스너는 역할별로 나누지 않는다. | OPS-002, OPS-013, OPS-032, OPS-069 |
| `startup_failed` | `error` | 시작 실패가 확정됐을 때. 자원을 회수한 뒤 프로세스가 끝난다. | `reason`(시작을 막은 오류 코드, 1.6절), `config_key?`, `listener?`, `detail?`. | OPS-001, OPS-002, OPS-028, OPS-034, OPS-038, OPS-039, OPS-040, OPS-070 |
| `shutdown_started` | `info`. `trigger`가 `fatal_error`이면 `error` | 종료 유예 중 상태로 들어갔을 때. 이후 신규 중계 접속을 받지 않는다. Windows 콘솔 창 닫기도 `requested`로 쓴다. Windows 서비스의 중지 요청과 서비스로 실행 중의 시스템 종료 알림도 `requested`로 쓴다. | `trigger`: `requested` 또는 `fatal_error`. `grace_secs`: 정수(G). `active_sessions`: 정수. `detail?` | OPS-007, OPS-008, OPS-044, OPS-076, OPS-078 |
| `shutdown_forced` | `warn` | G가 지나거나, 유예 중 두 번째 종료 요청을 받거나, Windows 콘솔 창이 닫히거나, 서비스로 실행 중 시스템 종료 알림을 받아 남은 활성 세션을 강제 정리하기 시작했을 때. 남은 세션이 없으면 기록하지 않는다. | `remaining_sessions`: 정수. | OPS-009, OPS-042, OPS-044, OPS-078 |
| `server_stopped` | `info` | 자원 정리를 마치고 프로세스가 끝나기 직전 | 없음 | OPS-010 |

순서 규칙: `server_starting`은 시작 단계의 첫 사건이다. 시작 경고(아래 "설정 경고"의 사건)와 `phase = startup`인 IP 정보 DB 사건은 `server_starting` 뒤, `server_ready` 앞에 쓴다. `shutdown_forced`는 `server_shutdown` 원인의 `session_closed`보다 앞에 쓴다. `server_stopped`는 마지막 줄이다.

#### 설정 경고

사건 이름은 [설정 계약 2.2절](config-v4.md#22-경고)의 경고 코드와 같다. 발생 조건과 전체 허용 항목의 판정 범위는 그 절과 [1.3절](config-v4.md#13-ip-항목-문법)이 정한다. 이 계약은 필드만 정한다. `item`은 항목 표기만 담으며 비밀이 아니다.

| 사건 | `level` | 필드 | 스펙 근거 |
| --- | --- | --- | --- |
| `plaintext_ws_enabled` | `warn` | `listener`: 항상 `ws`. `listen`: 바인딩 주소 문자열. WS 역할이 하나라도 켜졌을 때 기록한다. | OPS-016, WS-024 |
| `allow_all_source` | `warn` | `listener`: `tcp_viewer`, `tcp_server`, `ws`, `wss`. `item`: 항목 문자열. `phase`. WS·WSS 역할 목록이면 `role`(`viewer`·`server`)이 필수다. TCP에서는 생략한다. 진입점과 항목마다 한 줄이다. | OPS-038, OPS-069 |
| `trust_all_proxies` | `warn` | `listener`: `ws` 또는 `wss`. `item`: 항목 문자열. `phase`: `startup` 또는 `reload`. 신뢰 프록시 목록에 전체 허용 항목이 있을 때 리스너와 항목마다 기록한다. | OPS-067 |
| `allow_all_target` | `warn` | `item`: 대상 항목 문자열. `phase`: `startup` 또는 `reload`. 항목마다 한 줄이다. | RLY-007, OPS-058 |
| `status_listen_not_loopback` | `warn` | `listen`: 바인딩 주소 문자열. | OPS-032 |
| `ticket_tcp_bypass` | `warn` | `role`(`viewer`·`server`), `listener`(`tcp_viewer`·`tcp_server`), `phase`. ticket을 켠 역할의 TCP 진입점이 켜져 있으면 시작·재로드 때 기록한다. TCP는 ticket을 확인하지 못한다. | OPS-071 |
| `ticket_plaintext_ws` | `warn` | `role`(`viewer`·`server`), `listener`: 항상 `ws`, `phase`. ticket을 켠 역할이 평문 WS에서 켜져 있으면 시작·재로드 때 기록한다. 그 구간은 믿을 수 있는 망 안에 있어야 한다. | OPS-071 |
| `ip_db_unavailable` | `warn` | `config_key`: `ip_info.country_db` 또는 `ip_info.asn_db`. `reason`: 1.6절의 IP 정보 DB 원인 값. `phase`: `startup` 또는 `reload`. `detail?`: 원인 문장(운영체제 오류·형식 오류 문장). 경로와 파일 내용은 쓰지 않는다. 키마다 한 줄이며, 두 키가 같은 파일이면 두 줄을 쓴다. 실패한 내용과 같은 내용을 다시 감지하면 쓰지 않는다. | OPS-096, OPS-097 |

#### 자동 재로드

재로드 대상은 [설정 계약 1.8절](config-v4.md#18-재로드-구분)의 44개 키다. 관리 API 편집 대상은 인증서·개인키 경로 4개와 접속 ticket 공개 키 파일 경로 2개를 뺀 38개다. 적용 시점과 기존 연결 유지는 [운영 스펙 3.3절](../specs/operations.md#33-파일-변경-자동-감지와-부분-재로드)을 따른다. 이 절의 사건은 파일 변경 감지와 관리 API 편집이 함께 쓴다.

| 사건 | `level` | 발생 시점 | 필드 | 스펙 근거 |
| --- | --- | --- | --- | --- |
| `reload_change_detected` | `info` | 설정·인증서·개인키 파일, `enabled = true`인 ticket 표의 공개 키 파일, `[ip_info]`가 가리키는 IP 정보 DB 파일의 변경을 처음 감지했을 때 | `source`: `os_notify`(OS 파일 알림) 또는 `poll`(주기 확인). 처음 감지한 쪽이다. | OPS-024, OPS-041, OPS-072, OPS-097 |
| `reload_applied` | `info` | 후보를 검증하고 재로드 대상 값을 원자적으로 교체했을 때 | `trigger`: `file_watch` 또는 `config_edit`. `applied`: 실제 값·경로·파일 내용이 바뀐 재로드 대상 키의 문자열 배열. 허용 값 44개는 아래 목록을 따른다. | OPS-024, OPS-025, OPS-035, OPS-040, OPS-047, OPS-050~060, OPS-067, OPS-072, OPS-101 |
| `reload_rejected` | `warn` | 후보 검증에 실패해 후보 전체를 거절했을 때. 기존 설정과 연결은 그대로다. | `reason`(설정 오류 코드, 1.6절), `config_key?`, `detail?`. | OPS-026, OPS-027, OPS-040, OPS-070, OPS-072 |
| `reload_restart_required` | `warn` | 재로드 대상이 아닌 값이 실행 중 값과 달라 후보 전체를 거절했을 때. 차이가 남은 동안 이후 변경에도 기록한다. | `config_keys`: 문자열 배열. 다른 값을 가진 설정 키의 경로. 값은 쓰지 않는다. | OPS-003, OPS-035 |
| `reload_watch_failed` | `error` | 실행 중 파일 변경 감지(OS 알림 또는 주기 확인)에 장애가 생겼을 때. 이전 유효 설정으로 계속 운영한다. | `reason`: `notify_error` 또는 `poll_error`. `detail?` | OPS-028 |

`reload_rejected`와 `reload_restart_required`는 서로 다른 사건이다. 재시작 필요는 검증 실패로 세지 않는다. 자동 재로드가 성공하지 않았는데 `reload_applied`를 쓰지 않는다. 시작 전에 감지 준비가 불가능하면 `reload_watch_failed`가 아니라 `startup_failed`(`watch_unavailable`)다.

`reload_applied.applied`의 허용 값:

- 출발지 IP 6개: `relay.tcp_viewer.allow`, `relay.tcp_server.allow`, `relay.ws.viewer.allow`, `relay.ws.server.allow`, `relay.wss.viewer.allow`, `relay.wss.server.allow`.
- 신뢰 프록시 2개: `relay.ws.trusted_proxies`, `relay.wss.trusted_proxies`.
- 차단 목록 1개: `relay.deny.sources`.
- TLS 경로·내용 4개: `relay.wss.cert_file`, `relay.wss.key_file`, `management.cert_file`, `management.key_file`.
- 접속 ticket 10개: `relay.ticket.viewer.enabled`, `relay.ticket.viewer.issuer`, `relay.ticket.viewer.relay_host`, `relay.ticket.viewer.public_keys_file`, `relay.ticket.viewer.max_lifetime_secs`, `relay.ticket.server.enabled`, `relay.ticket.server.issuer`, `relay.ticket.server.relay_host`, `relay.ticket.server.public_keys_file`, `relay.ticket.server.max_lifetime_secs`.
- 접근 정책 5개: `relay.ws.viewer.allowed_origins`, `relay.ws.server.allowed_origins`, `relay.wss.viewer.allowed_origins`, `relay.wss.server.allowed_origins`, `mode1.allow_targets`.
- 연결 상한 5개: `limits.relay_connections`, `limits.admin_connections`, `limits.waiting`, `limits.buffer_bytes`, `limits.ws_message_bytes`.
- 시간 제한 6개: `timeouts.handshake_secs`, `timeouts.tls_upgrade_secs`, `timeouts.wait_secs`, `timeouts.connect_secs`, `timeouts.close_secs`, `timeouts.shutdown_grace_secs`.
- 연결 유지 5개: `keepalive.tcp_idle_secs`, `keepalive.tcp_interval_secs`, `keepalive.tcp_probes`, `keepalive.ws_ping_secs`, `keepalive.ws_pong_timeout_secs`.

`trigger`는 기존 소비자에 대한 호환 추가 필드다. 새 Producer는 항상 기록하고, 이전 Producer의 줄에는 없을 수 있다. 뒤따르는 파일 감지가 같은 설정을 읽어 차이가 없으면 `reload_applied`를 다시 쓰지 않는다. `reload_change_detected`는 기록될 수 있다.

`relay.deny.sources`의 값이 바뀐 교체는 `reload_applied.applied`에 이 키를 담는다. 그 직후 열린 연결을 끊으면 연결마다 `waiting_removed`(`reason = ip_denied`) 또는 `session_closed`(`close_cause = ip_denied`)를 기록한다. 끊은 연결을 요약하는 사건은 따로 두지 않는다.

로그 파일은 열 수 있지만 로그 디렉터리의 알림 등록만 실패하면 예외다. `reload_watch_failed`(`notify_error`)를 쓰고 시작을 계속한다. Dp마다 로그 파일을 다시 열고 알림 등록도 다시 시도한다. 설정·TLS 파일 감시 준비 실패에는 이 예외를 적용하지 않는다. 접속 ticket 공개 키 파일의 감시 준비 실패도 설정 파일과 같이 다룬다.

#### IP 정보 DB

IP 정보 DB 파일의 읽기는 설정 재로드와 따로 한다([설정 계약 1.12절](config-v4.md#112-ip-정보-db-파일-형식)). 실패는 위 "설정 경고"의 `ip_db_unavailable`이다. IP 정보 DB 파일만 바뀌면 `reload_change_detected` 뒤에 설정 후보 검사가 차이를 찾지 못하므로 `reload_applied`를 쓰지 않는다. 재시작이 필요한 차이가 남아 있으면 `reload_restart_required`는 다른 변경과 같이 다시 기록된다.

| 사건 | `level` | 발생 시점 | 필드 | 스펙 근거 |
| --- | --- | --- | --- | --- |
| `ip_db_loaded` | `info` | 시작 때 또는 실행 중 파일 내용이 바뀌어 IP 정보 DB를 읽고 그 뒤 사건부터 쓰기 시작했을 때 | `config_key`: `ip_info.country_db` 또는 `ip_info.asn_db`. `database_type`. `build_time`. `phase`: `startup` 또는 `reload`. 키마다 한 줄이며, 두 키가 같은 파일이면 두 줄을 쓴다. 경로는 쓰지 않는다. | OPS-096, OPS-097 |

#### 로그 출력

| 사건 | `level` | 발생 시점 | 필드 | 스펙 근거 |
| --- | --- | --- | --- | --- |
| `log_reopen_failed` | `warn` | 로그 파일 다시 열기(1.1절)가 실패하기 시작했을 때. 이전 파일에 쓴다. 실패가 이어지는 동안은 다시 쓰지 않는다. | `reason`: 항상 `log_file_unavailable`. `config_key`: 항상 `log.output`. `detail?`: 원인 문장(상위 디렉터리 없음·권한 없음·디렉터리임·운영체제 오류 문장). 경로 값은 쓰지 않는다. | OPS-045 |

#### 접속 수락·거절·중단

| 사건 | `level` | 발생 시점 | 필드 | 스펙 근거 |
| --- | --- | --- | --- | --- |
| `connection_accepted` | `info` | 중계 리스너가 접속을 수락하고 수락 단계의 IP 검사와 L 검사를 통과했을 때 | `conn_id`, `listener`(`tcp_viewer`·`tcp_server`·`ws`·`wss`), `peer_ip`. `role`은 TCP에서 필수이고 WS·WSS에서는 생략한다. 이 시점에는 `forwarded_ip`도 없다. | RLY-001, OPS-037, OPS-069, WS-021 |
| `ticket_accepted` | `info` | ticket을 켠 역할의 업그레이드가 ticket 확인과 사용 기록 쓰기를 통과했을 때. 101 전이다. | `conn_id`, `listener`(`ws`·`wss`), `role`, `peer_ip`, `forwarded_ip?`, `ticket_jti`, `ticket_sub?`, `session_expires_at?` | WS-038, WS-045 |
| `connection_rejected` | `warn` | VNC Relay가 정책·제한·형식 때문에 접속을 거절했을 때 | `listener`(1.4절의 6개 값), `stage`, `reason`, `peer_ip`. 선택: `conn_id`, `role`, `mode`, `relay_id_ref`, `limit`, `origin`, `target_host`, `target_ip`, `target_port`, `forwarded_ip`, `ticket_jti`. WS·WSS의 `role`은 경로 판정 뒤에만 쓴다. `forwarded_ip`는 주소 도출에 성공한 뒤에만 쓴다. `ticket_jti`는 ticket의 서명을 확인한 뒤의 거절과 `ticket_relay_mismatch`에만 쓴다. 값 목록은 1.6절. | RLY-004, RLY-007, RLY-008, RLY-022, RLY-023, RLY-029, RLY-030, RLY-032, OPS-004, OPS-005, OPS-023, OPS-036, OPS-037, WS-012, WS-014, WS-015, WS-022, WS-023, WS-027, WS-032, WS-033, WS-036, WS-039~045 |
| `connection_aborted` | `warn`. `reason`이 `peer_eof`, `server_shutdown`, `session_expired`이면 `info` | 수락한 연결이 대기 등록이나 세션 시작 전에 끝났을 때. VNC Relay가 정책으로 거절한 경우는 제외한다. | `conn_id`, `listener`(`tcp_viewer`·`tcp_server`·`ws`·`wss`), `stage`, `reason`, `peer_ip`, `elapsed_ms`. 선택: `role`, `mode`, `error_kind`, `forwarded_ip`. WS·WSS의 `role`은 경로 판정 뒤에만 쓴다. `forwarded_ip`는 주소 도출에 성공한 뒤에만 쓴다. 값 목록은 1.6절. | RLY-005, RLY-031, OPS-006, OPS-007, WS-019, WS-023, WS-032 |

`conn_id`는 `connection_accepted`를 기록한 연결에만 있다. 접속 수락 단계에서 거절한 연결(`stage`가 `accept`)은 수락 사건이 없으므로 `conn_id`도 없다.

WS·WSS의 수락·TLS·경로 판정 전 업그레이드 거절·중단에는 `role`이 없다. 전달 주소를 도출한 뒤에는 업그레이드 단계의 Origin·서브프로토콜·IP 거절에도 `forwarded_ip`를 쓴다. 이후 초기 교환·등록·대상 연결 단계의 거절·중단에도 유지한다. 도출 실패(`forwarded_for_invalid`)에는 `forwarded_ip`를 쓰지 않는다.

관리·상태 리스너는 `connection_accepted`를 기록하지 않는다. 폴링 요청이 로그를 채우지 않게 하기 위해서다. 두 리스너의 거절(Lm 도달, 관리 HTTPS의 TLS 1.2 미만)은 `connection_rejected`로 기록한다.

#### Mode II 대기

| 사건 | `level` | 발생 시점 | 필드 | 스펙 근거 |
| --- | --- | --- | --- | --- |
| `waiting_registered` | `info` | 유효한 Mode II 요청을 대기에 등록했을 때 | `conn_id`, `listener`(`tcp_viewer`·`tcp_server`·`ws`·`wss`), `role`, `mode`(항상 `mode2`), `relay_id_ref`, `peer_ip`, `forwarded_ip?` | RLY-010, RLY-011, OPS-005, WS-032 |
| `waiting_expired` | `info` | 대기 시간 W가 지나 대기 연결을 제거했을 때 | `conn_id`, `role`, `relay_id_ref`, `waited_ms` | 시나리오 없음. [중계 스펙 4절](../specs/relay.md#4-오류와-복구) "대기 단절·만료", RLY-R09, [관리 스펙 2.3절](../specs/management.md#23-최근-종료-목록) "대기 만료는 로그에만 남긴다", OPS 5절 W |
| `waiting_removed` | `info`. `reason`이 `connection_error` 또는 `dead_connection`이면 `warn` | 대기 연결이 대기 시간 W 만료 외의 이유로 제거됐을 때 | `conn_id`, `role`, `relay_id_ref`, `reason`, `waited_ms`. 선택: `error_kind`. `reason`은 `peer_eof`, `connection_error`, `dead_connection`, `server_shutdown`, `session_expired`, `ip_denied` 중 하나. `session_expired`는 ticket의 `sx`에 도달한 경우이며 `info`다. `ip_denied`는 차단 목록이 바뀐 직후 클라이언트 주소가 항목과 일치해 대기에서 뺀 경우이며 `info`다. | RLY-025, RLY-031, RLY-034, WS-048, OPS-007, OPS-101 |

#### 매칭·대상 연결·세션

| 사건 | `level` | 발생 시점 | 필드 | 스펙 근거 |
| --- | --- | --- | --- | --- |
| `session_started` | `info` | Mode II 매칭이 성공했거나 Mode I 대상 연결이 성공해 활성 세션이 생겼을 때 | `session_id`, `mode`, `viewer_transport`, `viewer_conn_id`, `viewer_peer_ip`, `viewer_forwarded_ip?`. Mode II 전용: `server_transport`(`tcp`·`ws`·`wss`), `server_conn_id`, `server_peer_ip`, `server_forwarded_ip?`, `relay_id_ref`, `waited_ms`(먼저 대기한 연결의 대기 시간). Mode I 전용: `target_ip`, `target_port`, `target_host?`. | RLY-006, RLY-010, RLY-011, MGT-001, MGT-009, WS-028, WS-032 |
| `target_connect_failed` | `warn` | Mode I 대상 연결을 시도했지만 실패했을 때. 정책 거절은 `connection_rejected`이며 이 사건이 아니다. | `conn_id`, `listener`(`tcp_viewer`·`ws`·`wss`), `viewer_transport`, `reason`, `peer_ip`, `target_host`, `target_port`, `elapsed_ms`. 선택: `target_ip`, `forwarded_ip`. `reason`은 `resolve_failed`, `connect_refused`, `connect_timeout`, `connect_error` 중 하나. | RLY-009, RLY-R06, OPS-023, WS-032 |
| `session_closed` | 원인별. 1.6절 | 활성 세션의 정리가 끝났을 때. 한 세션에 한 번만 쓴다. | `session_id`, `mode`, `viewer_transport`, `started_at`(UTC ISO 8601), `duration_ms`, `bytes_viewer_to_target`, `bytes_target_to_viewer`, `close_cause`. 선택: `side`, `error_kind`, `relay_id_ref`. `close_cause`가 `admin_terminated`이면 `admin_account`가 필수다. | RLY-019, RLY-020, RLY-026, RLY-031, WS-016, WS-017, WS-026, OPS-008, OPS-009, MGT-009, MGT-023, MGT-052 |

`bytes_viewer_to_target`과 `bytes_target_to_viewer`는 [관리 API 계약 1.8절](management-api-v1.md#18-활성-세션-get-apiv1sessions-get-apiv1sessionssession_id)의 같은 이름 필드와 방향·의미가 같다([관리 스펙 2.1절](../specs/management.md#21-전송량의-의미)). 상대 전송 계층에 쓰기에 성공한 바이트 정수만 쓴다.

`session_started`와 `session_closed`의 `session_id`는 서로 짝이다. 같은 `session_id`에서 `session_started`가 먼저 온다. 같은 중계 ID의 새 세션은 새 `session_id`를 가진다(RLY-R11).

#### 관리자 로그인과 종료

로그인 사건은 [운영 스펙 OPS-R06](../specs/operations.md#41-판정-가능한-규칙)과 [관리 스펙 3.0절](../specs/management.md#30-관리자-로그인과-세션)의 제한 동작에 대응한다. 로그인 시도 기록 자체를 직접 요구하는 스펙 시나리오는 없지만 아래 세 사건의 정의를 따른다.

| 사건 | `level` | 발생 시점 | 필드 | 스펙 근거 |
| --- | --- | --- | --- | --- |
| `admin_login_succeeded` | `info` | 로그인에 성공해 관리 세션을 발급했을 때 | `admin_account`, `peer_ip` | MGT-017, OPS-029 |
| `admin_login_failed` | `warn` | 로그인 요청을 거절했을 때 | `reason`: `invalid_credentials` 또는 `limited`. `peer_ip`. 선택: `admin_account`. `reason`이 `limited`이면 `scope`(`account` 또는 `source`)도 쓴다. | MGT-018, MGT-022 |
| `admin_login_limited` | `warn` | 실패 횟수가 F를 넘어 제한을 시작했을 때. 제한 중 시도마다 기록하지 않는다(그 시도는 `admin_login_failed`다). | `scope`: `account` 또는 `source`. `peer_ip`(기준을 넘게 한 시도의 출발지). `lockout_secs`: 정수. 선택: `admin_account`. | MGT-022, MGT-R14 |
| `admin_terminate` | `info`. `result`가 `failed`이면 `error` | 유효한 관리 세션과 CSRF 검증을 통과한 세션 종료 요청을 처리했을 때. 응답과 같은 시점이며 정리가 끝난 뒤다. | `admin_account`, `result`(`terminated`, `already_closed`, `not_found`, `failed`), `peer_ip`. `result`가 `not_found`가 아니면 `session_id`. | OPS-011, MGT-009, MGT-010, MGT-011, MGT-012 |
| `config_edited` | `info` | `PATCH /api/v1/config`가 파일 쓰기와 유효 설정 교체를 마쳤을 때. 편집 전후 파일 바이트가 다를 때만 기록 | `admin_account`(필수), `peer_ip`, `config_keys`(요청 `changes` 가운데 파일 값이 바뀐 키만), `revision`(새 파일). 키 이름은 위 44개 목록 중 TLS 경로 4개와 공개 키 파일 경로 2개를 뺀 편집 대상 38개다. 설정 값은 쓰지 않는다. | OPS-060, OPS-062, OPS-072, MGT-031 |
| `config_edit_rejected` | `warn` | 인증·CSRF를 통과한 `PATCH /api/v1/config`를 거절했을 때. `invalid_request`는 제외 | `admin_account`(필수), `peer_ip`, `reason`(관리 API 오류 코드 중 `invalid_request` 제외). 선택: `config_keys`(요청한 키 배열). 재로드 대상 키를 기록할 때는 위 44개 목록의 이름을 쓴다. 편집 대상은 38개이며 편집 불가 키의 거절은 관리 API 계약을 따른다. 설정 값은 쓰지 않는다. | OPS-061, OPS-063, OPS-072, MGT-030, MGT-033~036, MGT-038 |

`result`는 [관리 API 계약 1.9절](management-api-v1.md#19-관리자-종료-post-apiv1sessionssession_idterminate)의 종료 결과에 대응한다. 이 요청이 종료를 시작하면 `terminated`, 이미 시작한 정리에 합류해 완료를 확인하면 `already_closed`, 대상이 없으면 `not_found`, 종료를 확인하지 못하면 `failed`다. `already_closed`는 `info`이며 `session_id`를 포함한다. 유효한 관리 세션이 없는 요청은 관리 세션 없음으로 응답하며 이 사건을 기록하지 않는다. `result`가 `not_found`이면 `session_id`를 쓰지 않는다. 존재하지 않는 세션 식별자를 연결된 것처럼 기록하지 않기 위해서다.

대상을 정리한 `session_closed`가 먼저 오고, 같은 요청의 `admin_terminate`가 뒤따른다. 이 요청이 종료를 시작했고 `close_cause`가 `admin_terminated`이면 두 줄은 같은 `admin_account`를 가진다. `already_closed`일 때 `session_closed`는 실제 최초 종료 원인을 유지한다. 합류 요청은 자신의 인증 계정으로 `admin_terminate(result=already_closed)`를 기록하며 `session_closed`를 추가로 만들지 않는다. 관리자 정리의 E 만료 때 계정과 `side`는 1.6절을 따른다.

같은 설정 편집에서는 `reload_applied`(`trigger = config_edit`)가 먼저 오고 `config_edited`가 뒤따른다. 두 사건의 키 목록은 범위가 다르다. `reload_applied.applied`는 실제 실행 값이 바뀐 모든 키다. 파일에 먼저 저장되어 아직 반영되지 않은 변경도 포함할 수 있다. `config_edited.config_keys`는 요청 `changes` 가운데 파일 값이 바뀐 키만 담는다. 편집 전후 파일 바이트가 같으면 두 성공 사건을 만들지 않는다. 반영 전 파일 변경이 남아 있어도 이 요청은 적용하지 않고 파일 감지가 적용한다. 성공 사건에는 IP 정책 값·Origin·대상 항목·수치를 쓰지 않는다. 거절은 `config_edit_rejected`만 기록한다. 오류 코드의 의미는 [관리 API 계약](management-api-v1.md)이 정한다.

### 1.6 원인 코드

`reason`과 `close_cause`의 값이다. 사건마다 쓸 수 있는 값이 다르다.

#### `startup_failed`와 `reload_rejected`의 `reason`

값은 다른 계약이 정한 오류 코드다. 이 계약은 코드를 다시 정의하지 않는다.

| 사건 | `reason` 값의 출처 |
| --- | --- |
| `startup_failed` | [설정 계약 2.1절](config-v4.md#21-설정-오류와-준비-실패)의 `config_*`·`tls_file_invalid`·`ticket_keys_invalid`·`log_file_unavailable`, [CLI 계약 2.2절](cli-v1.md#22-오류-코드)의 `accounts_*`·`listener_bind_failed`·`watch_unavailable`·`runtime_unavailable` |
| `reload_rejected` | [설정 계약 2.1절](config-v4.md#21-설정-오류와-준비-실패)의 `config_*`·`tls_file_invalid`·`ticket_keys_invalid` |

`startup_failed`는 진단 하나에 한 줄을 쓴다. stderr의 오류 줄([CLI 계약 1.5절](cli-v1.md#15-입력과-출력-규칙))과 1:1로 대응한다. 같은 오류 코드의 진단이 여럿이면 줄도 여럿이며, 줄마다 그 진단의 `config_key`·`listener`를 쓴다. 종료 코드는 [CLI 계약 2.1절](cli-v1.md#21-종료-코드)이 정한다. 인증서·개인키 한쪽만 바뀐 후보와 참조 파일을 읽을 수 없는 후보는 `reload_rejected`의 `tls_file_invalid`다.

차단 목록과 신뢰 프록시가 겹치면 `config_deny_overlaps_proxy`다([설정 계약 1.6절](config-v4.md#16-값-관계-검사)). `config_key`는 `relay.deny.sources[i]`다(OPS-100).

접속 ticket 공개 키 파일의 오류는 `ticket_keys_invalid`다([설정 계약 1.11절](config-v4.md#111-접속-ticket-공개-키-파일-형식)). 시작은 종료 코드 3으로 실패하고 재로드는 기존 공개 키를 유지한다(OPS-070, OPS-072). 파일 내용은 오류 문장에도 넣지 않는다(OPS-073).

#### `ip_db_unavailable`의 `reason`

값은 [설정 계약 1.12절](config-v4.md#112-ip-정보-db-파일-형식)의 원인 코드 `file_unreadable`, `file_too_large`, `invalid_format`, `unsupported_data`다. 설정 오류 코드가 아니며 시작·재로드를 막지 않는다.

#### `connection_rejected`의 `stage`와 `reason`

| `stage` | 뜻 | `reason` | 의미 | 근거 |
| --- | --- | --- | --- | --- |
| `accept` | 접속 수락 직후 | `ip_not_allowed` | TCP는 해당 역할의 허용 목록에 소켓 상대 IP가 없음. WS·WSS는 소켓 상대 IP가 신뢰 프록시도 아니고 켜진 역할의 허용 목록 합집합에도 없음. 데이터를 읽거나 TLS를 협상하기 전에 거절한다. | RLY-023, OPS-037, WS-014, WS-023, WS-030 |
| `accept` | 접속 수락 직후 | `ip_denied` | 클라이언트 주소가 출발지 IP 차단 목록에 있음. TCP는 소켓 상대 IP를 검사한다. WS·WSS는 소켓 상대 IP가 신뢰 프록시가 아닐 때만 검사하며 허용 목록 합집합 검사보다 앞선다. 소켓을 바로 닫는다. | OPS-100, RLY-033, WS-047 |
| `accept` | 접속 수락 직후 | `relay_connection_limit` | 중계 연결 상한 L에 도달함. `limit`을 쓴다. | OPS-004 |
| `accept` | 접속 수락 직후 | `admin_connection_limit` | 관리·상태 연결 상한 Lm에 도달함. `listener`는 `management` 또는 `status`다. `limit`을 쓴다. | OPS-033 |
| `tls` | TLS 협상 | `tls_version_unsupported` | TLS 1.2 미만 협상. `listener`는 `wss` 또는 `management`다. `wss`이면 수락 때 준 `conn_id`를 쓴다. | OPS-036 |
| `upgrade` | WebSocket 업그레이드 직전 | `ip_not_allowed` | 업그레이드 직전 재검사에서 클라이언트 주소가 경로로 정한 역할의 허용 목록에 없음 | WS-022, WS-030, WS-032 |
| `upgrade` | WebSocket 업그레이드 직전 | `ip_denied` | 클라이언트 주소가 차단 목록에 있음. 역할 허용 검사보다 앞선다. HTTP 403이다. `forwarded_ip`는 전달 주소를 도출한 경우에 쓴다. ticket은 소비하지 않는다(WS-044). | WS-047 |
| `upgrade` | WebSocket 업그레이드 직전 | `forwarded_for_invalid` | 신뢰 프록시의 `X-Forwarded-For`가 없거나 비어 있음, 검사한 요소가 잘못됨, 또는 모든 요소가 신뢰 프록시 주소임. 마지막 경우 프록시 주소가 역할 허용 목록에 있어도 거절한다. HTTP 400이다. `peer_ip`는 프록시 소켓 IP이며 `forwarded_ip`는 생략한다. | WS-033, WS-036 |
| `upgrade` | WebSocket 업그레이드 직전 | `origin_not_allowed` | Origin 헤더 값이 허용 값과 정확히 일치하지 않음. `origin`에 헤더 값을 쓴다. | WS-015, WS-027 |
| `upgrade` | WebSocket 업그레이드 직전 | `invalid_upgrade` | 업그레이드 경로가 아니거나 업그레이드 요청이 잘못됨. 관리 API 경로와 꺼진 역할의 경로 요청도 여기에 속한다. 꺼진 역할의 경로는 알 수 없는 경로와 같이 HTTP 404다. | WS-012, WS-029 |
| `upgrade` | WebSocket 업그레이드 직전 | `subprotocol_not_supported` | 서브프로토콜이 지원 값(`binary`)이 아닌 값만 제시됨. ticket을 켠 역할에 `binary` 없이 ticket만 제시해도 HTTP 400이다. | WS-039, [WebSocket 계약](websocket-relay-v2.md) 1.2절 |
| `upgrade` | WebSocket 업그레이드 직전 | `ticket_missing` | ticket을 켠 역할에 ticket이 없음. HTTP 401이다. | WS-039 |
| `upgrade` | WebSocket 업그레이드 직전 | `ticket_invalid` | ticket 개수·형식·서명·클레임·역할 대조가 유효하지 않음. HTTP 401이다. | WS-040, WS-045 |
| `upgrade` | WebSocket 업그레이드 직전 | `ticket_expired` | `exp + 5`초 또는 `sx`에 도달함. HTTP 401이다. | WS-041 |
| `upgrade` | WebSocket 업그레이드 직전 | `ticket_issued_before_start` | `iat`가 프로세스 기동 시각 + 5초 이하임. HTTP 401이다. | WS-043 |
| `upgrade` | WebSocket 업그레이드 직전 | `ticket_reused` | 같은 역할의 `jti`가 사용 기록에 있음. HTTP 401이다. | WS-042 |
| `initial_exchange` | Repeater 초기 교환 | `handshake_invalid` | 초기 요청의 형식 오류. [Repeater 계약 2절](repeater-handshake-v1.md#2-오류)의 같은 이름 코드다. | RLY-004 |
| `initial_exchange` | Repeater 초기 교환 | `mode_disabled` | 설정으로 끈 모드의 요청. `mode`를 쓴다. | RLY-022 |
| `initial_exchange` | Repeater 초기 교환 | `ticket_relay_mismatch` | 기존 초기 요청 검사를 통과했으나 중계 ID가 ticket의 `rid`와 다르거나 뷰어 Mode I 요청임. Close 4012다. `mode`, `ticket_jti`를 쓰며 Mode II이면 `relay_id_ref`도 쓴다. 대기 등록·대상 연결은 없다. | RLY-029, RLY-030, RLY-032 |
| `register` | 대기 등록 | `waiting_full` | 대기 상한 Lw에 도달함. `limit`을 쓴다. | OPS-005 |
| `register` | 대기 등록 | `duplicate_id` | 같은 중계 ID와 같은 역할이 이미 대기 중이거나 활성 세션이 점유함. 후발 연결을 거절한다. `relay_id_ref`를 쓴다. | [중계 스펙 3.3절](../specs/relay.md#33-mode-ii) 4항, RLY-015 |
| `register` | 대기 등록 | `ip_denied` | 초기 교환 뒤 대기 등록·매칭·Mode I 대상 연결 전에 현재 차단 목록으로 다시 검사한 클라이언트 주소가 차단됨. 교체 전에 수락돼 초기 교환 중이던 연결이 해당한다. TCP는 닫고 WS·WSS는 Close 4014로 닫는다. | RLY-033, WS-048 |
| `target_connect` | Mode I 대상 연결 준비 | `target_not_allowed` | 최종 대상 IP·포트가 허용 정책을 만족하지 못함. `target_host`, `target_ip?`, `target_port`를 쓴다. | RLY-007, RLY-008 |
| `target_connect` | Mode I 대상 연결 준비 | `relay_connection_limit` | 대상 연결 소켓을 위한 L 예산이 없음. `limit`을 쓴다. | OPS-023 |

ticket 업그레이드 거절은 `connection_rejected`의 `warn`과 공통 필드 규칙을 따른다. HTTP 401에는 `WWW-Authenticate: VncRelayTicket`을 보낸다. 서명을 확인한 뒤의 거절에만 `ticket_jti`를 쓴다. 검사 순서는 [WebSocket 계약 1.9.3절](websocket-relay-v2.md#193-확인-순서와-거절)과 같은 계약 1.1절을 따른다. 앞 검사에서 거절하면 ticket을 소비하지 않는다(WS-044). ticket 원문은 쓰지 않는다(OPS-073).

#### `connection_aborted`의 `stage`와 `reason`

`stage`는 `tls`, `upgrade`, `initial_exchange`, `register`, `target_connect` 중 하나다.
`register`는 초기 교환을 마쳤으나 대기 등록이 완료되기 전에 VNC Relay 종료로 끝난 Mode II 연결이다. 이때 `reason`은 `server_shutdown`이다. 매칭되었으나 아직 활성화되지 않은 상태에서 VNC Relay가 종료되면 이미 등록되어 있던 쪽은 `waiting_removed(reason=server_shutdown)`로, 도착하여 등록 중이던 쪽은 `connection_aborted(stage=register, reason=server_shutdown)`로 기록한다. Mode I의 대응 단계는 `target_connect`를 유지한다.

수락한 TCP·WS·WSS 소켓의 TCP keepalive 설정이 실패하면 `reason=connection_error`, `error_kind=io_error`로 중단한다. 같은 단계에서 켜는 `TCP_NODELAY`의 설정 실패도 같다. `stage`는 현재 단계인 TCP의 `initial_exchange`, WS의 `upgrade`, WSS의 `tls`다. `conn_id`를 가진 중단 사건을 남기기 위해 `connection_accepted`를 먼저 기록하고 keepalive를 설정한다. 실패하면 `connection_aborted`를 기록한 뒤 소켓을 닫고 L 자리를 돌려준다.

| `reason` | 의미 | 근거 |
| --- | --- | --- |
| `handshake_timeout` | Repeater 초기 교환이 H 안에 끝나지 않음 | RLY-005, OPS-006 |
| `tls_upgrade_timeout` | TLS 협상 또는 HTTP 업그레이드가 U 안에 끝나지 않음. U가 지난 뒤 상대가 닫은 경우도 이 사유다. | OPS-006, WS-023 |
| `peer_eof` | 상대가 초기 교환을 마치기 전에 연결을 닫음(미완료 단절). `stage`가 `initial_exchange`이면 [Repeater 계약 2절](repeater-handshake-v1.md#2-오류)의 `handshake_incomplete`다. | RLY-005 |
| `connection_error` | 네트워크·프로토콜 오류 또는 수락 소켓의 TCP keepalive·`TCP_NODELAY` 설정 실패. `error_kind`를 함께 쓴다. | WS-019 |
| `dead_connection` | 응답하지 않는 상대를 TCP keepalive 또는 WebSocket ping으로 판정함 | RLY-026 |
| `server_shutdown` | 정상 종료가 시작돼 미완성 작업을 종료함 | OPS-007 |
| `session_expired` | 초기 교환 중 ticket의 `sx`에 도달함. `stage=initial_exchange`, `level=info`로 기록하고 Close 4013으로 닫는다. | RLY-031 |

#### `session_closed`의 `close_cause`와 `level`

`close_cause`의 값 이름과 의미는 [관리 API 계약 1.10절](management-api-v1.md#110-최근-종료-목록-get-apiv1closed-sessions)이 정한다([관리 스펙 2.3절](../specs/management.md#23-최근-종료-목록)의 종료 원인과 같은 뜻이다). 이 표는 로그에서 값마다 쓰는 `level`·`side`와 추가 필드만 정한다. [WebSocket 계약 2.2절](websocket-relay-v2.md#22-close-코드와-스펙-오류의-대응)이 close 코드를 정한다. `internal_error` 경로에서는 panic 되감기가 스트림을 버리므로 WebSocket Close를 보내지 못한다([WebSocket 계약 2.2절 끝](websocket-relay-v2.md#22-close-코드와-스펙-오류의-대응)).

| `close_cause` | `level` | `side` | 추가 규칙 | 근거 |
| --- | --- | --- | --- | --- |
| `peer_eof` | `info` | 먼저 끝낸 쪽 | 없음 | RLY-019, RLY-020, WS-016 |
| `connection_error` | `warn` | 오류가 난 쪽 | `error_kind`를 함께 쓴다. | RLY-020, WS-016, WS-017 |
| `admin_terminated` | `info` | 없음 | `admin_account`가 필수다. | MGT-009, OPS-011 |
| `dead_connection` | `warn` | 응답이 없는 쪽 | 없음 | RLY-026, WS-026 |
| `close_timeout` | `warn` | 먼저 EOF를 낸 쪽. EOF를 관찰하지 않았으면 생략 | 관리자 정리도 E가 만료되면 이 원인을 쓴다. `admin_account`는 생략하고 `admin_terminate`로 계정을 추적한다. | WS-017, RLY-019, MGT-009, OPS-011 |
| `server_shutdown` | `warn` | 없음 | 정상 종료의 강제 정리 또는 Grace·Force 중 소유 작업 취소 때 기존 원인이 없는 세션에만 쓴다. Running 중 소유 작업의 버그 종료는 제외한다. | OPS-009, OPS-042 |
| `internal_error` | `warn`(1.3절의 서비스가 계속되는 비정상 종료 원인) | 없음 | 없음 | [관리 API 계약 1.10절](management-api-v1.md#110-최근-종료-목록-get-apiv1closed-sessions) |
| `session_expired` | `info` | 없음 | ticket의 `sx`에 도달해 끝낸 세션이다. 양쪽에 `sx`가 있으면 이른 시각을 쓴다. | RLY-031, MGT-052 |
| `ip_denied` | `info` | 없음 | 차단 목록이 바뀐 직후 뷰어 또는 Mode II 서버의 클라이언트 주소가 항목과 일치해 세션 전체를 끊었다. `admin_account`는 없다. 이미 정리 중인 세션은 기존 원인을 유지한다. | RLY-034, WS-048, OPS-101 |

한 세션의 `close_cause`는 정리를 시작하게 한 첫 원인이다. 그 뒤 생긴 정리 오류로 바꾸지 않는다. 단, 한쪽 EOF 뒤 또는 관리자 종료로 시작한 정리에서 E가 지나면 `close_cause`는 `close_timeout`이다. 관리자 정리에서 EOF를 관찰하지 않았으면 `side`를 생략한다. 이때 `session_closed`에는 `admin_account`를 넣지 않으며, 같은 `session_id`의 `admin_terminate`로 수행 계정을 추적한다. G 만료나 유예 중 두 번째 종료 요청으로 인한 강제 정리(`shutdown_forced`) 때도, 이미 정리를 시작한 세션(한쪽 EOF 관찰, I/O 오류, `dead_connection`, 관리자 종료 시작)은 최초 정리 시작 원인을 유지하며 `server_shutdown`으로 덮어쓰지 않는다. 아직 정리를 시작하지 않은 세션만 `close_cause=server_shutdown`이 된다.

VNC Relay가 Running일 때 세션 소유 작업이 panic 등 버그로 끝났고 기존 종료 원인이 없으면 `internal_error`를 쓴다. 정상 종료 중(Grace·Force)에 소유 작업이 취소되면 기존 원인이 없는 경우 `server_shutdown`을 쓴다. 어느 경우에도 기존 원인은 덮어쓰지 않는다.

### 1.7 `error_kind` 값

`connection_error`일 때 `connection_aborted`, `waiting_removed`, `session_closed`가 함께 쓴다.

| 값 | 의미 | 근거 |
| --- | --- | --- |
| `io_error` | 소켓 읽기·쓰기 오류, 연결 재설정, 소켓 옵션(TCP keepalive·`TCP_NODELAY`) 설정 실패 | RLY-020 |
| `tls_failed` | TLS 협상 실패(인증서 검증 실패 등). TLS 1.2 미만은 `connection_rejected`다. | WS-013 |
| `ws_protocol_error` | 잘못된 프레임 순서, 유효하지 않은 제어 프레임 | WS-019 |
| `ws_text_message` | 텍스트 데이터 메시지를 받음 | WS-008 |
| `ws_message_too_large` | 논리 메시지가 M을 넘음 | WS-010 |

### 1.8 관리자 계정명 기록 규칙

- 관리자 종료(`admin_terminate`, `admin_terminated` 원인의 `session_closed`), 로그인 성공, 설정 편집 성공·거절(`config_edited`, `config_edit_rejected`)에는 `admin_account`가 필수다. 로그인한 계정이므로 존재가 확정된 값이다.
- 로그인 실패·제한에서는 입력한 이름이 준비된 계정과 같을 때만 `admin_account`를 쓴다. 다르면 생략한다. 사용자가 계정명 칸에 비밀번호를 잘못 입력해도 로그에 남지 않게 하기 위해서다. `reason`은 두 경우 모두 `invalid_credentials`이며 구분하지 않는다.
- 로그 읽는 사람은 `admin_account` 유무로 계정 존재 여부를 알 수 있다. 로그 접근 권한은 운영자 책임이므로 이 차이는 허용한다. 응답에는 계정 존재 여부가 드러나지 않는다([관리 스펙 3.0절](../specs/management.md#30-관리자-로그인과-세션) 2항).

### 1.9 중계 ID 노출 방식

근거: 중계 ID는 인증 비밀이 아니지만 운영 정보로 취급하며 로그 노출 범위를 제한한다([PRD](../requirements/PRD.md) FR-06). 상태 응답에는 넣지 않는다. 대시보드의 대기 연결 목록은 유효한 관리 세션이 있을 때만 중계 ID를 보여준다([관리 스펙 2절](../specs/management.md#2-조회-대상과-상태)).

- 로그에는 중계 ID 원문을 어떤 사건에도 쓰지 않는다. 형식 오류·거절 사건에도 쓰지 않는다.
- 대신 `relay_id_ref`를 쓴다. 값은 아래 방식으로 만든다.
  1. 프로세스를 시작할 때 운영체제 난수로 32바이트 키를 만든다. 키를 저장하거나 기록하지 않는다.
  2. 매칭 비교에 쓰는 중계 ID 바이트에 HMAC-SHA-256을 계산한다.
  3. 결과의 앞 8바이트를 소문자 16진수 16자로 쓴다.
- 같은 프로세스에서 같은 중계 ID는 같은 `relay_id_ref`를 가진다. 그래서 대기 등록·만료·매칭·중복 거절을 연결해서 볼 수 있다.
- 프로세스를 재시작하면 키가 바뀌어 값도 달라진다. 재시작 전 로그와 연결하지 않는다. 재시작은 새 프로세스 수명이고 이전 중계 ID 매칭을 복원하지 않는다는 스펙과 맞다([운영 스펙 3.4절](../specs/operations.md#34-정상-종료와-재시작), [ADR-0003](../decisions/ADR-0003-single-instance-memory-state.md)).
- 키 없는 해시를 쓰지 않는 이유: 중계 ID는 숫자 1~247자리라([Repeater 초기 교환 계약 1.2절](repeater-handshake-v1.md#12-초기-요청-250바이트)) 짧을 수 있다. 짧은 ID는 해시만으로 되돌릴 수 있다. 키 있는 해시는 로그만으로 되돌릴 수 없다.
- `relay_id_ref`는 짝을 찾는 용도이며 중계 ID의 식별자가 아니다. 서로 다른 ID가 같은 값을 가질 가능성은 매우 작지만 0은 아니다. 수집 도구는 이 값을 접속 권한 증거로 쓰지 않는다.
- 운영자가 원문 ID가 필요하면 대시보드의 대기 연결 목록을 쓴다.

### 1.10 상대 주소 표기

- `peer_ip`는 실제 소켓 상대 IP다. 전달 헤더로 바꾸지 않는다. `viewer_peer_ip`·`server_peer_ip`도 각 쪽의 소켓 IP다.
- `forwarded_ip`는 WS·WSS 리스너의 신뢰 프록시에서 도출한 전달 주소다. 설정 범위는 [설정 계약](config-v4.md#12-키-표), 도출 규칙은 [WebSocket 계약 1.8절](websocket-relay-v2.md#18-신뢰-프록시와-x-forwarded-for)을 따른다. 신뢰하지 않는 소켓 상대가 보낸 헤더는 무시하며 `forwarded_ip`를 쓰지 않는다.
- `trusted_proxies`에는 프록시 주소만 넣어야 한다. 클라이언트 주소 범위를 넣으면 안 된다. 전달 체인의 모든 요소가 신뢰 프록시이면 클라이언트 주소를 프록시 주소로 대신하지 않고 거절한다(WS-036). 프록시 호스트나 이 목록 범위 안에서 시작한 요청도 프록시를 거치면 거절된다. 같은 출발지의 직접 접속도 신뢰 프록시 연결로 취급한다. 그래서 유효한 전달 헤더가 없으면 거절되고, 있으면 같은 전달 주소 판정과 역할 허용 목록 검사를 받는다. 전달 헤더 없이 직접 접속하려면 출발지가 그 리스너의 `trusted_proxies` 밖에 있고 해당 역할의 허용 목록에 들어야 한다.
- `viewer_peer_ip`·`server_peer_ip`를 가진 다른 사건에도 해당 쪽의 전달 주소를 도출했으면 `viewer_forwarded_ip`·`server_forwarded_ip`를 함께 써야 한다. 전달 주소가 없으면 선택 필드를 생략하며 소켓 주소를 복사하지 않는다.
- 관리 사건은 계속 전달 헤더를 무시한다(MGT-015). 관리자 로그인·종료·설정 편집의 `peer_ip`는 소켓 IP이며 `forwarded_ip`를 쓰지 않는다.
- IP 필드에는 포트를 쓰지 않는다. 클라이언트의 임시 포트는 추적 가치가 낮고, 노출을 줄이기 위해서다.
- IPv4는 점으로 구분한 십진수다. IPv6는 RFC 5952의 소문자 축약형이며 영역 식별자(`%` 뒤)는 쓰지 않는다. IPv4 매핑 IPv6는 IPv4로 정규화한다. 전달 주소도 같은 표기를 따른다.
- 업그레이드 때 역할 허용 목록 판정에 쓴 클라이언트 주소는 전달 주소가 있으면 `forwarded_ip`, 없으면 `peer_ip`와 같아야 한다.
- 같은 공인 IP를 공유하는 사용자는 구분하지 못한다. 소켓 IP와 전달 주소는 개별 사용자 식별자가 아니다.
- Mode I 대상은 `target_ip`·`target_port`로 쓴다. 정책 판정에 쓴 최종 IP·포트다. 이름이 있으면 `target_host`에 요청한 값을 쓴다.

### 1.11 로그에 넣지 않는 값

어떤 사건, 어떤 `level`, 어떤 `detail`에도 아래 값을 쓰지 않는다. 오류 메시지 전달 과정에서도 같다.

| 값 | 비고 |
| --- | --- |
| 관리자 평문 비밀번호, Argon2id 해시 | OPS-R07, OPS-030 |
| 관리 세션 쿠키(`relay_admin_session`) 값, `csrf_token` 값, `Cookie`·`Set-Cookie`·`X-CSRF-Token` 헤더 값 | OPS-012, MGT-013 |
| TLS 개인키 내용 | 인증서·개인키의 파일 경로와 설정 키 이름은 써도 된다. |
| 접속 ticket 원문(JWS 전체·서명 부분)과 공개 키 파일 내용 | OPS-073. `ticket_jti`·`ticket_sub`와 선택 `session_expires_at`은 1.4·1.5절에 따라 쓴다. |
| 로그인 요청 본문 | 입력한 계정명은 1.8절 규칙으로만 쓴다. |
| VNC 인증 데이터(챌린지·응답), 화면 데이터, 사용자 입력, 원시 페이로드 | OPS-012, [WS·WSS 스펙 5절](../specs/websocket.md#5-항상-지켜야-할-규칙과-비보장-범위) |
| Repeater 초기 교환의 원시 바이트 | 중계 ID 원문이 들어 있다. |
| 중계 ID 원문 | 1.9절 |
| 전달 헤더 원문 | `X-Forwarded-For` 원문·체인을 쓰지 않는다. 신뢰 프록시에서 도출한 IP만 1.10절의 `forwarded_ip`·`*_forwarded_ip`로 쓴다. |
| 설정 값 | 오류 사건은 설정 키 이름(`config_key`)만 쓴다. 항목 내용은 쓰지 않는다. |
| IP 정보 DB 파일의 경로·내용, 연결마다 찾은 국가·ASN | 접속 출처 통계는 관리 API로만 제공한다(OPS-R54). `ip_db_loaded`의 `database_type`·`build_time`은 쓴다. IP 정보 DB 사건은 경로 대신 `config_key`를 쓴다. 감시 실패(`reload_watch_failed`)의 `detail`에도 DB 파일 경로와 DB 파일만을 위해 등록한 디렉터리를 쓰지 않는다. |

### 1.12 상태 리스너와 엔드포인트

상태 리스너는 관리 리스너와 다른 리스너다. 설정 키는 `[status] listen`이다. 필수 키이며 기본 포트는 없다([설정 계약 1.2절](config-v4.md#12-키-표)). 루프백을 쓰기를 권장하며 루프백이 아니면 경고 `status_listen_not_loopback`을 기록한다. 상태 리스너는 평문 HTTP다. 상태 연결은 관리 연결과 함께 Lm에서 센다. Lm에 도달하면 새 연결을 거절하고, 이때 `connection_rejected`(`admin_connection_limit`)를 기록한다.

| 항목 | 값 |
| --- | --- |
| 인증 | 없다. `Authorization`·`Cookie` 헤더가 있어도 무시한다. 대시보드의 로그인 예외 규칙과 무관하다. |
| 메서드 | `GET`만 받는다. 다른 메서드는 405다. |
| 쿼리 문자열·본문 | 무시한다. |
| 본문 형식 | `/livez`·`/readyz`는 `Content-Type: application/json`, UTF-8. `Cache-Control: no-store`를 보낸다. |
| 본문에 넣는 필드 | `/livez`·`/readyz`는 아래 표의 필드만. 세션 목록, 중계 ID, 상대 주소, 인증 비밀, 연결 수, 세션 수, 가동 시간, 버전은 넣지 않는다. |
| 메트릭 경로 | `[metrics] enabled = true`이면 `GET /metrics`도 응답한다. 형식과 노출 범위는 [메트릭 계약](metrics-v1.md)이 정한다. 인증·메서드·쿼리 문자열 규칙은 이 표와 같다. 꺼져 있으면 정의하지 않은 경로처럼 404다. |

| 경로 | 조건 | 상태 코드 | 본문 |
| --- | --- | --- | --- |
| `GET /livez` | 프로세스가 요청에 응답할 수 있음 | 200 | `{"status":"alive"}` |
| `GET /readyz` | 준비됨 상태 | 200 | `{"status":"ready"}` |
| `GET /readyz` | 준비됨 상태가 아님 | 503 | `{"status":"not_ready","reason":"<code>"}` |

`/livez`는 종료 유예 중에도 프로세스가 응답하는 한 200이다. 생존 여부를 묻는 경로이므로 준비 여부와 섞지 않는다. `/readyz`의 `reason`은 2절 표의 값이다.

상태 리스너는 설정 검증이 끝난 뒤, 다른 필수 리스너를 준비하기 전에 연다. 그래야 시작 중 상태 조회가 `starting`으로 구분된다. 상태 리스너 바인딩에 실패하면 `startup_failed`(`listener_bind_failed`, `listener`가 `status`)다. 설정 검증 중이거나 프로세스가 없을 때의 조회는 연결 거절로 보인다. 도구는 이를 준비 안 됨으로 취급한다.

#### 실행 중 필수 리스너 장애

상태·관리·설정으로 켠 TCP·WS·WSS는 모두 필수다. 아래 복구 불가능 코드만 장애 종료를 시작한다. 나머지 오류와 원시 OS 코드가 없는 오류는 재시도한다.

| OS | 복구 불가능한 수락 오류 | 재시도하는 오류 |
| --- | --- | --- |
| Linux x64 | `EBADF=9`, `EINVAL=22`(수신 상태 아님), `ENOTSOCK=88` | `EINTR=4`, `EAGAIN=11`, `ENFILE=23`, `EMFILE=24`, `ENOMEM=12`, `ENOBUFS=105`, 접속 중단·네트워크 오류와 나머지 코드. `EOPNOTSUPP=95`도 새 연결의 보류된 네트워크 오류일 수 있어 재시도한다. |
| Windows x64 | `WSAEBADF=10009`, `WSAEINVAL=10022`, `WSAENOTSOCK=10038`, `WSAEOPNOTSUPP=10045`, `WSANOTINITIALISED=10093` | `WSAEINTR=10004`, `WSAEWOULDBLOCK=10035`, `WSAEMFILE=10024`, `WSAENOBUFS=10055`, `WSAENETDOWN=10050`, 접속 재설정·자원 부족과 나머지 코드. 개별 비동기 작업 취소 `ERROR_OPERATION_ABORTED=995`도 재시도한다. |

근거: [Linux accept(2)의 Error handling·ERRORS](https://man7.org/linux/man-pages/man2/accept.2.html), [Microsoft accept 오류 표](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-accept), [Winsock 오류 코드](https://learn.microsoft.com/en-us/windows/win32/winsock/windows-sockets-error-codes-2). 닫혔거나 잘못된 소켓·수신 상태 소실·Winsock 수명 소실은 같은 소켓으로 복구할 수 없다. 안전한 Rust가 인수·플래그를 고정하므로 Linux `EINVAL`은 수신 상태 소실로 판정한다. 알려지지 않은 오류를 추측해 치명 오류로 넓히지 않는다.

일시 오류는 기존 100밀리초 뒤 재시도한다. 임계값은 없다. 복구 불가능 오류는 리스너 이름과 원시 OS 코드를 감독자에게 전달한다. 장애가 먼저 종료를 시작하면 준비 상태를 `listener_failed`로 바꾸고 `shutdown_started(trigger=fatal_error)`를 `error`로 한 번 기록한다. `detail`에는 리스너 이름과 원시 OS 코드만 넣는다. G와 정리는 정상 종료와 같다. `server_stopped`가 마지막이며 종료 코드는 [CLI 계약 2.1절](cli-v1.md#21-종료-코드)의 1이다.

살아 있는 상태 HTTP는 장애 종료의 유예·정리 동안 `/readyz` 503 `listener_failed`와 `/livez` 200을 유지한다. 상태 소켓 자체 장애의 응답은 보장하지 않는다. 관찰용 대기는 없다. 활성 세션이 없으면 곧바로 끝낸다. 정상 종료가 먼저 시작되면 뒤늦은 장애는 최초 원인·유예를 바꾸지 않는다.

## 2. 오류

상태 엔드포인트의 응답 코드다. 로그 사건의 원인 코드는 1.6절과 1.7절에 있다.

| 오류 코드 | 의미·형식 | 관련 스펙 시나리오 |
| --- | --- | --- |
| `starting` | 준비 전. `/readyz`가 503과 `{"status":"not_ready","reason":"starting"}`를 응답한다. 설정 검증 뒤 시작 중 상태에서 쓴다. 필수 리스너가 모두 준비되면 200이 된다. | OPS-013, OPS-002 |
| `shutting_down` | 종료 유예 중 또는 종료 정리 중. `/readyz`가 503과 `{"status":"not_ready","reason":"shutting_down"}`를 응답한다. 신규 중계 접속을 받지 않는다. | OPS-013, OPS-007, OPS-043 |
| `listener_failed` | 필수 리스너의 복구 불가능 수락 오류. 살아 있는 상태 HTTP는 503 `{"status":"not_ready","reason":"listener_failed"}`를 답한다. 감독자가 즉시 `shutdown_started(trigger=fatal_error, level=error)`를 시작한다. 장애 종료의 유예·정리 동안 이 원인을 유지한다. | OPS-046 |
| `not_found` | 정의하지 않은 경로. `[metrics] enabled = false`일 때의 `/metrics`도 포함한다. 404와 `{"error":{"code":"not_found","message":"<사람용 문장>"}}`. 관리 API의 오류 본문과 같은 모양이다. | OPS-032, OPS-082 |
| `method_not_allowed` | `GET`이 아닌 메서드. 켜진 `/metrics`도 같다. 405와 `Allow: GET` 헤더, `{"error":{"code":"method_not_allowed","message":"<사람용 문장>"}}`. | OPS-032, OPS-084 |

Lm 상한에서 거절한 연결은 HTTP 응답을 받지 못하고 연결이 닫힌다. 이 경우는 오류 코드가 없다([운영 스펙 5절](../specs/operations.md#5-오류와-복구) Lm 행).

## 3. 호환성 규칙

- 소비자는 알 수 없는 필드를 무시해야 한다. 알 수 없는 `event` 줄은 버리지 않고 알 수 없는 사건으로 보존하거나 건너뛴다. 알 수 없는 `level`·`reason`·`close_cause`·`error_kind` 값도 오류로 처리하지 않는다.
- 소비자는 `event`, `level`, 사건별 필수 필드로 분기해야 한다. `detail` 문자열로 분기하지 않는다.
- 상태 도구는 HTTP 상태 코드로 생존·준비를 판정해야 한다. 200은 정상이고, 503은 준비 안 됨이다. 본문은 보조 정보다. 본문의 알 수 없는 `reason` 값을 오류로 처리하지 않는다.
- 호환 변경: 사건 추가, 기존 사건에 선택 필드 추가, `reason`·`close_cause`·`error_kind`·`stage`·`result` 값 추가, 상태 응답 본문에 선택 필드 추가. 수락 소켓의 keepalive 설정 실패 기록, `already_closed`, 관리자 E 만료의 `side`·계정 표기, `internal_error`는 호환 변경이다.
- 호환성을 깨는 변경(Breaking change): 사건·필드 이름 변경이나 삭제, 필드 자료형·단위·의미 변경, 필수 필드 삭제, 기존 `reason`·`close_cause` 값의 삭제나 의미 변경, `ts` 형식 변경, 줄 구분·인코딩 변경, `level` 값의 의미 변경, 상태 엔드포인트의 경로·상태 코드·`/readyz` 준비 판정 의미 변경. 이런 변경은 새 major 버전(`ops-info` 2.0.0)과 함께 한다. 마이그레이션은 소비자를 새 형식에 먼저 맞춘 뒤 `vnc-relay`를 올리는 순서로 한다. 소비자는 `server_starting`의 `log_contract`로 버전을 구분한다. 롤백은 이전 버전의 `vnc-relay`를 다시 시작하는 것이다. 영속 상태가 없으므로(ADR-0003) 데이터 되돌림은 필요 없다. 마이그레이션·롤백 계획은 major 변경 때 별도로 작성한다.

2.0.0은 `listener`의 의미와 허용 값을 바꾸고 WS·WSS의 수락 시점 필수 `role`을 없앤 major 변경이다. 전달 주소 필드, Mode II의 필수 `server_transport`와 새 거절 원인을 이 major에 함께 반영한다.

2.1.0의 접속 ticket 사건·선택 필드·원인 값·설정 경고는 호환 변경이다. 1.3.0의 호환 추가는 버전을 유지하지만 호환성 규칙이 버전 유지를 요구하지는 않는다. 접속 ticket 범위는 minor 버전 2.1.0이며 그때 `server_starting.log_contract`는 `ops-info/2.1.0`이었다. 소비자는 2.x의 알 수 없는 필드·사건·값 처리 규칙을 따른다.

2.2.0의 IP 정보 DB 사건 `ip_db_loaded`·`ip_db_unavailable`, 필드 `database_type`·`build_time`, `ip_db_unavailable`의 원인 값은 호환 변경이다. `reload_change_detected`의 감지 대상에 IP 정보 DB 파일이 더해졌으며 필드는 같다. WSS의 TLS 단계 `connection_rejected`에 선택 필드 `conn_id`를 쓰기 시작한 것도 호환 변경이다. `server_starting.log_contract`는 `ops-info/2.2.0`이다.

2.3.0의 `ip_denied` 값은 호환 변경이다. 이 값은 `connection_rejected.reason`(`stage`는 `accept`·`upgrade`·`register`), `waiting_removed.reason`, `session_closed.close_cause`에 더해진다. 설정 오류 코드 `config_deny_overlaps_proxy`는 `startup_failed`·`reload_rejected`의 `reason`에 더해지며, `relay.deny.sources` 키는 `reload_applied.applied`와 `config_edited.config_keys`의 허용 값에 더해진다. `server_starting.log_contract`는 `ops-info/2.3.0`이다.

상태 리스너의 `/metrics`는 [메트릭 계약](metrics-v1.md)이 소유하며 기본으로 꺼져 있다. 이 경로를 더해도 로그 형식과 `/livez`·`/readyz`의 경로·상태 코드·본문은 바뀌지 않는다. 그래서 메트릭을 더할 때 이 계약은 2.1.0을 유지했고 `log_contract`도 바꾸지 않았다. 메트릭의 속성 값은 이 계약의 원인 값을 그대로 쓰므로, 원인 값의 이름을 바꾸는 major 변경은 메트릭 계약의 major 변경과 함께 한다.

### 로그 소비자 마이그레이션

| 1.x의 값·규칙 | 2.0.0의 값·규칙 |
| --- | --- |
| `listener = ws_viewer` | `listener = ws`. 경로 판정 뒤 뷰어 사건이면 `role = viewer`다. |
| `listener = wss_viewer` | `listener = wss`. 경로 판정 뒤 뷰어 사건이면 `role = viewer`다. |
| WS·WSS `connection_accepted`의 `role = viewer` | `role`이 없다. 수락 시점에는 역할을 알 수 없다. |
| WS·WSS 경로 판정 전 `connection_rejected`·`connection_aborted`의 `role` | `role`이 없다. `accept`·`tls`·경로 판정 전 `upgrade`가 해당한다. |
| `server_starting.log_contract = ops-info/1.3.0` | `ops-info/2.0.0`. 이 값으로 파서를 고른다. |

먼저 소비자가 1.x와 2.x를 구분하게 바꾼다. `listener`만으로 WS·WSS의 역할을 단정하지 않아야 한다. `server_ready.listeners[].name`도 같은 새 이름을 쓴다. 재로드·편집 키를 수집하면 1.5절의 v4 키 이름으로 바꾼다. 그 뒤 설정 파일을 [v4로 옮기고](config-v4.md#33-v3-파일을-v4로-옮기기) 2.0.0 Producer를 배포한다.

롤백할 때는 이전 VNC Relay와 그 VNC Relay가 읽는 설정 파일로 되돌린다. 설정 파일의 롤백 조건은 설정 계약 3.3절을 따른다. 이전 VNC Relay는 1.3.0 로그를 내므로 소비자의 1.x 파서는 유지한다. 2.0.0 VNC Relay가 1.x 형식으로 출력하는 호환 모드는 없다.

## 4. 버전과 지원 단계

| 상태 | 조건 | 지원 종료·대체 |
| --- | --- | --- |
| Draft | 채택 전 상태. | 해당 없음 |
| Active | 현재 `ops-info` 2.3.0. Producer 계약 테스트의 현재 결과는 5절의 방법으로 확인한다. 저장소에 외부 소비 도구가 없어 Consumer 시험은 해당 없음이다. 지원 판정은 Producer 시험을 따른다. | 2.2.0 이하 2.x는 2.3.0으로 대체됨(Superseded)이다. |
| Deprecated | 새 major가 있고 이전 major를 쓰는 소비자가 남아 있다. | 신규 적용을 피하고 대체 문서를 확인한다. |
| Retired | 지원 종료. | 새 구현의 근거로 쓰지 않는다. |
| Superseded | `ops-info` 2.2.0 이하. 2.x는 호환 추가인 2.3.0으로 대체한다. | 새 구현의 근거로 쓰지 않는다. 기존 1.x 로그를 읽을 때는 3절의 마이그레이션 표를 따른다. |

## 5. 예제와 계약 테스트

예제의 주소는 문서용 주소(`192.0.2.0/24`, `198.51.100.0/24`, `203.0.113.0/24`, `2001:db8::/32`)다. 값은 모두 예시이며 실제 비밀이 아니다.

로그 줄(실제로는 줄바꿈 없이 한 줄씩 쓴다).

```jsonl
{"ts":"2026-10-16T01:00:00.000Z","level":"info","event":"server_starting","log_contract":"ops-info/2.3.0"}
{"ts":"2026-10-16T01:00:00.010Z","level":"warn","event":"plaintext_ws_enabled","listener":"ws","listen":"127.0.0.1:8080"}
{"ts":"2026-10-16T01:00:00.011Z","level":"warn","event":"allow_all_source","listener":"tcp_viewer","item":"0.0.0.0/0","phase":"startup"}
{"ts":"2026-10-16T01:00:00.120Z","level":"info","event":"server_ready","listeners":[{"name":"tcp_viewer","listen":"0.0.0.0:5900"},{"name":"tcp_server","listen":"0.0.0.0:5500"},{"name":"ws","listen":"127.0.0.1:8080"},{"name":"wss","listen":"0.0.0.0:8443"},{"name":"management","listen":"127.0.0.1:9443"},{"name":"status","listen":"127.0.0.1:9000"}]}
{"ts":"2026-10-16T01:02:03.456Z","level":"info","event":"connection_accepted","conn_id":"c-1001","listener":"tcp_server","role":"server","peer_ip":"198.51.100.21"}
{"ts":"2026-10-16T01:02:03.470Z","level":"info","event":"waiting_registered","conn_id":"c-1001","listener":"tcp_server","role":"server","mode":"mode2","relay_id_ref":"3fa9c0d1e27b4a68","peer_ip":"198.51.100.21"}
{"ts":"2026-10-16T01:02:10.002Z","level":"info","event":"session_started","session_id":"ses_3k9d2m7q4x8v1b6n5c0z2h4w7y","mode":"mode2","viewer_transport":"wss","server_transport":"tcp","viewer_conn_id":"c-1002","viewer_peer_ip":"2001:db8::5","server_conn_id":"c-1001","server_peer_ip":"198.51.100.21","relay_id_ref":"3fa9c0d1e27b4a68","waited_ms":6532}
{"ts":"2026-10-16T01:05:00.100Z","level":"info","event":"session_closed","session_id":"ses_3k9d2m7q4x8v1b6n5c0z2h4w7y","mode":"mode2","viewer_transport":"wss","started_at":"2026-10-16T01:02:10.002Z","duration_ms":170098,"bytes_viewer_to_target":10240,"bytes_target_to_viewer":4194304,"close_cause":"admin_terminated","admin_account":"admin-a","relay_id_ref":"3fa9c0d1e27b4a68"}
{"ts":"2026-10-16T01:05:00.101Z","level":"info","event":"admin_terminate","admin_account":"admin-a","result":"terminated","session_id":"ses_3k9d2m7q4x8v1b6n5c0z2h4w7y","peer_ip":"203.0.113.7"}
{"ts":"2026-10-16T01:06:00.000Z","level":"warn","event":"connection_rejected","listener":"wss","role":"viewer","stage":"upgrade","reason":"origin_not_allowed","peer_ip":"203.0.113.9","conn_id":"c-1003","origin":"https://x.a.example:8443"}
{"ts":"2026-10-16T01:07:00.000Z","level":"warn","event":"reload_rejected","reason":"config_ip_item_invalid","config_key":"relay.tcp_viewer.allow[2]"}
{"ts":"2026-10-16T01:08:00.000Z","level":"error","event":"startup_failed","reason":"listener_bind_failed","listener":"management","detail":"주소가 이미 사용 중이다."}
{"ts":"2026-11-08T05:00:01.000Z","level":"info","event":"reload_applied","trigger":"config_edit","applied":["limits.relay_connections"]}
{"ts":"2026-11-08T05:00:01.001Z","level":"info","event":"config_edited","admin_account":"admin-a","peer_ip":"203.0.113.7","config_keys":["limits.relay_connections"],"revision":"sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}
{"ts":"2026-11-08T05:01:00.000Z","level":"warn","event":"config_edit_rejected","admin_account":"admin-a","peer_ip":"203.0.113.7","reason":"config_conflict","config_keys":["limits.relay_connections"]}
{"ts":"2026-11-08T05:02:00.000Z","level":"warn","event":"allow_all_target","item":"0.0.0.0/0:5900","phase":"reload"}
{"ts":"2026-11-08T05:02:00.001Z","level":"info","event":"reload_applied","trigger":"file_watch","applied":["mode1.allow_targets"]}
{"ts":"2026-11-08T06:00:00.000Z","level":"info","event":"connection_accepted","conn_id":"c-2001","listener":"ws","peer_ip":"192.0.2.10"}
{"ts":"2026-11-08T06:00:00.010Z","level":"info","event":"waiting_registered","conn_id":"c-2001","listener":"ws","role":"server","mode":"mode2","relay_id_ref":"9b0a1c2d3e4f5678","peer_ip":"192.0.2.10","forwarded_ip":"198.51.100.21"}
{"ts":"2026-11-08T06:00:00.020Z","level":"warn","event":"allow_all_source","listener":"wss","role":"server","item":"::/0","phase":"reload"}
{"ts":"2026-11-08T06:00:01.000Z","level":"info","event":"session_started","session_id":"ses_4m8q2v6x0b3n7c1z5h9w2y6k0d","mode":"mode2","viewer_transport":"ws","server_transport":"ws","viewer_conn_id":"c-2002","viewer_peer_ip":"192.0.2.10","viewer_forwarded_ip":"203.0.113.9","server_conn_id":"c-2001","server_peer_ip":"192.0.2.10","server_forwarded_ip":"198.51.100.21","relay_id_ref":"9b0a1c2d3e4f5678","waited_ms":990}
```

접속 ticket 로그 예다. 실행 결과가 아니다.

```jsonl
{"ts":"2026-11-11T01:00:01.000Z","level":"warn","event":"ticket_tcp_bypass","role":"viewer","listener":"tcp_viewer","phase":"startup"}
{"ts":"2026-11-11T01:00:01.001Z","level":"warn","event":"ticket_plaintext_ws","role":"viewer","listener":"ws","phase":"startup"}
{"ts":"2026-11-11T01:02:00.000Z","level":"info","event":"ticket_accepted","conn_id":"c-3001","listener":"wss","role":"viewer","peer_ip":"192.0.2.10","forwarded_ip":"203.0.113.9","ticket_jti":"example-jti-1","ticket_sub":"example-session-1","session_expires_at":"2026-11-11T01:05:00.000Z"}
{"ts":"2026-11-11T01:02:01.000Z","level":"warn","event":"connection_rejected","conn_id":"c-3002","listener":"wss","role":"viewer","stage":"upgrade","reason":"ticket_expired","peer_ip":"203.0.113.9","ticket_jti":"example-jti-2"}
{"ts":"2026-11-11T01:05:00.000Z","level":"info","event":"session_closed","session_id":"ses_4m8q2v6x0b3n7c1z5h9w2y6k0d","mode":"mode2","viewer_transport":"wss","started_at":"2026-11-11T01:02:01.000Z","duration_ms":179000,"bytes_viewer_to_target":10240,"bytes_target_to_viewer":4194304,"close_cause":"session_expired","relay_id_ref":"9b0a1c2d3e4f5678"}
```

IP 정보 DB 로그 예다. 실행 결과가 아니다.

```jsonl
{"ts":"2026-11-20T01:00:00.020Z","level":"info","event":"ip_db_loaded","config_key":"ip_info.country_db","database_type":"DBIP-Country-Lite","build_time":"2026-11-01T00:00:00.000Z","phase":"startup"}
{"ts":"2026-11-20T01:00:00.021Z","level":"warn","event":"ip_db_unavailable","config_key":"ip_info.asn_db","reason":"file_unreadable","phase":"startup","detail":"파일을 찾을 수 없다."}
{"ts":"2026-11-20T02:00:00.000Z","level":"info","event":"reload_change_detected","source":"os_notify"}
{"ts":"2026-11-20T02:00:00.400Z","level":"info","event":"ip_db_loaded","config_key":"ip_info.asn_db","database_type":"DBIP-ASN-Lite (compat=GeoLite2-ASN)","build_time":"2026-11-01T00:00:00.000Z","phase":"reload"}
```

출발지 IP 차단 목록 로그 예다. 실행 결과가 아니다.

```jsonl
{"ts":"2026-11-20T03:00:00.000Z","level":"warn","event":"connection_rejected","listener":"tcp_viewer","role":"viewer","stage":"accept","reason":"ip_denied","peer_ip":"203.0.113.9"}
{"ts":"2026-11-20T03:00:01.000Z","level":"warn","event":"connection_rejected","conn_id":"c-4001","listener":"wss","role":"viewer","stage":"upgrade","reason":"ip_denied","peer_ip":"192.0.2.10","forwarded_ip":"198.51.100.7"}
{"ts":"2026-11-20T03:10:00.000Z","level":"info","event":"reload_applied","trigger":"config_edit","applied":["relay.deny.sources"]}
{"ts":"2026-11-20T03:10:00.002Z","level":"info","event":"waiting_removed","conn_id":"c-4002","role":"server","relay_id_ref":"3fa9c0d1e27b4a68","reason":"ip_denied","waited_ms":42000}
{"ts":"2026-11-20T03:10:00.004Z","level":"info","event":"session_closed","session_id":"ses_5n9r3w7y1c4p8d2a6j0t3x7m5b","mode":"mode2","viewer_transport":"wss","started_at":"2026-11-20T03:00:00.000Z","duration_ms":600004,"bytes_viewer_to_target":10240,"bytes_target_to_viewer":4194304,"close_cause":"ip_denied","relay_id_ref":"9b0a1c2d3e4f5678"}
```

상태 응답.

```http
GET /livez  ->  200 {"status":"alive"}
GET /readyz ->  200 {"status":"ready"}
GET /readyz ->  503 {"status":"not_ready","reason":"shutting_down"}
```

Producer 계약 테스트는 다음 위치에서 관리한다. 현재 시나리오별 결과는 [테스트 컨벤션](../conventions/testing.md)에 따라 `cargo xtask scenarios`로 생성한 `scenario-report.md`와 CI 결과에서 확인한다. 계약 시험은 스펙 시나리오의 동작 검사에 이 문서의 필드·사건 이름 검사를 더한다.

| 범위 | 테스트 위치 |
| --- | --- |
| JSON Lines 형식·사건·필드·상태 응답 | `crates/adapters/tests/ops_info_contract.rs`, `crates/adapters/src/ops_log.rs`·`crypto.rs`, `crates/domain/src/peer_ip.rs`·`allow_list.rs`, `crates/app/src/lifecycle.rs` |
| 실제 프로세스의 시작·접속·거절·중계·비밀 비노출 | `tests/serve.rs`, `tests/relay_log_scenarios.rs` |
| ticket 사건·거절·ID 대조·만료·비노출 | `tests/ticket_system/main.rs`, `crates/adapters/tests/websocket_ticket.rs` |
| 역할·신뢰 프록시·전달 주소 | `crates/adapters/tests/websocket_scenarios.rs`, `crates/adapters/tests/websocket.rs` |
| 재로드·설정 편집·로그 다시 열기 | `tests/reload.rs`, `tests/log_reopen.rs`, `tests/management_query.rs`, `crates/adapters/tests/management.rs`, `crates/adapters/src/file_watch/reopen_tests.rs` |
| IP 정보 DB 사건·순서·비노출 | `crates/adapters/tests/ops_info_contract.rs`, `crates/adapters/src/ip_db.rs`, `crates/app/src/ip_info.rs`, `tests/sources.rs` |
| 출발지 IP 차단 목록의 거절·끊김 사건과 원인 | `tests/serve.rs`, `tests/reload.rs`, `crates/adapters/tests/ops_info_contract.rs` |
| 로그인·관리자 종료·정상 종료·리스너 장애 | `tests/management_session.rs`, `tests/management_terminate.rs`, `tests/unix_shutdown.rs`, `tests/windows_console.rs`, `src/serve/listener_tests.rs` |

외부 수집 도구·상태 확인 도구는 저장소에 없으므로 Consumer 시험은 해당 없음이다. 3절은 외부 소비자를 위한 지침이다. Windows 트레이 앱의 `/readyz` 소비 동작은 [트레이 앱 스펙](../specs/windows-tray.md)의 수용 테스트를 따른다.

`waiting_expired`의 W 만료를 직접 검증하는 독립 스펙 시나리오는 없다. 이 계약의 사건 정의는 그대로 적용한다. 로그 형식 단위 시험의 통과는 실제 접속·매칭·자원 회수나 OS 신호를 검증했다는 뜻이 아니다.

### 계약 테스트 실행 기록

자동 시험의 현재 결과는 위 보고서에서 확인한다. 수동 조립 실행의 실제 OS 오류 관찰은 다음 기록을 유지한다.

| 날짜·대상 | 환경 | 명령·방법 | 실제 결과 |
| --- | --- | --- | --- |
| 2026-11-05, 제품 수명·리스너·중계 구성요소를 조립한 임시 `listener_smoke` 실행파일 | Windows x64, Linux(WSL) | 임시 `crates/adapters/examples/listener_smoke.rs`를 `cargo run -p vnc-relay-adapters --example listener_smoke`로 실행하고 제거했다. 비수신 소켓(Windows)과 바인드한 리스너의 안전한 shutdown(Linux)으로 실제 수락 오류를 발생시켰다. | Windows OS 오류 10022, Linux OS 오류 22가 감독자에게 전달됐다. `shutdown_started(error,fatal_error,G=2,active=1)` 뒤 `/readyz` 503 `{"reason":"listener_failed","status":"not_ready"}`와 `/livez` 200을 확인했다. 실제 5바이트 `smoke`가 중계됐다. 강제 정리·`session_closed(server_shutdown)`·`server_stopped`를 확인했고 최종 L=0·Lm=0이었다. 제품 바이너리 소켓의 외부 손상이나 CLI stderr 진단의 증거는 아니다. |
| 2026-11-02, `bf27d30`의 끊긴 연결 조건 | Ubuntu 24.04 LTS x64(WSL2), nftables v1.0.9 | 응답을 DROP하는 조건과 같은 조건의 커널 탐침에서 실제 소켓 오류를 관찰했다. | 실제 오류는 `ETIMEDOUT`(`Connection timed out`)이었다. `EHOSTUNREACH`·`ENETUNREACH`는 실제 오류로 관찰하지 않았다. 자동 시험의 통과 결과와는 구분한다. |

release 실행파일의 로그 파일 이름 변경·회전 수동 기록은 [CLI 계약 실행 기록](cli-v1.md#계약-테스트-실행-기록)을 따른다.

## 6. 초안을 확정하기 전에 해결할 사항

미결정 사항은 없다. 이 계약이 정하지 않는 범위는 다음과 같다.

- `reload_watch_failed` 복구 사건은 정의하지 않는다. 복구 통지가 필요하면 호환 사건 추가 규칙을 따른다.
- 로그 폭증 때의 기록 속도 제한은 정하지 않는다. 스펙에 요구가 없다.
- 중계 ID의 키 있는 해시 구현에 쓰는 라이브러리는 정하지 않는다.

