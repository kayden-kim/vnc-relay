# 계약 목록

계약(Contract)은 구성요소가 주고받는 데이터 형식과 버전 호환성을 정한다.

| 계약 | 상태 | 만드는 쪽 → 사용하는 쪽 | 기준 형식 파일 | 관련 스펙 |
| --- | --- | --- | --- | --- |
| [설정 파일 v4(vnc-relay-config 4.0.0)](config-v4.md) | Active(공유 리스너·역할 표: 사용자 결정 2026-11-08(Q188), 재로드 33개·편집 29개: Q184·Q188) | 운영자·관리 API 편집 → `vnc-relay serve` | 없음(Markdown 표) | [운영](../specs/operations.md), [WS·WSS](../specs/websocket.md), [관리](../specs/management.md), [화면](../specs/dashboard.md), [중계](../specs/relay.md) |
| [CLI와 관리자 계정 파일 v1](cli-v1.md) | Active | `vnc-relay` 실행파일 → 운영자·스크립트, `vnc-relay serve`(계정 파일) | 없음(Markdown 표) | [운영](../specs/operations.md) |
| [관리 API v1](management-api-v1.md) | Active | 관리 리스너(서버) → 내장 대시보드, 외부 HTTP 클라이언트 | 없음(Markdown 표) | [관리](../specs/management.md), [화면](../specs/dashboard.md) |
| [WebSocket 중계 v1](websocket-relay-v1.md) | Active | 외부 WebSocket 클라이언트 ↔ `vnc-relay` WS·WSS 리스너 | 없음(Markdown 표) | [WS·WSS](../specs/websocket.md) |
| [Repeater 초기 교환 v1](repeater-handshake-v1.md) | Active | UltraVNC Viewer·Server, 외부 WebSocket 클라이언트 → `vnc-relay` 진입점(Viewer 쪽 시작 바이트는 반대 방향) | 없음(Markdown 표) | [중계](../specs/relay.md), [WS·WSS](../specs/websocket.md) |
| [운영 로그와 상태 엔드포인트 v2(ops-info 2.0.0)](ops-info-v2.md) | Active(리스너·역할 의미 변경: 사용자 결정 2026-11-08(Q188)) | `vnc-relay serve` → 운영자의 로그 수집·상태 확인 도구, 관리 API·내장 대시보드 | 없음(Markdown 표) | [운영](../specs/operations.md), [관리](../specs/management.md), [화면](../specs/dashboard.md) |

설정 v4와 ops-info v2는 호환성을 깨는 변경이다. 이전·롤백은 각 계약 3절을 따른다. 사용자 지시 2026-11-08로 더한 WS·WSS Server·프록시 TLS 종료·최근 로그의 구현·테스트는 미실행(구현 전)이다. `trust_all_proxies`, 최근 로그 API의 질의·응답 필드 이름과 세션 필터 URL은 사용자 결정 2026-11-08(Q191)으로 채택했다. 이름과 필드의 기준은 관련 계약을 따른다.

## 호환성 검증 기록

[Repeater·UltraVNC·noVNC 호환성 검증 기록](repeater-compat-record.md)은 관찰 허용 검토 결과, 검증 대상 버전과 배포본 해시, 관찰 증거, 모드·전송 조합 검증 결과를 남긴다. 관찰 증거는 2026-11-01에 기록했고(TODO 1.4), 모드·전송 조합 검증은 2026-11-06에 기록했다(TODO 5.5).

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.
