# 테스트 컨벤션

스펙 시나리오와 테스트의 연결을 도구가 읽을 수 있게 하고, 테스트 종류마다 둘 위치를 하나로 정한다. 시나리오 연결은 테스트 함수 위의 문서 주석 표식으로 하고, 커버리지·결과 보고서는 `cargo xtask scenarios`가 표식과 `cargo test` 결과를 모아 만든다. 규칙은 [검증 기록](../AGENTS.md#검증-기록)을 구체화한다.

| 항목 | 값 |
| --- | --- |
| 상태 | Active(2026-10-20 사용자 채택) |
| 적용 대상 | Rust 테스트 코드(단위·통합·계약·E2E), 시나리오 커버리지 보고서 |
| 기준 버전 | Rust 1.99.1 기본 테스트 도구(`cargo test`), tokio 테스트 매크로 |
| 관련 아키텍처 문서 | [코드 지도](../architectures/code-map.md) |
| 관련 ADR | 없음 |
| 적용 시작일 | 2026-10-20 |
| 규칙 준수 확인 방법 | `cargo xtask scenarios`, [Rust 코드 컨벤션](rust.md)의 서식·lint 검사, 코드 리뷰 |

## 1. 규칙 우선순위

[컨벤션 작성 규칙](../conventions/AGENTS.md#규칙-우선순위)을 따른다. 서식·lint·이름 규칙은 [Rust 코드 컨벤션](rust.md)을 따르고, 이 문서는 테스트에만 해당하는 규칙을 더한다.

## 2. 규칙

| 대상 | 해야 할 것 | 피해야 할 것 | 판정 방법 |
| --- | --- | --- | --- |
| 시나리오 표식 | 시나리오를 검증하는 테스트 함수 바로 위 문서 주석에 `/// spec: <specs/ 기준 경로>:<시나리오 ID>`를 쓴다. 테스트가 시나리오 기대 결과의 일부만 확인하면 끝에 ` partial`을 붙인다(예: 시스템 동작 전에 설정 검증 부분만 확인하는 계약 테스트). 여러 시나리오는 줄을 나눠 쓴다. 표식은 테스트 속성(`#[test]`, `#[tokio::test]`)보다 위에 둔다. | 일반 주석(`//`)·테스트 이름에 ID 넣기, 한 줄에 ID 여러 개, 일부만 확인하는 테스트에 ` partial` 빼기 | `cargo xtask scenarios`가 표식을 읽지 못하면 `오류`로 보고한다. ` partial` 여부는 리뷰로 확인한다. |
| 단위 테스트 | 독립된 규칙·상태 전이는 같은 파일의 `#[cfg(test)] mod tests`에 둔다. domain·app 테스트는 소켓·파일을 쓰지 않는다. 시각·난수는 포트(`Clock` 등)로 바꿔 넣는다. | 실제 시각을 기다리는 `sleep`, 테스트 사이 공유 상태 | 리뷰 |
| 통합 테스트 | crate 안 여러 모듈과 루프백 소켓·임시 파일을 함께 쓰는 테스트는 그 crate의 `tests/<주제>.rs`에 둔다. 포트는 `127.0.0.1:0`으로 받아 운영체제가 고르게 한다. | 고정 포트, 저장소 안 파일 쓰기 | 리뷰, 병렬 실행에서 실패 없음 |
| 계약 테스트 | 계약 5절 표가 정한 규칙의 테스트는 해당 crate의 `tests/<계약 이름>_contract.rs`에 둔다(예: `crates/adapters/tests/config_contract.rs`). 같은 변경에서 계약 5절 표의 테스트 위치에 파일 경로와 함수 이름을 적는다. | 계약 5절 표에 없는 위치, 표를 갱신하지 않은 테스트 추가 | 리뷰 |
| 시스템 테스트 | 실행파일 `vnc-relay`를 프로세스로 띄워 확인하는 테스트는 루트 `tests/<주제>.rs`에 둔다. Cargo가 주는 `env!("CARGO_BIN_EXE_vnc-relay")`로 실행파일을 찾는다. | 미리 빌드한 경로를 손으로 적기 | 리뷰 |
| 외부 준비가 필요한 테스트 | 실제 UltraVNC·noVNC·두 번째 OS가 필요한 테스트는 `#[ignore = "<필요한 것>; <how-to 링크>"]`를 붙인다. 보고서는 실행하지 않은 시나리오를 `미실행`으로 표시한다. 대시보드 브라우저 테스트는 Chromium이 없으면 실패해야 하므로 `#[ignore]`를 붙이지 않는다(7절). | 준비가 없을 때 성공으로 끝나는 테스트 | 리뷰, 보고서 |
| 테스트 이름 | 함수 이름은 확인하는 동작을 `snake_case` 문장으로 쓴다(`rejects_second_server_with_same_relay_id`). | `test1`, 시나리오 ID를 이름에 넣기 | 리뷰 |
| 판정 | 기대 결과의 관찰값을 직접 비교한다(`assert_eq!`). 오류는 계약의 오류 코드로 비교한다. | 오류 문장(`message`) 비교, 패닉 없음만 확인 | 리뷰 |
| 비밀 값 | 테스트 비밀번호·키는 테스트 안에서 만든다. 문서용 IP 대역(`192.0.2.0/24` 등)을 쓴다. | 실제 비밀, 공인 주소 | 리뷰 |
| 대시보드 | 정적 HTML·JS를 검사하는 테스트는 `tests/` 아래 Rust 통합 테스트로 둔다. 브라우저 테스트는 Chromium 헤드리스로 페이지를 열고 `data-result`를 읽는다. 세부 위치와 방법은 [프런트엔드 컨벤션](frontend.md)을 따른다(사용자 결정 2026-10-22(Q35)). 표식 문법은 이 문서를 따른다. | 이 문서와 다른 표식, 브라우저 테스트에 `#[ignore]` 붙이기 | 리뷰, 테스트 실행 |

## 3. 예제

```rust
/// spec: relay.md:RLY-013
/// spec: relay.md:RLY-015
#[tokio::test]
async fn rejects_second_server_with_same_relay_id() {
    // ...
}

/// spec: operations.md:OPS-001 partial
#[test]
fn reports_every_problem_in_one_pass() {
    // Checks only the config diagnostics, not the process exit.
}
```

```text
좋음: /// spec: relay.md:RLY-013
나쁨: // RLY-013, fn rly_013_duplicate()
이유: 보고 도구는 문서 주석의 `spec:` 표식만 읽는다. 경로가 없으면 같은 ID가 다른 스펙에 생겼을 때 구분할 수 없다.

좋음: /// spec: operations.md:OPS-001 partial   (설정 진단만 확인하는 계약 테스트)
나쁨: /// spec: operations.md:OPS-001           (같은 테스트)
이유: 보고서가 시나리오 전체를 통과로 표시해, 아직 확인하지 않은 시작 실패·종료 코드까지 확인한 것처럼 보인다.
```

## 4. 예외와 자동 생성 산출물

- 보고서는 자동 생성 산출물이다. Cargo 대상 디렉터리의 `scenario-report.md`에 만들고 저장소에 넣지 않는다. 기본 위치는 `target/scenario-report.md`이고, `CARGO_TARGET_DIR`을 정하면 그 디렉터리에 만든다.
- 시나리오가 아닌 규칙(계약 테스트의 세부 규칙, 내부 함수 검사)에는 표식을 쓰지 않아도 된다.
- 하위 폴더의 스펙은 `specs/`를 뺀 상대 경로를 쓴다(예: `auth/login-screen.md:SCN-003`). 지금은 하위 폴더 스펙이 없다.

## 5. 검증 명령과 통과 기준

전체 검증 절차는 [개발 환경·검증 how-to](../how-to/set-up-and-verify.md)가 정한다. 이 절은 시나리오 보고에 필요한 것만 적는다.

- 실행 환경: Ubuntu 24.04 LTS의 bash 또는 Windows 11의 PowerShell, Rust 1.99.1
- 실행 위치: 저장소 루트
- 성공 조건: 종료 코드 0. 보고서에 `오류`와 `실패`가 없고 `cargo test` 전체(표식이 없는 테스트 포함)가 통과한다. `부분`·`미연결`·`미실행`은 단계가 끝날 때까지 남을 수 있다. 작업 5.9는 `--require-all`로 `부분`·`미연결`·`미실행`도 0건이어야 한다.
- 종료 코드: 0은 통과, 1은 실패 또는 도구 오류(테스트 빌드 실패 포함), 2는 사용법 오류다.

```shell
cargo xtask scenarios
cargo xtask scenarios --require-all
```

## 6. 기존 코드 마이그레이션

이 컨벤션을 만들 때 테스트 코드는 없었다. 옮길 코드는 없다.

## 7. 테스트와 스펙 시나리오 연결

- 메타 정보 방식: 테스트 함수 위 문서 주석의 표식 `/// spec: <specs/ 기준 경로>:<시나리오 ID>`. 한 줄에 하나다. 시나리오의 일부만 확인하는 테스트는 끝에 ` partial`을 붙인다. 그 밖의 낱말이 붙으면 `오류`다.
- 형식 예: `/// spec: relay.md:RLY-001`, `/// spec: operations.md:OPS-001 partial`
- 커버리지·결과 보고 명령: `cargo xtask scenarios`. `docs/specs/`의 스펙(`README.md`·`AGENTS.md` 제외)에서 "수용 테스트" 절 표의 시나리오 ID를 모은다. `xtask`를 뺀 작업 공간 패키지의 `src/`·`tests/` 아래 `.rs` 파일에서 표식을 모은다. `cargo test --workspace --no-fail-fast`를 실행해 테스트별 결과를 대조한다. 동작 방식은 [xtask 모듈 설계](../architectures/xtask.md)에 있다.
- 표식을 둘 수 없는 곳: 여러 통합 테스트가 함께 쓰는 `tests/common/` 같은 하위 모듈은 속한 테스트 대상을 정할 수 없다. 이런 파일의 표식은 `오류`가 된다. 표식은 테스트 함수가 있는 파일에 둔다.
- 대시보드 브라우저 테스트: `tests/` 아래 Rust 통합 테스트가 Chromium을 헤드리스로 실행한다. 컴포넌트·목 응답 시험은 페이지의 `data-result`를 읽는다(Q35). 실서버 시험은 직접 만든 CDP 클라이언트로 실제 화면을 조작한다(사용자 결정 2026-11-05(Q112)). 실패·지연·응답 유실은 시험용 전달 프록시로 만들며, 관리 세션 Si 만료는 실제 300초를 기다린다(Q113). 표식은 Rust 테스트 함수에 두며 스캐너는 바꾸지 않는다. Chromium이 없으면 `#[ignore]` 없이 실패한다. Chromium(Chrome 또는 Edge)은 Windows와 Ubuntu 모두 필요하다. 상세 기준은 [프런트엔드 컨벤션](frontend.md)이 정한다.
- 보고서·CI 결과 위치: Cargo 대상 디렉터리의 `scenario-report.md`(4절). 표준 출력에는 상태별 수, 보고서 경로, 판정을 쓴다. CI는 없다. 로컬 실행 기록은 작업의 완료 보고와, 절차를 바꾼 경우 [개발 환경·검증 how-to](../how-to/set-up-and-verify.md#4-검증)의 검증 표에 남긴다.
- 통과 조건: 5절의 성공 조건과 같다.

보고서의 시나리오별 상태는 다음 중 하나다.

| 상태 | 뜻 |
| --- | --- |
| `통과` | ` partial`이 없는 연결된 테스트가 하나 이상 통과했고 실패한 테스트가 없다. 무시되었거나 실행되지 않은 테스트가 함께 있어도 통과다. 테스트 칸에 테스트마다 결과를 적는다. |
| `부분` | 실패한 테스트가 없고, 통과한 테스트가 모두 ` partial` 표식이다. 시나리오의 나머지 기대 결과는 아직 어떤 테스트도 확인하지 않았다. 테스트 칸에서 해당 테스트 뒤에 `(부분)`을 붙인다. |
| `실패` | 연결된 테스트 중 하나 이상이 실패했다. ` partial` 표식의 테스트도 같다. |
| `미실행` | 연결된 테스트가 있지만 하나도 실행되지 않았다. 무시(`#[ignore]`)되었거나, 이번 실행 결과에 없다(예: 이 OS에서 `#[cfg]`로 빠짐). |
| `미연결` | 연결된 테스트가 없다. |

다음 경우는 시나리오 상태와 따로 `오류`로 나열한다. 위치는 `파일:줄`이다.

- 표식 문법이 틀렸다.
- 표식이 테스트 함수(`#[test]`·`#[tokio::test]`)에 붙어 있지 않다.
- 표식이 가리키는 스펙 파일이나 시나리오 ID가 없다.
- 표식이 있는 파일이 속한 테스트 대상을 정할 수 없다.
- 스펙에 "수용 테스트" 절이 없거나, 표에 시나리오가 없거나, 시나리오 ID가 중복된다.

### 보고 도구 확인 기록

| 대상 | 환경 | 명령 또는 확인 방법 | 결과 |
| --- | --- | --- | --- |
| `xtask`를 추가한 커밋(작업 트리 기준 `9c7ddd0` + `xtask/` 추가 변경) | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `cargo xtask scenarios` | 통과. 시나리오 148건(dashboard 25, management 27, operations 42, relay 27, websocket 27)이 모두 `미연결`, 오류 0건, 판정 통과, 종료 코드 0 |
| 같음 | 같음 | `cargo xtask scenarios --require-all` | 통과. 기대대로 판정 실패(미실행·미연결 148건), 종료 코드 1 |
| 같음 | 같음 | 저장소를 임시 디렉터리로 복사하고 시험용 테스트를 넣어 `cargo xtask scenarios` 실행. 시험 테스트: 통과·실패·무시·`#[cfg(windows)]`, 없는 ID·없는 스펙·테스트가 아닌 함수의 표식, `tests/` 통합 테스트 | 통과. 기대대로 시나리오 상태 통과 3·실패 1·미실행 2(무시됨, 이번 실행에 없음), 오류 3건(없는 ID, 없는 스펙, 테스트가 아닌 함수), `cargo test` 실패, 종료 코드 1. 임시 디렉터리는 삭제했다. |
| 같음 | 같음 | `cargo xtask bogus` | 통과. 사용법 출력, 종료 코드 2 |
| 부분 표식을 추가한 변경(기준 커밋 `4fb7acb` + `xtask/src/scenarios*`·`config_contract.rs` 표식·이 문서 수정) | 같음 | `cargo xtask scenarios` | 통과. 시나리오 148건: 통과 0, 부분 3(OPS-001·OPS-038·OPS-040), 미연결 145, 오류 0건, 판정 통과. OPS-001 행의 테스트마다 `통과 (부분)` 표시 |
| 같음 | 같음 | `cargo xtask scenarios --require-all` | 통과. 기대대로 판정 `실패 (--require-all: 부분·미실행·미연결 시나리오 148건)`, 종료 코드 1 |
| 같음 | 같음 | xtask 단위 테스트 `markers::tests::*`(` partial` 인식, `partly`·`partial extra`는 표식 오류), `report::tests::status_prefers_failure_then_full_pass_then_partial_then_not_run` | 통과 |
