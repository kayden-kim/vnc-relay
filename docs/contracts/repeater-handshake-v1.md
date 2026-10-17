# Repeater 초기 교환 v1 계약

UltraVNC Viewer·Server와 외부 WebSocket 클라이언트가 VNC Relay에 접속한 직후 주고받는 초기 교환의 호환 범위를 정한다. 초기 교환은 중계 방식(Mode I·II)과 대상 주소 또는 중계 ID를 정하는 절차다([용어집](../../GLOSSARY.md)).

**이 계약은 바이트 형식을 정하지 않았다.** 메시지 방향·길이·필드·패딩·종료 문자·ID 비교 규칙은 [6절](#6-초안을-확정하기-전에-해결할-사항)의 관찰이 끝난 뒤에 1절에 적는다. 관찰 전에는 이 문서를 구현 기준으로 쓰지 않는다. 지금 적은 내용은 두 가지뿐이다. 공개 자료로 확인한 사실(출처 링크 포함)과 스펙·PRD가 이미 정한 제품 제약이다.

| 항목 | 값 |
| --- | --- |
| 이름과 버전 | `repeater-handshake` 1.0.0 |
| 상태 | Draft |
| 소유자 | vnc-relay 프로젝트 관리자 |
| 만드는 쪽(Producer) | UltraVNC Viewer, UltraVNC Server, 외부 WebSocket 클라이언트(검증 대상은 고정 버전 noVNC의 `repeaterID` 옵션) |
| 사용하는 쪽(Consumer) | `vnc-relay` (TCP Viewer·TCP Server·WS Viewer·WSS Viewer 진입점) |
| 방향 | 끝점 → `vnc-relay` 요청이 기본이다. `vnc-relay` → 끝점 방향의 응답·오류 바이트가 있는지는 관찰 전 미확정이다. |
| 스키마 기준 파일 | 없음. 관찰 뒤 1절의 표가 형식의 기준이 된다. |
| 기준 스펙 | [TCP 중계와 연결 매칭](../specs/relay.md), [WS·WSS 전송](../specs/websocket.md) |
| 관련 요구사항 | [PRD](../requirements/PRD.md) FR-01·FR-02·FR-03·FR-05, 9절·9.1절 |
| 호환성 범위 | 목표: Mode I·II, TCP·WS·WSS Viewer 진입. 지원을 주장하는 버전은 아직 없다. 대상 버전은 검증 시작 시 고정한다(RLY-TBD-001). |
| 관련 테스트 | [RLY-001~005, RLY-010~015, RLY-022](../specs/relay.md#7-수용-테스트), [WS-003~006, WS-018, WS-025](../specs/websocket.md#7-수용-테스트). 계약 테스트는 5절을 따른다. |

관련 계약: [WebSocket 중계](websocket-relay-v1.md)(WS·WSS 쪽 전송 규칙과 종료 코드), [운영 정보](ops-info-v1.md)(로그에서 중계 ID 노출 방식과 오류 코드의 로그 이름), [설정](config-v1.md)(`initial_message_bytes`·`handshake_secs`·`mode1`·`mode2`·`allow_targets` 키). 관련 결정: [ADR-0002 출발지 IP 허용 목록](../decisions/ADR-0002-source-ip-allowlist.md).

## 1. 형식과 의미

### 1.1 메시지별 확인 상태

아래 표는 메시지마다 공개 문서로 확인한 내용과 관찰로 확정할 항목을 나눈다. 오른쪽 열이 비어 있는 곳이 아니라, 관찰 전에는 단정하지 않는 항목이다. 공개 문서에 없는 내용은 추정해서 채우지 않는다.

| 메시지(논리 이름) | 공개 문서로 확인한 내용 | 관찰로 확정할 항목 |
| --- | --- | --- |
| A. Mode II에서 Server가 보내는 등록 요청 | Server는 Repeater로 나가는 연결을 만든다. Server 설정에서 `-id:????` 또는 `-autoreconnect ID:????`로 ID를 주고, `-connect`의 host는 Repeater다. 포트를 생략하면 5500이다([Server 명령줄](https://uvnc.com/docs/ultravnc-server/51-ultravnc-server-commandline-parameters.html)). ID는 최대 9자리 숫자 예시가 있다([사용법](https://uvnc.com/docs/ultravnc-repeater/72-ultravnc-repeater-usage.html)). Repeater 설정 화면에서 Server가 접속하는 포트는 `Listen`이다([Repeater 소개](https://uvnc.com/addons/repeater.html)). | 연결 직후 누가 먼저 바이트를 보내는지(방향). 메시지 길이와 고정 길이 여부, 패딩 문자, 종료 문자. 와이어에 `ID:` 접두어가 실제로 실리는지. ID 길이·문자 종류의 허용 범위. Repeater가 응답 바이트를 보내는지. |
| B. Mode II에서 Viewer가 보내는 요청 | Viewer는 `-proxy host:port ID:1234`로 접속한다([Viewer 명령줄](https://uvnc.com/docs/ultravnc-viewer/52-ultravnc-viewer-commandline-parameters.html)). Repeater 설정 화면에서 Viewer가 접속하는 포트는 `Accept`다([Repeater 소개](https://uvnc.com/addons/repeater.html)). 같은 ID의 Server와 Viewer를 매칭하며, ID만으로는 보안이 없다고 설명한다([Connections](https://uvnc.com/docs/documentation/132-ultravnc-connections.html)). | 메시지A와 같은 항목. 추가로 Viewer 쪽의 방향·길이·패딩이 Server 쪽과 같은지. 요청 뒤에 이어지는 RFB 바이트와 경계가 어디인지. |
| C. Mode I에서 Viewer가 보내는 요청 | Viewer는 `-proxy host:port -connect hostname`로 접속한다([Viewer 명령줄](https://uvnc.com/docs/ultravnc-viewer/52-ultravnc-viewer-commandline-parameters.html)). Repeater를 실행하는 PC가 로컬 DNS를 쓸 수 있으면 IP 대신 DNS 이름을 쓸 수 있다고 설명한다([Repeater 소개](https://uvnc.com/addons/repeater.html)). 입력 예시는 화면 이미지이며, 텍스트에서 입력 형식은 확인되지 않는다. | 대상 주소·포트의 문법(구분 문자, 포트 생략 시 기본값, 이름과 IP 표기, IPv6 표기). 길이·패딩·종료 문자. 방향. Repeater가 응답 바이트를 보내는지. |
| D. `vnc-relay`가 끝점에 보내는 응답·오류 표시 | 공개 문서에서 확인한 내용 없음. | 응답 바이트가 있는지. 있으면 형식과 방향. 형식 오류·비허용·중복 ID·대기 시간 초과 때 Repeater가 연결만 닫는지, 바이트를 보내는지. 스펙은 호환 계약이 없는 상태에서 오류 응답 바이트를 만들지 않는다고 정했다([relay.md 4절](../specs/relay.md#4-오류와-복구)). |
| E. 초기 교환 뒤 RFB 스트림의 시작 | RFB는 서버가 먼저 `RFB xxx.yyy\n` 형식의 12바이트 ProtocolVersion 메시지를 보낸다([RFC 6143 7.1.1절](https://www.rfc-editor.org/rfc/rfc6143#section-7.1.1)). RFC 6143은 Repeater 확장을 규정하지 않는다([PRD 13절](../requirements/PRD.md#13-참고-자료)). | 초기 교환과 RFB 첫 바이트의 경계. 초기 교환 메시지와 같은 읽기 단위로 들어온 후속 바이트의 처리 순서. Mode II에서 매칭 전후에 이미 도착한 바이트의 처리 순서(PRD 9.1절). |
| F. 외부 WebSocket 클라이언트(noVNC)의 요청 | noVNC `RFB()`의 `repeaterID` 옵션은 "만나는 VNC repeater에 제공할 ID"이다. `wsProtocols` 기본값은 빈 배열이다([noVNC API](https://novnc.com/noVNC/docs/API.html)). 같은 문서는 WebSocket이 표준 RFB 스트림을 제공해야 한다고 설명한다. `connect` 이벤트는 서버와의 핸드셰이크가 모두 끝난 뒤 발생한다([noVNC API](https://novnc.com/noVNC/docs/API.html)). | `repeaterID`를 어떤 바이트로 보내는지. 메시지A·B와 같은 형식인지. 클라이언트가 먼저 보내는지, `vnc-relay`의 바이트를 기다리는지. 실제로 `Sec-WebSocket-Protocol`을 보내는지. 한 WebSocket 메시지에 초기 교환과 후속 바이트가 함께 들어가는지. 대상은 검증 시작 시 고정한 noVNC 버전이다(WS-TBD-005). |

공개 문서 사이의 차이도 있다. Repeater 기본 포트를 5901과 5500으로 적은 문서가 있고([Repeater 안내 PDF](https://www.uvnc.eu/download/1210/Repeater.pdf)), Viewer 포트를 5900, Server 포트를 5500으로 적은 2005년 Linux 이식판 안내도 있다([Single Click Repeater](https://uvnc.com/pchelpware/sc/repeater.html)). 이 차이는 버전별로 달라질 수 있다는 뜻이다. 포트 번호는 `vnc-relay` 설정이 정하므로 이 계약의 형식 항목이 아니다. `vnc-relay`의 기본 포트는 [설정 계약 1.2절](config-v1.md#12-키-표)이 정한다. 오래된 설명은 대상 버전으로 다시 확인한다.

### 1.2 논리 필드

필드의 자료형·길이·범위는 관찰 전 미확정이다. 필수 여부는 공개 문서에서 확인한 범위에서만 적는다.

| 필드·메시지 | 자료형 | 필수 여부·null 허용 여부 | 의미·제약 |
| --- | --- | --- | --- |
| 중계 모드 | 관찰 필요 | 필수. 모드를 말로 지정하는 필드가 따로 있는지는 관찰 필요 | Mode I 또는 Mode II. 공개 문서상 Viewer가 접속하는 포트(`Accept`)와 Server가 접속하는 포트(`Listen`)가 구분된다. `vnc-relay`는 진입점(TCP Viewer·TCP Server)으로 역할을 구분하되, 요청 안의 표시와 일치하는지는 관찰 필요. |
| 역할(Viewer·Server) | 관찰 필요 | Mode II에서 필수 | 같은 ID의 반대 역할만 매칭한다([relay.md 3.3절](../specs/relay.md#33-mode-ii)). WS·WSS는 Viewer만 있다([websocket.md 1절](../specs/websocket.md#1-범위와-적용-상태)). |
| 중계 ID | 관찰 필요(공개 문서 예시는 `ID:` 표기와 최대 9자리 숫자) | Mode II에서 필수 | 짝을 찾는 값이다. 접속 권한을 증명하지 않는다([ADR-0002](../decisions/ADR-0002-source-ip-allowlist.md), [Connections](https://uvnc.com/docs/documentation/132-ultravnc-connections.html)). 비교 규칙은 관찰 필요. 그 전에도 대소문자 변환·공백 제거·숫자 변환으로 서로 다른 ID를 합치지 않는다([relay.md 3.3절](../specs/relay.md#33-mode-ii)). |
| 대상 주소 | 관찰 필요 | Mode I에서 필수 | `vnc-relay`가 연결할 대상이다. 문법은 관찰 필요. 최종 연결 IP와 포트에 허용 정책을 적용한다([relay.md 3.2절](../specs/relay.md#32-mode-i)). |
| 대상 포트 | 관찰 필요 | Mode I에서 필수 여부 관찰 필요 | 생략 시 기본값이 있는지 관찰 필요. |
| 길이·패딩·종료 문자·버전 문자열 | 관찰 필요 | 관찰 필요 | 스펙은 이 값을 추정하지 않는다([relay.md 3.1절](../specs/relay.md#31-초기-교환)). |

### 1.3 형식과 무관한 제품 제약

아래 규칙은 바이트 형식이 정해져도 바뀌지 않는다. 스펙과 PRD가 이미 정했다.

| 규칙 | 내용 | 근거 |
| --- | --- | --- |
| 분할·병합 수신 | 수신 청크 경계와 관계없이 같은 초기 요청을 같은 의미로 해석한다. 한 번의 읽기를 한 메시지로 가정하지 않는다. WS·WSS의 바이너리 메시지 경계도 초기 교환 경계로 보지 않는다. | RLY-R02, WS-R03, [RLY-002](../specs/relay.md#7-수용-테스트), [WS-018](../specs/websocket.md#7-수용-테스트) |
| 후속 바이트 보존 | 초기 요청 뒤에 이미 받은 데이터는 중계 스트림의 첫 바이트로 한 번만 전달한다. 이 데이터도 버퍼 상한 B에 포함한다. | RLY-R03, [RLY-003](../specs/relay.md#7-수용-테스트), [WS-018](../specs/websocket.md#7-수용-테스트) |
| 크기와 시간 | 초기 메시지는 상한 I, 초기 교환은 제한 시간 H를 지킨다. 설정 키는 `initial_message_bytes`, `handshake_secs`([설정 계약](config-v1.md)). | [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수), [RLY-004](../specs/relay.md#7-수용-테스트), [RLY-005](../specs/relay.md#7-수용-테스트) |
| 모드 비활성 | 설정으로 끈 모드의 요청은 대기 등록과 대상 연결 전에 거절한다. | RLY-R17, [RLY-022](../specs/relay.md#7-수용-테스트) |
| 인증 필드 없음 | 초기 교환에 인증 필드를 추가하지 않는다. 접근은 진입점별 출발지 IP 허용 목록으로 통제한다. | [ADR-0002](../decisions/ADR-0002-source-ip-allowlist.md) |
| 대상 선택 단계 없음 | 별도의 대상 선택 메시지나 URL API를 추가하지 않는다. | [relay.md 3.1절](../specs/relay.md#31-초기-교환), [websocket.md 1절](../specs/websocket.md#1-범위와-적용-상태) |
| 중계 ID 노출 제한 | 초기 교환 바이트와 중계 ID 원문을 로그에 그대로 남기지 않는다. 노출 방식은 [운영 정보 계약](ops-info-v1.md)이 정한다. | [websocket.md 5절](../specs/websocket.md#5-항상-지켜야-할-규칙과-비보장-범위) |
| 독립 구현 | 이 계약의 형식은 공개 문서와 허가된 관찰로만 확정한다. UltraVNC·websockify·noVNC 소스 코드를 읽거나 복사·번역하지 않는다. | [PRD 9절](../requirements/PRD.md#9-호환성-조사와-독립-구현-원칙), [relay.md 6절](../specs/relay.md#6-검증-전제) |

재접속과 종료는 대상 버전의 차이를 관찰로 확인한다. 공개 문서에는 다음 설명이 있다. Server는 `-autoreconnect`로 Repeater에 반복 접속한다([Connections](https://uvnc.com/docs/documentation/132-ultravnc-connections.html)). Viewer의 `-autoreconnect`는 Repeater가 막는다([Viewer 명령줄](https://uvnc.com/docs/ultravnc-viewer/52-ultravnc-viewer-commandline-parameters.html)). 서버가 세션을 자동으로 재매칭하지 않는다는 스펙 규칙은 그대로다([websocket.md 4절](../specs/websocket.md#4-오류와-종료)).

## 2. 오류

외부에서 관찰하는 응답 바이트의 형식은 관찰 전 미확정이다. 아래 코드는 결과를 구분하기 위해 이 계약이 **제안**한 이름이며, 채택 전에는 확정이 아니다. WS 종료 사유와 로그의 `reason`이 같은 이름을 쓴다. 로그에서의 대응은 [운영 정보 계약 1.6절](ops-info-v1.md#16-원인-코드)이 정한다. 이름이 다른 `handshake_incomplete`의 대응도 거기에 있다.

TCP 진입점은 오류 응답 바이트를 만들지 않고 연결을 닫는다([relay.md 4절](../specs/relay.md#4-오류와-복구)). 닫는 방식(정상 종료 또는 재설정)은 관찰 뒤에 정한다. WS·WSS 진입점은 연결이 성립한 뒤이므로 WebSocket close 코드로 알린다. 아래 코드 이름은 close 사유(reason) 문자열과 같다. 번호와 규칙은 [WebSocket 중계 계약 1.5절](websocket-relay-v1.md#15-close-코드)이 정한다.

| 오류 코드 | 의미·형식 | 관련 스펙 시나리오 |
| --- | --- | --- |
| `handshake_invalid` | 형식 오류, 미지원 모드, 상한 I 초과. 대기 등록과 대상 연결을 만들지 않는다. | [relay.md 4절](../specs/relay.md#4-오류와-복구) 첫 행, [RLY-004](../specs/relay.md#7-수용-테스트) |
| `handshake_incomplete` | 초기 교환이 끝나기 전에 상대가 연결을 끊음. 상대가 이미 없으므로 알릴 방법이 없다. 시간 초과와 구분해 기록한다. | [RLY-R04](../specs/relay.md#21-판정-가능한-규칙), [RLY-005](../specs/relay.md#7-수용-테스트) |
| `handshake_timeout` | 제한 시간 H가 지나도 초기 교환이 끝나지 않음. | [RLY-005](../specs/relay.md#7-수용-테스트), [OPS-006](../specs/operations.md#7-수용-테스트) |
| `mode_disabled` | 설정으로 끈 모드의 요청. | [RLY-022](../specs/relay.md#7-수용-테스트) |
| `target_not_allowed` | Mode I 대상의 최종 IP·포트가 허용 정책 위반. 대상 연결을 만들지 않는다. | [relay.md 4절](../specs/relay.md#4-오류와-복구) 셋째 행, [RLY-007](../specs/relay.md#7-수용-테스트), [RLY-008](../specs/relay.md#7-수용-테스트) |
| `target_connect_failed` | Mode I의 주소 해석 실패, 연결 거절, 연결 시간 초과(C). 원인은 로그·대시보드에서 구분하고 끝점에는 하나의 코드로 알린다. | [relay.md 4절](../specs/relay.md#4-오류와-복구) 넷째 행, [RLY-009](../specs/relay.md#7-수용-테스트) |
| `duplicate_id` | Mode II에서 대기 중이거나 활성 세션이 쓰는 ID에 같은 역할이 추가됨. 제안 정책은 후발 거절이며 기존 연결은 유지한다. | [relay.md 3.3절](../specs/relay.md#33-mode-ii) 4항, [RLY-013](../specs/relay.md#7-수용-테스트), [RLY-015](../specs/relay.md#7-수용-테스트) |
| `waiting_full` | 대기 연결 상한 Lw 도달로 새 대기 등록을 거절. | [OPS-005](../specs/operations.md#7-수용-테스트) |
| `wait_timeout` | 대기 시간 W가 지나도 상대가 도착하지 않음. | [RLY-014](../specs/relay.md#7-수용-테스트), [operations.md 5절](../specs/operations.md#5-오류와-복구) |

## 3. 호환성 규칙

- `vnc-relay`는 해석할 수 없는 초기 메시지와 지원하지 않는 모드를 거절한다(`handshake_invalid`). 추측해서 통과시키지 않는다.
- 호환 변경: 관찰 기록으로 확인한 새 UltraVNC 버전을 지원 범위에 추가하되 기존 해석을 바꾸지 않는 경우다. 이때 `1.x.0`으로 올린다. 관찰 기록이 없는 버전은 호환 지원으로 표기하지 않는다([relay.md 5절](../specs/relay.md#5-항상-지켜야-할-규칙과-비보장-범위)).
- 호환성을 깨는 변경(Breaking change): 이미 지원한 버전의 초기 메시지 해석이나 ID 비교 규칙이 바뀌는 경우다. 새 major 버전(`repeater-handshake` 2.0.0)으로 올리고 마이그레이션·롤백 계획을 함께 작성한다. 이 계약은 아직 Draft이므로 계획 문서는 없다.
- 버전별 차이는 관찰 기록에 버전 단위로 남긴다. 차이가 있으면 1절 표에 버전 열을 추가한다.

## 4. 버전과 지원 단계

| 상태 | 조건 | 지원 종료·대체 |
| --- | --- | --- |
| Draft | 현재 상태. 공개 자료와 스펙 제약만 있고 관찰 증거가 없다. 구현 기준이 아니다. | 해당 없음 |
| Active | 6절의 해소 조건이 모두 충족되고 1절 표의 '관찰로 확정할 항목'이 증거와 함께 채워진 뒤 프로젝트 소유자가 채택한다. | 종료·대체 정책은 채택할 때 정한다. |

## 5. 예제와 계약 테스트

바이트 예제는 싣지 않는다. 관찰 증거 없이 만든 바이트 열은 실제 UltraVNC 동작과 다를 수 있기 때문이다. 관찰이 끝나면 증거 기록에서 가져온 문서용 값(문서용 IP, 가상 ID)으로 예제를 추가한다.

논리 흐름 예시(바이트 아님): Mode II에서 Server가 ID `1001`(문서용 값)로 먼저 접속해 대기하고, Viewer가 같은 ID로 접속하면 두 연결이 한 번만 매칭된다. 이 예시는 [RLY-010](../specs/relay.md#7-수용-테스트)과 [RLY-011](../specs/relay.md#7-수용-테스트)의 두 접속 순서를 말로 설명한 것이다.

결과와 증거는 [검증 기록](../AGENTS.md#검증-기록)을 따른다. 스펙과 같은 테스트는 해당 시나리오의 기록을 연결한다.

| 주체·테스트 위치 | 확인할 계약 규칙 | 결과 | 실행 기록 |
| --- | --- | --- | --- |
| Producer: UltraVNC Viewer·Server → Consumer: `vnc-relay` TCP 진입점 / 미생성 | 유효한 초기 요청 수락과 분할·병합 해석: [RLY-001](../specs/relay.md#7-수용-테스트), [RLY-002](../specs/relay.md#7-수용-테스트), [RLY-003](../specs/relay.md#7-수용-테스트) | 미실행 | 구현·테스트 파일 없음. 관찰 기록 없음 |
| Producer: 잘못된 초기 요청 생성기 → Consumer: `vnc-relay` / 미생성 | 형식 오류·상한 초과·미완료 단절·시간 초과: [RLY-004](../specs/relay.md#7-수용-테스트), [RLY-005](../specs/relay.md#7-수용-테스트), [RLY-022](../specs/relay.md#7-수용-테스트) | 미실행 | 구현·테스트 파일 없음. 관찰 기록 없음 |
| Producer: UltraVNC Server·Viewer(Mode II) → Consumer: `vnc-relay` / 미생성 | 두 접속 순서의 매칭, 다른 ID·정규화 충돌 ID, 중복 ID: [RLY-010](../specs/relay.md#7-수용-테스트), [RLY-011](../specs/relay.md#7-수용-테스트), [RLY-012](../specs/relay.md#7-수용-테스트), [RLY-013](../specs/relay.md#7-수용-테스트), [RLY-015](../specs/relay.md#7-수용-테스트) | 미실행 | 구현·테스트 파일 없음. 관찰 기록 없음 |
| Producer: UltraVNC Viewer(Mode I) → Consumer: `vnc-relay` / 미생성 | 실제 Server 대상 접속: [RLY-006](../specs/relay.md#7-수용-테스트) | 미실행 | 구현·테스트 파일 없음. 관찰 기록 없음 |
| Producer: WS·WSS 클라이언트 → Consumer: `vnc-relay` WS·WSS 진입점 / 미생성 | 초기 교환과 후속 바이트를 한 메시지에 넣음, WS·WSS Mode II 두 순서: [WS-018](../specs/websocket.md#7-수용-테스트), [WS-003~006](../specs/websocket.md#7-수용-테스트) | 미실행 | 구현·테스트 파일 없음 |
| Producer: 고정 버전 noVNC(`repeaterID`) → Consumer: `vnc-relay` WS·WSS / 미생성 | 같은 ID로 매칭 성립, noVNC의 RFB 연결 완료: [WS-025](../specs/websocket.md#7-수용-테스트) | 미실행 | noVNC 버전 미고정(WS-TBD-005), 구현 없음 |

## 6. 초안을 확정하기 전에 해결할 사항

이 계약의 이름·코드 선택(2절의 오류 코드 이름)은 사용자 채택 전 제안이다.

### 6.1 관찰 절차

관찰은 아래 순서로 한다. 앞 단계가 끝나기 전에 다음 단계를 시작하지 않는다.

1. 사용 권한과 법적 허용 여부를 확인한다. 패킷 관찰의 허용 여부는 관할법·제품 약관·사용 권한에 따라 별도로 검토한다([PRD 9절](../requirements/PRD.md#9-호환성-조사와-독립-구현-원칙)). 검토 결과를 증거 기록에 남긴다. 검토 전에는 관찰하지 않는다.
2. 대상 버전을 고정한다. UltraVNC 공식 배포처([다운로드](https://uvnc.com/downloads/ultravnc.html))의 최신 안정판 Server·Viewer·Repeater와 noVNC의 버전을 각각 기록하고 고정한다. 대상 OS는 Windows 11 x64와 Ubuntu 24.04 LTS x64다.
3. 통제된 환경을 만든다. 인터넷과 분리한 테스트 네트워크에서 Viewer·Server·Repeater를 각각 별도 호스트 또는 가상 머신에 둔다. 주소는 문서용 IP 대역만 쓴다. 실제 비밀번호·인증 정보는 쓰지 않는다.
4. 패킷 캡처 도구를 Repeater 쪽 네트워크 구간에 둔다. 관찰 대상은 공식 Repeater와 공식 Viewer·Server의 통신이다. 같은 시나리오를 `vnc-relay` 구현 뒤에도 캡처해 비교한다.
5. 아래 시나리오를 실행한다. 시나리오마다 독립 실행을 2회 이상 해 결과가 같은지 확인한다(횟수는 제안).
   - Mode I: 정상 대상, 포트 생략, 이름과 IP 표기, 금지 대상, 연결 실패, IPv6 대상.
   - Mode II: Server 선접속·Viewer 선접속, 서로 다른 ID, 같은 역할 중복 ID, ID 길이·문자 종류의 경계, `0123`과 `123`처럼 숫자로 보면 같은 ID, 대소문자만 다른 ID.
   - 오류: 형식이 틀린 초기 메시지, 길이가 모자란 상태에서 단절, 길이가 지나친 메시지, 시간 초과.
   - 분할·후속 바이트: 초기 메시지를 나눠 전송, 초기 메시지와 후속 바이트를 함께 전송, 매칭 전후에 도착한 바이트.
   - 종료·재접속: 한쪽 종료, Server `-autoreconnect`, Viewer 재접속.
   - 외부 WebSocket: 고정 버전 noVNC의 `repeaterID`로 WS·WSS 접속, 서브프로토콜 헤더 확인.
6. 캡처한 바이트를 방향별로 분석해 1절 표의 '관찰로 확정할 항목'을 채운다. 해석은 증거 기록에 연결하고, 증거로 확인하지 못한 항목은 미확정으로 남긴다.

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

기록은 별도의 호환성 검증 기록에 남긴다. 이 문서는 아직 없으며, 만들면 이 계약과 [OPS-020](../specs/operations.md#7-수용-테스트)에서 링크한다.

### 6.3 미결정 항목의 해소 조건

| ID | 해소 조건 | 해소 후 갱신할 문서 |
| --- | --- | --- |
| RLY-TBD-001 | 최신 안정판 UltraVNC Server·Viewer·Repeater와 noVNC의 버전·배포본 해시·OS를 6.2절 형식으로 기록하고 고정한다. | 호환성 검증 기록, [PRD](../requirements/PRD.md) 호환 버전 행, [relay.md 8절](../specs/relay.md#8-미결정-사항) |
| RLY-TBD-002 | 1.1절 표의 모든 '관찰로 확정할 항목'(방향·길이·패딩·종료 문자·ID 비교·오류 형식·후속 바이트 순서)에 증거를 연결한다. Mode I·II의 모든 시나리오가 끝난다. 1절을 증거 기반 표로 바꾸고 프로젝트 소유자가 채택한다. | 이 계약 1절·4절, [relay.md](../specs/relay.md)의 RLY-002~005 실행 기록, 필요하면 [WebSocket 중계 계약](websocket-relay-v1.md)의 close 코드 대응 |
| RLY-TBD-003 | 대상 버전에서 같은 역할 중복 ID의 결과(후발 거절·기존 교체·함께 대기 등)를 관찰해 기록한다. 제안 정책(후발 거절)과 같으면 채택한다. 다르면 차이를 기록하고 프로젝트 소유자가 정책을 고른다. | 이 계약 2절, [relay.md 3.3절](../specs/relay.md#33-mode-ii), PRD |

### 6.4 그 밖의 해결할 사항

- WS-TBD-005: noVNC 검증 버전을 고정하고 기록한다. 고정 전에는 [WS-025](../specs/websocket.md#7-수용-테스트) 결과를 특정 버전의 증거로 쓰지 않는다.
- 1.1절 메시지D: 오류 응답 바이트가 있는지 관찰로 확인한다. 있으면 2절의 TCP 처리 방식과 [relay.md 4절](../specs/relay.md#4-오류와-복구)의 문장을 함께 고친다.
- 2절의 오류 코드 이름을 프로젝트 소유자가 채택한다. close 코드 번호는 채택된 [WebSocket 중계 계약](websocket-relay-v1.md)(Active)을 따른다. 이 계약의 이름을 바꾸면 그 계약의 대응도 함께 확인한다.
