# 릴리스 묶음을 만들고 배포하는 방법

Windows 11 x64와 Ubuntu 24.04 LTS x64용 배포 묶음을 만들고, 버전 태그로 GitHub Release에 올린다. 성공하면 Release에 두 OS 묶음과 SHA-256 파일이 붙고, 묶음을 푼 디렉터리에서 [수동 운영 안내](operate-relay.md) 3.2절부터 진행할 수 있다.

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 릴리스 담당자 |
| 검증 상태 | Verified — 두 OS Release 묶음의 해시·내용·버전은 4절의 수동 기록으로 확인했다. 로컬 묶음 생성과 CI의 현재 결과는 3.1·3.2절로 확인한다. 6절 롤백은 미실행이다. |
| 검증 환경·버전 | Windows 11 Pro 10.0.26200 x64(PowerShell 7, Python 3), WSL2 Ubuntu 24.04.4 LTS x64(GNU coreutils `sha256sum`), Rust 1.99.1, GitHub Actions 호스팅 러너 `windows-2025`·`ubuntu-24.04` |
| 최종 검증일 | 2026-11-21 |
| 관련 기준 문서 | [PRD 확정 기준](../requirements/PRD.md#확정-기준)의 "배포 산출물"·"버전 규칙" 행, [수동 운영 안내 3.1절](operate-relay.md#31-release-빌드와-묶음), [xtask 모듈 설계](../architectures/xtask.md), [개발 환경·검증 how-to](set-up-and-verify.md) |

## 1. 대상과 영향

- 대상: 루트 `Cargo.toml`의 `[workspace.package] version`, `cargo xtask package`가 만드는 묶음, GitHub Actions 워크플로 `.github/workflows/ci.yml`, 저장소 `kayden-kim/vnc-relay`의 태그와 GitHub Release다.
- 예상 사용자 영향: 저장소가 공개 저장소이므로 Release와 묶음은 누구나 내려받을 수 있다. 태그를 올리면 Release가 바로 공개된다.
- 작업 허용 시간: 제한 없다.

### 1.1 버전 규칙

버전은 `연.월.순서`다.

- 연은 네 자리, 월은 1~12다. 앞자리 0을 쓰지 않는다. 4월은 `2026.4.0`이다. Cargo는 버전 숫자의 앞자리 0을 받지 않는다. 그래서 `Cargo.toml`, 묶음 이름, `vnc-relay --version`은 같은 버전 문자열을 쓴다. 태그는 그 앞에 `v`만 붙인다(`v2026.4.0`).
- 순서는 그 달에 낸 릴리스 수를 0부터 센다. 같은 달의 두 번째 릴리스는 `2026.4.1`이다. 연월이 바뀌면 순서를 0으로 되돌린다.
- 연월은 릴리스하는 날의 연월이다.
- 한 번 Release에 올린 버전 번호는 다시 쓰지 않는다. 잘못 올린 Release를 지웠으면 순서를 올려 새로 낸다. 같은 번호로 다른 내용을 내면 내려받은 사람이 두 묶음을 구분할 수 없다.

`cargo xtask package`는 버전이 이 형식이 아니면 빌드하기 전에 실패한다. 연월이 오늘 날짜와 같은지는 검사하지 않는다.

### 1.2 묶음 이름과 내용

| 대상 | 묶음 | 내용 |
| --- | --- | --- |
| Windows 11 x64 | `vnc-relay-<버전>-windows-x64.zip` | `vnc-relay-<버전>-windows-x64/` 디렉터리 하나. 그 안에 서버 `vnc-relay.exe`, 트레이 앱 `vnc-relay-tray.exe`와 [수동 운영 안내 3.1절](operate-relay.md#31-release-빌드와-묶음)의 공통 파일 |
| Ubuntu 24.04 LTS x64 | `vnc-relay-<버전>-linux-x64.tar.gz` | 서버 `vnc-relay`와 같은 공통 파일. 트레이 앱은 넣지 않는다 |

묶음마다 `<묶음 파일 이름>.sha256`이 함께 생긴다. 한 줄에 `<SHA-256 16진수>  <묶음 파일 이름>`을 쓴다. `sha256sum -c`가 읽는 형식이다.

## 2. 전제 조건

- [ ] 빌드 담당자는 [개발 환경 준비](set-up-and-verify.md)에 따라 Rust와 C 빌드 도구를 준비했다. Windows에서 NASM이 없으면 `AWS_LC_SYS_PREBUILT_NASM=1`을 쓴다.
- [ ] Ubuntu에는 GNU `tar`가, Windows에는 기본 제공 `%SystemRoot%\System32\tar.exe`가 있다.
- [ ] 릴리스할 커밋에서 두 OS의 [전체 완료 검증](set-up-and-verify.md#34-전체-완료-검증)이 통과했다. GitHub Actions는 전체 시험을 실행하지 않는다.
- [ ] 태그를 올리려면 `kayden-kim/vnc-relay`에 push 권한이 있다.

## 3. 절차

- 실행 환경: Windows PowerShell 또는 Ubuntu bash다.
- 실행 위치: 저장소 루트다.
- 바꿔 넣을 값: `<버전>`은 1.1절 규칙에 따른 새 버전이다(예: `2026.11.0`).
- 최대 대기 시간: 로컬 `cargo xtask package`는 10분으로 둔다. GitHub Actions 한 번은 60분으로 둔다. 넘으면 로컬은 Ctrl+C로, Actions는 실행 화면의 Cancel로 중단한다.

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

`cargo xtask scenarios`(전체 시험)는 실행하지 않는다. 브라우저·`nftables`·성능 시험의 실행 환경이 로컬 게이트와 다르기 때문이다. xtask 단위 시험은 실행 환경을 타지 않아 함께 실행한다.

### 3.3 버전을 올리고 Release 내기

1. 새 버전을 정한다. 1.1절 규칙을 따른다. 이미 낸 버전은 `git tag --list "v*"`로 확인한다.
2. 루트 `Cargo.toml`의 `[workspace.package] version`을 새 버전으로 바꾼다. 루트 `README.md`(영어)·`README.ko.md`(한국어)의 버전 줄(`Version`·`버전`)도 같은 값으로 바꾼다. 두 README의 기준 줄(`Basis`·`기준`)과 연결 구성 예 절(`Example deployment`·`연결 구성 예`)에 이전 공개 묶음과 다른 동작을 적은 문장이 있으면 지운다. 새 묶음은 현재 소스와 같기 때문이다. 도움말 두 책도 같은 변경에서 고친다([도움말 작성 규칙](../help/AGENTS.md)). 설치 페이지의 예시 파일 이름과 다른 페이지의 Release 안내를 새 버전에 맞춘다.
3. `Cargo.lock`을 갱신하고 버전을 확인한다.

   ```shell
   cargo build
   cargo run --quiet -- --version
   ```

   기대 결과: `vnc-relay <버전>`. `Cargo.lock`에서 작업 공간 패키지 다섯 개(`vnc-relay`, `vnc-relay-domain`, `vnc-relay-application`, `vnc-relay-adapters`, `vnc-relay-tray`)의 버전이 바뀐다.

4. 배포 고지를 다시 만든다. `THIRD-PARTY-NOTICES.txt`는 `Cargo.lock` 전체를 담으므로 버전을 바꾸면 반드시 달라진다.

   ```shell
   cargo xtask notices
   ```

   기대 결과: `notices: 통과 (THIRD-PARTY-NOTICES.txt)`. 이어 `cargo xtask help-site`로 고친 도움말을 검사한다. 기대 결과는 `help-site: 통과`로 시작하는 줄이다([도움말 게시 how-to](publish-help.md)). 그다음 두 OS의 [전체 완료 검증](set-up-and-verify.md#34-전체-완료-검증)을 실행한다.

5. 변경을 커밋하고 `main`에 push한다. 커밋 메시지는 `chore(release): <버전>`이다. 이 push로 워크플로 `help`가 도움말 사이트를 다시 게시한다. 이때부터 8단계에서 Release가 생길 때까지 게시된 도움말의 설치 예시는 아직 없는 Release를 가리킨다. 그래서 6단계가 통과하면 7단계를 바로 이어서 한다.
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

빌드·시험·CI의 현재 결과는 3.1·3.2절과 `cargo xtask scenarios`가 만드는 `scenario-report.md`로 확인한다. 다음 표는 Release 묶음을 직접 내려받아 확인한 최신 수동 기록이다.

| 대상·날짜 | 환경·방법 | 실제 결과 |
| --- | --- | --- |
| Release `v2026.11.10`, 커밋 `42d3f89`, 2026-11-21 | Windows 11 Pro 10.0.26200 x64(PowerShell 7, Git), WSL2 Ubuntu 24.04.4 LTS x64(GNU coreutils `sha256sum` 9.4). [Release](https://github.com/kayden-kim/vnc-relay/releases/tag/v2026.11.10)와 묶음 두 개·`.sha256` 두 개를 확인했다. 내려받은 묶음의 해시를 독립 계산하고, 풀어서 파일 구성·권한·`--version`을 확인했다. `docs/`는 `git archive v2026.11.10 docs`로 푼 태그 파일과 바이트 단위로 비교했다 | 통과. Linux SHA-256 `79579495926744d82df8386ccb9fb8709f4787c88d48457ed9cc3140c6dd6e10`, Windows `612e4d4e9c74629db08432bbf17cced4b86a6ec7237d41ec63876b86b44d4658`. 각 `.sha256`과 같고 WSL `sha256sum -c`는 `OK`였다. tar.gz 파일 160개는 `-rw-r--r--`, 실행파일 1개는 `-rwxr-xr-x`, 디렉터리 21개는 `drwxr-xr-x`, 소유자는 `0/0`이다. tar.gz의 최상위 디렉터리는 하나다. 두 묶음 모두 태그와 경로가 같은 `docs/` 파일 154개를 담았다. Linux 묶음의 `docs/`는 태그와 바이트까지 같다. Windows 묶음의 `docs/`에서 PNG 12개는 바이트까지 같고, 텍스트 파일 142개는 CR 바이트를 빼면 같다. 최상위 항목은 Linux 8종(`README.ko.md` 추가), Windows는 트레이 앱을 포함한 9종이다. 두 서버 실행파일은 `vnc-relay 2026.11.10`을 출력했다. 같은 Release로 바꾼 Windows 서비스의 기록은 [수동 운영 안내 4절](operate-relay.md#4-검증)에 있다 |

Windows에서는 PowerShell 7의 `Get-FileHash`나 `certutil -hashfile <파일> SHA256`으로 해시를 확인하기를 권장한다. Git Bash의 `sha256sum 0.8.0`은 CR 바이트가 든 해시 파일을 `-c`로 확인할 때 실제 해시가 같아도 `FAILED`로 보고할 수 있다. Ubuntu의 GNU coreutils `sha256sum`을 쓰거나 해시 값을 직접 비교한다.

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
