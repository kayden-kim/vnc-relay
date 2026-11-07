# 실행파일을 묶고 수동 운영하는 방법

Windows 11 x64와 Ubuntu 24.04 LTS x64에서 release 실행파일을 만들고 수동으로 운영한다. 계정을 준비한 뒤 시작·양방향 중계·자동 재로드·정상 종료를 확인한다. OS 서비스 등록과 컨테이너는 이 절차의 대상이 아니다(사용자 결정 2026-11-04(Q105)).

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 배포 담당자와 서버를 직접 실행하는 운영자 |
| 검증 상태 | Draft — 두 OS 수동 실행 기록 전이다. 자동 시험만으로 안내 전체를 검증했다고 표시하지 않는다. |
| 검증 환경·버전 | Windows 11 x64의 PowerShell, Ubuntu 24.04 LTS x64의 bash. Rust 버전은 `rust-toolchain.toml`을 따른다. 실제 OS·Rust·실행파일 버전은 실행 기록에 적는다. |
| 최종 검증일 | 없음(Draft 2026-11-04) |
| 관련 기준 문서 | [운영 스펙](../specs/operations.md), [설정 계약](../contracts/config-v2.md), [CLI 계약](../contracts/cli-v1.md), [운영 정보 계약](../contracts/ops-info-v1.md), [ADR-0001](../decisions/ADR-0001-built-in-tls.md) |

## 1. 대상과 영향

- 대상: 실행파일 하나, 설정 파일, 계정 파일, 운영자가 제공한 인증서·개인키, 운영 로그다.
- 예상 사용자 영향: 시작은 리스너를 연다. 재로드는 새 연결의 허용 정책·TLS 협상을 바꾼다. 종료·재시작은 연결과 관리 세션을 끝낸다.
- 작업 허용 시간: 재시작이 필요한 변경은 운영자가 승인한 중단 시간에 한다.

예시는 루프백 TCP의 Mode II만 켠다. 관리 HTTP와 상태 리스너도 루프백이다. 이 구성 선택은 **이 문서의 판단, 사용자 결정 없음**이다. 공개 주소에 그대로 옮기는 예시가 아니다.

인터넷에 공개하는 WebSocket 진입점에는 WSS를 권장한다. 평문 WS는 보호된 네트워크에서만 켜기를 권장한다. 보호 여부와 공개 배치의 책임은 운영자에게 있다(사용자 결정 2026-11-04(Q96)). 관리 리스너를 외부에 열 때는 내장 HTTPS가 필수다. TLS 종료를 역방향 프록시에 맡기지 않는다.

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

   로컬 검증에서는 예시 그대로 쓴다. 공개 배치에서는 리스너 주소와 출발지 `allow`를 의도한 값으로 정한다. `[]`는 모두 거절이며 목록 생략은 시작 실패다. `0.0.0.0/0`·`::/0`는 전체 허용과 경고를 뜻한다. Mode I을 켜면 `mode1.allow_targets`도 명시해야 한다. WSS는 `listen`·`allow`·필요한 `allowed_origins`·`cert_file`·`key_file`을 설정한다. 브라우저 Origin은 IP 제한을 대신하지 않는다.

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

재로드 대상은 켠 진입점의 출발지 IP `allow`와 TLS 인증서·개인키의 경로·내용뿐이다. 기존에 허용한 초기 교환·대기·활성 연결은 유지한다. 새 허용 판정과 새 TLS 핸드셰이크부터 적용한다.

리스너 주소·포트, 진입점 켜기·끄기, HTTP/HTTPS 전환, 모드·계정·Origin·Mode I 목적지 정책, 한도·시간·keepalive·대시보드·로그·재로드 주기는 재시작해야 한다. 비재로드 값이 실행 중 값과 다르면 후보 전체를 `reload_restart_required`로 거절한다. 그 차이를 되돌리거나 서버를 재시작하기 전에는 급한 인증서 교체도 적용되지 않는다.

1. 실행 중인 예시의 `relay.tcp_viewer.allow`만 `["127.0.0.1", "::1"]`로 바꾸어 저장한다.

   기대 결과: OS 알림 또는 Dp 주기 확인으로 변경을 감지한다. 예시의 D는 5초, Dp는 30초다. 알림을 놓쳐도 Dp + D 안에 `reload_applied` 또는 실패 결과를 확인한다.

2. `relay.log`에서 `reload_applied`를 확인한다.

   실패하면 이전 유효 설정과 기존 중계를 유지한다. `reload_rejected`·`reload_restart_required`·감지 장애 사건을 성공으로 판단하지 않는다. 수동 재로드 CLI와 웹 재로드 API는 없다.

3. 3.4절의 클라이언트를 다시 실행한다.

   기대 결과: 양방향 바이트가 다시 일치한다. 이 확인은 IP 후보 적용 뒤 새 연결의 중계 확인이다.

### 3.6 인증서 교체

TLS를 켠 배치에만 적용한다. TLS 1.2 이상이 필요하다. 지원 PEM 체인·키 형식은 [설정 계약 1.9절](../contracts/config-v2.md#19-인증서개인키-파일-형식)을 따른다. 비밀키 내용은 로그·명령행에 넣지 않는다.

1. 새 인증서 체인과 일치하는 개인키를 보호된 새 파일 두 개에 저장한다.

   기대 결과: 서버 실행 OS 계정이 두 파일을 읽을 수 있다. 다른 일반 OS 계정은 개인키를 읽거나 바꿀 수 없도록 제한하기를 권장한다.

2. 설정의 해당 `cert_file`·`key_file` 경로를 새 쌍으로 함께 바꾸어 저장한다.

   새 파일을 먼저 준비한 뒤 경로 쌍을 한 번에 바꾸는 절차는 **이 문서의 판단, 사용자 결정 없음**이다. 같은 경로의 파일을 각각 교체해도 감지하지만 한쪽만 바뀐 후보는 거절한다.

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

### 3.8 정상 종료와 재시작

1. 서버를 실행한 터미널에서 Ctrl+C를 한 번 누른다.

   Ubuntu는 SIGINT(Ctrl+C)와 `kill -TERM <PID>`를 정상 종료 요청으로 받는다. Windows는 Ctrl+C와 Ctrl+Break를 받으며 SIGTERM은 없다. 신규 중계를 중지하고 미완성 작업을 정리한다. 활성 세션에는 `timeouts.shutdown_grace_secs`(G)의 유예를 준다. 기본값은 30초다. G가 0이면 바로 정리한다. 유예 중 관리 로그인·조회·세션 종료는 계속 가능하다.

2. 서버의 종료 결과를 확인한다.

   Windows: 서버 명령이 돌아온 같은 PowerShell에서 `$LASTEXITCODE`를 읽는다.

   Ubuntu: 서버 명령이 돌아온 같은 bash에서 `echo $?`를 실행한다.

   기대 결과: `0`이고 로그에 `server_stopped`가 있다. G 만료로 남은 세션을 정리한 경우도 정상 종료 코드 0이다.

유예 중 두 번째 종료 요청은 남은 유예를 건너뛰고 즉시 강제 정리한다. Windows 콘솔 창 닫기도 처음부터 이 방식으로 처리한다. G를 기다리지 않으며 OS의 기본 5초 제한 안에 정리와 종료 기록을 마쳐야 한다(사용자 결정 2026-11-04(Q102)). 일반적인 운영 종료에는 Ctrl+C를 권장한다.

Windows 로그오프·시스템 종료는 강제 종료다. 대화형 콘솔 프로그램의 정상 유예·종료 기록을 보장하지 않는다. SIGKILL·작업 관리자 종료·전원 장애도 같다. 다시 시작하면 이전 소켓·대기·세션·관리 로그인은 복원되지 않는다.

## 4. 검증

각 OS에서 3.1~3.8절을 실행한다. TLS 배치가 아니면 3.6절은 해당 없음으로 기록한다. OPS-031의 일반 배포 확인에 없는 TLS 동작을 확인한 것처럼 표시하지 않는다. 인증서 교체 자체의 자동 증거는 OPS-024가 담당한다.

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

2026-11-07 갱신: 위 2026-11-05·커밋 `1c7c7a9` 묶음의 `LICENSE 없음(Q97 대기)`는 당시 기록으로 보존한다. 루트 `LICENSE` 작성·검토는 마쳤다(사용자 결정 2026-11-07(Q97), [검토 기록](check-dependency-licenses.md#42-프로젝트-license-검토-기록2026-11-07)). 앞으로 만드는 두 OS 배포 묶음에는 3.1절에 따라 루트 `LICENSE`를 포함한다. 과거 묶음을 다시 만들거나 LICENSE 포함을 재확인했다는 뜻은 아니다.

자동 증거는 `tests/distribution.rs`와 `tests/windows_console.rs`에 있다. 공통 본문은 예시를 복사하고 포트·경로만 바꾼다. 각 OS의 실제 CLI 계정 준비 뒤 시작·양방향 중계·IP 재로드·정상 종료를 확인한다. `cargo test`는 debug 실행파일을 빌드한다. release 실행파일과 안내 전체의 증거는 두 OS 수동 기록으로 보완한다(사용자 결정 2026-11-04(Q107)). 이 시험 분리와 OPS-019의 OS별 비부분 표식은 **이 문서의 판단, 사용자 결정 없음**이다. OPS-031 자동 표식은 부분이다.

수동 기록에는 날짜, 커밋, OS 버전·아키텍처, `rustc --version`, 빌드 명령·결과, 실행파일 버전·SHA-256, 각 단계 명령과 관찰값을 남긴다. Windows 해시는 `Get-FileHash .\vnc-relay.exe -Algorithm SHA256`, Ubuntu 해시는 `sha256sum ./vnc-relay`로 읽는다. 비밀 없는 로그와 종료 코드를 붙인다. 두 OS 기록과 자동 시험을 함께 검토한 뒤 OPS-031을 판정한다.

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

## 7. 해결되지 않을 때 전달할 대상

- 전달 대상: 이 배포의 운영 책임자와 프로젝트 관리자다.
- 보존할 증거: OS·실행파일 버전·해시, 실패 명령, 종료 코드, 비밀 없는 진단 코드, 해당 사건 전후 로그다.
- 비밀 값 제외 규칙: 비밀번호·해시·개인키·계정 파일 본문을 보내지 않는다. 원시 VNC 데이터와 사용자 입력을 수집하지 않는다. 접속 주소·관리자 계정명이 필요하지 않으면 가린다.
