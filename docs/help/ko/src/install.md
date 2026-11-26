# 설치

이 페이지로 OS에 맞는 배포 묶음을 받고 해시를 확인한 뒤 첫 실행에 필요한 파일을 준비할 수 있다.

## 요구 환경

지원 OS는 Windows 11 x64와 Ubuntu 24.04 LTS x64다. Windows Server와 다른 OS·아키텍처는 검증 범위가 아니다. 수동 실행과 Windows 서비스 등록을 다룬다. 컨테이너는 다루지 않는다.

배포 묶음을 실행할 때 Rust 빌드 도구는 필요하지 않다. 소스에서 직접 묶음을 만들 때만 빌드 환경을 준비한다.

## 배포 묶음 받기

1. [GitHub Releases](https://github.com/kayden-kim/vnc-relay/releases)에서 사용할 Release를 연다.
2. OS에 맞는 묶음을 내려받는다.
3. 같은 이름 뒤에 `.sha256`이 붙은 파일을 같은 디렉터리에 내려받는다.

| OS | 묶음 | 해시 파일 |
| --- | --- | --- |
| Windows 11 x64 | `vnc-relay-2026.11.11-windows-x64.zip` | `vnc-relay-2026.11.11-windows-x64.zip.sha256` |
| Ubuntu 24.04 LTS x64 | `vnc-relay-2026.11.11-linux-x64.tar.gz` | `vnc-relay-2026.11.11-linux-x64.tar.gz.sha256` |

아래 명령은 `v2026.11.11`을 예로 든다. 다른 버전을 받으면 모든 파일 이름의 버전을 함께 바꾼다.

## SHA-256 확인하기

압축을 풀기 전에 다운로드 디렉터리에서 실행한다.

### Windows PowerShell

1. 묶음의 SHA-256을 계산한다.

   ```powershell
   Get-FileHash .\vnc-relay-2026.11.11-windows-x64.zip -Algorithm SHA256
   ```

2. 해시 파일의 내용을 표시한다.

   ```powershell
   Get-Content .\vnc-relay-2026.11.11-windows-x64.zip.sha256
   ```

3. `Get-FileHash`의 `Hash`와 해시 파일의 첫 낱말을 비교한다. 영문 대소문자는 무시하고 같은 값인지 확인한다.

### Ubuntu bash

1. 해시 파일로 묶음을 확인한다.

   ```shell
   sha256sum -c vnc-relay-2026.11.11-linux-x64.tar.gz.sha256
   ```

2. 결과가 `OK`인지 확인한다.

> **주의:** 해시가 다르면 압축을 풀거나 실행하지 않는다. 묶음과 해시 파일을 같은 Release에서 다시 받는다.

## 압축 풀기

운영 파일을 둘 디렉터리를 정한다. 아래 명령은 다운로드 디렉터리에서 실행하며 현재 위치 아래에 묶음을 푼다. 다른 운영 위치를 쓰면 풀린 디렉터리 전체를 그 위치에 둔다.

### Windows PowerShell

1. 묶음을 푼다.

   ```powershell
   Expand-Archive -Path .\vnc-relay-2026.11.11-windows-x64.zip -DestinationPath .
   ```

2. 풀린 디렉터리로 이동한다.

   ```powershell
   Set-Location .\vnc-relay-2026.11.11-windows-x64
   ```

### Ubuntu bash

1. 묶음을 푼다.

   ```shell
   tar -xzf vnc-relay-2026.11.11-linux-x64.tar.gz
   ```

2. 풀린 디렉터리로 이동한다.

   ```shell
   cd vnc-relay-2026.11.11-linux-x64
   ```

이후 명령은 실행파일과 `relay.toml`이 있는 이 디렉터리에서 실행한다.

## 묶음 내용과 파일 위치

| 파일·디렉터리 | 용도 |
| --- | --- |
| `vnc-relay.exe` 또는 `vnc-relay` | 해당 OS의 실행파일 |
| `vnc-relay-tray.exe` | Windows 서비스 상태·시작·중지용 트레이 앱. Windows 묶음에만 있다 |
| `relay.toml` | 저장소 `examples/relay.toml`의 루프백 설정 |
| `README.md`, `README.ko.md` | 제품 소개와 빠른 시작. 영어판과 한국어판 |
| `docs/`, `GLOSSARY.md` | 오프라인으로 읽을 수 있는 운영 안내·기준 문서·용어집 |
| `LICENSE` | 프로젝트의 MIT 라이선스 |
| `THIRD-PARTY-NOTICES.txt` | 실행파일에 포함된 제삼자 구성요소의 고지 |

계정 파일·개인키·비밀번호·운영 로그는 묶음에 없다. 예시 설정에서는 관리자 계정을 만들면 `admin-accounts.toml`이 생긴다. VNC Relay를 실행하면 `relay.log`에 운영 로그를 쓴다.

설정 파일 안의 상대 경로는 터미널 위치가 아니라 `relay.toml`의 디렉터리를 기준으로 푼다. 예시의 계정 파일과 로그는 이 디렉터리에 둔다. TLS를 켜면 인증서와 개인키를 별도로 준비하고 설정에서 그 경로를 지정해야 한다.

계정 파일·개인키·로그 디렉터리의 접근 권한을 제한해야 한다. Windows에서는 VNC Relay 실행 OS 계정과 관리자만 읽고 쓰도록 ACL을 제한하기를 권장한다. Ubuntu에서는 계정 파일·개인키에 `0600`을 권장한다. 자세한 절차는 [수동 운영 안내](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md) 3.2절에 있다.

## Windows 서비스 설치와 해제

먼저 [빠른 시작](quick-start.md)에 따라 설정과 관리자 계정을 준비한다. 서비스의 `log.output`은 파일 경로여야 한다. `%ProgramData%\vnc-relay\`에 운영 파일을 두기를 권장한다. 설치·해제에는 관리자 권한 PowerShell을 쓴다.

1. 준비한 설정·계정·사용 중인 TLS·키 파일을 `%ProgramData%\vnc-relay\`에 배치한다. 상대 경로도 이 위치를 기준으로 맞춘다.
2. 관리자 권한 PowerShell에서 묶음을 푼 디렉터리로 이동한다.
3. 묶음의 파일을 관리자만 고칠 수 있는 폴더에 복사한다.

   ```powershell
   New-Item -ItemType Directory -Force -Path "$env:ProgramFiles\vnc-relay"
   Copy-Item -Path .\* -Destination "$env:ProgramFiles\vnc-relay" -Recurse
   ```

   일반 사용자가 쓸 수 있는 폴더(예: 다운로드 폴더)의 실행파일로 서비스를 등록하지 않는다. 그 사용자가 실행파일을 바꾸면 다음 시작 때 바뀐 파일이 서비스 계정으로 실행된다. 설치 뒤에도 실행파일을 이 위치에 유지한다.

4. 복사한 폴더로 이동한다.

   ```powershell
   Set-Location "$env:ProgramFiles\vnc-relay"
   ```

5. 서비스를 설치한다. `<사용자>`·`<그룹>`은 UAC 없이 시작·중지할 실제 Windows 사용자나 그룹으로 바꾼다. `--operator`는 필요한 수만큼 반복한다.

   ```powershell
   .\vnc-relay.exe service install --config "$env:ProgramData\vnc-relay\relay.toml" --operator "<사용자>" --operator "<그룹>"
   ```

6. 서비스 계정 `NT SERVICE\vnc-relay`의 파일 접근 권한을 확인한다.

설치는 설정과 계정·TLS 등 사용 중인 파일을 검사하고 자동(지연 시작) 서비스를 등록한다. 복구 동작은 설정하지 않는다. 설치만으로 서비스를 시작하지 않는다. 설치 검사는 관리자 권한으로 실행하므로 서비스 계정의 파일 접근까지 보장하지 않는다.

서비스는 설정·계정·사용 중인 키 파일을 읽고 로그 디렉터리에 쓸 수 있어야 한다. 계정 파일과 개인키는 Administrators, SYSTEM, `NT SERVICE\vnc-relay`만 읽도록 제한하기를 권장한다. 트레이 사용자는 설정 파일 읽기 권한만 필요하다. 기본 `%ProgramData%` 권한에 의존하지 않는다. ACL 명령과 관리 API 설정 편집 권한은 [운영 안내 3.3.1절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md#331-windows-서비스와-트레이-앱)을 따른다.

서비스 시작·중지는 [Windows 서비스와 트레이 운영](operate.md#windows-서비스와-트레이)을 따른다. 해제할 때는 다음 순서를 쓴다.

1. 서비스 운영자 터미널에서 중지를 요청한다.

   ```powershell
   sc.exe stop vnc-relay
   ```

2. `sc.exe query vnc-relay`로 `STOPPED`가 될 때까지 기다린다.
3. 관리자 권한 PowerShell에서 등록을 해제한다.

   ```powershell
   .\vnc-relay.exe service uninstall
   ```

해제는 실행파일·설정·계정·인증서·로그를 지우지 않는다. 트레이 앱은 Windows 묶음의 `vnc-relay-tray.exe`를 일반 사용자로 실행한다. 트레이 앱은 서비스를 설치·해제하지 않는다. 명령과 오류의 기준은 [CLI 계약 1.9절](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md#19-windows-서비스-실행)이다.

## 대안: 소스에서 묶음 만들기

대상 OS에서 [개발 환경 준비와 검증](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/set-up-and-verify.md)에 따라 Rust와 C 빌드 도구를 준비한다. 묶음을 만들 소스가 Release 태그인지 `main`인지 구분한다.

1. 저장소 루트에서 묶음을 만든다.

   Windows PowerShell에서 NASM이 없으면 먼저 다음 환경 변수를 설정한다.

   ```powershell
   $env:AWS_LC_SYS_PREBUILT_NASM = "1"
   ```

   두 OS의 묶음 생성 명령은 같다.

   ```shell
   cargo xtask package
   ```

2. `target/package/`에서 OS용 묶음과 `.sha256` 파일을 찾는다. `CARGO_TARGET_DIR`을 정했으면 `target` 대신 그 경로를 쓴다.
3. 이 페이지의 해시 확인·압축 풀기 절차를 따른다.

묶음 이름·내용과 생성 절차의 기준은 [릴리스 안내](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/release.md) 1.2절·3.1절과 [수동 운영 안내](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md) 3.1절이다.

## 첫 실행 체크리스트

- [ ] 묶음의 해시가 일치한다.
- [ ] 실행파일과 `relay.toml`이 운영 디렉터리에 있다.
- [ ] 사용할 포트가 비어 있고 방화벽·출발지 IP 정책이 의도한 범위다.
- [ ] 계정 파일·개인키·로그 디렉터리의 접근 권한을 제한했다.
- [ ] VNC Relay를 시작하기 전에 관리자 계정을 만든다. 기본 계정·기본 비밀번호는 없으며 계정이 없으면 시작하지 않는다.

관리자 계정 생성부터는 [빠른 시작](quick-start.md)을 따른다. 공개 배치 전에는 [설정](configure.md)과 [운영](operate.md)을 따라야 한다.
