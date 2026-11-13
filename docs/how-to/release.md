# 릴리스 묶음을 만들고 배포하는 방법

Windows 11 x64와 Ubuntu 24.04 LTS x64용 배포 묶음을 만들고, 버전 태그로 GitHub Release에 올린다. 성공하면 Release에 두 OS 묶음과 SHA-256 파일이 붙고, 묶음을 푼 디렉터리에서 [수동 운영 안내](operate-relay.md) 3.2절부터 진행할 수 있다.

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 릴리스 담당자 |
| 검증 상태 | Verified — 1.1·1.2·3.1·3.2절과 3.3절 7·8단계는 첫 버전 `2026.11.0`에서 두 OS와 GitHub Actions로 실행해 확인했다. 그때는 구현 커밋 `7859b5d`가 버전을 바꿨으므로 3.3절 1~6단계를 따로 실행하지 않았다. `2026.11.1`과 `2026.11.2`에서는 3.3절 1~8단계를 모두 실행했다. 3.1절 로컬 묶음은 실행하지 않았고, CI가 만든 Release 묶음 두 개를 내려받아 해시와 버전을 확인했다([4절](#4-검증)). 6절 롤백은 실행하지 않았다. |
| 검증 환경·버전 | Windows 11 Pro 10.0.26200 x64(PowerShell 7, Python 3), WSL2 Ubuntu 24.04.4 LTS x64(GNU coreutils `sha256sum`), Rust 1.99.1, GitHub Actions 호스팅 러너 `windows-2025`·`ubuntu-24.04` |
| 최종 검증일 | 2026-11-09 |
| 관련 기준 문서 | [PRD 확정 기준](../requirements/PRD.md#확정-기준)의 "배포 산출물"·"버전 규칙" 행, [수동 운영 안내 3.1절](operate-relay.md#31-release-빌드와-묶음), [xtask 모듈 설계](../architectures/xtask.md), [개발 환경·검증 how-to](set-up-and-verify.md) |

## 1. 대상과 영향

- 대상: 루트 `Cargo.toml`의 `[workspace.package] version`, `cargo xtask package`가 만드는 묶음, GitHub Actions 워크플로 `.github/workflows/ci.yml`, 저장소 `kayden-kim/vnc-relay`의 태그와 GitHub Release다.
- 예상 사용자 영향: 저장소가 공개 저장소이므로 Release와 묶음은 누구나 내려받을 수 있다. 태그를 올리면 Release가 바로 공개된다.
- 작업 허용 시간: 제한 없다.

### 1.1 버전 규칙

버전은 `연.월.순서`다(사용자 결정 2026-11-08(Q165)).

- 연은 네 자리, 월은 1~12다. 앞자리 0을 쓰지 않는다. 4월은 `2026.4.0`이다. Cargo는 버전 숫자의 앞자리 0을 받지 않는다. 그래서 `Cargo.toml`, 묶음 이름, `vnc-relay --version`은 같은 버전 문자열을 쓴다. 태그는 그 앞에 `v`만 붙인다(`v2026.4.0`, 사용자 결정 2026-11-08(Q163)).
- 순서는 그 달에 낸 릴리스 수를 0부터 센다. 같은 달의 두 번째 릴리스는 `2026.4.1`이다. 연월이 바뀌면 순서를 0으로 되돌린다.
- 연월은 릴리스하는 날의 연월이다.
- 첫 버전은 `2026.11.0`이다(사용자 결정 2026-11-08(Q166)).
- 한 번 Release에 올린 버전 번호는 다시 쓰지 않는다. 잘못 올린 Release를 지웠으면 순서를 올려 새로 낸다(이 문서의 판단, 사용자 결정 없음). 같은 번호로 다른 내용을 내면 내려받은 사람이 두 묶음을 구분할 수 없다.

`cargo xtask package`는 버전이 이 형식이 아니면 빌드하기 전에 실패한다. 연월이 오늘 날짜와 같은지는 검사하지 않는다.

### 1.2 묶음 이름과 내용

| 대상 | 묶음 | 내용 |
| --- | --- | --- |
| Windows 11 x64 | `vnc-relay-<버전>-windows-x64.zip` | `vnc-relay-<버전>-windows-x64/` 디렉터리 하나. 그 안에 [수동 운영 안내 3.1절](operate-relay.md#31-release-빌드와-묶음)의 파일 |
| Ubuntu 24.04 LTS x64 | `vnc-relay-<버전>-linux-x64.tar.gz` | 같은 구성. 실행파일 이름만 `vnc-relay`다 |

묶음마다 `<묶음 파일 이름>.sha256`이 함께 생긴다. 한 줄에 `<SHA-256 16진수>  <묶음 파일 이름>`을 쓴다. `sha256sum -c`가 읽는 형식이다. 묶음의 형식과 내용은 사용자 결정 2026-11-08(Q161·Q164)을 따른다.

## 2. 전제 조건

- [ ] 빌드 담당자는 [개발 환경 준비](set-up-and-verify.md)에 따라 Rust와 C 빌드 도구를 준비했다. Windows에서 NASM이 없으면 `AWS_LC_SYS_PREBUILT_NASM=1`을 쓴다.
- [ ] Ubuntu에는 GNU `tar`가, Windows에는 기본 제공 `%SystemRoot%\System32\tar.exe`가 있다.
- [ ] 릴리스할 커밋에서 두 OS의 [전체 완료 검증](set-up-and-verify.md#34-전체-완료-검증)이 통과했다. GitHub Actions는 전체 시험을 실행하지 않는다(사용자 결정 2026-11-08(Q162)).
- [ ] 태그를 올리려면 `kayden-kim/vnc-relay`에 push 권한이 있다.

## 3. 절차

- 실행 환경: Windows PowerShell 또는 Ubuntu bash다.
- 실행 위치: 저장소 루트다.
- 바꿔 넣을 값: `<버전>`은 1.1절 규칙에 따른 새 버전이다(예: `2026.11.0`).
- 최대 대기 시간: 로컬 `cargo xtask package`는 10분으로 둔다. GitHub Actions 한 번은 60분으로 둔다(이 문서의 판단, 사용자 결정 없음). 넘으면 로컬은 Ctrl+C로, Actions는 실행 화면의 Cancel로 중단한다.

### 3.1 로컬에서 묶음 만들기

1. 묶음을 만든다. 이 명령은 다음을 차례로 한다.
   - 두 OS 의존성 소스를 받는다(`cargo fetch --locked`).
   - release 실행파일을 빌드한다(`cargo build --release --locked --package vnc-relay`).
   - 배포 고지를 대조한다(`cargo xtask notices --check`).
   - 파일을 모아 압축하고 SHA-256 파일을 쓴다.

   ```shell
   cargo xtask package
   ```

   기대 결과: 종료 코드 0이다. 출력 끝에 아래 두 줄이 나온다. `CARGO_TARGET_DIR`을 정했으면 `target` 대신 그 경로다.

   ```text
   package: target/package/vnc-relay-<버전>-windows-x64.zip
   sha256: <64자리 16진수>
   ```

   Ubuntu에서는 `vnc-relay-<버전>-linux-x64.tar.gz`다. 압축하기 전의 디렉터리는 `target/package/vnc-relay-<버전>-<os>-x64/`에 남는다. 다시 실행하면 같은 이름의 디렉터리·묶음·SHA-256 파일을 지우고 새로 만든다.

2. 묶음을 빈 디렉터리에 풀고 버전을 확인한다.

   ```powershell
   .\vnc-relay.exe --version
   ```

   ```shell
   ./vnc-relay --version
   ```

   기대 결과: `vnc-relay <버전>`

### 3.2 GitHub Actions 검사

`main`에 push하거나 PR을 열면 워크플로 `ci`가 실행된다. `windows-2025`와 `ubuntu-24.04`에서 같은 작업 `check`를 실행한다.

| 순서 | 명령 | 실패하면 |
| --- | --- | --- |
| 1 | `rustup toolchain install`(`rust-toolchain.toml`의 버전과 구성요소) | 툴체인 설치 문제 |
| 2 | `cargo install cargo-deny --version 0.20.2 --locked` | 도구 설치 문제 |
| 3 | `cargo fmt --all --check` | 서식 위반 |
| 4 | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 경고·컴파일 오류 |
| 5 | `cargo test --package xtask --locked` | 개발 도구 단위 시험 실패(버전 형식, SHA-256 등) |
| 6 | `cargo xtask layers` | 레이어 위반 |
| 7 | `cargo xtask tokens --check` | 토큰·복사본 불일치 |
| 8 | `cargo deny check licenses` | 허용 목록 밖 라이선스 |
| 9 | `cargo xtask package` | 버전 형식, 빌드, 고지 대조, 압축 |

통과하면 실행 결과의 Artifacts에 `package-Windows`와 `package-Linux`가 생긴다. Artifacts는 GitHub의 보관 기간이 지나면 지워진다. 오래 둘 묶음은 3.3절의 Release로 낸다.

`cargo xtask scenarios`(전체 시험)는 실행하지 않는다. 브라우저·`nftables`·성능 시험의 실행 환경이 로컬 게이트와 다르기 때문이다(사용자 결정 2026-11-08(Q162)). xtask 단위 시험은 실행 환경을 타지 않아 함께 실행한다(이 문서의 판단, 사용자 결정 없음).

### 3.3 버전을 올리고 Release 내기

1. 새 버전을 정한다. 1.1절 규칙을 따른다. 이미 낸 버전은 `git tag --list "v*"`로 확인한다.
2. 루트 `Cargo.toml`의 `[workspace.package] version`을 새 버전으로 바꾼다. 루트 `README.md`의 "버전" 줄도 같은 값으로 바꾼다. README의 "기준" 줄과 "연결 구성 예" 절에 이전 공개 묶음과 다른 동작을 적은 문장이 있으면 지운다. 새 묶음은 현재 소스와 같기 때문이다. 도움말 두 책도 같은 변경에서 고친다(사용자 결정 2026-11-11(Q221), [도움말 작성 규칙](../help/AGENTS.md)). 첫 페이지 `index.md`의 기준 버전 절에 새 최신 Release와 공개 날짜를 적고, 최신 Release와 다른 동작 목록을 다시 쓴다. 설치 페이지의 예시 파일 이름과 다른 페이지의 Release 안내도 새 버전에 맞춘다.
3. `Cargo.lock`을 갱신하고 버전을 확인한다.

   ```shell
   cargo build
   cargo run --quiet -- --version
   ```

   기대 결과: `vnc-relay <버전>`. `Cargo.lock`에서 작업 공간 패키지 네 개(`vnc-relay`, `vnc-relay-domain`, `vnc-relay-app`, `vnc-relay-adapters`)의 버전이 바뀐다.

4. 배포 고지를 다시 만든다. `THIRD-PARTY-NOTICES.txt`는 `Cargo.lock` 전체를 담으므로 버전을 바꾸면 반드시 달라진다.

   ```shell
   cargo xtask notices
   ```

   기대 결과: `notices: 통과 (THIRD-PARTY-NOTICES.txt)`. 이어 `cargo xtask help-site`로 고친 도움말을 검사한다. 기대 결과는 `help-site: 통과`로 시작하는 줄이다([도움말 게시 how-to](publish-help.md)). 그다음 두 OS의 [전체 완료 검증](set-up-and-verify.md#34-전체-완료-검증)을 실행한다.

5. 변경을 커밋하고 `main`에 push한다. 커밋 메시지는 `chore(release): <버전>`이다.
6. 3.2절의 `ci` 실행이 두 OS 모두 통과했는지 확인한다.

   ```shell
   gh run list --workflow ci --branch main --limit 1
   ```

   기대 결과: 최신 실행의 상태가 `completed`, 결론이 `success`다.

7. 태그를 만들고 올린다.

   영향: Release가 공개된다. 태그와 Release는 지울 수 있지만, 이미 내려받은 묶음은 회수할 수 없다.

   ```shell
   git tag v<버전>
   git push origin v<버전>
   ```

   기대 결과: 태그로 `ci`가 다시 실행된다. 두 OS의 `check`가 통과한 뒤 작업 `release`가 실행된다.

8. 작업 `release`의 결과를 확인한다. 이 작업은 다음을 차례로 한다.
   - 두 OS 묶음을 내려받는다.
   - 태그의 버전과 같은 이름의 묶음이 두 개 있는지 확인한다.
   - `sha256sum -c`로 해시를 대조한다.
   - `gh release create`로 Release를 만든다.

   ```shell
   gh release view v<버전>
   ```

   기대 결과: 제목이 `vnc-relay <버전>`이고, 묶음 두 개와 `.sha256` 파일 두 개가 붙어 있다.

## 4. 검증

[검증 기록](../AGENTS.md#검증-기록)에 따라 실제 결과와 성공 조건을 대조한다.

| 관찰 대상 | 성공 조건 | 확인 명령·방법 | 실제 결과 | 실행 증거 위치 |
| --- | --- | --- | --- | --- |
| Windows 로컬 묶음 | 3.1절 기대 결과. 푼 실행파일의 `--version`이 `Cargo.toml` 버전과 같다. SHA-256 파일이 독립 계산값과 같다 | 3.1절. `Expand-Archive`로 임시 디렉터리에 풀어 확인 | 통과(2026-11-08). 커밋 `7859b5d` 직전 작업 트리. 이후 바뀐 코드는 저장소 밖 대상 디렉터리의 경로 표시뿐이다. `vnc-relay-2026.11.0-windows-x64.zip`, SHA-256 `dfd416907d9e6f1a09a97a80a838e58148ea6aa993e857f41b718f40e14a50e7`이 `pwsh` `Get-FileHash`·Python `hashlib` 값과 같았다. 최상위 디렉터리 하나에 3.1절 파일 7종, `docs/` 파일 89개(저장소와 같은 수), `vnc-relay 2026.11.0` | 이 표 |
| Ubuntu 로컬 묶음 | 같은 조건. SHA-256 파일이 `sha256sum -c`를 통과한다 | 3.1절. `CARGO_TARGET_DIR=$HOME/vnc-relay-target` | 통과(2026-11-08, 커밋 `7859b5d`와 같은 코드). `package: /home/kaydenkim/vnc-relay-target/package/vnc-relay-2026.11.0-linux-x64.tar.gz`, `sha256sum -c` `OK`, 묶음 안 소유자 `0/0`, `docs/` 파일 89개, `vnc-relay 2026.11.0` | 이 표 |
| `main` push의 `ci` | 두 OS `check` 통과, Artifacts 두 개 | 3.2절 | 통과. [실행 42408213998](https://github.com/kayden-kim/vnc-relay/actions/runs/42408213998)(커밋 `7859b5d`). `check (windows-2025)` 8분 16초, `check (ubuntu-24.04)` 3분 7초, `release`는 태그가 아니어서 건너뜀. Artifacts `package-Windows`·`package-Linux`를 내려받아 계산한 SHA-256이 각 `.sha256` 파일과 같았다(Linux `7fee58dbdd36b3b1135e80773f8c8543a05f957e58fc90a6feb4d59e54b05e7f`, Windows `c2f1c71e2bacddb792462515ff2d22dfd390ec7220a83025801b1f2c2b3b6035`) | 이 표 |
| 태그 Release | 3.3절 8단계 기대 결과 | 3.3절 7·8단계. 태그 `v2026.11.0`을 커밋 `0b49ae3`에 붙여 push했다(프로젝트 소유자 요청, 2026-11-08) | 통과. [실행 42413077369](https://github.com/kayden-kim/vnc-relay/actions/runs/42413077369)에서 두 OS `check`와 `release`가 성공했다. [Release `vnc-relay 2026.11.0`](https://github.com/kayden-kim/vnc-relay/releases/tag/v2026.11.0)은 초안·사전 릴리스가 아니고, 묶음 2개와 `.sha256` 2개가 붙었다. 내려받은 묶음의 SHA-256이 각 `.sha256`과 같았다(Linux `b31d26178748a09992798b7f2c94c8768ddc8b4380c8e23d0b11f6dc0867e434`, Windows `c273f9cff25fa68676a24e3ee9ec8088609aaa427a1015be1669a9f0fd4ce891`). WSL `sha256sum -c` `OK`. 푼 실행파일은 두 OS에서 `vnc-relay 2026.11.0`을 출력했다 | 이 표 |

같은 커밋 `7859b5d`의 전체 완료 검증은 두 OS에서 통과했다. Windows `vr-win.ps1 -Only all`은 `ALL PASSED (all)`, 시나리오 153건 중 통과 137·부분 11·실패 0·미실행 3·미연결 2·오류 0이다(보고 2026-11-07T20:08:44Z). WSL `vr-full.sh`는 `ALL PASSED`, 통과 140·부분 7·실패 0·미실행 4·미연결 2·오류 0이다(보고 2026-11-07T20:26:57Z). WSL 보고의 `+ 미커밋 변경`은 줄 끝 차이뿐이다(`git -c core.autocrlf=true diff --exit-code` 종료 코드 0).

커밋 `aa15af8`에서 묶음 파일 권한을 정규화하고 CI에 `cargo test --package xtask --locked`를 더했다. WSL에서 `/mnt/d` 작업 트리로 만든 tar.gz는 이전에 모든 파일이 `-rwxr-xr-x`였다. 바꾼 뒤에는 파일 94개가 `-rw-r--r--`, 실행파일 `vnc-relay`만 `-rwxr-xr-x`, 디렉터리 14개가 `drwxr-xr-x`였고 `sha256sum -c`는 `OK`였다. 같은 커밋의 [실행 42411305345](https://github.com/kayden-kim/vnc-relay/actions/runs/42411305345)에서 두 OS `check`가 성공했고, 두 OS 모두 xtask 단위 시험 16건이 통과했다. 기록 커밋 `e7eed86`의 실행 42411172978도 성공했다. 전체 완료 검증은 `7859b5d` 기준이다. `aa15af8`은 `xtask/src/package.rs`·워크플로·문서만 바꿨으므로 [개발 환경·검증 how-to 3.3절](set-up-and-verify.md#33-변경-범위별-검증)의 해당 행 명령으로 확인했다.

확인 중 관찰한 도구 차이를 적는다. 검증 PC의 Git Bash에 있는 `sha256sum 0.8.0`은 `-c`에서 CR 바이트가 든 파일을 자기가 쓴 해시 파일로도 `FAILED`로 보고했다. 직접 계산한 값은 맞았다. GNU coreutils `sha256sum`(Ubuntu)은 같은 파일을 `OK`로 판정했다. 이 PC의 Windows PowerShell 5.1은 `PSModulePath` 앞에 PowerShell 7 경로가 있어 `Get-FileHash`를 찾지 못했다. Windows에서는 PowerShell 7의 `Get-FileHash`나 `certutil -hashfile <파일> SHA256`으로 확인하기를 권장한다.

`2026.11.1` 릴리스(2026-11-08)는 3.3절 1~8단계를 모두 실행했다. 1~4단계: 버전을 바꾼 뒤 `cargo run --quiet -- --version`이 `vnc-relay 2026.11.1`을 출력했다. `Cargo.lock`은 작업 공간 패키지 네 개의 버전만 바뀌었고, `cargo xtask notices`는 `notices: 통과 (THIRD-PARTY-NOTICES.txt)`였다. 두 OS의 전체 완료 검증은 `e3fa56f`에 버전 변경을 더한 작업 트리에서 시작했다. 두 보고서의 대상 커밋은 `e3fa56f` + 미커밋 변경이다. Windows `vr-win.ps1 -Only all`은 `ALL PASSED (all)`, 시나리오 215건 중 통과 198·부분 12·실패 0·미실행 3·미연결 2·오류 0이다(보고 2026-11-08T11:22:35Z). WSL `vr-full.sh`는 `ALL PASSED`, 통과 201·부분 8·실패 0·미실행 4·미연결 2·오류 0이다(보고 2026-11-08T11:25:50Z).

게이트가 실행되는 동안 스펙이 아닌 문서 8개의 링크 대상과 문서 안내 행 하나를 바꿨다(아래 커밋 `70aba2d`). 그래서 두 게이트는 커밋 `c9194a8` 자체를 대상으로 하지 않았다. 코드·시험·스펙은 게이트를 시작할 때와 같다. 이 문서 변경은 [변경 범위별 검증](set-up-and-verify.md#33-변경-범위별-검증)의 "문서만" 행에 따라 확인했다. Python 검사로 저장소 Markdown의 상대 링크·앵커 3,007개를 확인했고 문제는 0건이었다. 새 고정 링크는 push 뒤 HTTP 200으로 열렸다. `code-map.md`를 읽는 `tests/code_map.rs`는 게이트 안에서 이 변경보다 먼저 실행됐을 수 있다. 그래서 `c9194a8`과 같은 코드·시험·코드 지도로 두 OS에서 다시 실행했고, 두 OS 모두 4건이 통과했다. "작업 공간 버전" 행의 `cargo xtask package`는 로컬에서 실행하지 않았다. 같은 커밋 `c9194a8`에서 CI가 두 OS로 실행했다(아래 두 실행). `cargo test --package xtask`는 두 게이트와 CI에서 통과했다.

5·6단계: 링크 정리 커밋 `70aba2d`와 `chore(release): 2026.11.1` 커밋 `c9194a8`을 `main`에 push했다. [실행 42475766731](https://github.com/kayden-kim/vnc-relay/actions/runs/42475766731)에서 `check (windows-2025)`(4분 44초)와 `check (ubuntu-24.04)`(3분 10초)가 성공했고, `release`는 태그가 아니어서 건너뛰었다.

7·8단계: 태그 `v2026.11.1`을 커밋 `c9194a8`에 붙여 push했다(프로젝트 소유자 요청, 2026-11-08). [실행 42476169218](https://github.com/kayden-kim/vnc-relay/actions/runs/42476169218)에서 두 OS `check`(Windows 7분 43초, Ubuntu 3분 59초)와 `release`가 성공했다. [Release `vnc-relay 2026.11.1`](https://github.com/kayden-kim/vnc-relay/releases/tag/v2026.11.1)은 초안·사전 릴리스가 아니고, 묶음 2개와 `.sha256` 2개가 붙었다. 내려받은 묶음의 SHA-256이 각 `.sha256`과 같았다(Linux `41b1a6881e76900dcf118228ce5560993988bbc78eecae212c78dd054cd6a378`, Windows `400cffcae4696d840b709fba7a29777e731b917036800ef60f828279b12c421e`). WSL `sha256sum -c`는 `OK`였다. tar.gz 안의 파일 95개는 `-rw-r--r--`, 실행파일 1개는 `-rwxr-xr-x`, 디렉터리 14개는 `drwxr-xr-x`이고 소유자는 `0/0`이다. Windows 묶음은 최상위 디렉터리 하나에 3.1절 파일 7종과 `docs/` 파일 90개(저장소와 같은 수)를 담았다. 푼 실행파일은 두 OS에서 `vnc-relay 2026.11.1`을 출력했다.

`2026.11.2` 릴리스(2026-11-09)도 3.3절 1~8단계를 모두 실행했다. 1~4단계: 버전 커밋 `f520316`(`chore(release): 2026.11.2`)은 네 파일만 바꿨다. `Cargo.toml`과 `README.md`는 버전 줄, `Cargo.lock`은 작업 공간 패키지 네 개의 버전, `THIRD-PARTY-NOTICES.txt`는 그 안에 담긴 같은 네 버전이다. 두 OS의 전체 완료 검증은 먼저 `f520316`에서 실행했다. Windows `vr-win.ps1 -Only all`은 `ALL PASSED (all)`, 시나리오 246건 중 통과 229·부분 12·실패 0·미실행 3·미연결 2·오류 0이었다(보고 2026-11-09T04:31:42Z). WSL `vr-full.sh`는 `FAILED: scenarios`였다. 통과 230·부분 8·실패 2·미실행 4·미연결 2이고, 실패는 DASH-044·056이다.

두 실패의 원인은 시험 쪽에 있었다. 커밋 `0264863`에서 시험만 고쳤고 제품 코드는 바꾸지 않았다([대시보드 검증 4.33절](verify-dashboard-frontend.md#433-실서버-시험-간헐-실패-수정-dash-044056-2026-11-09)). 이 커밋에서 다시 실행한 두 게이트가 통과했다. Windows는 `ALL PASSED (all)`, 통과 229·부분 12·실패 0·미실행 3·미연결 2·오류 0이다(보고 2026-11-09T05:50:15Z). WSL은 `ALL PASSED`, 통과 231·부분 9·실패 0·미실행 4·미연결 2·오류 0이다(보고 2026-11-09T06:23:14Z). WSL 보고의 `+ 미커밋 변경`은 줄 끝 차이뿐이다(`git -c core.autocrlf=true diff --exit-code` 종료 코드 0). 두 게이트의 버전 단계는 `vnc-relay 2026.11.2`를 출력했고, 고지 대조 시험 `distribution_notices_match_locked_dependencies_and_license_texts`가 통과했다.

5·6단계: `f520316`과 `0264863`을 함께 `main`에 push했다. [실행 42572759934](https://github.com/kayden-kim/vnc-relay/actions/runs/42572759934)에서 `check (windows-2025)`(7분 49초)와 `check (ubuntu-24.04)`(4분 3초)가 성공했고, `release`는 태그가 아니어서 건너뛰었다.

7·8단계: 태그 `v2026.11.2`를 커밋 `0264863`에 붙여 push했다(프로젝트 소유자 요청, 2026-11-09). 버전 커밋 `f520316`이 아니라 두 OS 게이트와 CI가 통과한 시험 수정 커밋에 붙였다. 두 커밋의 제품 코드와 버전은 같다. [실행 42573633172](https://github.com/kayden-kim/vnc-relay/actions/runs/42573633172)에서 두 OS `check`(Windows 6분 57초, Ubuntu 4분 2초)와 `release`가 성공했다. [Release `vnc-relay 2026.11.2`](https://github.com/kayden-kim/vnc-relay/releases/tag/v2026.11.2)는 초안·사전 릴리스가 아니고, 묶음 2개와 `.sha256` 2개가 붙었다. 내려받은 묶음의 SHA-256이 각 `.sha256`과 같았다(Linux `beb1f1694851015ec3aebd9d798950aea11dd5ebdfcc35f79b3603e89f82bd52`, Windows `1b0d2f22189a037af66f572304866fc379884a0f0a2367a41b14ba224c563d1d`). WSL `sha256sum -c`는 `OK`였다. tar.gz 안의 파일 97개는 `-rw-r--r--`, 실행파일 1개는 `-rwxr-xr-x`, 디렉터리 14개는 `drwxr-xr-x`이고 소유자는 `0/0`이다. Windows 묶음은 최상위 디렉터리 하나에 3.1절 파일 7종과 `docs/` 파일 92개(저장소와 같은 수)를 담았다. 푼 실행파일은 두 OS에서 `vnc-relay 2026.11.2`를 출력했다. 이 Windows 묶음으로 실제 배치를 바꾼 기록은 [수동 운영 안내 4절](operate-relay.md#4-검증)에 있다.

## 5. 중단 조건

- `cargo xtask package`가 버전 형식 오류를 내면 1.1절에 맞게 버전을 고칠 때까지 진행하지 않는다.
- 전체 완료 검증이나 `ci`가 실패하면 태그를 만들지 않는다.
- 작업 `release`가 묶음 이름 불일치로 실패하면 태그와 `Cargo.toml` 버전이 다르다. Release는 만들어지지 않는다. 6절 1단계로 태그를 지운다.

## 6. 롤백 또는 복구

1. 잘못 만든 태그나 Release를 지운다. Release가 없으면 태그만 지운다.

   ```shell
   gh release delete v<버전> --cleanup-tag --yes
   git tag --delete v<버전>
   ```

2. 1.1절에 따라 순서를 올린 새 버전으로 3.3절을 다시 진행한다. Release를 만들기 전에 실패해 지운 태그는 같은 버전으로 다시 만들 수 있다.
3. `gh release list`와 `git ls-remote --tags origin`으로 지운 태그와 Release가 남지 않았는지 확인한다.

## 7. 해결되지 않을 때 전달할 대상

- 전달 대상: 프로젝트 관리자다.
- 보존할 증거: 태그, 커밋, GitHub Actions 실행 주소, 실패한 단계 이름과 로그, 묶음 파일 이름과 SHA-256이다.
- 비밀 값 제외 규칙: 토큰과 계정 비밀번호를 기록하지 않는다. Actions 로그는 `GITHUB_TOKEN`을 가린 상태로 보존한다.
