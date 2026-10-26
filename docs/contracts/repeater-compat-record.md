# 호환성 검증 기록

이 기록은 UltraVNC Server·Viewer·Repeater와 noVNC의 호환성 검증 결과를 남긴다. 관찰 허용 검토(1절), 검증 대상 고정(2절), 관찰 증거(3절)를 담는다. 증거 항목은 [Repeater 초기 교환 계약 6.2절](repeater-handshake-v1.md#62-기록할-증거)을 따른다.

- 상태: 검증 대상 고정 완료(TODO 1.3). 관찰은 아직 하지 않았다(TODO 1.4). 사용자 결정 2026-10-22(Q19·Q20) 반영. 2026-10-27에 관찰 준비(설치 확인과 캡처 도구 설치)를 마쳤다(3.1절). 관찰은 1절 범위와 [관찰 절차 6.1절](repeater-handshake-v1.md#61-관찰-절차)(인터넷과 분리한 테스트 네트워크, 별도 호스트·VM, Repeater 구간 캡처)에 따라 한다. 그 환경은 아직 준비하지 않았다.
- 작성일: 2026-10-21. 갱신일: 2026-10-22(사용자 결정 Q19·Q20 반영, 설치 스크립트 근거 추가), 2026-10-27(설치 확인과 캡처 도구 설치 기록, 3.1절·5.1절).
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
| 4.8 | 6.1절 관찰 환경 미준비 | 1.4의 시나리오 관찰을 하지 않았다 | 1절 범위대로 인터넷과 분리한 테스트 네트워크에 Viewer·Server·Repeater 호스트(또는 VM)를 준비하고, Repeater 구간에서 캡처한다 |

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
- 관찰: 미실행. 6.1절 관찰 환경을 아직 준비하지 않았다(4절 4.8).
