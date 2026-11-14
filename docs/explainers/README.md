# 해설 목록

해설은 개념·정상 흐름·실패 흐름·선택의 이유를 실제 코드와 연결한다. 요구사항이나 설계를 새로 정하지 않는다.

| 문서 | 답하는 질문 | 주요 관련 문서 |
| --- | --- | --- |
| [중계 접속 ticket 해설](relay-access-ticket.html) | 역할의 ticket 확인을 켜거나 끄면 무엇이 달라지는가? 검사 순서·사용 기록·재시작·101 뒤 ID 대조와 만료는 어떻게 이어지는가? Server 등록을 따로 확인해야 하는 이유와 TCP만 쓰는 기기에 필요한 전송 기능은 무엇인가? ticket이 보호하지 않는 범위는 어디인가? | [PRD 확정 기준](../requirements/PRD.md#확정-기준), [ADR-0009](../decisions/ADR-0009-optional-relay-access-ticket.md), [접속 ticket 계약 §1.9](../contracts/websocket-relay-v2.md#19-접속-ticket), [설정 계약 v4](../contracts/config-v4.md), [중계와 매칭 스펙](../specs/relay.md), [접속 ticket 모듈 설계](../architectures/relay-access-ticket.md), [운영 안내 §3.10](../how-to/operate-relay.md#310-접속-ticket-켜기와-공개-키-교체), [시험용 키와 ticket §3.10.1](../how-to/operate-relay.md#3101-시험용-키와-ticket으로-확인하기) |

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.
