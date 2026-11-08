# 실제 UltraVNC로 TCP 중계를 수동 확인하는 방법

프로젝트 소유자가 고정 버전 UltraVNC Server·Viewer로 Mode I·II의 화면과 입력을 확인한다. 이 문서는 TODO 2.13의 절차와 결과 양식이다. 자동 시험으로 확인할 부분도 구분한다. 절차를 작성했거나 화면이 보였다는 이유만으로 TODO 2.13을 완료 처리하지 않는다.

실행 주체는 사용자 결정 2026-11-05(Q116)로 바뀌었다. 에이전트가 VM 안의 실제 Viewer·Server를 UI 자동화로 실행·조작하고, 프로젝트 소유자는 판정 캡처를 검토한다. 이 결정은 사용자 결정 2026-11-02(Q87)의 "사용자가 수동 확인을 수행한다"를 대신한다. 절차와 판정 기준은 그대로다.

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 절차를 실행하는 사람(Q116에 따라 에이전트 자동 실행 포함)과 판정 캡처를 검토할 프로젝트 소유자 |
| 검증 상태 | 실행 완료(2026-11-05·06, Windows VNC Relay). 3절의 22~31·34단계를 모두 실행했고 모두 성공 조건과 맞았다(4.1·4.2절). 32단계 자동 증거 대조는 4.3절이다. 프로젝트 소유자가 2026-11-06에 판정 캡처를 검토했고 문제가 없었다. AC-08은 작업 5.4로 옮겼다(사용자 결정 2026-11-06(Q117)). Ubuntu VNC Relay는 분리 실행 환경이 없어 2절 규칙대로 미실행으로 남긴다 |
| 검증 환경·버전 | Hyper-V VM 3대(`vr-lab` `Private` 스위치, 게이트웨이 없음), 게스트 Windows 11 Enterprise Evaluation 10.0.26300, UltraVNC Server·Viewer 1.8.3.0, VNC Relay 커밋 `ceb6a7c` Windows debug 빌드. Ubuntu VNC Relay는 미실행 |
| 최종 검증일 | 2026-11-06. 작성일 2026-11-02 |
| 관련 기준 문서 | [TODO 2.13](https://github.com/kayden-kim/vnc-relay/blob/556fab23d15e77125ca2ad5c575089857b312049/docs/TODO.md#2-tcp-repeater), [PRD 10절](../requirements/PRD.md#10-검증-및-수용-기준), [중계 스펙 7절](../specs/relay.md#7-수용-테스트), [초기 교환 계약](../contracts/repeater-handshake-v1.md), [설정 계약](../contracts/config-v3.md), [CLI 계약](../contracts/cli-v1.md), [운영 정보 계약](../contracts/ops-info-v1.md), [호환성 검증 기록](../contracts/repeater-compat-record.md) |

## 1. 대상과 영향

- 대상: 시험용 `vnc-relay serve`, 실제 UltraVNC Server·Viewer, 시험용 설정·계정·로그 파일이다.
- 범위: RLY-006·RLY-010·RLY-011과 AC-01~04·AC-06~07·AC-15·AC-18·AC-26의 TCP 부분이다. 4절에서 각 ID의 수동 확인과 추가 증거를 나눈다. AC-08은 처음 범위에 있었지만 작업 5.4로 옮겼다(사용자 결정 2026-11-06(Q117)).
- 예상 사용자 영향: Server의 화면을 Viewer에 보내고 마우스·키보드 입력으로 Server의 데스크톱을 바꾼다. 오류 시험은 시험 연결을 거절하거나 끊는다. 운영 장비·실제 사용자 화면에서는 실행하지 않는다.
- 작업 허용 시간: 프로젝트 소유자가 정한 시험 시간에만 한다. 인터넷과 분리한 테스트 네트워크에서만 한다. 빌드와 도구 내려받기는 분리 환경 밖에서 먼저 마친다.
- 환경 경계: [관찰 환경 결정](../contracts/repeater-compat-record.md#32-관찰-환경-구성-2026-11-01)은 Hyper-V VM 세 대와 `Private` 스위치를 채택했다. 호스트 한 대의 루프백 방식으로 바꾸지 않는다. 이 절차는 같은 격리 경계를 재사용하되 Repeater 역할의 VM에서 `vnc-relay`를 실행한다.
- 범위 밖: WS·WSS·noVNC, 공식 UltraVNC Repeater 재관찰, 관리자 종료 API 검증이다. TCP에 해당하지 않거나 다른 TODO의 작업이다.
- 자동 시험으로 확인할 부분: 정확한 바이트 경계·B 상한·동시성·Lm 포화는 GUI 조작으로 증명하지 않는다. AC-26의 무응답 단절은 사용자 결정 2026-11-02(Q86)에 따라 TODO 2.11의 WSL Linux netns와 nftables 또는 iptables `DROP` 자동 시험으로 확인한다. Windows는 부분 시험으로 남긴다. 이 문서에는 차단 명령을 만들거나 수동 실행 단계로 넣지 않는다. 해당 자동 시험은 이 절차 작성 시점에 미구현이다.
- UltraVNC·websockify·noVNC 소스 코드를 읽거나 복사·번역·포팅하지 않는다. 기존 관찰 기록과 공개 사용자 문서만 참조한다.

## 2. 전제 조건

- [ ] 프로젝트 소유자의 관찰 허용 범위 안이다. [기록 1절](../contracts/repeater-compat-record.md#1-관찰-허용-검토-todo-11)의 허용은 법률 검토 결과가 아니다.
- [ ] [고정 배포본](../contracts/repeater-compat-record.md#21-ultravnc-serverviewer-windows-x64--고정)으로 Server·Viewer를 준비했다. 두 실행 파일의 파일 버전이 모두 **1.8.3.0**이다. 새 버전으로 바꾸지 않는다.
- [ ] 설치 파일 `UltraVNC_1830_x64_Setup.exe`의 SHA-256이 `e9c22419ef3128a707b0d004a2c58e25459528c5c12d104dea56e5e662bd131e`다. 파일별 해시와 기존 설치 위치는 [설치 확인 기록](../contracts/repeater-compat-record.md#31-관찰-준비-설치-확인과-캡처-도구-2026-10-27)을 따른다.
- [ ] 실행할 `vncviewer.exe`와 `winvnc.exe`의 실제 경로·파일 버전·해시를 기록했다. VM의 기존 경로는 `C:\Program Files\uvnc\UltraVNC\`다. 호스트 설치 경로와 혼동하지 않는다.
- [ ] 아래 세 역할이 서로 다른 VM에 있다. 기본 경로·인터넷 연결이 없고, 시험용 문서 대역만 쓴다.

  | 역할 | 기존 VM | IPv4 | 필요한 연결 |
  | --- | --- | --- | --- |
  | Viewer | `vr-viewer` | `192.0.2.10` | VNC Relay의 TCP Viewer 포트 5901로 접속 |
  | VNC Relay | `vr-repeater` | `192.0.2.20` | Viewer 5901·Server 5500 수락, Mode I 대상 5900으로 접속 |
  | Server | `vr-server` | `192.0.2.30` | Mode I은 5900 수락, Mode II는 VNC Relay의 5500으로 역접속 |

- [ ] 공식 `repeater.exe`가 같은 포트를 점유하지 않는다. 이번 시험에서 중계하는 프로그램은 `vnc-relay`다.
- [ ] 방화벽이 위 방향·IP·TCP 포트만 허용한다. 기존 `vr-lab repeater` 규칙은 `repeater.exe`용이므로 `vnc-relay.exe`에 자동 적용된다고 보지 않는다. 광범위한 기존 UltraVNC 허용 규칙도 확인한다. 방화벽을 통째로 끄지 않는다.
- [ ] Server와 Viewer를 로그온한 데스크톱 세션에서 실행할 수 있다. Server는 시험용 VNC 인증을 사용하고 마우스·키보드 입력을 허용한다. 실제 인증 정보는 쓰지 않는다.
- [ ] Server의 VNC 인증·입력 허용 설정 방법과 현재 UI 필드명을 실행자가 실제 1.8.3.0 화면과 대조해 기록한다. 이 문서는 UI 이름이나 비밀번호 옵션을 추측하지 않는다. 2026-11-05 실행에서 확인한 화면은 다음과 같다. Server 트레이 아이콘 메뉴 `Settings` → 창 `UltraVNC Server - Settings - Config file: C:\ProgramData\UltraVNC\ultravnc.ini` → `Security` 탭 `Authentication`의 `VNC Password` 옆 `CHANGE` → 창 `UltraVNC Server - Change Password`의 `Enter a new password`·`Confirm the new password` → `SAVE`. 마우스로 `SAVE`를 누른 회차는 두 칸이 채워져 있어도 `Password can not be empty.` 경고가 났고, Enter 키로 저장한 회차는 `passwd` 줄이 바뀌었다. 같은 순서의 `View-Only Password` 저장은 Enter 키로도 같은 경고가 나서 baseline 값을 그대로 두었다. 저장 뒤 `ultravnc.ini`에는 `InputsEnabled=1`이 있었다. 입력 허용 UI(`Input/FT` 탭)는 열어 보지 않았다.
- [ ] Windows 11 x64 또는 Ubuntu 24.04 LTS x64용 VNC Relay 실행파일이 있다. Windows·Ubuntu의 증거는 각각 기록한다. Ubuntu 비교 실행은 별도 분리 호스트/VM의 역할을 같은 주소·정책으로 구성한 뒤 한다. 실행 환경이 없으면 그 OS는 미실행으로 남긴다.
- [ ] 설정·계정·로그를 둘 시험 전용 디렉터리 `C:\vr\manual-ultravnc\`를 마련했다. 기존 파일이 없는지 확인했다. 계정 파일은 실행 계정과 관리자만 읽고 쓸 수 있게 제한한다. Ubuntu에서는 이에 대응하는 저장소 밖 디렉터리와 계정 파일 권한 `0600`을 쓴다.
- [ ] 기존 설정·Server 설정·방화벽 규칙을 기록해 복구할 수 있다. VM의 기존 `baseline`으로 되돌리면 그 뒤 만든 파일을 잃을 수 있으므로 증거 반출을 먼저 한다.

## 3. 절차

- 실행 환경: 아래 명령은 Windows PowerShell 기준이다. 빌드는 저장소 루트, `serve`와 `admin`은 VNC Relay VM에서 실행한다. UltraVNC 명령은 각 역할의 VM에서 실행한다.
- 바꿔 넣을 값: VNC Relay 실행파일 경로는 실제 반입 위치로 바꾼다. UltraVNC 경로는 2절에서 확인한 위치를 쓴다. 주소·포트는 위 격리 구성과 같을 때만 그대로 쓴다. `1234`·`5678`은 비밀이 아닌 시험 전용 중계 ID다.
- 명령줄에 비밀번호·인증 키·계정 해시를 넣지 않는다. VNC 인증은 Viewer의 입력 창에서 하고, 관리자 비밀번호는 `admin add`의 숨김 입력으로만 넣는다.
- 최대 대기 시간: 빌드는 [개발 환경 how-to](set-up-and-verify.md#3-절차)의 10분을 따른다. 아래 설정의 초기 교환 H·대상 연결 C·종료 정리 E는 각 10초다. Mode II 대기 W는 3600초다. K는 `60 + 10 × 6 = 120`초다. 화면·입력 확인은 상대 접속 뒤 1분 안에 판단한다. 이 1분은 작업 중단 기준이지 제품 응답 시간 보장이 아니다. 기한을 넘으면 5절을 따른다.

### 3.1 실행파일과 설정 준비

1. 빌드 호스트의 저장소 루트에서 실행파일을 빌드한다. 준비된 바이너리를 쓰면 이 단계 대신 빌드한 커밋·OS·해시·반입 경로를 기록한다.

   ```powershell
   cargo build
   ```

   기대 결과: `Finished`로 끝나고 `target\debug\vnc-relay.exe`가 생긴다. Windows의 링커·NASM 준비는 [개발 환경 how-to](set-up-and-verify.md#31-도구-설치)를 따른다. Ubuntu 빌드 결과는 `target/debug/vnc-relay`다.

2. 실행파일을 VNC Relay VM의 `C:\vr\manual-ultravnc\vnc-relay.exe`에 반입한다. 네트워크 격리를 풀지 말고 [기존 PowerShell Direct 반입 방식](../contracts/repeater-compat-record.md#32-관찰-환경-구성-2026-11-01)을 쓴다.

   기대 결과: 분리된 VM 안에 해당 OS용 실행파일이 있다.

3. VM에서 반입한 실행파일의 버전을 읽는다.

   ```powershell
   C:\vr\manual-ultravnc\vnc-relay.exe --version
   ```

   기대 결과: 버전 한 줄이 나오고 종료 코드가 0이다. 실제 출력과 빌드한 커밋을 기록한다. 제품 버전만으로 커밋을 대신하지 않는다.

4. 텍스트 편집기로 `C:\vr\manual-ultravnc\relay.toml`을 UTF-8로 저장한다.

   ```toml
   [admin]
   accounts_file = "admin-accounts.toml"

   [relay.tcp_viewer]
   enabled = true
   listen = "192.0.2.20:5901"
   allow = ["192.0.2.10"]

   [relay.tcp_server]
   enabled = true
   listen = "192.0.2.20:5500"
   allow = ["192.0.2.30"]

   [mode1]
   enabled = true
   allow_targets = ["192.0.2.30:5900"]

   [mode2]
   enabled = true

   [limits]
   relay_connections = 1000
   admin_connections = 64
   waiting = 200
   buffer_bytes = 131072

   [timeouts]
   handshake_secs = 10
   wait_secs = 3600
   connect_secs = 10
   close_secs = 10

   [keepalive]
   tcp_idle_secs = 60
   tcp_interval_secs = 10
   tcp_probes = 6

   [management]
   listen = "127.0.0.1:18080"

   [status]
   listen = "127.0.0.1:18081"

   [log]
   output = "relay.jsonl"
   ```

   기대 결과: Mode I·II와 TCP 진입점만 켠 설정 파일이 생긴다. 수치형 상한·시간은 [설정 계약 1.2절](../contracts/config-v3.md#12-키-표)의 기본값이다. 관리·상태 포트는 같은 계약 5절의 시험 예시이며 제품 기본 포트가 아니다. 상대 경로인 계정·로그 파일은 설정 파일 디렉터리에서 찾는다. Mode I만 시험하면 `mode2.enabled`를 `false`로, Mode II만 시험하면 `mode1.enabled`를 `false`로 바꾼 별도 사본을 쓸 수 있다. 본 절차는 두 모드를 켠 원본을 쓴다.

5. 대화형 터미널에서 시험용 관리자 계정을 만든다. TCP 중계에 관리자 로그인을 쓰지는 않지만 `serve` 시작에는 계정이 하나 이상 필요하다.

   ```powershell
   C:\vr\manual-ultravnc\vnc-relay.exe admin add lab-admin --config C:\vr\manual-ultravnc\relay.toml
   ```

   기대 결과: 12~128자의 시험 전용 비밀번호를 두 번 숨김 입력하면 성공 문장과 종료 코드 0이 나온다. `admin-accounts.toml`에는 CLI가 만든 해시가 저장된다. 직접 해시를 작성하지 않는다.

6. VNC Relay VM의 터미널에서 서버를 시작한다.

   ```powershell
   C:\vr\manual-ultravnc\vnc-relay.exe serve --config C:\vr\manual-ultravnc\relay.toml
   ```

   기대 결과: 명령은 포그라운드에서 계속 실행된다. `relay.jsonl`에 `server_starting` 뒤 `server_ready`가 나온다. `listeners`에는 `tcp_viewer`·`tcp_server`·`management`·`status`와 설정한 주소가 있다. 파일 로그를 선택했으므로 stdout은 비어 있다. stderr도 별도로 보존한다.

### 3.2 Mode I: Viewer → VNC Relay → Server

아래 UltraVNC 옵션과 `host::port` 표기는 [실제 관찰 입력](../contracts/repeater-compat-record.md#34-시나리오별-입력과-결과) M1-01·M1-05에서 가져왔다. 전송되는 초기 요청은 `host:port`다. UI 필드를 추측하지 않도록 명령줄을 쓴다.

7. Server VM의 로그온한 화면에서 Server를 실행한다.

   ```powershell
   & 'C:\Program Files\uvnc\UltraVNC\winvnc.exe' -run
   ```

   기대 결과: 시험용 Server가 5900에서 접속을 받을 수 있다. 실제 인증·입력 설정이 준비되지 않았으면 여기서 멈춘다.

8. Viewer VM에서 허용 대상으로 접속한다.

   ```powershell
   & 'C:\Program Files\uvnc\UltraVNC\vncviewer.exe' -connect 192.0.2.30::5900 -proxy 192.0.2.20:5901
   ```

   기대 결과: 시험용 VNC 인증 뒤 Server의 화면이 보인다. 로그의 `connection_accepted`는 `tcp_viewer`이고 `session_started`는 `mode=mode1`, `viewer_transport=tcp`, `target_ip=192.0.2.30`, `target_port=5900`이다.

9. Viewer 화면에서 마우스로 Server의 시험용 편집 창을 선택한다.

   기대 결과: Server 콘솔에서도 같은 창 선택이 보인다. 화면만 받은 상태를 입력 성공으로 기록하지 않는다.

10. Viewer에서 시험용 문자열을 입력한다.

    기대 결과: Server 콘솔의 편집 창에 같은 문자열이 보인다. 시험용 화면과 문자열만 캡처한다. RLY-006의 화면·마우스·키보드 결과를 각각 기록한다.

11. Viewer 창을 닫아 접속을 끝낸다.

    기대 결과: 해당 `session_id`의 `session_closed`가 한 번 나온다. `close_cause`, 방향별 `bytes_viewer_to_target`·`bytes_target_to_viewer`를 보존한다. 종료 원인은 실제 EOF·오류에 따라 기록하고 특정 값을 지어내지 않는다.

### 3.3 Mode II: Server 먼저 접속

Mode I Server를 종료한 뒤 시작한다. 두 Server 프로세스를 겹쳐 실행하지 않는다. 아래 입력은 [관찰 기록](../contracts/repeater-compat-record.md#34-시나리오별-입력과-결과)의 Mode II 명령이다. UI를 쓰면 필요한 값은 Server 쪽 VNC Relay 주소 `192.0.2.20:5500`, Viewer 쪽 주소 `192.0.2.20:5901`, 양쪽 값 `ID:1234`다. 실제 UI 필드명은 **확인 필요**이며 실행자가 기록한다. 중계 ID만으로는 인증이 되지 않는다.

2026-11-05 실행에서 `winvnc -id:1234 -connect 192.0.2.20::5500 -run`으로 역접속한 Server는 VNC 인증을 요구하지 않았다. Viewer는 `UltraVNC Viewer` `Security` 창에 "The server has been configured without authentication. Can it be trusted?"를 띄웠다. `Do you trust it and connect without authentication`을 고른 뒤 화면이 열렸다. 이 창을 닫은 회차에서는 `You refused a untrusted server.`가 나왔다. 이것은 UltraVNC Server·Viewer 사이의 RFB 보안 협상이며, VNC Relay는 매칭 뒤 바이트를 그대로 전달한다. 역접속에서 인증을 요구하게 하는 Server 설정은 확인하지 않았다. 아래 13·18단계의 "인증 뒤"는 이 실행에서 "인증 없음 확인 창을 신뢰로 통과한 뒤"였다. Mode I(3.2절)에서는 `UltraVNC Viewer - Authentication` 창에 시험용 VNC 비밀번호를 넣어 인증했다.

12. Server VM에서 시험 ID로 역접속한다.

    ```powershell
    & 'C:\Program Files\uvnc\UltraVNC\winvnc.exe' -id:1234 -connect 192.0.2.20::5500 -run
    ```

    기대 결과: `waiting_registered`의 `listener=tcp_server`, `role=server`, `mode=mode2`가 나온다. 해당 `conn_id`와 `relay_id_ref`를 적는다. 아직 `session_started`는 없다.

13. Viewer VM에서 같은 ID로 접속한다.

    ```powershell
    & 'C:\Program Files\uvnc\UltraVNC\vncviewer.exe' -proxy 192.0.2.20:5901 ID:1234
    ```

    기대 결과: 하나의 `session_started`가 나온다. `mode=mode2`, `viewer_transport=tcp`, 같은 `relay_id_ref`, 서로 다른 `viewer_conn_id`·`server_conn_id`, 새 `session_id`가 있다. 인증 뒤 화면이 보인다.

14. Viewer에서 마우스로 시험용 편집 창을 선택한다.

    기대 결과: Server 콘솔에서 같은 선택이 보인다.

15. Viewer에서 시험용 문자열을 입력한다.

    기대 결과: Server 콘솔에 같은 문자열이 보인다. RLY-010의 화면·마우스·키보드 결과를 기록한다.

16. Viewer 창을 닫는다.

    기대 결과: 해당 `session_id`의 종료 사건이 나온다. 자동 재접속 옵션은 쓰지 않았으므로 다음 회차는 양 끝점을 새로 실행한다.

### 3.4 Mode II: Viewer 먼저 접속

17. Server 프로세스가 종료된 상태에서 13단계의 Viewer 명령을 먼저 실행한다.

    기대 결과: `waiting_registered`의 `listener=tcp_viewer`, `role=viewer`가 먼저 나온다. 아직 세션은 없다.

18. 12단계의 Server 명령을 실행한다.

    기대 결과: 같은 ID의 반대 역할이 매칭되어 새 `session_id`가 생긴다. 인증 뒤 화면이 보인다.

19. Viewer에서 마우스로 시험용 편집 창을 선택한다.

    기대 결과: Server 콘솔에서 같은 선택이 보인다.

20. Viewer에서 다른 시험용 문자열을 입력한다.

    기대 결과: Server 콘솔에 같은 문자열이 보인다. RLY-011의 화면·마우스·키보드 결과를 기록한다.

21. Viewer 창을 닫는다.

    기대 결과: 새 세션의 종료 사건이 나온다. 같은 중계 ID여도 13단계와 18단계의 `session_id`는 다르다. 이전 화면·입력 상태가 새 세션의 중계 상태에 섞이지 않는다. 이전 정리 작업을 의도적으로 지연하는 부분은 자동 시험 증거로 보완한다.

### 3.5 거절·ID 분리·재접속 확인

각 회차 전에 앞 회차의 Viewer·Server 연결을 정리한다. 정상 세션 하나를 유지하며 오류를 주는 회차는 별도로 표시한다. 대기 연결을 남긴 채 ID를 재사용하지 않는다.

22. Mode I Viewer 명령의 대상만 `192.0.2.30::5999`로 바꾸어 접속한다.

    기대 결과: `connection_rejected`의 `reason=target_not_allowed`가 나온다. `allow_targets`에 없는 포트이므로 대상 연결 실패 시험이 아니다. `session_started`나 대상 연결 패킷이 없어야 한다.

23. Mode I Viewer 명령의 대상만 `192.0.2.10::5900`으로 바꾸어 접속한다.

    기대 결과: 금지 IP에 대한 `target_not_allowed`가 나온다. 대상 연결 패킷이 없어야 한다. 대상 연결이 없다는 증거는 VNC Relay 쪽 캡처 또는 자동 시험을 함께 쓴다. Viewer 오류 창만으로 판정하지 않는다.

24. Server를 `-id:1234`로 등록한 뒤 Viewer를 `ID:5678`로 접속한다.

    기대 결과: 서로 다른 `relay_id_ref`로 대기하고 매칭되지 않는다. 어느 쪽에도 상대 화면·입력이 전달되지 않는다. 관찰 뒤 양쪽을 닫는다. 대기 연결의 `waiting_removed`와 `reason`을 보존한다. 정확한 바이트 비교·앞자리 0·같은 역할·동시 접속은 자동 시험으로 보완한다.

25. ID `1234`의 Viewer 하나만 대기시킨 상태에서 같은 명령으로 Viewer를 하나 더 접속한다.

    기대 결과: 후발 연결은 `connection_rejected(reason=duplicate_id)`로 닫힌다. 후발 연결의 `waiting_registered`는 없다. 처음 연결은 유지된다. Server를 연결한 뒤 첫 Viewer가 정상 동작하는지도 별도 회차로 확인한다. Server 중복·활성 세션 중복·동시 도착은 자동 시험 증거도 요구한다.

26. 정상 Mode II 세션을 종료한 뒤 양 끝점을 같은 ID로 새로 실행한다.

    기대 결과: 새 `session_id`가 생기고 화면·입력이 정상 동작한다. 이전 세션의 `session_closed`와 새 `session_started`를 모두 보존한다. `relay_id_ref`는 한 프로세스 안의 같은 ID라면 같을 수 있지만 `session_id`는 같아서는 안 된다.

### 3.6 출발지 IP 경계와 살아 있는 유휴 연결

IP 정책 변형은 원본이 아닌 설정 사본에서 한다. 현재 프로세스를 Ctrl+C로 끝내고 정리가 끝난 뒤 사본으로 시작한다. 자동 재로드가 이 시험의 선행 조건이라고 가정하지 않는다.

27. `relay.tcp_viewer.allow = []`인 설정 사본으로 실행 중인 VNC Relay에 Viewer 접속을 시도한다.

    기대 결과: `connection_rejected(listener=tcp_viewer, stage=accept, reason=ip_not_allowed)`가 나온다. 해당 접속의 `connection_accepted`·대기 등록·세션 시작은 없다. TCP Server 목록은 바꾸지 않았으므로 Server 등록 허용 여부는 독립이다.

28. `relay.tcp_server.allow = []`인 별도 설정 사본으로 실행 중인 VNC Relay에 Server 등록을 시도한다.

    기대 결과: 같은 거절 사건의 `listener`가 `tcp_server`다. Viewer 목록으로 Server를 허용하지 않는다. 반대 방향의 진입점 목록 분리·허용 IP를 주장하는 데이터·기존 세션 유지의 증거는 자동 시험으로 보완한다.

29. 켠 진입점의 `allow` 줄을 지운 설정 사본으로 시작한다.

    기대 결과: 설정 오류로 시작에 실패하고 `server_ready`가 없다. 누락과 명시적 빈 목록을 같은 것으로 취급하지 않는다. stderr의 오류 코드·키와 실제 종료 코드를 기록한다. Viewer·Server 각각 독립 회차로 한다.

30. 격리가 유지된 상태에서 해당 진입점 `allow`만 `["0.0.0.0/0"]`로 바꾼 설정 사본으로 접속한다.

    영향: 해당 진입점은 시험망의 모든 IPv4 출발지를 허용한다. 회차가 끝나면 프로세스를 종료하고 원본 설정으로 되돌린다. 인터넷·운영 네트워크에 연결된 환경에서는 실행하지 않는다.

    기대 결과: 시작 때 `allow_all_source(level=warn, listener=해당 진입점, phase=startup)`가 나오고 IPv4 시험 연결이 허용된다. TCP Viewer·TCP Server 각각 독립 회차로 한다. IPv6 전체 허용 경계는 자동 시험 증거로 보완한다.

31. 원본 설정으로 만든 정상 활성 세션을 입력 없이 K보다 오래 유지한다.

    기대 결과: 120초가 지난 뒤에도 세션이 유지된다. 이어서 마우스와 키보드를 각각 조작하면 정상 전달된다. 시작·종료 시각과 실제 유휴 시간을 기록한다. RFB 자체의 주기 바이트가 있으면 함께 기록한다. 애플리케이션 데이터가 실제로 없었는지는 자동 시험의 RLY-027 증거로 보완한다. 프로세스 종료·케이블 분리·VM 종료를 무응답 `DROP` 시험으로 대신하지 않는다.

### 3.7 자동 증거와 운영 로그 수집

32. 같은 대상 커밋의 자동 시나리오 보고서를 확인한다. 새 보고서가 필요하면 격리 중계 시험과 분리된 개발 환경에서 기존 검증 명령을 실행한다.

    ```powershell
    cargo xtask scenarios
    ```

    기대 결과: Cargo 대상 디렉터리에 `scenario-report.md`가 생긴다. [개발 환경 how-to](set-up-and-verify.md#34-전체-완료-검증)의 브라우저 등 전제 조건을 먼저 충족한다. 전체 명령의 종료 코드 0은 모든 AC의 완료를 뜻하지 않는다. 4절에 적은 각 시나리오의 개별 결과·환경·시험 내용을 확인한다. `부분`·`미실행`·`미연결`은 전체 통과로 올리지 않는다. RLY-006·010·011의 실제 끝점 증거는 이 문서의 수동 기록으로 별도 전달한다.

33. 각 회차의 `relay.jsonl`에서 아래 사건을 연결해 증거 사본을 만든다.

    기대 결과: ID별 결과를 증거 파일과 줄 번호로 찾을 수 있다. 로그는 UTF-8 JSON Lines다. 사건 이름·필드가 없으면 “확인하지 못함”으로 남긴다. 로그가 보장하지 않는 버퍼 크기·소켓 수·화면 성공을 추정하지 않는다.

    | 확인할 동작 | 보존할 사건과 필드 |
    | --- | --- |
    | 시작 | `server_starting`, `server_ready(listeners)`; 실패하면 `startup_failed(reason, config_key, listener)`와 stderr |
    | 접속 수락·IP 거절 | `connection_accepted(conn_id, listener, role, peer_ip)`, `connection_rejected(listener, stage, reason, peer_ip)` |
    | 대상 거절·실패 | `connection_rejected(reason=target_not_allowed, target_ip/target_host/target_port 중 실제 있는 필드)`, `target_connect_failed(reason, elapsed_ms, conn_id)` |
    | 대기·매칭 순서 | `waiting_registered(conn_id, role, relay_id_ref)`, `session_started(session_id, mode, viewer_transport, viewer_conn_id, server_conn_id, relay_id_ref, waited_ms)` |
    | 중복·만료·단절 | `connection_rejected(reason=duplicate_id)`, `waiting_expired(conn_id, relay_id_ref, waited_ms)`, `waiting_removed(reason, relay_id_ref)`, `connection_aborted(stage, reason, elapsed_ms)` |
    | 재접속·종료 | `session_started`와 `session_closed`의 `session_id`; 종료의 `close_cause`, `side`·`error_kind`가 있으면 함께, 방향별 최종 바이트 |
    | 상한·끊긴 연결 자동 증거 | `connection_rejected(reason=relay_connection_limit/admin_connection_limit/waiting_full, limit)`, `waiting_removed(reason=dead_connection)`, `session_closed(close_cause=dead_connection)` |

    `relay_id_ref`는 원문 중계 ID를 숨긴 참조 값이다. 원문 `relay_id`, 초기 교환 바이트, VNC 인증, 화면 데이터, 사용자 입력은 운영 로그에 넣지 않는다. `relay_id_ref`는 프로세스 재시작 뒤 달라질 수 있으므로 회차·프로세스와 함께 기록한다.

34. RLY-006·010·011의 화면과 입력 회차를 각각 독립적으로 한 번 더 실행한다.

    기대 결과: 두 회차의 결과를 각각 기록할 수 있다. 비교 캡처를 하면 [초기 교환 계약 6.1절](../contracts/repeater-handshake-v1.md#61-관찰-절차)에 따라 VNC Relay 쪽에서 수집한다. 공식 Repeater와 의도적으로 다른 동작은 [계약 3.1절](../contracts/repeater-handshake-v1.md#31-대상-버전-repeater와-다르게-처리하는-입력)을 따른다. 모든 차이를 결함이나 호환 성공으로 단정하지 않는다.

## 4. 검증

[검증 기록](../AGENTS.md#검증-기록)에 따라 실제 결과와 성공 조건을 대조한다. 아래 칸은 실행자가 채운다. 수동 범위 밖으로 적은 부분도 TODO 2.13의 완료 조건에서 없어진 것이 아니다. 필요한 자동 증거가 없으면 해당 AC 전체를 미실행 또는 부분으로 보고한다.

| 관찰 대상 | 성공 조건 | 확인 명령·방법 | 실제 결과 | 실행 증거 위치 |
| --- | --- | --- | --- | --- |
| RLY-006 | 허용된 TCP Mode I의 실제 화면·마우스·키보드 동작 | 7~11·34단계. 화면과 Server 콘솔 대조 | | |
| RLY-010 | Server 선접속 뒤 같은 ID Viewer 한 쌍 매칭, 실제 화면·입력 | 12~16·34단계. 대기·매칭 로그와 화면 대조 | | |
| RLY-011 | Viewer 선접속도 같은 화면·입력 결과 | 17~21·34단계. Viewer 대기가 먼저인 로그 | | |
| AC-01 | 허용 실제 대상은 동작하고 금지 IP·포트·최종 IP는 연결하지 않음 | RLY-006 수동 + 22~23단계. RLY-007~009 자동 증거 추가. 이름→금지 최종 IP, 주소 해석 실패·연결 거절·C 초과는 GUI 수동 범위 밖: 결정적인 주입이 필요하다 | | |
| AC-02 | 양쪽 접속 순서 모두 같은 ID로 원격 제어 성립 | RLY-010·RLY-011의 두 회차 화면·입력·매칭 증거 | | |
| AC-03 | 다른 ID·같은 역할은 분리, 동시 도착에서 한 연결은 한 세션에만 속함 | 24~25단계 + RLY-012~014 자동 증거. 정확한 ID 바이트·동시성·만료 경합은 수동 범위 밖: GUI로 도착 시점을 제어할 수 없다 | | |
| AC-04 | 대기·활성 ID의 후발 같은 역할 거절, 기존 연결 유지, 재접속은 새 세션 | 25~26단계 + RLY-015~016 자동 증거. Server 중복·활성 중복·이전 정리 지연을 포함한다 | | |
| AC-06 | TCP 초기 요청 분할·병합·후속 바이트와 양방향 스트림에 유실·중복 없음 | RLY-002~003·RLY-017 자동 증거. 수동 범위 밖: 화면·패킷 경계만으로 정확한 전체 바이트를 증명하지 못한다. WS 부분은 TODO 3.5 범위 | | |
| AC-07 | 형식 오류·미완료 단절·H·W·C 초과·연결 거절·양쪽 종료 후 자원 회수 | 11·16·21·24단계 종료 로그 + RLY-004~005·009·014·019~020과 OPS-006의 TCP 자동 증거. half-close·정확한 자원 회수·경합은 수동 범위 밖: 제어 끝점·상태 관측이 필요하다. 관리자 종료 부분은 TODO 4.5, WS 부분은 TODO 3.5이며 이 절차 통과로 완료 주장하지 않는다 | | |
| AC-08 | 느린 수신에서도 버퍼 B·연결 L·대기 Lw 준수, 다른 세션 유지 | RLY-018·RLY-021·OPS-004~005·OPS-017의 TCP 자동 증거. 정량 B·포화·느린 수신자 시험은 수동 범위 밖: GUI와 프로세스 전체 메모리만으로 연결별 B를 재지 못한다. 작성 당시 OPS-017의 허용 오버헤드·잔여 자원 판정 구간은 미확정이었다. 이후 사용자 결정 2026-11-06(Q118·Q119·Q120)으로 운영 스펙 6절에 정했고, AC-08은 작업 5.4로 옮겼다(Q117) | | |
| AC-15 | TCP 진입점별 출발지 목록 적용, 비허용 IP 사전 거절, 누락 실패·빈 목록 거절·전체 허용 경고 | 27~30단계 + RLY-023~024·OPS-037~038의 TCP 자동 증거. 상대가 주장하는 IP·기존 세션 유지·IPv6 경계를 포함한다. WS·WSS 부분은 TODO 3.5 범위 | | |
| AC-18 | 중계 L과 관리·상태 Lm이 서로 수락을 막지 않음 | OPS-033 자동 증거에서 L 포화 중 관리·상태 수락과 Lm 포화 중 중계 수락을 모두 확인. 수동 범위 밖: GUI 연결 수만으로 양 예산의 포화·회수를 제어할 수 없다 | | |
| AC-26 | TCP 무응답 대기·활성 연결을 K 안에 회수하고 살아 있는 유휴 연결 유지 | 31단계 + RLY-027 자동 증거. RLY-025~026은 수동 범위 밖이며 TODO 2.11의 Q86 Linux netns + DROP 자동 시험 증거가 필요하다(작성 시 미구현). Windows는 부분으로 구분한다. WS Kw 부분은 TODO 3.4 범위 | | |

자동 시험 위치·결과는 최신 `scenario-report.md`에서 찾는다. 과거 문서의 통과 기록을 현재 작업 트리의 실행 결과로 복사하지 않는다. 특히 Windows에서 RLY-009가 부분인 이유와 Linux의 전체 증거를 구분한다([TODO 2.8](https://github.com/kayden-kim/vnc-relay/blob/556fab23d15e77125ca2ad5c575089857b312049/docs/TODO.md#2-tcp-repeater)). 운영 로그에 사건이 있다는 것만으로 메모리·자원 회수·바이트 무결성을 증명하지 않는다.

### 복사해서 쓸 실행 기록

이 양식의 빈칸은 실행자가 채운다. 실행 전 상태는 모두 미실행이다.

```text
실행일·시각·시간대:
실행자:
대상 커밋:
미커밋 변경 범위:
VNC Relay --version / 실행파일 경로·SHA-256:
Viewer 파일 버전·경로·SHA-256:
Server 파일 버전·경로·SHA-256:
VNC Relay 호스트 OS·빌드 / Viewer OS / Server OS:
격리 네트워크·방화벽 확인:
설정 사본 위치 / L·Lm·Lw·B·H·W·C·E·K:
VNC 인증·입력 허용의 실제 설정 방법(UI를 썼으면 필드명):
수동 회차별 시작·종료 시각 / 명령(비밀 제외):
프로세스별 relay_id_ref·session_id·conn_id와 사건 로그 파일:줄:
화면·마우스·키보드 증거 위치:
자동 시험 명령·OS·보고서 위치·대상 커밋:

ID       결과(통과/실패/미실행, 부분이면 범위) / 관찰 / 증거
RLY-006:
RLY-010:
RLY-011:
AC-01:
AC-02:
AC-03:
AC-04:
AC-06:
AC-07:
AC-08:
AC-15:
AC-18:
AC-26:

확인 필요 항목과 미실행 이유:
중단 시각·직전 단계·오류:
복구 결과 / 증거 반출 위치 / 임시 파일 제거 여부:
```

### 4.1 2026-11-05 실행 기록

결론: 실제 UltraVNC Server·Viewer 1.8.3.0이 VNC Relay를 거쳐 Mode I, Mode II(Server 먼저), Mode II(Viewer 먼저)에서 화면·마우스·키보드 동작을 각 1회 보였다. 22~24단계와 26단계도 성공 조건과 맞았다. 25단계는 중단했고 27~31·34단계는 실행하지 않았다. 따라서 RLY-006·RLY-010·RLY-011과 아래 AC 중 어느 것도 완료로 보지 않는다.

실행 방법은 사용자 결정 2026-11-05(Q116)를 따랐다. 호스트의 관리자 PowerShell이 PowerShell Direct로 각 VM에 명령을 보냈다. Server VM에는 시험 창(분홍 단추와 하늘색 입력 칸, 클릭·초점·입력을 파일에 기록)을 띄웠다. Viewer VM의 스크립트는 Viewer 화면에서 분홍 단추를 찾아 누르고, 입력 칸을 눌러 회차 문자열을 입력했다. 판정은 Server 시험 창 기록, 두 VM의 캡처, VNC Relay 운영 로그를 함께 대조했다. 이 스크립트는 저장소 밖 시험 도구이며 제품 코드가 아니다(이 문서의 판단, 사용자 결정 없음).

```text
실행일·시각·시간대: 2026-11-05 20:31~20:42 KST(UTC+9). 로그 ts는 UTC 11:31~11:42
실행자: 에이전트 자동 실행(Q116). 캡처 검토: 프로젝트 소유자, 2026-11-06 검토 완료, 문제 없음
대상 커밋: ceb6a7c
미커밋 변경 범위: 없음
VNC Relay --version / 실행파일 경로·SHA-256: vnc-relay 0.1.0 / vr-repeater C:\vr\manual-ultravnc\vnc-relay.exe(Windows debug 빌드) / cc70349a18142684fa0a3a09db1ca58702021455ede98316512a6cc734c79f28
Viewer 파일 버전·경로·SHA-256: 1.8.3.0 / C:\Program Files\uvnc\UltraVNC\vncviewer.exe / 고정 기록과 같음
Server 파일 버전·경로·SHA-256: 1.8.3.0 / C:\Program Files\uvnc\UltraVNC\winvnc.exe / 고정 기록과 같음
VNC Relay 호스트 OS·빌드 / Viewer OS / Server OS: 세 VM 모두 Windows 11 Enterprise Evaluation 10.0.26300(평가 기간 만료 상태). Ubuntu VNC Relay 미실행
격리 네트워크·방화벽 확인: vr-lab 스위치 Private, 세 VM 게이트웨이 없음. vr-repeater에 규칙 그룹 vnc-relay-manual-2.13(5901←192.0.2.10, 5500←192.0.2.30) 추가. Server의 기존 규칙 UltraVNC Server TCP는 원격 주소 Any였고 그대로 두었다
설정 사본 위치 / L·Lm·Lw·B·H·W·C·E·K: 3.1절 4단계 relay.toml 그대로 / 기본값
VNC 인증·입력 허용의 실제 설정 방법(UI를 썼으면 필드명): 2절 전제 조건에 기록. Mode II 역접속은 인증 없음 확인 창을 거쳤다(3.3절 앞 설명)
수동 회차별 시작·종료 시각 / 명령(비밀 제외): 아래 회차 표
프로세스별 relay_id_ref·session_id·conn_id와 사건 로그 파일:줄: 아래 회차 표. 줄 번호는 relay.jsonl 기준
화면·마우스·키보드 증거 위치: 저장소 밖 D:\vr-captures\manual-2.13\run-20261105\ (SHA256SUMS.txt 포함)
자동 시험 명령·OS·보고서 위치·대상 커밋: 이번 기록에서 다시 대조하지 않음

ID       결과(통과/실패/미실행, 부분이면 범위) / 관찰 / 증거
RLY-006: 부분. m1-c 1회 통과. 34단계 반복 미실행
RLY-010: 부분. m2s-b 1회 통과. 34단계 반복 미실행
RLY-011: 부분. m2v-a 1회 통과. 34단계 반복 미실행
AC-01:   부분. RLY-006 1회와 22~23단계 통과. 자동 증거 대조 미실행
AC-02:   부분. 두 접속 순서 각 1회 통과. 34단계 반복 미실행
AC-03:   부분. 24단계 통과. 25단계 중단. 자동 증거 대조 미실행
AC-04:   부분. 26단계 통과(아래 회차 표의 26단계 행). 25단계 중단. 자동 증거 대조 미실행
AC-06:   미실행. 수동 범위 밖이며 자동 증거 대조 미실행
AC-07:   부분. 11·16·21·24단계의 종료 로그 확인. 자동 증거 대조 미실행
AC-08:   미실행. 수동 범위 밖이며 자동 증거 대조 미실행
AC-15:   미실행. 27~30단계 미실행
AC-18:   미실행. 수동 범위 밖이며 자동 증거 대조 미실행
AC-26:   미실행. 31단계 미실행

확인 필요 항목과 미실행 이유: 25단계 작업이 VM 호출에서 멈췄고, 이어진 관리자 권한 요청을 사용자가 취소해 남은 단계를 멈췄다
중단 시각·직전 단계·오류: 20:42 KST, 25단계 시작. 21:17에 PowerShell Direct 세션을 만들지 못한 오류(Session 인수가 null)로 끝났다. 2026-11-06 05:34에 확인했을 때 세 VM이 모두 Off였다. 언제, 왜 꺼졌는지는 확인하지 못했다
복구 결과 / 증거 반출 위치 / 임시 파일 제거 여부: VM 쪽 복구 미실행(serve·방화벽 규칙 그룹·시험 비밀번호·계정 파일이 남아 있을 수 있음. 다음 실행 전에 baseline으로 되돌린다) / 위 증거 위치 / 호스트의 시험용 비밀번호 파일 제거
```

| 회차 | 단계·ID | 결과 | 로그 줄과 식별자 | 화면·입력 증거 |
| --- | --- | --- | --- | --- |
| `m1-c` | 7~11단계, RLY-006 | 통과 | 12~14줄. `c4`, `session_started` `mode1` 대상 `192.0.2.30:5900`, `ses_zctebxpeaasu4o6hbnq3xb7c3i`, `session_closed` `peer_eof` | `vr-viewer-viewer-m1-c-*.png`, `vr-server-m1-c-console.png`, `vr-server-form-m1-c.log`(클릭 1회, 입력 칸 초점, 문자열 `mode1c`) |
| `m2s-b` | 12~16단계, RLY-010 | 통과 | 20~24줄. Server `c7` `waiting_registered`(`relay_id_ref` `f87efe0db4756c86`) → Viewer `c8` → `ses_bkf2gcetupdljslb6mdefydnfq` → `peer_eof` | `vr-viewer-viewer-m2s-b-*.png`(인증 없음 확인 창 포함), `vr-server-form-m2s-b.log`(문자열 `mode2serverb`) |
| `m2v-a` | 17~21단계, RLY-011 | 통과 | 25~29줄. Viewer `c9` `waiting_registered`가 먼저 → Server `c10` → `ses_4l2eg6fyqfutso6nphug6zxdii` → `peer_eof` | `vr-viewer-viewer-m2v-a-*.png`, `vr-server-form-m2v-a.log`(문자열 `mode2viewera`) |
| 22단계 | 금지 포트 `192.0.2.30::5999` | 통과 | 30~31줄. `connection_rejected` `target_not_allowed` | `vr-repeater-rej-22-23.pcapng`: VNC Relay에서 대상 포트로 나간 패킷 0건 |
| 23단계 | 금지 IP `192.0.2.10::5900` | 통과 | 32~33줄. `target_not_allowed` | 같은 캡처 |
| 24단계 | Server `ID:1234`, Viewer `ID:5678` | 통과 | 34~39줄. 서로 다른 `relay_id_ref`(`f87efe0db4756c86`·`76282a50fa7822cb`)로 각각 대기, 매칭 없음. Viewer 종료 → `waiting_removed` `connection_error`, Server `-kill` → `waiting_removed` `peer_eof` | `viewer-r24.png`, `vr-server-r24-console.png` |
| 25단계 | 중복 ID | 중단 | 사건 없음 | 없음 |
| 26단계 | 정상 종료 뒤 같은 ID로 재실행 | 통과 | `m2s-b` 세션이 24줄 `session_closed` `peer_eof`로 정상 종료한 뒤, 두 끝점을 모두 새로 실행한 `m2v-a`가 25~29줄에서 같은 ID `1234`(같은 `relay_id_ref` `f87efe0db4756c86`)로 새 `ses_4l2eg6fyqfutso6nphug6zxdii`를 받았다. 이전 `session_closed`와 새 `session_started`가 모두 남아 있다. 별도 26단계 회차가 아니라 RLY-011 회차를 26단계의 조건에 대조한 결과다 | `m2v-a` 행의 화면·입력 증거 |

판정에서 뺀 회차도 기록한다. 3~5줄(`m1r1`)은 입력 스크립트 오류로 문자열을 보내지 못한 진단 회차다. 6~11줄(`m1-a`·`m1-b`)은 자동화가 Viewer 인증 창의 비밀번호 칸을 먼저 누른 회차다. 두 회차 모두 `session_closed` `connection_error`(`side=viewer`)로 끝났다. 칸을 누르지 않고 입력한 `m1-c`는 통과했다. 원인은 분석하지 않았다. 15~19줄(`m2s-a`)은 자동화가 인증 없음 확인 창을 닫아 Viewer가 연결을 거부한 회차다. 세션은 `close_timeout`으로 끝났다.

운영 로그 사본 `vr-repeater-relay.jsonl.lines-1-39.jsonl`은 VM의 `relay.jsonl` 파일을 직접 반출한 것이 아니다. 각 작업 출력에 남긴 1~39줄을 모은 것이다.

### 4.2 2026-11-06 실행 기록

결론: 남은 수동 단계 25·27~31·34단계를 모두 실행했고 모두 성공 조건과 맞았다. 4.1절과 합치면 RLY-006·RLY-010·RLY-011은 각각 독립 2회 통과했다(Windows VNC Relay). Ubuntu VNC Relay는 분리 실행 환경이 없어 2절 규칙대로 미실행이다. 프로젝트 소유자가 Q116에 따라 2026-11-06에 판정 캡처를 검토했고 문제가 없었다. 자동 증거 대조(32단계)는 [4.3절](#43-자동-증거-대조-32단계)에 있다.

```text
실행일·시각·시간대: 2026-11-06 05:34~05:49 KST(UTC+9). 로그 ts는 UTC 2026-11-05 20:38~20:48
실행자: 에이전트 자동 실행(Q116). 캡처 검토: 프로젝트 소유자, 2026-11-06 검토 완료, 문제 없음
대상 커밋·실행파일: 4.1절과 같은 ceb6a7c Windows debug 빌드(SHA-256 cc70349a…9f28)
VM 준비: 세 VM을 baseline으로 되돌린 뒤 05:35:29에 부팅했다. 파일 버전·해시·IP·게이트웨이 없음·방화벽 규칙은 4.1절과 같았다. 게스트 OS 조회값은 10.0.26300(build 26300)이지만 바탕 화면 워터마크에는 Build 26100이 보였다. 이 기록은 조회값을 따른다
VNC 인증 설정: 2절의 같은 화면에서 VNC Password만 바꾸었다. Enter 키 저장 한 번으로 passwd 줄이 baseline 값에서 바뀌었다. View-Only Password는 바꾸지 않았다
설정 사본: relay-27·28·29v·29s·30v·30s.toml. 원본에서 allow 한 줄과 log.output만 바꾸었고, 각 사본은 자기 로그 파일에 썼다
증거 위치: 저장소 밖 D:\vr-captures\manual-2.13\run-20261106\. 파일 77개이고 SHA256SUMS.txt에 77행이 있다. 같은 캡처를 VM에서 바로 받은 사본과 Fetch 사본이 함께 있어 서로 다른 내용은 60개다(예: viewer-r25-dup.png와 vr-viewer-viewer-r25-dup.png). 운영 로그는 VM의 파일을 직접 반출했다(vr-repeater-relay*.jsonl, serve-stderr-*.txt, serve-exit-*.txt). 정상 종료 회차의 serve-exit는 exit=0, stderr는 비어 있다
복구 결과 / 임시 파일 제거: VNC Relay 프로세스 종료(매 회차 server_stopped가 마지막 줄), 방화벽 규칙 그룹 vnc-relay-manual-2.13 제거(남은 규칙 0개), Viewer·Server 종료, 세 VM 종료. VM 안 비밀 파일 0개를 확인했다. 호스트의 시험용 비밀번호 파일을 지웠다. 계정 파일은 반출하지 않았다
```

| 회차 | 단계·ID | 결과 | 로그 파일:줄과 식별자 | 화면·입력 증거 |
| --- | --- | --- | --- | --- |
| `r25` | 25단계 | 통과 | `relay.jsonl` 3~9줄. Viewer A `c1` `waiting_registered`(`relay_id_ref` `3bb6e14cba08770c`) → 같은 ID의 Viewer B `c2`는 `connection_rejected` `stage=register` `reason=duplicate_id`, `c2`의 `waiting_registered` 없음 → B 종료 뒤 Server `c3`가 A와 매칭(`ses_fg7mku5lbwj6pwldcagudgpdh4`, `waited_ms` 22846, A가 처음부터 대기를 유지함) → `peer_eof` | `viewer-r25-dup.png`(B에 `Connection failed - End of Stream`), `vr-server-form-r25.log`(문자열 `dupidfirst`) |
| `m1-d` | 34단계, RLY-006 2회차 | 통과 | 10~12줄. `c4`, `mode1` 대상 `192.0.2.30:5900`, `ses_v6vmhwacvqfajc233ajelt7pzm`, `peer_eof` | `vr-viewer-viewer-m1-d-*.png`, `vr-server-form-m1-d.log`(문자열 `modeonerepeat`) |
| `m2s-c` | 34단계, RLY-010 2회차 | 통과 | 13~17줄. Server `c5` 대기 → Viewer `c6` → `ses_ehv4vvp6nx5v76nfa5p53b7ms4` → `peer_eof` | `vr-viewer-viewer-m2s-c-*.png`(인증 없음 확인 창 포함), `vr-server-form-m2s-c.log`(문자열 `serverfirstrepeat`) |
| `m2v-b` | 34단계, RLY-011 2회차. 26단계 2회차 | 통과 | 18~22줄. Viewer `c7` 대기가 먼저 → Server `c8` → `ses_e2t3sr25flg37mqlhhjjqzhxba` → `peer_eof`. `m2s-c`가 17줄에서 정상 종료한 뒤 두 끝점을 모두 새로 실행했고, 같은 `relay_id_ref`로 새 `session_id`를 받았다 | `vr-viewer-viewer-m2v-b-*.png`, `vr-server-form-m2v-b.log`(문자열 `viewerfirstrepeat`) |
| `k31` | 31단계 | 통과 | 23~25줄. Mode I `ses_2r3xe55gse7jazv7gyt4k3y27u`. 입력 없음 05:43:05.008~05:45:23.303(138초, K = 60 + 10 × 6 = 120초). 그 사이 사건 없음. 유휴 뒤 클릭·문자열 전달 후 `peer_eof`(`duration_ms` 167686) | `viewer-k31-idle-start.png`, `vr-viewer-viewer-k31-in-*.png`, `vr-server-form-k31.log`(05:45:26 클릭, 문자열 `afteridle`) |
| `27` | 27단계 | 통과 | `relay-27.jsonl` 3줄 `connection_rejected` `listener=tcp_viewer` `stage=accept` `reason=ip_not_allowed` `peer_ip=192.0.2.10`. 이 접속의 `connection_accepted`·대기·세션 없음. 같은 프로세스에서 Server 등록은 허용(4~6줄) | `viewer-r27.png`(`Proxy Connection failed - Error reading Protocol Version`) |
| `28` | 28단계 | 통과 | `relay-28.jsonl` 3~5줄 `listener=tcp_server` `ip_not_allowed`(Server가 3번 다시 접속). 같은 프로세스에서 Viewer 등록은 허용(6~7줄). 8줄의 Viewer `waiting_removed` `connection_error`는 자동화가 Viewer 프로세스를 강제 종료한 결과이며 28단계 판정과 무관하다 | 로그 |
| `29v`·`29s` | 29단계 | 통과 | 종료 코드 3, stderr `error[config_missing_key] key=relay.tcp_viewer.allow`(29v)·`key=relay.tcp_server.allow`(29s). 로그 파일이 생기지 않아 `server_ready`가 없다 | `vr-repeater-serve-stderr-29*.txt`, `vr-repeater-serve-exit-29*.txt` |
| `30v`·`30s` | 30단계 | 통과 | `relay-30v.jsonl` 2줄 `allow_all_source` `level=warn` `listener=tcp_viewer` `item=0.0.0.0/0` `phase=startup`, 4~5줄 Viewer 수락·대기. `relay-30s.jsonl`은 같은 경고의 `listener=tcp_server`와 Server 수락·대기(2·4~5줄) | 로그 |

31단계 관찰: 화면에 시계 등 바뀌는 영역이 있어 유휴 동안 RFB 화면 갱신 바이트가 오갔을 수 있다. VNC Relay 로그는 시간별 바이트를 남기지 않으므로 애플리케이션 데이터가 없었는지는 판정하지 않는다. 이 부분은 RLY-027 자동 증거로 확인한다.

이 실행에서 계약과 다른 점을 하나 찾았다. `server_starting`의 `log_contract`가 `ops-info/1.2.0`이었다. [운영 정보 계약 1.5절](../contracts/ops-info-v1.md#15-사건-목록)은 `ops-info/1.3.0`을 요구한다. 계약은 사용자 결정 2026-11-01(Q81)로 1.3.0이 되었지만 상수 `LOG_CONTRACT`와 시험의 기대값이 1.2.0으로 남아 있었다. 코드와 시험을 1.3.0으로 고쳤다. 위 회차의 실행파일은 고치기 전 빌드다.

### 4.3 자동 증거 대조 (32단계)

두 OS의 전체 게이트를 `log_contract` 수정을 넣은 작업 트리에서 실행했다. 그 코드는 커밋 `13129d1`과 같다(게이트 뒤 문서만 따로 커밋했다). 수동 회차의 실행파일(`ceb6a7c`)과 자동 증거의 기준 커밋은 다르다. 두 커밋 사이의 코드 차이는 `log_contract` 문자열뿐이다. 두 게이트 모두 판정 `통과`였다. 게이트 전체 판정과 아래 ID별 판정은 따로 본다.

- Windows 11 Pro 10.0.26200: `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all`, 마지막 줄 `ALL PASSED (all)`. 결과는 `시나리오 153건: 통과 134, 부분 14, 실패 0, 미실행 1, 미연결 4. 오류 0건.`
- Ubuntu 24.04.4(WSL2): `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh`, 마지막 줄 `ALL PASSED`. 결과는 `시나리오 153건: 통과 138, 부분 9, 실패 0, 미실행 2, 미연결 4. 오류 0건.`

| ID | 수동 증거(4.1·4.2절) | 자동 증거(Windows / Linux 보고서) | 결과(TCP 부분) |
| --- | --- | --- | --- |
| RLY-006 | 통과, 독립 2회(`m1-c`·`m1-d`) | 부분 / 부분. 실제 화면·입력은 시험 코드로 확인하지 않는다 | 통과(Windows VNC Relay). 소유자 캡처 검토 완료(2026-11-06). Ubuntu VNC Relay 미실행 |
| RLY-010 | 통과, 독립 2회(`m2s-b`·`m2s-c`) | 부분 / 부분. 같은 이유 | 통과(같은 조건) |
| RLY-011 | 통과, 독립 2회(`m2v-a`·`m2v-b`) | 부분 / 부분. 같은 이유 | 통과(같은 조건) |
| AC-01 | RLY-006, 22~23단계 통과 | RLY-007·008 통과 / 통과. RLY-009 부분 / 통과 | 통과. RLY-009의 Windows 부분은 TODO 2.8에서 받아들인 OS별 범위다 |
| AC-02 | RLY-010·011 각 2회 통과 | 위와 같음 | 통과 |
| AC-03 | 24·25단계 통과 | RLY-012~014 통과 / 통과 | 통과 |
| AC-04 | 25·26단계 통과 | RLY-015·016 통과 / 통과 | 통과 |
| AC-06 | 수동 범위 밖 | RLY-002·003·017 통과 / 통과 | 통과 |
| AC-07 | 11·16·21·24단계 종료 로그 | RLY-004·005·014·019·020, OPS-006 통과 / 통과. RLY-009 부분 / 통과 | 통과. OS 범위는 AC-01과 같다 |
| AC-08 | 수동 범위 밖 | RLY-018·021, OPS-005 통과 / 통과. OPS-004 부분 / 부분. OPS-017 미연결 / 미연결 | 미실행. OPS-017의 판정 구간이 미확정이고 시험이 없다. 작업 5.4로 옮겼다(사용자 결정 2026-11-06(Q117)) |
| AC-15 | 27~30단계 통과 | RLY-023·024, OPS-037·038 통과 / 통과 | 통과 |
| AC-18 | 수동 범위 밖 | OPS-033 통과 / 통과 | 통과 |
| AC-26 | 31단계 통과 | RLY-027 통과 / 통과. RLY-025·026 부분 / 통과 | 통과. RLY-025·026의 Windows 부분은 사용자 결정 2026-11-02(Q86)의 범위다 |

## 5. 중단 조건

- 인터넷·운영 네트워크에 연결되어 있거나 시험 대상이 아닌 화면이 보이면 바로 중단한다.
- 버전이 1.8.3.0이 아니거나 실행한 파일 경로·해시를 식별할 수 없으면 다음 회차로 진행하지 않는다.
- Server의 인증·입력 허용 설정이 불명확하면 화면·입력 성공을 추정하지 않는다. 실제 필드명과 설정을 먼저 확인한다.
- `server_ready`가 없거나 `startup_failed`·`log_write_failed`가 나오면 중계 시험을 하지 않는다. 오류와 stderr를 보존한다.
- 금지 IP·포트가 연결되거나 다른 ID가 매칭되면 그 회차를 실패로 기록하고 추가 입력을 멈춘다.
- 기존 세션이 오류 시험 때문에 끊기거나 같은 연결이 두 세션에 나타나면 자동 재시도하지 않는다.
- 최대 대기 시간을 넘기거나 종료 정리가 끝나지 않으면 직전 사건·실제 대기 시간을 기록한다. keepalive 동작과 W·H·C·E 만료를 혼동하지 않는다.
- 로그·스크린샷·캡처에 비밀번호·VNC 인증 바이트·실제 사용자 데이터가 들어 있으면 공유하지 않는다. 비밀을 제외한 증거 사본을 만든다.
- 자동 시나리오가 부분·미실행·미연결이면 해당 범위는 완료로 처리하지 않는다. 도구 부족을 허용 목록 확대나 방화벽 전체 해제로 우회하지 않는다.

## 6. 롤백 또는 복구

1. 시험용 Viewer와 Server를 종료한다.

   기대 결과: 대기·활성 연결이 정리되고 관련 종료 사건이 남는다.

2. `serve` 터미널에서 Ctrl+C로 정상 종료를 요청한다.

   기대 결과: 프로세스가 끝나고 `server_stopped`가 마지막 사건으로 남는다. 유예 중 두 번째 종료 요청은 강제 정리로 이어질 수 있다. 콘솔 창 닫기나 작업 관리자의 프로세스 종료를 정상 종료 대신 쓰지 않는다.

3. 운영 로그·stderr·기록·가린 스크린샷을 저장소 밖 증거 디렉터리로 복사한다.

   기대 결과: 회차·커밋·OS와 연결된 증거를 보존한다. 원시 캡처는 인증·입력 데이터를 포함할 수 있으므로 제한된 위치에 따로 둔다.

4. 이번 시험에서 바꾼 Server 설정과 추가한 방화벽 규칙만 원래 상태로 되돌린다.

   기대 결과: 작업 전 기록과 설정이 같다. 기존 UltraVNC 설치나 다른 작업자의 파일·규칙은 제거하지 않는다.

5. 증거 반출을 마친 뒤 이번 시험에서만 만든 설정 사본·계정 파일·실행파일·임시 화면 파일을 제거한다.

   영향: 시험용 계정과 파일을 복구할 수 없다. 원본 증거의 보존 위치를 먼저 확인한다. VM 체크포인트 복원을 선택하면 그 뒤 만든 다른 파일도 사라지므로 이 시험만의 변경인지 먼저 대조한다.

   기대 결과: 시험용 비밀·계정 해시가 작업 디렉터리나 저장소에 남지 않는다. 보존하기로 한 로그와 기록은 증거 디렉터리에 남는다.

## 7. 해결되지 않을 때 전달할 대상

- 전달 대상: 이 저장소를 작업하는 에이전트다. 사용자가 직접 실행한 결과와 절차 작성만 한 상태를 구분해 전달한다.
- 전달할 기록: 4절 양식, ID별 실제 결과, 대상 커밋·미커밋 범위, 각 호스트 OS, 실행파일 버전·경로·해시, 설정 사본의 비밀 없는 내용이다.
- 전달할 로그: 실패 전후 JSON Lines와 stderr, 시각, `conn_id`·`session_id`·`relay_id_ref`, 해당 파일과 줄 번호다. 원문 중계 ID를 로그에 덧붙이거나 숨긴 값을 복원하지 않는다. 재시작별로 로그를 구분한다.
- 전달할 화면: 화면 수신·마우스 선택·키보드 입력을 구분한 스크린샷이다. 비밀번호 입력 창, 실제 계정·사용자 데이터, UI에 보이는 원문 중계 ID는 가린다.
- 전달할 자동 증거: `scenario-report.md`와 해당 시험 출력·OS·커밋이다. Q86의 무응답 단절 시험이 아직 없으면 AC-26의 그 부분을 미실행으로 적는다.
- 확인 필요 항목: 실행 호스트의 실제 버전·경로·방화벽 상태, Server의 인증·입력 허용 설정과 UI 필드명, UI를 선택했을 때의 Repeater·대상·ID 필드명이다. 실행자가 확인한 값과 근거를 기록한다. 이 절차는 확인하지 않은 값을 채우지 않는다.
- 원시 pcap·VNC 인증·원시 페이로드·비밀번호·계정 해시·개인키는 대화나 저장소에 올리지 않는다. 원본 캡처가 필요하면 소유자가 보관 위치와 접근 권한을 먼저 정한다.
