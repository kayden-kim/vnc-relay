# 참조

이 페이지로 CLI, HTTP·WebSocket 경로, 설정 테이블, 종료 원인, 운영 로그 사건과 기준 계약을 빠르게 찾을 수 있다.

## CLI 명령

아래 표는 실행파일 뒤에 붙이는 인수다. Windows PowerShell에서는 `.\vnc-relay.exe`를 실행한다. Ubuntu bash에서는 실행파일 디렉터리에서 `./vnc-relay`를 실행한다.

| 인수 | 하는 일 |
| --- | --- |
| `serve --config <path>` | 서버를 포그라운드로 실행한다. |
| `admin add <account> --config <path>` | 관리자 계정을 추가한다. 비밀번호와 확인 값을 숨김 입력으로 받는다. |
| `admin passwd <account> --config <path>` | 기존 계정의 비밀번호를 바꾼다. 새 비밀번호와 확인 값을 숨김 입력으로 받는다. |
| `admin remove <account> [--yes] --config <path>` | 계정을 삭제한다. `--yes`는 삭제 확인을 생략한다. |
| `admin list --config <path>` | 관리자 계정 이름을 이름순으로 한 줄에 하나씩 출력한다. |
| `--help` 또는 `-h` | 사용법을 출력한다. 다른 인수와 함께 쓰지 않는다. |
| `--version` 또는 `-V` | 실행파일 버전을 출력한다. 다른 인수와 함께 쓰지 않는다. |

`--config`는 `serve`와 모든 `admin` 명령에서 필수다. 비밀번호는 명령행·환경 변수로 받지 않는다. 계정 변경은 실행 중 서버에 반영되지 않는다. 정상 종료하고 다시 시작해야 한다.

압축을 푼 묶음의 실행파일 디렉터리에서, 준비한 설정 파일이 `relay.toml`일 때 다음과 같이 실행한다.

Windows PowerShell:

```powershell
.\vnc-relay.exe serve --config .\relay.toml
```

Ubuntu bash:

```shell
./vnc-relay serve --config ./relay.toml
```

종료 코드는 성공 `0`, 실행 중 오류 `1`, 사용법 오류 `2`, 설정·TLS·ticket 공개 키·로그 파일 준비 오류 `3`, 리스너 바인드 실패 `4`, 계정 파일 오류 `5`로 구분한다. 각 코드의 정확한 범위와 오류 이름은 [CLI 계약 1.1·1.7·2절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md)을 따른다. 실제 시작·종료 절차는 [운영](operate.md)을 따른다.

## HTTP 엔드포인트

관리 리스너, 상태 리스너, 중계 WS·WSS 리스너는 서로 다르다. 같은 포트에서 모든 경로를 제공하지 않는다.

| 리스너 | 경로 | 용도 |
| --- | --- | --- |
| 상태 | `GET /livez` | 프로세스가 응답할 수 있으면 `200`과 `status: alive`를 반환한다. |
| 상태 | `GET /readyz` | 준비됨이면 `200`과 `status: ready`를 반환한다. 준비되지 않았으면 `503`과 `status: not_ready`를 반환한다. |
| 관리 | `/` | 내장 대시보드를 제공한다. |
| 관리 | `/api/v1/` 아래 | 관리자 로그인·로그아웃, 서버·연결·세션·설정·로그 조회, 관리자 종료와 동적 설정 편집 API다. |

상태 확인은 인증이 없다. 상태 리스너는 평문 HTTP다. 관리 리스너는 HTTPS가 원칙이며 평문 HTTP는 루프백에서만 허용한다. 관리 API의 개별 경로·인증·CSRF 요구는 [관리 API 계약 1.2절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/management-api-v1.md)을 따른다. 생존과 준비의 차이와 전체 응답 형식은 [운영 정보 계약 1.12절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md)을 따른다.

## WebSocket 경로

| 경로 | 역할 |
| --- | --- |
| `/relay/viewer` | Viewer. ticket을 켜지 않았으면 Mode I·II를 받는다. |
| `/relay/server` | Server. Mode II만 받는다. |

WS와 WSS는 같은 경로를 쓴다. 쿼리는 대상이나 중계 ID를 선택하지 않는다. 꺼진 역할과 다른 경로는 HTTP 404다. 끝 슬래시는 다른 경로다. `main`은 이전 Viewer 경로 `/relay`도 404로 거절한다.

> **참고:** `v2026.11.2` 이하 Release는 Viewer 경로 `/relay`를 쓴다. 버전 차이는 [소개](index.md), 클라이언트 구성은 [VNC 연결 구성](connect.md)을 따른다.

업그레이드와 종료 코드 전체는 [WebSocket 계약 1.1·1.5·1.7절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md)을 따른다. ticket을 켠 역할의 추가 규칙은 [접속 ticket](access-ticket.md)을 따른다.

## 설정 테이블 지도

아래 표는 위치를 찾기 위한 지도다. 키 전체 목록·기본값·유효 범위는 [설정 계약 1.2절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md)을 따른다. 작성 방법과 부분 재로드 구분은 [설정](configure.md)을 따른다.

| 테이블 | 용도 |
| --- | --- |
| `[admin]` | 관리자 계정 파일 경로 |
| `[admin.argon2]` | 새 비밀번호 해시의 매개변수 |
| `[relay.tcp_viewer]` | TCP Viewer 진입점 |
| `[relay.tcp_server]` | TCP Server 진입점 |
| `[relay.ws]` | 평문 WS 공유 리스너와 신뢰 프록시 |
| `[relay.wss]` | WSS 공유 리스너, 신뢰 프록시와 TLS 파일 |
| `[relay.ws.viewer]` | WS Viewer 역할의 활성화·IP·Origin 정책 |
| `[relay.ws.server]` | WS Server 역할의 활성화·IP·Origin 정책 |
| `[relay.wss.viewer]` | WSS Viewer 역할의 활성화·IP·Origin 정책 |
| `[relay.wss.server]` | WSS Server 역할의 활성화·IP·Origin 정책 |
| `[relay.ticket.viewer]` | WS·WSS Viewer의 접속 ticket 확인 |
| `[relay.ticket.server]` | WS·WSS Server의 접속 ticket 확인 |
| `[mode1]` | Mode I 활성화와 대상 허용 정책 |
| `[mode2]` | Mode II 활성화 |
| `[limits]` | 연결·대기·버퍼·WebSocket 메시지 상한 |
| `[timeouts]` | 단계별 제한 시간과 종료 유예 |
| `[keepalive]` | TCP keepalive와 WebSocket ping/pong |
| `[management]` | 관리 리스너, TLS 파일, 편집 기능 켜기 |
| `[management.login_limit]` | 계정별·출발지별 로그인 실패 제한 |
| `[management.session]` | 관리 세션 유휴 만료와 절대 최대 수명 |
| `[dashboard]` | 조회 주기와 조회 제한 시간 |
| `[recent_closed]` | 최근 종료 목록의 보관 크기 |
| `[recent_logs]` | 최근 로그의 메모리 보관 크기 |
| `[status]` | 상태 리스너 주소 |
| `[log]` | 운영 로그 출력 위치 |
| `[reload]` | 파일 변경 결과 확인 제한 시간과 주기 확인 간격 |

## 종료 원인

종료 원인은 정리가 끝난 활성 세션이 왜 끝났는지 나타낸다. 아래 화면 문구는 대시보드가 표시하는 원문이다.

| 값 | 화면 문구 |
| --- | --- |
| `peer_eof` | "정상 EOF" |
| `connection_error` | "연결 오류" |
| `admin_terminated` | "관리자 종료" |
| `session_expired` | "세션 만료" |
| `dead_connection` | "끊긴 연결 감지" |
| `close_timeout` | "정리 제한 시간 초과" |
| `server_shutdown` | "서버 종료" |
| `internal_error` | "내부 오류" |

이 값은 WebSocket close 코드나 접속 거절의 `reason` 전체 목록과 다르다. 대기 연결은 활성 세션이 아니므로 대기 만료를 최근 종료 목록에서 찾지 않는다. 값과 의미의 기준은 [관리 API 계약 1.10절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/management-api-v1.md)이다. 로그의 수준과 추가 필드는 [운영 정보 계약 1.6절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md)을 따른다. 화면 문구의 기준은 [대시보드 스펙 9절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/dashboard.md)과 [copy.js](https://github.com/kayden-kim/vnc-relay/blob/main/web/dashboard/app/copy.js)다.

## 자주 확인할 운영 로그 사건

운영 로그는 JSON Lines이며 `event`로 사건을 구분한다. 아래는 자주 확인할 일부다. 전체 목록과 필드는 [운영 정보 계약 1.5~1.6절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md)을 따른다.

| 사건 | 확인할 내용 |
| --- | --- |
| `server_ready`, `startup_failed` | 시작 성공 여부와 실패 원인 |
| `connection_rejected` | 거절한 단계와 `reason` |
| `waiting_registered`, `waiting_expired`, `waiting_removed` | Mode II 대기 등록, 대기 시간 만료, 그 밖의 원인으로 제거 |
| `session_started`, `session_closed` | 세션 시작·종료. 종료 원인은 `session_closed.close_cause`에 있다. |
| `ticket_accepted` | ticket 확인과 사용 기록 쓰기를 통과했는지 |
| `target_connect_failed` | Mode I 대상 주소 해석·연결 실패 |
| `reload_applied`, `reload_rejected`, `reload_restart_required`, `reload_watch_failed` | 부분 재로드 성공·실패·재시작 필요·감지 장애 |
| `config_edited`, `config_edit_rejected` | 동적 설정 편집 결과 |
| `admin_login_failed`, `admin_login_limited`, `admin_terminate` | 관리 로그인 실패·제한과 관리자 종료 결과 |
| `allow_all_source`, `trust_all_proxies`, `allow_all_target` | 넓은 접근 정책 경고 |
| `ticket_tcp_bypass`, `ticket_plaintext_ws` | ticket 확인이 적용되지 않는 TCP와 평문 WS 노출 경고 |
| `log_reopen_failed` | 교체한 로그 파일을 다시 열지 못했는지 |
| `shutdown_started`, `shutdown_forced`, `server_stopped` | 종료 시작, 남은 세션 강제 정리, 종료 완료 |

로그 연결 방법과 대응 절차는 [운영](operate.md)과 [문제 해결](troubleshooting.md)을 따른다.

## main이 구현한 계약 버전

이 표는 제품 Release 번호가 아니라 외부 형식의 계약 버전이다. 실행파일의 `--version`만으로 `main`의 추가 기능을 구분할 수는 없다. [소개](index.md)의 Release 차이를 함께 확인한다.

| 계약 이름 | 버전 | 기준 문서 |
| --- | --- | --- |
| `vnc-relay-config` | 4.1.0 | [설정 파일 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md) |
| `ops-info` | 2.1.0 | [운영 정보 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md) |
| `websocket-relay` | 2.1.0 | [WebSocket 중계 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md) |
| `vnc-relay-cli` | 1.7.0 | [CLI·계정 파일 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md) |
| `management-api` | 1.1.0 | [관리 API 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/management-api-v1.md) |

## 기준 문서 찾기

- 요구사항: [PRD](https://github.com/kayden-kim/vnc-relay/blob/main/docs/requirements/PRD.md). 1절의 확정 기준과 미확정 제안을 구분한다.
- 스펙: [중계](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/relay.md), [WS·WSS](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/websocket.md), [운영](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/operations.md), [관리](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/management.md), [대시보드](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/dashboard.md).
- 계약: [설정](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md), [CLI·계정 파일](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md), [WebSocket](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md), [운영 정보](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md), [관리 API](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/management-api-v1.md), [Repeater 초기 교환](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/repeater-handshake-v1.md).
- 호환성 근거: [Repeater 호환성 검증 기록](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/repeater-compat-record.md).
- 운영 절차: [수동 운영 안내](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md), [릴리스 안내](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/release.md), [소스 빌드·검증](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/set-up-and-verify.md), [UltraVNC 수동 확인](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/verify-ultravnc-manually.md).
- 설정 예시: [examples/relay.toml](https://github.com/kayden-kim/vnc-relay/blob/main/examples/relay.toml). 모든 설정 키를 담는다.
- 용어: [GLOSSARY.md](https://github.com/kayden-kim/vnc-relay/blob/main/GLOSSARY.md).
- 배포 묶음: [GitHub Releases](https://github.com/kayden-kim/vnc-relay/releases).
