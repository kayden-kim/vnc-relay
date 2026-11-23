# Rust 코드 컨벤션

같은 개념은 어느 crate에서나 같은 이름으로 쓰고, 서식과 위험한 코드 패턴은 사람이 아니라 도구가 판정하게 한다. 이름은 [용어집](../../GLOSSARY.md)의 코드 영어 이름에서 시작하고, 모듈은 [코드 지도](../architectures/code-map.md)의 레이어와 위치를 따른다. 서식은 rustfmt 기본값, 금지 패턴은 작업 공간 lint로 막는다.

| 항목 | 값 |
| --- | --- |
| 상태 | Active |
| 적용 대상 | 사람이 작성한 Rust 소스 코드와 테스트 코드, `Cargo.toml`, `clippy.toml` |
| 기준 버전 | Rust 1.99.1(`rust-toolchain.toml`), edition 2024, [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/naming.html) |
| 관련 아키텍처 문서 | [코드 지도](../architectures/code-map.md) |
| 관련 ADR | [ADR-0004 tokio·rustls](../decisions/ADR-0004-tokio-rustls.md) |
| 규칙 준수 확인 방법 | `cargo fmt`, `cargo clippy`(작업 공간 lint), `cargo xtask layers`, 코드 리뷰 |

## 1. 규칙 우선순위

[컨벤션 작성 규칙](../conventions/AGENTS.md#규칙-우선순위)을 따른다. 이 문서는 컴파일러, rustfmt·clippy, Rust API Guidelines가 정하지 않았거나 선택지를 남긴 부분만 정한다.

## 2. 규칙

| 대상 | 해야 할 것 | 피해야 할 것 | 판정 방법 |
| --- | --- | --- | --- |
| 서식 | rustfmt 기본값을 쓴다. 설정 파일 `rustfmt.toml`을 두지 않는다. | 손으로 맞춘 정렬, rustfmt 설정 추가 | `cargo fmt --all --check` |
| 대소문자 | 타입·trait·enum 값은 `UpperCamelCase`, 함수·변수·모듈·파일은 `snake_case`, 상수는 `SCREAMING_SNAKE_CASE`. crate 이름은 `vnc-relay-<레이어>`. | `relayID`, `Relay_Id` | 컴파일러 경고(`non_camel_case_types` 등), 리뷰 |
| 업무 이름 | 업무 개념의 타입·모듈은 용어집 코드 영어 이름으로 시작한다(7절 prefix 목록). 용어집에 없는 업무 개념이면 먼저 용어집에 등록한다. | 용어집과 다른 동의어(`MatchId`, `Token`으로 중계 ID를 부름) | 리뷰 |
| 유형 suffix | 7절 목록의 역할을 하는 타입은 그 suffix로 끝낸다. trait은 모두 `Service` `Store` `Source` `Sink` `Gateway` `Control` `Clock` `Listener` 가운데 하나로 끝낸다. 포트 하나만 구현하는 구현체는 `<기술><trait 이름>`으로 짓기를 권장한다. 업무 값이나 기술 핸들이 포트를 겸해 구현하면 그 타입의 이름을 유지한다(`AccountsFile`, `RelayIdKey`, `RelayRuntime`, `Scm`). 업무 값·데이터 타입(`CloseCause`, `Role`, `Mode`, `ConnectionBudget`, `TicketClaims`)은 suffix 없이 업무 이름만 쓴다. trait의 예외는 용어집 이름 `RelayStream`뿐이다. | 7절 금지 이름(`Manager`, `Helper`, `Util`, `Handler`, `UseCase`, `Interactor`, `Repository`, `Dto`, `Impl`), 목록에 없는 역할 suffix | 리뷰 |
| 테스트 전용 타입 | `#[cfg(test)]` 모듈과 `tests/`에만 있는 타입에는 유형 suffix 규칙을 적용하지 않는다. 대역 이름은 `Fake`나 `Test`로 시작하기를 권장한다. | 운영 코드에서 `Fake`·`Test` 이름을 쓰는 것 | 리뷰 |
| 모듈 파일 | 하위 모듈이 있으면 `foo.rs`와 `foo/` 디렉터리를 쓴다. | `foo/mod.rs` | clippy `mod_module_files`(deny) |
| 모듈 위치 | 코드 지도의 레이어와 위치에 둔다. 새 모듈이 생기면 같은 변경에서 코드 지도를 고친다. | domain·app에서 소켓·파일·TLS·HTTP 라이브러리 사용 | `cargo xtask layers`, `crates/app/clippy.toml`, 리뷰 |
| 주석 언어 | 주석과 문서 주석은 영어로 쓴다. 용어는 용어집의 코드 영어 이름을 쓴다. | 한국어 주석, 용어집과 다른 영어 표현 | 리뷰 |
| 문서 주석 | domain·app의 공개 항목에는 `///`로 무엇을 보장하는지 쓴다. 근거가 되는 스펙·계약 절을 적는다(예: `/// See cli-v1 §1.2.`). | 코드를 다시 읽어 주는 주석 | 리뷰 |
| 오류 | 모듈마다 `Error` suffix enum을 두고 `std::fmt::Display`와 `std::error::Error`를 직접 구현한다. 계약의 오류 코드가 있으면 enum 값에서 그 코드 문자열을 얻는 함수를 둔다. | PRD 목록에 없는 오류 crate(`anyhow`, `thiserror`) 직접 의존, 문자열 오류 | 리뷰, [의존성 라이선스 검사](../how-to/check-dependency-licenses.md) |
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
이유: 용어집 이름 "대기 연결(waiting)"과 suffix 목록의 Table을 쓴다. mod.rs와 금지 이름 Manager를 쓰지 않는다.
```

```text
좋음: crates/app/src/ports.rs 의 trait AccountStore, crates/adapters/src/accounts_file/store.rs 의 AccountsFile
나쁨: crates/app/src/ports.rs 의 trait TomlAccountRepository
이유: app의 포트 이름은 기술(TOML)을 드러내지 않고, 메모리 밖 상태를 저장하는 포트는 Store로 끝낸다. 구현체 AccountsFile은 계정 파일이라는 업무 이름을 유지한다.
```

```text
좋음: crates/app/src/ports.rs 의 trait TargetGateway, crates/adapters/src/target.rs 의 TcpTargetGateway
나쁨: crates/app/src/ports.rs 의 trait TargetConnector
이유: 대상 호스트에 연결하는 포트는 Store·Source·Sink·Control·Clock에 맞지 않아 Gateway로 끝낸다. 동사에서 만든 이름(Connector, Hasher, Loader)을 따로 늘리지 않는다.
```

```text
좋음: crates/app/src/ports.rs 의 trait IpDbFileGateway, crates/adapters/src/ip_db.rs 의 MmdbFileGateway
나쁨: crates/adapters/src/ip_db.rs 의 MmdbIpDbFileGateway
이유: 포트 하나만 구현하는 구현체는 `<기술><포트 이름>`을 권장한다. 기술 이름(Mmdb)이 IP DB 형식을 이미 말하면 포트 이름 앞의 IpDb를 빼서 5단어 이상이 되지 않게 한다.
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
- `xtask/`는 개발 도구다. 7절 prefix·suffix 규칙을 적용하지 않는다. 서식·lint 규칙은 적용한다.
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

## 7. 이름 짓기 목록

규칙은 [이름 짓기 기본 규칙](../conventions/AGENTS.md#이름-짓기-기본-규칙)을 따른다. Rust에서는 타입·trait 이름과 모듈 파일 이름에 적용한다. 함수 이름에는 적용하지 않는다. 모듈 이름이 이미 prefix를 담으면 타입에서 반복하지 않아도 된다(`matching::WaitingTable`). suffix는 레이어와 의존 방향에서 맡은 역할을 나타낸다. 데이터의 모양을 설명하는 말(`Claims`, `Parts`, `Binding`, `Budget`)은 suffix가 아니라 업무 이름의 일부다.

도메인 prefix:

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

suffix 목록(21개):

| suffix | 레이어 | 역할 | 근거 |
| --- | --- | --- | --- |
| `Id` | domain | 식별자 값 타입(`RelayId`, `SessionId`, `ConnId`) | 계약의 `*_id` 필드와 대응한다. |
| `Key` | domain·adapters | 서명 확인·HMAC 계산에 쓰는 키 값(`TicketKey`, `RelayIdKey`, `SessionIdKey`) | 서명 확인·HMAC 계산용 키를 나타낸다. |
| `Policy` | domain | 허용·거절을 판정하는 규칙(`TargetPolicy`) | 클린 아키텍처의 도메인 서비스에 해당한다. 판정 규칙을 나타낸다. |
| `Table` | domain | 키로 찾는 메모리 상태 모음(`WaitingTable`) | 키별 조회와 갱신을 나타낸다. |
| `List` | domain | 순서가 있는 값 목록(`AllowList`, `RecentClosedList`) | 설정 키 `allow`와 최근 종료 목록에 대응한다. |
| `Parser` | domain | 바이트·문자열을 점진적으로 해석하는 상태 기계(`HandshakeParser`) | 분할 수신을 다루는 해석기다. |
| `Settings` | app | 검증을 마친 설정 값(`RelaySettings`) | 파일 형식과 무관한 값이다. |
| `View` | app | 계층 밖으로 내주는 읽기 전용 출력 모델(`SessionView`, `SummaryView`) | adapters가 이 값을 JSON 응답으로 바꾼다. 별도 `Dto` suffix를 두지 않는다. |
| `Snapshot` | app | 한 시점에 고정한 상태 묶음(`RelaySnapshot`, `MetricsSnapshot`, `ReloadSnapshot`) | `View`는 계층 밖으로 내주는 출력이고 `Snapshot`은 app 안에서 시점을 고정한 상태라 역할이 다르다. |
| `Service` | app | adapters가 부르는 사용 사례의 입구(`RelayService`, `AdminService`, `ReloadService`, `LifecycleService`, `ConfigEditService`)와 그 trait(`ConfigService`) | 클린 아키텍처의 UseCase·Interactor에 해당한다. 사용 사례 하나가 메서드 하나이며, 사용 사례마다 타입을 만들지 않는다. |
| `Store` | app(trait), adapters(구현) | 메모리 밖에 두는 상태를 읽고 저장하는 포트(`AccountStore`, `ConfigFileStore`) | 클린 아키텍처의 Repository에 해당한다. |
| `Source` | app(trait), adapters(구현) | 실행 상태나 바깥에서 값을 읽어 오는 포트(`TokenSource`, `RelayIdSource`, `RelayStateSource`, `ProcessStatsSource`) | 값을 공급하는 쪽을 나타낸다. |
| `Sink` | app(trait), app·adapters(구현) | 사건을 받아 내보내는 포트(`EventSink`)와 그 구현(`JsonLinesEventSink`, `MetricsSink`) | 사건 출력 포트를 나타낸다. |
| `Gateway` | app(trait), adapters(구현) | 바깥 기능을 부르는 포트 가운데 `Store`·`Source`·`Sink`·`Control`·`Clock`에 맞지 않는 것(`TargetGateway`, `PasswordHashGateway`, `PasswordVerifyGateway`, `IpDbFileGateway`, `IpDbLookupGateway`, `ConfigDocumentGateway`, `AccountInputGateway`). adapters나 실행파일 안에서만 쓰는 포트(`WatchGateway`, `ServiceInstallGateway`)도 같다. | 클린 아키텍처의 Gateway에 해당한다. 앞의 다섯 suffix에 맞으면 그 suffix를 쓴다. 맞는 것이 없을 때만 쓴다. |
| `Control` | app | 실행 중인 중계에 명령을 넣는 포트(`RelaySessionControl`, `RelayDenyControl`) | `RelayRuntime`이 구현하고 관리·재로드 서비스가 부른다. 읽기 포트 `Source`와 짝이다. |
| `Clock` | app(trait), adapters(구현) | 현재 시각을 주는 포트(`SystemClock`, `ReloadClock`) | 테스트에서 시각을 바꾸기 위해 둔다. |
| `EntryPoint` | adapters | 중계 접속을 받는 리스너(`TcpEntryPoint`, `WebSocketEntryPoint`) | 용어집 "진입점"과 같다. |
| `Listener` | adapters | 진입점이 아닌 HTTP 리스너와 리스너 trait(`ManagementListener`, `StatusListener`, `Listener`) | 용어집 "관리 리스너"·"상태 리스너"와 같다. |
| `Watcher` | adapters | 파일 변경을 감지하는 작업(`ReloadWatcher`) | 변경 감지를 나타낸다. |
| `Response` | adapters | adapters가 JSON 본문으로 바로 만드는 응답 모델(`LogsResponse`) | 관리 API 계약의 응답이다. adapters가 보관하는 값이라 app의 `View`가 아니다. wire 형식 전용 타입이 더 필요하면 adapters 경계에 두고 이 suffix를 쓴다. |
| `Error` | 모든 레이어 | 모듈의 오류 enum(`ConfigError`) | 모듈 오류를 나타낸다. |

클린 아키텍처 이름은 따로 만들지 않고 위 suffix로 받는다.

| 클린 아키텍처 이름 | 이 저장소의 이름 |
| --- | --- |
| Entity·Value Object | suffix 없는 업무 이름(`Session`, `CloseCause`) |
| Domain Service | `Policy` |
| UseCase·Interactor·Application Service | `Service`의 메서드 |
| Repository | `Store` |
| Gateway | `Gateway` |
| Presenter 입력·Output DTO | `View`(app), `Response`(adapters) |
| Controller | `EntryPoint`, `Listener` |

금지 이름:

| 이름 | 쓰지 않는 이유 | 대신 쓰는 이름 |
| --- | --- | --- |
| `Manager` `Helper` `Util` `Handler` | 역할을 말하지 않는다. | 역할에 맞는 suffix. 값 타입이면 업무 이름 |
| `UseCase` `Interactor` | 사용 사례마다 타입을 만들면 타입 수가 늘어난다. | `Service`의 메서드 |
| `Repository` | `Store`와 역할이 같다. | `Store` |
| `Dto` | `View`와 `Response`가 같은 역할을 한다. | `View`, `Response` |
| `Impl` | 구현체 이름은 기술을 드러내야 한다. | `<기술><포트 이름>` |
| `Connector` `Hasher` `Verifier` `Loader` | 동사에서 만든 이름은 포트마다 새 suffix를 늘린다. | `Gateway` |
