# 계약 목록

계약(Contract)은 구성요소가 주고받는 데이터 형식과 버전 호환성을 정한다.

| 계약 | 상태 | 만드는 쪽 → 사용하는 쪽 | 기준 형식 파일 | 관련 스펙 |
| --- | --- | --- | --- | --- |
| [설정 파일 v1](config-v1.md) | Active | 운영자 → `vnc-relay serve` | 없음(Markdown 표) | [운영](../specs/operations.md), [WS·WSS](../specs/websocket.md), [관리](../specs/management.md), [화면](../specs/dashboard.md), [중계](../specs/relay.md) |
| [CLI와 관리자 계정 파일 v1](cli-v1.md) | Active | `vnc-relay` 실행파일 → 운영자·스크립트, `vnc-relay serve`(계정 파일) | 없음(Markdown 표) | [운영](../specs/operations.md) |
| [관리 API v1](management-api-v1.md) | Active | 관리 리스너(서버) → 내장 대시보드, 외부 HTTP 클라이언트 | 없음(Markdown 표) | [관리](../specs/management.md), [화면](../specs/dashboard.md) |
| [WebSocket 중계 v1](websocket-relay-v1.md) | Active | 외부 WebSocket 클라이언트 ↔ `vnc-relay` WS·WSS 리스너 | 없음(Markdown 표) | [WS·WSS](../specs/websocket.md) |
| [Repeater 초기 교환 v1](repeater-handshake-v1.md) | Draft | UltraVNC Viewer·Server, 외부 WebSocket 클라이언트 → `vnc-relay` 진입점 | 없음(Markdown 표) | [중계](../specs/relay.md), [WS·WSS](../specs/websocket.md) |
| [운영 로그와 상태 엔드포인트 v1](ops-info-v1.md) | Active | `vnc-relay serve` → 운영자의 로그 수집·상태 확인 도구 | 없음(Markdown 표) | [운영](../specs/operations.md), [관리](../specs/management.md) |

## 호환성 검증 기록

[Repeater·UltraVNC·noVNC 호환성 검증 기록](repeater-compat-record.md)은 관찰 허용 검토 결과, 검증 대상 버전과 배포본 해시, 관찰 증거를 남긴다. 관찰 증거는 2026-11-01에 기록했다(TODO 1.4).

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.
