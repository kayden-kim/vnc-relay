# Rust 코드 컨벤션

공통 표준: project-docs `docs/conventions/rust-naming-standard.md` (2026-11-24 확정, [원문](https://github.com/kayden-kim/project-docs/blob/main/docs/conventions/rust-naming-standard.md)). 7절의 이름 규칙은 이 표준을 옮긴 것이므로 원본이 바뀌면 이 문서도 같은 변경에서 고쳐야 한다.

같은 개념은 어느 crate에서나 같은 이름으로 쓰고, 서식과 위험한 코드 패턴은 사람이 아니라 도구가 판정하게 한다. 이름은 [용어집](../../GLOSSARY.md)의 코드 영어 이름에서 시작하고, 모듈은 [코드 지도](../architectures/code-map.md)의 레이어와 위치를 따른다. 이름 규칙의 본문은 [7절](#7-이름-짓기-규칙과-목록)에 모두 적었다. 이 저장소에서 작업하는 사람은 이 문서만 읽고 타입·trait·모듈·crate 이름을 지을 수 있어야 한다. 서식은 rustfmt 기본값, 금지 패턴은 작업 공간 lint로 막는다.

| 항목 | 값 |
| --- | --- |
| 상태 | Active |
| 적용 대상 | 사람이 작성한 Rust 소스 코드와 테스트 코드, `Cargo.toml`, `clippy.toml` |
| 기준 버전 | Rust 1.99.1(`rust-toolchain.toml`), edition 2024, [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/naming.html), 이름 규칙은 위 공통 표준(2026-11-24 확정) |
| 관련 아키텍처 문서 | [코드 지도](../architectures/code-map.md) |
| 관련 ADR | [ADR-0004 tokio·rustls](../decisions/ADR-0004-tokio-rustls.md) |
| 적용 시작일 | 2026-11-24(7절 이름 규칙) |
| 규칙 준수 확인 방법 | `cargo fmt`, `cargo clippy`(작업 공간 lint), `cargo xtask layers`, 코드 리뷰. 이름 규칙은 코드 리뷰로 판정한다([5절](#5-검증-명령과-통과-기준)) |

## 1. 규칙 우선순위

[컨벤션 작성 규칙](../conventions/AGENTS.md#규칙-우선순위)을 따른다. 이 문서는 컴파일러, rustfmt·clippy, Rust API Guidelines가 정하지 않았거나 선택지를 남긴 부분만 정한다. 이름 규칙은 공통 표준과 같다. 표준과 다른 규칙은 이 문서에 쓰지 않는다. 표준이 허용한 예외만 [4절](#4-예외와-자동-생성-산출물)과 [7.9절](#79-등록-suffix와-예외-이름)에 이유와 함께 등록한다.

## 2. 규칙

| 대상 | 해야 할 것 | 피해야 할 것 | 판정 방법 |
| --- | --- | --- | --- |
| 서식 | rustfmt 기본값을 쓴다. 설정 파일 `rustfmt.toml`을 두지 않는다. | 손으로 맞춘 정렬, rustfmt 설정 추가 | `cargo fmt --all --check` |
| 대소문자 | 타입·trait·enum 값은 `UpperCamelCase`, 함수·변수·모듈·파일은 `snake_case`, 상수는 `SCREAMING_SNAKE_CASE`. crate 이름은 `kebab-case`이며 [7.5절](#75-모듈-파일-crate-이름)을 따른다. | `relayID`, `Relay_Id` | 컴파일러 경고(`non_camel_case_types` 등), 리뷰 |
| 업무 이름 | 업무 개념의 타입·모듈은 용어집 코드 영어 이름으로 시작한다([7.10절](#710-도메인-prefix) prefix 목록). 용어집에 없는 업무 개념이면 먼저 용어집에 등록한다. | 용어집과 다른 동의어(`MatchId`, `Token`으로 중계 ID를 부름) | 리뷰 |
| 역할 suffix | 역할 타입은 `<개념 prefix><역할 suffix>`로 끝낸다. suffix는 [7.8절](#78-suffix-목록)의 닫힌 목록과 [7.9절](#79-등록-suffix와-예외-이름)의 등록 suffix에서 고른다. 개념 타입(업무 값·enum·데이터)은 suffix 없이 업무 이름만 쓴다. 포트 trait은 `Source` `Sink` `Store` `Gateway` `Control` `Clock` 가운데 하나로 끝낸다. | [7.7절](#77-금지-이름과-부딪히는-지점)의 금지 이름(`Manager`, `Handler`, `UseCase`, `Interactor`, `Repository`, `Dto`, `Impl` 등), 목록에 없는 suffix, 경로와 suffix에 역할을 두 번 쓰기 | 리뷰 |
| 테스트 전용 타입 | `#[cfg(test)]` 모듈과 `tests/`에만 있는 타입에는 suffix 규칙을 적용하지 않는다. 시험 대역 이름은 `Fake`, `Stub`, `Spy` 중 하나로 시작한다. | 운영 코드에서 `Fake`·`Stub`·`Spy` 이름을 쓰는 것 | 리뷰 |
| 모듈 파일 | 하위 모듈이 있으면 `foo.rs`와 `foo/` 디렉터리를 쓴다. 시험 코드도 같다. | `foo/mod.rs`, `#[path]` | clippy `mod_module_files`(deny)는 `mod.rs`를 막는다. `#[path]`는 리뷰 |
| 모듈 위치 | 코드 지도의 레이어와 위치에 둔다. 새 모듈이 생기면 같은 변경에서 코드 지도를 고친다. | domain·app에서 소켓·파일·TLS·HTTP 라이브러리 사용 | `cargo xtask layers`, `crates/app/clippy.toml`, 리뷰 |
| 주석 언어 | 주석과 문서 주석은 영어로 쓴다. 용어는 용어집의 코드 영어 이름을 쓴다. | 한국어 주석, 용어집과 다른 영어 표현 | 리뷰 |
| 문서 주석 | domain·app의 공개 항목에는 `///`로 무엇을 보장하는지 쓴다. 근거가 되는 스펙·계약 절을 적는다(예: `/// See cli-v1 §1.2.`). | 코드를 다시 읽어 주는 주석 | 리뷰 |
| 오류 | 오류 enum은 `<개념>Error`로 짓고 `std::fmt::Display`와 `std::error::Error`를 직접 구현한다. 표준 trait은 `impl std::error::Error for TlsError`처럼 경로로 써서 별칭이 필요 없게 한다. 계약의 오류 코드가 있으면 enum 값에서 그 코드 문자열을 얻는 함수를 둔다. | 단독 `Error`, `Failure`로 끝나는 오류 이름, PRD 목록에 없는 오류 crate(`anyhow`, `thiserror`) 직접 의존, 문자열 오류 | 리뷰, [의존성 라이선스 검사](../how-to/check-dependency-licenses.md) |
| 실패 처리 | 테스트 밖 코드에서 `unwrap`·`expect`를 쓰지 않는다. 오류를 반환하거나 연결 하나만 정리한다. | 연결 오류로 프로세스가 끝나는 코드 | clippy `unwrap_used`·`expect_used`(deny, 테스트 허용) |
| 출력 | 명령행 출력은 실행파일 조립 crate에서 `std::io::Write`로 쓴다. 운영 로그는 adapters의 운영 로그 모듈만 쓴다. | `println!`·`eprintln!`·`dbg!` | clippy `print_stdout`·`print_stderr`·`dbg_macro`(deny) |
| 비밀 값 | 비밀번호·해시·쿠키 값·CSRF 토큰·개인키를 담는 타입은 `Debug`를 직접 구현해 값을 가린다(`"<redacted>"`). | 비밀 값 타입에 `#[derive(Debug)]` | 리뷰, 운영 정보 계약 1.11절 계약 테스트 |
| unsafe | `unsafe` 코드를 쓰지 않는다. 예외는 아래 여섯 파일·모듈뿐이다. | `unsafe` 블록(예외 파일·모듈 밖), crate 전체 `allow` | rustc `unsafe_code`(deny). 각 예외 모듈 선언에 이유 있는 `#[expect(unsafe_code, reason = "...")]`를 쓰고, 호출마다 `SAFETY` 주석으로 안전 조건을 설명한다. |
| 미완성 표시 | 끝나지 않은 코드를 병합하지 않는다. | `todo!()`, `unimplemented!()` | clippy `todo`·`unimplemented`(deny) |
| lint 예외 | 꼭 필요하면 가장 좁은 범위에 `#[expect(lint, reason = "...")]`를 쓴다. | crate 전체 `#![allow(...)]`, 이유 없는 `allow` | clippy `allow_attributes_without_reason`(deny), 리뷰 |
| 의존성 | [PRD "구현 기술" 행](../requirements/PRD.md#확정-기준)의 crate만 쓴다. 버전은 작업 공간 `[workspace.dependencies]`에 한 번만 적는다. | crate마다 다른 버전, 목록 밖 crate | `cargo xtask layers`, `cargo deny check licenses` |

### 2.1 `unsafe` 예외

| 허용 파일·모듈 | 이유와 범위 | 예외 선언 위치·근거 |
| --- | --- | --- |
| `crates/adapters/src/terminal/windows_console.rs` | Win32 콘솔 모드를 읽고 바꾸며 복구한다. | `crates/adapters/src/terminal.rs` |
| `crates/adapters/src/windows_service/security.rs` | Windows 서비스 운영자 이름을 계정 SID로 바꾸고 서비스 DACL에 시작·중지·상태 조회·구성 조회 권한을 더한다. windows-service에는 이 기능이 없다. | `crates/adapters/src/windows_service.rs`의 `mod security`. reason은 `Win32 account lookup and service DACL change; see docs/conventions/rust.md`다. [서비스 모듈 설계](../architectures/windows-service.md) |
| `crates/tray/src/message_loop.rs` | tray-icon이 Windows에서 요구하는 같은 스레드의 Win32 메시지 루프를 돌린다. 다른 트레이 기능은 직접 `unsafe`를 쓰지 않는다. | `crates/tray/src/main.rs`의 `mod message_loop`. reason은 `Win32 message loop that tray-icon needs; see docs/conventions/rust.md`다. [트레이 모듈 설계](../architectures/windows-tray.md) |
| `crates/adapters/src/process_stats/windows.rs` | 현재 프로세스의 CPU 시간·작업 집합·전용 메모리·핸들 수·스레드 수를 읽는다(`GetProcessTimes`, `K32GetProcessMemoryInfo`, `GetProcessHandleCount`, Toolhelp32 스레드 목록). 표준 라이브러리에는 이 값을 읽는 API가 없다. 읽기만 하며 프로세스 상태를 바꾸지 않는다. | `crates/adapters/src/process_stats.rs`의 `mod windows`. reason은 `Win32 process statistics for metrics; see docs/conventions/rust.md`다. [메트릭 모듈 설계](../architectures/metrics.md) |
| `tests/windows_console.rs`의 의사 콘솔 모듈 | 실제 실행파일을 ConPTY에서 띄우고 제어 신호·숨김 입력·콘솔 닫기를 시험한다. 표준 라이브러리에는 다른 프로세스의 콘솔을 만들고 제어하는 API가 없다. | 같은 파일의 모듈 선언 |
| `tests/performance.rs`의 Windows 핸들 표본 모듈 | 중계 프로세스의 핸들 목록에서 Event 핸들을 따로 센다. 표준 라이브러리와 `Get-Process`는 종류별 핸들 수를 주지 않는다. | 같은 파일의 모듈 선언 |

Win32 호출은 작업 공간의 `windows-sys`를 쓴다. 시험 쪽은 개발 의존성이다. 허용 파일을 넓힐 때는 컨벤션과 작업 공간 `Cargo.toml`의 lint 설명을 함께 바꾼다.

## 3. 예제

```text
좋음: crates/domain/src/matching.rs 의 WaitingTable
나쁨: crates/domain/src/matcher/mod.rs 의 MatchManager
이유: 용어집 이름 "대기 연결(waiting)"과 값 모양 단어 Table을 쓴다. 개념 타입이라 역할 suffix를 더 붙이지 않는다. mod.rs와 금지 이름 Manager를 쓰지 않는다.
```

```text
좋음: crates/app/src/ports.rs 의 trait AccountStore, crates/adapters/src/accounts_file/store.rs 의 AccountsFile
나쁨: crates/app/src/ports.rs 의 trait TomlAccountRepository
이유: app의 포트 이름은 기술(TOML)을 드러내지 않고, 메모리 밖 상태를 저장하는 포트는 Store로 끝낸다. Repository는 쓰지 않는다. 구현체 AccountsFile은 계정 파일이라는 업무 이름을 유지하는 등록 예외다(7.9절).
```

```text
좋음: crates/app/src/ports.rs 의 trait TargetGateway, crates/adapters/src/target.rs 의 TcpTargetGateway
나쁨: crates/app/src/ports.rs 의 trait TargetConnector
이유: 대상 호스트에 연결하는 포트는 Store·Source·Sink·Control·Clock에 맞지 않아 Gateway로 끝낸다. 구현체는 <기술><포트 이름>이다. 동사에서 만든 이름(Connector, Hasher, Loader)은 포트 어휘 밖이다.
```

```text
좋음: crates/app/src/ports.rs 의 trait IpDbFileGateway, crates/adapters/src/ip_db.rs 의 MmdbFileGateway
나쁨: crates/adapters/src/ip_db.rs 의 MmdbIpDbFileGateway
이유: 포트 하나만 구현하는 구현체는 <기술><포트 이름>이다. 기술 이름(Mmdb)이 IP DB 형식을 이미 말하면 포트 이름 앞의 IpDb를 빼서 5단어 이상이 되지 않게 한다.
```

```text
좋음: crates/app/src/settings.rs 의 ConfigError, crates/adapters/src/tls.rs 의 TlsError
나쁨: AdminFailure, 단독 Error
이유: 오류 타입은 항상 <개념>Error다. Failure로 끝나는 오류 이름을 쓰지 않는다. 단독 Error는 std::error::Error와 겹친다.
```

```text
좋음: crates/app/src/reload/service.rs 의 ReloadOutcome
나쁨: ReloadResult
이유: alias가 아닌 타입에는 Result suffix를 쓰지 않는다. fmt::Result·io::Result처럼 독자가 alias로 읽기 때문이다. 분류는 Outcome, 묶음은 개념 이름으로 짓는다.
```

```text
좋음: crates/app/src/settings.rs 의 RelaySettings(검증을 마친 설정 모델), crates/app/src/relay.rs 의 RelayRuntimeConfig(생성 옵션)
나쁨: RelaySettingsDto, 공개 단독 Config
이유: Settings는 검증을 마친 설정 모델이고 Config는 생성 옵션이다. 둘을 한 단어로 합치지 않는다. Dto는 쓰지 않는다.
```

```text
좋음: crates/app/src/management.rs 의 SessionView(app이 내주는 읽기 전용 출력), crates/adapters/src/ops_log.rs 의 LogsResponse(adapters가 만드는 JSON 응답)
나쁨: SessionDto, LogsDto
이유: Dto는 입력과 출력을 모두 덮어 역할을 말하지 않는다. 방향에 따라 Command·Query·View·Request·Response를 쓴다.
```

```text
좋음: crates/domain/src/session.rs 의 session::State(모듈 경로 session이 맥락을 준다)
나쁨: session::SessionState(개념 타입이 모듈 이름을 되풀이한다), session::SessionStateEntity
이유: 개념 타입은 모듈 경로가 맥락을 주므로 suffix도, 모듈 이름 되풀이도 필요 없다.
```

```rust
// Good: secret value hides its content in Debug output.
pub struct PasswordHash(String);

impl std::fmt::Debug for PasswordHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PasswordHash(<redacted>)")
    }
}
```

## 4. 예외와 자동 생성 산출물

- 자동 생성 Rust 코드는 없다. 생기면 생성 위치와 생성 명령을 이 절에 적고 rustfmt·clippy 대상에서 제외하는 방법을 정한다.
- 외부 계약이 정한 이름은 계약을 따른다. 예: 운영 로그 필드 `relay_id_ref`, 쿠키 이름 `relay_admin_session`, 설정 계약 [2.1절](../contracts/config-v4.md#21-설정-오류와-준비-실패)·[2.2절](../contracts/config-v4.md#22-경고)의 진단·경고(`ConfigDiagnostic`, `ConfigWarning`). 이런 이름을 담는 Rust 필드는 계약 이름과 같게 쓴다.
- `xtask/`는 개발 도구다. 7.3절 A·7.4절의 계층별 suffix 규칙을 적용하지 않는다. 대소문자·모듈·파일·crate 규칙, 7.7절의 금지 이름, 서식·lint 규칙은 적용한다. 도구 API의 역할명과 일반 자료 이름(`State` `Kind` `Record` `Entry`)은 문맥에 맞으면 유지할 수 있다.
- `#[cfg(test)]` 모듈과 `tests/`에만 있는 타입에는 suffix 규칙을 적용하지 않는다. 시험 대역은 `Fake` `Stub` `Spy` 중 하나로 시작한다.
- 이 문서에 이유와 함께 등록한 이름([7.9절](#79-등록-suffix와-예외-이름))은 suffix 규칙보다 우선한다. 새 예외는 같은 변경에서 7.9절에 등록한다. 이 문서에 등록하지 않은 이름은 규칙을 따라야 한다.
- lint 예외는 2절 "lint 예외" 행을 따른다. 예외는 코드에 이유와 함께 남기며 별도 목록을 두지 않는다.

## 5. 검증 명령과 통과 기준

- 실행 환경: Ubuntu 24.04 LTS의 bash 또는 Windows 11의 PowerShell, Rust 1.99.1
- 실행 위치: 저장소 루트
- 성공 조건: 세 명령 모두 종료 코드 0. `cargo clippy`는 경고가 없어야 한다.

```shell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask layers
```

`cargo xtask layers`는 패키지마다 허용한 내부 crate·외부 crate만 쓰는지, 같은 외부 crate의 버전 요구가 하나인지 검사한다([xtask 모듈 설계](../architectures/xtask.md)). 전체 검증 절차는 [개발 환경·검증 how-to](../how-to/set-up-and-verify.md)가 정한다.

이름 규칙에는 자동 검사가 없다. `mod.rs` 금지만 clippy `mod_module_files`가 검사한다. 리뷰어가 [7절](#7-이름-짓기-규칙과-목록)의 표로 판정한다. 성공 조건은 새 타입·trait·모듈·crate 이름이 7절에 맞고 예외가 [7.9절](#79-등록-suffix와-예외-이름)에 등록되어 있는 것이다. `clippy::module_name_repetitions`는 켜지 않는다([7.3절](#73-suffix가-필요한-조건) D).

## 6. 기존 코드 마이그레이션

2026-11-24에 7절과 다르던 기존 이름과 시험 코드 구조를 모두 고쳤다. 이 절은 바꾼 내용과 바꾸지 않은 예외를 기록한다. 새 코드는 이 절이 아니라 7절을 따른다. 바꾼 이름은 import한 곳과 코드 지도·도메인 모델·모듈 설계의 심볼을 같은 변경에서 고쳤다.

| 규칙 | 바꾼 내용 |
| --- | --- |
| 오류 이름: `Failure` → `<개념>Error`([7.7절](#77-금지-이름과-부딪히는-지점)) | `AdminFailure`→`AdminError`, `ListenerFailure`→`ListenerError`, `BindFailure`→`BindError`, `WatchFailure`→`WatchError`, `HandshakeFailure`(`crates/adapters/src/tls_listener.rs`)→`TlsHandshakeError`, `IpDbFailure`→`IpDbError`. 원래 `IpDbError`였던 감싸는 구조체는 `IpDbReadError`로 바꿨다. 비공개 `Failure`는 `crates/app/src/relay/target.rs`에서 `TargetConnectError`, `src/service.rs`에서 `ServiceError`가 되었다 |
| `Result` suffix → `Outcome` 또는 개념 이름 | `ReloadResult`→`LastReloadOutcome`(`ReloadOutcome`이 이미 있어서 구분했다), `AdminTerminateResult`→`AdminTerminateOutcome`, `ConfigEditResult`→`ConfigEditOutcome` |
| 같은 이름 충돌([7.3절](#73-suffix가-필요한-조건) A2) | 도메인 enum `entry_point::Listener`→`ListenerKind`(adapters trait `Listener`는 그대로). `target_policy::TargetHost`→`TargetItemHost`(`handshake::TargetHost`는 그대로) |
| `Control`이 포트가 아닌 타입 | `IdleControl`→`IdleTimerHandle`, `StopControl`→`StopRequest`, 비공개 `SessionControl`(`crates/app/src/relay.rs`)→`SessionHandle` |
| crate 이름: `vnc-relay-app` → `vnc-relay-application`([7.5절](#75-모듈-파일-crate-이름)) | 패키지 이름과 `vnc_relay_application` import, `Cargo.toml`, `xtask/src/layers.rs`, 배포 고지, 문서를 함께 바꿨다. 디렉터리 `crates/app`은 그대로다 |
| 개념 타입의 모듈 이름 되풀이([7.3절](#73-suffix가-필요한-조건) C) | `account::AccountName`→`Name`, `admin_session::{AdminSessionToken→Token, AdminSessionPolicy→Policy, AdminSessionTable→Table}`, `budget::BudgetPermit`→`Permit`, `handshake::HandshakePolicy`→`Policy`, `login_limit::LoginLimitPolicy`→`Policy`, `recent_closed::RecentClosedList`→`List`, `session::{SessionId→Id, SessionState→State}`, `ticket::{TicketParts→Parts, TicketClaims→Claims, TicketPolicy→Policy, TicketKey→Key, TicketKeyList→KeyList, TicketUseTable→UseTable}`. 이름이 겹치는 `Policy`는 부모 모듈을 붙여 쓴다(`handshake::Policy`). `crates/app`·`crates/adapters`도 같은 규칙을 적용했다. `accounts::{AccountsDiagnostic→Diagnostic, AccountsWarning→Warning}`, `relay::{RelayCounts→Counts, RelayState→State, RelayEndpoint→Endpoint}`(뒤의 둘은 비공개), `reload::ReloadTrigger`→`Trigger`, `reload::snapshot::ReloadCell`→`Cell`, `metrics::history::{HistoryWindow→Window, HistoryPoint→Point}`, `sources::SourcesReport`→`Report`, `tls::TlsFile`→`File`. 표준 라이브러리나 도메인 이름과 겹치는 것은 부모 모듈을 붙여 쓴다(`tls::File`, `snapshot::Cell`). `crates/app/src/relay.rs`는 도메인 `session::State`를 `vnc_relay_domain::session::State`로 쓴다. 이 파일에 하위 모듈 `session`이 있어서 부모 모듈 import로 풀 수 없다 |
| 함수 직접 import([7.7절](#77-금지-이름과-부딪히는-지점)) | 작업 공간·표준·외부 crate의 함수 import를 모두 부모 모듈 경유 호출로 바꿨다(`toml_text::error_position(..)`). 매크로(`json!`, `pin!`, `ready!`)와 모듈 import는 대상이 아니다 |
| 시험 대역 이름([2절](#2-규칙)) | 포트·소켓·스트림을 대신하는 시험 전용 타입을 모두 `Fake`·`Stub`·`Spy`로 시작하게 바꿨다(`TestClock`→`FakeClock`, `TestStream`→`FakeRelayStream`, `Recorder`→`SpyEventSink`, `FixedProcess`→`StubProcessStatsSource`). 대역이 아닌 보조 타입(`TestCase`, `TestResult`, `PartialWrites`, `FrameGate` 등)은 그대로다 |
| 시험 코드의 `#[path]`와 `mod.rs`([2절](#2-규칙)) | `#[path]` 속성 43개를 모두 없앴다. 여러 시험 대상이 함께 쓰던 `tests/common/`과 `crates/adapters/tests/common/`의 도구는 작업 공간 crate `vnc-relay-testkit`(`crates/testkit`)의 모듈로 옮겼다. `tests/dashboard_live.rs`와 `tests/ticket_system.rs`는 `tests/<주제>/main.rs`와 하위 모듈로 나눴다. `src/serve.rs`의 불필요한 `#[path]`는 지웠다 |

- 금지 이름(`Dto` `Repository` `UseCase` `Interactor` `Manager` `Impl` `Handler` 등)을 쓰는 운영 코드 선언은 0개다. 시험 코드와 `xtask/`는 이 점검에서 뺐다. 시험 전용 타입은 suffix 규칙 대상이 아니다([2절](#2-규칙)).
- 코드를 바꾸지 않고 이 문서가 허용하도록 고친 것: adapters의 `Policy`(`AcceptPolicy`, `WsPolicy`)와 비공개 `Snapshot`(`QuerySnapshot`), 닫힌 목록 밖 suffix(`Guard` `Parser` 등, [7.9절](#79-등록-suffix와-예외-이름)).
- 비공개 `Wire`(`crates/adapters/src/websocket.rs`)는 소켓을 감싸는 enum이다. 어댑터 내부 저장·전송 형식을 뜻하는 `Wire` suffix와 뜻이 다르다. 모듈 안 비공개 보조 타입이므로 바꾸지 않았다.
- `LogsFilter`는 조회 조건 입력이다. 표준은 `Query`로 바꾸기를 필요할 때로 둔다. 바꾸지 않았다.
- 7.3절 C의 예외로 남긴 이름: 7.7절이 업무 이름으로 정한 `Config…`(`ConfigView`, `ConfigPatch`, `ConfigItem`, `ConfigChange`, `ConfigValue`, `ConfigActor`, `ConfigReloadKind`), 7.9절의 `RelayRuntime`, 등록 suffix를 쓰는 `FrameDecoder`다. 역할 suffix가 붙은 이름(`RelayService`, `ReloadSnapshot` 등)은 개념 타입이 아니라서 대상이 아니다.
- `management::config`가 다시 내보내는 `config_items`는 공개 경로라서 그대로 두었다. 호출하는 쪽은 `management::config::config_items(..)`로 부른다.
- `crates/app/src/relay/session.rs`의 시험용 `Control`은 대역이 흉내 내는 스트림을 조작하는 시험 쪽 핸들이라 바꾸지 않았다.

## 7. 이름 짓기 규칙과 목록

공통 표준의 규칙을 이 저장소의 crate·모듈에 맞춰 옮겼다. 7.1~7.7절이 규칙이고 7.8~7.10절이 목록이다. 표의 "이 저장소의 예"는 현재 코드의 이름이다.

### 7.1 원칙

1. 타입은 두 부류로 나눈다. **개념 타입**은 업무 개념 자체다(`AllowList`, `CloseCause`, `Role`). **역할 타입**은 계층 경계에서 일을 맡는 타입이다(`RelayService`, `EventSink`, `TlsListener`).
2. 개념 타입은 모듈 경로가 맥락을 준다. 이름에 역할 suffix를 붙이지 않는다.
3. 역할 타입은 이름이 **개념 prefix + 역할 suffix**로 스스로 뜻을 말한다. 경로를 보지 않아도 읽혀야 한다.
4. 역할 suffix는 닫힌 목록([7.8절](#78-suffix-목록))에서 고른다. 목록 밖 단어는 이유를 적어 [7.9절](#79-등록-suffix와-예외-이름)에 등록한 뒤 쓴다. 같은 역할에 단어를 둘 쓰지 않고, 같은 단어에 역할을 둘 주지 않는다.
5. 역할을 경로와 suffix에 두 번 쓰지 않는다. 모듈 이름이 역할 단어(`repository`, `service`, `dto`)이면 타입에 같은 suffix를 붙이지 않는다.
6. 이름은 5단어 미만으로 짓는다. 개념 prefix + 역할 suffix가 4단어를 넘으면 개념 이름을 다시 정한다. 약어로 자르지 않는다(약어는 한 단어다. 예: `Uuid`). `TerminalAccountInputGateway`는 4단어이므로 허용 범위의 끝이다.

### 7.2 역할별 이름 표현

방식 기호: **P**는 경로만, **S**는 suffix(prefix 포함)만, **PS**는 둘 다다. PS(`repository::OrderRepository`)는 어떤 역할에도 쓰지 않는다.

| 역할 | 방식 | 이름 형태 | 이 저장소의 좋은 예 | 나쁜 예 |
| --- | --- | --- | --- | --- |
| 도메인 개념(엔티티·값·enum) | P | `<개념>` | `AllowList`, `CloseCause`, `Role` | `AllowListModel`, `CloseCauseEntity` |
| 값 모양 단어 | P(개념 이름의 일부) | `<개념>Id` `Key` `Kind` `State` `Policy` `Table` `List` | `RelayId`, `RelayIdKey`, `TargetPolicy`, `WaitingTable` | `RelayIdentifier` |
| 이터레이터형 보조 타입 | P | `Iter` `IntoIter` | 해당 없음(현재 0개). 예: `sessions::Iter` | `SessionsIterator` |
| 오류 | S | `<개념>Error` | `TlsError`, `ConfigError` | 단독 `Error`, `TlsErr`, `AdminFailure` |
| 유스케이스 입구 | S | `<개념>Service` | `RelayService`, `ReloadService` | `Service`, `RelayUseCase`, `RelayManager` |
| 포트 trait | S | `<개념><포트 역할>` | `EventSink`, `AccountStore`, `TokenSource`, `TargetGateway` | `EventPort`, `IEventSink`, `EventSinkTrait` |
| 어댑터 구현체 | S | `<기술><포트 이름>` | `JsonLinesEventSink`, `TcpTargetGateway`, `SystemClock` | `EventSinkImpl`, `DefaultEventSink` |
| 생성 옵션 | S | `<대상>Config` | `RelayRuntimeConfig` | 공개 단독 `Config` |
| 검증을 마친 설정 모델 | S | `Settings`, `<부분>Settings` | `RelaySettings` | `RelaySettingsDto` |
| 경계 입력 | S | `<동작><개념>Command`, `<개념>Query` | `SessionCommand` | `SessionCommandDto` |
| 경계 출력(읽기 전용) | S | `<개념>View` | `SessionView`, `SummaryView` | `SessionDto` |
| 어댑터가 소유한 요청·응답 | S | `<개념>Request` `<개념>Response` | `LogsResponse` | `LogsDto` |
| 어댑터 내부 저장·전송 형식 | S | `<개념>Wire` | 해당 없음(현재 0개) | 공개 `Wire` |
| 사건 | S | `<개념>Event` | `SourceEvent` | `SourceMessage` |
| 시점 고정 상태 | S | `<개념>Snapshot` | `RelaySnapshot`, `MetricsSnapshot` | `RelayStateCopy` |
| 빌더 | S | `<대상>Builder`. 더 나은 이름이 있으면 그 이름 | 해당 없음(현재 0개) | `ServerFactory` |
| 확장 trait | S | `<대상>Ext` | 해당 없음(현재 0개) | `StreamHelper` |
| 핸들·클라이언트 | S | `<대상>Handle`, `<프로토콜>Client` | 해당 없음(현재 0개) | `JwksManager` |
| 시험 대역 | prefix | `Fake` `Stub` `Spy` + 포트 이름 | `FakeClock`(시험 코드) | `MockClockImpl` |
| 모듈 안 비공개 보조 타입 | P | 짧은 일반 이름 | `State`, `Shared`, `Inner` | `RecentLogsInnerSharedState` |

### 7.3 suffix가 필요한 조건

이 절의 S 방식과 P 방식은 타입의 이름 표현이다. 개념 타입(엔티티·값·enum)은 [7.2절](#72-역할별-이름-표현)에 따라 항상 P 방식이며, 아래 A 조건이 있어도 역할 suffix를 붙이지 않는다. 개념 타입이 A1·A2에 걸리면 개념 이름을 구분하거나(`Listener` → `ListenerKind`) 부모 모듈을 import해서 푼다([7.7절](#77-금지-이름과-부딪히는-지점)).

**A. 역할 타입은 아래 중 하나라도 해당하면 S 방식을 쓴다(suffix 필수).**

1. 타입이 자기 모듈 밖으로 나간다. `pub use`로 올리거나, 다른 crate가 직접 import한다(조립 코드 포함).
2. 같은 스코프에서 같은 이름의 다른 타입과 함께 import될 수 있다. 다른 모듈의 같은 역할 타입, 표준 라이브러리 또는 외부 crate의 동명 타입이 포함된다. 이 저장소의 사례는 도메인 enum `ListenerKind`와 adapters trait `Listener`, 도메인 `handshake::TargetHost`와 `target_policy::TargetItemHost`다. 개념 타입은 suffix 대신 이름 자체를 구분한다([6절](#6-기존-코드-마이그레이션)).
3. trait과 구현체가 같은 이름이 된다. 구현체에 기술·구현 성격 prefix를 붙인다(`Clock` → `SystemClock`).
4. 계층 경계를 넘는다. 포트, 서비스, 경계 데이터, 오류는 읽는 쪽이 계층 모듈 경로를 보지 않아도 역할을 알아야 한다.

**B. 아래를 모두 만족하면 P 방식을 쓸 수 있다(suffix 없음).**

1. 업무 개념이거나, 한 모듈 안에서만 쓰는 타입이다. 비공개이거나 `pub(super)`·`pub(crate)` 보조 타입이다.
2. 같은 스코프에서 같은 이름과 함께 import될 가능성이 없다.
3. 모듈 밖으로 `pub use`하지 않는다.

**C. 경로로 표현하는 개념 타입은 모듈 이름을 되풀이하지 않는다.** 되풀이하면(`session::SessionState`) 사용 쪽이 모듈 경로를 이미 알고 있으므로 중복이다. 모듈 이름에는 선언한 모듈과 그 상위 모듈이 모두 해당한다(`reload::snapshot::ReloadCell` → `snapshot::Cell`). 도메인뿐 아니라 app·adapters의 개념 타입에도 적용한다. [7.4절](#74-계층별-규칙과-이-저장소의-배치)의 `Service` `Command` `View` 같은 suffix 목록은 역할 타입에 대한 것이고, 이 규칙은 그 목록에 해당하지 않는 개념 타입에만 적용한다. S 방식 역할 타입은 개념 prefix가 모듈 이름과 겹쳐도 된다(`relay::RelayService`). 이름이 모듈 경로 없이도 뜻을 가져야 하기 때문이다.

**D. lint 전제.** `clippy::module_name_repetitions`는 공개 모듈의 공개 항목만 본다. S 방식은 모듈 이름과 겹치는 이름을 허용하므로 이 저장소는 이 lint를 켜지 않는다(작업 공간 lint에 없다). 이름 중복 검사는 코드 리뷰로 한다.

### 7.4 계층별 규칙과 이 저장소의 배치

계층은 crate 경계다. 계층 안은 기능 단위 모듈이다. 의존 방향은 [코드 지도의 레이어 표](../architectures/code-map.md#레이어와-의존-방향)와 `cargo xtask layers`가 정한다.

| 계층 | 이 저장소의 위치 | 타입 이름 | trait 이름 | 모듈·파일 이름 | 금지 |
| --- | --- | --- | --- | --- | --- |
| domain(엔티티·값) | `crates/domain`(`vnc-relay-domain`) | 개념 이름(P). 값 모양 단어 허용. 오류는 `<개념>Error` | 없는 것이 기본. 규칙 확장점이 필요하면 능력 이름 | 개념 단위 `snake_case`, `pub mod`, 파일 이름 = 모듈 이름(`allow_list.rs`, `target_policy.rs`) | `Service` `Manager` `Dto` `View` `Impl` |
| application(유스케이스) | `crates/app`(`vnc-relay-application`) | `<개념>Service`, `Command` `Query` `View` `Snapshot` `Event` `Settings` `Config` | 포트 trait은 `ports` 모듈 | 기능 단위 `snake_case`(`relay`, `reload`, `management`, `metrics`) | `UseCase` `Interactor` `Handler` `Manager` `Impl` |
| ports(app이 소유하는 trait) | `crates/app/src/ports.rs`. 기능 안에서만 쓰는 포트 묶음은 그 기능의 `ports.rs`(`crates/app/src/reload/ports.rs`)에 둘 수 있다. 하위 파일이 필요하면 `ports/<개념>.rs`로 두고 이름은 `ports::` 아래로 평면 노출한다 | 포트가 소유한 데이터는 `Command` `Query` `View` `Event` | `<개념><Source·Sink·Store·Gateway·Control·Clock>` | `ports.rs` | `Port` suffix, `I` prefix, 기술 이름 |
| adapters(구동되는 쪽. 포트 구현) | `crates/adapters`(`vnc-relay-adapters`, 한 crate) | `<기술><포트 이름>`. 내부 형식은 `Wire` `Request` `Response` | 포트 trait은 만들지 않는다(app이 소유한다). 예외는 [7.9절](#79-등록-suffix와-예외-이름) | 기술 이름 또는 `<개념>_<기술>`(`tls`, `accounts_file`, `ip_db`). HTTP 경로 묶음은 `<개념>_api`(`management_http/session_api.rs`) | `Impl` `Default<포트>` |
| interface(구동하는 쪽: 진입점·리스너·CLI) | adapters 안의 모듈. 실행 인자는 루트 crate `src/`(`cli.rs`) | `<기술>EntryPoint`(`TcpEntryPoint`, `WsEntryPoint`), `<이름>Listener`(`ManagementListener`, `StatusListener`). HTTP 핸들러는 타입이 아닌 함수 | 필요 시 `Listener` | `<개념>_api`, `<기술>_http`(`management_http`, `status_http`) | `Controller` `Handler` `Presenter` suffix |
| infrastructure(런타임·OS 연동) | 별도 crate 없이 adapters 안. 기술 이름이 이미 맥락이다 | `<기술>…`(`Tokio` `System` `Fs` `Rustls` `Mmdb` `Tcp`) | 해당 없음 | 기술 이름(`process_stats`, `windows_service`, `terminal`) | `Infrastructure` suffix |
| composition root(조립) | 루트 crate `vnc-relay`(`src/`)와 `vnc-relay-tray`(`crates/tray`) | 새 역할 타입을 만들지 않는다. CLI 타입은 `Cli` `Args` 등 clap 관례 | 해당 없음 | `main.rs` + 조립 파일(`serve.rs`, `service.rs`) | 조립 코드에 업무 규칙 |

개념 하나가 계층을 지나는 이름의 예:

```text
대상 호스트 연결: crates/app/src/ports.rs  trait TargetGateway  →  crates/adapters/src/target.rs  TcpTargetGateway
사건 출력:        crates/app/src/ports.rs  trait EventSink      →  crates/adapters/src/ops_log.rs  JsonLinesEventSink
현재 시각:        crates/app/src/ports.rs  trait Clock          →  crates/adapters/src/clock.rs    SystemClock
```

### 7.5 모듈, 파일, crate 이름

**모듈과 파일**

- `snake_case`이고 단어는 `_`로 나눈다(`process_stats`, `management_http`). 붙여 쓰지 않는다. 개념 명사 하나를 기본으로 한다.
- 파일은 `foo.rs` + `foo/`다. `mod.rs`와 `#[path]`는 쓰지 않는다. 시험 코드도 같다.
- 한 파일의 주 타입은 파일 이름의 CamelCase와 같게 한다(`target_policy.rs` → `TargetPolicy`). 모듈 이름과 똑같은 이름은 clippy 기본 설정에서 허용된다.
- 모듈 이름에 역할 단어를 쓰는 것은 `ports`와 어댑터 crate뿐이다. `utils` `common` `helpers` `models` `interfaces` `impl` `types`는 쓰지 않는다.
- 부모와 같은 이름의 하위 모듈을 만들지 않는다(`module_inception`).
- 어댑터 모듈 이름은 기술 이름 또는 `<개념>_<기술>`이다(`tls`, `accounts_file`, `management_http`).

**crate**

| 종류 | 이름 | 이 저장소 |
| --- | --- | --- |
| 도메인 | `<제품>-domain` | `vnc-relay-domain` |
| 애플리케이션 | `<제품>-application` | `vnc-relay-application`(디렉터리는 `crates/app`) |
| 어댑터 하나로 묶음 | `<제품>-adapters` | `vnc-relay-adapters` |
| 어댑터를 기술별로 나눔 | `<제품>-adapter-<기술>` | 쓰지 않는다. 이 저장소는 어댑터를 한 crate에 모은다 |
| 실행 파일 | `<제품>` | `vnc-relay` |
| 보조 UI·도구 | `<제품>-<이름>`, 개발 도구는 `xtask` | `vnc-relay-tray`, `xtask` |
| 시험 하네스 | `<제품>-sim` 또는 `<제품>-testkit` | `vnc-relay-testkit`(`crates/testkit`, 배포하지 않는다) |

crate 이름은 `kebab-case`이고 `-rs`·`-rust`를 붙이지 않는다. 단수 `adapter-<기술>`과 복수 `adapters`를 섞어 쓰지 않는다. 한 crate에 어댑터를 모으면 `adapters`, 기술마다 crate를 나누면 `adapter-<기술>`이다.

### 7.6 함수 이름

- getter는 `get_`을 쓰지 않는다. `get`·`get_mut`처럼 컬렉션의 단일 대상 접근자는 예외다. 포트 메서드도 같다(`source.value()`, `store.load()`).
- 변환은 `as_`(비용 없음, 빌림→빌림), `to_`(비용 있음), `into_`(소유권 이동)를 따른다. `to_view()`처럼 비용이 있으면 `to_`다.
- 경계 변환은 `From`·`TryFrom` 구현을 먼저 고려한다. 도메인 값은 `new`와 `parse`, `from_<출처>`로 만든다.
- 7.8절의 suffix 목록은 함수에 적용하지 않는다.

### 7.7 금지 이름과 부딪히는 지점

금지 이름:

| 이름 | 쓰지 않는 이유 | 대신 쓰는 이름 |
| --- | --- | --- |
| `Manager` `Helper` `Util` `Handler` | 역할을 말하지 않는다. | 역할에 맞는 suffix. 값 타입이면 업무 이름. HTTP 핸들러는 타입이 아닌 함수 |
| `UseCase` `Interactor` | 사용 사례마다 타입을 만들면 타입 수가 늘어난다. | `<개념>Service`의 메서드 |
| `Repository` | `Store`와 역할이 같다. | `Store` |
| `Dto` | 입력과 출력을 모두 덮어 역할을 말하지 않는다. | `Command` `Query` `View` `Request` `Response` |
| `Impl`, `Default<포트>` | 구현체 이름은 기술을 드러내야 한다. 구현이 하나뿐인데 `Default<포트>`를 만들지 않는다. | `<기술><포트 이름>` |
| `Port` suffix, `I` prefix, `Trait` suffix | trait은 역할 어휘로 끝낸다. | `EventSink`처럼 `<개념><포트 역할>` |
| `Controller` `Presenter` | 접속을 받는 쪽은 `EntryPoint`·`Listener`, 출력 모델은 `View`가 맡는다. | `EntryPoint` `Listener` `View` |
| `Infrastructure` | 기술 이름이 이미 맥락이다. | `<기술>…`(`Tokio` `System` `Fs` `Rustls` `Mmdb`) |
| 단독 `Error`, `Failure`로 끝나는 오류 이름 | `std::error::Error`와 겹치고 개념을 말하지 않는다. | `<개념>Error` |
| alias가 아닌 타입의 `Result` suffix | `fmt::Result`·`io::Result`처럼 alias로 읽힌다. | 분류는 `Outcome`, 묶음은 개념 이름 |
| 닫힌 어휘 밖의 동사형 이름(`Connector` `Hasher` `Loader`) | 포트마다 새 suffix가 늘어난다. | `Gateway` 등 포트 어휘 |
| 모듈 이름 `utils` `common` `helpers` `models` `interfaces` `impl` `types` | 역할 없는 이름이다. | 개념 단위 모듈 |

부딪히는 지점별 규칙:

| 주제 | 규칙 |
| --- | --- |
| 모듈 경로 중복(`repository::OrderRepository`) | 역할 타입은 S(suffix 포함, 경로 반복 허용), 개념 타입은 P다. 역할 단어를 경로와 suffix에 둘 다 쓰지 않는다. |
| `Service` 남용 | `Service`는 유스케이스 입구에만 쓴다. 규칙은 domain 타입의 메서드에, 판정 규칙은 `Policy`에 둔다. 한 `Service` 안의 메서드가 사용 사례 하나다. |
| 오류 타입 이름 | 항상 `<개념>Error`다. 표준 trait은 `impl std::error::Error for TlsError`처럼 경로로 써서 `as StdError` 별칭이 필요 없게 한다. |
| `Result` suffix | alias가 아닌 타입에는 쓰지 않는다. 분류는 `Outcome`, 묶음은 개념 이름(`VerificationReport` 식)으로 짓는다. |
| `Config`와 `Settings` | `Config`는 생성 옵션, `Settings`는 검증을 마친 설정 모델이다. 한 단어로 합치지 않는다. `Config`가 prefix인 이름(`ConfigView`, `ConfigPatch`)은 용어집의 설정 보기·동적 설정 편집 개념이라 업무 이름의 일부다. |
| 경계 데이터 | `Dto`를 쓰지 않는다. 방향에 따라 `Command` `Query` `View` `Request` `Response`로 나눈다. |
| 같은 이름 충돌 | 별칭(`as`)보다 이름을 구분한다. 예: 도메인 enum `Listener`는 `ListenerKind`처럼 바꾸고 adapters trait은 `Listener`를 유지한다. |
| 빌더와 `Default` | 필수 값이 적으면 `new(config)`, 선택 값이 많으면 `<대상>Builder`를 쓴다. |
| `Wire` | 어댑터 내부 저장·전송 형식이다. 어댑터 밖으로 내보내지 않는다. 공개하면 `View`·`Response`로 바꾼다. |
| re-export와 prelude | `prelude` 모듈은 만들지 않는다. 평면 `pub use`는 adapters crate에서 허용한다. |
| `use` 스타일 | 타입은 직접 import한다. 개념 타입의 동명 충돌만 부모 모듈 import로 푼다. 함수는 부모 모듈을 import한다(`use crate::toml_text;` → `toml_text::error_position(…)`). |

### 7.8 suffix 목록

닫힌 목록이다. 새 suffix는 이유와 함께 [7.9절](#79-등록-suffix와-예외-이름)에 등록한 뒤 쓴다.

| suffix | 계층 | 뜻 | 이 저장소의 예 |
| --- | --- | --- | --- |
| `Service` | application | 유스케이스 입구. 메서드 하나가 사용 사례 하나다 | `RelayService`, `AdminService`, `ReloadService`, `LifecycleService`, `ConfigEditService` |
| `Command` | application·ports | 상태를 바꾸는 입력 | `SessionCommand` |
| `Query` | application·ports | 조회 조건 입력 | 현재 없음 |
| `View` | application·ports | 읽기 전용 출력 모델 | `SessionView`, `SummaryView` |
| `Snapshot` | application. adapters 안의 비공개 보조 타입에도 쓴다 | 한 시점에 고정한 상태 | `RelaySnapshot`, `MetricsSnapshot`, `ReloadSnapshot`, `QuerySnapshot`(adapters 비공개) |
| `Event` | application·ports | 일어난 사건 | `Event`, `SourceEvent` |
| `Settings` | application | 검증을 마친 설정 모델 | `RelaySettings` |
| `Config` | application·adapters | 생성 시 받는 옵션 | `RelayRuntimeConfig` |
| `Source` | ports·adapters | 외부·환경에서 값을 읽어 온다 | `TokenSource`, `RelayIdSource`, `ProcessStatsSource` |
| `Sink` | ports·adapters | 사건·기록을 밖으로 내보낸다 | `EventSink`, `JsonLinesEventSink`, `MetricsSink` |
| `Store` | ports·adapters | 메모리 밖 상태를 읽고 저장한다 | `AccountStore`, `ConfigFileStore`, `FsConfigFileStore` |
| `Gateway` | ports·adapters | 위 넷에 맞지 않는 외부 시스템 호출 | `TargetGateway`, `PasswordHashGateway`, `IpDbFileGateway`, `TcpTargetGateway` |
| `Control` | ports | 실행 중인 런타임에 명령을 넣는다. 포트 trait에만 쓴다 | `RelaySessionControl`, `RelayDenyControl` |
| `Clock` | ports·adapters | 시각. 포트 이름 자체로도 쓴다 | `Clock`, `ReloadClock`, `SystemClock` |
| `Error` | 모든 계층 | 오류 타입. 항상 `<개념>Error` | `TlsError`, `ConfigError` |
| `Outcome` | 모든 계층 | 동작 결과를 분류하는 enum 또는 struct | `ReloadOutcome`, `ArrivalOutcome` |
| `Reason` | 모든 계층 | 사유를 분류하는 enum | `RejectReason`, `AbortReason` |
| `Wire` | adapters | 어댑터 안에서만 쓰는 저장·전송 형식 | 현재 없음 |
| `Request` `Response` | adapters | 어댑터가 소유한 API 요청·응답 모델 | `LogsResponse` |

공통 표준이 이 목록 밖에 등록한 suffix `Provider` `Verifier` `Renderer` `Visitor` `Scheduler` `Timer`가 있다. 이 저장소는 아직 쓰지 않는다. 필요하면 이유를 적어 7.9절에 등록한 뒤 쓴다.

값 모양 단어는 역할 suffix가 아니라 개념 이름의 일부다(P 방식).

| 단어 | 뜻 | 이 저장소의 예 |
| --- | --- | --- |
| `Id` | 식별자 값 | `RelayId`, `ConnId` |
| `Key` | 서명 확인·HMAC 계산에 쓰는 키 값 | `RelayIdKey` |
| `Kind` `State` | 종류, 상태 | `State`(모듈 안 비공개 보조 타입) |
| `Policy` | 허용·거절을 판정하는 규칙이나 값. domain에 두며, adapters가 소유한 수락 설정 값에도 쓴다 | `TargetPolicy`, `RegistrationPolicy`, `AcceptPolicy`, `WsPolicy` |
| `Table` | 키로 찾는 메모리 상태 모음 | `WaitingTable` |
| `List` | 순서가 있는 값 목록 | `AllowList`, `DenyList` |

데이터 모양을 설명하는 말(`Claims` `Parts` `Binding` `Budget` `Item` `Plan`)도 업무 이름의 일부다. 예: `ConnectionBudget`, `IpItem`, `ListenerPlan`.

### 7.9 등록 suffix와 예외 이름

표준의 규칙과 다르게 지은 이름은 이 절에 이유와 함께 등록한 것만 허용한다. 새 예외는 같은 변경에서 이 절에 추가한다.

닫힌 목록 밖의 suffix:

| suffix | 계층 | 뜻 | 이 저장소의 예 | 등록 이유 |
| --- | --- | --- | --- | --- |
| `EntryPoint` | adapters | 중계 접속을 받는 리스너 | `TcpEntryPoint`, `WsEntryPoint` | 용어집 "진입점"의 이름이다. 공통 표준이 `<기술>EntryPoint`를 정한다. domain의 `EntryPoint` enum은 업무 개념이라 suffix 규칙 대상이 아니다. |
| `Listener` | adapters | 진입점이 아닌 HTTP 리스너, 수락 trait과 이를 감싸는 타입 | `ManagementListener`, `StatusListener`, `TlsListener`, `SupervisedListener`, trait `Listener` | 용어집 "관리 리스너"·"상태 리스너"의 이름이다. 공통 표준이 `<이름>Listener`와 trait `Listener`를 정한다. |
| `Parser` | domain | 분할 수신한 바이트를 모아 해석한다 | `HandshakeParser` | 초기 교환 요청 한 건을 누적해 해석하는 상태 기계다. 닫힌 목록의 suffix는 이 동작을 말하지 않는다. |
| `Decoder` | adapters | 바이트 스트림에서 WS 프레임을 점진적으로 해석한다 | `FrameDecoder` | RFC 6455 프레임 해석 전용 상태 기계다. `Parser`는 초기 교환 요청, `Decoder`는 WS 프레임으로 대상 형식이 다르다. |
| `Watcher` | adapters | 파일 변경을 감지하는 작업 | `ReloadWatcher` | `Source`는 값을 읽어 오는 포트다. 변경 감지는 작업을 소유한다. |
| `Guard` | app | drop할 때 등록·점유를 정리하는 RAII 값 | `HandshakingGuard`, `WaitingGuard`, `ClaimGuard`, `ActiveGuard` | 표준 라이브러리 `MutexGuard`와 같은 관용어다. 정리 의무를 이름에 남긴다. |
| `Limiter` | domain | 실패 횟수를 세어 잠금을 판정하는 상태 | `LoginLimiter` | 용어집 "로그인 제한"을 상태로 가진 타입이다. 설정 값은 `LoginLimitPolicy`이며 `Policy`는 상태를 갖지 않는다. |
| `Reporter` | adapters | SCM에 서비스 상태를 보고한다 | `StatusReporter` | 시작·중지 중 체크포인트 갱신까지 맡는 Windows 서비스 전용 어댑터다. 사건을 내보내는 `Sink`와 다르다. |

규칙과 다르게 지은 이름:

| 이름 | 위치·종류 | 규칙과 다른 점 | 이유 |
| --- | --- | --- | --- |
| `RelayStream`, `TcpRelayStream`, `WsRelayStream`, `BoxRelayStream` | app trait, adapters 구현체, 타입 alias | 포트 어휘로 끝나지 않는다. 구현체가 `<기술><포트 이름>` 모양이 아니다. | 용어집 "중계 스트림"이 업무 이름이다. 닫힌 어휘에 이 개념을 부르는 단어가 없다. |
| `AccountsFile` | adapters, `AccountStore` 구현체 | `<기술><포트 이름>`이 아니다. | 용어집 "관리자 계정 파일"이라는 업무 값이 포트를 겸해 구현한다. 이름을 바꾸면 업무 이름이 사라진다. |
| `RelayIdKey` | adapters, `RelayIdSource` 구현체 | 〃 | 중계 ID를 HMAC으로 계산하는 키 값(`Key`)이 포트를 겸한다. |
| `RelayRuntime` | app, `RelaySessionControl`·`RelayDenyControl`·`RelayStateSource` 구현체 | 포트 하나의 이름을 따르지 않는다. | 실행 중 중계 상태를 소유하는 타입이 포트 3개를 구현한다. 포트 이름 하나를 붙일 수 없다. |
| `Scm` | adapters, `ServiceInstallGateway` 구현체 | `<기술><포트 이름>`이 아니다. | Windows 서비스 제어 관리자(SCM)라는 기술 핸들이 포트를 겸한다. 기술 이름이 이름이다. |
| `ConfigService` | app trait | `Service`가 trait 이름이다. 포트가 아니다. | `management_http`는 `Arc<dyn ConfigService>`만 받고, 구현체 `ConfigEditService<A>`는 제네릭이다. adapters 시험은 이 trait의 대역을 쓴다. |
| `WatchGateway`, `ServiceInstallGateway` | adapters와 `src/`의 crate 안 포트 | 포트 trait을 app이 아닌 곳에 둔다. | app의 사용 사례가 부르지 않는 OS 기능(파일 감시, SCM)을 시험에서 바꿔 끼우는 용도다. suffix 어휘와 구현체 규칙은 같다. |
| `HandshakeRequest` | domain enum | adapters가 소유한 API 요청 모델이 아닌데 `Request`로 끝난다. | [Repeater 초기 교환 계약](../contracts/repeater-handshake-v1.md)이 "초기 요청"이라 부르는 업무 값이다. |
| 실행 crate 안의 보조 타입 | `vnc-relay-tray`의 `worker::Request` `Reply` | suffix 없이 짧은 이름을 쓴다. | 실행 crate의 타입은 다른 crate가 import하지 않고 계층 경계를 넘지 않는다([7.3절](#73-suffix가-필요한-조건) A1·A4에 해당하지 않는다). 금지 이름 규칙은 그대로 적용한다. |
| `tls::File`, `snapshot::Cell` | adapters enum, app 공유 셀 | 표준 라이브러리 `std::fs::File`·`std::cell::Cell`과 이름이 같다([7.3절](#73-suffix가-필요한-조건) A2). `snapshot::Cell`은 다른 crate도 import한다(A1). | 7.3절 C를 app·adapters에도 적용하면서 `TlsFile`·`ReloadCell`의 모듈 이름 되풀이를 없앴다. 항상 부모 모듈을 붙여 쓴다(`tls::File`, `snapshot::Cell`). 단독으로 import하지 않는다. |
| `sources::Report` | app struct | 경계 출력이다. [7.2절](#72-역할-단어-선택)은 `View`를 정하고 닫힌 어휘에 `Report`가 없다. | management-api-v1 §1.17의 `200` 본문을 JSON 변환 전에 담는 값이다. `sources` 모듈 경로가 맥락을 주므로 `sources::Report`로 쓴다. |

### 7.10 도메인 prefix

같은 업무 도메인의 이름은 같은 prefix로 시작한다. 역할 타입은 이 prefix를 붙여 `<개념 prefix><역할 suffix>`로 짓는다. 개념 타입은 [7.3절](#73-suffix가-필요한-조건) C에 따라 모듈 이름을 되풀이하지 않는다.

| prefix | 업무 용어 | 용어집 링크 |
| --- | --- | --- |
| `Relay` | 중계 ID, 중계 측 연결, 중계 스트림 | [용어집](../../GLOSSARY.md#중계-방식과-세션) |
| `Handshake` | 초기 교환 | [용어집](../../GLOSSARY.md#중계-방식과-세션) |
| `Waiting` | 대기 연결 | [용어집](../../GLOSSARY.md#중계-방식과-세션) |
| `Session` | 활성 세션, 세션 식별자 | [용어집](../../GLOSSARY.md#중계-방식과-세션) |
| `Target` | Mode I 대상, 대상 허용 정책 | [용어집](../../GLOSSARY.md#중계-방식과-세션) |
| `EntryPoint` | 진입점 | [용어집](../../GLOSSARY.md#접근-경계) |
| `Allow` | 출발지 IP 허용 목록, 전체 허용 항목 | [용어집](../../GLOSSARY.md#접근-경계) |
| `Ticket` | 접속 ticket, 사용 기록, ticket 공개 키 | [용어집](../../GLOSSARY.md#접근-경계) |
| `Management` | 관리 리스너, 관리 API | [용어집](../../GLOSSARY.md#관리-기능) |
| `Status` | 상태 리스너 | [용어집](../../GLOSSARY.md#관리-기능) |
| `Admin` | 관리자 계정, 관리자 계정 준비, 관리 세션 | [용어집](../../GLOSSARY.md#관리-기능) |
| `Account` | 관리자 계정 파일 | [용어집](../../GLOSSARY.md#관리-기능) |
| `Close` | 종료 원인 | [용어집](../../GLOSSARY.md#세션-종료와-기록) |
| `RecentClosed` | 최근 종료 목록 | [용어집](../../GLOSSARY.md#세션-종료와-기록) |
| `Event` | 사건, 운영 로그 | [용어집](../../GLOSSARY.md#세션-종료와-기록) |
| `Reload` | 부분 재로드, 재로드 후보 | [용어집](../../GLOSSARY.md#설정과-수명) |
| `Shutdown` | 정상 종료, 종료 유예 | [용어집](../../GLOSSARY.md#설정과-수명) |
