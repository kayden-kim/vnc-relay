# VNC 연결 구성

이 페이지로 상황에 맞는 연결 방식을 고르고 UltraVNC의 TCP 연결, noVNC의 WSS 연결, WS·WSS Server 역할, Mode I 대상 연결을 구성할 수 있다.

## 연결 방식 고르기

두 방식은 VNC Relay와 Server 사이의 연결을 누가 여는지가 다르다. Mode II에서는 Server가 VNC Relay에 연결한다(역접속). Mode I에서는 VNC Relay가 Server에 연결한다.

```text
Mode II: Viewer --> VNC Relay <-- Server
Mode I:  Viewer --> VNC Relay --> Server
```

| 방식 | Viewer가 하는 일 | Server가 하는 일 |
| --- | --- | --- |
| Mode II | VNC Relay의 Viewer 진입점에 중계 ID로 접속한다. | 같은 중계 ID로 Server 진입점에 역접속한다. |
| Mode I | VNC Relay의 Viewer 진입점에 대상 주소를 보낸다. | 대상 VNC 포트에서 기다린다. VNC Relay에 역접속하지 않는다. |

### 상황별 선택

| 상황 | 방식 | 이유 |
| --- | --- | --- |
| 원격 지원처럼 Server PC가 공유기(NAT)나 방화벽 뒤에 있어 바깥에서 들어오는 연결을 받을 수 없다. | Mode II | Viewer와 Server가 모두 VNC Relay에 연결한다. Server는 들어오는 연결을 받지 않아도 된다. |
| VNC Relay에서 Server의 VNC 포트에 연결할 수 있다. 예를 들어 VNC Relay와 Server는 같은 내부망에 있고 Viewer만 바깥에 있다. | Mode I | Server는 역접속 설정 없이 VNC 포트에서 기다린다. VNC Relay는 Viewer가 보낸 대상이 대상 허용 정책에 있을 때만 연결한다. |
| 브라우저의 noVNC로 접속한다. | Mode II | 검증한 noVNC 1.7.0은 `repeaterID`의 중계 ID를 보낸다. 대상 주소를 보내는 Mode I 요청은 보내지 않는다. |
| Server 쪽 클라이언트가 WS·WSS로 접속한다. | Mode II | Server 역할은 전송과 관계없이 Mode II만 받는다. |
| Viewer의 WS·WSS 접속에 접속 ticket 확인을 켠다. | Mode II | ticket 확인을 켠 Viewer 역할은 Mode I 요청을 거절한다. |

사용 시나리오의 기준은 [PRD](https://github.com/kayden-kim/vnc-relay/blob/main/docs/requirements/PRD.md) 5절에 있다.

### Mode II를 쓸 때 확인할 것

VNC Relay는 Viewer와 Server가 모두 접속할 수 있는 곳에 두어야 한다. Viewer와 Server는 서로 직접 연결하지 않는다. 두 역할 가운데 어느 쪽이 먼저 접속해도 된다. 상대가 오지 않으면 대기 연결은 `timeouts.wait_secs`가 지난 뒤 끝난다.

두 역할이 쓸 중계 ID는 미리 정해 두어야 한다. 중계 ID는 접속 권한을 증명하는 비밀값이 아니다. 진입점마다 출발지 IP 허용 목록을 의도한 범위로 제한해야 한다. WS·WSS 역할에는 [접속 ticket](access-ticket.md) 확인을 켤 수 있다.

한 중계 ID로는 동시에 Viewer 하나와 Server 하나만 짝지을 수 있다. 같은 역할이 이미 쓰는 중계 ID로 접속하면 거절된다. 자세한 동작은 [중복 ID 거절 이해하기](#중복-id-거절-이해하기)에 있다.

### Mode I을 쓸 때 확인할 것

VNC Relay에서 대상 Server의 VNC 포트로 연결할 수 있어야 한다. Server는 그 포트에서 기다려야 한다. VNC Relay가 `timeouts.connect_secs` 안에 대상에 연결하지 못하면 Viewer 연결을 닫는다.

Mode I을 켜면 Viewer 진입점에 접속할 수 있는 클라이언트는 VNC Relay를 거쳐 허용된 대상에 연결할 수 있다. 그래서 `mode1.allow_targets`에는 필요한 대상과 포트만 적어야 한다. Viewer 진입점의 출발지 IP 허용 목록도 의도한 범위로 제한해야 한다. 이름으로 지정한 대상의 규칙은 [Mode I 대상 허용 정책](configure.md#mode-i-대상-허용-정책)을 따른다.

WS·WSS로 Mode I을 쓰려면 대상 주소를 초기 요청으로 보내는 WebSocket 클라이언트가 필요하다. UltraVNC Viewer는 WebSocket을 쓰지 않는다.

## Mode II 예시 설정

아래 예시는 `main` 기준이며 접속 ticket 확인은 꺼져 있다. ticket을 켜려면 [접속 ticket](access-ticket.md)의 별도 클라이언트 연동을 준비해야 한다. 아래 noVNC URL만으로는 ticket을 보내지 않는다.

예시 주소는 문서용이다. 실제 배치에서는 호스트·IP·출발지 IP 허용 목록·Origin·인증서 경로를 맞게 바꾸어야 한다.

| 구성요소 | 예시 값 |
| --- | --- |
| VNC Relay | `relay.example.test`, `192.0.2.20` |
| Viewer PC | `192.0.2.10` |
| Server PC | `192.0.2.30` |
| noVNC 페이지 제공 사이트 | `https://viewer.example.test` |
| 중계 ID | `1234` |

다음은 [README의 연결 구성 예](https://github.com/kayden-kim/vnc-relay/blob/main/README.md)를 사용한 `relay.toml` 전체다. [빠른 시작](quick-start.md)의 루프백 파일에 그대로 덧붙이지 말고 별도 구성으로 준비한다.

```toml
[admin]
accounts_file = "admin-accounts.toml"

[relay.tcp_viewer]
enabled = true
listen = "0.0.0.0:5901"
allow = ["192.0.2.10"]

[relay.tcp_server]
enabled = true
listen = "0.0.0.0:5500"
allow = ["192.0.2.30"]

[relay.wss]
listen = "0.0.0.0:8443"
cert_file = "tls/relay-chain.pem"
key_file = "tls/relay-key.pem"

[relay.wss.viewer]
enabled = true
allow = ["192.0.2.10"]
allowed_origins = ["https://viewer.example.test"]

[mode2]
enabled = true

[management]
listen = "127.0.0.1:18080"

[status]
listen = "127.0.0.1:18081"

[log]
output = "relay.log"
```

`allow`는 해당 진입점에 접속할 출발지 IP를 제한한다. 목록 밖의 출발지는 거절한다. 이 예시는 신뢰 프록시를 쓰지 않는 직접 연결 구성이다.

`cert_file`과 `key_file`은 `relay.example.test`에 맞는 인증서 체인과 개인키를 가리킨다. 브라우저가 신뢰하는 인증서를 준비해야 한다. 경로는 `relay.toml`의 디렉터리를 기준으로 푼다. 개인키 본문은 설정 파일에 넣지 않는다.

`output`은 운영 로그의 출력 위치다. 파일 경로 또는 `"stdout"`을 넣는다. 이 예시는 `relay.log` 파일에 이어 쓴다. `"stdout"`이면 표준 출력으로 쓴다. `output`을 생략해도 표준 출력이다. 자세한 내용은 [설정](configure.md#관리상태-리스너와-기록)을 따른다.

1. 운영 디렉터리에 `relay.toml`을 준비한다.
2. 설정에서 지정한 인증서 체인과 개인키 파일을 준비한다.
3. [빠른 시작](quick-start.md)에 따라 관리자 계정을 만든다.
4. [운영](operate.md)에 따라 VNC Relay를 시작한다.

> **주의:** 이 예시는 중계 리스너를 모든 IPv4 인터페이스에 바인드한다. 시작 전에 방화벽과 출발지 IP 허용 목록을 의도한 범위로 제한해야 한다. 관리 HTTP와 상태 리스너는 루프백에 유지한다. 공개 배치 준비는 [설정](configure.md)을 따른다.

## TCP로 UltraVNC 연결하기

UltraVNC 명령 형식은 UltraVNC 1.8.3.0에서 확인했다. 아래 PowerShell 명령은 각 PC의 UltraVNC 설치 디렉터리에서 실행한다. VNC Relay는 지원하는 두 OS 가운데 어느 쪽에서든 실행할 수 있다. 이 명령은 Windows UltraVNC 클라이언트용이며 Ubuntu 클라이언트 명령을 뜻하지 않는다.

| 역할 | 실행할 PC | 접속 대상 | 실행할 명령 |
| --- | --- | --- | --- |
| UltraVNC Server | `192.0.2.30` | TCP Server `192.0.2.20:5500` | 아래 Server 명령 |
| UltraVNC Viewer | `192.0.2.10` | TCP Viewer `192.0.2.20:5901` | 아래 Viewer 명령 |
| noVNC Viewer | `192.0.2.10`의 브라우저 | WSS Viewer `relay.example.test:8443/relay/viewer` | 다음 절의 noVNC URL |

1. Server PC에서 역접속을 시작한다.

   ```powershell
   .\winvnc.exe -id:1234 -connect 192.0.2.20::5500 -run
   ```

2. Viewer PC에서 같은 중계 ID로 접속한다.

   ```powershell
   .\vncviewer.exe -proxy 192.0.2.20:5901 ID:1234
   ```

이 순서는 예시다. Viewer를 먼저 실행해도 된다. `::5500`과 `:5901`의 표기는 각 UltraVNC 옵션의 형식이므로 임의로 통일하지 않는다.

실제 검증 범위는 [호환성 검증 기록](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/repeater-compat-record.md) 6절을 따른다. Windows VNC Relay와 실제 UltraVNC 조합을 확인했다. Ubuntu VNC Relay와 실제 UltraVNC 조합은 그 기록에서 미실행이며 Ubuntu 자동 시험과 구분한다.

## WSS로 noVNC 연결하기

noVNC는 별도로 준비하는 외부 클라이언트다. VNC Relay는 noVNC 파일을 제공하지 않는다. 아래 예시는 noVNC를 `https://viewer.example.test`에서 제공한다고 가정한다. 검증한 noVNC 버전은 1.7.0이다.

1. TCP 절의 Server 명령으로 Server를 접속시킨다.
2. Viewer PC의 브라우저에서 다음 URL을 연다. UltraVNC Viewer와 같은 ID로 동시에 접속하지 않는다.

   ```text
   https://viewer.example.test/vnc.html?autoconnect=true&host=relay.example.test&port=8443&encrypt=true&path=relay/viewer&repeaterID=1234
   ```

| URL 값 | 의미 |
| --- | --- |
| `host=relay.example.test&port=8443` | VNC Relay의 WSS 리스너 |
| `encrypt=true` | WSS 사용 |
| `path=relay/viewer` | Viewer 역할의 `/relay/viewer` 경로 |
| `repeaterID=1234` | Mode II 중계 ID. noVNC가 `ID:`를 붙여 초기 요청으로 보낸다. |

> **참고:** `v2026.11.2` 이하 Release는 Viewer 경로가 `/relay`다. 그 버전에 접속할 때는 위 URL에서 `path=relay/viewer`를 `path=relay`로 바꾼다. 자세한 차이는 [소개](index.md)에 있다.

브라우저는 noVNC 페이지를 제공한 사이트의 origin을 `Origin` 헤더로 보낸다. 이 예시에서는 `https://viewer.example.test`다. WSS 접속 대상인 `relay.example.test`와 혼동하지 않는다.

`allowed_origins`는 역할마다 설정한다. 빈 목록은 `Origin` 헤더가 있는 요청을 모두 거절한다. 헤더가 없는 클라이언트는 Origin 비교만 건너뛴다. IP 검사와 해당 역할에 켠 접속 ticket 확인은 그대로 받는다. Origin은 사용자 인증 수단이 아니며 출발지 IP 허용 목록을 대신하지 않는다.

Origin의 정확한 문법과 비교 규칙은 [설정 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md) 1.4절에 있다.

## WS·WSS Server 역할 연결하기

Server 역할은 `/relay/server`를 사용하며 Mode II만 받는다. 앞의 설정에 다음 표를 추가하면 WSS Server 역할도 켜진다.

```toml
[relay.wss.server]
enabled = true
allow = ["192.0.2.30"]
allowed_origins = []
```

이 목록은 `Origin` 없는 Server 클라이언트용이다. 브라우저 클라이언트라면 해당 페이지의 origin을 명시해야 한다.

보호된 네트워크에서 평문 WS Server를 쓰려면 별도로 다음 표를 추가할 수 있다.

```toml
[relay.ws]
listen = "192.0.2.20:8081"

[relay.ws.server]
enabled = true
allow = ["192.0.2.30"]
allowed_origins = []
```

| 전송·역할 | 접속 주소 | 필요한 클라이언트 동작 |
| --- | --- | --- |
| WSS Server | `wss://relay.example.test:8443/relay/server` | WebSocket 업그레이드 뒤 Server의 `ID:` 초기 요청을 바이너리 메시지로 먼저 보낸다. |
| WS Server | `ws://relay.example.test:8081/relay/server` | 같은 Server 초기 교환을 쓴다. 평문 구간은 암호화되지 않는다. |

Server 경로에는 VNC Relay가 시작 바이트를 보내지 않는다. 이를 기다리는 Viewer 방식으로 구현하면 안 된다. UltraVNC의 위 TCP 명령에 WSS URL을 넣는 방식이 아니라, Server 역할의 WebSocket 클라이언트가 필요하다. 바이트 형식은 [Repeater 초기 교환 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/repeater-handshake-v1.md) 1절과 [WebSocket 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md) 1.4절·1.7절을 따른다.

인터넷에 공개하는 WebSocket 진입점에는 WSS 또는 역방향 프록시 TLS 종료를 권장한다. 평문 WS는 보호된 네트워크에서만 켜기를 권장한다. 설정을 바꾼 뒤 역할을 켜려면 VNC Relay를 재시작해야 한다.

## Mode I 대상 연결하기

Mode I에서는 VNC Relay가 Server의 VNC 포트에 연결한다. 출발지 IP 허용 목록과 대상 허용 정책은 별개다. 기존 `relay.toml`에 다음 표를 추가한다.

```toml
[mode1]
enabled = true
allow_targets = ["192.0.2.30:5900"]
```

`allow_targets`는 켠 Mode I에서 필수다. 빈 목록은 모든 대상을 거절한다. 이 예시에서는 `192.0.2.30:5900`만 허용한다.

1. Server PC의 UltraVNC 설치 디렉터리에서 Server를 실행한다.

   ```powershell
   .\winvnc.exe -run
   ```

2. VNC Relay를 재시작해 Mode I을 켠다.
3. Viewer PC의 UltraVNC 설치 디렉터리에서 대상과 프록시를 지정한다.

   ```powershell
   .\vncviewer.exe -connect 192.0.2.30::5900 -proxy 192.0.2.20:5901
   ```

Server는 역접속하지 않고 `5900` 포트에서 기다려야 한다. VNC Relay에서 그 포트에 도달할 수 있어야 한다. 대상 허용 정책의 세부 형식은 [설정 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md) 1.5절을 따른다.

## 중복 ID 거절 이해하기

같은 중계 ID를 같은 역할이 이미 쓰고 있으면 뒤에 온 연결을 거절한다. 대기 연결과 활성 세션 모두에 적용한다. 먼저 온 연결과 활성 세션은 유지한다. 이 규칙은 TCP·WS·WSS에 공통이다.

예를 들어 UltraVNC Viewer가 `1234`로 대기 중이면 noVNC Viewer도 `1234`로 접속할 수 없다. 같은 ID의 Server 한 개가 들어오는 것은 중복이 아니라 정상 매칭이다. 연결을 다시 구성할 때는 이전 연결이 정리됐는지 확인한다.

정확한 거절 동작은 [Repeater 초기 교환 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/repeater-handshake-v1.md) 2절에 있다. 접속이 실패하면 [문제 해결](troubleshooting.md)과 [대시보드](dashboard.md)의 최근 로그 안내를 따른다.
