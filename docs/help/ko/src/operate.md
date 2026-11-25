# 운영

VNC Relay를 수동 또는 Windows 서비스로 시작하고 종료하며 상태, 로그와 설정 반영 결과를 확인할 수 있다. 인증서·공개 키·관리자 계정을 교체하고 업그레이드나 롤백을 준비할 수도 있다.

이 절차는 Windows 11 x64의 PowerShell과 Ubuntu 24.04 LTS x64의 bash를 대상으로 한다. 컨테이너는 범위에 포함하지 않는다. 명령은 배포 묶음을 푼 운영 디렉터리에서 실행한다. 먼저 [빠른 시작](quick-start.md)에 따라 설정과 관리자 계정을 준비해야 한다.

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

## Windows 서비스와 트레이

먼저 [서비스 설치](install.md#windows-서비스-설치와-해제)를 마친다. 설치 때 `--operator`로 지정한 사용자나 그룹은 관리자 권한 없이 시작·중지할 수 있다.

1. 일반 PowerShell에서 서비스를 시작한다.

   ```powershell
   sc.exe start vnc-relay
   ```

2. `sc.exe query vnc-relay`로 상태를 확인한다.
3. 중지할 때 일반 PowerShell에서 중지를 요청한다.

   ```powershell
   sc.exe stop vnc-relay
   ```

4. `sc.exe query vnc-relay`로 `STOPPED`가 될 때까지 기다린다.

Windows 서비스 관리 도구(`services.msc`)나 트레이로도 같은 요청을 할 수 있다. 서비스 중지는 G의 정상 종료 유예를 적용한다. 중지 중 두 번째 `stop`은 SCM이 거절하므로 콘솔의 두 번째 종료 요청처럼 유예를 건너뛰지 않는다. 다시 시작하려면 중지가 끝난 뒤 시작한다. `service run`을 터미널에서 직접 실행하지 않는다.

트레이를 쓰려면 일반 사용자로 `vnc-relay-tray.exe`를 실행하고 알림 영역의 아이콘을 클릭한다. 아이콘은 대시보드 상단 막대의 브랜드 표식과 같다. 어두운 둥근 사각형 안에 서로 반대 방향을 가리키는 흰 화살표 두 개가 있다. 그림은 [Windows 트레이 앱 스펙](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/windows-tray.md)의 공통 시각 기준을 따른다. 메뉴는 다음 순서다.

- 상태 줄: `VNC Relay: <상태>`다. 서비스와 `/readyz`의 상태를 보여 준다.
- `시작`: 중지된 서비스를 시작한다.
- `중지`: 확인 대화상자 없이 정상 종료를 요청한다.
- `대시보드 열기`: 준비됨 상태에서 설정의 관리 주소를 기본 브라우저로 연다.
- `로그 파일 위치 열기`: 파일 탐색기에서 로그 파일을 선택한다. 파일이 없으면 폴더를 연다.
- `설정 파일 위치 열기`: 파일 탐색기에서 등록된 설정 파일을 선택한다.
- `로그인할 때 시작`: 현재 사용자가 로그인할 때 트레이 앱을 실행한다. 기본은 꺼짐이다.
- `트레이 끝내기`: 아이콘과 트레이 프로세스만 끝낸다.

트레이 시작·종료는 서비스를 시작·중지하지 않는다. `트레이 끝내기`는 `로그인할 때 시작` 설정도 유지한다. 다시 시작 메뉴는 없다. 트레이 사용자는 설정 파일 읽기 권한이 필요하다. 계정 파일·개인키를 읽을 필요는 없다.

`실행 중(준비 안 됨)`은 서비스 프로세스는 실행 중이지만 `/readyz` 성공을 확인하지 못한 상태다. `중지됨 (종료 코드 N)`은 시작 실패 조사에 쓸 수 있다([문제 해결](troubleshooting.md#windows-서비스나-트레이에-오류가-나온다)). 상태·메뉴 켜짐의 전체 규칙은 [Windows 트레이 앱 스펙](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/windows-tray.md)을 따른다. 파일 권한과 서비스 교체 절차는 [운영 안내 3.3.1절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md#331-windows-서비스와-트레이-앱)에 있다.

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

상태 리스너에는 인증이 없다. 생존·준비 결과와 선택형 메트릭을 제공하며 세션 목록은 제공하지 않는다. 루프백 바인딩을 권장한다. 대시보드 사용법은 [대시보드](dashboard.md)를 따른다.

## 메트릭 수집

메트릭은 연결·세션·관리 활동과 프로세스 자원을 수로 나타낸 값이다. Prometheus나 OpenTelemetry Collector의 `prometheus` 수신기로 수집할 수 있다. VNC Relay는 OTLP로 직접 내보내지 않는다.

1. 외부 편집기로 설정 파일에 다음 값을 설정한다. `[metrics]` 테이블이 이미 있으면 그 안의 `enabled`만 바꾼다.

   ```toml
   [metrics]
   enabled = true
   ```

2. VNC Relay를 정상 종료한 뒤 다시 시작한다.
3. 수집 도구에서 상태 리스너의 `GET /metrics`를 읽도록 설정한다.

기본값은 `false`이며 이때 `/metrics`는 404다. 이 키는 재로드하거나 대시보드에서 편집할 수 없다. 꺼져 있어도 값은 계속 센다. 로그인한 관리자는 [메트릭 화면](dashboard.md#메트릭)과 관리 API `GET /api/v1/metrics`를 항상 쓸 수 있다.

> **주의:** `/metrics`에는 로그인이 필요 없으며 평문 HTTP를 쓴다. 상태 리스너는 루프백에 두거나 모니터링 네트워크에서만 접근할 수 있게 제한해야 한다. 수집 연결 하나는 관리·상태 연결 상한 Lm(`limits.admin_connections`)의 자리 하나를 차지한다. 수집 응답의 관리 연결 사용량에는 그 연결도 포함한다.

Prometheus 설정 파일의 `scrape_configs` 예시는 다음과 같다. 수집 도구와 VNC Relay가 같은 호스트에 있고 상태 리스너가 `127.0.0.1:18081`인 경우다. 15초는 이 예시의 수집 주기다.

```yaml
scrape_configs:
  - job_name: vnc-relay
    scrape_interval: 15s
    metrics_path: /metrics
    scheme: http
    static_configs:
      - targets: ["127.0.0.1:18081"]
```

OpenTelemetry Collector에서는 같은 호스트 조건으로 다음 `prometheus` 수신기를 설정한다. 기존 메트릭 파이프라인의 `service.pipelines.metrics.receivers`에도 `prometheus`를 추가한다. 내보내기 대상은 운영 중인 Collector 설정을 따른다.

```yaml
receivers:
  prometheus:
    config:
      scrape_configs:
        - job_name: vnc-relay
          scrape_interval: 15s
          metrics_path: /metrics
          scheme: http
          static_configs:
            - targets: ["127.0.0.1:18081"]
```

`/metrics`는 Prometheus 텍스트 0.0.4 형식으로 다음과 같은 값을 제공한다.

- `vnc_relay_session_count`, `vnc_relay_waiting_count`: 현재 활성 세션과 대기 연결 수다.
- `vnc_relay_connection_rejected_total`: 리스너·단계·원인별 연결 거절 누적 수다.
- `vnc_relay_session_io_bytes_total`: 방향별 전송량 누적 바이트 수다.
- `vnc_relay_handshake_duration_seconds`, `vnc_relay_session_duration_seconds`: 초기 교환과 세션 지속 시간의 분포다.
- `process_cpu_time_seconds_total`, `process_memory_usage_bytes`: 프로세스 CPU 시간과 물리 메모리 사용량이다.

전체 목록, OpenTelemetry 이름과 Prometheus 이름의 대응, 단위와 속성은 [메트릭 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/metrics-v1.md)을 따른다. 읽지 못한 프로세스 값의 표본은 빠지며 0으로 대신하지 않는다.

메트릭은 메모리에만 있다. 누적 수와 시간 분포는 VNC Relay를 재시작하면 0부터 다시 센다. 재로드는 값을 지우지 않는다. 장기 보관은 수집 도구에서 해야 한다. 메트릭에는 중계 ID, 세션 식별자, 주소와 계정명이 없다.

## 정상 종료와 재시작

1. VNC Relay를 실행한 터미널에서 Ctrl+C를 한 번 누른다.
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

콘솔 실행에서 유예 중 두 번째 종료 요청은 남은 유예를 건너뛰고 즉시 강제 정리한다. Ubuntu는 SIGINT(Ctrl+C)와 SIGTERM을 받는다. Windows는 Ctrl+C와 Ctrl+Break를 받으며 SIGTERM은 없다. 서비스 중지는 [Windows 서비스와 트레이](#windows-서비스와-트레이)를 따른다.

> **주의:** Windows 콘솔 창 닫기는 G를 기다리지 않고 처음부터 즉시 강제 정리한다. OS의 기본 5초 제한 안에 종료해야 한다. 일반 콘솔 운영 종료에는 Ctrl+C를 권장한다. 대화형 콘솔 실행은 Windows 로그오프·시스템 종료 때 정상 유예나 종료 기록을 보장하지 않는다. 서비스는 로그오프로 멈추지 않는다. 시스템 종료·재시작 처리는 [운영 스펙 3.7절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/operations.md#37-windows-서비스-실행)을 따른다. 작업 관리자 종료, SIGKILL과 전원 장애에서는 정상 종료를 보장하지 않는다.

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

VNC Relay는 이름 변경·삭제·생성 알림을 받으면 원래 경로를 다시 연다. 알림을 놓쳐도 `reload.poll_secs`마다 다시 연다. 다시 열기 전의 사건은 옛 파일에 남을 수 있다. 복사 후 원본 자르기(copytruncate)는 이 회전 절차로 쓰지 않는다.

다시 열기에 실패하면 이전 파일에 계속 쓴다. 실패가 시작될 때 `log_reopen_failed` 사건과 stderr 경고가 나온다. 경로와 권한을 복구한 뒤 다음 알림이나 주기 확인 결과를 확인한다. 로그 줄 쓰기 실패는 처음 한 번만 stderr의 `log_write_failed` 경고로 알린다. 실패한 줄은 버리고 서비스를 계속한다. 누락된 줄이 자동 복구된다고 기대하지 않는다.

회전 시점, 보관 기간과 접근 권한은 운영자 책임이다. 로그와 보관 파일을 승인된 운영자만 읽도록 제한하고 디스크 여유와 stderr를 감시하기를 권장한다. [대시보드의 최근 로그](dashboard.md)는 별도 메모리 목록이며 파일 보관을 대신하지 않는다.

## 파일 변경 자동 재로드

VNC Relay는 설정 파일과 사용 중인 TLS 파일, 접속 ticket 확인을 켠 역할의 공개 키 파일 변경을 자동 감지한다. 모든 역할이 꺼진 WSS 리스너의 TLS 파일은 감시하지 않는다. OS 알림을 주기 확인으로 보완한다. `reload.poll_secs`(Dp)와 `reload.settle_secs`(D)의 기본값은 각각 30초와 5초다. 파일 변경이 멈춘 상태라면 알림을 놓친 변경도 Dp + D 안에 적용·실패 결과를 확인한다. 연속 변경에서는 마지막 변경을 감지한 뒤 D 안에 안정된 최신 파일의 결과를 확인한다.

재로드 대상에는 출발지 IP·Origin·신뢰 프록시 정책, 출발지 IP 차단 목록, Mode I 대상 허용 정책, 연결 상한·시간 제한·keepalive, TLS 파일 경로와 내용, 접속 ticket 설정과 공개 키 파일이 있다. 키별 적용 시점은 [설정 계약 1.8절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md)을 따른다.

1. 보호된 위치에 현재 유효 파일의 복구본을 보관한다.
2. 외부 편집기로 필요한 재로드 대상 값만 바꾸어 저장한다.
3. 운영 로그에서 `reload_applied`를 확인한다.
4. 바꾼 정책이 적용될 새 연결이나 요청으로 결과를 확인한다.

이미 끝난 검사, 기존 연결, 시작한 타이머와 설정한 소켓 옵션은 이전 결과를 유지한다. 상한을 현재 점유보다 낮춰도 기존 연결을 끊지 않고 새 점유를 거절한다.

차단 목록 `relay.deny.sources`만 예외다. 목록이 바뀌면 그 주소의 열린 활성 세션과 대기 연결을 바로 끊는다. 끊은 연결마다 `session_closed`(`close_cause = ip_denied`) 또는 `waiting_removed`(`reason = ip_denied`)를 기록한다. 규칙은 [출발지 IP 차단 목록](configure.md#출발지-ip-차단-목록)을 따른다.

리스너 주소·포트, 중계 역할·모드 켜기/끄기, 관리 HTTP/HTTPS 전환, 계정·관리 세션 정책과 `management.config_edit` 등은 재시작해야 한다. 재시작 필요 값이 실행값과 다르면 후보 전체를 `reload_restart_required`로 거절한다. 그 차이가 남아 있으면 이후의 급한 인증서 교체도 막힌다.

후보 검증에 실패하면 `reload_rejected`를 기록한다. 감지 장애는 `reload_watch_failed`로 알린다. 실패하면 이전 유효 설정과 기존 중계를 유지한다. 수동 재로드 CLI나 웹 재로드 API는 없다. 파일 상태와 마지막 결과는 대시보드 "설정 보기"에서도 확인할 수 있다.

`[ip_info]`가 가리키는 IP 정보 DB 파일도 같은 방식으로 감지한다. 다만 설정 재로드의 대상이 아니다. 파일 내용이 바뀌면 그 파일만 다시 읽고 `ip_db_loaded` 또는 `ip_db_unavailable`을 기록한다. `reload_applied`는 남지 않는다. 절차는 [IP 정보 DB 준비와 갱신](#ip-정보-db-준비와-갱신)을 따른다.

## 인증서와 공개 키 파일 교체

### TLS 인증서와 개인키

TLS를 켠 배치에만 적용한다. 인증서와 개인키는 유효한 쌍이어야 한다. 개인키 내용을 로그나 명령행에 넣지 않는다.

1. 새 인증서 체인과 일치하는 개인키를 보호된 새 파일에 준비한다.
2. VNC Relay 실행 OS 계정이 두 파일을 읽을 수 있는지 확인한다.
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

## IP 정보 DB 준비와 갱신

대시보드 "접속 출처"의 국가와 ASN은 운영자가 둔 IP 정보 DB 파일에서 찾는다. IP 정보 DB는 IP 주소 범위마다 국가나 ASN을 담은 MMDB(MaxMind DB 형식) 파일이다. 설정하지 않아도 VNC Relay와 주소별 통계는 동작한다. VNC Relay는 IP 정보를 찾으려고 외부에 요청하지 않으며 DB 파일을 내려받지도 않는다. 배포 묶음에도 DB 파일은 없다.

지원하는 제공자는 다음과 같다. 이용 조건은 파일을 받은 운영자가 지켜야 한다. 받기 전에 제공자 페이지의 현재 조건을 확인한다.

| 제공자 | 파일 | 계정 | 이용 조건 |
| --- | --- | --- | --- |
| [DB-IP Lite](https://db-ip.com/db/lite.php) | 국가·ASN 파일 각각. 매월 나온다 | 필요 없음 | CC BY 4.0 |
| [IPinfo Lite](https://ipinfo.io/developers/ipinfo-lite-database) | 국가와 ASN을 함께 담은 `ipinfo_lite.mmdb` | 필요 | CC BY-SA 4.0 |
| [MaxMind GeoLite](https://dev.maxmind.com/geoip/geolite2-free-geolocation-data) | `GeoLite2-Country.mmdb`(또는 City), `GeoLite2-ASN.mmdb` | 필요 | [GeoLite EULA](https://www.maxmind.com/en/geolite/eula). 새 판이 나오면 바로 바꿔야 한다 |

대시보드는 파일의 DB 종류로 제공자를 판정하고 그 제공자가 요구하는 출처 표시를 보인다.

1. 운영 디렉터리에 `ipdb` 디렉터리를 만든다.
2. 제공자 사이트에서 MMDB 파일을 받는다. `.gz`·`.tar.gz`로 받았으면 압축을 풀어 `.mmdb` 파일을 `ipdb`에 둔다. VNC Relay는 압축 파일을 읽지 않는다. 파일은 256 MiB 이하여야 한다.
3. VNC Relay 실행 OS 계정이 파일을 읽을 수 있는지 확인한다.
4. VNC Relay를 정상 종료한다.
5. 설정 파일에 `[ip_info]`를 더한다. 상대 경로는 설정 파일의 디렉터리 기준이다.

   ```toml
   [ip_info]
   country_db = "ipdb/dbip-country-lite.mmdb"
   asn_db = "ipdb/dbip-asn-lite.mmdb"
   ```

   IPinfo Lite는 두 키에 같은 경로를 쓴다. VNC Relay는 이 파일을 한 번만 읽는다.

6. VNC Relay를 시작하고 운영 로그에서 `server_ready` 전에 키마다 `ip_db_loaded`가 남았는지 확인한다. 로그에는 경로 대신 `config_key`가 나온다.
7. 대시보드 "접속 출처"의 "IP 정보 DB"에서 두 행이 "사용 중"인지 확인한다.

파일을 읽지 못해도 VNC Relay는 시작하고 그 정보 없이 운영한다. `ip_db_unavailable` 경고의 `reason`으로 원인을 찾는다.

| `reason` | 확인할 내용 |
| --- | --- |
| `file_unreadable` | 경로, 파일 존재, 읽기 권한 |
| `file_too_large` | 파일이 256 MiB 이하인지 |
| `invalid_format` | 압축을 풀었는지, 복사가 끝났는지, 손상되지 않았는지 |
| `unsupported_data` | 키와 파일 종류가 맞는지. 예: 국가 키에 ASN 전용 파일 |

갱신은 같은 경로의 파일 내용을 바꾸는 방법으로 한다. 재시작은 필요 없다.

1. 새 파일을 같은 디렉터리에 임시 이름으로 둔다. 예: `ipdb/dbip-country-lite.mmdb.new`.
2. 임시 파일의 이름을 기존 파일 이름으로 바꿔 덮어쓴다. Windows PowerShell은 `Move-Item -Force`, Ubuntu는 `mv`를 쓴다. 같은 디렉터리 안의 이름 바꾸기는 한 번에 끝나므로 반쯤 복사된 파일을 읽지 않는다.
3. 운영 로그에서 `ip_db_loaded`(`phase = reload`)를 확인한다. 기본값으로는 35초(Dp + D) 안에 남는다.

새 파일을 읽지 못하면 이전 내용을 계속 쓰고 `ip_db_unavailable`(`phase = reload`)을 기록한다. 화면의 행은 "이전 DB 사용 중"이 된다. 정상 파일로 다시 바꾸면 자동으로 다시 읽는다. 이미 센 사건의 국가·ASN은 바뀌지 않는다. 경로를 바꾸려면 재시작해야 한다. 제공자별 내려받기와 압축 풀기 명령은 [수동 운영 안내 3.11절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md)을 따른다.

## 관리자 계정 관리

계정 추가·비밀번호 변경·삭제에는 로컬 CLI를 쓴다. 기본 계정과 기본 비밀번호는 없다. 명령은 설정의 `admin.accounts_file`을 사용한다.

1. VNC Relay를 정상 종료한다.
2. 필요한 명령 하나를 실행한다.
3. `admin list`로 남은 계정명을 확인한다.
4. VNC Relay를 다시 시작한다.
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

계정 파일 내용은 자동 재로드하지 않는다. 실행 중 파일만 바꾸면 이전 자격 증명과 기존 관리 세션이 재시작 전까지 유효하다. 마지막 계정을 지우면 VNC Relay가 시작하지 않는다. 계정 파일 접근 권한은 [CLI 계약 1.6절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md)을 따른다.

## 업그레이드와 롤백

재시작은 모든 연결과 관리 세션을 끝낸다. 승인된 중단 시간에 작업해야 한다.

1. 현재 실행파일·설정·계정 파일·TLS 파일과 사용하는 접속 ticket 공개 키 파일을 보호된 복구 위치에 보관한다. 실행파일 버전과 해시도 기록한다.
2. [Releases](https://github.com/kayden-kim/vnc-relay/releases)에서 대상 OS 묶음을 받아 해시를 확인한다. 설치 절차는 [설치](install.md)를 따른다.
3. 별도 디렉터리에 새 실행파일을 준비한다. 설정 사본은 운영 설정과 같은 디렉터리에 만든다. 상대 파일 경로의 기준을 유지하기 위해서다. 설정 계약이 바뀌면 사본을 먼저 이관한다.
4. 설정 사본의 켠 리스너들을 서로 다른 미사용 루프백 포트로 바꾼다. 로그는 `stdout`으로 바꾼다. 운영 계정·TLS 파일은 수정하지 않는다.
5. 새 실행파일과 그 사본으로 시작한다. `server_ready`와 상태 리스너 `/readyz`의 200을 확인한다.
6. 시험 프로세스를 Ctrl+C로 정상 종료한다.
7. 중단 시간에 기존 VNC Relay를 정상 종료한다.
8. 새 실행파일과 검증한 **운영 주소 설정**을 운영 위치에 설치한다. 대체 포트 사본을 설치하지 않는다.
9. 시작·상태·관리 재로그인·실제 중계·재로드를 확인한다.

별도 설정 검증 전용 CLI는 없다. 대체 포트 시작은 운영 포트, 방화벽과 실제 클라이언트 중계를 대신 검증하지 않는다. 소스 빌드와 배포 묶음은 [운영 how-to](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md)의 상세 절차를 따른다.

> **주의:** WS·WSS 뷰어는 `/relay/viewer`로 접속해야 한다. `/relay`를 쓰는 클라이언트가 있으면 VNC Relay와 모든 뷰어 URL을 함께 옮긴다. TCP와 `/relay/server`는 바꾸지 않는다. `main` 빌드는 다음 릴리스 전까지 최신 Release와 같은 버전을 출력하므로 `--version`만으로 둘을 구분하지 않는다. 배포 출처와 커밋도 기록한다.

롤백할 때는 새 VNC Relay를 정상 종료하고 이전 실행파일과 그 버전에 맞는 설정을 함께 복원한다. 뷰어 경로가 다른 버전 사이에서는 클라이언트 URL도 함께 되돌린다. 접속 ticket을 지원하지 않는 버전에는 `[relay.ticket.*]` 테이블을 남기지 않는다. 접속 출처를 지원하지 않는 버전에는 `[ip_info]` 테이블을 남기지 않는다. v4에서 v3로 되돌릴 때는 보호한 v3 설정을 복원하는 방법을 권장한다. 전달 주소 검사가 없는 버전으로 되돌릴 때는 프록시 경유 접근을 중지하거나 직접 내장 WSS와 실제 소켓 IP 정책으로 먼저 바꿔야 한다. 프록시 IP를 모든 클라이언트 대신 허용해서는 안 된다.

복원 뒤 시작·준비·실제 중계와 관리 재로그인을 확인한다. 정확한 버전별 이관·복구는 [운영 how-to 3.2절·6절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md)을 따른다.

## 공개 배치 보안 점검

- 외부 관리 리스너는 내장 HTTPS를 써야 한다. 평문 HTTP는 루프백에만 둔다. 관리 리스너에는 중계용 프록시 TLS 종료 규칙을 적용하지 않는다.
- 출발지 IP 허용 목록을 역할별로 필요한 범위로 좁힌다. 빈 목록은 모두 거절한다. 전체 허용을 의도 없이 사용하지 않는다.
- 문제가 있는 출처는 출발지 IP 차단 목록(`relay.deny.sources`)으로 막는다. 차단 목록은 허용 목록보다 우선한다. 빈 목록은 아무것도 차단하지 않는다. 허용 목록의 빈 목록과 뜻이 반대다. 차단하면 그 주소의 열린 세션과 대기 연결이 바로 끊긴다.
- 인터넷 공개 WebSocket에는 WSS 또는 중계용 프록시 TLS 종료를 권장한다. 불필요한 평문 WS 역할은 꺼 둔다. 프록시에서 VNC Relay까지의 구간도 보호해야 한다.
- 브라우저가 보내는 Origin을 역할별 `allowed_origins`와 맞춘다. Origin은 IP 검사나 사용자 인증을 대신하지 않는다.
- `trusted_proxies`에는 프록시 주소만 넣어야 한다. 클라이언트 범위를 넣지 않는다. 차단 목록의 항목과 겹치면 설정 오류다.
- Mode I의 `allow_targets`를 필요한 대상 IP·포트로 좁힌다.
- 계정 파일·개인키·설정·공개 키 파일과 로그를 보호한다. 공개 키도 변조되지 않도록 쓰기 권한을 제한한다.
- 설정 편집을 켰으면 VNC Relay 실행 OS 계정의 설정 파일 읽기·교체 권한과 상위 디렉터리의 임시 파일 생성·교체 권한을 확인한다. 다른 일반 OS 계정의 쓰기는 막는다.
- 접속 ticket만으로 해당 역할의 접근을 통제하려면 같은 역할의 TCP 진입점을 꺼야 한다. ticket은 TCP에 적용하지 않는다.
- 접속 출처의 ipinfo.io 조회 링크를 고르면 그 주소나 ASN이 외부 사이트로 간다. 외부 조회가 허용되는지 운영 정책을 확인한다. IP 정보 DB 파일은 다른 일반 OS 계정이 바꾸지 못하게 한다.

세부 기준은 [설정 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md), [운영 정보 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md)과 [PRD 확정 기준](https://github.com/kayden-kim/vnc-relay/blob/main/docs/requirements/PRD.md)을 따른다. 실패 원인별 복구는 [문제 해결](troubleshooting.md)을 참고한다.
