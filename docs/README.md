# vnc-relay 문서

프로젝트 문서는 답하는 질문에 따라 나눈다. 문서 유형은 아래 이름을 사용한다.

| 이름 | 디렉터리 | 답하는 질문 |
| --- | --- | --- |
| PRD(제품 요구사항 정의서) | [requirements/](requirements/README.md) | 무엇이 필요하며 왜 필요한가? |
| 스펙(외부 동작의 명세) | [specs/](specs/README.md) | 입력·상태·오류에 따라 어떤 결과가 보이는가? |
| 디자인(화면의 시각 기준과 시안) | [design/](design/README.md) | 화면은 어떤 공통 시각 기준으로 어떻게 보이는가? |
| 계약(Contract, 구성요소 사이의 교환 형식과 호환성 약속) | [contracts/](contracts/README.md) | 경계에서 무엇을 어떤 형식으로 주고받는가? |
| 아키텍처 문서 | [architectures/](architectures/README.md) | 코드는 어디에 있고 구성요소는 어떻게 연결되는가? |
| ADR(기술 결정 기록) | [decisions/](decisions/README.md) | 왜 이 선택을 했는가? |
| 해설 | [explainers/](explainers/README.md) | 핵심 개념과 원리는 어떻게 작동하며 코드에서 어떻게 드러나는가? |
| 컨벤션(코드·저장소 작성 규칙) | [conventions/](conventions/README.md) | 소스와 저장소 산출물을 어떤 규칙으로 작성하는가? |
| how-to(작업 절차) | [how-to/](how-to/README.md) | 특정 작업을 안전하게 어떻게 수행하는가? |
| 도움말(운영자용 안내 사이트) | [help/](help/README.md) | 운영자는 vnc-relay를 어떻게 설치·설정·운영하는가? |
| 사고 기록(Incident) | [incidents/](incidents/README.md) | 사고의 영향·원인·대응·후속 조치는 무엇인가? |

아키텍처 문서는 세 가지를 기본으로 둔다.

- 코드 지도: 구성요소별 코드 위치와 진입점
- 도메인 모델: 용어집의 업무 용어를 표현하는 코드 타입·파일과 그 연결
- 모듈 설계: 한 모듈의 내부 구조와 처리 순서

업무 용어는 [용어집 규칙](AGENTS.md#용어집)에 따라 관리한다. 용어집은 위 문서 유형에 포함하지 않는다.

## 처음 읽는 순서

다음 순서로 읽으면 프로젝트의 전체 기준을 파악할 수 있다. 개별 작업에서는 관련 범위만 읽는다.

1. PRD: 무엇을 왜 만드는지 이해한다.
2. [용어집](../GLOSSARY.md): 업무 용어의 뜻을 확인한다.
3. 도메인 모델: 그 용어가 코드의 어떤 타입과 파일에 있는지 확인한다.
4. 코드 지도: 구성요소·진입점·요청 흐름을 확인한다.
5. 스펙: 기능이 외부에서 어떻게 동작해야 하는지 시나리오로 확인한다.
6. 모듈 설계: 관심 있는 모듈의 내부 구조, 데이터, 처리 순서를 파일·함수 단위로 확인한다.
7. 해설: 이해하기 어려운 개념과 선택의 이유를 코드와 연결해 읽는다.

## 프로젝트 작업 안내

제품 요구사항은 [PRD.md](requirements/PRD.md)에서 관리한다. 현재 문서 상태는 초안 0.19이며, [사용자 확정 기준](requirements/PRD.md#확정-기준)과 나머지 제안·미확정 사항을 구분한다. 명령과 규칙은 연결한 문서 한 곳에서 관리한다.

| 확인할 내용 | 프로젝트의 기준 문서 |
| --- | --- |
| 외부 동작과 수용 시나리오 | [스펙 목록](specs/README.md) — Active 6건과 Draft 1건([Windows 트레이 앱](specs/windows-tray.md), 메뉴·상태 문구 미결정). 채택과 검증 완료는 구분한다. |
| 교환 형식(계약)과 기술 결정 | [계약 목록](contracts/README.md) — Active 7건. 설정은 [vnc-relay-config 4.4.0](contracts/config-v4.md), 운영 정보는 [ops-info 2.3.0](contracts/ops-info-v2.md), 메트릭은 [vnc-relay-metrics 1.2.0](contracts/metrics-v1.md), WebSocket 중계는 [websocket-relay 2.2.0](contracts/websocket-relay-v2.md), CLI는 [vnc-relay-cli 1.8.0](contracts/cli-v1.md), 관리 API는 [management-api 1.5.0](contracts/management-api-v1.md)을 따른다. Viewer 경로는 `/relay/viewer`다. [ADR 목록](decisions/README.md) — Accepted 11건·Superseded 2건. [ADR-0007](decisions/ADR-0007-proxy-tls-termination.md)은 중계 프록시 TLS 종료, [ADR-0008](decisions/ADR-0008-shared-ws-listener-path-roles.md)은 공유 리스너와 경로별 역할, [ADR-0009](decisions/ADR-0009-optional-relay-access-ticket.md)는 선택형 접속 ticket 확인, [ADR-0010](decisions/ADR-0010-windows-service-and-tray.md)은 Windows 서비스 실행과 트레이 앱, [ADR-0011](decisions/ADR-0011-pull-metrics-on-status-listener.md)은 상태 리스너의 메트릭 내보내기, [ADR-0012](decisions/ADR-0012-local-ip-info-databases.md)는 접속 출처의 국가·ASN을 찾는 로컬 IP 정보 DB, [ADR-0013](decisions/ADR-0013-relay-source-deny-list.md)은 허용 목록과 공통 차단 목록으로 하는 중계 접근 통제(ADR-0002 대체)를 정한다. |
| 코드 지도(변경 대상의 코드 위치와 연결 관계) | [코드 지도](architectures/code-map.md) — Current, Draft, Verified. 현재 코드는 `serve` 시작과 CLI 골격([`serve` 시작 모듈 설계](architectures/serve-startup.md): `src/`와 계정 파일·리스너 모듈), 설정 로더([설정 로딩 모듈 설계](architectures/config-loading.md): `crates/domain`·`crates/app`·`crates/adapters`), 운영 로그([운영 로그 모듈 설계](architectures/ops-log.md): 사건 타입·JSON Lines 출력·`relay_id_ref`), TCP 진입점([TCP 진입점 모듈 설계](architectures/tcp-entry-point.md): 접속 수락·출발지 IP 검사), 연결 상한과 시간 제한([연결 상한과 시간 제한 모듈 설계](architectures/connection-limits.md): 연결 예산·L·H·TCP keepalive), 상태 리스너와 준비 상태([상태 리스너와 준비 상태 모듈 설계](architectures/status-listener.md): `/livez`·`/readyz`·Lm·`server_ready`), 대시보드 공통 컴포넌트([대시보드 공통 컴포넌트 모듈 설계](architectures/dashboard-components.md): 조각 DOM·옵션·상태 속성과 미리보기), 개발 도구 [`xtask/`](architectures/xtask.md)다 |
| Windows 서비스·트레이 코드 위치와 동작 | [Windows 서비스 모듈 설계](architectures/windows-service.md) — `src/service.rs`·`src/serve.rs`·`crates/adapters/src/windows_service.rs`·`crates/adapters/src/windows_service/security.rs`. [Windows 트레이 앱 모듈 설계](architectures/windows-tray.md) — `crates/tray/src/`의 GUI·작업 스레드·규칙·레지스트리·메시지 루프. 둘 다 Current, Draft다. 서비스 설치·실행·해제는 [운영 스펙 3.7절](specs/operations.md#37-windows-서비스-실행)과 [CLI 계약 1.9절](contracts/cli-v1.md#19-windows-서비스-실행)을, 트레이 앱은 [Windows 트레이 앱 스펙](specs/windows-tray.md)을 따른다. 검증은 부분이며 [Windows 서비스 시험](how-to/set-up-and-verify.md#windows-서비스-시험)에 확인 범위와 남은 시험이 있다. 코드 대조와 실행 검증은 구분한다. |
| 도메인 모델(업무 용어와 코드 타입의 대응) | 없음. 업무 개념을 코드 타입으로 만들 때 [도메인 모델 작성 규칙](architectures/AGENTS.md#도메인-모델)에 따라 쓴다. |
| 적용할 컨벤션 | [컨벤션 목록](conventions/README.md) — [Rust 코드 컨벤션](conventions/rust.md)·[테스트 컨벤션](conventions/testing.md)·[프런트엔드 컨벤션](conventions/frontend.md)은 Active다. 디자인 기준은 [디자인 목록](design/README.md)의 [운영 대시보드 디자인 시스템](design/DESIGN.md)(Accepted)과 채택된 [대시보드 HTML 시안](design/mockups/dashboard.html)이다. |
| 개발 환경 준비와 실행 방법 | [개발 환경을 준비하고 변경을 검증하는 방법](how-to/set-up-and-verify.md) — Draft. Ubuntu와 Windows의 개발 환경·빌드·전체 완료 검증 절차를 다룬다. rustup 설치 단계는 두 OS 모두 미실행이다. |
| 실행파일 배포와 수동 운영 | [실행파일을 묶고 수동 운영하는 방법](how-to/operate-relay.md) — Draft. Windows 11 x64·Ubuntu 24.04 LTS x64의 release 묶음과 계정·재로드·종료 절차. 두 OS 수동 기록 전에는 검증 완료로 표시하지 않는다 |
| 버전 규칙과 릴리스 | [릴리스 묶음을 만들고 배포하는 방법](how-to/release.md) — Verified. `연.월.순서` 버전, `cargo xtask package`, GitHub Actions 검사, 태그로 GitHub Release 내기를 다룬다. 확인 범위와 미실행 단계는 본문 검증 기록을 따른다. |
| 변경 범위별 테스트와 전체 완료 검증 | [개발 환경·검증 how-to 3.3·3.4절](how-to/set-up-and-verify.md#33-변경-범위별-검증). 시나리오 표식과 보고 명령은 [테스트 컨벤션](conventions/testing.md)이 정한다. 현재 결과는 `cargo xtask scenarios`의 `scenario-report.md`와 CI에서 확인한다. 의존성을 바꾸면 [의존성 라이선스 검사](how-to/check-dependency-licenses.md)를 실행한다. |
| `.work/` 밖에 생기는 빌드 결과·캐시의 위치와 정리 방법 | Cargo 대상 디렉터리 `target/`(WSL은 `CARGO_TARGET_DIR`로 정한 위치)이 `.work/` 밖에 생긴다. 위치는 [개발 환경·검증 how-to 1절](how-to/set-up-and-verify.md#1-대상과-영향)과 [2절](how-to/set-up-and-verify.md#2-전제-조건)에 있다. 정리 방법은 아직 쓰지 않았다. |
| 운영자 도움말 | [도움말 목록](help/README.md)과 [도움말 작성 규칙](help/AGENTS.md). 한국어·영어 mdBook 책을 `main` 기준으로 [GitHub Pages](https://kayden-kim.github.io/vnc-relay/)에 게시한다. 사이트 화면과 동작은 [도움말 사이트 화면 스펙](specs/help-site.md)(Active)을 따른다. 빌드·검사·미리보기·게시는 [도움말 게시 how-to](how-to/publish-help.md)에 있다. |

WS·WSS는 공유 리스너에서 경로별로 Viewer·Server 역할을 나눈다. 중계 리스너 앞의 프록시 TLS 종료와 최근 로그 화면을 제공한다. 관련 수용 범위는 [PRD AC-44~48](requirements/PRD.md#10-검증-및-수용-기준)에 있다.

WS·WSS의 접속 ticket 확인은 역할별 선택 기능이며 기본으로 꺼져 있다. 형식과 확인 규칙은 [WebSocket 계약 1.9절](contracts/websocket-relay-v2.md#19-접속-ticket)에, 설정 키와 재로드 44개·편집 38개 키의 범위는 [설정 계약](contracts/config-v4.md)에 있다. 대시보드는 ticket 키 편집 입력과 `session_expired` 종료 원인 표시를 제공한다. 관련 수용 범위는 [PRD AC-49~52](requirements/PRD.md#10-검증-및-수용-기준)다.

운영자 도움말과 대시보드 도움말 링크의 수용 범위는 [PRD AC-53~55](requirements/PRD.md#10-검증-및-수용-기준), Windows 서비스·트레이 앱은 [PRD AC-57~59](requirements/PRD.md#10-검증-및-수용-기준)를 따른다. 각 범위와 스펙의 연결은 [스펙 추적](specs/README.md#적용-범위와-추적)에서 확인한다.

프런트엔드 프로젝트는 위 표에 실제 디자인 기준·컨벤션·검증 절차를 연결한다. 작업은 `frontend-workflow` 스킬을 따른다.

검증 절차에는 환경·버전·실행 위치, 변경 범위별 명령, 전체 완료 명령과 성공 조건을 적는다. 결과 기록은 [검증 규칙](AGENTS.md#검증-기록)을 따른다.

작업 순서는 루트 [작업 지침](../AGENTS.md#작업-순서와-필수-참조), 문서 채택 여부와 충돌 판단은 [기준 문서 적용과 충돌 처리](AGENTS.md#기준-문서-적용과-충돌-처리)를 따른다.

## 변경에 따른 문서 선택

기존 문서로 다룰 수 있으면 갱신한다. 독립된 질문이 생길 때만 문서를 추가한다. 여러 조건에 해당하면 관련 유형을 함께 확인한다.

| 변경 상황 | 우선 확인하고 필요한 경우 갱신할 문서 |
| --- | --- |
| 사용자에게 보이는 동작 추가·변경 | PRD·스펙. 화면 문서의 분리 기준은 [스펙 규칙](specs/AGENTS.md)을 따른다. 화면 스펙에 HTML 시안이 있으면 함께 갱신한다. 시안 규칙은 [디자인 규칙](design/AGENTS.md)을 따른다. |
| 디자인 시스템·토큰 사용 규칙 변경 | 디자인 시스템과 프런트엔드 컨벤션. 소유 범위는 [디자인 규칙](design/AGENTS.md)과 [컨벤션 규칙](conventions/AGENTS.md)을 따른다. 영향받는 HTML 시안도 함께 갱신한다. 시안은 DESIGN.md 토큰만 쓴다. |
| 기존 요구사항과 다른 동작을 고치는 버그 수정 | 기존 스펙과 해당 시나리오의 테스트. 증거는 [검증 기록](AGENTS.md#검증-기록)에 따른 커버리지·결과 보고서를 확인한다. 요구사항 자체가 달라지지 않으면 새로 만들지 않는다. |
| API·메시지·파일의 교환 형식이나 호환성 변경 | 계약과 관련 스펙 |
| 모듈·파일 추가, 호출 경로·진입점 변경 | 코드 지도 |
| 모듈 내부 구조·데이터 모델·처리 순서·초기화 시점 변경 | 모듈 설계 |
| 구성요소의 책임·의존 관계·배포 구조 변경 | 코드 지도와 관련 아키텍처 문서 |
| 중요한 설계 선택 | [ADR 작성 조건](decisions/AGENTS.md)을 모두 만족하면 ADR과 영향받는 기준 문서 |
| 이름 짓기·서식·코드 작성 방식 변경 | 컨벤션 |
| 반복할 개발·검증·운영 작업 정리 | how-to |
| 사고의 영향·원인·대응·후속 조치 기록 | 사고 기록과 후속 조치로 바뀌는 기준 문서 |
| 업무 용어를 새로 정하거나 이름·뜻을 변경 | 해당 맥락의 용어집과 그 용어를 쓰는 문서. 맥락 폴더를 추가·이동하면 `GLOSSARY-MAP.md` |
| 업무 개념 사이의 소속·개수 변경 | 스펙의 「용어·관계·상태」 절을 먼저 고치고 도메인 모델에 반영 |
| 업무 개념을 표현하는 코드 타입의 추가·이름 변경·삭제, 필드·관계·조건 변경 | 도메인 모델. 새 용어나 뜻 변경이 있으면 용어집도 |
| 이해하기 어려운 원리나 구현 설명 | 해설 |

영향이 없는 유형은 빈 문서를 미리 만들지 않는다.

## 문서 목록 관리

[문서 작성과 목록 갱신](AGENTS.md#문서-작성과-목록-갱신) 규칙을 따른다.
