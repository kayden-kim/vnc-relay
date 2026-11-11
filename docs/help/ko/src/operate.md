# 운영

서버를 수동으로 시작하고 종료하며 상태, 로그와 설정 반영 결과를 확인할 수 있다. 인증서·공개 키·관리자 계정을 교체하고 업그레이드나 롤백을 준비할 수도 있다.

이 절차는 Windows 11 x64의 PowerShell과 Ubuntu 24.04 LTS x64의 bash를 대상으로 한다. OS 서비스 등록과 컨테이너는 범위에 포함하지 않는다. 명령은 배포 묶음을 푼 운영 디렉터리에서 실행한다. 먼저 [빠른 시작](quick-start.md)에 따라 설정과 관리자 계정을 준비해야 한다.

## 포그라운드 실행

Windows:

```powershell
.\vnc-relay.exe serve --config .\relay.toml
```

Ubuntu:

```shell
./vnc-relay serve --config ./relay.toml
```

프로세스는 종료할 때까지 터미널을 점유한다. `server_ready` 사건이 기록되기 전에는 준비 완료로 판단하지 않는다. 설정 안의 상대 파일 경로는 터미널 위치가 아니라 설정 파일 디렉터리를 기준으로 푼다.

## 상태 확인

예시 설정은 관리 리스너를 `127.0.0.1:18080`, 상태 리스너를 `127.0.0.1:18081`에 둔다. 상태 확인은 **상태 리스너**로 보낸다.

다른 터미널에서 다음 두 요청을 실행한다.

Windows:

```powershell
curl.exe --fail http://127.0.0.1:18081/livez
curl.exe --fail http://127.0.0.1:18081/readyz
```

Ubuntu:

```shell
curl --fail http://127.0.0.1:18081/livez
curl --fail http://127.0.0.1:18081/readyz
```

`/livez`는 응답 가능한 프로세스에 HTTP 200과 `{"status":"alive"}`를 돌려준다. `/readyz`는 준비됨 상태에 HTTP 200과 `{"status":"ready"}`를 돌려준다. 준비되지 않았으면 503이다. 종료 유예 중에도 `/livez`는 응답 가능한 동안 200이지만 `/readyz`는 503이다.

상태 리스너에는 인증이 없다. 생존·준비 결과만 제공하며 세션 목록은 제공하지 않는다. 루프백 바인딩을 권장한다. 대시보드 사용법은 [대시보드](dashboard.md)를 따른다.

## 정상 종료와 재시작

1. 서버를 실행한 터미널에서 Ctrl+C를 한 번 누른다.
2. `server_stopped` 사건과 명령 종료를 기다린다.
3. 같은 터미널에서 종료 코드를 확인한다.

Windows:

```powershell
$LASTEXITCODE
```

Ubuntu:

```shell
echo $?
```

정상 종료가 끝나면 종료 코드는 0이다. 신규 중계는 중지하며 활성 세션에는 `timeouts.shutdown_grace_secs`(G)의 종료 유예를 준다. 기본값은 30초다. G가 0이면 바로 정리한다. 유예 만료로 남은 세션을 정리한 경우도 정상 종료 코드 0이다.

종료 유예 중에도 관리 로그인·조회·관리자 종료와 설정 보기·편집은 가능하다. 파일 감지의 새 적용은 멈추지만 설정 편집의 저장·적용은 허용한다. 이미 시작한 종료 유예는 G를 편집해도 바뀌지 않는다.

유예 중 두 번째 종료 요청은 남은 유예를 건너뛰고 즉시 강제 정리한다. Ubuntu는 SIGINT(Ctrl+C)와 SIGTERM을 받는다. Windows는 Ctrl+C와 Ctrl+Break를 받으며 SIGTERM은 없다.

> **주의:** Windows 콘솔 창 닫기는 G를 기다리지 않고 처음부터 즉시 강제 정리한다. OS의 기본 5초 제한 안에 종료해야 한다. 일반 운영 종료에는 Ctrl+C를 권장한다. Windows 로그오프·시스템 종료, 작업 관리자 종료, SIGKILL과 전원 장애에서는 정상 유예나 종료 기록을 보장하지 않는다.

다시 시작하려면 같은 `serve` 명령을 실행한다. 이전 소켓, 대기 연결, 활성 세션과 관리 세션은 복원하지 않는다. 관리자는 다시 로그인해야 한다.

## 운영 로그와 파일 회전

운영 로그는 UTF-8 JSON Lines다. 한 줄에 JSON 객체 하나를 쓴다. `log.output = "stdout"`이면 표준 출력으로 내보낸다. 파일 경로이면 기존 파일 뒤에 이어 쓰며 파일이 없으면 만든다. 상위 디렉터리는 운영자가 미리 만들어야 한다. 출력 경로를 바꾸려면 재시작해야 한다.

다음 절차는 `log.output = "relay.log"`인 경우다. 보관 이름 `relay.previous.log`가 아직 없어야 한다.

1. 실행 중인 로그 파일의 이름을 바꾼다.

   Windows:

   ```powershell
   Rename-Item -LiteralPath .\relay.log -NewName relay.previous.log
   ```

   Ubuntu:

   ```shell
   mv -- ./relay.log ./relay.previous.log
   ```

2. 시험 연결 등으로 운영 로그 사건을 만든다.
3. 원래 경로 `relay.log`에 새 사건이 이어지는지 확인한다.

서버는 이름 변경·삭제·생성 알림을 받으면 원래 경로를 다시 연다. 알림을 놓쳐도 `reload.poll_secs`마다 다시 연다. 다시 열기 전의 사건은 옛 파일에 남을 수 있다. 복사 후 원본 자르기(copytruncate)는 이 회전 절차로 쓰지 않는다.

다시 열기에 실패하면 이전 파일에 계속 쓴다. 실패가 시작될 때 `log_reopen_failed` 사건과 stderr 경고가 나온다. 경로와 권한을 복구한 뒤 다음 알림이나 주기 확인 결과를 확인한다. 로그 줄 쓰기 실패는 처음 한 번만 stderr의 `log_write_failed` 경고로 알린다. 실패한 줄은 버리고 서비스를 계속한다. 누락된 줄이 자동 복구된다고 기대하지 않는다.

회전 시점, 보관 기간과 접근 권한은 운영자 책임이다. 로그와 보관 파일을 승인된 운영자만 읽도록 제한하고 디스크 여유와 stderr를 감시하기를 권장한다. [대시보드의 최근 로그](dashboard.md)는 별도 메모리 목록이며 파일 보관을 대신하지 않는다.

## 파일 변경 자동 재로드

서버는 설정 파일과 사용 중인 TLS 파일, 접속 ticket 확인을 켠 역할의 공개 키 파일 변경을 자동 감지한다. 모든 역할이 꺼진 WSS 리스너의 TLS 파일은 감시하지 않는다. OS 알림을 주기 확인으로 보완한다. `reload.poll_secs`(Dp)와 `reload.settle_secs`(D)의 기본값은 각각 30초와 5초다. 파일 변경이 멈춘 상태라면 알림을 놓친 변경도 Dp + D 안에 적용·실패 결과를 확인한다. 연속 변경에서는 마지막 변경을 감지한 뒤 D 안에 안정된 최신 파일의 결과를 확인한다.

재로드 대상에는 출발지 IP·Origin·신뢰 프록시 정책, Mode I 대상 허용 정책, 연결 상한·시간 제한·keepalive, TLS 파일 경로와 내용, 접속 ticket 설정과 공개 키 파일이 있다. 키별 적용 시점은 [설정 계약 1.8절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md)을 따른다.

1. 보호된 위치에 현재 유효 파일의 복구본을 보관한다.
2. 외부 편집기로 필요한 재로드 대상 값만 바꾸어 저장한다.
3. 운영 로그에서 `reload_applied`를 확인한다.
4. 바꾼 정책이 적용될 새 연결이나 요청으로 결과를 확인한다.

이미 끝난 검사, 기존 연결, 시작한 타이머와 설정한 소켓 옵션은 이전 결과를 유지한다. 상한을 현재 점유보다 낮춰도 기존 연결을 끊지 않고 새 점유를 거절한다.

리스너 주소·포트, 중계 역할·모드 켜기/끄기, 관리 HTTP/HTTPS 전환, 계정·관리 세션 정책과 `management.config_edit` 등은 재시작해야 한다. 재시작 필요 값이 실행값과 다르면 후보 전체를 `reload_restart_required`로 거절한다. 그 차이가 남아 있으면 이후의 급한 인증서 교체도 막힌다.

후보 검증에 실패하면 `reload_rejected`를 기록한다. 감지 장애는 `reload_watch_failed`로 알린다. 실패하면 이전 유효 설정과 기존 중계를 유지한다. 수동 재로드 CLI나 웹 재로드 API는 없다. 파일 상태와 마지막 결과는 대시보드 "설정 보기"에서도 확인할 수 있다.

## 인증서와 공개 키 파일 교체

### TLS 인증서와 개인키

TLS를 켠 배치에만 적용한다. 인증서와 개인키는 유효한 쌍이어야 한다. 개인키 내용을 로그나 명령행에 넣지 않는다.

1. 새 인증서 체인과 일치하는 개인키를 보호된 새 파일에 준비한다.
2. 서버 실행 OS 계정이 두 파일을 읽을 수 있는지 확인한다.
3. 외부 편집기로 해당 `cert_file`과 `key_file` 경로를 새 쌍으로 함께 바꾸어 저장한다.
4. `reload_applied`를 확인한다.
5. 새 TLS 연결에서 공개 인증서의 일련번호나 지문을 대조한다.

같은 경로의 파일 내용 교체도 자동 감지한다. 한쪽만 바뀌어 쌍이 맞지 않거나 파일을 읽지 못하면 후보를 거절하고 이전 TLS를 유지한다. 기존 TLS 연결의 인증서는 바뀌지 않는다. 대시보드에서는 TLS 경로를 편집하지 못한다. 파일 교체 중에는 대시보드 저장을 동시에 하지 않는다.

### 접속 ticket 공개 키

접속 ticket을 켠 역할에 적용한다. 서명 키는 발급 쪽에만 두고 VNC Relay에는 공개 키만 둔다. 파일 경로는 대시보드에서 편집할 수 없으므로 같은 경로의 파일 내용을 교체한다.

1. 발급 쪽에서 새 서명 키 쌍을 준비한다.
2. 해당 역할의 기존 공개 키 파일 `keys` 목록에 새 공개 키를 추가한다.
3. `reload_applied`에서 해당 `public_keys_file`의 적용을 확인한다.
4. 발급 쪽을 새 서명 키로 바꾼다.
5. 그 역할의 `max_lifetime_secs` + 10초를 기다린다.
6. 파일에서 옛 공개 키를 지운다.
7. 다시 `reload_applied`를 확인한다.

잘못된 공개 키 파일은 `ticket_keys_invalid`로 거절하며 이전 공개 키를 유지한다. 이미 업그레이드한 연결과 사용 기록은 교체로 지우지 않는다. 파일 쓰기 권한도 제한하기를 권장한다. 다른 키를 넣을 수 있는 사람은 자신이 만든 ticket을 통과시킬 수 있다. 형식과 전제 조건은 [접속 ticket](access-ticket.md)을 따른다.

## 관리자 계정 관리

계정 추가·비밀번호 변경·삭제에는 로컬 CLI를 쓴다. 기본 계정과 기본 비밀번호는 없다. 명령은 설정의 `admin.accounts_file`을 사용한다.

1. 서버를 정상 종료한다.
2. 필요한 명령 하나를 실행한다.
3. `admin list`로 남은 계정명을 확인한다.
4. 서버를 다시 시작한다.
5. 남아 있는 계정으로 다시 로그인한다.

Windows:

```powershell
.\vnc-relay.exe admin add opsadmin --config .\relay.toml
.\vnc-relay.exe admin passwd opsadmin --config .\relay.toml
.\vnc-relay.exe admin remove opsadmin --config .\relay.toml
.\vnc-relay.exe admin list --config .\relay.toml
```

Ubuntu:

```shell
./vnc-relay admin add opsadmin --config ./relay.toml
./vnc-relay admin passwd opsadmin --config ./relay.toml
./vnc-relay admin remove opsadmin --config ./relay.toml
./vnc-relay admin list --config ./relay.toml
```

위 명령은 선택지다. 모두 차례로 실행하지 않는다. `add`는 추가, `passwd`는 비밀번호 변경, `remove`는 삭제다. `list`는 이름만 보여 준다. `add`와 `passwd`는 대화형 터미널에서 비밀번호를 숨겨 두 번 받는다. 비밀번호를 명령행이나 파이프에 넣지 않는다. `remove`는 삭제 확인을 묻는다.

계정 파일 내용은 자동 재로드하지 않는다. 실행 중 파일만 바꾸면 이전 자격 증명과 기존 관리 세션이 재시작 전까지 유효하다. 마지막 계정을 지우면 서버가 시작하지 않는다. 계정 파일 접근 권한은 [CLI 계약 1.6절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md)을 따른다.

## 업그레이드와 롤백

재시작은 모든 연결과 관리 세션을 끝낸다. 승인된 중단 시간에 작업해야 한다.

1. 현재 실행파일·설정·계정 파일·TLS 파일과 사용하는 접속 ticket 공개 키 파일을 보호된 복구 위치에 보관한다. 실행파일 버전과 해시도 기록한다.
2. [Releases](https://github.com/kayden-kim/vnc-relay/releases)에서 대상 OS 묶음을 받아 해시를 확인한다. 설치 절차는 [설치](install.md)를 따른다.
3. 별도 디렉터리에 새 실행파일을 준비한다. 설정 사본은 운영 설정과 같은 디렉터리에 만든다. 상대 파일 경로의 기준을 유지하기 위해서다. 설정 계약이 바뀌면 사본을 먼저 이관한다.
4. 설정 사본의 켠 리스너들을 서로 다른 미사용 루프백 포트로 바꾼다. 로그는 `stdout`으로 바꾼다. 운영 계정·TLS 파일은 수정하지 않는다.
5. 새 실행파일과 그 사본으로 시작한다. `server_ready`와 상태 리스너 `/readyz`의 200을 확인한다.
6. 시험 프로세스를 Ctrl+C로 정상 종료한다.
7. 중단 시간에 기존 서버를 정상 종료한다.
8. 새 실행파일과 검증한 **운영 주소 설정**을 운영 위치에 설치한다. 대체 포트 사본을 설치하지 않는다.
9. 시작·상태·관리 재로그인·실제 중계·재로드를 확인한다.

별도 설정 검증 전용 CLI는 없다. 대체 포트 시작은 운영 포트, 방화벽과 실제 클라이언트 중계를 대신 검증하지 않는다. 소스 빌드와 배포 묶음은 [운영 how-to](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md)의 상세 절차를 따른다.

> **주의:** 이 도움말은 `main` 기준이다. 최신 Release `v2026.11.2`의 WS·WSS Viewer 경로는 `/relay`지만 `main`은 `/relay/viewer`다. 접속 ticket도 `main`에만 있다. `main` 빌드가 같은 버전을 출력할 수 있으므로 `--version`만으로 두 동작을 구분하지 않는다. 배포 출처와 커밋도 기록한다.

롤백할 때는 새 서버를 정상 종료하고 이전 실행파일과 그 버전에 맞는 설정을 함께 복원한다. Viewer 경로가 다른 버전 사이에서는 클라이언트 URL도 함께 되돌린다. 접속 ticket을 지원하지 않는 버전에는 `[relay.ticket.*]` 테이블을 남기지 않는다. v4에서 v3로 되돌릴 때는 보호한 v3 설정을 복원하는 방법을 권장한다. 전달 주소 검사가 없는 버전으로 되돌릴 때는 프록시 경유 접근을 중지하거나 직접 내장 WSS와 실제 소켓 IP 정책으로 먼저 바꿔야 한다. 프록시 IP를 모든 클라이언트 대신 허용해서는 안 된다.

복원 뒤 시작·준비·실제 중계와 관리 재로그인을 확인한다. 정확한 버전별 이관·복구는 [운영 how-to 3.2절·6절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md)을 따른다.

## 공개 배치 보안 점검

- 외부 관리 리스너는 내장 HTTPS를 써야 한다. 평문 HTTP는 루프백에만 둔다. 관리 리스너에는 중계용 프록시 TLS 종료 규칙을 적용하지 않는다.
- 출발지 IP 허용 목록을 역할별로 필요한 범위로 좁힌다. 빈 목록은 모두 거절한다. 전체 허용을 의도 없이 사용하지 않는다.
- 인터넷 공개 WebSocket에는 WSS 또는 중계용 프록시 TLS 종료를 권장한다. 불필요한 평문 WS 역할은 꺼 둔다. 프록시에서 VNC Relay까지의 구간도 보호해야 한다.
- 브라우저가 보내는 Origin을 역할별 `allowed_origins`와 맞춘다. Origin은 IP 검사나 사용자 인증을 대신하지 않는다.
- `trusted_proxies`에는 프록시 주소만 넣어야 한다. 클라이언트 범위를 넣지 않는다.
- Mode I의 `allow_targets`를 필요한 대상 IP·포트로 좁힌다.
- 계정 파일·개인키·설정·공개 키 파일과 로그를 보호한다. 공개 키도 변조되지 않도록 쓰기 권한을 제한한다.
- 설정 편집을 켰으면 서버 실행 OS 계정의 설정 파일 읽기·교체 권한과 상위 디렉터리의 임시 파일 생성·교체 권한을 확인한다. 다른 일반 OS 계정의 쓰기는 막는다.
- 접속 ticket만으로 해당 역할의 접근을 통제하려면 같은 역할의 TCP 진입점을 꺼야 한다. ticket은 TCP에 적용하지 않는다.

세부 기준은 [설정 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md), [운영 정보 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md)과 [PRD 확정 기준](https://github.com/kayden-kim/vnc-relay/blob/main/docs/requirements/PRD.md)을 따른다. 실패 원인별 복구는 [문제 해결](troubleshooting.md)을 참고한다.
