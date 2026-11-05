# Repeater 초기 교환 v1 계약

UltraVNC Viewer·Server와 외부 WebSocket 클라이언트가 VNC Relay에 접속한 직후 주고받는 초기 교환의 형식과 호환 범위를 정한다. 초기 교환은 중계 방식(Mode I·II)과 대상 주소 또는 중계 ID를 정하는 절차다([용어집](../../GLOSSARY.md)).

이 계약은 2026-11-01에 프로젝트 소유자가 채택했다(상태 Active, 사용자 결정 2026-11-01(Q73)). 1절의 형식은 대상 버전의 허가된 관찰([호환성 검증 기록](repeater-compat-record.md) 3절)에서 확정했다. 관찰로 정할 수 없던 선택은 사용자 결정 2026-11-01(Q68~Q72)로 정했다. 같은 역할 중복 ID의 정책(RLY-TBD-003)은 채택 때 제안으로 남겼다가 같은 날 사용자 결정 2026-11-01(Q74·Q75)로 정해 2절에 반영했다.

| 항목 | 값 |
| --- | --- |
| 이름과 버전 | `repeater-handshake` 1.0.0. 2절 `duplicate_id`의 정책 확정(사용자 결정 2026-11-01(Q74))은 버전을 올리지 않았다. 채택 때 제안으로 둔 후발 거절을 그대로 정했으므로 처리가 바뀌는 요청이 없다. 1.2절의 빈 값·모드 검사 순서 명확화도 버전을 유지한다. 기존 “값을 더 검사하지 않고”와 Server의 `ID:` 필수 규칙을 풀어 쓴 것이며, 형식이나 허용 범위를 바꾸지 않는다(사용자 결정 2026-11-01(Q80)). |
| 상태 | Active(2026-11-01 채택) |
| 소유자 | vnc-relay 프로젝트 관리자 |
| 만드는 쪽(Producer) | 초기 요청: UltraVNC Viewer, UltraVNC Server, 외부 WebSocket 클라이언트(검증 대상은 noVNC 1.7.0의 `repeaterID` 옵션). 시작 바이트: `vnc-relay` |
| 사용하는 쪽(Consumer) | 초기 요청: `vnc-relay` (TCP Viewer·TCP Server·WS Viewer·WSS Viewer 진입점). 시작 바이트: Viewer와 외부 WebSocket 클라이언트 |
| 방향 | 끝점이 `vnc-relay`에 250바이트 초기 요청 하나를 보낸다. Viewer 쪽 연결에는 `vnc-relay`가 먼저 12바이트 시작 바이트를 보낸다. 오류를 알리는 바이트는 없다. |
| 스키마 기준 파일 | 없음. 1절의 표가 형식의 기준이다. |
| 기준 스펙 | [TCP 중계와 연결 매칭](../specs/relay.md), [WS·WSS 전송](../specs/websocket.md) |
| 관련 요구사항 | [PRD](../requirements/PRD.md) FR-01·FR-02·FR-03·FR-05, 9절·9.1절 |
| 호환성 범위 | 관찰 대상 버전: UltraVNC Server·Viewer 1.8.3.0, UltraVNC Repeater(Windows) 1.6.0.0, noVNC 1.7.0([호환성 검증 기록](repeater-compat-record.md) 2절). 이 버전과의 호환 지원은 `vnc-relay` 구현 뒤 5절의 계약 테스트와 같은 시나리오의 비교를 통과한 뒤에 주장한다. 대상 버전 Repeater와 일부러 다르게 처리하는 입력은 [3.1절](#31-대상-버전-repeater와-다르게-처리하는-입력)에 있다. |
| 관련 테스트 | [RLY-001~005, RLY-010~015, RLY-022](../specs/relay.md#7-수용-테스트), [WS-003~006, WS-018, WS-025](../specs/websocket.md#7-수용-테스트). 계약 테스트는 5절을 따른다. |

관련 계약: [WebSocket 중계](websocket-relay-v1.md)(WS·WSS 쪽 전송 규칙과 종료 코드), [운영 정보](ops-info-v1.md)(로그에서 중계 ID 노출 방식과 오류 코드의 로그 이름), [설정](config-v2.md)(`handshake_secs`·`mode1`·`mode2`·`allow_targets` 키). 관련 결정: [ADR-0002 출발지 IP 허용 목록](../decisions/ADR-0002-source-ip-allowlist.md).

## 1. 형식과 의미

### 1.1 메시지

이 절의 근거는 [호환성 검증 기록 3절](repeater-compat-record.md#3-관찰-증거-todo-14)이다. 근거 열의 ID는 기록 [3.4절](repeater-compat-record.md#34-시나리오별-입력과-결과)의 시나리오다. 메시지 이름(A~F)은 기록 [3.5절](repeater-compat-record.md#35-계약-1절-항목별-해석)과 같다.

| 메시지 | 방향과 시점 | 형식 | 근거 |
| --- | --- | --- | --- |
| A. Mode II Server 등록 요청 | Server → TCP Server 진입점. Server가 연결 직후 먼저 보낸다. `vnc-relay`는 매칭 전까지 Server에 아무 바이트도 보내지 않는다. | 초기 요청 250바이트(1.2절). 값은 `ID:<중계 ID>`다. | M2-01, M2-02, T-03 |
| B. Mode II Viewer 요청 | Viewer → Viewer 진입점(TCP Viewer·WS·WSS). Viewer는 D의 시작 바이트를 받은 뒤 보낸다. | 초기 요청 250바이트. 값은 `ID:<중계 ID>`다. A와 형식이 같다. | M2-01, M2-02, M2-05 |
| C. Mode I Viewer 요청 | B와 같다. | 초기 요청 250바이트. 값은 `<호스트>` 또는 `<호스트>:<포트>`다(1.3절). `ID:` 같은 접두어는 없다. | M1-01~M1-03 |
| D. `vnc-relay`가 보내는 바이트 | `vnc-relay` → Viewer. 초기 교환에 들어가면 곧바로 한 번 보낸다. TCP는 출발지 IP 검사와 L 검사를 통과한 직후, WS·WSS는 `101` 응답 직후다. Server 쪽에는 보내지 않는다. | 시작 바이트 12바이트 `RFB 000.000\n`(`52 46 42 20 30 30 30 2E 30 30 30 0A`). WS·WSS는 바이너리 메시지 하나에 담는다. 입력과 관계없이 같으므로 결과를 알리지 않는다. 이 밖에 `vnc-relay`가 만들어 보내는 초기 교환 바이트는 없다. 오류는 2절의 연결 종료로만 알린다. | M2-01, E-01, E-04, W-02, W-03b |
| E. 초기 교환 뒤 스트림 | 양방향 | 초기 요청 250바이트 바로 다음 바이트부터 중계 스트림이다. 초기 요청은 상대에게 전달하지 않는다. 250바이트 뒤에 이미 받은 바이트와 매칭 전에 도착한 바이트는 도착 순서대로, 매칭이나 대상 연결 직후 상대에게 첫 바이트로 전달한다. Viewer가 처음 받는 RFB 바이트는 Server(Mode I은 대상)가 보낸 ProtocolVersion(`RFB 003.008\n` 등)이다. | M1-01, M2-01, S-02~S-05, E-03 |
| F. 외부 WebSocket 클라이언트(noVNC) 요청 | B와 같다. noVNC 1.7.0은 D를 받은 뒤 67~110ms 안에 보냈다. | noVNC 1.7.0은 `repeaterID`를 B와 같은 250바이트(`ID:<repeaterID>` + NUL 채움)로 바이너리 메시지 하나에 담아 보낸다. WS·WSS 메시지 경계는 초기 요청의 경계가 아니다(1.4절). | W-02, W-03b |

공개 문서의 사용법은 다음과 같다. Server는 `-id:<ID>` 또는 `-autoreconnect ID:<ID>`와 Repeater를 가리키는 `-connect`로 접속한다([Server 명령줄](https://uvnc.com/docs/ultravnc-server/51-ultravnc-server-commandline-parameters.html)). Viewer는 Mode II에서 `-proxy <Repeater>:<포트> ID:<ID>`, Mode I에서 `-proxy <Repeater>:<포트> -connect <대상>`으로 접속한다([Viewer 명령줄](https://uvnc.com/docs/ultravnc-viewer/52-ultravnc-viewer-commandline-parameters.html)). noVNC는 `RFB()`의 `repeaterID` 옵션을 쓴다([noVNC API](https://novnc.com/noVNC/docs/API.html)). 중계 ID만으로는 보안이 없다고 공식 문서도 설명한다([Connections](https://uvnc.com/docs/documentation/132-ultravnc-connections.html)).

포트 번호는 이 계약의 형식 항목이 아니다. 공개 문서 사이에는 Repeater 기본 포트를 다르게 적은 곳이 있다([Repeater 안내 PDF](https://www.uvnc.eu/download/1210/Repeater.pdf), [Single Click Repeater](https://uvnc.com/pchelpware/sc/repeater.html)). `vnc-relay`의 기본 포트는 [설정 계약 1.2절](config-v2.md#12-키-표)이 정한다.

### 1.2 초기 요청 250바이트

| 규칙 | 내용 | 근거 |
| --- | --- | --- |
| 길이 | 정확히 250바이트다. TCP 청크와 WebSocket 메시지의 경계와 관계없이 250바이트를 모은 뒤 해석한다. 250바이트가 다 오기 전에는 해석·대기 등록·대상 연결을 하지 않는다. 250바이트를 넘게 받은 바이트는 1.1절 E의 후속 바이트다. | S-01, E-02, E-03, E-04 |
| 값과 채움 | 값은 첫 NUL(`00`) 앞까지다. NUL이 없으면 250바이트 전체가 값이다. 첫 NUL 뒤의 채움 바이트는 내용과 관계없이 무시한다. 본문 문법 검사에서 빈 값은 `handshake_invalid`다. 모드 검사와의 선후는 표 아래 검사 순서를 따른다. | M1-01, M2-01, M2-06b, T-03 |
| 모드 판정 | Viewer 진입점(TCP Viewer·WS·WSS): 값이 `ID:`(대문자, `49 44 3A`)로 시작하면 Mode II, 아니면 Mode I이다. TCP Server 진입점: 값이 `ID:`로 시작해야 한다. 아니면 `handshake_invalid`다. | M1-01, M2-01, E-01 |
| 꺼진 모드 | 판정한 모드가 설정에서 꺼져 있으면 값을 더 검사하지 않고 `mode_disabled`로 거절한다. | [RLY-R17](../specs/relay.md#21-판정-가능한-규칙) |
| 역할 | 진입점이 정한다. TCP Server 진입점은 Server, Viewer 진입점은 Viewer다. 요청 안에는 역할 표시가 없다. | M2-01 |
| 중계 ID | `ID:` 뒤의 값이다. ASCII 숫자(`0`~`9`) 1~247자리이고, 247자리는 250바이트에 들어가는 최대 길이다. 숫자가 아닌 바이트가 하나라도 있거나 숫자가 없으면 `handshake_invalid`다. 두 ID는 바이트열이 정확히 같을 때만 같다. 앞자리 0, 수 값, 대소문자로 서로 다른 ID를 합치지 않는다(`0123`과 `123`은 다른 ID). 사용자 결정 2026-11-01(Q68·Q69). | [기록 3.5절](repeater-compat-record.md#35-계약-1절-항목별-해석) "중계 ID 비교"·"중계 ID 길이" 행 |
| 버전 문자열 | Viewer 쪽의 시작 바이트 `RFB 000.000\n` 하나다(1.1절 D). Server 쪽에는 없다. | M2-01 |
| 시간 | 250바이트는 제한 시간 H 안에 다 와야 한다. TCP는 초기 교환에 들어간 때부터, WS·WSS는 `101` 응답을 보낸 때부터 잰다([설정 계약](config-v2.md)의 `handshake_secs`). | E-04 |

검사 순서는 다음과 같다(사용자 결정 2026-11-01(Q80)). Viewer는 빈 값도 먼저 모드를 판정한다. 빈 값은 `ID:`로 시작하지 않으므로 Mode I이다. 그 모드가 꺼졌으면 빈 값·본문을 더 검사하지 않고 `mode_disabled`로 거절한다. 모드가 켜졌을 때 빈 값·본문 문법을 검사한다. 따라서 빈 Viewer 값은 Mode I이 켜졌으면 `handshake_invalid`다. TCP Server는 모드 활성 검사보다 먼저 `ID:` 접두어를 검사한다. 빈 값과 비접두어 값은 항상 `handshake_invalid`다. `ID:`만 있는 값은 Mode II가 꺼졌으면 `mode_disabled`이고, 켜졌으면 빈 ID라 `handshake_invalid`다.

### 1.3 Mode I 대상 문법

값은 `<호스트>` 또는 `<호스트>:<포트>`다. 공백과 그 밖의 구분 문자는 받지 않는다. 규칙에 맞지 않으면 `handshake_invalid`다.

| 부분 | 문법 | 근거 |
| --- | --- | --- |
| 호스트: IPv4 | 십진수 넷을 점으로 이은 표기. 각 자리 0~255이고 앞자리 0은 받지 않는다([설정 계약 1.3절](config-v2.md#13-ip-항목-문법)의 단일 IPv4와 같다). | M1-01 |
| 호스트: DNS 이름 | [설정 계약 1.5절](config-v2.md#15-mode-i-대상-허용-항목-문법)의 DNS 이름 문법과 같다. 대소문자를 구분하지 않는다. UltraVNC Viewer는 이름을 대문자로 바꿔 보낸다. 숫자와 점만으로 된 값은 IPv4로 해석하고, IPv4 문법에 맞지 않으면 `handshake_invalid`다. | M1-03 |
| 호스트: IPv6 | 대괄호로 묶은 RFC 4291 텍스트 표기다(`[2001:db8::30]`). 영역 ID(`%`)는 받지 않는다. 대괄호 없는 IPv6는 포트와 구분할 수 없으므로 받지 않는다. 대상 버전 Repeater에는 없는 확장이다(3.1절). 사용자 결정 2026-11-01(Q70). | M1-06 |
| 포트 | `:` 뒤의 십진수 1~65535. 앞자리 0은 받지 않는다. 생략하면 5900이다. | M1-02 |
| 주소 해석과 정책 | 이름은 IPv4·IPv6 해석 결과를 모두 쓴다(Q70). 허용 정책은 실제로 연결할 IP와 포트에 적용한다([중계 스펙 3.2절](../specs/relay.md#32-mode-i), [설정 계약 1.5절](config-v2.md#15-mode-i-대상-허용-항목-문법)). IPv4-mapped IPv6 주소(`::ffff:0:0/96`)는 IPv4 주소로 바꾼 뒤 판정하고 연결한다. 설정 항목에는 이 대역이 없으므로, 바꾸지 않으면 IPv6 전체 허용 항목으로 IPv4 정책을 우회할 수 있다. | M1-01, M1-03, M1-04 |

### 1.4 형식과 무관한 제품 제약

아래 규칙은 스펙과 PRD가 정했다. 1절의 형식과 함께 적용한다.

| 규칙 | 내용 | 근거 |
| --- | --- | --- |
| 분할·병합 수신 | 수신 청크 경계와 관계없이 같은 초기 요청을 같은 의미로 해석한다. 한 번의 읽기를 한 메시지로 가정하지 않는다. WS·WSS의 바이너리 메시지 경계도 초기 요청의 경계로 보지 않는다. | RLY-R02, WS-R03, [RLY-002](../specs/relay.md#7-수용-테스트), [WS-018](../specs/websocket.md#7-수용-테스트) |
| 후속 바이트 보존 | 초기 요청 뒤에 이미 받은 데이터는 중계 스트림의 첫 바이트로 한 번만 전달한다. 이 데이터도 버퍼 상한 B에 포함한다. | RLY-R03, [RLY-003](../specs/relay.md#7-수용-테스트), [WS-018](../specs/websocket.md#7-수용-테스트) |
| 크기와 시간 | 초기 요청은 250바이트 고정이다. 초기 교환은 제한 시간 H를 지킨다. 설정 키는 `handshake_secs`다([설정 계약](config-v2.md)). | [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수), [RLY-004](../specs/relay.md#7-수용-테스트), [RLY-005](../specs/relay.md#7-수용-테스트) |
| 모드 비활성 | 설정으로 끈 모드의 요청은 대기 등록과 대상 연결 전에 거절한다. | RLY-R17, [RLY-022](../specs/relay.md#7-수용-테스트) |
| 인증 필드 없음 | 초기 교환에 인증 필드를 추가하지 않는다. 접근은 진입점별 출발지 IP 허용 목록으로 통제한다. | [ADR-0002](../decisions/ADR-0002-source-ip-allowlist.md) |
| 대상 선택 단계 없음 | 별도의 대상 선택 메시지나 URL API를 추가하지 않는다. | [relay.md 3.1절](../specs/relay.md#31-초기-교환), [websocket.md 1절](../specs/websocket.md#1-범위와-적용-상태) |
| 중계 ID 노출 제한 | 초기 교환 바이트와 중계 ID 원문을 로그에 그대로 남기지 않는다. 노출 방식은 [운영 정보 계약](ops-info-v1.md)이 정한다. | [websocket.md 5절](../specs/websocket.md#5-항상-지켜야-할-규칙과-비보장-범위) |
| 독립 구현 | 이 계약의 형식은 공개 문서와 허가된 관찰로만 확정했다. UltraVNC·websockify·noVNC 소스 코드를 읽거나 복사·번역하지 않는다. | [PRD 9절](../requirements/PRD.md#9-호환성-조사와-독립-구현-원칙), [relay.md 6절](../specs/relay.md#6-검증-전제) |

재접속은 새 연결의 새 초기 교환이다. 대상 버전에서 Server의 `-autoreconnect`는 끊긴 뒤 약 2초에 다시 등록했고, 새 Viewer와 매칭됐다(T-03). 서버가 세션을 자동으로 재매칭하지 않는다는 스펙 규칙은 그대로다([websocket.md 4절](../specs/websocket.md#4-오류와-종료)).

## 2. 오류

아래 코드 이름은 사용자 결정 2026-11-01(Q72)로 채택했다. WS close 사유(reason)와 로그의 `reason`이 같은 이름을 쓴다. 로그에서의 대응은 [운영 정보 계약 1.6절](ops-info-v1.md#16-원인-코드)이 정한다. 이름이 다른 `handshake_incomplete`의 대응도 거기에 있다.

TCP 진입점은 오류를 알리는 바이트를 보내지 않고 정상 종료(FIN)로 연결을 닫는다. 대상 버전 Repeater도 오류 바이트를 보내지 않았다([기록 3.5절](repeater-compat-record.md#35-계약-1절-항목별-해석) D행). 닫을 때 읽지 않은 수신 데이터가 남아 있으면 OS가 재설정(RST)을 보낼 수 있다. WS·WSS 진입점은 연결이 성립한 뒤이므로 WebSocket close 코드로 알린다. 번호와 규칙은 [WebSocket 중계 계약 1.5절](websocket-relay-v1.md#15-close-코드)이 정한다.

| 오류 코드 | 의미·형식 | 관련 스펙 시나리오 |
| --- | --- | --- |
| `handshake_invalid` | 초기 요청의 값이 1.2·1.3절 규칙에 맞지 않음. 빈 값, 숫자가 아닌 바이트가 든 중계 ID, 숫자가 없는 중계 ID, TCP Server 진입점에서 `ID:`로 시작하지 않는 값, Mode I 대상 문법 위반이 해당한다. 대기 등록과 대상 연결을 만들지 않는다. | [relay.md 4절](../specs/relay.md#4-오류와-복구) 첫 행, [RLY-004](../specs/relay.md#7-수용-테스트) |
| `handshake_incomplete` | 초기 요청 250바이트가 다 오기 전에 상대가 연결을 끊음. 상대가 이미 없으므로 알릴 방법이 없다. 시간 초과와 구분해 기록한다. | [RLY-R04](../specs/relay.md#21-판정-가능한-규칙), [RLY-005](../specs/relay.md#7-수용-테스트) |
| `handshake_timeout` | 제한 시간 H가 지나도 초기 요청 250바이트가 다 오지 않음. | [RLY-005](../specs/relay.md#7-수용-테스트), [OPS-006](../specs/operations.md#7-수용-테스트) |
| `mode_disabled` | 설정으로 끈 모드의 요청. | [RLY-022](../specs/relay.md#7-수용-테스트) |
| `target_not_allowed` | Mode I 대상의 최종 IP·포트가 허용 정책 위반. 대상 연결을 만들지 않는다. | [relay.md 4절](../specs/relay.md#4-오류와-복구) 셋째 행, [RLY-007](../specs/relay.md#7-수용-테스트), [RLY-008](../specs/relay.md#7-수용-테스트) |
| `target_connect_failed` | Mode I의 주소 해석 실패, 연결 거절, 연결 시간 초과(C). 원인은 로그·대시보드에서 구분하고 끝점에는 하나의 코드로 알린다. | [relay.md 4절](../specs/relay.md#4-오류와-복구) 넷째 행, [RLY-009](../specs/relay.md#7-수용-테스트) |
| `duplicate_id` | Mode II에서 대기 중이거나 활성 세션이 쓰는 ID에 같은 역할이 추가됨. 뒤에 온 연결을 바로 닫는다. TCP는 FIN, WS·WSS는 close 4004다. 뒤에 온 연결은 대기 등록을 만들지 않고, 먼저 온 연결과 활성 세션은 유지한다. 사용자 결정 2026-11-01(Q74). | [relay.md 3.3절](../specs/relay.md#33-mode-ii) 4항, [RLY-013](../specs/relay.md#7-수용-테스트), [RLY-015](../specs/relay.md#7-수용-테스트) |
| `waiting_full` | 대기 연결 상한 Lw 도달로 새 대기 등록을 거절. | [OPS-005](../specs/operations.md#7-수용-테스트) |
| `wait_timeout` | 대기 시간 W가 지나도 상대가 도착하지 않음. | [RLY-014](../specs/relay.md#7-수용-테스트), [operations.md 5절](../specs/operations.md#5-오류와-복구) |

## 3. 호환성 규칙

- `vnc-relay`는 1절 규칙에 맞지 않는 초기 요청을 거절한다(`handshake_invalid`). 추측해서 통과시키지 않는다.
- 호환 변경: 관찰 기록으로 확인한 새 UltraVNC·noVNC 버전을 지원 범위에 추가하되 기존 해석을 바꾸지 않는 경우다. 이때 `1.x.0`으로 올린다. 관찰 기록이 없는 버전은 호환 지원으로 표기하지 않는다([relay.md 5절](../specs/relay.md#5-항상-지켜야-할-규칙과-비보장-범위)).
- 호환성을 깨는 변경(Breaking change): 초기 요청의 해석, 시작 바이트, 중계 ID 비교 규칙, 대상 문법이 바뀌어 지금 받는 요청을 다르게 처리하는 경우다. 새 major 버전(`repeater-handshake` 2.0.0)으로 올리고 마이그레이션·롤백 계획을 함께 작성한다.
- 버전별 차이는 관찰 기록에 버전 단위로 남긴다. 차이가 있으면 1절 표에 버전 열을 추가한다.

### 3.1 대상 버전 Repeater와 다르게 처리하는 입력

아래 입력은 대상 버전 Repeater 1.6.0.0과 결과가 다르다. 양쪽에 숫자로만 된 같은 ID를 넣는 Mode II와 IPv4·이름 대상을 쓰는 Mode I에서도 시작 바이트 시점, Repeater가 거절한 큰 숫자 ID(`999999999`), 대기·초기 교환 시간 제한은 다르다. 그 밖의 차이는 Repeater가 서로 다른 ID를 매칭하던 경우(앞자리 0, 5900 차이, 20자리 ID 쌍), 숫자 뒤에 다른 바이트가 붙은 ID, 오류 입력, IPv6 대상, 같은 역할 중복 ID, WebSocket·TLS 접속에서 생긴다.

| 입력 | Repeater 1.6.0.0(관찰) | `vnc-relay` | 근거 |
| --- | --- | --- | --- |
| 시작 바이트의 시점 | 접속 약 1.0초 뒤에 보낸다. | 초기 교환에 들어가자마자 보낸다. Viewer·noVNC는 시작 바이트를 받은 뒤에 요청을 보내므로 기다릴 이유가 없다. | 기록 3.5절 B·F행. [INFERENCE: 시점 차이는 끝점 동작을 바꾸지 않는다. 작업 2.7의 실제 끝점 시험으로 확인한다.] |
| 앞자리 0만 다른 ID(`0123`/`123`), 100 미만 ID와 5900을 더한 ID(`12`/`5912`), 서로 다른 20자리 ID 한 쌍 | 매칭된다. | 다른 ID다. 매칭하지 않는다. | Q68 |
| 숫자 뒤에 다른 바이트가 있는 ID(`56-78`, `12-34`, `12 34`, `12ab`) | `56-78`/`56`, 같은 값끼리의 `12-34`·`12 34`, `12ab`/`12AB`가 매칭됐다(M2-07, M2-12). | `handshake_invalid` | Q68 |
| Server ID `999999999` | 거절된다. | 받는다. | Q68·Q69 |
| 형식이 틀린 요청(TCP Server 진입점의 `XX:1234`·`1234`처럼 `ID:`로 시작하지 않는 값), 꺼진 모드의 요청 | 연결을 닫지 않고 둔다(E-01, M2-06b, M1-00). 숫자로 시작하지 않는 ID(`ID:abcd`)와 빈 ID는 바로 닫는다. | 모두 닫는다(`handshake_invalid`, `mode_disabled`). | [relay.md 4절](../specs/relay.md#4-오류와-복구) |
| 짝이 없는 대기 | 60초 넘게 유지했고 대기 시간 상한은 보이지 않았다. | W가 지나면 닫는다(`wait_timeout`). | [설정 계약](config-v2.md)의 `wait_secs` |
| 초기 교환 시간 | 접속 약 6초 뒤(Viewer 쪽은 마지막 수신 약 5초 뒤) 닫는다. | H가 지나면 닫는다(`handshake_timeout`). | `handshake_secs`, E-04 |
| IPv6 대상(대괄호 IPv6, 이름의 IPv6 결과) | 대상에 연결하지 않고 닫는다. | 허용 정책을 만족하면 연결한다. 호환 지원이 아닌 확장이다. | Q70, M1-06 |
| 같은 역할 중복 ID | 대기 중인 ID: 먼저 온 연결을 유지하고, 뒤에 온 연결은 등록하지 않으며 닫지도 않는다. 활성 세션이 쓰는 ID: 관찰하지 않았다(사용자 결정 2026-11-01(Q75)). | 대기 중이거나 활성 세션이 쓰는 ID 모두 뒤에 온 연결을 바로 닫는다(`duplicate_id`). 먼저 온 연결과 활성 세션은 유지한다. | Q74·Q75, 기록 3.5절 |
| WebSocket·TLS 접속 | Viewer 포트에서 처리하지 않는다. | WS·WSS Viewer 진입점에서 처리한다. | [WS·WSS 스펙](../specs/websocket.md) |

## 4. 버전과 지원 단계

| 상태 | 조건 | 지원 종료·대체 |
| --- | --- | --- |
| Draft | 2026-11-01 채택 전 상태. 관찰 증거가 없어 구현 기준이 아니었다. | 해당 없음 |
| Active | 현재. 사용자 채택(2026-11-01, Q73). 1절이 구현 기준이다. 구현과 계약 테스트의 결과는 5절 표가 관리하며, 통과하기 전에는 호환 지원 완료로 표시하지 않는다. | v2가 Active가 된 뒤 Deprecated로 바꾼다. |
| Deprecated | 대체 계약이 Active이다. | 지원 종료 날짜는 대체 계약에서 정한다. |
| Retired | 지원 종료. | 새 구현의 근거로 쓰지 않는다. |

## 5. 예제와 계약 테스트

예제의 주소는 문서용 IP(`192.0.2.0/24`, `2001:db8::/32`)이고 ID는 가상 값이다. 바이트는 16진수로 적는다.

| 예제 | 바이트 | 결과 |
| --- | --- | --- |
| Server 등록(ID `1001`) | Server → `vnc-relay`: `49 44 3A 31 30 30 31 00`(`ID:1001` + NUL) + `00` 242개 | Mode II, Server, 중계 ID `1001`로 대기 등록 |
| Viewer 요청(ID `1001`) | `vnc-relay` → Viewer: `52 46 42 20 30 30 30 2E 30 30 30 0A`. Viewer → `vnc-relay`: `ID:1001` + NUL + 아무 값 242바이트 | 위 Server와 매칭. 이어서 Viewer는 Server의 `RFB 003.008\n`부터 받는다. |
| Mode I 요청 | 시작 바이트 뒤 Viewer → `vnc-relay`: `192.0.2.30:5900`(15바이트) + NUL + 234바이트 | 대상 `192.0.2.30`, 포트 5900. 허용 정책을 만족하면 연결 |
| 포트 생략 | 값 `192.0.2.30` | 포트 5900 |
| IPv6 대상 | 값 `[2001:db8::30]:5900` | 대상 `2001:db8::30`, 포트 5900 |
| 대괄호 없는 IPv6 | 값 `2001:db8::30:5900` | `handshake_invalid` |
| 숫자가 아닌 ID | 값 `ID:12ab` | `handshake_invalid` |
| 앞자리 0만 다른 ID | Server 값 `ID:0123`, Viewer 값 `ID:123` | 서로 다른 ID. 매칭하지 않음 |
| 요청과 후속 바이트를 함께 보냄 | Viewer가 250바이트 요청 바로 뒤에 `56 3E 61`(`V>a`)을 같은 쓰기로 보냄 | 매칭 직후 Server가 받는 첫 바이트가 `V>a`다. 250바이트 요청은 Server에 전달하지 않는다. |

형식의 증거는 [호환성 검증 기록 3절](repeater-compat-record.md#3-관찰-증거-todo-14)이다. 아래 계약 테스트는 `vnc-relay`를 대상으로 한다. 결과와 증거는 [검증 기록](../AGENTS.md#검증-기록)을 따른다. 스펙과 같은 테스트는 해당 시나리오의 기록을 연결한다.

| 주체·테스트 위치 | 확인할 계약 규칙 | 결과 | 실행 기록 |
| --- | --- | --- | --- |
| Consumer: `crates/domain/src/handshake.rs` 단위 시험(`parses_the_same_request_at_every_split_boundary`, `rejects_empty_values_invalid_ids_and_invalid_targets`, `rejects_disabled_modes_before_validating_their_payloads`, `ignores_padding_after_the_first_nul_and_accepts_no_nul_requests`, `parses_targets_with_the_existing_address_and_name_rules` 등 8개), `crates/domain/src/relay_id.rs` 단위 시험(`keeps_leading_zeroes_and_accepts_one_to_247_digits` 등 2개) | 1.2·1.3절: 250바이트 경계, 첫 NUL과 채움, 모드·빈 값 검사 순서, 중계 ID 문자·길이·정확 일치, 대상 문법, IPv4-mapped 변환, 오류 코드. 실제 I/O는 범위 밖이다. | 통과(단위 범위) | [초기 교환 설계의 실행 기록](../architectures/initial-exchange.md#실행-기록). 커밋 `0cf18b6`에서 Windows·WSL 게이트 통과(시나리오 통과 23/24, 부분 87/86). |
| Producer: UltraVNC Viewer·Server(테스트 클라이언트로 대체) → Consumer: `vnc-relay` TCP 진입점 / `crates/adapters/src/websocket_stream/tests.rs` `sends_the_greeting_as_one_binary_message`, `crates/app/src/relay/handshake.rs` `sends_the_viewer_greeting_once_and_sends_none_to_the_server`, `tests/relay_entry_scenarios.rs` `accepts_both_tcp_roles_and_preserves_split_and_merged_requests_with_prefixes_once`, `crates/adapters/src/websocket_stream/tests.rs` `keeps_initial_exchange_bytes_across_frames_and_messages`, `crates/app/src/relay/handshake.rs` `parses_split_and_coalesced_requests_identically`, `crates/domain/src/handshake.rs` `parses_the_same_request_at_every_split_boundary` | 시작 바이트 전송, 유효한 초기 요청 수락과 분할·병합 해석: [RLY-001](../specs/relay.md#7-수용-테스트), [RLY-002](../specs/relay.md#7-수용-테스트), [RLY-003](../specs/relay.md#7-수용-테스트) | 통과. 테스트 클라이언트 시나리오(RLY-001~003)는 통과(4e4a271). 실제 UltraVNC 1.8.3.0 Viewer·Server의 초기 요청도 작업 2.13에서 모두 수락되어 세션이 성립했다(Windows VNC Relay) | [초기 교환 설계의 실행 기록](../architectures/initial-exchange.md#실행-기록), 커밋 `4e4a271`. [실제 UltraVNC 수동 확인 기록 4.1·4.2절](../how-to/verify-ultravnc-manually.md#41-2026-11-05-실행-기록)
| Producer: 잘못된 초기 요청 생성기 → Consumer: `vnc-relay` / `tests/relay_entry_scenarios.rs` `rejects_every_malformed_initial_value_without_registration_or_target_socket`, `incomplete_disconnect_and_h_timeout_have_distinct_causes_and_return_l_for_each_role`, `disabled_modes_reject_before_waiting_registration_or_target_connect`; `crates/adapters/tests/ops_info_contract.rs` `writes_initial_exchange_rejections_with_conn_id_and_disabled_mode`; `crates/adapters/tests/websocket_relay.rs` `invalid_initial_requests_close_4008_and_log_the_rejection`, `incomplete_initial_exchange_closes_4009_and_logs_h_timeout`, `disabled_modes_close_1008_before_registration_or_target_connect`; `crates/adapters/tests/websocket.rs` `initial_exchange_timeout_sends_4009_after_the_binary_greeting_on_ws_and_wss`; `crates/app/src/relay/handshake.rs` `distinguishes_peer_eof_from_the_absolute_h_timeout`; `crates/domain/src/handshake.rs` `rejects_disabled_modes_before_validating_their_payloads` | 형식 오류·미완료 단절·시간 초과·꺼진 모드: [RLY-004](../specs/relay.md#7-수용-테스트), [RLY-005](../specs/relay.md#7-수용-테스트), [RLY-022](../specs/relay.md#7-수용-테스트) | 통과(시나리오, 4e4a271) | [초기 교환 설계의 실행 기록](../architectures/initial-exchange.md#실행-기록), 커밋 `4e4a271`
| Producer: UltraVNC Server·Viewer(Mode II) → Consumer: `vnc-relay` / `crates/app/src/relay.rs` `matches_both_arrival_orders_and_finishes_once_with_exact_counters`, `crates/domain/src/matching.rs` `matches_server_first`, `matches_viewer_first`, `tests/serve.rs` `mode2_relays_bytes_in_both_orders_with_trailing_data`, `tests/relay_matching_scenarios.rs` `different_ids_same_roles_and_leading_zero_collisions_never_cross_match`, `simultaneous_same_id_arrivals_form_one_pair…`, `arrival_races_disconnect_and_wait_expiry…`, `tcp_ws_and_wss_duplicates_close_without_registering_or_replacing_the_existing_pair` | 두 접속 순서의 매칭, 다른 ID·앞자리 0만 다른 ID, 중복 ID: [RLY-010](../specs/relay.md#7-수용-테스트), [RLY-011](../specs/relay.md#7-수용-테스트), [RLY-012](../specs/relay.md#7-수용-테스트), [RLY-013](../specs/relay.md#7-수용-테스트), [RLY-015](../specs/relay.md#7-수용-테스트) | 통과. RLY-012·013·015는 테스트 클라이언트 시나리오로 통과(4e4a271). RLY-010·011은 실제 UltraVNC 1.8.3.0으로 각각 독립 2회 통과(Windows VNC Relay, 프로젝트 소유자 캡처 검토 전). Ubuntu VNC Relay 미실행 | [초기 교환 설계의 실행 기록](../architectures/initial-exchange.md#실행-기록), 커밋 `4e4a271`. [실제 UltraVNC 수동 확인 기록 4.1·4.2절](../how-to/verify-ultravnc-manually.md#41-2026-11-05-실행-기록)
| Producer: UltraVNC Viewer(Mode I) → Consumer: `vnc-relay` / 시험 파일 없음. 실제 UltraVNC 1.8.3.0 수동 확인(TODO 2.13) | 실제 Server 대상 접속: [RLY-006](../specs/relay.md#7-수용-테스트) | 통과(Windows VNC Relay, 독립 2회, 프로젝트 소유자 캡처 검토 전). Ubuntu VNC Relay 미실행 | [실제 UltraVNC 수동 확인 기록 4.1·4.2절](../how-to/verify-ultravnc-manually.md#41-2026-11-05-실행-기록), 커밋 `ceb6a7c` 실행파일
| Producer: WS·WSS 클라이언트 → Consumer: `vnc-relay` WS·WSS 진입점 / `crates/adapters/tests/websocket_relay.rs` `ws_and_wss_forward_initial_trailing_data_once` | 101 직후 시작 바이트, 초기 요청과 후속 바이트를 한 메시지에 넣음, WS·WSS Mode II 두 순서: [WS-018](../specs/websocket.md#7-수용-테스트), [WS-003~006](../specs/websocket.md#7-수용-테스트) | 통과(시나리오, 4e4a271) | [초기 교환 설계의 실행 기록](../architectures/initial-exchange.md#실행-기록), 커밋 `4e4a271`
| Producer: noVNC 1.7.0(`repeaterID`) -> Consumer: `vnc-relay` WS·WSS / `tests/websocket_browser.rs` `novnc_repeater_id_completes_rfb_through_ws_and_wss_in_both_server_orders`(`#[ignore]`) | 같은 ID로 매칭 성립, noVNC의 RFB 연결 완료: [WS-025](../specs/websocket.md#7-수용-테스트). TCP Server는 시험 코드의 최소 RFB 3.8 서버다 | 통과(커밋 `f62f11d`, 두 OS `--include-ignored`) | [WebSocket 중계 계약 실행 기록](websocket-relay-v1.md#계약-테스트-실행-기록) |

## 6. 초안을 확정하기 전에 해결할 사항

2절의 오류 코드 이름은 사용자 결정 2026-11-01(Q72)로 채택했다. 이 절에는 관찰 절차와 아직 남은 결정을 둔다.

### 6.1 관찰 절차

2026-11-01에 1~6단계를 마쳤다([호환성 검증 기록 3절](repeater-compat-record.md#3-관찰-증거-todo-14)). 새 버전을 지원 범위에 더하거나 `vnc-relay` 구현 뒤 비교할 때도 이 순서를 따른다. 앞 단계가 끝나기 전에 다음 단계를 시작하지 않는다.

1. 사용 권한과 법적 허용 여부를 확인한다. 패킷 관찰의 허용 여부는 관할법·제품 약관·사용 권한에 따라 별도로 검토한다([PRD 9절](../requirements/PRD.md#9-호환성-조사와-독립-구현-원칙)). 검토 결과를 증거 기록에 남긴다. 검토 전에는 관찰하지 않는다. 프로젝트 소유자는 2026-10-21에 관찰을 허용했고, 사용자 결정 2026-11-01(Q65)로 이 허용을 1단계 완료로 본다([호환성 검증 기록 1절](repeater-compat-record.md#1-관찰-허용-검토-todo-11)). 이 판정은 프로젝트 절차상의 판정이다. 법률 판단이 아니며, 법적으로 허용된다는 뜻이 아니다.
2. 대상 버전을 고정한다. UltraVNC 공식 배포처([다운로드](https://uvnc.com/downloads/ultravnc.html))의 최신 안정판 Server·Viewer·Repeater와 noVNC의 버전을 각각 기록하고 고정한다. 대상 OS는 Windows 11 x64와 Ubuntu 24.04 LTS x64다. 공식 Repeater 관찰은 Windows Repeater만 한다. Ubuntu `uvncrepeater`는 사용자 결정 2026-10-22(Q19)로 보류했다. Ubuntu 24.04 LTS x64는 `vnc-relay` 구현 뒤 같은 시나리오의 비교 캡처에 쓴다(사용자 결정 2026-11-01(Q64)).
3. 통제된 환경을 만든다. 인터넷과 분리한 테스트 네트워크에서 Viewer·Server·Repeater를 각각 별도 호스트 또는 가상 머신에 둔다. 주소는 문서용 IP 대역만 쓴다. 실제 비밀번호·인증 정보는 쓰지 않는다.
4. 패킷 캡처 도구를 Repeater 쪽 네트워크 구간에 둔다. 관찰 대상은 공식 Repeater와 공식 Viewer·Server의 통신이다. 같은 시나리오를 `vnc-relay` 구현 뒤에도 캡처해 비교한다.
5. 아래 시나리오를 실행한다. 시나리오마다 독립 실행을 2회 이상 해 결과가 같은지 확인한다(횟수는 제안).
   - Mode I: 정상 대상, 포트 생략, 이름과 IP 표기, 금지 대상, 연결 실패, IPv6 대상.
   - Mode II: Server 선접속·Viewer 선접속, 서로 다른 ID, 같은 역할 중복 ID, ID 길이·문자 종류의 경계, `0123`과 `123`처럼 숫자로 보면 같은 ID, 대소문자만 다른 ID.
   - 오류: 형식이 틀린 초기 메시지, 길이가 모자란 상태에서 단절, 길이가 지나친 메시지, 시간 초과.
   - 분할·후속 바이트: 초기 메시지를 나눠 전송, 초기 메시지와 후속 바이트를 함께 전송, 매칭 전후에 도착한 바이트.
   - 종료·재접속: 한쪽 종료, Server `-autoreconnect`, Viewer 재접속.
   - 외부 WebSocket: 고정 버전 noVNC의 `repeaterID`로 WS·WSS 접속, 서브프로토콜 헤더 확인.
6. 캡처한 바이트를 방향별로 분석해 1절의 형식과 비교한다. 해석은 증거 기록에 연결하고, 증거로 확인하지 못한 항목은 미확정으로 남긴다.

UltraVNC·websockify·noVNC 소스 코드는 관찰·분석에 쓰지 않는다.

### 6.2 기록할 증거

| 증거 | 내용 |
| --- | --- |
| 대상 | 구성요소별 이름·버전·배포본 파일 해시·내려받은 날짜와 주소, OS와 버전 |
| 환경 | 호스트·가상 머신 구성, 네트워크 구성(문서용 IP), 캡처 도구 이름과 버전, Repeater 설정(활성 모드, 포트, 허용·거절 항목) |
| 입력 | 시나리오 ID, 실행한 명령줄·화면 설정, 입력한 ID·대상 값 |
| 원본 | 캡처 원본 파일의 해시와 보관 위치. 방향별 바이트 덤프와 시각 |
| 결과 | Repeater와 끝점이 보인 반응. 매칭 성립 여부와 연결 종료 방식 |
| 해석 | 1절 항목(방향·길이·패딩·종료 문자·ID 비교·오류 형식)에 대한 결론과 근거 증거 |
| 승인 | 6.1절 1단계의 사용 권한·법적 검토 결과 |

기록은 [호환성 검증 기록](repeater-compat-record.md)에 남긴다. 이 계약과 [OPS-020](../specs/operations.md#7-수용-테스트)은 그 기록을 링크한다.

### 6.3 미결정 항목의 해소 조건

남은 미결정 항목은 없다.

RLY-TBD-001은 2026-10-22에 해소되어 이 절에서 뺐다. 근거는 [호환성 검증 기록](repeater-compat-record.md) 2절이다. RLY-TBD-002는 2026-11-01에 해소되어 이 절에서 뺐다. 1절을 관찰 증거로 확정하고 사용자 결정 2026-11-01(Q68~Q73)로 채택했다. RLY-TBD-003도 2026-11-01에 해소되어 이 절에서 뺐다. 대상 버전의 결과(뒤에 온 연결을 닫지 않음)를 [기록 3.5절](repeater-compat-record.md#35-계약-1절-항목별-해석)에 남기고, 사용자 결정 2026-11-01(Q74·Q75)로 후발 거절을 정해 2절·3.1절에 반영했다. 세 ID는 재사용하지 않는다.

### 6.4 그 밖의 해결할 사항

- `vnc-relay` 구현 뒤 6.1절 4단계의 비교 캡처를 한다. 3.1절에 없는 차이가 나오면 이 계약이나 구현을 고친다.
