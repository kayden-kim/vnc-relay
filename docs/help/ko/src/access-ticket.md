# 접속 ticket

이 페이지로 WS·WSS 역할에 접속 ticket 확인을 켜고, 발급 쪽과 클라이언트의 준비 사항을 정하고, 공개 키를 교체할 수 있다.

> **참고:** 접속 ticket 확인은 `main`에만 있다. 최신 Release `v2026.11.2`에는 없다. 기본은 꺼짐이다. [소개](index.md)에서 기준 버전을 확인한다.

## 접속 ticket이 하는 일

접속 ticket은 외부 발급 쪽이 서명한 일회용 접속 허가 값이다. VNC Relay는 역할별 공개 키 파일로 서명을 확인한다. 켠 역할의 WS·WSS 업그레이드는 유효한 ticket을 반드시 보내야 한다. ticket이 있을 때만 확인하는 방식은 없다.

한 ticket은 한 역할과 한 중계 ID에 묶인다. Viewer용 ticket으로 Server 역할에 접속할 수 없다. 업그레이드 뒤 초기 교환의 중계 ID도 ticket의 `rid`와 같아야 한다. 중계 ID는 짝을 찾는 값이지 접속 권한을 증명하는 비밀값이 아니다.

ticket은 관리 세션과 무관하다. 관리자 로그인 쿠키로 대신할 수 없다. 출발지 IP 허용 목록과 Origin 검사도 계속 적용한다. 개념과 용어는 [용어집](https://github.com/kayden-kim/vnc-relay/blob/main/GLOSSARY.md), 결정 배경은 [ADR-0009](https://github.com/kayden-kim/vnc-relay/blob/main/docs/decisions/ADR-0009-optional-relay-access-ticket.md)를 따른다.

## 보호하지 않는 경로

TCP 진입점은 ticket을 확인하지 못한다. ticket을 켠 역할의 TCP 진입점도 열려 있으면 시작·부분 재로드 때 `ticket_tcp_bypass` 경고를 남긴다. 경고가 TCP 접속을 막거나 시작·부분 재로드를 거절하지는 않는다.

평문 WS는 ticket을 암호화하지 않는다. 해당 역할에서 ticket과 WS를 함께 켜면 `ticket_plaintext_ws` 경고를 남긴다.

> **주의:** 평문 WS는 믿을 수 있는 망 안에서만 써야 한다. 외부 접속에는 WSS를 쓰기를 권장한다. 프록시에서 TLS를 종료한다면 프록시와 VNC Relay 사이 구간도 보호해야 한다. ticket으로 같은 역할의 TCP 접속까지 통제하려면 그 TCP 진입점을 열어 두면 안 된다.

리스너와 IP·Origin·신뢰 프록시 구성은 [설정](configure.md)을 따른다.

## 발급 쪽과 클라이언트 준비

발급 쪽은 역할마다 다른 P-256 서명 키를 써야 한다. 개인키는 발급 쪽만 보관한다. VNC Relay에는 공개 키 파일만 둔다. 두 역할의 ticket 확인을 모두 켰을 때 두 파일에 같은 공개 키가 있으면 거절한다.

발급 쪽은 ES256으로 JWS 압축 형식의 ticket을 만든다. 구조는 헤더·내용·서명을 각각 패딩 없는 base64url로 인코딩하고 점으로 잇는 형식이다. 실제 ticket이나 서명 키는 이 도움말에 예시로 싣지 않는다.

- 헤더에는 `alg = ES256`과 공개 키를 고르는 `kid`가 필요하다.
- 내용에는 `iss`, `aud`, `rh`, `rid`, `jti`, `iat`, `exp`가 필요하다.
- `iss`는 역할의 `issuer`, `rh`는 역할의 `relay_host`와 같아야 한다. `rh`는 요청의 `Host`와 비교하지 않는다.
- `aud`는 Viewer에서 `vnc-relay/viewer`, Server에서 `vnc-relay/server`다.
- `jti`는 ticket마다 128비트 이상 난수로 만들어야 한다. 사용 기록은 역할과 `jti`의 조합으로 같은 ticket의 두 번째 사용을 거절한다.
- `iat`와 `exp`는 발급·접속 만료 시각이다. 0 이상인 Unix 초 정수로 적는다. `iat < exp`여야 한다. `exp − iat`는 그 역할의 `max_lifetime_secs` 이하여야 한다.
- 선택 값 `sx`는 세션 만료 시각이다. 0 이상인 Unix 초 정수로 적는다. 업그레이드 때 현재 시각이 `sx`보다 작아야 하며 5초 오차를 허용하지 않는다. 이 시각이 되면 초기 교환·대기·활성 상태에 있는 연결을 끝낸다. `exp`와 달리 업그레이드 뒤에도 종료 시각으로 작용한다. `sx`가 없으면 ticket이 연결 종료 시각을 정하지 않는다.
- 선택 값 `sub`는 운영 로그에 남을 수 있다. 비밀이나 개인정보를 넣으면 안 된다.

발급 쪽과 VNC Relay의 시계는 5초 안으로 맞춰야 한다. 확인 때 `iat ≤ 현재 시각 + 5초`이고 `현재 시각 < exp + 5초`여야 한다. 서버는 `iat ≤ 프로세스 기동 시각 + 5초`인 ticket을 거절한다. 시작·재시작 뒤에는 `iat > 프로세스 기동 시각 + 5초`인 새 ticket을 발급받아야 한다. 부분 재로드는 기동 시각을 바꾸지 않는다.

전체 형식과 확인 순서는 [WebSocket 계약 1.9.2~1.9.4절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md)을 따른다.

## 역할별 확인 켜기

발급 쪽과 ticket을 전송할 클라이언트를 먼저 준비해야 한다. 기존에 ticket 없이 접속하던 클라이언트는 확인을 켠 뒤 거절된다. 아래는 Viewer 설정 부분이다. Server는 `[relay.ticket.server]`에 같은 종류의 설정을 두되 다른 서명 키의 공개 키 파일을 써야 한다.

1. 해당 역할의 공개 키 파일을 준비한다.
2. 설정 파일에 역할별 ticket 테이블을 추가한다.

   ```toml
   [relay.ticket.viewer]
   enabled = true
   issuer = "issuer.example.test"
   relay_host = "relay.example.test"
   public_keys_file = "keys/viewer-public.jwks.json"
   max_lifetime_secs = 60
   ```

3. 서버가 실행 중이면 파일 변경 감지의 부분 재로드 결과를 기다린다. 실행 중이 아니면 [운영](operate.md) 절차로 시작한다.
4. 운영 로그의 `reload_applied` 또는 시작 때의 `server_ready`를 확인한다.
5. 준비한 클라이언트로 새 ticket을 받아 접속한다.

`enabled`는 확인을 켠다. 같은 역할의 WS·WSS에 함께 적용한다. `issuer`, `relay_host`, `public_keys_file`은 켠 역할에서 필수다. `max_lifetime_secs`는 ticket의 접속 유효 기간 상한이다. 기본은 60초, 허용 범위는 10~600초다. 시계 오차 허용값은 5초 고정이며 설정 키가 없다.

상대 공개 키 경로는 설정 파일의 디렉터리를 기준으로 한다. 잘못된 파일은 시작 때 `ticket_keys_invalid`로 실패한다. 부분 재로드 때는 후보를 거절하고 이전 유효 설정과 공개 키를 유지한다.

키 이름과 적용 조건은 [설정 계약 1.2·1.8절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md)을 따른다.

### 공개 키 파일 구조

`public_keys_file`은 UTF-8 JSON 객체인 JWKS 파일이다. 파일은 64 KiB 이하여야 한다. `keys` 배열에 공개 키 1~16개를 둔다.

각 키는 `kty`가 `EC`, `crv`가 `P-256`이어야 한다. `kid`는 파일 안에서 고유한 1~128바이트 문자열이다. `x`와 `y`는 각각 32바이트 좌표의 패딩 없는 base64url 표기다. 좌표는 P-256 곡선 위의 점이어야 한다.

선택 멤버 `use`는 있으면 `sig`, `alg`는 있으면 `ES256`이어야 한다. 개인키 멤버 `d`는 넣으면 안 된다. 전체 파일 규칙은 [설정 계약 1.11절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md)을 따른다.

## 클라이언트가 보내는 값

클라이언트는 업그레이드 요청의 `Sec-WebSocket-Protocol`에 두 값을 함께 보낸다. 하나는 `binary`다. 다른 하나는 정확히 소문자 `ticket.` 뒤에 JWS 압축 형식을 붙인 값 하나다. 자리표시자로 나타내면 헤더 구조는 `Sec-WebSocket-Protocol: binary, ticket.<JWS>`다. `<JWS>`는 실제 요청에 그대로 쓰는 값이 아니다.

브라우저 noVNC는 `wsProtocols` 옵션으로 두 값을 보낸다. ticket은 URL 쿼리, `Authorization`, 관리자 쿠키에 싣지 않는다. 서버는 성공 응답에서 `binary`만 고른다. ticket 값은 응답에 돌려주지 않는다.

재시도를 포함한 연결 시도마다 새 ticket을 써야 한다. 401 뒤에는 같은 ticket을 다시 보내면 안 된다. 사용 기록을 쓴 뒤 101 응답 전송이 실패해도 ticket은 사용한 것으로 남는다. 클라이언트 교환 형식은 [WebSocket 계약 1.9.1·1.9.7절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md)을 따른다.

## 거절과 세션 만료

ticket 없음·무효·만료·기동 전 발급·재사용은 101 전에 HTTP 401로 거절한다. 응답에는 `WWW-Authenticate: VncRelayTicket`이 있다. 다만 요청 구문·경로·Origin·서브프로토콜 같은 앞선 검사에서 실패하면 그 검사의 응답이 먼저다. `binary` 없이 ticket만 보내면 HTTP 400이다.

ticket으로 업그레이드한 연결이 초기 교환에서 다른 중계 ID를 보내면 Close 4012 `ticket_relay_mismatch`로 거절한다. Viewer Mode I도 허용하지 않는다. Mode I가 꺼져 있으면 기존 모드 거절 Close 1008이 먼저다. Mode I가 켜져 있고 기존 문법 검사를 통과한 요청은 Close 4012다. 대상 연결을 시도하지 않는다.

`sx`에 도달하면 WS·WSS 쪽에 Close 4013 `session_expired`를 보낸다. 활성 세션의 반대쪽 연결도 끝낸다. 양쪽 ticket에 `sx`가 있으면 더 이른 시각을 쓴다. 활성 세션은 정리가 끝난 뒤 종료 원인 `session_expired`로 최근 종료 목록에 오른다. 대기 연결의 만료는 운영 로그에 남는다.

전체 응답·종료 규칙은 [WebSocket 계약 1.9.3~1.9.5절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md)을 따른다. 문제를 추적할 때는 [문제 해결](troubleshooting.md)을 참고한다.

## 공개 키 교체

서버는 켠 역할의 공개 키 파일 변경을 감지해 부분 재로드한다. 두 역할의 공개 키 파일 경로와 내용 모두 재로드 대상이다. 대시보드에서는 경로를 편집할 수 없다.

1. 같은 경로의 공개 키 파일에 새 공개 키를 추가한다.
2. `reload_applied`로 새 공개 키 반영을 확인한다.
3. 발급 쪽을 새 서명 키로 바꾼다.
4. 발급 쪽을 바꾼 뒤 해당 역할의 `max_lifetime_secs + 10초`가 지나면 파일에서 옛 공개 키를 제거한다.
5. 다시 `reload_applied`를 확인한다.

교체 후보가 잘못되면 `reload_rejected`를 기록하고 이전 공개 키를 유지한다. 재시작 필요 변경이 설정 파일에 남아 있어도 후보 전체를 거절한다. 자세한 조건은 [설정](configure.md)을 따른다.

키 교체·부분 재로드·ticket 확인 끄고 켜기는 사용 기록을 지우지 않는다. 이미 101을 보낸 연결의 중계 ID 대조와 `sx` 타이머도 유지한다. 교체 순서는 [설정 계약 1.11절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md)을 따른다.

## 대시보드와 로그에서 확인

"설정 보기"의 "접속 ticket"에서 두 역할의 키를 확인할 수 있다. "동적 설정 편집"은 역할별 `enabled`, `issuer`, `relay_host`, `max_lifetime_secs`를 편집한다. 해당 역할이 WS 또는 WSS에서 하나라도 켜져 있어야 한다. 공개 키 파일은 경로만 보여 주며 내용을 보여 주지 않는다.

최근 종료 목록은 `session_expired`를 "세션 만료"로 표시한다. 화면 사용법은 [대시보드](dashboard.md)를 따른다.

운영 로그에서는 `ticket_accepted`, `connection_rejected`의 ticket 관련 `reason`, `connection_aborted`, `waiting_removed`, `session_closed`를 확인한다. `sx` 만료는 초기 교환 중이면 `connection_aborted`, 대기 중이면 `waiting_removed`, 활성 세션이면 `session_closed`에 기록한다. ticket 원문과 공개 키 파일 내용은 로그·진단·상태·관리 API 응답에 넣지 않는다. 필드와 전체 사건은 [운영 정보 계약 1.4~1.6·1.11절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md)을 따른다.
