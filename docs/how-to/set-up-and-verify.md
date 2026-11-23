# 개발 환경을 준비하고 변경을 검증하는 방법

Windows 11 x64 또는 Ubuntu 24.04 LTS x64에서 고정된 Rust 툴체인으로 저장소를 빌드하고, 변경 범위에 맞는 검증 명령과 전체 완료 검증을 실행한다. 마지막 절차의 명령이 모두 종료 코드 0으로 끝나면 변경을 완료로 보고할 수 있다.

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 이 저장소의 코드를 바꾸는 개발자와 코딩 에이전트 |
| 검증 상태 | Draft — rustup 설치와 Windows 서비스·트레이의 미실행 분기가 남아 있다. 자동 검사 결과는 3.4절의 보고서와 CI에서 확인한다. 수동 확인 범위는 [4절](#4-검증)을 따른다. |
| 검증 환경·버전 | Ubuntu 24.04.4 LTS x86_64(WSL2), rustup 1.29.1, Rust 1.99.1, cc 13.3.0, git 2.43.0, cargo-deny 0.20.2. Windows 11 Pro 10.0.26200 x64, rustup 1.29.1, Rust 1.99.1, Visual Studio Build Tools 2022 17.14.37806.7, git 2.55.0.windows.3, cargo-deny 0.20.2, NASM 미설치 |
| 최종 수동 검증일 | 2026-11-18 |
| 관련 기준 문서 | [Rust 코드 컨벤션](../conventions/rust.md), [테스트 컨벤션](../conventions/testing.md), [코드 지도](../architectures/code-map.md), [의존성 라이선스 검사](check-dependency-licenses.md), [PRD 확정 기준](../requirements/PRD.md#확정-기준)의 지원 OS |

## 1. 대상과 영향

- 대상: 개발용 컴퓨터 한 대와 이 저장소의 작업 트리
- 예상 사용자 영향: 없다. 사용자 홈에 Rust 툴체인을 설치하고, Cargo 대상 디렉터리에 빌드 결과를, 저장소의 `.work/reports/`에 시나리오 보고서를 만든다. Windows에서는 Visual Studio Build Tools를 시스템에 설치한다.
- 작업 허용 시간: 제한 없다.

명령에서 `cargo`는 저장소 루트의 `rust-toolchain.toml`이 정한 툴체인(Rust 1.99.1, rustfmt, clippy)을 쓴다. rustup은 저장소 안에서 처음 Cargo를 실행할 때 이 툴체인을 내려받는다.

두 OS의 차이는 다음과 같다.

| 항목 | Windows 11 x64 | Ubuntu 24.04 LTS x64 |
| --- | --- | --- |
| Rust 대상 | `x86_64-pc-windows-msvc` | `x86_64-unknown-linux-gnu` |
| 링커 | Visual Studio Build Tools의 MSVC `link.exe`(C++ 빌드 도구 작업 부하) | 시스템 C 컴파일러 `cc`(gcc) |
| C 컴파일러·어셈블러(aws-lc-sys 빌드) | MSVC `cl.exe`와 NASM. NASM이 없으면 환경 변수 `AWS_LC_SYS_PREBUILT_NASM=1`로 crate에 든 미리 만든 NASM 오브젝트를 쓴다([aws-lc-rs Windows 요구 사항](https://aws.github.io/aws-lc-rs/requirements/windows.html)). | `cc`(gcc). 추가 도구는 없다. |
| 셸 | PowerShell | bash |
| 실행파일 | `target\debug\vnc-relay.exe` | `target/debug/vnc-relay` |
| 관리자 권한 | Build Tools 설치에 UAC 승인이 필요하다. | Rust 설치에는 필요 없다. `cc`가 없을 때 `apt` 설치와 Chrome 설치에 root 권한이 필요하다. WSL에서는 Windows에서 `wsl -d Ubuntu -u root`로 root 셸을 열 수 있다. |

## 2. 전제 조건

- [ ] 운영체제가 Windows 11 x64 또는 Ubuntu 24.04 LTS x64다.
- [ ] git이 설치되어 있고 저장소를 내려받았다.
- [ ] 인터넷에 연결되어 있다. rustup과 Cargo가 툴체인과 crate를 내려받는다.
- [ ] Chromium(Chrome 또는 Edge)이 설치되어 있다. Windows와 Ubuntu 모두 필수이며, 브라우저 테스트에 쓴다. 실행 파일 경로는 환경 변수 `DASHBOARD_BROWSER`로 준다(이 문서의 기본값). Ubuntu에 없으면 Google Chrome 안정판 .deb를 설치한다.

  영향: 시스템에 `google-chrome-stable` 패키지와 Google 패키지 저장소 설정(`/etc/apt/sources.list.d/google-chrome.sources`)을 더한다. `sudo apt remove google-chrome-stable`로 패키지를 지울 수 있다.

  ```shell
  wget https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb
  sudo apt install -y ./google-chrome-stable_current_amd64.deb
  google-chrome --version
  ```

  기대 결과: 마지막 명령이 `Google Chrome 156.0.8078.47`처럼 버전을 출력한다.
- [ ] mdBook 0.5.4가 설치되어 있다. Windows와 Ubuntu 모두 필수이며, 도움말 사이트 빌드(`cargo xtask help-site`)와 도움말 브라우저 시험(`tests/help_site.rs`)에 쓴다. 실행 파일 경로는 환경 변수 `MDBOOK`으로 주거나 `PATH`에 둔다. 내려받기와 SHA-256 확인은 [도움말 게시 how-to 2.1절](publish-help.md#21-mdbook-054-설치)을 따른다.
- [ ] Windows: 설치 승인(UAC)을 할 수 있는 사용자가 컴퓨터 앞에 있다.
- [ ] WSL에서 Windows 드라이브(`/mnt/<드라이브>/`)의 작업 트리를 쓰면 `CARGO_TARGET_DIR`을 Linux 홈 아래로 정하는 것을 권장한다. 예: `export CARGO_TARGET_DIR="$HOME/vnc-relay-target"`. Windows 쪽 Cargo와 빌드 결과가 섞이지 않는다.

## 3. 절차

- 실행 환경: Ubuntu는 bash, Windows는 PowerShell
- 실행 위치: 1단계와 2단계는 아무 디렉터리, 3단계부터는 저장소 루트
- 바꿔 넣을 값: 없다.
- 최대 대기 시간: 설치 단계는 각각 20분, 빌드·검증 단계는 각각 10분이다. 넘으면 Ctrl+C로 중단하고 네트워크 연결과 출력의 마지막 오류를 확인한다.

### 3.1 도구 설치

1. 링커를 준비한다.

   Ubuntu에서 `cc`가 있는지 확인한다.

   ```shell
   cc --version
   ```

   기대 결과: `cc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0`처럼 버전이 나온다. 명령을 찾지 못하면 `sudo apt install build-essential`로 설치한다.

   Windows에서는 Visual Studio Build Tools 2022의 C++ 빌드 도구를 설치한다.

   영향: 시스템에 Build Tools를 설치한다. UAC 승인 창이 뜬다. Windows "설치된 앱"에서 제거할 수 있다.

   ```powershell
   winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --source winget --accept-package-agreements --accept-source-agreements --override "--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
   ```

   기대 결과: winget이 설치 성공을 알리고 종료 코드 0으로 끝난다. 종료 코드 1602는 UAC 승인이 거절되었거나 승인하지 않은 상태다.

   Windows에서는 aws-lc-sys(HMAC·난수·TLS 암호 모듈의 C 라이브러리) 빌드에 NASM도 필요하다. NASM을 설치하지 않으면 빌드하는 PowerShell에서 `$env:AWS_LC_SYS_PREBUILT_NASM = "1"`을 먼저 실행한다. crate에 든 미리 만든 NASM 오브젝트를 쓴다. [nasm.us](https://nasm.us/)에서 NASM을 설치하고 `PATH`에 더할 수도 있다.

   Linux에서 `--target x86_64-pc-windows-msvc`로 `cargo clippy`를 실행하는 타입 검사는 할 수 없다. aws-lc-sys의 빌드 스크립트가 Windows용 C 컴파일러를 요구한다. Windows 분기는 Windows에서 6단계를 실행해 확인한다.

2. rustup을 설치한다. 이미 있으면 건너뛴다.

   영향: 사용자 홈의 `.cargo`와 `.rustup`에 설치한다. `rustup self uninstall`로 지울 수 있다.

   Ubuntu:

   ```shell
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
   . "$HOME/.cargo/env"
   ```

   Windows:

   ```powershell
   Invoke-WebRequest https://win.rustup.rs/x86_64 -OutFile rustup-init.exe
   .\rustup-init.exe -y --profile minimal
   ```

   기대 결과: 새 셸에서 `rustup --version`이 버전을 출력한다.

   Windows에서 명령을 찾지 못하면 `%USERPROFILE%\.cargo\bin`이 `PATH`에 없는 것이다. 그 PowerShell에서 `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`를 먼저 실행한다. 사용자 `PATH`에 영구로 더할 수도 있다.

#### WSL 끊긴 연결 감지 시험 준비

Linux 전용 `tests/relay_dead_peer_linux.rs`는 사용자·네트워크 네임스페이스 안에서 루프백 패킷을 `nft`로 버린다. 호스트의 방화벽과 네트워크 설정은 바꾸지 않는다. Windows에서는 응답 없이 끊긴 네트워크를 root 없이 만들 수단이 없어 부분 시험으로 확인한다.

1. WSL Ubuntu에서 사용자가 `nftables`를 설치한다.

   ```shell
   sudo apt-get install -y nftables
   ```

   영향: WSL 배포판에 `nftables` 패키지를 설치한다. 시험 자체에는 `sudo`를 쓰지 않는다.

   WSL의 `sudo` 암호를 입력할 수 없으면 Windows PowerShell에서 root로 설치할 수 있다.

   ```powershell
   wsl -d Ubuntu -u root -- bash -lc 'DEBIAN_FRONTEND=noninteractive apt-get install -y nftables'
   ```

2. WSL Ubuntu에서 필수 도구를 확인한다.

   ```shell
   command -v unshare
   command -v ip
   command -v nft
   ```

   기대 결과: 각 명령이 도구 경로를 출력하고 종료 코드 0으로 끝난다.

3. 권한 없는 사용자로 네임스페이스를 만들 수 있는지 확인한다.

   ```shell
   unshare -rn true
   ```

   기대 결과: 출력 없이 종료 코드 0으로 끝난다. 시험은 `unshare --user --map-root-user --net`으로 자신을 다시 실행한다. 필수 도구나 사용자 네임스페이스가 없으면 누락 조건을 밝히고 실패한다. 조용히 건너뛰지 않는다.

4. 저장소 루트에서 Linux 전용 시험을 실행한다.

   ```shell
   cargo test --test relay_dead_peer_linux -- --nocapture
   ```

   기대 결과: 대기 Server와 활성 세션의 Viewer·Server·Mode I 대상에 실제 DROP을 적용한 시험 2건이 통과한다. 한 번 실행에 약 47초 걸린다. K는 설정 최솟값인 11초다. Linux 타이머·스케줄링 오차는 최대 5초로 제한한다(DROP 직전 시각 기준 최대 16초). 이 오차를 K 설정값에 더하지는 않는다.

   시험은 DROP 직전 시각과 정리 사건 줄을 읽은 시각을 단조 시계(`Instant`)로 비교한다. 로그 `ts`는 벽시계라서 쓰지 않는다. WSL2는 실행 중에 벽시계를 보정할 수 있어 경과 시간이 왜곡될 수 있다. 로그를 읽는 지연(10ms 주기 확인)은 측정값을 늘리기만 하므로 상한 판정을 느슨하게 하지 않는다. 마지막 활동 이후 9초 이상이라는 하한 판정은 그 지연만큼 느슨해진다.

#### noVNC 상호운용 시험 준비

`tests/websocket_browser.rs`의 WS-025 시험은 고정한 noVNC 1.7.0 압축 파일이 있어야 실행할 수 있다. [테스트 컨벤션](../conventions/testing.md)에 따라 `#[ignore]`가 붙어 있어 6단계의 기본 실행에서는 `미실행`으로 보고된다. 같은 파일의 WS-020 시험은 noVNC가 필요 없으며 6단계에서 함께 실행된다. noVNC는 시험 중에만 임시 디렉터리에 풀고, 저장소와 배포물에 넣지 않는다.

1. 고정한 압축 파일 `novnc-v1.7.0.zip`을 준비한다. 받은 주소와 SHA-256은 [호환성 검증 기록 2.4절](../contracts/repeater-compat-record.md)에 있다. 아래 예시는 `D:\vr-lab\installers\novnc-v1.7.0.zip`(WSL에서는 `/mnt/d/vr-lab/installers/novnc-v1.7.0.zip`)을 쓴다.
2. 압축을 풀 도구를 확인한다. Windows는 기본 제공 `%SystemRoot%\System32\tar.exe`를, Ubuntu는 `python3 -m zipfile`을 쓴다.

   ```shell
   python3 --version
   ```

   기대 결과: Ubuntu에서 Python 3 버전이 출력된다. Windows에서는 이 단계를 건너뛴다.

3. 저장소 루트에서 압축 파일 경로를 환경 변수 `VNC_RELAY_NOVNC_ZIP`에 주고 무시된 시험만 실행한다.

   ```shell
   VNC_RELAY_NOVNC_ZIP=/mnt/d/vr-lab/installers/novnc-v1.7.0.zip cargo test --test websocket_browser -- --ignored
   ```

   ```powershell
   $env:VNC_RELAY_NOVNC_ZIP = 'D:\vr-lab\installers\novnc-v1.7.0.zip'
   cargo test --test websocket_browser -- --ignored
   ```

   기대 결과: 시험 1건이 통과한다. 시험은 압축 파일의 SHA-256이 고정값과 다르거나, 환경 변수가 없거나, 압축을 풀 수 없으면 실패한다. 조용히 건너뛰지 않는다.

4. 실행 결과를 완료 보고에 적는다. 대상 커밋, OS, 명령, 결과를 구분한다. 시나리오 상태는 `scenario-report.md`에서 확인한다. 실제 UltraVNC·noVNC 수동 상호운용 결과는 [호환성 검증 기록](../contracts/repeater-compat-record.md)에 남긴다.

### 3.2 빌드

3. 저장소 루트에서 고정 툴체인을 확인한다.

   ```shell
   rustc --version
   ```

   기대 결과: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`. 처음 실행하면 rustup이 툴체인을 먼저 내려받는다.

4. 빌드하고 실행파일을 확인한다.

   ```shell
   cargo build
   cargo run --quiet -- --version
   ```

   기대 결과: 빌드가 `Finished`로 끝나고 `vnc-relay <버전>`이 출력된다. `<버전>`은 루트 `Cargo.toml`의 `[workspace.package] version`이다. 형식은 [버전 규칙](release.md#11-버전-규칙)을 따른다.

### 3.3 변경 범위별 검증

5. 바꾼 범위에 맞는 명령을 실행한다. 여러 범위에 걸치면 해당 행을 모두 실행한다.

   | 바꾼 범위 | 실행할 명령 | 성공 조건 |
   | --- | --- | --- |
   | Rust 코드·테스트 | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` | 모두 종료 코드 0, clippy 경고 없음 |
   | 시나리오 테스트(표식 추가·변경), 스펙 수용 테스트 표 | `cargo xtask scenarios` | 종료 코드 0, 보고서에 `오류`·`실패` 없음([테스트 컨벤션 5절](../conventions/testing.md#5-검증-명령과-통과-기준)) |
   | `Cargo.toml` 의존성, crate 추가 | `cargo xtask layers`, `cargo deny check licenses`([의존성 라이선스 검사](check-dependency-licenses.md)) | `레이어 검사: 통과`, `licenses ok` |
   | `docs/design/DESIGN.md` YAML, `web/dashboard/` | `cargo xtask tokens --check`, 브라우저 테스트는 `cargo xtask scenarios`가 함께 실행한다([대시보드 프런트엔드 검증 how-to](verify-dashboard-frontend.md)) | `tokens --check: 통과`, 종료 코드 0 |
   | `docs/help/`, `web/help/`, 도움말 도구(`xtask/src/help_site.rs`·`help_site/`·`help_serve.rs`·`static_server.rs`), `.github/workflows/help.yml` | `cargo xtask help-site`, `cargo test --test help_site`, `web/help/` CSS를 바꾸면 `cargo xtask tokens --check`. 워크플로를 바꾸면 push 뒤 `help` 실행 결과를 확인한다([도움말 게시 how-to](publish-help.md)) | `help-site: 통과`, 시험 17개 통과, `help` 결론 `success` |
   | 배포 묶음(`xtask/src/package.rs`), GitHub Actions 워크플로(`.github/workflows/`), 작업 공간 버전 | `cargo xtask package`, `cargo test --package xtask`. 워크플로를 바꾸면 push 뒤 `ci` 실행 결과를 확인한다([릴리스 how-to](release.md)) | 종료 코드 0, `package:`·`sha256:` 줄, `ci` 결론 `success` |
   | 시험용 접속 ticket 도구(`xtask/src/ticket.rs`) | `cargo test --package xtask`. 실제 서버로 확인할 때는 [수동 운영 안내 3.10.1절](operate-relay.md#3101-시험용-키와-ticket으로-확인하기)을 따른다 | 종료 코드 0, 3.10.1절의 기대 결과 |
   | 문서만 | 이 절차의 명령은 필요 없다. 문서 유형별 작성 규칙과 링크를 확인한다. | — |

   한 crate만 빠르게 확인할 때는 `cargo test --package <패키지 이름>`을 쓸 수 있다. 완료 보고 전에는 6단계를 실행해야 한다.

설정 예시·시험용 파일을 v4에 맞출 때는 [설정 계약 3.3절](../contracts/config-v4.md#33-v3-파일을-v4로-옮기기)의 매핑을 따른다. TCP 키는 그대로다. WS·WSS의 공통 리스너 키와 역할 키는 분리한다. 별도 버전 키는 넣지 않는다. 운영 파일 이관·대체 포트 검증·롤백은 [수동 운영 안내 3.2.2절](operate-relay.md#322-v3-파일을-v4로-옮기기)을 따른다.

### 3.4 전체 완료 검증

`Cargo.lock`이 바뀌었으면 6단계 전에 각 호스트에서 [의존성 라이선스 검사 3.1절](check-dependency-licenses.md#31-배포-고지-생성과-대조) 1단계로 두 대상의 crate를 받는다. `cargo xtask scenarios`가 실행하는 고지 시험(`tests/dependency_notices.rs`)은 네트워크 없이 실행되므로, 받지 않은 crate가 있으면 실패한다.

6. 저장소 루트에서 다음 명령을 순서대로 실행한다. 하나라도 실패하면 멈추고 [5절](#5-중단-조건)을 따른다.

   ```shell
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo xtask layers
   cargo xtask tokens --check
   cargo xtask scenarios
   cargo deny check licenses
   ```

   기대 결과: 여섯 명령 모두 종료 코드 0. `cargo xtask scenarios`가 `cargo test --workspace --no-fail-fast`를 함께 실행하므로 따로 실행하지 않는다. 대시보드 브라우저 테스트도 이때 실행되며 Chromium이 없으면 실패한다(2절). 도움말 브라우저 시험도 이때 실행되며 mdBook 0.5.4가 없으면 실패한다(2절). 출력 마지막에 `판정: 통과`, 라이선스 검사 마지막 줄에 `licenses ok`가 나온다. 보고서에 남은 `부분`·`미실행`·`미연결`은 자동 검사의 범위를 뜻한다. 해당 범위는 [4절의 수동 근거](#4-검증)와 대조하고, 확인하지 않은 분기는 통과로 합치지 않는다. `cargo xtask scenarios --require-all`은 완료 명령이 아니다. 남은 시나리오 수를 세는 진단용이며, 그런 시나리오가 하나라도 있으면 판정 실패와 종료 코드 1로 끝난다([테스트 컨벤션 5절](../conventions/testing.md#5-검증-명령과-통과-기준)).

7. 결과를 기록한다. 시나리오 보고서는 저장소 루트의 `.work/reports/scenario-report.md`다. Windows와 WSL이 같은 작업 트리를 쓰면 나중에 실행한 쪽이 보고서를 덮어쓴다. 각 OS의 결과는 실행 직후 기록한다. 완료 보고에는 대상 커밋, OS, 명령, 판정을 적는다([검증 기록](../AGENTS.md#검증-기록)).

   WSL에서 Windows 작업 트리를 쓰면 줄 끝(CRLF·LF) 차이 때문에 보고서의 대상 커밋에 `+ 미커밋 변경`이 붙을 수 있다. 이때는 Windows의 `git status`로 실제 변경 여부를 확인한다.

## 4. 검증

자동 검사 결과는 `cargo xtask scenarios`가 `.work/reports/scenario-report.md`에 만드는 보고서와 CI에서 확인한다. 이 절에는 자동 검사로 다시 얻을 수 없는 최신 수동 근거와 미실행 범위만 둔다. 기록 방법은 [검증 기록](../AGENTS.md#검증-기록)을 따른다.

### 4.1 도구 설치 확인

| 확인 대상 | 날짜·대상 | 환경·방법 | 실제 결과 |
| --- | --- | --- | --- |
| Windows Build Tools 설치 | 2026-10-20, 커밋 `e1cfc4f` 기반 작업 트리 | Windows 11 Pro 10.0.26200 x64. 3.1절의 winget 설치 명령을 UAC 승인 뒤 실행하고 `vswhere`로 확인했다 | 통과. VCTools 작업 부하가 있는 Build Tools 2022 17.14.37806.7을 찾았다. NASM 없이 `AWS_LC_SYS_PREBUILT_NASM=1`을 사용했다 |
| Ubuntu Chrome 설치 | 2026-10-22, 커밋 `df3b5b2` | Ubuntu 24.04.4 LTS x86_64(WSL2, 커널 6.6.87.2-microsoft-standard-WSL2). Windows의 root 셸에서 아래 명령으로 설치했다 | 통과. `command -v google-chrome`은 `/usr/bin/google-chrome`, `dpkg-query -W google-chrome-stable`은 `156.0.8078.47-1`이었다. 내려받은 .deb는 설치 뒤 지웠다 |
| rustup 설치 | 두 OS의 3.1절 설치 절차 | 검증 환경에 rustup이 이미 설치되어 있었다 | 미실행. 설치 명령 자체는 확인하지 않았다 |

Chrome 설치에 사용한 명령은 다음과 같다.

```powershell
wsl -d Ubuntu -u root -- bash -lc 'cd /tmp && wget -q https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb && DEBIAN_FRONTEND=noninteractive apt-get install -y -q ./google-chrome-stable_current_amd64.deb'
```

### 4.2 수동 근거 위치와 판정 범위

다음 확인은 각 문서의 최신 수동 기록을 따른다. 대상·환경·방법·날짜·결과는 연결한 기록에서 확인한다. 같은 확인의 결과를 이 문서에 중복해서 옮기지 않는다.

| 확인 대상 | 근거 위치·범위 |
| --- | --- |
| DASH-015·023의 실제 키 입력·브라우저 흐름과 화면 시안 대조 | [대시보드 프런트엔드 검증](verify-dashboard-frontend.md). 실제 Safari, 최신 안정판 브라우저, 실기기, 스크린 리더의 확인 범위를 구분한다 |
| OPS-017·018의 성능 측정 | [성능 측정](measure-performance.md). v4 설정의 재측정은 미실행이다 |
| OPS-020, RLY-006·010·011, WS-025의 실제 UltraVNC·noVNC 상호운용 | [호환성 검증 기록](../contracts/repeater-compat-record.md), [UltraVNC 수동 확인](verify-ultravnc-manually.md). Windows VNC Relay와 UltraVNC 1.8.3.0·noVNC 1.7.0의 확인 범위를 따른다. Ubuntu VNC Relay와 실제 UltraVNC, WS·WSS Server 역할의 실제 클라이언트 조합은 미실행이다 |
| OPS-021의 출처·프로젝트 LICENSE 수동 검토 | [의존성 라이선스 검사](check-dependency-licenses.md). 자동 고지 대조와 수동 검토를 구분한다 |
| OPS-031의 배포 묶음 운영, 설정 이관·롤백, 접속 ticket 운영 | [수동 운영 안내](operate-relay.md). v3→v4 이관은 Windows에서 관리 재로그인을 빼고 실행했다. 롤백과 Ubuntu 이관은 미실행이다. ticket 켜기·공개 키 교체·끄기는 두 OS의 Release `v2026.11.3` 묶음으로 실행했다 |
| OPS-096·097의 실제 IP 정보 DB 파일 읽기 | [수동 운영 안내 4절](operate-relay.md#4-검증). Windows debug 실행파일로 DB-IP Lite, IPinfo Lite 견본, GeoLite 시험 파일 조합의 시작 시 읽기를 확인했다. IPinfo Lite·GeoLite 전체 파일, 실행 중 교체(OPS-097), Release 묶음, Ubuntu는 미실행이다 |
| 도움말 사이트의 시안 대조·게시 수동 확인 | [도움말 게시](publish-help.md). 실제 Safari·Edge·Firefox·실기기·스크린 리더, 사람이 하는 시안 비교와 Pages 끄기는 미실행이다 |

자동 시험의 OS별 관찰 범위도 구분한다. Windows에서는 큐가 찬 `listen()`이 WSAECONNREFUSED로 거절되어 연결 중 대상 소켓을 만들 수 없다. 따라서 OPS-023·RLY-009의 해당 경계는 Ubuntu 시험으로 확인한다. RLY-025·026의 실제 DROP은 3.1절의 Linux 전용 시험으로 확인한다. OPS-044는 Windows 전용이다. OPS-040의 IPv6 경계는 단위 시험으로 확인한다.

OPS-050의 Windows 시험은 수락·대상 소켓의 K 값을 다시 읽어 대조하지 못한다. socket2 0.6.5의 `tcp_keepalive_time`·`tcp_keepalive_interval`·`tcp_keepalive_retries`가 Windows를 지원하지 않기 때문이다. 두 OS의 수락 쪽 K는 같은 `admit` 경로가 같은 설정 세대에서 복사한다. Ubuntu 시험은 실제 값을 대조하고 Windows 대상 시험은 keepalive 켜짐을 확인한다.

다음 수동 확인에는 별도 환경이나 승인이 필요하다. 담당은 프로젝트 관리자이며, 저장소 밖의 연결 확인은 사용처 담당자가 맡는다.

- Nginx 실제 배치의 TLS 종료, 공유 WSS Viewer·Server, 최근 로그 운영 절차.
- 두 OS의 Viewer 경로 이전·롤백과 사용처 페이지의 접속 URL 변경 뒤 실제 연결.
- 실제 발급 쪽과 noVNC·VNC Server 클라이언트가 ticket을 보내는 연결.
- 별도 서비스 로그온 계정의 설정 디렉터리 생성·쓰기·교체 권한. 자동 시험은 시험 실행 계정에서만 확인한다.

### 4.3 Windows 서비스·트레이 수동 근거

- 날짜·대상: 2026-11-18, 기반 커밋 `6e5cb2d` 위의 미커밋 Windows 서비스·트레이 구현 작업 트리다. 구현·시험·배포 고지 변경을 포함한다. 관리자 시험은 정리 가드 강화 전의 실행이다. PE 헤더 확인 대상은 커밋 `738d068`의 코드로 만든 실행파일이다.
- 환경: 한국어 Windows 11 Pro 빌드 26200, 로컬 관리자 계정이다. 일반 터미널은 실제 UAC 필터링 세션의 Medium IL(`S-1-16-8192`)이다. 트레이는 debug 빌드다.
- 방법: 관리자 시험은 [8.1절](#81-자동-시험과-안전-조건)의 빌드와 시험 실행파일 직접 실행을 썼다. 수동 시험은 실제 일반 터미널의 CLI·`sc.exe`와 서비스 상태 조회를 썼다. 트레이는 UI Automation과 Win32 메뉴 메시지로 관찰했다.
- 범위: 아래 개별 결과만 기록한다. 전체 게이트·Release 검증이나 모든 시나리오 통과를 뜻하지 않는다. AC-57~59는 미실행 분기가 남아 부분 충족이다.

| 확인 대상 | 실제 결과·증거 |
| --- | --- |
| 관리자 권한의 실제 SCM 시험(OPS-074~077·080) | 통과. 사용자 승인으로 실행한 `--ignored --test-threads=1 --nocapture` 결과는 `4 passed; 0 failed`, 13.73초다. 등록 속성·운영자 권한·설치 실패 정리·SCM 시작과 유예 종료·시작 실패 종료 코드를 확인했다. 시작 실패의 서비스 종료 코드는 3·3·4·5였다. 실행 중 전 상태는 `[StartPending]`, 중지 중 두 번째 요청은 `exit Some(1061)`이었다 |
| 최종 정리 가드의 관리자 실행 | 미실행. 위 실행 뒤 fixture 정리 가드를 실행파일 토큰 정확 일치 조건으로 강화했다. 최종 가드로 등록·정리하는 관리자 실행 결과는 없다 |
| 서비스 운영자(OPS-079) | 통과. `--operator`로 지정한 사용자의 실제 일반 터미널에서 `sc.exe stop`은 0 뒤 STOPPED, `sc.exe start`는 0 뒤 RUNNING이었다. UAC 창은 없었다 |
| 일반 세션 설치(OPS-075) | 통과. `vnc-relay.exe service install --config relay.toml`은 종료 코드 1과 `error[service_access_denied]: 서비스를 다룰 권한이 없다. 관리자 권한 터미널에서 실행한다.`였다 |
| 일반 세션 해제(OPS-080) | 통과. 실행 중 `service uninstall`은 종료 코드 1과 같은 권한 오류였다. 서비스는 RUNNING을 유지했다 |
| 운영자 지정 없음(OPS-079) | 통과(비상승 관리자 세션 범위). `--operator` 없이 설치한 서비스는 트레이에 `시작 실패: 권한 없음`·`중지 실패: 권한 없음`을 표시했다. 관리자 권한 시작은 성공했다 |
| 실제 표준 사용자 | 미실행. PC에 해당 계정이 없다. 비상승 관리자 세션의 거절로 대체하지 않는다 |
| 로그오프·재시작(OPS-078) | 미실행. Windows 로그오프·재시작이 필요하다. 시스템 종료 시간 제한(OPS-TBD-014)도 관찰하지 못했다 |
| 기본 파일 접근 | 통과(관찰 환경). 관리자가 `%ProgramData%` 아래 만든 폴더에 별도 허용을 주지 않아도 서비스가 설정·계정을 읽고 로그를 만들고 썼다. 일반 세션 사용자도 설정을 읽어 트레이를 썼다. 기본 ACL에서 서비스 RUNNING을 확인했다. 로컬 사용자에게 비밀 파일도 읽힐 수 있으므로 [파일 접근 제한](operate-relay.md#331-windows-서비스와-트레이-앱)을 권장한다 |

OPS-074가 기록한 기본 서비스 보안 설명자(SDDL)는 다음과 같다. IU(대화형 사용자)는 상태·구성 조회만 할 수 있고 시작·중지는 할 수 없다. `--operator`는 시작·중지·상태 조회·구성 조회를 추가한다.

```text
D:(A;;CCLCSWRPWPDTLOCRRC;;;SY)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCSWLOCRRC;;;IU)(A;;CCLCSWLOCRRC;;;SU)S:(AU;FA;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;WD)
```

트레이 관찰 결과는 다음과 같다. 시간은 클릭·설치 완료 같은 측정 기준점부터 잰 값이다.

| 시나리오 | 결과 | 실제 관찰·미실행 이유 |
| --- | --- | --- |
| TRAY-001 | 통과 | 상태 줄, 시작, 중지, 대시보드 열기, 로그 파일 위치 열기, 설정 파일 위치 열기, 로그인할 때 시작, 트레이 끝내기 순서다. 다시 시작과 UAC 창은 없었다. `vnc-relay-tray.exe`의 PE 헤더 Subsystem 값은 2(GUI), 같은 빌드의 `vnc-relay.exe`는 3(콘솔)이었다 |
| TRAY-002 | 통과 | 두 번째 프로세스는 실행 시간을 포함해 약 1.1초에 종료 코드 0으로 끝났다. 아이콘 하나·프로세스 하나였다 |
| TRAY-003 | 통과(관찰한 상태·켜짐), 미실행(실제 503·반영 시간) | 중지됨·시작 중·실행 중·실행 중(준비 안 됨)·중지 중·중지됨 (종료 코드 3)을 확인했다. 준비 안 됨은 설정의 상태 주소를 미사용 포트로 바꾼 연결 실패다. 실제 `/readyz`는 200이었다. 잘못된 설정의 시작 실패는 SCM `WIN32_EXIT_CODE` 1066, `SERVICE_EXIT_CODE` 3이었다. 켜짐은 스펙 4절과 같았다. 실제 503과 상태 변화의 다음 조회 반영 시간은 확인하지 못했다 |
| TRAY-004 | 통과 | 클릭 뒤 시작 중까지 약 1.5초, 실행 중까지 약 2.9초였다. UAC 창은 없었다 |
| TRAY-005 | 통과 | TCP Mode II 활성 세션 1개와 G=10초에서 클릭 뒤 0.3초에 중지 중, 10.2초에 중지됨이었다. 확인 대화상자는 없었다. `shutdown_started`의 `trigger`는 `requested`, `grace_secs`는 10, `active_sessions`는 1이었다. 이어 `shutdown_forced`의 `remaining_sessions`는 1이고 `server_stopped`를 기록했다 |
| TRAY-006 | 통과(시작·중지 거절), 미실행(조회 거절·실제 표준 사용자) | 운영자가 아닌 일반 세션에서도 기본 IU 조회 권한으로 상태가 보였다. 시작·중지 권한 실패 뒤 서비스 상태는 그대로였고 다른 동작을 고르면 실패 줄이 사라졌다. 기본 SDDL이 조회를 허용하므로 조회 거절 분기는 만들지 못했다. 실제 표준 사용자 계정은 없었다 |
| TRAY-007 | 통과(서비스 없음 메뉴·해제 반영·해제 지연 없음), 판정 보류(설치 반영 시간) | 서비스 없음에서 시작·중지·대시보드·두 위치 열기는 꺼졌다. 해제 완료 06:22:50.276→서비스 없음 06:22:50.726은 0.45초다. 트레이 실행 중 해제는 81~86ms로 지연되지 않았다. 설치 완료 06:22:56.437→중지됨 06:22:58.734는 2.30초다. 관찰기의 표본 간격이 0.3~0.5초라서 실제 표시 시각은 설치 완료 뒤 1.8~2.3초 사이로만 좁혀진다. 기준은 다음 조회(시작 기준 2초 간격)에서의 반영이다. 표시에는 조회 시간과 최대 0.1초의 UI 대기가 더해진다. 조회 시간을 재지 않아 통과 여부를 확정하지 못했다. 정밀 측정에는 관리자 권한 설치와 사용자 승인이 필요하다 |
| TRAY-008 | 미실행(실제 브라우저) | 사용자의 기본 브라우저를 열게 되므로 실행하지 않았다. 실제 브라우저 열기까지 확인한 것은 아니다 |
| TRAY-009 | 통과 | 실행 중·중지됨에서 읽기 거절→`설정을 읽지 못함: 권한 없음`, 삭제→`설정을 읽지 못함: 파일 없음`, 잘못된 파일→`설정을 읽지 못함: 설정 오류`였다. 실행 중 상태는 실행 중(준비 안 됨)이었다. 대시보드·로그 위치는 꺼지고 설정 위치는 켜졌다. 원인을 없애면 다음 갱신에서 정상으로 돌아왔다 |
| TRAY-010 | 통과(파일 있음), 미실행(로그 없음) | 설정 위치는 탐색기에서 `relay.toml`, 로그 위치는 `relay.log`를 선택했다. 로그 파일이 없는 경우의 디렉터리 열기는 확인하지 못했다 |
| TRAY-011 | 통과(Run 값), 미실행(다시 로그인) | 값 없음은 체크 꺼짐이었다. 켜면 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`의 `vnc-relay-tray` 값에 따옴표 친 실행파일 경로를 쓰고 체크됐다. 끄면 지웠다. 다른 경로 값은 체크 꺼짐이고 켜면 현재 실행파일 경로로 덮어썼다. 시험 뒤 원래의 값 없음으로 복구했다. 다시 로그인은 실행하지 않아 실제 자동 실행은 확인하지 못했다 |
| TRAY-012 | 미실행(표시 언어 전환) | Windows 표시 언어 변경과 다시 로그인이 필요하다. 언어별 실제 메뉴는 확인하지 못했다 |
| TRAY-013 | 통과(종료·서비스·Run 값), 미실행(요청·파일 열기 관찰) | 서비스 실행 중과 로그인할 때 시작 켜짐에서 트레이 끝내기를 골랐다. 아이콘이 사라지고 프로세스는 0으로 끝났다. 서비스는 RUNNING이고 Run 값은 남았다. 시험 뒤 그 값을 지웠다. 짧은 `/readyz` 연결을 표본 수집으로 잡지 못해 네트워크·파일 열기 관찰은 완료하지 못했다. 코드 검토로만 worker가 상태 리스너에 연결하고 설정 파일만 읽는 것을 확인했다 |

트레이 프로세스가 확인 중 한 번 끝난 관찰은 원인 미확인으로 남아 있다. 대상은 1077 표시 수정 전 빌드다. Windows Error Reporting 사건은 없었고 종료 코드는 수집하지 못했다. 이후 세 번 실행에서는 구분선 클릭을 포함해 재현하지 못했다. 정상 종료나 해결된 문제로 처리하지 않는다.

설치 직후 SCM 값 1077을 종료 코드 없음으로 다루는 빌드에서는 `중지됨`을 확인했다. 이는 부팅 뒤 아직 시작하지 않은 서비스의 표시다. 모든 시작 실패를 숨기는 예외는 아니다.

남은 Windows 수동 확인 담당은 프로젝트 관리자다. 최종 정리 가드의 관리자 실행, TRAY-007 정밀 측정, 로그오프·재시작, 실제 표준 사용자, 표의 미실행 분기를 확인해야 한다. 절차는 [8절](#windows-서비스-시험)을 따른다.


## 5. 중단 조건

- 1단계 Build Tools 설치가 1602로 끝나면 이후 단계를 진행하지 않는다. UAC를 승인할 수 있을 때 다시 실행한다.
- 3단계 버전이 `rustc 1.99.1`이 아니면 진행하지 않는다. `rust-toolchain.toml`과 `rustup show active-toolchain` 출력을 확인한다.
- 6단계 명령이 하나라도 실패하면 완료로 보고하지 않는다. 실패한 명령의 출력을 고친 뒤 6단계를 처음부터 다시 실행한다.

## 6. 롤백 또는 복구

빌드 결과와 보고서는 소스에서 다시 만든다. 지워도 소스는 바뀌지 않는다.

1. 시나리오 보고서와 임시 파일을 지우려면 저장소 루트의 `.work/reports/`와 `.work/tmp/`를 지운다. `.work/evals/`는 사용자가 결과를 검토하기 전에는 지우지 않는다. 다른 하위 폴더의 지우는 시점은 [루트 지침의 부수 파일 위치](../../AGENTS.md#부수-파일-위치)를 따른다.
2. 빌드 결과가 꼬였다고 의심되면 저장소 루트에서 `cargo clean`을 실행한다. Cargo 대상 디렉터리(`target/`, `CARGO_TARGET_DIR`을 정했으면 그 위치)를 지운다. 지우기 전에 `cargo clean --dry-run`으로 지울 파일 수와 크기를 확인할 수 있다. `--dry-run`은 파일을 지우지 않는다.
3. Cargo가 내려받은 의존성 캐시(Cargo 홈의 `registry/`)는 `cargo clean`이 지우지 않는다. 지우면 다음 빌드에서 다시 내려받는다. 디스크가 모자랄 때만 지운다.
4. 절차 4단계를 다시 실행해 `vnc-relay <버전>`이 나오는지 확인한다.

## 7. 해결되지 않을 때 전달할 대상

- 전달 대상: vnc-relay 프로젝트 관리자
- 보존할 증거: 실패한 명령과 전체 출력, OS와 `rustc --version` 출력, 대상 커밋, `scenario-report.md`
- 비밀 값 제외 규칙: 검증 명령은 비밀 값을 다루지 않는다. 환경 변수 전체를 붙여 넣지 않는다.

<a id="windows-서비스-시험"></a>

## 8. Windows 서비스 시험

Windows 11 x64에서 실제 서비스 등록과 트레이 앱을 확인한다. 기준은 [운영 스펙 3.7절](../specs/operations.md#37-windows-서비스-실행), [CLI 계약 1.9절](../contracts/cli-v1.md#19-windows-서비스-실행), [Windows 트레이 앱 스펙](../specs/windows-tray.md)이다. 운영 배치는 [수동 운영 안내 3.3.1절](operate-relay.md#331-windows-서비스와-트레이-앱)을 따른다.

### 8.1 자동 시험과 안전 조건

`tests/windows_service.rs`의 관리자 권한 시험 4개는 PC의 실제 서비스 `vnc-relay`를 등록한다. 그래서 기본 실행에서는 `#[ignore]`로 제외한다. 기본 시험은 터미널의 `service run`이 종료 코드 2로 끝나고 파일·리스너를 열지 않는지만 확인한다.

운영 서비스가 있는 PC에서는 관리자 권한 시험을 실행하지 않는다. 시험은 기존 `vnc-relay` 서비스나 같은 fixture 디렉터리가 있으면 시작하지 않는다. 관리자 권한 시험의 fixture는 `%ProgramData%\vnc-relay-tests\<pid>-<name>`에 둔다. 정리할 때는 등록 명령의 실행파일 토큰이 자기 복사본과 정확히 일치하는 서비스만 지운다. 이름만 보고 기존 서비스를 지우지 않는다.

1. Windows PowerShell에서 기존 서비스 유무를 확인한다. 서비스가 있으면 이 절차를 중단한다.

   ```powershell
   sc.exe query vnc-relay
   ```

   기대 결과: 서비스 없음(1060)이다.

2. 저장소 루트의 개발 터미널에서 시험 실행파일을 빌드한다. NASM이 없으면 3.2절의 환경 변수 설정을 먼저 적용한다.

   ```powershell
   cargo test --locked --test windows_service --no-run
   ```

3. 관리자 권한 PowerShell에서 빌드 출력에 나온 시험 실행파일을 실행한다. `<시험 실행파일 절대 경로>`는 출력의 실제 경로로 바꾼다.

   ```powershell
   & "<시험 실행파일 절대 경로>" --ignored --test-threads=1 --nocapture
   ```

   기대 결과: 관리자 권한 시험 4개가 통과한다. 등록 속성·운영자 권한·설치 실패 정리·SCM 시작과 유예 종료·시작 실패 종료 코드를 확인한다.

4. 시험 뒤 서비스가 남지 않았는지 확인한다.

   ```powershell
   sc.exe query vnc-relay
   ```

   기대 결과: 서비스 없음(1060)이다. 예상 밖의 등록이 있으면 실행파일 경로와 소유 관계를 먼저 확인한다. 다른 서비스는 삭제하지 않는다.

### 8.2 실제 일반 터미널 확인

관리자 권한 프로세스에서 `runas /trustlevel:0x20000`로 만든 프로세스를 일반 터미널의 대용으로 쓰지 않는다. UAC로 필터링된 실제 일반 세션에서 확인해야 한다. 시험용 서비스의 설정·계정·로그 파일은 운영 파일과 분리한다.

1. 관리자 권한 터미널에서 [서비스 설치 절차](operate-relay.md#331-windows-서비스와-트레이-앱)에 따라 시험 사용자를 `--operator "<사용자>"`로 지정한다.
2. 실제 일반 터미널에서 토큰의 무결성 수준을 확인한다.

   ```powershell
   whoami /groups
   ```

   기대 결과: Medium IL(`S-1-16-8192`)이다.

3. 같은 일반 터미널에서 서비스를 시작한다.

   ```powershell
   sc.exe start vnc-relay
   ```

4. `sc.exe query vnc-relay`로 `RUNNING`을 확인한다.
5. 같은 일반 터미널에서 서비스를 중지한다.

   ```powershell
   sc.exe stop vnc-relay
   ```

6. `sc.exe query vnc-relay`로 `STOPPED`를 확인한다.
7. 같은 일반 터미널에서 설치 명령을 실행한다.

   ```powershell
   .\vnc-relay.exe service install --config "<시험 설정 파일 경로>"
   ```

   기대 결과: UAC 창 없이 종료 코드 1과 `service_access_denied`다.

8. 서비스를 다시 시작한다.
9. 같은 일반 터미널에서 해제 명령을 실행한다.

   ```powershell
   .\vnc-relay.exe service uninstall
   ```

   기대 결과: 종료 코드 1과 `service_access_denied`다. 서비스는 계속 실행 중이다.

10. 관리자가 서비스를 중지한다.
11. 관리자가 서비스를 해제한다.
12. 운영자 지정 없는 권한 시험은 다음 순서로 진행한다.

    1. 관리자가 `--operator` 없이 서비스를 다시 설치한다.
    2. 같은 일반 사용자로 트레이의 `시작`을 고른다.
    3. 관리자가 서비스를 시작한다.
    4. 같은 일반 사용자로 트레이의 `중지`를 고른다.

   기대 결과: `시작 실패: 권한 없음`·`중지 실패: 권한 없음`이며 서비스 상태는 바뀌지 않는다. 실제 표준 사용자 계정에서도 별도로 확인한다.

13. 시험이 끝나면 관리자가 시험 서비스를 중지한다.
14. 관리자가 시험 서비스를 해제한다.

### 8.3 트레이 확인 목록

일반 세션에서 Windows 묶음의 `vnc-relay-tray.exe`를 실행한다. 아래 목록은 [트레이 수용 테스트](../specs/windows-tray.md#7-수용-테스트)의 확인 순서다. 비밀 파일은 사용하지 않는다. 로그인·표시 언어를 바꾸는 확인은 사용자 승인을 받은 환경에서 한다.

- [ ] TRAY-001: 아이콘 하나, 콘솔·UAC 창 없음, 메뉴 순서와 다시 시작 항목 없음을 확인한다.
- [ ] TRAY-002: 두 번째 실행이 종료 코드 0으로 끝나고 아이콘이 하나인지 확인한다.
- [ ] TRAY-003: 각 서비스 상태와 종료 코드, `/readyz` 200·503·연결 실패의 표시 및 메뉴 켜짐을 확인한다.
- [ ] TRAY-004: `시작` 뒤 시작 중→실행 중을 확인한다.
- [ ] TRAY-005: 활성 세션이 있을 때 `중지` 뒤 유예와 종료 로그를 확인한다.
- [ ] TRAY-006: 운영자가 아닌 사용자의 시작·중지 거절과 다음 동작 때 실패 줄이 사라짐을 확인한다. 조회 거절도 별도로 확인한다.
- [ ] TRAY-007: 서비스 없음의 비활성 메뉴, 설치·해제 반영 시간, 트레이가 해제를 늦추지 않음을 확인한다.
- [ ] TRAY-008: IPv4·IPv6, wildcard·루프백, HTTP·HTTPS 조합에서 기본 브라우저로 여는 URL을 확인한다.
- [ ] TRAY-009: 설정 읽기 거절·파일 없음·설정 오류의 표시와 복구 뒤 다음 갱신을 확인한다.
- [ ] TRAY-010: 설정·로그 파일 선택과 로그 파일이 없을 때 디렉터리 열기를 확인한다.
- [ ] TRAY-011: `로그인할 때 시작`의 기본 꺼짐·켜기·다시 로그인·끄기·다른 경로 덮어쓰기를 확인한다.
- [ ] TRAY-012: 한국어·영어·일본어 표시 언어에서 로그인한 뒤 메뉴 언어를 확인한다.
- [ ] TRAY-013: `트레이 끝내기` 뒤 서비스와 Run 값이 그대로인지 확인한다. 네트워크 요청과 파일 열기 범위도 관찰한다.

## 9. 생성 파일과 정리

이 문서의 절차가 `.work/` 밖에 만드는 파일이다. `.work/` 안의 부수 파일은 [루트 지침](../../AGENTS.md#부수-파일-위치)을 따른다. 정리 순서는 [6절](#6-롤백-또는-복구)에 있다.

| 위치 | 만드는 명령·도구 | 정리 방법 | `.work/` 밖에 두는 이유 |
| --- | --- | --- | --- |
| `target/`. `CARGO_TARGET_DIR`을 정했으면 그 위치다. `cargo xtask package`의 `target/package/`와 `cargo xtask help-site`의 `target/help-site/`도 여기에 있다. | `cargo build`, `cargo test`, `cargo xtask` | 저장소 루트에서 `cargo clean`을 실행한다. 지우기 전에 `cargo clean --dry-run`으로 크기를 확인할 수 있다. | Cargo가 위치를 정한다. |
| Cargo 홈의 `registry/` | Cargo가 의존성을 내려받는다. | `cargo clean`은 지우지 않는다. 디스크가 모자랄 때만 직접 지운다. 다음 빌드에서 다시 내려받는다. | Cargo가 위치를 정한다. |
