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

이 도움말은 실행파일과 설정 파일을 직접 배치하는 운영자를 위한 안내서다. Windows 11 x64와 Ubuntu 24.04 LTS x64를 대상으로 한다. Windows 서비스·트레이 운영도 다룬다. 컨테이너 배치는 다루지 않는다.

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
| 브라우저 Viewer와 Android Server를 WSS·접속 ticket으로 연결하기 | [추천 구성 사례](recommended-setup.md) |
| 시작 실패와 접속 실패 조사하기 | [문제 해결](troubleshooting.md) |
| CLI·설정·프로토콜 기준 찾기 | [참조](reference.md) |
| 라이선스와 제삼자 고지 확인하기 | [고지](notices.md) |

## 기준 버전

이 도움말은 저장소의 `main`을 따른다. [최신 Release](https://github.com/kayden-kim/vnc-relay/releases/latest)는 `v2026.11.6`이며 2026-11-18에 공개됐다. 배포 묶음은 [GitHub Releases](https://github.com/kayden-kim/vnc-relay/releases)에서 받는다.

현재 `main`의 동작은 최신 Release와 같다. 다음 릴리스 전에 `main`의 동작이 달라지면 아래 표에 적는다. `main`으로 빌드해도 다음 릴리스 전까지 버전은 `2026.11.6`으로 표시된다.

| 차이 | `main`의 동작 | `v2026.11.6` Release 사용자 |
| --- | --- | --- |
| 없음 | 최신 Release와 같다. | 이 도움말을 그대로 따른다. |

`v2026.11.5`에서 올리면 다음 동작이 바뀐다.

| 동작 | `v2026.11.6`부터 | `v2026.11.5` |
| --- | --- | --- |
| 트레이 아이콘 | 대시보드 상단 막대의 브랜드 표식과 같은 그림이다. 어두운 둥근 사각형 안에 서로 반대 방향을 가리키는 흰 화살표 두 개가 있다. | 짙은 녹색 둥근 사각형 안에 흰 사각형 틀이 있다. |

트레이 아이콘은 [Windows 트레이 앱 스펙](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/windows-tray.md)의 공통 시각 기준을 따른다.

`v2026.11.4` 이하에서 올리면 다음 동작도 바뀐다.

| 동작 | `v2026.11.5`부터 | `v2026.11.4` 이하 |
| --- | --- | --- |
| Windows 서비스·트레이 앱 | `service install`·`service uninstall`·SCM 전용 `service run`과 Windows 묶음의 `vnc-relay-tray.exe`를 제공한다. [설치](install.md#windows-서비스-설치와-해제)와 [운영](operate.md#windows-서비스와-트레이)을 따른다. | 이 기능과 트레이 실행파일이 없다. `serve`로 수동 실행한다. |
| 대시보드 대기 연결·활성 세션 표 | 접속 시각과 시작 시각을 날짜와 시각 두 줄로 보인다. 활성 세션 상대 주소의 레이블은 값 위 줄에 있다. | 폭이 넉넉하면 시각을 한 줄로 보이고, 활성 세션 주소의 레이블이 값과 같은 줄에 있다. [대시보드](dashboard.md)의 그림이 화면과 다르다. |
| 대시보드 자동 갱신 표시 | 자동 갱신이 1초 안에 끝나면 `갱신 중`·`조회 중` 배지와 새로고침 버튼 문구를 바꾸지 않는다. 1초가 지나도 끝나지 않으면 `갱신 중`을 보인다. | 자동 갱신 때마다 `갱신 중`·`조회 중` 배지와 새로고침 버튼 문구가 잠깐 바뀐다. |

서비스는 [운영 스펙 3.7절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/operations.md#37-windows-서비스-실행)과 [CLI 계약 1.9절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md#19-windows-서비스-실행)을, 트레이는 [Windows 트레이 앱 스펙](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/windows-tray.md)을 따른다. 대시보드 표시는 [대시보드 스펙](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/dashboard.md)의 SCR-001과 4.1절을 따른다.

`v2026.11.3` 이하에서 올리면 다음 동작도 바뀐다.

| 동작 | `v2026.11.4`부터 | `v2026.11.3` 이하 |
| --- | --- | --- |
| 대기·최근 종료 목록 정보 | 관리 API의 대기 항목에 전송 경로·상대 주소·전달 주소가 있다. 최근 종료 항목과 관리자 종료 응답에는 지속 시간과 양 끝의 상대 주소·전달 주소가 있다. 대시보드 운영 현황의 두 표가 이 값을 보이고, 최근 종료 행에서 그 세션의 로그를 연다. | 이 값과 최근 종료 행의 로그 보기 링크가 없다. [대시보드](dashboard.md)의 대기 연결·최근 종료 표 설명과 그림이 화면과 다르다. |

목록 정보는 [관리 API 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/management-api-v1.md) 1.7·1.9·1.10절과 [대시보드 스펙](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/dashboard.md)의 SCR-001을 따른다.

`v2026.11.2` 이하에서 올리면 다음 동작도 바뀐다.

| 동작 | `v2026.11.3`부터 | `v2026.11.2` 이하 |
| --- | --- | --- |
| WS·WSS Viewer 경로 | `/relay/viewer`를 쓴다. 이전 `/relay` 경로는 받지 않는다. | `/relay`를 쓴다. 이 도움말의 noVNC URL에서 `path=relay/viewer`를 `path=relay`로 바꾼다. 올릴 때는 서버와 모든 Viewer URL을 함께 옮긴다([업그레이드와 롤백](operate.md#업그레이드와-롤백)). TCP와 `/relay/server`는 바꾸지 않는다. |
| 선택형 접속 ticket 확인 | `[relay.ticket.viewer]`·`[relay.ticket.server]`에서 역할마다 켠다. 해당 역할의 WS·WSS에 함께 적용하며 TCP에는 적용하지 않는다. 기본은 꺼짐이다. 관련 운영 로그, 대시보드의 ticket 설정 입력과 종료 원인 `session_expired` 표시도 제공한다. | 이 기능을 쓸 수 없다. ticket 설정 테이블을 넣지 않는다. |

Viewer 경로는 [WebSocket 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md) 1.7절·3.1절을 따른다. 접속 ticket은 같은 계약 1.9절과 [설정 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md) 1.2절을 따른다. 관련 로그는 [운영 정보 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md)을 따른다.

대시보드의 ticket 입력과 종료 원인 표시는 [대시보드 스펙](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/dashboard.md)의 SCR-005·SCR-001을 따른다.
