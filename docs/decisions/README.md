# ADR 목록

ADR(Architecture Decision Record, 기술 결정 기록)은 중요하고 되돌리기 어려운 기술·구조 선택과 당시의 근거를 남기는 문서다.

| ADR | 상태 | 결정 |
| --- | --- | --- |
| [ADR-0001](ADR-0001-built-in-tls.md) | Superseded by ADR-0007(2026-11-08) | WSS·관리 HTTPS의 TLS를 서버가 직접 종료한다. 당시 결정과 검증 기록을 보존한다. |
| [ADR-0002](ADR-0002-source-ip-allowlist.md) | Accepted | 진입점별 출발지 IP 허용 목록. 2026-11-08 후속에서 리스너+역할과 신뢰 프록시 전달 주소를 연결한다. |
| [ADR-0003](ADR-0003-single-instance-memory-state.md) | Accepted | 단일 인스턴스로 운영하고 상태를 메모리에만 둔다. |
| [ADR-0004](ADR-0004-tokio-rustls.md) | Accepted | 비동기 런타임은 tokio, TLS는 rustls를 쓴다. |
| [ADR-0005](ADR-0005-in-house-ws-streaming-frame-codec.md) | Accepted | WebSocket 데이터 프레임 처리는 자체 스트리밍 상태 머신을 쓴다. |
| [ADR-0006](ADR-0006-config-edit-through-file.md) | Accepted(사용자 결정 2026-11-08(Q177)으로 전체 채택) | 대시보드 편집을 같은 설정 파일에 서식을 보존해 기록하고 같은 재로드 규칙으로 적용한다. |
| [ADR-0007](ADR-0007-proxy-tls-termination.md) | Accepted(사용자 결정 2026-11-08(Q182·Q183)) | 내장 TLS를 유지하고 중계 WS·WSS 앞의 프록시 TLS 종료와 신뢰 프록시 전달 주소를 지원한다. 관리 리스너는 제외한다. |
| [ADR-0008](ADR-0008-shared-ws-listener-path-roles.md) | Accepted(사용자 결정 2026-11-08(Q181·Q188)) | WS·WSS 리스너를 공유하고 경로로 Viewer·Server 역할을 고른다. 설정 4.0.0·ops-info 2.0.0으로 이전한다. |

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.
