# ADR 목록

ADR(Architecture Decision Record, 기술 결정 기록)은 중요하고 되돌리기 어려운 기술·구조 선택과 당시의 근거를 남기는 문서다.

| ADR | 상태 | 결정 |
| --- | --- | --- |
| [ADR-0001](ADR-0001-built-in-tls.md) | Superseded by ADR-0007 | WSS·관리 HTTPS의 내장 TLS에 관한 결정이다. 현재 TLS 종료 기준은 ADR-0007이다. |
| [ADR-0002](ADR-0002-source-ip-allowlist.md) | Accepted | 진입점별 출발지 IP 허용 목록. 리스너·역할별 검사와 신뢰 프록시 전달 주소를 연결한다. |
| [ADR-0003](ADR-0003-single-instance-memory-state.md) | Accepted | 단일 인스턴스로 운영하고 상태를 메모리에만 둔다. |
| [ADR-0004](ADR-0004-tokio-rustls.md) | Accepted | 비동기 런타임은 tokio, TLS는 rustls를 쓴다. |
| [ADR-0005](ADR-0005-in-house-ws-streaming-frame-codec.md) | Accepted | WebSocket 데이터 프레임 처리는 자체 스트리밍 상태 머신을 쓴다. |
| [ADR-0006](ADR-0006-config-edit-through-file.md) | Accepted | 대시보드 편집을 같은 설정 파일에 서식을 보존해 기록하고 같은 재로드 규칙으로 적용한다. |
| [ADR-0007](ADR-0007-proxy-tls-termination.md) | Accepted | 내장 TLS를 유지하고 중계 WS·WSS 앞의 프록시 TLS 종료와 신뢰 프록시 전달 주소를 지원한다. 관리 리스너는 제외한다. |
| [ADR-0008](ADR-0008-shared-ws-listener-path-roles.md) | Accepted | WS·WSS 리스너를 공유하고 경로로 Viewer·Server 역할을 고른다. 설정 v4·ops-info v2를 따른다. |
| [ADR-0009](ADR-0009-optional-relay-access-ticket.md) | Accepted | 진입점별 출발지 IP 허용 목록과 함께 WS·WSS 역할별 선택형 접속 ticket 확인을 제공한다. TCP에는 적용하지 않는다. |
| [ADR-0010](ADR-0010-windows-service-and-tray.md) | Accepted | Windows 11에서 같은 실행파일의 `service` 하위 명령으로 서비스를 설치·실행한다. 별도 GUI 실행파일인 트레이 앱으로 상태 확인과 시작·중지를 한다. Ubuntu는 수동 실행한다. |

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.
