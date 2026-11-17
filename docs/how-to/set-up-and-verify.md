# 개발 환경을 준비하고 변경을 검증하는 방법

Windows 11 x64 또는 Ubuntu 24.04 LTS x64에서 고정된 Rust 툴체인으로 저장소를 빌드하고, 변경 범위에 맞는 검증 명령과 전체 완료 검증을 실행한다. 마지막 절차의 명령이 모두 종료 코드 0으로 끝나면 변경을 완료로 보고할 수 있다.

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 이 저장소의 코드를 바꾸는 개발자와 코딩 에이전트 |
| 검증 상태 | Draft — 1·3·4·6단계는 Ubuntu와 Windows에서 실행해 확인했다. 2단계(rustup 설치)는 두 OS 모두 이미 설치되어 있어 실행하지 않았다([4절](#4-검증)). |
| 검증 환경·버전 | Ubuntu 24.04.4 LTS x86_64(WSL2), rustup 1.29.1, Rust 1.99.1, cc 13.3.0, git 2.43.0, cargo-deny 0.20.2. Windows 11 Pro 10.0.26200 x64, rustup 1.29.1, Rust 1.99.1, Visual Studio Build Tools 2022 17.14.37806.7, git 2.55.0.windows.3, cargo-deny 0.20.2, NASM 미설치 |
| 최종 검증일 | 2026-10-20 |
| 관련 기준 문서 | [Rust 코드 컨벤션](../conventions/rust.md), [테스트 컨벤션](../conventions/testing.md), [코드 지도](../architectures/code-map.md), [의존성 라이선스 검사](check-dependency-licenses.md), [PRD 확정 기준](../requirements/PRD.md#확정-기준)의 지원 OS |

## 1. 대상과 영향

- 대상: 개발용 컴퓨터 한 대와 이 저장소의 작업 트리
- 예상 사용자 영향: 없다. 사용자 홈에 Rust 툴체인을 설치하고 Cargo 대상 디렉터리에 빌드 결과를 만든다. Windows에서는 Visual Studio Build Tools를 시스템에 설치한다.
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
- [ ] Chromium(Chrome 또는 Edge)이 설치되어 있다. Windows와 Ubuntu 모두 필수이며, 브라우저 테스트에 쓴다(사용자 결정 2026-10-22(Q35)). 실행 파일 경로는 환경 변수 `DASHBOARD_BROWSER`로 준다(이 문서의 기본값). Ubuntu에 없으면 Google Chrome 안정판 .deb를 설치한다(사용자 결정 2026-10-22(Q41)).

  영향: 시스템에 `google-chrome-stable` 패키지와 Google 패키지 저장소 설정(`/etc/apt/sources.list.d/google-chrome.sources`)을 더한다. `sudo apt remove google-chrome-stable`로 패키지를 지울 수 있다.

  ```shell
  wget https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb
  sudo apt install -y ./google-chrome-stable_current_amd64.deb
  google-chrome --version
  ```

  기대 결과: 마지막 명령이 `Google Chrome 156.0.8078.47`처럼 버전을 출력한다.
- [ ] mdBook 0.5.4가 설치되어 있다. Windows와 Ubuntu 모두 필수이며, 도움말 사이트 빌드(`cargo xtask help-site`)와 도움말 브라우저 시험(`tests/help_site.rs`)에 쓴다(사용자 결정 2026-11-11(Q223)). 실행 파일 경로는 환경 변수 `MDBOOK`으로 주거나 `PATH`에 둔다. 내려받기와 SHA-256 확인은 [도움말 게시 how-to 2.1절](publish-help.md#21-mdbook-054-설치)을 따른다.
- [ ] Windows: 설치 승인(UAC)을 할 수 있는 사용자가 컴퓨터 앞에 있다.
- [ ] WSL에서 Windows 드라이브(`/mnt/<드라이브>/`)의 작업 트리를 쓰면 `CARGO_TARGET_DIR`을 Linux 홈 아래로 정하는 것을 권장한다. 예: `export CARGO_TARGET_DIR="$HOME/vnc-relay-target"`. Windows 쪽 Cargo와 빌드 결과가 섞이지 않는다. 검증 기록은 이 설정으로 실행했다.

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

   Windows에서는 aws-lc-sys(HMAC·난수·TLS 암호 모듈의 C 라이브러리) 빌드에 NASM도 필요하다. NASM을 설치하지 않으면 빌드하는 PowerShell에서 `$env:AWS_LC_SYS_PREBUILT_NASM = "1"`을 먼저 실행한다. crate에 든 미리 만든 NASM 오브젝트를 쓴다. 검증 환경은 NASM 없이 이 방법으로 빌드했다. [nasm.us](https://nasm.us/)에서 NASM을 설치하고 `PATH`에 더하는 방법도 있지만 실행해 확인하지 않았다.

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

   Windows에서 명령을 찾지 못하면 `%USERPROFILE%\.cargo\bin`이 `PATH`에 없는 것이다. 검증 환경도 그랬다. 그 PowerShell에서 `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`를 먼저 실행한다. 사용자 `PATH`에 영구로 더할 수도 있다.

#### WSL 끊긴 연결 감지 시험 준비

Linux 전용 `tests/relay_dead_peer_linux.rs`는 사용자·네트워크 네임스페이스 안에서 루프백 패킷을 `nft`로 버린다. 호스트의 방화벽과 네트워크 설정은 바꾸지 않는다. Windows에서는 기존 부분 시험을 유지한다(사용자 결정 2026-11-02(Q86)).

1. WSL Ubuntu에서 사용자가 `nftables`를 설치한다(사용자 결정 2026-11-02(Q93)).

   ```shell
   sudo apt-get install -y nftables
   ```

   영향: WSL 배포판에 `nftables` 패키지를 설치한다. 시험 자체에는 `sudo`를 쓰지 않는다.

   WSL의 `sudo` 암호를 입력할 수 없으면 Windows PowerShell에서 root로 설치할 수 있다. 검증 환경은 사용자 요청(2026-11-02)으로 이 방법을 써서 `nftables v1.0.9`를 설치했다.

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

   기대 결과: 출력 없이 종료 코드 0으로 끝난다. 시험은 `unshare --user --map-root-user --net`으로 자신을 다시 실행한다. 필수 도구나 사용자 네임스페이스가 없으면 누락 조건을 밝히고 실패한다. 조용히 건너뛰지 않는다(사용자 결정 2026-11-02(Q93)).

4. 저장소 루트에서 Linux 전용 시험을 실행한다.

   ```shell
   cargo test --test relay_dead_peer_linux -- --nocapture
   ```

   기대 결과: 대기 Server와 활성 세션의 Viewer·Server·Mode I 대상에 실제 DROP을 적용한 시험 2건이 통과한다. 한 번 실행에 약 47초 걸린다. K는 설정 최솟값인 11초다. Linux 타이머·스케줄링 오차는 최대 5초로 제한한다(DROP 직전 시각 기준 최대 16초). 이 오차를 K 설정값에 더하지는 않는다.

   시험은 DROP 직전 시각과 정리 사건 줄을 읽은 시각을 단조 시계(`Instant`)로 비교한다. 로그 `ts`는 벽시계라서 쓰지 않는다. WSL2는 실행 중에 벽시계를 몇 초씩 앞으로 보정한다. 검증 환경에서는 40초 동안 +2.933초 보정이 한 번 일어났다. 로그 `ts`로 재면 실제로 11초 안에 끝난 정리가 14초 이상으로 계산된다. 로그를 읽는 지연(10ms 주기 확인)은 측정값을 늘리기만 하므로 상한 판정을 느슨하게 하지 않는다. 마지막 활동 이후 9초 이상이라는 하한 판정은 그 지연만큼 느슨해진다.

#### noVNC 상호운용 시험 준비

`tests/websocket_browser.rs`의 WS-025 시험은 고정한 noVNC 1.7.0 압축 파일이 있어야 실행할 수 있다. 그래서 [테스트 컨벤션](../conventions/testing.md)에 따라 `#[ignore]`를 붙였고, 6단계의 기본 실행에서는 `미실행`으로 보고된다. 같은 파일의 WS-020 시험은 noVNC가 필요 없으며 6단계에서 함께 실행된다. noVNC는 시험 중에만 임시 디렉터리에 풀고, 저장소와 배포물에 넣지 않는다.

1. 고정한 압축 파일 `novnc-v1.7.0.zip`을 준비한다. 받은 주소와 SHA-256은 [호환성 검증 기록 2.4절](../contracts/repeater-compat-record.md)에 있다. 검증 환경은 `D:\vr-lab\installers\novnc-v1.7.0.zip`(WSL에서는 `/mnt/d/vr-lab/installers/novnc-v1.7.0.zip`)을 썼다.
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

4. 실행 결과를 [WebSocket 중계 계약 5절](../contracts/websocket-relay-v2.md#5-예제와-계약-테스트)의 실행 기록에 남긴다. 대상 커밋, OS, 명령, 결과를 적는다.

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
   | `docs/help/`, `web/help/`, 도움말 도구(`xtask/src/help_site.rs`·`help_site/`·`help_serve.rs`·`static_server.rs`), `.github/workflows/help.yml` | `cargo xtask help-site`, `cargo test --test help_site`, `web/help/` CSS를 바꾸면 `cargo xtask tokens --check`. 워크플로를 바꾸면 push 뒤 `help` 실행 결과를 확인한다([도움말 게시 how-to](publish-help.md)) | `help-site: 통과`, 시험 18개 통과, `help` 결론 `success` |
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

   기대 결과: 여섯 명령 모두 종료 코드 0. `cargo xtask scenarios`가 `cargo test --workspace --no-fail-fast`를 함께 실행하므로 따로 실행하지 않는다. 대시보드 브라우저 테스트도 이때 실행되며 Chromium이 없으면 실패한다(2절). 도움말 브라우저 시험도 이때 실행되며 mdBook 0.5.4가 없으면 실패한다(2절). 출력 마지막에 `판정: 통과`, 라이선스 검사 마지막 줄에 `licenses ok`가 나온다. 단계를 마칠 때(작업 5.9)는 보고서에 남은 `부분`·`미실행`·`미연결` 시나리오를 [4.1절 판정표](#41-작업-59-판정표)에서 판정한다(사용자 결정 2026-11-06(Q132)). `cargo xtask scenarios --require-all`은 완료 명령이 아니다. 남은 시나리오 수를 세는 진단용이며, 그런 시나리오가 하나라도 있으면 판정 실패와 종료 코드 1로 끝난다([테스트 컨벤션 5절](../conventions/testing.md#5-검증-명령과-통과-기준)).

7. 결과를 기록한다. 시나리오 보고서는 Cargo 대상 디렉터리의 `scenario-report.md`다. 완료 보고에는 대상 커밋, OS, 명령, 판정을 적는다([검증 기록](../AGENTS.md#검증-기록)).

   WSL에서 Windows 작업 트리를 쓰면 줄 끝(CRLF·LF) 차이 때문에 보고서의 대상 커밋에 `+ 미커밋 변경`이 붙을 수 있다. 이때는 Windows의 `git status`로 실제 변경 여부를 확인한다.

## 4. 검증

[검증 기록](../AGENTS.md#검증-기록)에 따라 실제 결과와 성공 조건을 대조한다.

| 관찰 대상 | 성공 조건 | 확인 명령·방법 | 실제 결과 | 실행 증거 위치 |
| --- | --- | --- | --- | --- |
| Ubuntu 링커·툴체인 | 3단계 기대 결과 | 1단계 `cc --version`, 3단계 | 통과(2026-10-20). `cc 13.3.0`, `rustc 1.99.1 (58cf8f9c3 2026-10-13)` | 아래 실행 기록 1 |
| Ubuntu 빌드 | `vnc-relay 0.1.0` | 4단계(실행파일 직접 실행) | 통과(2026-10-20) | 아래 실행 기록 1 |
| Ubuntu 전체 완료 검증 | 다섯 명령 종료 코드 0 | 6단계 | 통과(2026-10-20). 레이어 검사 통과(패키지 2개), 시나리오 148건 모두 `미연결`·오류 0건·판정 통과, 단위 테스트 12건 통과, `licenses ok` | 아래 실행 기록 1 |
| Ubuntu 도구 설치 | rustup 설치 | 2단계 | 미실행. 검증 환경에는 rustup이 이미 설치되어 있었다. 영향: 설치 명령 자체는 이 문서에서 확인하지 않았다. | — |
| Windows 링커·툴체인 | 1단계 설치 성공, 3단계 기대 결과 | 1단계, 3단계 | 통과(2026-10-20). Build Tools 설치는 UAC 승인 뒤 성공했다. `vswhere`가 VCTools 작업 부하가 있는 Build Tools 2022 17.14.37806.7을 찾았다. `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, 활성 툴체인 `1.99.1-x86_64-pc-windows-msvc` | 아래 실행 기록 3 |
| Windows 빌드 | `vnc-relay 0.1.0` | 4단계(`AWS_LC_SYS_PREBUILT_NASM=1`, 실행파일 직접 실행) | 통과(2026-10-20). 처음 빌드는 1분 걸렸다. | 아래 실행 기록 3 |
| Windows 전체 완료 검증 | 다섯 명령 종료 코드 0 | 6단계 | 통과(2026-10-20). 처음 실행에서 clippy가 Windows에서만 나는 경고 2건으로 실패했고, 고친 뒤 다섯 명령 모두 통과했다. 시나리오 148건: 통과 3, 부분 12, 미실행 1, 미연결 132, 오류 0, 판정 통과. `licenses ok` | 아래 실행 기록 3 |
| Windows 도구 설치 | rustup 설치 | 2단계 | 미실행. 검증 환경에는 rustup이 이미 설치되어 있었다. `%USERPROFILE%\.cargo\bin`은 사용자 `PATH`에 없었다. | 아래 실행 기록 2 |
| Ubuntu 브라우저 준비와 브라우저 테스트 | `google-chrome --version`이 버전 출력, 6단계 종료 코드 0 | 2절 Chrome 설치, 6단계 | 통과(2026-10-22). Google Chrome 156.0.8078.47. 설치 전에는 대시보드 브라우저 테스트 2건이 `Chromium is required for the dashboard browser tests`로 실패했다 | 아래 실행 기록 4 |

실행 기록 1: 커밋 `b7d23ee`에서 실행했다. Windows 쪽 `git status`는 변경 없음이었다. 환경은 Ubuntu 24.04.4 LTS x86_64(WSL2), rustup 1.29.1, cargo 1.99.1, cargo-deny 0.20.2, `CARGO_TARGET_DIR=$HOME/vnc-relay-target`이다. 3·4·6단계의 명령은 모두 종료 코드 0이었다.

실행 기록 2: 2026-10-20 Windows 11 x64에서 1단계 winget 명령을 처음 실행했다. UAC를 승인하지 않아 종료 코드는 1602였다. rustup은 `%USERPROFILE%\.cargo\bin`에 설치되어 있지만 사용자 PATH에 없다. 이 상태의 `cargo build`는 `` `link.exe` returned an unexpected error ``로 실패했다.

실행 기록 3: 2026-10-20 같은 Windows 11 Pro 10.0.26200 x64에서 UAC를 승인해 1단계 설치를 마쳤다. 커밋 `e1cfc4f`에 Windows 경고 수정(`crates/adapters/src/net.rs` 테스트 모듈을 Linux 전용으로, `tests/serve.rs`의 `lifecycle_events`를 Unix 전용으로)을 더한 작업 트리에서 실행했다. PowerShell에서 `PATH`에 `%USERPROFILE%\.cargo\bin`을 더하고 `AWS_LC_SYS_PREBUILT_NASM=1`을 정했다. Cargo 대상 디렉터리는 저장소의 `target\`이다. 3·4·6단계의 명령은 모두 종료 코드 0이었다. 수정 전 clippy 오류는 `unused import: super::*`(`crates\adapters\src\net.rs`)와 `function lifecycle_events is never used`(`tests\serve.rs`)였다. 같은 작업 트리에서 Ubuntu 6단계도 다시 통과했다. 테스트별 결과는 [운영 정보 계약 실행 기록](../contracts/ops-info-v2.md#계약-테스트-실행-기록)에 있다.

실행 기록 4: 2026-10-22 Ubuntu 24.04.4 LTS x86_64(WSL2, 커널 6.6.87.2-microsoft-standard-WSL2)에서 Chrome을 설치했다. `sudo`가 암호를 요구해 2절의 `sudo apt install` 명령은 실행하지 않았다. 대신 Windows에서 root 셸로 같은 .deb를 설치했다: `wsl -d Ubuntu -u root -- bash -lc 'cd /tmp && wget -q https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb && DEBIAN_FRONTEND=noninteractive apt-get install -y -q ./google-chrome-stable_current_amd64.deb'`. 받은 .deb 파일은 설치 뒤 지웠다. 설치 뒤 `command -v google-chrome`은 `/usr/bin/google-chrome`, `dpkg-query -W google-chrome-stable`은 `156.0.8078.47-1`이었다. 커밋 `df3b5b2`(작업 트리 변경 없음)에서 6단계를 다시 실행했다. 여섯 명령이 모두 종료 코드 0이었고 브라우저 테스트 2건이 통과했다. 시나리오 150건: 통과 16, 부분 23, 실패 0, 미실행 0, 미연결 111, 오류 0건. 같은 커밋의 Windows 6단계도 통과했다(통과 15, 부분 23, 미실행 1 = OPS-007, 미연결 111).

### 4.1 작업 5.9 판정표

결론: 두 OS 보고서의 `오류`와 `실패`는 0건이다. 보고서에 `통과`가 아닌 상태로 남은 17개 시나리오 모두 아래 근거로 통과로 판정한다. 17개는 두 OS 보고서를 합친 수다. OS별로는 Windows 보고서에 16개, Ubuntu 보고서에 13개가 있다. OPS-021은 사용자 결정 2026-11-07(Q97)에 따른 루트 LICENSE 작성·검토까지 마쳤다. 전체 153개 시나리오를 통과로 판정했고, AC-01~37을 모두 충족했다. 작업 5.9는 완료다.

이 절의 위치와 형식은 이 문서의 판단, 사용자 결정 없음이다. 판정 규칙은 [TODO 5.9](https://github.com/kayden-kim/vnc-relay/blob/556fab23d15e77125ca2ad5c575089857b312049/docs/TODO.md#5-운영배포-검증)의 완료 조건, [테스트 컨벤션 5절](../conventions/testing.md#5-검증-명령과-통과-기준)(사용자 결정 2026-11-06(Q132)), 사용자 결정 2026-11-04(Q99)를 따른다. 보고서 도구는 바꾸지 않았다.

기준 보고서는 커밋 `ee0314f`의 두 OS 전체 게이트(2026-11-07)다. 커밋 `0c0ba5e`는 계약 시험 두 개(OPS-003·OPS-036 부분 표식)를, 커밋 `84f4086`은 WS 요청 머리 상한 경계 시험(WS-012 부분 표식)을 더했다. 두 커밋의 두 OS 게이트도 같은 상태 수로 통과했고 새 시험이 통과했다. 그래서 아래 판정은 바뀌지 않는다. 마지막 확인은 루트 LICENSE를 더한 뒤의 커밋 `eb5f3ee`다. 바로 앞 커밋 `d7608e3`의 Windows 게이트는 코드 지도에 넣은 `LICENSE` 행 때문에 `tests/code_map.rs`가 실패했다(`symbol `LICENSE` has no preceding path`). 코드가 아닌 그 행을 지운 `eb5f3ee`에서 두 OS 게이트가 통과했다.

커밋 `5b4aed3`은 [작업 6](https://github.com/kayden-kim/vnc-relay/blob/556fab23d15e77125ca2ad5c575089857b312049/docs/TODO.md#6-설정-보기와-동적-설정-편집)의 기준 문서와 시안을 더했다. 코드는 주석과 `crates/app/clippy.toml`의 설명 문자열만 바꿨다. 이 커밋의 두 OS 게이트도 통과했다. 시나리오는 215건으로 늘었고, 새 62건은 구현 전이라 `미연결`이다. 기존 153건의 상태 수는 위와 같으므로 아래 판정은 바뀌지 않는다. 작업 6의 판정은 [TODO 6.9](https://github.com/kayden-kim/vnc-relay/blob/556fab23d15e77125ca2ad5c575089857b312049/docs/TODO.md#6-설정-보기와-동적-설정-편집)에서 따로 한다. 후속(2026-11-08, 작업 6.9 검토): 이 커밋은 `cargo xtask tokens`가 만드는 `web/dashboard/app/tokens.css`·`web/dashboard/login/tokens.css`에도 `--type-input-data-*` 변수 5개씩을 더했다. 이 커밋에는 그 변수를 쓰는 CSS가 없어 화면 동작은 바뀌지 않았다. Rust 변경은 주석뿐이다. 작업 6의 판정은 [4.2절](#42-작업-69-판정표)에 있다.

| 확인 대상 | 기준 | 환경 | 결과 | 증거 |
| --- | --- | --- | --- | --- |
| Windows 시나리오 보고서 | `오류`·`실패` 0건 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | 통과. `ALL PASSED (all)`. 153건: 통과 137, 부분 11, 실패 0, 미실행 3, 미연결 2, 오류 0 | `target/scenario-report.md`(`ee0314f`, 2026-11-07T09:31:01Z) |
| Ubuntu 시나리오 보고서 | `오류`·`실패` 0건 | Ubuntu 24.04.4 LTS x64(WSL2), Rust 1.99.1 | 통과. `ALL PASSED`. 153건: 통과 140, 부분 7, 실패 0, 미실행 4, 미연결 2, 오류 0. 머리글의 `+ 미커밋 변경`은 줄 끝 차이뿐이다(`git -c core.autocrlf=true diff --exit-code` 종료 코드 0) | `~/vnc-relay-target/scenario-report.md`(`ee0314f`, 2026-11-07T09:48:43Z) |
| Windows 재실행 | 같음 | 같음 | 통과. `ALL PASSED (all)`. 상태 수는 위와 같다. `account_file_content_changes_apply_only_after_restart`·`management_legacy_tls_rejections_have_exact_json_fields_and_keep_tls12_available` 통과 | `target/scenario-report.md`(`0c0ba5e`, 2026-11-07T12:46:13Z) |
| Ubuntu 재실행 | 같음 | 같음 | 통과. `ALL PASSED`. 상태 수와 두 시험 결과는 위와 같다. `+ 미커밋 변경`은 줄 끝 차이뿐이다(문서 밖 `git -c core.autocrlf=true diff --exit-code` 종료 코드 0) | `~/vnc-relay-target/scenario-report.md`(`0c0ba5e`, 2026-11-07T13:03:45Z) |
| Windows 재실행 | 같음 | 같음 | 통과. `ALL PASSED (all)`. 상태 수는 위와 같다. `request_head_accepts_16384_bytes_and_refuses_16385_bytes` 통과 | `target/scenario-report.md`(`84f4086`, 2026-11-07T13:56:07Z) |
| Ubuntu 재실행 | 같음 | 같음 | 통과. `ALL PASSED`. 상태 수와 새 시험 결과는 위와 같다. `+ 미커밋 변경`은 줄 끝 차이뿐이다(`git -c core.autocrlf=true diff --exit-code` 종료 코드 0) | `~/vnc-relay-target/scenario-report.md`(`84f4086`, 2026-11-07T14:13:58Z) |
| Windows 최종 | 같음 | 같음 | 통과. `ALL PASSED (all)`. 153건: 통과 137, 부분 11, 실패 0, 미실행 3, 미연결 2, 오류 0 | `target/scenario-report.md`(`eb5f3ee`, 2026-11-07T15:13:17Z) |
| Ubuntu 최종 | 같음 | 같음 | 통과. `ALL PASSED`. 153건: 통과 140, 부분 7, 실패 0, 미실행 4, 미연결 2, 오류 0. `+ 미커밋 변경`은 줄 끝 차이뿐이다(`git -c core.autocrlf=true diff --exit-code` 종료 코드 0) | `~/vnc-relay-target/scenario-report.md`(`eb5f3ee`, 2026-11-07T15:30:57Z) |
| Windows 작업 6 문서 | 같음 | 같음 | 통과. `ALL PASSED (all)`. 215건: 통과 137, 부분 11, 실패 0, 미실행 3, 미연결 64, 오류 0. 머리글의 `+ 미커밋 변경`은 게이트 실행 중 편집한 채택 기록 문서다. 시나리오 ID와 시험 코드는 바뀌지 않았다 | `target/scenario-report.md`(`5b4aed3`, 2026-11-08T05:44:12Z) |
| Ubuntu 작업 6 문서 | 같음 | 같음 | 통과. `ALL PASSED`. 215건: 통과 140, 부분 7, 실패 0, 미실행 4, 미연결 64, 오류 0. `+ 미커밋 변경`은 Windows 행과 같다 | `~/vnc-relay-target/scenario-report.md`(`5b4aed3`, 2026-11-08T05:46:11Z) |

보고서에 `통과`가 아닌 상태로 남은 시나리오의 판정은 다음과 같다. 보고서 열은 Windows / Ubuntu 순서다.

| 시나리오 | 보고서 | 판정 | 근거 |
| --- | --- | --- | --- |
| DASH-015 | 부분 / 부분 | 통과 | 네 엔진의 실제 키 입력·로그인 Tab 순서·긴 값 확인([대시보드 검증 4.26절](verify-dashboard-frontend.md#426-작업-412-완료-판정-q144q146-2026-11-07), 사용자 결정 2026-11-07(Q145)). 저장소 시험 `session_cancel_sends_no_request_and_restores_focus_in_browser`는 부분 연결이다 |
| DASH-023 | 미연결 / 미연결 | 통과 | Chrome·Edge·Firefox 마우스 흐름 18회(Q146), Safari는 WebKit 대체 6회(Q135)([같은 절](verify-dashboard-frontend.md#426-작업-412-완료-판정-q144q146-2026-11-07)). Firefox의 페이지 오류는 모으지 않았다 |
| OPS-017 | 미실행 / 미실행 | 통과 | 무시된 시험의 실행 기록. 커밋 `509ca52`의 두 OS 정식 측정에서 세 전송 모두 통과했다(Q148 기준)([성능 측정 4절](measure-performance.md#4-검증-기록)) |
| OPS-018 | 미실행 / 미실행 | 통과 | 같은 정식 측정에서 두 OS·세 전송 모두 통과했다([성능 측정 4절](measure-performance.md#4-검증-기록)) |
| OPS-020 | 미연결 / 미연결 | 통과 | Windows VNC Relay와 실제 UltraVNC 1.8.3.0·noVNC 1.7.0의 아홉 조합 통과(Q128·Q130·Q131)([호환성 기록 6.3절](../contracts/repeater-compat-record.md#63-조합별-결과)). Ubuntu는 자동 시험을 증거로 인정했다(Q129)([6.6절](../contracts/repeater-compat-record.md#66-ubuntu-2404-증거)). 실제 UltraVNC와 Ubuntu VNC Relay의 조합은 미실행이다 |
| OPS-021 | 부분 / 부분 | 통과 | 기존 부분 시험 `tests/dependency_notices.rs`가 위 두 OS 보고서에서 통과했다. 출처·의존성 고지·대시보드 JS 의존성 검토([라이선스 검사 4.1절](check-dependency-licenses.md#41-출처-검토-기록-2026-11-04))와 루트 MIT LICENSE 작성·수동 검토([4.2절](check-dependency-licenses.md#42-프로젝트-license-검토-기록2026-11-07))를 합쳐 판정했다. 실제 저작권자·연도는 사용자 결정 2026-11-07(Q97)과 일치한다 |
| OPS-023 | 부분 / 통과 | 통과 | Q99. Ubuntu에서 비부분 시험 `counts_connecting_and_connected_target_sockets_but_not_management_tls`가 통과했다. Windows는 큐가 찬 `listen()`이 WSAECONNREFUSED로 거절하므로 연결 중 소켓을 만들 수 없다(`tests/relay_target_scenarios.rs` 927~929행). Windows 부분 시험 `target_budget_denial_preserves_sessions_and_excludes_management_tls`는 통과했다([중계 수명 설계 7절 현재 시험](../architectures/relay-session.md#현재-시험)) |
| OPS-031 | 부분 / 부분 | 통과 | 두 OS release 묶음의 수동 기록과 자동 시험을 함께 검토했다(Q107)([운영 how-to 4절](operate-relay.md#4-검증)). 2026-11-05 묶음의 LICENSE 누락은 당시 기록으로 보존한다. 루트 LICENSE 작성·검토는 마쳤고(사용자 결정 2026-11-07(Q97)), 앞으로 만드는 묶음에는 포함한다. 기존 묶음을 다시 검증한 것은 아니다 |
| OPS-040 | 부분 / 부분 | 통과 | 모든 조건을 부분 시험들이 나누어 확인한다. IPv6 경계는 단위 시험으로 인정했다(Q58)([파일 감지 설계](../architectures/file-watch.md#사용자-결정-반영-2026-10-23)) |
| OPS-044 | 통과 / 미실행 | 통과 | Windows 전용 시나리오다. Windows에서 비부분 시험 `console_close_forces_cleanup`이 통과했다([작업 5.3 완료 대조](../architectures/relay-session.md#작업-53-완료-대조2026-11-04)) |
| RLY-006 | 부분 / 부분 | 통과 | Windows VNC Relay와 실제 UltraVNC의 독립 2회 수동 통과, 소유자 캡처 검토(Q116)([수동 확인 4.1](verify-ultravnc-manually.md#41-2026-11-05-실행-기록)·[4.2](verify-ultravnc-manually.md#42-2026-11-06-실행-기록)·[4.3절](verify-ultravnc-manually.md#43-자동-증거-대조-32단계)). Ubuntu VNC Relay의 실제 UltraVNC 확인은 미실행이다. Ubuntu 쪽은 Q129처럼 자동 시험을 증거로 본다. 이 확장은 이 문서의 판단, 사용자 결정 없음이다 |
| RLY-009 | 부분 / 통과 | 통과 | Q99. Ubuntu에서 비부분 시험이 통과했다. Windows 이유는 OPS-023과 같다(C 시간 초과에 연결 중 소켓이 필요하다). Windows 부분 시험 `connection_refusal_is_reported_and_releases_both_socket_places`는 통과했다 |
| RLY-010 | 부분 / 부분 | 통과 | RLY-006과 같은 근거(Mode II, Server 먼저 독립 2회) |
| RLY-011 | 부분 / 부분 | 통과 | RLY-006과 같은 근거(Mode II, Viewer 먼저 독립 2회) |
| RLY-025 | 부분 / 통과 | 통과 | Q86·Q99. Ubuntu의 netns·nftables 비부분 시험이 통과했다. Windows에는 응답 없이 끊긴 네트워크를 root 없이 만들 수단이 없다([중계 수명 설계 bf27d30 기록](../architectures/relay-session.md#전체-게이트-실행-기록-커밋-bf27d30)) |
| RLY-026 | 부분 / 통과 | 통과 | RLY-025와 같은 근거 |
| WS-025 | 미실행 / 미실행 | 통과 | 무시된 시험의 실행 기록. 커밋 `d4c5678`에서 두 OS 모두 `--include-ignored`로 통과했다([WebSocket 계약 실행 기록](../contracts/websocket-relay-v2.md#계약-테스트-실행-기록)). 실제 VM의 noVNC 확인도 통과했다([호환성 기록 6.3절](../contracts/repeater-compat-record.md#63-조합별-결과)) |

PRD 10절의 AC-01~37을 같은 기준으로 대조했다. 각 AC 행이 링크한 시나리오 ID를 두 보고서와 위 표에 맞춰 보았다. 37개 모두 연결된 시나리오가 두 보고서에서 통과이거나 위 표에서 통과다. AC-12는 OPS-021의 부분 시험과 출처·LICENSE 검토 기록을 합쳐 충족으로 판정했다.

남은 일은 없다. Q97의 저작권자·적용 연도 확정과 LICENSE 작성·검토로 OPS-021·AC-12를 통과로 판정해 작업 5.9를 닫았다. 스펙·계약의 상태 문장은 사용자 결정 2026-11-07(Q150~Q152)에 따라 갱신했고, 계약 시험 표에 남았던 규칙은 새 시험 두 개와 Q152로 닫았다([TODO 5.9](https://github.com/kayden-kim/vnc-relay/blob/556fab23d15e77125ca2ad5c575089857b312049/docs/TODO.md#5-운영배포-검증)).

### 4.2 작업 6.9 판정표

결론: 커밋 `eae442f`의 두 OS 최종 게이트에서 `오류`와 `실패`는 0건이다. 새 범위 62개 시나리오 가운데 61개는 두 OS 보고서에서 `통과`다. OPS-050은 두 OS에서 `부분`이며 아래 조항별 시험으로 통과로 판정한다. 기존 153개 가운데 `통과`가 아닌 상태로 남은 17개는 4.1절과 같은 근거로 통과로 판정한다. 이 17개는 OS별로 Windows 보고서에 16개, Ubuntu 보고서에 13개다. OPS-050을 더하면 두 최종 보고서의 `통과`가 아닌 시나리오는 Windows 17개, Ubuntu 14개다. 확대된 AC-13, 새 AC-38~43, 기존 AC-01~37을 모두 충족했다. 작업 6.9는 완료다.

이 절의 위치와 형식은 이 문서의 판단, 사용자 결정 없음이다. 판정 규칙은 [TODO 6.9](https://github.com/kayden-kim/vnc-relay/blob/556fab23d15e77125ca2ad5c575089857b312049/docs/TODO.md#6-설정-보기와-동적-설정-편집)의 완료 조건, [테스트 컨벤션 5절](../conventions/testing.md#5-검증-명령과-통과-기준)(사용자 결정 2026-11-06(Q132)), 사용자 결정 2026-11-04(Q99)를 따른다. TODO 6.9에 따라 새 시나리오는 4.1절의 판정으로 대체하지 않는다. 4.1절의 근거는 기존 17개 시나리오에만 다시 쓴다. 보고서 도구는 바꾸지 않았다.

구현 커밋 `7ce077f`의 첫 게이트는 두 OS에서 동시에 실행했고, 둘 다 실패했다. 원인은 시험 두 개의 결함이었다. 제품 코드는 바꾸지 않았다.

1. 유예 중 설정 편집 시험 `config_edit_works_during_grace_without_changing_the_started_grace_deadline`이 두 OS에서 실패했다. 이 시험은 종료가 시작된 뒤에도 TCP Server 포트에 접속할 수 있다고 가정했다. 제품은 종료가 시작되면 중계 리스너를 닫으므로 접속이 거절된다(Windows os error 10061, Ubuntu os error 111). [운영 스펙 3.3절](../specs/operations.md#33-파일-변경-자동-감지와-부분-재로드) 항목 7은 새 중계 수락을 다시 시작하지 않는 것만 요구한다. 기존 `stop_unfinished_work` 시험처럼 접속 거절과 닫힌 연결을 모두 수락 중지로 인정하도록 고쳤다(커밋 `0bc9a47`, `tests/common/shutdown.rs`의 `assert_relay_accept_stopped`). Windows에서 이 시험만 따로 실행해도 4.12초 만에 실패했으므로 부하가 원인이 아니다. OPS-043·OPS-050의 실패는 이 시험의 표식에서 나왔다.
2. DASH-046 시험 `dashboard_046_config_response_loss_and_timeout_require_get_before_retry`가 Ubuntu에서 `query_timeout_secs must elapse`로 실패했다. 시험은 프록시가 요청을 다 읽은 시각부터 5초 하한을 쟀다. 브라우저 타이머는 그보다 먼저 시작한다(`web/dashboard/app/api.js`의 `exchange`). 그래서 부하가 크면 하한 검사가 거짓으로 실패할 수 있다. 실제 지연 값은 기록하지 않았으므로 이 원인은 추정이다. 저장 단추를 누르기 직전부터 재도록 고쳤다(같은 커밋, `tests/dashboard_live/settings_recovery.rs`). MGT-040의 실패는 같은 실행기의 표식에서 나왔다.

커밋 `eae442f`는 OPS-050의 수락 쪽 K 시험을 더했다(Linux 전용, 아래 OPS-050 표). Windows 진단 실행은 `0bc9a47` 트리에서 시험을 빌드했다. 실행 중에 `eae442f`를 커밋했으므로 보고서 머리글은 `eae442f`다. 두 커밋의 차이는 Linux 전용 시험뿐이다. 최종 판정은 `eae442f`에서 다시 실행한 두 OS 게이트를 기준으로 한다. 최종 두 게이트도 동시에 실행했다.

| 확인 대상 | 기준 | 환경 | 결과 | 증거 |
| --- | --- | --- | --- | --- |
| Windows 첫 실행 | `오류`·`실패` 0건 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | 실패. `FAILED (all): scenarios`. 215건: 통과 197, 부분 11, 실패 2(OPS-043·OPS-050), 미실행 3, 미연결 2, 오류 0. 원인은 위 1번이다. 머리글의 `+ 미커밋 변경`은 이 검증 기록을 위한 대시보드 검증 문서 초안이다 | `target/scenario-report.md`(`7ce077f`, 2026-11-08T08:52:36Z) |
| Ubuntu 첫 실행 | 같음 | Ubuntu 24.04.4 LTS x64(WSL2), Rust 1.99.1 | 실패. `FAILED: scenarios`. 215건: 통과 198, 부분 7, 실패 4(DASH-046·MGT-040·OPS-043·OPS-050), 미실행 4, 미연결 2, 오류 0. 원인은 위 1·2번이다. `+ 미커밋 변경`은 Windows 행과 같다 | `~/vnc-relay-target/scenario-report.md`(`7ce077f`, 2026-11-08T08:55:33Z) |
| Windows 진단 실행 | 같음 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | 통과. `ALL PASSED (all)`. 215건: 통과 198, 부분 12, 실패 0, 미실행 3, 미연결 2, 오류 0. 시험은 `0bc9a47` 트리에서 빌드했다 | `target/scenario-report.md`(`eae442f`, 2026-11-08T09:27:40Z) |
| Windows 최종 | 같음 | 같음 | 통과. `ALL PASSED (all)`. 215건: 통과 198, 부분 12, 실패 0, 미실행 3, 미연결 2, 오류 0. 시나리오별 상태는 진단 실행과 같다 | `target/scenario-report.md`(`eae442f`, 2026-11-08T09:56:59Z) |
| Ubuntu 최종 | 같음 | Ubuntu 24.04.4 LTS x64(WSL2), Rust 1.99.1 | 통과. `ALL PASSED`. 215건: 통과 201, 부분 8, 실패 0, 미실행 4, 미연결 2, 오류 0. 머리글의 `+ 미커밋 변경`은 줄 끝 차이뿐이다(`git -c core.autocrlf=true diff --exit-code` 종료 코드 0) | `~/vnc-relay-target/scenario-report.md`(`eae442f`, 2026-11-08T09:59:46Z) |

새 범위 62개 시나리오의 판정은 다음과 같다. 보고서 열은 Windows / Ubuntu 순서다.

| 시나리오 | 보고서 | 판정 | 근거 |
| --- | --- | --- | --- |
| OPS-047~049·051~059·063~064 | 통과 / 통과 | 통과 | 시나리오마다 비부분 시험이 두 OS 보고서에서 통과했다 |
| OPS-050 | 부분 / 부분 | 통과 | 부분 시험만 연결한 시나리오다. 아래 조항별 표의 시험이 모두 통과했다. Windows에서는 수락·대상 소켓의 K 값을 다시 읽어 대조하지 못한다. socket2 0.6.5의 `tcp_keepalive_time`·`tcp_keepalive_interval`·`tcp_keepalive_retries`가 Windows를 지원하지 않기 때문이다. 두 OS의 수락 쪽 K는 같은 `admit` 경로가 같은 설정 세대에서 복사한 값이다(`crates/adapters/src/net.rs`). 이 OS별 범위 판단은 Q99와 같은 방식이며 이 문서의 판단, 사용자 결정 없음이다 |
| MGT-028~045 | 통과 / 통과 | 통과 | Producer 계약 시험(`crates/adapters/tests/management.rs`, `tests/management_query.rs`)과 거래 단위 시험(`crates/app/src/management/config/tests.rs`)이 통과했다. MGT-040·042·043의 전체 표식은 DASH-046·048·042 실서버 실행기에 있다 |
| OPS-060~062 | 통과 / 통과 | 통과 | 사건 계약 시험과 실행파일 시험이 두 OS에서 통과했다 |
| DASH-028~053 | 통과 / 통과 | 통과 | `tests/dashboard_live.rs`의 실서버 실행기 26개가 통과했다([대시보드 검증 4.28절](verify-dashboard-frontend.md#428-설정-화면-실서버-시험-dash-028053-mgt-040042043-2026-11-08)). 시험 쪽 주입은 Q113·Q180 범위이며 같은 절의 출처 표를 따른다. 가짜 API 시험 26개와 시안 대조 124쌍([4.27절](verify-dashboard-frontend.md#427-설정-화면-구현과-시안-대조-작업-66-2026-11-08)), 네 엔진 16회 실행([4.29절](verify-dashboard-frontend.md#429-설정-화면-네-엔진-확인-작업-66-2026-11-08))을 함께 보았다 |

OPS-050 조항별 시험은 다음과 같다. 조항은 [설정 계약 1.8절](../contracts/config-v4.md#18-재로드-구분)의 적용 시점이다.

| 조항 | 확인한 시험 | OS |
| --- | --- | --- |
| B: 수락 때 값 유지, 새 수락부터 새 값 | `relay::handshake::tests::exchange_keeps_accepted_b_and_started_h_after_reload`, `relay::session::tests::mode_two_directions_keep_distinct_accepted_buffer_limits`, 실행파일 시험 `tests/reload.rs`의 `runtime_groups_take_effect_only_at_their_documented_boundaries` | 두 OS |
| M·Kw: 업그레이드 때 값 | `websocket::tests::reload_separates_accept_upgrade_and_exchange_values`, 같은 실행파일 시험 | 두 OS |
| H: 초기 교환 시작 때 값 | 위 handshake 단위 시험, 같은 실행파일 시험 | 두 OS |
| U: 수락 때 값 | `admission::tests::reloaded_admin_limit_preserves_connections_and_captures_u_at_accept`, `request_deadline::tests::delayed_wrapping_keeps_the_original_accept_deadline_and_u`, 같은 실행파일 시험 | 두 OS |
| W: 등록 때 값, 기존 대기 기한 유지 | `relay::tests::reload_keeps_waiting_deadlines_and_reads_new_registration_policy` | 두 OS |
| C: 대상 연결 시도 때 값 | `relay::tests::reload_keeps_started_c_and_policy_but_changes_the_next_target_check` | 두 OS |
| E: 첫 종료 전이 때 값 | `relay::session::tests::reload_reads_e_at_first_close_and_keeps_started_close_fixed` | 두 OS |
| G: 시작한 종료 유예 유지 | `tests/windows_console.rs`·`tests/unix_shutdown.rs`의 `config_edit_works_during_grace_without_changing_the_started_grace_deadline`. 유예 중 G를 0과 60으로 편집해도 이미 시작한 6초 유예가 그대로다 | 두 OS |
| K: 새 TCP 수락·대상 연결부터 새 값 | `net::accept_tests::reloaded_keepalive_applies_to_new_accepts_and_keeps_accepted_sockets`(Linux 전용), `crates/adapters/tests/tcp_target.rs`의 `reloaded_keepalive_is_used_only_for_new_target_sockets` | Ubuntu는 값 대조. Windows는 대상 시험이 두 소켓의 keepalive 켜짐만 확인 |
| 일부만 적용하지 않음 | `reload::service::tests::prepared_snapshot_publishes_all_runtime_groups_once` | 두 OS |

기존 153개 가운데 `통과`가 아닌 상태로 남은 시나리오는 4.1절과 같은 17개다. 두 최종 보고서의 상태도 4.1절의 기준 보고서와 같다. 판정 근거는 4.1절의 같은 행을 따른다.

| 시나리오 | 보고서 | 판정 | 근거 |
| --- | --- | --- | --- |
| DASH-015 | 부분 / 부분 | 통과 | 4.1절과 같다 |
| DASH-023 | 미연결 / 미연결 | 통과 | 4.1절과 같다. 작업 6.6의 [대시보드 검증 4.29절](verify-dashboard-frontend.md#429-설정-화면-네-엔진-확인-작업-66-2026-11-08)은 설정 내비게이션을 더한 공통 셸에서 로그인·로그아웃을 네 엔진·두 폭으로 다시 실행했다(16회 통과) |
| OPS-017 | 미실행 / 미실행 | 통과 | 4.1절과 같다 |
| OPS-018 | 미실행 / 미실행 | 통과 | 4.1절과 같다 |
| OPS-020 | 미연결 / 미연결 | 통과 | 4.1절과 같다 |
| OPS-021 | 부분 / 부분 | 통과 | 4.1절과 같다. 작업 6의 `toml_edit` 추가는 [라이선스 검사 실행 기록 9](check-dependency-licenses.md#4-검증)에서 확인했다 |
| OPS-023 | 부분 / 통과 | 통과 | 4.1절과 같다(Q99) |
| OPS-031 | 부분 / 부분 | 통과 | 4.1절과 같다 |
| OPS-040 | 부분 / 부분 | 통과 | 4.1절과 같다 |
| OPS-044 | 통과 / 미실행 | 통과 | 4.1절과 같다(Windows 전용 시나리오) |
| RLY-006 | 부분 / 부분 | 통과 | 4.1절과 같다 |
| RLY-009 | 부분 / 통과 | 통과 | 4.1절과 같다(Q99) |
| RLY-010 | 부분 / 부분 | 통과 | 4.1절과 같다 |
| RLY-011 | 부분 / 부분 | 통과 | 4.1절과 같다 |
| RLY-025 | 부분 / 통과 | 통과 | 4.1절과 같다(Q86·Q99) |
| RLY-026 | 부분 / 통과 | 통과 | 4.1절과 같다(Q86·Q99) |
| WS-025 | 미실행 / 미실행 | 통과 | 4.1절과 같다 |

[PRD 10절](../requirements/PRD.md#10-검증-및-수용-기준)의 AC를 같은 기준으로 대조했다. 각 AC 행이 링크한 시나리오 ID를 두 최종 보고서와 위 표에 맞춰 보았다.

| AC | 연결 시나리오 | 판정 |
| --- | --- | --- |
| AC-13(확대) | OPS-003·024~028·047~059·063~064, WS-022 | 충족. OPS-050은 위 표에서 통과다 |
| AC-38 | MGT-028, DASH-028~034 | 충족 |
| AC-39 | MGT-031·039·041, OPS-060, DASH-035·037~039·052 | 충족 |
| AC-40 | MGT-029~030·032~038, OPS-061, DASH-036·040~041·043~044 | 충족 |
| AC-41 | MGT-040·042~045, OPS-062, DASH-042·045~049 | 충족 |
| AC-42 | MGT-030~031·033~036·038, OPS-059~061 | 충족 |
| AC-43 | DASH-050~051·053 | 충족 |
| AC-01~37(회귀) | 4.1절과 같은 연결 | 충족. 기존 153개는 두 최종 보고서에서 통과이거나 위 표에서 통과다 |

TODO 6.7의 운영 권한 확인은 다음과 같다. [수동 운영 안내 3.5.1절](operate-relay.md#351-대시보드-설정-보기편집)은 서버 실행 계정에 설정 디렉터리의 파일 생성·쓰기·교체 권한이 필요하다고 안내한다. DASH-044·045 실서버 시험이 두 OS에서 그 권한을 실제로 막았다. Windows는 `icacls <dir> /deny <USERNAME>:(WD,AD)`, Ubuntu는 디렉터리 `0o555`다. 그 결과 `500 config_write_failed`가 나고 원본 바이트·revision·실행 값·`last_reload`가 그대로였다. 권한을 되돌린 뒤 명시적으로 다시 저장하면 한 번만 적용됐다. Windows에서 공유 삭제 없이 연 핸들이 교체를 막고 임시 파일을 지우는 시험(`windows_open_handle_without_share_delete_prevents_rename_and_cleans_temp`)도 통과했다. 인증서 경로는 편집 대상이 아니며 같은 경로의 파일 교체는 [3.6절](operate-relay.md#36-인증서-교체)이 안내한다. 시험은 시험 실행 계정으로 했다. 별도 서비스 로그온 계정에서는 확인하지 않았다.

라이선스: 두 OS 최종 게이트의 deny 단계가 `licenses ok`, 종료 코드 0이었다. 고지 대조는 게이트 안의 `tests/dependency_notices.rs`(`notices --check`)가 두 OS에서 통과했다([라이선스 검사 실행 기록 9](check-dependency-licenses.md#4-검증)).

남은 일은 없다. 스펙·계약·설계·PRD의 상태 문장은 같은 변경에서 이 판정에 맞췄다([TODO 6.9](https://github.com/kayden-kim/vnc-relay/blob/556fab23d15e77125ca2ad5c575089857b312049/docs/TODO.md#6-설정-보기와-동적-설정-편집)).

후속: 사용자 결정 2026-11-08(Q181~Q188)의 설정 v4·ops-info 2.0.0·공유 WS·WSS 역할·프록시 TLS 종료·최근 로그는 위 작업 6.9 판정 뒤에 추가된 기준이다. 위 실행 기록과 판정은 당시 범위로 보존한다. 새 OPS-065~069와 변경된 기존 시나리오의 v4 범위는 미실행 (구현 전)이다. 과거 전체 게이트의 통과를 새 기준의 구현·검증 증거로 적용하지 않는다. 후속: 이 범위의 구현은 커밋 `3df0693`이다. [4.3절](#43-wss-server프록시-tls-종료최근-로그-판정표)에서 커밋 `51bd747`의 두 OS 게이트로 판정했다.

### 4.3 WSS Server·프록시 TLS 종료·최근 로그 판정표

결론: 커밋 `51bd747`의 두 OS 최종 게이트에서 `오류`와 `실패`는 0건이다. 새 시나리오 31개(WS-028~036, RLY-028, OPS-065~069, MGT-046~051, DASH-054~063)는 두 OS 보고서에서 모두 `통과`다. 기존 ID로 범위를 넓힌 시나리오 22개 가운데 DASH-023을 뺀 21개도 두 OS에서 `통과`다. 기존 215개 가운데 `통과`가 아닌 상태로 남은 시나리오는 4.2절과 같은 18개다. DASH-023은 범위를 넓힌 시나리오 표에서, 나머지 17개는 그 아래 표에서 4.1·4.2절과 같은 근거로 통과로 판정한다. 18개는 두 OS 보고서를 합친 수다. OS별로는 Windows 보고서에 17개, Ubuntu 보고서에 14개가 있다. 새 AC-44~48과 기존 AC-01~43을 모두 충족했다.

이 절의 위치와 형식은 이 문서의 판단, 사용자 결정 없음이다. 새 범위는 작업 목록의 작업이 아니라 사용자 지시 2026-11-08로 더했다. 그래서 판정 기준은 [PRD AC-44~48](../requirements/PRD.md#10-검증-및-수용-기준), 각 스펙 7절, [테스트 컨벤션 5절](../conventions/testing.md#5-검증-명령과-통과-기준)(사용자 결정 2026-11-06(Q132)), 사용자 결정 2026-11-04(Q99)다. 새 시나리오는 이전 판정으로 대체하지 않는다. 4.1·4.2절의 근거는 기존 시나리오에만 다시 쓴다. 보고서 도구는 바꾸지 않았다.

게이트는 세 커밋에서 실행했다. 실패의 원인은 모두 시험 결함이었으며 제품 코드는 바꾸지 않았다. 구현 커밋 `3df0693`의 첫 게이트는 두 OS에서 동시에 실행했다. Windows는 통과했고 Ubuntu는 아래 1·2번 결함으로 실패했다.

1. `tests/serve.rs`의 Unix 전용 시험 `writes_the_log_to_the_configured_file_from_start_to_stop`이 Ubuntu에서 실패했다. 이 시험은 `server_starting.log_contract`로 `ops-info/1.3.0`을 기대했다. 현재 계약은 [운영 정보 2.0.0](../contracts/ops-info-v2.md)이고 제품은 `ops-info/2.0.0`을 썼다. 기대값을 고쳤다(커밋 `f772408`). Windows는 이 시험을 빌드하지 않는다. 시나리오 표식이 없는 시험이므로 실패 시나리오로 집계되지 않았다.
2. DASH-060의 가짜 API 브라우저 시험 `logs_downloads_json_lines_in_browser`가 Ubuntu에서 12회 가운데 1회 실패했다(`lang=ko`, 폭 1023, `data-result=running`). 사례가 끝나기 전에 실행기가 DOM을 덤프했다는 뜻이다. 실행기는 `--virtual-time-budget=10000`으로 Chromium을 실행한다. 사례는 내려받을 내용을 `Blob.text()`로 다시 읽었다. 일회용 계측으로 단계별 가상 시간을 기록했다. 부하가 없을 때 24회의 사례 끝 시각은 328~1181ms였고, `text()` 대기 단계만 0~842ms로 크게 달랐다. CPU 부하를 걸자 24회 가운데 1회에서 `text()` 대기 중 가상 시간이 674ms에서 10654ms로 건너뛰어 같은 증상이 재현됐다. `text()`는 브라우저 프로세스의 응답을 실제 시간으로 기다리며, 그동안 가상 시간이 흐른다고 판단한다. 한 번에 한도 끝까지 건너뛴 이유는 Chromium 내부 동작이며 확인하지 않았다(추정). 하네스가 페이지가 `Blob`에 넘긴 문자열 조각을 보관하고 그 내용을 검사하도록 고쳤다(같은 커밋, `web/dashboard/tests/logs-scenarios.js`의 `captureDownloads`). 실제로 저장된 파일 바이트는 실서버 실행기 `dashboard_060_real_json_lines_download_and_no_partial_files`가 계속 확인한다. 이 실행기는 첫 게이트의 두 OS에서 통과했다.

수정 뒤 두 OS에서 로그 화면 브라우저 시험 9개(사례마다 12회)를 다시 실행했고 모두 통과했다. 시험 수정 커밋 `f772408`의 게이트는 처음에 두 OS에서 함께 시작했다. 같은 PC에서 운영 서버가 실행 중이어서 부하를 줄이려고 Ubuntu 게이트를 중단했다. Windows 게이트가 끝난 뒤 Ubuntu 게이트를 처음부터 다시 실행했다. Windows는 통과했고 Ubuntu는 아래 3번 결함으로 실패했다.

3. 기존 라우터 가짜 API 시험 `router_opens_session_detail_from_the_list_in_browser`가 Ubuntu에서 12회 가운데 1회 실패했다(`lang=ko`, 폭 320, `data-result=fail`). 실패한 검사는 `DASH-006: list row links to the selected session` 하나였다. 이 시험이 연결된 DASH-006·028·039·052·053이 실패로 집계됐고, 각 시나리오의 다른 시험은 통과했다. `router-scenarios.js`와 `operations-scenarios.js`의 `waitFor`는 `Date.now()`로 5000ms 기한을 쟀다. WSL의 시스템 시계를 70초 동안 0.1초 간격으로 쟀다. 시계는 약 30초마다 약 30초 뒤로 갔다가 0.2초 뒤 약 32초 앞으로 뛰었다. `systemd-timesyncd`는 실행 중이었지만 `timedatectl`은 `System clock synchronized: no`였다. 같은 Chrome으로 이 사례만 계측해 실행했을 때도 페이지의 `Date.now()`가 약 29초 뒤로 갔다. 대기 중에 시계가 측정값만큼(+32051ms) 앞으로 뛰도록 주입하자 5회 가운데 4회에서 같은 검사 하나만 실패했다(목록 링크 없음). 실패한 게이트 실행의 값은 직접 보지 못했으므로 이 원인은 추정이다. 커밋 `51bd747`은 두 하네스의 `waitFor`를 `session`·`settings`·`logs` 하네스와 같은 페이지 시간 단계(20ms 타이머 횟수)로 바꿨다. 5000ms 상한과 검사는 바꾸지 않았다. 같은 주입에서 5회 모두 통과했고, Ubuntu의 `router_`·`operations_` 브라우저 시험 17개도 통과했다.

최종 판정은 `51bd747`에서 Windows, Ubuntu 순서로 다시 실행한 두 OS 게이트를 기준으로 한다.

| 확인 대상 | 기준 | 환경 | 결과 | 증거 |
| --- | --- | --- | --- | --- |
| Windows 첫 실행 | `오류`·`실패` 0건 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | 통과. `ALL PASSED (all)`. 246건: 통과 229, 부분 12, 실패 0, 미실행 3, 미연결 2, 오류 0 | `target/scenario-report.md`(`3df0693`, 2026-11-08T22:26:25Z) |
| Ubuntu 첫 실행 | 같음 | Ubuntu 24.04.4 LTS x64(WSL2), Rust 1.99.1 | 실패. `FAILED: scenarios`. 246건: 통과 231, 부분 8, 실패 1(DASH-060), 미실행 4, 미연결 2, 오류 0. `cargo test` 실패 대상은 `dashboard_browser`·`serve`다. 원인은 위 1·2번이다 | `~/vnc-relay-target/scenario-report.md`(`3df0693`, 2026-11-08T22:30:21Z) |
| Windows 두 번째 실행 | 같음 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | 통과. `ALL PASSED (all)`. 246건: 통과 229, 부분 12, 실패 0, 미실행 3, 미연결 2, 오류 0. 작업 트리는 깨끗했다 | `target/scenario-report.md`(`f772408`, 2026-11-08T23:12:58Z) |
| Ubuntu 두 번째 실행 | 같음 | Ubuntu 24.04.4 LTS x64(WSL2), Rust 1.99.1 | 실패. `FAILED: scenarios`. 246건: 통과 227, 부분 8, 실패 5(DASH-006·028·039·052·053), 미실행 4, 미연결 2, 오류 0. `cargo test` 실패 대상은 `dashboard_browser`다. 원인은 위 3번이다 | `~/vnc-relay-target/scenario-report.md`(`f772408`, 2026-11-08T23:45:34Z) |
| Windows 최종 | 같음 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | 통과. `ALL PASSED (all)`. 246건: 통과 229, 부분 12, 실패 0, 미실행 3, 미연결 2, 오류 0. 작업 트리는 깨끗했다 | `target/scenario-report.md`(`51bd747`, 2026-11-09T00:32:47Z) |
| Ubuntu 최종 | 같음 | Ubuntu 24.04.4 LTS x64(WSL2), Rust 1.99.1 | 통과. `ALL PASSED`. 246건: 통과 232, 부분 8, 실패 0, 미실행 4, 미연결 2, 오류 0. `+ 미커밋 변경`은 줄 끝 차이다 | `~/vnc-relay-target/scenario-report.md`(`51bd747`, 2026-11-09T01:05:57Z) |

새 시나리오 31개의 판정은 다음과 같다. 보고서 열은 Windows / Ubuntu 순서다.

| 시나리오 | 보고서 | 판정 | 근거 |
| --- | --- | --- | --- |
| WS-028~031, RLY-028 | 통과 / 통과 | 통과 | `crates/adapters/tests/websocket_scenarios.rs`의 실제 공유 WS·WSS 리스너 시험이 통과했다. 두 전송의 Server·Viewer 매칭과 접속 순서, 경로별 역할과 404, 역할별 IP·Origin, 전송을 가로지르는 같은 ID 중복을 확인한다 |
| WS-032~034·036 | 통과 / 통과 | 통과 | 같은 파일의 신뢰 프록시 시험이 통과했다. 오른쪽 첫 비신뢰 주소, 잘못된 헤더와 모든 요소가 신뢰 프록시인 체인의 400, 비신뢰 상대 헤더 무시를 확인한다 |
| WS-035 | 통과 / 통과 | 통과 | 같은 파일의 `tls_terminating_proxy_adds_each_client_address_and_pipes_mode_two_both_ways`가 통과했다. 시험 안의 TLS 종료 프록시는 클라이언트 쪽에서 TLS를 받고, 중계 쪽에는 평문 WS로 `X-Forwarded-For`를 덧붙여 보낸다. Viewer·Server Mode II 양방향 전달과 운영 로그·관리 조회의 전달 주소를 확인한다. Nginx 실제 배치는 미실행이다([수동 운영 안내 3.9절](operate-relay.md#39-역방향-프록시로-tls-종료)) |
| OPS-065~069 | 통과 / 통과 | 통과 | 실제 실행파일 시험 `tests/serve.rs`(OPS-065·066·068·069)와 `tests/reload.rs`(OPS-067)가 통과했다 |
| MGT-046~050 | 통과 / 통과 | 통과 | Producer 계약 시험 `crates/adapters/tests/management.rs`가 통과했다 |
| MGT-051 | 통과 / 통과 | 통과 | 실제 실행파일 시험 `tests/serve.rs`의 `real_mode_i_and_proxied_ws_wss_sessions_expose_transport_and_forwarded_addresses`가 통과했다 |
| DASH-054~063 | 통과 / 통과 | 통과 | `tests/dashboard_live.rs`의 실서버 실행기 10개가 통과했다([대시보드 검증 4.31절](verify-dashboard-frontend.md#431-로그-화면-실서버-시험-dash-054063-2026-11-08)). 가짜 API 브라우저 시험 9개(부분 표식), 시안 대조 230쌍([4.30절](verify-dashboard-frontend.md#430-로그-화면-구현과-시안-대조-2026-11-08)), 네 엔진 16회 실행([4.32절](verify-dashboard-frontend.md#432-로그-화면-네-엔진-확인-2026-11-08))을 함께 보았다 |

기존 ID로 범위를 넓힌 시나리오는 다음과 같다. 변경 내용은 각 스펙 7절 끝의 2026-11-08 후속 기록에 있다.

| 시나리오 | 보고서 | 판정 | 근거 |
| --- | --- | --- | --- |
| OPS-016·037·038·058~064 | 통과 / 통과 | 통과 | 시나리오마다 비부분 시험이 두 OS 보고서에서 통과했다 |
| WS-012·014·022~024 | 통과 / 통과 | 통과 | 같다 |
| RLY-023·024 | 통과 / 통과 | 통과 | 같다 |
| DASH-028·029·035·037 | 통과 / 통과 | 통과 | 확장한 실서버 실행기가 통과했다([대시보드 검증 4.31절](verify-dashboard-frontend.md#431-로그-화면-실서버-시험-dash-054063-2026-11-08)) |
| DASH-023 | 미연결 / 미연결 | 통과 | 4.1절과 같은 수동 근거다. 넓힌 범위는 [대시보드 검증 4.32절](verify-dashboard-frontend.md#432-로그-화면-네-엔진-확인-2026-11-08)에서 Chrome·Edge·Firefox·WebKit(Safari 대체, 사용자 결정 2026-11-07(Q135))으로 두 폭·두 언어 16회를 실행했다. 로그인, 상세 이동, 세션 종료 확인·취소, 로그 필터·상세·더 보기·다운로드, 로그아웃이 가려지지 않고 동작했다. 쓴 버전(Chrome 157.0.8103.38, Edge 156.0.4314.58, Firefox 159.0)은 최신 안정판보다 낮다. 사용자 결정 2026-11-07(Q146)의 설치 버전과 같아 그 결정을 선례로 판정했다. 이 적용은 이 문서의 판단이다. 실제 Safari는 실행하지 않았다 |

나머지 17개는 다음과 같다. 판정 근거는 4.1·4.2절의 같은 행을 따른다.

| 시나리오 | 보고서 | 판정 | 근거 |
| --- | --- | --- | --- |
| DASH-015 | 부분 / 부분 | 통과 | 4.1절과 같다 |
| OPS-017 | 미실행 / 미실행 | 통과 | 4.1절과 같다. v4 설정의 재측정은 미실행이다 |
| OPS-018 | 미실행 / 미실행 | 통과 | 4.1절과 같다. v4 설정의 재측정은 미실행이다 |
| OPS-020 | 미연결 / 미연결 | 통과 | 4.1절과 같다 |
| OPS-021 | 부분 / 부분 | 통과 | 4.1절과 같다. 이번 범위는 의존성을 바꾸지 않았다 |
| OPS-023 | 부분 / 통과 | 통과 | 4.1절과 같다(Q99) |
| OPS-031 | 부분 / 부분 | 통과 | 4.1절과 같다 |
| OPS-040 | 부분 / 부분 | 통과 | 4.1절과 같다 |
| OPS-044 | 통과 / 미실행 | 통과 | 4.1절과 같다(Windows 전용 시나리오) |
| OPS-050 | 부분 / 부분 | 통과 | 4.2절의 조항별 판정과 같다 |
| RLY-006 | 부분 / 부분 | 통과 | 4.1절과 같다 |
| RLY-009 | 부분 / 통과 | 통과 | 4.1절과 같다(Q99) |
| RLY-010 | 부분 / 부분 | 통과 | 4.1절과 같다 |
| RLY-011 | 부분 / 부분 | 통과 | 4.1절과 같다 |
| RLY-025 | 부분 / 통과 | 통과 | 4.1절과 같다(Q86·Q99) |
| RLY-026 | 부분 / 통과 | 통과 | 4.1절과 같다(Q86·Q99) |
| WS-025 | 미실행 / 미실행 | 통과 | 4.1절과 같다 |

[PRD 10절](../requirements/PRD.md#10-검증-및-수용-기준)의 AC를 같은 기준으로 대조했다. 각 AC 행이 링크한 시나리오 ID를 두 최종 보고서와 위 표에 맞춰 보았다.

| AC | 연결 시나리오 | 판정 |
| --- | --- | --- |
| AC-44 | WS-028~031, RLY-028 | 충족 |
| AC-45 | WS-032~036, OPS-067, MGT-051 | 충족. 프록시 TLS 종료는 WS-035의 시험 프록시로 확인했다. Nginx 예시는 추정이며 미실행이다 |
| AC-46 | OPS-068, MGT-046~050, DASH-054~061·063 | 충족 |
| AC-47 | OPS-065~066·069 | 충족 |
| AC-48 | MGT-051, DASH-062 | 충족 |
| AC-01~43(회귀) | 4.1·4.2절과 같은 연결 | 충족. 기존 215개는 두 최종 보고서에서 통과이거나 위 두 표에서 통과다 |

라이선스: 두 OS 최종 게이트의 deny 단계가 `licenses ok`, 종료 코드 0이었다. 이번 범위는 의존성을 바꾸지 않았다. `f001385`부터 `51bd747`까지 `Cargo.lock`과 매니페스트·`deny.toml`의 변경이 없다. 고지 대조 시험 `tests/dependency_notices.rs`도 두 OS 게이트에서 통과했다.

자동 판정에 남은 일은 없다. 다음 확인은 미실행이며 이 판정에 포함하지 않는다.

- [수동 운영 안내 4절](operate-relay.md#4-검증)의 v3→v4 이관·롤백, 공유 WSS Viewer·Server, 프록시 TLS 종료(Nginx 실제 배치), 최근 로그 절차. 후속: v3→v4 이관은 2026-11-09 Windows 실제 배치에서 관리 재로그인을 빼고 실행했다. 롤백과 Ubuntu 이관은 미실행이다.
- v4 설정의 [성능 재측정](measure-performance.md).
- 로그 화면의 실제 Safari·최신 안정판 Chrome·Edge·Firefox·실기기·스크린 리더 확인과 사람의 시안 비교.
- 실제 UltraVNC와 WS·WSS Server 역할 조합의 수동 관찰.

### 4.4 Viewer 경로 이전 판정표

결론: 커밋 `41dda1f`의 두 OS 전체 게이트에서 `오류`와 `실패`는 0건이다. 범위를 바꾼 WS-012·WS-029는 두 OS 보고서에서 모두 `통과`다. 시나리오는 246건 그대로이며 새 ID는 없다. `통과`가 아닌 시나리오는 Windows 17개, Ubuntu 14개이고, 두 보고서를 합치면 4.3절 표의 18개와 같다. 시나리오마다 OS별 보고서 상태도 4.3절과 같다. 이 18개는 4.3절과 같은 근거로 통과로 판정한다. AC-01~48을 모두 충족했다.

이 절의 위치와 형식은 이 문서의 판단, 사용자 결정 없음이다. 변경은 사용자 결정 2026-11-10으로 더했다. 판정 기준은 4.3절과 같다.

| 확인 대상 | 기준 | 환경 | 결과 | 증거 |
| --- | --- | --- | --- | --- |
| Windows | `오류`·`실패` 0건 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | 통과. `ALL PASSED (all)`. 246건: 통과 229, 부분 12, 실패 0, 미실행 3, 미연결 2, 오류 0. 작업 트리는 깨끗했다 | `target/scenario-report.md`(`41dda1f`, 2026-11-10T12:05:09Z) |
| Ubuntu | 같음 | Ubuntu 24.04.4 LTS x64(WSL2), Rust 1.99.1 | 통과. `ALL PASSED`. 246건: 통과 232, 부분 8, 실패 0, 미실행 4, 미연결 2, 오류 0. `+ 미커밋 변경`은 줄 끝 차이다 | `~/vnc-relay-target/scenario-report.md`(`41dda1f`, 2026-11-10T12:39:25Z) |
| noVNC 1.7.0 | 무시 표시 시험을 포함한 실행에서 WS-025 시험 통과 | 두 OS, 고정 noVNC 압축 파일 | 통과. Windows 2 passed(43.54초), Ubuntu 2 passed(42.02초). noVNC `vnc.html`에 `path=relay/viewer`를 주면 WS·WSS와 두 접속 순서에서 Viewer로 등록하고 RFB 교환을 마쳤다 | [WebSocket 계약 실행 기록](../contracts/websocket-relay-v2.md#계약-테스트-실행-기록) |

범위를 바꾼 시나리오의 판정은 다음과 같다. 보고서 열은 Windows / Ubuntu 순서다.

| 시나리오 | 보고서 | 판정 | 근거 |
| --- | --- | --- | --- |
| WS-029 | 통과 / 통과 | 통과 | `crates/adapters/tests/websocket_scenarios.rs`의 `exact_enabled_paths_select_roles_and_rejections_never_register`가 통과했다. WS·WSS에서 `/relay/viewer`는 Viewer, `/relay/server`는 Server를 선택한다. 꺼진 역할의 경로와 이전 경로 `/relay`, `/relay/`, `/relay/viewer/`, `/relay/Viewer`, `/relay%2Fviewer`는 404다. 이때 `connection_rejected`(`invalid_upgrade`)만 남고 대기 등록이나 세션은 생기지 않는다 |
| WS-012 | 통과 / 통과 | 통과 | 같은 파일의 `management_paths_and_invalid_upgrades_cannot_register_or_connect_a_target`와 `crates/adapters/tests/websocket.rs`의 부분 시험이 통과했다. 이전 경로 `/relay`와 `/relay/viewer/`는 404다 |

WS-025는 두 게이트에서 `미실행`(무시 표시 시험)이며 4.3절 표와 같다. 별도 실행 결과는 위 noVNC 행에 있다.

[PRD 10절](../requirements/PRD.md#10-검증-및-수용-기준)의 AC 문구는 PRD 0.8에서 바뀌지 않았다. AC-01~48은 4.3절과 같은 방법으로 대조했고 모두 충족했다. 라이선스: 두 OS 게이트의 deny 단계가 `licenses ok`, 종료 코드 0이었다. 이번 범위는 `Cargo.lock`·매니페스트·`deny.toml`을 바꾸지 않았다.

다음 확인은 미실행이며 이 판정에 포함하지 않는다.

- [수동 운영 안내 3.2.3절](operate-relay.md#323-viewer-경로를-relayviewer로-옮기기)의 Viewer 경로 이전·롤백을 두 OS에서 실행하는 일.
- 사용처 페이지의 접속 URL 변경과 실제 연결 확인. 이 저장소 밖의 일이다.

### 4.5 접속 ticket 판정표

결론: 커밋 `4be8e0e`의 두 OS 전체 게이트에서 `오류`와 `실패`는 0건이다. 새 시나리오 20개(WS-037~046, RLY-029~032, OPS-070~073, MGT-052~053)는 두 OS 보고서에서 모두 `통과`다. 기존 ID로 범위를 넓힌 OPS-059·063·064·069와 DASH-020·029·035~037도 두 OS에서 `통과`다. 시나리오는 266건이다. `통과`가 아닌 시나리오는 Windows 17개, Ubuntu 14개이고, 두 보고서를 합치면 4.3절 표의 18개와 같다. 시나리오마다 OS별 보고서 상태도 4.3절과 같다. 이 18개는 4.3절과 같은 근거로 통과로 판정한다. 새 AC-49~52, 넓힌 AC-13, 기존 AC-01~48을 모두 충족했다. 고치기 전 구현 커밋의 첫 Windows 실행은 기존 시험 5개의 기대값 때문에 실패했다. 원인과 수정은 아래에 있다.

이 절의 위치와 형식은 이 문서의 판단, 사용자 결정 없음이다. 범위는 사용자 결정 2026-11-11(Q192~Q219)로 채택했고 사용자 지시 2026-11-11로 구현했다. 판정 기준은 [PRD AC-49~52](../requirements/PRD.md#10-검증-및-수용-기준), 각 스펙 7절, [테스트 컨벤션 5절](../conventions/testing.md#5-검증-명령과-통과-기준)(사용자 결정 2026-11-06(Q132)), 사용자 결정 2026-11-04(Q99)다. 새 시나리오는 이전 판정으로 대체하지 않는다.

첫 Windows 전체 게이트는 고치기 전의 구현 커밋에서 실행했다. 그 커밋은 아래 수정을 합쳐 `4be8e0e`로 다시 만들었으므로 공개 이력에는 없다. 기존 시험 5개가 ticket 이전의 기대값 때문에 실패했고, 제품 동작은 계약과 같았다.

1. DASH-039는 `applied`를 알파벳 순서로 기대했다. 관리 API 계약은 키 집합만 정하고 순서는 정하지 않는다. 시험을 순서와 무관하게 비교하도록 고쳤다.
2. DASH-050은 모든 입력에 `aria-describedby`가 있다고 보았다. 도움말·오류가 없는 ticket 문자열 입력에는 연결할 설명이 없다. 스펙의 "라벨·도움말·오류 연결"에 맞춰, 설명이 없는 입력을 허용하고 그려진 도움말이 입력에 연결되는지 확인하도록 고쳤다.
3. MGT-028은 ticket 키 10개가 빠진 65개 키 목록을 기대했다. 설정 계약 1.2절 순서의 75개 키로 고쳤다.
4. `tests/serve.rs`의 두 시험(OPS-065·OPS-069·WS-024)은 `ops-info/2.0.0`을 기대했다. `ops-info/2.1.0`으로 고쳤다. 같은 원인의 Unix 전용 시험 `writes_the_log_to_the_configured_file_from_start_to_stop`도 함께 고쳤다.

이 실행 도중 위 수정을 작업 트리에 시작해 보고서 대상에 `+ 미커밋 변경`이 붙었다. 실행된 시험 바이너리는 수정 전 소스로 만든 것이다. 최종 판정은 `4be8e0e`에서 Windows, Ubuntu 순서로 다시 실행한 두 OS 게이트를 기준으로 한다(이 문서의 판단, 사용자 결정 없음).

| 확인 대상 | 기준 | 환경 | 결과 | 증거 |
| --- | --- | --- | --- | --- |
| Windows 첫 실행 | `오류`·`실패` 0건 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | 실패. `FAILED (all): scenarios`. 266건: 통과 243, 부분 12, 실패 6(DASH-039·050, MGT-028, OPS-065·069, WS-024), 미실행 3, 미연결 2, 오류 0. `cargo test` 실패 대상은 `dashboard_live`·`management_query`·`serve`다. 새 시나리오 20개는 모두 통과였다. deny 단계는 `licenses ok`였다. 원인은 위 1~4번이다 | `target/scenario-report.md`(고치기 전 구현 커밋, 2026-11-11T12:10:30Z) |
| Windows 최종 | 같음 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | 통과. `ALL PASSED (all)`. 266건: 통과 249, 부분 12, 실패 0, 미실행 3, 미연결 2, 오류 0. 작업 트리는 깨끗했다. deny 단계는 `licenses ok`였다 | `target/scenario-report.md`(`4be8e0e`, 2026-11-11T12:54:30Z) |
| Ubuntu 최종 | 같음 | Ubuntu 24.04.4 LTS x64(WSL2), Rust 1.99.1 | 통과. `ALL PASSED`. 266건: 통과 252, 부분 8, 실패 0, 미실행 4, 미연결 2, 오류 0. `+ 미커밋 변경`은 줄 끝 차이다(`git diff --ignore-cr-at-eol` 출력 없음). deny 단계는 `licenses ok`였다 | `~/vnc-relay-target/scenario-report.md`(`4be8e0e`, 2026-11-11T13:31:18Z) |
| noVNC 1.7.0 | 무시 표시 시험을 포함한 실행에서 WS-025 시험 통과 | 두 OS, 고정 noVNC 압축 파일 | 통과. Windows 2 passed(43.64초), Ubuntu 2 passed(42.22초). ticket을 끈 기본 설정에서 noVNC `vnc.html`이 WS·WSS와 두 접속 순서로 Viewer 등록과 RFB 교환을 마쳤다 | [WebSocket 계약 실행 기록](../contracts/websocket-relay-v2.md#계약-테스트-실행-기록) |

새 시나리오 20개의 판정은 다음과 같다. 보고서 열은 Windows / Ubuntu 순서다.

| 시나리오 | 보고서 | 판정 | 근거 |
| --- | --- | --- | --- |
| WS-037, WS-039~045 | 통과 / 통과 | 통과 | `crates/adapters/tests/websocket_ticket.rs`의 실제 WS·WSS 리스너 시험 8개다. 꺼진 역할의 기존 서브프로토콜 동작(WS-037), ticket·`binary` 요구와 400·401 구분(WS-039), 형식·헤더·서명·클레임 오류(WS-040), `exp + 5`와 `sx` 경계(WS-041), 사용 기록의 원자성·WS·WSS 공유·`exp + 5` 만료(WS-042), 기동 시각 + 5초 기준(WS-043), Origin → 서브프로토콜 → ticket → IP 재검사 순서와 거절 때 미소비(WS-044), 역할별 발급자·키·대상(WS-045)을 확인한다 |
| WS-038, OPS-073 | 통과 / 통과 | 통과 | 실제 실행파일 시험 `tests/ticket_system.rs`의 `viewer_ws_wss_tickets_match_and_all_observable_surfaces_keep_secrets_private`다. WS·WSS Viewer ticket의 101과 응답 `binary`, `ticket_accepted` 필드, 같은 중계 ID의 Mode II 매칭을 확인한다. 성공·서명 실패·재사용·공개 키 파일 오류 뒤 운영 로그·stderr·`/livez`·`/readyz`·관리 API 응답에 ticket 원문·서명 부분·공개 키 좌표가 없는지 확인한다 |
| WS-046 | 통과 / 통과 | 통과 | 같은 파일의 `watched_and_api_ticket_reconfiguration_preserves_existing_bindings_timers_and_use_records`다. 파일 감시와 관리 API 편집으로 켜기·끄기·키 교체·값 교체를 한 뒤에도 기존 연결의 `rid` 결속(4012)·`sx` 타이머(4013)·사용 기록이 유지되는지 확인한다 |
| RLY-029, RLY-032 | 통과 / 통과 | 통과 | 같은 파일의 `both_ticket_roles_compare_rid_bytes_and_leave_the_waiting_opposite_peer_intact`다. 두 역할에서 `0123`과 `123`을 다른 중계 ID로 보고 Close 4012와 `ticket_relay_mismatch`를 남기는지 확인한다. 기다리던 상대 연결이 그대로 남아 맞는 ticket과 매칭되는지도 본다 |
| RLY-030 | 통과 / 통과 | 통과 | 같은 파일의 `ticket_mode_one_rejection_precedes_target_connect_but_not_disabled_mode_or_syntax`다. ticket 연결의 Mode I 요청이 대상 연결 없이 4012인지 확인한다. `mode1.enabled = false`의 1008과 구문 오류의 4008이 먼저인지도 본다 |
| RLY-031, MGT-052 | 통과 / 통과 | 통과 | 같은 파일의 `sx_expires_every_lifecycle_state_and_earliest_side_with_distinct_management_cause`다. 초기 교환·대기·활성 상태의 `sx` 만료, 두 쪽 가운데 이른 `sx`, TCP 상대의 종료를 확인한다. 최근 종료 목록에 `session_expired`가 관리자 종료와 구분되어 남는지도 본다 |
| OPS-070, OPS-072 | 통과 / 통과 | 통과 | `tests/reload.rs`의 `full_ticket_validation_at_startup_and_reload_preserves_the_last_valid_generation`과 `full_ticket_dynamic_reload_rotation_and_management_capabilities`다. 시작 때 진단·종료 코드 3·리스너 미바인드, 같은 후보의 재로드 거절과 기존 세대 유지, ticket 키 10개의 재로드·편집 구분, 공개 키 파일 교체 감지를 확인한다 |
| OPS-071 | 통과 / 통과 | 통과 | 같은 파일의 `ticket_tcp_and_plaintext_warnings_cover_both_roles_at_start_and_reload`다 |
| MGT-053 | 통과 / 통과 | 통과 | Producer 계약 시험 `crates/adapters/tests/management.rs`의 `ticket_config_view_edits_and_refusals_follow_management_contract`다 |

후속(2026-11-13): 두 번의 WSL 전체 게이트에서 RLY-031·MGT-052의 시험이 실패했다. 게이트와 따로 실행한 시험 파일 전체 실행에서는 WS-046의 시험도 한 번 실패했다. 원인은 WSL 벽시계 보정으로 추정한다. `sx` 만료 확인을 단조 시계로 재도록 시험만 고쳤다(커밋 `edd66fa`). 위 표의 2026-11-11 판정은 그대로 둔다. 실패 기록, 수정, 두 OS 게이트 결과는 [접속 ticket 모듈 설계의 수정 기록](../architectures/relay-access-ticket.md#세션-만료-시스템-시험의-시간-측정-수정)에 있다.

기존 ID로 범위를 넓힌 시나리오는 다음과 같다.

| 시나리오 | 보고서 | 판정 | 근거 |
| --- | --- | --- | --- |
| OPS-059·063·064·069 | 통과 / 통과 | 통과 | 재로드 43개·편집 37개와 `ops-info/2.1.0`을 기대하는 비부분 시험이다 |
| DASH-020·029·035~037 | 통과 / 통과 | 통과 | `tests/dashboard_live.rs`의 실서버 실행기 5개다. DASH-020은 실제 `sx` 만료 세션의 CPY-190 중립 배지를 확인한다. 가짜 API 브라우저 시험(부분 표식)과 시안 대조는 [대시보드 검증 4.34절](verify-dashboard-frontend.md#434-접속-ticket-화면-구현과-시안-대조-2026-11-11)에 있다 |

WS-025는 두 게이트에서 `미실행`(무시 표시 시험)이며 4.3절 표와 같다. 별도 실행 결과는 위 noVNC 행에 있다.

[PRD 10절](../requirements/PRD.md#10-검증-및-수용-기준)의 AC를 같은 기준으로 대조했다. 각 AC 행이 링크한 시나리오 ID를 두 보고서와 위 표에 맞춰 보았다.

| AC | 연결 시나리오 | 판정 |
| --- | --- | --- |
| AC-49 | WS-037~046 | 충족 |
| AC-50 | RLY-029~032 | 충족 |
| AC-51 | OPS-070~073 | 충족 |
| AC-52 | MGT-052~053 | 충족 |
| AC-13(넓힌 범위) | OPS-072, WS-046과 기존 연결 시나리오 | 충족 |
| AC-01~48(회귀) | 4.1~4.4절과 같은 연결 | 충족. 기존 시나리오는 두 보고서에서 통과이거나 4.3절 표에서 통과다 |

라이선스: 두 OS 게이트의 deny 단계가 `licenses ok`, 종료 코드 0이었다. 루트 `vnc-relay`에 시험 전용 dev-dependency `aws-lc-rs`를 더했고 잠금 파일의 새 패키지는 없다. 고지 대조 시험(`tests/dependency_notices.rs`)도 두 OS에서 통과했다([의존성 라이선스 확인 4절](check-dependency-licenses.md#4-검증) 실행 기록 10).

다음 확인은 판정 때 미실행이었으며 이 판정에 포함하지 않는다.

- [수동 운영 안내 3.10절](operate-relay.md#310-접속-ticket-켜기와-공개-키-교체)의 켜기·공개 키 교체·끄기를 두 OS에서 실행하는 일. 후속: 2026-11-14에 두 OS의 Release `v2026.11.3` 묶음으로 실행했다([수동 운영 안내 4절의 기록](operate-relay.md#4-검증)). 위 판정은 바꾸지 않는다.
- 실제 발급 쪽과 noVNC·VNC Server 클라이언트가 ticket을 보내는 연결 확인. 사용처 저장소의 일이다.
- 접속 ticket 화면의 실제 Safari·Edge·Firefox·실기기·스크린 리더 확인과 사람의 시안 비교.

### 4.6 운영 도움말 판정표

결론: 커밋 `92749c8`의 두 OS 전체 게이트에서 `오류`와 `실패`는 0건이다. 새 시나리오 19개(HELP-001~018, DASH-064)는 두 OS 보고서에서 모두 `통과`다. 시나리오는 285건이며 4.5절의 266건보다 19건 많다. `통과`가 아닌 시나리오는 Windows 17개, Ubuntu 14개이고, 두 보고서를 합치면 4.3절 표의 18개와 같다. 시나리오마다 OS별 보고서 상태도 4.3절과 같으므로 같은 근거로 통과로 판정한다. `main`에 push한 뒤 GitHub Pages 게시까지 마쳤다. 새 AC-53~55와 기존 AC-01~52를 모두 충족했다.

이 절의 위치와 형식은 이 문서의 판단, 사용자 결정 없음이다. 범위는 사용자 결정 2026-11-11(Q220~Q227)과 2026-11-12(Q228~Q231)로 채택했다. 판정 기준은 [PRD AC-53~55](../requirements/PRD.md#10-검증-및-수용-기준), 각 스펙 7절, [테스트 컨벤션 5절](../conventions/testing.md#5-검증-명령과-통과-기준)(사용자 결정 2026-11-06(Q132))이다.

| 확인 대상 | 기준 | 환경 | 결과 | 증거 |
| --- | --- | --- | --- | --- |
| Windows 최종 | `오류`·`실패` 0건 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1, mdBook 0.5.4 | 통과. `ALL PASSED (all)`. 285건: 통과 268, 부분 12, 실패 0, 미실행 3, 미연결 2, 오류 0. 작업 트리는 깨끗했다. deny 단계 종료 코드 0 | `target/scenario-report.md`(`92749c8`, 2026-11-12T00:21:05Z) |
| Ubuntu 최종 | 같음 | Ubuntu 24.04.4 LTS x64(WSL2), Rust 1.99.1, mdBook 0.5.4 | 통과. `ALL PASSED`. 285건: 통과 271, 부분 8, 실패 0, 미실행 4, 미연결 2, 오류 0. `+ 미커밋 변경`은 줄 끝 차이다(`git diff --ignore-cr-at-eol` 출력 없음). deny 단계 종료 코드 0 | `~/vnc-relay-target/scenario-report.md`(`92749c8`, 2026-11-12T00:55:53Z) |
| GitHub Actions `ci` | 결론 `success` | `ci` 실행 43131018506(`windows-2025`·`ubuntu-24.04`) | 통과. 두 `check` 작업이 형식·clippy·xtask 단위 시험·`layers`·`tokens --check`·`deny`·`cargo xtask package`를 마쳤다. `release`는 태그 push가 아니어서 건너뛰었다 | GitHub Actions 실행 기록 |
| 게시 | AC-55 | GitHub Actions `help` 실행 43131018701(`ubuntu-24.04`) | 통과. mdBook 자산 해시 `OK`, `help-site: 통과 (target/help-site)`, `deploy` 성공. 게시 주소가 HLP-002를 응답하고 저장소 About의 웹사이트가 같은 주소다 | [도움말 게시 how-to 4절](publish-help.md#4-검증) |

새 시나리오 19개의 판정은 다음과 같다. 보고서 열은 Windows / Ubuntu 순서다.

| 시나리오 | 보고서 | 판정 | 근거 |
| --- | --- | --- | --- |
| HELP-001~018 | 통과 / 통과 | 통과 | `tests/help_site.rs`의 18개 시험이다. `cargo xtask help-site`로 만든 사이트를 시험 안의 정적 서버(`/vnc-relay/` 마운트)에 올리고 헤드리스 Chromium으로 언어 선택·목차·검색·키보드·코드 복사·없는 페이지·고지·링크·좁은 폭을 확인한다. 시안 대조는 [도움말 게시 how-to 4.1절](publish-help.md#41-시안-대조)에 있다 |
| DASH-064 | 통과 / 통과 | 통과 | `tests/dashboard_browser.rs`의 `every_screen_localizes_safe_help_links_without_requesting_the_help_site_in_browser`다. 다섯 화면 페이지를 1280·390px와 ko·en에서 연다. 시안 대조는 [대시보드 검증 4.35절](verify-dashboard-frontend.md#435-대시보드-도움말-링크-구현과-시안-대조-dash-064-2026-11-12)에 있다 |

[PRD 10절](../requirements/PRD.md#10-검증-및-수용-기준)의 AC를 같은 기준으로 대조했다.

| AC | 연결 시나리오·절차 | 판정 |
| --- | --- | --- |
| AC-53 | HELP-001~018 | 충족 |
| AC-54 | DASH-064 | 충족 |
| AC-55 | [도움말 게시 how-to](publish-help.md) 3.5절 | 충족. 위 게시 행 |
| AC-01~52(회귀) | 4.1~4.5절과 같은 연결 | 충족. 기존 시나리오는 두 보고서에서 통과이거나 4.3절 표에서 통과다 |

라이선스: 두 OS 게이트의 deny 단계는 종료 코드 0이었다. 이 변경은 `Cargo.lock`과 `Cargo.toml`을 바꾸지 않았다. 도움말 사이트에 싣는 제3자 파일은 사이트의 고지 페이지가 다루며 HELP-017이 두 OS에서 통과했다([의존성 라이선스 검사 1절](check-dependency-licenses.md#1-대상과-영향)).

다음 확인은 미실행이며 이 판정에 포함하지 않는다.

- 도움말 사이트와 대시보드 도움말 링크의 실제 Safari·Edge·Firefox·실기기·스크린 리더 확인과 사람의 시안 비교.
- pull request에서 `help` 워크플로가 빌드·검사만 하고 게시하지 않는지 확인하는 일. 아직 pull request가 없다.
- [도움말 게시 how-to 6절](publish-help.md#6-롤백-또는-복구)의 Pages 끄기. 게시 중인 사이트를 내리는 일이라 실행하지 않았다.

#### 후속 수정 확인(커밋 `8f99434`)

결론: 검토 의견에 따라 고친 커밋 `8f99434`의 두 OS 전체 게이트에서 `오류`와 `실패`는 0건이다. HELP-001~018과 DASH-064는 두 OS 보고서에서 모두 `통과`다. 시나리오는 285건 그대로이고, 상태별 건수와 `통과`가 아닌 시나리오의 OS별 상태도 위와 같다. 그래서 위 판정과 AC 대조는 바뀌지 않는다.

| 고친 내용 | 관련 시나리오·AC | 확인 |
| --- | --- | --- |
| 워크플로 `help`의 동시 실행 그룹을 작업 `deploy`에서 실행 전체로 옮기고 ref마다 나눴다. 같은 ref의 실행은 하나씩 진행한다. 기다리던 실행은 더 새 실행으로 바뀌고, 진행 중인 실행은 취소하지 않는다 | AC-55 | 아래 게시 행과 [도움말 게시 how-to 4절](publish-help.md#4-검증)의 게시 순서 행 |
| `cargo xtask help-site` 검사가 빈 주소(공백뿐인 값 포함)를 문제로 보고한다 | HELP-016 | xtask 단위 시험 `empty_addresses_are_reported`, 두 게이트의 HELP-016 |
| HELP-007 시험이 제목에만 있는 단어(`엔드포인트`·`endpoints`)의 결과와 본문 없는 절의 결과를 확인한다 | HELP-007 | 두 게이트 |
| HELP-010 시험이 `ko/configure.html`의 탭 순회가 코드 복사 단추와 이전·다음 장 링크에 닿는지 확인한다 | HELP-010 | 두 게이트 |

| 확인 대상 | 기준 | 환경 | 결과 | 증거 |
| --- | --- | --- | --- | --- |
| Windows 최종 | `오류`·`실패` 0건 | 위 Windows 최종 행과 같음 | 통과. `ALL PASSED (all)`. 285건: 통과 268, 부분 12, 실패 0, 미실행 3, 미연결 2, 오류 0. 작업 트리는 깨끗했다. deny 단계 종료 코드 0 | `target/scenario-report.md`(`8f99434`, 2026-11-12T02:01:59Z) |
| Ubuntu 최종 | 같음 | 위 Ubuntu 최종 행과 같음 | 통과. `ALL PASSED`. 285건: 통과 271, 부분 8, 실패 0, 미실행 4, 미연결 2, 오류 0. `+ 미커밋 변경`은 줄 끝 차이다(`git diff --ignore-cr-at-eol` 출력 없음). deny 단계 종료 코드 0 | `~/vnc-relay-target/scenario-report.md`(`8f99434`, 2026-11-12T02:36:53Z) |
| 배포 묶음 | [3.3절](#33-변경-범위별-검증)의 워크플로 행 | Windows 11 Pro 10.0.26200 x64, Ubuntu 24.04.4 LTS x64(WSL2) | 통과. 두 OS에서 종료 코드 0이고 `notices --check: 통과`, `package:`·`sha256:` 줄이 나왔다(`vnc-relay-2026.11.2-windows-x64.zip`, `vnc-relay-2026.11.2-linux-x64.tar.gz`) | 이 표 |
| GitHub Actions `ci` | 결론 `success` | `ci` 실행 43142697338(`windows-2025`·`ubuntu-24.04`) | 통과. 두 `check` 작업이 형식·clippy·xtask 단위 시험·`layers`·`tokens --check`·`deny`·`cargo xtask package`를 마쳤다. `release`는 태그 push가 아니어서 건너뛰었다 | GitHub Actions 실행 기록 |
| 게시 | AC-55 | GitHub Actions `help` 실행 43142697359(`ubuntu-24.04`) | 통과. mdBook 자산 해시 `OK`, `help-site: 통과 (target/help-site)`, `build`(23초) 뒤 `deploy`(10초) 성공. 게시 주소의 루트·`ko/`·`en/`·장 페이지·글꼴 라이선스는 200, `/vnc-relay/ko`는 `/vnc-relay/ko/`로 301, 없는 경로는 404와 HLP-003이다 | [도움말 게시 how-to 4절](publish-help.md#4-검증) |

### 4.7 목록 정보 확대 판정표

결론: 커밋 `a94058d`의 두 OS 전체 게이트에서 `오류`와 `실패`는 0건이다. 새 시나리오 4개(MGT-054·055, DASH-065·066)는 두 OS 보고서에서 모두 `통과`다. 시나리오는 289건이며 4.6절의 285건보다 4건 많다. `통과`가 아닌 시나리오는 Windows 17개, Ubuntu 14개다. 두 보고서를 합치면 4.3절 표의 18개와 같고, 시나리오마다 OS별 보고서 상태도 4.3절 표와 같다. 그래서 같은 근거로 통과로 판정한다. `main`에 push한 뒤 CI와 도움말 게시도 성공했다. 새 AC-56과 기존 AC-01~55를 모두 충족했다.

이 절의 위치와 형식은 4.6절을 따른다(이 문서의 판단, 사용자 결정 없음). 범위는 사용자 결정 2026-11-17(Q234·Q235)로 채택했다. 판정 기준은 [PRD AC-56](../requirements/PRD.md#10-검증-및-수용-기준), 각 스펙 7절, [테스트 컨벤션 5절](../conventions/testing.md#5-검증-명령과-통과-기준)(사용자 결정 2026-11-06(Q132))이다.

먼저 구현 커밋 `8169bbf`의 Windows 게이트가 실패했다. 289건 가운데 통과 262, 부분 12, 실패 10, 미실행 3, 미연결 2, 오류 0이었다. 실패한 10건은 MGT-009·010·011·013·014·023·052, OPS-011, RLY-020·031이다. 원인은 기존 시험 두 곳이 Q234 전의 "주소 필드 없음"을 기대한 것이다. `tests/management_terminate.rs`의 종료 요약 검사(`summary`)를 쓰는 시험 5개와 `tests/ticket_system.rs`의 `session_expired` 항목 검사(`assert_expired_session`)를 쓰는 시험 1개가 실패했다. 변경 범위 시험에는 이 두 파일이 없었다. 두 검사를 새 계약에 맞게 고친 커밋이 `a94058d`이며 제품 코드는 바꾸지 않았다. 고친 뒤 Windows에서 두 파일의 시험 12개가 통과했다. 실패한 게이트의 보고서는 다음 게이트가 덮어썼다. 위 수치는 게이트 출력에서, 실패 시나리오는 덮어쓰기 전 보고서에서 옮겼다.

| 확인 대상 | 기준 | 환경 | 결과 | 증거 |
| --- | --- | --- | --- | --- |
| Windows 최초 | `오류`·`실패` 0건 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1, mdBook 0.5.4 | 실패. `FAILED (all): scenarios`. 실패 시나리오는 위 문단의 10건이다. 빌드·형식·clippy·`layers`·`tokens`·deny 단계는 종료 코드 0 | 게이트 출력(`8169bbf`) |
| Windows 최종 | 같음 | 같음 | 통과. `ALL PASSED (all)`. 289건: 통과 272, 부분 12, 실패 0, 미실행 3, 미연결 2, 오류 0. 작업 트리는 깨끗했다. deny 단계 종료 코드 0 | `target/scenario-report.md`(`a94058d`, 2026-11-17T01:32:53Z) |
| Ubuntu 최종 | 같음 | Ubuntu 24.04.4 LTS x64(WSL2), Rust 1.99.1, mdBook 0.5.4 | 통과. `ALL PASSED`. 289건: 통과 275, 부분 8, 실패 0, 미실행 4, 미연결 2, 오류 0. `+ 미커밋 변경`은 줄 끝 차이다(`git diff --ignore-cr-at-eol` 출력 없음). deny 단계 종료 코드 0 | `~/vnc-relay-target/scenario-report.md`(`a94058d`, 2026-11-17T02:09:30Z) |
| GitHub Actions `ci` | 결론 `success` | `ci` 실행 44001001684(`windows-2025`·`ubuntu-24.04`) | 통과. 두 `check` 작업이 성공했다. `release`는 태그 push가 아니어서 건너뛰었다 | GitHub Actions 실행 기록 |
| 게시 | AC-55 | GitHub Actions `help` 실행 44001001602(`ubuntu-24.04`) | 통과. `build`(47초)와 `deploy`(9초)가 성공했다. 게시된 `ko/`·`en/`은 200이다. 두 언어의 `images/operations.png`는 200이고 저장소 파일과 SHA-256이 같다 | [도움말 게시 how-to 4절](publish-help.md#4-검증) |

새 시나리오 4개의 판정은 다음과 같다. 보고서 열은 Windows / Ubuntu 순서다.

| 시나리오 | 보고서 | 판정 | 근거 |
| --- | --- | --- | --- |
| MGT-054 | 통과 / 통과 | 통과 | `tests/serve.rs`의 `real_waiting_lists_preserve_all_entry_transports_and_proxy_addresses`다. 실제 실행파일에서 모든 진입 전송 경로의 대기 항목과 프록시 전달 주소를 확인한다 |
| MGT-055 | 통과 / 통과 | 통과 | `tests/serve.rs`의 `real_closed_lists_and_termination_preserve_addresses_and_logged_monotonic_duration`다. 최근 종료 항목과 관리자 종료 응답의 주소를 확인하고, `duration_ms`를 `session_closed` 로그 값과 비교한다 |
| DASH-065 | 통과 / 통과 | 통과 | `tests/dashboard_browser.rs`의 `operations_waiting_rows_show_transport_and_peer_addresses_in_browser`다. 실서버 시험 `dashboard_065_066_real_waiting_and_closed_rows_with_forwarded_addresses`(부분 표식)도 통과했다. 시안 대조는 [대시보드 검증 4.36절](verify-dashboard-frontend.md#436-목록-정보-확대-q234q235-2026-11-17)에 있다 |
| DASH-066 | 통과 / 통과 | 통과 | `tests/dashboard_browser.rs`의 `operations_closed_rows_show_times_addresses_and_open_session_logs_in_browser`다. 이 시험의 키보드 선택은 초점과 해시 변경으로 대신하고, 실제 Enter 키는 위 실서버 시험이 확인한다. 시안 대조는 4.36절에 있다 |

[PRD 10절](../requirements/PRD.md#10-검증-및-수용-기준)의 AC를 같은 기준으로 대조했다.

| AC | 연결 시나리오·절차 | 판정 |
| --- | --- | --- |
| AC-56 | MGT-054·055, DASH-065·066 | 충족 |
| AC-01~55(회귀) | 4.1~4.6절과 같은 연결 | 충족. 기존 시나리오는 두 보고서에서 통과이거나 4.3절 표에서 통과다. AC-55의 게시는 위 게시 행에서 다시 성공했다 |

라이선스: 두 OS 게이트의 deny 단계는 종료 코드 0이었다. 이 변경은 `Cargo.lock`과 `Cargo.toml`을 바꾸지 않았다.

다음 확인은 미실행이며 이 판정에 포함하지 않는다.

- 대기 연결·최근 종료 표의 실제 Safari·Edge·Firefox·실기기·스크린 리더 확인과 사람의 시안 비교.

### 4.8 Windows 서비스·트레이 판정표

결론: 커밋 `738d068`의 두 OS 최종 게이트에서 `오류`와 `실패`는 0건이다. 이 커밋의 코드는 구현 커밋 `9c2a78b`와 같고, 그 뒤에는 문서만 바꿨다. Ubuntu 첫 실행은 의존성 캐시 준비를 빠뜨려 OPS-021이 실패했다. 준비한 뒤 다시 실행해 통과했다. 새 시나리오는 22개(OPS-074~081, TRAY-001~013, DASH-067)다. DASH-067은 두 OS 보고서에서 `통과`다. OPS-074~081과 TRAY-001~013은 보고서에서 `부분`·`미실행`·`미연결`이므로 시나리오마다 [8.4절](#84-검증-기록2026-11-18)의 실행 기록으로 판정했다. 남은 확인이 있어 AC-57~59는 모두 부분 충족이다.

이 절의 위치와 형식은 4.6절을 따른다(이 문서의 판단, 사용자 결정 없음). 범위는 사용자 결정 2026-11-17(Q239~Q257)과 사용자 지시 2026-11-17(DASH-067)이다. 판정 기준은 [PRD AC-57~59](../requirements/PRD.md#10-검증-및-수용-기준), 각 스펙 7절, [테스트 컨벤션 5절](../conventions/testing.md#5-검증-명령과-통과-기준)(사용자 결정 2026-11-06(Q132))이다.

| 확인 대상 | 기준 | 환경 | 결과 | 증거 |
| --- | --- | --- | --- | --- |
| Windows | `오류`·`실패` 0건 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1, mdBook 0.5.4 | 통과. `ALL PASSED (all)`. 311건: 통과 273, 부분 23, 실패 0, 미실행 6, 미연결 9, 오류 0. 게이트는 `9c2a78b`에서 시작했다. 실행 중에 문서 세 개만 바꾼 `738d068`을 커밋했고, 보고서를 만들 때 작업 트리는 깨끗했다. deny 단계 종료 코드 0 | `target/scenario-report.md`(`738d068`, 2026-11-17T22:28:32Z) |
| Ubuntu 최초 | 같음 | Ubuntu 24.04.4 LTS x64(WSL2), Rust 1.99.1, mdBook 0.5.4 | 실패. `FAILED: scenarios`. 311건: 통과 276, 부분 8, 실패 1(OPS-021), 미실행 17, 미연결 9, 오류 0. 고지 시험이 네트워크 없이 Windows 대상의 메타데이터를 읽다가 `adler2 v2.0.1`의 소스가 없어 실패했다. `9c2a78b`가 더한 Windows 전용 crate가 WSL의 Cargo 캐시에 없었다. [의존성 라이선스 검사 3.1절](check-dependency-licenses.md#31-배포-고지-생성과-대조) 1단계의 준비를 빠뜨린 것이다. 다른 단계의 종료 코드는 0 | 게이트 출력(`738d068`, 2026-11-17T23:05:42Z) |
| Ubuntu 최종 | 같음 | 같음 | 통과. 두 대상의 `cargo fetch --locked --target …`이 종료 코드 0으로 끝나고 `notices --check: 통과`를 확인한 뒤 다시 실행했다. `ALL PASSED`. 311건: 통과 275, 부분 10, 실패 0, 미실행 17, 미연결 9, 오류 0. `+ 미커밋 변경`은 줄 끝 차이다(`git diff --ignore-cr-at-eol` 출력 없음, Windows `git status` 깨끗함). deny 단계 종료 코드 0 | `~/vnc-relay-target/scenario-report.md`(`738d068`, 2026-11-17T23:45:45Z) |
| 배포 묶음 | [3.3절](#33-변경-범위별-검증)의 배포 묶음 행 | 위 두 OS | 통과. 두 OS에서 `cargo xtask package`가 종료 코드 0이고 `notices --check: 통과`, `package:`·`sha256:` 줄이 나왔다. Windows 묶음(`vnc-relay-2026.11.4-windows-x64.zip`)에는 `vnc-relay.exe`와 `vnc-relay-tray.exe`가 있다. Linux 묶음(`vnc-relay-2026.11.4-linux-x64.tar.gz`)에는 실행파일 `vnc-relay`만 있다. 실행파일은 `738d068`의 코드로 빌드했다. 묶음의 `docs/`에는 이 기록을 쓰던 중의 미커밋 문서가 들어갔으므로 이 묶음은 배포에 쓰지 않는다 | 이 표 |

Ubuntu 최종 보고서의 OPS-027 `부분`은 출력이 섞여 생긴 집계 차이다. `file_watch::tests::overtaken_candidate_is_never_applied_when_the_newest_write_is_valid_or_invalid`의 결과 줄 중간에 같은 시험 실행 파일이 표준 출력에 쓴 운영 로그 JSON 줄이 끼었고, `ok`는 다음 줄로 밀렸다. 보고 도구는 이 결과를 읽지 못해 `미실행(이번 실행에 없음)`으로 표시했다. `cargo test`는 통과했다. 같은 시험은 Windows와 Ubuntu 최초 실행에서 `통과`였다. 그래서 OPS-027은 통과로 판정한다.

기존 시나리오에서 `통과`가 아닌 것은 Windows 17개, Ubuntu 15개다. Windows 17개와 Ubuntu 14개는 4.3절 표와 같은 상태다. 나머지 Ubuntu 1개는 위 OPS-027이다.

새 시나리오의 판정은 다음과 같다. 보고서 열은 Windows / Ubuntu 최종 순서다. 관리자 자동 시험의 결과는 커밋 전 작업 트리에서 한 2차 실행이다. 그 뒤 시험의 정리 가드를 강화했고, 최종 시험 코드로 관리자 시험을 다시 실행하지 않았다(8.4절).

| 시나리오 | 보고서 | 판정 | 근거 |
| --- | --- | --- | --- |
| OPS-074 | 부분 / 미실행 | 통과(2차 실행), 미실행(최종 시험 코드의 관리자 재실행) | 단위 시험 `install_runs_the_steps_in_order_and_grants_every_operator`, 관리자 자동 시험 2차의 `install_registers_a_stopped_service_and_uninstall_keeps_the_files`와 기본 SDDL 기록 |
| OPS-075 | 부분 / 미실행 | 통과(2차 실행·일반 세션), 미실행(최종 시험 코드의 관리자 재실행) | `src/service.rs`의 단위 시험 3개, 2차의 `install_refuses_bad_input_and_leaves_no_registration`, 8.4절 일반 세션 설치 행 |
| OPS-076 | 미실행 / 미실행 | 통과(2차 실행), 미실행(최종 시험 코드의 관리자 재실행) | 2차의 `the_scm_runs_the_server_and_a_stop_request_applies_the_grace`. 실행 중 전 상태는 `[StartPending]`이었다 |
| OPS-077 | 미실행 / 미실행 | 통과(2차 실행), 미실행(최종 시험 코드의 관리자 재실행) | 2차의 `a_start_failure_stops_the_service_with_the_serve_exit_code`. 서비스 종료 코드 3·3·4·5 |
| OPS-078 | 미연결 / 미연결 | 미실행 | 로그오프와 Windows 재시작이 필요하다 |
| OPS-079 | 미연결 / 미연결 | 통과(지정한 운영자·관리자 권한·비상승 관리자 거절), 미실행(실제 표준 사용자 계정) | 8.4절의 서비스 운영자·운영자 지정 없음·실제 표준 사용자 행 |
| OPS-080 | 미실행 / 미실행 | 통과(2차 실행·일반 세션), 미실행(최종 시험 코드의 관리자 재실행) | 2차의 설치·SCM 시험(중지된 서비스 해제, `service_running`, `service_not_found`), 8.4절 일반 세션 해제 행 |
| OPS-081 | 부분 / 부분 | 통과 | Windows에서 `cli::tests::parses_service_subcommands_on_windows`와 `service_run_from_a_terminal_exits_2_without_opening_anything`, Ubuntu에서 `cli::tests::has_no_service_subcommand_outside_windows`와 `service_commands_are_usage_errors_outside_windows`가 통과했다. 각 OS 보고서에서 다른 OS 전용 시험이 `미실행(이번 실행에 없음)`이라 `부분`이다 |
| TRAY-001 | 미연결 / 미연결 | 통과 | 8.4절 트레이 판정표. 콘솔 창 없음은 실행파일의 PE 헤더로 확인했다 |
| TRAY-002 | 부분 / 미실행 | 통과 | 8.4절 트레이 판정표 |
| TRAY-003 | 부분 / 미실행 | 통과(관찰한 상태·켜짐), 미실행(실제 503·반영 시간) | 8.4절 트레이 판정표 |
| TRAY-004 | 미연결 / 미연결 | 통과 | 8.4절 트레이 판정표 |
| TRAY-005 | 미연결 / 미연결 | 통과 | 8.4절 트레이 판정표 |
| TRAY-006 | 미연결 / 미연결 | 통과(시작·중지 거절), 미실행(조회 거절·실제 표준 사용자) | 8.4절 트레이 판정표 |
| TRAY-007 | 부분 / 미실행 | 통과(서비스 없음 메뉴·해제 반영·해제 지연 없음), 판정 보류(설치 반영 시간) | 8.4절 트레이 판정표 |
| TRAY-008 | 부분 / 미실행 | 미실행(실제 브라우저) | 8.4절 트레이 판정표. URL 규칙은 단위 시험만 통과했다 |
| TRAY-009 | 부분 / 미실행 | 통과 | 8.4절 트레이 판정표 |
| TRAY-010 | 부분 / 미실행 | 통과(파일 있음), 미실행(로그 없음) | 8.4절 트레이 판정표 |
| TRAY-011 | 부분 / 미실행 | 통과(Run 값), 미실행(다시 로그인) | 8.4절 트레이 판정표 |
| TRAY-012 | 부분 / 미실행 | 미실행(표시 언어 전환) | 8.4절 트레이 판정표 |
| TRAY-013 | 미연결 / 미연결 | 통과(종료·서비스·Run 값), 미실행(요청·파일 열기 관찰) | 8.4절 트레이 판정표 |
| DASH-067 | 통과 / 통과 | 통과 | `tests/dashboard_browser.rs`의 `automatic_polls_under_one_second_keep_the_status_marks_in_browser`다. 기록은 [대시보드 검증 4.38절](verify-dashboard-frontend.md#438-자동-갱신-표시-지연-dash-067-2026-11-17)에 있다 |

`미연결` 7개(OPS-078·079, TRAY-001·004·005·006·013)에는 자동 시험이 없다. 확인에 Windows 재시작·로그오프, 다른 계정의 실제 UAC 필터링 세션, 알림 영역 메뉴의 실제 조작이 필요하기 때문이다. 게이트는 개발자가 쓰는 PC에서 돌므로 이런 조작을 기본 시험에 넣지 않았다(이 문서의 판단, 사용자 결정 없음). TRAY-001의 콘솔 없음은 실행파일 형식으로 자동 확인할 수 있다. 이번에는 한 번 확인한 결과만 기록했고 시험으로 만들지 않았다.

[PRD 10절](../requirements/PRD.md#10-검증-및-수용-기준)의 AC를 같은 기준으로 대조했다. DASH-067은 FR-06의 시나리오이며 연결된 AC가 없다.

| AC | 연결 시나리오·절차 | 판정 |
| --- | --- | --- |
| AC-57 | OPS-074·075·080·081 | 부분 충족. 모든 분기를 실행해 통과했다. 관리자 권한 분기는 2차 실행 결과이며 최종 시험 코드로 다시 실행하지 않았다 |
| AC-58 | OPS-076~079 | 부분 충족. OPS-078과 표준 사용자 권한 확인이 미실행이다. 관리자 권한 분기는 AC-57과 같다 |
| AC-59 | TRAY-001~013 | 부분 충족. 위 표의 미실행 확인과 TRAY-007 판정 보류가 남았다 |
| AC-01~56(회귀) | 4.1~4.7절과 같은 연결 | 충족. 기존 시나리오는 두 보고서에서 통과이거나 4.3절 표에서 통과다. Ubuntu 최종의 OPS-027은 위 집계 차이이며 통과로 판정한다 |

라이선스: 두 OS 최종 게이트의 deny 단계는 종료 코드 0이었다. 이 변경은 `Cargo.lock`에 서비스·트레이 의존성을 더했다. 통합 검사는 두 대상 그래프의 합집합을 검사한다([의존성 라이선스 검사](check-dependency-licenses.md)). 배포 고지는 두 OS의 `notices --check`가 통과했다.

다음 확인은 미실행이며 이 판정에 포함하지 않는다.

- 최종 시험 코드의 관리자 자동 시험 재실행과 TRAY-007 설치 반영 시간의 정밀 측정. 둘 다 관리자 권한 실행과 사용자 승인이 필요하다.
- OPS-078의 로그오프·재시작, 실제 표준 사용자 계정의 권한 경계.
- TRAY-003의 실제 503, TRAY-006의 조회 거절, TRAY-008의 실제 브라우저, TRAY-010의 로그 없음, TRAY-011의 다시 로그인, TRAY-012의 표시 언어 전환, TRAY-013의 네트워크·파일 열기 관찰.
- GitHub Actions `ci`. 아직 push하지 않았다.

## 5. 중단 조건

- 1단계 Build Tools 설치가 1602로 끝나면 이후 단계를 진행하지 않는다. UAC를 승인할 수 있을 때 다시 실행한다.
- 3단계 버전이 `rustc 1.99.1`이 아니면 진행하지 않는다. `rust-toolchain.toml`과 `rustup show active-toolchain` 출력을 확인한다.
- 6단계 명령이 하나라도 실패하면 완료로 보고하지 않는다. 실패한 명령의 출력을 고친 뒤 6단계를 처음부터 다시 실행한다.

## 6. 롤백 또는 복구

1. 빌드 결과가 꼬였다고 의심되면 `cargo clean`으로 Cargo 대상 디렉터리를 지운다. 소스는 바뀌지 않는다.
2. 4단계를 다시 실행해 `vnc-relay <버전>`이 나오는지 확인한다.

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

### 8.4 검증 기록(2026-11-18)

- 대상: 기반 커밋 `6e5cb2d` 위의 미커밋 Windows 서비스·트레이 구현 작업 트리다. 구현·시험·배포 고지 변경을 포함한다. 아래 실행 사이의 시험 수정과 1077 표시 수정은 구분해서 적는다.
- 환경: 한국어 Windows 11 Pro 빌드 26200, 로컬 관리자 계정이다. 일반 터미널은 실제 UAC 필터링 세션의 Medium IL(`S-1-16-8192`)이다. 트레이는 debug 빌드다. WSL의 비Windows CLI 시험 결과는 앞서 실행한 결과다.
- 방법: 자동 시험은 8.1절의 빌드와 시험 실행파일 직접 실행을 썼다. 사용자 승인으로 UAC 실행을 두 번 했다. 수동 시험은 실제 일반 터미널의 CLI·`sc.exe`와 서비스 상태 조회를 썼다. 트레이는 UI Automation과 Win32 메뉴 메시지로 관찰했다.
- 판정 범위: 아래 개별 결과만 기록한다. 전체 게이트·Release 검증이나 모든 시나리오 통과를 뜻하지 않는다. 미실행 분기는 통과로 합치지 않는다.
- 후속: 커밋 뒤의 두 OS 전체 게이트·배포 묶음 결과와 시나리오별 판정은 [4.8절](#48-windows-서비스트레이-판정표)에 있다.

| 확인 대상 | 기준 | 결과 | 확인 방법·증거 |
| --- | --- | --- | --- |
| 관리자 자동 시험 1차 | 일반 세션 거절도 올바른 토큰으로 확인 | 실패. OPS-074·076·077 시험 3개는 통과했다. 나머지 2개는 토큰 사전 확인에서 중단했다. 관리자 프로세스의 `runas /trustlevel:0x20000` 결과가 일반 세션이 아니라 High IL이었다 | 시험 출력·토큰 확인이다. 잘못된 대용 프로세스 부분을 시험에서 제거하고 일반 세션 사례를 수동 확인으로 옮겼다 |
| 관리자 자동 시험 2차 | 무시된 시험 4개 통과 | 통과. `4 passed; 0 failed`, 13.73초다. 실행 중 전 상태는 `[StartPending]`이었다. 중지 중 두 번째 요청은 `exit Some(1061)`이었다 | `--ignored --test-threads=1 --nocapture` 출력의 `record:` 줄이다 |
| fixture 정리 가드 강화 뒤 관리자 재실행 | 실행파일 토큰 정확 일치 가드가 있는 최종 코드 | 미실행. 2차 뒤 정리 가드를 강화했다. 컴파일과 기본 시험만 통과했다. 최종 가드로 등록·정리하는 관리자 시험은 다시 실행하지 않았다 | 최종 코드의 관리자 실행 결과는 이 기록에 없다 |
| 기본 Windows 시험 | 기본 시험·단위 시험 통과 | 통과. `windows_service` 기본 시험, `src/service.rs` 단위 시험 4개, 트레이 crate 단위 시험 11개와 추가 시험 1개가 통과했다 | 기본 권한 실행 결과다 |
| 비Windows CLI | 서비스 명령을 사용법 오류로 거절 | 통과. WSL의 `tests/cli.rs` 해당 시험이 앞서 통과했다 | 기존 WSL 실행 결과다 |
| 서비스 운영자(OPS-079) | 일반 세션에서 UAC 없이 시작·중지 | 통과. `--operator`로 지정한 사용자의 `sc.exe stop`은 0 뒤 STOPPED, `sc.exe start`는 0 뒤 RUNNING이었다 | 실제 일반 터미널이다 |
| 일반 세션 설치(OPS-075) | 권한 거절 | 통과. 종료 코드 1과 `error[service_access_denied]: 서비스를 다룰 권한이 없다. 관리자 권한 터미널에서 실행한다.`였다 | `vnc-relay.exe service install --config relay.toml`이다 |
| 일반 세션 해제(OPS-080) | 권한 거절, 기존 서비스 유지 | 통과. 실행 중 `service uninstall`은 종료 코드 1과 같은 권한 오류였다. 서비스는 RUNNING을 유지했다 | 실제 일반 터미널이다 |
| 운영자 지정 없음(OPS-079) | 일반 세션에서 시작·중지 거절 | 통과(비상승 관리자 세션 범위). 다시 설치할 때 `--operator`를 빼자 트레이에 `시작 실패: 권한 없음`·`중지 실패: 권한 없음`이 나왔다. 관리자 권한 시작은 성공했다 | 같은 일반 세션 사용자다. 실제 표준 사용자와 같은 계정 조건은 아니다 |
| 실제 표준 사용자 | 비관리자 계정에서 권한 경계 확인 | 미실행. PC에 해당 계정이 없다. 표준 사용자 계정 조건은 확인하지 못했다 | 비상승 관리자 세션의 거절로 대체 완료 처리하지 않는다 |
| 로그오프·재시작(OPS-078) | 서비스 지속·시스템 종료 처리 | 미실행. Windows 로그오프·재시작이 필요하다. 시스템 종료 시간 제한(OPS-TBD-014)도 관찰하지 못했다 | 서비스 종료 경계 전체를 검증한 것은 아니다 |
| 기본 파일 접근 | 서비스와 트레이에 필요한 접근 | 통과(관찰 환경). 관리자가 `%ProgramData%` 아래 만든 폴더에 별도 허용을 주지 않아도 서비스가 설정·계정을 읽고 로그를 만들고 썼다. 일반 세션 사용자도 설정을 읽어 트레이를 썼다 | 기본 ACL에서 서비스 RUNNING을 확인했다. 로컬 사용자에게 비밀 파일도 읽힐 수 있으므로 [파일 접근 제한](operate-relay.md#331-windows-서비스와-트레이-앱)을 권장한다 |
| 통합 라이선스 검사 | `licenses ok` | 통과. 작업 공간 통합 검사가 `vnc-relay-tray`를 포함했다. 대상별 후속 검사는 필요하지 않다 | `cargo deny check licenses` 결과다 |

OPS-074가 기록한 기본 서비스 보안 설명자(SDDL)는 다음과 같다. IU(대화형 사용자)는 상태·구성 조회만 할 수 있고 시작·중지는 할 수 없다. `--operator`는 시작·중지·상태 조회·구성 조회를 추가한다.

```text
D:(A;;CCLCSWRPWPDTLOCRRC;;;SY)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCSWLOCRRC;;;IU)(A;;CCLCSWLOCRRC;;;SU)S:(AU;FA;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;WD)
```

트레이의 판정표는 다음과 같다. 관찰 시간은 클릭·설치 완료 같은 측정 기준점을 함께 적는다.

| 시나리오 | 결과 | 실제 관찰·미실행 이유 |
| --- | --- | --- |
| TRAY-001 | 통과 | 상태 줄, 시작, 중지, 대시보드 열기, 로그 파일 위치 열기, 설정 파일 위치 열기, 로그인할 때 시작, 트레이 끝내기 순서다. 다시 시작과 UAC 창은 없었다. 콘솔 창 없음은 실행파일 형식으로 확인했다. 게이트(4.8절)가 만든 `vnc-relay-tray.exe`의 PE 헤더 Subsystem 값은 2(GUI)였고, 같은 빌드의 `vnc-relay.exe`는 3(콘솔)이었다 |
| TRAY-002 | 통과 | 두 번째 프로세스는 실행 시간을 포함해 약 1.1초에 종료 코드 0으로 끝났다. 아이콘 하나·프로세스 하나였다 |
| TRAY-003 | 통과(관찰한 상태·켜짐), 미실행(실제 503·반영 시간) | 중지됨·시작 중·실행 중·실행 중(준비 안 됨)·중지 중·중지됨 (종료 코드 3)을 확인했다. 준비 안 됨은 설정의 상태 주소를 미사용 포트로 바꾼 연결 실패다. 실제 `/readyz`는 200이었다. 잘못된 설정의 시작 실패는 SCM `WIN32_EXIT_CODE` 1066, `SERVICE_EXIT_CODE` 3이었다. 켜짐은 스펙 4절과 같았다. 503 분기는 단위 시험만 통과했고 실제로 만들지 않았다. 상태 변화가 다음 조회에서 반영되는지는 시간을 재지 않았다 |
| TRAY-004 | 통과 | 클릭 뒤 시작 중까지 약 1.5초, 실행 중까지 약 2.9초였다. UAC 창은 없었다 |
| TRAY-005 | 통과 | TCP Mode II 활성 세션 1개와 G=10초에서 클릭 뒤 0.3초에 중지 중, 10.2초에 중지됨이었다. 확인 대화상자는 없었다. `shutdown_started`의 `trigger`는 `requested`, `grace_secs`는 10, `active_sessions`는 1이었다. 이어 `shutdown_forced`의 `remaining_sessions`는 1이고 `server_stopped`를 기록했다 |
| TRAY-006 | 통과(시작·중지 거절), 미실행(조회 거절·실제 표준 사용자) | 운영자가 아닌 일반 세션에서도 기본 IU 조회 권한으로 상태가 보였다. 시작·중지 권한 실패 뒤 서비스 상태는 그대로였고 다른 동작을 고르면 실패 줄이 사라졌다. 기본 SDDL이 조회를 허용하므로 조회 거절 분기는 만들지 못했다. 실제 표준 사용자 계정은 없었다 |
| TRAY-007 | 통과(서비스 없음 메뉴·해제 반영·해제 지연 없음), 판정 보류(설치 반영 시간) | 서비스 없음에서 시작·중지·대시보드·두 위치 열기는 꺼졌다. 해제 완료 06:22:50.276→서비스 없음 06:22:50.726은 0.45초다. 트레이 실행 중 해제는 81~86ms로 지연되지 않았다. 설치 완료 06:22:56.437→중지됨 06:22:58.734는 2.30초다. 관찰기의 표본 간격이 0.3~0.5초라서 실제 표시 시각은 설치 완료 뒤 1.8~2.3초 사이로만 좁혀진다. 기대 결과는 다음 조회에서의 반영이다. 조회는 시작 기준 2초 간격이고, 표시까지 조회 시간과 최대 0.1초의 UI 대기가 더해진다. 조회 시간을 재지 않았으므로 이 범위가 기대 결과 안인지 확정하지 못했다. 그래서 설치 반영은 통과로 판정하지 않는다. 정밀 측정에는 관리자 권한 설치를 다시 해야 하며 사용자 승인이 필요하다. 이 관찰 뒤 스펙의 "2초 안에"를 "다음 조회(2초 간격)에서"로 고쳤다. 2초 간격 조회로는 변화 뒤 2초 안의 표시를 보장할 수 없기 때문이다(이 문서의 판단, 사용자 결정 없음) |
| TRAY-008 | 미실행(실제 브라우저) | 사용자의 기본 브라우저를 열게 되므로 실행하지 않았다. URL 규칙은 단위 시험으로 확인했다. 실제 브라우저 열기까지 통과한 것은 아니다 |
| TRAY-009 | 통과 | 실행 중·중지됨에서 읽기 거절→`설정을 읽지 못함: 권한 없음`, 삭제→`설정을 읽지 못함: 파일 없음`, 잘못된 파일→`설정을 읽지 못함: 설정 오류`였다. 실행 중 상태는 실행 중(준비 안 됨)이었다. 대시보드·로그 위치는 꺼지고 설정 위치는 켜졌다. 원인을 없애면 다음 갱신에서 정상으로 돌아왔다 |
| TRAY-010 | 통과(파일 있음), 미실행(로그 없음) | 설정 위치는 탐색기에서 `relay.toml`, 로그 위치는 `relay.log`를 선택했다. 로그 파일이 없는 경우는 실행하지 않아 디렉터리 열기를 확인하지 못했다 |
| TRAY-011 | 통과(Run 값), 미실행(다시 로그인) | 값 없음은 체크 꺼짐이었다. 켜면 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`의 `vnc-relay-tray` 값에 따옴표 친 실행파일 경로를 쓰고 체크됐다. 끄면 지웠다. 다른 경로 값은 체크 꺼짐이고 켜면 현재 실행파일 경로로 덮어썼다. 시험 뒤 원래의 값 없음으로 복구했다. 다시 로그인은 실행하지 않아 실제 자동 실행은 확인하지 못했다 |
| TRAY-012 | 미실행(표시 언어 전환) | Windows 표시 언어 변경과 다시 로그인이 필요하다. 언어 선택 규칙은 단위 시험으로 확인했다. 언어별 실제 메뉴는 확인하지 못했다 |
| TRAY-013 | 통과(종료·서비스·Run 값), 미실행(요청·파일 열기 관찰) | 서비스 실행 중과 로그인할 때 시작 켜짐에서 트레이 끝내기를 골랐다. 아이콘이 사라지고 프로세스는 0으로 끝났다. 서비스는 RUNNING이고 Run 값은 남았다. 시험 뒤 그 값을 지웠다. 짧은 `/readyz` 연결을 표본 수집으로 잡지 못해 네트워크·파일 열기 관찰은 완료하지 못했다. 코드 검토로만 worker가 상태 리스너에 연결하고 설정 파일만 읽는 것을 확인했다 |

1077 표시를 고치기 전 빌드의 트레이 프로세스가 확인 중 한 번 끝났다. Windows Error Reporting 사건은 없었고 종료 코드는 수집하지 못했다. 원인은 설명하지 못했다. 이후 세 번 실행에서는 구분선 클릭을 포함해 재현하지 못했다. 이 단일 종료를 정상 종료나 해결된 문제로 처리하지 않는다.

설치 직후 `중지됨 (종료 코드 1077)` 표시는 1077을 종료 코드 없음으로 다루도록 고친 뒤 `중지됨`이 됐다. 이 표시는 부팅 뒤 아직 시작하지 않은 서비스에 대한 SCM 값의 처리다. 모든 시작 실패를 숨기는 예외는 아니다.
