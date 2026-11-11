# 설정

이 페이지로 TOML 설정의 역할을 이해하고, 접속 정책과 리스너를 구성하고, 재시작 없이 바꿀 수 있는 값을 구분할 수 있다.

## 설정 파일 읽기

설정은 UTF-8 TOML 파일 하나로 관리한다. 서버를 시작할 때 `--config`로 파일을 지정한다. 설정용 환경 변수나 명령행 값 덮어쓰기는 없다.

상대 경로는 실행 위치가 아니라 설정 파일이 있는 디렉터리를 기준으로 한다. Windows 경로는 `/` 또는 TOML 작은따옴표 문자열을 쓰기를 권장한다. 시간은 정수 초, 크기는 정수 바이트로 적는다. 알 수 없는 키나 잘못된 자료형은 오류다.

[배포 예시](https://github.com/kayden-kim/vnc-relay/blob/main/examples/relay.toml)를 출발점으로 쓸 수 있다. 아래 예시는 필요한 부분만 보여 준다. 완전한 파일을 대신하지 않는다. 키별 필수 여부·기본값·범위는 [설정 계약 1.1~1.2절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md)을 따른다.

| 테이블 | 제어하는 내용 |
| --- | --- |
| `[admin]`, `[admin.argon2]` | 관리자 계정 파일과 새 비밀번호 해시의 매개변수 |
| `[relay.tcp_viewer]`, `[relay.tcp_server]` | 역할이 고정된 TCP 리스너와 출발지 IP 허용 목록 |
| `[relay.ws]`, `[relay.wss]` | 공유 리스너 주소, 신뢰 프록시, WSS TLS 파일 |
| `[relay.ws.viewer]`, `[relay.ws.server]`, `[relay.wss.viewer]`, `[relay.wss.server]` | 역할별 활성화, 출발지 IP 허용 목록, Origin 허용 목록 |
| `[relay.ticket.viewer]`, `[relay.ticket.server]` | 역할별 선택형 접속 ticket 확인 |
| `[mode1]`, `[mode2]` | 중계 모드 활성화와 Mode I 대상 허용 정책 |
| `[limits]` | 연결 수, 대기 수, 버퍼, WebSocket 메시지 상한 |
| `[timeouts]` | 초기 교환, 업그레이드, 대기, 대상 연결, 정리, 종료 유예 시간 |
| `[keepalive]` | TCP keepalive와 WebSocket ping/pong |
| `[management]`, `[management.login_limit]`, `[management.session]` | 관리 리스너, 편집 기능 켜기, 로그인 제한, 관리 세션 수명 |
| `[dashboard]` | 대시보드 조회 주기와 조회 제한 시간 |
| `[recent_closed]`, `[recent_logs]` | 메모리에 보관할 최근 종료 목록과 최근 로그의 크기 |
| `[status]` | 생존·준비 확인용 상태 리스너 |
| `[log]` | 운영 로그 출력 위치 |
| `[reload]` | 파일 변경 감지의 결과 확인 제한 시간과 주기 확인 간격 |

## 리스너와 역할

TCP Viewer와 TCP Server는 각각 별도 리스너를 쓴다. `[relay.tcp_viewer]`와 `[relay.tcp_server]`에서 `enabled`, `listen`, `allow`를 정한다. `listen`은 IP 주소를 써야 한다. 호스트 이름은 받지 않는다. IPv6 주소에 포트를 붙일 때는 대괄호로 묶는다.

WS와 WSS는 리스너 하나에서 두 역할을 함께 받는다. `[relay.ws]` 또는 `[relay.wss]`에 `listen`을 둔다. 하위 역할 테이블에서 역할마다 켠다. 역할이 하나라도 켜지면 리스너가 열린다.

- `/relay/viewer`: Viewer 역할이다. ticket을 켜지 않았으면 Mode I·II를 받는다.
- `/relay/server`: Server 역할이다. Mode II만 받는다.

두 역할은 같은 주소·포트를 쓰지만 접근 정책은 따로 둔다. WS·WSS의 `listen`에는 포트를 반드시 적어야 한다. [VNC 연결 구성](connect.md)에서 클라이언트 구성을 확인할 수 있다.

> **주의:** 이 도움말은 `main` 기준이다. 최신 Release `v2026.11.2`의 Viewer 경로는 `/relay`이며, `main`은 `/relay/viewer`를 쓴다. Release에는 접속 ticket 확인이 없다. 버전 차이는 [소개](index.md)를 따른다.

## 출발지 IP 허용 목록

각 진입점의 `allow`는 단일 IP, CIDR, 양 끝을 포함한 범위 `a-b`를 받는다. IPv4와 IPv6를 쓸 수 있다. 항목 안에 공백을 넣으면 안 된다.

```toml
[relay.tcp_viewer]
enabled = true
listen = "127.0.0.1:5901"
allow = ["127.0.0.1", "192.0.2.0/24", "198.51.100.10-198.51.100.20", "2001:db8::/32"]
```

- 켠 진입점에서 `allow`를 생략하면 시작 실패다.
- `allow = []`는 모든 출발지를 거절한다.
- `0.0.0.0/0`과 `::/0`은 허용할 수 있지만 시작·부분 재로드 때 `allow_all_source` 경고를 남긴다.
- CIDR의 호스트 비트는 0이어야 한다. 범위는 시작 주소가 끝 주소 이하여야 하며 두 주소의 체계가 같아야 한다.

TCP는 실제 소켓 상대 IP를 검사한다. WS·WSS는 수락 때 신뢰 프록시 여부 또는 켠 역할의 `allow` 합집합을 검사한다. 업그레이드 때는 선택한 역할의 `allow`를 클라이언트 주소와 비교한다. 신뢰 프록시를 거쳤으면 전달 주소를 쓴다.

전체 문법은 [설정 계약 1.3절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md)을 따른다.

## Origin 허용 목록

각 WS·WSS 역할의 `allowed_origins`는 브라우저 페이지의 출처를 제한한다. `scheme://host[:port]` 형식만 받는다. 스킴은 `http` 또는 `https`다. 경로, 끝의 `/`, 쿼리, 와일드카드를 넣으면 안 된다.

```toml
[relay.wss.viewer]
enabled = true
allow = ["192.0.2.0/24"]
allowed_origins = ["https://viewer.example.test"]
```

스킴·호스트는 대소문자를 구분하지 않는다. 기본 포트의 생략과 명시는 같은 값이다. 그 밖에는 정규화한 스킴·호스트·포트가 모두 같아야 한다. 하위 도메인은 다른 호스트다.

`allowed_origins = []`이면 Origin이 있는 요청을 모두 거절한다. Origin이 없으면 이 비교만 생략한다. IP 검사와 켠 역할의 ticket 확인은 유지한다. Origin은 사용자 인증 수단이 아니다. 세부 비교는 [설정 계약 1.4절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md)을 따른다.

## TLS 파일과 신뢰 프록시

WSS 역할을 하나라도 켜면 `[relay.wss]`에 `cert_file`과 `key_file`이 모두 있어야 한다. 관리 HTTPS도 `[management]`에 같은 두 키를 쓴다.

`cert_file`은 PEM 인증서 체인이다. 첫 인증서는 서버 인증서이고, 뒤에 중간 인증서를 둔다. `key_file`은 그 인증서와 맞는 PEM 개인키다. PKCS#8을 권장한다. 개인키 내용은 설정 파일에 넣지 않는다. 서버 실행 계정만 읽도록 권한을 제한하기를 권장한다. 파일 형식은 [설정 계약 1.9절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md)을 따른다.

중계 WS·WSS 앞에서 프록시가 TLS를 종료할 수 있다. 해당 리스너의 `trusted_proxies`에는 프록시 IP만 넣어야 한다. 클라이언트 범위를 넣으면 안 된다. 문법은 `allow`와 같다. 빈 목록이면 전달 헤더를 신뢰하지 않는다.

신뢰 프록시가 보낸 `X-Forwarded-For`는 오른쪽부터 읽는다. 첫 비신뢰 주소를 전달 주소로 정하고 역할의 `allow`와 비교한다. 헤더가 없거나 비었거나, 검사한 요소가 잘못됐거나, 모든 요소가 신뢰 프록시이면 요청을 거절한다. 실제 소켓 상대 IP는 기록에 그대로 남는다.

> **주의:** `trusted_proxies`를 넓게 잡으면 클라이언트가 전달 주소를 제시할 권한까지 얻는다. 전체 주소 항목은 `trust_all_proxies` 경고를 남긴다. 신뢰 프록시 IP에서 직접 접속해도 유효한 전달 헤더가 필요하다.

이 지원은 중계 WS·WSS에만 적용한다. 관리 리스너는 전달 헤더를 무시한다. 프록시를 두어도 관리 HTTPS·루프백 HTTP 규칙은 바뀌지 않는다. 운영자는 프록시와 VNC Relay 사이 구간을 보호해야 한다. 세부 규칙은 [WebSocket 계약 1.8절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md)과 [ADR-0007](https://github.com/kayden-kim/vnc-relay/blob/main/docs/decisions/ADR-0007-proxy-tls-termination.md)을 따른다.

## Mode I 대상 허용 정책

`mode1.allow_targets`는 VNC Relay가 연결할 대상과 포트를 제한한다. 출발지 IP 허용 목록과 별개다. Mode I를 켰는데 이 키가 없으면 시작 실패다. 빈 배열이면 모든 대상을 거절한다.

```toml
[mode1]
enabled = true
allow_targets = ["192.0.2.20:5900", "server.example.test:5900"]
```

IP로 지정한 대상은 IP·포트 항목과 일치해야 한다. DNS 이름으로 지정한 대상은 이름·포트와 최종 연결 IP·포트가 각각 허용되어야 한다. 위 예시의 이름이 `192.0.2.20`으로 해석되면 두 조건을 만족한다. 다른 IP로 해석되면 그 IP를 별도로 허용해야 한다.

대상·포트 범위와 IPv6 표기는 [설정 계약 1.5절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md)을 따른다. 임의 대상에 접속하는 공개 프록시가 되지 않도록 정책을 좁혀야 한다.

## 상한과 시간, 연결 유지

`[limits]`는 자원 사용량을 제한한다. `relay_connections`는 수락한 중계 연결과 Mode I 대상 연결을 센다. `admin_connections`는 관리·상태 연결을 하나의 공통 상한으로 제한하며 중계 연결과 따로 센다. `waiting`은 Mode II 대기 수를 제한한다. `buffer_bytes`와 `ws_message_bytes`는 각각 연결당 중계 버퍼와 WebSocket 논리 메시지의 상한이다.

`[timeouts]`는 끝나지 않는 절차가 자원을 계속 차지하지 않도록 한다. 초기 교환, TLS·HTTP 업그레이드, Mode II 대기, Mode I 대상 연결, 종료 교환, 정상 종료 유예를 각각 조절한다.

`[keepalive]`는 응답 없는 연결을 찾는다. TCP keepalive는 WS·WSS 하부 TCP에도 적용한다. WS·WSS에는 ping/pong 검사도 적용한다. 데이터가 없어도 상대가 응답하는 유휴 연결은 유지한다. 활성 세션의 유휴 시간을 제한하는 기능과는 다르다.

수치를 바꾸기 전 [설정 계약 1.2절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md)의 기본값·범위·값 관계를 확인해야 한다.

## 관리·상태 리스너와 기록

관리 리스너는 대시보드와 관리 API를 제공한다. HTTPS가 원칙이다. `management.cert_file`과 `management.key_file`을 모두 생략하면 평문 HTTP다. 이때는 루프백에만 바인드해야 한다. 루프백에서도 관리자 로그인은 필요하다.

상태 리스너는 평문 HTTP로 `/livez`와 `/readyz`만 제공한다. 인증은 없다. 세션·중계 ID·상대 주소는 응답하지 않는다. 루프백을 쓰기를 권장한다. 배포 예시는 관리 `127.0.0.1:18080`, 상태 `127.0.0.1:18081`을 쓴다. 기본 주소를 뜻하지 않는다.

`log.output`은 `"stdout"` 또는 파일 경로다. 운영 로그는 JSON Lines다. 파일에는 이어 쓴다. 운영자가 파일을 교체하면 같은 경로를 다시 연다. 회전과 장기 보관은 운영자 책임이다.

`recent_logs.max_entries`는 별도 메모리 목록의 크기다. `0`이면 보관하지 않는다. 재시작하면 목록이 빈다. 로그 출력 위치나 출력 성공 여부와는 무관하다. 최근 로그는 장기 저장소가 아니다. `[recent_closed]`는 정리가 끝난 활성 세션의 최근 종료 목록을 제어한다.

[운영](operate.md)에서 상태 확인과 로그 보관 절차를 확인할 수 있다.

## 부분 재로드와 동적 설정 편집

서버는 설정 파일과 사용 중인 TLS 파일, 켠 ticket 표의 공개 키 파일 변경을 자동 감지한다. 재로드 대상은 43개다. IP·Origin·신뢰 프록시 정책, TLS 파일, ticket 설정, Mode I 대상 허용 정책, 상한·시간·연결 유지 값이 포함된다. 정확한 목록과 적용 시점은 [설정 계약 1.8절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md)을 따른다.

새 값은 키별 다음 검사·요청부터 적용한다. 이미 통과한 검사, 시작한 타이머, 설정한 소켓 옵션은 유지한다. 기존 중계 세션은 계속 동작한다.

대시보드의 "설정 보기"는 실행 중 유효 설정과 파일 상태를 보여 준다. `management.config_edit = true`로 바꾸고 재시작하면 "동적 설정 편집"을 쓸 수 있다. 기본은 꺼짐이다. 편집은 같은 TOML 파일에 저장하고 실행 값에 반영한다. 실행 계정에는 파일 쓰기와 같은 디렉터리의 임시 파일 생성·교체 권한이 필요하다.

편집 대상은 37개다. 재로드 대상에서 TLS 경로 4개와 ticket 공개 키 파일 경로 2개를 제외한다. 이 경로는 보기에만 나온다. 인증서·공개 키 갱신은 같은 경로의 파일 내용을 교체한다. 역할·모드에 속한 키는 계약에서 정한 활성화 조건도 만족해야 편집할 수 있다. 화면 절차는 [대시보드](dashboard.md)를 따른다.

> **주의:** 유효한 설정 파일에 재시작이 필요한 값의 차이가 하나라도 남아 있으면 파일 감지는 후보 전체를 `reload_restart_required`로 거절한다. 그 차이를 남기면 이후 인증서·공개 키 교체도 거절한다. 동적 설정 편집은 HTTP 409 `config_restart_pending`으로 거절한다([관리 API 계약 1.14절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/management-api-v1.md)). 해당 값을 되돌리거나 서버를 정상 종료하고 다시 시작해야 한다.

리스너 주소, 리스너·역할·모드 활성화, 관리 HTTP·HTTPS 전환, 계정 설정, 관리 로그인 제한·관리 세션 수명, `management.config_edit`, 대시보드·기록 보관·로그·감지 설정 등은 재시작이 필요하다. ticket 표의 `enabled`는 예외로 재로드 대상이다. 후보 검증 실패는 `reload_rejected`로 기록하고 이전 유효 설정과 연결을 유지한다.

## v3에서 v4로 옮기기

v4는 이전 `[relay.ws_viewer]`와 `[relay.wss_viewer]`를 받지 않는다. 리스너 설정과 역할 설정을 나누어야 한다.

1. 실행 중인 파일과 별도로 사본을 준비한다.
2. [설정 계약 3.3절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md)의 대응표대로 사본을 고친다.
3. 서버를 정상 종료한다.
4. v4 파일과 새 실행파일로 함께 교체한다.
5. 새 설정으로 서버를 시작한다.

Viewer 경로도 실행파일의 WebSocket 계약에 맞춰야 한다. `main`은 `/relay/viewer`다. 롤백도 같은 계약 절의 절차를 따른다. 접속 ticket 구성은 [접속 ticket](access-ticket.md)을 따른다.
