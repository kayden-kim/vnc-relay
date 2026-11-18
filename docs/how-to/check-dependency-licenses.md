# 의존성 라이선스를 검사하는 방법

Rust 의존성이 승인된 라이선스 허용 목록을 지키는지 검사하고, 실행파일과 함께 보낼 고지를 만든다. `cargo deny`의 허용 판정과 배포 고지의 최신 상태는 별도 검사다. 독립 구현의 출처 검토는 [4.1절](#41-출처-검토-기록-2026-11-04)에 기록한다. 프로젝트 자체 MIT 고지는 루트 [`LICENSE`](../../LICENSE)에 있다. 저작권 고지는 `Copyright (c) 2026 vnc-relay contributors`다. 이 파일은 `THIRD-PARTY-NOTICES.txt`와 별도로 배포한다. 검토 기록은 [4.2절](#42-프로젝트-license-검토-기록2026-11-07)에 있다.

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 의존성을 추가·갱신하는 개발자, 배포를 준비하는 프로젝트 관리자 |
| 검증 상태 | Verified |
| 검증 환경·버전 | Ubuntu 24.04.4 LTS x86_64(WSL2)와 Windows 11 Pro 10.0.26200, Rust 1.99.1, cargo-deny 0.20.2 |
| 최종 검증일 | 2026-11-08 — 수동 검토 대상별 날짜는 4절에 있다. 자동 검사는 현재 의존성으로 다시 실행한다. |
| 관련 기준 문서 | [PRD 확정 기준](../requirements/PRD.md#확정-기준) "구현 기술"·"라이선스" 행, [OPS-021](../specs/operations.md#7-수용-테스트), 허용 목록 [`deny.toml`](../../deny.toml) |

## 1. 대상과 영향

- 대상: 저장소 루트의 `Cargo.lock`이 가리키는 의존성 전체. `deny.toml`의 `[graph] targets`에 따라 Windows(`x86_64-pc-windows-msvc`)와 Ubuntu(`x86_64-unknown-linux-gnu`)의 의존성을 함께 검사한다. 한 OS에서 실행해도 두 대상을 모두 검사한다.
- 예상 사용자 영향: 검사와 `notices --check`는 없다. `cargo xtask notices`는 저장소 루트의 배포 파일 `THIRD-PARTY-NOTICES.txt`를 덮어쓴다. 제품 코드·의존성은 바꾸지 않는다.
- 작업 허용 시간: 제한 없다.

허용 목록은 MIT, Apache-2.0, ISC, BSD-3-Clause, Unlicense, CC0-1.0, BSL-1.0, Unicode-3.0, MIT-0의 9종이다. GPL·LGPL·MPL 등 copyleft 라이선스는 허용하지 않는다. 목록을 바꾸려면 사용자 승인이 필요하다.

허용 라이선스의 고지 의무는 다음과 같다. [3.1절](#31-배포-고지-생성과-대조)의 파일 전체를 실행파일과 함께 배포한다. `OR`는 선택 가능한 조건, `AND`는 함께 지킬 조건이다. 단순히 MIT 전문 하나만 배포해서는 모든 의존성의 조건을 충족하지 못한다.

| 라이선스 | 배포할 때 할 일 |
| --- | --- |
| MIT, ISC, BSD-3-Clause, BSL-1.0, Unicode-3.0 | 저작권 고지와 라이선스 전문을 배포물에 넣는다. BSL-1.0은 소스 배포에만 요구하지만 같은 방식으로 넣는다. |
| Apache-2.0 | 라이선스 전문을 넣는다. 의존성에 `NOTICE` 파일이 있으면 그 내용도 넣는다. |
| CC0-1.0, Unlicense, MIT-0 | 고지 의무가 없다. |

대시보드 글꼴 Pretendard 1.3.9는 Rust crate가 아니어서 `cargo deny` 검사 대상이 아니다. 라이선스는 SIL Open Font License 1.1(OFL-1.1)이고 위 허용 목록에 없다. 이 글꼴 파일을 OFL 전문과 함께 실행파일에 넣는 것은 승인된 예외다. 이 예외는 이 글꼴에만 적용하며 crate 허용 목록을 넓히지 않는다. OFL-1.1은 저작권 고지와 라이선스 전문을 함께 배포하도록 요구한다. 글꼴 파일을 고쳐 배포하면 예약 글꼴 이름 `Pretendard`를 쓸 수 없으므로 배포본 파일을 바꾸지 않고 넣는다([프런트엔드 컨벤션 2.4절](../conventions/frontend.md#24-글꼴-파일)).

운영자 도움말 사이트는 실행파일과 따로 GitHub Pages로 게시한다. 이 사이트에만 수정하지 않은 mdBook 프런트엔드 파일(MPL-2.0)과 Font Awesome Free 6.2.0 아이콘(CC BY 4.0)을 싣는다. Pretendard(OFL-1.1), highlight.js 10.1.1(BSD-3-Clause), clipboard.js 2.0.4(MIT)도 함께 싣는다. 이 구성은 승인된 예외이며 도움말 사이트에만 적용한다. crate 허용 목록을 넓히지 않는다. 구성요소마다 이름·버전·라이선스·출처를 사이트의 고지 페이지(`docs/help/ko/src/notices.md`, `docs/help/en/src/notices.md`)에 적는다. [HELP-017](../specs/help-site.md#7-수용-테스트)이 이 페이지를 확인한다. 실행파일의 `THIRD-PARTY-NOTICES.txt`에는 넣지 않는다.

## 2. 전제 조건

- [ ] Rust 툴체인을 설치했다. 버전은 저장소 루트의 `rust-toolchain.toml`이 정한다.
- [ ] 인터넷에 연결되어 있다. cargo-deny가 crates.io 색인과 crate 파일을 내려받는다.
- [ ] 저장소 루트에 `Cargo.lock`과 `deny.toml`이 있다.

## 3. 절차

- 실행 환경: Ubuntu 24.04 LTS의 bash 또는 Windows 11의 PowerShell
- 실행 위치: 저장소 루트
- 바꿔 넣을 값: 없다.
- 최대 대기 시간: cargo-deny 설치는 5분, 검사는 1분이다. 넘으면 Ctrl+C로 중단하고 네트워크 연결을 확인한다.

1. cargo-deny가 설치되어 있는지 확인한다.

   ```shell
   cargo deny --version
   ```

   기대 결과: `cargo-deny 0.20.2`

2. 1단계에서 명령을 찾지 못했거나 버전이 다르면 cargo-deny를 설치한다.

   영향: 사용자 홈의 `.cargo/bin`에 실행파일을 설치한다. `cargo uninstall cargo-deny`로 지울 수 있다.

   ```shell
   cargo install --locked cargo-deny@0.20.2
   ```

   기대 결과: ``Installed package `cargo-deny v0.20.2` ``

3. 라이선스를 검사한다.

   ```shell
   cargo deny check licenses
   ```

   기대 결과: 종료 코드 0과 마지막 줄 `licenses ok`

   이 통합 검사는 작업 공간의 `vnc-relay-tray`도 포함한다. `deny.toml`의 `[graph] targets`가 Windows와 Linux 두 대상을 모두 지정하므로 통합 검사는 두 대상 의존성 그래프의 합집합을 검사한다. 검사 범위가 각 대상별 검사보다 넓으므로 통합 검사가 통과하면 대상별 검사도 통과한다. `tray-icon`은 `default-features = false`이므로 Linux용 `libappindicator` 기능과 그 `dirs` 의존성을 켜지 않는다. 현재 그래프에는 `option-ext`도 없다. 트레이만을 위한 대상별 추가 검사는 필요하지 않다.

### 3.1 배포 고지 생성과 대조

루트 [`THIRD-PARTY-NOTICES.txt`](../../THIRD-PARTY-NOTICES.txt)는 두 대상의 일반·빌드 의존성 합집합이다. 각 crate의 이름·버전·라이선스 식과 고지 전문을 담는다. crate 고지 뒤에는 실행파일에 넣는 글꼴의 `web/dashboard/fonts/LICENSE.txt` 전문을 담는다. Windows·Linux 배포물에는 같은 파일 전체와 루트 [`LICENSE`](../../LICENSE)를 별도로 넣는다. `LICENSE`는 MIT 전문과 `Copyright (c) 2026 vnc-relay contributors`를 담는다. 의존성 고지는 프로젝트 자체 MIT 고지를 대신하지 않는다.

생성기는 `xtask`의 std·serde_json을 쓴다. 새 crate와 추가 설치 도구는 필요 없다. 코드 위치와 처리 순서는 [xtask 설계](../architectures/xtask.md)에 있다.

1. 네트워크가 있는 준비 단계에서 두 대상의 crate를 캐시에 받는다. 이미 준비된 캐시가 있으면 생략한다.

   ```shell
   cargo fetch --locked --target x86_64-pc-windows-msvc
   cargo fetch --locked --target x86_64-unknown-linux-gnu
   ```

   기대 결과: 두 명령 모두 종료 코드 0. Windows·Linux의 Cargo 캐시는 별개이므로 각 호스트에서 준비한다. 고지 검사 자체는 `--offline --locked`만 쓴다.

2. 고지를 처음 만들거나 의존성을 바꿨으면 다시 만든다.

   ```shell
   cargo xtask notices
   ```

   기대 결과: `notices: 통과`, 종료 코드 0. 이 단계만 고지 파일을 바꾼다. crate·파일 상대 경로 순으로 정렬하고 줄 끝을 LF로 통일한다. 실행 시각·개인 경로·Cargo 대상 디렉터리는 넣지 않는다.

3. 생성 결과를 대조한다.

   ```shell
   cargo xtask notices --check
   ```

   기대 결과: `notices --check: 통과`, 종료 코드 0. 차이가 있으면 종료 코드 1과 첫 차이의 줄을 출력한다. crate 목록뿐 아니라 고지 전문도 대조한다. 마지막의 Cargo.lock 입력 전체도 대조하므로 개발 의존성만 바뀌어도 재생성 없이는 실패한다. 줄 끝만 다른 경우는 같은 입력으로 본다.

4. 실행파일과 고지 파일 전체를 같은 배포 묶음에 넣는다.

   기대 결과: 받는 사람이 crate별 라이선스 식·저작권 고지·라이선스 전문·존재하는 NOTICE를 오프라인으로 읽을 수 있다. 고지 파일만 배포하고 프로그램의 MIT LICENSE를 빼도 된다는 뜻은 아니다.

고지 범위는 `cargo tree -p vnc-relay -p vnc-relay-tray --target <대상> --edges normal,build --prefix none --format "{p}" --charset ascii`로 고른다. 두 대상의 `cargo metadata --format-version 1 --filter-platform <대상>`를 합쳐 이름·버전·라이선스와 패키지 위치를 찾는다. 위 내부 명령에는 모두 `--offline --locked`를 붙인다. 호스트 전용 빌드 간선은 Cargo 트리가 OS마다 다를 수 있으므로 메타데이터의 빌드 의존성과 그 하위 일반·빌드 의존성도 합친다. 실행파일에 링크되지 않는 빌드 도구·절차 매크로까지 보수적으로 고지한다. 개발 전용 의존성과 xtask 자체는 배포 목록의 시작점이 아니다. 제품 기능이 바뀌면 다시 생성해야 한다.

생성기는 각 crate 안의 `LICENSE*`·`LICENCE*`·`COPYING*`·`NOTICE*`·`UNLICENSE*`·`COPYRIGHT*` 파일을 대소문자 구분 없이 재귀 수집한다. Cargo의 `license_file`도 포함한다. 하위 고지를 놓치지 않기 위해 배포에 쓰이지 않는 고지까지 들어갈 수 있다. 해당 파일을 넣었다고 선택적 라이선스의 모든 가지를 채택하는 것은 아니다. 고지 파일 누락·읽기 실패·심볼릭 링크·메타데이터 연결 실패는 성공으로 숨기지 않고 중단한다. 구현 소스의 코드 본문은 고지 파일에 복사하지 않는다.

`tests/dependency_notices.rs`는 위 대조를 오프라인으로 실행한다. 내부 명령은 `cargo run --offline --locked --quiet -p xtask -- notices --check`다. Cargo가 위치를 정하므로 `CARGO_TARGET_DIR`의 특정 값에 의존하지 않는다. 개별 실행은 `cargo test --test dependency_notices`다. Windows·WSL 게이트는 모두 `cargo xtask scenarios`를 호출한다. 이 도구가 `cargo test --workspace --no-fail-fast`로 이 시험도 실행한다. `xtask/src/scenarios.rs`는 xtask 시험을 제외하므로 OPS-021 표식은 루트 시험에만 `partial`로 붙인다. 이 시험은 프로젝트 MIT 고지와 독립 구현의 법적 판정을 확인하지 않는다.


## 4. 검증

[검증 기록](../AGENTS.md#검증-기록)에 따라 실제 결과와 성공 조건을 대조한다.

자동 검사의 현재 결과는 3·3.1절 명령과 `cargo xtask scenarios`가 만드는 `scenario-report.md`, CI에서 확인한다. 수동 출처·LICENSE·암호 모듈 검토는 아래 절에 보존한다.

| 검토 대상·날짜 | 환경·방법 | 실제 결과 |
| --- | --- | --- |
| `toml_edit 0.25.17+spec-1.1.0`, 커밋 `eae442f`의 설정 편집 범위, 2026-11-08 | Windows Cargo 캐시의 패키지 `Cargo.toml`과 라이선스·NOTICE 파일 목록을 수동 확인했다 | `license = "MIT OR Apache-2.0"`, `LICENSE-MIT`·`LICENSE-APACHE`가 있고 별도 `NOTICE`는 없다. MIT 저작권·전문과 Apache 전문은 배포 고지에 포함한다. 허용 목록은 넓히지 않는다. 이 수동 확인은 `cargo deny`와 `notices --check`의 현재 실행 결과를 대신하지 않는다 |

### 4.1 출처 검토 기록 (2026-11-04)

확인한 자료와 작업 트리는 독립 구현 경위를 뒷받침한다. 다만 문자열 검색과 로컬 이력만으로 복사·번역·포팅의 부재를 증명할 수는 없다. 엄격한 클린룸 인증이나 법률 의견을 완료했다고 주장하지 않는다([PRD 9절](../requirements/PRD.md#9-호환성-조사와-독립-구현-원칙)).

- 대상 버전: 검토 시작 시 로컬 HEAD 이력 `24e97b4b1aaee5451a6435a303ee0335e6841ebf`까지와 이번 고지 생성기·루트 시험·문서 변경을 포함한 작업 트리. 동시에 진행 중인 다른 작업의 변경은 이 검토의 작성 이력으로 소급하지 않는다.
- 범위: `src/`, `crates/`, `web/dashboard/`, `xtask/`의 파일 이름·폴더 구조와 본문. TLS 시험용 PEM도 파일 목록에 포함했다. 빌드 출력·Cargo 캐시·저장소 밖 관찰 배포본은 작성 제품 코드가 아니다.
- 방법: Git 명령은 실행하지 않았다. 로컬 `.git/logs/HEAD`의 변경 식별자·주체·제목, 아래 계약, 현재 파일 목록과 문자열 검색을 대조했다. reflog 주체는 커밋 작성자 전체를 인증하거나 법적 저작권자를 결정하는 자료가 아니다.
- 금지 범위 준수: 이번 검토에서 UltraVNC·websockify·noVNC 소스를 열거나 비교하지 않았다. 이전 문서에 외부 소스·설치 스크립트 링크가 있어도 이번 검토는 그 링크를 열지 않았다.

| 근거 | 확인한 사실 | 해석의 한계 |
| --- | --- | --- |
| 로컬 HEAD 이력의 초기 `153c9f6`·Rust 골격 `17a9305`·설정 `4fb7acb`·xtask `b7d23ee`·TLS `ead7be0` | 요구사항·설계 → Rust 골격 → 기능별 구현이라는 변경 경로가 있다. 해당 reflog 주체는 `kayden.kim`이다. | 제목·주체는 코드의 독창성이나 저작권 귀속을 보증하지 않는다. reflog는 로컬 자료이며 배포 파일에 포함하지 않는다. |
| 대시보드 `d504667`·`aaca485`·`023d2bd`·`172764a` | 공통 컴포넌트 → 로그인 → 운영 현황 → 세션 상세의 구현 기록이 있다. 해당 reflog 주체도 `kayden.kim`이다. | UI의 유사성·외부 디자인 권리 전체를 판정하는 자료는 아니다. |
| 초기 교환 `5e53570`·스트림 코덱 `4391b14` | 관찰·계약 채택 뒤 Rust 초기 교환과 RFC 6455 스트림 코덱을 구현한 변경 제목이 있다. | 과거 모든 작업자의 열람 행위나 소스 유사도를 조사한 것은 아니다. |
| [관찰 기록](../contracts/repeater-compat-record.md) 1·2·3절 | 소유자 허용, 고정 배포본 해시, 분리 VM, 입력·관찰·해석을 분리한 근거가 있다. | 사용 권한의 법률 검토를 완료한 것은 아니다. |
| [초기 교환 계약](../contracts/repeater-handshake-v1.md) 1절·1.4절 | 250바이트 요청·12바이트 시작 값·ID 규칙이 관찰 증거와 확정 기준에 연결된다. 계약은 외부 세 프로젝트의 소스를 읽거나 복사·번역하지 않도록 정한다. | 같은 통신 상수는 호환 계약의 결과다. 상수가 같다는 이유로 소스가 같거나 다르다고 단정하지 않는다. |
| 파일 목록 | 위 네 영역에 vendor·node_modules·외부 세 프로젝트의 소스 트리나 배포 아카이브가 없다. Rust·HTML·CSS·JS와 프로젝트 설정·시험 PEM으로 구성된다. | 이름을 바꿔 넣은 코드의 부재까지 증명하지는 못한다. |

본문 검색은 `(?i)copyright|spdx|license|websockify|ultravnc|novnc`를 위 네 영역 전체에 적용했다. 나온 항목은 아래와 같이 모두 분류했다. 별도 외부 라이선스 머리말은 찾지 못했다.

| 위치·검색 결과 | 분류 |
| --- | --- |
| `src/cli.rs`의 `UltraVNC` | CLI 설명에 쓰는 호환 대상 이름이다. 외부 코드 고지가 아니다. |
| `crates/{domain,app,adapters}/Cargo.toml`, `xtask/Cargo.toml`의 `license.workspace = true` | 이 저장소가 정한 패키지 메타데이터다. 실제 MIT 저작권 고지를 작성했다는 뜻은 아니다. |
| `xtask/src/scenarios/markers.rs`의 `needs UltraVNC` | 표식 스캐너 시험의 가상 입력 문자열이다. |
| `xtask/src/scenarios/test_run.rs`의 `needs_ultravnc`·`needs UltraVNC` 두 항목 | 무시된 시험 출력과 기대 키를 검사하는 시험 데이터다. |
| `xtask/src/main.rs`의 라이선스 how-to 경로, `xtask/src/notices.rs`의 모든 `license`·`copyright` 항목 | 이번 고지 생성기의 필드·출력·파일 이름 판별 상수·회귀 시험이다. 외부 구현의 머리말이 아니다. |
| `websockify`, `noVNC`, `SPDX` | 네 영역에 일치 항목이 없다. `web/dashboard/`에는 위 검색 전체가 일치하지 않는다. |

`permission is hereby|redistribution and use|all rights reserved|gnu general public|spdx-license-identifier`도 대소문자 구분 없이 검색했다. 위 네 영역에 일치 항목이 없다. 별도 생성 산출물 `THIRD-PARTY-NOTICES.txt`에는 의도적으로 외부 고지 전문을 보존한다. 이는 외부 구현 소스를 vendor로 넣은 것이 아니다.

대시보드의 HTML `script`, JS `import`·`require(`·동적 `import(`, `http://`·`https://`, CSS `@import`·`url(`를 정적 검색했다. `app/`·`login/`의 모듈 경로는 모두 자체 파일의 상대 경로나 `/app/main.js`다. `preview/`·`tests/`도 자체 모듈만 참조한다. 외부 CDN·bare package import·import map·node_modules는 없다. 유일한 HTTP 형태 값은 `app/components/gauge.js`의 SVG 네임스페이스 `http://www.w3.org/2000/svg`다. 이는 브라우저 DOM 이름 공간이며 JS 다운로드 주소가 아니다. CSS 외부 import·URL도 없다. 따라서 현재 정적 대시보드에는 외부 JS 의존성이 없다는 구조 증거를 확보했다. 축소·번역한 코드를 사람이 숨겨 넣지 않았다는 수학적 증명은 아니다.

재검토 때는 PowerShell의 `Get-ChildItem src,crates,web/dashboard,xtask -Recurse -File | Select-String -Pattern 'copyright|spdx|license|websockify|ultravnc|novnc'`로 같은 범위를 검색할 수 있다. 현재 기록은 저장소 검색 도구와 전체 파일 목록으로 확인했다. 이 PowerShell 명령은 이번에 별도로 실행하지 않았다. 외부 세 프로젝트의 원본 소스 비교를 재검토 절차로 추가하지 않는다.

### 4.2 프로젝트 LICENSE 검토 기록(2026-11-07)

프로젝트 MIT LICENSE의 수동 검토 기록이다. 저작권자는 `vnc-relay contributors`, 적용 연도는 `2026`이다.

대상은 루트 `LICENSE`를 추가한 작업 트리다. `Cargo.toml`의 라이선스 값과 `xtask/src/notices.rs`의 고지 분리 문구는 바꾸지 않았다. LICENSE 전문은 소스 문자열 시험을 새로 만들지 않고 사람이 대조했다.

| 확인 대상 | 기준 | 환경 | 결과 | 증거 |
| --- | --- | --- | --- | --- |
| LICENSE 전문 | 표준 MIT License 전문과 일치. 저작권 행만 실제 값으로 채운다 | Windows 11, 2026-11-07. 파일·표준 전문 수동 대조 | 통과. 줄바꿈을 제외한 허가·고지 유지 조건·면책 문구가 같다. 저작권 행만 프로젝트 값이다 | 루트 [`LICENSE`](../../LICENSE), [OSI MIT](https://opensource.org/license/mit), [SPDX `MIT` 전문](https://spdx.org/licenses/MIT.html) |
| 저작권 고지 | `Copyright (c) 2026 vnc-relay contributors` | 같음 | 통과. `LICENSE` 3행이 기준 값과 같다 | [`LICENSE`](../../LICENSE) |
| 패키지 라이선스 | `license = "MIT"` | 같음 | 통과. 루트 LICENSE와 패키지 메타데이터가 일치한다 | [`Cargo.toml`](../../Cargo.toml) `[workspace.package]` |
| 고지 머리말 | 프로젝트 자체 MIT LICENSE를 별도 파일로 안내한다 | Windows 11, 2026-11-07. 소스 수동 대조 | 통과. 루트 LICENSE를 별도로 배포하는 절차와 일치한다 | [`xtask/src/notices.rs`](../../xtask/src/notices.rs) `render`의 `The project's own MIT LICENSE is separate and is not supplied here.` |

OPS-021의 출처·라이선스 수동 증거는 4.1절의 출처 검토, 이 절의 LICENSE 검토와 4.3절의 의존성·암호 모듈 검토다. 자동 고지 대조 결과는 `cargo xtask scenarios`의 `scenario-report.md`에서 확인한다. 배포 묶음에는 `LICENSE`와 의존성 고지 전체를 넣어야 한다.

### 4.3 배포 의존성·암호 모듈 검토 (2026-11-04)

2026-11-04에 4.1절의 기준 커밋과 고지 생성기·루트 시험·문서 변경을 포함한 작업 트리를 검토했다. 환경은 WSL2 Ubuntu 24.04.4 LTS x86_64, Rust 1.99.1이다. `cargo metadata --locked --format-version 1 --filter-platform x86_64-pc-windows-msvc`로 메타데이터를 읽고 캐시의 고지 전문과 생성 파일을 수동 대조했다. 당시 고지는 외부 crate 110개를 담았다. 두 대상의 일반·빌드 의존성 합집합이며 Windows 전용 `windows-sys`·Linux 전용 `inotify`·빌드 도구도 포함했다. tokio·rustls·tokio-rustls·rustls-pki-types·axum·tokio-tungstenite·toml·serde_json·argon2·rpassword·notify·clap과 암호 모듈 aws-lc-rs·aws-lc-sys가 있었다. serde는 직접 쓰지 않고 toml 등에서 `serde_core`를 썼다.

| 암호 모듈 | Cargo 메타데이터의 라이선스 식 | 배포 고지에 보존한 내용 |
| --- | --- | --- |
| aws-lc-rs 1.18.2 | `ISC AND (Apache-2.0 OR ISC)` | 패키지 LICENSE의 ISC와 Apache-2.0 전문·고지 |
| aws-lc-sys 0.46.0 | `ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND MIT AND BSD-3-Clause AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR ISC OR MIT-0)` | 패키지 LICENSE·`aws-lc/LICENSE`·`aws-lc/third_party/fiat/LICENSE`. Amazon·Google·OpenSSL·Eric Young 등 원래 고지와 전문을 유지한다. |

검토한 aws-lc-sys의 LICENSE는 OpenSSL 유래 부분을 **Apache-2.0**으로 명시했다. 별도의 구형 `OpenSSL` SPDX 식으로 취급하거나 허용 목록을 늘리지 않았다. BoringSSL 유래의 기존 부분은 ISC, 새 부분은 Apache-2.0이다. Jitter Entropy 부분은 Amazon이 GPL 가지가 아니라 BSD-3-Clause를 선택했다고 고지한다. 생성 파일은 이 설명과 원래 저작권을 그대로 담았다. `AND`의 의무와 선택된 `OR` 가지의 조건을 지켜야 하며, 암호 모듈 전체를 ISC 하나로만 다루지 않는다. 독립된 NOTICE 파일이 있으면 생성기가 재귀 수집한다.

검토 결과, 제품 코드에 외부 구현을 직접 복사하거나 수정한 것으로 분류한 항목은 없었다. [INFERENCE] 계약·이력·검토 대상 트리 증거는 독립 구현 주장과 일치했다. 이 추론은 외부 원본과의 소스 대조를 수행했다는 뜻이 아니다. 생성기 대조는 파일이 최신인지 확인할 뿐, 허용 목록 판정이나 법률 검토를 대체하지 않는다.

실제 배포 묶음에는 고지 전체가 포함되었는지 배포 단계에서 확인해야 한다. 이 검토는 배포 아카이브의 검증을 포함하지 않는다.

## 5. 중단 조건

- `error[rejected]`가 나오면 그 의존성을 병합하지 않는다. 다른 crate를 찾거나 사용자에게 허용 목록 검토를 요청한다. `deny.toml`의 허용 목록을 승인 없이 늘리지 않는다.
- `error[unlicensed]`가 나오면 해당 crate의 라이선스 파일을 직접 확인하고 사용자 검토를 받는다.
- `notices --check`가 실패하면 그 배포를 중단한다. 3.1절에 따라 캐시를 준비하고 생성 결과와 변경된 고지를 검토한다. 고지 누락 오류를 빈 파일로 덮지 않는다.

## 6. 롤백 또는 복구

1. `cargo deny`와 `notices --check`는 파일을 바꾸지 않으므로 되돌릴 것이 없다.
2. 생성 결과가 잘못되었으면 원인을 고친 뒤 `cargo xtask notices`로 다시 만든다. 고지 전문을 손으로 줄이지 않는다.
3. 의존성 변경으로 허용 검사가 실패했으면 그 의존성 변경을 취소하거나 승인을 받은 뒤 두 검사를 다시 실행한다.

## 7. 해결되지 않을 때 전달할 대상

- 전달 대상: vnc-relay 프로젝트 관리자
- 보존할 증거: `cargo deny check licenses`의 전체 출력, 문제 crate의 이름·버전, 추가하려던 변경
- 비밀 값 제외 규칙: 이 검사는 비밀 값을 다루지 않는다.
