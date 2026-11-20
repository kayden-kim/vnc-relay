# 스펙 목록

스펙은 시스템 밖에서 관찰할 수 있는 동작, 상태 전이, 오류와 보장 범위를 정한다.

| 문서 | 상태 | 외부 동작 | 관련 요구사항 |
| --- | --- | --- | --- |
| [TCP 중계와 연결 매칭](relay.md) | Active | 초기 교환, 진입점별 IP 검사, Mode I·II, 바이트 보존, 끊긴 연결 감지, 대기·종료. WS·WSS Server도 같은 Mode II·중복 ID 규칙 적용. 접속 ticket 연결의 중계 ID 대조와 세션 만료 | FR-01~04·FR-05 |
| [WS·WSS 전송](websocket.md) | Active | 공유 리스너 경로별 Viewer·Server, 수락 합집합·업그레이드 역할별 IP·Origin 검사, 역할별 선택형 접속 ticket 확인, 내장 TLS와 중계 프록시 TLS 종료, 프레임·오류·종료 | FR-05, 보안 |
| [관리 조회와 세션 종료](management.md) | Active | 내장 로그인·관리 세션·실패 제한, 설정 보기·편집, 조회·전송량·양쪽 전송 경로·전달 주소, 최근 종료 목록·최근 로그, 메트릭·접속 출처 조회, 정리 후 응답하는 관리자 종료 | FR-06·FR-08 |
| [운영 대시보드 화면](dashboard.md) | Active | 정적 HTML·JS, 데스크톱·모바일, 로그인·현황·상세·설정 보기·편집·SCR-006 로그·SCR-007 메트릭·SCR-008 접속 출처, 폴링·필터·검색·JSON Lines 내려받기 | FR-06 |
| [설정·프로세스 수명·운영 보안](operations.md) | Active | 로컬 계정 준비, 단일 인스턴스, 상태·연결 상한, 설정 v4 이전, 43개 키 재로드·37개 키 편집, 접속 ticket 설정 검증·경고, 최근 로그 보관, 메트릭, IP 정보 DB 읽기와 접속 출처 통계, ops-info 2.2.0, 시작·종료·부하·배포, Windows 서비스 설치·실행·해제 | FR-06~08·FR-10, PRD 7~9절 |
| [도움말 사이트 화면](help-site.md) | Active | GitHub Pages의 한국어·영어 도움말, 언어 선택·목차·검색·언어 전환·코드 복사·이전과 다음·없는 페이지 | FR-09 |
| [Windows 트레이 앱](windows-tray.md) | Draft | 알림 영역 아이콘과 메뉴, 서비스 상태 표시와 갱신, 시작·중지, 대시보드 열기, 로그·설정 파일 위치 열기, 로그인할 때 시작, 메뉴 언어 | FR-10 |

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.

## 적용 범위와 추적

- [PRD 확정 기준](../requirements/PRD.md#확정-기준)을 적용한다. 각 스펙의 상태와 적용 범위를 구분하며, 채택 여부를 검증 완료로 간주하지 않는다.
- 각 문서의 규칙 ID는 독립적인 동작을, 시나리오 ID는 그 동작의 수용 조건을 가리킨다. PRD FR ID와 AC ID는 유지한다.
- [PRD의 추적 표](../requirements/PRD.md#14-관련-스펙과-추적)가 기능·보안·비기능 기준을 스펙과 시나리오에 연결한다.
- 업무 용어는 루트 [용어집](../../GLOSSARY.md)을 따른다.
- 수용 테스트 표는 시나리오와 기대 결과만 정의한다. 시나리오와 테스트의 연결은 테스트 소스 코드의 메타 정보로, 커버리지·결과는 메타 정보에서 생성한 보고서로 관리한다([검증 기록](../AGENTS.md#검증-기록)). 메타 정보 문법과 보고 명령은 [테스트 컨벤션](../conventions/testing.md)이 정한다. 보고서는 `cargo xtask scenarios`가 Cargo 대상 디렉터리의 `scenario-report.md`로 만든다. 시나리오별 연결과 결과는 이 보고서에서 확인하며 이 문서에는 적지 않는다. 테스트와 연결되지 않은 시나리오는 보고서에 `미연결`로 나온다. 문서 검증을 통신·UI·호환성 테스트 통과로 간주하지 않는다.
- 교환 계약 7건은 [계약 목록](../contracts/README.md)에 Active로 등록되어 있다. 정확한 형식과 호환성은 각 계약이 정한다. 계약 테스트의 연결과 실행 방법도 각 계약에서 관리한다. 기본값·유효 범위·기본 포트는 [설정 계약](../contracts/config-v4.md), Rust 라이브러리는 [PRD 확정 기준](../requirements/PRD.md#확정-기준)의 "구현 기술" 행이 기준이다. rustls 암호 모듈은 aws-lc-rs이며 허용 암호 스위트는 그 모듈의 rustls 기본 목록이다. 시각 기준은 [디자인 기준](../design/DESIGN.md)을 따른다.

- 현재 계약 버전은 [vnc-relay-config 4.3.0](../contracts/config-v4.md), [websocket-relay 2.1.0](../contracts/websocket-relay-v2.md), [ops-info 2.2.0](../contracts/ops-info-v2.md), [management-api 1.4.0](../contracts/management-api-v1.md), [vnc-relay-metrics 1.1.0](../contracts/metrics-v1.md)이다. v4 서버는 v3 설정 파일을 거절한다. 설정 이전·롤백은 [설정 계약 3.3절](../contracts/config-v4.md#33-v3-파일을-v4로-옮기기)을 따른다. 고정 상호운용 대상과 관찰 결과는 [호환성 검증 기록](../contracts/repeater-compat-record.md), 성능 합격 기준은 [운영 스펙 6절](operations.md#6-비기능호환성출처-수용-조건), 출처·MIT 고지는 [라이선스 검토 기록](../how-to/check-dependency-licenses.md)을 따른다.

| 수용 범위 | 시나리오 | PRD 연결 |
| --- | --- | --- |
| 공유 리스너·Server·신뢰 프록시 | [WS-028~036](websocket.md#7-수용-테스트), [RLY-028](relay.md#7-수용-테스트) | AC-44~45 |
| 설정 v4·신뢰 목록 재로드·최근 로그 보관·로그 v2 | [OPS-065~069](operations.md#7-수용-테스트) | AC-13·AC-45~47 |
| 최근 로그 API·양쪽 전송 경로·전달 주소 | [MGT-046~051](management.md#7-수용-테스트) | AC-45~46·AC-48 |
| 최근 로그 화면·기존 화면의 양쪽 전송 경로 | [DASH-054~063](dashboard.md#7-수용-테스트) | AC-46·AC-48 |
| 역할별 접속 ticket 확인 | [WS-037~046](websocket.md#7-수용-테스트) | AC-49 |
| ticket 연결의 중계 ID 대조와 세션 만료 | [RLY-029~032](relay.md#7-수용-테스트) | AC-50 |
| ticket 설정 검증·경고·재로드·비노출 | [OPS-070~073](operations.md#7-수용-테스트) | AC-51 |
| 종료 원인 `session_expired`와 ticket 설정 보기·편집 | [MGT-052~053](management.md#7-수용-테스트) | AC-52 |
| 도움말 사이트 | [HELP-001~012·014~018](help-site.md#7-수용-테스트) | AC-53·AC-55 |
| 대시보드 도움말 링크 | [DASH-064](dashboard.md#7-수용-테스트) | AC-54 |
| 대기·최근 종료 목록의 주소·전송 경로·지속 시간 API | [MGT-054~055](management.md#7-수용-테스트) | AC-56 |
| 대기·최근 종료 목록의 열과 세션 로그 링크 | [DASH-065~066](dashboard.md#7-수용-테스트) | AC-56 |
| 접속 출처: IP 정보 DB 읽기·통계 API·화면 | [OPS-095~099](operations.md#7-수용-테스트), [MGT-062~064](management.md#7-수용-테스트), [DASH-082~088](dashboard.md#7-수용-테스트) | AC-63~65 |

Windows 서비스 명령은 [CLI 계약 1.9절](../contracts/cli-v1.md#19-windows-서비스-실행), 서비스 동작은 [운영 스펙 3.7절](operations.md#37-windows-서비스-실행), 트레이 동작은 [Windows 트레이 앱 스펙](windows-tray.md)을 따른다. 수동 확인 범위와 남은 시험은 [Windows 서비스 시험](../how-to/set-up-and-verify.md#windows-서비스-시험)에 있다. 트레이 앱 스펙은 메뉴·상태 문구가 미결정이므로 Draft다.

| 수용 범위 | 시나리오 | PRD 연결 |
| --- | --- | --- |
| 서비스 설치·해제·설정 검사·명령 오용 | [OPS-074~075](operations.md#7-수용-테스트), [OPS-080~081](operations.md#7-수용-테스트) | AC-57 |
| 서비스 시작·중지·시스템 종료·시작·중지 권한 | [OPS-076~079](operations.md#7-수용-테스트) | AC-58 |
| 트레이 앱 | [TRAY-001~013](windows-tray.md#7-수용-테스트) | AC-59 |

## 문서 검증 기록

- 확인일: 2026-11-19.
- 대상: 커밋 `c0fa253` 위의 문서 정리 작업 트리. `docs/` 아래 문서와 루트 `README.md`를 최종 형상 기준으로 정리한 상태다.
- 환경: Windows 11 Pro 10.0.26200, Python 3.14.6. 표준 라이브러리 정규식으로 메모리에서 검사했다.

| 확인 대상 | 방법 | 결과 | 관찰·제한 |
| --- | --- | --- | --- |
| 로컬 링크 | 루트와 `docs/` 아래 Markdown 130개(템플릿 15개 포함)의 파일·제목 링크를 GitHub 제목 규칙으로 검사했다. `docs/` 아래 HTML 3개의 상대 링크와 같은 페이지 앵커도 검사했다 | 통과 | 로컬 링크 3299개(제목 앵커 1882개) 가운데 끊긴 링크는 도움말 `notices.md`의 `../assets/fonts/LICENSE.txt` 2개다. 이 경로는 빌드한 사이트에만 있으며 `cargo xtask help-site` 검사는 통과했다. 시안 `help-site.html`의 `#hlp001-shortcuts-*-bg` 앵커 30개는 이 정리 전부터 대상 id가 없다 |
| ID 정의와 참조 | 표 첫 열의 ID 정의를 정리 전 작업 트리와 비교했다 | 통과 | 정의 ID는 698개에서 673개가 됐다. 빠진 ID는 해결한 미결정 항목과 도움말의 HELP-013·HCP-002뿐이다. 정의 없이 남은 참조는 ADR-0010의 `TRAY-TBD-002` 하나이며, ADR은 결정 당시 기록으로 둔다 |
| 수용 시나리오 | 스펙 7개의 수용 테스트 표 첫 열을 셌다 | 통과 | 화면 67, 도움말 17, 관리 55, 운영 80, 중계 32, WebSocket 46, 트레이 13, 합계 310개다. 도움말은 HELP-013을 폐기해 18개에서 17개가 됐고 나머지는 같다 |
| 이력 표현 | 결정 인용(`사용자 결정`, 질문 번호)·작업 번호·커밋 해시를 검색했다. ADR·템플릿·사고 기록·작업 지침은 제외했다 | 통과 | 결정 인용과 작업 번호는 0건이다. 커밋 해시는 검증·대조 기록 안에만 남았다 |

이 기록은 문서 구조 확인 결과다. 통신·UI·호환성 테스트의 실행 결과를 뜻하지 않는다. 시나리오별 테스트 연결과 실행 결과는 생성 보고서에서 확인한다.

