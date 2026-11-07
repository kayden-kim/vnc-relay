# 릴리스 묶음을 만들고 배포하는 방법

Windows 11 x64와 Ubuntu 24.04 LTS x64용 배포 묶음을 만들고, 버전 태그로 GitHub Release에 올린다. 성공하면 Release에 두 OS 묶음과 SHA-256 파일이 붙고, 묶음을 푼 디렉터리에서 [수동 운영 안내](operate-relay.md) 3.2절부터 진행할 수 있다.

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 릴리스 담당자 |
| 검증 상태 | Verified — 1.1·1.2·3.1·3.2절과 3.3절 7·8단계(태그·Release)를 두 OS와 GitHub Actions에서 실행해 확인했다([4절](#4-검증)). 첫 버전 `2026.11.0`은 구현 커밋 `7859b5d`가 버전을 바꿨으므로 3.3절 1~6단계를 따로 실행하지 않았다. 6절 롤백은 실행하지 않았다. |
| 검증 환경·버전 | Windows 11 Pro 10.0.26200 x64(PowerShell 7, Python 3), WSL2 Ubuntu 24.04.4 LTS x64(GNU coreutils `sha256sum`), Rust 1.99.1, GitHub Actions 호스팅 러너 `windows-2025`·`ubuntu-24.04` |
| 최종 검증일 | 2026-11-08 |
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
2. 루트 `Cargo.toml`의 `[workspace.package] version`을 새 버전으로 바꾼다. 루트 `README.md`의 "버전" 줄도 같은 값으로 바꾼다.
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

   기대 결과: `notices: 통과 (THIRD-PARTY-NOTICES.txt)`. 이어 두 OS의 [전체 완료 검증](set-up-and-verify.md#34-전체-완료-검증)을 실행한다.

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
