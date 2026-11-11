# VNC Relay 용어집

VNC Relay 제품 전체를 하나의 맥락으로 보고, [PRD](docs/requirements/PRD.md)·스펙·ADR·계약·코드가 함께 쓰는 업무 용어를 정의한다. 용어의 정의는 기능의 채택 상태나 구현 완료를 뜻하지 않는다.

## 용어

### 제품과 역할

**VNC Relay(vnc-relay)**:
이 프로젝트가 만드는 VNC 중계 제품이다. 실행파일 이름도 `vnc-relay`다.
_Avoid_: 중계기, 중계 서버

**UltraVNC Repeater**:
호환 대상인 기존 중계 프로그램이다. VNC Relay와 구분한다.

**Viewer(viewer)**:
원격 화면을 받고 사용자 입력을 보내는 VNC 클라이언트 역할이다.

**Server(server)**:
원격 화면을 제공하고 사용자 입력을 처리하는 VNC 서버 역할이다. VNC Relay와 구분한다.

**역할(role)**:
중계 연결에서 Viewer 또는 Server가 맡는 쪽이다. 전송 방식이나 리스너와 구분한다.

**역접속**:
Mode II에서 Server가 VNC Relay에 먼저 연결해 오는 방식이다.

### 중계 방식과 세션

**Mode I(mode1)**:
Viewer가 지정한 VNC Server 주소로 VNC Relay가 연결하는 중계 방식이다.

**Mode II(mode2)**:
Viewer와 Server가 각각 VNC Relay에 연결하고 같은 중계 ID로 짝을 이루는 중계 방식이다.

**Mode I 대상(target)**:
Mode I에서 Viewer가 지정해 VNC Relay가 연결하는 VNC Server다. VNC Relay가 이 대상으로 여는 연결을 대상 연결이라 한다.

**중계 ID(relay_id)**:
Mode II에서 Viewer와 Server의 짝을 찾는 값이다. 접속 권한을 증명하는 비밀값과는 다르다.
_Avoid_: 매칭 ID, 인증 ID, 인증 토큰

**초기 교환(handshake)**:
접속 직후 중계 방식과 대상 주소 또는 중계 ID를 정하는 Repeater 호환 절차다.
_Avoid_: 핸드셰이크(Repeater의 이 절차를 가리킬 때)

**대기 연결(waiting)**:
Mode II에서 짝이 되는 상대 연결을 기다리는 연결이다.

**활성 세션(session)**:
Viewer와 Server 또는 Mode I 대상 사이의 중계가 시작되어 아직 정리되지 않은 연결 쌍이다.

**세션 식별자(session_id)**:
중계 중인 한 연결 쌍을 다른 연결 쌍과 구분하는 값이다. 중계 ID와 구분하며, 관리자 종료의 대상을 가리킨다.

**연결 식별자(conn_id)**:
중계 리스너가 접속을 수락할 때 연결마다 붙이는 값이다. 세션 식별자와 다르다.

**중계 스트림(relay_stream)**:
Repeater의 초기 교환 절차가 처리된 뒤 양 끝점 사이에 전달하는 바이트 흐름이다. VNC 인증과 화면 데이터의 해석은 양 끝점이 맡는다.

**Viewer 전송 경로(viewer_transport)**:
Viewer가 들어온 전송 방식(TCP·WS·WSS)이다.

**전송량(bytes_viewer_to_target, bytes_target_to_viewer)**:
활성 세션에서 Viewer → Server/Mode I 대상 방향과 그 반대 방향으로 나누어 센 바이트 수다. 세는 기준은 [관리 스펙](docs/specs/management.md#21-전송량의-의미)이 정한다.

**끊긴 연결(dead_connection)**:
상대가 응답하지 않아 끊어진 것으로 판정한 대기·활성 연결이다. 데이터가 오가지 않아도 상대가 응답하는 유휴 연결과 다르다.

### 접근 경계

**리스너(listener)**:
접속을 받도록 바인드한 소켓 하나다. 중계 리스너는 TCP Viewer·TCP Server·WS·WSS이며, 관리·상태 리스너와 구분한다.

**진입점(entry_point)**:
중계 연결이 들어오는 리스너와 역할의 조합이다(TCP Viewer·TCP Server·WS Viewer·WS Server·WSS Viewer·WSS Server). TCP 리스너는 역할이 고정되며, WS·WSS 리스너는 `/relay/viewer`와 `/relay/server` 경로로 역할을 구분한다.
_Avoid_: 진입 경로, 리스너(진입점을 가리킬 때)

**출발지 IP(peer_ip)**:
접속을 수락한 소켓에서 확인한 실제 상대 IP다. 전달 헤더가 알려 주는 주소가 아니다.

**신뢰 프록시(trusted_proxy)**:
WS·WSS 리스너의 `trusted_proxies`에 IP가 속하는 소켓 상대다. 이 상대가 보낸 `X-Forwarded-For`가 전달 주소의 근거다.

**전달 주소(forwarded_ip)**:
신뢰 프록시의 `X-Forwarded-For`에서 구한 클라이언트 IP다. 소켓에서 확인한 출발지 IP와 구분한다.

**클라이언트 주소(client_address)**:
WS·WSS 업그레이드 때 역할별 출발지 IP 허용 목록과 대조하는 주소다. 신뢰 프록시를 거쳤으면 전달 주소이며, 나머지는 소켓 상대 IP다.

**출발지 IP 허용 목록(allow)**:
진입점마다 중계 접속을 허용할 IP를 적은 목록이다. WS·WSS에서는 클라이언트 주소와 대조하며, 항목은 단일 IP·CIDR·범위(`a-b`) 중 하나다.

**전체 허용 항목(allow_all)**:
출발지 IP 허용 목록에서 IPv4나 IPv6 주소 전체를 덮는 항목이다(`0.0.0.0/0`, `::/0`).

**대상 허용 정책(allow_targets)**:
Mode I에서 VNC Relay가 연결할 수 있는 대상의 주소와 포트를 설정으로 적은 목록이다. 출발지 IP 허용 목록과 별개다.
_Avoid_: 목적지 허용 정책

**접속 ticket(ticket)**:
발급 쪽이 서명해 WS·WSS 업그레이드 요청에 싣는 일회용 접속 허가 값이다. 한 역할과 한 중계 ID에 묶이며, 설정으로 켠 역할에서만 확인한다. 중계 ID나 관리 세션과 다르다.
_Avoid_: 접속 토큰, 티켓, 인증 토큰

**발급 쪽(issuer)**:
접속 ticket에 서명하는 외부 서비스다. 서명 키는 발급 쪽만 가지며, VNC Relay는 역할마다 설정한 공개 키로 서명을 확인한다.

**사용 기록**:
VNC Relay가 받아들인 접속 ticket의 `jti`를 만료 뒤까지 메모리에 남긴 목록이다. 같은 ticket의 두 번째 사용을 거절하는 근거다.
_Avoid_: 재사용 캐시, replay cache

**세션 만료 시각(sx)**:
접속 ticket이 알려 주는 시각이다. 이 시각이 되면 VNC Relay는 그 ticket으로 연 연결을 끝낸다.

**중계 측 연결(relay_connections)**:
진입점이 수락한 연결과 Mode I 대상 연결이다. 관리·상태 연결과 구분한다.

**중계 연결 상한 거절(relay_connection_limit)**:
Mode I에서 진입점 연결 수락 후 대상 호스트에 연결하려 할 때 동시 중계 연결 상한 L에 도달하여 연결을 거절하는 일이다. WebSocket에서는 4011 close 코드로 전달한다.
_Avoid_: 대상 연결 한도 초과, 연결 상한 거절
**관리·상태 연결(admin_connections)**:
관리 리스너와 상태 리스너가 수락한 연결이다. 중계 측 연결과 구분한다.

### 관리 기능

**관리 리스너(management)**:
대시보드와 관리 API를 제공하는 리스너다. 진입점과 상태 리스너와 다르다.

**상태 리스너(status)**:
서버의 생존·준비 결과(`/livez`, `/readyz`)만 응답하는 리스너다. 관리 리스너와 다르다.

**대시보드(dashboard)**:
관리 리스너가 제공하는 내장 웹 화면이다. 관리자가 서버·연결·세션·설정을 보고 활성 세션 종료와 동적 설정 편집을 하는 곳이다.
_Avoid_: 관리 화면, 관리 대시보드

**관리 API(management_api)**:
관리 리스너가 제공하는 JSON over HTTP API다. 경로는 `/api/v1/` 아래에 있다.

**설정 보기**:
실행 중 유효 설정과 설정 파일의 반영 상태를 확인하는 관리 기능이다.

**동적 설정 편집**:
관리자가 편집 대상 키의 값을 바꾸어 설정 파일에 남기고 재시작 없이 실행 값에 반영하는 관리 기능이다.

**관리자 계정(admin_account)**:
관리자가 대시보드와 관리 API에 로그인할 때 쓰는 계정이다. 관리자 계정 준비로 만든다.

**관리자 계정 준비(admin)**:
서버를 시작하기 전에 로컬 CLI의 `vnc-relay admin` 하위 명령으로 관리자 계정을 만들고 고치는 일이다.
_Avoid_: 관리자 준비

**관리자 계정 파일(accounts_file)**:
관리자 계정의 이름과 비밀번호 해시를 담은 TOML 파일이다. 관리자 계정 준비가 쓰고 서버가 읽는다.

**관리 세션(admin_session)**:
관리자 로그인으로 발급하는 대시보드·관리 API 접근 상태다. 활성 세션과 다르다.
_Avoid_: 로그인 세션, 관리자 세션, 세션(단독으로 관리자 로그인을 가리킬 때)

**관리 세션 쿠키(relay_admin_session)**:
브라우저가 관리 세션을 가리키려고 보내는 불투명 식별자 쿠키다.
_Avoid_: 관리 쿠키

**로그인 제한(login_limit)**:
관리 로그인 실패가 계정별 또는 출발지 IP별 기준 F를 넘었을 때 일정 시간 로그인을 거절하는 상태다.
_Avoid_: 로그인 차단, 잠금

**CSRF 토큰(csrf_token)**:
관리 세션마다 하나씩 발급하는 값이다. 상태를 바꾸는 요청(POST·DELETE·PATCH)에 헤더 `X-CSRF-Token`으로 보낸다.
_Avoid_: CSRF 키, 보안 토큰

### 세션 종료와 기록

**관리자 종료(admin_terminated)**:
관리자가 세션 식별자로 활성 세션 하나를 지정해 끝내는 일이다. 종료 원인의 하나다.
_Avoid_: 세션 강제 종료, 강제 종료(세션을 가리킬 때), 세션 종료(단독으로 이 일을 가리킬 때)

**세션 만료(session_expired)**:
접속 ticket의 세션 만료 시각이 되어 VNC Relay가 대기 연결이나 활성 세션을 끝낸 종료 원인이다. 관리자 종료와 구분한다.

**서버 종료(server_shutdown)**:
서버가 내려갈 때 아직 정리를 시작하지 않은 활성 세션을 강제로 정리해 끝난 종료 원인이다. 강제 정리 전에 이미 정리를 시작한 세션은 최초 원인을 유지한다. 서버 프로세스를 끝내는 일 자체인 정상 종료와 구분한다.

**종료 원인(close_cause)**:
정리가 끝난 활성 세션이 왜 끝났는지를 구분한 값이다. 정상 EOF(`peer_eof`), 연결 오류(`connection_error`), 관리자 종료(`admin_terminated`), 세션 만료(`session_expired`), 끊긴 연결 감지(`dead_connection`), 정리 제한 시간 초과(`close_timeout`), 서버 종료(`server_shutdown`), 내부 오류(`internal_error`) 중 하나다.
_Avoid_: 종료 사유, 종료 이유

**최근 종료 목록(recent_closed)**:
정리가 끝난 활성 세션의 요약과 종료 원인을 담은 목록이다.

**운영 로그(log)**:
VNC Relay가 표준 출력이나 지정한 파일로 내보내는 JSON Lines 기록이다.

**최근 로그(recent_logs)**:
운영 로그 사건의 최근 N개를 메모리에 보관한 목록이다. 대시보드의 SCR-006 로그 화면에서 확인하는 기록이며 장기 로그 저장소와 구분한다.

**사건(event)**:
운영 로그의 한 줄이 기록하는 일이다. 이름은 `event` 필드에 쓴다.
_Avoid_: 이벤트

### 설정과 수명

**부분 재로드(reload)**:
서버를 재시작하지 않고 재로드 대상 설정과 TLS 인증서·개인키, 접속 ticket 공개 키 파일을 실행 중에 교체하는 설정 반영이다.

**재로드 대상(dynamic)**:
재시작 없이 실행 값을 바꿀 수 있는 설정 키다. 범위는 [설정 계약](docs/contracts/config-v4.md#18-재로드-구분)이 정한다.

**편집 대상**:
재로드 대상 가운데 관리 API와 대시보드로 값을 바꿀 수 있는 설정 키다. 인증서·개인키 경로, 접속 ticket 공개 키 파일 경로와 편집 기능 켜기는 포함하지 않는다.

**편집 기능 켜기(config_edit)**:
동적 설정 편집을 쓸 수 있게 하는 운영 설정이다. 설정 보기와는 별개다.

**재로드 후보(reload_candidate)**:
파일 변경 감지나 관리 API 편집으로 읽거나 만든 새 설정 묶음이다. 실행 값으로 교체하기 전에 검증한다.
_Avoid_: 변경 후보

**재시작 필요(reload_restart_required)**:
재로드 대상이 아닌 값이 실행 중 값과 달라 재로드 후보 전체를 거절하는 재로드 결과다.

**정상 종료(shutdown)**:
종료 요청을 받은 서버가 종료 유예를 두고 끝나는 종료 방식이다. 프로세스를 즉시 죽이는 강제 종료와 구분한다.

**종료 유예(shutdown_grace)**:
정상 종료 때 활성 세션이 끝나기를 서버가 기다려 주는 구간이다. 서버 요약의 실행 상태 `shutdown_grace`가 이 구간이고, 최대 길이는 설정 `shutdown_grace_secs`(G)다.

**시작 실패(startup_failed)**:
설정이나 필수 리스너 준비가 잘못되어 서버가 준비됨 상태에 들어가지 못하고 끝나는 일이다.
