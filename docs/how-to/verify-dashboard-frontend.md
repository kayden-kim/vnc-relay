# 대시보드 프런트엔드를 검증하는 방법

대시보드 프런트엔드(`web/dashboard/`)가 [프런트엔드 컨벤션](../conventions/frontend.md)과 [디자인 기준](../design/DESIGN.md)을 지키는지 확인한다. 완료 상태는 4절의 모든 행이 실제 결과와 함께 "통과"로 기록된 상태다. 수동 검토를 하지 못한 행은 "미실행"과 이유를 적고, 완료로 표시하지 않는다.

이 문서는 Draft다. 절차의 방법은 사용자 결정 2026-10-22(Q32)~(Q38)를 따른다. 작업 4.8에서 `cargo xtask tokens`, `cargo xtask dashboard-serve`, `tests/dashboard_browser.rs`를 구현했다. `tests/dashboard_assets.rs`는 작업 4.6에서 만들었다. 실행한 결과는 4절에 적는다.

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 대시보드 프런트엔드 구현자와 검토자 |
| 검증 상태 | Draft |
| 검증 환경·버전 | 작업 4.8(2026-10-22): Windows 10.0.26200 x64, PowerShell, Chrome 155.0.8059.84. Ubuntu 24.04.4 LTS x64(WSL2), Google Chrome 156.0.8078.47. rustc는 두 OS 모두 1.99.1(`rust-toolchain.toml`). 나머지 환경은 실행할 때 기록한다 |
| 최종 검증일 | 2026-10-22. 3.1 1단계, 3.2 1단계, 3.3 1단계, 3.4 일부를 실행했다(4절). 작업 4.10 재실행(4.5절, 커밋 `3944dc2`): WSL·Windows 게이트 모두 `ALL PASSED`. 3.5~3.7은 미실행 |
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
| 브라우저 테스트 | 모든 테스트 페이지의 `data-result`가 `pass` | 3.3 1단계 | 통과. 작업 4.8(Windows): `cargo test --test dashboard_browser` 시험 2개 통과(2026-10-22 재실행, 19.19초). Chrome 155.0.8059.84. Ubuntu 24.04(WSL2), 커밋 `df3b5b2`: 시험 2개 통과(10.39초), Google Chrome 156.0.8078.47(`/usr/bin/google-chrome`). 작업 4.9 재작업(4.3절): 시험 7개 통과(Windows, 27.20초). 작업 4.10(4.4절, Windows, 작업 트리): 시험 21개 통과(146.6초, 운영 화면 시험 포함). 작업 4.10 재실행(4.5절, 커밋 `3944dc2`): WSL 시험 21개 통과(61.78초), Windows 시험 21개 통과(97.16초). 기준 `61e03e9`의 WSL 실행은 시험 6개가 실패했다(4.5절) | 검증 기록의 테스트 출력과 브라우저 버전 |
| 시나리오 보고서 | 자동 연결된 DASH 항목이 `통과` | 3.3 2단계 | 부분. 작업 4.10(4.4절, 작업 트리): 150건 중 통과 23, 부분 40, 실패 0, 미실행 1, 미연결 86. DASH 27건은 통과 0, 부분 17, 미연결 10이다. 미실행 1건은 OPS-007이다. 4.9 재작업 수치(WSL 통과 18, Windows 통과 17)보다 통과가 늘었다. 늘어난 항목은 확인하지 않았다. 작업 4.10 재실행(4.5절, 커밋 `3944dc2`): WSL 통과 24, 부분 40, 실패 0, 미실행 0, 미연결 86. Windows 통과 23, 부분 40, 실패 0, 미실행 1(OPS-007), 미연결 86. DASH 세부 수치는 이 재실행에서 세지 않았다. 최종 게이트 수치는 4.5절에 있다 | `<Cargo 대상 디렉터리>/scenario-report.md` |
| 폭별 시각 대조 | 6개 폭에서 가로 스크롤 0건, 차이가 스펙·디자인 기준에 맞음 | 3.4 | 부분. 작업 4.8: 미리보기 6폭 × ko·en 12장(4.1절). 작업 4.9(2026-10-22): 로그인 화면 `/login/`을 6폭 × 7장으로 찍어 42장을 얻었다(`target/dashboard-shots/<폭>/{input-ko,invalid-ko,busy-ko,failed-ko,request-failed-ko,input-en,invalid-en}.png`). 6폭 모두 `scrollWidth`와 `clientWidth`가 같았고(320 포함), CSP 위반은 0건이었다. 콘솔 오류는 가로챈 401·500 응답의 "Failed to load resource"뿐이었다. 시안과의 차이는 4.2절. 작업 4.9 재작업(4.3절)에서 같은 42장을 다시 찍었다. 6폭 모두 가로 스크롤 0건, CSP 위반 0건이었다. 4.9 기준선과는 42장 모두 바이트가 달랐다(화면을 구성 요소로 바꾼 결과). 작업 4.10(4.4절): 실제 서버에서 `/login/`(ko 입력 화면)과 `/`(ko 운영 화면)를 6폭으로 찍었다. 가로 스크롤 0건. 320 폭의 상단 막대 언어 선택은 밑줄과 가까워 사람 확인이 필요하다. 사람 비교는 4.12에서 한다. 작업 4.10 화면 상태 대조(4.6절): SCR-001 목업 21장과 1280·390·320 × ko·en × 상태 8종 48장을 대조했다. 6폭·언어 모두 가로 스크롤 0건, CSP 위반 0건. 수정 필요 6건이 나왔다(상단 막대 5건, `Unavailable` 줄바꿈 1건). 1280은 정상 상태와 빈 목록 문구만 판정했다 |
| 키보드 | 3.5의 4개 항목 통과 | 3.5 | 미실행 | 검토 기록 |
| 스크린 리더 | 3.6의 항목 통과 | 3.6 | 미실행 | 검토 기록과 스크린 리더 이름·버전 |
| 다른 브라우저·실기기 | 3.7 통과 또는 미실행 사유 기록 | 3.7 | 미실행 | 브라우저 버전과 검토 기록 |

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
