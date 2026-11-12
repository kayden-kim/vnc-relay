# 소개

이 도움말로 VNC Relay를 설치하고 VNC 연결을 구성하며 대시보드에서 운영할 수 있다.

## VNC Relay가 하는 일

`vnc-relay`는 UltraVNC Repeater와 호환되는 VNC Relay다. TCP·WS·WSS 중계와 내장 대시보드를 실행파일 하나로 제공한다. WS는 WebSocket이고 WSS는 TLS로 암호화한 WebSocket이다.

| 기능 | 사용 방법 |
| --- | --- |
| Mode I | Viewer가 대상 주소를 지정하면 VNC Relay가 허용된 VNC Server에 연결한다. VNC Relay에서 Server의 VNC 포트에 연결할 수 있어야 한다. |
| Mode II | Viewer와 Server가 각각 VNC Relay에 연결한다. VNC Relay는 같은 중계 ID로 들어온 두 역할을 짝지어 중계한다. Server가 바깥에서 들어오는 연결을 받을 수 없어도 쓸 수 있다. |
| 대시보드 | 관리자 계정으로 로그인해 대기 연결·활성 세션·최근 종료 목록·최근 로그·설정을 확인한다. 활성 세션의 관리자 종료와 동적 설정 편집도 할 수 있다. |

상황에 맞는 방식은 [연결 방식 고르기](connect.md#연결-방식-고르기)에서 고른다.

Rust로 독립 구현했다. UltraVNC·websockify·noVNC 소스를 복사·번역·포팅하지 않았다. 프로젝트는 [MIT 라이선스](https://github.com/kayden-kim/vnc-relay/blob/main/LICENSE)를 따른다.

noVNC는 포함하지 않는다. 운영자가 별도로 준비하는 외부 Viewer 클라이언트다. 대시보드는 원격 화면을 표시하는 noVNC 페이지가 아니다.

제품 범위와 독립 구현 원칙은 [저장소 README](https://github.com/kayden-kim/vnc-relay/blob/main/README.md)에 있다.

## 대상 독자

이 도움말은 실행파일과 설정 파일을 직접 배치하는 운영자를 위한 안내서다. Windows 11 x64와 Ubuntu 24.04 LTS x64를 대상으로 한다. OS 서비스 등록과 컨테이너 배치는 다루지 않는다.

## 읽는 순서

| 할 일 | 페이지 |
| --- | --- |
| 배포 묶음을 내려받고 해시 확인하기 | [설치](install.md) |
| 루프백에서 처음 실행하기 | [빠른 시작](quick-start.md) |
| UltraVNC·noVNC와 Mode I·II 연결하기 | [VNC 연결 구성](connect.md) |
| 리스너·접근 제한·TLS 설정하기 | [설정](configure.md) |
| 로그인하고 연결·세션·로그·설정 다루기 | [대시보드](dashboard.md) |
| 외부 발급 쪽과 접속 허가 연동하기 | [접속 ticket](access-ticket.md) |
| 시작·종료·부분 재로드·업그레이드하기 | [운영](operate.md) |
| 시작 실패와 접속 실패 조사하기 | [문제 해결](troubleshooting.md) |
| CLI·설정·프로토콜 기준 찾기 | [참조](reference.md) |
| 라이선스와 제삼자 고지 확인하기 | [고지](notices.md) |

## 기준 버전

이 도움말은 저장소의 `main`을 따른다. [최신 Release](https://github.com/kayden-kim/vnc-relay/releases/latest)는 `v2026.11.2`이며 2026-11-09에 공개됐다. 배포 묶음은 [GitHub Releases](https://github.com/kayden-kim/vnc-relay/releases)에서 받는다.

현재 `main`으로 빌드해도 다음 릴리스 전까지 버전은 `2026.11.2`로 표시된다. 버전 출력만으로 아래 두 동작을 구분하지 않는다.

| 차이 | `main`의 동작 | `v2026.11.2` Release 사용자 |
| --- | --- | --- |
| WS·WSS Viewer 경로 | `/relay/viewer`를 쓴다. 이전 `/relay` 경로는 받지 않는다. | `/relay`를 써야 한다. 이 도움말의 noVNC URL에서 `path=relay/viewer`를 `path=relay`로 바꾼다. TCP와 `/relay/server`는 바꾸지 않는다. |
| 선택형 접속 ticket 확인 | `[relay.ticket.viewer]`·`[relay.ticket.server]`에서 역할마다 켠다. 해당 역할의 WS·WSS에 함께 적용하며 TCP에는 적용하지 않는다. 기본은 꺼짐이다. 관련 운영 로그, 대시보드의 ticket 설정 입력과 종료 원인 `session_expired` 표시도 제공한다. | 이 기능을 쓸 수 없다. ticket 설정 테이블을 넣지 않는다. 사용하려면 `main`으로 묶음을 만들어야 한다. |

Viewer 경로는 [WebSocket 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md) 1.7절·3.1절을 따른다. 접속 ticket은 같은 계약 1.9절과 [설정 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md) 1.2절을 따른다. 관련 로그는 [운영 정보 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md)을 따른다.

대시보드의 ticket 입력과 종료 원인 표시는 [대시보드 스펙](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/dashboard.md)의 SCR-005·SCR-001을 따른다.
