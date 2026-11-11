# 빠른 시작

이 페이지로 루프백 예시 설정을 실행하고 준비 상태와 대시보드 로그인을 확인한 뒤 정상 종료할 수 있다.

## 시작 전 확인

[설치](install.md)를 마친 뒤 묶음을 푼 디렉터리에서 실행한다. 실행파일과 `relay.toml`이 같은 디렉터리에 있어야 한다. 묶음의 `relay.toml`은 저장소의 [examples/relay.toml](https://github.com/kayden-kim/vnc-relay/blob/main/examples/relay.toml)이다.

이 예시는 같은 PC에서만 접속할 수 있다. TCP Mode II만 켜고 WS·WSS와 Mode I은 끈다.

| 용도 | 예시 주소 |
| --- | --- |
| TCP Viewer | `127.0.0.1:5901` |
| TCP Server | `127.0.0.1:5500` |
| 대시보드·관리 API | `http://127.0.0.1:18080/` |
| 준비 상태 | `http://127.0.0.1:18081/readyz` |

> **주의:** 이 구성은 공개 배치용이 아니다. 주소만 공개 주소로 바꾸어 사용하지 않는다. 공개 배치 전에 [설정](configure.md)과 [운영](operate.md)을 따라 리스너·출발지 IP 허용 목록·TLS·파일 권한을 준비해야 한다.

## 1. 관리자 계정 만들기

대화형 터미널에서 실행한다. 아래 `opsadmin`은 예시 관리자 계정 이름이다. 기본 계정·기본 비밀번호는 없다. 관리자 계정이 없으면 서버가 시작하지 않는다.

1. 관리자 계정을 추가한다.

   Windows PowerShell:

   ```powershell
   .\vnc-relay.exe admin add opsadmin --config .\relay.toml
   ```

   Ubuntu bash:

   ```shell
   ./vnc-relay admin add opsadmin --config ./relay.toml
   ```

2. 숨김 입력으로 묻는 비밀번호를 입력한다.
3. 확인용으로 같은 비밀번호를 다시 입력한다.

비밀번호는 12~128자여야 하며 UTF-8로 512바이트 이하여야 한다. 제어 문자는 쓸 수 없다. 비밀번호를 명령줄에 넣거나 파이프로 보내지 않는다. 성공하면 `relay.toml` 옆에 `admin-accounts.toml`이 생긴다.

## 2. 서버 시작하기

1. 같은 디렉터리의 전용 터미널에서 서버를 시작한다.

   Windows PowerShell:

   ```powershell
   .\vnc-relay.exe serve --config .\relay.toml
   ```

   Ubuntu bash:

   ```shell
   ./vnc-relay serve --config ./relay.toml
   ```

프로세스가 터미널을 점유한다. 예시 설정은 `relay.log`에 JSON Lines 운영 로그를 이어 쓴다. `server_ready` 사건이 기록되기 전에는 준비 완료로 판단하지 않는다.

## 3. 준비 상태 확인하기

1. 같은 PC에서 다른 터미널을 연다.
2. 상태 리스너의 `/readyz`를 조회한다.

   Windows PowerShell:

   ```powershell
   curl.exe --fail http://127.0.0.1:18081/readyz
   ```

   Ubuntu bash:

   ```shell
   curl --fail http://127.0.0.1:18081/readyz
   ```

준비되면 HTTP 200과 `{"status":"ready"}`가 나온다. 상태 리스너에는 인증이 없다. 대시보드가 제공하는 관리 리스너와는 다르다.

## 4. 대시보드 열기

1. 같은 PC의 브라우저에서 `http://127.0.0.1:18080/`을 연다.
2. 앞에서 만든 관리자 계정으로 로그인한다.

로그인하기 전에는 운영 데이터를 볼 수 없다. 화면 사용법은 [대시보드](dashboard.md)에 있다. VNC 연결은 [VNC 연결 구성](connect.md)을 따른다. 이 빠른 시작만으로 원격 화면 연결을 확인한 것은 아니다.

## 5. 정상 종료하기

1. `serve`를 실행한 터미널에서 Ctrl+C를 누른다.
2. 프로세스가 끝나고 프롬프트가 돌아올 때까지 기다린다.

Windows에서는 콘솔 창을 닫는 대신 Ctrl+C를 쓴다. 창 닫기는 남은 종료 유예 없이 연결을 정리한다.

전체 수동 절차는 [수동 운영 안내](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md) 3.2절·3.3절·3.8절에 있다. 계정 입력과 종료 규칙은 [CLI 계약](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md) 1.2절·1.5절·1.8절을 따른다. 실패하면 [문제 해결](troubleshooting.md)을 확인한다.
