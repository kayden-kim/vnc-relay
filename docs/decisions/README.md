# ADR 목록

ADR(Architecture Decision Record, 기술 결정 기록)은 중요하고 되돌리기 어려운 기술·구조 선택과 당시의 근거를 남기는 문서다.

| ADR | 상태 | 결정 |
| --- | --- | --- |
| [ADR-0001](ADR-0001-built-in-tls.md) | Accepted | WSS·관리 HTTPS의 TLS를 서버가 직접 종료한다. |
| [ADR-0002](ADR-0002-source-ip-allowlist.md) | Accepted | 중계 접근 통제를 진입점별 출발지 IP 허용 목록으로 한다. |
| [ADR-0003](ADR-0003-single-instance-memory-state.md) | Accepted | 단일 인스턴스로 운영하고 상태를 메모리에만 둔다. |
| [ADR-0004](ADR-0004-tokio-rustls.md) | Accepted | 비동기 런타임은 tokio, TLS는 rustls를 쓴다. |

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.
