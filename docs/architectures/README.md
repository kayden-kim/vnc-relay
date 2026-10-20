# 아키텍처 문서 목록

아키텍처 문서는 구성요소의 책임과 연결 관계를 다룬다. 코드 지도는 전체 구조를, 모듈 설계는 내부를, 관점별 문서는 배포·보안 같은 특정 질문을 설명한다.

| 문서 | 답하는 질문 | 구조 시점 | 문서 상태 | 구현 대조 |
| --- | --- | --- | --- | --- |
| [코드 지도](code-map.md) | 코드를 어떤 레이어와 구성요소로 나누고, 대표 요청은 어디서 시작해 어디서 끝나는가 | Target | Draft | 미대조 |
| [xtask 개발 도구 모듈 설계](xtask.md) | 시나리오 커버리지 보고와 레이어 의존 검사는 무엇을 읽고 어떻게 판정하는가 | Current | Draft | Verified — [대조 기록](xtask.md#대조-기록) |
| [설정 로딩 모듈 설계](config-loading.md) | 설정 파일을 어떻게 읽고 검증해 유효 설정이나 진단 목록을 만드는가 | Current | Draft | Verified — [대조 기록](config-loading.md#대조-기록) |
| [`serve` 시작 모듈 설계](serve-startup.md) | `serve`가 어떤 순서로 설정·계정 파일을 읽고 리스너를 바인드하며, 실패를 어떤 종료 코드와 진단으로 알리는가 | Current | Draft | Verified — [대조 기록](serve-startup.md#대조-기록) |
| [운영 로그 모듈 설계](ops-log.md) | 운영 로그 사건을 어떤 타입으로 만들고, 어떤 형식으로 어디에 쓰며, 쓰기 실패와 비밀 값을 어떻게 다루는가 | Current | Draft | Verified — [대조 기록](ops-log.md#대조-기록) |

- 구조 시점: 문서가 설명하는 구조가 현재 코드(`Current`), 목표(`Target`), 전환 중(`Transitional`), 과거(`Historical`) 중 무엇인지 나타낸다.
- 문서 상태: 구현 기준으로 채택했는지 나타낸다. `Draft`는 초안, `Active`는 채택되어 적용 중, `Retired`는 더 이상 쓰지 않음을 뜻한다.
- 구현 대조: `미대조` 또는 `Verified`와 [대조 기록](../AGENTS.md#검증-기록)을 적는다. 채택 여부와 동작 테스트 결과는 별도로 확인한다.

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.
