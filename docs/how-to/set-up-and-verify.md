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
| 관리자 권한 | Build Tools 설치에 UAC 승인이 필요하다. | Rust 설치에는 필요 없다. `cc`가 없을 때 `apt` 설치에 `sudo`가 필요하다. |

## 2. 전제 조건

- [ ] 운영체제가 Windows 11 x64 또는 Ubuntu 24.04 LTS x64다.
- [ ] git이 설치되어 있고 저장소를 내려받았다.
- [ ] 인터넷에 연결되어 있다. rustup과 Cargo가 툴체인과 crate를 내려받는다.
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
   | 문서만 | 이 절차의 명령은 필요 없다. 문서 유형별 작성 규칙과 링크를 확인한다. | — |

   한 crate만 빠르게 확인할 때는 `cargo test --package <패키지 이름>`을 쓸 수 있다. 완료 보고 전에는 6단계를 실행해야 한다.

### 3.4 전체 완료 검증

6. 저장소 루트에서 다음 명령을 순서대로 실행한다. 하나라도 실패하면 멈추고 [5절](#5-중단-조건)을 따른다.

   ```shell
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo xtask layers
   cargo xtask scenarios
   cargo deny check licenses
   ```

   기대 결과: 다섯 명령 모두 종료 코드 0. `cargo xtask scenarios`가 `cargo test --workspace --no-fail-fast`를 함께 실행하므로 따로 실행하지 않는다. 출력 마지막에 `판정: 통과`, 라이선스 검사 마지막 줄에 `licenses ok`가 나온다. 단계를 마칠 때(작업 5.9)는 `cargo xtask scenarios --require-all`로 바꿔 실행한다.

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

실행 기록 1: 커밋 `b7d23ee`에서 실행했다. Windows 쪽 `git status`는 변경 없음이었다. 환경은 Ubuntu 24.04.4 LTS x86_64(WSL2), rustup 1.29.1, cargo 1.99.1, cargo-deny 0.20.2, `CARGO_TARGET_DIR=$HOME/vnc-relay-target`이다. 3·4·6단계의 명령은 모두 종료 코드 0이었다.

실행 기록 2: 2026-10-20 Windows 11 x64에서 1단계 winget 명령을 처음 실행했다. UAC를 승인하지 않아 종료 코드는 1602였다. rustup은 `%USERPROFILE%\.cargo\bin`에 설치되어 있지만 사용자 PATH에 없다. 이 상태의 `cargo build`는 `` `link.exe` returned an unexpected error ``로 실패했다.

실행 기록 3: 2026-10-20 같은 Windows 11 Pro 10.0.26200 x64에서 UAC를 승인해 1단계 설치를 마쳤다. 커밋 `e1cfc4f`에 Windows 경고 수정(`crates/adapters/src/net.rs` 테스트 모듈을 Linux 전용으로, `tests/serve.rs`의 `lifecycle_events`를 Unix 전용으로)을 더한 작업 트리에서 실행했다. PowerShell에서 `PATH`에 `%USERPROFILE%\.cargo\bin`을 더하고 `AWS_LC_SYS_PREBUILT_NASM=1`을 정했다. Cargo 대상 디렉터리는 저장소의 `target\`이다. 3·4·6단계의 명령은 모두 종료 코드 0이었다. 수정 전 clippy 오류는 `unused import: super::*`(`crates\adapters\src\net.rs`)와 `function lifecycle_events is never used`(`tests\serve.rs`)였다. 같은 작업 트리에서 Ubuntu 6단계도 다시 통과했다. 테스트별 결과는 [운영 정보 계약 실행 기록](../contracts/ops-info-v1.md#계약-테스트-실행-기록)에 있다.

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
