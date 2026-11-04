# 의존성 라이선스를 검사하는 방법

Rust 의존성이 승인된 라이선스 허용 목록을 지키는지 검사하고, 실행파일과 함께 보낼 고지를 만든다. `cargo deny`의 허용 판정과 배포 고지의 최신 상태는 별도 검사다. 독립 구현의 출처 검토는 [4.1절](#41-출처-검토-기록-2026-11-04)에 기록한다. 프로젝트 자체 MIT LICENSE는 저작권자·연도 결정 대기이며 이 문서의 의존성 고지가 대신하지 않는다(사용자 결정 2026-11-04(Q97)).

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 의존성을 추가·갱신하는 개발자, 배포를 준비하는 프로젝트 관리자 |
| 검증 상태 | Verified |
| 검증 환경·버전 | Ubuntu 24.04.4 LTS x86_64(WSL2)와 Windows 11 Pro 10.0.26200, Rust 1.99.1, cargo-deny 0.20.2 |
| 최종 검증일 | 2026-11-04 |
| 관련 기준 문서 | [PRD 확정 기준](../requirements/PRD.md#확정-기준) "구현 기술"·"라이선스" 행, [OPS-021](../specs/operations.md#7-수용-테스트), 허용 목록 [`deny.toml`](../../deny.toml) |

## 1. 대상과 영향

- 대상: 저장소 루트의 `Cargo.lock`이 가리키는 의존성 전체. `deny.toml`의 `[graph] targets`에 따라 Windows(`x86_64-pc-windows-msvc`)와 Ubuntu(`x86_64-unknown-linux-gnu`)의 의존성을 함께 검사한다. 한 OS에서 실행해도 두 대상을 모두 검사한다.
- 예상 사용자 영향: 검사와 `notices --check`는 없다. `cargo xtask notices`는 저장소 루트의 배포 파일 `THIRD-PARTY-NOTICES.txt`를 덮어쓴다. 제품 코드·의존성은 바꾸지 않는다.
- 작업 허용 시간: 제한 없다.

허용 목록은 사용자가 2026-10-20에 승인했다. MIT, Apache-2.0, ISC, BSD-3-Clause, Unlicense, CC0-1.0, BSL-1.0, Unicode-3.0, MIT-0의 9종이다. GPL·LGPL·MPL 등 copyleft 라이선스는 허용하지 않는다. 목록을 바꾸려면 사용자 승인이 필요하다.

허용 라이선스의 고지 의무는 다음과 같다. [3.1절](#31-배포-고지-생성과-대조)의 파일 전체를 실행파일과 함께 배포한다. `OR`는 선택 가능한 조건, `AND`는 함께 지킬 조건이다. 단순히 MIT 전문 하나만 배포해서는 모든 의존성의 조건을 충족하지 못한다.

| 라이선스 | 배포할 때 할 일 |
| --- | --- |
| MIT, ISC, BSD-3-Clause, BSL-1.0, Unicode-3.0 | 저작권 고지와 라이선스 전문을 배포물에 넣는다. BSL-1.0은 소스 배포에만 요구하지만 같은 방식으로 넣는다. |
| Apache-2.0 | 라이선스 전문을 넣는다. 의존성에 `NOTICE` 파일이 있으면 그 내용도 넣는다. |
| CC0-1.0, Unlicense, MIT-0 | 고지 의무가 없다. |

## 2. 전제 조건

- [ ] Rust 툴체인을 설치했다. 버전은 저장소 루트의 `rust-toolchain.toml`이 정한다.
- [ ] 인터넷에 연결되어 있다. cargo-deny가 crates.io 색인과 crate 파일을 내려받는다.
- [ ] 저장소 루트에 `Cargo.lock`과 `deny.toml`이 있다.

## 3. 절차

- 실행 환경: Ubuntu 24.04 LTS의 bash 또는 Windows 11의 PowerShell
- 실행 위치: 저장소 루트
- 바꿔 넣을 값: 없다.
- 최대 대기 시간: cargo-deny 설치는 5분, 검사는 1분이다. 넘으면 Ctrl+C로 중단하고 네트워크 연결을 확인한다.

1. cargo-deny가 설치되어 있는지 확인한다.

   ```shell
   cargo deny --version
   ```

   기대 결과: `cargo-deny 0.20.2`

2. 1단계에서 명령을 찾지 못했거나 버전이 다르면 cargo-deny를 설치한다.

   영향: 사용자 홈의 `.cargo/bin`에 실행파일을 설치한다. `cargo uninstall cargo-deny`로 지울 수 있다.

   ```shell
   cargo install --locked cargo-deny@0.20.2
   ```

   기대 결과: ``Installed package `cargo-deny v0.20.2` ``

3. 라이선스를 검사한다.

   ```shell
   cargo deny check licenses
   ```

   기대 결과: 종료 코드 0과 마지막 줄 `licenses ok`

### 3.1 배포 고지 생성과 대조

루트 [`THIRD-PARTY-NOTICES.txt`](../../THIRD-PARTY-NOTICES.txt)는 두 대상의 일반·빌드 의존성 합집합이다. 각 crate의 이름·버전·라이선스 식과 고지 전문을 담는다. Windows·Linux 배포물에는 같은 파일 전체를 넣는다. 프로젝트의 실제 MIT LICENSE는 별도로 넣어야 한다. 아직 결정되지 않은 저작권자·연도를 이 파일에 대신 넣지 않는다.

생성기는 기존 `xtask`의 std·serde_json만 쓴다. 새 crate와 추가 설치 도구를 쓰지 않기 위한 선택이다. cargo-about·cargo-license 추가 설치, Python 스크립트, 수동 목록 편집보다 기존 Rust 실행 환경을 재사용하기 쉽다. **이 문서의 판단, 사용자 결정 없음**이다. 코드 위치와 처리 순서는 [xtask 설계](../architectures/xtask.md)에 있다.

1. 네트워크가 있는 준비 단계에서 두 대상의 crate를 캐시에 받는다. 이미 준비된 캐시가 있으면 생략한다.

   ```shell
   cargo fetch --locked --target x86_64-pc-windows-msvc
   cargo fetch --locked --target x86_64-unknown-linux-gnu
   ```

   기대 결과: 두 명령 모두 종료 코드 0. Windows·Linux의 Cargo 캐시는 별개이므로 각 호스트에서 준비한다. 고지 검사 자체는 `--offline --locked`만 쓴다.

2. 고지를 처음 만들거나 의존성을 바꿨으면 다시 만든다.

   ```shell
   cargo xtask notices
   ```

   기대 결과: `notices: 통과`, 종료 코드 0. 이 단계만 고지 파일을 바꾼다. crate·파일 상대 경로 순으로 정렬하고 줄 끝을 LF로 통일한다. 실행 시각·개인 경로·Cargo 대상 디렉터리는 넣지 않는다.

3. 생성 결과를 대조한다.

   ```shell
   cargo xtask notices --check
   ```

   기대 결과: `notices --check: 통과`, 종료 코드 0. 차이가 있으면 종료 코드 1과 첫 차이의 줄을 출력한다. crate 목록뿐 아니라 고지 전문도 대조한다. 마지막의 Cargo.lock 입력 전체도 대조하므로 개발 의존성만 바뀌어도 재생성 없이는 실패한다. 줄 끝만 다른 경우는 같은 입력으로 본다.

4. 실행파일과 고지 파일 전체를 같은 배포 묶음에 넣는다.

   기대 결과: 받는 사람이 crate별 라이선스 식·저작권 고지·라이선스 전문·존재하는 NOTICE를 오프라인으로 읽을 수 있다. 고지 파일만 배포하고 프로그램의 MIT LICENSE를 빼도 된다는 뜻은 아니다.

고지 범위는 `cargo tree -p vnc-relay --target <대상> --edges normal,build --prefix none --format "{p}" --charset ascii`로 고른다. 두 대상의 `cargo metadata --format-version 1 --filter-platform <대상>`를 합쳐 이름·버전·라이선스와 패키지 위치를 찾는다. 위 내부 명령에는 모두 `--offline --locked`를 붙인다. 호스트 전용 빌드 간선은 Cargo 트리가 OS마다 다를 수 있으므로 메타데이터의 빌드 의존성과 그 하위 일반·빌드 의존성도 합친다. 실행파일에 링크되지 않는 빌드 도구·절차 매크로까지 보수적으로 고지한다. 개발 전용 의존성과 xtask 자체는 배포 목록의 시작점이 아니다. 제품 기능이 바뀌면 다시 생성해야 한다.

생성기는 각 crate 안의 `LICENSE*`·`LICENCE*`·`COPYING*`·`NOTICE*`·`UNLICENSE*`·`COPYRIGHT*` 파일을 대소문자 구분 없이 재귀 수집한다. Cargo의 `license_file`도 포함한다. 하위 고지를 놓치지 않기 위해 배포에 쓰이지 않는 고지까지 들어갈 수 있다. 해당 파일을 넣었다고 선택적 라이선스의 모든 가지를 채택하는 것은 아니다. 고지 파일 누락·읽기 실패·심볼릭 링크·메타데이터 연결 실패는 성공으로 숨기지 않고 중단한다. 구현 소스의 코드 본문은 고지 파일에 복사하지 않는다.

`tests/dependency_notices.rs`는 위 대조를 오프라인으로 실행한다. 내부 명령은 `cargo run --offline --locked --quiet -p xtask -- notices --check`다. Cargo가 위치를 정하므로 `CARGO_TARGET_DIR`의 특정 값에 의존하지 않는다. 개별 실행은 `cargo test --test dependency_notices`다. 기존 Windows·WSL 게이트는 모두 `cargo xtask scenarios`를 호출한다. 이 도구가 `cargo test --workspace --no-fail-fast`로 이 시험도 실행한다. `xtask/src/scenarios.rs`는 xtask 시험을 제외하므로 OPS-021 표식은 루트 시험에만 `partial`로 붙인다. 이 시험은 프로젝트 MIT 고지와 독립 구현의 법적 판정을 확인하지 않는다.


## 4. 검증

[검증 기록](../AGENTS.md#검증-기록)에 따라 실제 결과와 성공 조건을 대조한다.

| 관찰 대상 | 성공 조건 | 확인 명령·방법 | 실제 결과 | 실행 증거 위치 |
| --- | --- | --- | --- | --- |
| 채택 라이브러리 묶음의 라이선스 | `licenses ok`, 종료 코드 0 | 아래 묶음으로 만든 임시 crate에서 3단계 명령 | 통과(2026-10-20). 잠금 파일 166개 패키지, 오류·경고 없음 | 아래 실행 기록 1 |
| 저장소의 현재 의존성 | `licenses ok`, 종료 코드 0 | 3단계 명령 | 통과(2026-10-20). 외부 의존성은 아직 없다. | 아래 실행 기록 2 |
| 채택 묶음의 고지 파일 | 모든 패키지에 라이선스 파일이 있다. `NOTICE` 파일 유무를 안다. | 두 대상의 의존성 패키지 디렉터리에서 파일 이름 확인 | 통과(2026-10-20). 142개 패키지 모두 라이선스 파일이 있고 `NOTICE` 파일은 없다. | 아래 실행 기록 1 |
| socket2 직접 의존성(작업 2.5) | `licenses ok`, 종료 코드 0. 라이선스 파일이 있다. `NOTICE` 파일 유무를 안다. | 3단계 명령, 패키지 디렉터리에서 파일 이름 확인 | 통과(2026-10-20). socket2 0.6.5는 `MIT OR Apache-2.0`이고 `LICENSE-MIT`·`LICENSE-APACHE`가 있으며 `NOTICE` 파일은 없다. | 아래 실행 기록 3 |
| axum 직접 의존성(작업 2.6) | `licenses ok`, 종료 코드 0. 새로 들어온 패키지마다 라이선스 파일이 있다. `NOTICE` 파일 유무를 안다. | 3단계 명령, 패키지 디렉터리에서 파일 이름 확인 | 통과(2026-10-20). 새 패키지 23개 모두 라이선스 파일이 있고 `NOTICE` 파일은 없다. | 아래 실행 기록 4 |
| argon2·rpassword 직접 의존성(작업 4.1) | `licenses ok`, 종료 코드 0. 새로 들어온 패키지마다 라이선스 파일이 있다. `NOTICE` 파일 유무를 안다. | 3단계 명령, 패키지 디렉터리에서 파일 이름 확인 | 통과(2026-10-20). 새 패키지 15개 모두 라이선스 파일이 있고 `NOTICE` 파일은 없다. | 아래 실행 기록 5 |
| 작업 3.1 직접 의존성(rustls 0.23.46, rustls-pki-types 1.15.1, tokio-rustls 0.26.6, tokio `io-util`) | `licenses ok`, 종료 코드 0. 새로 들어온 패키지마다 라이선스 파일이 있다. `NOTICE` 파일 유무를 안다. | 3단계 명령, 패키지 디렉터리에서 파일 이름 확인 | 통과(2026-10-21). 새 패키지 19개 모두 라이선스 파일이 있고 `NOTICE` 파일은 없다. Ubuntu와 Windows 모두 `licenses ok`다. | 아래 실행 기록 6 |
| windows-sys 직접 의존성(작업 4.1, Windows 전용) | `licenses ok`, 종료 코드 0. 새로 들어온 패키지마다 라이선스 파일이 있다. `NOTICE` 파일 유무를 안다. | 3단계 명령, 잠금 파일 변화와 패키지 디렉터리에서 파일 이름 확인 | 통과(2026-10-21). 잠금 파일에 새 패키지는 없다. windows-sys 0.61.2는 MIT OR Apache-2.0이고 `license-mit`·`license-apache-2.0`이 있으며 `NOTICE` 파일은 없다. Windows와 Ubuntu 모두 deny 단계를 통과했다. | 아래 실행 기록 7 |
| 작업 3.2 직접 의존성(tokio-tungstenite 0.30.0, adapters, WS·WSS 업그레이드) | `licenses ok`, 종료 코드 0. 새로 들어온 패키지마다 라이선스 파일이 있다. `NOTICE` 파일 유무를 안다. | 3단계 명령, 잠금 파일 변화와 패키지 디렉터리에서 파일 이름 확인 | 통과(2026-10-21). 새 패키지 12개 모두 라이선스 파일이 있고 `NOTICE` 파일은 없다. Windows와 Ubuntu 모두 deny 단계가 `licenses ok`였다. | 아래 실행 기록 8 |

실행 기록 1: 의존성을 추가하기 전에 [PRD "구현 기술" 행](../requirements/PRD.md#확정-기준)의 라이브러리를 검토했다.

- 대상: 저장소 밖의 임시 crate(`license = "MIT"`)에 아래 직접 의존성을 넣고 `cargo generate-lockfile`로 잠금 파일을 만들었다. 저장소의 `deny.toml`을 복사해 검사했다. 임시 crate는 남기지 않았다.
- 직접 의존성: tokio 1.54.0, rustls 0.23.46(`aws_lc_rs`), aws-lc-rs 1.18.2, tokio-rustls 0.26.6, rustls-pki-types 1.15.1, axum 0.8.9, tokio-tungstenite 0.30.0, toml 1.1.8, serde 1.0.229, serde_json 1.0.154, argon2 0.6.0, notify 8.2.0, clap 4.6.7, tracing 0.1.44, tracing-subscriber 0.3.23, rpassword 7.5.4
- 환경: Ubuntu 24.04.4 LTS x86_64(WSL2), Rust 1.99.1, cargo-deny 0.20.2
- 결과: `licenses ok`, 종료 코드 0. 발견한 라이선스는 허용 목록 9종뿐이었다.
- 고지 파일 확인: `cargo metadata --filter-platform`으로 두 대상의 의존성(일반·빌드 의존성, 142개 패키지)과 패키지 경로를 구했다. 각 패키지 디렉터리에서 `LICENSE*`·`COPYING*`·`UNLICENSE*`·`NOTICE*` 파일 이름을 확인했다. 모든 패키지에 라이선스 파일이 있었고, Apache-2.0의 `NOTICE` 파일을 둔 패키지는 없었다.
- 남은 일: 3단계 검사는 crate의 라이선스 식과 라이선스 파일로만 판정하고 고지 파일을 만들지 않는다. 배포물에 넣을 의존성 고지와 이 프로젝트의 MIT 저작권 고지는 [OPS-021](../specs/operations.md#7-수용-테스트)에서 확인한다.

실행 기록 2: 커밋 `17a9305`에 `deny.toml`을 더한 작업 트리에서 실행했다. 환경은 실행 기록 1과 같다. 결과는 `licenses ok`, 종료 코드 0이다.

실행 기록 3: 커밋 `c89beb4`에 작업 2.5를 더한 작업 트리에서 실행했다. 환경은 실행 기록 1과 같다. socket2 0.6.5는 그 전부터 tokio의 의존성으로 `Cargo.lock`에 있었고, 작업 2.5가 adapters의 직접 의존성(`features = ["all"]`)으로 더했다. 결과는 `licenses ok`, 종료 코드 0이다. `~/.cargo/registry/src/index.crates.io-*/socket2-0.6.5/`의 파일 이름과 `Cargo.toml`의 `license` 값을 확인했다.

실행 기록 4: 커밋 `b1f2ad5`에 작업 2.6을 더한 작업 트리에서 실행했다. 환경은 실행 기록 1과 같다. 작업 2.6이 adapters의 직접 의존성으로 axum 0.8.9(`default-features = false`, `features = ["http1", "tokio"]`)를 더했다. 이 변경으로 `Cargo.lock`에 23개 패키지가 새로 들어왔다. atomic-waker 1.1.2, axum 0.8.9, axum-core 0.5.6, bytes 1.12.1, futures-core·futures-task·futures-util 0.3.34, http 1.5.0, http-body 1.1.0, http-body-util 0.1.5, httparse 1.10.1, httpdate 1.0.3, hyper 1.12.0, hyper-util 0.1.21, matchit 0.8.4, mime 0.3.17, percent-encoding 2.3.2, slab 0.4.12, smallvec 1.16.3, sync_wrapper 1.0.2, tower 0.5.3, tower-layer·tower-service 0.3.3이다. 결과는 `licenses ok`, 종료 코드 0이다. 패키지마다 `~/.cargo/registry/src/index.crates.io-*/<이름>-<버전>/`의 `LICENSE*`·`COPYING*`·`UNLICENSE*`·`NOTICE*` 파일 이름과 `Cargo.toml`의 `license` 값을 확인했다. 라이선스는 `MIT`, `Apache-2.0`, `MIT OR Apache-2.0`, `Apache-2.0 OR MIT`, `MIT AND BSD-3-Clause`(matchit)였다.

실행 기록 5: 커밋 `ed68104`에 작업 4.1을 더한 작업 트리에서 실행했다. 환경은 실행 기록 1과 같다. 같은 실행을 Windows 11 Pro 10.0.26200(cargo-deny 0.20.2)에서도 했다. 작업 4.1이 adapters의 직접 의존성으로 argon2 0.6.0(`default-features = false`, `features = ["alloc", "password-hash"]`)과 rpassword 7.5.4를 더했다. 이 변경으로 `Cargo.lock`에 15개 패키지가 새로 들어왔다. argon2 0.6.0, base64ct 1.8.3, blake2 0.11.0, block-buffer 0.12.1, cmov 0.5.4, cpufeatures 0.3.1, crypto-common 0.2.2, ctutils 0.4.3, digest 0.11.3, hybrid-array 0.4.15, password-hash 0.6.1, phc 0.6.1, rpassword 7.5.4, rtoolbox 0.0.6, typenum 1.20.1이다. 결과는 두 환경 모두 `licenses ok`, 종료 코드 0이다. 패키지마다 레지스트리 디렉터리의 `LICENSE*`·`COPYING*`·`UNLICENSE*`·`NOTICE*` 파일 이름과 `Cargo.toml`의 `license` 값을 확인했다. rpassword와 rtoolbox의 라이선스는 `Apache-2.0`이고 `LICENSE-APACHE`만 있다. 나머지는 `MIT OR Apache-2.0`이고 `LICENSE-MIT`·`LICENSE-APACHE`가 있다. typenum은 `LICENSE` 파일도 있다. `NOTICE` 파일은 없다.

실행 기록 5 재실행(2026-10-21): 커밋 `1aed781`과 작업 트리에서 3단계 명령을 다시 실행했다. 작업 트리의 변경은 `Cargo.toml`·`Cargo.lock`을 건드리지 않았다. 결과는 WSL과 Windows 11 모두 `licenses ok`, 종료 코드 0이다. 고지 파일 확인은 다시 하지 않았다.

실행 기록 6: 커밋 `b7b43cb`에 작업 3.1을 더한 작업 트리에서 실행했다. 환경은 실행 기록 1과 같다. 같은 실행을 Windows 11 Pro 10.0.26200에서도 했다. 작업 3.1이 adapters의 직접 의존성으로 rustls 0.23.46(`default-features = false`, `features = ["std", "tls12", "aws_lc_rs"]`), rustls-pki-types 1.15.1(`std`), tokio-rustls 0.26.6(`tls12`, `aws_lc_rs`)을 더했고, `[dev-dependencies]`에 tokio의 `io-util`을 더했다. 이 변경으로 `Cargo.lock`에 19개 패키지 항목이 새로 들어왔다. 직접 의존성 세 개 외의 패키지는 getrandom 0.2.17, once_cell 1.21.4, ring 0.17.14, rustls-webpki 0.103.16, subtle 2.6.1, untrusted 0.9.0, windows-sys 0.52.0, windows-targets 0.52.6, windows_aarch64_gnullvm·windows_aarch64_msvc·windows_i686_gnu·windows_i686_gnullvm·windows_i686_msvc·windows_x86_64_gnu·windows_x86_64_gnullvm·windows_x86_64_msvc 0.52.6이다.

라이선스는 모두 허용 목록 안에 있다. rustls-pki-types, tokio-rustls, getrandom, once_cell, windows-sys, windows-targets, windows_* 패키지는 MIT OR Apache-2.0이고 `LICENSE-MIT`·`LICENSE-APACHE` 또는 소문자 `license-mit`·`license-apache-2.0`이 있다. rustls는 Apache-2.0 OR ISC OR MIT이고 `LICENSE-APACHE`·`LICENSE-ISC`·`LICENSE-MIT`가 있다. ring은 Apache-2.0 AND ISC이고 `LICENSE`·`LICENSE-BoringSSL`·`LICENSE-other-bits`가 있다. rustls-webpki는 ISC(`LICENSE`), untrusted 0.9.0은 ISC(`LICENSE.txt`), subtle은 BSD-3-Clause(`LICENSE`)다. 19개 패키지 모두 라이선스 파일이 있고 `NOTICE` 파일은 없다. 결과는 Ubuntu와 Windows 모두 `licenses ok`, 종료 코드 0이다.

ring 0.17.14는 rustls-webpki의 `alloc` 기능이 `ring?/alloc`으로 켜는 약한 간선이다. `cargo tree -i ring`에 출력이 없으므로 컴파일되지 않는다. 잠금 파일에는 남는다. 배포 고지 의무는 [OPS-021](../specs/operations.md#7-수용-테스트)에서 확인한다.

실행 기록 7: 커밋 `f21a6bc`에 작업 4.1의 콘솔 입력 모드 복원을 더했다. 측정 커밋은 `ec2e6e6`이다. 환경은 실행 기록 1과 같고, Windows에서는 실행 기록 5의 Windows 환경이다. 이 변경으로 adapters의 Windows 전용 직접 의존성 `windows-sys` 0.61.2(`default-features = false`, `features = ["Win32_Foundation", "Win32_Security", "Win32_Storage_FileSystem", "Win32_System_Console"]`)가 생겼다. `f21a6bc`는 `Cargo.lock`을 바꾸지 않았다. 0.61.2는 그 전부터 잠금 파일에 있었다. 라이선스는 `Cargo.toml`의 `license = "MIT OR Apache-2.0"`과 패키지 디렉터리의 `license-mit`·`license-apache-2.0` 파일로 확인했다. `NOTICE` 파일은 없다. 결과: Windows의 `vr-win.ps1 -Only all`에서 deny 단계가 종료 코드 0이고 `licenses ok`를 출력했다. Ubuntu의 `vr-full.sh`는 `ALL PASSED`로 끝났다. `deny.toml`의 대상에 Windows가 있으므로 이 검사는 Ubuntu에서도 windows-sys를 본다.

실행 기록 8: 커밋 `ba80d7b`에 작업 3.2(WS·WSS 업그레이드, 101 응답까지)를 더했다. 3단계 명령은 커밋 `35cbcb4`(작업 트리 변경 없음)에서 두 OS의 검증 스크립트로 실행했다. Ubuntu `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh`와 Windows 11 Pro 10.0.26200 `vr-win.ps1 -Only all`이다. 두 실행 모두 `licenses ok`, `deny exit=0`이었다.

직접 의존성: 루트 `Cargo.toml`에 워크스페이스 의존성 `tokio-tungstenite = { version = "0.30.0", default-features = false, features = ["handshake"] }`를 더했다. `crates/adapters/Cargo.toml`에는 `tokio-tungstenite.workspace = true`를 더했다.

잠금 파일: `Cargo.lock`에 115줄이 더해졌고 지워진 줄은 없다. 새 패키지 항목 12개는 chacha20 0.10.2, const-oid 0.10.2, data-encoding 2.11.1, futures-sink 0.3.34, log 0.4.34, rand 0.10.3, rand_core 0.10.1, sha1 0.11.0, thiserror 2.0.21, thiserror-impl 2.0.21, tokio-tungstenite 0.30.0, tungstenite 0.30.0다. 기존 패키지 digest, futures-util, getrandom에는 의존 간선만 더해졌다.

라이선스: Windows의 `%USERPROFILE%\.cargo\registry\src\index.crates.io-*\<이름>-<버전>\`에서 `Cargo.toml`의 `license` 값과 `LICENSE*`·`COPYING*`·`NOTICE*` 파일 이름을 확인했다.

| 패키지 | `license` | 라이선스 파일 |
| --- | --- | --- |
| chacha20 0.10.2 | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| const-oid 0.10.2 | Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-MIT` |
| data-encoding 2.11.1 | MIT | `LICENSE` |
| futures-sink 0.3.34 | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| log 0.4.34 | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| rand 0.10.3 | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| rand_core 0.10.1 | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| sha1 0.11.0 | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| thiserror 2.0.21 | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| thiserror-impl 2.0.21 | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| tokio-tungstenite 0.30.0 | MIT | `LICENSE` |
| tungstenite 0.30.0 | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |

`NOTICE` 파일을 둔 패키지는 없다. 배포물의 의존성 고지는 실행 기록 1과 같이 [OPS-021](../specs/operations.md#7-수용-테스트)에서 확인한다.

### 4.1 출처 검토 기록 (2026-11-04)

확인한 자료와 작업 트리는 독립 구현 경위를 뒷받침한다. 다만 문자열 검색과 로컬 이력만으로 복사·번역·포팅의 부재를 증명할 수는 없다. 엄격한 클린룸 인증이나 법률 의견을 완료했다고 주장하지 않는다([PRD 9절](../requirements/PRD.md#9-호환성-조사와-독립-구현-원칙)).

- 대상 버전: 검토 시작 시 로컬 HEAD 이력 `24e97b4b1aaee5451a6435a303ee0335e6841ebf`까지와 이번 고지 생성기·루트 시험·문서 변경을 포함한 작업 트리. 동시에 진행 중인 다른 작업의 변경은 이 검토의 작성 이력으로 소급하지 않는다.
- 범위: `src/`, `crates/`, `web/dashboard/`, `xtask/`의 파일 이름·폴더 구조와 본문. TLS 시험용 PEM도 파일 목록에 포함했다. 빌드 출력·Cargo 캐시·저장소 밖 관찰 배포본은 작성 제품 코드가 아니다.
- 방법: Git 명령은 실행하지 않았다. 로컬 `.git/logs/HEAD`의 변경 식별자·주체·제목, 아래 계약, 현재 파일 목록과 문자열 검색을 대조했다. reflog 주체는 커밋 작성자 전체를 인증하거나 법적 저작권자를 결정하는 자료가 아니다.
- 금지 범위 준수: 이번 검토에서 UltraVNC·websockify·noVNC 소스를 열거나 비교하지 않았다. 이전 문서에 외부 소스·설치 스크립트 링크가 있어도 이번 검토는 그 링크를 열지 않았다.

| 근거 | 확인한 사실 | 해석의 한계 |
| --- | --- | --- |
| 로컬 HEAD 이력의 초기 `153c9f6`·Rust 골격 `17a9305`·설정 `4fb7acb`·xtask `b7d23ee`·TLS `ead7be0` | 요구사항·설계 → Rust 골격 → 기능별 구현이라는 변경 경로가 있다. 해당 reflog 주체는 `kayden.kim`이다. | 제목·주체는 코드의 독창성이나 저작권 귀속을 보증하지 않는다. reflog는 로컬 자료이며 배포 파일에 포함하지 않는다. |
| 대시보드 `d504667`·`aaca485`·`023d2bd`·`172764a` | 공통 컴포넌트 → 로그인 → 운영 현황 → 세션 상세의 구현 기록이 있다. 해당 reflog 주체도 `kayden.kim`이다. | UI의 유사성·외부 디자인 권리 전체를 판정하는 자료는 아니다. |
| 초기 교환 `5e53570`·스트림 코덱 `4391b14` | 관찰·계약 채택 뒤 Rust 초기 교환과 RFC 6455 스트림 코덱을 구현한 변경 제목이 있다. | 과거 모든 작업자의 열람 행위나 소스 유사도를 조사한 것은 아니다. |
| [관찰 기록](../contracts/repeater-compat-record.md) 1·2·3절 | 소유자 허용, 고정 배포본 해시, 분리 VM, 입력·관찰·해석을 분리한 근거가 있다. | 사용 권한의 법률 검토를 완료한 것은 아니다. |
| [초기 교환 계약](../contracts/repeater-handshake-v1.md) 1절·1.4절 | 250바이트 요청·12바이트 시작 값·ID 규칙이 관찰 증거와 사용자 결정에 연결된다. 계약은 외부 세 프로젝트의 소스를 읽거나 복사·번역하지 않도록 정한다. | 같은 통신 상수는 호환 계약의 결과다. 상수가 같다는 이유로 소스가 같거나 다르다고 단정하지 않는다. |
| 파일 목록 | 위 네 영역에 vendor·node_modules·외부 세 프로젝트의 소스 트리나 배포 아카이브가 없다. Rust·HTML·CSS·JS와 프로젝트 설정·시험 PEM으로 구성된다. | 이름을 바꿔 넣은 코드의 부재까지 증명하지는 못한다. |

본문 검색은 `(?i)copyright|spdx|license|websockify|ultravnc|novnc`를 위 네 영역 전체에 적용했다. 나온 항목은 아래와 같이 모두 분류했다. 별도 외부 라이선스 머리말은 찾지 못했다.

| 위치·검색 결과 | 분류 |
| --- | --- |
| `src/cli.rs`의 `UltraVNC` | CLI 설명에 쓰는 호환 대상 이름이다. 외부 코드 고지가 아니다. |
| `crates/{domain,app,adapters}/Cargo.toml`, `xtask/Cargo.toml`의 `license.workspace = true` | 이 저장소가 정한 패키지 메타데이터다. 실제 MIT 저작권 고지를 작성했다는 뜻은 아니다. |
| `xtask/src/scenarios/markers.rs`의 `needs UltraVNC` | 표식 스캐너 시험의 가상 입력 문자열이다. |
| `xtask/src/scenarios/test_run.rs`의 `needs_ultravnc`·`needs UltraVNC` 두 항목 | 무시된 시험 출력과 기대 키를 검사하는 시험 데이터다. |
| `xtask/src/main.rs`의 라이선스 how-to 경로, `xtask/src/notices.rs`의 모든 `license`·`copyright` 항목 | 이번 고지 생성기의 필드·출력·파일 이름 판별 상수·회귀 시험이다. 외부 구현의 머리말이 아니다. |
| `websockify`, `noVNC`, `SPDX` | 네 영역에 일치 항목이 없다. `web/dashboard/`에는 위 검색 전체가 일치하지 않는다. |

`permission is hereby|redistribution and use|all rights reserved|gnu general public|spdx-license-identifier`도 대소문자 구분 없이 검색했다. 위 네 영역에 일치 항목이 없다. 별도 생성 산출물 `THIRD-PARTY-NOTICES.txt`에는 의도적으로 외부 고지 전문을 보존한다. 이는 외부 구현 소스를 vendor로 넣은 것이 아니다.

대시보드의 HTML `script`, JS `import`·`require(`·동적 `import(`, `http://`·`https://`, CSS `@import`·`url(`를 정적 검색했다. `app/`·`login/`의 모듈 경로는 모두 자체 파일의 상대 경로나 `/app/main.js`다. `preview/`·`tests/`도 자체 모듈만 참조한다. 외부 CDN·bare package import·import map·node_modules는 없다. 유일한 HTTP 형태 값은 `app/components/gauge.js`의 SVG 네임스페이스 `http://www.w3.org/2000/svg`다. 이는 브라우저 DOM 이름 공간이며 JS 다운로드 주소가 아니다. CSS 외부 import·URL도 없다. 따라서 현재 정적 대시보드에는 외부 JS 의존성이 없다는 구조 증거를 확보했다. 축소·번역한 코드를 사람이 숨겨 넣지 않았다는 수학적 증명은 아니다.

재검토 때는 PowerShell의 `Get-ChildItem src,crates,web/dashboard,xtask -Recurse -File | Select-String -Pattern 'copyright|spdx|license|websockify|ultravnc|novnc'`로 같은 범위를 검색할 수 있다. 현재 기록은 저장소 검색 도구와 전체 파일 목록으로 확인했다. 이 PowerShell 명령은 이번에 별도로 실행하지 않았다. 외부 세 프로젝트의 원본 소스 비교를 재검토 절차로 추가하지 않는다.

### 4.2 배포 의존성·암호 모듈 검토 (2026-11-04)

현재 고지에는 외부 crate 110개가 있다. 두 대상의 일반·빌드 의존성 합집합이며 Windows 전용 `windows-sys`·Linux 전용 `inotify`·빌드 도구도 포함한다. PRD 승인 기술 중 현재 쓰는 tokio·rustls·tokio-rustls·rustls-pki-types·axum·tokio-tungstenite·toml·serde_json·argon2·rpassword·notify·clap과 암호 모듈 aws-lc-rs·aws-lc-sys가 들어간다. serde는 현재 직접 쓰지 않고 toml 등에서 `serde_core`를 쓴다. 승인된 기술을 아직 직접 의존하지 않는 경우와 배포 고지 누락을 구분한다. 과거 채택 묶음 검사는 이 절 위의 실행 기록 1에 남아 있다.

| 암호 모듈 | Cargo 메타데이터의 라이선스 식 | 배포 고지에 보존한 내용 |
| --- | --- | --- |
| aws-lc-rs 1.18.2 | `ISC AND (Apache-2.0 OR ISC)` | 패키지 LICENSE의 ISC와 Apache-2.0 전문·고지 |
| aws-lc-sys 0.46.0 | `ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND MIT AND BSD-3-Clause AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR ISC OR MIT-0)` | 패키지 LICENSE·`aws-lc/LICENSE`·`aws-lc/third_party/fiat/LICENSE`. Amazon·Google·OpenSSL·Eric Young 등 원래 고지와 전문을 유지한다. |

현재 aws-lc-sys의 LICENSE는 OpenSSL 유래 부분을 **Apache-2.0**으로 명시한다. 별도의 구형 `OpenSSL` SPDX 식이라고 추측하거나 허용 목록을 늘리지 않았다. BoringSSL 유래의 기존 부분은 ISC, 새 부분은 Apache-2.0이다. Jitter Entropy 부분은 Amazon이 GPL 가지가 아니라 BSD-3-Clause를 선택했다고 고지한다. 생성 파일은 이 설명과 원래 저작권을 그대로 담는다. `AND`의 의무와 선택된 `OR` 가지의 조건을 지켜야 하며, 암호 모듈 전체를 ISC 하나로만 다루지 않는다. 독립된 NOTICE 파일이 있으면 생성기가 재귀 수집한다.

`cargo deny check licenses`는 현재 잠금 파일 전체의 식과 허용 목록을 검사했다. 생성기 대조는 파일이 최신인지 확인할 뿐, 허용 목록 판정이나 법률 검토를 대체하지 않는다. 현재 제품 코드에 외부 구현을 직접 복사하거나 수정한 것으로 분류한 항목은 없다. [INFERENCE] 계약·이력·현재 트리 증거는 독립 구현 주장과 일치한다. 이 추론은 외부 원본과의 소스 대조를 수행했다는 뜻이 아니다.

남은 OPS-021 차단 항목은 Q97의 실제 MIT 저작권자·적용 연도 결정과 LICENSE 작성·검토다. 이번 검토 범위에서 그 밖의 미완료 항목은 발견하지 않았다. 실제 배포 묶음에는 이 고지 전체가 포함되었는지 배포 단계에서 다시 확인해야 한다. 아직 생성되지 않은 배포 아카이브를 검증했다고 주장하지 않는다.

### 4.3 실행 기록 (2026-11-04)

대상은 4.1절의 기준 커밋에 이번 `xtask/src/notices.rs`·명령 분기·루트 시험·고지 파일·문서 변경을 더한 작업 트리다. Cargo.toml·Cargo.lock·제품 Rust·대시보드 파일은 이 작업에서 바꾸지 않았다. WSL은 `CARGO_TARGET_DIR=$HOME/vnc-relay-target`, Windows는 Cargo의 기본 대상 디렉터리를 썼다. Windows는 기존 빌드 조건인 `AWS_LC_SYS_PREBUILT_NASM=1`을 썼다.

| 환경 | 명령·확인 방법 | 결과 |
| --- | --- | --- |
| Windows 파일 시스템 | 4.1절의 소스 문자열 검색·영역별 전체 파일 목록·로컬 reflog·관찰 계약 대조 | 위 표에 모든 일치 항목을 분류했다. 외부 소스 비교는 하지 않았다. |
| WSL Ubuntu 24.04.4, Rust 1.99.1 | `cargo metadata --locked --format-version 1 --filter-platform x86_64-pc-windows-msvc` | 종료 코드 0. aws-lc-rs·aws-lc-sys의 버전·식과 캐시 경로 확인. |
| 같음, cargo-deny 0.20.2 | `cargo deny check licenses` | 통과. 종료 코드 0, `licenses ok`. 두 대상·현재 잠금 파일 전체가 허용 목록을 지킨다. |
| 같음 | `cargo xtask notices` | 통과. 종료 코드 0. 외부 crate 110개의 목록과 수집한 고지 전문을 생성했다. |
| Windows 11 Pro 10.0.26200, Rust 1.99.1 | `cargo run --offline --locked --quiet -p xtask -- notices --check` | 통과. 종료 코드 0. WSL에서 생성한 파일을 Windows에서도 그대로 재현했다. |
| WSL Ubuntu | `cargo test --offline --locked -p xtask` | 통과. 14 passed, 0 failed. 고지 이름 판별·트리 이름/버전 수집 시험 2개 포함. |
| WSL Ubuntu | `cargo test --offline --locked --test code_map --test dependency_notices` | 통과. 코드 지도 4개, 오프라인 고지 대조 1개. 각 0 failed. |
| Windows | `cargo test --offline --locked --test code_map --test dependency_notices` | 통과. 코드 지도 4개, 오프라인 고지 대조 1개. 각 0 failed, 종료 코드 0. |
| WSL Ubuntu | `rustfmt --edition 2024 --config skip_children=true xtask/src/main.rs xtask/src/notices.rs tests/dependency_notices.rs` | 통과. 마지막 수정한 notices.rs도 같은 명령으로 포맷했다. Windows 셸의 `rustfmt`는 PATH에서 찾지 못해 WSL의 고정 도구를 썼다. |
| Windows | `python D:/vr-tools/tmp/doc_check.py` | `errors_total=4 known=4 other=0`. 기존 오류만 남았다: dashboard-operations-overview.md:73, tls.md:126, verify-dashboard-frontend.md:179·873. 루트 README.md는 기존처럼 검사 대상 밖이다. |

개발 중 첫 메타데이터 연결은 Windows 타깃에서 호스트 빌드 도구 `pkg-config`를 놓쳐 중단했다. 메타데이터 합집합으로 고쳤다. 이어 첫 Windows 대조는 호스트 빌드 간선 차이로 109개와 110개가 달라 실패했다. 메타데이터의 빌드 의존성 하위 그래프를 합친 뒤 위 최종 시험이 두 OS에서 통과했다. 누락을 무시하거나 OS별 정답 파일을 나누지 않았다.

전체 게이트·시나리오 보고서 생성은 이번 작업에서 실행하지 않았다. OPS-021 전체 통과는 MIT LICENSE가 없으므로 주장하지 않는다.



## 5. 중단 조건

- `error[rejected]`가 나오면 그 의존성을 병합하지 않는다. 다른 crate를 찾거나 사용자에게 허용 목록 검토를 요청한다. `deny.toml`의 허용 목록을 승인 없이 늘리지 않는다.
- `error[unlicensed]`가 나오면 해당 crate의 라이선스 파일을 직접 확인하고 사용자 검토를 받는다.
- `notices --check`가 실패하면 그 배포를 중단한다. 3.1절에 따라 캐시를 준비하고 생성 결과와 변경된 고지를 검토한다. 고지 누락 오류를 빈 파일로 덮지 않는다.

## 6. 롤백 또는 복구

1. `cargo deny`와 `notices --check`는 파일을 바꾸지 않으므로 되돌릴 것이 없다.
2. 생성 결과가 잘못되었으면 원인을 고친 뒤 `cargo xtask notices`로 다시 만든다. 고지 전문을 손으로 줄이지 않는다.
3. 의존성 변경으로 허용 검사가 실패했으면 그 의존성 변경을 취소하거나 승인을 받은 뒤 두 검사를 다시 실행한다.

## 7. 해결되지 않을 때 전달할 대상

- 전달 대상: vnc-relay 프로젝트 관리자
- 보존할 증거: `cargo deny check licenses`의 전체 출력, 문제 crate의 이름·버전, 추가하려던 변경
- 비밀 값 제외 규칙: 이 검사는 비밀 값을 다루지 않는다.
