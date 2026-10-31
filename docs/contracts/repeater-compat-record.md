# 호환성 검증 기록

이 기록은 UltraVNC Server·Viewer·Repeater와 noVNC의 호환성 검증 결과를 남긴다. 관찰 허용 검토(1절), 검증 대상 고정(2절), 관찰 증거(3절)를 담는다. 증거 항목은 [Repeater 초기 교환 계약 6.2절](repeater-handshake-v1.md#62-기록할-증거)을 따른다.

- 상태: 검증 대상 고정 완료(TODO 1.3). 관찰은 아직 하지 않았다(TODO 1.4). 사용자 결정 2026-10-22(Q19·Q20) 반영. 2026-10-27에 관찰 준비(설치 확인과 캡처 도구 설치)를 마쳤다(3.1절). 2026-11-01에 [관찰 절차 6.1절](repeater-handshake-v1.md#61-관찰-절차) 3·4단계의 관찰 환경을 정하고 구성했다(3.2절, 사용자 결정 Q62~Q66).
- 작성일: 2026-10-21. 갱신일: 2026-10-22(사용자 결정 Q19·Q20 반영, 설치 스크립트 근거 추가), 2026-10-27(설치 확인과 캡처 도구 설치 기록, 3.1절·5.1절), 2026-11-01(관찰 환경 결정과 구성, 1절·3.2절·4절).
- 기준 커밋: `f01598a`(이 갱신 시작 시점 HEAD). 이 기록은 `docs(compat)` 커밋에 담는다.
- 작업 트리: 다른 작업과 같은 트리를 썼다. 문서 작업이며 빌드와 테스트는 하지 않았다.
- 환경: Windows 11 x64 호스트. 내려받기는 `curl.exe`, 해시 계산은 `certutil`, 버전 확인은 PowerShell 파일 버전 정보를 썼다.

## 1. 관찰 허용 검토 (TODO 1.1)

| 항목 | 내용 |
| --- | --- |
| 결정 | 허용 |
| 결정 주체 | 프로젝트 소유자 (사용자 결정 2026-10-21) |
| 범위 | UltraVNC 트래픽의 패킷 관찰. 인터넷과 분리한 테스트 네트워크에서만 한다 |
| 이유 | 제시되지 않았다 |
| 제약 | UltraVNC·websockify·noVNC 소스 코드는 관찰·분석에 쓰지 않는다([관찰 절차 6.1절](repeater-handshake-v1.md#61-관찰-절차)) |
| 6.1절 1단계 판정 | 사용자 결정 2026-11-01(Q65): 이 허용으로 6.1절 1단계를 마친 것으로 본다. 판정 주체는 프로젝트 소유자다. 법률 검토는 하지 않았다 |

이 결정은 프로젝트 소유자의 허용이다. 법률 검토 결과가 아니다. [PRD 9절](../requirements/PRD.md#9-호환성-조사와-독립-구현-원칙)은 패킷 관찰과 역공학 절차의 허용 여부를 관할법·제품 약관·사용 권한에 따라 따로 검토한다고 정한다. 이 기록은 그 결과를 대신하지 않는다.

## 2. 검증 대상 고정 (TODO 1.3)

공식 최신 안정판을 공식 배포 주소에서 내려받아 SHA-256을 계산했다. 버전은 파일 버전 정보로 확인했다. UltraVNC 설치 파일의 해시는 공식 게시값과 비교했다.

### 2.1 UltraVNC Server·Viewer (Windows x64) — 고정

- 버전: 1.8.3.0. 근거: [공식 다운로드 페이지](https://uvnc.com/downloads/ultravnc.html)의 "Latest release version: UltraVNC 1.8.3.0". 공식 발표는 [포럼 글](https://forum.uvnc.com/viewtopic.php?t=38204)(2026-09-14).
- 배포본: `UltraVNC_1830_x64_Setup.exe`, 6,239,336바이트.
- SHA-256: `e9c22419ef3128a707b0d004a2c58e25459528c5c12d104dea56e5e662bd131e`. [1.8.3.0 항목](https://uvnc.com/downloads/ultravnc/172-ultravnc-1-8-3-0.html)의 게시값과 같다.
- 받은 주소: https://uvnc.eu/download/1800/UltraVNC_1830_x64_Setup.exe. 공식 항목의 다운로드 링크가 이 주소로 넘어간다.
- 받은 날짜: 2026-10-21. 2026-10-22에 메인 에이전트가 같은 주소에서 다시 받아 SHA-256을 비교했다. 일치했다.
- OS: Windows 11 x64. 파일 버전 정보는 1.8.3.0, 제품명 `UltraVNC`, 회사명 `uvnc`.
- Viewer 구성요소(설치 스크립트 근거, 추정 한계 있음): 공식 소스 저장소의 설치 스크립트 [`UltraVNC_installer_x64.iss`](https://github.com/ultravnc/UltraVNC/blob/main/UltraVNC_installer_x64.iss)(main 브랜치, 2026-10-22 메인 에이전트가 읽음)는 구성요소 `UltraVNC_Server`(`winvnc.exe`), `UltraVNC_Viewer`(`vncviewer.exe`), `UltraVNC_Repeater`(`repeater.exe`)를 정의한다. 완전 설치 유형 `full`은 셋을 모두 포함한다. 출력 파일 이름 `UltraVNC_{#MyAppOutputVersion}_x64_Setup`이 배포본 이름과 맞는다. 그래서 Viewer는 이 설치 파일에 든다고 본다.
- 한계: 저장소에는 태그와 릴리스가 없다(GitHub API가 두 목록 모두 `[]`를 반환). 이 스크립트가 1.8.3.0 빌드에 쓰였다는 증거는 없다. Viewer 파일 버전은 설치 뒤 확인한다(TODO 1.4, 3절).
- 참고 관찰(고정 아님): 같은 스크립트는 설치 파일에 `repeater.exe` 구성요소도 둔다. 그 파일의 버전은 확인하지 않았다. 고정 대상 Repeater는 2.2절의 `repeater_v160.zip`이다(사용자 결정 2026-10-22(Q20)).

### 2.2 UltraVNC Repeater (Windows x64) — 고정 (사용자 결정 2026-10-22(Q20))

- 버전: 1.6.0.0(파일 버전 정보).
- 배포본: `repeater_v160.zip`, 224,709바이트. 압축 안에는 `repeater.exe` 한 개가 있다.
- SHA-256: `82d9e7f06266a291b4988e36f4d33e127d63adb93269f3b24c432718f35de1de`.
- 받은 주소: https://uvnc.eu/download/repeater/repeater_v160.zip. [공식 Repeater 페이지](https://uvnc.com/downloads/ultravnc-repeater/83-ultravnc-repeater.html)의 항목 "ultravnc_repeater_1400"(표기 1.4.0.0)의 다운로드 링크가 이 주소로 넘어간다.
- 받은 날짜: 2026-10-21.
- OS: Windows 11 x64 대상. 실행 검증은 1.4에서 한다.
- 불일치: 같은 항목은 1.4.0.0, 72.05 KB로 표기한다. 받은 파일은 1.6.0.0, 224,709바이트다. 원인은 확인하지 않았다. 사용자 결정 2026-10-22(Q20): 받은 파일 1.6.0.0을 고정 대상 버전으로 채택한다. 공식 표기 1.4.0.0과 다르다는 사실은 이 기록에 남긴다(4절 4.2).

### 2.3 UltraVNC Repeater (Ubuntu 24.04 LTS x64) — 고정 범위 밖 (사용자 결정 2026-10-22(Q19))

- 결정: 사용자 결정 2026-10-22(Q19). Ubuntu `uvncrepeater`는 관찰을 미룬다. 고정 대상에서 뺀다. 고정 범위는 Windows Repeater(2.2절)다.
- 참고 기록(고정 아님): `uvncrepeater.tar.gz`, 57,888바이트, SHA-256 `a0fe948ba477685d4190223b35a0691b2e657751d30b97a9838bbfa31ca6ed8f`, 받은 주소 https://uvnc.eu/download/repeater/uvncrepeater.tar.gz(공식 Repeater 페이지의 unix 링크), 받은 날짜 2026-10-21. C++ 소스 패키지다.
- 버전 미확인: 파일 이름과 폴더 이름에 버전이 없다. 공식 페이지에서도 Ubuntu 빌드 버전을 찾지 못했다. 압축 안 파일은 소스 코드이므로 읽지 않았다.
- 영향: 없음(범위 밖). RLY-TBD-001은 Windows Repeater 고정으로 해소했다(4절 4.1). Ubuntu 쪽 검증은 2.6절에 따로 적는다.

### 2.4 noVNC (브라우저 클라이언트) — 고정

- 버전: 1.7.0. 근거: [GitHub 릴리스 v1.7.0](https://github.com/novnc/noVNC/releases/tag/v1.7.0)(2026-04-28 공개, 태그 커밋 `63107bd06d9e1f6136ff21aeda8cd62cbf0d433e`).
- 배포본: `novnc-v1.7.0.zip`(GitHub 태그 아카이브 `v1.7.0.zip`의 저장 이름), 823,923바이트.
- SHA-256: `d55e9cce9a611294df33c7200f32f35108652de2d86a468bebcfc6f2a1c9a196`.
- 받은 주소: https://github.com/novnc/noVNC/archive/refs/tags/v1.7.0.zip
- 받은 날짜: 2026-10-21.
- OS: 브라우저에서 실행하는 클라이언트라 OS에 묶이지 않는다. 검증 OS는 1.4에서 기록한다.
- 주의: GitHub가 만든 아카이브를 다시 받아 해시를 비교하지 않았다.

### 2.5 받지 않은 배포본

- UltraVNC 1.8.3.0의 다른 배포본(zip, msi, x86 설치 파일)은 받지 않았다. 이 기록은 x64 설치 파일 하나를 고정한다.
- 받은 파일은 해시와 주소를 기록한 뒤 지웠다. 해시와 주소가 증거다.

### 2.6 Ubuntu 24.04 LTS x64 검증 (별도 기록)

- 결정: 사용자 결정 2026-10-22(Q19). Ubuntu 서버·클라이언트 검증은 2.3절의 Ubuntu Repeater 항목과 따로 기록한다.
- 범위: PRD의 '서버 플랫폼' 행은 Windows 11 x64·Ubuntu 24.04 LTS x64 양쪽에서 실행·종료·호환성을 검증하라고 정한다. 이 절은 Ubuntu 쪽 증거를 모은다.
- 상태: 미실행. 실행 증거는 3절에 적는다.

## 3. 관찰 증거 (TODO 1.4)

관찰은 아직 하지 않았다. 6.2절의 증거 항목은 모두 미실행이다.

| 증거 | 상태 | 이유 |
| --- | --- | --- |
| 대상·환경·입력·원본·결과·해석 | 미실행 | 1.4를 시작하지 않았다. 선행 1.3은 2026-10-22에 고정을 마쳤다(2절) |
| 설치 뒤 `vncviewer.exe` 파일 버전 확인 | 확인(2026-10-27) | 1.8.3.0. Viewer 구성 요소가 설치됐다(3.1절) |
| 승인 | 1절 결정(2026-10-21) | 6.1절 1단계 검토 결과 |

### 3.1 관찰 준비: 설치 확인과 캡처 도구 (2026-10-27)

이 절은 관찰 준비 기록이다. 6.2절의 관찰 증거가 아니다. 관찰은 1절 범위와 [관찰 절차 6.1절](repeater-handshake-v1.md#61-관찰-절차)을 따른다.

- 결정: 사용자 결정 2026-10-27(Q61). 이 PC(Windows 11 Pro x64, 빌드 26200)에 Wireshark·Npcap 설치를 허용한다. UltraVNC 1.8.3.0 업그레이드의 경위는 아래 표에 winget 로그로 확인한 사실만 적는다.
- 기준 커밋: `e753c89`(이 갱신 시작 시점 HEAD). 문서 작업이며 빌드와 테스트는 하지 않았다.

#### UltraVNC 1.8.3.0 설치

| 항목 | 결과 |
| --- | --- |
| 이전 설치 | 1.8.2.4(설치일 20260807). 구성 요소 Server·Viewer·Repeater, 서비스 등록 작업은 선택하지 않음 |
| 실행 시각 | 2026-10-24 20:31에 `winget upgrade UltraVNC`가 실행됐다(winget 로그 `WinGet-2026-10-24-20-31-19.273.log`). 메인 에이전트가 실행한 명령이 아니다. 같은 날 17:49에 메인 에이전트가 실행한 `winget upgrade --id uvncbvba.UltraVNC --exact --version 1.8.3.0 --silent`는 관리자 권한 요청 단계에서 끝났다(`ShellExecute installer failed: 2`). 2026-10-27에 다시 실행했을 때는 `사용 가능한 업그레이드를 찾을 수 없습니다.`로 끝났다 |
| 설치 파일 | winget 로그에 `Installer hash verified`가 있다. 비교 기준은 winget 매니페스트 `uvncbvba.UltraVNC` 1.8.3.0의 SHA-256 `e9c22419ef3128a707b0d004a2c58e25459528c5c12d104dea56e5e662bd131e`이고, 2.1절 고정값과 같다. 매니페스트 주소도 2.1절의 받은 주소와 같다. winget이 설치 뒤 설치 파일을 지워 메인 에이전트가 직접 해시를 계산하지는 않았다 |
| 설치 방식 | Inno Setup 6.6.1, 인수 `/SP- /SILENT /SUPPRESSMSGBOXES /NORESTART`. 설치 로그 `uvncbvba.UltraVNC.1.8.3.0-26-10-24-20-31-42.log` |
| 구성 요소·작업 | 선택 구성 요소 `ultravnc_repeater,ultravnc_server,ultravnc_viewer`. 선택 작업 없음. 해제 작업 `installservice,startservice,desktopicon,associate,installdriver`. 설치 날짜 20261024. UltraVNC 서비스는 없다 |
| 파일 버전 | `winvnc.exe` 1.8.3.0(제품명 `UltraVNC Server`), `vncviewer.exe` 1.8.3.0(제품명 `UltraVNC Viewer`), `repeater.exe` 1.6.0.0. 세 파일 모두 Authenticode 서명이 유효하다(서명자 `Open Source Developer Rudi De Vos`) |
| 설치 파일 SHA-256 | `winvnc.exe` `9ebd1ec90ba1274d4a183f5bf6588374700052da907fa764a2ae7072e8ae19fe`, `vncviewer.exe` `18760910769dc497266d6ba7a28892925177d0208fe4433bf62f0b16061f3427`, `repeater.exe` `094139d80f1b08f5bb7cbd5222c4a40996cf1bb48154e4920396d2873314ce3b`(698,448바이트) |
| 설정 파일 | `ultravnc.ini`가 업그레이드 전 사본과 바이트 단위로 같다 |
| 방화벽 | 설치 프로그램이 같은 이름의 허용 규칙을 다시 추가해 UltraVNC 규칙이 8개에서 12개가 됐다. 12개 모두 이름별로 속성이 같다. `UltraVNC Server TCP/UDP`는 `winvnc.exe` 인바운드 허용, `UltraVNC Viewer TCP/UDP`는 `vncviewer.exe` 아웃바운드 허용이다. 프로필·원격 주소·로컬 포트는 모두 `Any`다. 허용 범위는 업그레이드 전과 같다 |
| 새 버전 | 2026-10-27 `winget list`에서 1.8.3.1이 보인다. 고정 버전은 1.8.3.0이므로 올리지 않는다 |

#### 설치 파일의 Repeater와 고정 Repeater

| 파일 | 버전 | 크기 | SHA-256 |
| --- | --- | --- | --- |
| 설치 파일이 설치한 `C:\Program Files\uvnc bvba\UltraVNC\repeater.exe` | 1.6.0.0 | 698,448바이트 | `094139d80f1b08f5bb7cbd5222c4a40996cf1bb48154e4920396d2873314ce3b` |
| 2.2절 `repeater_v160.zip` 안의 `repeater.exe` | 1.6.0.0 | 524,904바이트 | `c17655192a0bdac369178156227f6cf6bbfa638514996fd286279f3dee712ae0` |

- 두 파일은 파일 버전이 같지만 다른 파일이다. 둘 다 Authenticode 서명이 유효하다.
- `repeater_v160.zip`은 2026-10-27에 2.2절의 받은 주소에서 다시 받았다. 압축 파일의 SHA-256은 2.2절 고정값 `82d9e7f06266a291b4988e36f4d33e127d63adb93269f3b24c432718f35de1de`와 같다.
- 관찰에는 2.2절 고정 파일을 쓴다(사용자 결정 2026-10-22(Q20)). 보관 위치는 `D:\vr-tools\uvnc\repeater_v160\repeater.exe`다. 저장소 밖에 둔다.
- 시작 메뉴의 "UltraVNC Repeater" 바로 가기는 설치 파일의 `repeater.exe`를 연다. 관찰 기록에는 실행한 파일의 경로와 SHA-256을 함께 적어야 한다.
- 고정 파일 경로에는 방화벽 규칙이 없다. 이 파일이 루프백이 아닌 주소에서 연결을 받으면 Windows 방화벽 확인 창이 뜰 수 있다(추정).

#### 캡처 도구

| 도구 | 버전 | 받은 곳·확인 | 설치 |
| --- | --- | --- | --- |
| Wireshark(`Wireshark.exe`·`dumpcap.exe`·`tshark.exe`) | 4.6.9 x64 | winget `WiresharkFoundation.Wireshark`. 설치 파일 `https://2.na.dl.wireshark.org/win64/all-versions/Wireshark-4.6.9-x64.msi`, 매니페스트 SHA-256 `1975dfc96581a9b674fda3f41ac1585963b4ba52ea9459f3b6d9cedfac3a7217`. winget이 해시를 확인했다 | 2026-10-27 `winget install --silent`, 종료 코드 0. 사용자가 관리자 권한 요청을 승인했다 |
| Npcap 무료판 | 1.90 | `https://npcap.com/dist/npcap-1.90.exe`, 1,325,256바이트, SHA-256 `682b57e1c05ef375931975c9d6ded40888f4bfdf9f4b11571e654f962f1199b6`. 공식 페이지에서 해시 게시값은 찾지 못했다. Authenticode 서명이 유효하다(서명자 `Nmap Software LLC`) | 2026-10-27 사용자가 설치 창을 진행했다. 종료 코드 0. 무료판은 무인 설치를 지원하지 않는다 |

- Npcap 설치 상태: `npcap` 서비스 실행 중. `AdminOnly = 0`, `WinPcapCompatible = 1`. 루프백 캡처 대상 `\Device\NPF_Loopback`(`Adapter for loopback traffic capture`)이 있다.
- 공식 안내에 따르면 `loopback_support` 옵션은 Wireshark에 필요 없다. 이 설치에서도 켜지 않았다.
- Npcap 무료 라이선스는 Nmap·Wireshark·Microsoft Defender for Identity와만 쓰면 설치 대수 제한이 없다. 그 밖의 용도는 5대까지다.
- 설치 프로그램이 만든다고 공식 안내에 적힌 예약 작업 `npcapwatchdog`는 조회하지 못했다(`schtasks` 결과 `ERROR: Access is denied.`).
- 루프백 캡처 시험(관리자 권한 없음): 임시 TCP 연결로 `ping-smoke`를 보내고 `pong:ping-smoke`를 받았다. `dumpcap -i \Device\NPF_Loopback`이 종료 코드 0으로 11개 패킷을 받고 0개를 버렸다. `tshark`로 두 데이터 바이트열(`70696e672d736d6f6b65`, `706f6e673a70696e672d736d6f6b65`)을 확인했다. 시험 파일은 지웠다. 이 시험은 캡처 도구의 동작 확인이다. 6.1절의 캡처 위치(Repeater 쪽 네트워크 구간)를 대신하지 않는다.

### 3.2 관찰 환경 구성 (2026-11-01)

이 절은 6.1절 3·4단계의 환경 기록이다. 6.2절의 "환경" 항목에 쓴다. 시나리오 관찰 증거가 아니다. 환경 준비와 시나리오 캡처는 따로 진행한다. 시나리오 캡처는 이 절의 스모크 확인을 마친 뒤 TODO 1.4의 다음 작업에서 시작한다.

#### 결정

| 결정 | 내용 |
| --- | --- |
| 사용자 결정 2026-11-01(Q62) | 이 PC(Windows 11 Pro x64, 빌드 26200)의 Hyper-V에 VM 세 대를 만들고 `Private` 가상 스위치로 묶는다. 캡처는 Repeater VM 안에서 한다. 호스트 루프백 방식과 캡처 전용 VM 방식은 쓰지 않는다 |
| 사용자 결정 2026-11-01(Q63) | 게스트 OS는 Windows 11 Enterprise 90일 평가판이다 |
| 사용자 결정 2026-11-01(Q64) | 6.1절 2단계에 Ubuntu Repeater 보류(Q19)를 명시한다. Ubuntu 24.04 LTS x64는 `vnc-relay` 비교 캡처에만 쓴다 |
| 사용자 결정 2026-11-01(Q65) | 1절의 소유자 허용으로 6.1절 1단계를 마친 것으로 본다(1절 표). 법률 검토는 하지 않았다 |
| 사용자 결정 2026-11-01(Q66) | Hyper-V 작업은 스크립트 단위로 묶고, 실행할 때마다 사용자가 관리자 권한 요청(UAC)을 승인한다. 계정 그룹은 바꾸지 않는다 |

#### 구성

| VM | 컴퓨터 이름 | IPv4 | IPv6 | 역할과 설치 |
| --- | --- | --- | --- | --- |
| `vr-viewer` | `VR-VIEWER` | `192.0.2.10/24` | `2001:db8:1::10/64` | UltraVNC 1.8.3.0 Viewer 구성 요소만 설치한다. noVNC 1.7.0 압축 파일을 둔다 |
| `vr-repeater` | `VR-REPEATER` | `192.0.2.20/24` | `2001:db8:1::20/64` | 고정 `repeater.exe`(2.2절)를 `C:\vr\repeater\`에 둔다. Wireshark 4.6.9와 Npcap 1.90으로 캡처한다 |
| `vr-server` | `VR-SERVER` | `192.0.2.30/24` | `2001:db8:1::30/64` | UltraVNC 1.8.3.0 Server 구성 요소만 설치한다 |

- 네트워크: Hyper-V `Private` 스위치 `vr-lab`. 호스트 어댑터·게이트웨이·DNS·DHCP가 없다. 주소는 문서용 대역(`192.0.2.0/24`, `2001:db8::/32`)만 쓴다.
- 이름 표기 시나리오: Repeater VM의 `hosts` 파일에 `vnc-server.test`(IPv4)와 `vnc-server6.test`(IPv6)를 Server VM 주소로 등록한다. `.test`는 시험용 예약 최상위 도메인이다.
- VM 사양: 2세대, 2 vCPU, 동적 메모리(시작·최소 2 GB, 최대 4 GB), Secure Boot(`MicrosoftWindows` 템플릿), vTPM, 자동 체크포인트 끔. 처음에는 고정 메모리 4 GB로 정했다. 그러나 2026-11-01 구성 때 호스트의 사용 가능한 메모리가 약 1.5 GB였고, 세 번째 VM이 `0x800705AA`(시스템 리소스 부족)로 시작하지 못했다. 그래서 동적 메모리로 바꿨다.
- 디스크: 평가판 ISO의 이미지를 기본 VHDX 한 개에 적용하고, VM마다 차등 VHDX를 만든다. 기본 VHDX는 읽기 전용으로 둔다. 무인 설정 파일(`unattend.xml`)이 컴퓨터 이름, 시험 전용 로컬 관리자 계정 `lab`, 자동 로그온을 정한다. UltraVNC Server와 Viewer는 로그온한 화면 세션에서 실행해야 하므로 자동 로그온을 켠다.
- 저장 위치: VM과 디스크는 `D:\vr-lab\`, 캡처 원본은 `D:\vr-captures\`에 둔다. 둘 다 저장소 밖이다. C: 드라이브는 여유 공간이 6 GB뿐이라 쓰지 않는다.
- 인증 정보: `lab` 계정 비밀번호는 이 환경에서만 쓰는 무작위 값이다. 실제 인증 정보가 아니다. 저장소 밖 `D:\vr-lab\secrets\lab-account.txt`에 둔다.
- 파일 반입·반출: 호스트에서 PowerShell Direct(`New-PSSession -VMName`, `Copy-Item -ToSession`·`-FromSession`)를 쓴다. PowerShell Direct는 네트워크가 아닌 VMBus로 통신하므로 분리 상태를 깨지 않는다.
- 회차 독립성: 구성과 스모크 확인을 마친 뒤 VM 세 대를 끄고 체크포인트 `baseline`을 만든다. 시나리오 회차마다 이 체크포인트로 되돌린다.
- 구성 스크립트: `D:\vr-lab\scripts\`에 둔다. 실행 기록은 `D:\vr-lab\logs\`에 남는다.

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
| 게스트 OS | 세 VM 모두 `Microsoft Windows 11 Enterprise Evaluation`, 버전 10.0.26300. ISO의 이미지 1번(`Windows 11 Enterprise Evaluation`, 10.0.26300.9457, en-US)을 적용했다. 90일 평가판이다. 평가 기간의 시작 기준과 만료일은 조회하지 않았다(4.9) |
| VM | 세 VM 모두 2세대, 2 vCPU, 동적 메모리 2048~4096 MB, vTPM 켬, Secure Boot 켬. 네트워크 어댑터는 VM마다 한 개이고 `vr-lab`에만 연결된다 |
| 주소 | 구성 표의 IPv4·IPv6 주소와 링크 로컬 주소만 있다. 기본 경로는 VM마다 0개다 |
| `vr-viewer` | `C:\Program Files\uvnc\UltraVNC\vncviewer.exe` 1.8.3.0, SHA-256 `18760910769dc497266d6ba7a28892925177d0208fe4433bf62f0b16061f3427`. 3.1절 호스트 설치 파일과 같다. 설치 경로는 호스트(`uvnc bvba`)와 다르다. 설치 프로그램이 `UltraVNC Viewer TCP`·`UDP` 아웃바운드 허용 규칙을 만들었다. `C:\vr\installers\novnc-v1.7.0.zip`을 두었다(압축은 풀지 않았다) |
| `vr-server` | `C:\Program Files\uvnc\UltraVNC\winvnc.exe` 1.8.3.0, SHA-256 `9ebd1ec90ba1274d4a183f5bf6588374700052da907fa764a2ae7072e8ae19fe`. 3.1절 호스트 설치 파일과 같다. 설치 프로그램이 `UltraVNC Server TCP`·`UDP` 인바운드 허용 규칙을 만들었다. 서비스는 등록하지 않았다 |
| `vr-repeater` | `C:\vr\repeater\repeater.exe` 1.6.0.0, SHA-256 `c17655192a0bdac369178156227f6cf6bbfa638514996fd286279f3dee712ae0`(2.2절 고정 파일). 방화벽 규칙 `vr-lab repeater`는 이 파일의 인바운드를 `192.0.2.0/24`·`2001:db8:1::/64`에서만 허용한다. `hosts`에 `192.0.2.30 vnc-server.test`, `2001:db8:1::30 vnc-server6.test`가 한 번씩 있다 |
| 캡처 도구 | `vr-repeater`에 Wireshark 4.6.9(MSI, `TShark (Wireshark) 4.6.9 (v4.6.9-0-g2d548b197c75)`)와 Npcap 1.90을 설치했다. Visual C++ 런타임은 따로 설치하지 않았다. Npcap은 사용자가 VM 콘솔에서 설치 창을 기본값으로 진행했다. 드라이버 설정은 `AdminOnly=0`, `WinPcapCompatible=1`, `LoopbackSupport=1`이다. 캡처 대상은 `\Device\NPF_{96B13FFE-E67F-48F3-B027-339FFAF761AA} (vr-lab)`다 |
| 체크포인트 | 세 VM이 꺼진 상태에서 만든 `baseline`(Standard, VM 상태 `Off`)이 VM마다 한 개 있다. 세 VM은 꺼져 있다 |

#### 스모크 확인 (2026-11-01)

이 확인은 격리와 캡처 도구의 동작 확인이다. UltraVNC를 실행하지 않았으므로 6.2절의 관찰 증거가 아니다.

| 확인 | 결과 |
| --- | --- |
| 스위치·어댑터 | `vr-lab`의 형식이 `Private`이다. VM 어댑터 세 개가 모두 `vr-lab`에 연결된다 |
| 호스트 분리 | 호스트에 `192.0.2.0/24`·`2001:db8:1::/64` 주소가 없다. 호스트에서 `192.0.2.20` ping이 실패했다 |
| 게스트 분리 | 세 VM 모두 기본 경로 0개. `1.1.1.1:443`과 `[2606:4700:4700::1111]:443` TCP 연결이 실패했다. `www.microsoft.com` DNS 조회가 시간 초과로 실패했다 |
| VM 간 통신 | 세 VM이 서로 IPv4·IPv6 ping에 응답했다(VM마다 4/4) |
| 캡처 | `vr-repeater`에서 `dumpcap`(종료 코드 0)으로 `vr-lab` 어댑터를 25초 캡처했다(패킷 17개). 그동안 `vr-viewer`가 TCP 15000번으로 `ping-smoke`(IPv4)와 `ping-smoke6`(IPv6)을 보내고 `pong:` 응답을 받았다. VM 안과 호스트로 옮긴 사본의 `tshark` 결과가 같았다: `192.0.2.10 70696e672d736d6f6b65`, `192.0.2.20 706f6e673a70696e672d736d6f6b65`, `2001:db8:1::10 70696e672d736d6f6b6536`, `2001:db8:1::20 706f6e673a70696e672d736d6f6b6536`. 사본의 SHA-256은 `1d15d6575192645ebb17aac861a2dd836a1ef61c535bd8100e6d9fe66d29852e`이다. 시험 파일과 임시 방화벽 규칙은 지웠다 |

#### 구성 중 고친 문제

- Hyper-V 통합 서비스 이름이 한국어로 표시되어 `Guest Service Interface` 이름으로 켜지 못했다. 파일 이동은 PowerShell Direct만 쓰므로 이 설정을 뺐다.
- 고정 메모리 4 GB로는 세 번째 VM이 시작하지 못했다. 동적 메모리로 바꿨다(구성 절의 VM 사양).
- UltraVNC 설치 경로를 `uvnc bvba`로 가정해 첫 확인이 실패했다. 실제 경로는 `C:\Program Files\uvnc\UltraVNC`였다. 경로를 검색해 확인하도록 고쳤다.
- 다시 실행하면서 Repeater VM의 `hosts` 항목이 두 번 들어갔다. 중복을 지웠고, 스모크 게이트가 한 번씩만 있는지 확인한다.
- 첫 `baseline` 직전에 `vr-repeater`가 다시 시작되어, 그 VM의 체크포인트가 실행 중 상태로 만들어졌다. 열려 있던 VM 콘솔 창에서 시작된 것으로 보인다(추정). 그 체크포인트를 지우고 꺼진 상태에서 다시 만들었다. 이제 체크포인트 스크립트는 세 VM이 모두 꺼져 있고 `baseline`의 상태가 `Off`인지 확인한다.

#### 구성 스크립트

스크립트는 저장소 밖 `D:\vr-lab\scripts\`에 있다. 모두 관리자 권한으로 실행한다(Q66).

| 파일 | 역할 |
| --- | --- |
| `01-build.ps1`, `unattend-template.xml` | 스위치, 기본 디스크, 차등 디스크, VM 세 대, 게스트 네트워크와 소프트웨어를 구성한다. 다시 실행하면 이미 끝난 단계를 건너뛴다. 설치 파일 해시가 고정값과 다르면 실패로 끝낸다 |
| `02-smoke.ps1` | 분리·통신·캡처 스모크를 실행한다. 조건이 하나라도 다르면 체크포인트를 만들지 않고 실패로 끝낸다. 통과하면 `03-baseline.ps1`을 부른다 |
| `03-baseline.ps1` | VM을 끄고 `baseline`을 만든 뒤 상태를 확인한다 |
| `install-npcap-then-smoke.ps1`, `npcap-ui.ps1`, `open-npcap-installer.ps1` | Npcap 설치 창을 VM 콘솔 세션에 띄우고, 설치를 확인한 뒤 스모크를 실행한다 |
| `check-npcap.ps1`, `status.ps1` | 설치 흔적과 VM·체크포인트 상태를 조회한다 |

## 4. 미해결 항목

| 번호 | 항목 | 영향 | 필요한 결정·작업 |
| --- | --- | --- | --- |
| 4.1 | Repeater Ubuntu 24.04 빌드 버전 미확인 | 고정 범위 밖. RLY-TBD-001 해소에 영향 없음 | 없음. 사용자 결정 2026-10-22(Q19): Ubuntu `uvncrepeater` 관찰을 미루고 Windows Repeater만 고정한다. Ubuntu 쪽 검증은 2.6절 |
| 4.2 | Repeater Windows 표기 1.4.0.0과 받은 파일 1.6.0.0 불일치 | 해소. 사용자 결정 2026-10-22(Q20): 받은 파일 1.6.0.0을 고정 대상으로 채택. 표기 차이의 원인은 확인하지 않음 | 없음 |
| 4.3 | 설치 뒤 `vncviewer.exe` 파일 버전 미확인 | 해소(2026-10-27). `vncviewer.exe` 1.8.3.0과 Viewer 구성 요소 설치를 확인했다(3.1절) | 없음 |
| 4.4 | noVNC 기본 `Sec-WebSocket-Protocol` 값 미관찰 | WS-TBD-005 | 1.4에서 관찰한다 |
| 4.5 | 저작권자·적용 연도 미정 | OPS-TBD-005, LICENSE 고지 | 보류 중인 결정 Q10 |
| 4.6 | 관찰 증거 전부 미수집 | RLY-TBD-002·003, WS-025 | 1.4 실행 |
| 4.7 | 설치 파일의 `repeater.exe`가 고정 `repeater_v160.zip`의 파일과 다름(둘 다 1.6.0.0) | 잘못된 파일로 관찰하면 증거가 고정 대상과 맞지 않는다 | 없음. Q20에 따라 고정 파일을 쓰고, 관찰 기록에 실행 파일 경로와 SHA-256을 적는다(3.1절) |
| 4.8 | 6.1절 관찰 환경 미준비 | 해소(2026-11-01). 3.2절 환경을 구성하고 스모크 확인과 `baseline` 체크포인트를 마쳤다 | 없음. 시나리오 관찰은 4.6(TODO 1.4)에서 한다 |
| 4.9 | 게스트 Windows 평가판 기간과 정품 인증 | 90일 평가판이다. 시작 기준과 만료일은 조회하지 않았다. VM은 인터넷과 분리되어 정품 인증을 할 수 없다. [Evaluation Center 안내](https://www.microsoft.com/en-us/evalcenter/download-windows-11-enterprise)에 따르면, 정품 인증을 하지 않았거나 평가 기간이 끝나면 바탕 화면이 검게 바뀌고, 정품이 아니라는 알림이 계속 뜨며, PC가 한 시간마다 종료된다. 분리 환경에서 이 동작이 언제 시작되는지는 확인하지 않았다 | 시나리오 관찰을 시작하기 전에 한 VM에서 `slmgr /dli`와 `slmgr /xpr`로 라이선스 상태와 만료일을 확인한다. 관찰에 지장이 있으면 3.2절 스크립트로 VM을 다시 만들거나 게스트 OS를 다시 정한다 |

## 5. 검증 명령과 결과

작업 디렉터리는 `D:/vr-scratch/compat/`였다. 명령과 출력 줄은 다음과 같다.

```
curl.exe -L -m 300 -o UltraVNC_1830_x64_Setup.exe https://uvnc.eu/download/1800/UltraVNC_1830_x64_Setup.exe
certutil -hashfile UltraVNC_1830_x64_Setup.exe SHA256
SHA256의 UltraVNC_1830_x64_Setup.exe 해시:
e9c22419ef3128a707b0d004a2c58e25459528c5c12d104dea56e5e662bd131e

certutil -hashfile repeater_v160.zip SHA256
SHA256의 repeater_v160.zip 해시:
82d9e7f06266a291b4988e36f4d33e127d63adb93269f3b24c432718f35de1de

certutil -hashfile uvncrepeater.tar.gz SHA256
SHA256의 uvncrepeater.tar.gz 해시:
a0fe948ba477685d4190223b35a0691b2e657751d30b97a9838bbfa31ca6ed8f

certutil -hashfile novnc-v1.7.0.zip SHA256
SHA256의 novnc-v1.7.0.zip 해시:
d55e9cce9a611294df33c7200f32f35108652de2d86a468bebcfc6f2a1c9a196

PowerShell: Get-Item <파일>.VersionInfo
rep_x/repeater.exe | FileVersion=1.6.0.0 | ProductVersion=1.6.0.0 | ProductName=  distributer
UltraVNC_1830_x64_Setup.exe | FileVersion=1.8.3.0 | ProductVersion=1.8.3.0 | ProductName=UltraVNC | Company=uvnc
```

- 해시 계산: 통과. 4개 파일의 해시를 기록했다. UltraVNC 설치 파일은 공식 게시값과 같다.
- 버전 확인: 통과. UltraVNC 설치 파일 1.8.3.0, Repeater 실행 파일 1.6.0.0.
- 관찰: 미실행. 이유는 3절과 4절에 적었다.
- 2026-10-22 갱신(읽기 전용): 공식 UltraVNC 1.8.3.0 항목 페이지를 읽었다. X64 설치 파일 해시 게시값은 2.1절과 같다. 설치 파일의 Viewer 포함 여부는 이 페이지에 적혀 있지 않다.
- 2026-10-22 추가 확인(메인 에이전트가 실행, 이 기록의 작성자는 실행하지 않음): `UltraVNC_1830_x64_Setup.exe`를 같은 주소에서 다시 받아 SHA-256 `e9c22419ef3128a707b0d004a2c58e25459528c5c12d104dea56e5e662bd131e`와 일치했다. 설치 스크립트 `UltraVNC_installer_x64.iss`(main 브랜치)를 읽어 2.1절의 구성요소 정의를 확인했다.
- 2026-10-22 추가 확인(메인 에이전트가 실행): `innoextract 1.9`로 설치 파일의 파일 목록을 읽으려 했으나 실패했다. 출력: `Unexpected setup loader revision: 2`, `Could not determine setup data version!`. 설치 프로그램은 실행하지 않았다. 설치 파일 안 파일 목록과 `vncviewer.exe` 버전은 아직 확인하지 않았다.

### 5.1 2026-10-27 설치 확인 (메인 에이전트가 실행)

```
winget show --id WiresharkFoundation.Wireshark --exact
설치 관리자 SHA256: 1975dfc96581a9b674fda3f41ac1585963b4ba52ea9459f3b6d9cedfac3a7217

winget install --id WiresharkFoundation.Wireshark --exact --version 4.6.9 --silent
설치 관리자 해시를 확인했습니다.
설치 성공
winget exit=0

Start-Process npcap-1.90.exe -Wait -PassThru
npcap installer exit=0

dumpcap -D
11. \Device\NPF_Loopback (Adapter for loopback traffic capture)

winget upgrade --id uvncbvba.UltraVNC --exact --version 1.8.3.0 --silent
사용 가능한 업그레이드를 찾을 수 없습니다.
winget exit=-1978335189

winget list --id uvncbvba.UltraVNC --exact
UltraVNC uvncbvba.UltraVNC 1.8.3.0 1.8.3.1   winget

PowerShell: Get-Item <파일>.VersionInfo
winvnc.exe | FileVersion=1.8.3.0 | ProductName=UltraVNC Server
vncviewer.exe | FileVersion=1.8.3.0 | ProductName=UltraVNC Viewer
repeater.exe | FileVersion=1.6.0.0
```

- 설치 확인: 통과. 파일 버전, 구성 요소, 서명, 설정 파일 보존을 확인했다.
- 캡처 도구: 통과. 관리자 권한 없이 캡처가 된다(루프백 시험, 준비 확인).
- 관찰: 미실행. 이 기록 시점(2026-10-27)에는 6.1절 관찰 환경을 준비하지 않았다. 환경 준비 결과는 5.2절에 있다.

### 5.2 2026-11-01 관찰 환경 구성 (메인 에이전트가 실행)

관리자 권한이 필요한 단계는 사용자가 UAC를 승인해 실행했다(Q66). 스크립트별 결과 파일은 `D:\vr-lab\logs\`에 있다.

```
certutil -hashfile win11-enterprise-eval-en-us.iso SHA256
bc3f24086ebadc94489066b5ad78089e2cf5c3491e90e790bb81a2b199c10e38

01-build.ps1 → 01-summary.json
gate: pass
vr-viewer gen2 cpu2 dynamic=True startup=2048MB min=2048MB max=4096MB tpm=True secureboot=On (세 VM 같음)
vncviewer.exe 1.8.3.0 18760910769dc497266d6ba7a28892925177d0208fe4433bf62f0b16061f3427 C:\Program Files\uvnc\UltraVNC\vncviewer.exe
winvnc.exe 1.8.3.0 9ebd1ec90ba1274d4a183f5bf6588374700052da907fa764a2ae7072e8ae19fe C:\Program Files\uvnc\UltraVNC\winvnc.exe
C:\vr\repeater\repeater.exe 1.6.0.0 c17655192a0bdac369178156227f6cf6bbfa638514996fd286279f3dee712ae0
TShark (Wireshark) 4.6.9 (v4.6.9-0-g2d548b197c75).

install-npcap-then-smoke.ps1 → install-npcap.log
service: Running, Npcap 1.90, AdminOnly=0 WinPcapCompatible=1 LoopbackSupport=1

02-smoke.ps1 → 02-smoke.json
gate: pass
switch: Private; vmAdapters: vr-repeater vr-lab, vr-server vr-lab, vr-viewer vr-lab; hostLabAddresses: (없음); hostPing: False
vr-viewer routes=0 v4=fail v6=fail dns=fail(timeout) peers=4/4 True (세 VM 같음)
192.0.2.20 sent=ping-smoke got=pong:ping-smoke
2001:db8:1::20 sent=ping-smoke6 got=pong:ping-smoke6
capture exit=0, Number of packets: 17

03-baseline.ps1 → 03-baseline.json
gate: pass
vr-viewer baseline Standard vmstate=Off 2026-11-01T01:27:11
vr-server baseline Standard vmstate=Off 2026-11-01T01:27:12
vr-repeater baseline Standard vmstate=Off 2026-11-01T01:29:03
```

- 환경 구성: 통과. 3.2절 구성 결과와 같다.
- 분리·통신·캡처 스모크: 통과. 실패 조건이 하나라도 있으면 체크포인트를 만들지 않는 게이트로 확인했다.
- 관찰: 미실행. 시나리오 캡처는 TODO 1.4의 다음 작업이다.
