# 해설 목록

해설은 개념·정상 흐름·실패 흐름·선택의 이유를 실제 코드와 연결한다. 요구사항이나 설계를 새로 정하지 않는다.

| 문서 | 답하는 질문 | 주요 관련 문서 |
| --- | --- | --- |
| [중계 접속 ticket 해설](relay-access-ticket.html) | vnc-relay가 ticket을 확인할 때 검사 순서·사용 기록·101 뒤 동작은 어떻게 이어지는가? 역할마다 기본으로 꺼 두면 무엇이 그대로이고, 어디에 ticket 없는 길이 남는가? Server 역할에 켜려면 droidVNC-NG 기기에 무엇이 더 필요한가? 채택으로 어떤 기준이 바뀌었고, 무엇은 여전히 막지 못하는가? | [PRD 확정 기준](../requirements/PRD.md#확정-기준), [ADR-0002](../decisions/ADR-0002-source-ip-allowlist.md), [ADR-0003](../decisions/ADR-0003-single-instance-memory-state.md), [ADR-0009](../decisions/ADR-0009-optional-relay-access-ticket.md), [WebSocket 중계 계약 v2](../contracts/websocket-relay-v2.md), [접속 ticket 계약 §1.9](../contracts/websocket-relay-v2.md#19-접속-ticket), [설정 계약 v4](../contracts/config-v4.md), [중계와 매칭 스펙](../specs/relay.md), [WebSocket 진입점 모듈 설계](../architectures/websocket-entry-point.md) |

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.
