# 계약 목록

계약(Contract)은 구성요소가 주고받는 데이터 형식과 버전 호환성을 정한다.

| 계약 | 상태 | 만드는 쪽 → 사용하는 쪽 | 기준 형식 파일 | 관련 스펙 |
| --- | --- | --- | --- | --- |
| [설정 파일 v4(vnc-relay-config 4.2.0)](config-v4.md) | Active | 운영자·관리 API 편집 → `vnc-relay serve`·Windows 서비스(`service install`·`service run`), Windows 트레이 앱(키 4개만) | 없음(Markdown 표) | [운영](../specs/operations.md), [WS·WSS](../specs/websocket.md), [관리](../specs/management.md), [화면](../specs/dashboard.md), [중계](../specs/relay.md), [Windows 트레이 앱](../specs/windows-tray.md) |
| [CLI와 관리자 계정 파일 v1(vnc-relay-cli 1.8.0)](cli-v1.md) | Active | `vnc-relay` 실행파일 → 운영자·스크립트·Windows SCM·Windows 트레이 앱(서비스 이름·등록 명령), `vnc-relay serve`(계정 파일) | 없음(Markdown 표) | [운영](../specs/operations.md), [Windows 트레이 앱](../specs/windows-tray.md) |
| [관리 API v1(management-api 1.2.0)](management-api-v1.md) | Active | 관리 리스너(서버) → 내장 대시보드, 외부 HTTP 클라이언트 | 없음(Markdown 표) | [관리](../specs/management.md), [화면](../specs/dashboard.md) |
| [WebSocket 중계 v2(websocket-relay 2.1.0)](websocket-relay-v2.md) | Active | 외부 WebSocket 클라이언트 ↔ `vnc-relay` WS·WSS 리스너 | 없음(Markdown 표) | [WS·WSS](../specs/websocket.md), [중계](../specs/relay.md) |
| [Repeater 초기 교환 v1](repeater-handshake-v1.md) | Active | UltraVNC Viewer·Server, 외부 WebSocket 클라이언트 → `vnc-relay` 진입점(Viewer 쪽 시작 바이트는 반대 방향) | 없음(Markdown 표) | [중계](../specs/relay.md), [WS·WSS](../specs/websocket.md) |
| [운영 로그와 상태 엔드포인트 v2(ops-info 2.1.0)](ops-info-v2.md) | Active | `vnc-relay serve`·Windows 서비스 → 운영자의 로그 수집·상태 확인 도구, 관리 API·내장 대시보드, Windows 트레이 앱(`/readyz`만) | 없음(Markdown 표) | [운영](../specs/operations.md), [관리](../specs/management.md), [화면](../specs/dashboard.md), [WS·WSS](../specs/websocket.md), [중계](../specs/relay.md), [Windows 트레이 앱](../specs/windows-tray.md) |
| [메트릭 v1(vnc-relay-metrics 1.0.0)](metrics-v1.md) | Active | `vnc-relay serve`·Windows 서비스 → 운영자의 메트릭 수집 도구(상태 리스너 `/metrics`), 관리 API·내장 대시보드 | 없음(Markdown 표) | [운영](../specs/operations.md), [관리](../specs/management.md), [화면](../specs/dashboard.md) |

설정 v4, ops-info v2, WebSocket 중계 v2는 호환성을 깨는 변경이다. 이전·롤백은 각 계약 3절을 따른다. `trust_all_proxies`, 최근 로그 API의 질의·응답 필드 이름과 세션 필터 URL은 관련 계약이 정한다.

WS·WSS 역할의 선택형 접속 ticket 확인은 기본으로 꺼져 있다. 기본값(`enabled = false`)에서는 기존 설정 파일과 클라이언트의 판정이 바뀌지 않는다. ticket을 다루는 설정 4.1.0, WebSocket 중계 2.1.0, ops-info 2.1.0, CLI 계약 1.7.0은 각 계약 3절의 호환 변경이다. 관리 API는 허용 값 추가 규칙에 따라 1.1.0을 사용한다. 결정 이유는 [ADR-0009](../decisions/ADR-0009-optional-relay-access-ticket.md)에 있다.

Windows 서비스 실행은 CLI 계약 1.8.0의 `service install`·`service uninstall`·`service run`과 서비스 오류 코드를 따른다. 이 명령과 오류 코드는 CLI 계약 3절의 호환 변경이다. 서비스·트레이에 별도 사건·필드·값은 없으며 ops-info는 2.1.0을 사용한다. 서비스 실행에서는 설정 계약의 `log.output`이 파일 경로여야 한다. 결정 이유는 [ADR-0010](../decisions/ADR-0010-windows-service-and-tray.md)에 있다. 시나리오 연결은 테스트 소스의 메타 정보로 관리한다. 현재 자동 시험 결과는 [테스트 컨벤션](../conventions/testing.md)의 `cargo xtask scenarios`와 `scenario-report.md`에서 확인한다. 수동 확인 범위와 남은 시험은 [Windows 서비스 시험](../how-to/set-up-and-verify.md#windows-서비스-시험)을 따른다.

메트릭은 OpenTelemetry 의미 규칙의 이름·단위·속성을 쓰고, 상태 리스너의 `/metrics`에서 Prometheus 텍스트로 내보낸다. `/metrics`는 기본으로 꺼져 있다(`[metrics] enabled = false`). 설정 4.2.0은 이 키를 더한 호환 변경이고, 관리 API 1.2.0은 `GET /api/v1/metrics`를 더한 호환 변경이다. 로그 형식과 `/livez`·`/readyz`는 바뀌지 않으므로 ops-info는 2.1.0을 유지한다. 결정 이유는 [ADR-0011](../decisions/ADR-0011-pull-metrics-on-status-listener.md)에 있다.

## 호환성 검증 기록

[Repeater·UltraVNC·noVNC 호환성 검증 기록](repeater-compat-record.md)은 관찰 허용 검토 결과, 검증 대상 버전과 배포본 해시, 관찰 증거, 모드·전송 조합 검증 결과를 남긴다.

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.
