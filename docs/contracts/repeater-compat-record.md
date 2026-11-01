# 호환성 검증 기록

이 기록은 UltraVNC Server·Viewer·Repeater와 noVNC의 호환성 검증 결과를 남긴다. 관찰 허용 검토(1절), 검증 대상 고정(2절), 관찰 증거(3절)를 담는다. 증거 항목은 [Repeater 초기 교환 계약 6.2절](repeater-handshake-v1.md#62-기록할-증거)을 따른다.

- 상태: 검증 대상 고정 완료(TODO 1.3). 관찰 완료(TODO 1.4): 2026-11-01에 [관찰 절차 6.1절](repeater-handshake-v1.md#61-관찰-절차) 5단계의 여섯 시나리오 묶음을 각각 2회 이상 캡처하고 6.2절의 증거를 3절에 기록했다. 사용자 결정 2026-10-22(Q19·Q20), 2026-11-01(Q62~Q67) 반영. 계약 1절 반영과 채택(TODO 1.5)은 2026-11-01에 마쳤다(사용자 결정 Q68~Q73). 중복 ID 정책(TODO 1.6)은 2026-11-01에 정했다(사용자 결정 Q74·Q75). 다른 계약 반영(TODO 1.7)도 2026-11-01에 마쳤다.
- 작성일: 2026-10-21. 갱신일: 2026-10-22(사용자 결정 Q19·Q20 반영, 설치 스크립트 근거 추가), 2026-10-27(설치 확인과 캡처 도구 설치 기록, 3.1절·5.1절), 2026-11-01(관찰 환경 결정과 구성, 1절·3.2절·4절), 2026-11-01(시나리오 관찰과 해석, 3절·3.3~3.6절·4절·5.3절, 사용자 결정 Q67), 2026-11-01(계약 채택에 따른 4.6·4.10·4.13 갱신, 사용자 결정 Q68~Q73), 2026-11-01(중복 ID 정책 결정에 따른 4.6·4.11 갱신, 사용자 결정 Q74·Q75), 2026-11-01(다른 계약 반영에 따른 4.4·4.6 갱신, TODO 1.7).
- 기준 커밋: `ff6bbb5`(시나리오 관찰 기록 시작 시점 HEAD). 이 기록은 `docs(compat)` 커밋에 담는다.
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

관찰은 2026-11-01에 마쳤다. 6.2절의 증거 항목별 위치는 다음과 같다.

| 증거 | 상태 | 위치 |
| --- | --- | --- |
| 대상 | 기록 | 2절(고정 버전과 해시), 3.2절(VM에 설치한 실행 파일의 경로·버전·해시), 3.4절 외부 WebSocket(Edge 버전) |
| 환경 | 기록 | 3.2절(VM·네트워크·캡처 도구), 3.3절(라이선스 상태, Repeater 기본 설정과 Mode I 설정, 회차 진행) |
| 입력 | 기록 | 3.4절(시나리오별 명령줄·입력 바이트·ID·대상 값) |
| 원본 | 기록 | 3.6절(캡처 해시와 보관 위치). 방향별 바이트 덤프와 시각은 시나리오별 `summary.md` |
| 결과 | 기록 | 3.4절(Repeater와 끝점의 반응, 매칭 여부, 종료 방식) |
| 해석 | 기록 | 3.5절(계약 1절 항목별 결론과 근거 시나리오) |
| 승인 | 기록 | 1절 결정(2026-10-21)과 3.2절 Q65(6.1절 1단계) |
| 설치 뒤 `vncviewer.exe` 파일 버전 확인 | 확인(2026-10-27) | 1.8.3.0. Viewer 구성 요소가 설치됐다(3.1절) |

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

### 3.3 관찰 실행 방법 (2026-11-01)

이 절은 6.2절의 "환경"과 "입력" 중 모든 회차에 공통인 사항이다. 시나리오별 입력과 결과는 3.4절, 해석은 3.5절, 원본은 3.6절에 있다.

#### 결정과 라이선스 상태

| 결정 | 내용 |
| --- | --- |
| 사용자 결정 2026-11-01(Q67) | 게스트 Windows를 평가판 알림 모드 상태 그대로 쓴다. 회차마다 `baseline`에서 새로 부팅하고 한 시간 안에 끝낸다. 회차 도중에 VM이 꺼지면 그 회차는 무효로 하고 다시 실행한다. 라이선스 보호를 끄거나 우회하지 않는다. 이 라이선스 상태가 약관에 맞는지는 판단하지 않았다 |

- 라이선스 상태(2026-11-01, 세 VM 같음): `slmgr /dli`가 `License Status: Notification`, `Notification Reason: 0xC004F009 (grace time expired).`를 출력했다. `slmgr /xpr`는 `Windows is in Notification mode`를 출력했다. 결과 파일은 `D:\vr-lab\logs\04-explore.json`이다.
- 회차 길이: 부팅부터 종료까지 회차당 약 4~6분이었다. 모든 증거 회차가 한 시간 안에 끝났고, 도중에 꺼진 VM은 없었다.

#### 회차 진행

하네스 `05-observe.ps1`이 계획 파일(JSON) 하나를 받아 시나리오마다 독립 회차를 2회 실행한다. 회차는 다음 순서로 진행한다.

1. 세 VM을 `baseline`으로 되돌리고 시작한다.
2. Repeater VM에서 `C:\vr\repeater\repeater.exe`를 처음 실행한다. 첫 실행 때 Repeater는 "UltraVNC Repeater - Admin Password" 모달 창을 띄운다. 창에는 "first run configuration saved", 웹 관리 주소 `http://localhost:80/`, 사용자 `admin`, 무작위 비밀번호가 있다. 이 창이 열려 있는 동안 Repeater는 포트를 열지 않는다. 하네스는 UI 자동화로 비밀번호를 읽어 메모리에만 둔다. 기록에는 창 제목과 읽기 성공 여부만 남기고, 이 창의 화면은 캡처하지 않는다.
3. 시나리오에 설정 변경이 있으면 웹 관리 화면의 저장 요청(`/testaction.cgi?...`, GET 한 번)으로 바꾼다. 이 요청에는 HTTP 응답이 오지 않고, 그 뒤 웹 서버도 응답하지 않는다. 그래도 `settings2.txt`는 다시 쓰인다. 그래서 하네스는 이 파일의 해시가 바뀔 때까지 최대 30초 기다린 뒤 Repeater를 강제 종료한다. `taskkill`(강제 옵션 없음)로는 종료되지 않았다.
4. `dumpcap -i <vr-lab 어댑터> -a duration:<captureSec> -w capture.pcapng`로 캡처를 시작한다. 2초 뒤 Repeater를 다시 시작한다. `/settings.html`에서 실제 설정을 읽어 `run.json`의 `repeaterSettings`에 기록한다. 계획의 값과 다르면 그 회차는 실패로 끝난다.
5. 계획의 단계를 순서대로 실행한다. 단계에는 실제 Viewer·Server 실행(자동 로그온한 화면 세션), 원시 TCP 클라이언트, 프로세스 종료, 상태 수집, 스크린숏, 대기가 있다. 상태 수집은 세 VM의 프로세스와 TCP 연결 상태, Repeater 웹 상태 페이지(`/`)를 남긴다. 웹 요청을 빼는 상태 수집(`noWeb`)도 있다(M2-11 이후).
6. 캡처가 끝나면 원본을 호스트로 복사하고 SHA-256을 계산한다.
7. 회차는 오류가 없고, 세 VM이 회차 내내 실행 중이었고(가동 시간 비교), `dumpcap`이 종료 코드 0으로 끝났을 때만 유효하다. 무효 회차는 한 번 다시 실행한다. 증거 회차는 모두 첫 시도에서 유효했다(3.6절).

- 원시 TCP 클라이언트: 게스트 PowerShell 스크립트로 Repeater에 접속해 계획에 적힌 바이트를 정해진 간격으로 보낸다. 받은 바이트와 시각(접속 시작 기준 ms)을 `raw-*.txt`에 남긴다. 실제 Viewer·Server가 만들 수 없는 입력(형식 오류, 분할, 경계 ID, 같은 역할 중복)에 쓴다. 3.4절에서 "패딩"은 250바이트가 될 때까지 NUL(0x00)을 채운 것이다. "표식"은 매칭 뒤 전달 여부를 판정하려고 보낸 짧은 바이트열(예: `S>after`, `V>after`)이다.
- Repeater 대기 포트: 매 회차 `0.0.0.0:5901`, `0.0.0.0:5500`, `0.0.0.0:80`만 대기했다. IPv6 대기 포트는 없다. Viewer는 5901, Server는 5500으로 접속했다.
- Repeater 기본 설정(첫 실행 뒤 `/settings.html`): `keepalive=False mode1=False mode2=True server_port=5901 viewer_port=5500 allow_on=True allow_con=(빈 값) refuse_on=False refuse_con=(빈 값) id_on=False id_con=(빈 값) web_port=80 ucom_on=False`. 필드 이름은 화면의 이름 그대로다.
- Mode I 시나리오(M1-01~M1-06)는 `mode1=True`, `allow_on=False`로 바꿨다. M1-04는 `refuse_on=True`, `refuse_con=192.0.2.30`도 바꿨다. 나머지 시나리오는 기본 설정이다.
- 회차 비교: `summarize.py`가 시나리오마다 `summary.md`를 만든다. TCP 스트림마다 방향별 바이트 총량, 앞 300바이트, FIN·RST 순서를 비교한다. 세그먼트 분할은 비교하지 않는다. 다음 차이는 같은 결과로 본다. Viewer가 보내는 250바이트 메시지의 NUL 뒤 바이트, 보안 유형 선택 뒤 Server가 보내는 16바이트(회차마다 다름), 화면 갱신 데이터의 크기, TLS ClientHello의 크기, 세그먼트 분할, 양쪽 FIN의 순서 경쟁, Repeater가 FIN·RST로 닫은 뒤 상대의 FIN이 캡처에 남았는지 여부.
- TCP keepalive 탐침: tshark가 `tcp.analysis.keep_alive`로 표시한 1바이트 세그먼트는 스트림 데이터에서 뺐다. `summary.md`에는 플래그 `KA`로 남긴다. winvnc 연결에서 약 10초마다, Edge 연결에서 약 45초마다 보였다. 이 탐침은 이미 확인된 바이트를 다시 보내는 것이며 상대에게 전달되지 않는다.
- 확인 실행: 계획 p1~p5(형식 파악과 하네스 검증)와 p14(WebSocket, 하네스 결함 두 개, 3.4절)는 증거가 아니다. 증거 회차는 p10~p13, p15~p18이다.

#### 하네스 파일

스크립트는 저장소 밖 `D:\vr-lab\scripts\`에, 계획은 `D:\vr-lab\plans\`에, 실행 요약은 `D:\vr-lab\logs\05-<계획>.json`에 있다. 관리자 권한이 필요한 실행은 사용자가 UAC를 승인했다(Q66).

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
| M1-00 Mode I 꺼짐(기본 설정) | Viewer 하나를 Repeater Viewer 포트로 접속시켰다. 명령줄 원문은 남지 않았다. 계획 파일은 뒤에 M1-00을 뺀 판으로 바뀌었고, 첫 실행의 요약 `05-p10-mode1-part1.json`과 회차 파일에도 명령줄이 없다. [INFERENCE: 와이어의 메시지가 M1-01과 같으므로 M1-01과 같은 `-connect 192.0.2.30::5900 -proxy 192.0.2.20:5901`로 보인다] | 약 1.0초 뒤 Repeater가 `RFB 000.000\n`(12바이트)을 보낸다. Viewer는 같은 ms에 250바이트(`192.0.2.30:5900` + NUL + 정리되지 않은 바이트)를 보낸다. Repeater는 대상에 연결하지 않고, 더 보내지 않으며, 닫지도 않았다(40초 넘게 연결 유지). 웹 상태 페이지의 대기 목록에도 없다 |
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
| M2-06 ID 길이 | 원시 쌍, 양쪽 같은 값: `ID:1`, `ID:123456789`, `ID:1234567890` | 세 쌍 모두 매칭된다. `ID:1234567890` 쌍이 매칭된 동안 상태 수집이 웹 상태 페이지를 요청하자 응답이 없었고, Repeater 프로세스가 끝났으며, 두 연결이 RST로 끊겼다. 그 뒤의 네 쌍(20자리, 빈 ID, 접두어 없음, NUL 없는 247자리)은 TCP 연결이 시간 초과로 실패해 관찰되지 않았다. 그래서 M2-06b와 M2-11로 나눠 다시 관찰했다 |
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
- noVNC 연결 완료 판정: 시험용 중계기 로그의 RFB 바이트로 판정한다. 기준은 세 가지다. noVNC가 ServerInit(데스크톱 이름 포함)을 받는다. 이어서 SetPixelFormat·SetEncodings·FramebufferUpdateRequest를 보낸다. 화면 갱신 데이터를 받는다. 보조 근거로 원격 디버깅 대상 목록(`/json`)도 기록했다. p16의 이 기록은 대상 두 개(page와 Edge 내장 확장의 service_worker)의 필드가 공백으로 이어진 한 문자열이다. Windows PowerShell 5.1의 `ConvertFrom-Json`이 배열을 한 객체로 넘기기 때문이며, 호스트에서 같은 모양을 재현했다. 첫 대상(type `page`, url `…/vnc.html?…`)의 제목은 연결된 회차에서 `vr-server ( 192.0.2.30 ) - application mode - noVNC`, 연결되지 않은 회차에서 `noVNC`였다. p18부터는 대상을 하나씩 기록한다.

| 시나리오 | 입력 | 결과 |
| --- | --- | --- |
| W-01 noVNC → Repeater 직접(`ws://`) | `vnc.html?autoconnect=true&reconnect=false&repeaterID=1234&path=ws%3A%2F%2F192.0.2.20%3A5901%2F` | noVNC가 HTTP 업그레이드 요청(`GET / HTTP/1.1`, 510바이트)을 보낸다. Repeater는 이를 해석하지 않고, 접속 약 1.0초 뒤 `RFB 000.000\n`을 보낸다. noVNC는 0~42ms 안에 FIN으로 닫는다. Repeater는 닫지 않았다. 페이지 제목은 `noVNC`로 남았다. 같은 회차에서 p16의 `wss://` 시도도 평문 요청이 되어 결과가 같았다(아래 p14 설명) |
| W-02 noVNC → WS 중계기 → Repeater | `path=ws%3A%2F%2F192.0.2.20%3A8081%2Fwebsockify` | 요청 헤더에 `Sec-WebSocket-Protocol`이 없다. 중계기는 서브프로토콜 없이 수락했다. noVNC는 먼저 보내지 않는다. 중계기가 TCP 연결 약 1.0초 뒤 `RFB 000.000\n`을 바이너리 메시지 하나로 전달하자, noVNC는 67~85ms 뒤 바이너리 메시지 하나로 250바이트(`ID:1234` + NUL 243개)를 보냈다. 약 1.0초 뒤 winvnc와 매칭되어 Server의 `RFB 003.008\n`이 전달됐다. noVNC는 보안 유형 목록 `03 11 75 01`에서 `01`(None)을 골랐고, ServerInit(이름 `vr-server ( 192.0.2.30 ) - application mode`)을 받은 뒤 SetPixelFormat·SetEncodings·FramebufferUpdateRequest를 보내고 화면 갱신 데이터를 받았다. 페이지 제목도 데스크톱 이름으로 바뀌었다. Edge 종료 뒤 Repeater가 Server에 FIN을 보냈다 |
| W-01b noVNC → Repeater 직접(WSS) | `vnc.html?autoconnect=true&reconnect=false&repeaterID=1234&host=192.0.2.20&port=5901&encrypt=true&path=websockify` | 각 회차에서 Viewer→Repeater 5901 TCP 연결이 두 개 관찰됐다. 두 연결 모두 같은 순서였다. 브라우저가 TLS ClientHello(1729~1793바이트)를 보낸다. Repeater는 접속 약 1.0초 뒤 평문 `RFB 000.000\n`을 보낸다. 브라우저는 2~18ms 안에 TLS 경고 레코드 7바이트(`15 03 01 00 02 02 46`, fatal `protocol_version`)를 보내고 FIN으로 닫는다. Repeater는 캡처가 끝날 때까지(약 70초) 이 연결을 닫지 않았다. 페이지 제목은 `noVNC`로 남았다. 연결이 두 개인 이유는 확인하지 않았다 |
| W-03b noVNC → WSS 중계기 → Repeater | `host=192.0.2.20&port=8443&encrypt=true&path=websockify` | TLS 접속이 성립했다(중계기 로그 `scheme=https`). 요청 헤더에 `Sec-WebSocket-Protocol`이 없다. 이후 순서는 W-02와 같다. 중계기가 `RFB 000.000\n`을 전달하자 noVNC는 84~110ms 뒤 바이너리 메시지 하나로 250바이트(`ID:1234` + NUL 243개)를 보냈다. 매칭 뒤 보안 유형 `01`(None)을 고르고, ServerInit을 받은 뒤 SetPixelFormat·SetEncodings·FramebufferUpdateRequest를 보내고 화면 갱신 데이터를 받았다. 페이지 제목은 `vr-server ( 192.0.2.30 ) - application mode - noVNC`로 바뀌었다. Edge 종료 때 TCP RST가 났고, 약 21~31ms 뒤 Repeater가 Server에 FIN을 보냈다 |

확인 실행 p14(증거 아님)에서 하네스 결함 두 개를 찾아 고친 뒤 p16을 실행했다.

- 시험용 중계기가 서브프로토콜 없는 요청을 수락하지 못했다. PowerShell이 .NET 문자열 인수의 `$null`을 빈 문자열로 바꾸기 때문이다. `[NullString]::Value`로 고쳤고, 서브프로토콜을 제안하는 경우와 제안하지 않는 경우를 호스트에서 시험했다.
- 페이지가 `http:`일 때, noVNC 1.7.0은 절대 URL `path`의 `wss` scheme을 쓰지 않고 평문으로 접속했다. `encrypt=true`를 더해도 같았다(p16의 W-01 두 번째 접속과 W-03). 호스트에서 블랙박스로 시험한 결과, `host`·`port`·`encrypt=true`·`path`를 따로 주면 TLS로 접속했다. 이 방식은 noVNC `docs/EMBEDDING.md`에 있는 매개변수다(지원 중단 예정으로 표기). WSS 시나리오는 이 방식으로 p18에서 다시 관찰했다.

### 3.5 계약 1절 항목별 해석

이 절은 [초기 교환 계약](repeater-handshake-v1.md) 1.1절의 "관찰로 확정할 항목"과 1.2절의 논리 필드에 대한 결론이다. 대상은 2절의 고정 버전(Viewer·Server 1.8.3.0, Repeater 1.6.0.0, noVNC 1.7.0)이다. 근거 열은 3.4절의 시나리오 ID다. 결론은 관찰한 바이트와 연결 결과만 쓴다. 내부 동작을 추정한 부분에는 `[INFERENCE]`를 붙였다. 이 결론은 계약 1절에 반영해 2026-11-01에 채택했다(TODO 1.5). 계약의 절 번호는 채택 때 바뀌었고, 이 절의 "1.1절"·"1.2절"은 채택 전 초안의 절 번호다.

#### 메시지별 결론

| 계약 1.1절 메시지 | 결론 | 근거 |
| --- | --- | --- |
| A. Mode II Server 등록 요청 | Server가 TCP 연결 직후 먼저 보낸다. 길이는 250바이트 고정이다. 내용은 `ID:` + 10진 ID 문자열 + NUL + 채움 바이트다. `-id:`의 채움은 모두 NUL이고, `-autoreconnect ID:`의 채움에는 0이 아닌 바이트가 있다. Repeater는 매칭 전까지 Server에 아무 바이트도 보내지 않는다. Repeater가 만들어 보내는 응답 바이트는 없다. 매칭 뒤 Server가 받는 바이트는 Viewer가 보낸 바이트다(E행) | M2-01, M2-02, T-03 |
| B. Mode II Viewer 요청 | Repeater가 TCP 연결 약 1.0초 뒤 `RFB 000.000\n`(12바이트)을 먼저 보낸다. Viewer는 이것을 받은 뒤 250바이트를 보낸다. 내용 형식은 A와 같다. Viewer의 채움 바이트는 정리되지 않은 값이다. A와 B의 차이는 접속 포트와 Repeater의 첫 12바이트뿐이다 | M2-01, M2-02, M2-05 |
| C. Mode I Viewer 요청 | 방향은 B와 같다(`RFB 000.000\n` 다음에 Viewer가 보냄). 길이는 250바이트, 내용은 `<대상>:<포트>` + NUL + 채움이다. `ID:` 같은 접두어는 없다. Viewer는 명령줄의 `host::port`를 `host:port`로 바꾸고, 이름을 대문자로 바꾸며, 포트를 생략하면 `:5900`을 붙인다. Repeater는 포트 없는 입력에 5900을 쓰고, 이름은 Repeater 쪽에서 푼다. IPv6 대상은 세 가지 표기 모두 즉시 닫혔다. Repeater는 250바이트를 대상에 전달하지 않는다. Viewer 메시지 뒤에 Repeater가 만들어 보내는 응답 바이트는 없다. 대상 연결 뒤 Viewer가 받는 바이트는 대상 Server가 보낸 바이트다(E행) | M1-01~M1-06 |
| D. 응답·오류 표시 | 오류를 알리는 바이트는 한 번도 없었다. Viewer 포트의 연결은 입력 내용과 관계없이 접속 약 1.0초 뒤 `RFB 000.000\n`(B행)을 받았다. 오류 경우(M1-04, M2-07, E-01, E-04)와 HTTP·TLS 입력(W-01, W-01b)도 같다. 이 12바이트는 어떤 입력에도 같으므로 결과를 알리지 않는다. 이 12바이트 말고 Repeater가 만들어 보낸 바이트는 없었다. 매칭이나 대상 연결 뒤에 오는 바이트는 상대가 보낸 바이트를 중계한 것이다(E행). 오류 결과는 연결 처리로만 드러났다. 연결 처리는 세 가지다. 빠른 FIN(메시지 도착 직후, Server 쪽 ID 거절은 접속 약 1.0초 뒤): 금지 대상, IPv6 대상, 숫자로 시작하지 않는 ID와 빈 ID, Server ID `999999999`. 지연 FIN: 초기 교환 시간 초과, Mode I 대상 연결 실패(약 21초). 닫지 않고 유지: 형식 오류, Mode I 꺼짐, 같은 역할 중복, 짝이 없는 대기(60초 넘게 관찰, 대기 시간 상한은 보이지 않음) | M1-00, M1-04~M1-06, M2-03~M2-05, M2-06b, M2-07, M2-09, M2-12, M2-13, E-01~E-04, W-01, W-01b |
| E. RFB 스트림 시작과 후속 바이트 | 초기 교환의 경계는 250바이트 직후다. 250바이트를 넘게 보낸 바이트(같은 쓰기에 붙은 바이트 포함)는 매칭 순간 상대에게 첫 바이트로 전달된다. 매칭 전에 도착한 바이트는 도착 순서대로 매칭 순간 전달된다. 한쪽이 22초 넘게 기다린 뒤에도 같다. Mode II에서 Viewer가 처음 받는 RFB 바이트는 Server의 `RFB 003.008\n`이다. Mode I에서는 대상 Server의 `RFB 003.008\n`이다 | M1-01, M2-01, S-02~S-05, E-03 |
| F. noVNC 요청 | noVNC 1.7.0은 `repeaterID`를 B와 같은 250바이트(`ID:<repeaterID>` + NUL 채움)로 보낸다. 먼저 보내지 않고, `RFB 000.000\n`을 받은 뒤 67~110ms 안에 바이너리 메시지 하나로 보낸다. WS와 WSS 모두 같다. 두 경우 모두 요청에 `Sec-WebSocket-Protocol` 헤더가 없다. noVNC가 공식 Repeater의 Viewer 포트에 직접 접속하면 WS·WSS 모두 연결되지 않았다. Repeater는 HTTP 업그레이드 요청과 TLS ClientHello에 응답하지 않고 `RFB 000.000\n`만 보냈다. 페이지가 `http:`일 때 WSS로 접속하려면 `path`에 절대 URL을 쓰지 않고 `host`·`port`·`encrypt=true`·`path`를 따로 줘야 했다 | W-01, W-01b, W-02, W-03b |

#### 논리 필드와 규칙

| 계약 1.2절 필드·규칙 | 결론 | 근거 |
| --- | --- | --- |
| 중계 모드 | 모드를 밝히는 별도 필드는 없다. Viewer 포트로 온 메시지가 `ID:`로 시작하면 Mode II로 처리됐다. `ID:`로 시작하지 않는 메시지는 Mode I이 켜져 있으면 대상 주소로 쓰였고, 꺼져 있으면 연결을 유지한 채 아무 일도 일어나지 않았다 | M1-00, M1-01, E-01, M2-06b |
| 역할 | 메시지에 역할 표시는 없다. 접속한 포트(Server 5500, Viewer 5901)로 구분된다. Server 포트로 온 메시지가 `ID:`로 시작하지 않으면(`XX:1234`, `1234`) 매칭도 거절도 없이 유지된다 | M2-01, E-01, M2-06b |
| 길이·패딩·종료 문자 | 길이는 정확히 250바이트다. 나눠 보내도 되고, 250바이트가 다 오기 전에는 매칭하지 않는다. 첫 NUL이 값의 끝이다. NUL 뒤의 채움 내용은 결과에 영향이 없었다. NUL 없이 250바이트를 숫자로 채워도 매칭된다. 버전 문자열은 Viewer 쪽 `RFB 000.000\n` 하나다 | S-01, E-03, E-04, M2-01, M2-06b |
| 초기 교환 시간 | 아무것도 보내지 않으면 접속 약 6.0초 뒤 FIN이 온다. 100바이트만 보내면 Server 쪽은 접속 약 6.0초 뒤, Viewer 쪽은 마지막 수신 약 5.0초 뒤 FIN이 온다. 메시지 도착 전 클라이언트가 닫으면 1~2ms 안에 Repeater도 FIN을 보낸다 | E-02, E-04 |
| 중계 ID 비교 | 문자열 비교가 아니다(`0123`과 `123`이 매칭됨). 관찰한 쌍은 다음과 같다. 앞자리 0만 다른 쌍은 매칭됐다(`0123`/`123`, `789`/`0789`). 숫자 뒤에 다른 문자가 붙은 값은 앞 숫자와 같게 처리된 사례가 있다(`56-78`/`56` 매칭, `12-34`·`12 34` 쌍은 `5912`로 표시). `12ab`/`12AB`도 매칭됐지만, 이 쌍만으로는 대소문자를 무시한 것인지 뒤 문자를 무시한 것인지 구분되지 않는다. 관찰한 100 미만 값은 5900을 더한 값과 매칭됐다(`12`/`5912`, `0`/`5900`, `99`/`5999`). `100`/`6000`은 매칭되지 않았다. 영문자로 시작하는 ID와 빈 ID는 거절됐다(`ABCD`, `abcd`, `ab12`, `AB12`, `cd34`, 빈 ID). 같은 값끼리도 거절되므로 영문 ID의 대소문자 비교는 시험되지 않았다. 서로 다른 20자리 값 한 쌍(`12345678901234567890`/`98765432109876543210`)이 매칭됐고, `2147483647`/`2147483648`은 매칭되지 않았다. Server ID `999999999`는 거절되지만 `999999998`과 `99999999`는 매칭된다. [INFERENCE: ID의 앞부분 숫자를 수로 바꿔 비교하는 것처럼 보인다.] 변환 규칙, 큰 값의 처리, `999999999` 거절의 이유는 확정하지 못했다(4.13) | M2-06, M2-06b, M2-07~M2-09, M2-12~M2-14 |
| 중계 ID 길이 | 1자리부터 247자리까지 받아들였다. 공개 문서 예시의 9자리 제한은 보이지 않았다. 다만 서로 다른 20자리 값 한 쌍이 매칭됐다(위 행) | M2-06, M2-06b, M2-13 |
| 대상 주소·포트 | C행과 같다. 관찰한 형식은 `<IPv4 또는 이름>:<포트>`이고 포트는 생략할 수 있었다(기본 5900). IPv6는 원시 입력 세 가지(`2001:db8:1::30:5900`, `[2001:db8:1::30]:5900`, `vnc-server6.test:5900`)에서 Repeater가 대상 연결을 시도하지 않고 FIN으로 닫았다. Viewer는 IPv6 `-connect`로 Repeater 연결을 만들지 못했다. 두 결과의 원인은 확인하지 않았다 | M1-01~M1-03, M1-06 |
| 같은 역할 중복 ID(RLY-TBD-003) | 먼저 등록한 연결이 유지되고 매칭된다. 뒤에 온 같은 역할 연결은 등록되지 않고, 닫히지도 않으며, 보낸 바이트는 전달되지 않는다. Server와 Viewer 모두 같다. 계약의 제안 정책(후발 거절)과 기존 유지라는 점은 같고, 후발 연결을 닫지 않는 점이 다르다 | M2-04, M2-05 |
| 매칭과 종료 | 짝이 되는 메시지가 도착한 뒤 약 1.0~2.0초에 매칭된다. 매칭된 한쪽이 닫히면 약 1ms 안에 상대에게 FIN이 간다. FIN 약 3ms 뒤 RST가 뒤따른 경우도 있었다. Server의 `-autoreconnect`는 끊긴 뒤 약 2초에 다시 등록하고, 새 Viewer와 매칭된다 | M2-01, M2-02, T-01~T-03, M1-02 |

#### 형식과 무관한 관찰 사실

- Repeater 1.6.0.0은 10자리 ID 세션이 활성인 동안 웹 상태 페이지를 요청하면 응답하지 않고 프로세스가 끝났다. 세션의 두 연결은 RST로 끊겼다(M2-11, 2/2). `vnc-relay`의 동작 기준이 아니며, 관찰 하네스가 상태 요청 시점을 조절한 근거다.
- 웹 상태 페이지는 매칭 상태를 항상 정확히 보여주지 않았다. `ID:12`/`ID:5912` 쌍은 매칭 중에도 대기 Viewer로만 표시됐다(M2-14). 그래서 매칭 판정은 상태 페이지가 아니라 표식 전달로 했다.

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

- 시각은 호스트 시각(2026-11-01)이다. 증거 76회차는 모두 첫 시도에서 유효했고, 도중에 꺼진 VM은 없었다.
- 2026-11-01 기록 시점에 76개 `capture.pcapng`의 SHA-256을 다시 계산했다. 모두 `run.json`의 값과 같았다.
- M1-00의 두 회차는 p10의 첫 실행(10:42~10:54)에서 얻었다. 이 실행은 M1-01 run1의 두 시도가 무효(VM은 모두 실행 중, 캡처 파일 없음)가 되어 멈췄다. 이 두 시도는 증거가 아니다. 첫 실행의 요약은 `05-p10-mode1-part1.json`에 있다. 그 뒤 M1-00을 뺀 계획으로 p10을 다시 실행했고(11:04~12:03, `05-p10-mode1.json`), M1-01 회차는 같은 위치에 새로 기록됐다.
- p16의 W-03과 W-01 두 번째 접속은 평문 접속이었다(3.4절). WSS 증거는 p18의 W-01b·W-03b다.

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
| p16-websocket | W-03-novnc-wss-bridge | 1 | 15:48:12 | 15:55:05 | `2a8856f7a94940c5e9d8fd7f9314c27f5e9c8aa67e9073f142ed4c1a9d917330` |
| p16-websocket | W-03-novnc-wss-bridge | 2 | 15:55:05 | 16:01:43 | `547a44b6e65ba4e1dd39dacbcd7dfe32ad2c79ce76e721386c5436a086f9ea68` |
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

## 4. 미해결 항목

| 번호 | 항목 | 영향 | 필요한 결정·작업 |
| --- | --- | --- | --- |
| 4.1 | Repeater Ubuntu 24.04 빌드 버전 미확인 | 고정 범위 밖. RLY-TBD-001 해소에 영향 없음 | 없음. 사용자 결정 2026-10-22(Q19): Ubuntu `uvncrepeater` 관찰을 미루고 Windows Repeater만 고정한다. Ubuntu 쪽 검증은 2.6절 |
| 4.2 | Repeater Windows 표기 1.4.0.0과 받은 파일 1.6.0.0 불일치 | 해소. 사용자 결정 2026-10-22(Q20): 받은 파일 1.6.0.0을 고정 대상으로 채택. 표기 차이의 원인은 확인하지 않음 | 없음 |
| 4.3 | 설치 뒤 `vncviewer.exe` 파일 버전 미확인 | 해소(2026-10-27). `vncviewer.exe` 1.8.3.0과 Viewer 구성 요소 설치를 확인했다(3.1절) | 없음 |
| 4.4 | noVNC 기본 `Sec-WebSocket-Protocol` 값 미관찰 | 해소(2026-11-01). noVNC 1.7.0은 WS와 WSS 모두 이 헤더를 보내지 않았다(3.4절 W-02·W-03b) | 없음. TODO 1.7에서 [WebSocket 중계 계약](websocket-relay-v1.md) 1.2절·6절에 반영했다(2026-11-01) |
| 4.5 | 저작권자·적용 연도 미정 | OPS-TBD-005, LICENSE 고지 | 보류 중인 결정 Q10 |
| 4.6 | 관찰 증거 전부 미수집 | 해소(2026-11-01). 76회차의 증거와 해석을 3절에 기록했다. RLY-TBD-002·003과 WS-025의 입력이다 | 없음. TODO 1.5(계약 1절 반영과 채택), 1.6(중복 ID 정책), 1.7(다른 계약 반영)을 2026-11-01에 모두 마쳤다 |
| 4.7 | 설치 파일의 `repeater.exe`가 고정 `repeater_v160.zip`의 파일과 다름(둘 다 1.6.0.0) | 잘못된 파일로 관찰하면 증거가 고정 대상과 맞지 않는다 | 없음. Q20에 따라 고정 파일을 쓰고, 관찰 기록에 실행 파일 경로와 SHA-256을 적는다(3.1절) |
| 4.8 | 6.1절 관찰 환경 미준비 | 해소(2026-11-01). 3.2절 환경을 구성하고 스모크 확인과 `baseline` 체크포인트를 마쳤다 | 없음. 시나리오 관찰은 4.6(TODO 1.4)에서 한다 |
| 4.9 | 게스트 Windows 평가판 기간과 정품 인증 | 해소(2026-11-01). 세 VM 모두 알림 모드(`0xC004F009`, 유예 기간 만료)임을 확인했다(3.3절). 사용자 결정 2026-11-01(Q67)에 따라 회차마다 `baseline`에서 새로 부팅하고 한 시간 안에 끝냈다. 도중에 꺼진 VM은 없었다. 이 라이선스 상태가 약관에 맞는지는 판단하지 않았다 | 없음 |
| 4.10 | 대상 Repeater의 ID 비교가 스펙의 비정규화 규칙과 다름 | 해소(2026-11-01). Repeater 1.6.0.0에서는 서로 다른 ID 문자열이 매칭됐다. `0123`과 `123`, `12`와 `5912`, `12ab`와 `12AB`, 서로 다른 20자리 값 한 쌍이 그 예다(3.5절). `vnc-relay`는 숫자로만 된 ID를 받고 바이트열이 정확히 같을 때만 같은 ID로 본다(사용자 결정 2026-11-01(Q68)). 그래서 숫자로만 된 세 쌍(`0123`·`123`, `12`·`5912`, 20자리 한 쌍)은 서로 다른 ID이고, `12ab`·`12AB`는 `handshake_invalid`다. 차이는 [계약 3.1절](repeater-handshake-v1.md#31-대상-버전-repeater와-다르게-처리하는-입력)에 있다 | 없음 |
| 4.11 | 같은 역할 중복 ID 결과가 제안 정책과 일부 다름(RLY-TBD-003) | 해소(2026-11-01). 대기 중 ID에 같은 역할이 오면 Repeater는 기존 연결을 유지하고 후발 연결을 등록하지 않으며 닫지도 않는다(3.5절). `vnc-relay`는 후발 연결을 바로 닫아 거절한다(사용자 결정 2026-11-01(Q74)). 이 차이는 [계약 3.1절](repeater-handshake-v1.md#31-대상-버전-repeater와-다르게-처리하는-입력)에 있다. 활성 세션이 점유한 ID에 같은 역할이 오는 경우는 관찰하지 않았고, 추가 관찰도 하지 않기로 했다(Q75) | 없음 |
| 4.12 | Repeater 1.6.0.0이 10자리 ID 세션 중 웹 상태 요청에 종료됨 | 관찰 하네스에만 영향이 있다. M2-06의 뒷부분 사례가 관찰되지 않아 M2-06b·M2-11로 다시 관찰했다. `vnc-relay` 동작 기준이 아니다 | 없음 |
| 4.13 | 큰 ID 값과 `999999999`의 처리 이유 미확정 | 해소(2026-11-01). 서로 다른 20자리 값 한 쌍이 매칭됐다. Server ID `999999999`는 거절되고, 이웃 값 `999999998`과 `99999999`는 매칭된다. 내부 변환 규칙은 블랙박스 관찰로 확정하지 못했다(3.5절). `vnc-relay`는 1~247자리 숫자를 값 크기와 관계없이 바이트열로 비교하므로(사용자 결정 2026-11-01(Q68·Q69)) 그 규칙을 확정할 필요가 없다. `999999999`도 받는다 | 없음 |

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

### 5.3 2026-11-01 시나리오 관찰 (메인 에이전트가 실행)

관리자 권한이 필요한 실행은 사용자가 UAC를 승인했다(Q66). p18은 승인 요청이 두 번 연속 승인되지 않아(시간 초과 또는 취소) 사용자에게 확인한 뒤 세 번째 요청에서 실행했다. 계획 파일은 `D:\vr-lab\plans\`, 실행 요약은 `D:\vr-lab\logs\05-<계획>.json`이다.

```
호스트 응용 프로그램(전사 로그 D:\vr-lab\logs\05-<계획>.log의 원문, 관리자 권한 PowerShell 5.1):
C:\WINDOWS\System32\WindowsPowerShell\v1.0\powershell.exe -NoProfile -ExecutionPolicy Bypass -File D:\vr-lab\scripts\05-observe.ps1 -Plan D:\vr-lab\plans\p10-mode1.json
C:\WINDOWS\System32\WindowsPowerShell\v1.0\powershell.exe -NoProfile -ExecutionPolicy Bypass -File D:\vr-lab\scripts\06-observe-many.ps1 -Plans D:\vr-lab\plans\p10-mode1.json,D:\vr-lab\plans\p11-mode2.json,D:\vr-lab\plans\p12-errors-split.json,D:\vr-lab\plans\p13-termination.json
C:\WINDOWS\System32\WindowsPowerShell\v1.0\powershell.exe -NoProfile -ExecutionPolicy Bypass -File D:\vr-lab\scripts\06-observe-many.ps1 -Plans D:\vr-lab\plans\p14-websocket.json,D:\vr-lab\plans\p15-followup.json
C:\WINDOWS\System32\WindowsPowerShell\v1.0\powershell.exe -NoProfile -ExecutionPolicy Bypass -File D:\vr-lab\scripts\06-observe-many.ps1 -Plans D:\vr-lab\plans\p16-websocket.json
C:\WINDOWS\System32\WindowsPowerShell\v1.0\powershell.exe -NoProfile -ExecutionPolicy Bypass -File D:\vr-lab\scripts\06-observe-many.ps1 -Plans D:\vr-lab\plans\p17-followup2.json
C:\WINDOWS\System32\WindowsPowerShell\v1.0\powershell.exe -NoProfile -ExecutionPolicy Bypass -File D:\vr-lab\scripts\06-observe-many.ps1 -Plans D:\vr-lab\plans\p18-websocket-wss.json

05-p10-mode1-part1.json (첫 실행): 2026-11-01T10:42:27 ~ 10:54:31, M1-00 runs=2 valid=2, M1-01 run1 두 시도 무효(캡처 없음)로 중단
p10-mode1: 2026-11-01T11:04:27 ~ 2026-11-01T12:03:54 scenarios=6 runs=12 valid=12 attempt>1=0
p11-mode2: 2026-11-01T12:03:54 ~ 2026-11-01T13:11:06 scenarios=9 runs=18 valid=18 attempt>1=0
p12-errors-split: 2026-11-01T13:11:06 ~ 2026-11-01T13:39:47 scenarios=7 runs=14 valid=14 attempt>1=0
p13-termination: 2026-11-01T13:39:48 ~ 2026-11-01T14:01:04 scenarios=3 runs=6 valid=6 attempt>1=0
p15-followup: 2026-11-01T14:45:13 ~ 2026-11-01T15:15:14 scenarios=4 runs=8 valid=8 attempt>1=0
p16-websocket: 2026-11-01T15:16:24 ~ 2026-11-01T16:01:45 scenarios=3 runs=6 valid=6 attempt>1=0
p17-followup2: 2026-11-01T16:02:11 ~ 2026-11-01T16:29:31 scenarios=3 runs=6 valid=6 attempt>1=0
p18-websocket-wss: 2026-11-01T16:47:37 ~ 2026-11-01T17:13:02 scenarios=2 runs=4 valid=4 attempt>1=0

capture.pcapng SHA-256 재계산과 run.json 비교: 76/76 일치

python D:\vr-lab\scripts\summarize.py D:\vr-captures\<계획>   → <시나리오>\summary.md
STRUCTURE DIFFERS: M2-06, M2-07, M2-08, E-03, M2-12, M2-13, M2-14 (양쪽 FIN 순서)
                   M1-02, T-03 (run2에만 Repeater FIN·RST 뒤 Server FIN)
                   W-02, W-03b (화면 갱신 데이터 크기), W-01b (TLS ClientHello 크기)
                   S-05 (run1 RST, run2 FIN: 하네스 재시작과 겹친 것으로 추정, 3.4절)

tshark -r D:\vr-captures\p11-mode2\M2-04-duplicate-server-id\run1\capture.pcapng -Y "tcp.len==1" -T fields -e frame.time_relative -e ip.src -e tcp.srcport -e tcp.dstport -e tcp.payload -e tcp.analysis.keep_alive
(출력 발췌)
10.044379900	192.0.2.30	49674	5500	00	1
14.071571600	192.0.2.10	49673	5901	11
24.314159400	192.0.2.30	49674	5500	00	1
(S-04 run1의 원시 클라이언트 `00`: 2.682744600 192.0.2.30 49673 5500 00 — keep_alive 표시 없음)
```

- 시나리오 관찰: 통과. 6.1절 5단계의 여섯 묶음(Mode I, Mode II, 오류, 분할·후속 바이트, 종료·재접속, 외부 WebSocket)을 각각 2회 이상 캡처했다. 증거 76회차는 모두 첫 시도에서 유효했다. 증거가 아닌 실패 시도는 p10 첫 실행의 M1-01 run1 두 시도다.
- 회차 비교: 통과. 구조 차이가 있는 13개 시나리오 중 12개는 3.3절에 적은 무시할 차이(FIN 순서 경쟁, Repeater 종료 뒤 상대 FIN의 유무, 화면 갱신 데이터·TLS ClientHello 크기)다. S-05의 종료 방식 차이는 원인을 추정으로만 남겼다(3.4절). 초기 교환과 매칭 결과에는 회차 간 차이가 없었다.
- 원본 무결성: 통과. 캡처 76개의 해시가 실행 요약의 값과 같다(3.6절).
- keepalive 구분: 통과. winvnc의 10초 간격 1바이트는 tshark가 TCP keepalive로 표시했고, 원시 클라이언트가 보낸 `00`은 표시되지 않았다.
