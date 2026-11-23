# 아키텍처 문서 목록

아키텍처 문서는 구성요소의 책임과 연결 관계를 다룬다. 코드 지도는 전체 구조를, 도메인 모델은 업무 용어와 코드 타입의 대응을, 모듈 설계는 내부를, 관점별 문서는 배포·보안 같은 특정 질문을 설명한다.

| 문서 | 답하는 질문 | 구조 시점 | 문서 상태 | 구현 대조 |
| --- | --- | --- | --- | --- |
| [코드 지도](code-map.md) | 코드를 어떤 레이어와 구성요소로 나누고, 대표 요청은 어디서 시작해 어디서 끝나는가 | Current | Draft | Verified — [대조 기록](code-map.md#대조-기록) |
| [VNC Relay 도메인 모델](domain-model.md) | 용어집의 업무 용어(중계 핵심·접근 경계·관리 기능·기록과 통계·설정과 수명·Windows 운영)는 코드의 어떤 타입·필드로 표현되고, 코드는 어디서 어떤 조건을 지키는가 | Current | Draft | Partial — [대조 기록](domain-model.md#대조-기록) |
| [xtask 개발 도구 모듈 설계](xtask.md) | 시나리오 커버리지 보고, 레이어 의존 검사, 토큰 CSS 생성과 대조, 미리보기 서버는 무엇을 읽고 어떻게 판정하거나 띄우는가 | Current | Draft | Verified — [대조 기록](xtask.md#대조-기록) |
| [설정 로딩 모듈 설계](config-loading.md) | 설정 파일을 어떻게 읽고 검증해 유효 설정이나 진단 목록을 만드는가 | Current | Draft | Verified — [대조 기록](config-loading.md#대조-기록) |
| [`serve` 시작 모듈 설계](serve-startup.md) | `serve`가 어떤 순서로 설정·계정 파일을 읽고 리스너를 바인드하며, 실패를 어떤 종료 코드와 진단으로 알리는가 | Current | Draft | Verified — [대조 기록](serve-startup.md#대조-기록) |
| [Windows 서비스 모듈 설계](windows-service.md) | 설치·해제·SCM 실행이 `serve` 검사·시작·종료와 어떻게 연결되고 실패한 등록을 어떻게 되돌리는가 | Current | Draft | 파일·심볼 대조 — [대조 기록](windows-service.md#8-코드-대조와-시험-범위) |
| [Windows 트레이 앱 모듈 설계](windows-tray.md) | UI·작업 스레드가 서비스 상태·메뉴·준비 상태 조회·로그인할 때 시작을 어떻게 나누는가 | Current | Draft | 파일·심볼 대조 — [대조 기록](windows-tray.md#코드-대조-기록) |
| [운영 로그 모듈 설계](ops-log.md) | 운영 로그 사건을 어떤 타입으로 만들고, 어떤 형식으로 어디에 쓰며, 쓰기 실패와 비밀 값을 어떻게 다루는가 | Current | Draft | Verified — [대조 기록](ops-log.md#대조-기록) |
| [TCP 진입점 모듈 설계](tcp-entry-point.md) | TCP Viewer·Server 진입점이 접속을 어떻게 받고, 데이터를 읽기 전에 출발지 IP를 어떻게 판정하며, 허용·거절·종료를 어떻게 기록하는가 | Current | Draft | Verified — [대조 기록](tcp-entry-point.md#대조-기록) |
| [WebSocket 진입점 모듈 설계](websocket-entry-point.md) | WS·WSS Viewer·Server 진입점이 접속을 어떻게 받고, WSS 협상과 제한 시간 U를 어떻게 적용하며, 업그레이드 요청을 어떤 순서로 검사해 101로 답하거나 거절하고, 101 뒤를 공통 중계로 어떻게 넘기는가 | Current | Draft | Verified — [대조 기록](websocket-entry-point.md#대조-기록) |
| [WebSocket 스트림 어댑터 모듈 설계](websocket-stream.md) | 101 뒤 바이너리 페이로드를 공통 RelayStream으로 옮기고, M·B·초기 교환·Close·Kw 진행을 어떻게 연결하는가 | Current | Draft | Verified — [대조 기록](websocket-stream.md#대조-기록) |
| [접속 ticket 모듈 설계](relay-access-ticket.md) | 선택형 접속 ticket을 어느 레이어에서 확인하고, 공개 키·사용 기록·세션 만료 시각을 누가 소유하며, 101 뒤 중계 ID 대조와 세션 만료에 어떻게 잇는가 | Current | Draft | Verified — [대조 기록](relay-access-ticket.md#구조-대조-기록) |
| [연결 상한과 시간 제한 모듈 설계](connection-limits.md) | 연결 상한 L·Lm·Lw를 어떻게 세고, TCP 진입점이 수락한 연결에 초기 교환 시간 H와 TCP keepalive를 어떻게 적용하는가 | Current | Draft | Verified — [대조 기록](connection-limits.md#대조-기록) |
| [초기 교환 처리 모듈 설계](initial-exchange.md) | 250바이트 요청과 Viewer 시작 바이트를 어떻게 처리하고, H·오류 사건·후속 바이트·B·L 자리를 어떻게 이어 관리하는가 | Current | Draft | Verified — [대조 기록](initial-exchange.md#대조-기록) |
| [대상 연결·매칭·중계 수명 모듈 설계](relay-session.md) | 유효한 PreparedConnection을 어떻게 대상 연결 또는 상대 매칭에 넘기고, B·C·W·E·G와 연결 예산을 지키며 세션 정리까지 이어 가는가 | Current | Draft | Verified — [대조 기록](relay-session.md#대조-기록) |
| [상태 리스너와 준비 상태 모듈 설계](status-listener.md) | 상태 리스너가 `/livez`·`/readyz`에 어떻게 답하고, 준비 상태를 언제 바꾸며, 상태 연결을 Lm으로 어떻게 세는가 | Current | Draft | Verified — [대조 기록](status-listener.md#대조-기록) |
| [관리자 계정 준비 CLI 모듈 설계](admin-accounts.md) | `vnc-relay admin add\|passwd\|remove\|list`가 관리자 계정 파일을 어떻게 읽고 잠금 파일로 보호해 원자적으로 바꾸며, 비밀번호를 어떻게 입력받고 해시하는가 | Current | Draft | Verified — [대조 기록](admin-accounts.md#대조-기록) |
| [내장 TLS 모듈 설계](tls.md) | PEM 인증서·개인키를 읽어 TLS 1.2·1.3 서버 설정을 만들고, 시작 때 WSS·관리 TLS 파일을 어떻게 검사하는가 | Current | Draft | Verified — [대조 기록](tls.md#대조-기록) |
| [관리 리스너 모듈 설계](management-listener.md) | 관리 리스너가 관리 HTTPS와 루프백 평문 HTTP 접속을 어떻게 받고, Lm을 어떻게 세며, TLS 1.2 미만 거절을 어떻게 기록하고, 요청 경로 순서를 어떻게 처리하는가 | Current | Draft | Verified — [대조 기록](management-listener.md#대조-기록) |
| [관리 로그인과 세션 모듈 설계](management-session.md) | 관리자 로그인이 계정과 비밀번호를 어떻게 검증하고, 실패를 계정별·출발지별로 어떻게 세어 로그인을 제한하며, 관리 세션과 CSRF 토큰을 어떻게 만들고 검사·연장·무효화하는가 | Current | Draft | Verified — [대조 기록](management-session.md#대조-기록) |
| [관리 조회 API 모듈 설계](management-query.md) | 인증된 관리자에게 서버 요약·대기 연결·활성 세션·최근 종료 목록을 어떻게 제공하고, 중계 스트림 멈춤 없이 단일 기준 시점 스냅샷을 어떻게 구성하는가 | Current | Draft | Verified — [대조 기록](management-query.md#대조-기록) |
| [관리자 종료 API 모듈 설계](management-terminate.md) | 인증된 관리자가 지정한 활성 세션만 종료하고, 중계 작업을 막지 않으며 정리 완료·경합 결과·계정명 사건을 어떻게 전달받는가 | Current | Active | Verified — [대조 기록](management-terminate.md#8-현재-구조와-검증-범위) |
| [메트릭 모듈 설계](metrics.md) | 메트릭 값을 어디에서 기록하고, 응답할 때 현재 값과 프로세스 값을 어떻게 읽으며, 추이를 어떻게 보관하고, 상태 리스너와 관리 API가 같은 값을 어떻게 내보내는가 | Current | Draft | Verified — [대조 기록](metrics.md#대조-기록) |
| [접속 출처 모듈 설계](source-stats.md) | 중계 연결 사건을 클라이언트 주소마다 어떻게 세어 보관하고, 국가·ASN을 로컬 IP 정보 DB에서 어떻게 찾으며, DB 파일을 어떻게 읽고 다시 읽고, 관리 API가 기간별 집계를 어떻게 만드는가 | Target | Draft | 미대조 |
| [대시보드 공통 컴포넌트 모듈 설계](dashboard-components.md) | 대시보드 공통 조각(버튼·입력·배지·경고·카드·지표·게이지·상단 메뉴·언어 선택·데이터 표·링크·대화상자)과 아이콘을 어떤 DOM·옵션·상태 속성으로 만들고, 색과 크기를 토큰 CSS에서 가져오는가 | Current | Draft | Verified — [대조 기록](dashboard-components.md#대조-기록) |
| [대시보드 운영 현황 앱 구조](dashboard-operations-overview.md) | 운영 현황 화면(SCR-001)의 파일 배치, 모듈 책임, 조회·갱신 흐름, 실패 처리, 로그아웃, 표시 형식을 어떻게 나누는가 | Current | Draft | Partial — [대조 기록](dashboard-operations-overview.md#대조-기록) |
| [파일 변경 감지 모듈 설계](file-watch.md) | 설정 파일과 인증서·개인키 파일의 변경을 어떻게 감지하고, 안정된 후보를 어떻게 한 번에 검증해 교체하며, 실패와 재시작 필요를 어떻게 알리는가 | Current | Draft | Verified — [대조 기록](file-watch.md#대조-기록). 편집기 공유 모드·디렉터리 삭제와 Ubuntu 알림의 미검증 범위는 해당 문서를 따른다. |
| [설정 보기·편집 모듈 설계](config-edit.md) | GET/PATCH가 실행 값·파일 상태를 읽고, 같은 TOML을 보존 편집하며, 재로드와 직렬화해 파일·실행 값·감사를 연결하는가 | Current | Active | Verified — [대조 기록](config-edit.md#8-current-구조-대조) |

- 구조 시점: 문서가 설명하는 구조가 현재 코드(`Current`), 목표(`Target`), 전환 중(`Transitional`), 과거(`Historical`) 중 무엇인지 나타낸다.
- 문서 상태: 구현 기준으로 채택했는지 나타낸다. `Draft`는 초안, `Active`는 채택되어 적용 중, `Retired`는 더 이상 쓰지 않음을 뜻한다.
- 구현 대조: `미대조` 또는 `Verified`와 [대조 기록](../AGENTS.md#검증-기록)을 적는다. 채택 여부와 동작 테스트 결과는 별도로 확인한다.

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.
