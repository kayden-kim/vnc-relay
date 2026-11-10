# 해설 목록

해설은 개념·정상 흐름·실패 흐름·선택의 이유를 실제 코드와 연결한다. 요구사항이나 설계를 새로 정하지 않는다.

| 문서 | 답하는 질문 | 주요 관련 문서 |
| --- | --- | --- |
| [중계 접속 ticket 해설](relay-access-ticket.html) | WebSocket 중계 접속에 ticket을 붙이면 누가 무엇을 확인해야 하며, 무엇을 막고 무엇을 막지 못하는가? 철회한 설계와 미승인 대안은 어떻게 다른가? | [ADR-0002](../decisions/ADR-0002-source-ip-allowlist.md), [ADR-0007](../decisions/ADR-0007-proxy-tls-termination.md), [WebSocket 중계 계약 v1](../contracts/websocket-relay-v1.md), [WebSocket 진입점 모듈 설계](../architectures/websocket-entry-point.md) |

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.
