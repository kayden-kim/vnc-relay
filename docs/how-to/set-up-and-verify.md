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

4. 실행 결과를 [WebSocket 중계 계약 5절](../contracts/websocket-relay-v1.md#5-예제와-계약-테스트)의 실행 기록에 남긴다. 대상 커밋, OS, 명령, 결과를 적는다.

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

   기대 결과: 빌드가 `Finished`로 끝나고 `vnc-relay 0.1.0`이 출력된다.

### 3.3 변경 범위별 검증

5. 바꾼 범위에 맞는 명령을 실행한다. 여러 범위에 걸치면 해당 행을 모두 실행한다.

   | 바꾼 범위 | 실행할 명령 | 성공 조건 |
   | --- | --- | --- |
   | Rust 코드·테스트 | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` | 모두 종료 코드 0, clippy 경고 없음 |
   | 시나리오 테스트(표식 추가·변경), 스펙 수용 테스트 표 | `cargo xtask scenarios` | 종료 코드 0, 보고서에 `오류`·`실패` 없음([테스트 컨벤션 5절](../conventions/testing.md#5-검증-명령과-통과-기준)) |
   | `Cargo.toml` 의존성, crate 추가 | `cargo xtask layers`, `cargo deny check licenses`([의존성 라이선스 검사](check-dependency-licenses.md)) | `레이어 검사: 통과`, `licenses ok` |
   | `docs/design/DESIGN.md` YAML, `web/dashboard/` | `cargo xtask tokens --check`, 브라우저 테스트는 `cargo xtask scenarios`가 함께 실행한다([대시보드 프런트엔드 검증 how-to](verify-dashboard-frontend.md)) | `tokens --check: 통과`, 종료 코드 0 |
   | 문서만 | 이 절차의 명령은 필요 없다. 문서 유형별 작성 규칙과 링크를 확인한다. | — |

   한 crate만 빠르게 확인할 때는 `cargo test --package <패키지 이름>`을 쓸 수 있다. 완료 보고 전에는 6단계를 실행해야 한다.

### 3.4 전체 완료 검증

6. 저장소 루트에서 다음 명령을 순서대로 실행한다. 하나라도 실패하면 멈추고 [5절](#5-중단-조건)을 따른다.

   ```shell
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo xtask layers
   cargo xtask tokens --check
   cargo xtask scenarios
   cargo deny check licenses
   ```

   기대 결과: 여섯 명령 모두 종료 코드 0. `cargo xtask scenarios`가 `cargo test --workspace --no-fail-fast`를 함께 실행하므로 따로 실행하지 않는다. 대시보드 브라우저 테스트도 이때 실행되며 Chromium이 없으면 실패한다(2절). 출력 마지막에 `판정: 통과`, 라이선스 검사 마지막 줄에 `licenses ok`가 나온다. 단계를 마칠 때(작업 5.9)는 `cargo xtask scenarios --require-all`로 바꿔 실행한다.

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

실행 기록 3: 2026-10-20 같은 Windows 11 Pro 10.0.26200 x64에서 UAC를 승인해 1단계 설치를 마쳤다. 커밋 `e1cfc4f`에 Windows 경고 수정(`crates/adapters/src/net.rs` 테스트 모듈을 Linux 전용으로, `tests/serve.rs`의 `lifecycle_events`를 Unix 전용으로)을 더한 작업 트리에서 실행했다. PowerShell에서 `PATH`에 `%USERPROFILE%\.cargo\bin`을 더하고 `AWS_LC_SYS_PREBUILT_NASM=1`을 정했다. Cargo 대상 디렉터리는 저장소의 `target\`이다. 3·4·6단계의 명령은 모두 종료 코드 0이었다. 수정 전 clippy 오류는 `unused import: super::*`(`crates\adapters\src\net.rs`)와 `function lifecycle_events is never used`(`tests\serve.rs`)였다. 같은 작업 트리에서 Ubuntu 6단계도 다시 통과했다. 테스트별 결과는 [운영 정보 계약 실행 기록](../contracts/ops-info-v1.md#계약-테스트-실행-기록)에 있다.

실행 기록 4: 2026-10-22 Ubuntu 24.04.4 LTS x86_64(WSL2, 커널 6.6.87.2-microsoft-standard-WSL2)에서 Chrome을 설치했다. `sudo`가 암호를 요구해 2절의 `sudo apt install` 명령은 실행하지 않았다. 대신 Windows에서 root 셸로 같은 .deb를 설치했다: `wsl -d Ubuntu -u root -- bash -lc 'cd /tmp && wget -q https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb && DEBIAN_FRONTEND=noninteractive apt-get install -y -q ./google-chrome-stable_current_amd64.deb'`. 받은 .deb 파일은 설치 뒤 지웠다. 설치 뒤 `command -v google-chrome`은 `/usr/bin/google-chrome`, `dpkg-query -W google-chrome-stable`은 `156.0.8078.47-1`이었다. 커밋 `df3b5b2`(작업 트리 변경 없음)에서 6단계를 다시 실행했다. 여섯 명령이 모두 종료 코드 0이었고 브라우저 테스트 2건이 통과했다. 시나리오 150건: 통과 16, 부분 23, 실패 0, 미실행 0, 미연결 111, 오류 0건. 같은 커밋의 Windows 6단계도 통과했다(통과 15, 부분 23, 미실행 1 = OPS-007, 미연결 111).

### 4.1 작업 5.9 판정표

결론: 두 OS 보고서의 `오류`와 `실패`는 0건이다. 보고서에 `통과`가 아닌 상태로 남은 17개 시나리오 가운데 16개는 아래 근거로 통과로 판정한다. OPS-021은 MIT LICENSE의 저작권자·적용 연도(사용자 결정 2026-11-04(Q97))가 정해지지 않아 통과로 판정하지 않는다. 그래서 AC-12와 작업 5.9는 아직 완료가 아니다.

이 절의 위치와 형식은 이 문서의 판단, 사용자 결정 없음이다. 판정 규칙은 [TODO 5.9](../TODO.md#5-운영배포-검증)의 완료 조건, [테스트 컨벤션 5절](../conventions/testing.md#5-검증-명령과-통과-기준)(사용자 결정 2026-11-06(Q132)), 사용자 결정 2026-11-04(Q99)를 따른다. 보고서 도구는 바꾸지 않았다.

기준 보고서는 커밋 `ee0314f`의 두 OS 전체 게이트(2026-11-07)다.

| 확인 대상 | 기준 | 환경 | 결과 | 증거 |
| --- | --- | --- | --- | --- |
| Windows 시나리오 보고서 | `오류`·`실패` 0건 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1 | 통과. `ALL PASSED (all)`. 153건: 통과 137, 부분 11, 실패 0, 미실행 3, 미연결 2, 오류 0 | `target/scenario-report.md`(2026-11-07T09:31:01Z) |
| Ubuntu 시나리오 보고서 | `오류`·`실패` 0건 | Ubuntu 24.04.4 LTS x64(WSL2), Rust 1.99.1 | 통과. `ALL PASSED`. 153건: 통과 140, 부분 7, 실패 0, 미실행 4, 미연결 2, 오류 0. 머리글의 `+ 미커밋 변경`은 줄 끝 차이뿐이다(`git -c core.autocrlf=true diff --exit-code` 종료 코드 0) | `~/vnc-relay-target/scenario-report.md`(2026-11-07T09:48:43Z) |

보고서에 `통과`가 아닌 상태로 남은 시나리오의 판정은 다음과 같다. 보고서 열은 Windows / Ubuntu 순서다.

| 시나리오 | 보고서 | 판정 | 근거 |
| --- | --- | --- | --- |
| DASH-015 | 부분 / 부분 | 통과 | 네 엔진의 실제 키 입력·로그인 Tab 순서·긴 값 확인([대시보드 검증 4.26절](verify-dashboard-frontend.md#426-작업-412-완료-판정-q144q146-2026-11-07), 사용자 결정 2026-11-07(Q145)). 저장소 시험 `session_cancel_sends_no_request_and_restores_focus_in_browser`는 부분 연결이다 |
| DASH-023 | 미연결 / 미연결 | 통과 | Chrome·Edge·Firefox 마우스 흐름 18회(Q146), Safari는 WebKit 대체 6회(Q135)([같은 절](verify-dashboard-frontend.md#426-작업-412-완료-판정-q144q146-2026-11-07)). Firefox의 페이지 오류는 모으지 않았다 |
| OPS-017 | 미실행 / 미실행 | 통과 | 무시된 시험의 실행 기록. 커밋 `509ca52`의 두 OS 정식 측정에서 세 전송 모두 통과했다(Q148 기준)([성능 측정 4절](measure-performance.md#4-검증-기록)) |
| OPS-018 | 미실행 / 미실행 | 통과 | 같은 정식 측정에서 두 OS·세 전송 모두 통과했다([성능 측정 4절](measure-performance.md#4-검증-기록)) |
| OPS-020 | 미연결 / 미연결 | 통과 | Windows VNC Relay와 실제 UltraVNC 1.8.3.0·noVNC 1.7.0의 아홉 조합 통과(Q128·Q130·Q131)([호환성 기록 6.3절](../contracts/repeater-compat-record.md#63-조합별-결과)). Ubuntu는 자동 시험을 증거로 인정했다(Q129)([6.6절](../contracts/repeater-compat-record.md#66-ubuntu-2404-증거)). 실제 UltraVNC와 Ubuntu VNC Relay의 조합은 미실행이다 |
| OPS-021 | 부분 / 부분 | **미판정(통과 아님)** | 출처·의존성 고지·대시보드 JS 의존성 검토는 마쳤다([라이선스 검사 4.1절](check-dependency-licenses.md#41-출처-검토-기록-2026-11-04)). 실제 MIT 저작권자·적용 연도로 LICENSE를 쓰는 일이 사용자 결정(Q97)을 기다린다 |
| OPS-023 | 부분 / 통과 | 통과 | Q99. Ubuntu에서 비부분 시험 `counts_connecting_and_connected_target_sockets_but_not_management_tls`가 통과했다. Windows는 큐가 찬 `listen()`이 WSAECONNREFUSED로 거절하므로 연결 중 소켓을 만들 수 없다(`tests/relay_target_scenarios.rs` 927~929행). Windows 부분 시험 `target_budget_denial_preserves_sessions_and_excludes_management_tls`는 통과했다([중계 수명 설계 7절 현재 시험](../architectures/relay-session.md#현재-시험)) |
| OPS-031 | 부분 / 부분 | 통과 | 두 OS release 묶음의 수동 기록과 자동 시험을 함께 검토했다(Q107)([운영 how-to 4절](operate-relay.md#4-검증)). LICENSE 미완료는 그 기록에 따로 표시했다 |
| OPS-040 | 부분 / 부분 | 통과 | 모든 조건을 부분 시험들이 나누어 확인한다. IPv6 경계는 단위 시험으로 인정했다(Q58)([파일 감지 설계](../architectures/file-watch.md#사용자-결정-반영-2026-10-23)) |
| OPS-044 | 통과 / 미실행 | 통과 | Windows 전용 시나리오다. Windows에서 비부분 시험 `console_close_forces_cleanup`이 통과했다([작업 5.3 완료 대조](../architectures/relay-session.md#작업-53-완료-대조2026-11-04)) |
| RLY-006 | 부분 / 부분 | 통과 | Windows VNC Relay와 실제 UltraVNC의 독립 2회 수동 통과, 소유자 캡처 검토(Q116)([수동 확인 4.1](verify-ultravnc-manually.md#41-2026-11-05-실행-기록)·[4.2](verify-ultravnc-manually.md#42-2026-11-06-실행-기록)·[4.3절](verify-ultravnc-manually.md#43-자동-증거-대조-32단계)). Ubuntu VNC Relay의 실제 UltraVNC 확인은 미실행이다. Ubuntu 쪽은 Q129처럼 자동 시험을 증거로 본다. 이 확장은 이 문서의 판단, 사용자 결정 없음이다 |
| RLY-009 | 부분 / 통과 | 통과 | Q99. Ubuntu에서 비부분 시험이 통과했다. Windows 이유는 OPS-023과 같다(C 시간 초과에 연결 중 소켓이 필요하다). Windows 부분 시험 `connection_refusal_is_reported_and_releases_both_socket_places`는 통과했다 |
| RLY-010 | 부분 / 부분 | 통과 | RLY-006과 같은 근거(Mode II, Server 먼저 독립 2회) |
| RLY-011 | 부분 / 부분 | 통과 | RLY-006과 같은 근거(Mode II, Viewer 먼저 독립 2회) |
| RLY-025 | 부분 / 통과 | 통과 | Q86·Q99. Ubuntu의 netns·nftables 비부분 시험이 통과했다. Windows에는 응답 없이 끊긴 네트워크를 root 없이 만들 수단이 없다([중계 수명 설계 bf27d30 기록](../architectures/relay-session.md#전체-게이트-실행-기록-커밋-bf27d30)) |
| RLY-026 | 부분 / 통과 | 통과 | RLY-025와 같은 근거 |
| WS-025 | 미실행 / 미실행 | 통과 | 무시된 시험의 실행 기록. 커밋 `d4c5678`에서 두 OS 모두 `--include-ignored`로 통과했다([WebSocket 계약 실행 기록](../contracts/websocket-relay-v1.md#계약-테스트-실행-기록)). 실제 VM의 noVNC 확인도 통과했다([호환성 기록 6.3절](../contracts/repeater-compat-record.md#63-조합별-결과)) |

PRD 10절의 AC-01~37을 같은 기준으로 대조했다. 각 AC 행이 링크한 시나리오 ID를 두 보고서와 위 표에 맞춰 보았다. AC-12를 뺀 36개는 연결된 시나리오가 모두 두 보고서에서 통과이거나 위 표에서 통과다. AC-12의 시나리오는 OPS-021 하나이므로 AC-12는 충족되지 않았다.

남은 일: Q97이 정해지면 LICENSE를 쓰고 OPS-021과 AC-12를 다시 판정한다. 그 뒤 스펙·계약의 상태와 지원 범위 문장을 실제 결과에 맞춰 갱신하고 작업 5.9를 닫는다.

## 5. 중단 조건

- 1단계 Build Tools 설치가 1602로 끝나면 이후 단계를 진행하지 않는다. UAC를 승인할 수 있을 때 다시 실행한다.
- 3단계 버전이 `rustc 1.99.1`이 아니면 진행하지 않는다. `rust-toolchain.toml`과 `rustup show active-toolchain` 출력을 확인한다.
- 6단계 명령이 하나라도 실패하면 완료로 보고하지 않는다. 실패한 명령의 출력을 고친 뒤 6단계를 처음부터 다시 실행한다.

## 6. 롤백 또는 복구

1. 빌드 결과가 꼬였다고 의심되면 `cargo clean`으로 Cargo 대상 디렉터리를 지운다. 소스는 바뀌지 않는다.
2. 4단계를 다시 실행해 `vnc-relay 0.1.0`이 나오는지 확인한다.

## 7. 해결되지 않을 때 전달할 대상

- 전달 대상: vnc-relay 프로젝트 관리자
- 보존할 증거: 실패한 명령과 전체 출력, OS와 `rustc --version` 출력, 대상 커밋, `scenario-report.md`
- 비밀 값 제외 규칙: 검증 명령은 비밀 값을 다루지 않는다. 환경 변수 전체를 붙여 넣지 않는다.
