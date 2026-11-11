# WS·WSS 전송 스펙

외부 Viewer·Server가 WS·WSS로 중계 스트림에 접속하는 동작을 정의한다. 내장 WSS와 릴레이 리스너 앞의 역방향 프록시 TLS 종료를 지원한다. 중계 접근은 관리자 로그인과 분리한 진입점별 IP 허용 정책을 따른다. 접속 ticket 확인은 기본으로 꺼져 있고, 설정으로 켠 역할에만 더한다. 신뢰 프록시를 거치면 전달 주소로 클라이언트 주소를 판정한다.

| 항목 | 값 |
| --- | --- |
| 상태 | Active(2026-11-07 채택, 사용자 결정 2026-11-07(Q150)) |
| 관련 요구사항 | [PRD](../requirements/PRD.md) FR-05, 7절 보안 경계, AC-05~08·AC-49 |
| 용어 | [VNC Relay 용어집](../../GLOSSARY.md) |
| 관련 스펙 | [중계와 매칭](relay.md), [설정·운영](operations.md) |
| 관련 계약 | [WebSocket 중계 v2](../contracts/websocket-relay-v2.md), [설정 파일 v4](../contracts/config-v4.md). URL·서브프로토콜·HTTP 상태·종료 코드의 이름·번호와 Origin 정규화는 이 계약이 기준이다. 2026-11-08 추가 범위와 2026-11-10 Viewer 경로 변경은 아래 1절을 따른다. |
| 관련 아키텍처 문서·ADR | [코드 지도](../architectures/code-map.md), [WS·WSS 진입점 모듈 설계](../architectures/websocket-entry-point.md), [WS 스트림 모듈 설계](../architectures/websocket-stream.md), [TLS 모듈 설계](../architectures/tls.md), [ADR-0001 내장 TLS](../decisions/ADR-0001-built-in-tls.md), [ADR-0002 출발지 IP 허용 목록](../decisions/ADR-0002-source-ip-allowlist.md), [ADR-0004 tokio·rustls](../decisions/ADR-0004-tokio-rustls.md), [ADR-0005 자체 WS 스트리밍 프레임 코덱](../decisions/ADR-0005-in-house-ws-streaming-frame-codec.md). |

## 1. 범위와 적용 상태

- WS(`ws://`)·WSS(`wss://`) 지원은 사용자 확정 사항이다. 세부 동작도 채택했다(사용자 결정 2026-11-07(Q150)).
- 공유 WS·WSS 리스너는 Viewer·Server 역할을 제공한다. `/relay/viewer`는 Viewer, `/relay/server`는 Server다(사용자 결정 2026-11-08(Q181), 사용자 결정 2026-11-08(Q188)). 별도 Server 리스너는 만들지 않는다.
- Viewer 경로는 2026-11-10에 `/relay`에서 `/relay/viewer`로 바꿨다(사용자 결정 2026-11-10). 이전 경로 `/relay`는 다른 경로와 같이 404다. 당시 계약은 `websocket-relay` 2.0.0이며, 이전·롤백은 [WebSocket 계약 3.1절](../contracts/websocket-relay-v2.md#31-1x-클라이언트를-200으로-옮기기)을 따른다. 현재 계약은 접속 ticket 확인을 더한 2.1.0이다(사용자 결정 2026-11-11(Q192·Q195)).
- Viewer는 Mode I·II, Server는 TCP Server와 같이 Mode II만 지원한다. 접속 ticket을 켠 Viewer 역할은 Mode I을 거절한다(사용자 결정 2026-11-11(Q203)). WS·WSS Server와 TCP·WS·WSS Viewer의 모든 조합을 양 접속 순서로 검증한다.
- 평문 WS 리스너는 기본 비활성이다. 설정에서 명시적으로 켠 경우에만 연다(WS-R12, WS-024). 테스트에서는 WS 리스너를 켜고 검증한다.
- noVNC, websockify 실행 파일, 원격 화면 렌더러를 의존성으로 추가하지 않는다. noVNC는 번들·배포물에도 넣지 않고 외부 상호운용 검증 대상으로만 쓴다.
- 검증 대상은 검증 시작 시 고정한 noVNC 버전의 [`repeaterID` 옵션](https://novnc.com/noVNC/docs/API.html)으로 접속하는 WS·WSS Mode II다(WS-025). Mode I은 대상이 아니다. 수용 판정은 매칭 성립과 noVNC의 RFB 연결 완료 관찰이다. noVNC 화면 렌더링은 수용 기준이 아니다.
- URL로 대상 주소나 중계 ID를 직접 지정하는 별도 중계 API를 만들지 않는다.
- 릴레이 WS·WSS 리스너의 역방향 프록시 TLS 종료를 지원한다(사용자 지시 2026-11-08). 해당 리스너의 `trusted_proxies`와 `X-Forwarded-For`로 클라이언트 주소를 판정한다(사용자 결정 2026-11-08(Q182)). 관리 리스너의 TLS·전달 헤더 정책은 바꾸지 않는다(사용자 결정 2026-11-08(Q183)).
- 후속: 위 2026-11-08 추가 범위의 구현은 커밋 `3df0693`이다. 시험은 커밋 `51bd747`의 두 OS 게이트에서 통과했다([WSS Server·프록시·최근 로그 판정표](../how-to/set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)).
- 접속 ticket의 형식과 확인은 [WebSocket 계약 1.9절](../contracts/websocket-relay-v2.md#19-접속-ticket)을 따른다. 기본으로 꺼져 있고, 역할마다 WS·WSS에 함께 적용한다(사용자 결정 2026-11-11(Q192·Q197)). 결정 이유는 [ADR-0009](../decisions/ADR-0009-optional-relay-access-ticket.md)를 따른다. 이 추가 범위의 구현·계약 테스트는 미실행(구현 전)이다(사용자 결정 2026-11-11(Q211)).

## 2. 상태와 리스너 책임

| 상태 | 진입 조건 | 종료 조건 |
| --- | --- | --- |
| 접속 수락 시 IP 검사 | WS·WSS 리스너가 TCP 접속을 수락한 직후, 데이터 읽기·TLS 협상 전. 역할은 아직 모름 | 소켓 상대가 신뢰 프록시이거나 켠 역할의 `allow` 합집합에 있으면 진행. 둘 다 아니면 TLS·HTTP 처리 전에 연결 종료 |
| TLS 협상 | accept 검사를 통과한 WSS 연결이 VNC Relay의 내장 TLS 리스너에 접속 | TLS 성립 또는 협상 실패(TLS 1.2 미만 포함) |
| 업그레이드 대기 | HTTP WebSocket 요청 수신 | 요청 머리·경로·역할 → 클라이언트 주소 → Origin → 서브프로토콜 → 켠 역할의 ticket 확인 → 역할별 IP 재검사 → 켠 역할의 사용 기록 쓰기 뒤 업그레이드 또는 거절 |
| Repeater 초기 교환 | WebSocket 연결 성립 | 호환 요청 완료, 오류, 시간 초과, 단절 |
| 대기·중계 | 유효한 Mode I·II 요청 | 중계 스펙의 정상 전이 또는 종료 사건 |
| 종료 중 | close, TCP 오류·EOF, 관리자 종료 | 종료 교환 완료 또는 정리 제한 만료 |
| 종료됨 | WebSocket·중계 자원 회수 | 같은 연결을 재사용하지 않음 |

TLS 협상과 HTTP 업그레이드 단계의 연결은 중계 연결 상한 L에 포함하고, 제한 시간 U를 적용한다([운영 스펙](operations.md#2-상태와-검증-매개변수)). 관리 API 요청은 중계 스트림으로 취급하지 않으며 관리 리스너 연결 상한 Lm으로 따로 센다.

내장 WSS에서는 VNC Relay가 리스너·인증서를 직접 관리하며 클라이언트부터 VNC Relay까지 TLS가 보호한다. 역방향 프록시가 TLS를 종료하면 클라이언트부터 프록시까지 TLS가 보호한다. 프록시에서 WS 리스너까지의 평문 구간은 별도다. 평문 WS 리스너는 기본 비활성이고, 역할을 하나라도 켜면 시작 경고를 기록한다. 보호 구간과 프록시 배치의 책임은 운영자에게 있다.

WS Viewer·WS Server·WSS Viewer·WSS Server는 각각 자기 진입점의 `allow`와 `allowed_origins`를 가진다. 켠 역할의 `allow`가 없으면 시작에 실패하고, 명시적으로 빈 목록이면 그 역할의 모든 클라이언트 주소를 거절한다([운영 스펙](operations.md#4-보안과-신뢰-경계)). accept에서는 역할이 아직 없으므로 소켓 상대를 켠 역할의 `allow` 합집합 또는 해당 리스너의 `trusted_proxies`와 대조한다. 업그레이드에서는 선택한 역할의 `allow`를 클라이언트 주소와 대조한다. 신뢰 프록시는 [계약 1.8절](../contracts/websocket-relay-v2.md#18-신뢰-프록시와-x-forwarded-for)로 주소를 전달한다. 신뢰하지 않는 상대의 헤더는 무시하며 소켓 상대 IP가 클라이언트 주소다.

목록의 IP 문법과 전체 허용 항목 경고는 [설정 계약](../contracts/config-v4.md)이 정한다. 관리자 로그인 쿠키와 중계 ID는 IP 제한을 우회하지 못한다. IP 정책은 같은 공인 IP를 공유하는 개별 사용자를 식별하지 않는다. `trusted_proxies`에는 프록시 주소만 넣어야 하며 클라이언트 범위를 넣어서는 안 된다. 모든 XFF 요소가 신뢰 프록시이면 주소를 대신 고르지 않고 거절한다(WS-036).

IP 목록·`trusted_proxies`·인증서 재로드는 [운영 스펙](operations.md#33-파일-변경-자동-감지와-부분-재로드)에 따른다. 각 accept·upgrade 검사는 그 시점의 유효 설정을 쓴다. 이미 통과한 검사의 결과는 유지한다. 업그레이드를 완료한 연결은 IP 제거 뒤에도 유지한다. 업그레이드 전 HTTP·TLS 연결과 재로드 뒤 새 업그레이드의 역할별 IP 검사에는 그 순간 유효한 목록을 적용한다.

Origin 정책: Origin 헤더가 있으면 해당 역할의 `allowed_origins`와 일치할 때만 업그레이드한다. 허용 값은 `scheme://host[:port]` 형식이며 정확 일치로 비교한다. 와일드카드는 없다. 스킴·호스트·포트 중 하나라도 다르면 거절한다. 하위 도메인은 다른 호스트다. 정규화 세부는 [설정 계약 1.4절](../contracts/config-v4.md#14-origin-값-문법과-비교)이 정하고, 비교 결과의 처리는 [WebSocket 계약 1.3절](../contracts/websocket-relay-v2.md#13-origin-비교)이 정한다. Origin 헤더가 없어도 클라이언트 주소가 해당 역할의 `allow`에 있어야 한다. Origin은 사용자 인증 수단이 아니다.

접속 ticket은 IP 허용 목록을 대신하지 않는다. `[relay.ticket.viewer]`·`[relay.ticket.server]`로 역할마다 켜며, TCP 진입점에는 적용하지 않는다. 평문 WS에서도 켤 수 있지만 ticket 원문이 암호화되지 않으므로 그 구간은 믿을 수 있는 망 안에 있어야 한다(사용자 결정 2026-11-11(Q197·Q199)). 같은 역할의 TCP 진입점과 평문 WS 경고는 [운영 스펙](operations.md#4-보안과-신뢰-경계)을 따른다.

## 3. 정상 흐름과 규칙

1. WSS는 TLS가 성립한 뒤 HTTP 업그레이드를 진행한다. WS는 TLS 없이 업그레이드를 진행한다.
2. accept IP 검사를 통과한 뒤 요청 경로로 역할을 고른다. 클라이언트 주소·역할별 Origin·서브프로토콜·켠 역할의 ticket·역할별 IP 재검사 순서로 검사한다. 거절 우선순위는 Origin 403 → 서브프로토콜 400 → ticket 401 → IP 재검사 403이다. 모두 통과한 뒤 사용 기록을 쓰고 `101`을 보낸다. 앞 검사에서 거절한 요청은 ticket을 소비하지 않는다. 세부 순서는 [계약 1.1절](../contracts/websocket-relay-v2.md#11-업그레이드-요청)이 정한다.
3. WebSocket 연결 뒤 바이너리 메시지의 페이로드를 선택한 역할의 연속 바이트 스트림으로 취급한다.
4. Viewer에는 `101` 응답 직후 시작 바이트를 보낸다. Server에는 보내지 않으며 Server가 `ID:` 초기 요청을 먼저 보낸다. [중계 스펙](relay.md#3-정상-흐름)의 초기 교환으로 대상 주소 또는 ID를 얻는다. 형식은 [Repeater 초기 교환 계약](../contracts/repeater-handshake-v1.md)을 따른다. 계약 밖의 별도 선택 단계를 만들지 않는다.
5. 상대 TCP·WS·WSS 스트림은 바이너리 메시지로 전달한다. 출력 메시지 크기나 경계가 입력과 같을 필요는 없다.

ticket을 켠 역할은 `binary`와 `ticket.` 뒤에 JWS 압축 형식을 붙인 값 하나를 함께 받아야 한다. `binary` 없이 다른 값이 있으면 먼저 HTTP 400으로 거절한다. 헤더가 없거나 ticket 값이 없으면 HTTP 401로 거절한다. ticket 값이 둘 이상이어도 HTTP 401로 거절한다. 수락 응답은 `Sec-WebSocket-Protocol: binary`만 고른다. ticket을 켜지 않은 역할은 기존 서브프로토콜 판정을 유지하며 ticket 값을 확인·기록하지 않는다([계약 1.9.1절](../contracts/websocket-relay-v2.md#191-서브프로토콜에-싣는-값), 사용자 결정 2026-11-11(Q196·Q213)).

| 규칙 ID | PRD 근거 | 동작 |
| --- | --- | --- |
| WS-R01 | FR-05 | WS와 WSS 모두 바이너리 스트림 중계를 제공해야 한다. |
| WS-R02 | FR-05 | 여러 바이너리 메시지를 이어 붙인 바이트의 내용과 순서를 보존해야 한다. |
| WS-R03 | FR-05 | WebSocket 메시지 경계를 RFB 또는 Repeater 메시지 경계로 간주해서는 안 된다. |
| WS-R04 | FR-05 | 텍스트 데이터 메시지를 지원하지 않는 입력으로 거절해야 한다. |
| WS-R05 | FR-05 | ping·pong·close 제어 페이로드를 TCP 중계 스트림에 섞어서는 안 된다. |
| WS-R06 | FR-05 | 분할된 메시지에도 논리 메시지 전체 크기 제한을 적용해야 한다. |
| WS-R07 | FR-05 | 느린 TCP·WebSocket 수신자 양쪽에서 중계 버퍼 상한을 지켜야 한다. |
| WS-R08 | FR-05 | 관리 API 요청을 WebSocket 중계로 업그레이드해서는 안 된다. |
| WS-R09 | 7절 | WSS 협상 실패를 평문 WS 접속으로 자동 전환해서는 안 된다. |
| WS-R10 | 7절 | accept에서는 당시 유효한 켠 역할의 `allow` 합집합 또는 리스너의 `trusted_proxies`에 소켓 상대 IP가 있는지 검사해야 한다. 실패하면 TLS·HTTP 전에 닫아야 한다. 업그레이드 직전에는 클라이언트 주소를 선택한 역할의 당시 유효한 `allow`와 대조해야 한다. 해당 역할의 Origin 정책도 업그레이드에서 검사해야 한다. 실패한 연결은 중계 등록 전에 거절해야 한다. |
| WS-R11 | FR-04·FR-05 | 연결 종료 후 WebSocket과 상대 TCP 연결의 자원을 정리해야 한다. |
| WS-R12 | 7절 | 평문 WS 리스너는 설정에서 명시적으로 켠 경우에만 열어야 한다. |
| WS-R13 | FR-05 | WS·WSS 연결에 Kw 주기로 ping을 보내고, Kw의 응답 제한 안에 pong이 없으면 끊긴 연결로 처리해 정리해야 한다. 서버 ping·pong은 WS-R05에 따라 TCP 중계 스트림에 섞어서는 안 된다. |
| WS-R14 | FR-05 | 공유 리스너의 정확한 `/relay/viewer`는 Viewer, `/relay/server`는 Server로 정해야 한다. Server는 Mode II만 받고 시작 바이트를 보내서는 안 된다. |
| WS-R15 | 7절 | 리스너의 당시 유효한 `trusted_proxies`에 소켓 상대가 있을 때만 `X-Forwarded-For`를 읽어야 한다. 주소 판정은 [계약 1.8절](../contracts/websocket-relay-v2.md#18-신뢰-프록시와-x-forwarded-for)의 1~8단계를 따라야 한다. 신뢰하지 않는 상대의 전달 헤더로 클라이언트 주소를 바꾸어서는 안 된다. |
| WS-R16 | FR-05 | 알 수 없는 경로와 해당 리스너에서 꺼진 역할의 경로는 HTTP 404로 거절하고 `invalid_upgrade`를 기록해야 한다. 쿼리 문자열은 무시하고 끝 슬래시는 다른 경로로 보아야 한다. |
| WS-R17 | 7절 | 신뢰 프록시의 전달 주소 판정 실패는 HTTP 400 `forwarded_for_invalid`로 거절해야 한다. 헤더 누락·빈 값, 잘못된 검사 대상 요소, 모든 요소가 신뢰 프록시인 체인이 해당한다. 등록과 `forwarded_ip`를 남겨서는 안 된다. |
| WS-R18 | FR-05·7절 | 릴레이 WS·WSS 리스너 앞의 프록시 TLS 종료를 지원해야 한다. 소켓 주소와 전달 주소를 구분하고, 성공한 전달 주소를 역할별 IP 판정·운영 로그·관리 조회에 사용해야 한다. TLS 보호 구간을 프록시 뒤 평문 구간과 구분해야 한다. |
| WS-R19 | FR-05·7절·AC-49 | 접속 ticket 확인은 역할마다 설정으로 켤 수 있어야 한다. 기본은 꺼짐이어야 한다. 꺼진 역할의 업그레이드 판정은 ticket 값이 있어도 2.0.0과 같아야 한다(사용자 결정 2026-11-11(Q192·Q197)). |
| WS-R20 | FR-05·7절·AC-49 | ticket을 켠 역할의 업그레이드 요청은 유효한 ticket 하나를 담아야 한다. 없거나 확인에 실패하면 `101` 전에 HTTP 401로 거절해야 한다. 대기 등록·대상 연결을 만들어서는 안 된다(사용자 결정 2026-11-11(Q196·Q202)). |
| WS-R21 | FR-05·7절·AC-49 | 한 ticket은 한 번만 받아들여야 한다. 사용 기록은 업그레이드 검사를 모두 통과한 뒤에 써야 한다. `iat ≤ 기동 시각 + 5`인 ticket은 거절해야 한다([계약 1.9.3·1.9.4절](../contracts/websocket-relay-v2.md#193-확인-순서와-거절), 사용자 결정 2026-11-11(Q201·Q213)). |
| WS-R22 | FR-05·7절·AC-49 | ticket을 켠 역할의 `101` 응답은 `binary`만 골라야 한다. ticket 값을 응답·운영 로그·관리 API에 넣어서는 안 된다(사용자 결정 2026-11-11(Q213)). |
| WS-R23 | FR-05·7절·AC-49 | ticket 설정의 교체는 새 업그레이드 요청부터 적용해야 한다. 이미 `101`을 보낸 연결의 중계 ID 대조와 세션 만료, 사용 기록은 유지해야 한다(사용자 결정 2026-11-11(Q200)). |

메시지 크기 제한 M은 프레임별 크기가 아니라 한 논리 메시지에 적용한다. B와 M 사이에는 대소 제약이 없다(사용자 결정 2026-11-01(Q78)). 프레임 안에서 스트리밍하며 방향당 미전달 보관 바이트는 B 이하다. B보다 큰 유효 프레임도 수락해 스트리밍하며 전체를 저장하지 않는다. 스트리밍 중 제한 초과를 확인할 때까지 일부 바이너리 데이터가 이미 전달될 수 있다. 초과 확인 뒤 추가 데이터 전달을 중지하고 종료 코드 1009로 닫는다. 실패한 메시지 전체를 원자적으로 취소하는 보장은 하지 않는다.

끊긴 연결은 두 단계로 감지한다. WS·WSS의 하부 TCP 소켓에는 TCP keepalive(K)를 적용한다. WebSocket 계층에는 ping·pong(Kw)을 적용한다. 기본값과 유효 범위는 [설정 계약](../contracts/config-v4.md)이 정한다. pong에 응답하는 연결은 데이터가 없다는 이유만으로 끊지 않는다.

## 4. 오류와 종료

| 조건 | 클라이언트·상대가 관찰하는 결과 | 정리·재시도 |
| --- | --- | --- |
| TLS 협상·인증서 검증 실패 | WSS 연결 성립 실패 | 중계 등록 없음, 클라이언트가 설정을 고친 뒤 새 접속 |
| TLS 1.2 미만 협상 | WSS 협상 실패, 평문 전환 없음 | 중계 등록 없음, 규칙 [OPS-R17](operations.md#41-판정-가능한-규칙) |
| 잘못된 경로·업그레이드 요청 | HTTP 단계 거절 | 중계 ID 등록·대상 연결 없음 |
| 접속 수락 시 소켓 상대가 켠 역할의 `allow` 합집합에도 신뢰 프록시 목록에도 없음 | TLS·HTTP 처리 전에 연결 종료 | 중계 등록·대상 연결 없음, 관리자 로그인으로 우회 불가 |
| 업그레이드 직전 클라이언트 주소가 역할별 `allow`에 없거나 역할별 Origin 정책 위반 | HTTP 403, IP 거절은 `ip_not_allowed` | 신규 중계 세션 없음, 관리자 로그인으로 우회 불가 |
| 신뢰 프록시의 XFF 누락·빈 값, 잘못된 검사 대상 요소, 모든 요소가 신뢰 프록시인 체인 | HTTP 400, `connection_rejected(stage=upgrade, reason=forwarded_for_invalid)` | 등록·대상 연결·`forwarded_ip` 없음 |
| ticket을 켠 역할의 ticket 없음·무효·만료·기동 전 발급·재사용 | `101` 전 HTTP 401, `WWW-Authenticate: VncRelayTicket`. `connection_rejected(stage=upgrade)`의 `reason`은 [계약 1.9.3·1.9.4절](../contracts/websocket-relay-v2.md#193-확인-순서와-거절)을 따름 | 대기 등록·대상 연결·`ticket_accepted` 없음. 클라이언트는 새 ticket으로 새 접속 |
| 텍스트 메시지 | 지원하지 않는 데이터로 종료 | 텍스트를 TCP에 전달하지 않음 |
| 잘못된 프레임·논리 메시지 M 초과 | 프로토콜·크기 오류로 종료 | 원인 구분, 추가 전달 중지, 자원 회수 |
| Repeater 초기 교환 오류·시간 초과 | 호환 계약에 따른 종료 | 해당 대기·대상 연결 정리 |
| 101 성공 후 Mode I 대상 연결용 L 상한 도달 | 중계 상한 도달(close 4011)로 종료 | 대상 연결을 만들지 않고 해당 연결 정리(사용자 결정 2026-11-01(Q79)) |
| ticket 연결의 중계 ID 불일치 또는 ticket을 켠 Viewer의 유효한 Mode I 요청 | close 4012 `ticket_relay_mismatch`([계약 1.9.5절](../contracts/websocket-relay-v2.md#195-101-뒤의-중계-id-대조와-세션-만료)) | 기존 초기 요청 검사를 통과한 뒤 거절. 대기 등록·대상 연결 없음, 기다리던 상대 연결 유지 |
| ticket의 세션 만료 시각(`sx`) 도달 | WS·WSS 쪽 close 4013 `session_expired`. 활성 세션의 TCP 쪽은 소켓 닫힘([계약 1.9.5절](../contracts/websocket-relay-v2.md#195-101-뒤의-중계-id-대조와-세션-만료)) | 해당 연결·대기·세션 정리. 활성 세션의 종료 원인은 `session_expired`. 남은 데이터 전달은 보장하지 않음 |
| WebSocket close 수신 | 전체 WebSocket 종료 시작 | 상대 TCP에 EOF 의미 전달 후 제한 시간 안에 정리 |
| TCP 정상 EOF | WebSocket 종료 시작 | 이미 수락한 TCP 데이터 소진 후 close, 소진 불가 시 중단 원인 기록 |
| 네트워크 오류·종료 교환 시간 초과 | 비정상 종료 | 상대와 등록 자원 회수 |
| pong 응답 없음 | Kw의 응답 제한이 지난 뒤 비정상 종료. 서버 ping은 TCP 스트림에 전달하지 않음(WS-R05) | 끊긴 연결로 처리해 상대 TCP와 등록 자원 회수, 규칙 WS-R13 |
| 관리자 종료·프로세스 유예 만료·유예 중 두 번째 종료 요청 | 해당 세션의 양쪽 경로 종료. 두 번째 종료 요청은 남은 유예 없이 바로 종료([운영 스펙 3.4절](operations.md#34-정상-종료와-재시작)). 아직 정리를 시작하지 않은 활성 세션에 close 1001을 보냄(사용자 결정 2026-11-01(Q82)) | 완전한 잔여 데이터 전달은 보장하지 않음 |

WebSocket에는 TCP와 같은 독립적인 양방향 half-close가 없다. 따라서 WS·WSS가 포함된 연결은 한쪽 스트림 EOF 이후 반대 방향을 무기한 유지하지 않는다. 이 종료 의미와 오류별 HTTP 상태·close 코드의 대응은 [WebSocket 계약](../contracts/websocket-relay-v2.md)이 정한다(Active, 2026-10-16 채택). 코드 번호는 이 스펙이 아니라 그 계약이 기준이다.

서버가 자동으로 세션을 재접속·재매칭하지 않는다. 새 접속은 새로운 연결 수명이다.

## 5. 항상 지켜야 할 규칙과 비보장 범위

- TLS가 보호하는 구간과 TCP 평문 구간을 구분해 표시한다. WSS라는 이유로 전체 중계 경로가 종단 간 암호화된다고 표시하지 않는다.
- 다른 세션의 바이너리 데이터를 공유해서는 안 된다.
- 바이너리 페이로드 안의 RFB 인증·화면 데이터를 로그로 남겨서는 안 된다.
- 접속 ticket 원문을 운영 로그·관리 API에 남겨서는 안 된다. 응답의 서브프로토콜에도 넣어서는 안 된다([계약 1.9절](../contracts/websocket-relay-v2.md#19-접속-ticket)).
- 임의의 기존 WebSocket VNC 클라이언트가 호환된다고 보장하지 않는다. Repeater 초기 교환을 수행하는 클라이언트 계약이 필요하다. 다만 검증 시작 시 고정한 noVNC 버전의 `repeaterID`로 접속하는 WS·WSS Mode II는 검증 대상이다(WS-025). 이 검증은 그 버전과 테스트 환경에 한정한다.
- WS·WSS는 Viewer·Server의 스트림 진입 방식이다. 원격 화면 표시 기능을 제공하지 않는다.

## 6. 전송·모드 검증 조합

아래 조합은 모두 검증 대상이다. noVNC 행은 외부 상호운용 검증이며 제품 의존성이 아니다. 일부 조합의 성공으로 전체 지원을 대체하지 않는다. TCP 전용 실제 화면·입력 테스트는 [중계 수용 테스트](relay.md#7-수용-테스트)이 소유한다.

| Viewer 진입 | 모드 | 상대·접속 순서 | 수용 시나리오 |
| --- | --- | --- | --- |
| WS | Mode I | 허용된 TCP 대상 | WS-001 |
| WSS | Mode I | 허용된 TCP 대상 | WS-002 |
| WS | Mode II | TCP Server 선접속 | WS-003 |
| WS | Mode II | WS Viewer 선접속 | WS-004 |
| WSS | Mode II | TCP Server 선접속 | WS-005 |
| WSS | Mode II | WSS Viewer 선접속 | WS-006 |
| WS | Mode II | noVNC `repeaterID`, TCP Server 선접속·후접속 | WS-025 |
| WSS | Mode II | noVNC `repeaterID`, TCP Server 선접속·후접속 | WS-025 |
| TCP·WS·WSS | Mode II | WS·WSS Server × Viewer의 모든 조합, 양 접속 순서 | WS-028 |
| 프록시 뒤 WS | Mode II | TLS 종료 프록시를 통한 Viewer·Server | WS-035 |

## 7. 수용 테스트

WS-007 이후의 공통 중계 테스트는 WS·WSS 각각에 적용한다. 내장 TLS 전용 테스트는 서버 내장 WSS에 적용하고, 프록시 TLS 종료는 WS-035로 검증한다. WS 테스트는 평문 WS 리스너를 명시적으로 켠 설정에서 실행한다(WS-024). 테스트 코드는 `specs/`를 기준으로 한 이 문서의 상대 경로와 시나리오 ID를 메타 정보에 기록한다(예: `websocket.md:WS-001`). 연결 방식은 [검증 기록](../AGENTS.md#검증-기록)을 따른다. 테스트 위치와 결과는 이 문서에 적지 않는다.

별도 조건이 없는 기존 시나리오는 접속 ticket을 끈 상태에 적용한다(이 문서의 판단, 사용자 결정 없음). 접속 ticket 시나리오는 [PRD AC-49](../requirements/PRD.md)와 [계약 1.9절](../contracts/websocket-relay-v2.md#19-접속-ticket)을 따른다.

| 시나리오 ID | 요구사항 ID | 조건과 입력 | 기대 결과 |
| --- | --- | --- | --- |
| WS-001 | FR-05 / WS-R01 | WS Mode I로 실제 TCP 대상에 접속 | 알려진 양방향 바이트 정확히 일치 |
| WS-002 | FR-05 / WS-R01 | WSS Mode I로 실제 TCP 대상에 접속 | 양방향 일치, VNC Relay까지 TLS 암호화 |
| WS-003 | FR-05 / WS-R01 | TCP Server 선접속 후 WS Mode II Viewer | 같은 ID 매칭 뒤 양방향 바이트 일치 |
| WS-004 | FR-05 / WS-R01 | WS Mode II Viewer 선접속 후 TCP Server | 반대 접속 순서에서도 양방향 일치 |
| WS-005 | FR-05 / WS-R01 | TCP Server 선접속 후 WSS Mode II Viewer | 양방향 일치, WSS 구간 TLS 성립 |
| WS-006 | FR-05 / WS-R01 | WSS Mode II Viewer 선접속 후 TCP Server | 반대 접속 순서에서도 양방향 일치 |
| WS-007 | FR-05 / WS-R02·WS-R03 | 같은 스트림의 메시지·프레임 경계를 바꿔 전송 | 상대가 받은 전체 바이트 내용·순서 동일 |
| WS-008 | FR-05 / WS-R04 | 텍스트 데이터 메시지 전송 | 미지원 데이터로 종료, 텍스트는 TCP에 도착하지 않음 |
| WS-009 | FR-05 / WS-R05 | 분할 바이너리 전송 중 ping·pong 삽입 | 제어 교환 처리, 스트림에 제어 바이트 없음 |
| WS-010 | FR-05 / WS-R06 | M 크기와 M+1 크기 메시지를 분할·비분할로 전송 | 경계 내 수락, 초과는 종료·추가 전달 중지 |
| WS-011 | FR-05 / WS-R07 | TCP 수신자·WS 수신자를 각각 지연 | 양 방향 버퍼 B 이내, 수신 재개 시 바이트 보존 |
| WS-012 | FR-05 / WS-R08·WS-R14·WS-R16 | 관리 API·알 수 없는 경로·끝 슬래시 경로 또는 잘못된 업그레이드 요청. `/relay/viewer`·`/relay/server`도 각각 검사 | 잘못된 요청은 중계 업그레이드·대기 등록·대상 연결 없음. 두 유효 경로는 켠 역할로 판정. 후속: 사용자 결정 2026-11-08(Q188)으로 경로별 역할을 추가했다. 후속: 사용자 결정 2026-11-10으로 Viewer 경로를 `/relay/viewer`로 바꿨다. |
| WS-013 | 7절 / WS-R09 | 만료·신뢰되지 않음·호스트 불일치 인증서에 검증 클라이언트 접속 | 각각 TLS 실패, 평문 자동 전환 없음 |
| WS-014 | 7절 / WS-R10·WS-R15 | 신뢰하지 않는 상대이며 켠 역할의 `allow` 합집합 밖인 IP에서 유효 관리자 쿠키·허용 IP 주장 XFF를 포함해 접속 | TLS·HTTP 처리 전 연결 종료, 쿠키·헤더로 우회 불가. 후속: 사용자 결정 2026-11-08(Q182·Q188)에 따라 비신뢰 상대에 한정하며 accept는 합집합을 쓴다. |
| WS-015 | 7절 / WS-R10 | 허용 IP에서 비허용 Origin을 포함해 접속 | Origin 정책에 따라 업그레이드 거절 |
| WS-016 | FR-05 / WS-R11 | 양 끝에서 각각 close·TCP EOF 발생 | 종료 의미에 따라 정리, 수락한 데이터 처리와 원인 일치 |
| WS-017 | FR-05 / WS-R11 | 전송 중 단절·close 응답 미도착 | 정리 제한 후 자원 회수, 다른 세션 영향 없음 |
| WS-018 | FR-05 / WS-R03 | 초기 Repeater 메시지와 후속 바이트를 한 메시지에 넣음 | 초기 교환 처리 뒤 후속 바이트 한 번만 전달 |
| WS-019 | FR-05 / WS-R11 | 잘못된 프레임 순서 또는 유효하지 않은 제어 프레임 | 프로토콜 오류 종료, 등록 자원 회수 |
| WS-020 | FR-05 | noVNC·websockify 없는 배포에서 중계 테스트 | 선택한 WS·WSS 조합이 외부 독립 검증 도구로 동작 |
| WS-021 | 7절 / WS-R10 | 허용 IP의 비브라우저 클라이언트가 Origin·관리 세션 없이 접속 | IP 허용 및 호환 절차로 중계 가능, 관리자 로그인 요구 없음 |
| WS-022 | FR-05·FR-07 / WS-R10 | 역할별 `allow`에서 IP 제거 전 업그레이드 완료한 중계와 업그레이드 전 HTTP·TLS 연결을 유지. accept에서는 켠 역할의 목록 합집합을 통과 | 기존 중계는 계속 전달. 제거 뒤 새 업그레이드는 클라이언트 주소를 요청 역할의 유효 목록으로 재검사해 403. 후속: 사용자 결정 2026-11-08(Q188)에 따라 accept 합집합과 upgrade 역할별 목록을 구분한다. |
| WS-023 | 7절 / WS-R10 | 신뢰 프록시가 아니며 켠 역할의 `allow` 합집합에도 없는 IP가 WSS에 접속 | TLS 협상 전 연결 종료, U 동안 L을 점유하지 않음. 후속: 사용자 결정 2026-11-08(Q182·Q188)으로 accept 조건을 바꿨다. |
| WS-024 | 7절 / WS-R12 | 기본 설정 뒤 `[relay.ws.viewer]` 또는 `[relay.ws.server]`의 역할 하나 이상을 명시적으로 켜고 재시작 | 기본: WS 리스너 없음. 하나 이상 켬: 공유 WS 리스너 열림, `plaintext_ws_enabled(listener=ws)` 경고. 후속: 사용자 결정 2026-11-08(Q188)의 공유 리스너 표기로 바꿨다. |
| WS-025 | FR-05 / WS-R01 | 검증 시작 시 고정한 noVNC 버전의 repeaterID로 WS·WSS Mode II에 접속, TCP Server 선접속·후접속 각각 | 같은 ID로 매칭 성립, noVNC의 RFB 연결 완료 관찰. 화면 렌더링은 수용 기준이 아님 |
| WS-026 | FR-05 / WS-R13 | pong에 응답하지 않는 WS·WSS 클라이언트와 pong에 응답하는 유휴 클라이언트를 각각 둠 | 응답 없는 클라이언트: Kw 뒤 종료, 상대 TCP와 등록 자원 회수. 응답하는 유휴 클라이언트: 연결 유지. 서버 ping 바이트는 TCP 스트림에 섞이지 않음(WS-R05) |
| WS-027 | FR-05·7절 / WS-R10 | 허용 Origin 값으로 `https://a.example:8443`을 설정하고 `https://a.example:8443`, 스킴만 다른 Origin, 포트만 다른 Origin, 하위 도메인 `https://x.a.example:8443`으로 각각 업그레이드 요청 | 같은 값은 업그레이드. 스킴만 다름·포트만 다름·하위 도메인은 업그레이드 전에 거절 |
| WS-028 | FR-03·FR-05 / WS-R01·WS-R14 | WS·WSS `/relay/server`의 Server와 TCP·WS·WSS Viewer를 Mode II로 조합. 각 조합에서 양 접속 순서 | 같은 ID 매칭, 양방향 바이트 일치. 운영 로그 `server_transport`와 관리 API `target_transport`는 Server 전송 경로와 일치 |
| WS-029 | FR-05 / WS-R14·WS-R16 | `/relay/viewer`, `/relay/server`, 꺼진 역할의 경로와 그 밖의 경로를 요청. 1.x의 Viewer 경로 `/relay`도 요청. 쿼리·끝 슬래시도 비교 | 두 정확한 경로는 각각 Viewer·Server. 쿼리는 무시. 꺼진 역할·다른 경로·`/relay`·끝 슬래시는 404 `invalid_upgrade`, 등록 없음. 후속: 사용자 결정 2026-11-10으로 Viewer 경로를 바꿨다. |
| WS-030 | 7절 / WS-R10 | Viewer `allow`에만 있는 비프록시 IP가 accept 뒤 `/relay/server` 요청. 켠 역할 어느 목록에도 없는 비프록시 IP도 접속 | 첫 연결은 accept를 통과하지만 Server 업그레이드에서 403 `ip_not_allowed`. 둘째 연결은 TLS·HTTP 전에 닫힘 |
| WS-031 | FR-05·7절 / WS-R10·WS-R14 | `/relay/server`에 Server 역할의 허용·비허용 Origin을 보냄. Viewer와 Server의 목록은 다르게 설정 | Server `allowed_origins`만 적용. 허용 값은 진행, 비허용 값은 403, Viewer 목록으로 우회 불가 |
| WS-032 | 7절 / WS-R10·WS-R15·WS-R18 | 신뢰 프록시가 여러 XFF 필드와 SP·HTAB, 신뢰 중간 프록시, 가장 오른쪽 비신뢰 IP, 그 왼쪽 위조 요소를 전달. IPv4·IPv6·mapped 표기도 비교 | 가장 오른쪽 비신뢰 정규 IP가 클라이언트 주소. 왼쪽 요소는 검사하지 않음. 역할 `allow`에 사용, `peer_ip`는 소켓 주소 유지, `forwarded_ip`와 관리 API의 해당 `*_forwarded_ip`에 전달 주소 표시 |
| WS-033 | 7절 / WS-R15·WS-R17 | 신뢰 프록시의 XFF 없음·빈 값 또는 검사 대상에 포트·영역 ID·unknown·난독화 토큰·빈 요소·잘못된 IP | HTTP 400 `forwarded_for_invalid`, 등록·대상 연결·`forwarded_ip` 없음 |
| WS-034 | 7절 / WS-R10·WS-R15 | 신뢰하지 않는 상대가 허용 주소를 주장하는 XFF 전송 | XFF 무시. 소켓 IP가 판정 기준이며 `forwarded_ip` 없음 |
| WS-035 | FR-03·FR-05·7절 / WS-R14·WS-R15·WS-R18 | TLS 종료 시험 프록시 앞에 Viewer·Server를 두고 평문 WS 리스너로 Mode II 연결. 프록시 주소는 `trusted_proxies`에 등록 | 두 역할 매칭과 양방향 전달. 클라이언트 주소는 각 전달 주소이며 역할별 `allow`에 사용. TLS 구간은 클라이언트~프록시로 표시 |
| WS-036 | 7절 / WS-R15·WS-R17 | 신뢰 프록시의 XFF 요소가 모두 `trusted_proxies`에 속함. 프록시 자신의 주소 하나인 경우와 역할 `allow`에도 그 주소가 있는 경우 포함 | HTTP 400 `forwarded_for_invalid`. 프록시 주소를 클라이언트 주소로 대신 쓰지 않음. 등록·`forwarded_ip` 없음 |
| WS-037 | FR-05·7절·AC-49 / WS-R19·WS-R22 | ticket을 켜지 않은 역할의 기본 설정에서 `binary`와 ticket 값을 함께, 헤더 없이, `binary`만 각각 전송 | 모두 2.0.0과 같이 `101`. 응답 서브프로토콜은 `binary`이며 요청 헤더가 없으면 생략. ticket 값은 응답에 없고 `ticket_accepted`도 없음 |
| WS-038 | FR-05·7절·AC-49 / WS-R20·WS-R22 | Viewer ticket을 켠 WS·WSS 리스너에 유효한 Viewer ticket(`sub`·`sx` 포함)과 `binary` 전송. 이후 같은 `rid`의 Mode II 초기 요청 | `101`, 응답 `Sec-WebSocket-Protocol: binary`. `ticket_accepted`에 `ticket_jti`·`ticket_sub`·`session_expires_at` 기록. ticket 원문은 응답·운영 로그·관리 API에 없음. 같은 중계 ID의 상대와 정상 매칭 |
| WS-039 | FR-05·7절·AC-49 / WS-R20 | ticket을 켠 역할에 헤더 없음, `binary`만, `binary` 없이 ticket만 각각 전송 | 앞 둘은 HTTP 401과 `WWW-Authenticate: VncRelayTicket`, `reason=ticket_missing`. 셋째는 HTTP 400 `subprotocol_not_supported`. 대기 등록·대상 연결·`ticket_accepted` 없음 |
| WS-040 | FR-05·7절·AC-49 / WS-R20 | `binary`와 함께 ticket 두 개, JWS 구문 오류, `alg ≠ ES256`, `kid` 없음·모름, `crit` 있음, 서명 불일치·DER 서명, 필수 클레임 누락·자료형 오류, `iss`·`aud`·`rh` 불일치(배열 `aud` 포함), `exp ≤ iat`, `exp − iat > max_lifetime_secs`, `iat > 현재 + 5`를 각각 전송 | 모두 HTTP 401과 `WWW-Authenticate: VncRelayTicket`, `reason=ticket_invalid`. 사용 기록·대기 등록·대상 연결 없음 |
| WS-041 | FR-05·7절·AC-49 / WS-R20 | 다른 검사를 통과하는 새 ticket마다 현재 시각을 `exp + 5` 직전·도달로 두고 요청. 별도로 `sx`를 현재보다 1초 늦음·같음으로 두고 요청 | `exp + 5` 직전과 `sx = 현재 + 1`은 `101`. `exp + 5` 도달과 `sx ≤ 현재`는 HTTP 401과 `WWW-Authenticate: VncRelayTicket`, `reason=ticket_expired`. `sx`에는 시계 오차를 허용하지 않음 |
| WS-042 | FR-05·7절·AC-49 / WS-R21 | 같은 ticket으로 두 번 업그레이드. 별도로 새 ticket 하나로 두 요청을 동시에 보냄. 또 업그레이드 직전 IP 재검사에서 403으로 거절한 요청의 ticket을 IP 허용 뒤 다시 사용. 사용 기록 만료 시각도 관찰 | 첫 성공 뒤 두 번째는 HTTP 401 `ticket_reused`와 `WWW-Authenticate: VncRelayTicket`. 동시에 보낸 두 요청은 하나만 `101`이고 다른 하나는 HTTP 401 `ticket_reused`. 403 요청은 사용 기록을 쓰지 않아 같은 ticket으로 다음 요청이 `101`. 기록은 `exp + 5`에 사라지며 그 뒤 요청은 `ticket_expired`로 거절 |
| WS-043 | FR-05·7절·AC-49 / WS-R21 | 재시작 뒤 다른 검사를 통과하는 ticket의 `iat`를 기동 시각 + 5 이하·초과로 각각 두고 요청 | 이하는 HTTP 401과 `WWW-Authenticate: VncRelayTicket`, `reason=ticket_issued_before_start`. 초과는 `101` |
| WS-044 | FR-05·7절·AC-49 / WS-R20·WS-R21 | Origin·서브프로토콜·ticket·업그레이드 직전 IP 재검사를 함께 어기는 요청을 조합별로 전송. 앞 검사를 하나씩 통과시킴 | [계약 1.1절](../contracts/websocket-relay-v2.md#11-업그레이드-요청) 순서로 먼저 걸린 상태 하나만 응답: Origin 403 → 서브프로토콜 400 → ticket 401 → IP 재검사 403. 거절한 요청은 ticket을 소비하지 않음 |
| WS-045 | FR-05·7절·AC-49 / WS-R20·WS-R22 | 역할마다 다른 `issuer`·공개 키로 ticket을 켬. `/relay/server`에 Server·Viewer ticket을 각각 전송하고 Viewer 역할에도 Server ticket 전송 | 자기 역할 ticket만 `101`, `ticket_accepted.role`이 요청 역할과 일치. 다른 역할 ticket은 HTTP 401과 `WWW-Authenticate: VncRelayTicket`, `reason=ticket_invalid` |
| WS-046 | FR-05·7절·AC-49 / WS-R23 | 재로드·관리 API 편집으로 `enabled`를 켜고 끔. `issuer`·`relay_host`·`max_lifetime_secs`와 공개 키 파일 내용을 교체. 이미 `101`을 보낸 연결과 사용 기록을 유지 | 새 업그레이드 요청부터 교체 값으로 판정. 이미 `101`을 보낸 연결의 중계 ID 대조와 `sx` 타이머는 이전 값 유지. 사용 기록은 설정을 끄고 켜거나 공개 키를 교체해도 유지되며, 확인을 통과하는 같은 ticket의 재사용은 401 `ticket_reused`. ticket을 끈 뒤 ticket 없는 유효 요청은 `101` |

## 8. 미결정 사항

확정 책임은 프로젝트 요구사항·운영 결정 담당자에게 있다. 외부 URL·HTTP 상태·종료 코드는 Active 계약이 기준이다. 구현과 테스트의 지원 판정은 [작업 5.9 판정표](../how-to/set-up-and-verify.md#41-작업-59-판정표)를 따른다.

접속 ticket 범위는 사용자 결정 2026-11-11(Q192~Q204·Q207·Q212·Q213)으로 채택했다. 구현·계약 테스트는 미실행(구현 전)이다. 새 시나리오는 테스트 연결 전이며 보고서에서는 `미연결`로 추적한다. 계약 테스트 계획은 [WebSocket 계약 5절](../contracts/websocket-relay-v2.md#5-예제와-계약-테스트)을 따른다.

(해결) 사용자 결정 2026-11-08(Q191)으로 채택했다: `trusted_proxies` 전체 허용 경고 코드·사건명 `trust_all_proxies`. 이름과 필드의 채택은 [운영 정보 계약](../contracts/ops-info-v2.md)의 결정 기록을 따른다. WS-028~036과 변경된 WS-012·014·022·023·024는 미실행 (구현 전)이며 계약 테스트 계획은 [WebSocket 계약 5절](../contracts/websocket-relay-v2.md#5-예제와-계약-테스트)에 있다. 후속: 구현은 커밋 `3df0693`이며, 시험은 커밋 `51bd747`의 두 OS 게이트에서 통과했다([WSS Server·프록시·최근 로그 판정표](../how-to/set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)).
WS-TBD-001은 결번이며 재사용하지 않는다. WS-TBD-002는 [WebSocket 중계 v1](../contracts/websocket-relay-v2.md) 채택(2026-10-16)으로 결정되어 4절과 계약에 반영했으며 재사용하지 않는다. WS-TBD-003은 [설정 파일 v3](../contracts/config-v4.md) 1.4절의 정규화 규칙 채택으로 결정되어 2절과 계약에 반영했으며 재사용하지 않는다. WS-TBD-006은 결정되어 1절에 반영했으며 재사용하지 않는다. WS-TBD-005는 2026-11-01에 해소되어 이 표에서 뺐다. noVNC 1.7.0을 [호환성 검증 기록](../contracts/repeater-compat-record.md) 2.4절에 고정했고(버전·해시·받은 주소·날짜), 같은 기록 3.4절에 WS·WSS 관찰 결과를 남겼다. 이 버전은 `Sec-WebSocket-Protocol` 헤더를 보내지 않았고, 이는 [WebSocket 중계 v1](../contracts/websocket-relay-v2.md) 1.2절 규칙과 같다. 재사용하지 않는다.
WS-TBD-004는 사용자 결정 2026-11-01(Q78)로 해소되어 이 절에서 뺐다. B와 M 사이에 크기 제약은 없다. 프레임 안에서 스트리밍하며 방향당 미전달 보관 바이트는 B 이하다. M은 분할 프레임을 합친 논리 메시지에 적용되며 M+1부터 1009로 닫고 추가 전달을 멈춘다. B보다 큰 유효 프레임도 수락해 스트리밍하며 전체를 저장하지 않는다. 재사용하지 않는다.

후속: 사용자 결정 2026-11-08(Q188)으로 설정 계약은 v4로 옮겼다. 위 WS-TBD-003의 v3는 당시 채택 기록이며 링크는 현재 계약을 가리킨다. 같은 방식으로, 위 WS-TBD-002·WS-TBD-005의 "WebSocket 중계 v1"은 당시 계약 이름이며 링크는 현재 계약(v2, 사용자 결정 2026-11-10)을 가리킨다.
