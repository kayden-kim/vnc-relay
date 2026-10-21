# 의존성 라이선스를 검사하는 방법

Rust 의존성의 라이선스가 사용자가 승인한 허용 목록 안에 있는지 검사한다. 검사가 통과하면 모든 의존성이 허용 라이선스이고, 실패하면 승인되지 않은 라이선스가 들어온 것이다. 의존성을 추가·갱신하는 변경과 배포 전에 이 절차를 실행한다.

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 의존성을 추가·갱신하는 개발자, 배포를 준비하는 프로젝트 관리자 |
| 검증 상태 | Verified |
| 검증 환경·버전 | Ubuntu 24.04.4 LTS x86_64(WSL2)와 Windows 11 Pro 10.0.26200, Rust 1.99.1, cargo-deny 0.20.2 |
| 최종 검증일 | 2026-10-21 |
| 관련 기준 문서 | [PRD 확정 기준](../requirements/PRD.md#확정-기준) "구현 기술"·"라이선스" 행, [OPS-021](../specs/operations.md#7-수용-테스트), 허용 목록 [`deny.toml`](../../deny.toml) |

## 1. 대상과 영향

- 대상: 저장소 루트의 `Cargo.lock`이 가리키는 의존성 전체. `deny.toml`의 `[graph] targets`에 따라 Windows(`x86_64-pc-windows-msvc`)와 Ubuntu(`x86_64-unknown-linux-gnu`)의 의존성을 함께 검사한다. 한 OS에서 실행해도 두 대상을 모두 검사한다.
- 예상 사용자 영향: 없다. 파일을 바꾸지 않는 읽기 전용 검사다.
- 작업 허용 시간: 제한 없다.

허용 목록은 사용자가 2026-10-20에 승인했다. MIT, Apache-2.0, ISC, BSD-3-Clause, Unlicense, CC0-1.0, BSL-1.0, Unicode-3.0, MIT-0의 9종이다. GPL·LGPL·MPL 등 copyleft 라이선스는 허용하지 않는다. 목록을 바꾸려면 사용자 승인이 필요하다.

허용 라이선스의 고지 의무는 다음과 같다. 배포 고지 파일의 작성은 OPS-021에서 확인한다.

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

## 5. 중단 조건

- `error[rejected]`가 나오면 그 의존성을 병합하지 않는다. 다른 crate를 찾거나 사용자에게 허용 목록 검토를 요청한다. `deny.toml`의 허용 목록을 승인 없이 늘리지 않는다.
- `error[unlicensed]`가 나오면 해당 crate의 라이선스 파일을 직접 확인하고 사용자 검토를 받는다.

## 6. 롤백 또는 복구

1. 검사는 파일을 바꾸지 않으므로 되돌릴 것이 없다.
2. 의존성 추가로 검사가 실패했으면 그 변경의 `Cargo.toml`·`Cargo.lock`을 되돌리고 3단계를 다시 실행한다.

## 7. 해결되지 않을 때 전달할 대상

- 전달 대상: vnc-relay 프로젝트 관리자
- 보존할 증거: `cargo deny check licenses`의 전체 출력, 문제 crate의 이름·버전, 추가하려던 변경
- 비밀 값 제외 규칙: 이 검사는 비밀 값을 다루지 않는다.
