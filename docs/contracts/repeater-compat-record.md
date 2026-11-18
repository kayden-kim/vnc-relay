# 호환성 검증 기록

이 기록은 고정 버전 UltraVNC Server·Viewer·Repeater와 noVNC의 실제 상호운용 증거를 담는다. 3절은 Repeater의 초기 교환 관찰, 6절은 VNC Relay의 모드·전송 조합 검증, 7절은 두 프로그램의 비교 캡처다. 증거 항목은 [Repeater 초기 교환 계약 6.2절](repeater-handshake-v1.md#62-기록할-증거)을 따른다.

- 상태: Windows 관찰·모드·전송 조합 검증·비교 캡처 완료. 실제 UltraVNC와 Ubuntu VNC Relay의 조합과 Ubuntu 비교 캡처는 미실행이다.
- 검증일: Repeater 관찰 2026-11-01, VNC Relay 조합 검증 2026-11-06, 비교 캡처 2026-11-06~07.
- 대상: Repeater 관찰은 2절의 고정 배포본이다. 관찰 기록의 기준 커밋은 `ff6bbb5`다. 6절과 7절은 커밋 `70ff541`의 Windows release 빌드를 검증했다.
- 환경: Windows 11 x64 호스트. 내려받기는 `curl.exe`, 해시 계산은 `certutil`, 버전 확인은 PowerShell 파일 버전 정보를 썼다.

## 1. 관찰 허용 검토

| 항목 | 내용 |
| --- | --- |
| 결정 | 허용 |
| 결정 주체 | 프로젝트 소유자 |
| 범위 | UltraVNC 트래픽의 패킷 관찰. 인터넷과 분리한 테스트 네트워크에서만 한다 |
| 제약 | UltraVNC·websockify·noVNC 소스 코드는 관찰·분석에 쓰지 않는다([관찰 절차 6.1절](repeater-handshake-v1.md#61-관찰-절차)) |
| 6.1절 1단계 판정 | 소유자 허용으로 완료. 법률 검토는 하지 않았다 |

이 결정은 프로젝트 소유자의 허용이다. 법률 검토 결과가 아니다. [PRD 9절](../requirements/PRD.md#9-호환성-조사와-독립-구현-원칙)은 패킷 관찰과 역공학 절차의 허용 여부를 관할법·제품 약관·사용 권한에 따라 따로 검토한다고 정한다. 이 기록은 그 결과를 대신하지 않는다.

## 2. 검증 대상 고정

검증에 쓴 배포본은 아래 버전과 SHA-256으로 식별한다. 2026-11-01에 반입 파일의 해시를 다시 계산해 고정값과 일치함을 확인했다(3.2절).

### 2.1 UltraVNC Server·Viewer (Windows x64) — 고정

- 버전: 1.8.3.0. [공식 다운로드 항목](https://uvnc.com/downloads/ultravnc/172-ultravnc-1-8-3-0.html).
- 배포본: `UltraVNC_1830_x64_Setup.exe`, 6,239,336바이트.
- SHA-256: `e9c22419ef3128a707b0d004a2c58e25459528c5c12d104dea56e5e662bd131e`. [1.8.3.0 항목](https://uvnc.com/downloads/ultravnc/172-ultravnc-1-8-3-0.html)의 게시값과 같다.
- 받은 주소: https://uvnc.eu/download/1800/UltraVNC_1830_x64_Setup.exe. 공식 항목의 다운로드 링크가 이 주소로 넘어간다.
- 받은 날짜: 2026-11-01. SHA-256이 고정값과 같았다.
- OS: Windows 11 x64. 설치한 `winvnc.exe`와 `vncviewer.exe`의 파일 버전이 모두 1.8.3.0이다. 실행 파일의 경로·해시는 3.2절에 있다.
- 설치 파일의 `repeater.exe`는 고정 관찰 대상과 다른 파일이다(3.1절). 관찰에는 2.2절 파일만 썼다.

### 2.2 UltraVNC Repeater (Windows x64) — 고정

- 버전: 1.6.0.0(파일 버전 정보).
- 배포본: `repeater_v160.zip`, 224,709바이트. 압축 안에는 `repeater.exe` 한 개가 있다.
- SHA-256: `82d9e7f06266a291b4988e36f4d33e127d63adb93269f3b24c432718f35de1de`.
- 받은 주소: https://uvnc.eu/download/repeater/repeater_v160.zip. [공식 Repeater 페이지](https://uvnc.com/downloads/ultravnc-repeater/83-ultravnc-repeater.html)의 항목 "ultravnc_repeater_1400"(표기 1.4.0.0)의 다운로드 링크가 이 주소로 넘어간다.
- 해시 확인일: 2026-11-01. 반입 파일의 SHA-256이 고정값과 같았다.
- OS: Windows 11 x64. 실행 파일의 경로·해시는 3.2절에 있다.
- 표기 불일치: 공식 항목은 1.4.0.0, 72.05 KB로 표기하지만 받은 파일은 1.6.0.0, 224,709바이트다. 원인은 확인하지 않았다. 검증 대상은 받은 파일의 1.6.0.0이다.

### 2.3 UltraVNC Repeater (Ubuntu 24.04 LTS x64) — 고정 범위 밖

Ubuntu `uvncrepeater`는 관찰 대상에 포함하지 않는다. 고정 Repeater는 Windows 배포본이다. Ubuntu VNC Relay의 검증 범위는 2.6절에 있다.

### 2.4 noVNC (브라우저 클라이언트) — 고정

- 버전: 1.7.0. [GitHub 릴리스 v1.7.0](https://github.com/novnc/noVNC/releases/tag/v1.7.0).
- 배포본: `novnc-v1.7.0.zip`(GitHub 태그 아카이브 `v1.7.0.zip`의 저장 이름), 823,923바이트.
- SHA-256: `d55e9cce9a611294df33c7200f32f35108652de2d86a468bebcfc6f2a1c9a196`.
- 받은 주소: https://github.com/novnc/noVNC/archive/refs/tags/v1.7.0.zip
- 해시 확인일: 2026-11-01. 반입 파일의 SHA-256이 고정값과 같았다.
- 검증 환경: Windows 게스트의 Microsoft Edge다. 구체적인 버전은 3.4절과 6.2절에 있다.

### 2.5 받지 않은 배포본

- UltraVNC 1.8.3.0의 다른 배포본(zip, msi, x86 설치 파일)은 받지 않았다. 이 기록은 x64 설치 파일 하나를 고정한다.
- 받은 파일은 해시와 주소를 기록한 뒤 지웠다. 해시와 주소가 증거다.

### 2.6 Ubuntu 24.04 LTS x64 검증 (별도 기록)

- 범위: Windows 11 x64·Ubuntu 24.04 LTS x64 양쪽의 VNC Relay 실행·종료·호환성 검증 중 Ubuntu 쪽 증거다.
- 상태: 실제 UltraVNC와 Ubuntu VNC Relay의 조합은 미실행이다. 분리 실행 환경이 없다. Ubuntu 지원 판정에는 자동 시험을 쓴다. 현재 결과 확인 방법은 [6.6절](#66-ubuntu-2404-증거)에 있다.

## 3. 관찰 증거

관찰은 2026-11-01에 마쳤다. 6.2절의 증거 항목별 위치는 다음과 같다.

| 증거 | 상태 | 위치 |
| --- | --- | --- |
| 대상 | 기록 | 2절(고정 버전과 해시), 3.2절(VM에 설치한 실행 파일의 경로·버전·해시), 3.4절 외부 WebSocket(Edge 버전) |
| 환경 | 기록 | 3.2절(VM·네트워크·캡처 도구), 3.3절(라이선스 상태, Repeater 기본 설정과 Mode I 설정, 실행 방법) |
| 입력 | 기록 | 3.4절(시나리오별 명령줄·입력 바이트·ID·대상 값) |
| 원본 | 기록 | 3.6절(캡처 해시와 보관 위치). 방향별 바이트 덤프와 시각은 시나리오별 `summary.md` |
| 결과 | 기록 | 3.4절(Repeater와 끝점의 반응, 매칭 여부, 종료 방식) |
| 해석 | 기록 | 3.5절(계약 1절 항목별 결론과 근거 시나리오) |
| 관찰 허용 | 기록 | 1절의 프로젝트 소유자 허용 |
| 설치 뒤 `vncviewer.exe` 파일 버전 | 확인 | 1.8.3.0(3.2절) |

### 3.1 실행 파일과 캡처 도구

실행 파일은 2026-11-01에 3.2절의 VM에서 확인했다. 설치 파일에 포함된 Repeater와 고정 Repeater는 파일 버전이 같지만 해시와 크기가 다르다.

| 파일 | 버전 | 크기 | SHA-256 |
| --- | --- | --- | --- |
| 설치 파일의 `repeater.exe` | 1.6.0.0 | 698,448바이트 | `094139d80f1b08f5bb7cbd5222c4a40996cf1bb48154e4920396d2873314ce3b` |
| 고정 `repeater_v160.zip`의 `repeater.exe` | 1.6.0.0 | 524,904바이트 | `c17655192a0bdac369178156227f6cf6bbfa638514996fd286279f3dee712ae0` |

두 파일 모두 Authenticode 서명이 유효했다. 관찰에는 고정 파일을 썼다. 실행 파일의 실제 경로와 해시를 함께 확인해야 한다. 시작 메뉴의 "UltraVNC Repeater" 바로 가기는 설치 파일의 Repeater를 연다.

| 도구 | 버전 | 배포 주소 | 설치 파일 SHA-256 |
| --- | --- | --- | --- |
| Wireshark(`Wireshark.exe`·`dumpcap.exe`·`tshark.exe`) | 4.6.9 x64 | `https://2.na.dl.wireshark.org/win64/all-versions/Wireshark-4.6.9-x64.msi` | `1975dfc96581a9b674fda3f41ac1585963b4ba52ea9459f3b6d9cedfac3a7217` |
| Npcap 무료판 | 1.90 | `https://npcap.com/dist/npcap-1.90.exe` | `682b57e1c05ef375931975c9d6ded40888f4bfdf9f4b11571e654f962f1199b6` |

Wireshark 해시는 winget 매니페스트 값과 같았다. Npcap은 공식 해시 게시값을 찾지 못했지만 Authenticode 서명이 유효했다(서명자 `Nmap Software LLC`). Npcap 무료 라이선스는 Nmap·Wireshark·Microsoft Defender for Identity와만 쓰면 설치 대수 제한이 없고, 그 밖의 용도는 5대까지다. VM의 실제 설치 상태와 캡처 확인 결과는 3.2절에 있다.

### 3.2 관찰 환경 구성 (2026-11-01)

관찰 환경은 Hyper-V VM 세 대와 인터넷에 연결되지 않은 `Private` 가상 스위치다. 캡처는 Repeater VM 안에서 했다. 호스트 루프백 방식이나 캡처 전용 VM은 쓰지 않았다. 게스트 OS는 Windows 11 Enterprise 90일 평가판이다. 관리자 권한이 필요한 스크립트는 사용자가 실행마다 UAC를 승인했다. 계정 그룹은 바꾸지 않았다.

#### 구성

| VM | 컴퓨터 이름 | IPv4 | IPv6 | 역할과 설치 |
| --- | --- | --- | --- | --- |
| `vr-viewer` | `VR-VIEWER` | `192.0.2.10/24` | `2001:db8:1::10/64` | UltraVNC 1.8.3.0 Viewer 구성 요소만 설치한다. noVNC 1.7.0 압축 파일을 둔다 |
| `vr-repeater` | `VR-REPEATER` | `192.0.2.20/24` | `2001:db8:1::20/64` | 고정 `repeater.exe`(2.2절)를 `C:\vr\repeater\`에 둔다. Wireshark 4.6.9와 Npcap 1.90으로 캡처한다 |
| `vr-server` | `VR-SERVER` | `192.0.2.30/24` | `2001:db8:1::30/64` | UltraVNC 1.8.3.0 Server 구성 요소만 설치한다 |

- 네트워크: Hyper-V `Private` 스위치 `vr-lab`. 호스트 어댑터·게이트웨이·DNS·DHCP가 없다. 주소는 문서용 대역(`192.0.2.0/24`, `2001:db8::/32`)만 쓴다.
- 이름 표기 시나리오: Repeater VM의 `hosts` 파일에 `vnc-server.test`(IPv4)와 `vnc-server6.test`(IPv6)를 Server VM 주소로 등록한다. `.test`는 시험용 예약 최상위 도메인이다.
- VM 사양: 2세대, 2 vCPU, 동적 메모리(시작·최소 2 GB, 최대 4 GB), Secure Boot(`MicrosoftWindows` 템플릿), vTPM, 자동 체크포인트 끔.
- 디스크: 평가판 ISO의 이미지를 기본 VHDX 한 개에 적용하고, VM마다 차등 VHDX를 만든다. 기본 VHDX는 읽기 전용으로 둔다. 무인 설정 파일(`unattend.xml`)이 컴퓨터 이름, 시험 전용 로컬 관리자 계정 `lab`, 자동 로그온을 정한다. UltraVNC Server와 Viewer는 로그온한 화면 세션에서 실행해야 하므로 자동 로그온을 켠다.
- 저장 위치: VM과 디스크는 `D:\vr-lab\`, 캡처 원본은 `D:\vr-captures\`를 썼다. 둘 다 저장소 밖이다.
- 인증 정보: `lab` 계정 비밀번호는 이 환경에서만 쓰는 무작위 값이다. 실제 인증 정보가 아니다. 저장소 밖 `D:\vr-lab\secrets\lab-account.txt`에 둔다.
- 파일 반입·반출: 호스트에서 PowerShell Direct(`New-PSSession -VMName`, `Copy-Item -ToSession`·`-FromSession`)를 쓴다. PowerShell Direct는 네트워크가 아닌 VMBus로 통신하므로 분리 상태를 깨지 않는다.
- 회차 독립성: 구성과 스모크 확인을 마친 뒤 VM 세 대를 끄고 체크포인트 `baseline`을 만든다. 시나리오 회차마다 이 체크포인트로 되돌린다.
- 구성 스크립트: `D:\vr-lab\scripts\`에 둔다. 실행 기록은 `D:\vr-lab\logs\`에 남는다.
- 환경 보존 상태(2026-11-14 확인): VM 세 대의 Hyper-V 등록, `vr-lab` 스위치, 기본·차등 VHDX와 체크포인트, 평가판 ISO는 없다. 구성 스크립트·실행 기록·반입 파일·인증 정보 파일은 같은 위치에 남아 있다. 재검증에는 아래 ISO의 해시 확인과 VM 재구성이 필요하다. 남은 스크립트로 재구성하는 과정은 확인하지 않았다.

#### 게스트 이미지와 반입 파일

| 파일 | 받은 곳 | 크기 | SHA-256 | 확인 |
| --- | --- | --- | --- | --- |
| `26300.9457.260913-1737.26h2_ge_release_svc_refresh_CLIENTENTERPRISEEVAL_OEMRET_x64FRE_en-us.iso`(저장 이름 `win11-enterprise-eval-en-us.iso`) | [Evaluation Center](https://www.microsoft.com/en-us/evalcenter/download-windows-11-enterprise)의 English (United States) Enterprise 64-bit 링크. 2026-11-01에 받았다 | 8,225,329,152바이트 | `bc3f24086ebadc94489066b5ad78089e2cf5c3491e90e790bb81a2b199c10e38` | Microsoft 게시값(KB5130867, "Enterprise Eval x64 Eval EN-US DVD9")과 같다 |
| `UltraVNC_1830_x64_Setup.exe` | 2.1절 주소. 2026-11-01에 다시 받았다 | - | `e9c22419ef3128a707b0d004a2c58e25459528c5c12d104dea56e5e662bd131e` | 2.1절 고정값과 같다 |
| `repeater_v160.zip` | 3.1절 보관본 사본 | - | `82d9e7f06266a291b4988e36f4d33e127d63adb93269f3b24c432718f35de1de` | 2.2절 고정값과 같다 |
| `novnc-v1.7.0.zip` | 2.4절 주소. 2026-11-01에 다시 받았다 | - | `d55e9cce9a611294df33c7200f32f35108652de2d86a468bebcfc6f2a1c9a196` | 2.4절 고정값과 같다 |
| `Wireshark-4.6.9-x64.msi` | 3.1절 주소. 2026-11-01에 다시 받았다 | - | `1975dfc96581a9b674fda3f41ac1585963b4ba52ea9459f3b6d9cedfac3a7217` | 3.1절 winget 매니페스트 값과 같다 |
| `npcap-1.90.exe` | 3.1절 주소. 2026-11-01에 다시 받았다 | - | `682b57e1c05ef375931975c9d6ded40888f4bfdf9f4b11571e654f962f1199b6` | 3.1절 값과 같다 |
| `vc_redist.x64.exe` 14.44.35211.0 | `https://aka.ms/vs/17/release/vc_redist.x64.exe`. Wireshark MSI가 런타임 없이 실행되지 않을 때만 설치한다 | - | `cc0ff0eb1dc3f5188ae6300faef32bf5beeba4bdd6e8e445a9184072096b713b` | 게시값을 찾지 않았다 |

- 반입 파일은 `D:\vr-lab\installers\`에 둔다. 2026-11-01에 이 표의 다섯 개 고정 파일을 다시 받았고, SHA-256이 모두 기록값과 같았다.

#### 구성 결과 (2026-11-01)

| 항목 | 결과 |
| --- | --- |
| 게스트 OS | 세 VM 모두 `Microsoft Windows 11 Enterprise Evaluation`, 버전 10.0.26300. ISO 이미지 1번(`Windows 11 Enterprise Evaluation`, 10.0.26300.9457, en-US)을 적용했다. 90일 평가판이며, 실제 라이선스 조회 결과는 3.3절에 있다 |
| VM | 세 VM 모두 2세대, 2 vCPU, 동적 메모리 2048~4096 MB, vTPM 켬, Secure Boot 켬. 네트워크 어댑터는 VM마다 한 개이고 `vr-lab`에만 연결된다 |
| 주소 | 구성 표의 IPv4·IPv6 주소와 링크 로컬 주소만 있다. 기본 경로는 VM마다 0개다 |
| `vr-viewer` | `C:\Program Files\uvnc\UltraVNC\vncviewer.exe` 1.8.3.0, SHA-256 `18760910769dc497266d6ba7a28892925177d0208fe4433bf62f0b16061f3427`. 설치 프로그램이 `UltraVNC Viewer TCP`·`UDP` 아웃바운드 허용 규칙을 만들었다. `C:\vr\installers\novnc-v1.7.0.zip`을 두었다 |
| `vr-server` | `C:\Program Files\uvnc\UltraVNC\winvnc.exe` 1.8.3.0, SHA-256 `9ebd1ec90ba1274d4a183f5bf6588374700052da907fa764a2ae7072e8ae19fe`. 설치 프로그램이 `UltraVNC Server TCP`·`UDP` 인바운드 허용 규칙을 만들었다. 서비스는 등록하지 않았다 |
| `vr-repeater` | `C:\vr\repeater\repeater.exe` 1.6.0.0, SHA-256 `c17655192a0bdac369178156227f6cf6bbfa638514996fd286279f3dee712ae0`(2.2절 고정 파일). 방화벽 규칙 `vr-lab repeater`는 이 파일의 인바운드를 `192.0.2.0/24`·`2001:db8:1::/64`에서만 허용한다. `hosts`에 `192.0.2.30 vnc-server.test`, `2001:db8:1::30 vnc-server6.test`가 한 번씩 있다 |
| 캡처 도구 | `vr-repeater`에 Wireshark 4.6.9(MSI, `TShark (Wireshark) 4.6.9 (v4.6.9-0-g2d548b197c75)`)와 Npcap 1.90을 설치했다. Visual C++ 런타임은 따로 설치하지 않았다. Npcap은 사용자가 VM 콘솔에서 설치 창을 기본값으로 진행했다. 드라이버 설정은 `AdminOnly=0`, `WinPcapCompatible=1`, `LoopbackSupport=1`이다. 캡처 대상은 `\Device\NPF_{96B13FFE-E67F-48F3-B027-339FFAF761AA} (vr-lab)`다 |
| 체크포인트 | 세 VM이 꺼진 상태에서 만든 `baseline`(Standard, VM 상태 `Off`)이 VM마다 한 개 있었다 |

#### 스모크 확인 (2026-11-01)

이 확인은 격리와 캡처 도구의 동작 확인이다. UltraVNC를 실행하지 않았으므로 6.2절의 관찰 증거가 아니다.

| 확인 | 결과 |
| --- | --- |
| 스위치·어댑터 | `vr-lab`의 형식이 `Private`이다. VM 어댑터 세 개가 모두 `vr-lab`에 연결된다 |
| 호스트 분리 | 호스트에 `192.0.2.0/24`·`2001:db8:1::/64` 주소가 없다. 호스트에서 `192.0.2.20` ping이 실패했다 |
| 게스트 분리 | 세 VM 모두 기본 경로 0개. 외부 IPv4·IPv6 주소의 443번 TCP 연결이 실패했다. 외부 DNS 조회도 시간 초과로 실패했다 |
| VM 간 통신 | 세 VM이 서로 IPv4·IPv6 ping에 응답했다(VM마다 4/4) |
| 캡처 | `vr-repeater`에서 `dumpcap`(종료 코드 0)으로 `vr-lab` 어댑터를 25초 캡처했다(패킷 17개). 그동안 `vr-viewer`가 TCP 15000번으로 `ping-smoke`(IPv4)와 `ping-smoke6`(IPv6)을 보내고 `pong:` 응답을 받았다. VM 안과 호스트로 옮긴 사본의 `tshark` 결과가 같았다: `192.0.2.10 70696e672d736d6f6b65`, `192.0.2.20 706f6e673a70696e672d736d6f6b65`, `2001:db8:1::10 70696e672d736d6f6b6536`, `2001:db8:1::20 706f6e673a70696e672d736d6f6b6536`. 사본의 SHA-256은 `1d15d6575192645ebb17aac861a2dd836a1ef61c535bd8100e6d9fe66d29852e`이다. 시험 파일과 임시 방화벽 규칙은 지웠다 |

#### 구성 스크립트

스크립트는 저장소 밖 `D:\vr-lab\scripts\`에 있다. 모두 관리자 권한으로 실행한다.

| 파일 | 역할 |
| --- | --- |
| `01-build.ps1`, `unattend-template.xml` | 스위치, 기본 디스크, 차등 디스크, VM 세 대, 게스트 네트워크와 소프트웨어를 구성한다. 다시 실행하면 이미 끝난 단계를 건너뛴다. 설치 파일 해시가 고정값과 다르면 실패로 끝낸다 |
| `02-smoke.ps1` | 분리·통신·캡처 스모크를 실행한다. 조건이 하나라도 다르면 체크포인트를 만들지 않고 실패로 끝낸다. 통과하면 `03-baseline.ps1`을 부른다 |
| `03-baseline.ps1` | VM을 끄고 `baseline`을 만든 뒤 상태를 확인한다 |
| `install-npcap-then-smoke.ps1`, `npcap-ui.ps1`, `open-npcap-installer.ps1` | Npcap 설치 창을 VM 콘솔 세션에 띄우고, 설치를 확인한 뒤 스모크를 실행한다 |
| `check-npcap.ps1`, `status.ps1` | 설치 흔적과 VM·체크포인트 상태를 조회한다 |

### 3.3 관찰 실행 방법 (2026-11-01)

이 절은 6.2절의 "환경"과 "입력" 중 모든 회차에 공통인 사항이다. 시나리오별 입력과 결과는 3.4절, 해석은 3.5절, 원본은 3.6절에 있다.

#### 라이선스 상태와 회차 유효 조건

게스트 Windows는 평가판 알림 모드였다. 각 회차는 `baseline`에서 새로 부팅해 한 시간 안에 끝냈다. 도중에 VM이 꺼지면 그 회차는 무효다. 라이선스 보호는 끄거나 우회하지 않았다. 이 라이선스 상태가 약관에 맞는지는 판단하지 않았다.

- 라이선스 상태(2026-11-01, 세 VM 같음): `slmgr /dli`가 `License Status: Notification`, `Notification Reason: 0xC004F009 (grace time expired).`를 출력했다. `slmgr /xpr`는 `Windows is in Notification mode`를 출력했다. 결과 파일은 `D:\vr-lab\logs\04-explore.json`이다.
- 회차 길이: 부팅부터 종료까지 회차당 약 4~6분이었다. 모든 증거 회차가 한 시간 안에 끝났고, 도중에 꺼진 VM은 없었다.

#### 회차 진행

하네스 `05-observe.ps1`이 계획 파일(JSON) 하나를 받아 시나리오마다 독립 회차를 2회 실행한다. 회차는 다음 순서로 진행한다.

1. 세 VM을 `baseline`으로 되돌리고 시작한다.
2. Repeater VM에서 `C:\vr\repeater\repeater.exe`를 처음 실행한다. 첫 실행 때 Repeater는 "UltraVNC Repeater - Admin Password" 모달 창을 띄운다. 창에는 "first run configuration saved", 웹 관리 주소 `http://localhost:80/`, 사용자 `admin`, 무작위 비밀번호가 있다. 이 창이 열려 있는 동안 Repeater는 포트를 열지 않는다. 하네스는 UI 자동화로 비밀번호를 읽어 메모리에만 둔다. 기록에는 창 제목과 읽기 성공 여부만 남기고, 이 창의 화면은 캡처하지 않는다.
3. 시나리오에 설정 변경이 있으면 웹 관리 화면의 저장 요청(`/testaction.cgi?...`, GET 한 번)으로 바꾼다. 이 요청에는 HTTP 응답이 오지 않고, 그 뒤 웹 서버도 응답하지 않는다. 그래도 `settings2.txt`는 다시 쓰인다. 그래서 하네스는 이 파일의 해시가 바뀔 때까지 최대 30초 기다린 뒤 Repeater를 강제 종료한다. `taskkill`(강제 옵션 없음)로는 종료되지 않았다.
4. `dumpcap -i <vr-lab 어댑터> -a duration:<captureSec> -w capture.pcapng`로 캡처를 시작한다. 2초 뒤 Repeater를 다시 시작한다. `/settings.html`에서 실제 설정을 읽어 `run.json`의 `repeaterSettings`에 기록한다. 계획의 값과 다르면 그 회차는 실패로 끝난다.
5. 계획의 단계를 순서대로 실행한다. 단계에는 실제 Viewer·Server 실행(자동 로그온한 화면 세션), 원시 TCP 클라이언트, 프로세스 종료, 상태 수집, 스크린숏, 대기가 있다. 상태 수집은 세 VM의 프로세스와 TCP 연결 상태, Repeater 웹 상태 페이지(`/`)를 남긴다. 웹 요청을 빼는 상태 수집(`noWeb`)도 있다.
6. 캡처가 끝나면 원본을 호스트로 복사하고 SHA-256을 계산한다.
7. 회차는 오류가 없고, 세 VM이 회차 내내 실행 중이었고(가동 시간 비교), `dumpcap`이 종료 코드 0으로 끝났을 때만 유효하다. 무효 회차는 한 번 다시 실행한다. 증거 회차는 모두 첫 시도에서 유효했다(3.6절).

- 원시 TCP 클라이언트: 게스트 PowerShell 스크립트로 Repeater에 접속해 계획에 적힌 바이트를 정해진 간격으로 보낸다. 받은 바이트와 시각(접속 시작 기준 ms)을 `raw-*.txt`에 남긴다. 실제 Viewer·Server가 만들 수 없는 입력(형식 오류, 분할, 경계 ID, 같은 역할 중복)에 쓴다. 3.4절에서 "패딩"은 250바이트가 될 때까지 NUL(0x00)을 채운 것이다. "표식"은 매칭 뒤 전달 여부를 판정하려고 보낸 짧은 바이트열(예: `S>after`, `V>after`)이다.
- Repeater 대기 포트: 매 회차 `0.0.0.0:5901`, `0.0.0.0:5500`, `0.0.0.0:80`만 대기했다. IPv6 대기 포트는 없다. Viewer는 5901, Server는 5500으로 접속했다.
- Repeater 기본 설정(첫 실행 뒤 `/settings.html`): `keepalive=False mode1=False mode2=True server_port=5901 viewer_port=5500 allow_on=True allow_con=(빈 값) refuse_on=False refuse_con=(빈 값) id_on=False id_con=(빈 값) web_port=80 ucom_on=False`. 필드 이름은 화면의 이름 그대로다.
- Mode I 시나리오(M1-01~M1-06)는 `mode1=True`, `allow_on=False`로 바꿨다. M1-04는 `refuse_on=True`, `refuse_con=192.0.2.30`도 바꿨다. 나머지 시나리오는 기본 설정이다.
- 회차 비교: `summarize.py`가 시나리오마다 `summary.md`를 만든다. TCP 스트림마다 방향별 바이트 총량, 앞 300바이트, FIN·RST 순서를 비교한다. 세그먼트 분할은 비교하지 않는다. 다음 차이는 같은 결과로 본다. Viewer가 보내는 250바이트 메시지의 NUL 뒤 바이트, 보안 유형 선택 뒤 Server가 보내는 16바이트(회차마다 다름), 화면 갱신 데이터의 크기, TLS ClientHello의 크기, 세그먼트 분할, 양쪽 FIN의 순서 경쟁, Repeater가 FIN·RST로 닫은 뒤 상대의 FIN이 캡처에 남았는지 여부.
- TCP keepalive 탐침: tshark가 `tcp.analysis.keep_alive`로 표시한 1바이트 세그먼트는 스트림 데이터에서 뺐다. `summary.md`에는 플래그 `KA`로 남긴다. winvnc 연결에서 약 10초마다, Edge 연결에서 약 45초마다 보였다. 이 탐침은 이미 확인된 바이트를 다시 보내는 것이며 상대에게 전달되지 않는다.

#### 하네스 파일

스크립트는 저장소 밖 `D:\vr-lab\scripts\`에, 계획은 `D:\vr-lab\plans\`에, 실행 요약은 `D:\vr-lab\logs\05-<계획>.json`에 있다. 관리자 권한이 필요한 실행은 사용자가 UAC를 승인했다.

| 파일 | 역할 |
| --- | --- |
| `05-observe.ps1` | 계획 하나를 실행한다. 위 회차 진행 1~7단계 |
| `06-observe-many.ps1` | 여러 계획을 UAC 승인 한 번으로 차례로 실행한다 |
| `lab-common.ps1`, `uia-dump.ps1` | VM 세션, 화면 세션 실행, 파일 복사, UI 자동화 창 읽기 |
| `guest\ws-lab.ps1` | WebSocket 시나리오용. `-Mode static`은 noVNC 정적 파일 서버, `-Mode bridge`는 WS·WSS를 받아 TCP로 중계하며 헤더와 바이트를 기록하는 시험용 중계기다 |
| `guest\ws-setup.ps1` | WebSocket 시나리오용. Repeater VM의 방화벽·자체 서명 인증서, Viewer VM의 noVNC 압축 해제(해시 확인)와 Edge 버전 확인 |
| `summarize.py` | 캡처를 스트림별 사건표로 바꾸고 두 회차를 비교한다 |

### 3.4 시나리오별 입력과 결과

시각은 해당 연결의 TCP 연결 시점 기준이다. 모든 행은 2회 실행했고, 3.3절의 무시할 차이를 빼면 두 회차의 결과가 같았다. 다른 점이 있으면 행에 적었다. 실행 파일은 Viewer `vncviewer.exe` 1.8.3.0, Server `winvnc.exe` 1.8.3.0, Repeater `repeater.exe` 1.6.0.0이다(경로와 해시는 3.2절).

#### Mode I (p10)

| 시나리오 | 입력 | 결과 |
| --- | --- | --- |
| M1-00 Mode I 꺼짐(기본 설정) | Viewer 하나를 Repeater Viewer 포트로 접속시켰다. 명령줄 원문은 남지 않았다. [INFERENCE: 와이어의 메시지가 M1-01과 같으므로 `-connect 192.0.2.30::5900 -proxy 192.0.2.20:5901`로 보인다] | 약 1.0초 뒤 Repeater가 `RFB 000.000\n`(12바이트)을 보낸다. Viewer는 같은 ms에 250바이트(`192.0.2.30:5900` + NUL + 정리되지 않은 바이트)를 보낸다. Repeater는 대상에 연결하지 않고, 더 보내지 않으며, 닫지도 않았다(40초 넘게 연결 유지). 웹 상태 페이지의 대기 목록에도 없다 |
| M1-01 IP와 포트 | Viewer `-connect 192.0.2.30::5900 -proxy 192.0.2.20:5901`, Server `winvnc -run` | Viewer 메시지는 `192.0.2.30:5900` + NUL(오프셋 15) + 정리되지 않은 바이트, 모두 250바이트다. Repeater는 메시지를 받고 약 1ms 뒤 `192.0.2.30:5900`으로 연결한다. 250바이트 메시지는 Server에 전달하지 않는다. Server의 `RFB 003.008\n`부터 양방향으로 중계한다 |
| M1-02 포트 생략 | Viewer `-connect 192.0.2.30 -proxy 192.0.2.20:5901`. 이어서 원시 클라이언트가 `192.0.2.30` + 패딩을 보냄 | Viewer는 포트를 붙여 `192.0.2.30:5900`을 보낸다. 원시 입력 `192.0.2.30`에도 Repeater는 `192.0.2.30:5900`으로 연결한다. Viewer 종료 뒤 Repeater는 Server에 FIN을 보내고 약 3ms 뒤 RST를 보냈다(두 회차 같음) |
| M1-03 이름 | Viewer `-connect vnc-server.test::5900 -proxy 192.0.2.20:5901` | Viewer는 대문자로 바꾼 `VNC-SERVER.TEST:5900`을 보낸다(NUL은 오프셋 20). Repeater는 `hosts`로 이름을 풀어 약 115~165ms 뒤 `192.0.2.30:5900`으로 연결한다 |
| M1-04 금지 대상 | M1-01과 같은 명령. 설정 `refuse_on=True`, `refuse_con=192.0.2.30` | 메시지를 받은 그 ms에 Repeater가 Viewer에 FIN을 보낸다. 오류 바이트는 없다. 대상에 연결하지 않는다 |
| M1-05 연결 실패 | Viewer `-connect 192.0.2.30::5999 -proxy 192.0.2.20:5901` | Repeater가 `192.0.2.30:5999`로 SYN을 보내고 1·2·4·8·16초 간격으로 다시 보낸다. 응답은 없다. 연결 시도 약 21초 뒤 Repeater가 Viewer에 FIN을 보낸다. 오류 바이트는 없다 |
| M1-06 IPv6 대상 | Viewer `-connect 2001:db8:1::30::5900 -proxy 192.0.2.20:5901`. 이어서 원시 입력 `2001:db8:1::30:5900`, `[2001:db8:1::30]:5900`, `vnc-server6.test:5900`(각각 + 패딩) | Viewer는 프로세스만 실행되고 창과 연결이 없다. 원시 입력 세 가지에는 모두 Repeater가 메시지를 받은 즉시 FIN을 보낸다. IPv6 연결 시도는 없다 |

#### Mode II (p11, p15, p17)

Server 명령은 `winvnc -id:<ID> -connect 192.0.2.20::5500 -run`, Viewer 명령은 `vncviewer -proxy 192.0.2.20:5901 ID:<ID>`다. 원시 쌍은 Server 쪽을 먼저 접속하고 약 1초 뒤 Viewer 쪽을 접속한다. 원시 Viewer는 `RFB 000.000\n`을 받은 뒤 250바이트를 보내고, 매칭 판정용 표식을 주고받는다.

| 시나리오 | 입력 | 결과 |
| --- | --- | --- |
| M2-01 Server 선접속 | ID `1234`. Server 실행, 약 6초 뒤 Viewer 실행 | Server는 연결 직후 250바이트(`ID:1234` + NUL 243개)를 보내고, 30~120ms 뒤 `RFB 003.008\n`을 보낸다. Repeater는 매칭 전에 Server에 아무것도 보내지 않는다. Viewer는 접속 약 1.0초 뒤 `RFB 000.000\n`을 받고 같은 ms에 250바이트(`ID:1234` + NUL + 정리되지 않은 바이트)를 보낸다. 약 1.0초 뒤 매칭되고, Viewer가 처음 받는 바이트는 Server의 `RFB 003.008\n`이다. 두 250바이트 메시지는 상대에게 전달되지 않는다 |
| M2-02 Viewer 선접속 | ID `1234`. Viewer 실행, 약 13초 뒤 Server 실행 | Server 메시지 도착 약 2.0초 뒤 매칭된다. 나머지는 M2-01과 같다 |
| M2-03 서로 다른 ID | Server `1234`, Viewer `5678` | 60초 넘게 둘 다 연결된 채 대기한다. 상태 페이지의 대기 Server 목록에 `1234`, 대기 Viewer 목록에 `5678`이 있다. Server 쪽 연결에는 약 10초마다 1바이트 세그먼트가 보이지만, tshark가 TCP keepalive 탐침(`tcp.analysis.keep_alive`)으로 표시한다. 스트림 데이터가 아니다 |
| M2-04 Server 중복 ID | winvnc `1234` 접속 뒤 원시 Server `ID:1234` + 패딩, 이어서 Viewer `1234` | 두 번째 Server의 TCP 연결은 닫히지 않고 유지된다. 상태 페이지의 대기 Server 목록에는 하나만 있다. Viewer는 먼저 등록한 winvnc와 매칭된다. 원시 Server의 표식은 어디에도 전달되지 않는다 |
| M2-05 Viewer 중복 ID | Viewer `1234` 접속 뒤 원시 Viewer `ID:1234` + 패딩, 이어서 Server `1234` | 두 번째 Viewer도 `RFB 000.000\n`을 받고 연결이 유지된다. 대기 Viewer 목록에는 먼저 온 Viewer 하나만 있다. Server는 먼저 온 Viewer와 매칭된다. 두 번째 Viewer의 표식은 전달되지 않는다 |
| M2-06 ID 길이 | 원시 쌍, 양쪽 같은 값: `ID:1`, `ID:123456789`, `ID:1234567890` | 세 쌍 모두 매칭된다. 10자리 ID와 웹 상태 요청의 결과는 M2-11, 20자리·247자리·빈 ID·접두어 없는 입력은 M2-06b에 있다 |
| M2-06b ID 경계(사례마다 Repeater 재시작) | 원시 쌍, 양쪽 같은 값: `ID:` + 20자리, `ID:`(빈 ID), `1234`(접두어 없음), `ID:` + `7` 247개(NUL 없이 250바이트를 채움) | 20자리와 247자리 쌍은 매칭된다. 빈 ID는 Server 쪽이 접속 약 1.0초 뒤, Viewer 쪽이 메시지를 보낸 직후 FIN으로 닫힌다. 접두어 없는 쌍은 매칭되지 않고, 닫히지도 않은 채 클라이언트가 닫을 때까지 유지된다. 각 사례 뒤 Repeater 프로세스는 살아 있었다 |
| M2-07 ID 문자 | 원시 쌍, 양쪽 같은 값: `ID:ABCD`, `ID:abcd`, `ID:12-34`, `ID:12 34`, `ID:12` + 0xE9 + `34` | `ID:ABCD`와 `ID:abcd`는 Server 쪽이 접속 약 1.0초 뒤, Viewer 쪽이 메시지를 보낸 직후 FIN으로 닫힌다. 나머지 세 쌍은 매칭된다. 매칭된 세 쌍의 상태 페이지 "ID nummer"는 모두 `5912`였다 |
| M2-08 앞자리 0 | 원시 쌍: `ID:0123`/`ID:123`, `ID:789`/`ID:0789`, `ID:0456`/`ID:0456` | 세 쌍 모두 매칭된다. 상태 페이지 "ID nummer"는 각각 `123`, `789`, `456`이다. 같은 실행에서 `ID:1`은 `5901`, `ID:123456789`는 `123456789`로 표시됐다(M2-06) |
| M2-09 대소문자만 다름 | 원시 쌍: `ID:ab12`/`ID:AB12`, `ID:cd34`/`ID:cd34` | 두 쌍 모두 M2-07의 영문 ID처럼 닫힌다. 같은 값(`cd34`)끼리도 닫힌다 |
| M2-11 10자리 ID와 웹 상태 페이지 | 원시 쌍 `ID:1234567890` 두 번. A: 세션 동안 웹 요청 없음. Repeater 재시작. B: 세션 동안 웹 상태 페이지 요청 | A는 매칭되고 정상 종료되며, 세션이 끝난 뒤의 웹 요청에도 응답한다. B는 웹 요청에 응답이 없고, 두 연결이 RST로 끊긴다. 다음 상태 수집(run1 약 6초 뒤, run2 약 2초 뒤) 때 Repeater 프로세스가 없었다 |
| M2-12 숫자 앞부분 | 원시 쌍(사례마다 재시작): `ID:12ab`/`ID:12AB`, `ID:56-78`/`ID:56`, `ID:999999999`/`ID:999999999` | 앞의 두 쌍은 매칭된다. `ID:999999999` 쌍은 Server 쪽이 접속 약 1.0초 뒤 FIN으로 닫히고, Viewer 쪽은 대기 상태로 유지된다 |
| M2-13 큰 수와 `999999999` | 원시 쌍(사례마다 재시작): `ID:12345678901234567890`/`ID:98765432109876543210`, `ID:2147483647`/`ID:2147483648`, `ID:999999998`/같은 값, `ID:99999999`/같은 값. 마지막으로 Viewer `ID:999999999` 선접속 약 3초 뒤 Server `ID:999999999` | 서로 다른 20자리 쌍은 매칭된다. `2147483647`/`2147483648`은 매칭되지 않고 둘 다 클라이언트가 닫을 때까지 대기한다. `999999998`과 `99999999` 쌍은 매칭된다. Viewer가 `999999999`로 대기하는 중에 같은 ID의 Server가 오면, Server 접속 약 2.0초 뒤 두 연결이 함께 FIN으로 닫힌다 |
| M2-14 100 미만 ID | 원시 쌍(사례마다 재시작, 매칭 중 웹 상태 페이지 요청): `ID:12`/`ID:5912`, `ID:99`/`ID:5999`, `ID:100`/`ID:6000`, `ID:0`/`ID:5900` | `12`/`5912`, `99`/`5999`, `0`/`5900` 쌍은 매칭된다. `100`/`6000` 쌍은 매칭되지 않고, 상태 페이지의 대기 Server 목록에 `100`이 있다. 매칭 중 상태 페이지의 "ID nummer"는 `99` 쌍이 `5999`, `0` 쌍이 `5900`이었다. `12` 쌍은 표식이 오갔는데도 상태 페이지에 연결이 없고 대기 Viewer `5912`만 있었다(두 회차 같음) |

#### 오류 (p12)

모두 원시 클라이언트로 입력했다. 설정은 기본값(Mode I 꺼짐)이다.

| 시나리오 | 입력 | 결과 |
| --- | --- | --- |
| E-01 형식 오류 | Server 포트에 `XX:1234` + 패딩. Viewer 포트에 `A` 250개 | 둘 다 닫히지 않고 클라이언트가 닫을 때까지(약 15초) 유지된다. 응답 바이트는 없다(Viewer 쪽은 `RFB 000.000\n`만 받음) |
| E-02 길이 부족 뒤 단절 | 양쪽 포트에 `ID:12`(5바이트)만 보내고 약 1초 뒤 클라이언트가 닫음. 이어서 정상 쌍 `ID:4321` | 클라이언트 FIN 1~2ms 뒤 Repeater도 FIN을 보낸다. 이어진 정상 쌍은 매칭되고 표식이 전달된다 |
| E-03 길이 초과 | Server `ID:1234` + 패딩 300바이트와 Viewer 250바이트, 이어서 Server 250바이트와 Viewer `ID:5678` + 패딩 300바이트 | 두 경우 모두 매칭된다. 앞 250바이트만 초기 메시지로 쓰고, 나머지 50바이트(NUL)는 매칭 순간 상대에게 첫 바이트로 전달된다. 표식도 전달된다. 종료 때 양쪽 FIN의 순서는 회차마다 달랐다(경쟁) |
| E-04 시간 초과 | 양쪽 포트에 접속만 하고 보내지 않음. 이어서 양쪽에 `ID:1234` + 패딩의 앞 100바이트만 보냄 | 아무것도 보내지 않으면 접속 약 6.0초 뒤 Repeater가 FIN을 보낸다. Viewer 쪽은 그 전에 `RFB 000.000\n`을 받는다. 100바이트만 보내면 Server 쪽은 접속 약 6.0초 뒤, Viewer 쪽은 100바이트를 보내고 약 5.0초 뒤(접속 약 6.6초 뒤) FIN이 온다. 응답 바이트는 없다 |

#### 분할·후속 바이트 (p12, p15, p17)

| 시나리오 | 입력 | 결과 |
| --- | --- | --- |
| S-01 초기 메시지 분할 | Server `ID:1234` + 패딩을 3·97·150바이트로 약 0.4초 간격에 나눠 보냄. Viewer는 5·245바이트로 나눔 | 매칭되고 표식이 전달된다 |
| S-02 초기 메시지와 후속 바이트를 함께 | Server는 250바이트 + `RFB 003.008\n`을 한 번에(262바이트), Viewer는 250바이트 + `V>combined`를 한 번에(260바이트) 보냄 | 매칭 순간 Viewer는 `RFB 003.008\n`을, Server는 `V>combined`를 첫 바이트로 받는다 |
| S-03 매칭 전후 바이트 | Server 선접속 뒤 `S>before`, 약 5초 뒤 Viewer 접속, 매칭 뒤 양쪽 `after` 표식. 이어서 Viewer 선접속으로 같은 순서 | 매칭 전에 보낸 `S>before`·`V>before`가 매칭 순간 상대에게 먼저 전달되고, 그 뒤 매칭 뒤 바이트가 전달된다 |
| S-04 매칭 전 `00` 1바이트 | 원시 쌍 네 개: Server가 등록 약 2초 뒤 단독 `00`, `00` 뒤 1초 간격 `S>b`, `00`과 `S>c`를 한 번에. 마지막은 Viewer 선접속 뒤 Viewer가 단독 `00` | 네 경우 모두 `00`이 매칭 순간 상대에게 전달된다. 순서도 보낸 순서와 같다 |
| S-05 오래 대기한 뒤의 매칭 전 바이트 | Server 선접속 뒤 약 2초와 14초에 `S>early`, `S>late`를 보내고, 약 22~24초 뒤 Viewer 접속. Repeater 재시작 뒤 Viewer 선접속으로 같은 순서 | 두 경우 모두 매칭 순간 `S>earlyS>late`, `V>earlyV>late`가 한 세그먼트로 상대에게 전달되고, 그 뒤 매칭 뒤 바이트가 전달된다. 첫 쌍의 종료는 회차마다 달랐다. run1은 Repeater가 두 연결에 같은 ms에 RST를 보냈고(캡처 기준 36.392초, 마지막 데이터 약 6.1초 뒤), run2는 Server 쪽 클라이언트가 먼저 FIN을 보냈다. [INFERENCE: run1의 RST는 하네스의 Repeater 재시작(프로세스 종료)과 겹친 것으로 보인다. RST의 VM 시각 16:04:16.924는 재시작 직전 상태 수집의 호스트 시각 16:04:16.708 직후다. 재시작 단계의 기록 시각 16:04:20.093은 재시작이 끝난 뒤의 시각이라 종료 시각이 아니다. 호스트와 VM 시계 차이는 측정하지 않았으므로 원인은 확정하지 않았다] |

#### 종료·재접속 (p13)

| 시나리오 | 입력 | 결과 |
| --- | --- | --- |
| T-01 Viewer 종료 | winvnc `1234`와 Viewer `1234`를 매칭한 뒤 Viewer 프로세스 강제 종료 | Viewer 쪽 연결이 RST로 끝나고 약 1ms 뒤 Repeater가 Server에 FIN을 보낸다. winvnc(`-autoreconnect` 없음)는 다시 접속하지 않는다 |
| T-02 Server 종료 | Viewer `1234`와 원시 Server `ID:1234` + 패딩을 매칭한 뒤 원시 Server가 닫음 | Server FIN과 같은 ms에 Repeater가 Viewer에 FIN을 보낸다. 바이트는 없다 |
| T-03 Server `-autoreconnect`와 Viewer 재접속 | Server `winvnc -autoreconnect ID:1234 -connect 192.0.2.20::5500 -run`, Viewer `1234` 매칭, Viewer 강제 종료, 약 20초 뒤 Viewer 다시 실행 | 이 모드의 Server 메시지는 `ID:1234` + NUL + 0이 아닌 바이트다. Viewer 종료 뒤 Repeater가 Server에 FIN을 보내고 약 3ms 뒤 RST를 보낸다. Server는 약 2초 뒤 같은 ID로 다시 등록한다. 다시 실행한 Viewer는 이 Server와 매칭된다. 두 회차의 차이는 Server 쪽 종료 순서 하나다(run2에만 Server의 FIN이 있음) |

#### 외부 WebSocket (p16, p18)

공식 Repeater 1.6.0.0에는 WebSocket 대기 포트가 없다(3.3절). 그래서 두 가지를 관찰했다. 하나는 noVNC가 Repeater의 Viewer 포트에 직접 접속하는 경우(W-01·W-01b)다. 다른 하나는 시험용 중계기 `ws-lab.ps1 -Mode bridge`를 Repeater VM에 두고, WebSocket 메시지의 데이터를 `127.0.0.1:5901`로 그대로 옮기는 경우(W-02·W-03b)다. 시험용 중계기는 websockify가 아니다. 이 프로젝트가 관찰용으로 만든 스크립트이며, 요청 헤더 전체와 양방향 바이트를 기록한다. 이 중계기의 동작은 `vnc-relay`의 설계가 아니다.

- 클라이언트: noVNC 1.7.0(2.4절 압축 파일, Viewer VM에서 해시 확인 뒤 압축 해제), Microsoft Edge 152.0.4191.66 headless(`--headless=new --remote-debugging-port=9222 --ignore-certificate-errors`, 회차마다 새 사용자 프로필). noVNC 정적 파일은 Viewer VM의 `http://127.0.0.1:8000/`에서 제공했다.
- Server: `winvnc -id:1234 -connect 192.0.2.20::5500 -run`.
- WSS 인증서: 회차마다 Repeater VM에서 만든 자체 서명 인증서(`CN=vr-repeater.test`)를 `0.0.0.0:8443`에 연결했다.
- noVNC 연결 완료 판정: 시험용 중계기 로그의 RFB 바이트로 판정한다. noVNC가 ServerInit(데스크톱 이름 포함)을 받고, SetPixelFormat·SetEncodings·FramebufferUpdateRequest를 보낸 뒤 화면 갱신 데이터를 받아야 한다. 원격 디버깅 대상 목록(`/json`)의 페이지 제목도 보조 근거로 썼다. 연결된 회차는 `vr-server ( 192.0.2.30 ) - application mode - noVNC`, 연결되지 않은 회차는 `noVNC`였다.

| 시나리오 | 입력 | 결과 |
| --- | --- | --- |
| W-01 noVNC → Repeater 직접(`ws://`) | `vnc.html?autoconnect=true&reconnect=false&repeaterID=1234&path=ws%3A%2F%2F192.0.2.20%3A5901%2F` | noVNC가 HTTP 업그레이드 요청(`GET / HTTP/1.1`, 510바이트)을 보낸다. Repeater는 이를 해석하지 않고, 접속 약 1.0초 뒤 `RFB 000.000\n`을 보낸다. noVNC는 0~42ms 안에 FIN으로 닫는다. Repeater는 닫지 않았다. 페이지 제목은 `noVNC`로 남았다 |
| W-02 noVNC → WS 중계기 → Repeater | `path=ws%3A%2F%2F192.0.2.20%3A8081%2Fwebsockify` | 요청 헤더에 `Sec-WebSocket-Protocol`이 없다. 중계기는 서브프로토콜 없이 수락했다. noVNC는 먼저 보내지 않는다. 중계기가 TCP 연결 약 1.0초 뒤 `RFB 000.000\n`을 바이너리 메시지 하나로 전달하자, noVNC는 67~85ms 뒤 바이너리 메시지 하나로 250바이트(`ID:1234` + NUL 243개)를 보냈다. 약 1.0초 뒤 winvnc와 매칭되어 Server의 `RFB 003.008\n`이 전달됐다. noVNC는 보안 유형 목록 `03 11 75 01`에서 `01`(None)을 골랐고, ServerInit(이름 `vr-server ( 192.0.2.30 ) - application mode`)을 받은 뒤 SetPixelFormat·SetEncodings·FramebufferUpdateRequest를 보내고 화면 갱신 데이터를 받았다. 페이지 제목도 데스크톱 이름으로 바뀌었다. Edge 종료 뒤 Repeater가 Server에 FIN을 보냈다 |
| W-01b noVNC → Repeater 직접(WSS) | `vnc.html?autoconnect=true&reconnect=false&repeaterID=1234&host=192.0.2.20&port=5901&encrypt=true&path=websockify` | 각 회차에서 Viewer→Repeater 5901 TCP 연결이 두 개 관찰됐다. 두 연결 모두 같은 순서였다. 브라우저가 TLS ClientHello(1729~1793바이트)를 보낸다. Repeater는 접속 약 1.0초 뒤 평문 `RFB 000.000\n`을 보낸다. 브라우저는 2~18ms 안에 TLS 경고 레코드 7바이트(`15 03 01 00 02 02 46`, fatal `protocol_version`)를 보내고 FIN으로 닫는다. Repeater는 캡처가 끝날 때까지(약 70초) 이 연결을 닫지 않았다. 페이지 제목은 `noVNC`로 남았다. 연결이 두 개인 이유는 확인하지 않았다 |
| W-03b noVNC → WSS 중계기 → Repeater | `host=192.0.2.20&port=8443&encrypt=true&path=websockify` | TLS 접속이 성립했다(중계기 로그 `scheme=https`). 요청 헤더에 `Sec-WebSocket-Protocol`이 없다. 이후 순서는 W-02와 같다. 중계기가 `RFB 000.000\n`을 전달하자 noVNC는 84~110ms 뒤 바이너리 메시지 하나로 250바이트(`ID:1234` + NUL 243개)를 보냈다. 매칭 뒤 보안 유형 `01`(None)을 고르고, ServerInit을 받은 뒤 SetPixelFormat·SetEncodings·FramebufferUpdateRequest를 보내고 화면 갱신 데이터를 받았다. 페이지 제목은 `vr-server ( 192.0.2.30 ) - application mode - noVNC`로 바뀌었다. Edge 종료 때 TCP RST가 났고, 약 21~31ms 뒤 Repeater가 Server에 FIN을 보냈다 |

noVNC 1.7.0의 페이지가 `http:`일 때 절대 URL `path`의 `wss` scheme으로는 TLS 접속이 되지 않았다. `encrypt=true`를 함께 줘도 평문이었다. WSS 증거는 `host`·`port`·`encrypt=true`·`path`를 각각 지정한 W-01b·W-03b다. 이 매개변수는 noVNC `docs/EMBEDDING.md`에 지원 중단 예정으로 표기되어 있다.

### 3.5 계약 1절 항목별 해석

이 절은 [초기 교환 계약](repeater-handshake-v1.md)의 메시지 A~F와 논리 필드에 대한 관찰 결론이다. 대상은 2절의 고정 버전이다. 근거 열은 3.4절의 시나리오 ID다. 결론은 관찰한 바이트와 연결 결과만 쓴다. 내부 동작을 추정한 부분에는 `[INFERENCE]`를 붙였다.

#### 메시지별 결론

| 메시지 | 결론 | 근거 |
| --- | --- | --- |
| A. Mode II Server 등록 요청 | Server가 TCP 연결 직후 먼저 보낸다. 길이는 250바이트 고정이다. 내용은 `ID:` + 10진 ID 문자열 + NUL + 채움 바이트다. `-id:`의 채움은 모두 NUL이고, `-autoreconnect ID:`의 채움에는 0이 아닌 바이트가 있다. Repeater는 매칭 전까지 Server에 아무 바이트도 보내지 않는다. Repeater가 만들어 보내는 응답 바이트는 없다. 매칭 뒤 Server가 받는 바이트는 Viewer가 보낸 바이트다(E행) | M2-01, M2-02, T-03 |
| B. Mode II Viewer 요청 | Repeater가 TCP 연결 약 1.0초 뒤 `RFB 000.000\n`(12바이트)을 먼저 보낸다. Viewer는 이것을 받은 뒤 250바이트를 보낸다. 내용 형식은 A와 같다. Viewer의 채움 바이트는 정리되지 않은 값이다. A와 B의 차이는 접속 포트와 Repeater의 첫 12바이트뿐이다 | M2-01, M2-02, M2-05 |
| C. Mode I Viewer 요청 | 방향은 B와 같다(`RFB 000.000\n` 다음에 Viewer가 보냄). 길이는 250바이트, 내용은 `<대상>:<포트>` + NUL + 채움이다. `ID:` 같은 접두어는 없다. Viewer는 명령줄의 `host::port`를 `host:port`로 바꾸고, 이름을 대문자로 바꾸며, 포트를 생략하면 `:5900`을 붙인다. Repeater는 포트 없는 입력에 5900을 쓰고, 이름은 Repeater 쪽에서 푼다. IPv6 대상은 세 가지 표기 모두 즉시 닫혔다. Repeater는 250바이트를 대상에 전달하지 않는다. Viewer 메시지 뒤에 Repeater가 만들어 보내는 응답 바이트는 없다. 대상 연결 뒤 Viewer가 받는 바이트는 대상 Server가 보낸 바이트다(E행) | M1-01~M1-06 |
| D. 응답·오류 표시 | 오류를 알리는 바이트는 한 번도 없었다. Viewer 포트의 연결은 입력 내용과 관계없이 접속 약 1.0초 뒤 `RFB 000.000\n`(B행)을 받았다. 오류 경우(M1-04, M2-07, E-01, E-04)와 HTTP·TLS 입력(W-01, W-01b)도 같다. 이 12바이트는 어떤 입력에도 같으므로 결과를 알리지 않는다. 이 12바이트 말고 Repeater가 만들어 보낸 바이트는 없었다. 매칭이나 대상 연결 뒤에 오는 바이트는 상대가 보낸 바이트를 중계한 것이다(E행). 오류 결과는 연결 처리로만 드러났다. 연결 처리는 세 가지다. 빠른 FIN(메시지 도착 직후, Server 쪽 ID 거절은 접속 약 1.0초 뒤): 금지 대상, IPv6 대상, 숫자로 시작하지 않는 ID와 빈 ID, Server ID `999999999`. 지연 FIN: 초기 교환 시간 초과, Mode I 대상 연결 실패(약 21초). 닫지 않고 유지: 형식 오류, Mode I 꺼짐, 같은 역할 중복, 짝이 없는 대기(60초 넘게 관찰, 대기 시간 상한은 보이지 않음) | M1-00, M1-04~M1-06, M2-03~M2-05, M2-06b, M2-07, M2-09, M2-12, M2-13, E-01~E-04, W-01, W-01b |
| E. RFB 스트림 시작과 후속 바이트 | 초기 교환의 경계는 250바이트 직후다. 250바이트를 넘게 보낸 바이트(같은 쓰기에 붙은 바이트 포함)는 매칭 순간 상대에게 첫 바이트로 전달된다. 매칭 전에 도착한 바이트는 도착 순서대로 매칭 순간 전달된다. 한쪽이 22초 넘게 기다린 뒤에도 같다. Mode II에서 Viewer가 처음 받는 RFB 바이트는 Server의 `RFB 003.008\n`이다. Mode I에서는 대상 Server의 `RFB 003.008\n`이다 | M1-01, M2-01, S-02~S-05, E-03 |
| F. noVNC 요청 | noVNC 1.7.0은 `repeaterID`를 B와 같은 250바이트(`ID:<repeaterID>` + NUL 채움)로 보낸다. 먼저 보내지 않고, `RFB 000.000\n`을 받은 뒤 67~110ms 안에 바이너리 메시지 하나로 보낸다. WS와 WSS 모두 같다. 두 경우 모두 요청에 `Sec-WebSocket-Protocol` 헤더가 없다. noVNC가 공식 Repeater의 Viewer 포트에 직접 접속하면 WS·WSS 모두 연결되지 않았다. Repeater는 HTTP 업그레이드 요청과 TLS ClientHello에 응답하지 않고 `RFB 000.000\n`만 보냈다. 페이지가 `http:`일 때 WSS로 접속하려면 `path`에 절대 URL을 쓰지 않고 `host`·`port`·`encrypt=true`·`path`를 따로 줘야 했다 | W-01, W-01b, W-02, W-03b |

#### 논리 필드와 규칙

| 필드·규칙 | 결론 | 근거 |
| --- | --- | --- |
| 중계 모드 | 모드를 밝히는 별도 필드는 없다. Viewer 포트로 온 메시지가 `ID:`로 시작하면 Mode II로 처리됐다. `ID:`로 시작하지 않는 메시지는 Mode I이 켜져 있으면 대상 주소로 쓰였고, 꺼져 있으면 연결을 유지한 채 아무 일도 일어나지 않았다 | M1-00, M1-01, E-01, M2-06b |
| 역할 | 메시지에 역할 표시는 없다. 접속한 포트(Server 5500, Viewer 5901)로 구분된다. Server 포트로 온 메시지가 `ID:`로 시작하지 않으면(`XX:1234`, `1234`) 매칭도 거절도 없이 유지된다 | M2-01, E-01, M2-06b |
| 길이·패딩·종료 문자 | 길이는 정확히 250바이트다. 나눠 보내도 되고, 250바이트가 다 오기 전에는 매칭하지 않는다. 첫 NUL이 값의 끝이다. NUL 뒤의 채움 내용은 결과에 영향이 없었다. NUL 없이 250바이트를 숫자로 채워도 매칭된다. 버전 문자열은 Viewer 쪽 `RFB 000.000\n` 하나다 | S-01, E-03, E-04, M2-01, M2-06b |
| 초기 교환 시간 | 아무것도 보내지 않으면 접속 약 6.0초 뒤 FIN이 온다. 100바이트만 보내면 Server 쪽은 접속 약 6.0초 뒤, Viewer 쪽은 마지막 수신 약 5.0초 뒤 FIN이 온다. 메시지 도착 전 클라이언트가 닫으면 1~2ms 안에 Repeater도 FIN을 보낸다 | E-02, E-04 |
| 중계 ID 비교 | 문자열 비교가 아니다(`0123`과 `123`이 매칭됨). 앞자리 0만 다른 쌍은 매칭됐다(`0123`/`123`, `789`/`0789`). 숫자 뒤에 다른 문자가 붙은 값은 앞 숫자와 같게 처리된 사례가 있다(`56-78`/`56` 매칭, `12-34`·`12 34` 쌍은 `5912`로 표시). `12ab`/`12AB`도 매칭됐지만, 이 쌍만으로는 대소문자를 무시한 것인지 뒤 문자를 무시한 것인지 구분되지 않는다. 관찰한 100 미만 값은 5900을 더한 값과 매칭됐다(`12`/`5912`, `0`/`5900`, `99`/`5999`). `100`/`6000`은 매칭되지 않았다. 영문자로 시작하는 ID와 빈 ID는 거절됐다(`ABCD`, `abcd`, `ab12`, `AB12`, `cd34`, 빈 ID). 같은 값끼리도 거절되므로 영문 ID의 대소문자 비교는 시험되지 않았다. 서로 다른 20자리 값 한 쌍(`12345678901234567890`/`98765432109876543210`)이 매칭됐고, `2147483647`/`2147483648`은 매칭되지 않았다. Server ID `999999999`는 거절되지만 `999999998`과 `99999999`는 매칭된다. [INFERENCE: ID의 앞부분 숫자를 수로 바꿔 비교하는 것처럼 보인다.] 변환 규칙, 큰 값의 처리, `999999999` 거절의 이유는 확정하지 못했다 | M2-06, M2-06b, M2-07~M2-09, M2-12~M2-14 |
| 중계 ID 길이 | 1자리부터 247자리까지 받아들였다. 공개 문서 예시의 9자리 제한은 보이지 않았다. 다만 서로 다른 20자리 값 한 쌍이 매칭됐다(위 행) | M2-06, M2-06b, M2-13 |
| 대상 주소·포트 | C행과 같다. 관찰한 형식은 `<IPv4 또는 이름>:<포트>`이고 포트는 생략할 수 있었다(기본 5900). IPv6는 원시 입력 세 가지(`2001:db8:1::30:5900`, `[2001:db8:1::30]:5900`, `vnc-server6.test:5900`)에서 Repeater가 대상 연결을 시도하지 않고 FIN으로 닫았다. Viewer는 IPv6 `-connect`로 Repeater 연결을 만들지 못했다. 두 결과의 원인은 확인하지 않았다 | M1-01~M1-03, M1-06 |
| 같은 역할 중복 ID | 먼저 등록한 연결이 유지되고 매칭된다. 뒤에 온 같은 역할 연결은 등록되지 않고, 닫히지도 않으며, 보낸 바이트는 전달되지 않는다. Server와 Viewer 모두 같다. VNC Relay는 기존 연결을 유지하지만 후발 연결은 닫아 거절한다 | M2-04, M2-05 |
| 매칭과 종료 | 짝이 되는 메시지가 도착한 뒤 약 1.0~2.0초에 매칭된다. 매칭된 한쪽이 닫히면 약 1ms 안에 상대에게 FIN이 간다. FIN 약 3ms 뒤 RST가 뒤따른 경우도 있었다. Server의 `-autoreconnect`는 끊긴 뒤 약 2초에 다시 등록하고, 새 Viewer와 매칭된다 | M2-01, M2-02, T-01~T-03, M1-02 |

#### 형식과 무관한 관찰 사실

- Repeater 1.6.0.0은 10자리 ID 세션이 활성인 동안 웹 상태 페이지를 요청하면 응답하지 않고 프로세스가 끝났다. 세션의 두 연결은 RST로 끊겼다(M2-11, 2/2). `vnc-relay`의 동작 기준이 아니며, 관찰 하네스가 상태 요청 시점을 조절한 근거다.
- 웹 상태 페이지는 매칭 상태를 항상 정확히 보여주지 않았다. `ID:12`/`ID:5912` 쌍은 매칭 중에도 대기 Viewer로만 표시됐다(M2-14). 그래서 매칭 판정은 상태 페이지가 아니라 표식 전달로 했다.

VNC Relay는 1~247자리 ASCII 숫자 ID를 값 크기와 관계없이 바이트열로 비교한다. `999999999`도 받는다. Repeater의 내부 숫자 변환 규칙은 VNC Relay의 동작 기준이 아니다. 활성 세션이 점유한 ID에 같은 역할이 오는 Repeater 동작은 관찰 범위 밖이다. 의도적인 차이는 [계약 3.1절](repeater-handshake-v1.md#31-대상-버전-repeater와-다르게-처리하는-입력)에 있다.

### 3.6 원본과 보관 위치

원본은 저장소 밖 관찰 PC의 `D:\vr-captures\<계획>\<시나리오>\run<N>\`에 있다. 저장소에는 해시와 위치만 남긴다.

| 파일 | 내용 |
| --- | --- |
| `capture.pcapng` | Repeater VM에서 잡은 캡처 원본. 아래 표의 SHA-256은 이 파일의 해시다 |
| `run.json` | 회차 시작·끝 시각, VM 상태, Repeater 설정, 상태 수집 결과, 캡처 해시(`sha256`) |
| `raw-*.txt` | 원시 클라이언트와 시험용 중계기의 로그(보낸·받은 바이트와 시각) |
| `web-*.html` | Repeater 웹 상태 페이지 원문 |
| `shot-*.png` | VM 화면 캡처(찍은 회차만) |
| `..\summary.md` | 시나리오별 방향별 바이트 덤프, 시각, 회차 비교(`summarize.py` 출력) |

- 시각은 호스트 시각(2026-11-01)이다. 유효 회차에서 도중에 꺼진 VM은 없었다.
- 2026-11-01에 보관 캡처 76개의 SHA-256을 다시 계산해 모두 `run.json`과 일치함을 확인했다. 아래 목록은 그중 최신 유효 관찰에 쓰는 74회차다.
- M1-00의 요약은 `05-p10-mode1-part1.json`, M1-01~06의 요약은 `05-p10-mode1.json`이다.
- WSS의 최신 유효 증거는 p18의 W-01b·W-03b다. 아래 목록에는 같은 입력을 재현한 두 회차를 함께 둔다. 두 회차는 재현성 확인과 7절의 비교 기준에 쓰인다.

| 계획 | 시나리오 | 회차 | 시작 | 끝 | 캡처 SHA-256 |
| --- | --- | --- | --- | --- | --- |
| p10-mode1 | M1-00-mode1-disabled-default | 1 | 10:42:27 | 10:46:35 | `0db3aca1e4581d3681bf46a119132a20038090d905c042ba89913bf42217d02e` |
| p10-mode1 | M1-00-mode1-disabled-default | 2 | 10:46:35 | 10:51:45 | `a912763a938141280fec4d7ae4484d5aa864faa55af38a27be84a407a060f24c` |
| p10-mode1 | M1-01-ip-port | 1 | 11:04:28 | 11:08:27 | `3a0748fe0103e3811b20e6f5a7b374c01bb6cc78fef8ff436b9c5976eb6a5e11` |
| p10-mode1 | M1-01-ip-port | 2 | 11:08:27 | 11:12:37 | `1840be0af894b9a72080307e2bca159de8e200fc2a504a70c7a67a99e5f42ce8` |
| p10-mode1 | M1-02-port-omitted | 1 | 11:12:37 | 11:19:19 | `c737ffd1aaaaccb74d663d77810a3c5eea56162346a62db899032854cf9562ab` |
| p10-mode1 | M1-02-port-omitted | 2 | 11:19:19 | 11:25:35 | `2e15db0d79a4e98b44a379d04dd55b1ca74f6dc5a7bfb5422fb0b90ad19794d9` |
| p10-mode1 | M1-03-name | 1 | 11:25:35 | 11:29:45 | `9307f604d2599872ab92dbfdc898ad071cb9eb0711c2ea2dd22725c7e14b8699` |
| p10-mode1 | M1-03-name | 2 | 11:29:45 | 11:34:04 | `d5ce6d091a0ad55988425f622c4d65362b2d30abf75230ac8fc256c55530f193` |
| p10-mode1 | M1-04-refused-target | 1 | 11:34:04 | 11:38:15 | `199d5bb0dd5927853b58edfc93a37cef8561a97206d308cef0b7da683dbf7b65` |
| p10-mode1 | M1-04-refused-target | 2 | 11:38:15 | 11:42:26 | `6603bed594ff01ad4f81f88e4a78f071977bc7c4b0dc88a4bc6b22d6cddb0ad0` |
| p10-mode1 | M1-05-connect-failure | 1 | 11:42:26 | 11:46:58 | `aebf738c9711a03662b0d33799991eb72161cc7de34f460273af849467a22cd2` |
| p10-mode1 | M1-05-connect-failure | 2 | 11:46:58 | 11:51:41 | `f09646392fe671ae2ec7a46f50eb512c03eac16737e4a87a8a4ce30bf80b1f12` |
| p10-mode1 | M1-06-ipv6-target | 1 | 11:51:41 | 11:57:47 | `6d6f3a3eb9d65d54e013f479c9ec57418e0478a2f9009bb1b25dd0e1838d6125` |
| p10-mode1 | M1-06-ipv6-target | 2 | 11:57:47 | 12:03:53 | `d61cba9555323a40bd5f44dda3e637f1f49e7a396befebc98838231b497ee96f` |
| p11-mode2 | M2-01-server-first | 1 | 12:03:54 | 12:07:10 | `bf7e4450c431fdefdae8d8766c1501c55436e89d7600ec3752e8005129351c36` |
| p11-mode2 | M2-01-server-first | 2 | 12:07:10 | 12:10:47 | `936d0d76550e8b8e7014a33ef4739488adf5694e5a32cfbc82f2645a40323022` |
| p11-mode2 | M2-02-viewer-first | 1 | 12:10:47 | 12:15:17 | `10d6c54ae34b1af48f0727aa4cfb2d3c95efd5e40192747efed48f24910fef42` |
| p11-mode2 | M2-02-viewer-first | 2 | 12:15:17 | 12:19:17 | `5705c5d73c31c37e273b5fec277ba4133249479ddde446fe7259fee8301647b8` |
| p11-mode2 | M2-03-different-ids | 1 | 12:19:17 | 12:24:24 | `fcfd17159aa2cb2f0fc4cf4f23cafe402fd92df914de7d8df29df402cab52da7` |
| p11-mode2 | M2-03-different-ids | 2 | 12:24:24 | 12:29:03 | `7619d1c12095e3462e9f63345497d42d47c13cd23810cbfd8d1d1387fd6320f9` |
| p11-mode2 | M2-04-duplicate-server-id | 1 | 12:29:03 | 12:33:31 | `7c507bc5dc4f51c9b6dac9520a96edfc4b8d8bff2e77ac24648c3e85715b05a5` |
| p11-mode2 | M2-04-duplicate-server-id | 2 | 12:33:31 | 12:37:20 | `acb62c1c2b31c0b4421ea8e23e53abe5fce352ed6ffd0e9a13da60301ffd4c51` |
| p11-mode2 | M2-05-duplicate-viewer-id | 1 | 12:37:20 | 12:41:06 | `e256d439d7603e7988a878a39e1bc98e0bc5be2d7aa271afd16ae6fde2afa7ef` |
| p11-mode2 | M2-05-duplicate-viewer-id | 2 | 12:41:06 | 12:45:26 | `5f1d5561436f0f24e9abbffbc09ffc4ccab9601c56c4e532527000ff613b7fdc` |
| p11-mode2 | M2-06-id-length-boundaries | 1 | 12:45:26 | 12:49:39 | `557063d3d721ad19a9e5fe56da00c7f12232e0ebc06a0e5cf52fff58f8b18215` |
| p11-mode2 | M2-06-id-length-boundaries | 2 | 12:49:39 | 12:53:51 | `b8160bb8ae94691c3c030978b59b8435f616756694814b6902e8238a645935bd` |
| p11-mode2 | M2-07-id-characters | 1 | 12:53:51 | 12:57:21 | `81df885cadb6ca3dbeb787aac8712a152bcdd81d8c1c562f20aaea9618cb8fd0` |
| p11-mode2 | M2-07-id-characters | 2 | 12:57:21 | 13:00:38 | `a8645b97a7c12a3dc002bb1927d1d010e8d3dfad2150dc015d5878b6a2cd3979` |
| p11-mode2 | M2-08-leading-zero | 1 | 13:00:38 | 13:03:29 | `98a53870cec4bfd9aba8d93454609c0b456eee656620579c401dd559fe5efea6` |
| p11-mode2 | M2-08-leading-zero | 2 | 13:03:29 | 13:06:19 | `de48918332c80f908e1adc7e28b4b8b4242a6c8ebba840e69e47eb63ef0bdea2` |
| p11-mode2 | M2-09-case-only | 1 | 13:06:19 | 13:08:48 | `71e26e32b46ddf88ba29723f8618393b49eafe61f7e8e1f1fe558167c38af389` |
| p11-mode2 | M2-09-case-only | 2 | 13:08:48 | 13:11:04 | `182c715528b18fa4aad9d2e3545970c9d8af2719d542f28e05fc61ce5e61735d` |
| p12-errors-split | E-01-malformed | 1 | 13:11:06 | 13:13:09 | `c508eb55f71802507659b93f723c417bbf3ba95fee780f91befd1fd63b073cf0` |
| p12-errors-split | E-01-malformed | 2 | 13:13:09 | 13:15:14 | `ae34d49e7094a99a5e8140a399f4fcb9d9227f4f16e12a5220f1467bd5870a34` |
| p12-errors-split | E-02-short-then-close | 1 | 13:15:14 | 13:16:51 | `d395e5bc71487d7966913a2171b0eac17260c1e3f5ed17cf95ab3a1ab9486bae` |
| p12-errors-split | E-02-short-then-close | 2 | 13:16:51 | 13:18:28 | `cdae1c45180689bcc3740dbf31000a0f96b88ef385861a4ddb2aab81b140c658` |
| p12-errors-split | E-03-too-long | 1 | 13:18:28 | 13:20:25 | `150735734ccb120eb41a1ef73948d39d97818dca2de3fe6a0b993d2eaa3090ed` |
| p12-errors-split | E-03-too-long | 2 | 13:20:25 | 13:22:37 | `d8915cfbf075e3e75ace287d570bf9e625c0328e7827f126878a4d97a624c557` |
| p12-errors-split | E-04-incomplete-timeout | 1 | 13:22:37 | 13:25:08 | `18b726f5c0452d46c2201424d1fdf61f50657eecd7c92144f82738a9e8367ca2` |
| p12-errors-split | E-04-incomplete-timeout | 2 | 13:25:08 | 13:27:54 | `ff864b03a362d9386a3be233eaa3d4b8c856a1c6b7d266f97aab03ebb365e64e` |
| p12-errors-split | S-01-split-initial | 1 | 13:27:54 | 13:29:49 | `4832abfce015a86da6ad4f5b883435c1ba36f346878a0847efa9b7288ca3ae6f` |
| p12-errors-split | S-01-split-initial | 2 | 13:29:49 | 13:31:43 | `e858b3739dcc2152d93765cef3361e22509317e0509ac3ebe4ae156f536ee2be` |
| p12-errors-split | S-02-initial-with-following | 1 | 13:31:43 | 13:33:25 | `72b6615237e3e486a17baab9d91ba68056c00a769f736327c5a22800fc3ebfb8` |
| p12-errors-split | S-02-initial-with-following | 2 | 13:33:25 | 13:35:19 | `b438a74bf6d669559cf3350839dfaba8468ffe1753b9fc6803cc604f3993bee0` |
| p12-errors-split | S-03-bytes-before-after-match | 1 | 13:35:19 | 13:37:28 | `7190fce089d50dae70ac95c2284739cb877ac74f40a597afcbeadd00d2c3db2b` |
| p12-errors-split | S-03-bytes-before-after-match | 2 | 13:37:28 | 13:39:46 | `d92c4f575753649dc81e5df159d87cbc80a3a9206c9edd8f1a4c55d854bea211` |
| p13-termination | T-01-viewer-closes | 1 | 13:39:48 | 13:41:56 | `ef8b911b06f220b319e021cfb9a4dc43104ebee4b04b3c984f62b765c8ddde9a` |
| p13-termination | T-01-viewer-closes | 2 | 13:41:56 | 13:43:51 | `48abf4e7ebe25d70e941535a12c618a050b8735ee466cea5f85c73a34b1532f5` |
| p13-termination | T-02-server-closes | 1 | 13:43:51 | 13:47:33 | `9893d51a90614bde8ba0f54d3b2e9194585fe69c2eb37096a8294b627527abf8` |
| p13-termination | T-02-server-closes | 2 | 13:47:33 | 13:51:23 | `2b09e79be1b8d2c4bff8507185e35c36124f1aed8f9de8bb03d0c386c28a9415` |
| p13-termination | T-03-server-autoreconnect | 1 | 13:51:23 | 13:55:48 | `79b736d472f648a352c6de70a37ddeffb2914f98d5d649028d460e30507370e4` |
| p13-termination | T-03-server-autoreconnect | 2 | 13:55:48 | 14:01:03 | `c22f6ff4331b077606dff25a865e3d8b84d595dfba2f42da493e8002bb3da454` |
| p15-followup | M2-06b-id-boundaries-isolated | 1 | 14:45:13 | 14:49:42 | `3a6f8ad10fff411ca785f7de9fd47a2fd66e91a2c52252577de06d235a0f4e13` |
| p15-followup | M2-06b-id-boundaries-isolated | 2 | 14:49:42 | 14:54:10 | `193483ba7876ca1383c1dec12a31d55937202014ee13171e62b538530610ac3d` |
| p15-followup | M2-11-ten-digit-id-web-status | 1 | 14:54:10 | 14:57:32 | `2297840d3b9d55d8c4a3aedef6a5531800ae5be68c4f496ae2adc3ae9acf443e` |
| p15-followup | M2-11-ten-digit-id-web-status | 2 | 14:57:32 | 15:00:42 | `7bd00156f98dba282d9b48831ee1c49be42a13a2ef18bf34f5205e66150fd9c5` |
| p15-followup | M2-12-numeric-prefix | 1 | 15:00:42 | 15:04:33 | `49ee2ac8a98713016494723a91166915d399b76a341f295a381ee1df73368114` |
| p15-followup | M2-12-numeric-prefix | 2 | 15:04:33 | 15:08:24 | `f6b954da6e1d2ad5548560b799dd08b11e2f4c686a1ccee1bb0c581e37595fb5` |
| p15-followup | S-04-prematch-zero-byte | 1 | 15:08:24 | 15:11:42 | `a8b49f2d8e3706c4fe204c3f4231df465f977226043fef7e2aa2b65a688bfbd3` |
| p15-followup | S-04-prematch-zero-byte | 2 | 15:11:42 | 15:15:13 | `20da620fbc41c0198999fee5d2eea614cc4035a2e34e1538ba5abfba4e2c1ca3` |
| p16-websocket | W-01-novnc-direct-to-repeater | 1 | 15:16:24 | 15:26:12 | `39d68e87c52978dd0ff4b02458ecf84c03e0472c8f65a3905e0e08b815a8d936` |
| p16-websocket | W-01-novnc-direct-to-repeater | 2 | 15:26:12 | 15:36:00 | `e88f15727c33943cfb05aee8b1435ca43fc00ebcdf8bd2d3de920e1a961bc3b7` |
| p16-websocket | W-02-novnc-ws-bridge | 1 | 15:36:00 | 15:41:51 | `aec37706d03daf77a2c8e26a48610ccc804851bdbe7ecea4c7e9427a5304711b` |
| p16-websocket | W-02-novnc-ws-bridge | 2 | 15:41:51 | 15:48:12 | `e3644498f26580593644c2fa6ce1c990f18da3e086afdaaae28274d3d7bb2189` |
| p17-followup2 | S-05-prematch-bytes-long-wait | 1 | 16:02:11 | 16:06:01 | `dc8d09b872f9c72dcbce3e6ac879c8da7248ab0f531e277d320c44db6f7fb968` |
| p17-followup2 | S-05-prematch-bytes-long-wait | 2 | 16:06:01 | 16:09:51 | `3052a1266d12fddcea7146f104746b492178aa3315a5d7adce314504346af742` |
| p17-followup2 | M2-13-id-999999999-and-overflow | 1 | 16:09:51 | 16:15:12 | `490931646e11564e9664894d0884e13fef964bade73ad6fc88540fe0966971f7` |
| p17-followup2 | M2-13-id-999999999-and-overflow | 2 | 16:15:12 | 16:20:29 | `4ecddfb1a5afbbe9343622b0ee84168f544c2a2caee30dbdebec27414fe9ec6e` |
| p17-followup2 | M2-14-display-number-ids | 1 | 16:20:29 | 16:24:59 | `f8d0f5442944d7c0632bb0b1ab10b9e6744fb3bffd064cf3d53c9f3487528c2d` |
| p17-followup2 | M2-14-display-number-ids | 2 | 16:24:59 | 16:29:30 | `4918787105c9d9cbc148fdfb0dc82fa498e7565048b403c7cf39bae81cb06536` |
| p18-websocket-wss | W-01b-novnc-wss-direct-to-repeater | 1 | 16:47:37 | 16:53:13 | `479740ee8d26b9a112ec1056fc6990f23b08bed3e27f1ec3b38ec453044b0b3e` |
| p18-websocket-wss | W-01b-novnc-wss-direct-to-repeater | 2 | 16:53:13 | 16:59:39 | `f27ccff53a2e819f84229b00f6562c9702b73dbf93aa10bc27a8af94f595c517` |
| p18-websocket-wss | W-03b-novnc-wss-bridge | 1 | 16:59:39 | 17:06:23 | `b34a00745af7ccadd8db43a31d6cd53a2b6ca547d3cee16dc27102ae929b7f3c` |
| p18-websocket-wss | W-03b-novnc-wss-bridge | 2 | 17:06:23 | 17:13:01 | `4e5291d7c1e6272026e0064eb6c28e26d339663088417f005be459ebff97a11b` |

## 5. 검증 명령과 결과

2026-11-01에 Windows 11 x64 호스트의 관리자 PowerShell 5.1에서 `D:\vr-lab\scripts\05-observe.ps1 -Plan <계획 파일>`과 여러 계획을 순서대로 실행하는 `06-observe-many.ps1 -Plans <계획 목록>`을 썼다. 계획은 p10·p11·p12·p13·p15·p16·p17·p18이며, 실제 입력·시간·캡처 해시는 3절에 있다.

캡처 분석 명령은 `python D:\vr-lab\scripts\summarize.py D:\vr-captures\<계획>`이다. 결과는 시나리오별 `summary.md`다. 초기 교환과 매칭 결과는 두 회차에서 같았다. 종료 순서·화면 갱신 크기·TLS ClientHello 크기에는 3.3절의 차이가 있었다. S-05의 종료 원인은 추정으로 남긴다.

keepalive 구분에는 `tshark -r <capture.pcapng> -Y "tcp.len==1" -T fields -e frame.time_relative -e ip.src -e tcp.srcport -e tcp.dstport -e tcp.payload -e tcp.analysis.keep_alive`를 썼다. winvnc의 약 10초 간격 `00`은 keepalive로 표시됐고, S-04의 원시 클라이언트 `00`은 표시되지 않았다.

## 6. 모드·전송 조합 검증

2절의 고정 버전으로 모든 모드·전송 조합을 검증했다. 실제 UltraVNC Server 1.8.3.0을 상대로 아홉 조합이 모두 통과했다. TCP Mode I, TCP Mode II(Server 먼저·Viewer 먼저), WS·WSS Mode I, noVNC 1.7.0의 WS·WSS Mode II(Server 먼저·noVNC 먼저)다. 이로써 [WebSocket 스펙 6절](../specs/websocket.md#6-전송모드-검증-조합) 표의 여덟 행을 모두 실제 Server로 확인했다. 이 실행에서 [계약 3.1절](repeater-handshake-v1.md#31-대상-버전-repeater와-다르게-처리하는-입력)에 없는 차이는 나오지 않았다(6.4절).

### 6.1 실행 방법과 범위

| 항목 | 내용 |
| --- | --- |
| 실행 환경 | 3.2절의 Hyper-V VM 3대다. 에이전트가 VM 안의 Viewer·Server·브라우저를 자동으로 조작하고, 프로젝트 소유자가 판정 캡처를 검토했다 |
| 캡처 검토 | 2026-11-06에 프로젝트 소유자가 TCP·noVNC 일곱 조합과 WS·WSS Mode I 두 조합의 판정 캡처를 검토했다. 문제가 없었다 |
| Ubuntu | 실제 UltraVNC와 Ubuntu VNC Relay의 조합은 미실행이다. Ubuntu 지원 판정에는 자동 시험을 쓴다(6.6절) |
| WS·WSS Mode I | WebSocket으로 Mode I 요청을 보내는 고정 버전 클라이언트는 없다. noVNC 1.7.0은 항상 `ID:<repeaterID>`를 보내고(3.4절), UltraVNC Viewer는 WebSocket을 쓰지 않는다. UltraVNC Viewer 1.8.3.0은 Viewer VM의 시험용 바이트 중계 스크립트 `tcp2ws.ps1`에 접속했다. 이 스크립트는 websockify가 아니다. TCP 바이트를 바꾸지 않고 WebSocket 이진 메시지로 VNC Relay에 보내며, 반대 방향도 그대로 옮긴다. 초기 요청은 Viewer가 보낸다. Origin 헤더는 보내지 않는다(WS-021) |
| WSS 인증서 | 저장소의 시험용 인증서(`crates/adapters/tests/fixtures/tls/server-chain.pem`·`server-key-p256.pem`)를 썼다. Edge는 `--ignore-certificate-errors`로 실행했고, `tcp2ws.ps1`은 인증서 검사를 껐다. 인증서 신뢰는 검증 범위 밖이다 |
| 판정 기준 | 회차마다 세 가지가 모두 맞으면 통과다. Server 시험 창 기록에 클릭 1회와 회차 문자열 전체가 있다. Viewer 화면 캡처에 Server 화면과 입력 결과가 보인다. VNC Relay 로그에 해당 전송의 `session_started`가 있다 |

### 6.2 환경과 버전

| 항목 | 값 |
| --- | --- |
| 실행 시각 | 2026-11-06 KST(UTC+9). 첫 실행 19:14~19:34(VM 시작 19:14:16, 회차 19:17~19:33, 로그 `ts`는 UTC 10:16~10:33). 두 번째 실행 19:42~19:46(VM 시작 19:42:05, 회차 19:44~19:45, 로그 `ts`는 UTC 10:44~10:45) |
| VM | 3.2절 구성. 두 실행 모두 세 VM을 `baseline`으로 되돌린 뒤 부팅했고, Server의 시험용 VNC Password를 새로 정했다. `vr-lab` `Private` 스위치, 세 VM 모두 기본 게이트웨이 없음. 게스트 OS는 Windows 11 Enterprise Evaluation 10.0.26300(build 26300). 실행마다 한 시간 안에 끝내고 VM을 껐다 |
| Viewer | `vr-viewer`(192.0.2.10) `C:\Program Files\uvnc\UltraVNC\vncviewer.exe` 1.8.3.0, SHA-256 `18760910769dc497266d6ba7a28892925177d0208fe4433bf62f0b16061f3427` |
| Server | `vr-server`(192.0.2.30) `C:\Program Files\uvnc\UltraVNC\winvnc.exe` 1.8.3.0, SHA-256 `9ebd1ec90ba1274d4a183f5bf6588374700052da907fa764a2ae7072e8ae19fe`. 시험용 VNC Password만 바꾸었다. `AuthRequired=1`, `ReverseAuthRequired=1`, `InputsEnabled=1` |
| noVNC | 1.7.0. `vr-viewer`에서 `novnc-v1.7.0.zip`(SHA-256 `d55e9cce9a611294df33c7200f32f35108652de2d86a468bebcfc6f2a1c9a196`, 2.4절 값과 같음)을 풀고 `ws-lab.ps1 -Mode static`으로 `http://127.0.0.1:8000/`에서 제공했다 |
| 브라우저 | `vr-viewer`의 Microsoft Edge 152.0.4191.66, `--headless=new`. DevTools 프로토콜로 화면 캡처·클릭·키 입력을 보냈다 |
| 바이트 중계 스크립트 | `vr-viewer`의 `C:\vr\ui\tcp2ws.ps1`. 127.0.0.1:5998(WS)·5999(WSS)에서 받고 `ws://192.0.2.20:8081/relay`·`wss://192.0.2.20:8443/relay`로 보낸다. Windows PowerShell 5.1과 .NET `ClientWebSocket`을 쓴다. VM 실행 전에 호스트 루프백에서 같은 스크립트로 Mode I 초기 교환과 에코를 WS·WSS 모두 확인했다 |
| noVNC 주소 | `http://127.0.0.1:8000/vnc.html?autoconnect=true&reconnect=false&repeaterID=1234&host=192.0.2.20&port=<8081 또는 8443>&encrypt=<false 또는 true>&path=relay` |
| VNC Relay | 커밋 `70ff541`의 Windows release 빌드(`cargo build --release --bin vnc-relay`), `vnc-relay 0.1.0`, SHA-256 `d94e4dd34a297cfdd008bf3e856c30deb2c5366dc6d844e6502aa3aaf097b5fc`. 빌드 산출물, 반입 묶음, `vr-repeater`의 `C:\vr\compat-5.5\vnc-relay.exe` 세 파일의 해시가 같았다. 미커밋 변경은 없었다 |
| VNC Relay 설정 | `vr-repeater`(192.0.2.20) `C:\vr\compat-5.5\relay.toml`, SHA-256 `20fa0505e23e5452c687328caa3a6bb02e3a0f68b0d33b9aab141e510b4d0e9e`. 리스너는 `tcp_viewer` 192.0.2.20:5901, `tcp_server` 192.0.2.20:5500, `ws_viewer` 192.0.2.20:8081, `wss_viewer` 192.0.2.20:8443, 관리 127.0.0.1:18080, 상태 127.0.0.1:18081이다. Viewer 진입점은 192.0.2.10만, Server 진입점은 192.0.2.30만 허용했다. WS·WSS `allowed_origins`는 `http://127.0.0.1:8000`이다. Mode I 허용 대상은 `192.0.2.30:5900` 하나다 |
| 방화벽 | `vr-repeater`에 `vnc-relay-compat-5.5` 그룹 규칙 4개를 두었다. 포트 5901·8081·8443은 192.0.2.10에서, 5500은 192.0.2.30에서만 받는다. 프로그램은 `vnc-relay.exe`로 한정했다. 실행 뒤 지웠다 |

### 6.3 조합별 결과

판정 회차는 조합마다 1회다. Mode II ID는 양쪽 모두 `1234`다. 독립 2회로 화면·입력을 확인한 TCP 증거와 거절·IP 경계·유휴 연결 증거는 [수동 확인 기록](../how-to/verify-ultravnc-manually.md#41-수동-실행-기록)에 있다.

| 조합 | 실행 | Server 시험 창 기록 | VNC Relay 로그 | 결과 |
| --- | --- | --- | --- | --- |
| TCP Mode I (`tcp-m1`, RLY-006) | Server `winvnc -run`. Viewer `vncviewer -connect 192.0.2.30::5900 -proxy 192.0.2.20:5901`. VNC 인증 창에 시험용 비밀번호를 넣었다 | 클릭 1회, `tcpmodeone` | `session_started` `mode1` `viewer_transport=tcp` `target_port=5900`, `session_closed` `peer_eof` | 통과 |
| TCP Mode II, Server 먼저 (`tcp-m2s-b`, RLY-010) | Server `winvnc -id:1234 -connect 192.0.2.20::5500 -run`, 약 10초 뒤 Viewer `vncviewer -proxy 192.0.2.20:5901 ID:1234`. Viewer의 인증 없음 확인 창에서 연결을 골랐다 | 클릭 1회, `tcpserverfirst` | `waiting_registered`(Server) 뒤 `session_started` `mode2` `tcp` `waited_ms=10410`, `peer_eof` | 통과 |
| TCP Mode II, Viewer 먼저 (`tcp-m2v-b`, RLY-011) | 같은 명령을 Viewer, 약 8초 뒤 Server 순서로 실행했다 | 클릭 1회, `tcpviewerfirst` | `waiting_registered`(Viewer) 뒤 `session_started` `mode2` `tcp` `waited_ms=7967`, `peer_eof` | 통과 |
| WS Mode I (`ws-m1`, WS-001) | Server `winvnc -run`. Viewer `vncviewer -connect 192.0.2.30::5900 -proxy 127.0.0.1:5998` → `tcp2ws.ps1` → `ws_viewer`. VNC 인증 창에 시험용 비밀번호를 넣었다 | 클릭 1회, `wsmodeone` | `session_started` `mode1` `viewer_transport=ws` `target_port=5900`, `session_closed` `peer_eof` | 통과 |
| WSS Mode I (`wss-m1`, WS-002) | 같은 방법으로 `-proxy 127.0.0.1:5999` → `wss_viewer` | 클릭 1회, `wssmodeone` | `session_started` `mode1` `viewer_transport=wss` `target_port=5900`, `session_closed` `peer_eof` | 통과 |
| WS Mode II, Server 먼저 (`ws-s-b`, WS-003·WS-025) | Server 명령 뒤 Edge로 noVNC 주소(`port=8081`, `encrypt=false`)를 열었다 | 클릭 1회, `wsserverfirst` | `session_started` `mode2` `viewer_transport=ws` `waited_ms=10427` | 통과. noVNC 상태 `Connected (unencrypted) to vr-server ( 192.0.2.30 ) - application mode` |
| WS Mode II, noVNC 먼저 (`ws-v-b`, WS-004·WS-025) | noVNC 주소를 연 뒤 약 5초 뒤 Server 명령 | 클릭 1회, `wsnovncfirst` | `waiting_registered`(`ws_viewer`) 뒤 `session_started` `ws` `waited_ms=5119` | 통과. 상태 `Connected (unencrypted) …` |
| WSS Mode II, Server 먼저 (`wss-s-b`, WS-005·WS-025) | Server 명령 뒤 noVNC 주소(`port=8443`, `encrypt=true`) | 클릭 1회, `wssserverfirst` | `session_started` `viewer_transport=wss` `waited_ms=10038` | 통과. 상태 `Connected (encrypted) …` |
| WSS Mode II, noVNC 먼저 (`wss-v-b`, WS-006·WS-025) | noVNC 주소를 연 뒤 약 5초 뒤 Server 명령 | 클릭 1회, `wssnovncfirst` | `waiting_registered`(`wss_viewer`) 뒤 `session_started` `wss` `waited_ms=5021` | 통과. 상태 `Connected (encrypted) …` |

- TCP와 WS·WSS Mode I 회차의 세션은 Viewer 창을 닫아 끝냈다. `session_closed`는 `close_cause=peer_eof`, `side=viewer`였다. `tcp2ws.ps1` 기록에서는 Viewer 쪽 TCP 종료 뒤 VNC Relay가 Close(1000)로 답했다.
- WS Mode I 회차에서 `tcp2ws.ps1`이 옮긴 바이트는 Viewer 쪽 1,536, Server 쪽 833,658이다. WSS Mode I은 1,506과 947,084다.
- noVNC 회차의 세션은 하네스가 Edge 프로세스를 강제로 끝내서 닫았다. 그래서 `session_closed`는 `close_cause=connection_error`, `error_kind=io_error`, `side=viewer`였다. 3.4절 W-03b에서 Edge를 끝냈을 때 RST가 관찰된 것과 맞다. noVNC가 정상 Close를 보내는 종료는 이 실행에서 확인하지 않았다.
- noVNC는 Server의 VNC 인증 없이 연결됐다. [INFERENCE: 역접속 Server가 None 보안 유형을 제시한 것으로 보인다. 3.4절 W-02의 보안 유형 선택과 수동 확인 기록의 "인증 없음" 창에 근거한다. 이 실행에서는 RFB 바이트를 캡처하지 않았다.]
- 두 실행 모두 끝낼 때 Ctrl+C로 VNC Relay를 멈췄다. 로그는 `shutdown_started` `trigger=requested` `active_sessions=0` 뒤 `server_stopped`였고, 종료 코드는 0, 표준 오류는 비어 있었다.

### 6.4 Repeater 1.6.0.0과의 차이

이 실행에서 확인한 차이는 모두 [계약 3.1절](repeater-handshake-v1.md#31-대상-버전-repeater와-다르게-처리하는-입력)에 있다. 3.1절에 없는 새 차이는 나오지 않았다.

| 계약 3.1절 행 | 이 실행의 확인 |
| --- | --- |
| WebSocket·TLS 접속 | 확인했다. Repeater 1.6.0.0에는 WebSocket 대기 포트가 없다(3.4절 W-01). `vnc-relay`는 WS·WSS 접속을 `ws_viewer`·`wss_viewer`에서 받아 Mode I·II 모두 실제 Server와 중계했다(6.3절) |
| 시작 바이트의 시점 | 실제 Viewer·Server·noVNC가 아홉 조합 모두에서 초기 교환을 마치고 세션에 들어갔다. 바이트 시점은 캡처하지 않았다 |
| ID 비교, Server ID `999999999`, 형식이 틀린 요청, 짝이 없는 대기 시간, 초기 교환 시간, IPv6 대상 | 이 실행의 입력에 없다. 양쪽에 같은 숫자 ID와 IPv4 대상만 썼다. 이 차이들의 증거는 계약 3.1절의 근거와 자동 시험이다 |

### 6.6 Ubuntu 24.04 증거

실제 UltraVNC와 Ubuntu VNC Relay의 조합은 미실행이다. Ubuntu 지원 판정은 자동 시험으로 확인한다. [테스트 컨벤션](../conventions/testing.md)에 따라 `cargo xtask scenarios`로 생성하는 `scenario-report.md`와 CI 결과를 확인한다. WS-001~006·WS-020·WS-025 및 TCP Mode I·II의 개별 결과와 OS별 범위를 구분한다. WS-025의 시험용 TCP Server는 최소 RFB 3.8 서버이며 실제 UltraVNC가 아니다.

### 6.7 원본과 명령

원본은 저장소 밖 두 폴더에 있다. `SHA256SUMS.txt`는 `<해시> *<파일 이름>` 형식이며, 목록의 모든 항목을 .NET SHA-256으로 다시 계산해 일치를 확인했다.

| 실행 | 폴더 | 파일 | `SHA256SUMS.txt` SHA-256 | VNC Relay 운영 로그 |
| --- | --- | --- | --- | --- |
| 첫 실행 | `D:\vr-captures\compat-5.5\run-20261106\` | 95개(실행 준비 기록 `host-prep.json`·`host-prep.log` 포함) | `49e59c46069adce5326aa13cde8cfbad252895505e0e8f00c7d991b194f56404` | `vr-repeater-relay.jsonl` 51줄, `f0476fa2ab76e28ab50a6f9e7eab26c8e50456f3743918147da676616f45bbd6` |
| 두 번째 실행 | `D:\vr-captures\compat-5.5\run-20261106-ws-mode1\` | 26개(`prep.json`·`prep.log` 포함) | `d332217102560ff00fd7804b2da4021e069da9be7c7efb83e48d0e30a77f1268` | `vr-repeater-relay.jsonl` 11줄, `f55c968ab7db46faedeffee61e12f7d731a9b97576f31e4dac305c5626d05e3b` |

판정 캡처는 회차마다 `vr-viewer-viewer-<회차>-after-keys.png` 또는 `vr-viewer-novnc-<회차>-after-keys.png`, `vr-server-<회차>-console.png`, `vr-server-form-<회차>.log`다. WS·WSS Mode I 회차에는 `vr-viewer-tcp2ws-<회차>.log`도 있다.

하네스는 저장소 밖 `D:\vr-tools\tmp\compat-5.5\`에 있다. `guest\novnc-drive.ps1`은 Edge DevTools 프로토콜로 캡처·클릭·키 입력을 보내고, `guest\tcp2ws.ps1`은 WS·WSS Mode I용 바이트 중계를 맡는다. 각 회차를 시작하기 전에 Viewer·Edge·winvnc와 앞 회차의 대기 연결을 정리한다. 관리자 권한 실행은 사용자가 UAC를 승인했다.

```
관리자 권한 PowerShell 5.1:
powershell -NoProfile -ExecutionPolicy Bypass -File D:\vr-tools\tmp\compat-5.5\elevate.ps1 -Script D:\vr-tools\tmp\compat-5.5\worker.ps1
작업 파일(D:\vr-tools\tmp\compat-5.5\q\): 첫 실행 0001-prep, 0002-fw-tray, 0003-settings, 0004-pwset-serve, 0005-tcp, 0006-novnc, 0007-rerun, 0008-finish. 두 번째 실행 0101-prep, 0102-setup, 0103-mode1-ws, 0104-finish

0004·0102 serve: server_ready listeners=status 127.0.0.1:18081, management 127.0.0.1:18080, tcp_viewer 192.0.2.20:5901, tcp_server 192.0.2.20:5500, ws_viewer 192.0.2.20:8081, wss_viewer 192.0.2.20:8443
0008·0104 finish: shutdown_started trigger=requested active_sessions=0 → server_stopped, serve-exit-main.txt exit=0, serve-stderr-main.txt 비어 있음
0008·0104 finish: VM 안의 비밀 파일 없음, 방화벽 규칙 0개, vr-repeater·vr-server·vr-viewer state=Off
```

- 조합 검증: 통과. 6.3절의 아홉 조합이 판정 기준을 모두 만족했다. 프로젝트 소유자가 판정 캡처를 검토했고 문제가 없었다.
- 실행파일 일치: 통과. 빌드 산출물, 반입 묶음, VM의 실행파일 해시가 같았다.
- 정리: 통과. 시험용 VNC 비밀번호 파일은 호스트와 VM에서 지웠다. VM 체크포인트 `baseline`은 바꾸지 않았다.

## 7. 계약 6.4 비교 캡처

3.3절의 계획 8개를 Repeater 1.6.0.0과 VNC Relay에 실행한 캡처로 비교했다. VNC Relay의 37회차가 모두 유효했다. 아래에는 각 확인 항목의 유효 결과를 남긴다. 차이는 [계약 3.1절](repeater-handshake-v1.md#31-대상-버전-repeater와-다르게-처리하는-입력), 3.3절의 무시할 차이, 3.5절의 형식과 무관한 관찰로 설명된다. Mode I 대상 연결 시간(M1-05)과 활성 세션의 정상 EOF 뒤 종료(T-02)는 7.4절에 자세히 적었다.

### 7.1 실행 방법과 범위

| 항목 | 내용 |
| --- | --- |
| 실행 환경 | 3.2절의 Windows VM이다. Ubuntu VM이 없어 Ubuntu VNC Relay의 비교 캡처는 미실행이다 |
| 입력 | 3.3절의 계획 `p10-mode1`·`p11-mode2`·`p12-errors-split`·`p13-termination`·`p15-followup`·`p17-followup2`·`p16-websocket`·`p18-websocket-wss`다(`D:\vr-lab\plans\`). 단계·명령·원시 바이트·캡처 시간은 Repeater 관찰과 같다 |
| Repeater 단계의 대응 | Repeater 시작·재시작 단계에 VNC Relay를 시작·재시작했다. VNC Relay에는 Repeater의 웹 상태 페이지가 없다. 상태 수집은 프로세스와 TCP 상태만 기록했고, 웹 요청은 보내지 않았다 |
| 회차 수 | 시나리오마다 1회다. 계약 6.1절 5단계의 회차 수는 제안이다. Repeater 쪽 재현성은 3.4절의 2회로 확인했다. VNC Relay 쪽 차이는 운영 로그의 사건과 설정값으로 원인을 확인했다 |
| 부팅 단위 | 계획마다 세 VM을 한 번 부팅했고, 게스트 준비 단계가 있는 WebSocket 시나리오는 시나리오마다 부팅했다. 37회차에 부팅은 11번이었다. 부팅마다 한 시간 안에 끝났고, 회차 기록의 VM 가동 시간은 최장 24분 10초였다. 모든 회차에서 세 VM이 `Running`이었다. 이 비교 캡처는 회차마다 새로 부팅하는 조건 대신 부팅마다 한 시간 안에 끝내고 도중에 꺼진 VM이 없는 조건을 적용했다 |
| 진입점 | Repeater 관찰의 입력과 연결 구조를 재현하려고 TCP 진입점만 켰다. W-01·W-01b는 TCP 5901로 가고, W-02·W-03b의 시험용 브리지는 `127.0.0.1:5901`로 넘긴다. VNC Relay 자체의 WS·WSS 진입점 검증은 6절에 있다 |
| 비교 방법 | `D:\vr-lab\scripts\summarize.py`의 함수로 Repeater run1과 VNC Relay run1의 TCP 스트림을 같은 방식으로 요약해 비교했다. 3.3절의 무시할 차이를 적용했다. 남은 차이는 VNC Relay 운영 로그의 사건으로 원인을 확인했다 |

### 7.2 환경과 버전

| 항목 | 값 |
| --- | --- |
| 실행 시각 | 2026-11-06 22:07:39 ~ 2026-11-07 00:09:07 KST(UTC+9). 운영 로그 `ts`는 UTC다 |
| VM | 3.2절 구성과 `baseline` 체크포인트. Viewer·Server·noVNC·Edge와 게스트 OS는 `baseline`의 것을 그대로 썼다. 하네스가 마지막에 세 VM을 `baseline`으로 되돌렸다. 하네스 요약 `observe-relay.json`에 오류는 없다 |
| VNC Relay | 커밋 `70ff541`의 Windows release 빌드, `vnc-relay 0.1.0`, SHA-256 `d94e4dd34a297cfdd008bf3e856c30deb2c5366dc6d844e6502aa3aaf097b5fc`(6.2절과 같은 파일). 부팅마다 `vr-repeater`의 `C:\vr\relay\vnc-relay.exe`에 반입하고 해시가 같은지 확인했다 |
| VNC Relay 설정 | 회차마다 `relay.toml`을 새로 썼다. `tcp_viewer` `0.0.0.0:5901`은 192.0.2.10과 브리지용 127.0.0.1을 허용했다. `tcp_server` `0.0.0.0:5500`은 192.0.2.30만 허용했다. 포트와 주소 범위는 Repeater와 같다. Mode II는 켰다. Mode I은 p10에서만 켰다. 허용 대상은 `192.0.2.30:5900`, `192.0.2.30:5999`, `vnc-server.test:5900`, `[2001:db8:1::30]:5900`, `vnc-server6.test:5900`이다. M1-04에서는 Repeater의 `refuse_con=192.0.2.30`에 맞춰 빈 목록으로 두었다. 관리 리스너는 127.0.0.1:18080, 상태 리스너는 127.0.0.1:18081이다. 시간 설정은 기본값이다(H·C·E 10초). 관리자 계정 파일은 6절과 같이 자리표시자 해시만 담았고, 로그인하지 않았다 |
| 방화벽 | `vr-repeater`에 프로그램 규칙 `vr-lab vnc-relay`를 두었다. 원격 주소는 192.0.2.0/24와 2001:db8:1::/64로 한정했다. 이 규칙은 `baseline` 복원으로 사라졌다 |
| 캡처 | 3.3절과 같다. `vr-repeater`의 `dumpcap`이 `vr-lab` 어댑터에서 계획의 캡처 시간 동안 기록했다 |
| 종료 | 회차마다 VNC Relay 프로세스를 강제로 끝냈다. 그래서 종료 기록 파일은 모두 `exit=-1`이다. 표준 오류 파일은 모두 비어 있다 |

### 7.3 시나리오별 결과

"Repeater와의 차이"는 3.4절 run1과 비교한 결과다. "같음"은 3.3절의 무시할 차이를 빼면 연결, 방향별 바이트, 닫는 방식이 같다는 뜻이다. 시작 바이트를 보내는 시점은 모든 회차에서 VNC Relay가 약 1초 빨랐다(계약 3.1절 첫 행). 이 열에는 따로 적지 않았다.

| 시나리오 | VNC Relay 결과(운영 로그) | Repeater와의 차이 | 근거 |
| --- | --- | --- | --- |
| M1-01 | `session_started`(대상 `192.0.2.30:5900`) | 같음. 대상 연결도 약 1초 빠르다 | 3.1절 시작 바이트 |
| M1-02 | Viewer와 원시 입력 모두 포트 5900으로 `session_started` | 닫은 뒤 상대 FIN이 남은 차이만 있다 | 3.3절 |
| M1-03 | `VNC-SERVER.TEST:5900`을 `vnc-server.test`로 해석해 `session_started` | 같음 | |
| M1-04 | `connection_rejected` `target_not_allowed`. 같은 ms에 Viewer에 FIN, 대상 연결 없음 | 같음 | |
| M1-05 | `target_connect_failed` `reason=connect_timeout` `elapsed_ms=10001` | Viewer에 FIN을 보내기까지 첫 SYN 기준 Repeater는 약 21초, VNC Relay는 약 10초다 | 7.4절 |
| M1-06 | 접두어 없는 `2001:db8:1::30:5900`은 `handshake_invalid`. 대괄호 IPv6와 `vnc-server6.test`는 IPv6 SYN을 보냈지만 응답이 없어 `connect_timeout`. Viewer 프로세스는 3.4절처럼 연결하지 않았다 | Repeater는 원시 입력 세 가지를 모두 바로 닫았고 IPv6 연결을 시도하지 않았다 | 3.1절 IPv6 대상. Server VM이 IPv6 SYN에 답하지 않은 이유는 확인하지 않았다[INFERENCE: winvnc가 IPv6에서 대기하지 않았거나 게스트 방화벽이 버렸다] |
| M2-01·M2-02 | `waiting_registered` 뒤 `session_started` | 같음 | |
| M2-03 | 두 연결 모두 대기 | 같음 | |
| M2-04·M2-05 | 뒤에 온 같은 역할 연결을 `duplicate_id`로 닫았다(FIN·RST). 먼저 온 연결은 매칭됐다 | Repeater는 뒤에 온 연결을 닫지 않았다 | 3.1절 중복 ID |
| M2-06 | `1`·`123456789`·`1234567890`·20자리·247자리 다섯 쌍이 매칭됐다. 빈 ID 두 연결과 접두어 없는 Server는 `handshake_invalid`, 접두어 없는 Viewer는 `mode_disabled`로 닫혔다 | 앞 세 쌍은 같다. Repeater는 10자리 세션 중 웹 요청에 끝나 나머지를 관찰하지 못했다(3.4절). 나머지는 M2-06b와 같다 | 3.1절 형식이 틀린 요청 |
| M2-07 | 다섯 쌍 열 연결이 모두 `handshake_invalid` | Repeater는 `12-34`·`12 34`·`12`+0xE9+`34` 쌍을 매칭했다 | 3.1절 숫자 뒤 바이트 |
| M2-08 | `0456` 쌍만 매칭됐다. 나머지 네 연결은 대기하다 클라이언트가 닫았다(`waiting_removed` `peer_eof`) | Repeater는 세 쌍 모두 매칭했다 | 3.1절 앞자리 0 |
| M2-09 | 네 연결 모두 `handshake_invalid` | 같음 | |
| E-01 | Server `XX:1234`는 `handshake_invalid`, Viewer `A`×250은 `mode_disabled`로 바로 닫혔다 | Repeater는 두 연결을 닫지 않았다 | 3.1절 형식이 틀린 요청·꺼진 모드 |
| E-02 | 짧게 보내고 닫은 두 연결은 `connection_aborted` `peer_eof`. 이어진 쌍은 매칭됐다 | 같음 | |
| E-03 | 두 쌍이 매칭되고 바이트가 전달됐다 | FIN 순서만 다르다 | 3.3절 |
| E-04 | 네 연결 모두 `connection_aborted` `handshake_timeout`(H 10초) | Repeater는 약 6초 뒤 닫았다 | 3.1절 초기 교환 시간 |
| S-01~S-03 | 매칭과 바이트 전달 | 같음 | |
| T-01 | Viewer RST 뒤 Server에 FIN(`connection_error`, `side=viewer`) | 같음 | |
| T-02 | Server FIN 0.87ms 뒤 Viewer에 FIN. Server 쪽 FIN은 10.025초 뒤(`session_closed` `close_cause=close_timeout` `side=server`) | Repeater는 0.75ms 안에 양쪽에 FIN을 보냈다 | 7.4절 |
| T-03 | Viewer 종료 뒤 Server에 FIN. Server가 다시 등록했고 다시 실행한 Viewer와 매칭됐다 | Repeater의 FIN 뒤 RST만 다르다 | 3.3절 |
| M2-06b | 20자리·247자리 쌍이 매칭됐다. 빈 ID 두 연결과 접두어 없는 Server는 `handshake_invalid`, 접두어 없는 Viewer는 `mode_disabled` | Repeater는 접두어 없는 쌍을 닫지 않았다 | 3.1절 형식이 틀린 요청 |
| M2-11 | 두 번 모두 매칭되고 정상 종료됐다 | Repeater B는 웹 요청에 끝났다(3.4절). VNC Relay에는 웹 상태 페이지가 없어 같은 입력이 없다. 나머지는 FIN 순서만 다르다 | 3.3절·3.5절 |
| M2-12 | `12ab`·`12AB`·`56-78`은 `handshake_invalid`. `56`은 대기하다 클라이언트가 닫았다. `999999999` 쌍은 매칭됐다 | Repeater는 앞 두 쌍을 매칭하고 `999999999` Server를 닫았다 | 3.1절 숫자 뒤 바이트·`999999999` |
| S-04 | 네 쌍 매칭 | 같음 | |
| S-05 | 두 쌍이 매칭되고 매칭 전 바이트가 전달됐다 | 종료 순서만 다르다. [INFERENCE: Repeater run1의 RST는 하네스 재시작과 겹친 것으로 보인다.] | 3.3절 |
| M2-13 | 서로 다른 20자리 쌍과 `2147483647`/`2147483648`은 매칭되지 않았다. `999999998`·`99999999` 쌍과 Viewer 먼저 `999999999` 쌍은 매칭됐다 | Repeater는 20자리 쌍을 매칭했고 `999999999` Server를 닫았다 | 3.1절 20자리·`999999999` |
| M2-14 | 네 쌍 여덟 연결이 모두 매칭되지 않고 대기하다 클라이언트가 닫았다 | Repeater는 `100`/`6000`을 뺀 세 쌍을 매칭했다 | 3.1절 5900 차이 |
| W-01 | noVNC의 평문 HTTP 업그레이드 요청 두 건을 `mode_disabled`로 바로 닫았다. Server는 대기했다 | Repeater는 닫지 않았다 | 3.1절 꺼진 모드·WebSocket·TLS |
| W-02 | 브리지를 거친 noVNC가 매칭됐다. 페이지 제목은 `vr-server ( 192.0.2.30 ) - application mode - noVNC`였다 | 화면 갱신 바이트 수만 다르다 | 3.3절 |
| W-01b | TLS ClientHello 두 건을 `mode_disabled`로 닫았다(FIN·RST) | Repeater는 닫지 않았다 | 3.1절 꺼진 모드·WebSocket·TLS |
| W-03b | WSS 브리지를 거친 noVNC가 매칭됐다. 페이지 제목은 W-02와 같다 | 화면 갱신과 TLS 레코드 크기만 다르다 | 3.3절 |

### 7.4 시간 제한에 따른 차이

두 차이는 VNC Relay의 시간 제한 설정에서 나온다. [계약 3.1절](repeater-handshake-v1.md#31-대상-버전-repeater와-다르게-처리하는-입력)에 명시된 차이이며, 아래 스펙과 설정값을 따른다.

| 시나리오 | Repeater 1.6.0.0 | VNC Relay | 스펙 근거 |
| --- | --- | --- | --- |
| M1-05 응답 없는 Mode I 대상 | 첫 SYN 뒤 1·2·4·8초 간격으로 SYN을 다시 보냈다. 첫 SYN 21.0초 뒤 Viewer에 FIN을 보냈다(run1·run2 같음) | 첫 SYN 뒤 1·2·4초 간격으로 다시 보냈다. 첫 SYN 약 10.0초 뒤 Viewer에 FIN을 보냈다. 로그는 `target_connect_failed` `reason=connect_timeout` `elapsed_ms=10001`이다 | [relay.md 3.2절](../specs/relay.md#32-mode-i), 설정 계약 `connect_secs`(C, 기본 10초, 1~120초) |
| T-02 활성 세션의 Server 정상 종료 | Server FIN 0.75ms 뒤 Viewer와 Server 양쪽에 FIN을 보냈다 | Server FIN 0.87ms 뒤 Viewer에 FIN을 보냈다. Viewer 방향 쓰기만 닫은 것이다. Viewer가 연결을 닫지 않아 Server 쪽 FIN은 E가 지난 10.025초 뒤에 나갔다. 로그는 `session_closed` `close_cause=close_timeout` `side=server`다 | [relay.md 3.4절](../specs/relay.md#34-중계와-eof), 설정 계약 `close_secs`(E, 기본 10초) |

### 7.6 원본과 명령

원본은 저장소 밖 `D:\vr-captures\compare-6.4\`에 있다. `SHA256SUMS.txt`는 `<해시> *<상대 경로>` 형식이다. 목록의 모든 항목을 .NET SHA-256으로 다시 계산해 일치를 확인했다.

| 실행 | 폴더 | 파일 | `SHA256SUMS.txt` SHA-256 |
| --- | --- | --- | --- |
| 비교 캡처(유효) | `D:\vr-captures\compare-6.4\` | 436개(캡처 37개, 운영 로그 사본 37개 447줄, `harness\` 6개 포함) | `e2d38ea3f153cd5b57ca6eac31a5d9be820e29c768d8fa17572fa995565b192a` |

회차 폴더는 `<계획>\<시나리오>\run1\`이다. 캡처 `capture.pcapng`, 회차 기록 `run.json`, 설정 `relay.toml`, 운영 로그 사본 `guest-relay-<회차>.jsonl`, 표준 오류·종료 기록, 원시 클라이언트 기록, 화면 캡처가 있다. 비교 결과는 `<계획>\<시나리오>\compare.md`다. 운영 로그 사본에는 하네스가 Windows PowerShell 5.1 `Set-Content -Encoding UTF8`로 저장할 때 붙인 UTF-8 BOM이 있다. VM 안의 원본 파일은 `baseline` 복원으로 사라졌다.

하네스 사본은 `harness\`에 있다. `observe-relay.ps1`은 계획을 실행하고, `lab-common.ps1`은 VM 작업을 맡는다. `compare.py`는 캡처 비교 결과를, `events.py`는 운영 로그 요약을 만든다. 유효 실행의 하네스 기록은 `observe-relay.log` 254행부터다. 캡처가 끝나면 VNC Relay를 멈추고 운영 로그를 읽기 공유 모드로 연다.

```
관리자 권한 PowerShell 5.1:
powershell -NoProfile -ExecutionPolicy Bypass -File D:\vr-tools\tmp\compare-6.4\restart.ps1
  → 인수 없이 observe-relay.ps1 실행. 기본값은 계획 8개(p10, p11, p12, p13, p15, p17, p16, p18 순서), 회차 1이다
python D:\vr-tools\tmp\compare-6.4\compare.py p10-mode1 p11-mode2 p12-errors-split p13-termination p15-followup p17-followup2 p16-websocket p18-websocket-wss

observe-relay.log 254~360행: valid=True 37개, valid=False 0개
observe-relay.json: started 2026-11-06T22:07:39, finished 2026-11-07T00:09:07, 오류 없음
```

- 비교 캡처: 통과. 37회차가 모두 유효했다. 차이는 계약 3.1절, 3.3절의 무시할 차이, 3.5절의 형식과 무관한 관찰과 맞았다.
- 실행파일 일치: 통과. 37회차 모두 같은 VNC Relay 해시였다.
- VM 유효 조건: 충족. 부팅마다 한 시간 안에 끝났고 도중에 꺼진 VM이 없었다(7.1절).
- Ubuntu 비교 캡처: 미실행.
