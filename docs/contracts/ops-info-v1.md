# 운영 로그와 상태 엔드포인트 v1 계약

VNC Relay가 운영자에게 내보내는 두 가지 정보의 교환 형식을 정한다. 하나는 JSON Lines 운영 로그이고, 다른 하나는 상태 리스너의 `/livez`·`/readyz` 응답이다. 로그 수집 도구와 상태 확인 도구가 이 문서의 표를 형식의 기준으로 쓴다. 외부 동작은 [운영 스펙](../specs/operations.md#35-운영-정보)이 소유하며, 이 문서는 그 동작을 바꾸지 않는다.

| 항목 | 값 |
| --- | --- |
| 이름과 버전 | `ops-info` 1.2.0. 1.1.0은 `startup_failed`의 `reason`에 `runtime_unavailable`을 더했다. 1.2.0은 `reason`에 `log_file_unavailable`을 더하고, 로그 쓰기 실패 처리(1.1절)를 정하고, `startup_failed`를 진단마다 한 줄로 정했다(1.6절). 모두 사용자가 결정했다(2026-10-20). 2026-11-01에 `handshake_invalid`의 조건 문장을 [Repeater 초기 교환 계약](repeater-handshake-v1.md) 채택(사용자 결정 2026-11-01(Q71·Q72))에 맞춰 "초기 요청의 형식 오류"로 고쳤다. 버전은 올리지 않았다. 빠진 조건 중 "미지원 모드"는 채택한 계약 2절에서 형식 오류에 들어간다. "I 초과"와 그때의 `limit`은 초기 요청이 250바이트 고정이고 I의 최솟값이 256이었으므로 생길 수 없다. 그래서 같은 입력이 다른 `reason`을 받는 경우가 없어 3절의 의미 변경으로 보지 않았다(이 문서의 판단, 사용자 결정 없음). 이 사건은 아직 구현하지 않았다. |
| 상태 | Active(2026-10-16 채택) |
| 소유자 | vnc-relay 프로젝트 관리자 |
| 만드는 쪽(Producer) | `vnc-relay` (`vnc-relay serve` 프로세스) |
| 사용하는 쪽(Consumer) | 운영자의 로그 수집 도구, 운영자의 상태 확인 도구(상태 점검·감시 도구) |
| 방향 | 로그: 발행(Producer가 표준 출력 또는 지정한 파일에 한 줄씩 씀). 상태: 요청/응답(Consumer가 상태 리스너에 `GET`) |
| 스키마 기준 파일 | 없음. 1절의 Markdown 표가 형식의 기준이다. |
| 기준 스펙 | [운영 스펙 3.5절](../specs/operations.md#35-운영-정보), [4절](../specs/operations.md#4-보안과-신뢰-경계), [5절](../specs/operations.md#5-오류와-복구), [중계 스펙](../specs/relay.md), [WS·WSS 스펙](../specs/websocket.md), [관리 스펙](../specs/management.md) |
| 관련 요구사항 | [PRD](../requirements/PRD.md) FR-08, FR-07, 7절 보안, AC-23, AC-25 |
| 호환성 범위 | 이 계약의 1.x 형식을 따르는 `vnc-relay` 구현과 소비자. 작업 2.3이 공통 규칙(1.1~1.4절), 시작 단계 사건(`server_starting`·`startup_failed`·`server_stopped`), 설정 경고 4종, `relay_id_ref` 계산(1.9절), `peer_ip` 표기(1.10절)를 구현했다. 작업 2.4·2.5가 TCP 진입점의 접속 사건과 그 `stage`·`reason` 일부를 구현했다. 작업 2.6이 `server_ready`, 상태 리스너의 `/livez`·`/readyz`(1.12·2절), `admin_connection_limit`을 구현했다. 그 밖의 사건과 `listener_failed`는 구현 전이다. |
| 관련 테스트 | [운영 스펙 7절](../specs/operations.md#7-수용-테스트)의 OPS-011·OPS-012·OPS-013·OPS-032를 중심으로 5절 표에 연결한다. 계약 테스트는 `crates/adapters/tests/ops_info_contract.rs`와 `tests/serve.rs`에 있다. |

관련 문서: [설정 계약](config-v2.md)(`[log]`·`[status]` 키, 경고 코드, 오류 코드, 재로드 결과 코드), [관리 API 계약](management-api-v1.md)(`session_id`·`close_cause`·바이트 필드 이름), [CLI·계정 파일 계약](cli-v1.md)(시작 실패 오류 코드), [WebSocket 중계 계약](websocket-relay-v1.md)(close 코드와 종료 원인 대응), [Repeater 초기 교환 계약](repeater-handshake-v1.md)(초기 교환 오류 코드), [ADR-0002 출발지 IP 허용 목록](../decisions/ADR-0002-source-ip-allowlist.md), [ADR-0003 단일 인스턴스·메모리 상태](../decisions/ADR-0003-single-instance-memory-state.md).

## 1. 형식과 의미

이 계약은 2026-10-16에 사용자가 채택했다(상태 Active). 필드 이름, 사건 이름, `level` 기준, 원인 코드, 중계 ID 노출 방식(1.9절), 상대 주소 표기(1.10절), 상태 응답 본문(1.12절)이 채택 범위다. 6절에 구현·관찰에서 확인한다고 남긴 항목은 채택으로 해소되지 않았다. 앞서 사용자가 확정한 값은 다음과 같다.

- 확정: 로그는 JSON Lines이고 한 줄에 사건 하나를 쓴다. 출력 위치는 표준 출력 또는 지정한 파일이며 VNC Relay는 출력만 한다. 관리자 종료 사건에는 계정명을 기록한다. 평문 WS 활성화와 전체 허용 항목은 경고로 기록한다. 상태 리스너는 관리 리스너와 분리하며 인증 없이 생존·준비 결과만 응답한다. 상태 응답에 세션 목록·중계 ID·상대 주소·인증 비밀을 넣지 않는다.

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
| 출력 실패 | 줄을 쓰지 못하면 그 줄을 버리고 서비스를 계속한다. 처음 실패할 때만 stderr에 `warning[log_write_failed] key=log.output: <message>`를 한 줄 쓴다([CLI 계약 1.5절](cli-v1.md#15-입력과-출력-규칙)의 형식). 이후 실패는 알리지 않는다(사용자 결정 2026-10-20). |

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

`debug` 같은 다른 값은 v1에 없다. 사건마다 `level`은 1.5절 표가 정한다. 종료 원인에 따라 달라지는 사건은 1.6절 표를 따른다.

### 1.4 필드 사전

사건 표에서 쓰는 필드의 뜻이다. 필드 이름 뒤 `?`는 선택 필드다. 같은 이름은 모든 사건에서 같은 뜻과 자료형을 가진다.

| 필드 | 자료형 | 의미 |
| --- | --- | --- |
| `conn_id` | 문자열 | 연결 식별자. 중계 리스너가 접속을 수락할 때 붙인다. 세션 식별자와 다른 값이며 프로세스 수명 안에서만 유일하다. 재시작 뒤 같은 값이 다시 나올 수 있다. |
| `session_id` | 문자열 | 세션 식별자. 관리 API 계약의 `session_id`와 같은 값이다. 형식은 관리 API 계약이 정한다([관리 스펙](../specs/management.md)은 외부 형식을 API 계약에 맡긴다). 세션이 생기기 전 사건에는 쓰지 않는다. |
| `listener` | 문자열 | `tcp_viewer`, `tcp_server`, `ws_viewer`, `wss_viewer`, `management`, `status` 중 하나. 앞의 넷이 진입점이다. |
| `role` | 문자열 | `viewer` 또는 `server`. 연결의 역할이다. WS·WSS 진입점은 항상 `viewer`다. |
| `mode` | 문자열 | `mode1` 또는 `mode2`. 초기 교환으로 모드를 알게 된 뒤의 사건에만 쓴다. |
| `viewer_transport` | 문자열 | Viewer 전송 경로. `tcp`, `ws`, `wss`. 관리 API 계약의 `viewer_transport`와 같은 이름·값이다. |
| `peer_ip` | 문자열 | 실제 소켓 상대 IP. 표기는 1.10절을 따른다. |
| `relay_id_ref` | 문자열 | 중계 ID의 참조 값. 중계 ID 원문이 아니다. 1.9절을 따른다. |
| `reason` | 문자열 | 사건이 일어난 이유의 코드. 값 목록은 사건마다 1.6절 이후 표가 정한다. |
| `error_kind` | 문자열 | 연결 오류의 세부 종류. 1.7절의 값. |
| `side` | 문자열 | 원인을 처음 낸 쪽. `viewer`, `server`, `target`(Mode I 대상) 중 하나. |
| `admin_account` | 문자열 | 관리자 계정명. 1.8절의 규칙을 따른다. |
| `config_key` | 문자열 | 문제가 있는 설정 키. [설정 계약 2.1절](config-v2.md#21-설정-오류와-준비-실패)의 진단 `key`와 같은 값이다(점으로 이은 전체 경로, 목록 항목은 0부터 센 `[i]` 첨자. 예: `relay.tcp_viewer.allow[2]`). 키 이름만 쓰고 값은 쓰지 않는다. |
| `detail` | 문자열 | 사람이 읽는 짧은 설명. 비밀과 설정 값을 넣지 않는다. 수집 도구는 이 필드로 분기하지 않는다. |
| `elapsed_ms`, `waited_ms`, `duration_ms` | 정수 | 밀리초. 단조 시계로 잰 경과 시간이다. 음수가 되지 않는다. |
| `limit` | 정수 | 거절 원인이 된 상한의 설정값. |
| `phase` | 문자열 | `startup` 또는 `reload`. |

### 1.5 사건 목록

각 사건은 아래 표에 있다. "스펙 근거"는 사건을 기록하라고 요구하거나 사건의 조건을 정한 스펙 시나리오 ID다. 시나리오 ID는 [운영](../specs/operations.md#7-수용-테스트)(OPS), [중계](../specs/relay.md#7-수용-테스트)(RLY), [WS·WSS](../specs/websocket.md#7-수용-테스트)(WS), [관리](../specs/management.md#7-수용-테스트)(MGT) 스펙의 수용 테스트 표에 있다.

#### 시작·준비·종료 단계

단계 이름은 [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)의 상태에 대응한다.

| 사건 | `level` | 발생 시점 | 필드 | 스펙 근거 |
| --- | --- | --- | --- | --- |
| `server_starting` | `info` | 설정이 유효하고 로그 출력을 연 뒤 시작 중 상태로 들어갔을 때 | `log_contract`: 문자열. 값은 `ops-info/1.2.0`이다. | OPS-001, OPS-010 |
| `server_ready` | `info` | 필수 리스너와 접근 경계가 모두 준비돼 준비됨 상태로 들어갔을 때 | `listeners`: 객체 배열. 각 객체는 `name`(`listener` 값)과 `listen`(바인딩 주소 문자열)을 가진다. | OPS-002, OPS-013, OPS-032 |
| `startup_failed` | `error` | 시작 실패가 확정됐을 때. 자원을 회수한 뒤 프로세스가 끝난다. | `reason`(시작을 막은 오류 코드, 1.6절), `config_key?`, `listener?`, `detail?`. | OPS-001, OPS-002, OPS-028, OPS-034, OPS-038, OPS-039, OPS-040 |
| `shutdown_started` | `info`. `trigger`가 `fatal_error`이면 `error` | 종료 유예 중 상태로 들어갔을 때. 이후 신규 중계 접속을 받지 않는다. | `trigger`: `requested` 또는 `fatal_error`. `grace_secs`: 정수(G). `active_sessions`: 정수. `detail?` | OPS-007, OPS-008 |
| `shutdown_forced` | `warn` | G가 지나거나 유예 중 두 번째 종료 요청을 받아 남은 활성 세션을 강제 정리하기 시작했을 때. 남은 세션이 없으면 기록하지 않는다. | `remaining_sessions`: 정수. | OPS-009, OPS-042 |
| `server_stopped` | `info` | 자원 정리를 마치고 프로세스가 끝나기 직전 | 없음 | OPS-010 |

순서 규칙: `server_starting`은 시작 단계의 첫 사건이다. 시작 경고(아래 "설정 경고"의 사건)는 `server_starting` 뒤, `server_ready` 앞에 쓴다. `shutdown_forced`는 `server_shutdown` 원인의 `session_closed`보다 앞에 쓴다. `server_stopped`는 마지막 줄이다.

#### 설정 경고

사건 이름은 [설정 계약 2.2절](config-v2.md#22-경고)의 경고 코드와 같다. 발생 조건과 전체 허용 항목의 판정 범위는 그 절과 [1.3절](config-v2.md#13-ip-항목-문법)이 정한다. 이 계약은 필드만 정한다. `item`은 항목 표기만 담으며 비밀이 아니다.

| 사건 | `level` | 필드 | 스펙 근거 |
| --- | --- | --- | --- |
| `plaintext_ws_enabled` | `warn` | `listener`: 항상 `ws_viewer`. `listen`: 바인딩 주소 문자열. | OPS-016, WS-024 |
| `allow_all_source` | `warn` | `listener`: 진입점. `item`: 항목 문자열. `phase`. 진입점과 항목마다 한 줄이다. | OPS-038 |
| `allow_all_target` | `warn` | `item`: 대상 항목 문자열. `phase`(항상 `startup`). 항목마다 한 줄이다. | RLY-007 |
| `status_listen_not_loopback` | `warn` | `listen`: 바인딩 주소 문자열. | OPS-032 |

#### 자동 재로드

재로드 대상은 진입점의 `allow`와, `cert_file`·`key_file`이 가리키는 파일 내용과 그 경로 값이다. 정의는 [운영 스펙 3.3절](../specs/operations.md#33-파일-변경-자동-감지와-부분-재로드)을 따른다.

| 사건 | `level` | 발생 시점 | 필드 | 스펙 근거 |
| --- | --- | --- | --- | --- |
| `reload_change_detected` | `info` | 설정·인증서·개인키 파일의 변경을 처음 감지했을 때 | `source`: `os_notify`(OS 파일 알림) 또는 `poll`(주기 확인). 처음 감지한 쪽이다. | OPS-024, OPS-041 |
| `reload_applied` | `info` | 후보를 검증하고 재로드 대상 값을 원자적으로 교체했을 때 | `applied`: 문자열 배열. 값은 [설정 계약 1.8절](config-v2.md#18-재로드-구분)의 재로드 대상 키(`relay.tcp_viewer.allow`, `relay.tcp_server.allow`, `relay.ws_viewer.allow`, `relay.wss_viewer.allow`, `relay.wss_viewer.cert_file`, `relay.wss_viewer.key_file`, `management.cert_file`, `management.key_file`) 중에서 경로 값이나 파일 내용이 실제 바뀐 것이다. | OPS-024, OPS-025, OPS-035, OPS-040 |
| `reload_rejected` | `warn` | 후보 검증에 실패해 후보 전체를 거절했을 때. 기존 설정과 연결은 그대로다. | `reason`(설정 오류 코드, 1.6절), `config_key?`, `detail?`. | OPS-026, OPS-027, OPS-040 |
| `reload_restart_required` | `warn` | 재로드 대상이 아닌 값이 실행 중 값과 달라 후보 전체를 거절했을 때. 차이가 남은 동안 이후 변경에도 기록한다. | `config_keys`: 문자열 배열. 다른 값을 가진 설정 키의 경로. 값은 쓰지 않는다. | OPS-003, OPS-035 |
| `reload_watch_failed` | `error` | 실행 중 파일 변경 감지(OS 알림 또는 주기 확인)에 장애가 생겼을 때. 이전 유효 설정으로 계속 운영한다. | `reason`: `notify_error` 또는 `poll_error`. `detail?` | OPS-028 |

`reload_rejected`와 `reload_restart_required`는 서로 다른 사건이다. 재시작 필요는 검증 실패로 세지 않는다. 자동 재로드가 성공하지 않았는데 `reload_applied`를 쓰지 않는다. 시작 전에 감지 준비가 불가능하면 `reload_watch_failed`가 아니라 `startup_failed`(`watch_unavailable`)다.

#### 접속 수락·거절·중단

| 사건 | `level` | 발생 시점 | 필드 | 스펙 근거 |
| --- | --- | --- | --- | --- |
| `connection_accepted` | `info` | 중계 리스너가 접속을 수락하고 출발지 IP 검사와 L 검사를 통과했을 때 | `conn_id`, `listener`(진입점), `role`, `peer_ip` | RLY-001, OPS-037, WS-021 |
| `connection_rejected` | `warn` | 서버가 정책·제한·형식 때문에 접속을 거절했을 때 | `listener`, `stage`, `reason`, `peer_ip`. 선택: `conn_id`, `role`, `mode`, `relay_id_ref`, `limit`, `origin`, `target_host`, `target_ip`, `target_port`. 값 목록은 1.6절. | RLY-004, RLY-007, RLY-008, RLY-022, RLY-023, OPS-004, OPS-005, OPS-023, OPS-036, OPS-037, WS-012, WS-014, WS-015, WS-022, WS-023, WS-027 |
| `connection_aborted` | `warn`. `reason`이 `peer_eof` 또는 `server_shutdown`이면 `info` | 수락한 연결이 대기 등록이나 세션 시작 전에 끝났을 때. 서버가 정책으로 거절한 경우는 제외한다. | `conn_id`, `listener`, `stage`, `reason`, `peer_ip`, `elapsed_ms`. 선택: `role`, `mode`, `error_kind`. 값 목록은 1.6절. | RLY-005, OPS-006, OPS-007, WS-019, WS-023 |

`conn_id`는 `connection_accepted`를 기록한 연결에만 있다. 접속 수락 단계에서 거절한 연결(`stage`가 `accept`)은 수락 사건이 없으므로 `conn_id`도 없다.

관리·상태 리스너는 `connection_accepted`를 기록하지 않는다. 폴링 요청이 로그를 채우지 않게 하기 위해서다. 두 리스너의 거절(Lm 도달, 관리 HTTPS의 TLS 1.2 미만)은 `connection_rejected`로 기록한다.

#### Mode II 대기

| 사건 | `level` | 발생 시점 | 필드 | 스펙 근거 |
| --- | --- | --- | --- | --- |
| `waiting_registered` | `info` | 유효한 Mode II 요청을 대기에 등록했을 때 | `conn_id`, `listener`, `role`, `mode`(항상 `mode2`), `relay_id_ref`, `peer_ip` | RLY-010, RLY-011, OPS-005 |
| `waiting_expired` | `info` | 대기 시간 W가 지나 대기 연결을 제거했을 때 | `conn_id`, `role`, `relay_id_ref`, `waited_ms` | 시나리오 없음. [중계 스펙 4절](../specs/relay.md#4-오류와-복구) "대기 단절·만료", RLY-R09, [관리 스펙 2.3절](../specs/management.md#23-최근-종료-목록) "대기 만료는 로그에만 남긴다", OPS 5절 W |
| `waiting_removed` | `info`. `reason`이 `connection_error` 또는 `dead_connection`이면 `warn` | 대기 연결이 만료 외의 이유로 제거됐을 때 | `conn_id`, `role`, `relay_id_ref`, `reason`, `waited_ms`. 선택: `error_kind`. `reason`은 `peer_eof`, `connection_error`, `dead_connection`, `server_shutdown` 중 하나. | RLY-025, OPS-007 |

#### 매칭·대상 연결·세션

| 사건 | `level` | 발생 시점 | 필드 | 스펙 근거 |
| --- | --- | --- | --- | --- |
| `session_started` | `info` | Mode II 매칭이 성공했거나 Mode I 대상 연결이 성공해 활성 세션이 생겼을 때 | `session_id`, `mode`, `viewer_transport`, `viewer_conn_id`, `viewer_peer_ip`. Mode II 전용: `server_conn_id`, `server_peer_ip`, `relay_id_ref`, `waited_ms`(먼저 대기한 연결의 대기 시간). Mode I 전용: `target_ip`, `target_port`, `target_host?`. | RLY-006, RLY-010, RLY-011, MGT-001, MGT-009 |
| `target_connect_failed` | `warn` | Mode I 대상 연결을 시도했지만 실패했을 때. 정책 거절은 `connection_rejected`이며 이 사건이 아니다. | `conn_id`, `listener`, `viewer_transport`, `reason`, `peer_ip`, `target_host`, `target_port`, `elapsed_ms`. 선택: `target_ip`. `reason`은 `resolve_failed`, `connect_refused`, `connect_timeout`, `connect_error` 중 하나. | RLY-009, RLY-R06, OPS-023 |
| `session_closed` | 원인별. 1.6절 | 활성 세션의 정리가 끝났을 때. 한 세션에 한 번만 쓴다. | `session_id`, `mode`, `viewer_transport`, `started_at`(UTC ISO 8601), `duration_ms`, `bytes_viewer_to_target`, `bytes_target_to_viewer`, `close_cause`. 선택: `side`, `error_kind`, `relay_id_ref`. `close_cause`가 `admin_terminated`이면 `admin_account`가 필수다. | RLY-019, RLY-020, RLY-026, WS-016, WS-017, WS-026, OPS-008, OPS-009, MGT-009, MGT-023 |

`bytes_viewer_to_target`과 `bytes_target_to_viewer`는 [관리 API 계약 1.8절](management-api-v1.md#18-활성-세션-get-apiv1sessions-get-apiv1sessionssession_id)의 같은 이름 필드와 방향·의미가 같다([관리 스펙 2.1절](../specs/management.md#21-전송량의-의미)). 상대 전송 계층에 쓰기에 성공한 바이트 정수만 쓴다.

`session_started`와 `session_closed`의 `session_id`는 서로 짝이다. 같은 `session_id`에서 `session_started`가 먼저 온다. 같은 중계 ID의 새 세션은 새 `session_id`를 가진다(RLY-R11).

#### 관리자 로그인과 종료

로그인 사건은 로그인 시도 자체를 로그에 남기라고 요구한 스펙 문장이 없다. [운영 스펙 OPS-R06](../specs/operations.md#41-판정-가능한-규칙)과 [관리 스펙 3.0절](../specs/management.md#30-관리자-로그인과-세션)의 제한 동작을 근거로 정한 사건이며 채택 범위다.

| 사건 | `level` | 발생 시점 | 필드 | 스펙 근거 |
| --- | --- | --- | --- | --- |
| `admin_login_succeeded` | `info` | 로그인에 성공해 관리 세션을 발급했을 때 | `admin_account`, `peer_ip` | MGT-017, OPS-029 |
| `admin_login_failed` | `warn` | 로그인 요청을 거절했을 때 | `reason`: `invalid_credentials` 또는 `limited`. `peer_ip`. 선택: `admin_account`. `reason`이 `limited`이면 `scope`(`account` 또는 `source`)도 쓴다. | MGT-018, MGT-022 |
| `admin_login_limited` | `warn` | 실패 횟수가 F를 넘어 제한을 시작했을 때. 제한 중 시도마다 기록하지 않는다(그 시도는 `admin_login_failed`다). | `scope`: `account` 또는 `source`. `peer_ip`(기준을 넘게 한 시도의 출발지). `lockout_secs`: 정수. 선택: `admin_account`. | MGT-022, MGT-R14 |
| `admin_terminate` | `info`. `result`가 `failed`이면 `error` | 유효한 관리 세션과 CSRF 검증을 통과한 세션 종료 요청을 처리했을 때. 응답과 같은 시점이며 정리가 끝난 뒤다. | `admin_account`, `result`(`terminated`, `not_found`, `failed`), `peer_ip`. `result`가 `terminated` 또는 `failed`이면 `session_id`. | OPS-011, MGT-009, MGT-010, MGT-011, MGT-012 |

`result`는 [관리 스펙 2절](../specs/management.md#2-조회-대상과-상태)의 종료 결과(종료 완료, 대상 없음, 종료 실패)에 대응한다. 유효한 관리 세션이 없는 요청은 관리 세션 없음으로 응답하며 이 사건을 기록하지 않는다. `result`가 `not_found`이면 `session_id`를 쓰지 않는다. 존재하지 않는 세션 식별자를 연결된 것처럼 기록하지 않기 위해서다.

`close_cause`가 `admin_terminated`인 `session_closed`가 먼저 오고, 같은 요청의 `admin_terminate`가 뒤따른다. 두 줄 모두 같은 `admin_account`를 가진다.

### 1.6 원인 코드

`reason`과 `close_cause`의 값이다. 사건마다 쓸 수 있는 값이 다르다.

#### `startup_failed`와 `reload_rejected`의 `reason`

값은 다른 계약이 정한 오류 코드다. 이 계약은 코드를 다시 정의하지 않는다.

| 사건 | `reason` 값의 출처 |
| --- | --- |
| `startup_failed` | [설정 계약 2.1절](config-v2.md#21-설정-오류와-준비-실패)의 `config_*`·`tls_file_invalid`·`log_file_unavailable`, [CLI 계약 2.2절](cli-v1.md#22-오류-코드)의 `accounts_*`·`listener_bind_failed`·`watch_unavailable`·`runtime_unavailable` |
| `reload_rejected` | [설정 계약 2.1절](config-v2.md#21-설정-오류와-준비-실패)의 `config_*`·`tls_file_invalid` |

`startup_failed`는 진단 하나에 한 줄을 쓴다. stderr의 오류 줄([CLI 계약 1.5절](cli-v1.md#15-입력과-출력-규칙))과 1:1로 대응한다. 같은 오류 코드의 진단이 여럿이면 줄도 여럿이며, 줄마다 그 진단의 `config_key`·`listener`를 쓴다(사용자 결정 2026-10-20). 종료 코드는 [CLI 계약 2.1절](cli-v1.md#21-종료-코드)이 정한다. 인증서·개인키 한쪽만 바뀐 후보와 참조 파일을 읽을 수 없는 후보는 `reload_rejected`의 `tls_file_invalid`다.

#### `connection_rejected`의 `stage`와 `reason`

| `stage` | 뜻 | `reason` | 의미 | 근거 |
| --- | --- | --- | --- | --- |
| `accept` | 접속 수락 직후 | `ip_not_allowed` | 출발지 IP가 해당 진입점 허용 목록에 없음. 데이터를 읽거나 TLS를 협상하기 전에 거절한다. | RLY-023, OPS-037, WS-014, WS-023 |
| `accept` | 접속 수락 직후 | `relay_connection_limit` | 중계 연결 상한 L에 도달함. `limit`을 쓴다. | OPS-004 |
| `accept` | 접속 수락 직후 | `admin_connection_limit` | 관리·상태 연결 상한 Lm에 도달함. `listener`는 `management` 또는 `status`다. `limit`을 쓴다. | OPS-033 |
| `tls` | TLS 협상 | `tls_version_unsupported` | TLS 1.2 미만 협상. `listener`는 `wss_viewer` 또는 `management`다. | OPS-036 |
| `upgrade` | WebSocket 업그레이드 직전 | `ip_not_allowed` | 업그레이드 직전 재검사에서 출발지 IP가 허용 목록에 없음 | WS-022 |
| `upgrade` | WebSocket 업그레이드 직전 | `origin_not_allowed` | Origin 헤더 값이 허용 값과 정확히 일치하지 않음. `origin`에 헤더 값을 쓴다. | WS-015, WS-027 |
| `upgrade` | WebSocket 업그레이드 직전 | `invalid_upgrade` | 업그레이드 경로가 아니거나 업그레이드 요청이 잘못됨. 관리 API 경로 요청도 여기에 속한다. | WS-012 |
| `upgrade` | WebSocket 업그레이드 직전 | `subprotocol_not_supported` | 서브프로토콜이 지원 값(`binary`)이 아닌 값만 제시됨 | [WS·WSS 스펙 4절](../specs/websocket.md#4-오류와-종료)에 코드 없음. WebSocket 계약이 정함 |
| `initial_exchange` | Repeater 초기 교환 | `handshake_invalid` | 초기 요청의 형식 오류. [Repeater 계약 2절](repeater-handshake-v1.md#2-오류)의 같은 이름 코드다. | RLY-004 |
| `initial_exchange` | Repeater 초기 교환 | `mode_disabled` | 설정으로 끈 모드의 요청. `mode`를 쓴다. | RLY-022 |
| `register` | 대기 등록 | `waiting_full` | 대기 상한 Lw에 도달함. `limit`을 쓴다. | OPS-005 |
| `register` | 대기 등록 | `duplicate_id` | 같은 중계 ID와 같은 역할이 이미 대기 중이거나 활성 세션이 점유함. 후발 연결을 거절한다. `relay_id_ref`를 쓴다. | [중계 스펙 3.3절](../specs/relay.md#33-mode-ii) 4항(사용자 결정 2026-11-01(Q74)), RLY-015 |
| `target_connect` | Mode I 대상 연결 준비 | `target_not_allowed` | 최종 대상 IP·포트가 허용 정책을 만족하지 못함. `target_host`, `target_ip?`, `target_port`를 쓴다. | RLY-007, RLY-008 |
| `target_connect` | Mode I 대상 연결 준비 | `relay_connection_limit` | 대상 연결 소켓을 위한 L 예산이 없음. `limit`을 쓴다. | OPS-023 |

#### `connection_aborted`의 `stage`와 `reason`

`stage`는 `tls`, `upgrade`, `initial_exchange`, `target_connect` 중 하나다.

| `reason` | 의미 | 근거 |
| --- | --- | --- |
| `handshake_timeout` | Repeater 초기 교환이 H 안에 끝나지 않음 | RLY-005, OPS-006 |
| `tls_upgrade_timeout` | TLS 협상 또는 HTTP 업그레이드가 U 안에 끝나지 않음. U가 지난 뒤 상대가 닫은 경우도 이 사유다(사용자 결정 2026-10-22(Q28)). | OPS-006, WS-023 |
| `peer_eof` | 상대가 초기 교환을 마치기 전에 연결을 닫음(미완료 단절). `stage`가 `initial_exchange`이면 [Repeater 계약 2절](repeater-handshake-v1.md#2-오류)의 `handshake_incomplete`다. | RLY-005 |
| `connection_error` | 네트워크·프로토콜 오류. `error_kind`를 함께 쓴다. | WS-019 |
| `dead_connection` | 응답하지 않는 상대를 TCP keepalive 또는 WebSocket ping으로 판정함 | RLY-026 |
| `server_shutdown` | 정상 종료가 시작돼 미완성 작업을 종료함 | OPS-007 |

#### `session_closed`의 `close_cause`와 `level`

`close_cause`의 값 이름과 의미는 [관리 API 계약 1.10절](management-api-v1.md#110-최근-종료-목록-get-apiv1closed-sessions)이 정한다([관리 스펙 2.3절](../specs/management.md#23-최근-종료-목록)의 종료 원인과 같은 뜻이다). 이 표는 로그에서 값마다 쓰는 `level`·`side`와 추가 필드만 정한다. [WebSocket 계약 2.2절](websocket-relay-v1.md#22-close-코드와-스펙-오류의-대응)이 같은 값을 close 코드에 대응시킨다.

| `close_cause` | `level` | `side` | 추가 규칙 | 근거 |
| --- | --- | --- | --- | --- |
| `peer_eof` | `info` | 먼저 끝낸 쪽 | 없음 | RLY-019, RLY-020, WS-016 |
| `connection_error` | `warn` | 오류가 난 쪽 | `error_kind`를 함께 쓴다. | RLY-020, WS-016, WS-017 |
| `admin_terminated` | `info` | 없음 | `admin_account`가 필수다. | MGT-009, OPS-011 |
| `dead_connection` | `warn` | 응답이 없는 쪽 | 없음 | RLY-026, WS-026 |
| `close_timeout` | `warn` | 먼저 EOF를 낸 쪽 | 없음 | WS-017, RLY-019 |
| `server_shutdown` | `warn` | 없음 | 없음 | OPS-009, OPS-042 |

한 세션의 `close_cause`는 정리를 시작하게 한 첫 원인이다. 그 뒤 생긴 정리 오류로 바꾸지 않는다. 단, 한쪽 EOF 뒤 E가 지나 정리하면 `close_cause`는 `close_timeout`이다.

### 1.7 `error_kind` 값

`connection_error`일 때 `connection_aborted`, `waiting_removed`, `session_closed`가 함께 쓴다.

| 값 | 의미 | 근거 |
| --- | --- | --- |
| `io_error` | 소켓 읽기·쓰기 오류, 연결 재설정 | RLY-020 |
| `tls_failed` | TLS 협상 실패(인증서 검증 실패 등). TLS 1.2 미만은 `connection_rejected`다. | WS-013 |
| `ws_protocol_error` | 잘못된 프레임 순서, 유효하지 않은 제어 프레임 | WS-019 |
| `ws_text_message` | 텍스트 데이터 메시지를 받음 | WS-008 |
| `ws_message_too_large` | 논리 메시지가 M을 넘음 | WS-010 |

### 1.8 관리자 계정명 기록 규칙

- 관리자 종료(`admin_terminate`, `admin_terminated` 원인의 `session_closed`)와 로그인 성공에는 `admin_account`가 필수다. 로그인한 계정이므로 존재가 확정된 값이다.
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

- `peer_ip`는 실제 소켓 상대 IP다. 전달 헤더(`X-Forwarded-For` 등)로 바꾸지 않고, 로그에도 쓰지 않는다([운영 스펙 4절](../specs/operations.md#4-보안과-신뢰-경계), MGT-015).
- 포트는 쓰지 않는다. 클라이언트의 임시 포트는 추적 가치가 낮고, 노출을 줄이기 위해서다.
- IPv4는 점으로 구분한 십진수다. IPv6는 RFC 5952의 소문자 축약형이며 영역 식별자(`%` 뒤)는 쓰지 않는다.
- 값은 출발지 IP 허용 목록 판정에 쓴 주소와 같아야 한다. 거절 원인을 로그만으로 설명하기 위해서다.
- 같은 공인 IP를 공유하는 사용자는 구분하지 못한다. `peer_ip`는 개별 사용자 식별자가 아니다.
- Mode I 대상은 `target_ip`·`target_port`로 쓴다. 정책 판정에 쓴 최종 IP·포트다. 이름이 있으면 `target_host`에 요청한 값을 쓴다.

### 1.11 로그에 넣지 않는 값

어떤 사건, 어떤 `level`, 어떤 `detail`에도 아래 값을 쓰지 않는다. 오류 메시지 전달 과정에서도 같다.

| 값 | 비고 |
| --- | --- |
| 관리자 평문 비밀번호, Argon2id 해시 | OPS-R07, OPS-030 |
| 관리 세션 쿠키(`relay_admin_session`) 값, `csrf_token` 값, `Cookie`·`Set-Cookie`·`X-CSRF-Token` 헤더 값 | OPS-012, MGT-013 |
| TLS 개인키 내용 | 인증서·개인키의 파일 경로와 설정 키 이름은 써도 된다. |
| 로그인 요청 본문 | 입력한 계정명은 1.8절 규칙으로만 쓴다. |
| VNC 인증 데이터(챌린지·응답), 화면 데이터, 사용자 입력, 원시 페이로드 | OPS-012, [WS·WSS 스펙 5절](../specs/websocket.md#5-항상-지켜야-할-규칙과-비보장-범위) |
| Repeater 초기 교환의 원시 바이트 | 중계 ID 원문이 들어 있다. |
| 중계 ID 원문 | 1.9절 |
| 전달 헤더 값 | 접근 판정에 쓰지 않는 값이다. |
| 설정 값 | 오류 사건은 설정 키 이름(`config_key`)만 쓴다. 항목 내용은 쓰지 않는다. |

### 1.12 상태 리스너와 엔드포인트

상태 리스너는 관리 리스너와 다른 리스너다. 설정 키는 `[status] listen`이다. 필수 키이며 기본 포트는 없다([설정 계약 1.2절](config-v2.md#12-키-표)). 루프백을 쓰기를 권장하며 루프백이 아니면 경고 `status_listen_not_loopback`을 기록한다. 상태 리스너는 평문 HTTP다. 상태 연결은 관리 연결과 함께 Lm에서 센다. Lm에 도달하면 새 연결을 거절하고, 이때 `connection_rejected`(`admin_connection_limit`)를 기록한다.

| 항목 | 값 |
| --- | --- |
| 인증 | 없다. `Authorization`·`Cookie` 헤더가 있어도 무시한다. 대시보드의 로그인 예외 규칙과 무관하다. |
| 메서드 | `GET`만 받는다. 다른 메서드는 405다. |
| 쿼리 문자열·본문 | 무시한다. |
| 본문 형식 | `Content-Type: application/json`, UTF-8. `Cache-Control: no-store`를 보낸다. |
| 본문에 넣는 필드 | 아래 표의 필드만. 세션 목록, 중계 ID, 상대 주소, 인증 비밀, 연결 수, 세션 수, 가동 시간, 버전은 넣지 않는다. |

| 경로 | 조건 | 상태 코드 | 본문 |
| --- | --- | --- | --- |
| `GET /livez` | 프로세스가 요청에 응답할 수 있음 | 200 | `{"status":"alive"}` |
| `GET /readyz` | 준비됨 상태 | 200 | `{"status":"ready"}` |
| `GET /readyz` | 준비됨 상태가 아님 | 503 | `{"status":"not_ready","reason":"<code>"}` |

`/livez`는 종료 유예 중에도 프로세스가 응답하는 한 200이다. 생존 여부를 묻는 경로이므로 준비 여부와 섞지 않는다. `/readyz`의 `reason`은 2절 표의 값이다.

상태 리스너는 설정 검증이 끝난 뒤, 다른 필수 리스너를 준비하기 전에 연다. 그래야 시작 중 상태 조회가 `starting`으로 구분된다. 상태 리스너 바인딩에 실패하면 `startup_failed`(`listener_bind_failed`, `listener`가 `status`)다. 설정 검증 중이거나 프로세스가 없을 때의 조회는 연결 거절로 보인다. 도구는 이를 준비 안 됨으로 취급한다.

## 2. 오류

상태 엔드포인트의 응답 코드다. 로그 사건의 원인 코드는 1.6절과 1.7절에 있다.

| 오류 코드 | 의미·형식 | 관련 스펙 시나리오 |
| --- | --- | --- |
| `starting` | 준비 전. `/readyz`가 503과 `{"status":"not_ready","reason":"starting"}`를 응답한다. 설정 검증 뒤 시작 중 상태에서 쓴다. 필수 리스너가 모두 준비되면 200이 된다. | OPS-013, OPS-002 |
| `shutting_down` | 종료 유예 중 또는 종료 정리 중. `/readyz`가 503과 `{"status":"not_ready","reason":"shutting_down"}`를 응답한다. 신규 중계 접속을 받지 않는다. | OPS-013, OPS-007, OPS-043 |
| `listener_failed` | 준비된 필수 리스너가 계속 운영할 수 없게 실패함. `/readyz`가 503과 `{"status":"not_ready","reason":"listener_failed"}`를 응답한다. 이 상태는 곧 `shutdown_started`(`fatal_error`)로 이어진다. | OPS-002, OPS-013 |
| `not_found` | 정의하지 않은 경로. 404와 `{"error":{"code":"not_found","message":"<사람용 문장>"}}`. 관리 API의 오류 본문과 같은 모양이다. | OPS-032 |
| `method_not_allowed` | `GET`이 아닌 메서드. 405와 `Allow: GET` 헤더, `{"error":{"code":"method_not_allowed","message":"<사람용 문장>"}}`. | OPS-032 |

Lm 상한에서 거절한 연결은 HTTP 응답을 받지 못하고 연결이 닫힌다. 이 경우는 오류 코드가 없다([운영 스펙 5절](../specs/operations.md#5-오류와-복구) Lm 행).

## 3. 호환성 규칙

- 소비자는 알 수 없는 필드를 무시해야 한다. 알 수 없는 `event` 줄은 버리지 않고 알 수 없는 사건으로 보존하거나 건너뛴다. 알 수 없는 `level`·`reason`·`close_cause`·`error_kind` 값도 오류로 처리하지 않는다.
- 소비자는 `event`, `level`, 사건별 필수 필드로 분기해야 한다. `detail` 문자열로 분기하지 않는다.
- 상태 도구는 HTTP 상태 코드로 생존·준비를 판정해야 한다. 200은 정상이고, 503은 준비 안 됨이다. 본문은 보조 정보다. 본문의 알 수 없는 `reason` 값을 오류로 처리하지 않는다.
- 호환 변경: 사건 추가, 기존 사건에 선택 필드 추가, `reason`·`close_cause`·`error_kind`·`stage` 값 추가, 상태 응답 본문에 선택 필드 추가.
- 호환성을 깨는 변경(Breaking change): 사건·필드 이름 변경이나 삭제, 필드 자료형·단위·의미 변경, 필수 필드 삭제, 기존 `reason`·`close_cause` 값의 삭제나 의미 변경, `ts` 형식 변경, 줄 구분·인코딩 변경, `level` 값의 의미 변경, 상태 엔드포인트의 경로·상태 코드·`/readyz` 준비 판정 의미 변경. 이런 변경은 새 major 버전(`ops-info` 2.0.0)과 함께 한다. 마이그레이션은 소비자를 새 형식에 먼저 맞춘 뒤 `vnc-relay`를 올리는 순서로 한다. 소비자는 `server_starting`의 `log_contract`로 버전을 구분한다. 롤백은 이전 버전의 `vnc-relay`를 다시 시작하는 것이다. 영속 상태가 없으므로(ADR-0003) 데이터 되돌림은 필요 없다. 마이그레이션·롤백 계획은 major 변경 때 별도로 작성한다.

## 4. 버전과 지원 단계

| 상태 | 조건 | 지원 종료·대체 |
| --- | --- | --- |
| Draft | 채택 전 상태. 이 계약은 2026-10-16에 채택해 Draft를 지났다. | 해당 없음 |
| Active | 현재. 사용자 채택(2026-10-16). 양쪽 계약 테스트의 실행 기록과 구현 증거는 5절 표가 관리하며, `통과`가 되기 전에는 지원 완료로 표시하지 않는다. | 새 major가 Active가 되면 이전 major를 Deprecated로 표시한다. |
| Deprecated | 새 major가 있고 이전 major를 쓰는 소비자가 남아 있다. | 신규 적용을 피하고 대체 문서를 확인한다. |
| Retired | 지원 종료. | 새 구현의 근거로 쓰지 않는다. |

## 5. 예제와 계약 테스트

예제의 주소는 문서용 주소(`192.0.2.0/24`, `198.51.100.0/24`, `203.0.113.0/24`, `2001:db8::/32`)다. 값은 모두 예시이며 실제 비밀이 아니다.

로그 줄(실제로는 줄바꿈 없이 한 줄씩 쓴다).

```jsonl
{"ts":"2026-10-16T01:00:00.000Z","level":"info","event":"server_starting","log_contract":"ops-info/1.2.0"}
{"ts":"2026-10-16T01:00:00.010Z","level":"warn","event":"plaintext_ws_enabled","listener":"ws_viewer","listen":"127.0.0.1:8080"}
{"ts":"2026-10-16T01:00:00.011Z","level":"warn","event":"allow_all_source","listener":"tcp_viewer","item":"0.0.0.0/0","phase":"startup"}
{"ts":"2026-10-16T01:00:00.120Z","level":"info","event":"server_ready","listeners":[{"name":"tcp_viewer","listen":"0.0.0.0:5900"},{"name":"status","listen":"127.0.0.1:9000"}]}
{"ts":"2026-10-16T01:02:03.456Z","level":"info","event":"connection_accepted","conn_id":"c-1001","listener":"tcp_server","role":"server","peer_ip":"198.51.100.21"}
{"ts":"2026-10-16T01:02:03.470Z","level":"info","event":"waiting_registered","conn_id":"c-1001","listener":"tcp_server","role":"server","mode":"mode2","relay_id_ref":"3fa9c0d1e27b4a68","peer_ip":"198.51.100.21"}
{"ts":"2026-10-16T01:02:10.002Z","level":"info","event":"session_started","session_id":"ses_3k9d2m7q4x8v1b6n5c0z2h4w7y","mode":"mode2","viewer_transport":"wss","viewer_conn_id":"c-1002","viewer_peer_ip":"2001:db8::5","server_conn_id":"c-1001","server_peer_ip":"198.51.100.21","relay_id_ref":"3fa9c0d1e27b4a68","waited_ms":6532}
{"ts":"2026-10-16T01:05:00.100Z","level":"info","event":"session_closed","session_id":"ses_3k9d2m7q4x8v1b6n5c0z2h4w7y","mode":"mode2","viewer_transport":"wss","started_at":"2026-10-16T01:02:10.002Z","duration_ms":170098,"bytes_viewer_to_target":10240,"bytes_target_to_viewer":4194304,"close_cause":"admin_terminated","admin_account":"admin-a","relay_id_ref":"3fa9c0d1e27b4a68"}
{"ts":"2026-10-16T01:05:00.101Z","level":"info","event":"admin_terminate","admin_account":"admin-a","result":"terminated","session_id":"ses_3k9d2m7q4x8v1b6n5c0z2h4w7y","peer_ip":"203.0.113.7"}
{"ts":"2026-10-16T01:06:00.000Z","level":"warn","event":"connection_rejected","listener":"wss_viewer","stage":"upgrade","reason":"origin_not_allowed","peer_ip":"203.0.113.9","conn_id":"c-1003","origin":"https://x.a.example:8443"}
{"ts":"2026-10-16T01:07:00.000Z","level":"warn","event":"reload_rejected","reason":"config_ip_item_invalid","config_key":"relay.tcp_viewer.allow[2]"}
{"ts":"2026-10-16T01:08:00.000Z","level":"error","event":"startup_failed","reason":"listener_bind_failed","listener":"management","detail":"주소가 이미 사용 중이다."}
```

상태 응답.

```http
GET /livez  ->  200 {"status":"alive"}
GET /readyz ->  200 {"status":"ready"}
GET /readyz ->  503 {"status":"not_ready","reason":"shutting_down"}
```

결과와 증거는 [검증 기록](../AGENTS.md#검증-기록)을 따른다. 아래 테스트는 스펙의 시나리오 기록을 연결한다. Producer 계약 테스트는 해당 시나리오 위에 이 계약의 필드·사건 이름 검사를 더한다. 작업 2.3~2.6의 테스트는 `crates/adapters/tests/ops_info_contract.rs`(아래 표에서 "운영 로그 계약 테스트". 상태 리스너 응답 테스트도 이 파일에 있다), `tests/serve.rs`(실행파일 시스템 테스트), `crates/adapters/src/ops_log.rs`·`crates/adapters/src/crypto.rs`·`crates/domain/src/peer_ip.rs`·`crates/domain/src/allow_list.rs`·`crates/app/src/lifecycle.rs` 단위 테스트에 있다. 실행 기록은 표 아래 [계약 테스트 실행 기록](#계약-테스트-실행-기록)에 있다.

| 주체·테스트 위치 | 확인할 계약 규칙 | 결과 | 실행 기록 |
| --- | --- | --- | --- |
| Producer / 운영 로그 계약 테스트 9건, `ops_log`(`cuts_origin_to_256_bytes_on_a_character_boundary` 포함)·`crypto`·`peer_ip` 단위 테스트 | 줄마다 JSON 객체 하나와 LF, 공통 필드 `ts`(UTC, 밀리초, `Z`)·`level`·`event`가 맨 앞, 값이 없는 선택 필드 생략(`null` 없음), 줄바꿈·따옴표·제어 문자 이스케이프, `detail` 512바이트와 `origin` 256바이트를 문자 경계에서 자름, 파일 이어 쓰기와 없는 파일 만들기, 동시 기록에서 줄이 섞이지 않음, 쓰기 실패 때 줄을 버리고 첫 실패만 알림(1.1~1.4절). 설정 경고 4종의 필드(1.5절). `peer_ip` 표기(1.10절). HMAC-SHA-256 참조 값 일치 | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Producer / 운영 로그 계약 테스트 `writes_connection_events_with_their_contract_fields`, `tests/serve.rs`의 TCP 진입점 테스트 4건(`checks_each_tcp_entry_point_against_its_own_allow_list`, `applies_item_boundaries_and_an_empty_list_at_accept`, `closes_and_logs_every_held_connection_when_stopping`, `checks_an_ipv4_peer_of_a_dual_stack_listener_against_ipv4_items`) | TCP 진입점의 `connection_accepted`(`conn_id`, `listener`, `role`, `peer_ip`, `info`), `connection_rejected`(`stage` = `accept`, `reason` = `ip_not_allowed`, `conn_id` 없음, `warn`), `connection_aborted`(`stage` = `initial_exchange`, `reason` = `server_shutdown`, `elapsed_ms`, `info`)의 필드와 순서(1.5·1.6절). 거절 줄에 상대가 보낸 데이터가 없음. 이중 스택 리스너의 IPv4 상대는 IPv4 표기(1.10절). 스펙 [RLY-023](../specs/relay.md#7-수용-테스트)(일부), [RLY-024](../specs/relay.md#7-수용-테스트), [OPS-037](../specs/operations.md#7-수용-테스트)(TCP 부분), [OPS-007](../specs/operations.md#7-수용-테스트)(초기 교환 단계 연결의 종료 부분) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Producer / `crates/adapters/tests/websocket.rs` `disallowed_source_on_wss_is_closed_before_tls_and_holds_no_relay_place` | WSS 진입점의 출발지 IP 거절 사건(`listener` = `wss_viewer`, `stage` = `accept`, `reason` = `ip_not_allowed`)을 쓰고 TLS 협상 전에 닫는다. 거절된 상대는 L 자리를 쓰지 않는다. 사건 값은 테스트가 받은 사건으로 확인했다. 줄 형식(필드 순서·`level`)은 이 행에서 확인하지 않았다. 스펙 [WS-023](../specs/websocket.md#7-수용-테스트)(WSS) | 부분 | [실행 기록](#계약-테스트-실행-기록) |
| Producer / 운영 로그 계약 테스트 `writes_limit_and_error_kind_with_the_reasons_that_carry_them`, `tests/serve.rs`의 `refuses_relay_connections_beyond_l_and_accepts_again_after_a_release`·`tells_a_peer_close_from_an_initial_exchange_that_outlives_h` | TCP 진입점의 `connection_rejected`(`stage` = `accept`, `reason` = `relay_connection_limit`, `limit`, `warn`). `connection_aborted`(`stage` = `initial_exchange`)의 `handshake_timeout`(`warn`), `peer_eof`(`info`), `connection_error`(`error_kind` = `io_error`, `warn`)와 필드 순서(1.5·1.6·1.7절). 자리를 돌려준 뒤 다시 수락함. 스펙 [OPS-004](../specs/operations.md#7-수용-테스트)(TCP 진입점 부분), [OPS-006](../specs/operations.md#7-수용-테스트)(H 부분), [RLY-005](../specs/relay.md#7-수용-테스트)(초기 교환 전 단계) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Producer / 운영 로그 계약 테스트 `writes_server_ready_with_the_name_and_address_of_every_listener`·`answers_liveness_and_readiness_in_each_state_with_only_the_contract_fields`·`ignores_credentials_and_queries_and_refuses_other_methods_and_paths`, `lifecycle` 단위 테스트 2건, `tests/serve.rs`의 `answers_status_queries_without_login_once_ready`·`refuses_status_connections_beyond_lm_and_keeps_lm_and_l_apart`·`writes_the_log_to_the_configured_file_from_start_to_stop` | `server_ready`(`info`, `listeners`의 `name`·`listen`)가 설정 경고 뒤에 옴(1.5절). `GET /livez`는 시작 중·준비됨·종료 중 모두 200 `{"status":"alive"}`, `GET /readyz`는 준비됨이면 200 `{"status":"ready"}`, 시작 중·종료 중이면 503 `not_ready`와 `starting`·`shutting_down`. 본문은 정한 필드만 가짐. `Content-Type: application/json`·`Cache-Control: no-store`. 쿼리·본문·`Authorization`·`Cookie` 무시. 정의한 경로의 `GET` 밖 메서드(`HEAD` 포함)는 405 `method_not_allowed`와 `Allow: GET`, 정의하지 않은 경로는 404 `not_found`(1.12·2절). Lm 도달 때 `connection_rejected`(`listener` = `status`, `stage` = `accept`, `reason` = `admin_connection_limit`, `limit`)를 기록하고 HTTP 응답 없이 닫음. Lm이 차도 중계 접속을 받고, 상태 연결은 L에 세지 않으며, L이 차도 상태 조회에 답함. 스펙 [OPS-013](../specs/operations.md#7-수용-테스트)(일부), [OPS-032](../specs/operations.md#7-수용-테스트), [OPS-033](../specs/operations.md#7-수용-테스트)(상태 연결 부분), [OPS-004](../specs/operations.md#7-수용-테스트)(상태 연결 부분) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Producer / 통합, 미생성 | 대기·매칭·연결 실패·세션 종료 사건의 필드, 접속 사건의 나머지 선택 필드(1.5절). 스펙 [OPS-011](../specs/operations.md#7-수용-테스트), [RLY-020](../specs/relay.md#7-수용-테스트) | 미실행 | 해당 사건 미구현 |
| Producer / 통합, 미생성 | 종료 원인 값 6종과 `level`(1.6절), `peer_eof`·`connection_error`·`admin_terminated`·`dead_connection`·`close_timeout`·`server_shutdown`. 스펙 [RLY-020](../specs/relay.md#7-수용-테스트), [RLY-026](../specs/relay.md#7-수용-테스트), [WS-016](../specs/websocket.md#7-수용-테스트), [WS-017](../specs/websocket.md#7-수용-테스트), [WS-026](../specs/websocket.md#7-수용-테스트), [OPS-009](../specs/operations.md#7-수용-테스트), [OPS-042](../specs/operations.md#7-수용-테스트), [MGT-023](../specs/management.md#7-수용-테스트) | 미실행 | 구현·테스트 파일 없음 |
| Producer / 통합, 미생성 | 관리자 종료 사건의 `admin_account`(1.8절). 스펙 [OPS-011](../specs/operations.md#7-수용-테스트), [OPS-029](../specs/operations.md#7-수용-테스트), [MGT-009](../specs/management.md#7-수용-테스트) | 미실행 | 구현·테스트 파일 없음 |
| Producer / 운영 로그 계약 테스트 `relay_id_ref_is_a_keyed_16_hex_digest_without_the_relay_id`, `tests/serve.rs`의 설정 오류·계정 파일 오류·바인드 실패·로그 파일 열기 실패 테스트 | 시작 실패 줄에 설정 값(IP 항목, 주소, 포트, 로그 파일 경로)과 해시·솔트가 없음. `relay_id_ref`는 키를 쓴 소문자 16진수 16자이고 키마다 다르며, 키의 `Debug`는 값을 가림(1.9·1.11절). 스펙 [OPS-012](../specs/operations.md#7-수용-테스트)의 시작 부분 | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Producer / 보안 통합, 일부 생성 | 세션·관리자·재로드 사건에서 로그에 넣지 않는 값(1.11절)과 실제 사건의 중계 ID 원문 비노출(1.9절). 스펙 [OPS-012](../specs/operations.md#7-수용-테스트), [OPS-030](../specs/operations.md#7-수용-테스트), [MGT-013](../specs/management.md#7-수용-테스트) | 미실행 | 로그인 성공 사건은 `tests/management_session.rs`의 `plain_loopback_login_opens_the_session_and_logs_success`가 확인한다(비밀번호·해시·쿠키 값·CSRF 값이 없다). 재로드 사건과 실제 사건의 중계 ID 원문 비노출은 시험이 없다. 로그인 실패·제한 사건의 값은 단위 시험(`login.rs`)만 있다. |
| Producer / `tests/serve.rs` `writes_the_log_to_the_configured_file_from_start_to_stop`, 운영 로그 계약 테스트 `writes_each_config_warning_with_its_contract_fields` | 시작 때 전체 허용 경고(`allow_all_source`, `phase` = `startup`)를 `server_starting` 다음에 기록. 경고 4종의 사건 이름·`level`·필드 | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Producer / `tests/serve.rs` `plaintext_ws_listener_is_off_by_default_and_warns_at_start_when_enabled` | 평문 WS 리스너는 기본 설정에서 열지 않고, 켜면 시작 경고를 기록한다. 스펙 [WS-024](../specs/websocket.md#7-수용-테스트)(전부), [OPS-016](../specs/operations.md#7-수용-테스트)(시작 부분) | 부분 | [실행 기록](#계약-테스트-실행-기록) |
| Producer / `tests/reload.rs` `allow_list_modes_hold_at_start_and_at_reload` | 시작 때 전체 허용 경고(`allow_all_source`, `phase` = `startup`)를 기록하고, 허용 목록 생략은 시작을 `config_missing_key`로 실패시킨다. 재로드 때 전체 허용 경고(`allow_all_source`, `phase` = `reload`)를 기록하고, 빈 허용 목록은 모든 출발지를 거절한다. 스펙 [OPS-038](../specs/operations.md#7-수용-테스트)(완전). 같은 시험의 [OPS-025](../specs/operations.md#7-수용-테스트) 조항은 부분이다. OPS-016(나머지 조항)은 이 시험이 확인하지 않는다 | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Producer / `tests/reload.rs` 10건, `crates/adapters/src/file_watch.rs`·`crates/app/src/reload/service.rs` 단위 시험 | 재로드 성공·실패·재시작 필요·감지 장애·주기 확인 감지 사건(1.5절). 스펙 [OPS-003](../specs/operations.md#7-수용-테스트)(완전), [OPS-024](../specs/operations.md#7-수용-테스트)(완전), [OPS-025](../specs/operations.md#7-수용-테스트)(부분), [OPS-026](../specs/operations.md#7-수용-테스트)(완전. 설정·TLS 파일 삭제와 쌍 불일치의 비부분 바이너리 시험), [OPS-027](../specs/operations.md#7-수용-테스트)(완전, 단위 시험), [OPS-028](../specs/operations.md#7-수용-테스트)(완전, 단위 시험), [OPS-035](../specs/operations.md#7-수용-테스트)(완전), [OPS-040](../specs/operations.md#7-수용-테스트)(부분. IPv6 출발지는 `::1`뿐이라 환경 한계가 남는다), [OPS-041](../specs/operations.md#7-수용-테스트)(완전, 단위 시험. 주기 확인(Dp)에서 감지(`poll`)되어 적용되는 것을 확인한다. D 상한은 Q+G < 1초로만 확인하며 읽기·검증·교체 시간은 주입하지 않는다). 감지 장애 사건(`reload_watch_failed`)의 자동 바이너리 시험은 없고 단위 시험만 있다. 복구 사건은 없다(file-watch §9 항목 5). | 부분 | [실행 기록](#계약-테스트-실행-기록) |
| Producer / `tests/serve.rs`: `rejects_invalid_config_with_exit_code_3_and_every_cause`, `rejects_toml_syntax_error_and_missing_config_file_with_exit_code_3`, `refuses_to_start_without_a_usable_account_with_exit_code_5`, `refuses_plaintext_management_on_a_public_address_with_exit_code_3`, `fails_before_ready_and_frees_every_port_when_a_port_is_taken_or_tls_fails`, `fails_with_exit_code_3_when_wss_key_does_not_match`, `fails_with_exit_code_3_when_management_certificate_is_missing`, `fails_with_exit_code_3_when_the_log_file_cannot_be_opened`, `rejects_reversed_mixed_and_bad_prefix_items_at_start_with_exit_code_3` | 시작 실패 원인 코드 `config_*`·`tls_file_invalid`·`log_file_unavailable`·`accounts_*`·`listener_bind_failed`를 진단 하나에 한 줄로 기록. 로그 출력을 열기 전에는 표준 출력에 씀. `config_key`·`listener`·`detail` 필드. `server_starting`(`log_contract`)이 처음, 종료 요청 뒤 `server_stopped`가 마지막(1.5·1.6절). 스펙 [OPS-001](../specs/operations.md#7-수용-테스트), [OPS-002](../specs/operations.md#7-수용-테스트)·[OPS-039](../specs/operations.md#7-수용-테스트)의 리스너 부분, [OPS-040](../specs/operations.md#7-수용-테스트)(역순·혼합·잘못된 표기 항목의 시작 실패 부분) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Producer / `src/serve.rs` 단위 시험 `watch_registration_refused_at_start_is_a_startup_failure` | 시작 때 감시 등록이 거절되면 `startup_failed`(`watch_unavailable`)를 기록하고 종료 코드 1로 끝낸다(1.5절). 스펙 [OPS-028](../specs/operations.md#7-수용-테스트)(부분, 단위 시험. 나머지 OPS-028 조항은 `crates/adapters/src/file_watch.rs`의 비부분 시험이 확인한다) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Producer / 통합, 미생성 | `shutdown_started`·`shutdown_forced`, `runtime_unavailable` 줄(코드는 있으나 실패를 일으킬 방법이 없어 테스트하지 않음). 스펙 [OPS-007](../specs/operations.md#7-수용-테스트), [OPS-010](../specs/operations.md#7-수용-테스트), [OPS-040](../specs/operations.md#7-수용-테스트) | 미실행 | 종료 유예 미구현(작업 5.3). |
| Producer / `crates/adapters/tests/management.rs`, `tests/serve.rs` | 관리 연결이 Lm을 넘을 때 접속 거절 사건 `connection_rejected`(`stage` = `accept`, `reason` = `admin_connection_limit`, `limit`, `listener` = `management`). 관리 연결은 L에 세지 않음. 스펙 [OPS-004](../specs/operations.md#7-수용-테스트)(관리 연결 부분), [OPS-033](../specs/operations.md#7-수용-테스트)(관리 부분). 테스트: `management.rs` `management_connection_beyond_lm_is_refused_at_accept`(사건 값), `tests/serve.rs` `refuses_management_connections_beyond_lm_and_keeps_lm_and_l_apart`(로그 줄, L 영향 없음) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Producer / 통합, 미생성 | `ip_not_allowed`·`relay_connection_limit`·`admin_connection_limit`(상태 리스너) 밖의 접속 거절 `stage`·`reason`(1.6절), WS 평문 진입점의 출발지 IP 거절(WSS는 위 행), 관리 TLS 버전 거절의 로그 줄 형식(`stage` = `tls`, `reason` = `tls_version_unsupported`, `listener` = `management`; 사건 값만 `management.rs` `tls10_and_tls11_hellos_are_refused_without_plaintext_fallback`에서 확인. 단, ClientHello가 첫 TLS 레코드를 넘으면 이 사건을 쓰지 않는다. 관리 리스너 모듈 설계 9절 열린 질문 10). 스펙 [OPS-005](../specs/operations.md#7-수용-테스트), [OPS-023](../specs/operations.md#7-수용-테스트), [OPS-036](../specs/operations.md#7-수용-테스트), [OPS-037](../specs/operations.md#7-수용-테스트)(WS·WSS 부분), [RLY-004](../specs/relay.md#7-수용-테스트), [RLY-022](../specs/relay.md#7-수용-테스트), [WS-012](../specs/websocket.md#7-수용-테스트), [WS-015](../specs/websocket.md#7-수용-테스트), [WS-022](../specs/websocket.md#7-수용-테스트), [WS-027](../specs/websocket.md#7-수용-테스트) | 미실행 | 구현·테스트 파일 없음 |
| Producer / `crates/adapters/tests/websocket.rs` `expired_untrusted_and_mismatched_certificates_fail_the_client_on_wss`(WSS 협상 실패 사건), `tls10_and_tls11_are_refused_and_tls12_and_tls13_succeed_on_wss_and_management`(WSS 버전 거절 사건) | WSS 진입점의 TLS 협상 실패 `connection_aborted`(`stage` = `tls`, `reason` = `connection_error`, `error_kind` = `tls_failed`)와 TLS 1.2 미만 거절 `connection_rejected`(`stage` = `tls`, `reason` = `tls_version_unsupported`)의 사건 값을 테스트가 받은 사건으로 확인했다. 줄 형식(필드 순서·`level`)은 확인하지 않았다. 스펙 [WS-013](../specs/websocket.md#7-수용-테스트), [OPS-036](../specs/operations.md#7-수용-테스트)(WSS 사건 값 부분) | 부분 | [실행 기록](#계약-테스트-실행-기록) |
| Producer / 통합, 미생성 | 대상 연결 실패 3종과 정책 거절 구분. 스펙 [RLY-007](../specs/relay.md#7-수용-테스트), [RLY-008](../specs/relay.md#7-수용-테스트), [RLY-009](../specs/relay.md#7-수용-테스트) | 미실행 | 구현·테스트 파일 없음 |
| Producer / 통합, 일부 생성 | 관리자 로그인 성공·실패·제한 사건. 스펙 [MGT-017](../specs/management.md#7-수용-테스트), [MGT-018](../specs/management.md#7-수용-테스트), [MGT-022](../specs/management.md#7-수용-테스트) | 미실행 | 구현: `ops_log.rs`(세 사건의 줄 형식), `login.rs`(사건 값 단위 시험). 시험: 로그인 성공 줄은 `tests/management_session.rs`의 `plain_loopback_login_opens_the_session_and_logs_success`가 확인한다. 실패·제한·제한 시작 줄 형식은 `ops_log.rs`의 `admin_login_lines_carry_only_their_own_fields`가 확인한다. |
| Producer / 통합, 미생성 | 실행 중인 프로세스에서 시작 중·종료 유예 중 `/readyz` 503(유예 중 `/livez`는 200), `listener_failed`. 스펙 [OPS-013](../specs/operations.md#7-수용-테스트)(나머지), [OPS-043](../specs/operations.md#7-수용-테스트) | 미실행 | 종료 유예·리스너 실패 판정 미구현 |
| Consumer / 수집 도구 계약 테스트, 위치 미정 | 알 수 없는 필드·사건·값 무시(3절), `level`·`event` 분기 | 미실행 | 소비자 테스트 위치 없음 |
| Consumer / 상태 도구 계약 테스트, 위치 미정 | HTTP 상태 코드로 판정, 알 수 없는 `reason` 무시(3절) | 미실행 | 소비자 테스트 위치 없음 |

### 계약 테스트 실행 기록

| 대상 | 환경 | 명령 | 결과 |
| --- | --- | --- | --- |
| 커밋 `f25983a`에 작업 2.3을 더한 작업 트리(`crates/*`의 운영 로그·암호·상대 IP 모듈, `crates/adapters/src/net.rs`, `src/serve.rs`, `tests/serve.rs`, `Cargo.toml`, `Cargo.lock`, `xtask/src/layers.rs`, 문서) | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행) | 통과. 운영 로그 계약 테스트 9건(Linux 전용 `/dev/full` 1건 포함), `tests/serve.rs` 10건, adapters 단위 5건, domain 단위 18건(`peer_ip` 3건 포함), 설정·계정 파일 계약 테스트 28건 모두 통과. 시나리오 보고: 통과 1(OPS-001), 부분 5(OPS-002, OPS-012, OPS-038, OPS-039, OPS-040), 미연결 142, 오류 0, 판정 통과 |
| 같음 | 같음 | `tests/serve.rs` 테스트 실행파일을 60번 반복 | 통과(60번 모두 통과) |
| 같음 | 같음 | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask layers`, `cargo deny check licenses` | 통과 |
| 같음 | 같음(교차 대상 `x86_64-pc-windows-msvc`) | `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings` | 실패. aws-lc-sys 빌드 스크립트가 끝나지 않았다(`NASM command not found! Build cannot continue.`). `AWS_LC_SYS_PREBUILT_NASM=1`을 주어도 Linux의 gcc가 Windows용 C 파일을 컴파일하지 못했다. 영향: Windows 분기는 타입 검사도 하지 못했다([개발 환경 how-to](../how-to/set-up-and-verify.md)). |
| 같음 | Windows 11 | `cargo xtask scenarios` | 미실행. Visual Studio Build Tools 설치가 UAC 미승인으로 멈춰(종료 코드 1602) MSVC가 없다. 사용자가 Windows 검증을 뒤로 미뤘다(2026-10-20). |
| 커밋 `6d6f789`에 작업 2.4를 더한 작업 트리(`crates/domain`의 허용 판정·`conn_id`·`Role`, `crates/app`의 접속 사건·`ListenerPlan::entry_point`, `crates/adapters/src/net.rs`·`ops_log.rs`, `src/serve.rs`, `tests/serve.rs`, `crates/adapters/tests/ops_info_contract.rs`, `Cargo.toml`, `Cargo.lock`, 문서) | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행) | 통과. 운영 로그 계약 테스트 10건, `tests/serve.rs` 14건(Unix 전용 1건, Linux 전용 2건 포함), domain 단위 23건(`allow_list` 8건, `conn_id` 1건 포함), 설정·계정 파일 계약 테스트 28건 모두 통과. 시나리오 보고: 통과 2(OPS-001, RLY-024), 부분 8, 미연결 138, 오류 0, 판정 통과 |
| 같음 | 같음 | `tests/serve.rs` 테스트 실행파일을 40번 반복 | 통과(40번 모두 통과) |
| 같음 | 같음 | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask layers`, `cargo deny check licenses` | 통과 |
| 같음 | Windows 11 | `cargo xtask scenarios` | 미실행. 작업 2.3 기록과 같은 이유다(MSVC 없음, 사용자가 Windows 검증을 뒤로 미뤘다). |
| 커밋 `c89beb4`에 작업 2.5를 더한 작업 트리(`crates/domain/src/budget.rs`, `crates/app/src/events.rs`, `crates/adapters/src/net.rs`·`ops_log.rs`, `src/serve.rs`, `tests/serve.rs`, `crates/adapters/tests/ops_info_contract.rs`, `Cargo.toml`, `Cargo.lock`, `crates/adapters/Cargo.toml`, `xtask/src/layers.rs`, 문서) | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행) | 통과. 운영 로그 계약 테스트 11건, `tests/serve.rs` 16건, adapters 단위 6건(Linux 전용 keepalive 1건 포함), domain 단위 26건(`budget` 3건 포함), 설정·계정 파일 계약 테스트 28건 모두 통과. 시나리오 보고: 통과 2, 부분 12, 미연결 134, 오류 0, 판정 통과 |
| 같음 | 같음 | `tests/serve.rs` 테스트 실행파일을 40번 반복 | 통과(40번 모두 통과) |
| 같음 | 같음 | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask layers`, `cargo deny check licenses` | 통과 |
| 같음 | Windows 11 | `cargo xtask scenarios` | 미실행. 작업 2.3 기록과 같은 이유다(MSVC 없음, 사용자가 Windows 검증을 뒤로 미뤘다). |
| 커밋 `b1f2ad5`에 작업 2.6을 더한 작업 트리(`crates/app/src/lifecycle.rs`·`events.rs`·`settings.rs`·`lib.rs`, `crates/adapters/src/status_http.rs`·`net.rs`·`ops_log.rs`·`lib.rs`, `src/serve.rs`, `tests/serve.rs`, `crates/adapters/tests/ops_info_contract.rs`, `Cargo.toml`, `Cargo.lock`, `crates/adapters/Cargo.toml`, 문서) | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행) | 통과. 운영 로그 계약 테스트 14건, `tests/serve.rs` 18건, adapters 단위 6건, app 단위 2건(`lifecycle`), domain 단위 26건, 설정·계정 파일 계약 테스트 28건 모두 통과. 시나리오 보고: 통과 3, 부분 13, 미연결 132, 오류 0, 판정 통과 |
| 같음 | 같음 | `tests/serve.rs` 테스트 실행파일을 80번 반복 | 통과(80번 모두 통과). 테스트 도구를 고치기 전 두 번의 40번 반복에서 `checks_an_ipv4_peer_of_a_dual_stack_listener_against_ipv4_items`가 3번과 1번 실패했다. 실패 진단은 `[::]:<포트>` 바인드 실패(`listener_bind_failed`, 이미 사용 중)였다. 포트를 `127.0.0.1:0`으로 골라서, 앞선 테스트가 다른 127.0.0.x 주소에 남긴 클라이언트 소켓과 포트가 겹친 것으로 판단했다. 그 소켓 목록은 확인하지 않았다. 이 테스트는 이제 `[::]:0`으로 포트를 고르고, 그 뒤 실패하지 않았다. |
| 같음 | 같음 | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask layers`, `cargo deny check licenses` | 통과 |
| 같음 | Windows 11 | `cargo xtask scenarios` | 미실행. 작업 2.3 기록과 같은 이유다(MSVC 없음, 사용자가 Windows 검증을 뒤로 미뤘다). |
| 커밋 `e1cfc4f`에 Windows 경고 수정(`crates/adapters/src/net.rs`의 테스트 모듈을 Linux 전용으로, `tests/serve.rs`의 `lifecycle_events`를 Unix 전용으로)을 더한 작업 트리 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 17.14.37806.7, `AWS_LC_SYS_PREBUILT_NASM=1` | `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행) | 통과. 운영 로그 계약 테스트 13건, `tests/serve.rs` 12건, adapters 단위 5건, app 단위 2건, domain 단위 26건, 설정·계정 파일 계약 테스트 27건, xtask 단위 12건 모두 통과. Unix·Linux 전용 테스트(`/dev/full` 쓰기 실패, Unix 신호, 이중 스택, 계정 파일 권한, keepalive 값 읽기)는 `cfg`로 빠졌다. 시나리오 보고: 통과 3, 부분 12, 미실행 1(OPS-007: Unix 전용 `closes_and_logs_every_held_connection_when_stopping`), 미연결 132, 오류 0, 판정 통과 |
| 같음 | 같음 | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask layers`, `cargo deny check licenses`(cargo-deny 0.20.2) | 통과. 수정 전 clippy는 실패했다. `net.rs` 테스트 모듈의 `unused import: super::*`와 `tests/serve.rs`의 `function lifecycle_events is never used`였다. |
| 같음 | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `cargo xtask scenarios`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask layers`, `cargo deny check licenses` | 통과. 시나리오 보고: 통과 3, 부분 13, 미연결 132, 오류 0, 판정 통과 |
| 커밋 `b7b43cb`에 작업 3.1을 더한 작업 트리(`crates/adapters/src/tls.rs`, `crates/adapters/tests/tls.rs`, `src/serve.rs`, `tests/serve.rs`, `Cargo.toml`, `Cargo.lock`, `crates/adapters/Cargo.toml`, 문서) | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행) | 통과. 운영 정보 계약 테스트 14건, `tests/serve.rs` 21건(`checks_tls_files_before_binding_a_taken_status_port` 포함), adapters 단위 7건, app 단위 9건, domain 단위 27건, `config_contract` 21건, `accounts_contract` 12건, xtask 단위 12건 모두 통과. 시나리오 보고: 통과 4, 부분 16, 미연결 128, 오류 0, 판정 통과 |
| 같은 작업 트리 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 17.14.37806.7, `AWS_LC_SYS_PREBUILT_NASM=1` | `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행) | 통과. 운영 정보 계약 테스트 13건, `tests/serve.rs` 15건(Unix 전용 테스트 제외), adapters 단위 6건, app 단위 9건, domain 단위 27건, `config_contract` 21건, `accounts_contract` 11건, xtask 단위 12건 모두 통과. 시나리오 보고: 통과 3, 부분 16, 미실행 1(OPS-007: Unix 전용 `closes_and_logs_every_held_connection_when_stopping`), 미연결 128, 오류 0, 판정 통과 |
| 커밋 `3fa396a`(Q9 서버 수준 테스트와 문서) | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행) | 통과. 테스트 전체 모두 통과. 시나리오 보고: 통과 4, 부분 19, 미연결 125, 오류 0, 판정 통과 |
| 같은 커밋 `3fa396a` | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 17.14.37806.7, `AWS_LC_SYS_PREBUILT_NASM=1` | `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행) | 통과. 테스트 전체 모두 통과(Unix 전용 테스트 제외). 시나리오 보고: 통과 3, 부분 19, 미실행 1(OPS-007: Unix 전용 `closes_and_logs_every_held_connection_when_stopping`), 미연결 125, 오류 0, 판정 통과 |
| 커밋 `c1c24d2`(작업 3.2 업그레이드 단계, 검토 결함 5건 수정 뒤). 작업 트리에 `docs/contracts/config-v1.md`의 커밋 안 된 변경 1개가 있었으나 이 작업과 무관하다 | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh`(빌드, `fmt`, `clippy`, `layers`, `cargo xtask scenarios`, `licenses`, `deny`) | 통과(`ALL PASSED`). 테스트 223건 모두 통과, 실패 0. `tests/websocket.rs` 14건 통과. 시나리오 보고: 통과 9, 부분 22, 미실행 0, 미연결 117, 오류 0, 판정 통과 |
| 같은 커밋 `c1c24d2`, 같은 작업 트리 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 17.14.37806.7, `AWS_LC_SYS_PREBUILT_NASM=1` | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 테스트 207건 모두 통과, 실패 0. `tests/websocket.rs` 14건 통과. 시나리오 보고: 통과 8, 부분 22, 미실행 1(OPS-007: Unix 전용 `closes_and_logs_every_held_connection_when_stopping`), 미연결 117, 오류 0, 판정 통과 |
| 커밋 `e701997`(작업 3.1 검증. 코드 커밋 `89a1001`·`e701997`, 작업 트리 깨끗함) | Ubuntu 24.04.4(WSL2), Rust 1.99.1 | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 새 WSS 사건 값 시험 2건 포함 전체 테스트 통과. 시나리오 보고: 통과 11, 부분 20, 실패 0, 미실행 0, 미연결 117, 오류 0, 판정 통과 |
| 같은 커밋 `e701997`, 작업 트리 깨끗함 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), 이전 Windows 기록과 같은 환경 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 보고: 통과 10, 부분 20, 실패 0, 미실행 1(OPS-007: Unix 전용 `closes_and_logs_every_held_connection_when_stopping`), 미연결 117, 오류 0, 판정 통과 |
| 커밋 `59f3f62`(작업 4.3 컴파일 오류 수정) + 문서 변경 작업 트리(관리 로그인과 세션 설계·대조 문서) | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 전체 테스트 통과. 시나리오 보고: 통과 11, 부분 27, 실패 0, 미실행 0, 미연결 110, 오류 0, 판정 통과 |
| 같은 커밋 `59f3f62`, 같은 작업 트리 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 17.14.37806.7, `AWS_LC_SYS_PREBUILT_NASM=1` | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 전체 테스트 통과(Unix 전용 테스트 제외). 시나리오 보고: 통과 10, 부분 27, 실패 0, 미실행 1(OPS-007: Unix 전용 `closes_and_logs_every_held_connection_when_stopping`), 미연결 110, 오류 0, 판정 통과 |
| 커밋 `a143539`(작업 4.3 설계 수정 구현, 검증 뒤 코드 변경 없음) | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 전체 테스트 통과. 시나리오 보고: 통과 11, 부분 27, 실패 0, 미실행 0, 미연결 110, 오류 0, 판정 통과 |
| 같은 커밋 `a143539`, 검증 뒤 코드 변경 없음 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 17.14.37806.7, `AWS_LC_SYS_PREBUILT_NASM=1` | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 전체 테스트 통과(Unix 전용 테스트 제외). 시나리오 보고: 통과 10, 부분 27, 실패 0, 미실행 1(OPS-007: Unix 전용 `closes_and_logs_every_held_connection_when_stopping`), 미연결 110, 오류 0, 판정 통과 |
| 커밋 `60f28b7` 뒤 작업 트리(`Cargo.toml`·`Cargo.lock`의 `rustls` 개발 의존성, `xtask/src/layers.rs`, `tests/serve.rs`의 OPS-014 시험과 HTTPS 헬퍼, 문서) | Ubuntu 24.04.4(WSL2), Rust 1.99.1 · Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 17.14.37806.7, `AWS_LC_SYS_PREBUILT_NASM=1` | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` · `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | Ubuntu `ALL PASSED`(시나리오 148건: 통과 16, 부분 22, 실패 0, 미실행 0, 미연결 110). Windows `ALL PASSED (all)`(시나리오 148건: 통과 15, 부분 22, 실패 0, 미실행 1, 미연결 110. 미실행 1건은 OPS-007). OPS-014 통과(`tests/serve.rs` `public_management_never_starts_on_plaintext_or_without_tls_or_an_account`)가 두 OS에서 같다. |
| 커밋 `80eac0e` 뒤 작업 트리(작업 5.2 자동 재로드 변경: `crates/adapters/src/file_watch.rs`, `crates/adapters/src/file_watch/feed.rs`, `crates/app/src/reload/service.rs`, 새 파일 `tests/reload.rs`, 문서) | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 시나리오 보고: 통과 18, 부분 35, 실패 0, 미실행 0, 미연결 97, 오류 0, 판정 통과 |
| 같은 작업 트리 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 17.14.37806.7, `AWS_LC_SYS_PREBUILT_NASM=1` | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 보고: 통과 17, 부분 35, 실패 0, 미실행 1(OPS-007: Unix 전용 `closes_and_logs_every_held_connection_when_stopping`), 미연결 97, 오류 0, 판정 통과 |
| 커밋 `c4c6cf6`, 코드는 커밋과 같음 | WSL2 Ubuntu 24.04.4, Rust 1.99.1 | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 시나리오 보고: 통과 23, 부분 30, 실패 0, 미실행 0, 미연결 97, 오류 0, 판정 통과 |
| 커밋 `c4c6cf6`, 코드는 커밋과 같음 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 보고: 통과 22, 부분 30, 실패 0, 미실행 1(OPS-007: Unix 전용), 미연결 97, 오류 0, 판정 통과 |
| 커밋 `834769d`(작업 5.2 시나리오 표식 정정. 코드 동작은 `c4c6cf6`과 같음) | WSL2 Ubuntu 24.04.4, Rust 1.99.1 | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 시나리오 보고: 통과 22, 부분 31, 실패 0, 미실행 0, 미연결 97, 오류 0, 판정 통과 |
| 같은 커밋 `834769d` | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 보고: 통과 21, 부분 31, 실패 0, 미실행 1(OPS-007: Unix 전용 `closes_and_logs_every_held_connection_when_stopping`), 미연결 97, 오류 0, 판정 통과 |
| 커밋 `6c7a551`·`72db643`(작업 5.2 시험 정리, OPS-041 `run` 기반 시험, 생성 파일 헤더 영문화) | WSL2 Ubuntu 24.04.4, Rust 1.99.1 | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 시나리오 보고: 통과 23, 부분 30, 실패 0, 미실행 0, 미연결 97, 오류 0, 판정 통과 |
| 같은 커밋 `6c7a551`·`72db643` | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 보고: 통과 22, 부분 30, 실패 0, 미실행 1(OPS-007: Unix 전용 `closes_and_logs_every_held_connection_when_stopping`), 미연결 97, 오류 0, 판정 통과 |
| 커밋 `307cfd8`(작업 5.2 최종 게이트. 코드는 `3eea38f`와 같고 뒤에 문서 커밋만 더해졌다). 게이트 실행 때 추적 파일 변경 없음 | Ubuntu 24.04.4(WSL2), Rust 1.99.1 | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). 시나리오 보고: 통과 24, 부분 29, 실패 0, 미실행 0, 미연결 97, 오류 0, 판정 통과. OPS-026 완전, OPS-040 부분(IPv6 출발지 환경 한계). 대조: [file-watch 작업 5.2 게이트 실행 기록](../architectures/file-watch.md#작업-52-게이트-실행-기록) |
| 같은 커밋 `307cfd8` | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 시나리오 보고: 통과 23, 부분 29, 실패 0, 미실행 1(OPS-007: Unix 전용 `closes_and_logs_every_held_connection_when_stopping`), 미연결 97, 오류 0, 판정 통과 |

## 6. 초안을 확정하기 전에 해결할 사항

- 이 계약의 이름·코드 선택은 2026-10-16에 사용자가 채택했다. 사건 이름, 필드 이름, `level` 기준, 원인 코드, `relay_id_ref` 방식, `peer_ip` 표기, 상태 응답 본문이 해당한다. 앞서 사용자가 확정한 항목은 1절 첫 목록에 있다.
- 이 계약의 채택으로 운영 스펙의 OPS-TBD-006(로그 필드·생존·준비 응답 형식)을 해소했다. 기준은 이 계약의 1절이다.
- 관리 API 계약과 이름을 맞췄다. `session_id`, `viewer_transport` 값(`tcp`·`ws`·`wss`), `mode` 값(`mode1`·`mode2`), `close_cause` 값 6종, 방향별 바이트 필드 이름이 해당한다. 설정 계약이 `[log] output`·`[status] listen`, 상태 리스너의 비루프백 바인딩(경고 `status_listen_not_loopback`), 전체 허용 항목의 판정 범위(1.3절)를 정했다. 두 계약 모두 채택했으며 이름이 계속 같아야 한다.
- 로그인 성공·실패·제한을 로그에 남기라는 스펙 문장과 시나리오가 없으며, 사용자가 스펙에 추가하지 않기로 했다(2026-10-16). 이 계약의 로그인 사건 3종 정의는 유지한다.
- `waiting_expired`(대기 시간 W 만료)를 검증하는 스펙 시나리오가 없으며, 사용자가 추가하지 않기로 했다(2026-10-16). 이 계약의 사건 정의는 유지한다.
- `reload_watch_failed`가 해소된 뒤 복구를 알리는 사건은 스펙에 없어 정하지 않았다. 운영자가 복구를 알아야 하면 사건을 추가한다(호환 변경).
- 이 계약은 로그가 폭증할 때의 제한(예: Lm·L 도달 거절 기록의 속도 제한)을 정하지 않는다. 스펙에 요구가 없다.
- 로그 출력이 실패했을 때의 동작은 스펙에 없어 사용자가 정했다(2026-10-20). 로그 파일을 열 수 없으면 시작 실패 `log_file_unavailable`이다. 실행 중 쓰기 실패는 1.1절 "출력 실패" 행을 따른다.
- 설정 파일 오류로 로그 출력 위치를 알 수 없을 때 표준 출력에 쓰는 규칙(1.1절)은 채택 범위다.
- 상태 리스너를 다른 필수 리스너보다 먼저 여는 순서(1.12절)는 채택 범위다.
- 중계 ID 키 있는 해시(1.9절)는 채택 범위다. 구현에서 쓸 수 있는 라이브러리는 이 계약이 정하지 않는다.
- `subprotocol_not_supported`는 WebSocket 계약의 서브프로토콜 규칙에 의존한다. 스펙에 근거 시나리오가 없다.
- `server_shutdown`(정상 종료 유예 만료나 유예 중 두 번째 종료 요청으로 강제 정리)은 [관리 스펙 2.3절](../specs/management.md#23-최근-종료-목록)의 종료 원인 목록에 포함된다. 최근 종료 목록은 프로세스 안에만 있으므로 이 값이 목록에서 실제로 조회되는지는 구현에서 확인해야 한다.
