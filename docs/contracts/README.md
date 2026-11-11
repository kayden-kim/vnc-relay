# 계약 목록

계약(Contract)은 구성요소가 주고받는 데이터 형식과 버전 호환성을 정한다.

| 계약 | 상태 | 만드는 쪽 → 사용하는 쪽 | 기준 형식 파일 | 관련 스펙 |
| --- | --- | --- | --- | --- |
| [설정 파일 v4(vnc-relay-config 4.1.0)](config-v4.md) | Active(공유 리스너·역할 표: 사용자 결정 2026-11-08(Q188), 접속 ticket 표: 사용자 결정 2026-11-11(Q197·Q200·Q212·Q213), 재로드 43개·편집 37개: Q184·Q188·Q200) | 운영자·관리 API 편집 → `vnc-relay serve` | 없음(Markdown 표) | [운영](../specs/operations.md), [WS·WSS](../specs/websocket.md), [관리](../specs/management.md), [화면](../specs/dashboard.md), [중계](../specs/relay.md) |
| [CLI와 관리자 계정 파일 v1](cli-v1.md) | Active | `vnc-relay` 실행파일 → 운영자·스크립트, `vnc-relay serve`(계정 파일) | 없음(Markdown 표) | [운영](../specs/operations.md) |
| [관리 API v1](management-api-v1.md) | Active | 관리 리스너(서버) → 내장 대시보드, 외부 HTTP 클라이언트 | 없음(Markdown 표) | [관리](../specs/management.md), [화면](../specs/dashboard.md) |
| [WebSocket 중계 v2(websocket-relay 2.1.0)](websocket-relay-v2.md) | Active(Viewer 경로 변경: 사용자 결정 2026-11-10, 접속 ticket: 사용자 결정 2026-11-11(Q192·Q195)) | 외부 WebSocket 클라이언트 ↔ `vnc-relay` WS·WSS 리스너 | 없음(Markdown 표) | [WS·WSS](../specs/websocket.md), [중계](../specs/relay.md) |
| [Repeater 초기 교환 v1](repeater-handshake-v1.md) | Active | UltraVNC Viewer·Server, 외부 WebSocket 클라이언트 → `vnc-relay` 진입점(Viewer 쪽 시작 바이트는 반대 방향) | 없음(Markdown 표) | [중계](../specs/relay.md), [WS·WSS](../specs/websocket.md) |
| [운영 로그와 상태 엔드포인트 v2(ops-info 2.1.0)](ops-info-v2.md) | Active(리스너·역할 의미 변경: 사용자 결정 2026-11-08(Q188), 접속 ticket 사건·경고 추가: 사용자 결정 2026-11-11(Q198·Q199·Q213)) | `vnc-relay serve` → 운영자의 로그 수집·상태 확인 도구, 관리 API·내장 대시보드 | 없음(Markdown 표) | [운영](../specs/operations.md), [관리](../specs/management.md), [화면](../specs/dashboard.md), [WS·WSS](../specs/websocket.md), [중계](../specs/relay.md) |

설정 v4, ops-info v2, WebSocket 중계 v2는 호환성을 깨는 변경이다. 이전·롤백은 각 계약 3절을 따른다. 사용자 지시 2026-11-08로 더한 WS·WSS Server·프록시 TLS 종료·최근 로그의 구현은 커밋 `3df0693`이다. 시험은 커밋 `51bd747`의 두 OS 게이트에서 통과했다([WSS Server·프록시·최근 로그 판정표](../how-to/set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)). `trust_all_proxies`, 최근 로그 API의 질의·응답 필드 이름과 세션 필터 URL은 사용자 결정 2026-11-08(Q191)으로 채택했다. 이름과 필드의 기준은 관련 계약을 따른다.

후속: 2026-11-11. 사용자 결정 2026-11-11(Q192~Q214)로 WS·WSS 역할의 선택형 접속 ticket 확인을 더했다. 설정 4.1.0, WebSocket 중계 2.1.0, ops-info 2.1.0, CLI 계약 1.7.0은 각 계약 3절의 호환 변경이다. 관리 API는 같은 절의 허용 값 추가 규칙에 따라 1.1.0을 유지한다. 기본값(`enabled = false`)에서는 기존 설정 파일과 클라이언트의 판정이 바뀌지 않는다. 결정 이유는 [ADR-0009](../decisions/ADR-0009-optional-relay-access-ticket.md)에 있다. 구현은 커밋 `4be8e0e`이며, 계약 테스트는 같은 커밋의 두 OS 전체 게이트에서 통과했다([접속 ticket 판정표](../how-to/set-up-and-verify.md#45-접속-ticket-판정표)).

## 호환성 검증 기록

[Repeater·UltraVNC·noVNC 호환성 검증 기록](repeater-compat-record.md)은 관찰 허용 검토 결과, 검증 대상 버전과 배포본 해시, 관찰 증거, 모드·전송 조합 검증 결과를 남긴다. 관찰 증거는 2026-11-01에 기록했고(TODO 1.4), 모드·전송 조합 검증은 2026-11-06에 기록했다(TODO 5.5).

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.
