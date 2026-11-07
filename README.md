# vnc-relay

`vnc-relay`는 UltraVNC Repeater와 호환되는 VNC 중계 서버다. TCP 중계와 WS·WSS ↔ TCP 중계, 웹 관리 대시보드를 실행파일 하나로 제공한다. Rust로 독립 구현했다.

- 버전: 2026.11.0. 버전은 `연.월.순서`다([릴리스 how-to](docs/how-to/release.md)).
- 지원 OS: Windows 11 x64, Ubuntu 24.04 LTS x64. Windows Server와 다른 OS·아키텍처는 검증 범위가 아니다.
- 라이선스: [MIT](LICENSE)

## 주요 기능

| 기능 | 내용 |
| --- | --- |
| 중계 모드 | Mode I: Viewer가 대상 주소를 보내면 서버가 그 대상에 연결한다. 대상은 허용 목록 안에서만 연다. Mode II: 같은 중계 ID로 들어온 Viewer와 Server를 짝지어 중계한다. |
| 진입점 | TCP Viewer, TCP Server(역접속), WS Viewer, WSS Viewer. WS·WSS는 Viewer만 받는다. 평문 WS는 기본으로 꺼져 있다. |
| TLS | WSS와 관리 HTTPS의 TLS를 서버가 직접 처리한다. TLS 1.2 미만은 거절한다. 인증서는 운영자가 준비한다. |
| 접근 제한 | 진입점마다 출발지 IP 허용 목록(단일 IP·CIDR·범위)을 둔다. WS·WSS는 `Origin`을 허용 값과 정확히 비교한다. |
| 관리 | 내장 로그인(Argon2id 해시)이 있는 대시보드와 관리 API(`/api/v1/`). 서버·대기 연결·활성 세션·최근 종료 목록을 조회하고 활성 세션을 종료한다. 화면은 한국어·영어다. |
| 운영 | 인증 없는 상태 리스너(`/livez`, `/readyz`), JSON Lines 운영 로그, 허용 목록·인증서의 자동 재로드, 종료 유예 G. |

noVNC 클라이언트는 포함하지 않는다. noVNC는 외부 클라이언트로 접속할 수 있고, 상호운용 시험에만 쓴다.

## 빠른 시작

아래 절차는 루프백 주소에서 TCP Mode II만 켜는 [예시 설정](examples/relay.toml)을 쓴다. 공개 배치에 그대로 쓰지 않는다. 전체 절차와 공개 배치 주의 사항은 [실행파일을 묶고 수동 운영하는 방법](docs/how-to/operate-relay.md)에 있다.

1. OS에 맞는 배포 묶음을 준비한다. 지금은 저장소에서 직접 만든다. 첫 Release를 낸 뒤에는 [GitHub Releases](https://github.com/kayden-kim/vnc-relay/releases)에서 내려받을 수도 있다. 직접 만들려면 빌드 도구가 필요하다. Windows는 Visual Studio Build Tools 2022의 C++ 빌드 도구, Ubuntu는 `cc`(gcc)를 쓴다. Rust 툴체인은 `rust-toolchain.toml`(Rust 1.99.1)이 정한다. 자세한 내용은 [개발 환경 준비](docs/how-to/set-up-and-verify.md)를 따른다.
2. 직접 만들 때는 저장소 루트에서 묶음을 만든다. Windows에서 NASM이 없으면 먼저 PowerShell에서 `$env:AWS_LC_SYS_PREBUILT_NASM = "1"`을 실행한다.

   ```shell
   cargo xtask package
   ```

   결과는 `target/package/`의 `vnc-relay-<버전>-windows-x64.zip` 또는 `vnc-relay-<버전>-linux-x64.tar.gz`와 SHA-256 파일이다.

3. 묶음을 풀고, 생긴 `vnc-relay-<버전>-<os>-x64/` 디렉터리로 이동한다. 실행파일과 `relay.toml`이 이 디렉터리에 있다. 아래 명령은 이 디렉터리에서 실행한다.
4. 관리자 계정을 만든다. 비밀번호는 숨김 입력으로 두 번 묻는다. 기본 계정은 없고, 계정이 없으면 서버가 시작하지 않는다.

   ```shell
   ./vnc-relay admin add opsadmin --config ./relay.toml
   ```

5. 서버를 시작한다. 프로세스는 터미널을 점유하고 `relay.log`에 운영 로그를 쓴다.

   ```shell
   ./vnc-relay serve --config ./relay.toml
   ```

6. 다른 터미널에서 준비 상태를 확인한다. 준비되면 HTTP 200과 `{"status":"ready"}`가 나온다.

   ```shell
   curl --fail http://127.0.0.1:18081/readyz
   ```

7. 브라우저에서 `http://127.0.0.1:18080/`을 열고 만든 계정으로 로그인한다.
8. 서버는 Ctrl+C로 정상 종료한다.

Windows PowerShell에서는 `./vnc-relay`를 `.\vnc-relay.exe`로, `curl`을 `curl.exe`로 바꾼다.

## 명령

| 명령 | 하는 일 |
| --- | --- |
| `vnc-relay serve --config <path>` | 서버를 포그라운드로 실행한다. |
| `vnc-relay admin add <account> --config <path>` | 관리자 계정을 추가한다. |
| `vnc-relay admin passwd <account> --config <path>` | 비밀번호를 바꾼다. |
| `vnc-relay admin remove <account> [--yes] --config <path>` | 계정을 삭제한다. |
| `vnc-relay admin list --config <path>` | 계정 이름을 나열한다. |
| `vnc-relay --help`, `vnc-relay --version` | 사용법이나 버전을 출력한다. |

계정 변경은 서버를 다시 시작해야 적용된다. 옵션·종료 코드·입력 규칙은 [CLI 계약](docs/contracts/cli-v1.md)이 정한다. 설정 키와 기본값은 [설정 계약](docs/contracts/config-v2.md)이 정한다.

## 개발

변경을 마치기 전에 저장소 루트에서 다음 명령을 실행한다. 모두 종료 코드 0이어야 한다. 대시보드 브라우저 시험에는 Chrome 또는 Edge가 필요하다.

```shell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask layers
cargo xtask tokens --check
cargo xtask scenarios
cargo deny check licenses
```

`cargo xtask scenarios`는 `cargo test --workspace`를 함께 실행하고, 스펙 시나리오별 결과를 `scenario-report.md`로 쓴다. 변경 범위별 명령과 OS별 준비는 [개발 환경 준비와 검증](docs/how-to/set-up-and-verify.md)을 따른다. 작업 규칙은 [AGENTS.md](AGENTS.md)에 있다.

## 저장소 구조

| 위치 | 내용 |
| --- | --- |
| `src/` | 실행파일 `vnc-relay`. CLI와 `serve`·`admin` 조립 |
| `crates/domain/` | 초기 교환·매칭·세션·주소·예산·계정 규칙. 입출력 없음 |
| `crates/app/` | 중계·관리·재로드·준비 상태 사용 사례. 외부 기능은 포트로 정의 |
| `crates/adapters/` | 네트워크·TLS·WebSocket·관리 HTTP·파일·로그 구현 |
| `web/dashboard/` | 정적 HTML·CSS·JS 대시보드. 빌드 단계와 JS 외부 의존성 없음 |
| `xtask/` | 시나리오 보고·레이어 검사·토큰 생성·배포 고지 생성 도구 |
| `tests/` | 실행파일 수준의 통합·시스템 시험 |
| `examples/` | 비밀 값 없는 예시 설정 |
| `docs/` | 요구사항·스펙·계약·설계·절차 문서 |

코드 위치와 호출 흐름은 [코드 지도](docs/architectures/code-map.md)에 있다.

## 문서

| 문서 | 내용 |
| --- | --- |
| [문서 안내](docs/README.md) | 문서 종류와 변경별로 읽을 문서 |
| [PRD](docs/requirements/PRD.md) | 제품 요구사항과 수용 기준 |
| [스펙 목록](docs/specs/README.md) | 중계·WebSocket·관리·대시보드·운영의 외부 동작과 수용 시나리오 |
| [계약 목록](docs/contracts/README.md) | 설정·CLI·관리 API·운영 로그·WebSocket·Repeater 초기 교환 형식 |
| [호환성 검증 기록](docs/contracts/repeater-compat-record.md) | UltraVNC 1.8.3.0·Repeater·noVNC 1.7.0과의 대조 결과 |
| [용어집](GLOSSARY.md) | 업무 용어의 이름과 뜻 |

## 독립 구현과 라이선스

UltraVNC·websockify·noVNC의 소스를 복사·번역·포팅하지 않았다. 공개 문서와 실행 관찰로 통신 규칙을 확인해 새로 작성했다. 검토 기록은 [의존성 라이선스 검사](docs/how-to/check-dependency-licenses.md)에 있다.

이 프로젝트는 [MIT 라이선스](LICENSE)를 따른다. Copyright (c) 2026 vnc-relay contributors. 실행파일에 들어가는 의존성과 글꼴(Pretendard, SIL Open Font License 1.1)의 고지는 [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt)에 있다. 배포 묶음에는 `LICENSE`와 `THIRD-PARTY-NOTICES.txt`를 함께 넣는다.
