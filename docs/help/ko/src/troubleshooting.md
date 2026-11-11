# 문제 해결

증상별로 로그와 상태를 확인하고 원인에 맞게 복구할 수 있다. 해결되지 않으면 비밀을 제외한 정보를 모아 보고할 수 있다.

먼저 문제가 생긴 단계를 구분한다. 프로세스 시작 실패, 관리자 로그인 실패, VNC 중계 접속 거절과 활성 세션 종료는 서로 다르다. HTTP 상태 코드, WebSocket close 코드와 CLI 종료 코드도 서로 다른 값이다. 숫자만 보지 말고 진단 코드와 운영 로그 사건을 함께 확인한다.

## 서버가 시작하지 않는다

서버를 실행한 터미널의 stderr에서 `error[<code>]`와 `key`를 확인한다. 로그의 `startup_failed`도 확인한다. 설정을 읽기 전이나 로그 파일을 열지 못한 시작 실패는 표준 출력에 기록될 수 있다.

명령이 돌아온 같은 터미널에서 종료 코드를 읽는다.

Windows:

```powershell
$LASTEXITCODE
```

Ubuntu:

```shell
echo $?
```

### 계정이 없거나 계정 파일이 잘못되었다

`accounts_file_missing`이나 `accounts_none`이면 관리자 계정이 준비되지 않은 상태다. 종료 코드는 5다. [빠른 시작](quick-start.md)에 따라 `admin add`로 계정을 준비한다. 기본 계정이나 기본 비밀번호는 없다.

계정 파일을 읽지 못하거나 형식이 잘못되어도 종료 코드 5가 나온다. `admin.accounts_file` 경로와 읽기 권한을 확인한다. 계정 파일을 임의의 해시나 평문 비밀번호로 고치지 않는다. 해시 하한 문제는 [CLI 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md)에 따라 `admin passwd`로 복구한다.

### 포트가 사용 중이거나 두 번째 인스턴스다

`listener_bind_failed`와 종료 코드 4이면 표시된 `listen` 주소에 바인드하지 못한 것이다. 해당 포트를 쓰는 프로세스와 설정한 주소를 확인한다. 이미 실행 중인 서버를 새로 시작한 것은 아닌지도 확인한다.

같은 리스너 주소로 시작한 두 번째 프로세스는 실패한다. 이 실패는 첫 프로세스의 대기 연결과 활성 세션을 교체하지 않는다. 기존 서버를 의도 없이 종료하지 않는다. 설정 안의 리스너 주소가 정확히 중복되면 바인드 전 `config_listen_conflict`로 거절할 수 있으므로 진단 코드를 구분한다.

### 설정·TLS·공개 키·로그 파일 오류다

종료 코드 3이면 `config_*`, `tls_file_invalid`, `ticket_keys_invalid`나 `log_file_unavailable` 진단을 확인한다.

1. 진단의 `key`가 가리키는 설정을 확인한다.
2. 파일 경로를 설정 파일 디렉터리 기준으로 확인한다.
3. 서버 실행 OS 계정의 읽기 권한을 확인한다. 로그에는 상위 디렉터리와 쓰기 권한도 필요하다.
4. TLS 인증서와 개인키의 형식 및 일치 여부를 확인한다.
5. 진단을 고친 뒤 다시 시작한다.

평문 관리 HTTP를 루프백 아닌 주소에 두면 `config_plaintext_management_not_loopback`으로 시작하지 않는다. 외부 관리는 내장 HTTPS로 구성하거나 관리 HTTP를 루프백에 둬야 한다.

종료 코드 2이면 사용법과 대화형 입력 조건을 확인한다. 예를 들어 `--config` 누락은 사용법 오류다. 종료 코드 1은 변경 감지 준비나 런타임 등 실행 환경 실패에도 쓰인다. 전체 의미와 안정적인 진단 코드는 [CLI 계약 2절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md)을 따른다.

## 관리자 로그인이 되지 않는다

잘못된 비밀번호, 없는 계정과 로그인 제한은 모두 `401 login_failed`로 응답한다. 화면 안내만으로 원인을 구분하지 않는다. 운영 로그의 `admin_login_failed`와 `admin_login_limited`를 확인한다.

- 로그인 제한 중이면 올바른 비밀번호도 거절한다. 설정한 제한 시간이 지난 뒤 다시 시도한다. 같은 출발지의 다른 계정도 출발지 제한을 받을 수 있다.
- 계정을 추가하거나 비밀번호를 바꾼 뒤 재시작하지 않았다면 이전 계정 집합을 쓰고 있다. 서버를 정상 종료하고 다시 시작한다.
- 평문 HTTP 외부 접근은 지원하지 않는다. 서버의 관리 리스너가 내장 HTTPS인지 확인한다. 프록시에서만 HTTPS로 바꾸어 이 제한을 우회하지 않는다.

비밀번호를 잊었으면 서버를 정상 종료한 뒤 운영 디렉터리의 대화형 터미널에서 다음 명령을 실행한다. `opsadmin`은 복구할 실제 계정명으로 바꾼다.

Windows:

```powershell
.\vnc-relay.exe admin passwd opsadmin --config .\relay.toml
```

Ubuntu:

```shell
./vnc-relay admin passwd opsadmin --config ./relay.toml
```

새 비밀번호를 숨김 입력으로 두 번 입력한다. 서버를 다시 시작한 뒤 로그인한다. 비밀번호를 명령행에 넣지 않는다. 재시작 전의 관리 세션도 재시작으로 무효화된다.

관리 세션 유휴 만료나 절대 최대 수명이 지나면 다시 로그인해야 한다. 자동 폴링은 유휴 만료를 연장하지 않는다. 자세한 수명과 로그아웃 동작은 [대시보드](dashboard.md)를 따른다.

## 중계 접속이 거절된다

운영 로그의 `connection_rejected`에서 `stage`, `reason`, 리스너와 역할을 확인한다. 아직 활성 세션이 되지 못한 연결은 "최근 종료 목록"에 나오지 않는다.

### 출발지 IP가 허용되지 않는다

`ip_not_allowed`이면 해당 진입점의 `allow`를 확인한다. TCP는 실제 소켓 출발지 IP를 검사한다. WS·WSS는 접속 수락 때 켠 역할 목록의 합집합 또는 신뢰 프록시 여부를 검사한다. 이 단계에서 실패하면 HTTP 응답 없이 닫을 수 있다. 업그레이드 직전에는 클라이언트 주소를 해당 역할의 목록과 비교하며 실패하면 HTTP 403이다.

프록시를 쓰면 `peer_ip`와 `forwarded_ip`를 구분한다. `forwarded_for_invalid`와 HTTP 400이면 `trusted_proxies`와 프록시의 `X-Forwarded-For` 전달을 확인한다. 프록시 주소만 신뢰해야 한다. 모든 전달 주소가 신뢰 프록시 범위인 요청도 거절한다. 접근을 되게 하려고 프록시 IP를 모든 클라이언트 대신 허용하지 않는다. 설정 방법은 [설정](configure.md)을 따른다.

### Origin이 맞지 않는다

`origin_not_allowed`와 HTTP 403이면 브라우저가 보낸 Origin과 그 역할의 `allowed_origins`를 비교한다. 정규화한 스킴·호스트·포트가 같아야 한다. 스킴·호스트의 ASCII 대소문자는 구분하지 않으며 기본 포트는 생략해도 같다. 와일드카드는 없다. 빈 목록은 Origin이 있는 요청을 모두 거절한다. Origin이 없으면 이 비교만 생략한다. IP 검사와 켠 역할의 접속 ticket 확인은 유지한다. 정확한 정규화 규칙은 [설정 계약 1.4절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md#14-origin-값-문법과-비교)을 따른다.

### WS·WSS 경로가 틀리거나 역할이 꺼져 있다

HTTP 404이면 URL 경로와 역할의 `enabled`를 확인한다. `main`의 Viewer 경로는 `/relay/viewer`, Server 경로는 `/relay/server`다. 끝에 `/`를 붙이지 않는다. 꺼진 역할도 404다.

최신 Release `v2026.11.2`의 Viewer 경로는 `/relay`다. 이 도움말은 `main` 기준이며 이전 경로를 함께 받지 않는다. `main` 빌드도 버전 `2026.11.2`를 출력할 수 있다. 버전 문자열뿐 아니라 묶음 출처나 빌드 커밋을 확인해야 한다. 서버와 모든 Viewer URL을 함께 옮긴다.

`connection_rejected`의 `reason = invalid_upgrade`는 다른 잘못된 요청에도 쓰인다. 로그에 요청 경로가 없으므로 이 원인만으로 이전 URL 사용을 단정하지 않는다. [VNC 연결 구성](connect.md)과 [운영](operate.md)의 이관 안내를 따른다.

### 같은 중계 ID와 역할이 이미 점유되어 있다

`duplicate_id`이면 같은 역할이 같은 중계 ID로 이미 대기 중이거나 활성 세션이 사용 중이다. 후발 TCP 연결은 FIN으로 닫고 WS·WSS는 close 4004로 거절한다. 기존 연결은 교체하지 않는다.

클라이언트가 이중 실행되었는지 확인한다. 새 연결에 다른 중계 ID가 필요하면 Viewer와 Server 양쪽을 같은 새 값으로 맞춘다. 기존 활성 세션을 끝내야 한다면 대시보드에서 대상을 확인하고 관리자 종료한다. 중계 ID는 비밀번호나 접속 권한 증명이 아니다.

### Mode I 대상이 허용되지 않는다

`target_not_allowed`와 close 4003이면 `mode1.allow_targets`를 확인한다. 대상 IP와 포트를 허용해야 한다. DNS 이름으로 요청하면 이름·포트 허용과 실제 연결할 최종 IP·포트 허용이 모두 필요하다. 이름만 추가해서는 충분하지 않다.

`target_connect_failed`와 close 4002는 별개다. 주소 해석, 대상의 연결 거절이나 연결 시간 초과를 운영 로그에서 구분하고 대상까지의 연결을 확인한다. 필요한 대상만 허용하며 전체 주소 허용으로 우회하지 않는다.

### 연결이나 대기 상한에 도달했다

대시보드 "중계 연결"과 "대기 연결"을 확인한다. 새 접속 수락에서 중계 연결 상한에 걸리면 HTTP 응답 없이 닫을 수 있다. **close 4011**(`relay_connection_limit`)은 Mode I에서 업그레이드 101 이후 대상 연결용 소켓 자리가 없는 경우다. 모든 상한 거절이 4011인 것은 아니다.

close 4010(`waiting_full`)이면 새 대기 등록 상한에 걸린 것이다. 중복 클라이언트와 불필요한 연결을 먼저 확인하고 배치 용량에 맞게 상한을 검토한다. 관리·상태 연결은 별도 상한이며 `connection_rejected`의 `reason = admin_connection_limit`으로 구분한다.

### 접속 ticket이 HTTP 401로 거절된다

이 기능은 `main`에만 있으며 기본은 꺼져 있다. 켠 역할에서는 유효한 접속 ticket이 필수다. `connection_rejected`의 ticket 관련 `reason`을 확인한다. 없음, 형식·서명·클레임 불일치, 만료, 재사용과 기동 시각 조건 위반을 구분한다. `ticket_issued_before_start`는 `iat`가 서버 기동 시각 + 5초 이하일 때다. 기동 직후 발급한 ticket도 이 조건에 걸릴 수 있다.

발급 쪽과 서버의 시계, 역할별 설정과 공개 키를 확인한다. 재시작 뒤에는 새 ticket이 필요하다. 같은 ticket을 다시 보내지 말고 발급 쪽에서 새 값을 받아야 한다. 관리자 로그인을 해도 중계용 ticket 실패는 해결되지 않는다.

101 뒤 close 4012(`ticket_relay_mismatch`)이면 초기 요청의 중계 ID를 ticket의 `rid`와 바이트 단위로 맞춘다. `0123`과 `123`은 다르다. 기존 모드·초기 요청 검사를 통과한 Viewer Mode I 요청도 4012로 거절한다. Mode I이 꺼져 있으면 `mode_disabled`(1008)가 먼저다. 전체 조건은 [접속 ticket](access-ticket.md)을 따른다. ticket 원문을 로그나 보고서에 붙이지 않는다.

HTTP 거절과 close 코드의 전체 기준은 [WebSocket 계약 1.5절·2절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md)을 따른다.

## 활성 세션이 예상보다 일찍 끝난다

"최근 종료 목록"의 종료 원인과 같은 `session_id`의 `session_closed` 사건을 확인한다. 세션 상세의 "로그 보기"로 관련 사건을 좁힐 수 있다. 최근 목록과 로그는 메모리 보관이므로 이미 지워졌으면 파일 로그를 확인한다.

- `peer_eof`는 양쪽이 정상 EOF로 끝난 경우다. 클라이언트가 연결을 끝냈는지 확인한다.
- `connection_error`이면 같은 사건의 `error_kind`와 앞선 연결 오류를 확인한다. 네트워크 재설정이나 WS 프로토콜·메시지 오류를 구분한다.
- `dead_connection`은 응답 없는 상대를 keepalive나 ping으로 감지한 경우다. 네트워크와 클라이언트 응답을 확인한다. 응답하는 유휴 연결은 화면 데이터가 없다는 이유만으로 끊지 않는다.
- `admin_terminated`이면 `admin_terminate` 사건에서 수행 계정을 확인한다. WS·WSS에는 close 4005를 보낸다.
- `session_expired`이면 접속 ticket의 세션 만료 시각 `sx`에 도달한 것이다. WS·WSS에는 close 4013을 보낸다. 양쪽에 만료 시각이 있으면 이른 시각에 끝낸다. 발급 쪽의 세션 수명 정책을 확인한다.
- `close_timeout`이면 `timeouts.close_secs`(E) 안에 반대 방향 전달이나 종료 교환을 마치지 못했다. 상대 응답과 네트워크를 확인한다. 관리자 종료에서도 정리 시간이 지나면 이 원인이 될 수 있다.
- `server_shutdown`이면 종료 유예 만료나 즉시 강제 정리 등 서버 종료 과정을 확인한다. `shutdown_started`와 `shutdown_forced`를 함께 찾는다.
- `internal_error`이면 서버 내부 버그로 세션 소유 작업이 끝났을 수 있다. 전후 로그와 배포 버전을 보존해 보고한다. close 1011은 예약 코드이며 현재 구현이 보내지 않는다.

클라이언트가 표시한 1006만으로 원인을 단정하지 않는다. 1006은 서버가 보내는 close 코드가 아니다. pong 제한 초과나 내부 오류처럼 Close 없이 소켓을 닫는 경우도 있다.

아직 대기 중이었다면 대기 시간 만료는 `waiting_expired`, 그 밖의 대기 제거는 `waiting_removed`에서 찾는다. `connection_aborted`는 대기 등록이나 세션 시작 전에 끝난 연결의 기록이다. close 4001은 Mode II 상대가 대기 시간 안에 오지 않은 경우다. 활성 세션 종료 원인과 혼동하지 않는다. 상세 대응은 [운영 정보 계약 1.5·1.6절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md)과 [관리 API 계약 1.10절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/management-api-v1.md)을 따른다.

## 재로드가 거절된다

`reload_restart_required`이면 재시작 필요 값이 실행값과 다르다. `config_keys`를 확인한다. 그 차이를 외부 편집기로 실행값으로 되돌리거나 승인된 시간에 서버를 재시작한다. 차이가 남으면 이후 동적 변경과 인증서 교체도 후보 전체를 거절한다.

`reload_rejected`이면 설정·TLS·공개 키 검증 진단에 따라 파일을 고친다. 실패한 후보를 일부만 적용하지 않으며 이전 유효 설정을 유지한다. `reload_watch_failed`는 감지 장애이므로 파일 접근과 감시 환경을 확인한다.

대시보드 "설정"의 "설정 파일 상태"와 "마지막 재로드"를 확인한다. 자동 폴링하지 않으므로 "새로고침"을 선택한다. 상태 `pending`은 적용 성공이 아니다. 수정 뒤 `reload_applied`와 실행 중 값을 확인한다. 수동 재로드 명령은 없다.

## 설정을 편집할 수 없다

- `config_edit_disabled`이면 외부 편집기로 `management.config_edit = true`로 바꾼 뒤 재시작·재로그인한다. 기본값은 꺼짐이다.
- `config_restart_pending`이면 먼저 재시작 필요 차이를 되돌리거나 재시작한다. `config_file_invalid`이면 원본 파일의 읽기·검증 문제를 외부에서 고친다.
- `config_conflict`이면 다른 관리자나 외부 편집기가 파일을 바꾼 것이다. 주석·공백 변경도 충돌한다. "다시 불러오기"로 입력을 버리고 최신 설정을 읽은 뒤 새로 편집한다. 옛 요청을 자동 재전송하지 않는다.
- `config_write_failed`이면 이번 요청은 파일과 실행값을 바꾸지 않았다. 서버 실행 OS 계정의 파일 읽기·교체 권한, 상위 디렉터리의 임시 파일 생성·교체 권한과 디스크 여유를 확인한다. Windows에서는 교체를 막는 열린 파일 핸들도 확인한다. Ubuntu에서는 디렉터리 write+search 권한과 읽기 전용 마운트도 확인한다.
- 인증서·개인키·공개 키 경로나 재시작 필요 키에는 입력란이 없다. 꺼진 역할·리스너·Mode I에 속한 키도 편집하지 못한다.

시간 초과나 응답 유실은 성공 여부가 불명이다. "설정 보기"에서 현재 값·파일 상태·최근 결과를 먼저 조회한다. 확인 전에는 다시 적용하지 않는다. 자세한 절차는 [대시보드](dashboard.md)와 [운영 how-to 3.5.1절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md)을 따른다.

## 최근 로그가 비어 있다

`recent_logs.max_entries = 0`이면 메모리에 보관하지 않는다. 보관하려면 설정 파일에서 1~10000으로 바꾸고 서버를 재시작해야 한다. 기본값은 1000이다.

보관이 켜져 있으면 수준·사건 이름·검색어와 세션 필터를 확인한다. 재시작은 이전 보관 목록을 지운다. 오래된 사건도 상한에 따라 지운다. 세션 시작 사건이 지워졌으면 연결별 관련 로그 일부가 세션 필터에 포함되지 않을 수 있다. 필요하면 보관한 운영 로그 파일을 확인한다.

조회 실패를 빈 결과로 판단하지 않는다. "오래된 데이터"와 마지막 성공 시각을 확인한다. 최근 로그는 파일 출력 성공 여부와 독립적이므로 화면만 보고 파일 로그가 정상이라고 판단하지 않는다.

## 해결되지 않을 때 보고할 정보

1. OS 버전과 아키텍처를 기록한다.
2. 실행파일 버전을 기록한다.
3. 배포 묶음 출처나 빌드 커밋과 실행파일 해시를 기록한다.
4. 실패한 명령, 종료 코드, HTTP 상태나 close 코드와 재현 순서를 기록한다.
5. 발생 시각과 시간대를 적고 전후 운영 로그를 수집한다.
6. 문제에 필요한 설정 사본에서 비밀과 불필요한 식별 정보를 제거한다.

버전 명령은 운영 디렉터리에서 실행한다.

Windows:

```powershell
.\vnc-relay.exe --version
```

Ubuntu:

```shell
./vnc-relay --version
```

> **주의:** 비밀번호·해시·개인키·서명 키·접속 ticket 원문, 관리 세션 쿠키와 CSRF 토큰을 보내지 않는다. 계정 파일 본문과 공개 키 파일 내용도 붙이지 않는다. 원시 VNC 화면 데이터나 사용자 입력을 수집하지 않는다. 불필요한 주소와 관리자 계정명은 가린다.

보고는 배포 운영 책임자나 프로젝트 관리자에게 전달한다. 로그 사건·필드 기준은 [운영 정보 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md)을 따른다. 복구 절차는 [운영](operate.md)을 참고한다.
