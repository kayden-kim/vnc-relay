# 실행파일을 묶고 수동 운영하는 방법

Windows 11 x64와 Ubuntu 24.04 LTS x64에서 release 실행파일을 만들고 수동으로 운영한다. 계정을 준비한 뒤 시작·양방향 중계·자동 재로드·정상 종료를 확인한다. OS 서비스 등록과 컨테이너는 이 절차의 대상이 아니다(사용자 결정 2026-11-04(Q105)).

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 배포 담당자와 서버를 직접 실행하는 운영자 |
| 검증 상태 | Draft — 과거 두 OS 수동 기록은 4절에 보존한다. v4 이관(3.2.2절)은 2026-11-09 Windows 실제 배치에서 관리 재로그인을 빼고 실행했다(4절). v4 롤백·Ubuntu 이관과 공유 WSS 역할·프록시 TLS 종료·최근 로그의 새 절차는 미실행이다. 이 기능들의 구현과 자동 시험은 커밋 `51bd747`의 두 OS 게이트에서 통과했다([WSS Server·프록시·최근 로그 판정표](set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)). 자동 시험만으로 안내 전체를 검증했다고 표시하지 않는다. |
| 검증 환경·버전 | Windows 11 x64의 PowerShell, Ubuntu 24.04 LTS x64의 bash. Rust 버전은 `rust-toolchain.toml`을 따른다. 실제 OS·Rust·실행파일 버전은 실행 기록에 적는다. |
| 최종 검증일 | 없음(Draft 2026-11-04) |
| 관련 기준 문서 | [운영 스펙](../specs/operations.md), [설정 계약 v4](../contracts/config-v4.md), [CLI 계약](../contracts/cli-v1.md), [운영 정보 계약 v2](../contracts/ops-info-v2.md), [WebSocket 계약](../contracts/websocket-relay-v2.md), [ADR-0001](../decisions/ADR-0001-built-in-tls.md) |

## 1. 대상과 영향

- 대상: 실행파일 하나, 설정 파일, 계정 파일, 운영자가 제공한 인증서·개인키, 운영 로그다.
- 예상 사용자 영향: 시작은 리스너를 연다. 재로드는 허용 정책·한도·시간 제한·keepalive·TLS의 이후 소비 시점에 영향을 준다. 이미 허용한 연결과 시작한 타이머는 유지한다. 종료·재시작은 연결과 관리 세션을 끝낸다.
- 작업 허용 시간: 재시작이 필요한 변경은 운영자가 승인한 중단 시간에 한다.

기본 검증 예시는 루프백 TCP의 Mode II만 켠다. 관리 HTTP와 상태 리스너도 루프백이다. 이 구성 선택은 **이 문서의 판단, 사용자 결정 없음**이다. 공개 주소에 그대로 옮기는 예시가 아니다. WSS와 프록시 배치는 3.2.1절·3.9절의 별도 v4 예시를 쓴다.

인터넷에 공개하는 WebSocket 진입점에는 내장 WSS 또는 역방향 프록시 TLS 종료를 권장한다(사용자 지시 2026-11-08). 평문 WS는 보호된 네트워크에서만 켜기를 권장한다. 프록시→VNC Relay 구간의 보호와 공개 배치의 책임은 운영자에게 있다. 관리 리스너를 외부에 열 때는 내장 HTTPS가 필수다. 관리 리스너는 프록시 TLS 종료 지원의 대상이 아니며 전달 헤더를 무시한다. 관리 쿠키와 로그인 제한 출발지 주소 규칙도 바꾸지 않는다(사용자 결정 2026-11-08(Q182·Q183)).

## 2. 전제 조건

- [ ] 대상이 Windows 11 x64 또는 Ubuntu 24.04 LTS x64다. Windows Server와 다른 OS·아키텍처는 검증 범위가 아니다.
- [ ] 빌드 담당자는 [개발 환경 준비](set-up-and-verify.md)에 따라 Rust와 C 빌드 도구를 준비했다.
- [ ] 운영자는 사용할 포트가 비어 있고 방화벽·출발지 IP 정책이 의도한 범위인지 확인했다.
- [ ] 계정 파일·개인키·로그 디렉터리의 접근 권한을 제한했다.
- [ ] TLS를 켜면 운영자가 발급받은 유효한 인증서 체인과 일치하는 개인키를 준비했다. 서버는 인증서를 자동 발급하지 않는다.
- [ ] 수동 중계 확인에는 Python 3가 있다. 아래 Python은 시험용 클라이언트이며 배포 실행파일의 필수 의존성이 아니다.
- [ ] 기존 설정과 인증서 쌍의 복구본을 안전한 위치에 보관했다.

## 3. 절차

- 실행 환경: Windows PowerShell 또는 Ubuntu bash다.
- 실행 위치: 3.1절은 저장소 루트다. 3.2절부터는 묶음을 푼 디렉터리다.
- 바꿔 넣을 값: `opsadmin`은 운영 계정명이다. 예시 포트를 바꾸면 확인 명령과 Python의 포트도 함께 바꾼다. 비밀번호는 명령줄에 넣지 않는다.
- 최대 대기 시간: 빌드 한도는 10분으로 둔다(이 문서의 판단, 사용자 결정 없음). 넘으면 Ctrl+C로 중단하고 빌드 오류를 확인한다. 시작은 `server_ready`를 확인할 때까지 기다린다. 재로드는 Dp + D 안에 결과를 확인한다. 정상 종료는 G와 정리 시간을 기다린다. Python 소켓 한도 10초는 이 문서의 판단, 사용자 결정 없음이다.

### 3.1 release 빌드와 묶음

각 OS에서 그 OS용 묶음을 만든다. 다른 OS의 실행파일을 복사해 쓰지 않는다. 묶음은 `cargo xtask package`로 만든다(사용자 결정 2026-11-08(Q161)). 태그로 낸 묶음은 [GitHub Release](https://github.com/kayden-kim/vnc-relay/releases)에서 내려받을 수 있다. 버전 규칙과 Release 절차는 [릴리스 how-to](release.md)에 있다.

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
   | `examples/relay.toml` | `relay.toml` |
   | `README.md` | `README.md`(사용자 결정 2026-11-08(Q164)) |
   | 저장소의 `docs/` 디렉터리와 `GLOSSARY.md` | 같은 상대 위치. 이 안내 `docs/how-to/operate-relay.md`와 기준 문서 링크를 오프라인에서도 읽는다. |
   | `THIRD-PARTY-NOTICES.txt` | `THIRD-PARTY-NOTICES.txt` 전체 |
   | `LICENSE` | 루트 `LICENSE` 전체(MIT) |

   루트 [`LICENSE`](../../LICENSE)는 MIT 전문과 `Copyright (c) 2026 vnc-relay contributors`를 담는다(사용자 결정 2026-11-07(Q97)). 배포 묶음에 이 파일을 반드시 넣는다. `THIRD-PARTY-NOTICES.txt`는 프로젝트 자체 MIT LICENSE를 대신하지 않는다. 검토 결과는 [라이선스 검사 4.2절](check-dependency-licenses.md#42-프로젝트-license-검토-기록2026-11-07)에 있다.

   개인키·계정 파일·실제 비밀번호·운영 로그는 배포 묶음에 넣지 않는다. `cargo xtask package`는 위 표의 파일만 복사한다.

3. 묶음을 운영 디렉터리에 풀고, 함께 받은 `.sha256` 파일로 해시를 확인한다. Ubuntu는 `sha256sum -c vnc-relay-<버전>-linux-x64.tar.gz.sha256`, Windows는 `Get-FileHash <묶음 파일>`의 값과 `.sha256` 파일의 첫 낱말을 비교한다.

   기대 결과: 해시가 같다. 실행파일과 `relay.toml`이 같은 디렉터리에 있다. 이후 명령은 이 디렉터리에서 실행한다.

### 3.2 설정과 계정 준비

상대 경로는 현재 터미널 위치가 아니라 `relay.toml`의 디렉터리를 기준으로 푼다. 설정에는 비밀번호·해시·개인키 본문을 넣지 않는다(사용자 결정 2026-11-04(Q106)).

1. 운영 디렉터리의 접근 권한을 제한한다.

   Ubuntu에서는 서버를 실행할 OS 계정을 소유자로 두고 다른 계정이 디렉터리를 쓰지 못하게 하기를 권장한다. 계정 파일·개인키는 `0600`을 권장한다. CLI는 새 계정 파일을 `0600`으로 만든다.

   Windows에서는 폴더 속성의 보안 설정에서 서버 실행 OS 계정과 관리자만 읽고 쓰도록 ACL(접근 제어 목록)을 제한하기를 권장한다. CLI는 ACL을 바꾸거나 경고하지 않는다. 새 파일은 디렉터리 ACL을 물려받으므로 운영자가 실제 권한을 확인해야 한다([CLI 계약 1.6절](../contracts/cli-v1.md#16-저장과-파일-권한)).

2. `relay.toml`을 대상 배치에 맞게 편집한다.

   로컬 검증에서는 예시 그대로 쓴다. 공개 배치에서는 리스너 주소와 역할별 출발지 `allow`를 의도한 값으로 정한다. `[]`는 모두 거절이며 켠 역할의 목록 생략은 시작 실패다. `0.0.0.0/0`·`::/0`는 전체 허용과 경고를 뜻한다. Mode I을 켜면 `mode1.allow_targets`도 명시해야 한다. WSS의 공통 `listen`·`cert_file`·`key_file`은 `[relay.wss]`에 둔다. `enabled`·`allow`·`allowed_origins`는 `[relay.wss.viewer]`와 `[relay.wss.server]`에 둔다. 브라우저 Origin은 IP 제한을 대신하지 않는다.

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

두 역할은 같은 WSS 소켓을 쓴다. Viewer는 `wss://HOST:8443/relay/viewer`, Server는 `wss://HOST:8443/relay/server`로 접속한다. `HOST`는 인증서에 맞는 실제 호스트명이다. 경로 뒤 `/`는 붙이지 않는다. 릴리스 `2026.11.2` 이하의 Viewer 경로는 `/relay`다(3.2.3절). Server 역할은 Mode II만 받으며 TCP Server와 같은 `ID:` 초기 교환을 쓴다. Server에는 greeting을 보내지 않는다(사용자 결정 2026-11-08(Q181·Q188)). 위 Server의 빈 Origin 목록은 `Origin` 없는 클라이언트만 허용한다.

#### 3.2.2 v3 파일을 v4로 옮기기

v4 서버는 v3의 WS·WSS 테이블을 읽지 않는다. 호환 지원 기간은 없다. 이관 전 파일을 넣으면 `config_unknown_key`로 시작 실패한다. 이 시작 실패는 OPS-066 자동 시험이 두 OS에서 확인했다. 실제 운영 `relay.toml`도 배포 전에 이관해야 한다(사용자 결정 2026-11-08(Q188)). 아래 절차는 2026-11-09 Windows 실제 배치에서 실행했다([4절 기록](#4-검증)). 8단계의 관리 재로그인과 Ubuntu 이관은 미실행이다.

1. 현재 실행파일·`relay.toml`·계정 파일·TLS 파일 쌍을 접근이 제한된 복구 위치에 복사한다. 실행파일 버전·해시와 파일 경로도 기록한다.
2. 운영 파일과 같은 디렉터리에 `relay-v4.toml` 사본을 만든다. 실행 중인 구버전의 파일은 아직 바꾸지 않는다.
3. [설정 계약 3.3절의 매핑](../contracts/config-v4.md#33-v3-파일을-v4로-옮기기)으로 사본을 편집한다.

   | v3 위치 | v4 위치 |
   | --- | --- |
   | `[relay.ws_viewer]`의 `enabled`·`allow`·`allowed_origins` | `[relay.ws.viewer]`의 같은 키 |
   | `[relay.ws_viewer]`의 `listen` | `[relay.ws]`의 `listen` |
   | `[relay.wss_viewer]`의 `enabled`·`allow`·`allowed_origins` | `[relay.wss.viewer]`의 같은 키 |
   | `[relay.wss_viewer]`의 `listen`·`cert_file`·`key_file` | `[relay.wss]`의 같은 키 |

   이전 테이블은 남기지 않는다. TOML 파일에 버전 키를 추가하지 않는다. 기존 Viewer 동작만 유지하려면 Server 역할·`trusted_proxies`·`[recent_logs]`를 추가할 필요는 없다. 새 역할은 기본 비활성이다. 새 실행파일이 릴리스 `2026.11.2`보다 새 버전이면 Viewer 경로도 바뀐다. 이때는 3.2.3절을 함께 따른다.

4. 이관한 파일의 별도 검증 사본 `relay-v4-check.toml`을 같은 디렉터리에 만든다. 켜진 모든 중계 리스너와 관리·상태 리스너의 `listen`을 서로 다른 미사용 루프백 포트로 바꾸고 `log.output = "stdout"`으로 바꾼다. 운영 파일과 계정·TLS 파일은 수정하지 않는다.
5. 운영 파일과 같은 디렉터리에서 새 실행파일의 실제 경로로 검증 사본을 시작한다. Windows는 `& 'C:\배포\v4\vnc-relay.exe' serve --config .\relay-v4-check.toml`, Ubuntu는 `/opt/vnc-relay-v4/vnc-relay serve --config ./relay-v4-check.toml`이다. 실행파일 경로는 새 묶음의 실제 경로로 바꾼다. 구버전 실행파일을 덮어쓰지 않는다.

   별도의 검증 전용 CLI는 없다. 이 시작으로 전체 설정·계정·TLS 검증과 대체 포트 바인드를 확인한다. `server_ready`와 검증 사본의 상태 포트 `/readyz` 200을 확인한 뒤 Ctrl+C로 정상 종료한다. 오류가 나면 운영 서버를 멈추지 말고 사본을 고친다. 이 절차는 운영 포트·방화벽·실제 중계의 검증을 대신하지 않는다.

6. 중단 시간에 3.8절로 구버전을 정상 종료한다. 이 단계는 모든 연결과 관리 세션을 끝낸다. 복구본으로 이전 실행파일과 파일을 함께 복원할 수 있는지 먼저 확인한다.
7. 새 실행파일과 검증된 `relay-v4.toml`을 운영 위치에 설치한다. 운영용 `listen`·로그 경로를 유지한 파일을 `relay.toml`로 쓴다. 대체 포트의 검증 사본을 설치하지 않는다.
8. 3.3절로 시작·상태·관리 재로그인을 확인한다. 기존 Viewer의 실제 중계와 3.5절 재로드도 확인한다. Viewer 경로는 새 실행파일의 경로를 쓴다(3.2.3절). 새 Server 역할은 `/relay/server`의 Mode II 양방향 중계를 따로 확인한다.

로그 소비자도 같은 배포에서 바꾼다. `server_starting.log_contract`는 `ops-info/2.0.0`이다. 예전 `listener=ws_viewer`는 `listener=ws`와 `role=viewer`, `wss_viewer`는 `wss`와 `role=viewer`로 대응한다. `ws`·`wss`의 `connection_accepted`에는 아직 `role`이 없다. 경로로 역할을 정한 뒤에만 `role`이 있다. 리스너 값은 `tcp_viewer`·`tcp_server`·`ws`·`wss`·`management`·`status`다. 자세한 범위는 [운영 정보 계약 v2](../contracts/ops-info-v2.md)를 따른다.

롤백은 6절의 v4→v3 복구 절차를 따른다. 검증 사본은 증거를 보관한 뒤 지운다. 계정·TLS 비밀을 일반 검증 로그에 복사하지 않는다.

#### 3.2.3 Viewer 경로를 `/relay/viewer`로 옮기기

릴리스 `2026.11.2`보다 새 실행파일은 `websocket-relay` 2.0.0을 구현한다. 이 실행파일은 WS·WSS Viewer 경로로 `/relay/viewer`만 받는다(사용자 결정 2026-11-10). 이전 경로 `/relay`는 404로 거절한다. 두 경로를 함께 받는 기간은 없다([WebSocket 계약 3.1절](../contracts/websocket-relay-v2.md#31-1x-클라이언트를-200으로-옮기기)). 설정 파일은 바꾸지 않는다. TCP 진입점과 Server 경로 `/relay/server`는 그대로다.

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

재로드 대상은 [설정 계약 1.8절](../contracts/config-v4.md#18-재로드-구분)의 43개 키다. 이 중 TLS 네 경로와 접속 ticket 공개 키 파일 경로 두 개를 뺀 37개가 편집 대상이다(사용자 결정 2026-11-08(Q184·Q188), 사용자 결정 2026-11-11(Q200)). 역할별 출발지 IP·Origin, 리스너별 `trusted_proxies`, 켠 Mode I의 대상 허용 정책, L·Lm·Lw·B·M, H·U·W·C·E·G, TCP/WS keepalive, TLS 경로·내용, 접속 ticket 설정과 공개 키 파일 내용을 포함한다(3.10절). `trusted_proxies`는 교체 뒤 다음 수락·업그레이드 검사부터 적용한다. 이미 통과한 검사 결과, 기존 연결과 시작한 타이머는 유지한다. 낮춘 상한을 이미 넘었으면 기존 점유를 유지하고 새 점유만 거절한다. 세부 적용 시점은 계약 표를 따른다.

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

**편집 기능을 켠 동안 외부 편집과 대시보드 편집을 동시에 하지 않는다.** 다른 관리자와의 저장도 순서를 정한다. 주석·공백만 외부에서 바뀌어도 원시 바이트 revision이 달라 충돌한다. 서버는 임시 파일 쓰기·flush 뒤 교체 직전에 원본 revision을 다시 확인한다. 다르거나 다시 읽지 못하면 임시 파일을 지우고 `409 config_conflict`로 거절한다. 마지막 확인과 원자 교체 사이의 아주 짧은 틈은 보장 범위 밖이다(사용자 결정 2026-11-08(Q178)).

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

   새 파일을 먼저 준비한 뒤 경로 쌍을 한 번에 바꾸는 절차는 **이 문서의 판단, 사용자 결정 없음**이다. TLS 경로는 대시보드/API에서 보기 전용이므로 경로 변경은 외부 편집기로 한다. 이때 대시보드 저장을 동시에 하지 않는다. 같은 경로의 파일 내용을 교체하는 방식도 자동 감지한다. 그 경우 설정 파일 revision은 그대로여도 파일 상태가 바뀔 수 있다. 두 내용이 유효한 쌍이 될 때까지 한쪽만 바뀐 후보는 거절하고 기존 TLS를 유지한다.

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

   이름 변경·삭제·생성 알림을 받으면 서버가 원래 경로를 곧바로 다시 연다. 알림을 놓쳐도 Dp마다 항상 다시 연다. 파일이 없으면 만든다(사용자 결정 2026-11-04(Q104·Q109)). 회전 직후 다시 열기 전 사건은 옛 파일에 남을 수 있다.

3. `relay.log`에 새 사건이 이어지는지 확인한다.

   다시 열기 실패 시 옛 파일에 계속 쓴다. 실패가 시작될 때 한 번 stderr 경고와 warn `log_reopen_failed`가 남는다. 경로·권한을 복구하면 다음 알림·Dp 주기에 다시 시도한다(사용자 결정 2026-11-04(Q110)). 운영자는 옛 파일도 확인해야 한다. 이 방식은 파일 이름 변경을 기준으로 한다. 복사 후 원본 자르기(copytruncate)를 이 절차의 회전 방식으로 쓰지 않는다.

로그 파일은 열리지만 로그 디렉터리의 알림 등록만 실패하면 서버는 시작을 계속한다. `reload_watch_failed`(`reason=notify_error`)를 기록하고 Dp마다 다시 열기와 알림 등록을 재시도한다(사용자 결정 2026-11-04(Q111)). 설정·TLS 감시 준비 실패에는 이 예외를 적용하지 않는다.

로그 회전 시점·보관 기간·접근 권한은 운영자 책임이다. 다음은 제품 강제 정책이 아닌 **권장** 사항이다.

- 장애 분석·감사 요구와 디스크 용량을 근거로 보관 기간을 정하기를 권장한다.
- 서버 실행 OS 계정만 로그를 쓰고 승인된 운영자만 읽도록 제한하기를 권장한다.
- 로그 디렉터리를 다른 일반 OS 계정이 바꾸지 못하게 하기를 권장한다. Ubuntu에서 서버가 새로 만드는 로그 파일의 권한은 umask를 따른다. 2026-11-05 수동 실행(umask `022`)에서는 `0644`였다. 디렉터리 권한(예: `0700`)으로 다른 계정의 읽기를 막기를 권장한다.
- 보관 파일의 접근 권한도 원본과 같이 제한하고 필요하면 보호된 저장소로 옮기기를 권장한다.
- 디스크 여유와 stderr 경고를 감시하기를 권장한다. 로그에는 관리자 계정명과 접속 관련 정보가 포함될 수 있다.

#### 3.7.1 대시보드 최근 로그

`[recent_logs] max_entries = 1000`은 메모리에 최근 사건 1000개를 보관하는 기본값이다. 범위는 0~10000이며 `0`이면 보관하지 않는다. 값을 바꾸면 재시작해야 한다. 재시작하면 목록은 빈다(사용자 결정 2026-11-08(Q185)). 보관 수를 늘릴 때는 최대 로그 줄 크기에 비례해 메모리를 더 쓴다는 점을 반영한다.

관리자로 로그인한 뒤 `/#/logs`를 연다. 수준·사건명 필터와 값 검색으로 좁힐 수 있다. 자동 갱신은 P 주기이며 일시정지할 수 있다. 세션 상세의 "로그 보기"로 해당 세션에 관련된 사건을 열 수 있다. 내려받기는 현재 필터 결과를 JSON Lines로 저장한다(사용자 결정 2026-11-08(Q186·Q187)).

이 목록은 `log.output`과 출력 쓰기 성공 여부에 관계없이 채운다. 파일 출력 실패여도 화면에 사건이 보일 수 있으므로 파일 기록 성공의 증거로 삼지 않는다. 목록은 오래된 항목부터 버리며 영속 보관·감사 로그를 대신하지 않는다. API 매개변수·응답 필드 이름과 세션 필터 URL은 사용자 결정 2026-11-08(Q191)으로, 화면 시안은 사용자 결정 2026-11-08(Q189)으로, 문구는 사용자 결정 2026-11-08(Q190)으로 채택했다. 기준은 [운영 스펙의 채택 기록](../specs/operations.md#8-미결정-사항)을 따른다.

### 3.8 정상 종료와 재시작

1. 서버를 실행한 터미널에서 Ctrl+C를 한 번 누른다.

   Ubuntu는 SIGINT(Ctrl+C)와 `kill -TERM <PID>`를 정상 종료 요청으로 받는다. Windows는 Ctrl+C와 Ctrl+Break를 받으며 SIGTERM은 없다. 신규 중계를 중지하고 미완성 작업을 정리한다. 활성 세션에는 종료 시작 때 한 번 읽은 `timeouts.shutdown_grace_secs`(G)의 유예를 준다. 기본값은 30초다. G가 0이면 바로 정리한다. 유예 중 관리 로그인·조회·세션 종료와 설정 GET/PATCH는 계속 가능하다. watcher의 새 적용은 멈추지만 PATCH commit은 허용된다. 이때 G를 편집해도 이미 시작한 유예 기한은 늘거나 줄지 않으며 신규 중계 수락도 재개하지 않는다.

2. 서버의 종료 결과를 확인한다.

   Windows: 서버 명령이 돌아온 같은 PowerShell에서 `$LASTEXITCODE`를 읽는다.

   Ubuntu: 서버 명령이 돌아온 같은 bash에서 `echo $?`를 실행한다.

   기대 결과: `0`이고 로그에 `server_stopped`가 있다. G 만료로 남은 세션을 정리한 경우도 정상 종료 코드 0이다.

유예 중 두 번째 종료 요청은 남은 유예를 건너뛰고 즉시 강제 정리한다. Windows 콘솔 창 닫기도 처음부터 이 방식으로 처리한다. G를 기다리지 않으며 OS의 기본 5초 제한 안에 정리와 종료 기록을 마쳐야 한다(사용자 결정 2026-11-04(Q102)). 일반적인 운영 종료에는 Ctrl+C를 권장한다.

Windows 로그오프·시스템 종료는 강제 종료다. 대화형 콘솔 프로그램의 정상 유예·종료 기록을 보장하지 않는다. SIGKILL·작업 관리자 종료·전원 장애도 같다. 다시 시작하면 이전 소켓·대기·세션·관리 로그인은 복원되지 않는다.

### 3.9 역방향 프록시로 TLS 종료

중계 WS·WSS 리스너만 대상이다. 이 예시는 Nginx가 외부 WSS를 받고 같은 호스트의 평문 `ws` 리스너로 전달하는 구성이다. Nginx 설정과 연결 절차는 이 저장소에서 검증하지 않았다. **추정 / 미실행**이다. 같은 연결 구조는 WS-035 자동 시험이 확인했다. 이 시험은 시험 코드 안의 TLS 종료 프록시가 평문 WS로 `X-Forwarded-For`를 덧붙여 보내는 구성이며, Nginx를 실행하지 않는다([WSS Server·프록시·최근 로그 판정표](set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)). 실제 Nginx 버전·경로·인증서·방화벽에 맞게 바꾸고 4절의 별도 검증을 해야 한다.

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

`trusted_proxies`에는 프록시 주소만 넣어야 한다. 클라이언트 대역이나 `0.0.0.0/0`·`::/0` 같은 전체 범위를 넣어서는 안 된다. 그렇지 않으면 비신뢰 상대가 주소를 위조하거나 정상 클라이언트 체인이 거절될 수 있다. 전체 신뢰 경고 이름 `trust_all_proxies`과 필드 `listener`·`item`·`phase`는 사용자 결정 2026-11-08(Q191)으로 채택했다. 시작·재로드·설정 편집 때의 경고는 OPS-067 자동 시험이 두 OS에서 확인했다. 경고는 설정 실수를 알리는 보조 수단이므로 경고에 의존해 안전성을 판단하지 않는다.

신뢰 프록시에서 온 `X-Forwarded-For`는 오른쪽부터 검사한다. 처음 만나는 `trusted_proxies` 밖 주소를 클라이언트 주소로 쓰며 그 왼쪽의 위조 요소는 읽지 않는다. Nginx의 `$proxy_add_x_forwarded_for`는 실제 접속 주소를 오른쪽에 덧붙인다. 여러 프록시를 거치면 각 프록시가 같은 방식으로 실제 상대 주소를 전달하고 모든 중간 프록시 주소만 신뢰 목록에 넣어야 한다. 신뢰하지 않은 소켓 상대의 전달 헤더는 무시한다([WebSocket 계약 1.8절](../contracts/websocket-relay-v2.md#18-신뢰-프록시와-x-forwarded-for)).

헤더 누락·빈 값·검사 중 잘못된 요소·모든 요소가 신뢰 프록시 주소인 체인은 `HTTP 400`과 `forwarded_for_invalid`로 거절한다. 이때 등록과 `forwarded_ip`는 없다. 프록시 주소가 역할 허용 목록에 있어도 모두 신뢰하는 체인은 거절한다([WS-033·WS-036](../specs/websocket.md#7-수용-테스트)). 따라서 이 예시에서 Nginx 호스트 자체의 루프백 요청은 프록시를 거치면 거절된다. `127.0.0.1:8080`에 전달 헤더 없이 직접 접속해도 거절된다. 출발지 `127.0.0.1`이 `trusted_proxies`에 있어 신뢰 프록시 연결로 취급되기 때문이다. 이 출발지의 직접 접속은 유효한 전달 헤더가 있어야 하며, 같은 전달 주소 판정과 역할 `allow` 검사를 받는다. 전달 헤더 없이 직접 접속하려면 출발지가 그 리스너의 `trusted_proxies` 밖에 있고 해당 역할의 `allow`에 들어야 한다. 이 예시의 호스트에서 전달 헤더 없이 직접 시험해야 하면 신뢰 프록시를 두지 않은 다른 리스너(예: TCP 진입점이나 `trusted_proxies`가 빈 WSS 리스너)를 쓴다.

**프록시→VNC Relay 구간은 운영자가 보호해야 한다.** 같은 호스트의 루프백 또는 접근을 제한한 사설망을 권장한다. 사설망 자체가 암호화를 보장하는 것은 아니다. 도청·변조 위험이 있으면 그 구간에도 TLS 등 보호를 적용한다. 프록시를 통하지 않는 평문 리스너의 외부 접근은 방화벽으로 막아야 한다. 관리 경로 `/`·`/api/`는 이 예시에 추가하지 않는다. 관리 리스너는 내장 HTTPS 또는 루프백 HTTP와 기존 로그인 규칙을 그대로 쓴다(사용자 결정 2026-11-08(Q183)).

### 3.10 접속 ticket 켜기와 공개 키 교체

WS·WSS 역할마다 접속 ticket 확인을 켤 수 있다. 접속 ticket(ticket)은 발급 쪽이 ES256으로 서명한 짧은 수명의 JWS다. 켠 역할의 WS·WSS 업그레이드는 유효한 ticket이 있을 때만 101로 답한다. 기본값은 꺼짐이다. 형식과 확인 순서는 [WebSocket 계약 1.9절](../contracts/websocket-relay-v2.md#19-접속-ticket)을, 설정 키는 [설정 계약 1.2절](../contracts/config-v4.md#12-키-표)과 [1.11절](../contracts/config-v4.md#111-접속-ticket-공개-키-파일-형식)을 따른다. 이 절차는 **추정 / 미실행**이다. 자동 증거는 4절 표에 있다.

먼저 다음 조건을 확인한다.

- `[relay.ticket.*]` 표는 설정 계약 4.1.0이다. 이 표를 쓴 파일은 이전 실행파일이 `config_unknown_key`로 거절한다. 실행파일을 먼저 바꾼 뒤 표를 더한다.
- ticket은 TCP 진입점에 적용하지 않는다. 같은 역할의 TCP 진입점이 켜져 있으면 `ticket_tcp_bypass` 경고가 남는다. ticket으로만 접속을 허용하려면 그 역할의 TCP 진입점을 끈다.
- 평문 `[relay.ws]`에서 그 역할을 켜면 `ticket_plaintext_ws` 경고가 남는다. ticket 원문이 암호화되지 않은 구간을 지나기 때문이다. WSS 또는 보호된 구간을 쓴다.
- 서버와 발급 쪽의 시계를 맞춘다. 허용 오차는 5초로 고정이다.

1. 발급 쪽에서 역할마다 다른 P-256 서명 키 쌍을 만든다. vnc-relay에는 공개 키만 둔다.

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

## 4. 검증

각 OS에서 기본 TCP 배치의 3.1~3.8절을 실행한다. TLS 배치가 아니면 3.6절은 해당 없음으로 기록한다. v3→v4 이관은 3.2.2절, Viewer 경로 이전은 3.2.3절, WSS 두 역할은 3.2.1절, 프록시 TLS 종료는 3.9절, 접속 ticket은 3.10절을 각각 추가 검증한다. OPS-031의 일반 배포 확인에 없는 TLS 동작을 확인한 것처럼 표시하지 않는다. 인증서 교체 자체의 자동 증거는 OPS-024가 담당한다.

| 관찰 대상 | 성공 조건 | 확인 명령·방법 | 실제 결과 | 실행 증거 위치 |
| --- | --- | --- | --- | --- |
| 각 OS release 묶음 | release 빌드·고지 검사 성공, 비밀 없는 예시·안내 포함. LICENSE 미완료 별도 표기 | 3.1절, 실행파일 SHA-256·버전·빌드 명령 기록 | Windows·Ubuntu 통과(2026-11-05, 커밋 `1c7c7a9`). LICENSE 없음(Q97 대기) | [CLI 계약 실행 기록](../contracts/cli-v1.md#계약-테스트-실행-기록) |
| 계정 준비 | 숨김 입력 성공, 이름만 출력 | 3.2절. 비밀번호·계정 파일 본문은 기록하지 않음 | Windows·Ubuntu 통과 | 같음 |
| 시작·상태 | `server_ready`, `/livez`·`/readyz` 200 | 3.3절 | Windows·Ubuntu 통과. 브라우저 로그인 뒤 운영 개요의 서버 상태 `Ready` | 같음 |
| 중계 | 양방향 시험 바이트 일치 | 3.4절 | Windows·Ubuntu 통과 | 같음 |
| IP 재로드 | `reload_applied`, 새 중계 성공 | 3.5절 | Windows·Ubuntu 통과. 저장 뒤 0.6초 안에 `reload_applied` | 같음 |
| 인증서 교체 | 새 전체 TLS 연결에 새 인증서, 이전 연결 유지 | TLS 배치에서 3.6절 | 해당 없음. 예시는 TLS를 끈다. 자동 증거는 OPS-024 | 해당 없음 |
| 로그 회전 | 원래 경로 재생성, 새 사건 기록 | 3.7절 | Windows·Ubuntu 통과 | [CLI 계약 실행 기록](../contracts/cli-v1.md#계약-테스트-실행-기록) |
| 종료 | 정상 종료 코드 0, `server_stopped` | 3.8절 | Windows·Ubuntu 통과 | 같음 |
| 설정 보기·편집·복구 | 읽기 전용 기본값, 활성화 뒤 재시작·재로그인, 다섯 파일 상태, 충돌·쓰기 실패·응답 불명 복구 | 3.5.1절과 6절. OS 계정·파일/디렉터리 권한·관찰값을 기록 | 미실행 — 2026-11-08 추가 절차. 과거 수동 기록은 새 편집 기능의 증거가 아님 | 통합 검증 담당자가 실행 기록을 추가 |
| v3→v4 이관·롤백 | 기존 Viewer 동작 유지, Viewer 경로는 릴리스 `2026.11.2`까지 유지(더 새 실행파일은 3.2.3절), 새 로그 표기, 이전 실행파일·설정의 함께 복구 | 3.2.2절·6절 | 부분 — Windows에서 이관 뒤 Viewer 동작·`/relay` 경로와 새 로그 표기 확인(2026-11-09, 릴리스 `2026.11.2`). 관리 재로그인·롤백·Ubuntu 미실행 | Windows는 아래 2026-11-09 기록. Ubuntu 수동 실행 기록 필요. v3 테이블의 시작 실패는 OPS-066 자동 시험이 통과([판정표](set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)) |
| 공유 WSS Viewer·Server | 같은 리스너의 두 경로로 Mode II 양방향 중계, 역할별 IP·Origin | 3.2.1절 | 미실행 | 두 OS 수동 실행 기록 필요. 자동 증거 OPS-065·OPS-069, WS-028~031은 통과([판정표](set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)) |
| 프록시 TLS 종료 | 외부 WSS→평문 WS, 오른쪽 첫 비신뢰 주소의 역할별 허용 판정, 잘못된·모두 신뢰하는 체인 거절 | 3.9절, Nginx 버전·`nginx -t`·양쪽 주소·비밀 없는 로그 기록 | 미실행, Nginx 예시 추정 | 실제 배치 기록 필요. 자동 증거 WS-032~036은 통과([판정표](set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)) |
| 최근 로그 | 설정 상한·오래된 항목 제거·재시작·0 보관·필터·JSON Lines 내려받기 | 3.7.1절 | 미실행 | 두 OS 수동 실행 기록 필요. 자동 증거 OPS-068, MGT-046~050, DASH-054~063은 통과([판정표](set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)) |
| Viewer 경로 이전·롤백 | `/relay/viewer` 중계 성공, 이전 경로 `/relay`는 404, 롤백 때 이전 실행파일과 `/relay` URL을 함께 복구 | 3.2.3절 | 미실행 | 두 OS 수동 실행 기록 필요. 자동 증거 WS-012·WS-029는 통과([판정표](set-up-and-verify.md#44-viewer-경로-이전-판정표)) |
| 접속 ticket | ticket이 있는 WS·WSS 업그레이드의 101(`binary`), 없거나 맞지 않는 ticket의 401, 공개 키 교체 뒤 새 키의 ticket 통과, 끄기 | 3.10절. ticket 원문과 공개 키 파일 내용은 기록하지 않음 | 미실행 | 두 OS 수동 실행 기록 필요. 자동 증거는 [접속 ticket 판정표](set-up-and-verify.md#45-접속-ticket-판정표) |

2026-11-07 갱신: 위 2026-11-05·커밋 `1c7c7a9` 묶음의 `LICENSE 없음(Q97 대기)`는 당시 기록으로 보존한다. 루트 `LICENSE` 작성·검토는 마쳤다(사용자 결정 2026-11-07(Q97), [검토 기록](check-dependency-licenses.md#42-프로젝트-license-검토-기록2026-11-07)). 앞으로 만드는 두 OS 배포 묶음에는 3.1절에 따라 루트 `LICENSE`를 포함한다. 과거 묶음을 다시 만들거나 LICENSE 포함을 재확인했다는 뜻은 아니다.

자동 증거는 `tests/distribution.rs`와 `tests/windows_console.rs`에 있다. 공통 본문은 예시를 복사하고 포트·경로만 바꾼다. 각 OS의 실제 CLI 계정 준비 뒤 시작·양방향 중계·IP 재로드·정상 종료를 확인한다. `cargo test`는 debug 실행파일을 빌드한다. release 실행파일과 안내 전체의 증거는 두 OS 수동 기록으로 보완한다(사용자 결정 2026-11-04(Q107)). 이 시험 분리와 OPS-019의 OS별 비부분 표식은 **이 문서의 판단, 사용자 결정 없음**이다. OPS-031 자동 표식은 부분이다.

수동 기록에는 날짜, 커밋, OS 버전·아키텍처, `rustc --version`, 빌드 명령·결과, 실행파일 버전·SHA-256, 각 단계 명령과 관찰값을 남긴다. Windows 해시는 `Get-FileHash .\vnc-relay.exe -Algorithm SHA256`, Ubuntu 해시는 `sha256sum ./vnc-relay`로 읽는다. 비밀 없는 로그와 종료 코드를 붙인다. 두 OS 기록과 자동 시험을 함께 검토한 뒤 OPS-031을 판정한다.

2026-11-08 실제 배치 교체 기록(Windows 11 Pro 10.0.26200 x64, PowerShell 7): 운영 중인 `2026.11.0` 서버를 [Release `v2026.11.1`](https://github.com/kayden-kim/vnc-relay/releases/tag/v2026.11.1)의 Windows 묶음으로 바꿨다. 설정 파일은 v2 형식 그대로 두었다. 이 배치는 TCP 두 진입점, WSS 진입점(내장 TLS), 루프백 평문 관리 리스너, 파일 로그를 쓴다.

1. 새 실행파일(SHA-256 `f97c18a55d3a2b2d1b2567b33537f6663c1b78acb6ffe91e379e3a4f63884064`)을 같은 설정·계정·TLS 파일로 먼저 실행했다. 리스너 주소를 루프백 대체 포트로, 로그 출력을 표준 출력으로 바꾼 설정 사본을 썼다. `server_ready`, `/livez`·`/readyz` 200, WSS TLS 1.3을 확인했고 Ctrl+Break 뒤 종료 코드는 0이었다.
2. 서버 터미널에서 Ctrl+C로 이전 서버(SHA-256 `f0727fcd828793254acd68278a1f5f253ce5a462f346480943445de21e40b9df`)를 멈췄다. 활성 세션이 없었고, 0.65초 뒤 종료 코드 0으로 끝났다. 로그에 `shutdown_started`와 `server_stopped`가 있었다.
3. 이전 설치 디렉터리는 다른 이름으로 옮겨 보존하고, 묶음을 같은 경로에 풀었다. 같은 명령으로 다시 시작했다. 정지부터 `server_ready`까지 13.6초였고, 리스너 다섯 개가 이전과 같은 주소로 열렸다. `/livez`·`/readyz` 200, 관리 `/login/` 200, `/` 303(`/login/`)이었다. WSS는 허용 Origin에 101, 다른 Origin에 403(`origin_not_allowed`)으로 답했다. 공인 주소로 되돌아오는 연결(공유기 NAT 루프백)에서도 TLS와 101을 확인했다.

이 기록은 6절의 "v2 파일을 수정 없이 v3 서버로 재시작"을 Windows에서 한 번 확인한 것이다. 3.4절 중계, 3.5절 재로드, 3.5.1절 설정 편집과 Ubuntu는 실행하지 않았다. 그래서 위 표와 문서 상태는 바꾸지 않는다.

후속: 사용자 결정 2026-11-08(Q181~Q188)의 설정 v4·ops-info 2.0.0·공유 역할·프록시·최근 로그는 위 실제 배치 교체 뒤의 기준이다. 위 기록은 바꾸지 않는다. 새 기준의 구현·배포·검증 증거로 확대하지 않는다.

2026-11-09 실제 배치 교체·v3→v4 이관 기록(Windows 11 Pro 10.0.26200 x64, PowerShell 7): 운영 중인 `2026.11.1` 서버를 [Release `v2026.11.2`](https://github.com/kayden-kim/vnc-relay/releases/tag/v2026.11.2)의 Windows 묶음으로 바꿨다. 위 2026-11-08 기록에서 v2 형식 그대로 쓰던 운영 `relay.toml`은 3.2.2절에 따라 v4 형식으로 옮겼다. 배치 구성은 위 기록과 같다.

1. 이전 실행파일(`2026.11.1`, SHA-256 `f97c18a55d3a2b2d1b2567b33537f6663c1b78acb6ffe91e379e3a4f63884064`), `relay.toml`, 계정 파일, TLS 파일 쌍을 복구 디렉터리에 복사했다. 이 디렉터리는 권한 상속을 끊고 Administrators·SYSTEM·현재 사용자만 접근하게 했다. 목록 파일에 실행파일 버전과 다섯 파일의 원래 경로·SHA-256을 남겼다.
2. 같은 디렉터리에 `relay-v4.toml` 사본을 만들었다. `[relay.wss_viewer]`의 `listen`·`cert_file`·`key_file`은 `[relay.wss]`로, `enabled`·`allow`·`allowed_origins`는 `[relay.wss.viewer]`로 옮겼다. 꺼져 있던 `[relay.ws_viewer]`는 `[relay.ws.viewer]`의 `enabled = false`로 옮겼다. Server 역할·`trusted_proxies`·`[recent_logs]`는 추가하지 않았다.
3. 검증 사본 `relay-v4-check.toml`은 리스너 다섯 개를 서로 다른 루프백 대체 포트로, 로그 출력을 표준 출력으로 바꿨다.
4. Release에서 내려받은 묶음을 설치 디렉터리 옆의 준비 디렉터리에 풀었다. 실행파일 SHA-256은 `3100e8033c153f6814376377eea6113b50202ff428fa6d1774f4874147875181`이다. 이 실행파일로 검증 사본을 시작했다. `server_starting.log_contract`는 `ops-info/2.0.0`이었고, 명령을 보낸 뒤 1.7초 안에 `server_ready`가 나왔다. `/livez`·`/readyz` 200, 관리 `/login/` 200, `/` 303(`/login/`)이었다. WSS는 TLS 1.3과 두 이름의 SAN을 보였다. 두 허용 Origin의 WSS Viewer와 TCP Server, TCP Viewer와 TCP Server를 각각 짝지어 양방향 시험 바이트가 일치했다. 다른 Origin은 403, `/relay/server`는 404였다(Server 역할 꺼짐). Ctrl+C 뒤 종료 코드는 0이었다.
5. 로그로 활성 세션과 짝을 기다리는 연결이 0건임을 확인했다. 서버 터미널에서 Ctrl+C로 이전 서버를 멈췄다. `shutdown_started`(활성 세션 0)와 `server_stopped`가 기록됐고 종료 코드는 0이었다.
6. 이전 설치 디렉터리는 다른 이름으로 옮겨 보존했다. 준비 디렉터리를 같은 경로로 옮기고, `relay-v4.toml`을 `relay.toml`로 바꿨다. 같은 명령으로 다시 시작했다. `shutdown_started`부터 `server_ready`까지 2.3초였고, 리스너 다섯 개가 이전과 같은 주소로 열렸다.
7. 시작 로그는 새 표기를 따랐다. `log_contract`는 `ops-info/2.0.0`이고, WSS의 `allow_all_source`는 `listener = wss`와 `role = viewer`로 기록됐다. 운영 포트에서 위 4번과 같은 확인이 모두 통과했다. 공인 주소로 되돌아오는 연결(공유기 NAT 루프백)에서도 두 이름의 TLS 1.3, 두 Origin의 WSS 짝 연결(101, 바이트 일치), TCP 짝 연결을 확인했다.
8. 3.5절 재로드는 실제 Viewer를 막지 않도록 바꿔 실행했다. `relay.tcp_viewer.allow`에 `"::1"`을 더했다. 이 리스너는 IPv4 주소에만 바인드하므로 실제 허용 범위는 같다. `reload_change_detected` 뒤 0.42초에 `reload_applied`(`trigger = file_watch`, `applied = ["relay.tcp_viewer.allow"]`)가 기록됐다. 되돌린 뒤에도 0.41초 만에 같은 사건이 기록됐고, 파일 해시는 교체 때와 같았다. 되돌린 뒤 TCP 짝 연결도 성공했다.

시험 Viewer·Server는 RFB 버전 문자열과 시험 바이트만 주고받는 Python 클라이언트다. ID는 `99`로 시작하는 12자리를 썼다. `server_ready`부터 확인을 마칠 때까지 기록된 경고 수준 사건 9건은 모두 이 확인에서 나왔다. 연결 경고 7건은 TLS만 맺고 닫은 연결, 다른 Origin과 `/relay/server` 거절, 연결만 하고 닫은 도달성 확인이다. 나머지 2건은 8번의 재로드마다 다시 기록된 `allow_all_source`다. 확인을 마친 뒤 활성 세션과 대기 연결은 0건이었다. 검증 사본의 출력은 저장소 밖에 보관하고 사본은 지웠다.

이 기록은 3.2.2절 이관을 Windows에서 한 번 실행한 것이다. 3.2.2절 8단계의 관리 재로그인은 계정 비밀번호가 필요해 실행하지 않았다. 6절 v4→v3 롤백, Ubuntu 이관, 3.5.1절 설정 편집, 실제 noVNC·UltraVNC·부스 기기의 중계도 실행하지 않았다. 이관 전 파일과 이전 실행파일은 복구 디렉터리와 보존한 설치 디렉터리에 남겨 두었다.

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

`trust_all_proxies` 이름과 필드, 로그 조회 매개변수·응답 필드 이름과 세션 필터 URL은 사용자 결정 2026-11-08(Q191)으로 채택했다. 로그 화면 시안·상단 메뉴 순서는 사용자 결정 2026-11-08(Q189)으로, CPY-149~189 ko/en 원문은 사용자 결정 2026-11-08(Q190)으로 채택했다. [운영 스펙 8절](../specs/operations.md#8-미결정-사항)에서 기록을 관리한다. 실제 배포의 구현 여부는 채택과 별도로 확인해야 한다.

## 7. 해결되지 않을 때 전달할 대상

- 전달 대상: 이 배포의 운영 책임자와 프로젝트 관리자다.
- 보존할 증거: OS·실행파일 버전·해시, 실패 명령, 종료 코드, 비밀 없는 진단 코드, 해당 사건 전후 로그다.
- 비밀 값 제외 규칙: 비밀번호·해시·개인키·계정 파일 본문을 보내지 않는다. 원시 VNC 데이터와 사용자 입력을 수집하지 않는다. 접속 주소·관리자 계정명이 필요하지 않으면 가린다.
