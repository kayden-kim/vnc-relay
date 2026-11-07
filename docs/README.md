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
| 사고 기록(Incident) | [incidents/](incidents/README.md) | 사고의 영향·원인·대응·후속 조치는 무엇인가? |

아키텍처 문서 가운데 코드 지도는 구성요소별 코드 위치와 진입점을, 모듈 설계는 한 모듈의 내부 구조와 처리 순서를 다룬다.

업무 용어는 [용어집 규칙](AGENTS.md#용어집)에 따라 관리한다. 용어집은 위 문서 유형에 포함하지 않는다.

## 처음 읽는 순서

처음에는 다음 순서로 읽는다. 이후 작업은 관련 범위만 읽는다.

1. PRD: 무엇을 왜 만드는지 이해한다.
2. [용어집](../GLOSSARY.md): 업무 용어의 뜻을 확인한다.
3. 코드 지도: 구성요소·진입점·요청 흐름을 확인한다.
4. 스펙: 기능이 외부에서 어떻게 동작해야 하는지 시나리오로 확인한다.
5. 모듈 설계: 관심 있는 모듈의 내부 구조, 데이터, 처리 순서를 파일·함수 단위로 확인한다.
6. 해설: 이해하기 어려운 개념과 선택의 이유를 코드와 연결해 읽는다.

## 프로젝트 작업 안내

제품 요구사항은 [PRD.md](requirements/PRD.md)에서 관리한다. 현재 문서 상태는 초안 0.4이며, [사용자 확정 기준](requirements/PRD.md#확정-기준)과 나머지 제안·미확정 사항을 구분한다. 명령과 규칙은 연결한 문서 한 곳에서 관리한다.

| 확인할 내용 | 프로젝트의 기준 문서 |
| --- | --- |
| 외부 동작과 수용 시나리오 | [스펙 목록](specs/README.md) — Active 5건(사용자 결정 2026-11-07(Q150)). 채택과 검증 완료는 구분한다. |
| 교환 형식(계약)과 기술 결정 | [계약 목록](contracts/README.md) — Active 6건(5건 2026-10-16 채택, Repeater 초기 교환 2026-11-01 채택). [ADR 목록](decisions/README.md) — Accepted 5건 |
| 코드 지도(변경 대상의 코드 위치와 연결 관계) | [코드 지도](architectures/code-map.md) — Current, Draft, Verified. 현재 코드는 `serve` 시작과 CLI 골격([`serve` 시작 모듈 설계](architectures/serve-startup.md): `src/`와 계정 파일·리스너 모듈), 설정 로더([설정 로딩 모듈 설계](architectures/config-loading.md): `crates/domain`·`crates/app`·`crates/adapters`), 운영 로그([운영 로그 모듈 설계](architectures/ops-log.md): 사건 타입·JSON Lines 출력·`relay_id_ref`), TCP 진입점([TCP 진입점 모듈 설계](architectures/tcp-entry-point.md): 접속 수락·출발지 IP 검사), 연결 상한과 시간 제한([연결 상한과 시간 제한 모듈 설계](architectures/connection-limits.md): 연결 예산·L·H·TCP keepalive), 상태 리스너와 준비 상태([상태 리스너와 준비 상태 모듈 설계](architectures/status-listener.md): `/livez`·`/readyz`·Lm·`server_ready`), 대시보드 공통 컴포넌트([대시보드 공통 컴포넌트 모듈 설계](architectures/dashboard-components.md): 조각 DOM·옵션·상태 속성과 미리보기), 개발 도구 [`xtask/`](architectures/xtask.md)다 |
| 적용할 컨벤션 | [컨벤션 목록](conventions/README.md) — [Rust 코드 컨벤션](conventions/rust.md)(Active, 2026-10-20), [테스트 컨벤션](conventions/testing.md)(Active, 2026-10-20), [프런트엔드 컨벤션](conventions/frontend.md)(Active, 2026-10-22, 사용자 결정 2026-10-22(Q32~Q39)). 디자인 시스템은 [디자인 목록](design/README.md)의 [운영 대시보드 디자인 시스템](design/DESIGN.md)(Accepted, 2026-10-20)과 채택된 [대시보드 HTML 시안](design/mockups/dashboard.html) |
| 개발 환경 준비와 실행 방법 | [개발 환경을 준비하고 변경을 검증하는 방법](how-to/set-up-and-verify.md) — Draft. 빌드와 전체 완료 검증은 Ubuntu와 Windows에서 확인했다. rustup 설치 단계는 두 OS 모두 미실행이다 |
| 실행파일 배포와 수동 운영 | [실행파일을 묶고 수동 운영하는 방법](how-to/operate-relay.md) — Draft. Windows 11 x64·Ubuntu 24.04 LTS x64의 release 묶음과 계정·재로드·종료 절차. 두 OS 수동 기록 전에는 검증 완료로 표시하지 않는다 |
| 버전 규칙과 릴리스 | [릴리스 묶음을 만들고 배포하는 방법](how-to/release.md) — Draft. `연.월.순서` 버전, `cargo xtask package`, GitHub Actions 검사, 태그로 GitHub Release 내기 |
| 변경 범위별 테스트와 전체 완료 검증 | [개발 환경·검증 how-to 3.3·3.4절](how-to/set-up-and-verify.md#33-변경-범위별-검증). 시나리오 표식과 보고 명령은 [테스트 컨벤션](conventions/testing.md)이 정한다. 의존성을 바꾸면 [의존성 라이선스 검사](how-to/check-dependency-licenses.md)를 실행한다. [스펙 문서 검증 기록](specs/README.md#문서-검증-기록)은 문서 구조 확인만 포함 |
| 구현 작업 순서와 완료 조건 | [구현 작업 목록](TODO.md) — 기준 문서가 아니다. 작업마다 기준 문서를 링크한다 |

프런트엔드 프로젝트는 위 표에 실제 디자인 기준·컨벤션·검증 절차를 연결한다. 작업은 [frontend-workflow 스킬](../.agents/skills/frontend-workflow/SKILL.md)을 따른다.

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
| 이해하기 어려운 원리나 구현 설명 | 해설 |

영향이 없는 유형은 빈 문서를 미리 만들지 않는다.

## 문서 목록 관리

[문서 작성과 목록 갱신](AGENTS.md#문서-작성과-목록-갱신) 규칙을 따른다.
