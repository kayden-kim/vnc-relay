# 대시보드 프런트엔드를 검증하는 방법

대시보드 프런트엔드(`web/dashboard/`)가 [프런트엔드 컨벤션](../conventions/frontend.md)과 [디자인 기준](../design/DESIGN.md)을 지키는지 확인한다. 완료 상태는 4절의 모든 행이 실제 결과와 함께 "통과"로 기록된 상태다. 수동 검토를 하지 못한 행은 "미실행"과 이유를 적고, 완료로 표시하지 않는다.

이 문서는 Draft다. 절차의 방법은 사용자 결정 2026-10-22(Q32)~(Q38)를 따른다. 작업 4.8에서 `cargo xtask tokens`, `cargo xtask dashboard-serve`, `tests/dashboard_browser.rs`를 구현했다. `tests/dashboard_assets.rs`는 작업 4.6에서 만들었다. 실행한 결과는 4절에 적는다.

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 대시보드 프런트엔드 구현자와 검토자 |
| 검증 상태 | Draft |
| 검증 환경·버전 | 작업 4.8(2026-10-22): Windows 10.0.26200 x64, PowerShell, Chrome 155.0.8059.84. Ubuntu 24.04.4 LTS x64(WSL2), Google Chrome 156.0.8078.47. rustc는 두 OS 모두 1.99.1(`rust-toolchain.toml`). 나머지 환경은 실행할 때 기록한다 |
| 최종 검증일 | 2026-10-22. 3.1 1단계, 3.2 1단계, 3.3 1단계, 3.4 일부를 실행했다(4절). 작업 4.10 재실행(4.5절, 커밋 `3944dc2`): WSL·Windows 게이트 모두 `ALL PASSED`. 작업 4.11(4.7절, 커밋 `5368dec`): 3.3 1단계, 3.4 SCR-002 일부, 3.5 자동 확인 일부를 실행했다. 최종 HEAD 게이트는 WSL·Windows 모두 `ALL PASSED`. 작업 4.11 세션 시험 보강(4.9절, 커밋 `d0ef8d0`): 3.4 SCR-002 일부(DASH-016·019·022·024·025 시험)를 실행했다. 이 게이트도 WSL·Windows 모두 `ALL PASSED`. 작업 4.11 라우터 언어 시험(4.10절, 커밋 `a917eb1`): 3.4 일부(DASH-025 라우터 시험)를 실행했다. 최종 HEAD `48918d3`의 게이트도 WSL·Windows 모두 `ALL PASSED`. 3.5의 사람 확인, 3.6, 3.7은 미실행 |
| 관련 기준 문서 | [프런트엔드 컨벤션](../conventions/frontend.md), [디자인 기준](../design/DESIGN.md), [화면 스펙](../specs/dashboard.md), [관리 API 계약 1.11절](../contracts/management-api-v1.md) |

## 1. 대상과 영향

- 대상: `web/dashboard/` 정적 파일, `docs/design/DESIGN.md`의 YAML 토큰, `docs/design/mockups/dashboard.html`의 토큰 블록, 대시보드 테스트 파일(파일 이름은 이 문서의 기본값: `tests/dashboard_assets.rs`, `tests/dashboard_browser.rs`)
- 예상 사용자 영향: 없음. 자동 검사는 파일을 읽고 `target/` 아래에 결과를 쓴다. 운영 서버를 멈추지 않는다.
- 작업 허용 시간: 자동 검사는 서버 중단이 필요 없다. 수동 검토(3.4~3.7절)는 검토자의 시간이 필요하다.
- 이 문서는 사용자 결정 2026-10-22(Q09)에 따라 작업 0.9와 함께 작성했다. 절차는 사용자 결정 2026-10-22(Q32)~(Q38)를 따른다.

## 2. 전제 조건

- [ ] 검증할 커밋을 정했고, `git status --short` 결과를 검증 기록에 적었다.
- [ ] 고정 Rust 툴체인 1.99.1을 쓴다(`rust-toolchain.toml`).
- [ ] Chromium(Chrome 또는 Edge)은 두 OS 모두 필수다. 실행 파일 경로를 환경 변수 `DASHBOARD_BROWSER`에 넣을 수 있다. 넣지 않으면 표준 설치 경로와 PATH에서 Chrome, 다음 Edge를 찾는다. 찾지 못하면 브라우저 시험은 실패한다(사용자 결정 2026-10-22(Q35)). 환경 변수 이름은 이 문서의 기본값이다.
- [ ] Firefox·Safari 수동 검토에 쓸 버전을 정했다. 실제 버전은 검증 기록에 적는다(사용자 결정 2026-10-22(Q36)).
- [ ] (선택) Node.js와 고정 버전의 `@google/design.md`를 준비했다. 고정 버전은 검증 기록에 적는다(사용자 결정 2026-10-22(Q38)).
- [ ] 수동 검토자와 스크린 리더 환경을 정했다.

`cargo test --test dashboard_assets`는 작업 4.6에서, `cargo xtask tokens`, `cargo xtask dashboard-serve`, `cargo test --test dashboard_browser`는 작업 4.8에서 구현했다.

## 3. 절차

- 실행 환경: Ubuntu 24.04 LTS x64는 bash, Windows 11 x64는 PowerShell을 쓴다. 지원 환경은 [개발 환경·검증 how-to 2절](set-up-and-verify.md#2-전제-조건)을 따른다.
- 실행 위치: 저장소 루트
- 바꿔 넣을 값: `DASHBOARD_BROWSER`에는 Chromium 실행 파일 경로를 넣는다. 확인하려면 경로를 직접 실행해 `--version`이 버전을 출력하는지 본다. 경로에 공백이 있으면 큰따옴표로 감싼다.
- 최대 대기 시간: 자동 명령은 10분(이 문서의 기본값)을 넘기면 중단하고 5절을 따른다. 수동 검토에는 시간 한도가 없다.

### 3.1 토큰과 문서 확인

1. 저장된 토큰 CSS를 DESIGN.md YAML에서 다시 만든 결과와 대조한다. 로그인 화면의 생성 복사본(`web/dashboard/login/components/`, `web/dashboard/login/styles/`)도 원본 `web/dashboard/app/`과 같은지 대조한다.

   영향: 없음. 파일을 쓰지 않는다.

   ```shell
   cargo xtask tokens --check
   ```

   기대 결과: 종료 코드 0. 차이가 있으면 파일 이름과 줄이 출력되고 종료 코드가 0이 아니다.

2. (선택) DESIGN.md 린트를 실행한다.

   영향: 없음. `npx`가 패키지를 내려받는다. 고정 버전은 아직 정하지 않았다. 처음 실행할 때 정하고 검증 기록에 적는다.

   ```shell
   npx -y -p @google/design.md@<고정 버전> designmd lint docs/design/DESIGN.md
   ```

   기대 결과: 오류 0건. 경고 수와 정보 수를 검증 기록에 적는다. 2026-10-20 실행 기록은 오류 0건, 경고 18건, 정보 1건이다([디자인 기준 검증 절](../design/DESIGN.md#검증)).

### 3.2 제공 파일과 실행 파일 내장 대조

1. 제공 표와 `web/dashboard/`의 실제 파일을 비교한다. 빠진 파일이나 표에 없는 파일이 있는지 확인한다.

   영향: 없음. 테스트 빌드 결과는 `target/` 아래에 쓴다.

   ```shell
   cargo test --test dashboard_assets
   ```

   기대 결과: `test result: ok.` 줄이 나온다. 실패한 테스트 이름이 있으면 실패다.

### 3.3 브라우저 테스트와 보고서

1. 헤드리스 Chromium으로 브라우저 테스트를 실행한다.

   영향: 임시 브라우저 프로필을 만든다. 실행이 끝나면 지운다(이 문서의 기본값).

   ```shell
   cargo test --test dashboard_browser
   ```

   기대 결과: `test result: ok.` 줄이 나온다. 각 테스트 페이지의 `data-result` 값이 `pass`다. `DASHBOARD_BROWSER`가 없거나 실행되지 않으면 테스트가 실패해야 한다(사용자 결정 2026-10-22(Q35)).

2. 시나리오 보고서를 갱신한다.

   영향: `<Cargo 대상 디렉터리>/scenario-report.md`를 덮어쓴다.

   ```shell
   cargo xtask scenarios
   ```

   기대 결과: 자동으로 연결된 DASH 항목의 상태가 `통과`다. `부분` 표식이 붙으면 검증 기록에 이유를 적는다. 현재 저장소에서는 DASH 시나리오가 모두 `미연결`이다. 브라우저 테스트 파일이 생기고 표식이 붙으면 연결된다([테스트 컨벤션 7절](../conventions/testing.md#7-테스트와-스펙-시나리오-연결)).

### 3.4 폭별 스크린샷과 시안 대조 (수동)

1. 미리보기 서버를 실행한다(사용자 결정 2026-10-22(Q34)).

   영향: 루프백 주소의 포트 하나를 연다. Ctrl+C로 멈춘다.

   ```shell
   cargo xtask dashboard-serve
   ```

   기대 결과: 로컬 주소 한 줄이 출력되고 프로세스가 계속 실행된다.

2. 같은 화면 상태를 폭 1280·1024·1023·768·390·320px에서 연다. 시안 `docs/design/mockups/dashboard.html`에서 같은 상태의 프레임도 같은 폭에서 연다.

   영향: 없음. 스크린샷은 `target/dashboard-shots/<폭>/<화면 상태>.png`에 저장하고 커밋하지 않는다.

   기대 결과: 페이지에 가로 스크롤이 없다. 화면 상태는 [화면 스펙 4절](../specs/dashboard.md#4-화면-상태와-갱신)의 SCR-001·SCR-002·SCR-003이다.

3. 구현 스크린샷과 시안 스크린샷을 나란히 두고 차이를 적는다. 자동 픽셀 비교는 하지 않는다(사용자 결정 2026-10-22(Q36)).

   영향: 없음.

   기대 결과: 차이가 없거나, 차이가 화면 스펙과 디자인 기준에 맞는다. 시안과 스펙 문장이 다르면 스펙을 따른다.

### 3.5 키보드 확인 (수동)

1. 로그인 화면에서 Tab으로 모든 조작 요소를 차례로 지나간다.

   기대 결과: 포커스 순서가 화면 순서와 같다. 포커스가 사라지지 않는다.

2. 포커스 표시를 확인한다.

   기대 결과: 링 두께 2px(`spacing.focus-ring-width`), 간격 2px(`spacing.focus-ring-offset`), 색 `colors.ring`이다. 상단 막대 안에서는 `colors.navbar-ring`이다.

3. 조작 요소를 Enter 또는 Space로 실행한다.

   기대 결과: 마우스로 실행한 것과 같은 행동이 일어난다.

4. 대화상자를 열고 닫는다.

   기대 결과: [화면 스펙](../specs/dashboard.md) 6절의 대화상자 포커스 규칙을 따른다.

검토자, 날짜, 브라우저 이름과 버전, 각 결과를 검증 기록에 적는다.

### 3.6 스크린 리더 확인 (수동)

1. 스크린 리더를 켜고 로그인 화면과 SCR-001 화면을 연다. 스크린 리더 이름과 버전을 기록한다.

   기대 결과: 아래 2단계의 항목을 모두 확인했다.

2. 다음 항목을 확인한다.

   - 상단 막대의 이름이 "주 메뉴"이고, 본문이 랜드마크로 읽힌다([디자인 기준 접근성 절](../design/DESIGN.md#접근성-기준)).
   - 폭 390px에서 활성 세션 카드 목록이 `table`·`row`·`columnheader`·`cell` 역할로 읽힌다.
   - 사용량 고리가 이름과 현재·최소·최대 값을 읽는다(`progressbar`).
   - 언어 선택의 이름이 "언어 (Language)" 또는 "Language (언어)"로 읽힌다.
   - 장식용 SVG는 읽히지 않는다.

### 3.7 다른 브라우저와 실기기 확인 (수동)

1. Firefox와 Safari에서 3.4의 2단계와 3.5 전체를 실행한다. 실행한 브라우저의 이름과 버전을 적는다. 실행할 수 없는 브라우저는 "미실행"과 이유를 적는다(사용자 결정 2026-10-22(Q36)).

   영향: 없음.

   기대 결과: 결과가 Chromium과 같다. 차이가 있으면 검증 기록에 적는다.

2. 실제 모바일 기기에서 390px 폭 화면을 확인한다. 릴리스 전 한 번 실행한다(사용자 결정 2026-10-22(Q36)).

   영향: 없음.

   기대 결과: 카드 목록에서 정보와 행동이 가려지지 않는다.

## 4. 검증

[검증 기록](../AGENTS.md#검증-기록)에 따라 실제 결과와 성공 조건을 대조한다. 이 초안을 작성한 시점의 실제 결과는 모두 미실행이었다. 작업 4.8(2026-10-22)의 실행 결과는 아래 표의 "실제 결과" 열에 적는다.

| 관찰 대상 | 성공 조건 | 확인 명령·방법 | 실제 결과 | 실행 증거 위치 |
| --- | --- | --- | --- | --- |
| 토큰 CSS와 저장된 파일 | 차이 0건, 종료 코드 0 | 3.1 1단계 | 통과. 작업 4.8(Windows): `tokens --check: 통과`, 종료 코드 0. DESIGN.md L6 값을 `#f4f6f6`으로 바꾸면 `실패 (3건)`과 종료 코드 1이 나왔고, 원래 값으로 되돌린 뒤 다시 통과했다. 작업 4.9 재작업(4.3절): 로그인 복사본 포함 `tokens --check: 통과`. 복사본·원본·복사 목록 밖 파일을 변조하면 `실패 (1건)`과 종료 코드 1이 나왔고, 되돌린 뒤 통과했다 | 작업 4.8 보고의 명령 출력 |
| DESIGN.md 린트(선택) | 오류 0건, 경고·정보 수 기록 | 3.1 2단계 | 미실행 | 검증 기록의 출력 |
| 제공 파일 표 | `dashboard_assets` 테스트 통과 | 3.2 1단계 | 통과. 2026-10-22 커밋 `df3b5b2`, Ubuntu 24.04(WSL2): `cargo test --test dashboard_assets` 시험 2개 통과. 같은 커밋의 Windows 전체 완료 검증도 이 시험을 포함해 통과했다. 작업 4.9 재작업(4.3절): 시험 2개 통과(Windows). 작업 4.10(4.4절): 시험 2개 통과(Windows, 작업 트리) | 검증 기록의 테스트 출력 |
| 브라우저 테스트 | 모든 테스트 페이지의 `data-result`가 `pass` | 3.3 1단계 | 통과. 작업 4.8(Windows): `cargo test --test dashboard_browser` 시험 2개 통과(2026-10-22 재실행, 19.19초). Chrome 155.0.8059.84. Ubuntu 24.04(WSL2), 커밋 `df3b5b2`: 시험 2개 통과(10.39초), Google Chrome 156.0.8078.47(`/usr/bin/google-chrome`). 작업 4.9 재작업(4.3절): 시험 7개 통과(Windows, 27.20초). 작업 4.10(4.4절, Windows, 작업 트리): 시험 21개 통과(146.6초, 운영 화면 시험 포함). 작업 4.10 재실행(4.5절, 커밋 `3944dc2`): WSL 시험 21개 통과(61.78초), Windows 시험 21개 통과(97.16초). 기준 `61e03e9`의 WSL 실행은 시험 6개가 실패했다(4.5절). 작업 4.11(4.7절, 커밋 `5368dec`): 시험 31개 통과(WSL 89.48초, Windows 148.88초, 최종 HEAD). 바이트·레이아웃 수정 전 실행과 수정 뒤 `session_`·`router_` 시험 12개 통과는 4.7절에 있다 | 검증 기록의 테스트 출력과 브라우저 버전 |
| 시나리오 보고서 | 자동 연결된 DASH 항목이 `통과` | 3.3 2단계 | 부분. 작업 4.10(4.4절, 작업 트리): 150건 중 통과 23, 부분 40, 실패 0, 미실행 1, 미연결 86. DASH 27건은 통과 0, 부분 17, 미연결 10이다. 미실행 1건은 OPS-007이다. 4.9 재작업 수치(WSL 통과 18, Windows 통과 17)보다 통과가 늘었다. 늘어난 항목은 확인하지 않았다. 작업 4.10 재실행(4.5절, 커밋 `3944dc2`): WSL 통과 24, 부분 40, 실패 0, 미실행 0, 미연결 86. Windows 통과 23, 부분 40, 실패 0, 미실행 1(OPS-007), 미연결 86. DASH 세부 수치는 이 재실행에서 세지 않았다. 최종 게이트 수치는 4.5절에 있다. 작업 4.11(4.7절, 커밋 `5368dec`): DASH-006·007·009~014·026이 `partial`로 연결됐다. 최종 HEAD 수치는 4.7절에 있다 | `<Cargo 대상 디렉터리>/scenario-report.md` |
| 폭별 시각 대조 | 6개 폭에서 가로 스크롤 0건, 차이가 스펙·디자인 기준에 맞음 | 3.4 | 부분. 작업 4.8: 미리보기 6폭 × ko·en 12장(4.1절). 작업 4.9(2026-10-22): 로그인 화면 `/login/`을 6폭 × 7장으로 찍어 42장을 얻었다(`target/dashboard-shots/<폭>/{input-ko,invalid-ko,busy-ko,failed-ko,request-failed-ko,input-en,invalid-en}.png`). 6폭 모두 `scrollWidth`와 `clientWidth`가 같았고(320 포함), CSP 위반은 0건이었다. 콘솔 오류는 가로챈 401·500 응답의 "Failed to load resource"뿐이었다. 시안과의 차이는 4.2절. 작업 4.9 재작업(4.3절)에서 같은 42장을 다시 찍었다. 6폭 모두 가로 스크롤 0건, CSP 위반 0건이었다. 4.9 기준선과는 42장 모두 바이트가 달랐다(화면을 구성 요소로 바꾼 결과). 작업 4.10(4.4절): 실제 서버에서 `/login/`(ko 입력 화면)과 `/`(ko 운영 화면)를 6폭으로 찍었다. 가로 스크롤 0건. 320 폭의 상단 막대 언어 선택은 밑줄과 가까워 사람 확인이 필요하다. 사람 비교는 4.12에서 한다. 작업 4.10 화면 상태 대조(4.6절): SCR-001 목업 21장과 1280·390·320 × ko·en × 상태 8종 48장을 대조했다. 6폭·언어 모두 가로 스크롤 0건, CSP 위반 0건. 수정 필요 6건이 나왔다(상단 막대 5건, `Unavailable` 줄바꿈 1건). 1280은 정상 상태와 빈 목록 문구만 판정했다. 작업 4.11(4.7절): SCR-002 상태 5종(SCR-001 목록, SCR-002 상세·대화상자·실패, SCR-003 로그인) × 6폭 × ko·en 60장을 찍었다. 가로 넘침 0, CSP 위반 0. 이 60장은 바이트·레이아웃 수정 전 촬영이다. 수정 뒤에는 ko 1280·390·320만 다시 찍었다. 시안 차이 4건은 4.7절과 개요 14절 차이 표에 있다. 사람 비교는 4.12에서 한다 |
| 키보드 | 3.5의 4개 항목 통과 | 3.5 | 부분. 작업 4.11(4.7절, 작업 트리): SCR-002에서 Tab 순서, 포커스 링 두께(2px)·간격(2px), Enter로 취소 실행, 대화상자 Escape·취소 뒤 포커스 복귀(ko·en)를 자동으로 확인했다. 대화상자 Tab 순환을 고쳤고 합성 keydown으로만 시험했다. 로그인 화면 Tab 순서(3.5의 1번)와 실제 키 입력 전체는 확인하지 않았다. 4개 항목 전체 통과가 아니다 | 검토 기록 4.7절 |
| 스크린 리더 | 3.6의 항목 통과 | 3.6 | 미실행 | 검토 기록과 스크린 리더 이름·버전 |
| 다른 브라우저·실기기 | 3.7 통과 또는 미실행 사유 기록 | 3.7 | 미실행. Firefox, Safari, 실기기는 이 검토 환경에 없어 실행하지 못했다. Edge 155.0.4283.62에서 3.4 캡처만 했다(4.11절) | 브라우저 버전과 검토 기록 |

### 4.1 검토 기록 (작업 4.8, 2026-10-22)

- 검토자: 에이전트 Fe48(작업 위임: Main). 사람의 검토가 아니다. 사람 검토는 작업 4.12에서 한다(사용자 결정 Q42).
- 환경: Windows 10.0.26200 x64, Chrome 155.0.8059.84, 헤드리스. 확인 범위는 미리보기 페이지(`web/dashboard/preview/`)다.
- 확인 방법: 폭 1280·1024·1023·768·390·320 × 언어 ko·en 12장을 전체 페이지 PNG로 찍었다. 각 장의 PNG 폭과 `window.innerWidth`가 요청한 폭과 같은지 확인했다. 축소 보기로 전체 구성과 잘림을 보았다. 1280 ko의 대화상자 열린 견본 4개는 확대 조각(`target/dashboard-shots/1280/detail/dialog-frame-1.png`~`dialog-frame-4.png`)으로 보았다. 조각에서 위험 버튼의 빨간색, 긴 문구의 줄바꿈, 버튼 위치가 맞았다.
- 수정 반영(리뷰 A–D): 게이지 현재 값과 상한 줄(`bc6248e`), 상단 막대 로그아웃 `navbar` 변형(`fc7c325`), 입력 포커스 견본(`e1ba857`), 닫힌 대화상자 숨김과 열린 견본 프레임(`0741c3c`, 시험 `4e65789`).
- 찾은 문제: 축소 보기와 확대 조각에서 잘림, 겹침, 가로 스크롤은 보지 못했다. 간격과 색 대비의 세부 값은 사람이 확인한다.
- 캡처 방법 기록: 1차 캡처는 뷰포트가 적용되지 않아 폭 1600으로 찍혔다. 그 파일은 버리고 `page.setViewport`로 다시 찍었다. 아래 결과는 2차 캡처다.
- 미확인: 시안(`docs/design/mockups/dashboard.html`)과의 사람 비교, SCR-001~003 화면, 키보드·스크린 리더·다른 브라우저·실기기 확인(3.5~3.7).

### 4.2 검토 기록 (작업 4.9, 2026-10-22)

- 검토자: 에이전트 Login49(작업 위임: Main). 사람의 검토가 아니다. 사람 검토는 작업 4.12에서 한다.
- 구현 커밋: `aaca485`(화면, 시험, 스펙 기록). 이 기록은 그 위에 올린 별도 커밋이다.
- 환경: Windows 10.0.26200 x64, Chrome 155.0.8059.84 헤드리스. WSL Ubuntu 24.04, Google Chrome 156.0.8078.47 헤드리스.
- 명령과 결과:
  - `cargo xtask tokens --check`: 통과.
  - `cargo test --test dashboard_browser`: 시험 7개 통과(로그인 5개, 구성요소와 미리보기 2개).
  - `cargo test --test dashboard_assets`: 시험 2개 통과.
  - `cargo test -p vnc-relay-adapters --test management`: 시험 38개 통과(로그인 페이지 시험 2개 포함).
  - 게이트 1(WSL, `vr-full.sh`): `ALL PASSED`. `시나리오 150건: 통과 16, 부분 37, 실패 0, 미실행 0, 미연결 97`.
  - 게이트 2(Windows, `vr-win.ps1 -Only all`): `ALL PASSED (all)`. `시나리오 150건: 통과 15, 부분 37, 실패 0, 미실행 1, 미연결 97`. 미실행 1건은 OPS-007(`closes_and_logs_every_held_connection_when_stopping`)이다. 이번 실행의 시험 목록에 없었고, 작업 4.9 이전 기준선에도 같은 미실행이 있었다. 같은 날 앞선 실행은 다른 작업의 미완성 파일(`tests/reload.rs` 서식, `file_watch` 시험 1건) 때문에 실패했다. 위 결과는 그 파일들이 바뀐 뒤의 실행이다.
- 시각 확인 방법: 브라우저 자동화로 `/login/`을 6폭에서 열었다. 401, 500, 응답 없음은 요청 가로채기로 만들었다. 실제 서버 흐름은 아래 스모크로 따로 확인했다.
- 실제 서버 스모크(로컬 임시 설정과 계정 `alice`): `serve`를 띄우고 Chrome에서 `/login/`을 열었다.
  - 처음 열기: CPY-075 알림이 보였다. `<html lang>`은 `ko`였다.
  - 잘못된 비밀번호: `POST /api/v1/session`이 401을 돌려줬다. CPY-074가 보였고, 비밀번호 칸이 비었고, 계정명은 남았고, 포커스는 비밀번호 칸으로 갔다.
  - 맞는 비밀번호: 200과 세션 쿠키를 받고 `/`로 이동했다. SCR-001(작업 4.10)이 아직 없어 `not_found` JSON 404가 나왔다. 이 단계의 기대 결과다.
  - 언어를 `en`으로 바꿈: 제목, 버튼, `<html lang>`이 바뀌었다. 계정명은 남았고, 언어 `en`이 저장소에 저장됐다.
  - 새로고침: 언어 `en`이 유지됐다. 계정명은 비었다(아래 한계 참고).
  - 서버 로그: `admin_login_failed`(`invalid_credentials`)와 `admin_login_succeeded`가 남았다. 비밀번호 문자열은 없었다.
- 시안(`docs/design/mockups/dashboard.html`)과의 차이(구현 판단):
  - `scr003-input`: 시안에는 알림이 없다. 구현은 처음 열 때 CPY-075를 보인다(스펙 행 181).
  - `scr003-busy`: 시안에는 알림이 없다. 구현은 CPY-073을 보인다(스펙 행 181).
  - `scr003-failed`, `scr003-request-failed`: 시안은 제목이 있는 알림이다. 구현은 CPY 한 줄만 쓴다(스펙 9절에 제목 문구가 없다).
  - `scr003-invalid`: 시안은 "계정명을 입력한다."이다. 구현은 CPY-071 "계정명을 입력하십시오."를 쓴다(스펙 우선).
  - `scr003-logout-unknown`: 구현하지 않았다. CPY-076은 로그아웃 결과를 아는 작업 4.10(DASH-019)에서 다룬다.
- 확인하지 못한 것과 한계:
  - 키보드 입력: 이 Chrome 창에서는 키 입력이 페이지에 전달되지 않았다. 스모크에서는 DOM 값 설정과 `input` 이벤트로 대신했다. 실제 키 입력 확인은 3.5에서 한다.
  - 새로고침 후 계정명: 복원하지 않는다. 스펙은 언어 전환에서 계정명 유지를 정했고, 새로고침 복원은 정하지 않았다(열린 질문).
  - 로그인 요청 제한 시간 10초: 구현 판단이다. 계약은 값을 정하지 않았다(열린 질문).
  - 401, 500, 응답 없음 흐름의 시험은 가짜 응답으로 했다. 실제 서버 흐름은 스모크에서 401과 200만 확인했다.
  - 다른 브라우저와 실기기: 3.7에서 미실행이다.
  - 로그인 성공 뒤 404 JSON은 SCR-001이 없어서 생긴다. 작업 4.10 전까지 기대한 결과다.

### 4.3 검토 기록 (작업 4.9 재작업, 2026-10-22)

- 검토자: 에이전트 Login49(작업 위임: Main). 사람의 검토가 아니다. 사람 검토는 작업 4.12에서 한다.
- 구현: 4.2절의 `aaca485` 위에서 로그인 화면을 구성 요소와 스타일 복사본으로 다시 만들었다. 복사본은 `cargo xtask tokens`가 쓰고, `tokens --check`가 원본과 같은지 확인한다. 이 기록은 구현과 같은 커밋 `f5310b9`에 들어 있다.
- 환경: Windows 10.0.26200 x64, Chrome 155.0.8059.84, Rust 1.99.1. 게이트 1은 WSL Ubuntu 24.04, 게이트 2는 위의 Windows 환경에서 실행했다.
- 작업 트리: 게이트를 돌릴 때 Reload52의 미커밋 파일이 있었다. `crates/adapters/src/file_watch.rs`, `crates/adapters/src/file_watch/feed.rs`, `crates/app/src/reload/service.rs`(수정), `tests/reload.rs`(추적되지 않음)다. 이 파일들은 커밋 `f5310b9`에 없다.
- 명령과 결과:
  - `cargo xtask tokens` 뒤 `cargo xtask tokens --check`: `tokens --check: 통과`, 종료 코드 0.
  - 변조 시험: 복사본(`web/dashboard/login/components/alert.js`)과 원본(`web/dashboard/app/components/alert.js`)에 줄을 더했고, 복사 목록에 없는 파일(`web/dashboard/login/components/stray.js`)을 만들었다. 셋 모두 `실패 (1건)`과 종료 코드 1이었다. 되돌린 뒤 셋 모두 `통과`였다.
  - `cargo test --test dashboard_assets`: 시험 2개 통과.
  - `cargo test --test dashboard_browser`: 시험 7개 통과(로그인 5개, 구성요소와 미리보기 2개), 27.20초.
  - `cargo test -p vnc-relay-adapters --test management`: 시험 38개 통과. 시험 `static_paths_match_the_table_exactly`는 처음에 `/login/styles/base.css`의 404 기대값 때문에 실패했다. 이 경로는 이제 실제 파일이다. 시험 경로를 `/login/styles/missing.css`로 바꾸고 404 기대는 그대로 두었다.
  - `vr-win.ps1 -Only check`: `ALL PASSED (check)`. 이 실행은 management 시험 수정 전에 돌았다. 수정 뒤 실행은 아래 게이트가 포함한다.
  - 게이트 1(WSL, `vr-full.sh`): `ALL PASSED`, 종료 코드 0. `시나리오 150건: 통과 18, 부분 35, 실패 0, 미실행 0, 미연결 97. 오류 0건`.
  - 게이트 2(Windows, `vr-win.ps1 -Only all`): `ALL PASSED (all)`, 종료 코드 0. `시나리오 150건: 통과 17, 부분 35, 실패 0, 미실행 1, 미연결 97. 오류 0건`. 미실행 1건은 OPS-007(`closes_and_logs_every_held_connection_when_stopping`)이다. 이 시험은 게이트 1(WSL)에서는 통과했고, 게이트 2의 Windows 실행 목록에는 없었다.
  - 4.2절 기준선 대비 통과 +2, 부분 −2(두 게이트 모두)이다. 4.9 기준선 보고서는 보관하지 않아 늘어난 항목은 확인하지 않았다. OPS-024와 OPS-035의 통과는 미커밋 `tests/reload.rs`의 시험을 근거로 한다. 두 항목이 4.9 기준선에서 이미 통과였는지는 확인하지 않았다. 이 파일의 시험이 시나리오 수 변화에 섞였을 수 있다(추정).
- 폭별 화면: 6폭(1280·1024·1023·768·390·320) × 7장(ko 5장, en 2장), 42장을 `target/dashboard-shots/<폭>/`에 다시 찍었다. 모든 장에서 `scrollWidth`와 `clientWidth`가 같았고, CSP 위반은 0건이었다. `<html lang>`은 화면 언어와 같았다. 콘솔 오류는 가로챈 401·500의 `Failed to load resource`뿐이었다.
- 기준선과 비교: 4.9 장(`target/dashboard-shots-baseline/`)과 42장 모두 바이트가 달랐다. 화면을 구성 요소와 스타일로 바꾼 결과다. 1280 ko 입력 화면과 320 ko 처리 중 화면을 눈으로 보았다. 알림, 입력 칸, 버튼 배치가 깨지지 않았다. 나머지 40장은 눈으로 보지 않았다(수치로만 확인).
- 실제 서버 스모크: 디버그 빌드 `vnc-relay serve`, 임시 설정 `target/login-smoke/relay.toml`(관리 `127.0.0.1:18701`, 상태 `127.0.0.1:18702`, 중계 진입점 없음), 계정 `alice`. 해시는 Bun의 argon2id로 `m=19456,t=2,p=1`에 만들어 계정 파일에 넣었다. 계정 파일 형식은 [CLI 계약 1.3·1.4절](../contracts/cli-v1.md)을 따랐다.
  - 처음 열기: CPY-075 알림, `<html lang>`=ko, 저장된 언어 없음.
  - 잘못된 비밀번호: `POST /api/v1/session` 401. `failed` 알림(CPY-074), 비밀번호 칸 비움, 계정명 `alice` 유지, 포커스는 비밀번호 칸.
  - 맞는 비밀번호: 200, `/`로 이동. 본문은 `not_found` JSON이다(SCR-001이 없어서 생긴다. 작업 4.10 전까지 기대 결과).
  - 언어 `en`: 제목·버튼·`<html lang>`이 바뀌었다. 계정명 `alice`는 남았고, 저장 키 `vnc-relay.language`=en.
  - 새로고침: `en` 유지, 계정명 비움.
  - 서버 로그: `admin_login_failed`(`invalid_credentials`, `alice`)와 `admin_login_succeeded`(`alice`). 비밀번호 문자열은 없었다.
  - 키 입력은 이 Chrome 창에 전달되지 않아 DOM 값 설정과 `input`·`change` 이벤트로 대신했다(4.2절과 같은 한계).
  - 콘솔 오류: 401 한 건, 404 한 건. 404는 로그인 뒤 `/`의 JSON 404로 보이지만 URL은 확인하지 않았다(추정).
- 구현 판단(사용자 결정 없음): 알림은 제목 한 줄짜리 알림 하나만 쓴다. 제목은 구성 요소 스타일대로 굵다. 로그인 요청 제한 시간은 10초다. 알림은 내용이 바뀔 때만 다시 만든다. 포커스 복원 장치는 두지 않는다. 입력 오류 표시는 `text-field.js`의 `setTextFieldError`를 쓴다.
- 확인하지 못한 것: 키보드·스크린 리더·다른 브라우저·실기기(3.5~3.7 미실행). 시안과의 사람 비교(4.12). 새로고침 뒤 계정명 복원과 제한 시간 10초는 열린 질문이다(4.2절).

### 4.4 검토 기록 (작업 4.10, 2026-10-22)

- 검토자: 에이전트 Dash410(작업 위임: Main). 사람의 검토가 아니다. 사람 검토는 작업 4.12에서 한다.
- 구현: 운영 현황 화면 `web/dashboard/app/`(구조는 [운영 현황 앱 구조](../architectures/dashboard-operations-overview.md)). 시험 페이지 `web/dashboard/tests/operations.html`과 `operations-scenarios.js`. 정적 경로는 `crates/adapters/src/dashboard_assets.rs`의 `ASSETS`에 `/`와 `/app/*` 10개를 더했다.
- 환경: Windows 10.0.26200 x64. 명령은 PowerShell에서 `cargo`로 실행했다. 시험 브라우저 버전은 기록하지 않았다.
- 작업 트리: 검증은 커밋 전 작업 트리에서 했다. 최종 게이트는 최종 기록의 커밋에서 다시 돌렸다.
- 명령과 결과:
  - `cargo fmt --all`: 종료 코드 0.
  - `cargo xtask tokens` 뒤 `cargo xtask tokens --check`: 통과.
  - `vr-win.ps1 -Only check`: `ALL PASSED (check)`.
  - `cargo test --test dashboard_assets`: 시험 2개 통과.
  - `cargo test --test dashboard_browser`: 시험 21개 통과, 146.6초. 운영 화면 시험을 포함한다.
  - `cargo test -p vnc-relay-adapters --test management`: 시험 38개 통과.
  - `cargo xtask scenarios`: 종료 코드 0. 150건: 통과 23, 부분 40, 실패 0, 미실행 1, 미연결 86. DASH 27건: 통과 0, 부분 17, 실패 0, 미실행 0, 미연결 10. 부분 17건은 DASH-001~005, 008, 014, 016~022, 024~025, 027이다. 미실행 1건은 OPS-007(`tests/serve.rs`, 기존 미실행)이다.
- 실제 서버 확인(루프백만 사용): 임시 설정 `target/screenshot/`(관리 `127.0.0.1:18711`, 상태 `127.0.0.1:18712`), 계정 `alice`(임시 비밀번호, 해시는 Bun argon2id). `vnc-relay serve`를 띄우고 Chrome으로 6폭을 찍었다.
  - `/login/`(ko, 입력 화면): `target/dashboard-shots/<폭>/real-login-input-ko.png`. 가로 스크롤 없음.
  - `/`(ko, 운영 화면, 로그인 뒤): `target/dashboard-shots/<폭>/real-ops-ko.png`. 가로 스크롤 없음.
  - 320 확대: `target/dashboard-shots/320/real-ops-ko-top-zoom.png`.
  - 404: `/api/v1/summary`, `waiting`, `sessions`, `closed-sessions`가 모두 404다. 핸들러가 아직 없다(TODO 4.4·4.5 미완료). 화면은 "조회 실패"와 "확인 불가"를 보인다. 4.4 전에 기대되는 결과이며 화면 결함이 아니다.
  - SCR-002(4.11)는 실제 세션이 없어 찍지 않았다.
  - 확인 뒤 서버를 종료했다. 다른 `vnc-relay` 프로세스(PID 79548)는 건드리지 않았다.
- 관찰(고치지 않음): 320 폭에서 한국어 언어 선택이 상단 막대 아래 가장자리에 붙고, 초록색 활성 탭 밑줄 일부를 가린다. 사람 확인 대상이다(작업 4.12).
- 검토 중 찾은 결함: `format.js` 주석 하나. 코드 변경은 없다.
- 구현 판단과 열린 항목(사용자 결정 대기):
  - D1: 2^53 − 1 바이트까지 정확하다. 그 위 표시 한계를 허용할지(Q1). 계약 §6 L562를 닫을지도 함께 본다.
  - D2: 세션 종료 중 403은 `/login/?notice=logout-unconfirmed`로 보낸다(잠정). 관리 세션은 서버에서 만료될 때까지 살아 있다(Q2).
  - navbar "주 메뉴"/"Main menu"는 잠정 문구다. 스펙 9.3.1에 짝이 없다(Q3).
  - 상세 링크 처리(D4)는 Main 확인 대기. 해시 라우팅은 임시이며 알 수 없는 해시는 SCR-001을 보인다.
- 확인하지 못한 것: 키보드, 스크린 리더, 다른 브라우저·실기기(3.5~3.7). 시안과 사람 비교(4.12). 실제 세션 데이터 표시와 폴링 값의 실서버 확인(4.4·4.5 뒤). SCR-002(4.11).

### 4.5 검토 기록 (작업 4.10 재실행, 2026-10-22)

- 검토자: 에이전트 Dash410(작업 위임: Main). 사람의 검토가 아니다. 사람 검토는 작업 4.12에서 한다.
- 기준과 수정: 기준 커밋 `61e03e9`(4.4절 기록 커밋)의 WSL `cargo test --test dashboard_browser`는 여러 차례 실패했다. 실패한 시험은 `operations-scenarios.js`의 DASH-001, 003, 004, 019, 020, 021 시험이었다. 수정 커밋 `3944dc2`는 `web/dashboard/tests/operations-scenarios.js` 한 파일만 바꿨다.
- 원인: 시험 쪽 `COPY` 표에 `loading` 키가 없었다. 그래서 `t.loading`이 `undefined`였고, `settled()`의 `includes(undefined)`는 항상 거짓이었다. 카드가 보이기만 하면 대기가 끝나서, 데이터가 그려지기 전에 검사가 돌았다. 고친 것: ko `조회 중`, en `Loading`을 `COPY`에 넣었다(`app/copy.js` CPY-016과 같다). 짧은 고정 대기 몇 곳은 조건 대기(`waitFor`)로 바꿨다. DASH-022는 요청 시각 `calls[].at`으로 간격을 잰다. 나머지 고정 대기는 간격 상한을 확인하려고 그대로 두었다.
- 작업 트리: 게이트는 커밋 `3944dc2`의 깨끗한 작업 트리에서 돌렸다. 이 문서 변경은 그 뒤 따로 커밋했다.
- 명령과 결과:
  - WSL 게이트 `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh`: 종료 코드 0, `ALL PASSED`, 139.42초. 시나리오 150건: 통과 24, 부분 40, 실패 0, 미실행 0, 미연결 86. 오류 0건. 같은 실행의 `cargo test --test dashboard_browser`: 시험 21개 통과, 61.78초.
  - Windows 게이트 `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all`: 종료 코드 0, `ALL PASSED (all)`, 176.36초. 시나리오 150건: 통과 23, 부분 40, 실패 0, 미실행 1(OPS-007, Unix 전용), 미연결 86. 오류 0건. 같은 실행의 `dashboard_browser`: 시험 21개 통과, 97.16초.
  - 두 게이트의 시험 바이너리는 모두 실패 0건이다.
  - WSL 통과 24와 Windows 통과 23의 차이는 OPS-007 한 건이다. WSL에서는 통과, Windows에서는 Unix 전용이라 미실행이다.
  - 로그의 `error[watch_unavailable]` 줄은 감시 등록 거부를 일부러 만든 시험의 출력이다([INFERENCE]: 시험 이름으로 판단). 실행은 통과했다.
  - WSL 전체 로그는 WSL 홈의 `gate-wsl-3944dc2.log`에 있다.
- 브라우저 버전은 기록하지 않았다(4.4절과 같다).
- 확인하지 못한 것: 키보드, 스크린 리더, 다른 브라우저·실기기(3.5~3.7 미실행). 시안과 사람 비교(4.12). 320 폭의 언어 선택 밑줄 가림(4.4절 관찰)은 이번에 다시 보지 않았다.

### 4.6 검토 기록 (작업 4.10 화면 상태 대조, 2026-10-22)

- 검토자: 에이전트 Dash410(작업 위임: Main). 사람의 검토가 아니다. 사람 검토는 작업 4.12에서 한다.
- 환경: Windows 10.0.26200 x64, Chrome 155.0.8059.84(`--headless=new`, `--hide-scrollbars`, `--force-device-scale-factor=1`). 기준 코드 커밋 `3944dc2`. 이 작업은 코드를 바꾸지 않았다.
- 실행: `cargo xtask dashboard-serve --port 18090`(정적 서버, 127.0.0.1)을 띄우고 `target/dashboard-shots/capture.mjs`로 찍었다. 캡처 뒤 서버와 Chrome을 종료했다. 결과는 `target/dashboard-shots/<폭>/<상태>-<언어>.png`와 `summary.json`이다. 두 경로는 git이 무시하며 커밋하지 않았다.
- 상태: 정상, 갱신 중, 빈 목록, 첫 조회 실패, 이전 데이터가 있는 실패는 시험 페이지 `web/dashboard/tests/operations.html`의 `dash-001`·`dash-002`·`dash-003` 상태로 찍었다. 조회 중, 전체 실패, 전체 이전 데이터 실패는 `screen-harness`로 찍었다. `screen-harness`는 실제 `operations.js`를 로컬 가짜 요청에 붙인 캡처 전용 페이지다. 배포된 시험이 아니다. 권한 없음(denied) 상태는 찍지 않았다.
- 가로 스크롤: 1280·390·320 × ko·en 6개 조합 모두 `scrollWidth`와 `clientWidth`가 같았다. 넘치는 요소는 0개다.
- CSP 위반: 6개 조합 모두 0건.
- 콘솔 오류: 1280 ko의 정상과 갱신 중 상태에서 각 1건(favicon 404, 정적 서버 부산물). 나머지 0건.
- 시험 판정: 시험 페이지의 `data-result`는 `pass`였다. `screen-harness`는 시험 판정 대상이 아니다.
- 출처 밖 요청: 이 캡처에서 따로 기록하지 않았다. 중단 조건(5절)은 확인하지 않았다.
- 판정 기준: 스펙 `docs/specs/dashboard.md` 4절 상태표, `docs/design/DESIGN.md` 화면 규칙, 목업 `docs/design/mockups/dashboard.html`. 목업 문구가 다르더라도 스펙 문구나 CPY 키와 맞으면 허용 차이로 봤다.
- 수정 필요 6건(코드 수정 대상. 이 작업에서 고치지 않았다):
  - 상단 막대 `운영 현황` 항목, 390·320(ko·en): 목업은 `display:none`, 구현은 표시된다. 근거: DESIGN.md 543·550행; `operations.js` 메뉴 항목에 `desktopOnly`가 없다; `navbar.js` 46행.
  - 상단 막대 높이와 행, 390 en·320: 목업 56px 한 행, 구현 92px 두 행. 390 ko는 56px로 일치한다. 근거: DESIGN.md 119·550행; `components.css` `.navbar`의 `flex-wrap: wrap`(476–485행).
  - 언어 선택 위치, 390 en·320: 목업 1행 top 6–50, 구현 2행 top 48–92. 390 ko는 일치한다. 근거: DESIGN.md 550·642행.
  - 활성 인디케이터, 390 ko: 목업 파생 top 53, 구현 57–60으로 막대 하단 56 밖 1px. 근거: DESIGN.md 641·122행; `components.css` 558–567행 `::after`가 36px 기준이다.
  - 활성 인디케이터, 390 en·320: 목업 파생 top 53(간격 3), 구현 51–54로 언어 선택(48–92)과 겹친다(간격 −41). 근거: 위 인디케이터 항목과 같은 원인.
  - 서버 요약 `Unavailable` 값, 1280 en: `Unavaila / ble`로 중간에서 나뉜다. 390 en은 `Unavailabl / e`. 근거: [INFERENCE] 스펙 6절(필수 정보가 읽히도록 유지, L209).
- 허용 차이(스펙·CPY 기준에 맞는 구현 차이):
  - 390 ko 조회 중: 목업 상단 배너(`조회 중 / 아직 모르는 값은 0으로 표시하지 않는다.`) 대신 배너 없이 카드별 `조회 중` 뱃지와 값 자리 `확인 불가`를 쓴다. 근거: 스펙 4절 170행, CPY-016.
  - 390 ko 조회 중: 목업 새로고침 `갱신 중`(비활성) 대신 `새로고침`이 활성이다. 중복 조회는 `query.inFlight`로 무시한다. 근거: 스펙 4절 174행. [INFERENCE] 무시 경로는 `operations.js` 306행 부근에서 코드로 더 확인해야 한다.
  - 390 ko 조회 중: 마지막 성공 갱신 값 목업 `아직 없음`, 구현 `확인 불가`. 근거: CPY-035, CPY-015. 목업 문구는 스펙에 없다.
  - 390·320 ko 첫 조회 실패: 목업 화면 배너와 배너 안 새로고침 버튼 대신 목록별 alert(CPY-021)와 값 `확인 불가`. 근거: 스펙 3절 93행, 4절 175행, DASH-003 223행. 빈 성공 목록으로 보이지 않는다는 조건은 양쪽이 충족한다.
  - 390·320 ko 이전 데이터가 있는 실패: 배너 문구가 목업 `관리 서버와의 연결이 중간에 끊겨서…`와 다르다. 구현은 CPY-020(`조회에 실패했습니다. 아래 값은 마지막 성공 시각 {time}의 이전 데이터입니다.`). 근거: CPY-020, 스펙 4절 175행.
  - 390 ko 갱신 중: 목업의 배너 `갱신 중 / 이전 성공 데이터를 유지한다.`, 버튼 `갱신 중`, 메타 `오래된` 대신 카드별 `갱신 중` 뱃지(CPY-017), 이전 값 유지, 버튼 활성. 근거: 스펙 4절 174행, CPY-017.
  - 390 ko 빈 목록: 목록 제목 옆 `0건` 뱃지가 목업에만 있다. 스펙은 건수 표시를 규정하지 않는다. 근거: 스펙 3절 84–95행.
  - 390 ko 빈 목록: 서버 요약 수치는 목업 `0`(샘플), 구현 `2 / 3 / 확인 불가`(fixture). 데이터 차이다. 근거: DASH-002 222행.
  - 라벨 `대기`·`활성`(목업)과 `대기 연결 수`·`활성 세션 수`(구현). 구현이 CPY-027·028과 일치한다.
  - 게이지 설명 문구(목업 문구 다름). 구현이 CPY-032·034와 일치한다.
  - 활성 세션 열 머리글 목업 `방향별 바이트`, 구현 `Viewer->대상 / 대상->Viewer`. 구현이 스펙 3절 92행, CPY-040·043과 일치한다.
  - 1280·390 en 화면 제목 목업 `Overview`, 구현 `Operations overview`. 구현이 CPY-004와 일치한다.
  - 샘플 데이터(행 수, 40자 ID, IPv6, 목록 개수)는 목업 내용이다. 동작 차이가 아니다.
- 일치: 1280 ko·en 활성 인디케이터 간격 7(53–56, DESIGN.md 641행). 빈 목록 문구 `대기 없음`·`활성 없음`·`최근 종료 없음`(ko, 1280·390·320; 스펙 3절 89–91행, CPY-038·041·044).
- 판정 범위: 1280은 정상 상태와 빈 목록 문구만 판정했다. 320은 상단 막대, 빈 목록 문구, 실패·이전 데이터 상태를 판정했다. 320의 조회 중·갱신 중은 수치만 확인했다. 나머지 육안 판정은 하지 않았다. en 비정상 상태는 목업이 없어 레이아웃 수치만 비교했다.
- 확인하지 못한 것: 목업 `.app` 클립은 18px 좁다(1262/372/302px). 320 판정은 302px 기준이다. `--hide-scrollbars` 효과는 확인하지 않았다. DPR은 1로 고정했다. 200% 줌, 글자 크기 조정, 다크 모드(목업은 라이트만)는 확인하지 않았다. 키보드, 스크린 리더, Edge·Firefox·Safari, 실기기 터치는 3.5–3.7에서 한다. en 카드 제목 목업 `Admin connections`와 구현 `Management connections`는 스펙 9절 영문 문구를 확인하지 않았다. 실행 상태 pill 스타일과 구현 텍스트 색은 DESIGN 뱃지 규칙을 확인하지 않았다. 권한 없음 상태(DASH-008)는 찍지 않았다. favicon 404와 en 실패 harness의 `loadingLeft` 결과는 점검 아티팩트다. `Loading failed` 문자열이 `Loading`을 포함해 생긴 결과이며 앱 결함이 아니다.
- 결과: 부분. 6개 조합 가로 스크롤 0건, CSP 위반 0건. 수정 필요 6건은 코드 수정 대상이다. 사람 비교는 4.12에서 한다.
- 수정 기록(2026-10-22, 같은 작업 트리, 커밋 대상): 수정 필요 6건 모두 코드를 고쳤다. 아래 재촬영은 수정 뒤에 다시 찍은 것이다. 위 본문의 수정 필요 목록은 수정 전 기록으로 남긴다.
  - 1–5(상단 막대): `operations.js`의 `운영 현황` 항목에 `desktopOnly: true`. `components.css`의 `.navbar`·`.navbar-tools`에서 `flex-wrap: wrap`을 뺐다. 재촬영: 390·320 ko·en 막대 56px 한 행, 언어 선택 top 6·높이 44, `운영 현황` 미표시(인디케이터 없음). 1280 인디케이터 top 53, 간격 7, 두 행 없음.
  - 6(`Unavailable`): 원인은 요약 카드 확인 불가 값이 `.stat-value`(`overflow-wrap: anywhere`) 안에서 최소 폭이 줄어 중간에서 나뉜 것이다. `anywhere`를 뺐다. 재촬영: 1280·390·320 en 정상 상태에서 한 줄(320 en 스크린샷 육안 확인, 나머지는 완전 문자열 텍스트 확인).
  - 추가로 드러난 결함 두 건을 고쳤다. (a) `anywhere`를 빼자 긴 숫자(미리보기 `stat-long-numbers`, `18,446,744,073,709,551,615`)가 flex 최소 폭 때문에 402px로 넘쳤다. `.stat-value`를 `display: block; overflow-wrap: break-word`로 바꿔 막았다(DESIGN 474행: 폭이 모자라면 줄바꿈). (b) 미리보기 navbar 샘플이 320에서 `nowrap` 최소 폭 295px로 늘어 docScroll 328이었다. `web/dashboard/preview/preview.css`에서 샘플 막대를 frame 안 가로 스크롤(`overflow-x: auto`, `contain: inline-size`)로 바꿨다. 1280에서도 샘플 폭이 frame 전체가 되는 부작용이 있다.
  - 코드 파일: `web/dashboard/app/operations.js`, `web/dashboard/app/styles/components.css`, `web/dashboard/preview/preview.css`, `web/dashboard/login/styles/components.css`(`cargo xtask tokens` 재생성). 문서 파일은 검증 기록, 대조 기록(`dashboard-components.md`·`dashboard-operations-overview.md`), 계약 6절과 5절 실행 기록이다.
- 재촬영 결과(`capture.mjs`, 48장): `scrollWidth == clientWidth`가 48장 모두 참, 넘치는 요소 0, CSP 위반 0. 앱 막대 12건 모두 한 행 56px, 문서 가로 넘침 0. 미리보기 390·320 docScroll 390·320, 넘침 0.
- 확인하지 못한 것(추가): 미리보기 320 전체 페이지 캡처는 16170px 세로라 육안 판독 불가. 측정치만 썼다. `LABEL.sr-only`(시각적으로 숨긴 중단 안내)가 320에서 뷰포트 밖 위치(right 366)에 있지만 넘침 계산에 들지 않는다. 키보드·스크린 리더·실기기는 여전히 3.5–3.7.
- 결과: 수정 필요 6건 해소(측정과 육안 확인). 최종 HEAD 게이트 결과는 이 절 끝에 적는다.
- 최종 HEAD 게이트(대상 HEAD `3b3a179`, 이 기록 커밋은 문서만 바꾼다):
  - WSL `vr-full.sh`: ALL PASSED. 시나리오 150건 통과 24, 부분 40, 실패 0, 미실행 0, 미연결 86. dashboard_browser 21 passed. 소요 137.98초. 로그 WSL `~/gate-wsl-3b3a179.log`.
  - Windows `vr-win.ps1 -Only all`: ALL PASSED (all). 시나리오 150건 통과 23, 부분 40, 실패 0, 미실행 1(OPS-007, Unix 전용), 미연결 86. dashboard_browser 21 passed. 소요 219.04초. 보고서 `D:\Workspace\vnc-relay\target\scenario-report.md`.

### 4.7 검토 기록 (작업 4.11 세션 상세, 2026-10-22)

- 검토자: 에이전트 Dash411(작업 위임: Main). 사람의 검토가 아니다. 사람 검토는 작업 4.12에서 한다.
- 구현: 세션 상세 `web/dashboard/app/session.js`, 패널 `panels.js`(`pageHead`, `panelCard`, `peerCell`), 복사 `copy.js`, 스타일 `app.css`, 해시 라우터 `router.js`, 공통 셸 `shell.js`, 조회 상태 `query.js`, 대화상자 Tab 순환 `components/dialog.js`. 시험 페이지는 `tests/session.html`(`session-scenarios.js`)과 `tests/router.html`(`router-scenarios.js`)이다.
- 환경: Windows 10.0.26200 x64, PowerShell, Rust 1.99.1. WSL Ubuntu에서도 같은 게이트를 돌렸다. 시험 브라우저는 헤드리스 Chrome이다. 버전은 이 기록에서 확인하지 않았다(4.8절: Chrome 155.0.8059.84).
- 기준과 커밋: 기준 `7294d96`(4.10 기록). 1차 구현 `172764a`, 시험 `9a35adf`, 문서 `5368dec`. 1차 수정 라운드: 구현 `e9fb429`, 스타일 `d88665c`, 시험 `db08863`, 문서 `8975537`(개요·TODO), 기록 `7624990`. 사용자 지시 개정 라운드: 구현·시험 `06753e3`. 최종 게이트는 `06753e3`에서 돌렸다(아래). 이 기록 커밋은 문서만 바꾼다.
- 사용자 지시 개정 라운드(`06753e3`, 5건):
  - 새로고침 이름: 머리말 버튼은 CPY-008 `새로고침`(`session-refresh`, outline 버튼)이다. 실패·확인 불가·요청 거절 알림의 조치 버튼은 CPY-014 `상태 재조회`다(`recheck`, 페이지에 하나만 있고 알림 종류가 바뀔 때만 알림을 다시 그린다). 알림 조치는 공유 `components/alert.js`의 `action` 인자로 붙인다. 스타일은 `.alert > .button`(`components.css`, 로그인 복사 재생성).
  - 세션 id 글꼴: 제목의 id와 주소 값은 데이터 글꼴 토큰(`--type-data-*`, DESIGN 473행)을 쓰고 `overflow-wrap: anywhere`로 긴 값이 넘치지 않게 했다. 시안은 monospace다. 차이는 아래 "남은 차이" 1.
  - 상태 캡처: SCR-002 상태 53장을 다시 찍고 육안으로 봤다(아래).
  - 조회 실패 중 종료 막기: 이전 성공 데이터가 있는 조회 실패에서 세션 종료가 켜져 있었다. 스펙 §4 l.175의 가능한 행동은 명시적 재조회뿐이고, l.174 갱신 중에도 종료는 없다. `canTerminate`에 `!query.failed`를 더했다. 캡처 `shot-stale-failed`에 종료 비활성 검사를 넣었다.
  - 시험: 커밋 전 `cargo fmt --all -- --check`, `cargo xtask tokens --check`, `cargo test --test dashboard_browser`(31 통과), `cargo test --test dashboard_assets`(2 통과)는 모두 통과했다. 이 실행은 조회 실패 종료 변경 전 코드다. 최종 게이트는 변경 뒤 `06753e3`에서 다시 돌렸다.
- 1차 명령 결과(`5368dec`): `cargo fmt --all` 종료 0. `cargo xtask tokens --check` 통과. `vr-win.ps1 -Only check` ALL PASSED(check). `dashboard_assets` 2 통과. `dashboard_browser` 31 통과(바이트·레이아웃 수정 전). 수정 뒤 `session_`·`router_` 12 통과.
- 수정 라운드 명령 결과: `vr-win.ps1 -Only all`과 `vr-full.sh`가 빌드, `fmt`, clippy, 레이어 검사, `tokens --check`, 전체 시험, 시나리오 보고서, `deny`를 모두 돌렸다. 결과는 아래 "최종 HEAD 게이트".
- 상태 캡처(3.4): SCR-002 53장을 `target/dashboard-shots/scr/<폭>/<언어>-scr002-<상태>.png`에 찍었다(드라이버 `target/dashboard-shots/capture-scr2.mjs`, 목 응답, 커밋하지 않음). 정상·종료 확인은 1280·390·320 × ko·en 12장, 나머지 상태는 ko 1280·390·320과 en 1280·320(40장), 이전 데이터가 있는 조회 실패는 ko 1280 한 장이다. 기록은 `scr/scr002-capture.json`(53행, 조회 실패 종료 변경 전)과 `scr/scr002-capture-stale-failed.json`(변경 뒤 재캡처)이다. 53장 모두 `pass`, 가로 넘침 0, CSP 위반 0, `scrollWidth == clientWidth`. SCR-001과 SCR-003은 이 라운드에서 다시 찍지 않았다.
- 재촬영 12장 측정(`dash-serve` 127.0.0.1:8765, 코드 `db08863`): 가로 넘침 0, 넘치는 요소 0, CSP 위반 0, 페이지 오류 0, `lang` 일치, 제목 `VNC Relay 세션 sess_1`/`VNC Relay Session sess_1`. 방향 표식은 장마다 2개(10×10px, `rgb(98,182,200)`, `rgb(79,180,136)`). 종료 아이콘은 12장 모두 있고 라벨은 `세션 종료`/`Terminate session`이다. `.fact` 열 수는 이 줄의 측정에 넣지 않았다. 당시 드라이버가 `.fact-list`를 재고 있었다. 열 수는 아래 `98b2fc1` 항목에서 `.fact` 행으로 다시 쟀다.
- 이번 라운드 재촬영(`98b2fc1`, 드라이버 `capture-scr2.mjs`, `dash-serve` 127.0.0.1:8765): 대상 없음 5장(ko 1280·390·320, en 1280·320)과 정상 6장(ko·en 1280·390·320)을 다시 찍었다. 기록은 `scr/scr002-capture-gone.json`·`scr/scr002-capture-normal.json`이다. `scr/scr002-capture.json`의 대상 없음·정상 행은 이 라운드 전 값이다. 11장 모두 `pass`, 가로 넘침 0, 넘치는 요소 0, CSP 위반 0, `scrollWidth == clientWidth`. 대상 없음 장에는 `.fact`가 없고 새로고침 라벨이 없으며 조치는 `목록으로`(ko)·`Back to list`(en)다. 정상 장은 `.fact` 5행, 1280에서 두 열, 390·320에서 한 열, 방향 표식 2개다. 열 수는 `.fact` 행의 `grid-template-columns`로 쟀다(`.fact-list`를 재면 항상 1이다).
- 상태 판정(3.4, 육안): 이번 라운드에 대상 없음 5장(ko 1280·390·320, en 1280·320)을 모두 봤고 정상 ko 1280도 봤다. 나머지 정상 장과 나머지 상태는 앞 라운드의 육안 판정이다. 이번 라운드에 바뀐 것은 `.fact` 중단점과 대상 없음 장뿐이다. 판정은 아래 표.

| 상태 | 폭·언어 | 본 것 | 시안 차이와 판단 |
| --- | --- | --- | --- |
| 정상 | 1280·390·320 ko, 1280·320 en, 390 en | 정보 카드(8칸)와 전송량 카드(4칸)가 나란히 있고, 1023.98px 이하에서 한 열이 된다. 세션 종료는 정보 카드 바닥. 머리말에 `목록으로`와 `새로고침`. | 배치는 시안과 같다. id는 데이터 글꼴이다(시안 monospace, 남은 차이 1). 방향 라벨은 스펙 표기다(남은 차이 2). |
| 종료 확인 | 정상과 같은 12장 | 대화상자 가운데, 제목에 id, 영향 문장, 초점은 `취소`. 390·320에서 버튼이 세로로 쌓여 전폭이다. | 시안과 같다. 전체 페이지 캡처에서 어두운 덮개가 첫 뷰포트 높이(900px)까지만 칠해졌고 390·320의 아래 띠가 밝다. 고정 덮개가 뷰포트에만 맞은 캡처 방식 탓이다 [INFERENCE]. 페이지 결함이 아니다. |
| 갱신 중 | ko 3폭, en 1280·320 | 이전 데이터 유지. 정보 카드 머리말에 `갱신 중`. 세션 종료 비활성. 새로고침은 누를 수 있으나 진행 중이면 요청을 보내지 않는다. | 스펙 §4 l.174의 가능한 행동에 종료가 없다. 일치. |
| 조회 실패 | ko 3폭, en 1280·320 | 이전 데이터 없음. 정보 카드 안에 빨간 `조회 실패` 알림. 세션 종료 비활성. 재조회는 머리말 새로고침. 320 폭에서 알림 아이콘이 제목 위 줄로 내려간다. | 스펙 l.175 일치. 320 아이콘 줄바꿈은 공유 `.alert-body`의 최소 폭이 카드 안에서 좁아진 결과다. 읽을 수 있어 그대로 두었다(공유 컴포넌트, SCR-001과 로그인도 같은 규칙). |
| 조회 실패, 이전 데이터 있음 | ko 1280 | 정보 카드 안 빨간 알림(이전 데이터 문장), 마지막 성공 시각, `오래된 데이터` 배지. 세션 종료 비활성. | 4번 수정 뒤 재캡처. 수정 전에는 종료가 켜져 있었다. 스펙 l.175와 맞춘 결과. 다른 폭은 찍지 않았다. |
| 종료 요청 중 | ko 3폭, en 1280·320 | 버튼 라벨이 `종료 요청 중…`이고 비활성. | 스펙 l.179 일치. |
| 종료 실패 | ko 3폭, en 1280·320 | 빨간 `종료 실패` 알림과 조치 `상태 재조회`. 종료 버튼은 다시 켜져 있다. 390·320에서 조치가 메시지 아래 줄에 붙고 아이콘 열 기준으로 왼쪽 정렬된다. | 스펙 l.180 일치. 390 정렬은 허용(공유 `.alert > .button`). |
| 종료 확인 불가 | ko 3폭, en 1280·320 | 경고 `종료 확인 불가`와 조치 `상태 재조회`. 종료 요청은 다시 보내지 않는다. | 스펙 l.180·CPY-066 일치. |
| 요청 거절 | ko 3폭, en 1280·320 | 경고 `요청 거절`(CPY-068)과 조치 `상태 재조회`. | 스펙 l.180 일치. |
| 대상 없음 | ko 1280·390·320, en 1280·320 | 중립 알림 `현재 세션을 찾을 수 없음`(CPY-059), 조치 `목록으로`(CPY-010, 목록 링크). 알림 조치와 머리말 `목록으로`는 같은 곳으로 간다. 정보 카드는 제목만 남고 본문이 비어 있다. 세션 종료 비활성. 전송량 카드와 머리말 새로고침 숨김. 포커스는 알림의 `목록으로`로 간다. | 스펙 l.177 일치(가능한 행동은 목록으로 이동). 목업과 다른 점은 남은 차이 6·7. 빈 카드 본문은 목업 프레임과 같아서 유지했다. |
| 잘못된 세션 식별자 | ko 3폭, en 1280·320 | 제목은 `세션`(CPY-005), 경고 `잘못된 세션 식별자`(CPY-067). 정보 카드 없음, 종료 없음, 목록 링크만. 요청을 보내지 않았다. | 스펙 l.178 일치. |

- 공통 관찰: 알림 조치 `상태 재조회`가 390·320에서 메시지 아래 줄로 내려가 왼쪽 정렬된다. 허용했다.
- 콘솔: `Failed to load resource: 404` 한 건이 캡처 실행의 첫 장에만 있다. 새 프로필의 첫 탐색에서 `/favicon.ico`가 404를 받는다(서버에서 `curl`로 404 확인) [INFERENCE]. 앱 오류가 아니다. 나머지 장에는 없다.
- 고친 것(수정 라운드, 요청 항목 기준. 시안에서 DESIGN 위반으로 확인된 항목):
  - 배치: 정보 카드는 `span-8`, 전송량 카드는 `span-4`다. 1023.98px 이하에서 한 열로 쌓인다(DESIGN 557행). 종료 버튼은 정보 카드 바닥(`card-foot`)에 있다.
  - 정보 행: `dl.fact-list`·`.fact` 행으로 바꿨다. 모드와 전송 경로는 중립 배지(`createBadge`, 아이콘 없음, DESIGN 626행). 주소는 monospace `fact-address` 한 줄이다(CPY-056·057). CPY-058 주소 안내를 붙였다.
  - 상태 행(`stateLabel`, `active`)을 지웠다.
  - 전송량: 방향 표식 `chart-1`(Viewer→대상)·`chart-2`(대상→Viewer)를 10×10 정사각형으로 붙였다(DESIGN 378행, `aria-hidden`). 공유 `createStat`은 바꾸지 않았고 `session.js`의 지역 `directionStat`이 맡는다. SCR-001에는 영향이 없다.
  - 종료 아이콘 `cancel`(DESIGN 623행). 렌더 중 `textContent`가 아이콘을 지우던 결함을 `.button-label`로 고쳤다.
  - 복사 키: `documentTitle`, `sessionDocumentTitle`, `peerViewer`, `peerTarget`, `stateLabel`을 지웠다. `transferTitle`(DESIGN 465행)을 넣었다. `sessionInfoTitle`(DESIGN 529행)은 남겼다.
  - 반응형: `.fact` 한 열 규칙은 단일 중단점 `max-width: 1023.98px`(DESIGN 543·546·557행) 안에 넣었다. 이전에는 `.fact`에 767px 별도 규칙이 있어 768–1023px에서 두 열이었다. 그 규칙을 지웠다. `.fact` 기본 규칙은 중단점 규칙 위로 옮겼다. 측정(`getComputedStyle`, `grid-template-columns`): 1023px 한 열(941px), 1024px 두 열(`160px 420.656px`), 1280 두 열, 390·320 한 열. 768–1023 구간은 이 라운드에서 캡처하지 않고 1023px 측정으로 확인했다. 같은 파일의 767px 구간(SCR-001 span·머리말 버튼 규칙)은 이 라운드에서 바꾸지 않았다. 4.8절에서 옮겼다.
  - 대상 없음: 알림에 조치 `목록으로`(CPY-010, `a.button.button-outline`, 목록 링크)를 넣었다. 포커스는 이 조치로 옮긴다(스펙 §6 l.211). 대상 없음 상태에서 머리말 `새로고침`은 숨겼다(스펙 §4 l.177).
  - 시험: DASH-006·007·012·014 확인을 갱신하고 `factValue` 도우미를 넣었다. DASH-015 표식(`tests/dashboard_browser.rs`의 `/// spec: dashboard.md:DASH-015 partial`)을 `session_cancel_sends_no_request_and_restores_focus_in_browser`에 달았다. 이 시험은 합성 keydown이다.
  - `pageHead`·`panelCard`·`peerCell`은 SCR-001과 공유된다. 목록 시험 `operations_*`는 두 게이트에서 모두 통과했다. SCR-001 캡처는 다시 찍지 않았다.
- 1차 라운드에서 고친 것(기록 유지): 바이트 행 누락(DASH-006, 스펙 117행, `createStat`), 요약 값 줄바꿈(`.session-facts .stat-value`), `세션 undefined`(`id` 저장), 대화상자 Tab 누출(DASH-015, `components/dialog.js`). 포커스 복귀 실패는 점검 아티팩트였다(`b.click()`은 포커스를 주지 않음).
- 남은 차이(시안 1280 ko 대조, 해소하지 않음):
  1. 제목의 세션 id: 시안은 monospace, 구현은 데이터 글꼴(DESIGN 473행)이다. 시안과 다르다. DESIGN을 따랐다.
  2. 방향 라벨: 구현은 스펙 표기 `Viewer->대상`·`대상->Viewer`(스펙 117·153–154행)를 쓴다. 시안은 `Viewer -> Server/대상`이다.
  3. CPY-058 주소 안내: 스펙에 따라 표시한다. 시안에는 없다.
  4. 구현 판단 키(Q3): `navLabel`(잠정), `limit`·`limitUnknown`(게이지 상한), `sessionInfoTitle`, `transferTitle`(영문 문구는 스펙에 없음). `dayUnit`은 DESIGN 491행 근거로 남았다.
  5. 표 접근 이름은 비어 있다. 스펙에 규칙이 없어 열린 관찰로 둔다.
  6. 대상 없음의 머리말 `새로고침`: 시안은 둔다. 스펙 §4 l.177은 대상 없음에서 가능한 행동을 목록으로 이동뿐이라고 정한다. 스펙을 따라 숨겼다. 시안과 다르다.
  7. 대상 없음 알림 제목·본문: 시안은 `대상 없음`과 `현재 세션을 찾을 수 없다. 같은 중계 ID의 다른 세션으로 바꾸지 않는다.`를 쓴다. 구현은 CPY-059 `현재 세션을 찾을 수 없음` 한 문장만 쓴다. 스펙 §9.3에 이 두 문구의 키가 없어 추가하지 않았다. 같은 중계 ID 규칙은 스펙 §5 l.200에 있고 화면 동작으로는 지킨다.
  - 해소: 새로고침 이름(사용자 지시: 머리말 CPY-008, 알림 조치 CPY-014).
- 접근성과 포커스(자동 대체, 청취 아님. 1차 관찰):
  - 포커스 표시: 포커스를 받을 수 있는 요소에 `outline: solid 2px`, `offset 2px`. 측정값만 적는다(상단 막대 rgb(255,255,255), 본문 rgb(18,53,43)). 토큰 `colors.ring`·`colors.navbar-ring`과 대조하지 않았고 대비 비율도 계산하지 않았다.
  - Tab 순서: 상단 막대(브랜드, 언어 선택, 로그아웃) → 페이지 동작 → 본문.
  - 접근성 트리: `banner`, `main`, `navigation`(`주 메뉴`/`Main menu`), 이름 있는 영역, `progressbar`(`중계 연결`, `관리 연결`), 언어 `combobox`(`언어 (Language)`), `columnheader`. 표 자체의 접근 이름은 비어 있다.
  - 대화상자 Tab 순환: 자동 시험은 합성 keydown으로 핸들러만 확인한다. 네이티브 Tab 키는 수동 실행 1회로 확인했다. 헤드리스 Chrome에서 `session.html?case=dash-026&lang=ko`를 열고 `#session-terminate`를 누르자 포커스가 `취소`로 갔다. Tab → `종료 확인` → Tab → `취소`(순환), Shift+Tab은 역방향이었다. 포커스는 열린 대화상자 밖으로 나가지 않았다. 이 수동 실행은 자동 시험에 넣지 않았다.
- 복사 키(스펙 9.3절 대조):
  - 스펙 문구와 같은 키 14개: `sessionScreen`(005), `sessionTitle`(051), `back`(010), `cancel`(011), `recheck`(014), `terminate`(060), `terminateTitle`(061), `terminateBody`(062), `terminateConfirm`(063), `terminating`(064), `terminateFailed`(065), `terminateUnknown`(066), `invalidSessionId`(067), `terminateRejected`(068). `notFound`는 CPY-059와 같다.
  - 스펙 CPY id가 있는 키: `directionViewerToTarget`·`directionTargetToViewer`(CPY-040·043), `modeLabel`(053), `transportLabel`(054), `startedLabel`(055), `viewerAddressLabel`·`targetAddressLabel`(056·057), `addressNote`(058).
  - 스펙 CPY id가 없는 키(DESIGN 근거 또는 Q3): `sessionInfoTitle`(DESIGN 529), `transferTitle`(DESIGN 465), `dayUnit`(DESIGN 491), `navLabel`(Q3 잠정).
- 바인딩: `cargo xtask dashboard-serve --port 8765`는 `127.0.0.1:8765`에서만 듣는다. `[::1]:8765`와 이 PC의 LAN 주소 3개는 연결이 거부됐다. 코드는 `xtask/src/dashboard_serve.rs`의 `TcpListener::bind(("127.0.0.1", port))`다.
- 확인하지 못한 것:
  - 3.5의 1번(로그인 화면 Tab 순서)과 실제 키 입력 전체. 네이티브 Tab은 대화상자 한 번만 수동으로 봤다.
  - 스크린 리더(3.6), Firefox·Safari·Edge·실기기(3.7)는 미실행이다.
  - 실서버 목록·상세·종료. `docs/TODO.md` 250행에서 `/api/v1/summary`, `waiting`, `sessions`, `closed-sessions`가 404다. 이 시험은 목 응답이다.
  - 실서버에서 실패·사라짐을 일으키지 않았다. 상태 53장은 목 응답이다. 이전 데이터가 있는 조회 실패는 ko 1280 한 장만 찍었다.
  - 시안 차이 7건(남은 차이 1–7)의 해소와 사람 비교(4.12).
  - 4.6의 `.sr-only`(320 뷰포트 밖), 목업 `.app` 클립, 200% 줌, 글자 크기 조정, 다크 모드는 보지 않았다. 대비 비율은 계산하지 않았다.
- 최종 HEAD 게이트(라운드 시작 HEAD `9263dcc`, 대상 HEAD `98b2fc1`; 제품·시험 코드를 바꾼 커밋이다. 이 기록 커밋은 문서만 바꾼다):
  - WSL `vr-full.sh`: ALL PASSED, 종료 0(마지막 줄 `ALL PASSED`, `WSL_EXIT=0`). 빌드, `fmt`, clippy, 레이어 검사, `tokens --check`, 전체 시험, 시나리오 보고서, `deny`가 모두 종료 0. 시나리오 150건: 통과 24, 부분 49, 실패 0, 미실행 0, 미연결 77, 오류 0건. dashboard_browser 31 passed, 0 failed(92.60초). dashboard_assets 2 passed. 보고서 `/home/kaydenkim/vnc-relay-target/scenario-report.md`.
  - Windows `vr-win.ps1 -Only all`: ALL PASSED (all), 종료 0(마지막 줄 `ALL PASSED (all)`, `WIN_EXIT=0`). 같은 항목이 모두 종료 0. 시나리오 150건: 통과 23, 부분 49, 실패 0, 미실행 1(OPS-007, Unix 전용), 미연결 77, 오류 0건. dashboard_browser 31 passed, 0 failed(197.67초). dashboard_assets 2 passed. 보고서 `D:\Workspace\vnc-relay\target\scenario-report.md`.
  - 시나리오 합계는 `06753e3`과 같다(통과·부분·미실행·미연결).
- 이전 게이트(대상 HEAD `06753e3`; 기록 유지):
  - WSL `vr-full.sh`: ALL PASSED, 종료 0. 빌드, `fmt`, clippy, 레이어 검사, `tokens --check`, 전체 시험, 시나리오 보고서, `deny`가 모두 종료 0. 시나리오 150건: 통과 24, 부분 49, 실패 0, 미실행 0, 미연결 77, 오류 0건. dashboard_browser 31 passed, 0 failed(92.22초). dashboard_assets 2 passed. 보고서 `/home/kaydenkim/vnc-relay-target/scenario-report.md`.
  - Windows `vr-win.ps1 -Only all`: ALL PASSED (all), 종료 0. 같은 항목이 모두 종료 0. 시나리오 150건: 통과 23, 부분 49, 실패 0, 미실행 1(OPS-007, Unix 전용), 미연결 77, 오류 0건. dashboard_browser 31 passed, 0 failed(140.44초). dashboard_assets 2 passed. 보고서 `D:\Workspace\vnc-relay\target\scenario-report.md`.
  - 시나리오 합계는 `8975537`과 같다(통과·부분·미실행·미연결). DASH 세부 판정은 이번 기록에서 따로 대조하지 않았다.
- 직전 게이트(대상 HEAD `8975537`; 코드는 `db08863`과 같고 `8975537`은 문서만 바꾼다. 기준 게이트는 4.6절 `3b3a179`. 기록 유지):
  - WSL `vr-full.sh`: ALL PASSED, 종료 0. 시나리오 150건: 통과 24, 부분 49, 실패 0, 미실행 0, 미연결 77, 오류 0건. dashboard_browser 31 passed, 0 failed(87.67초). 보고서 `/home/kaydenkim/vnc-relay-target/scenario-report.md`. 소요 173.26초.
  - Windows `vr-win.ps1 -Only all`: ALL PASSED (all), 종료 0. 시나리오 150건: 통과 23, 부분 49, 실패 0, 미실행 1(OPS-007, Unix 전용), 미연결 77, 오류 0건. dashboard_browser 31 passed, 0 failed(143.11초). 보고서 `D:\Workspace\vnc-relay\target\scenario-report.md`. 소요 238.19초.
  - DASH 27건(Windows 보고서): 통과 0, 부분 26, 미연결 1(DASH-023). 이전 미연결 2건 중 DASH-015는 표식으로 부분이 됐다. DASH-023은 이번 작업 대상이 아니다.
  - 1차 게이트(`5368dec`, 기록 유지): WSL 통과 24, 부분 48, 미연결 78. Windows 통과 23, 부분 48, 미실행 1, 미연결 78. dashboard_browser 31 passed(두 환경).

### 4.8 검토 기록 (작업 4.10·4.11 중단점 통합)

- 범위: 767px 블록을 지우고 그 규칙을 `(max-width: 1023.98px)` 블록 하나에 합쳤다. 배치 규칙(`.grid`, `.span-*`, `.fact-list`, `.fact`, `.page-actions`)은 `app/styles/components.css`로 옮겼다. `app.css`는 `components.css` 뒤에 로드되므로 배치 기본 규칙을 거기 두면 중단점 규칙을 덮는다. `app.css`에는 `@media`가 없다. 로그인 복사본(`login/styles/components.css`)은 `cargo xtask tokens`로 다시 만들었다.
- 바뀐 동작(768–1023px만): 이전 HEAD에서 SCR-001 중계·관리 연결 카드(`span-3`)는 이 구간에서 반폭(`span-6`)이었다. 이제 전체 폭이다. `.page-actions`는 이 구간에서 전체 폭이 되고 `.button`은 `flex: 1`을 받는다. 1024px 이상과 390px 이하는 바뀌지 않았다.
- 측정(목 응답, 헤드리스 Chrome, `getComputedStyle`). 카드 수는 격자 열 시작 위치의 수, `.fact` 수는 `grid-template-columns` 트랙 수다. 측정한 모든 폭에서 `scrollWidth`=`clientWidth`(가로 넘침 없음):

| 화면 | 768 | 1023 | 1024 | 1280 | 390 |
| --- | --- | --- | --- | --- | --- |
| SCR-001 카드 열 | 1 | 1 | 3 | 3 | 1 |
| SCR-002 카드 열 | 1 | 1 | 2 | 2 | 1 |
| SCR-002 `.fact` 트랙 | 1 | 1 | 2 | 2 | 1 |

- 로그인 390, 미리보기 390: `scrollWidth`=`clientWidth`=390, 가로 넘침 없음.
- 스크린샷(`target/dashboard-shots/765/`, git 무시): SCR-001 1023·390, SCR-002 1023·390, 로그인 390, 미리보기 390을 한 장씩 봤다. 미리보기는 전체 15474px 중 첫 1600px만 찍었다(전체는 읽을 수 없을 만큼 작았다). 1023 화면은 카드가 한 열이고, 중계·관리 연결 카드와 세션 종료 단추가 전체 폭이며, `.fact`가 한 열이다. 390 화면은 모두 한 열이다. 로그인 카드는 전체 폭이다.
- 콘솔: `scr1@768`에 `Failed to load resource: 404` 한 건이 있다. 첫 탐색의 `/favicon.ico`다(387행과 같다). 나머지 실행에는 없다.
- 확인하지 않은 것: 200% 줌, 글자 크기 조정, 다크 모드, 실기기 390. `web/dashboard/preview/preview.css`의 `@media (min-width: 1024px)`(15·61행)는 미리보기 전용이라 바꾸지 않았다. DESIGN 546행은 값을 "1024px"로 쓰고, 코드는 `max-width: 1023.98px`를 쓴다. 정수 픽셀에서 같은 구간을 가른다.
- 게이트(대상 HEAD `0ca9466`; 제품 코드를 바꾼 커밋이다. 기준 HEAD `a0907b1`, 이전 제품 코드 `98b2fc1`. 게이트는 커밋 뒤에 돌렸다. 이 기록 커밋은 문서만 바꾼다):
  - WSL `vr-full.sh`: ALL PASSED, 종료 0(마지막 줄 `ALL PASSED`). `fmt`, clippy, 레이어 검사, `tokens --check`, 시나리오 보고서, `deny`가 모두 종료 0. 시나리오 150건: 통과 24, 부분 49, 실패 0, 미실행 0, 미연결 77, 오류 0건. dashboard_browser 31 passed, 0 failed(93.80초). dashboard_assets 2 passed.
  - Windows `vr-win.ps1 -Only all`: ALL PASSED (all), 종료 0(마지막 줄 `ALL PASSED (all)`). 같은 항목과 라이선스 검사가 모두 종료 0. 시나리오 150건: 통과 23, 부분 49, 실패 0, 미실행 1(OPS-007, Unix 전용), 미연결 77, 오류 0건. dashboard_browser 31 passed, 0 failed(166.78초). dashboard_assets 2 passed.
  - 시나리오 합계는 `98b2fc1`과 같다(통과·부분·미실행·미연결).

### 4.9 검토 기록 (작업 4.11 세션 시험 보강, 2026-10-22)

- 검토자: 에이전트 Dash411(작업 위임: Main). 사람의 검토가 아니다. 사람 검토는 작업 4.12에서 한다.
- 범위: SCR-002 목 응답 시험 `web/dashboard/tests/session-scenarios.js`에 사례 5개(`dash-016`, `dash-019`, `dash-022`, `dash-024`, `dash-025`)를 더했다. `tests/dashboard_browser.rs`에는 `session_` 시험 5개를 더해 9개에서 14개가 됐다. 표식은 모두 `/// spec: dashboard.md:DASH-0xx partial`이다. 제품 코드(`web/dashboard/app/`)는 바꾸지 않았다.
- 시험 내용:
  - DASH-016 `session_shows_state_change_on_next_poll_in_browser`: 다음 조회에서 바이트 값이 바뀌어 보인다(1536→4096은 `4.0 KiB`, 1048575→2097152는 `2.0 MiB`). 마지막 상세 조회는 폴링 표시(`X-Relay-Poll`)를 보냈다. 머리말 `.page-meta`는 마지막 성공 시각 문구다. 바깥 단추는 새로고침·세션 종료·로그아웃뿐이고 입력 칸은 없다. 두 번째 마운트에서 조회가 `404`면 대상 없음 문구가 나오고 종료 단추가 꺼지며 종료 요청이 없다.
  - DASH-019 `session_sign_out_removes_data_or_shows_unconfirmed_in_browser`: 로그아웃을 두 번 눌러도 DELETE는 한 번이고 CSRF 토큰은 `csrf-token-1`이다. 성공하면 `/login/`으로 가고 화면 내용을 비운다. 응답 유실·`500`·`403`은 `/login/?notice=logout-unconfirmed`로 간다. `401`은 `/login/`으로 간다. 어느 경우에도 종료 요청은 없다.
  - DASH-022 `session_polls_at_configured_interval_without_overlap_in_browser`: 서버가 준 `poll_secs` 3이면 간격이 2950–3750 ms다. `poll_secs` 2이면 1950–2750 ms다. `poll_secs` 1로 1500 ms 걸리는 응답을 받아도 동시 조회는 최대 1개이고 간격은 2400 ms 이상이다.
  - DASH-024 `session_follows_the_browser_language_in_browser`: 선호 언어 `ko-KR`은 ko, `en-US`는 en, `ja-JP`만 있으면 en, `ja-JP`·`ko`면 ko다. `lang`, 언어 선택 값, 제목, 새로고침 문구, 시작 시각 형식, 바이트 값(`1.5 KiB`·`1.0 MiB`)을 확인했다. 저장소에는 아무것도 쓰지 않았다.
  - DASH-025 `session_keeps_the_chosen_language_in_browser`: 언어 선택을 바꾸면 상세를 다시 읽지 않고 바로 바뀐다. 저장한 값은 선호 언어 `ko-KR`보다 우선한다. 로그아웃 뒤에도 저장한 값이 남는다. 어떤 요청에도 언어 값이 없다. 알 수 없는 값 `mode9`·`quic`은 ko·en 모두 원문으로 보인다.
- 설계 판단:
  - 저장 키 `vnc-relay.language`(`language.js`의 `STORAGE_KEY`)는 SCR-001·SCR-003과 같다. 사례마다 가짜 저장소를 따로 둬서 한 사례의 선택이 다른 사례로 새지 않는다. 화면 사이 이동(해시 라우터)에서 언어가 이어지는지는 이 시험이 보지 않는다.
  - DASH-024·025는 SCR-002 쪽만 여기서 `partial`이다. SCR-003 쪽은 `login_keeps_the_chosen_language_and_account_name_in_browser`(4.9 로그인)다.
  - SCR-002에는 상태 행이 없어서 DASH-016의 "상태 변화"는 바이트 값과 조회 결과(사라짐 포함)로 본다.
  - 로그아웃 `403` 처리는 잠정이다(개요 D2, 사용자 질문 Q2가 열려 있다). 시험은 잠정 처리를 확인하며, 사용자가 결정을 바꾸면 이 사례도 바꾼다.
- 확인하지 않은 것: 실서버의 `poll_secs`와 폴링 간격, 실서버의 로그아웃 `403`, SCR-001과 SCR-002 사이 이동 뒤의 언어 유지, 키보드·스크린 리더(3.5·3.6), 사람 비교(4.12).
- 게이트(대상 HEAD `d0ef8d0`; 제품 코드 마지막 변경은 `aa0661f`이고 `d0ef8d0`은 서식만 바꿨다. 게이트는 `d0ef8d0`과 같은 작업 트리에서 돌렸다. 이 기록 커밋은 문서만 바꾼다):
  - WSL `vr-full.sh`: ALL PASSED, 종료 0(마지막 줄 `ALL PASSED`). fmt, build, version, clippy, 레이어 검사, `tokens --check`, 시나리오 보고서, `deny`가 모두 종료 0. 시나리오 150건: 통과 24, 부분 49, 실패 0, 미실행 0, 미연결 77, 오류 0건. dashboard_browser 36 passed, 0 failed(107.39초). dashboard_assets 2 passed.
  - Windows `vr-win.ps1 -Only all`: ALL PASSED (all), 종료 0(마지막 줄 `ALL PASSED (all)`). 같은 항목과 라이선스 검사가 모두 종료 0. 시나리오 150건: 통과 23, 부분 49, 실패 0, 미실행 1(OPS-007, Unix 전용), 미연결 77, 오류 0건. dashboard_browser 36 passed, 0 failed(199.41초). dashboard_assets 2 passed.
  - 시나리오 합계(통과·부분·미실행·미연결)는 `0ca9466` 기록과 같다.
  - 게이트 실패 기록: `5e290d8`의 첫 게이트는 실패했다. Windows에서 부하가 걸릴 때 `browser did not exit within 60 s` 10건이 났고, WSL에서는 DASH-022 P=3 사례가 `waitFor`의 벽시계 예산 때문에 실패했다.
  - 수정(`aa0661f`): 브라우저 동시 실행을 4개로 제한했다(`tests/dashboard_browser.rs`의 `BrowserSlot`, `run_chromium` 첫 줄). 제한 시간 60초와 시험 단언은 바꾸지 않았다. `web/dashboard/tests/session-scenarios.js`의 `waitFor`는 `Date.now()` 대신 페이지 시간 단계(20 ms씩 `wait`)로 셌다. 정상 속도에서는 같은 의미다. `operations-scenarios.js`는 바꾸지 않았다.
  - `aa0661f`의 게이트는 `cargo fmt --check`만 실패했다(`BrowserSlot` 잠금 호출 줄바꿈). `d0ef8d0`이 그 줄바꿈만 고쳤고, 이 기록은 그 뒤 전체 재실행 결과다.

### 4.10 검토 기록 (작업 4.11 라우터 언어 시험, 2026-10-22)

- 검토자: 에이전트 Dash411(작업 위임: Main). 사람의 검토가 아니다. 사람 검토는 작업 4.12에서 한다.
- 범위: 해시 라우터 목 응답 시험 `web/dashboard/tests/router-scenarios.js`에 사례 `dash-025`를 더했다. `tests/dashboard_browser.rs`에는 `router_keeps_the_chosen_language_across_screens_in_browser`(`/// spec: dashboard.md:DASH-025 partial`)를 더했다. 제품 코드(`web/dashboard/app/`)는 바꾸지 않았다. `docs/TODO.md`의 4.9~4.11 상태 3줄을 이 시험에 맞춰 고쳤다(커밋 `48918d3`).
- 시험 내용:
  - DASH-025 `router_keeps_the_chosen_language_across_screens_in_browser`(1280 px 한 폭): 실제 `mountRouter`에 가짜 저장소와 선호 언어 `ko-KR`을 주고 SCR-001에서 시작한다. 처음 문서 언어는 ko다. 목록의 언어 선택을 en으로 바꾸면 제목이 영어로 바뀌고 저장 키 `vnc-relay.language`에 `en`이 남는다. 행 링크 `#/sessions/sess_1`로 SCR-002에 들어가면 문서 언어와 제목, 언어 선택 값이 en이다. 해시를 비워 목록으로 돌아와도 같다. 모든 요청의 경로와 옵션 문자열에 언어 값(ko·en 태그, `lang`, `locale`)이 없다. 라우터는 외부 주소로 이동하지 않았다(`navigate` 0회). 마지막 상태에서 `checkLayout`으로 가로 넘침이 없는지 확인했다.
  - 이 사례는 `check_localized_case`(언어 2 × 폭 6, 12회) 대신 `check_runs`로 1280 px 한 번만 돌린다.
- 설계 판단:
  - 사례는 화면 사이 이동만 본다. 로그인·새로고침·로그아웃 뒤의 언어 유지는 화면별 시험(`session-scenarios.js`, `operations-scenarios.js`, `login-scenarios.js`)에 남긴다.
  - 저장소 `Map`은 한 사례 안에서 화면들이 공유하고, 사례 사이에는 공유하지 않는다.
  - `fakeApi`의 `calls`는 `key`와 함께 경로와 옵션 문자열 `text`를 담는다(요청 검사에 쓴다). `api.count`는 그대로다. `detailLinkOf`·`backOf`는 `linkNamed`를 쓴다.
- 확인하지 않은 것: 실서버에서 로그인 뒤 언어 유지(DASH-025, 4.4·4.5 뒤), 실서버 세션 조회, 키보드·스크린 리더(3.5·3.6), 사람 비교(4.12). 4.9절의 "SCR-001과 SCR-002 사이 이동 뒤의 언어 유지"는 목 응답으로 이 절에서 확인했다.
- 게이트(대상 HEAD `48918d3`; 시험 코드 마지막 변경은 `a917eb1`이고 `48918d3`은 `docs/TODO.md`만 바꿨다. 게이트는 이 HEAD에서 돌렸다. 이 기록 커밋은 문서만 바꾼다):
  - WSL `vr-full.sh`: ALL PASSED, 종료 0(마지막 줄 `ALL PASSED`). build, version, fmt, clippy, 레이어 검사, `tokens --check`, 시나리오 보고서, `deny`가 모두 종료 0. 시나리오 150건: 통과 24, 부분 49, 실패 0, 미실행 0, 미연결 77, 오류 0건. dashboard_browser 37 passed, 0 failed(105.81초). dashboard_assets 2 passed.
  - Windows `vr-win.ps1 -Only all`: ALL PASSED (all), 종료 0(마지막 줄 `ALL PASSED (all)`). 같은 항목과 라이선스 검사가 모두 종료 0. 시나리오 150건: 통과 23, 부분 49, 실패 0, 미실행 1(OPS-007, Unix 전용), 미연결 77, 오류 0건. dashboard_browser 37 passed, 0 failed(198.24초). dashboard_assets 2 passed.
  - 시나리오 합계는 `d0ef8d0` 기록(4.9절)과 같다. DASH-025는 이미 `partial`이라 합계가 바뀌지 않았고, `dashboard_browser` 시험 수만 36에서 37이 됐다.
  - 실행 기록: 첫 WSL 실행은 기본 배포판 `docker-desktop`에서 돌아 `bash`를 찾지 못했고 종료 127로 끝났다. 게이트 본체는 돌지 않았다. `wsl -d Ubuntu`로 같은 HEAD에서 다시 돌렸다. Windows 실행은 같은 HEAD에서 한 번 끝났고 그 결과를 그대로 쓴다.

### 4.11 검토 기록 (작업 4.12, 2026-10-22)

- 검토자: 에이전트 Dash411(작업 위임: Main). 사람의 검토가 아니다. 사람 검토는 작업 4.12에서 한다. 사람 확인이 남아 TODO 4.12는 `[ ]`로 둔다.
- 수정 커밋 2건. 각 커밋은 한 주제만 바꿨다.
  - `5cfe4c9` 종료 확인 대화상자(SCR-002): 긴 세션 ID가 설명문 밖으로 넘쳤다. `.dialog-description`에 `overflow-wrap: anywhere;`를 더했다. 원본 `web/dashboard/app/styles/components.css`와 생성 복사본 `web/dashboard/login/styles/components.css`에 같은 4줄을 넣었다.
  - `50ae0ba` 서버 요약(SCR-001): 확인 불가·조회 중 값을 큰 글씨 대신 배지로 보인다(DESIGN.md 639행). `web/dashboard/app/components/stat.js`에 선택 옵션 `badge`를 더했고, `web/dashboard/app/panels.js`의 `summaryGrid`가 이 두 값에만 옵션을 켠다. 실행 상태 "준비됨"은 바꾸지 않았다(결정 1).
- 게이트(대상 HEAD `50ae0ba`, 이 기록 커밋은 문서만 바꾼다):
  - WSL `vr-full.sh`: ALL PASSED, 종료 0. dashboard_browser 37 passed(146초). dashboard_assets 2 passed. 시나리오 150건: 통과 24, 부분 49, 실패 0, 미실행 0, 미연결 77. 보고서 `/home/kaydenkim/vnc-relay-target/scenario-report.md`.
  - Windows `vr-win.ps1 -Only all`: ALL PASSED (all), 종료 0. dashboard_browser 37 passed(411초). dashboard_assets 2 passed. 시나리오 150건: 통과 23, 부분 49, 실패 0, 미실행 1(OPS-007, Unix 전용), 미연결 77. 보고서 `D:\Workspace\vnc-relay\target\scenario-report.md`.
- 캡처(수정 뒤, 4.6절과 같은 구성: 1280·390·320 × ko·en × 상태 8종, 48장). 브라우저는 Microsoft Edge 155.0.4283.62다. 캡처 프로필의 `Last Version`과 설치본 버전이 같다. 산출물은 `target/dashboard-shots/edge-post412/`이며 git이 무시한다.
  - 가로 넘침: 48장 모두 `scrollWidth`와 `clientWidth`가 같았다. 넘치는 요소 0.
  - CSP 위반 0건. 시험 페이지의 `data-result`는 `pass`였다. 나머지 18장은 harness 상태(조회 중, 전체 실패, 전체 이전 데이터 실패)라 판정 값이 없다.
  - 콘솔: `/favicon.ico` 404가 2장에 있다. 4.6절과 같은 정적 서버 부산물이다.
  - 목업 프레임 21장: 프레임 가로 넘침 0.
  - 상단 막대: 높이 56px(폭 3개 모두). 언어 선택 높이는 1280에서 36px, 390·320에서 44px로 목업과 같다. 막대 인디케이터 위치는 측정하지 않았다.
  - 화면 글자 69건은 `summary.json`에 있다. 판정 값은 없다.
- 확인하지 않은 것(사람 확인 필요):
  - 시안과 사람 비교(4.12 본문): 하지 않았다. 4.2·4.3·4.6·4.7절의 차이 목록을 근거로 사람이 판정한다.
  - Firefox, Safari, 실기기(3.7): 이 환경에 없어 미실행. Edge에서는 3.4 캡처만 했고 3.5 키보드는 하지 않았다.
  - 스크린 리더(3.6): 미실행.
  - 키보드(3.5): 로그인 화면 Tab 순서(1번)와 실제 키 입력 전체는 확인하지 않았다.
  - 320 폭 언어 선택 밑줄 가까움(4.4·4.6절 관찰): 재확인하지 않았다.
  - 320 전체 페이지 캡처는 세로 16,170px이라 육안으로 읽지 않았다. 측정치만 썼다.
  - `.sr-only` 라벨은 320에서 오른쪽 366px 위치다. 시각적으로 숨긴 요소라 넘침 계산에서 뺐다. 3.6에서 함께 본다.
- 결정 요청(Main, 구현 판단 보류):
  - 결정 1, 실행 상태 "준비됨" 배지. DESIGN.md 391행은 서버 실행 상태 `ready`에만 배지를 쓰라고 한다("평소 값은 배지를 붙이지 않는다. 서버 실행 상태만 배지로 보인다"). 639행도 "실행 상태는 배지"라고 한다. 303행("정상 값에는 배지를 붙이지 않는다")의 예외다. 현재 구현은 "준비됨"을 큰 글씨 값으로 둔다. 391·639를 따르려면 `panels.js`의 실행 상태 칸에 배지 옵션을 켜고 게이트를 다시 돌려야 한다. 이 기록은 바꾸지 않았다. 권장: 배지를 붙인다.
  - 결정 2, SCR-003 요청 실패 뒤 비밀번호 칸을 비울지. 4.2절은 401 흐름만 적었다. 요청 실패 흐름은 가짜 응답 시험만 했다.
  - 결정 3, SCR-003 로그인 카드의 구분선과 브랜드 문구가 시안과 다르다. 어느 쪽을 따를지 정해야 한다.
  - 결정 4, SCR-003 알림 제목(4.2절 `scr003-failed`·`scr003-request-failed`)과 입력 문구(`scr003-invalid`)는 4.2절의 구현 판단을 따랐다. 확인 대상이다.
  - 결정 5, `/favicon.ico` 404: 앱 아이콘을 둘지 정해야 한다.
- 결과: 자동 게이트 통과(WSL·Windows), 수정 커밋 2건. 사람 확인(3.5 일부, 3.6, 3.7)과 시안 사람 비교가 남았다. TODO 4.12는 `[ ]`다.

### 4.12 검토 기록 (작업 4.12 결정 반영, 2026-10-22)

- 검토자: 에이전트 Finish412(작업 위임: Main). 이전 작업자의 미커밋 변경을 이어받아 검토·수정·캡처·검증·커밋을 맡았다. 추가 확인은 2026-10-22~23에 했다. 사람의 검토는 아니다. TODO 4.12는 `[ ]`로 둔다.
- 기준 커밋: `c4b68ac`. 검토 범위는 실행 상태 배지(SCR-001), 로그인 카드(SCR-003), 대상 없음 상태의 종료 버튼 시험이다. 이전 날짜의 기록은 고치지 않았다.
- 결정과 근거:
  - 결정 1 — 구현 판단(사용자 결정 없음), 기준 적용. 실행 상태는 항상 배지로 보인다([DESIGN.md](../design/DESIGN.md) 387~395·636·639행). `ready`는 성공, `shutdown_grace`는 경고, 나머지는 중립이다. 검토 중 `createStat`가 배지를 직접 만들어 상태 아이콘을 빠뜨린 것을 찾았다. 공통 `createBadge`를 호출하게 고쳤다. 이제 레이블, 작은 사각형, 색이 있고 성공·경고에는 아이콘도 있다. DASH-001·016에서 레이블·색 종류·아이콘을 확인한다.
  - 결정 2 — 구현 판단(사용자 결정 없음). 요청 실패(CPY-077)에서는 비밀번호를 유지한다. [화면 스펙 4절](../specs/dashboard.md#4-화면-상태와-갱신)의 182행은 인증 실패와 구분하고 "입력 수정·명시적 재시도"를 제공한다. 인증 실패(401, CPY-074)에서만 비운다(181행, DASH-018). 기존 DASH-027은 서버 오류 뒤 비밀번호 유지를 이미 확인했다. 응답 없음·제한 시간 초과의 유지 검사도 더했다. Edge 실제 키 입력에서 요청 실패 뒤 포커스가 본문으로 빠지는 결함을 찾았다. 비밀번호는 그대로 두고 다시 활성화한 [로그인]으로 포커스를 돌리게 고쳤다. 세 실패 경로의 포커스도 DASH-027에서 확인한다.
  - 결정 3 — 구현 판단(사용자 결정 없음), 기준 적용. 채택된 [SCR-003 시안](../design/mockups/dashboard.html)의 브랜드 표식과 `card`·`card-head`·`card-body`를 따랐다. 카드 위의 중복 제품 문구를 지웠다. 제품 이름은 제목 CPY-006에 남고, 제목 아래 구분선은 공통 카드 스타일이 맡는다. 추가 대조에서 언어 선택이 화면 맨 오른쪽에 있던 차이도 고쳤다. 이제 언어 선택 → 브랜드 표식 → 카드 순서이며 언어 선택의 오른쪽 끝은 카드와 같다(DESIGN.md 498·530행). 사이 간격은 시안과 같은 `spacing.lg`다. 문구는 스펙 9절의 CPY 값만 쓴다. 로그인 구성 요소·스타일은 [컴포넌트 설계](../architectures/dashboard-components.md)와 프런트엔드 컨벤션의 생성 복사본을 쓴다. `cargo xtask tokens`를 실행했다.
  - 결정 4 — 사용자 열린 질문. SCR-003 안내 제목(`scr003-failed`·`scr003-request-failed`)과 `scr003-invalid` 문구는 바꾸지 않았다. 현재 구현은 CPY-074·077 전체 문장을 제목 한 줄 역할로 쓴다. 시안의 짧은 제목과 본문 분리, 빈 입력 문구의 차이는 사람의 판단 대상으로 남긴다.
  - 결정 5 — 구현 판단(사용자 결정 없음). favicon을 배포하지 않는다. 스펙과 DESIGN.md에 favicon 항목이 없고 `/favicon.ico` 404는 화면 표시를 막지 않는다. 아이콘 파일이나 제공 경로를 추가하지 않았다.
- 시험과 구조 문서:
  - DASH-007(상세 조회 전·갱신 중), DASH-010·011·016은 대상 없음 상태에서 `#session-terminate`가 DOM에 있고 `disabled === true`인지 확인한다. 버튼이 없어도 통과하던 검사를 고쳤다(스펙 177행의 "종료 비활성").
  - 로그인 시험의 폭별 검사에 공통 카드의 머리·본문과 언어 선택의 오른쪽 정렬을 더했다.
  - 컴포넌트 설계의 `createStat` 옵션과 `badge.js` 의존을 갱신했다. 코드 지도는 기존 파일 위치·진입점·화면 모듈 경계가 그대로여서 바꾸지 않았다.
- 로그인 재캡처:
  - 환경: Windows 10.0.26200 x64, Chrome 155.0.8059.84, Edge 155.0.4283.62, 시스템 글꼴, 높이 900px, 배율 1, 라이트 테마. 설치 경로와 제품 버전을 확인했다. Edge 경로는 `C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe`다.
  - `cargo xtask dashboard-serve --port 18124`를 `127.0.0.1`에서만 실행했다. 실제 `/login/`을 열고 CDP로 로그인 응답 401·500·200을 제공했다. 실제 인증 서버 시험은 아니다. 언어 선택으로 ko·en을 바꿨다.
  - Chrome: 1280·390·320 × ko·en × 정상(CPY-075)·인증 실패(CPY-074)·요청 실패(CPY-077), 18장. Edge: 1280·390 × ko × 같은 상태, 6장.
  - 시안은 같은 폭에서 `scr003-input`·`scr003-input-en`·`scr003-failed`·`scr003-request-failed` 프레임만 본문에 남겨 12장을 찍었다. 시안 파일이나 CSS는 바꾸지 않았다. 영어 실패 프레임은 없으므로 한국어 프레임의 형태와 스펙 9절의 영어 문구를 따로 대조했다.
  - 산출물: `target/dashboard-shots/login-412/`의 PNG 36장과 `summary.json`. 커밋하지 않는다. 임시 캡처 스크립트와 브라우저 프로필은 지웠고 미리보기 서버를 종료했다.
  - 에이전트가 구현 24장과 시안 12장을 열어 보았다. 아래 표는 세 상태를 모두 본 결과다. 카드·브랜드·구분선·전체 폭 입력과 버튼이 보였다. 가려지거나 잘린 문구는 없었다.

| 폭·언어 | 정상 | 인증 실패(CPY-074) | 요청 실패(CPY-077) | 시안과의 차이·근거 |
| --- | --- | --- | --- | --- |
| 1280·ko, Chrome·Edge | 400px 카드, 제목 한 줄, 인증 필요 안내 | 계정명 유지, 비밀번호 비움, 비밀번호 포커스 링 | 비밀번호 가림 표시 유지, 로그인 포커스 링 | 시안처럼 표식만 카드 위에 있고 구분선이 있다. 정상 안내는 스펙 CPY-075에 따라 추가된다 |
| 1280·en, Chrome | 400px 카드, 제목 두 줄 | 공통 실패 문구 두 줄 | 요청 실패 문구 두 줄 | 시안의 `admin sign-in`·`Username` 대신 CPY-006·069의 `administrator sign in`·`Account name`을 쓴다 |
| 390·ko, Chrome·Edge | 358px 카드, 제목 한 줄 | 실패 문구 두 줄, 입력·버튼 유지 | 요청 실패 문구 두 줄, 입력·버튼 유지 | 언어 선택이 카드 오른쪽에 맞는다. 머리·본문 여백은 공통 모바일 카드 스타일이다 |
| 390·en, Chrome | 제목 두 줄, 안내 한 줄 | 실패 문구 두 줄 | 요청 실패 문구 두 줄 | 긴 영어 제목은 말줄임 없이 줄바꿈한다(DESIGN.md 464행) |
| 320·ko, Chrome | 288px 카드, 제목 두 줄 | 실패 문구 세 줄, 비밀번호 포커스 유지 | 요청 실패 문구 세 줄, 로그인 포커스 유지 | 시안의 16px 제목과 달리 24px다. 로그인 카드 제목을 페이지 제목으로 정한 DESIGN.md 464행을 따랐다 |
| 320·en, Chrome | 제목 두 줄, 안내 두 줄 | 실패 문구 세 줄 | 요청 실패 문구 세 줄 | 두 입력과 버튼이 카드 안에 남는다. 영어 문구는 스펙 9절을 따른다 |

- 공통 차이: 시안은 여러 상태를 나열한 프레임이고, 구현은 높이 900px의 화면 가운데에 로그인 묶음을 둔다(DESIGN.md 498·530행). 제목은 모든 폭에서 24px이며 시안의 16px보다 크다(DESIGN.md 464행). 안내의 제목·본문 분리 차이는 결정 4로 남긴다.
- 측정: 구현 24장 모두 `scrollWidth === clientWidth`였고 CSP 위반은 0건이다. 언어 선택과 카드의 오른쪽 끝도 모두 같았다. 카드 폭은 1280에서 400px, 390에서 358px, 320에서 288px다. 언어 선택 높이는 데스크톱 36px, 모바일 44px이며 경계가 전부 보였다. 시안 12개 프레임도 가로 넘침이 없었다.
- 새 로그인 키보드 확인(Edge 155.0.4283.62, 1280·390, ko):
  - CDP `Input.dispatchKeyEvent`로 실제 Tab·Enter를 보냈다. 합성 DOM `keydown`으로 대신하지 않았다.
  - Tab 순서: 언어 선택 → 계정명 → 비밀번호 → 로그인. 네 요소 모두 `:focus-visible`이 참이고 실선 포커스 링이 보였다.
  - 빈 입력에서 Enter: 로그인 요청 0건. 두 입력 오류를 보이고 계정명으로 포커스를 옮겼다.
  - 401에서 CPY-074: 요청 1건, 계정명 유지, 비밀번호 길이 0, 비밀번호에 포커스.
  - 500에서 CPY-077: 시험 비밀번호 길이 20을 유지했고 로그인으로 포커스가 돌아왔다. Enter로 명시적 재시도를 보냈다. 값 자체는 기록하지 않았다.
  - 200 뒤 `/` 탐색 요청을 확인했다. 개발 미리보기 서버가 `/`를 `/preview/index.html`로 다시 보내므로 최종 주소는 그 미리보기 경로였다. 이를 제품 화면의 로그인 후 이동 시험으로 확대 해석하지 않는다.
- 이전 작업자가 CDP로 확인한 Edge 155.0.4283.62 결과(배치 변경 전 관찰이며, 이번에 전달받아 기록):
  - SCR-003: 언어 선택 → 계정명 → 비밀번호 → 제출 순서. 잘못된 자격 증명은 안내를 보이고 비밀번호 길이 0, 포커스는 비밀번호였다. 올바른 자격 증명은 `/`로 이동했다. 배치를 바꾼 뒤의 재확인은 위 항목과 구분한다.
  - SCR-001: "운영 현황" 링크 → 언어 선택 → 로그아웃 → 새로고침 → 첫 "상세" 링크 순서. 상세에서 Enter는 `#/sessions/sess_1`을 열었다.
  - SCR-002: Tab으로 [세션 종료]에 닿았다. 대화상자는 취소에 포커스를 두고 열렸다. Tab·Shift+Tab은 취소와 종료 확인 사이를 돌았다. Escape와 취소에서 Enter는 닫은 뒤 [세션 종료]로 포커스를 돌렸다. 종료 확인에서 Enter는 목 404로 대상 없음 상태가 되었고 [목록으로]로 포커스를 옮겼다. 이 상태의 [세션 종료]는 DOM에 남는다. 이번 시험은 비활성 여부를 명시적으로 확인한다.
  - 새로고침 진행 중의 비활성 상태는 목 응답이 너무 빨라 찍지 못했다. Chrome DASH-005가 조회 중첩 금지를 확인한다.
  - 긴 값: 1280·320에서 64자 세션 식별자, 96자 중계 ID, 포트가 있는 IPv6 주소, u64 최댓값 바이트를 표시했다. SCR-001·002의 페이지 가로 스크롤은 없었다. 320 대화상자는 내용 폭 305px 안의 x 12~293에 들어갔다. SCR-001의 320 카드 목록은 설계대로 `.data-table thead`를 절대 위치의 1px 요소로 숨긴다.
  - 320 언어 선택(ko-KR·en-US, 로그인·SCR-001)은 높이 44px, 전체 경계 표시, 넘침 없음이었다. 이전의 "밑줄에 가깝다"는 관찰은 해소했다.
  - Edge 시험 도구 한계: `DASHBOARD_BROWSER`를 Edge 155로 두고 `cargo test --test dashboard_browser`를 실행한 결과는 0 passed, 37 failed였다. Edge 155의 `--headless --dump-dom`이 DOM을 쓰지 않았다. 제품 실패가 아니라 시험 도구의 한계다. 시험 도구는 바꾸지 않았다.
- 코드·시험 커밋(기준 `c4b68ac` 이후, 주제별 경로 한정 커밋):
  - `91f3fc1` `fix(dashboard): render server states with shared status badges`
  - `05f4989` `fix(dashboard): align sign-in layout and restore retry focus`
  - `3cf643d` `test(dashboard): require disabled terminate actions for gone sessions`
- 커밋 전 검사(최종 코드와 같은 작업 트리, Windows, Chrome 155.0.8059.84):
  - `cargo fmt --all`: 종료 0.
  - `cargo xtask tokens --check`: 통과, 종료 0.
  - `cargo test --test dashboard_browser`: 37 passed, 0 failed, 275.19초.
  - `cargo test --test dashboard_assets`: 2 passed, 0 failed.
- 최종 게이트(2026-10-23, 최종 코드 HEAD `3cf643d`, 기준 커밋 `c4b68ac`; 뒤의 기록 커밋은 문서만 바꾼다):
  - WSL Ubuntu: `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh`. 마지막 줄 `ALL PASSED`, 종료 0. 빌드·fmt·clippy·layers·tokens·scenarios·licenses를 통과했다. `dashboard_browser` 37 passed, 0 failed(193.13초), `dashboard_assets` 2 passed. 시나리오 150건: 통과 24, 부분 49, 실패 0, 미실행 0, 미연결 77, 오류 0건. 보고서 `/home/kaydenkim/vnc-relay-target/scenario-report.md`.
  - Windows: `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all`. 마지막 줄 `ALL PASSED (all)`, 종료 0. 빌드·fmt·clippy·layers·tokens·scenarios·licenses를 통과했다. `dashboard_browser` 37 passed, 0 failed(291.35초), `dashboard_assets` 2 passed. 시나리오 150건: 통과 23, 부분 49, 실패 0, 미실행 1(OPS-007, Unix 전용), 미연결 77, 오류 0건. 보고서 `target/scenario-report.md`.
  - DASH 27건 중 26건은 목 응답 등 범위를 한정한 `부분` 연결이고 DASH-023은 `미연결`이다. 게이트 통과를 브라우저·실기기 호환성 전체 완료로 해석하지 않는다.
- 미실행과 영향:
  - Firefox·Safari: 설치되어 있지 않고 설치가 허용되지 않아 미실행이다. 두 브라우저의 호환성은 확인하지 못했다.
  - 실제 모바일 기기: 기기가 필요해 미실행이다. 모바일 폭의 에뮬레이션은 실제 기기 확인을 대신하지 않는다.
  - 스크린 리더: 사람이 읽는 순서·발음을 확인해야 하므로 미실행이다.
  - 사람의 시안 나란히 비교: 사람이 필요해 미실행이다. 에이전트의 PNG 대조는 이를 대신하지 않는다.
- 결과: 최종 코드 `3cf643d`의 WSL·Windows 게이트를 모두 통과했다. 결정 1·3을 기준에 맞췄고, 결정 2·5는 구현 판단으로 기록했다. 로그인 새 배치의 캡처와 Edge 키보드 확인도 마쳤다. 결정 4와 위 사람 확인은 남아 있다. TODO 4.12는 `[ ]`다.

## 5. 중단 조건

- `cargo xtask tokens --check`의 종료 코드가 0이 아니면 3.2 이후 단계를 실행하지 않는다.
- `DASHBOARD_BROWSER`가 없거나 `--version`이 실행되지 않으면 3.3을 실행하지 않는다.
- `data-result`가 `pass`가 아닌 테스트 페이지가 하나라도 있으면 실패로 기록하고 3.4 이후를 실행하지 않는다.
- 브라우저 네트워크 목록에 대시보드 출처 밖 요청이 있으면 중단한다. 외부 자원 금지는 [프런트엔드 컨벤션 1.3절](../conventions/frontend.md)을 따른다.
- 스크린샷에 가로 스크롤이 생기면 그 폭을 실패로 기록하고, 나머지 폭은 계속 확인한다.

## 6. 롤백 또는 복구

검증 절차는 저장소 파일을 바꾸지 않는다. 생성 산출물이 원본과 다르면 원본을 고친 뒤 `cargo xtask tokens`로 다시 쓴다(사용자 결정 2026-10-22(Q32)).

1. `target/dashboard-shots/`와 `target/scenario-report.md`를 지운다. 저장소 밖에 만든 임시 파일도 지운다.
2. `git status --short`로 `web/dashboard/`와 `docs/design/mockups/`에 의도하지 않은 변경이 없는지 확인한다. 변경이 있으면 검증 기록에 적고 전달 대상에게 넘긴다.

## 7. 해결되지 않을 때 전달할 대상

- 전달 대상: 대시보드 프런트엔드 담당자와 검토자.
- 보존할 증거: 실행한 명령과 전체 출력, `rustc --version`, 브라우저 이름과 `--version` 결과, 스크린 리더 이름과 버전, `target/scenario-report.md`, 실패한 폭과 화면 상태의 스크린샷 경로, 커밋 해시와 `git status --short` 결과
- 비밀 값 제외 규칙: 스크린샷과 로그에 비밀번호, 세션 식별자, 쿠키 값이 보이면 공유 전에 지운다. 지운 자리에 `<redacted>`를 적는다.
