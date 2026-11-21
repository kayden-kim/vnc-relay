# 실행파일을 묶고 수동 운영하는 방법

Windows 11 x64와 Ubuntu 24.04 LTS x64에서 release 실행파일을 만들고 운영한다. 계정을 준비한 뒤 시작·양방향 중계·자동 재로드·정상 종료를 확인한다. Windows 서비스와 트레이 앱의 운영은 3.3.1절을 따른다. 컨테이너는 이 절차의 대상이 아니다.

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 배포 담당자와 서버를 직접 실행하는 운영자 |
| 검증 상태 | Draft — 실행한 수동 절차와 미확인 범위는 4절에 있다. 관리 재로그인을 포함한 전체 이관·롤백, Ubuntu 이관, 설정 편집·복구, 프록시 TLS 종료, 최근 로그, Windows 서비스 해제·트레이 메뉴는 수동 검증을 마치지 않았다. 자동 시험만으로 안내 전체를 검증했다고 표시하지 않는다. |
| 검증 환경·버전 | Windows 11 x64의 PowerShell, Ubuntu 24.04 LTS x64의 bash. Rust 버전은 `rust-toolchain.toml`을 따른다. 실제 OS·Rust·실행파일 버전은 실행 기록에 적는다. |
| 최종 검증일 | 없음 — 전체 절차는 Draft다. 개별 수동 기록의 날짜는 4절에 있다. |
| 관련 기준 문서 | [운영 스펙](../specs/operations.md), [설정 계약 v4](../contracts/config-v4.md), [CLI 계약](../contracts/cli-v1.md), [운영 정보 계약 v2](../contracts/ops-info-v2.md), [WebSocket 계약](../contracts/websocket-relay-v2.md), [ADR-0001](../decisions/ADR-0001-built-in-tls.md) |

## 1. 대상과 영향

- 대상: 서버 실행파일, Windows 트레이 실행파일, 설정 파일, 계정 파일, 운영자가 제공한 인증서·개인키, 운영 로그다.
- 예상 사용자 영향: 시작은 리스너를 연다. 재로드는 허용 정책·한도·시간 제한·keepalive·TLS의 이후 소비 시점에 영향을 준다. 이미 허용한 연결과 시작한 타이머는 유지한다. 종료·재시작은 연결과 관리 세션을 끝낸다.
- 작업 허용 시간: 재시작이 필요한 변경은 운영자가 승인한 중단 시간에 한다.

기본 검증 예시는 루프백 TCP의 Mode II만 켠다. 관리 HTTP와 상태 리스너도 루프백이다. 공개 주소에 그대로 옮기는 예시가 아니다. WSS와 프록시 배치는 3.2.1절·3.9절의 별도 v4 예시를 쓴다.

인터넷에 공개하는 WebSocket 진입점에는 내장 WSS 또는 역방향 프록시 TLS 종료를 권장한다. 평문 WS는 보호된 네트워크에서만 켜기를 권장한다. 프록시→VNC Relay 구간의 보호와 공개 배치의 책임은 운영자에게 있다. 관리 리스너를 외부에 열 때는 내장 HTTPS가 필수다. 관리 리스너는 프록시 TLS 종료 지원의 대상이 아니며 전달 헤더를 무시한다. 관리 세션 쿠키와 로그인 제한 출발지 주소 규칙도 바꾸지 않는다.

## 2. 전제 조건

- [ ] 대상이 Windows 11 x64 또는 Ubuntu 24.04 LTS x64다. Windows Server와 다른 OS·아키텍처는 검증 범위가 아니다.
- [ ] 빌드 담당자는 [개발 환경 준비](set-up-and-verify.md)에 따라 Rust와 C 빌드 도구를 준비했다.
- [ ] 운영자는 사용할 포트가 비어 있고 방화벽·출발지 IP 정책이 의도한 범위인지 확인했다.
- [ ] 계정 파일·개인키·로그 디렉터리의 접근 권한을 제한했다.
- [ ] TLS를 켜면 운영자가 발급받은 유효한 인증서 체인과 일치하는 개인키를 준비했다. 서버는 인증서를 자동 발급하지 않는다.
- [ ] 수동 중계 확인에는 Python 3가 있다. 아래 Python은 시험용 클라이언트이며 배포 실행파일의 필수 의존성이 아니다.
- [ ] 기존 설정과 인증서 쌍의 복구본을 안전한 위치에 보관했다.
- [ ] Windows 서비스를 설치·해제할 때 쓸 관리자 권한 터미널을 준비했다.
- [ ] Windows 서비스로 실행하면 `log.output`을 파일 경로로 정했다. 서비스 계정의 파일 권한과 트레이 사용자의 설정 읽기 권한은 3.3.1절에서 확인한다.

## 3. 절차

- 실행 환경: Windows PowerShell 또는 Ubuntu bash다.
- 실행 위치: 3.1절은 저장소 루트다. 3.2절부터는 묶음을 푼 디렉터리다.
- 바꿔 넣을 값: `opsadmin`은 운영 계정명이다. 예시 포트를 바꾸면 확인 명령과 Python의 포트도 함께 바꾼다. 비밀번호는 명령줄에 넣지 않는다.
- 최대 대기 시간: 빌드 한도는 10분으로 둔다. 넘으면 Ctrl+C로 중단하고 빌드 오류를 확인한다. 시작은 `server_ready`를 확인할 때까지 기다린다. 재로드는 Dp + D 안에 결과를 확인한다. 정상 종료는 G와 정리 시간을 기다린다. Python 소켓 한도는 10초다.

### 3.1 release 빌드와 묶음

각 OS에서 그 OS용 묶음을 만든다. 다른 OS의 실행파일을 복사해 쓰지 않는다. 묶음은 `cargo xtask package`로 만든다. 태그로 낸 묶음은 [GitHub Release](https://github.com/kayden-kim/vnc-relay/releases)에서 내려받을 수 있다. 버전 규칙과 Release 절차는 [릴리스 how-to](release.md)에 있다.

1. 저장소 루트에서 묶음을 만든다. 이 명령은 release 빌드(`cargo build --release --locked`), 배포 고지 대조(`cargo xtask notices --check`), 파일 복사, 압축, SHA-256 파일 쓰기를 차례로 한다.

   ```shell
   cargo xtask package
   ```

   Windows에서 NASM이 없으면 먼저 `$env:AWS_LC_SYS_PREBUILT_NASM = "1"`을 실행한다.

   기대 결과: 종료 코드 0이다. 마지막 두 줄은 `package: target/package/vnc-relay-<버전>-<os>-x64.<확장자>`와 `sha256: <64자리 16진수>`다. Windows는 `windows`·`zip`, Ubuntu는 `linux`·`tar.gz`다. `CARGO_TARGET_DIR`을 정했으면 `target` 대신 그 경로다. 고지 대조가 실패하면 `notices --check: 실패`가 나오고 묶음을 만들지 않는다. 이때는 [라이선스 검사 안내](check-dependency-licenses.md#31-배포-고지-생성과-대조)를 따른다.

2. 묶음 안의 `vnc-relay-<버전>-<os>-x64/` 디렉터리에 아래 파일이 있는지 확인한다.

   | 원본 | 묶음 안 위치 |
   | --- | --- |
   | 해당 OS의 release 실행파일 | `vnc-relay.exe` 또는 `vnc-relay` |
   | Windows 트레이 앱의 release 실행파일 | Windows 묶음에만 `vnc-relay-tray.exe` |
   | `examples/relay.toml` | `relay.toml` |
   | `README.md`·`README.ko.md` | 같은 이름. 영어판과 한국어판 |
   | 저장소의 `docs/` 디렉터리와 `GLOSSARY.md` | 같은 상대 위치. 이 안내 `docs/how-to/operate-relay.md`와 기준 문서 링크를 오프라인에서도 읽는다. |
   | `THIRD-PARTY-NOTICES.txt` | `THIRD-PARTY-NOTICES.txt` 전체 |
   | `LICENSE` | 루트 `LICENSE` 전체(MIT) |

   루트 [`LICENSE`](../../LICENSE)는 MIT 전문과 `Copyright (c) 2026 vnc-relay contributors`를 담는다. 배포 묶음에 이 파일을 반드시 넣는다. `THIRD-PARTY-NOTICES.txt`는 프로젝트 자체 MIT LICENSE를 대신하지 않는다. 검토 결과는 [라이선스 검사 4.2절](check-dependency-licenses.md#42-프로젝트-license-검토-기록2026-11-07)에 있다.

   개인키·계정 파일·실제 비밀번호·운영 로그는 배포 묶음에 넣지 않는다. `cargo xtask package`는 위 표의 파일만 복사한다.

3. 묶음을 운영 디렉터리에 풀고, 함께 받은 `.sha256` 파일로 해시를 확인한다. Ubuntu는 `sha256sum -c vnc-relay-<버전>-linux-x64.tar.gz.sha256`, Windows는 `Get-FileHash <묶음 파일>`의 값과 `.sha256` 파일의 첫 낱말을 비교한다.

   기대 결과: 해시가 같다. 실행파일과 `relay.toml`이 같은 디렉터리에 있다. 이후 명령은 이 디렉터리에서 실행한다.

### 3.2 설정과 계정 준비

상대 경로는 현재 터미널 위치가 아니라 `relay.toml`의 디렉터리를 기준으로 푼다. 설정에는 비밀번호·해시·개인키 본문을 넣지 않는다.

1. 운영 디렉터리의 접근 권한을 제한한다.

   Ubuntu에서는 서버를 실행할 OS 계정을 소유자로 두고 다른 계정이 디렉터리를 쓰지 못하게 하기를 권장한다. 계정 파일·개인키는 `0600`을 권장한다. CLI는 새 계정 파일을 `0600`으로 만든다.

   Windows에서는 폴더 속성의 보안 설정에서 서버 실행 OS 계정과 관리자만 읽고 쓰도록 ACL(접근 제어 목록)을 제한하기를 권장한다. CLI는 ACL을 바꾸거나 경고하지 않는다. 새 파일은 디렉터리 ACL을 물려받으므로 운영자가 실제 권한을 확인해야 한다([CLI 계약 1.6절](../contracts/cli-v1.md#16-저장과-파일-권한)).

2. `relay.toml`을 대상 배치에 맞게 편집한다.

   예시는 모든 설정 키를 담는다. 쓰지 않는 키는 `# 키 = 값` 주석 줄로 기본값이나 예시 값을 보여 준다. 그 키를 설정하려면 줄 앞의 `#`과 공백을 지우고 값을 정한다. 로컬 검증에서는 예시 그대로 쓴다. 공개 배치에서는 리스너 주소와 역할별 출발지 `allow`를 의도한 값으로 정한다. `[]`는 모두 거절이며 켠 역할의 목록 생략은 시작 실패다. `0.0.0.0/0`·`::/0`는 전체 허용과 경고를 뜻한다. Mode I을 켜면 `mode1.allow_targets`도 명시해야 한다. WSS의 공통 `listen`·`cert_file`·`key_file`은 `[relay.wss]`에 둔다. `enabled`·`allow`·`allowed_origins`는 `[relay.wss.viewer]`와 `[relay.wss.server]`에 둔다. 브라우저 Origin은 IP 제한을 대신하지 않는다.

3. 대화형 터미널에서 관리자 계정을 추가한다.

   Windows:

   ```powershell
   .\vnc-relay.exe admin add opsadmin --config .\relay.toml
   ```

   Ubuntu:

   ```shell
   ./vnc-relay admin add opsadmin --config ./relay.toml
   ```

   기대 결과: 비밀번호를 두 번 묻고 입력 글자는 보이지 않는다. 12~128자이며 제어 문자는 받지 않는다. 성공하면 계정 추가 문장이 나온다. 기본 계정·기본 비밀번호는 없다. 비밀번호를 파이프로 보내지 않는다.

4. 계정 이름 목록을 확인한다.

   Windows: `.\vnc-relay.exe admin list --config .\relay.toml`

   Ubuntu: `./vnc-relay admin list --config ./relay.toml`

   기대 결과: `opsadmin`이 보이고 비밀번호·해시는 보이지 않는다. 계정이 없으면 서버를 시작하지 않는다.

기존 계정을 바꾸려면 먼저 서버를 정상 종료한다. 같은 실행파일의 `admin passwd opsadmin --config ./relay.toml`로 비밀번호를 바꿀 수 있다. `admin remove opsadmin --config ./relay.toml`은 삭제 확인을 묻는다. Windows에서는 실행파일·경로 표기를 위 명령처럼 바꾼다. 마지막 계정을 삭제하면 시작할 수 없다. 계정 변경은 서버 재시작 전까지 적용되지 않으며 기존 관리 세션도 재시작 전까지 유효하다.

#### 3.2.1 v4 WSS Viewer·Server 설정 예시

다음은 v4 형식의 별도 `relay.toml` 전체 예시다. 기본 TCP 검증 예시에 그대로 덧붙이지 않는다. 문서용 주소·Origin·인증서 경로는 실제 배치 값으로 바꾸어야 한다. `tls/relay-chain.pem`과 `tls/relay-key.pem`은 운영자가 먼저 준비한 유효한 쌍이어야 한다. 이 예시는 추정이며 미실행이다.

```toml
[admin]
accounts_file = "admin-accounts.toml"

[relay.wss]
listen = "0.0.0.0:8443"
cert_file = "tls/relay-chain.pem"
key_file = "tls/relay-key.pem"
trusted_proxies = []

[relay.wss.viewer]
enabled = true
allow = ["192.0.2.10"]
allowed_origins = ["https://viewer.example.test"]

[relay.wss.server]
enabled = true
allow = ["192.0.2.30"]
allowed_origins = []

[mode2]
enabled = true

[management]
listen = "127.0.0.1:18080"

[status]
listen = "127.0.0.1:18081"

[recent_logs]
max_entries = 1000

[log]
output = "relay.log"
```

두 역할은 같은 WSS 소켓을 쓴다. Viewer는 `wss://HOST:8443/relay/viewer`, Server는 `wss://HOST:8443/relay/server`로 접속한다. `HOST`는 인증서에 맞는 실제 호스트명이다. 경로 뒤 `/`는 붙이지 않는다. `/relay`를 쓰는 Viewer 클라이언트는 3.2.3절에 따라 옮긴다. Server 역할은 Mode II만 받으며 TCP Server와 같은 `ID:` 초기 교환을 쓴다. Server에는 greeting을 보내지 않는다. 위 Server의 빈 Origin 목록은 `Origin` 없는 클라이언트만 허용한다.

#### 3.2.2 v3 파일을 v4로 옮기기

v4 서버는 v3의 WS·WSS 테이블을 읽지 않는다. 호환 지원 기간은 없다. 이관 전 파일을 넣으면 `config_unknown_key`로 시작 실패한다. 실제 운영 `relay.toml`도 배포 전에 이관해야 한다. 이 동작의 자동 시나리오는 OPS-066이다. 수동 실행 범위는 [4절 기록](#4-검증)을 따른다.

1. 현재 실행파일·`relay.toml`·계정 파일·TLS 파일 쌍을 접근이 제한된 복구 위치에 복사한다. 실행파일 버전·해시와 파일 경로도 기록한다.
2. 운영 파일과 같은 디렉터리에 `relay-v4.toml` 사본을 만든다. 실행 중인 구버전의 파일은 아직 바꾸지 않는다.
3. [설정 계약 3.3절의 매핑](../contracts/config-v4.md#33-v3-파일을-v4로-옮기기)으로 사본을 편집한다.

   | v3 위치 | v4 위치 |
   | --- | --- |
   | `[relay.ws_viewer]`의 `enabled`·`allow`·`allowed_origins` | `[relay.ws.viewer]`의 같은 키 |
   | `[relay.ws_viewer]`의 `listen` | `[relay.ws]`의 `listen` |
   | `[relay.wss_viewer]`의 `enabled`·`allow`·`allowed_origins` | `[relay.wss.viewer]`의 같은 키 |
   | `[relay.wss_viewer]`의 `listen`·`cert_file`·`key_file` | `[relay.wss]`의 같은 키 |

   이전 테이블은 남기지 않는다. TOML 파일에 버전 키를 추가하지 않는다. 기존 Viewer 동작만 유지하려면 Server 역할·`trusted_proxies`·`[recent_logs]`를 추가할 필요는 없다. Server 역할은 기본 비활성이다. Viewer 클라이언트가 `/relay`를 쓰면 3.2.3절도 함께 따른다.

4. 이관한 파일의 별도 검증 사본 `relay-v4-check.toml`을 같은 디렉터리에 만든다. 켜진 모든 중계 리스너와 관리·상태 리스너의 `listen`을 서로 다른 미사용 루프백 포트로 바꾸고 `log.output = "stdout"`으로 바꾼다. 운영 파일과 계정·TLS 파일은 수정하지 않는다.
5. 운영 파일과 같은 디렉터리에서 새 실행파일의 실제 경로로 검증 사본을 시작한다. Windows는 `& 'C:\배포\v4\vnc-relay.exe' serve --config .\relay-v4-check.toml`, Ubuntu는 `/opt/vnc-relay-v4/vnc-relay serve --config ./relay-v4-check.toml`이다. 실행파일 경로는 새 묶음의 실제 경로로 바꾼다. 구버전 실행파일을 덮어쓰지 않는다.

   별도의 검증 전용 CLI는 없다. 이 시작으로 전체 설정·계정·TLS 검증과 대체 포트 바인드를 확인한다. `server_ready`와 검증 사본의 상태 포트 `/readyz` 200을 확인한 뒤 Ctrl+C로 정상 종료한다. 오류가 나면 운영 서버를 멈추지 말고 사본을 고친다. 이 절차는 운영 포트·방화벽·실제 중계의 검증을 대신하지 않는다.

6. 중단 시간에 3.8절로 구버전을 정상 종료한다. 이 단계는 모든 연결과 관리 세션을 끝낸다. 복구본으로 이전 실행파일과 파일을 함께 복원할 수 있는지 먼저 확인한다.
7. 새 실행파일과 검증된 `relay-v4.toml`을 운영 위치에 설치한다. 운영용 `listen`·로그 경로를 유지한 파일을 `relay.toml`로 쓴다. 대체 포트의 검증 사본을 설치하지 않는다.
8. 3.3절로 시작·상태·관리 재로그인을 확인한다. 기존 Viewer의 실제 중계와 3.5절 재로드도 확인한다. Viewer 경로는 새 실행파일의 경로를 쓴다(3.2.3절). 새 Server 역할은 `/relay/server`의 Mode II 양방향 중계를 따로 확인한다.

로그 소비자도 같은 배포에서 바꾼다. `server_starting.log_contract`는 `ops-info/2.0.0`이다. 예전 `listener=ws_viewer`는 `listener=ws`와 `role=viewer`, `wss_viewer`는 `wss`와 `role=viewer`로 대응한다. `ws`·`wss`의 `connection_accepted`에는 아직 `role`이 없다. 경로로 역할을 정한 뒤에만 `role`이 있다. 리스너 값은 `tcp_viewer`·`tcp_server`·`ws`·`wss`·`management`·`status`다. 자세한 범위는 [운영 정보 계약 v2](../contracts/ops-info-v2.md)를 따른다.

롤백은 6절의 v4→v3 복구 절차를 따른다. 검증 사본은 증거를 보관한 뒤 지운다. 계정·TLS 비밀을 일반 검증 로그에 복사하지 않는다.

#### 3.2.3 Viewer 경로를 `/relay/viewer`로 옮기기

서버는 `websocket-relay` 2.0.0을 구현한다. WS·WSS Viewer 경로로 `/relay/viewer`만 받으며 이전 경로 `/relay`는 404로 거절한다. 두 경로를 함께 받는 기간은 없다([WebSocket 계약 3.1절](../contracts/websocket-relay-v2.md#31-1x-클라이언트를-200으로-옮기기)). 설정 파일은 바꾸지 않는다. TCP 진입점과 Server 경로 `/relay/server`는 그대로다.

1. 이 서버의 WS·WSS Viewer 경로로 접속하는 클라이언트를 모두 찾는다. 예: noVNC에 접속 URL을 넘기는 서비스 설정, 바이트 중계 스크립트. 클라이언트마다 URL을 바꿀 위치와 되돌릴 방법을 기록한다.
2. 3.2.2절 1단계와 같이 현재 실행파일·`relay.toml`·계정 파일·TLS 파일을 복구 위치에 복사한다.
3. 중단 시간을 정한다. 서버를 바꾼 뒤 URL을 바꾸기 전까지 Viewer 접속은 404로 실패한다. URL을 먼저 바꾸면 이전 서버에서 같은 404가 난다.
4. 3.8절로 이전 서버를 정상 종료한다. 새 실행파일을 설치한다. 설정 파일은 그대로 쓴다.
5. 1단계의 모든 클라이언트에서 접속 URL의 경로를 `/relay/viewer`로 바꾼다. 예: `wss://HOST:8443/relay`는 `wss://HOST:8443/relay/viewer`가 된다. 호스트·포트·쿼리는 바꾸지 않는다.
6. 3.3절로 시작과 상태를 확인한다. 바꾼 URL로 Viewer의 실제 중계를 확인한다.
7. 운영 로그의 `connection_rejected`(`stage=upgrade`, `reason=invalid_upgrade`)는 참고 신호로만 쓴다. 이 사유는 다른 모든 경로와 잘못된 업그레이드 요청에도 쓰이고, 로그에는 요청 경로가 없다. 그래서 이 로그로 이전 URL을 쓰는 클라이언트를 가려낼 수 없다. 이전 완료는 1단계 목록의 모든 클라이언트에서 5·6단계를 마쳤는지로 확인한다. 이 거절이 늘면 목록에서 빠진 클라이언트가 있는지 찾는다. 프록시가 요청 경로를 접근 로그에 남기는 배치라면 그 로그에서 `/relay` 요청을 찾을 수 있다.

롤백은 서버와 URL을 함께 되돌린다. 3.8절로 새 서버를 멈추고 2단계의 복구본으로 이전 실행파일을 설치한다. 1단계의 모든 클라이언트 URL을 `/relay`로 되돌린 뒤 3.3절로 확인한다. 설정 파일은 그대로 쓴다.

3.9절의 Nginx 예시는 `location /relay` 접두사로 두 경로를 모두 전달한다. 이 예시를 쓰면 프록시 설정은 바꾸지 않는다.

### 3.3 시작과 상태 확인

1. 전용 터미널에서 서버를 시작한다.

   Windows:

   ```powershell
   .\vnc-relay.exe serve --config .\relay.toml
   ```

   Ubuntu:

   ```shell
   ./vnc-relay serve --config ./relay.toml
   ```

   기대 결과: 프로세스가 터미널을 점유한다. 예시는 `relay.log`에 JSON Lines를 이어 쓴다. `server_ready`가 기록되기 전에는 준비 완료로 판단하지 않는다.

2. 다른 터미널에서 생존 상태를 조회한다.

   Windows: `curl.exe --fail http://127.0.0.1:18081/livez`

   Ubuntu: `curl --fail http://127.0.0.1:18081/livez`

   기대 결과: HTTP 200과 `{"status":"alive"}`다.

3. 같은 상태 리스너에서 준비 상태를 조회한다.

   Windows: `curl.exe --fail http://127.0.0.1:18081/readyz`

   Ubuntu: `curl --fail http://127.0.0.1:18081/readyz`

   기대 결과: 준비 후 HTTP 200과 `{"status":"ready"}`다. 종료 유예 중에는 503이다. 상태 리스너에는 인증이 없고 세션 목록도 없다. 관리 리스너와 혼동하지 않는다.

4. 브라우저에서 `http://127.0.0.1:18080/`을 열어 준비한 계정으로 로그인한다.

   기대 결과: 인증 전에는 운영 데이터를 볼 수 없다. 공개 관리 배치에서는 설정한 `https://` 주소를 쓰고 인증서 검증을 생략하지 않는다.

5. 설정에서 `[metrics] enabled = true`를 켰다면 같은 상태 리스너에서 메트릭을 조회한다. 이 키는 재시작해야 적용된다.

   Windows: `curl.exe --fail http://127.0.0.1:18081/metrics`

   Ubuntu: `curl --fail http://127.0.0.1:18081/metrics`

   기대 결과: HTTP 200과 `Content-Type: text/plain; version=0.0.4; charset=utf-8`다. 본문의 첫 계열은 `target_info`이고 `target_info{service_name="vnc-relay",service_version="<버전>"} 1` 줄이 있다. 꺼져 있으면 404이며 `--fail` 때문에 curl이 종료 코드 22로 끝난다. 이 경로에도 인증이 없다. 상태 리스너는 루프백이나 모니터링 네트워크로 제한해야 한다. 이름·단위·속성은 [메트릭 계약](../contracts/metrics-v1.md)을, Prometheus·OpenTelemetry Collector 수집 설정은 [운영자 도움말 "메트릭 수집"](../help/ko/src/operate.md#메트릭-수집)을 따른다.

#### 3.3.1 Windows 서비스와 트레이 앱

이 절은 Windows 11 x64에만 적용한다. 서비스와 트레이 앱은 Windows 묶음에 있다. 설치·해제에는 관리자 권한 터미널이 필요하다. 서비스 시작·중지에는 설치 때 지정한 서비스 운영자의 일반 터미널을 쓸 수 있다. 기준은 [운영 스펙 3.7절](../specs/operations.md#37-windows-서비스-실행)과 [CLI 계약 1.9절](../contracts/cli-v1.md#19-windows-서비스-실행)이다. 시험 절차와 실제 확인 범위는 [Windows 서비스 시험](set-up-and-verify.md#8-windows-서비스-시험)에 있다.

##### 설치

설정·계정·TLS·로그 파일은 `%ProgramData%\vnc-relay\` 아래에 두기를 권장한다. 먼저 3.2절에 따라 설정과 관리자 계정을 준비한다. `log.output`에는 `stdout` 대신 파일 경로를 쓴다. 실행파일은 설치 뒤에도 같은 위치에 유지해야 한다.

실행파일은 관리자만 고칠 수 있는 폴더(예: `%ProgramFiles%\vnc-relay\`)에 둔다. 일반 사용자가 쓸 수 있는 폴더에 두지 않는다. 그 사용자가 실행파일을 바꾸면 다음 시작 때 바뀐 파일이 서비스 계정으로 실행된다. Windows 방화벽의 허용 규칙을 프로그램 경로로 만들었으면 새 실행파일 경로로도 규칙을 만든다. 프로그램 경로 규칙은 다른 경로의 실행파일에 적용되지 않는다.

1. 준비한 설정·계정·사용 중인 TLS·키 파일을 `%ProgramData%\vnc-relay\`에 배치한다. 설정의 상대 경로도 이 위치를 기준으로 맞춘다.
2. 관리자 권한 PowerShell에서 묶음을 푼 디렉터리로 이동한다.
3. 묶음의 파일을 관리자만 고칠 수 있는 폴더에 복사한다.

   ```powershell
   New-Item -ItemType Directory -Force -Path "$env:ProgramFiles\vnc-relay"
   Copy-Item -Path .\* -Destination "$env:ProgramFiles\vnc-relay" -Recurse
   ```

4. 복사한 폴더로 이동한다.

   ```powershell
   Set-Location "$env:ProgramFiles\vnc-relay"
   ```

5. 서비스를 설치한다. `<사용자>`와 `<그룹>`은 Windows에서 찾을 수 있는 실제 이름으로 바꾼다. 필요한 수만큼 `--operator`를 반복한다.

   ```powershell
   .\vnc-relay.exe service install --config "$env:ProgramData\vnc-relay\relay.toml" --operator "<사용자>" --operator "<그룹>"
   ```

   설치는 기존 서비스와 운영자 이름을 확인하고 설정 경로를 절대 경로로 바꾼다. 이어서 `serve`의 리스너 바인드 전 파일 검사와 같은 범위로 설정·계정·사용 중인 TLS 파일·접속 ticket 공개 키 파일을 검사한다. 로그 열기·변경 감지 준비·리스너 바인드는 하지 않는다. 검사는 설치한 관리자 권한으로 실행되므로 서비스 계정의 접근 권한을 보장하지 않는다.

   성공하면 이름 `vnc-relay`, 표시 이름 `VNC Relay`, 계정 `NT SERVICE\vnc-relay`, 자동(지연 시작)으로 등록한다. 복구 동작은 설정하지 않는다. 지정한 운영자에게 시작·중지·상태 조회·구성 조회 권한을 준다. 설치만으로 서비스를 시작하지 않는다. 등록 뒤 설정 단계가 실패하면 새 등록을 지운다. 지우기까지 실패하면 진단의 수동 삭제 안내를 따른다.

6. 등록 속성을 확인한다.

   ```powershell
   sc.exe qc vnc-relay
   ```

##### 파일 접근 권한

서비스 계정은 설정·계정·사용 중인 인증서·개인키·접속 ticket 공개 키 파일을 읽을 수 있어야 한다. 로그 디렉터리에는 쓸 수 있어야 한다. 관리 API의 설정 편집을 켜면 설정 디렉터리에도 쓸 수 있어야 한다. 트레이 사용자는 설정 파일 읽기 권한만 있으면 된다. 트레이 앱에 계정 파일·개인키 읽기 권한을 줄 필요는 없다.

`%ProgramData%`의 기본 접근 권한에 의존하지 않는다. 계정 파일과 개인키는 Administrators, SYSTEM, `NT SERVICE\vnc-relay`만 읽도록 제한하기를 권장한다. 실제 ACL은 폴더와 기존 명시적 허용에 따라 달라진다.

다음은 서비스 등록 뒤 관리자 권한 PowerShell에서 적용하는 예시다. `<...>`는 실제 경로나 사용자·그룹으로 바꾼다. 비밀 파일마다 따로 적용한다. ACL 변경은 다른 사용자의 접근을 끊을 수 있으므로 변경 전에 현재 권한과 복구 방법을 확보한다.

1. 대상 파일의 현재 ACL을 확인한다.

   ```powershell
   icacls "<계정 파일 또는 개인키 경로>"
   ```

2. 비밀 파일의 상속을 끄고 필요한 읽기 권한을 준다. SID 표기는 Windows 표시 언어와 관계없이 Administrators와 SYSTEM을 지정한다.

   ```powershell
   icacls "<계정 파일 또는 개인키 경로>" /inheritance:r /grant:r "*S-1-5-32-544:F" "*S-1-5-18:F" "NT SERVICE\vnc-relay:R"
   ```

3. 1단계에서 다른 명시적 읽기 허용을 찾았으면 해당 허용을 제거한다. `/grant:r`만으로 다른 주체의 명시적 허용이 지워지지는 않는다.

   ```powershell
   icacls "<계정 파일 또는 개인키 경로>" /remove:g "<불필요한 사용자 또는 그룹>"
   ```

4. 트레이를 사용할 사용자나 그룹에 설정 파일 읽기를 허용한다.

   ```powershell
   icacls "$env:ProgramData\vnc-relay\relay.toml" /grant:r "<트레이 사용자 또는 그룹>:R"
   ```

5. 로그 디렉터리에 서비스의 파일 생성·회전 권한을 준다.

   ```powershell
   icacls "<로그 디렉터리 경로>" /grant:r "NT SERVICE\vnc-relay:(OI)(CI)M"
   ```

6. 관리 API의 설정 편집을 켰으면 설정 디렉터리에 서비스의 수정 권한을 준다.

   ```powershell
   icacls "<설정 디렉터리 경로>" /grant:r "NT SERVICE\vnc-relay:(OI)(CI)M"
   ```

7. 대상 파일과 디렉터리의 최종 ACL을 확인한다.

   ```powershell
   icacls "<확인할 파일 또는 디렉터리 경로>"
   ```

##### 시작·중지와 해제

서비스 운영자는 일반 PowerShell, Windows 서비스 관리 도구(`services.msc`), 트레이 앱으로 시작·중지할 수 있다. UAC 승인은 필요하지 않다. 운영자로 지정하지 않은 사용자는 기본적으로 조회만 할 수 있다.

1. 서비스를 시작한다.

   ```powershell
   sc.exe start vnc-relay
   ```

2. 서비스 상태를 조회한다. `RUNNING` 뒤에도 설정한 상태 리스너의 `/readyz`를 확인한다.

   ```powershell
   sc.exe query vnc-relay
   ```

3. 중단할 때 서비스를 중지한다. 활성 세션에는 G의 종료 유예를 준다.

   ```powershell
   sc.exe stop vnc-relay
   ```

4. `sc.exe query vnc-relay`로 `STOPPED`가 될 때까지 기다린다. 중지 중의 두 번째 `stop`은 SCM이 1061로 거절한다. 콘솔의 두 번째 종료 요청처럼 유예를 건너뛰지 않는다.
5. 해제할 때 관리자 권한 터미널에서 등록을 제거한다.

   ```powershell
   .\vnc-relay.exe service uninstall
   ```

   실행파일·설정·계정·인증서·로그는 남는다. 실행 중인 서비스는 해제하지 않는다.

`service run`은 SCM 전용 진입점이다. 터미널에서 직접 실행하지 않는다. 설치·해제 오류의 종료 코드와 `service_access_denied`·`service_exists`·`service_running`·`service_log_stdout` 등의 진단은 [CLI 계약 1.9절](../contracts/cli-v1.md#19-windows-서비스-실행)을 따른다. 서비스 시작에 실패하면 트레이의 종료 코드나 `sc.exe query`를 확인한다. 진단 문장이 필요하면 서비스를 중지한 뒤 같은 설정으로 `serve`를 실행한다. 서비스에는 콘솔이 없으므로 로그 파일을 열기 전 실패는 운영 로그에 남지 않을 수 있다.

##### 트레이 앱

Windows 묶음의 `vnc-relay-tray.exe`를 일반 사용자로 실행한다. 아이콘을 클릭하면 상태 줄 다음에 `시작`, `중지`, `대시보드 열기`, `로그 파일 위치 열기`, `설정 파일 위치 열기`, `로그인할 때 시작`, `트레이 끝내기`가 나온다. 다시 시작 메뉴는 없다. `중지`는 확인 대화상자 없이 정상 종료를 요청한다. 두 위치 열기는 파일 탐색기를 연다. 세부 상태와 켜짐 조건은 [Windows 트레이 앱 스펙](../specs/windows-tray.md#4-화면-구성과-와이어프레임)을 따른다.

`로그인할 때 시작`은 현재 사용자의 로그인 때 트레이 앱을 실행하도록 설정한다. 서비스 자동 시작과는 별개다. `트레이 끝내기`는 아이콘과 트레이 프로세스만 끝낸다. 서비스를 시작·중지하거나 로그인할 때 시작 설정을 지우지 않는다.

### 3.4 양방향 Mode II 중계 확인

이 절차는 루프백 예시의 TCP 시작·중계를 확인한다. 실제 VNC 화면 호환 시험을 대신하지 않는다. 기존 운영 ID와 겹치지 않는 테스트 ID를 쓴다.

1. 다음 내용을 운영 디렉터리의 `check-relay.py`에 저장한다.

   ```python
   import socket

   def receive(stream, length):
       data = bytearray()
       while len(data) < length:
           chunk = stream.recv(length - len(data))
           if not chunk:
               raise RuntimeError("relay closed early")
           data.extend(chunk)
       return bytes(data)

   frame = b"ID:560031".ljust(250, b"\0")
   with socket.create_connection(("127.0.0.1", 5500), timeout=10) as server:
       server.sendall(frame)
       with socket.create_connection(("127.0.0.1", 5901), timeout=10) as viewer:
           assert receive(viewer, 12) == b"RFB 000.000\n"
           viewer.sendall(frame)
           for source, target, payload in [
               (viewer, server, b"viewer-to-server"),
               (server, viewer, b"server-to-viewer"),
           ]:
               source.sendall(payload)
               assert receive(target, len(payload)) == payload
           print("Mode II: both directions passed")
   ```

2. 시험 클라이언트를 실행한다.

   Windows: `python .\check-relay.py`

   Ubuntu: `python3 ./check-relay.py`

   기대 결과: `Mode II: both directions passed`와 종료 코드 0이다. `relay.log`에는 매칭·세션 시작·종료 사건이 남는다.

### 3.5 자동 재로드

재로드 대상은 [설정 계약 1.8절](../contracts/config-v4.md#18-재로드-구분)의 44개 키다. 이 중 TLS 네 경로와 접속 ticket 공개 키 파일 경로 두 개를 뺀 38개가 편집 대상이다. 역할별 출발지 IP·Origin, 리스너별 `trusted_proxies`, 출발지 IP 차단 목록(`relay.deny.sources`), 켠 Mode I의 대상 허용 정책, L·Lm·Lw·B·M, H·U·W·C·E·G, TCP/WS keepalive, TLS 경로·내용, 접속 ticket 설정과 공개 키 파일 내용을 포함한다(3.10절). `trusted_proxies`는 교체 뒤 다음 수락·업그레이드 검사부터 적용한다. 이미 통과한 검사 결과, 기존 연결과 시작한 타이머는 유지한다. 다만 차단 목록의 교체는 그 주소의 열린 활성 세션과 대기 연결을 바로 끊는다. 낮춘 상한을 이미 넘었으면 기존 점유를 유지하고 새 점유만 거절한다. 세부 적용 시점은 계약 표를 따른다.

리스너 주소·포트, 역할·모드 켜기/끄기, 관리 HTTP/HTTPS 전환, 계정·로그인 제한·관리 세션 정책, `management.config_edit`, 대시보드·최근 종료 보관·최근 로그 보관 수·로그 출력·재로드 주기는 재시작해야 한다. 이런 값이 실행 중 값과 다르면 후보 전체를 `reload_restart_required`로 거절한다. 그 차이를 되돌리거나 서버를 재시작하기 전에는 동적 값과 급한 인증서 교체도 적용되지 않는다.

1. 실행 중인 예시의 `relay.tcp_viewer.allow`만 `["127.0.0.1", "::1"]`로 바꾸어 저장한다.

   기대 결과: OS 알림 또는 Dp 주기 확인으로 변경을 감지한다. 예시의 D는 5초, Dp는 30초다. 알림을 놓쳐도 Dp + D 안에 `reload_applied` 또는 실패 결과를 확인한다.

2. `relay.log`에서 `reload_applied`를 확인한다.

   실패하면 이전 유효 설정과 기존 중계를 유지한다. `reload_rejected`·`reload_restart_required`·감지 장애 사건을 성공으로 판단하지 않는다. 수동 재로드 CLI와 웹 재로드 API는 없다.

3. 3.4절의 클라이언트를 다시 실행한다.

   기대 결과: 양방향 바이트가 다시 일치한다. 이 확인은 IP 후보 적용 뒤 새 연결의 중계 확인이다.

#### 3.5.1 대시보드 설정 보기·편집

기본값 `management.config_edit = false`는 읽기 전용이다. 로그인한 모든 관리자는 설정 보기에서 **실행 중 유효 값**을 볼 수 있다. 파일에만 저장된 미적용 값을 현재 실행값으로 판단하지 않는다.

1. 편집이 필요하면 설정과 실행파일의 복구본을 보관한다.
2. 외부 편집기로 기존 `[management]` 테이블에 `config_edit = true`를 추가한다. 같은 테이블을 중복으로 만들지 않는다.
3. 3.8절로 서버를 재시작한다. 이 키는 자동 재로드되지 않는다.
4. 다시 로그인한 뒤 설정 보기에서 편집 허용과 파일 상태를 확인한다. 재시작 전 관리 세션은 무효다.
5. 편집 화면에서 허용된 값만 바꾸고 변경 내역·적용 시점을 확인한 뒤 한 번 저장한다. TLS 네 경로와 접속 ticket 공개 키 파일 경로 두 개는 보기 전용이며 입력란이 아니다.
6. 저장 뒤 설정을 다시 조회해 실행값·파일 상태·최근 재로드 결과를 확인한다. 적용 키 수는 이번 요청 키 수와 다를 수 있다. 기존 `pending` 변경까지 함께 적용한 경우 실행값 차이 전체를 센다.

**편집 기능을 켠 동안 외부 편집과 대시보드 편집을 동시에 하지 않는다.** 다른 관리자와의 저장도 순서를 정한다. 주석·공백만 외부에서 바뀌어도 원시 바이트 revision이 달라 충돌한다. 서버는 임시 파일 쓰기·flush 뒤 교체 직전에 원본 revision을 다시 확인한다. 다르거나 다시 읽지 못하면 임시 파일을 지우고 `409 config_conflict`로 거절한다. 마지막 확인과 원자 교체 사이의 아주 짧은 틈은 보장 범위 밖이다.

편집에 필요한 OS 권한은 다음과 같다. 브라우저 관리자 권한과 실제 파일을 쓰는 서버 실행 OS 계정의 권한은 다르다. 서비스로 감싼 별도 배치라면 서비스 로그온 계정에 적용한다. 서비스 등록 자체는 이 절차의 범위가 아니다.

| OS | 서버 실행 계정에 필요한 권한과 확인 |
| --- | --- |
| Windows | 설정 파일 읽기와 교체·삭제 권한, 상위 디렉터리의 파일 생성·쓰기·삭제/교체 권한을 ACL(접근 제어 목록)로 확인한다. 같은 디렉터리에 임시 파일을 만들고 기존 파일 위로 rename할 수 있어야 한다. 파일 ACL만 쓰기 허용해도 디렉터리가 막으면 실패한다. 백업·편집기·보안 도구가 원본을 연 채 공유 삭제(`FILE_SHARE_DELETE`)를 허용하지 않으면 교체가 실패할 수 있다. 해당 핸들을 닫거나 도구의 공유 모드를 고친다. 새 임시 파일에 적용되는 디렉터리 상속 ACL도 일반 계정의 변조를 허용하지 않게 제한한다. |
| Ubuntu | 설정 파일을 읽고 운영상 쓸 수 있는 계정인지 확인한다. 상위 경로에 search(`x`), 설정 디렉터리에 write+search(`wx`)가 있어야 임시 파일 생성·삭제와 같은 파일시스템 rename-over-existing이 가능하다. 파일의 write만 허용해도 디렉터리가 막으면 실패한다. 읽기 전용 마운트·sticky bit·소유권 제한도 확인한다. 교체는 새 임시 파일을 쓰므로 디렉터리 권한과 umask도 보호된 값으로 설정한다. |

설정 파일 전체를 다시 생성하지 않고 실제 다른 키의 값만 고친다. 주석·다른 키·순서·기존 LF/CRLF는 유지한다. 누락 키는 기존 테이블에, 누락 테이블은 끝에 넣는다. 값이 같아 파일 바이트가 바뀌지 않으면 쓰기·실행값 교체·성공 사건 없이 `applied=[]`다.

| 파일 상태 | 운영자 행동 |
| --- | --- |
| `in_effect` | 파일과 실행값이 같다. 편집 허용 상태라면 저장할 수 있다. |
| `pending` | 유효한 동적 변경이 아직 미적용이다. watcher 결과를 기다리거나 상태를 새로고침한다. PATCH는 파일에 있는 변경을 포함한 후보 전체를 검증한다. |
| `restart_required` | 표시된 재시작 필요 키를 외부에서 실행값으로 되돌리거나 승인된 시간에 재시작·재로그인한다. 해소 전 PATCH는 `config_restart_pending`으로 거절된다. |
| `invalid` | 코드·키 진단에 따라 외부 편집기로 TOML·설정 관계·TLS 파일을 고친 뒤 새로고침한다. 이전 실행값은 유지되며 PATCH로 잘못된 원본을 복구하지 않는다. |
| `unreadable` | 파일 경로·읽기 권한·UTF-8 인코딩을 복구한 뒤 새로고침한다. 파일 읽기 실패이면 revision은 null이고, 비UTF-8 바이트를 읽었으면 그 바이트 revision은 있다. 진단은 빈 배열이다. |

`config_edit_disabled`이면 위의 활성화·재시작·재로그인을 따른다. `config_conflict`이면 다시 불러와 최신 revision으로 새로 편집하며 옛 요청을 자동 재전송하지 않는다. `config_invalid`이면 입력과 진단을 고친다. `config_write_failed`이면 이번 요청은 원본·실행값을 바꾸지 않았으므로 위의 파일/디렉터리 권한·열린 핸들·디스크 여유를 복구하고 현재 상태를 확인한 뒤 명시적으로 다시 저장한다. timeout·연결 단절·그 밖의 5xx는 성공 여부가 불명이다. 먼저 GET으로 실행값·파일·최근 이력을 확인하고 재확인 전에는 PATCH를 재전송하지 않는다.

### 3.6 인증서 교체

TLS를 켠 배치에만 적용한다. TLS 1.2 이상이 필요하다. 지원 PEM 체인·키 형식은 [설정 계약 1.9절](../contracts/config-v4.md#19-인증서개인키-파일-형식)을 따른다. 비밀키 내용은 로그·명령행에 넣지 않는다.

1. 새 인증서 체인과 일치하는 개인키를 보호된 새 파일 두 개에 저장한다.

   기대 결과: 서버 실행 OS 계정이 두 파일을 읽을 수 있다. 다른 일반 OS 계정은 개인키를 읽거나 바꿀 수 없도록 제한하기를 권장한다.

2. 설정의 해당 `cert_file`·`key_file` 경로를 새 쌍으로 함께 바꾸어 저장한다.

   TLS 경로는 대시보드/API에서 보기 전용이므로 경로 변경은 외부 편집기로 한다. 이때 대시보드 저장을 동시에 하지 않는다. 같은 경로의 파일 내용을 교체하는 방식도 자동 감지한다. 그 경우 설정 파일 revision은 그대로여도 파일 상태가 바뀔 수 있다. 두 내용이 유효한 쌍이 될 때까지 한쪽만 바뀐 후보는 거절하고 기존 TLS를 유지한다.

3. 로그에서 `reload_applied`를 확인한다.

   실패·일시 소멸·키 불일치이면 이전 인증서·IP 정책·연결을 그대로 유지한다. 비재로드 차이가 남았으면 먼저 되돌린다.

4. 새 TLS 연결에서 인증서를 확인한다.

   관리 HTTPS는 새 브라우저 연결의 인증서 보기로 일련번호·지문을 대조한다. WSS는 TLS 클라이언트로 설정한 호스트의 새 전체 핸드셰이크를 확인한다. 예: `openssl s_client -connect HOST:PORT -servername HOST -showcerts`로 받은 공개 인증서를 대조한다. `HOST:PORT`는 실제 TLS 주소다. 이전 TLS 연결이 새 인증서로 바뀔 것으로 기대하지 않는다. 클라이언트의 인증서 신뢰·호스트 검증을 끄지 않는다.

### 3.7 로그 회전과 보관

JSON Lines는 한 줄에 JSON 객체 하나를 기록하는 형식이다. `log.output = "stdout"`이면 표준 출력으로 쓴다. 파일 경로이면 파일이 없을 때 만들고 기존 내용 뒤에 이어 쓴다. 파일 경로의 상위 디렉터리는 미리 만들어야 한다. 출력 경로 변경은 재시작해야 한다.

1. 실행 중인 `relay.log`의 이름을 아직 없는 보관 이름으로 바꾼다.

   Windows: `Rename-Item -LiteralPath .\relay.log -NewName relay.previous.log`

   Ubuntu: `mv -- ./relay.log ./relay.previous.log`

   보관 이름이 이미 있으면 먼저 다른 이름을 정한다. 기존 보관 파일을 덮어쓰지 않는다.

2. 3.4절의 클라이언트를 실행해 새 사건을 만든다.

   이름 변경·삭제·생성 알림을 받으면 서버가 원래 경로를 곧바로 다시 연다. 알림을 놓쳐도 Dp마다 항상 다시 연다. 파일이 없으면 만든다. 회전 직후 다시 열기 전 사건은 옛 파일에 남을 수 있다.

3. `relay.log`에 새 사건이 이어지는지 확인한다.

   다시 열기 실패 시 옛 파일에 계속 쓴다. 실패가 시작될 때 한 번 stderr 경고와 warn `log_reopen_failed`가 남는다. 경로·권한을 복구하면 다음 알림·Dp 주기에 다시 시도한다. 운영자는 옛 파일도 확인해야 한다. 이 방식은 파일 이름 변경을 기준으로 한다. 복사 후 원본 자르기(copytruncate)를 이 절차의 회전 방식으로 쓰지 않는다.

로그 파일은 열리지만 로그 디렉터리의 알림 등록만 실패하면 서버는 시작을 계속한다. `reload_watch_failed`(`reason=notify_error`)를 기록하고 Dp마다 다시 열기와 알림 등록을 재시도한다. 설정·TLS 감시 준비 실패에는 이 예외를 적용하지 않는다.

로그 회전 시점·보관 기간·접근 권한은 운영자 책임이다. 다음은 제품 강제 정책이 아닌 **권장** 사항이다.

- 장애 분석·감사 요구와 디스크 용량을 근거로 보관 기간을 정하기를 권장한다.
- 서버 실행 OS 계정만 로그를 쓰고 승인된 운영자만 읽도록 제한하기를 권장한다.
- 로그 디렉터리를 다른 일반 OS 계정이 바꾸지 못하게 하기를 권장한다. Ubuntu에서 서버가 새로 만드는 로그 파일의 권한은 umask를 따른다. 디렉터리 권한(예: `0700`)으로 다른 계정의 읽기를 막기를 권장한다.
- 보관 파일의 접근 권한도 원본과 같이 제한하고 필요하면 보호된 저장소로 옮기기를 권장한다.
- 디스크 여유와 stderr 경고를 감시하기를 권장한다. 로그에는 관리자 계정명과 접속 관련 정보가 포함될 수 있다.

#### 3.7.1 대시보드 최근 로그

`[recent_logs] max_entries = 1000`은 메모리에 최근 사건 1000개를 보관하는 기본값이다. 범위는 0~10000이며 `0`이면 보관하지 않는다. 값을 바꾸면 재시작해야 한다. 재시작하면 목록은 빈다. 보관 수를 늘릴 때는 최대 로그 줄 크기에 비례해 메모리를 더 쓴다는 점을 반영한다.

관리자로 로그인한 뒤 `/#/logs`를 연다. 수준·사건명 필터와 값 검색으로 좁힐 수 있다. 자동 갱신은 P 주기이며 일시정지할 수 있다. 세션 상세의 "로그 보기"로 해당 세션에 관련된 사건을 열 수 있다. 내려받기는 현재 필터 결과를 JSON Lines로 저장한다.

이 목록은 `log.output`과 출력 쓰기 성공 여부에 관계없이 채운다. 파일 출력 실패여도 화면에 사건이 보일 수 있으므로 파일 기록 성공의 증거로 삼지 않는다. 목록은 오래된 항목부터 버리며 영속 보관·감사 로그를 대신하지 않는다. 기준은 [운영 스펙](../specs/operations.md)을 따른다.

### 3.8 정상 종료와 재시작

1. 서버를 실행한 터미널에서 Ctrl+C를 한 번 누른다.

   Ubuntu는 SIGINT(Ctrl+C)와 `kill -TERM <PID>`를 정상 종료 요청으로 받는다. Windows는 Ctrl+C와 Ctrl+Break를 받으며 SIGTERM은 없다. 신규 중계를 중지하고 미완성 작업을 정리한다. 활성 세션에는 종료 시작 때 한 번 읽은 `timeouts.shutdown_grace_secs`(G)의 유예를 준다. 기본값은 30초다. G가 0이면 바로 정리한다. 유예 중 관리 로그인·조회·세션 종료와 설정 GET/PATCH는 계속 가능하다. watcher의 새 적용은 멈추지만 PATCH commit은 허용된다. 이때 G를 편집해도 이미 시작한 유예 기한은 늘거나 줄지 않으며 신규 중계 수락도 재개하지 않는다.

2. 서버의 종료 결과를 확인한다.

   Windows: 서버 명령이 돌아온 같은 PowerShell에서 `$LASTEXITCODE`를 읽는다.

   Ubuntu: 서버 명령이 돌아온 같은 bash에서 `echo $?`를 실행한다.

   기대 결과: `0`이고 로그에 `server_stopped`가 있다. G 만료로 남은 세션을 정리한 경우도 정상 종료 코드 0이다.

콘솔 실행에서 유예 중 두 번째 종료 요청은 남은 유예를 건너뛰고 즉시 강제 정리한다. Windows 콘솔 창 닫기도 처음부터 이 방식으로 처리한다. G를 기다리지 않으며 OS의 기본 5초 제한 안에 정리와 종료 기록을 마쳐야 한다. 일반적인 콘솔 운영 종료에는 Ctrl+C를 권장한다. 서비스 중지는 3.3.1절을 따른다.

Windows의 대화형 콘솔 실행은 로그오프·시스템 종료 때 정상 유예·종료 기록을 보장하지 않는다. 서비스는 로그오프로 멈추지 않는다. 서비스의 시스템 종료·재시작 알림 처리는 [운영 스펙 3.7절](../specs/operations.md#37-windows-서비스-실행)을 따른다. SIGKILL·작업 관리자 종료·전원 장애에서는 정상 종료를 보장하지 않는다. 다시 시작하면 이전 소켓·대기·세션·관리 로그인은 복원되지 않는다.

### 3.9 역방향 프록시로 TLS 종료

중계 WS·WSS 리스너만 대상이다. 이 예시는 Nginx가 외부 WSS를 받고 같은 호스트의 평문 `ws` 리스너로 전달하는 구성이다. Nginx 설정과 연결 절차는 이 저장소에서 검증하지 않았다. **추정 / 미실행**이다. 같은 연결 구조의 자동 시나리오 WS-035는 시험 코드 안의 TLS 종료 프록시를 쓰며 Nginx를 실행하지 않는다. 실제 Nginx 버전·경로·인증서·방화벽에 맞게 바꾸고 4절의 별도 검증을 해야 한다.

1. 프록시에서 제공할 호스트명의 인증서·개인키를 준비한다. 아래 `relay.example.test`와 `/etc/nginx/tls/` 경로를 실제 값으로 바꾼다.
2. 운영 `relay.toml`에 다음 구성을 적용한다. 기존 WSS 예시 대신 쓰는 별도 전체 예시다. 문서용 클라이언트 IP를 실제 허용 주소로 바꾼다.

   ```toml
   [admin]
   accounts_file = "admin-accounts.toml"

   [relay.ws]
   listen = "127.0.0.1:8080"
   trusted_proxies = ["127.0.0.1"]

   [relay.ws.viewer]
   enabled = true
   allow = ["192.0.2.10"]
   allowed_origins = ["https://viewer.example.test"]

   [relay.ws.server]
   enabled = true
   allow = ["192.0.2.30"]
   allowed_origins = []

   [mode2]
   enabled = true

   [keepalive]
   ws_ping_secs = 30
   ws_pong_timeout_secs = 30

   [management]
   listen = "127.0.0.1:18080"

   [status]
   listen = "127.0.0.1:18081"

   [recent_logs]
   max_entries = 1000

   [log]
   output = "relay.log"
   ```

   `trusted_proxies`는 서버가 소켓에서 실제로 보는 프록시 주소여야 한다. 이 예시는 Nginx의 접속을 `127.0.0.1`로 고정한다. 별도 호스트라면 해당 프록시의 실제 주소만 넣고 리스너 주소·방화벽도 함께 바꾼다.

3. Nginx의 `http` 블록 안에 다음 `map`과 `server`를 둔다. `proxy_pass`에는 URI를 붙이지 않는다. 그래야 `/relay/viewer`와 `/relay/server`를 그대로 전달한다.

   ```nginx
   map $http_upgrade $connection_upgrade {
       default upgrade;
       ''      close;
   }

   server {
       listen 443 ssl;
       server_name relay.example.test;
       ssl_certificate     /etc/nginx/tls/relay-chain.pem;
       ssl_certificate_key /etc/nginx/tls/relay-key.pem;
       ssl_protocols TLSv1.2 TLSv1.3;

       location /relay {
           proxy_pass http://127.0.0.1:8080;
           proxy_http_version 1.1;
           proxy_set_header Host $host;
           proxy_set_header Upgrade $http_upgrade;
           proxy_set_header Connection $connection_upgrade;
           proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
           proxy_read_timeout 120s;
           proxy_send_timeout 120s;
       }
   }
   ```

   120초는 이 예시의 Kw ping 30초와 pong 제한 30초보다 길다. Kw를 바꾸면 프록시 읽기·쓰기 제한도 그 주기·응답 대기보다 길게 정한다. 경로 일치 여부와 꺼진 역할의 404 판정은 VNC Relay가 맡는다.

4. Nginx 호스트에서 `nginx -t`로 설정을 검사한다. 성공한 뒤 `nginx -s reload`로 적용한다. 기존 배치의 실행 권한과 설정 경로가 다르면 그 배치 명령을 쓴다.
5. 3.8절로 VNC Relay를 재시작한 뒤 외부 Viewer의 `wss://relay.example.test/relay/viewer`와 Server의 `wss://relay.example.test/relay/server`로 Mode II 중계를 확인한다. 클라이언트는 인증서·호스트 검증을 생략하지 않는다.

`trusted_proxies`에는 프록시 주소만 넣어야 한다. 클라이언트 대역이나 `0.0.0.0/0`·`::/0` 같은 전체 범위를 넣어서는 안 된다. 그렇지 않으면 비신뢰 상대가 주소를 위조하거나 정상 클라이언트 체인이 거절될 수 있다. 전체 신뢰 경고는 `trust_all_proxies`이며 필드는 `listener`·`item`·`phase`다. 시작·재로드·설정 편집 때의 경고는 OPS-067 자동 시나리오로 확인한다. 경고는 설정 실수를 알리는 보조 수단이므로 경고에 의존해 안전성을 판단하지 않는다.

신뢰 프록시에서 온 `X-Forwarded-For`는 오른쪽부터 검사한다. 처음 만나는 `trusted_proxies` 밖 주소를 클라이언트 주소로 쓰며 그 왼쪽의 위조 요소는 읽지 않는다. Nginx의 `$proxy_add_x_forwarded_for`는 실제 접속 주소를 오른쪽에 덧붙인다. 여러 프록시를 거치면 각 프록시가 같은 방식으로 실제 상대 주소를 전달하고 모든 중간 프록시 주소만 신뢰 목록에 넣어야 한다. 신뢰하지 않은 소켓 상대의 전달 헤더는 무시한다([WebSocket 계약 1.8절](../contracts/websocket-relay-v2.md#18-신뢰-프록시와-x-forwarded-for)).

헤더 누락·빈 값·검사 중 잘못된 요소·모든 요소가 신뢰 프록시 주소인 체인은 `HTTP 400`과 `forwarded_for_invalid`로 거절한다. 이때 등록과 `forwarded_ip`는 없다. 프록시 주소가 역할 허용 목록에 있어도 모두 신뢰하는 체인은 거절한다([WS-033·WS-036](../specs/websocket.md#7-수용-테스트)). 따라서 이 예시에서 Nginx 호스트 자체의 루프백 요청은 프록시를 거치면 거절된다. `127.0.0.1:8080`에 전달 헤더 없이 직접 접속해도 거절된다. 출발지 `127.0.0.1`이 `trusted_proxies`에 있어 신뢰 프록시 연결로 취급되기 때문이다. 이 출발지의 직접 접속은 유효한 전달 헤더가 있어야 하며, 같은 전달 주소 판정과 역할 `allow` 검사를 받는다. 전달 헤더 없이 직접 접속하려면 출발지가 그 리스너의 `trusted_proxies` 밖에 있고 해당 역할의 `allow`에 들어야 한다. 이 예시의 호스트에서 전달 헤더 없이 직접 시험해야 하면 신뢰 프록시를 두지 않은 다른 리스너(예: TCP 진입점이나 `trusted_proxies`가 빈 WSS 리스너)를 쓴다.

**프록시→VNC Relay 구간은 운영자가 보호해야 한다.** 같은 호스트의 루프백 또는 접근을 제한한 사설망을 권장한다. 사설망 자체가 암호화를 보장하는 것은 아니다. 도청·변조 위험이 있으면 그 구간에도 TLS 등 보호를 적용한다. 프록시를 통하지 않는 평문 리스너의 외부 접근은 방화벽으로 막아야 한다. 관리 경로 `/`·`/api/`는 이 예시에 추가하지 않는다. 관리 리스너는 내장 HTTPS 또는 루프백 HTTP와 기존 로그인 규칙을 그대로 쓴다.

### 3.10 접속 ticket 켜기와 공개 키 교체

WS·WSS 역할마다 접속 ticket 확인을 켤 수 있다. 접속 ticket(ticket)은 발급 쪽이 ES256으로 서명한 짧은 수명의 JWS다. 켠 역할의 WS·WSS 업그레이드는 유효한 ticket이 있을 때만 101로 답한다. 기본값은 꺼짐이다. 형식과 확인 순서는 [WebSocket 계약 1.9절](../contracts/websocket-relay-v2.md#19-접속-ticket)을, 설정 키는 [설정 계약 1.2절](../contracts/config-v4.md#12-키-표)과 [1.11절](../contracts/config-v4.md#111-접속-ticket-공개-키-파일-형식)을 따른다. 수동 확인 범위는 4절에 있다.

먼저 다음 조건을 확인한다.

- `[relay.ticket.*]` 표는 설정 계약 4.1.0이다. 이 표를 쓴 파일은 이전 실행파일이 `config_unknown_key`로 거절한다. 실행파일을 먼저 바꾼 뒤 표를 더한다.
- ticket은 TCP 진입점에 적용하지 않는다. 같은 역할의 TCP 진입점이 켜져 있으면 `ticket_tcp_bypass` 경고가 남는다. ticket으로만 접속을 허용하려면 그 역할의 TCP 진입점을 끈다.
- 평문 `[relay.ws]`에서 그 역할을 켜면 `ticket_plaintext_ws` 경고가 남는다. ticket 원문이 암호화되지 않은 구간을 지나기 때문이다. WSS 또는 보호된 구간을 쓴다.
- 서버와 발급 쪽의 시계를 맞춘다. 허용 오차는 5초로 고정이다.

1. 발급 쪽에서 역할마다 다른 P-256 서명 키 쌍을 만든다. vnc-relay에는 공개 키만 둔다. 발급 쪽 없이 시험할 때는 3.10.1절의 개발 전용 명령으로 만들 수 있다.

   기대 결과: 두 역할을 함께 켜면 두 공개 키 파일의 키가 서로 다르다. 같은 키가 있으면 `ticket_keys_invalid`로 거절한다.

2. 공개 키를 JWKS 파일로 저장한다. 다음은 `keys/viewer-ticket.jwks`의 예시다. `x`·`y`는 실제 좌표로 바꾼다.

   ```json
   {"keys":[{"kty":"EC","crv":"P-256","kid":"viewer-2026-11","use":"sig","alg":"ES256","x":"<x 좌표>","y":"<y 좌표>"}]}
   ```

   `x`·`y`는 32바이트 좌표를 패딩 없는 base64url로 쓴 값이다. 파일은 64 KiB 이하, 키는 1~16개다. `kid`는 파일 안에서 고유해야 한다. 개인키 멤버 `d`가 있으면 거절한다. 공개 키는 비밀이 아니다. 다만 파일을 바꿀 수 있는 계정은 자신이 만든 키로 ticket을 통과시킬 수 있다. 그래서 다른 일반 OS 계정이 파일을 바꾸지 못하게 제한하기를 권장한다.

3. `relay.toml`에 그 역할의 표를 더한다. 값은 실제 발급 쪽 값으로 바꾼다.

   ```toml
   [relay.ticket.viewer]
   enabled = true
   issuer = "issuer.example/prod"
   relay_host = "relay.example"
   public_keys_file = "keys/viewer-ticket.jwks"
   max_lifetime_secs = 60
   ```

   `enabled = true`이면 `issuer`·`relay_host`·`public_keys_file`이 필수다. `issuer`는 ticket의 `iss`와, `relay_host`는 `rh`와 바이트 단위로 같아야 한다. 상대 경로는 `relay.toml`의 디렉터리를 기준으로 푼다. `max_lifetime_secs`는 10~600초이고 기본값은 60초다. ticket의 `exp − iat`가 이 값을 넘으면 거절한다.

4. 파일을 저장하고 `relay.log`를 확인한다.

   기대 결과: ticket 표의 키는 모두 재로드 대상이다. 재시작 없이 `reload_applied`가 기록되고 `applied`에 바뀐 `relay.ticket.*` 키가 있다. 공개 키 파일에 문제가 있으면 `reload_rejected`(`reason = ticket_keys_invalid`)로 후보 전체를 거절하고 이전 설정을 유지한다. 시작 때 문제가 있으면 `error[ticket_keys_invalid]`를 표시하고 종료 코드 3으로 끝난다. 파일 내용은 로그와 오류에 쓰지 않는다.

5. 클라이언트가 ticket을 붙여 접속하는지 확인한다. 클라이언트는 `Sec-WebSocket-Protocol`에 `binary`와 `ticket.<JWS>`를 함께 보낸다.

   기대 결과: 101 응답의 서브프로토콜은 `binary` 하나다. 로그에 `ticket_accepted`와 `ticket_jti`가 있다. ticket이 없거나 맞지 않으면 HTTP 401과 `WWW-Authenticate: VncRelayTicket`으로 거절한다. `connection_rejected`(`stage = upgrade`)에는 `ticket_missing`·`ticket_invalid`·`ticket_expired`·`ticket_reused`·`ticket_issued_before_start` 가운데 하나가 남는다. 같은 ticket은 한 번만 쓸 수 있다.

6. 서버를 다시 시작하면 클라이언트가 새 ticket을 받아야 한다.

   사용 기록은 메모리에만 있다. 그래서 `iat`가 기동 시각 + 5초 이하인 ticket은 `ticket_issued_before_start`로 거절한다.

공개 키는 다음 순서로 교체한다([설정 계약 1.11절](../contracts/config-v4.md#111-접속-ticket-공개-키-파일-형식)). 경로는 대시보드·API에서 바꿀 수 없으므로 같은 경로의 파일 내용을 바꾼다.

1. 새 서명 키 쌍을 만들고, 새 공개 키를 기존 파일의 `keys`에 더해 저장한다.

   기대 결과: `reload_applied`의 `applied`에 `relay.ticket.<역할>.public_keys_file`이 있다.

2. 발급 쪽을 새 서명 키로 바꾼다.
3. 그 역할의 `max_lifetime_secs` + 10초가 지난 뒤 옛 공개 키를 파일에서 지운다.

   기대 결과: 다시 `reload_applied`가 기록된다. 이미 101을 보낸 연결은 영향을 받지 않는다.

`enabled = false`로 바꾸면 다음 업그레이드부터 ticket 없이 받는다. 이미 101을 보낸 연결의 중계 ID 대조와 세션 만료 타이머는 유지한다. 사용 기록도 지우지 않는다. 그 역할이 WS 또는 WSS에서 켜져 있으면 `enabled`·`issuer`·`relay_host`·`max_lifetime_secs`를 설정 편집 화면의 마지막 구역 "접속 ticket"에서도 바꿀 수 있다(3.5.1절). `enabled`를 켜는 변경의 확인 대화상자는 ticket 없는 요청을 거절한다고 경고한다.

101 뒤 초기 요청의 중계 ID가 ticket의 `rid`와 다르면 Close 4012(`ticket_relay_mismatch`)로 끝낸다. Viewer ticket으로는 Mode I을 쓸 수 없다. ticket에 세션 만료 시각 `sx`가 있으면 그 시각에 연결을 끝낸다. 활성 세션에서는 WS·WSS 쪽을 Close 4013으로 닫고 TCP 쪽은 소켓을 닫는다. 최근 종료 목록의 원인은 `session_expired`이고 대시보드는 "세션 만료"로 표시한다.

#### 3.10.1 시험용 키와 ticket으로 확인하기

발급 쪽 없이 3.10절을 확인할 때는 개발 전용 명령 `cargo xtask ticket`으로 시험용 키와 ticket을 만들 수 있다. 이 절을 실행하려면 저장소를 받고 [개발 환경](set-up-and-verify.md)을 준비해야 한다. 명령은 저장소 루트에서 실행한다. 배포 묶음과 실행파일에는 이 명령이 없으므로 묶음만으로는 이 절을 실행할 수 없다. 만든 키는 시험에만 쓰고 운영 발급 쪽 키로 쓰지 않는다. 옵션과 파일 형식은 [xtask 모듈 설계](../architectures/xtask.md)에 있다. 아래는 Viewer 예시다. Server를 시험하려면 `viewer`를 `server`로, 경로 `/relay/viewer`를 `/relay/server`로 바꾼다.

1. 저장소 루트에서 역할의 키 쌍을 만든다.

   ```shell
   cargo xtask ticket keys viewer
   ```

   기대 결과: `공개 키:`·`개인키:`·`kid:` 세 줄이 출력된다. 기본 위치는 Cargo 대상 디렉터리의 `ticket/`이다. 보통 `target/ticket/`이다. 다른 위치는 `--dir <디렉터리>`로 정한다. 같은 이름의 파일이 이미 있으면 아무것도 쓰지 않고 종료 코드 1로 끝난다.

2. 운영 디렉터리에 `keys` 디렉터리가 없으면 만든다. 배포 묶음에는 이 디렉터리가 없다. 이어 공개 키 파일 `viewer-ticket.jwks`를 `keys/viewer-ticket.jwks`로 복사한다. 개인키 파일 `viewer-ticket.private.jwk`는 복사하지 않는다.

3. 서버를 멈춘 상태에서 `relay.toml`의 WS 역할과 ticket 표를 켠다. 리스너 주소와 역할의 `enabled`는 재시작해야 적용되기 때문이다. 예시 설정에서는 다음 줄의 `# `를 지우고 `enabled`를 `true`로 바꾼다.

   - `[relay.ws]`의 `listen`
   - `[relay.ws.viewer]`의 `enabled`와 `allow`
   - `[relay.ticket.viewer]`의 `enabled`, `issuer`, `relay_host`, `public_keys_file`

   `issuer`·`relay_host`의 예시 값은 `issue` 명령의 기본값(`issuer.example/prod`, `relay.example`)과 같다. 값을 바꾸면 4단계에서 `--issuer`·`--relay-host`로 같은 값을 준다.

4. 서버를 시작하고(3.3절) 5초 넘게 기다린다. 이어 저장소 루트에서 ticket을 만들고 업그레이드 요청을 보낸다. `1234`는 시험용 중계 ID다.

   Windows:

   ```powershell
   $t = cargo xtask ticket issue viewer 1234
   curl.exe -i --max-time 2 http://127.0.0.1:15902/relay/viewer -H "Connection: Upgrade" -H "Upgrade: websocket" -H "Sec-WebSocket-Version: 13" -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" -H "Sec-WebSocket-Protocol: binary, ticket.$t"
   ```

   Ubuntu:

   ```shell
   t=$(cargo xtask ticket issue viewer 1234)
   curl -i --max-time 2 http://127.0.0.1:15902/relay/viewer -H "Connection: Upgrade" -H "Upgrade: websocket" -H "Sec-WebSocket-Version: 13" -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" -H "Sec-WebSocket-Protocol: binary, ticket.$t"
   ```

   기대 결과: `HTTP/1.1 101`과 `Sec-WebSocket-Protocol: binary`가 출력된다. 그 뒤에 서버가 보낸 첫 WebSocket 프레임이 `RFB 000.000`과 함께 보인다. `relay.log`에 `ticket_accepted`가 남는다. curl은 WebSocket 통신을 이어 가지 않으므로 2초 뒤 시간 초과(종료 코드 28)로 끝난다.

5. 같은 `curl` 줄을 한 번 더 실행한다.

   기대 결과: HTTP 401과 `WWW-Authenticate: VncRelayTicket`이 출력된다. `relay.log`의 `connection_rejected`에 `ticket_reused`가 남는다. 연결 시도마다 `issue`로 새 ticket을 만들어야 한다.

수명은 `--lifetime <초>`(기본 60, 1~600)로 정한다. 설정의 `max_lifetime_secs`보다 길면 거절된다. 세션 만료 시각 `sx`는 `--session <초>`, 발급 쪽 세션 식별자 `sub`는 `--subject <값>`으로 더한다. 실제 VNC 클라이언트로 확인할 때는 출력한 ticket을 클라이언트에 넘긴다.

공개 키 교체를 확인할 때는 `--dir`로 다른 디렉터리에 같은 역할의 키를 하나 더 만든다. 그 JWKS 파일의 `keys` 배열에 있는 키 객체를 운영 디렉터리 공개 키 파일의 `keys` 배열에 더한다. `kid`는 키마다 다르므로 겹치지 않는다. 이후 `issue`에 같은 `--dir`를 주면 새 키로 서명한다. 나머지 순서는 3.10절의 교체 절차를 따른다.

시험용 공개 키를 운영 디렉터리에 남겨 두지 않는다. 남겨 두면 시험용 개인키를 가진 사람이 받아들여지는 ticket을 만들 수 있다. `enabled`를 `false`로 되돌려도 파일이 남아 있으면 나중에 다시 켤 때 시험용 키가 다시 쓰인다. 그래서 시험을 마치면 다음 가운데 하나를 한다.

- ticket을 계속 쓰면 공개 키 파일의 내용을 실제 발급 쪽 공개 키로 바꾸고, `issuer`·`relay_host`도 실제 값으로 맞춘다(3.10절 2·3단계). 이어 `reload_applied`를 확인한다.
- ticket을 쓰지 않으면 `[relay.ticket.viewer]`의 `enabled`를 `false`로 바꾸고 `reload_applied`를 확인한다. 이어 시험용 공개 키 파일을 지운다.

키 디렉터리도 지우기를 권장한다. 기본 위치는 `cargo clean`으로도 지워진다.

### 3.11 IP 정보 DB 준비와 갱신

대시보드의 "접속 출처" 화면은 연결 시도·거절·중단·세션 수를 클라이언트 주소마다 보인다. 국가와 ASN(인터넷 경로를 운영하는 조직의 번호)까지 보려면 IP 정보 DB 파일이 필요하다. IP 정보 DB는 IP 주소 범위마다 국가나 ASN을 담은 MMDB(MaxMind DB 형식) 파일이다. 이 절은 선택이다. 파일이 없으면 국가·ASN 칸만 "알 수 없음"이고 주소별 수는 그대로 센다. 동작 기준은 [운영 스펙 3.9절](../specs/operations.md#39-접속-출처-통계)과 [설정 계약 1.12절](../contracts/config-v4.md#112-ip-정보-db-파일-형식), 결정 근거는 [ADR-0012](../decisions/ADR-0012-local-ip-info-databases.md)에 있다.

먼저 다음 조건을 확인한다.

- 서버는 IP 정보를 찾으려고 외부에 요청하지 않는다. 파일도 내려받지 않는다. 운영자가 제공자 사이트에서 받아 운영 디렉터리에 둔다.
- 배포 묶음과 저장소에는 DB 파일이 없다. 제공자의 이용 조건은 파일을 받은 운영자가 지켜야 한다.
- `[ip_info]` 표는 설정 계약 4.3.0이다. 이 표를 쓴 파일은 이전 실행파일이 `config_unknown_key`로 거절한다. 실행파일을 먼저 바꾼 뒤 표를 더한다.
- 경로를 바꾸면 재시작해야 한다. 같은 경로의 파일 내용을 바꾸면 재시작 없이 다시 읽는다.
- 파일은 256 MiB 이하의 압축을 푼 `.mmdb` 파일이어야 한다. 서버는 파일 전체를 메모리로 읽으므로 파일 크기만큼 메모리를 더 쓴다.

지원하는 제공자는 다음 세 곳이다. 2026-11-20에 각 제공자 페이지에서 확인한 내용이다. 받기 전에 제공자 페이지의 현재 조건을 다시 확인한다.

| 제공자 | 받을 파일 | 계정 | 갱신 | 이용 조건 |
| --- | --- | --- | --- | --- |
| [DB-IP Lite](https://db-ip.com/db/lite.php) | 국가 `dbip-country-lite-<연>-<월>.mmdb.gz`, ASN `dbip-asn-lite-<연>-<월>.mmdb.gz` | 필요 없음 | 매월 | CC BY 4.0. 결과를 보이는 웹 화면에 `https://db-ip.com` 링크를 둬야 한다 |
| [IPinfo Lite](https://ipinfo.io/developers/ipinfo-lite-database) | 국가와 ASN을 함께 담은 `ipinfo_lite.mmdb` 하나 | 필요. 내려받을 때 계정 토큰을 쓴다 | 제공자 페이지의 갱신 시각을 확인한다 | CC BY-SA 4.0. `https://ipinfo.io` 링크로 출처를 밝혀야 한다 |
| [MaxMind GeoLite](https://dev.maxmind.com/geoip/geolite2-free-geolocation-data) | 국가 `GeoLite2-Country.mmdb`(또는 `GeoLite2-City.mmdb`), ASN `GeoLite2-ASN.mmdb` | 필요. 계정 ID와 라이선스 키를 쓴다 | 새 판이 나오면 바로 써야 하고 옛 판은 30일 안에 지워야 한다 | [GeoLite EULA](https://www.maxmind.com/en/geolite/eula). 출처 문장을 보여야 한다 |

대시보드는 쓰는 DB의 제공자를 파일의 `database_type`으로 판정하고 그 제공자의 출처 표시를 화면에 보인다. 운영자가 화면에 출처 표시를 따로 더하지 않아도 된다. 대시보드 밖에서 결과를 다시 쓰거나 공유하면 그곳에도 제공자의 조건이 적용된다.

처음 설정은 다음 순서로 한다. 예시는 DB-IP Lite이며, 운영 디렉터리(`relay.toml`이 있는 디렉터리)에서 실행한다.

1. 운영 디렉터리에 `ipdb` 디렉터리를 만든다.
2. 제공자 사이트에서 파일을 받는다. DB-IP Lite는 [내려받기 페이지](https://db-ip.com/db/lite.php)에서 이용 조건에 동의한 뒤 MMDB 파일을 받는다.
3. 받은 `.gz` 파일의 압축을 풀어 `ipdb`에 둔다.

   Windows PowerShell:

   ```powershell
   $in = [IO.File]::OpenRead("dbip-country-lite-2026-11.mmdb.gz")
   $gz = [IO.Compression.GZipStream]::new($in, [IO.Compression.CompressionMode]::Decompress)
   $out = [IO.File]::Create("ipdb\dbip-country-lite.mmdb")
   $gz.CopyTo($out); $out.Close(); $gz.Close()
   ```

   Ubuntu:

   ```shell
   gunzip -c dbip-country-lite-2026-11.mmdb.gz > ipdb/dbip-country-lite.mmdb
   ```

   ASN 파일도 같은 방법으로 `ipdb/dbip-asn-lite.mmdb`에 둔다. IPinfo Lite의 `ipinfo_lite.mmdb`는 압축 파일이 아니므로 그대로 둔다. GeoLite는 `tar.gz` 묶음 안의 `.mmdb` 파일만 꺼내 둔다.

4. 서버 실행 계정이 `ipdb`의 파일을 읽을 수 있는지 확인한다. Windows 서비스는 3.3.1절에서 데이터 폴더에 준 읽기 권한이 하위 폴더에도 적용되는지 확인한다.
5. 서버를 정상 종료한다(3.8절).
6. `relay.toml`에 `[ip_info]` 표를 더한다. 상대 경로는 `relay.toml`의 디렉터리를 기준으로 푼다.

   ```toml
   [ip_info]
   country_db = "ipdb/dbip-country-lite.mmdb"
   asn_db = "ipdb/dbip-asn-lite.mmdb"
   ```

   IPinfo Lite는 두 키에 같은 경로(`"ipdb/ipinfo_lite.mmdb"`)를 쓴다. 서버는 이 파일을 한 번만 읽는다. 한 키만 쓰면 다른 정보는 "알 수 없음"으로 둔다.

7. 서버를 시작하고(3.3절) 운영 로그를 확인한다.

   기대 결과: `server_ready` 전에 키마다 `ip_db_loaded`(`phase = startup`)가 한 줄씩 남는다. `config_key`는 `ip_info.country_db` 또는 `ip_info.asn_db`이고, `database_type`과 `build_time`이 있다. 파일 경로는 로그에 쓰지 않는다.

8. 대시보드의 "접속 출처"를 연다.

   기대 결과: "IP 정보 DB"의 두 행이 "사용 중"이고 제공자 출처 표시가 보인다. 연결이 들어오면 공인 주소의 국가와 ASN이 나온다. 루프백·사설 주소는 "로컬" 배지가 붙고 찾지 않는다.

읽지 못하면 서버는 시작을 계속하고 그 키의 정보 없이 운영한다. 운영 로그에 `ip_db_unavailable` 경고가 남고 화면의 그 행은 "읽기 실패"다. `reason`이 `file_unreadable`이면 경로와 권한, `file_too_large`이면 파일 크기, `invalid_format`이면 압축을 풀었는지와 파일이 손상되지 않았는지 확인한다. `unsupported_data`이면 국가 키에 ASN 전용 파일을 넣는 것처럼 키와 파일 종류가 맞지 않는다. 파일을 고치면 서버가 자동으로 다시 읽는다.

파일은 다음 순서로 갱신한다. 경로를 바꾸지 않으므로 재시작이 필요 없다.

1. 새 파일을 같은 디렉터리에 임시 이름으로 받거나 풀어 둔다. 예: `ipdb/dbip-country-lite.mmdb.new`.
2. 임시 파일로 기존 파일을 바꾼다. 같은 디렉터리 안의 이름 바꾸기는 한 번에 끝나므로, 서버가 반쯤 복사된 파일을 읽지 않는다.

   Windows PowerShell:

   ```powershell
   Move-Item -Force ipdb\dbip-country-lite.mmdb.new ipdb\dbip-country-lite.mmdb
   ```

   Ubuntu:

   ```shell
   mv ipdb/dbip-country-lite.mmdb.new ipdb/dbip-country-lite.mmdb
   ```

3. 운영 로그를 확인한다.

   기대 결과: Dp + D 안에 `reload_change_detected`와 `ip_db_loaded`(`phase = reload`)가 남는다. 기본값으로는 35초 안이다. 설정 파일은 바뀌지 않았으므로 `reload_applied`는 남지 않는다. 화면의 "빌드 시각"이 새 파일의 값으로 바뀐다. 이미 센 사건의 국가·ASN은 바뀌지 않고, 그 뒤 사건부터 새 파일로 찾는다.

새 파일을 읽지 못하면 이전 파일의 내용을 계속 쓴다. `ip_db_unavailable`(`phase = reload`)이 남고 화면의 행은 "이전 DB 사용 중"이다. 정상 파일로 다시 바꾸면 자동으로 다시 읽는다. 같은 잘못된 내용을 다시 감지하면 경고를 다시 쓰지 않는다.

GeoLite는 MaxMind가 권장하는 GeoIP Update 프로그램(`geoipupdate`)으로 정기 갱신할 수 있다. 데이터 디렉터리를 `ipdb`로 정하면 `GeoLite2-Country.mmdb`·`GeoLite2-ASN.mmdb`를 그 이름으로 둔다. `[ip_info]`의 경로도 이 이름으로 맞춘다. 설정 방법은 [MaxMind 갱신 안내](https://dev.maxmind.com/geoip/updating-databases)를 따른다. DB-IP Lite는 매월 새 파일이 나오므로 월 1회 위 갱신 순서를 실행하기를 권장한다.

## 4. 검증

각 OS에서 기본 TCP 배치의 3.1~3.8절을 실행한다. TLS 배치가 아니면 3.6절은 해당 없음으로 기록한다. v3→v4 이관은 3.2.2절, Viewer 경로 이전은 3.2.3절, WSS 두 역할은 3.2.1절, 프록시 TLS 종료는 3.9절, 접속 ticket은 3.10절, IP 정보 DB는 3.11절을 각각 추가 검증한다. OPS-031의 일반 배포 확인에 없는 TLS 동작을 확인한 것처럼 표시하지 않는다. 인증서 교체 자체의 자동 증거는 OPS-024가 담당한다.

자동 시험의 현재 결과는 `cargo xtask scenarios`가 만드는 `scenario-report.md`와 CI에서 확인한다([테스트 컨벤션](../conventions/testing.md)). `tests/distribution.rs`와 `tests/windows_console.rs`는 예시를 복사해 포트·경로만 바꾸고 CLI 계정 준비·시작·양방향 중계·IP 재로드·정상 종료를 확인한다. `cargo test`는 debug 실행파일을 빌드하므로 release 실행파일과 안내 전체의 증거를 대신하지 않는다. OPS-031 자동 표식은 부분이다.

수동 기록에는 날짜, 대상 버전·커밋과 미커밋 범위, OS 버전·아키텍처, `rustc --version`, 빌드 명령·결과, 실행파일 버전·SHA-256, 각 단계 명령과 관찰값을 남긴다. Windows 해시는 `Get-FileHash .\vnc-relay.exe -Algorithm SHA256`, Ubuntu 해시는 `sha256sum ./vnc-relay`로 읽는다. 비밀 없는 로그와 종료 코드를 붙인다. 두 OS 기록과 자동 시험을 함께 검토해 OPS-031의 확인 범위를 판단한다.

아래 표는 항목별 최신 수동 기록이다. 실제 noVNC·UltraVNC와 기기 사이의 상호운용 증거로 확대하지 않는다.

| 관찰 대상·날짜 | 대상·환경 | 방법과 실제 결과 | 확인 한계 |
| --- | --- | --- | --- |
| 계정 준비·관리 로그인·로그 회전, 2026-11-05 | 커밋 `1c7c7a9`의 release 실행파일 `vnc-relay 0.1.0`. Windows 11 Pro 10.0.26200 x64, Rust 1.99.1. 실행파일 SHA-256 `120f319ae67bc1907a3f1e7fcfdb7d5f425e84778a554e8747c8dfbf3e667278` | 3.2·3.3·3.7절을 의사 터미널과 브라우저로 실행했다. `admin add opsadmin`의 두 프롬프트에서 입력 글자가 보이지 않았고 종료 코드는 0이었다. 계정 파일에 입력 문자열은 없고 Argon2id `m=19456,t=2,p=1`이었다. `admin list`는 `opsadmin`만 보였다. 브라우저 로그인 뒤 서버 상태는 `Ready`였다. 로그 이름을 바꾼 뒤 원래 경로가 다시 생겼다. 회전 뒤 중계 사건 5개는 새 파일에만, 앞선 사건은 이전 파일에만 있었다 | TLS를 끈 기본 예시다. 인증서 교체는 해당 없음이다 |
| 계정 준비·관리 로그인·로그 회전, 2026-11-05 | 같은 커밋의 release 실행파일 `vnc-relay 0.1.0`. Ubuntu 24.04.4 LTS x86_64(WSL2), Rust 1.99.1. 실행파일 SHA-256 `dc7da5485cbf722381aaa8932051c34bb0cc58b475999e313107904c21749c40` | 같은 절차를 의사 터미널로 실행했다. `admin add`의 정상 입력은 종료 코드 0, 계정 파일 권한은 `0600`, 입력 문자열은 없고 Argon2id `m=19456,t=2,p=1`이었다. `admin list`는 `opsadmin`이었다. Windows 브라우저의 WSL localhost 전달로 로그인했고 서버 상태는 `Ready`였다. 로그 회전 결과는 Windows와 같았다. 새 로그 파일은 umask `022`에서 `0644`였다 | TLS를 끈 기본 예시다 |
| Ubuntu 시작·중계·IP 재로드·정상 종료, 2026-11-05 | 위 Ubuntu release 실행파일과 환경 | 3.3~3.5·3.8절을 실행했다. 두 상태 조회는 200, `check-relay.py`는 양방향 중계 통과였다. 설정 저장 0.6초 안에 `reload_applied`가 남았고 중계 재확인도 통과했다. 올바른 서버 PID에 SIGINT를 보낸 실행은 `shutdown_started`와 `server_stopped`를 남기고 종료 코드 0으로 끝났다 | Ubuntu의 실제 운영 배치 교체·이관은 미실행이다 |
| v3→v4 설정 이관·IP 재로드, 2026-11-09 | Windows 11 Pro 10.0.26200 x64, PowerShell 7. Release `v2026.11.2`, 실행파일 SHA-256 `3100e8033c153f6814376377eea6113b50202ff428fa6d1774f4874147875181` | 3.2.2절로 v3의 WSS 공통 키와 Viewer 키를 분리했다. Server 역할·`trusted_proxies`·`[recent_logs]`는 넣지 않았다. 리스너 다섯 개를 대체 루프백 포트로 옮긴 검증 사본을 시작한 뒤 운영 파일을 설치했다. `log_contract = ops-info/2.0.0`, 두 상태 조회 200, `/login/` 200, `/` 303이었다. 두 이름의 TLS 1.3과 두 허용 Origin의 WSS Viewer↔TCP Server, TCP Viewer↔TCP Server 양방향 시험 바이트를 확인했다. 운영 포트와 공유기 NAT 루프백에서도 통과했다. `relay.tcp_viewer.allow`에 `"::1"`을 넣고 되돌린 각각의 저장 뒤 0.42·0.41초에 `reload_applied`가 남았고 TCP 짝 연결도 통과했다 | 시험 클라이언트는 Python이며 RFB 버전 문자열과 시험 바이트만 주고받았다. 관리 재로그인·v4→v3 롤백·Ubuntu 이관은 미실행이다. 검증 사본 출력과 보호된 v3 복구본은 보관하지 않는다 |
| Viewer 경로 이전, 2026-11-14 | Windows 11 Pro 10.0.26200 x64, PowerShell 7. Release `v2026.11.3`, 실행파일 SHA-256 `90880e8dce547503220d2bf70e8bc48c4845618d72a41c97d2e3ad88e4743a7d` | 3.2.3절로 서버를 교체하고 운영 포트에서 Python 시험 클라이언트로 확인했다. 두 이름의 TLS 1.3과 인증서 검증, 두 허용 Origin의 WSS Viewer(`/relay/viewer`)↔TCP Server, TCP Viewer↔TCP Server 양방향 시험 바이트가 일치했다. 다른 Origin은 403, `/relay`·꺼진 `/relay/server`는 404였다. 공유기 NAT 루프백도 통과했다. 외부 웹 서비스의 배포 기록은 개발·운영 두 환경의 URL을 `/relay/viewer`로 바꾸고 두 이름 모두 `/relay` 404·`/relay/viewer` 101을 확인했다고 보고했다 | 외부 웹 서비스의 결과는 이 저장소에서 재실행하지 않았다. 변경된 URL로 실제 noVNC 중계는 미확인이다. 롤백·Ubuntu 경로 이전은 미실행이다 |
| 접속 ticket 켜기·확인·키 교체·끄기, 2026-11-14 | Windows 11 Pro 10.0.26200 x64, Ubuntu 24.04.4 LTS x64(WSL2, 커널 6.6.87.2). 두 OS Release `v2026.11.3`. Ubuntu 실행파일 SHA-256 `dfd49854a7d1a09737720f78835a5eb5a12ab61cbada6d3455d769aa0fad2148`, Windows는 위 경로 이전 기록과 같다. 시험용 키·ticket은 커밋 `eaab500`의 `cargo xtask ticket`(Rust 1.99.1)으로 만들었다 | 3.10·3.10.1절을 루프백 포트와 자체 서명 P-256 인증서로 실행했다. curl의 WSS 요청에는 `-k --http1.1`을 썼다. WS Viewer·WSS Viewer·WSS Server는 ticket이 있으면 101(`binary`)이었다. Viewer는 `RFB 000.000` 프레임 뒤 curl 종료 코드 28이었다. 재사용·누락·다른 역할·만료·시작 전 발급은 각각 401과 `ticket_reused`·`ticket_missing`·`ticket_invalid`·`ticket_expired`·`ticket_issued_before_start`였다. 새 Viewer 공개 키를 더하면 0.5초 뒤 `reload_applied`였고 두 키 모두 101이었다. 70초 뒤 옛 키를 지우면 옛 키 401·새 키 101이었다. Viewer ticket을 끄면 0.5초 뒤 재로드되고 ticket 없는 Viewer는 101, 켜 둔 Server는 401이었다. 두 OS 모두 정상 종료 코드 0이었다. 발급한 ticket 10개의 원문·서명과 공개 키 좌표·JWS 형태 값은 로그에 없었다. 시험 키 디렉터리는 지웠다 | 실제 발급 쪽·클라이언트, 101 뒤 중계 ID 대조·세션 만료, `ticket_keys_invalid`, 키 교체 중 기존 연결 유지, 대시보드 편집, 운영 배치에서 켜기는 이 실행의 범위가 아니다. `-k`를 쓴 결과는 인증서 검증의 증거가 아니다 |
| TCP 짝 중계와 실행파일 교체 복구 스크립트, 2026-11-17 | Windows 11 Pro 10.0.26200 x64, PowerShell 7. Release `v2026.11.4`, 실행파일 SHA-256 `6155ff8c58d4c4fc55e023b1aec12433bec6f701768fef1d9739d45823272181` | 운영 설정의 검증 사본에서 TCP Viewer를 루프백에 켰다. Python 클라이언트로 TCP Viewer↔TCP Server의 양방향 4,096바이트가 일치했다. Ctrl+C 뒤 종료 코드 0이었다. 교체 스크립트는 정상 종료를 확인한 뒤 디렉터리를 옮기고 30초 안에 준비되지 않으면 복원하도록 했다. 이전 실행파일의 시험 사본과 별도 루프백 포트로 교체 성공·복원 경로를 각각 실행해 확인했다 | 운영 포트는 TCP Viewer가 꺼져 있고 WSS 두 역할에 ticket이 필요해 짝 시험을 하지 않았다. 이 복원 시험은 Windows 서비스 전환·교체 스크립트의 복원을 확인한 것이 아니다 |
| Windows 서비스 설치·권한·시작, 2026-11-18 | Windows 11 Pro 10.0.26200 x64, PowerShell. Release `v2026.11.5`. 서버 SHA-256 `cae5595c16d366affc8f7c1931713b3405c3b9239be66f1d0d5098897b3d0e30`, 트레이 앱 `ac3e78ac3c42e055efcfe23f7cecc761241f3a3db60709223f4f4a842605f275` | 3.3.1절로 `%ProgramFiles%\vnc-relay\`에 복사해 관리자 권한으로 설치했다. `service install` 종료 코드 0, `sc.exe qc`는 자동(지연 시작)·`NT SERVICE\vnc-relay`·`VNC Relay`와 절대 설정 경로를 보였다. 운영자 SID 권한은 `CCLCRPWP`였다. 데이터 폴더에 서비스 `(OI)(CI)RX`, 로그 폴더에 `(OI)(CI)M`을 줬다. 새 실행파일 경로의 인바운드 규칙 두 개를 만들고 이전 서버의 로그인 자동 시작 바로 가기는 제거했다. 일반 PowerShell의 `sc.exe start`는 UAC 없이 성공했다. 0.30초 안에 `RUNNING`과 `/readyz` 200을 확인했고 네 리스너가 서비스 프로세스에 속했다 | 인증서 갱신 도구 때문에 데이터는 사용자 프로필 아래 기존 위치에 두었다. 비밀 파일의 상속 차단 권장 단계는 적용하지 않았다. 자동(지연) 시작의 재부팅 확인·서비스 해제·전환 실패 복원은 미실행이다 |
| IP 정보 DB 읽기와 접속 출처 조회, 2026-11-20 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1. 커밋 `40c67bb`에 접속 출처 변경을 더한 미커밋 작업 트리의 debug 실행파일. 시험 도구가 띄운 `target/debug/vnc-relay.exe`의 SHA-256은 `9949de447c7335222a5c2444872dda828ed3415b4ad0098d2740682c85611f02`이다. DB-IP Lite 2026-11 국가·ASN 파일(8,515,063·9,648,279바이트), IPinfo Lite 표본 파일 하나(3,301바이트), MaxMind 시험 파일 `GeoLite2-Country-Test`·`GeoLite2-City-Test`·`GeoLite2-ASN-Test` | 저장소의 실행파일 시험 도구(`tests/common/dashboard_live.rs`)로 임시 자동 시험을 만들어 실행한 뒤 지웠다. 루프백 포트에 서버를 띄우고 네 조합을 차례로 썼다: DB-IP 국가+ASN, IPinfo 한 파일을 두 키에, GeoLite Country+ASN, GeoLite City+ASN. 네 조합 모두 준비 전에 `ip_db_loaded` 두 개(`phase = startup`)가 남았다. `database_type`은 `DBIP-Country-Lite`, `DBIP-ASN-Lite (compat=GeoLite2-ASN)`, `ipinfo bundle_location_lite_sample.mmdb`, `GeoLite2-Country`·`GeoLite2-City`·`GeoLite2-ASN`이었다. `/api/v1/sources`의 두 DB는 `loaded`였고 `provider`는 차례로 `dbip`·`ipinfo`·`maxmind_geolite`였다. 루프백 신뢰 프록시를 거쳐 공인 주소 7개로 WS Viewer를 열고 닫았다. 모두 101이었고 주소마다 연결 시도 1·중단 1이었다. DB-IP는 7개 모두 국가·ASN·AS 이름을 찾았다. IPinfo 표본과 MaxMind 시험 파일은 파일에 든 범위의 주소만 찾았고 나머지는 `null`이었다. City 파일의 국가는 Country 파일과 같았다 | debug 실행파일의 자동 시험이며 release 묶음이나 수동 운영 확인이 아니다. 이 실행 뒤 DB 읽기·집계·조회 코드는 rustfmt 서식만 바뀌었다. 대시보드 자산은 그 뒤에도 바뀌었다. IPinfo Lite 전체 파일과 실제 GeoLite 파일은 토큰·계정이 필요해 확인하지 않았다. 실행 중 파일 교체, 대시보드 화면, Ubuntu는 이 기록의 범위 밖이다. 찾은 국가·ASN 값은 기록하지 않는다 |
| Windows 서비스 중지·실행파일 교체·서버 쪽 확인, 2026-11-21 | Windows 11 Pro 10.0.26200 x64, PowerShell 7.6.10. Release `v2026.11.9`. 서버 SHA-256 `7e77b73ad22ba59c7d7fd3f21803024be770aeb6981af25167aa90ee5e07dbe4`, 트레이 앱 `8d5a816ded12f174d6cdd97770ddbd21ab802e9126ad04396da1e3e6026eb024`. 이전 설치는 Release `v2026.11.8`(서버 SHA-256 `ac3ccea5eb233a71d4c89bbf07bdfcfb4ca5da9319addac4b05f92c8b17118c8`)이다 | `gh release download`로 받은 묶음의 해시를 대조하고 준비 디렉터리에 풀었다. 일반 PowerShell에서 같은 스크립트의 `-DryRun`으로 중지 전 조건만 먼저 확인했다. 그 뒤 관리자 PowerShell 스크립트 하나로 다음을 차례로 했다. 서비스 실행 경로(`PathName`)와 설치된 실행파일의 해시·버전 확인, 트레이 스레드에 `WM_QUIT`, 중지 직전 유휴 확인, 서비스 중지, 폴더 교체, 해시와 버전 대조, 시작, `Running`과 `/readyz` 200 확인, 서버 쪽 확인, 이전 폴더 삭제다. 트레이 앱은 0.11초 안에 끝났다. 유휴 확인에서 서비스 시작 이후 운영 로그의 대기 등록·세션 사건은 0건이었고 중계 포트 443·5500에 연결이 없었다. 중지 0.28초, 시작부터 준비 0.56초, 전체 2.77초, 로그의 중지부터 준비는 2.46초였다. `shutdown_started`는 `requested`·G 30·활성 0이었고 `server_stopped`가 남았다. `log_contract`는 `ops-info/2.3.0`이고, 시작 경고 네 건(`allow_all_source` 세 건, `ticket_tcp_bypass` 한 건)은 이전과 같았다. 서버 쪽 확인 18개가 통과했다. `/livez`·`/readyz` 200, 관리 `/` 303·`/login/` 200이었다. 루프백과 공유기 NAT 루프백에서 각각 WSS 두 이름이 TLS 1.3과 인증서 검증을 통과했다. ticket 없는 Viewer·Server는 401(`WWW-Authenticate: VncRelayTicket`), 허용 밖 Origin은 403, `/relay`는 404였다. TCP Server 연결도 서비스에 닿았다. `[metrics]`가 없는 운영 설정에서 상태 리스너의 `/metrics`는 404였다. 새 트레이 앱은 일반 사용자 세션(세션 1)에서 실행됐다 | 서비스 등록·운영 파일·방화벽 경로는 바꾸지 않았다. 복원은 일어나지 않았다. 관리 로그인이 필요한 확인(접속 출처 화면과 주소 차단)·트레이 메뉴·실제 기기와 Viewer 중계·롤백은 미실행이다 |
| 설정 편집 켜기와 IP 정보 DB 준비, 2026-11-21 | Windows 11 Pro 10.0.26200 x64, PowerShell 7.6.10. 서비스 `vnc-relay` Release `v2026.11.9`. DB-IP Lite 2026-11 국가·ASN 파일(압축 4,205,904·5,237,630바이트, 푼 파일 8,515,063·9,648,279바이트) | 3.11절로 데이터 폴더의 `ipdb`에 두 파일을 풀었다. 설정 파일과 데이터 폴더 ACL(19개 항목)을 먼저 보관했다. 관리자 PowerShell에서 3.3.1절 6단계의 `NT SERVICE\vnc-relay:(OI)(CI)M`을 데이터 폴더에 줬다. 이 배치는 2단계(비밀 파일 상속 차단)를 적용하지 않았으므로, 그 전에 `admin-accounts.toml`·`tls`·`keys`에 `/inheritance:d`를 적용해 기존 권한을 명시 항목으로 옮겼다. 결과는 데이터 폴더 자체와 그 안에서 상속받는 항목(`relay.toml`·`tray.lock`·`ipdb`·`logs`, 앞으로 만드는 임시 파일)이 서비스 수정(M), 계정 파일과 `tls`·`keys`의 파일은 서비스 읽기(RX)이고 다른 주체의 권한은 같았다. `[management]`에 `config_edit = true`, 끝에 `[ip_info]`를 더했다. 실행 중 서버는 `reload_restart_required`에 세 키(`management.config_edit`·`ip_info.country_db`·`ip_info.asn_db`)를 남겼다. 중지 직전 유휴 확인 뒤 재시작했다. 중지 0.07초, 전체 0.87초였다. `server_ready` 전에 `ip_db_loaded` 두 개(`DBIP-Country-Lite`, `DBIP-ASN-Lite (compat=GeoLite2-ASN)`, `build_time` 2026-11-01)가 남았다. 시작 경고 네 건은 이전과 같았다. 서버 쪽 확인 18개가 통과했다 | 대시보드 로그인이 필요한 확인(설정 보기의 편집 허용, 실제 저장과 파일 교체, 접속 출처의 "사용 중" 표시와 국가·ASN 값)은 미실행이다. 서비스 계정의 실제 쓰기는 ACL 확인으로만 판단했다. DB 월간 갱신은 자동화하지 않았다 |
| Windows 서비스 중지·실행파일 교체·서버 쪽 확인(2026.11.10), 2026-11-21 | Windows 11 Pro 10.0.26200 x64, PowerShell 7.6.10. Release `v2026.11.10`. 서버 SHA-256 `e81614b8d6d4a74c6e68d59ef0a25b1bc81d5572fcabe902f379f3a2e03eccc1`, 트레이 앱 `8d8896181c7ef4348fba811929d576c14c37894cd18069116556a5c45bd3918b`. 이전 설치는 Release `v2026.11.9`(서버 SHA-256 `7e77b73ad22ba59c7d7fd3f21803024be770aeb6981af25167aa90ee5e07dbe4`)이다 | 2026.11.9 교체와 같은 스크립트와 순서다. 받은 묶음의 해시를 대조하고 일반 PowerShell의 `-DryRun`으로 중지 전 조건을 먼저 확인했다. 첫 관리자 실행은 UAC 승인 창이 취소되어 시작되지 않았고, 서비스는 바뀌지 않았다. 다시 실행해 승인받았다. 트레이 앱은 0.10초 안에 끝났다. 유휴 확인에서 서비스 시작 이후 대기 등록·세션 사건은 0건이었고 중계 포트 443·5500에 연결이 없었다. 중지 0.27초, 시작부터 준비 0.71초, 전체 2.63초였다. `shutdown_started`는 `requested`·G 30·활성 0이었고 `server_stopped`가 남았다. `log_contract`는 `ops-info/2.3.0`이고 시작 경고 네 건(`allow_all_source` 세 건, `ticket_tcp_bypass` 한 건)은 이전과 같았다. `server_ready` 전에 `ip_db_loaded` 두 개(`DBIP-Country-Lite`, `DBIP-ASN-Lite (compat=GeoLite2-ASN)`)가 남았다. 서버 쪽 확인 18개가 통과한 뒤 이전 폴더를 지웠다. 관리 `/login/`은 새 제목 `VNC Relay 로그인`을 제공했다. 트레이 앱은 일반 사용자 PowerShell에서 다시 실행했고 세션 1에서 동작했다 | 서비스 등록·운영 파일·방화벽 경로는 바꾸지 않았다. 복원은 일어나지 않았다. 관리 로그인이 필요한 확인(설정 편집 저장, 접속 출처의 IP 정보 DB 표시)·트레이 메뉴·실제 기기와 Viewer 중계·롤백은 미실행이다 |

Windows 서비스 교체의 상태 확인에는 `Get-Service`를 쓴다. 2026-11-18 `v2026.11.6` 교체의 첫 시도는 `sc.exe query`에서 영문 `STATE`를 찾다가 서비스를 멈춘 직후 끝났다. 파일 교체 전이었고 일반 PowerShell의 `sc.exe start`로 이전 버전을 다시 시작했다. 중단은 19.3초였다. 한국어 Windows의 `sc.exe` 출력은 항목 이름을 현지화하므로 영문 문자열을 상태 판정 기준으로 쓰지 않는다.

실제 배치의 이전 설치 디렉터리와 복구 디렉터리, 이전 경로의 방화벽 규칙은 보관하지 않는다. 실행파일은 각 Release에서 다시 받을 수 있다. 운영 파일은 서비스 전환·교체에서 바꾸지 않았다. 접속 ticket을 지원하지 않는 실행파일로 복구할 때는 6절에 따라 `[relay.ticket.*]`를 지운 파일이 필요하다.

다음 수동 확인은 미완료다. 담당은 통합 검증 담당자이며 실제 배치가 필요한 확인은 운영 책임자와 함께 실행한다.

- 3.5.1절 설정 보기·편집·다섯 파일 상태·충돌·쓰기 실패·응답 불명 복구.
- 3.2.2절의 관리 재로그인·v4→v3 롤백·Ubuntu 이관.
- 3.2.1절의 같은 WSS 리스너에서 Viewer·Server 두 역할의 실제 Mode II 양방향 중계와 역할별 IP·Origin.
- 3.9절 Nginx TLS 종료·전달 주소·잘못된 체인 거절. 예시는 추정이다.
- 3.7.1절 최근 로그 상한·제거·재시작·0 보관·필터·JSON Lines 내려받기.
- 3.2.3절의 변경된 클라이언트 URL 실제 중계·롤백·Ubuntu 경로 이전.
- Windows 서비스 재부팅 뒤 자동(지연) 시작·해제·트레이 메뉴 동작·실제 기기와 Viewer 중계·서비스 교체 롤백.
- 3.11절의 IPinfo Lite 전체 파일·실제 GeoLite 파일, 실행 중 DB 파일 교체, release 실행파일과 Ubuntu에서의 IP 정보 DB 확인.

## 5. 중단 조건

- 설정·계정·TLS·리스너 준비 실패이면 운영을 시작하지 않는다. 진단 `error[<code>]`와 종료 코드를 확인한다.
- 재로드 실패이면 적용 성공으로 기록하지 않는다. 이전 유효 설정을 확인하고 후보를 고친다.
- 양방향 바이트가 다르면 배포 검증을 중단한다.
- LICENSE가 없으면 외부 배포 준비 완료로 표시하지 않는다.

종료 코드: 0 성공, 1 실행 중 오류, 2 사용법·비대화형 입력 오류, 3 설정·TLS·로그 파일 열기 오류, 4 리스너 준비 실패, 5 계정 파일 오류다. 자세한 코드는 [CLI 계약 2절](../contracts/cli-v1.md#2-오류)을 따른다.

## 6. 롤백 또는 복구

1. 재로드가 거절되면 설정과 인증서 경로를 이전 유효 쌍으로 되돌린다.
2. 로그에서 재로드 적용 결과를 확인한다.
3. 재시작이 필요하면 3.8절로 정상 종료한다.
4. 보호된 복구본을 복원한다.
5. 3.3~3.5절로 시작·중계·재로드를 다시 확인한다.

계정 변경 실패는 기존 파일을 유지한다. 강제 종료 뒤 `<계정 파일 이름>.lock`이 남았으면 실행 중인 `admin` 명령이 없는지 먼저 확인한다. 그때만 운영자가 잠금 파일을 지운다. 로그 다시 열기 실패는 경로·디렉터리·권한을 복구하고 다음 시도의 결과를 확인한다.

v2→v3 전환은 [설정 계약 3.2절](../contracts/config-v4.md#32-v2-파일을-v3로-옮기기)을 따른다. 기존 v2 파일은 수정이나 버전 키 추가 없이 v3 서버로 재시작한다. 다만 재시작용으로 미리 고쳐 둔 19개 키도 v3에서는 동적 적용될 수 있다. 미리 적용하면 안 되는 변경은 별도 사본에 두고 재시작 때 파일을 교체한다.

v2 서버(릴리스 v2026.11.0)로 롤백할 때는 정상 종료 뒤 `management.config_edit` 줄을 지운 파일과 이전 실행파일을 함께 복원한다. `false`로 바꾸는 것만으로는 충분하지 않다. 키가 남으면 이전 서버는 `config_unknown_key`로 시작하지 않는다. 이 키를 쓰지 않은 v3 파일은 v2가 그대로 읽는다. 롤백 뒤 시작·중계·관리 재로그인을 확인한다. 이전 서버의 더 좁은 재로드 구분도 다시 적용된다.

v4→v3(릴리스 `v2026.11.1`) 롤백은 정상 종료 뒤 이전 실행파일과 보호된 v3 파일을 함께 복원하는 방법을 권장한다. 새 WS·WSS Server 역할·전달 주소·최근 로그 기능은 잃는다. 관리 세션도 재시작으로 무효다. v4 파일을 되돌려 쓸 때는 WS·WSS Server 역할 테이블, `trusted_proxies`, `[recent_logs]`를 제거한 뒤 [설정 계약 3.3절](../contracts/config-v4.md#33-v3-파일을-v4로-옮기기)의 표를 역으로 적용해야 한다. 빈 테이블도 남기지 않는다. 공통 리스너 키는 기존 Viewer 테이블로 합친다. 이관 전 복구본이 있으면 그 파일을 쓰는 편이 안전하다. 릴리스 `2026.11.2`보다 새 실행파일에서 되돌리면 Viewer 접속 URL도 `/relay`로 함께 되돌린다(3.2.3절).

프록시 TLS 종료를 쓰던 배치는 v3의 전달 주소 검사가 없어진다. 롤백 전에 직접 내장 WSS와 실제 소켓 IP 정책으로 되돌리거나 프록시를 통한 접근을 중지해야 한다. 프록시 주소를 전체 클라이언트의 주소처럼 허용해 우회하지 않는다. 시작·준비·실제 Viewer 중계·관리 재로그인과 로그 소비자 표기를 다시 확인한다. 이전 서버의 재로드 구분을 다시 적용한다. 이 롤백 절차는 미실행이다.

접속 ticket을 켠 뒤 문제가 생기면 그 역할의 `enabled = false`로 되돌린다. 재시작은 필요 없다. 접속 ticket을 지원하지 않는 이전 실행파일로 되돌릴 때는 `[relay.ticket.*]` 표를 지운 설정 파일과 함께 복원한다. 표가 남으면 이전 서버는 `config_unknown_key`로 시작하지 않는다.

IP 정보 DB 파일에 문제가 있으면 이전 정상 파일을 같은 경로에 다시 둔다. 재시작은 필요 없다. 국가·ASN을 쓰지 않으려면 정상 종료한 뒤 `[ip_info]` 표를 지우고 다시 시작한다. 접속 출처를 지원하지 않는 이전 실행파일(릴리스 `2026.11.8` 이하)로 되돌릴 때는 `[ip_info]` 표를 지운 설정 파일과 함께 복원한다. 표가 남으면 이전 서버는 `config_unknown_key`로 시작하지 않는다. 내려받은 DB 파일은 남겨 두어도 이전 서버가 읽지 않는다.

로그 화면·상단 메뉴와 ko/en 문구(CPY-149~189)의 기준은 [운영 스펙](../specs/operations.md)을 따른다.

## 7. 해결되지 않을 때 전달할 대상

- 전달 대상: 이 배포의 운영 책임자와 프로젝트 관리자다.
- 보존할 증거: OS·실행파일 버전·해시, 실패 명령, 종료 코드, 비밀 없는 진단 코드, 해당 사건 전후 로그다.
- 비밀 값 제외 규칙: 비밀번호·해시·개인키·계정 파일 본문을 보내지 않는다. 원시 VNC 데이터와 사용자 입력을 수집하지 않는다. 접속 주소·관리자 계정명이 필요하지 않으면 가린다.
