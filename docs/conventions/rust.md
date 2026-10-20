# Rust 코드 컨벤션

같은 개념은 어느 crate에서나 같은 이름으로 쓰고, 서식과 위험한 코드 패턴은 사람이 아니라 도구가 판정하게 한다. 이름은 [용어집](../../GLOSSARY.md)의 코드 영어 이름에서 시작하고, 모듈은 [코드 지도](../architectures/code-map.md)의 레이어와 위치를 따른다. 서식은 rustfmt 기본값, 금지 패턴은 작업 공간 lint로 막는다.

| 항목 | 값 |
| --- | --- |
| 상태 | Draft |
| 적용 대상 | 사람이 작성한 Rust 소스 코드와 테스트 코드, `Cargo.toml`, `clippy.toml` |
| 기준 버전 | Rust 1.99.1(`rust-toolchain.toml`), edition 2024, [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/naming.html) |
| 관련 아키텍처 문서 | [코드 지도](../architectures/code-map.md) |
| 관련 ADR | [ADR-0004 tokio·rustls](../decisions/ADR-0004-tokio-rustls.md) |
| 적용 시작일 | 2026-10-20 |
| 규칙 준수 확인 방법 | `cargo fmt`, `cargo clippy`(작업 공간 lint), `cargo xtask layers`(작업 0.6), 코드 리뷰 |

## 1. 규칙 우선순위

[컨벤션 작성 규칙](../conventions/AGENTS.md#규칙-우선순위)을 따른다. 이 문서는 컴파일러, rustfmt·clippy, Rust API Guidelines가 정하지 않았거나 선택지를 남긴 부분만 정한다.

## 2. 규칙

| 대상 | 해야 할 것 | 피해야 할 것 | 판정 방법 |
| --- | --- | --- | --- |
| 서식 | rustfmt 기본값을 쓴다. 설정 파일 `rustfmt.toml`을 두지 않는다. | 손으로 맞춘 정렬, rustfmt 설정 추가 | `cargo fmt --all --check` |
| 대소문자 | 타입·trait·enum 값은 `UpperCamelCase`, 함수·변수·모듈·파일은 `snake_case`, 상수는 `SCREAMING_SNAKE_CASE`. crate 이름은 `vnc-relay-<레이어>`. | `relayID`, `Relay_Id` | 컴파일러 경고(`non_camel_case_types` 등), 리뷰 |
| 업무 이름 | 업무 개념의 타입·모듈은 용어집 코드 영어 이름으로 시작한다(7절 prefix 목록). 용어집에 없는 업무 개념이면 먼저 용어집에 등록한다. | 용어집과 다른 동의어(`MatchId`, `Token`으로 중계 ID를 부름) | 리뷰 |
| 유형 suffix | 역할이 있는 타입은 7절 suffix 목록의 suffix로 끝낸다. 업무 값 자체인 타입(`CloseCause`, `Role`, `Mode`, `ViewerTransport`)은 suffix 없이 용어집 이름만 쓴다. | 목록에 없는 suffix(`Manager`, `Helper`, `Util`) | 리뷰 |
| 모듈 파일 | 하위 모듈이 있으면 `foo.rs`와 `foo/` 디렉터리를 쓴다. | `foo/mod.rs` | clippy `mod_module_files`(deny) |
| 모듈 위치 | 코드 지도의 레이어와 위치에 둔다. 새 모듈이 생기면 같은 변경에서 코드 지도를 고친다. | domain·app에서 소켓·파일·TLS·HTTP 라이브러리 사용 | `cargo xtask layers`, `crates/app/clippy.toml`, 리뷰 |
| 주석 언어 | 주석과 문서 주석은 영어로 쓴다(사용자 결정, 2026-10-20). 용어는 용어집의 코드 영어 이름을 쓴다. | 한국어 주석, 용어집과 다른 영어 표현 | 리뷰 |
| 문서 주석 | domain·app의 공개 항목에는 `///`로 무엇을 보장하는지 쓴다. 근거가 되는 스펙·계약 절을 적는다(예: `/// See cli-v1 §1.2.`). | 코드를 다시 읽어 주는 주석 | 리뷰 |
| 오류 | 모듈마다 `Error` suffix enum을 두고 `std::fmt::Display`와 `std::error::Error`를 직접 구현한다. 계약의 오류 코드가 있으면 enum 값에서 그 코드 문자열을 얻는 함수를 둔다. | PRD 목록에 없는 오류 crate(`anyhow`, `thiserror`) 직접 의존, 문자열 오류 | 리뷰, [의존성 라이선스 검사](../how-to/check-dependency-licenses.md) |
| 실패 처리 | 테스트 밖 코드에서 `unwrap`·`expect`를 쓰지 않는다. 오류를 반환하거나 연결 하나만 정리한다. | 연결 오류로 프로세스가 끝나는 코드 | clippy `unwrap_used`·`expect_used`(deny, 테스트 허용) |
| 출력 | 명령행 출력은 실행파일 조립 crate에서 `std::io::Write`로 쓴다. 운영 로그는 adapters의 운영 로그 모듈만 쓴다. | `println!`·`eprintln!`·`dbg!` | clippy `print_stdout`·`print_stderr`·`dbg_macro`(deny) |
| 비밀 값 | 비밀번호·해시·쿠키 값·CSRF 토큰·개인키를 담는 타입은 `Debug`를 직접 구현해 값을 가린다(`"<redacted>"`). | 비밀 값 타입에 `#[derive(Debug)]` | 리뷰, 운영 정보 계약 1.11절 계약 테스트 |
| unsafe | `unsafe` 코드를 쓰지 않는다. | `unsafe` 블록 | rustc `unsafe_code`(forbid) |
| 미완성 표시 | 끝나지 않은 코드를 병합하지 않는다. | `todo!()`, `unimplemented!()` | clippy `todo`·`unimplemented`(deny) |
| lint 예외 | 꼭 필요하면 가장 좁은 범위에 `#[expect(lint, reason = "...")]`를 쓴다. | crate 전체 `#![allow(...)]`, 이유 없는 `allow` | clippy `allow_attributes_without_reason`(deny), 리뷰 |
| 의존성 | [PRD "구현 기술" 행](../requirements/PRD.md#확정-기준)의 crate만 쓴다. 버전은 작업 공간 `[workspace.dependencies]`에 한 번만 적는다. | crate마다 다른 버전, 목록 밖 crate | `cargo xtask layers`, `cargo deny check licenses` |

## 3. 예제

```text
좋음: crates/domain/src/matching.rs 의 WaitingTable
나쁨: crates/domain/src/matcher/mod.rs 의 MatchManager
이유: 용어집 이름 "대기 연결(waiting)"과 suffix 목록의 Table을 쓴다. mod.rs와 목록에 없는 suffix Manager를 쓰지 않는다.
```

```text
좋음: crates/app/src/ports.rs 의 trait AccountStore, crates/adapters/src/accounts_file.rs 의 FileAccountStore
나쁨: crates/app/src/ports.rs 의 trait TomlAccountRepository
이유: app의 포트 이름은 기술(TOML)을 드러내지 않는다. 포트 구현체는 `<기술><포트 이름>`으로 짓는다. 도메인 prefix는 포트 이름 안에 있다.
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
- 외부 계약이 정한 이름은 계약을 따른다. 예: 운영 로그 필드 `relay_id_ref`, 쿠키 이름 `relay_admin_session`. 이런 이름을 담는 Rust 필드는 계약 이름과 같게 쓴다.
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

`cargo xtask layers`는 작업 0.6에서 만든다. 만들기 전에는 앞의 두 명령만 실행한다. 전체 검증 절차는 개발 환경·검증 how-to(작업 0.8)가 정한다.

## 6. 기존 코드 마이그레이션

이 컨벤션을 만들 때 있던 코드는 `src/main.rs` 하나다. 같은 변경에서 주석을 영어로 바꾸고, `println!`·`eprintln!`을 `std::io::Write`로 바꾸고, 작업 공간 lint를 켰다. 그 밖에 옮길 코드는 없다.

## 7. 이름 짓기 목록

규칙은 [이름 짓기 기본 규칙](../conventions/AGENTS.md#이름-짓기-기본-규칙)을 따른다. Rust에서는 타입·trait 이름과 모듈 파일 이름에 적용한다. 함수 이름에는 적용하지 않는다. 모듈 이름이 이미 prefix를 담으면 타입에서 반복하지 않아도 된다(`matching::WaitingTable`).

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
| `Management` | 관리 리스너, 관리 API | [용어집](../../GLOSSARY.md#관리-기능) |
| `Status` | 상태 리스너 | [용어집](../../GLOSSARY.md#관리-기능) |
| `Admin` | 관리자 계정, 관리자 계정 준비, 관리 세션 | [용어집](../../GLOSSARY.md#관리-기능) |
| `Account` | 관리자 계정 파일 | [용어집](../../GLOSSARY.md#관리-기능) |
| `Close` | 종료 원인 | [용어집](../../GLOSSARY.md#세션-종료와-기록) |
| `RecentClosed` | 최근 종료 목록 | [용어집](../../GLOSSARY.md#세션-종료와-기록) |
| `Event` | 사건, 운영 로그 | [용어집](../../GLOSSARY.md#세션-종료와-기록) |
| `Reload` | 부분 재로드, 재로드 후보 | [용어집](../../GLOSSARY.md#설정과-수명) |
| `Shutdown` | 정상 종료, 종료 유예 | [용어집](../../GLOSSARY.md#설정과-수명) |

suffix 목록:

| suffix | 레이어 | 역할 | 추가 근거 |
| --- | --- | --- | --- |
| `Id` | domain | 식별자 값 타입(`RelayId`, `SessionId`, `ConnId`) | 최초 채택. 계약의 `*_id` 필드와 대응한다. |
| `Parser` | domain | 바이트·문자열을 점진적으로 해석하는 상태 기계(`HandshakeParser`) | 최초 채택. 분할 수신을 다루는 해석기다. |
| `Table` | domain | 키로 찾는 메모리 상태 모음(`WaitingTable`) | 최초 채택 |
| `List` | domain | 순서가 있는 값 목록(`AllowList`, `RecentClosedList`) | 최초 채택. 설정 키 `allow`와 최근 종료 목록에 대응한다. |
| `Policy` | domain | 허용·거절을 판정하는 규칙(`TargetPolicy`) | 최초 채택 |
| `Budget` | domain | 상한이 있는 자원 수 집계(`ConnectionBudget`) | 최초 채택. L·Lm·Lw를 센다. |
| `Settings` | app | 검증을 마친 설정 값(`RelaySettings`) | 최초 채택. 파일 형식과 무관한 값이다. |
| `Service` | app | adapters가 부르는 사용 사례의 입구(`RelayService`, `ManagementService`, `ReloadService`, `LifecycleService`) | 최초 채택 |
| `Store` | app(trait), adapters(구현) | 메모리 밖에 두는 상태의 읽기·저장 포트(`AccountStore`) | 최초 채택 |
| `Sink` | app(trait), adapters(구현) | 사건을 받아 내보내는 포트(`EventSink`) | 최초 채택 |
| `Connector` | app(trait), adapters(구현) | 바깥으로 연결을 여는 포트(`TargetConnector`) | 최초 채택. Mode I 대상 연결이다. |
| `Clock` | app(trait), adapters(구현) | 현재 시각을 주는 포트(`SystemClock`) | 최초 채택. 테스트에서 시각을 바꾸기 위해 둔다. |
| `EntryPoint` | adapters | 중계 접속을 받는 리스너(`TcpEntryPoint`, `WebSocketEntryPoint`) | 최초 채택. 용어집 "진입점"과 같다. |
| `Listener` | adapters | 진입점이 아닌 HTTP 리스너(`ManagementListener`, `StatusListener`) | 최초 채택. 용어집 "관리 리스너"·"상태 리스너"와 같다. |
| `Watcher` | adapters | 파일 변경을 감지하는 작업(`ReloadWatcher`) | 최초 채택 |
| `Error` | 모든 레이어 | 모듈의 오류 enum(`ConfigError`) | 최초 채택 |
