# 해설 목록

해설은 개념·정상 흐름·실패 흐름·선택의 이유를 실제 코드와 연결한다. 요구사항이나 설계를 새로 정하지 않는다.

| 문서 | 답하는 질문 | 주요 관련 문서 |
| --- | --- | --- |
| [중계 접속 ticket 해설](relay-access-ticket.html) | 철회한 ticket 확인을 vnc-relay 안에 넣으면 검사 순서·사용 기록·101 뒤 동작은 어떻게 바뀌는가? 기본으로 꺼 두고 역할마다 설정으로 켜면 무엇이 그대로이고, 어디에 ticket 없는 길이 남는가? 같은 확인을 기기 쪽 Server 역할로 넓히려면 droidVNC-NG 기기에 무엇이 더 필요한가? 무엇을 더 막고 무엇은 여전히 막지 못하며, 어떤 기준을 다시 정해야 하는가? | [PRD 확정 기준](../requirements/PRD.md#확정-기준), [ADR-0002](../decisions/ADR-0002-source-ip-allowlist.md), [ADR-0003](../decisions/ADR-0003-single-instance-memory-state.md), [WebSocket 중계 계약 v2](../contracts/websocket-relay-v2.md), [설정 계약 v4](../contracts/config-v4.md), [중계와 매칭 스펙](../specs/relay.md), [WebSocket 진입점 모듈 설계](../architectures/websocket-entry-point.md) |

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.
