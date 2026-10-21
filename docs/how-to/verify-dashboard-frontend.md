# 대시보드 프런트엔드를 검증하는 방법

대시보드 프런트엔드(`web/dashboard/`)가 [프런트엔드 컨벤션](../conventions/frontend.md)과 [디자인 기준](../design/DESIGN.md)을 지키는지 확인한다. 완료 상태는 4절의 모든 행이 실제 결과와 함께 "통과"로 기록된 상태다. 수동 검토를 하지 못한 행은 "미실행"과 이유를 적고, 완료로 표시하지 않는다.

이 문서는 Draft다. 아직 `web/dashboard/`, `tokens` 명령, 테스트 파일이 없다. 그래서 이 절차를 실행한 기록은 없다.

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 대시보드 프런트엔드 구현자와 검토자 |
| 검증 상태 | Draft |
| 검증 환경·버전 | 미실행. 실행할 때 OS, Rust 1.99.1, 브라우저와 버전, 스크린 리더와 버전을 기록한다 |
| 최종 검증일 | 없음(Draft 작성일 2026-10-22) |
| 관련 기준 문서 | [프런트엔드 컨벤션](../conventions/frontend.md), [디자인 기준](../design/DESIGN.md), [화면 스펙](../specs/dashboard.md), [관리 API 계약 1.11절](../contracts/management-api-v1.md) |

## 1. 대상과 영향

- 대상: `web/dashboard/` 정적 파일, `docs/design/DESIGN.md`의 YAML 토큰, `docs/design/mockups/dashboard.html`의 토큰 블록, 대시보드 테스트 파일(제안: `tests/dashboard_assets.rs`, `tests/dashboard_browser.rs`)
- 예상 사용자 영향: 없음. 자동 검사는 파일을 읽고 `target/` 아래에 결과를 쓴다. 운영 서버를 멈추지 않는다.
- 작업 허용 시간: 자동 검사는 서버 중단이 필요 없다. 수동 검토(3.4~3.7절)는 검토자의 시간이 필요하다.
- 이 문서는 사용자 결정 2026-10-22(Q09)에 따라 작업 0.9와 함께 초안으로 작성했다.

## 2. 전제 조건

- [ ] 검증할 커밋을 정했고, `git status --short` 결과를 검증 기록에 적었다.
- [ ] 고정 Rust 툴체인 1.99.1을 쓴다(`rust-toolchain.toml`).
- [ ] Chromium 계열 브라우저 실행 파일 경로를 환경 변수 `DASHBOARD_BROWSER`에 넣었다. 필수 여부는 [FE-TBD-006](../conventions/frontend.md#8-미결정-사항)에서 정한다.
- [ ] Firefox·Safari 수동 검토에 쓸 최신 버전을 정했다([FE-TBD-009](../conventions/frontend.md#8-미결정-사항)).
- [ ] (선택) Node.js와 고정 버전의 `@google/design.md`를 준비했다([FE-TBD-012](../conventions/frontend.md#8-미결정-사항)).
- [ ] 수동 검토자와 스크린 리더 환경을 정했다.

주의: `cargo xtask tokens`, `cargo xtask dashboard-serve`, `cargo test --test dashboard_assets`, `cargo test --test dashboard_browser`는 아직 저장소에 없다. 채택되고 구현된 뒤에만 실행한다. 지금 실행하면 실패하며, 그 실패를 대시보드 결함으로 기록하지 않는다.

## 3. 절차

- 실행 환경: Ubuntu 24.04 LTS x64는 bash, Windows 11 x64는 PowerShell을 쓴다. 지원 환경은 [개발 환경·검증 how-to 2절](set-up-and-verify.md#2-전제-조건)을 따른다.
- 실행 위치: 저장소 루트
- 바꿔 넣을 값: `DASHBOARD_BROWSER`에는 Chromium 실행 파일 경로를 넣는다. 확인하려면 경로를 직접 실행해 `--version`이 버전을 출력하는지 본다. 경로에 공백이 있으면 큰따옴표로 감싼다.
- 최대 대기 시간: 자동 명령은 10분(제안)을 넘기면 중단하고 5절을 따른다. 수동 검토에는 시간 한도가 없다.

### 3.1 토큰과 문서 확인

1. 저장된 토큰 CSS를 DESIGN.md YAML에서 다시 만든 결과와 대조한다.

   영향: 없음. 파일을 쓰지 않는다.

   ```shell
   cargo xtask tokens --check
   ```

   기대 결과: 종료 코드 0. 차이가 있으면 파일 이름과 줄이 출력되고 종료 코드가 0이 아니다.

2. (선택) DESIGN.md 린트를 실행한다.

   영향: 없음. `npx`가 패키지를 내려받는다. 버전을 고정하면 `@google/design.md@<고정 버전>`으로 바꾼다.

   ```shell
   npx -y -p @google/design.md designmd lint docs/design/DESIGN.md
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

   영향: 임시 브라우저 프로필을 만든다. 실행이 끝나면 지운다(제안).

   ```shell
   cargo test --test dashboard_browser
   ```

   기대 결과: `test result: ok.` 줄이 나온다. 각 테스트 페이지의 `data-result` 값이 `pass`다. `DASHBOARD_BROWSER`가 없거나 실행되지 않으면 테스트가 실패해야 한다([FE-TBD-006](../conventions/frontend.md#8-미결정-사항) 제안).

2. 시나리오 보고서를 갱신한다.

   영향: `<Cargo 대상 디렉터리>/scenario-report.md`를 덮어쓴다.

   ```shell
   cargo xtask scenarios
   ```

   기대 결과: 자동으로 연결된 DASH 항목의 상태가 `통과`다. `부분` 표식이 붙으면 검증 기록에 이유를 적는다. 현재 저장소에서는 DASH 25건이 모두 `미연결`이다([xtask 모듈 설계 8절](../architectures/xtask.md)).

### 3.4 폭별 스크린샷과 시안 대조 (수동)

1. 미리보기 서버를 실행한다(제안).

   영향: 루프백 주소의 포트 하나를 연다. Ctrl+C로 멈춘다.

   ```shell
   cargo xtask dashboard-serve
   ```

   기대 결과: 로컬 주소 한 줄이 출력되고 프로세스가 계속 실행된다.

2. 같은 화면 상태를 폭 1280·1024·1023·768·390·320px에서 연다. 시안 `docs/design/mockups/dashboard.html`에서 같은 상태의 프레임도 같은 폭에서 연다.

   영향: 없음. 스크린샷은 `target/dashboard-shots/<폭>/<화면 상태>.png`에 저장하고 커밋하지 않는다.

   기대 결과: 페이지에 가로 스크롤이 없다. 화면 상태는 [화면 스펙 4절](../specs/dashboard.md#4-화면-상태와-갱신)의 SCR-001·SCR-002·SCR-003이다.

3. 구현 스크린샷과 시안 스크린샷을 나란히 두고 차이를 적는다.

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

1. Firefox와 Safari에서 3.4의 2단계와 3.5 전체를 실행한다. 실행할 수 없는 브라우저는 "미실행"과 이유를 적는다.

   영향: 없음.

   기대 결과: 결과가 Chromium과 같다. 차이가 있으면 검증 기록에 적는다.

2. 실제 모바일 기기에서 390px 폭 화면을 확인한다. 릴리스 전 한 번 실행한다([FE-TBD-014](../conventions/frontend.md#8-미결정-사항) 제안).

   영향: 없음.

   기대 결과: 카드 목록에서 정보와 행동이 가려지지 않는다.

## 4. 검증

[검증 기록](../AGENTS.md#검증-기록)에 따라 실제 결과와 성공 조건을 대조한다. 이 초안을 작성한 시점의 실제 결과는 모두 미실행이다.

| 관찰 대상 | 성공 조건 | 확인 명령·방법 | 실제 결과 | 실행 증거 위치 |
| --- | --- | --- | --- | --- |
| 토큰 CSS와 저장된 파일 | 차이 0건, 종료 코드 0 | 3.1 1단계 | 미실행 | 검증 기록의 명령과 출력 |
| DESIGN.md 린트(선택) | 오류 0건, 경고·정보 수 기록 | 3.1 2단계 | 미실행 | 검증 기록의 출력 |
| 제공 파일 표 | `dashboard_assets` 테스트 통과 | 3.2 1단계 | 미실행 | 검증 기록의 테스트 출력 |
| 브라우저 테스트 | 모든 테스트 페이지의 `data-result`가 `pass` | 3.3 1단계 | 미실행 | 검증 기록의 테스트 출력과 브라우저 버전 |
| 시나리오 보고서 | 자동 연결된 DASH 항목이 `통과` | 3.3 2단계 | 미실행 | `<Cargo 대상 디렉터리>/scenario-report.md` |
| 폭별 시각 대조 | 6개 폭에서 가로 스크롤 0건, 차이가 스펙·디자인 기준에 맞음 | 3.4 | 미실행 | `target/dashboard-shots/`의 경로와 검토 기록 |
| 키보드 | 3.5의 4개 항목 통과 | 3.5 | 미실행 | 검토 기록 |
| 스크린 리더 | 3.6의 항목 통과 | 3.6 | 미실행 | 검토 기록과 스크린 리더 이름·버전 |
| 다른 브라우저·실기기 | 3.7 통과 또는 미실행 사유 기록 | 3.7 | 미실행 | 브라우저 버전과 검토 기록 |

## 5. 중단 조건

- `cargo xtask tokens --check`의 종료 코드가 0이 아니면 3.2 이후 단계를 실행하지 않는다.
- `DASHBOARD_BROWSER`가 없거나 `--version`이 실행되지 않으면 3.3을 실행하지 않는다.
- `data-result`가 `pass`가 아닌 테스트 페이지가 하나라도 있으면 실패로 기록하고 3.4 이후를 실행하지 않는다.
- 브라우저 네트워크 목록에 대시보드 출처 밖 요청이 있으면 중단한다. 외부 자원 금지는 [프런트엔드 컨벤션 1.3절](../conventions/frontend.md)을 따른다.
- 스크린샷에 가로 스크롤이 생기면 그 폭을 실패로 기록하고, 나머지 폭은 계속 확인한다.

## 6. 롤백 또는 복구

검증 절차는 저장소 파일을 바꾸지 않는다. 생성 산출물이 원본과 다르면 원본을 고친 뒤 `cargo xtask tokens`로 다시 쓴다(제안).

1. `target/dashboard-shots/`와 `target/scenario-report.md`를 지운다. 저장소 밖에 만든 임시 파일도 지운다.
2. `git status --short`로 `web/dashboard/`와 `docs/design/mockups/`에 의도하지 않은 변경이 없는지 확인한다. 변경이 있으면 검증 기록에 적고 전달 대상에게 넘긴다.

## 7. 해결되지 않을 때 전달할 대상

- 전달 대상: 대시보드 프런트엔드 담당자와 디자인 결정 담당자([DESIGN-TBD-005](../design/DESIGN.md#미결정-사항))
- 보존할 증거: 실행한 명령과 전체 출력, `rustc --version`, 브라우저 이름과 `--version` 결과, 스크린 리더 이름과 버전, `target/scenario-report.md`, 실패한 폭과 화면 상태의 스크린샷 경로, 커밋 해시와 `git status --short` 결과
- 비밀 값 제외 규칙: 스크린샷과 로그에 비밀번호, 세션 식별자, 쿠키 값이 보이면 공유 전에 지운다. 지운 자리에 `<redacted>`를 적는다.
