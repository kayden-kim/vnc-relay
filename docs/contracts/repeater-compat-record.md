# 호환성 검증 기록

이 기록은 UltraVNC Server·Viewer·Repeater와 noVNC의 호환성 검증 결과를 남긴다. 관찰 허용 검토(1절), 검증 대상 고정(2절), 관찰 증거(3절)를 담는다. 증거 항목은 [Repeater 초기 교환 계약 6.2절](repeater-handshake-v1.md#62-기록할-증거)을 따른다.

- 상태: 진행 중. 관찰 허용 결정과 검증 대상 고정 일부를 기록했다. 관찰은 아직 하지 않았다.
- 작성일: 2026-10-21
- 기준 커밋: `cfc703a`(작업 시작 시점 HEAD). 이 기록은 `docs(compat)` 커밋에 담는다.
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
- 받은 날짜: 2026-10-21.
- OS: Windows 11 x64. 파일 버전 정보는 1.8.3.0, 제품명 `UltraVNC`, 회사명 `uvnc`.
- 미확인: 설치 파일 하나에 Server와 Viewer가 함께 들어 있는지 확인하지 않았다. 설치 뒤 확인한다(4절 4.3).

### 2.2 UltraVNC Repeater (Windows x64) — 고정, 표기 불일치

- 버전: 1.6.0.0(파일 버전 정보).
- 배포본: `repeater_v160.zip`, 224,709바이트. 압축 안에는 `repeater.exe` 한 개가 있다.
- SHA-256: `82d9e7f06266a291b4988e36f4d33e127d63adb93269f3b24c432718f35de1de`.
- 받은 주소: https://uvnc.eu/download/repeater/repeater_v160.zip. [공식 Repeater 페이지](https://uvnc.com/downloads/ultravnc-repeater/83-ultravnc-repeater.html)의 항목 "ultravnc_repeater_1400"(표기 1.4.0.0)의 다운로드 링크가 이 주소로 넘어간다.
- 받은 날짜: 2026-10-21.
- OS: Windows 11 x64 대상. 실행 검증은 1.4에서 한다.
- 불일치: 같은 항목은 1.4.0.0, 72.05 KB로 표기한다. 받은 파일은 1.6.0.0, 224,709바이트다. 원인은 확인하지 않았다. 고정은 받은 파일 기준으로 한다. 채택 여부는 4절 4.2에 적는다.

### 2.3 UltraVNC Repeater (Ubuntu 24.04 LTS x64) — 미고정

- 배포본: `uvncrepeater.tar.gz`, 57,888바이트. C++ 소스 패키지다.
- SHA-256: `a0fe948ba477685d4190223b35a0691b2e657751d30b97a9838bbfa31ca6ed8f`.
- 받은 주소: https://uvnc.eu/download/repeater/uvncrepeater.tar.gz. 공식 Repeater 페이지의 unix 링크다. 어느 버전 항목에 속하는지 페이지에 적혀 있지 않다.
- 받은 날짜: 2026-10-21.
- 미고정 이유: 파일 이름과 폴더 이름에 버전이 없다. 공식 페이지에서도 Ubuntu 빌드 버전을 찾지 못했다. 압축 안 파일은 소스 코드이므로 내용을 읽지 않았다. 파일 이름 목록만 확인했다.
- 영향: RLY-TBD-001이 아직 해소되지 않았다. 1.4의 선행 1.3도 끝나지 않았다.

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

## 3. 관찰 증거 (TODO 1.4)

관찰은 아직 하지 않았다. 6.2절의 증거 항목은 모두 미실행이다.

| 증거 | 상태 | 이유 |
| --- | --- | --- |
| 대상·환경·입력·원본·결과·해석 | 미실행 | 1.4를 시작하지 않았다. 선행 1.3의 Ubuntu Repeater 버전이 미확정이다 |
| 승인 | 1절 결정(2026-10-21) | 6.1절 1단계 검토 결과 |

## 4. 미해결 항목

| 번호 | 항목 | 영향 | 필요한 결정·작업 |
| --- | --- | --- | --- |
| 4.1 | Repeater Ubuntu 24.04 빌드 버전 미확인 | RLY-TBD-001 미해소. 1.4 선행 미충족 | (a) 공식 자료에서 버전을 확인해 고정한다. (b) 확인 전까지 Ubuntu Repeater 관찰을 미룬다 |
| 4.2 | Repeater Windows 표기 1.4.0.0과 받은 파일 1.6.0.0 불일치 | 고정 대상 버전의 근거 | (a) 받은 파일 1.6.0.0을 고정 대상으로 채택한다. (b) 공식에 확인한 뒤 다시 고정한다 |
| 4.3 | 설치 파일에 Viewer가 들어 있는지 미확인 | Server·Viewer 고정 범위 | 설치 뒤 확인한다(1.4 전) |
| 4.4 | noVNC 기본 `Sec-WebSocket-Protocol` 값 미관찰 | WS-TBD-005 | 1.4에서 관찰한다 |
| 4.5 | 저작권자·적용 연도 미정 | OPS-TBD-005, LICENSE 고지 | 보류 중인 결정 Q10 |
| 4.6 | 관찰 증거 전부 미수집 | RLY-TBD-002·003, WS-025 | 1.4 실행 |

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
