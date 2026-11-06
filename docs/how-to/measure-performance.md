# 중계 성능과 잔여 자원을 측정하는 방법

`tests/performance.rs`로 실제 실행파일의 OPS-017·OPS-018과 AC-08 느린 수신자 부하를 측정한다. Windows와 WSL Ubuntu에서 각각 실행한다. 짧은 스모크 실행은 측정 도구의 동작을 확인할 뿐, 성능 합격 증거가 아니다.

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 개발·검증 담당자 |
| 검증 상태 | Draft. 실제 실행 기록은 4절에 별도로 남긴다. |
| 검증 환경·버전 | Windows 11 x64·WSL2 Ubuntu 24.04 LTS x64, 저장소의 Rust 도구 체인 |
| 최종 검증일 | 2026-11-06. 두 OS에서 정식 측정을 실행했다. OPS-018은 통과, OPS-017은 RSS 잔여 기준 실패다(4절). |
| 관련 기준 문서 | [운영 스펙 6절](../specs/operations.md#6-비기능호환성출처-수용-조건), [운영 정보 계약 1.5절](../contracts/ops-info-v1.md#15-사건-목록), [WebSocket 바이트 스트림 계약](../contracts/websocket-relay-v1.md#14-데이터-메시지와-제어-프레임), [설정 계약](../contracts/config-v2.md) |

## 1. 대상과 영향

- 대상은 시험이 새로 띄우는 `vnc-relay serve` 프로세스다. 운영 서버에 연결하지 않는다.
- 같은 PC의 루프백에서 Viewer TCP·WS·WSS를 차례로 측정한다. Server는 항상 TCP다. Mode II 세션마다 서로 다른 숫자 ID를 쓴다.
- CPU·메모리·루프백 대역폭을 사용한다. 다른 부하를 멈추고, 두 OS 시험을 동시에 실행하지 않는다.
- 시험마다 임시 포트를 예약하고, 설정·계정 파일을 만든다. 시험이 끝나면 자식 프로세스를 종료한다. 결과와 로그는 보존한다.
- 기본 설정은 `L=1000`, `Lw=200`, `B=131072`, `M=1048576`이다. WS ping·pong 제한은 각각 300초다. 느린 수신자 60초 구간에서 ping 만료가 부하를 중단하지 않도록 한다. 정상 수신자는 ping마다 pong을 보낸다.

## 2. 전제 조건

- [개발 환경 준비](set-up-and-verify.md)를 마친다. Windows의 MSVC 링커가 실행 가능해야 한다.
- release 빌드를 쓴다. debug 빌드의 수치는 합격 판정에 쓰지 않는다.
- Windows는 `powershell.exe`의 `Get-Process -Id <pid>` 실행을 허용해야 한다. RSS는 `WorkingSet64`, CPU는 `TotalProcessorTime`, 핸들 수는 `HandleCount`다. RSS라는 결과 필드 이름은 두 OS에서 같지만 Windows 값은 작업 집합 크기다.
- Linux는 시험 사용자가 `/proc/<pid>/status`, `/proc/<pid>/stat`, `/proc/<pid>/fd`를 읽을 수 있어야 한다. RSS는 `VmRSS`, CPU는 `utime + stime`, 자원 수는 fd 디렉터리 항목 수다. Linux x64의 `CLK_TCK=100`을 가정한다. 실행 전 `getconf CLK_TCK`가 100인지 확인한다. 다르면 이 도구로 CPU 합격을 판정하지 않는다.
- WSS 클라이언트는 저장소의 시험 CA만 신뢰한다. `crates/adapters/tests/fixtures/tls/`의 인증서와 개인키를 쓴다. 실제 운영 키는 필요 없다.
- OS 버전, CPU 모델, 논리 코어 수, RAM, WSL 버전·커널, Rust 버전과 대상 커밋·미커밋 범위를 기록한다. JSON의 `available_parallelism`은 하드웨어 기록을 대신하지 않는다.

## 3. 절차

실행 위치는 저장소 루트다. 각 테스트는 세 전송을 순차 실행하고, 각 전송마다 새 서버를 쓴다. 두 테스트도 `--test-threads=1`로 순차 실행한다. 기본 OPS-018은 직접 연결 기준선까지 네 구간을 측정하므로 약 41분 이상 걸린다. OPS-017은 전송마다 60초 부하와 15,000회 수명을 실행한다. OS 상태에 따라 더 오래 걸릴 수 있다. 중단은 `Ctrl+C`로 한다.

### 3.1 Windows PowerShell

1. release 빌드를 만든다.

   ```powershell
   $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
   $env:AWS_LC_SYS_PREBUILT_NASM = '1'
   cargo build --release
   ```

   기대 결과: 빌드 종료 코드 0. NASM을 설치했다면 `AWS_LC_SYS_PREBUILT_NASM`은 생략할 수 있다.

2. 결과 디렉터리를 정하고 정식 측정을 실행한다.

   ```powershell
   $env:VNC_RELAY_PERF_OUT = Join-Path $PWD 'target\perf-windows'
   Remove-Item Env:VNC_RELAY_PERF_SECS -ErrorAction SilentlyContinue
   cargo test --release --test performance -- --ignored --test-threads=1
   ```

   기대 결과: 테스트 2건 통과, 전송별 JSON의 모든 `criteria[].pass`가 `true`다.

3. 측정 도구만 짧게 확인하려면 별도 디렉터리에 스모크를 실행한다.

   ```powershell
   $env:VNC_RELAY_PERF_OUT = Join-Path $PWD 'target\perf-windows-smoke'
   $env:VNC_RELAY_PERF_SECS = '20'
   cargo test --release --test performance -- --ignored --test-threads=1
   Remove-Item Env:VNC_RELAY_PERF_SECS
   ```

   기대 결과: OPS-018 JSON에서 `judgeable_duration` 기준의 `pass`가 `false`이며, 전체 `judgeable=false`, `pass=false`다. 기간만 짧다는 이유로 패닉하지 않는다. 수치 기준이 모두 통과하면 `measurements_pass=true`이며 Rust 테스트는 통과할 수 있다. 이를 성능 합격으로 보고하면 안 된다. 다른 기준의 실패는 테스트 실패로 드러난다. OPS-017의 60초 부하와 수명 반복은 짧아지지 않는다.

### 3.2 WSL Ubuntu bash

1. Windows와 다른 대상 디렉터리로 release 빌드를 만든다.

   ```bash
   export CARGO_TARGET_DIR="$HOME/vnc-relay-target"
   getconf CLK_TCK
   cargo build --release
   ```

   기대 결과: `CLK_TCK` 출력 100, 빌드 종료 코드 0.

2. 결과 디렉터리를 정하고 정식 측정을 실행한다.

   ```bash
   export VNC_RELAY_PERF_OUT="$CARGO_TARGET_DIR/perf-ubuntu"
   unset VNC_RELAY_PERF_SECS
   cargo test --release --test performance -- --ignored --test-threads=1
   ```

   기대 결과: 테스트 2건 통과, 전송별 JSON의 모든 `criteria[].pass`가 `true`다.

3. 측정 도구만 확인하려면 스모크를 실행한다.

   ```bash
   VNC_RELAY_PERF_OUT="$CARGO_TARGET_DIR/perf-ubuntu-smoke" \
   VNC_RELAY_PERF_SECS=20 \
   cargo test --release --test performance -- --ignored --test-threads=1
   ```

   기대 결과: Windows 스모크와 같다. Windows에서 시작할 때는 `wsl -d Ubuntu -- bash -lc 'cd /mnt/d/Workspace/vnc-relay && export PATH="$HOME/.cargo/bin:$PATH" CARGO_TARGET_DIR="$HOME/vnc-relay-target" && VNC_RELAY_PERF_OUT="$CARGO_TARGET_DIR/perf-ubuntu-smoke" VNC_RELAY_PERF_SECS=20 cargo test --release --test performance -- --ignored --test-threads=1'`를 쓸 수 있다.

### 3.3 측정값과 판정

공통 전송 단위는 4 KiB다. 단위마다 세션 번호·방향·순번·같은 시험 프로세스의 단조 시계 송신 시각을 넣는다. 나머지 바이트는 세션·방향·순번에서 결정한다. 수신자는 바이트 스트림을 다시 4 KiB씩 묶어 비교한다. WS 메시지 경계가 TCP 읽기 경계와 같다고 가정하지 않는다. 유실·중복·순서 바뀜·다른 세션의 데이터가 들어오면 오류다. 송신을 멈춘 뒤 송신 완료 바이트가 모두 수신될 때까지 최대 120초 소진한다. 소진 실패와 I/O 오류도 데이터 오류에 포함한다.

OPS-018은 직접 TCP 루프백 100쌍을 먼저 측정한다. 그 뒤 TCP·WS·WSS 중계 경로를 측정한다. 모두 10초 예열 뒤 같은 길이로 측정하며, 세션마다 양방향 각 125,000 B/s로 보낸다. 지연은 송신 시각부터 완전한 단위를 수신한 시각까지다. 시계는 같은 프로세스 안에서 공유한다. 측정 시작 전에 보낸 단위는 지연 통계에서 뺀다. CPU·RSS는 시작·종료 시점과 5초 간격으로 표본을 얻는다.

| 테스트 | 기준 | 근거·측정 방법 |
| --- | --- | --- |
| OPS-018 | 측정 600초 이상 | Q98. `VNC_RELAY_PERF_SECS` 기본 600. 양의 정수만 허용하며 600 미만은 `judgeable=false`, 기간 기준 실패, 전체 결과 `pass=false`다. 기간만 짧으면 테스트를 패닉시키지 않지만, 수치 기준은 모두 단언한다. 짧은 실행을 합격으로 보고하지 않는다. |
| OPS-018 | 100세션의 양방향 각 1 Mbit/s 전송 유지 | Q98. 각 방향의 구간 송신 바이트가 `floor(기간 × 125000 / 4096) × 4096 − 4096` 이상인지 확인한다. 정해진 속도로 단위를 보내며, 구간 경계의 한 단위만 허용하는 계산은 이 문서의 판단, 사용자 결정 없음이다. 직접 연결 기준선에도 같은 기준을 쓴다. |
| OPS-018 | 데이터 오류 0 | Q98. 직접 연결 기준선과 중계 경로를 모두 확인하며, 소진 뒤 송수신 바이트도 비교한다. |
| OPS-018 | 늘어난 지연 p99 ≤ 10 ms | Q98. `added_p99 = p99_relay − p99_direct`. 음수도 그대로 기록한다. |
| OPS-018 | 서버 평균 CPU ≤ 1코어 | Q98. 표본의 CPU 시간 차이를 실제 벽시계 시간 차이로 나눈다. |
| OPS-018 | 서버 RSS 최댓값 ≤ 256 MiB | Q98. 측정 구간의 모든 자원 표본에서 최댓값을 고른다. |
| OPS-018 | RSS 증가 ≤ 10% | Q98. 구간 최대 RSS ≤ 시작 RSS × 1.10으로 판정한다. 이 보수적 해석은 이 문서의 판단, 사용자 결정 없음이다. |
| OPS-017·AC-08 | 50세션 양방향 수신 중지, 나머지 50세션 정상 전송, 60초 유지 | Q120. 느린 쪽은 속도 제한 없이 계속 쓰고, 각 방향의 쓰기가 250 ms 이상 기다린 뒤 60초 유지를 시작한다. 모두 막히지 않으면 실패다. 정상 쪽은 양방향 각 125,000 B/s다. 중계에 연결하는 모든 시험 클라이언트 소켓은 송수신 버퍼를 64 KiB로 둔다(Q121). |
| OPS-017 | 최대 RSS − 유휴 RSS ≤ 200 × (B + 64 KiB) | Q118. `server_ready` 뒤 5초 유휴 상태에서 기준 RSS를 얻는다. 200은 100세션의 양쪽 중계 연결 수다. |
| OPS-017·AC-08 | 정상 스트림 오류 0, 양방향 각 1 Mbit/s 유지 | Q120. 60초 구간에서 각 정상 방향의 송신 바이트가 `floor(60 × 125000 / 4096) × 4096 − 4096` 이상이고, 상대의 수신 바이트가 그 값에서 4096을 더 뺀 값 이상이어야 한다. 송신 쪽 한 단위는 구간 경계, 수신 쪽 추가 한 단위는 전송 중인 바이트로 허용한다. 이 허용 폭은 이 문서의 판단, 사용자 결정 없음이다. |
| OPS-017 | 느린 스트림도 수신 재개·소진 뒤 오류 0 | Q120의 부하에서 데이터 무결성을 확인한다. WS·WSS의 느린 Viewer는 크기가 정확히 M인 바이너리 메시지를 4 KiB씩 스트리밍한다. M 크기 부하에도 Q118의 메모리 한계를 그대로 적용한다. |
| OPS-017 | 라운드마다 대기·활성 0, 2~15라운드 핸들/fd 수 = 1라운드 값 | Q119·Q122. 1,000회 × 15라운드(Q123), 동시 수명 최대 50개다. 각 수명은 양방향 1 KiB를 비교한 뒤 양쪽을 닫는다. 로그가 정리를 확인한 뒤 5초 기다려 표본을 얻는다. 1라운드 값과의 엄격한 동등 비교를 완화하지 않는다. 유휴 기준값은 JSON `idle_baseline`에 기록만 한다. |
| OPS-017 | 15라운드 RSS − 10라운드 RSS ≤ 2 MiB | Q123. 앞 라운드는 할당기의 계단식 증가 구간으로 보고 비교에서 뺀다. |

직접 연결 기준선은 세 전송 모두 평문 TCP다. WS 프레이밍·WSS 처리의 클라이언트 비용도 중계 경로의 지연에 포함된다. 직접 연결 p99와 중계 p99를 빼는 방법, 기준선을 한 번만 실행하는 선택은 **이 문서의 판단, 사용자 결정 없음**이다. 이 차이는 개별 패킷의 지연 차이를 모아 구한 p99와 같지 않다. p50·p99는 10 µs 너비의 히스토그램 상한이며, 1초 초과 구간은 최댓값으로 보수적으로 계산한다. 최대 지연은 원래 표본값이다. 이 집계 방법도 **이 문서의 판단, 사용자 결정 없음**이다.

250 ms 쓰기 대기를 역압의 관찰값으로 쓰는 선택은 **이 문서의 판단, 사용자 결정 없음**이다. OS 소켓 내부와 서버의 B 버퍼를 직접 들여다보지는 않는다. 두 수신자를 멈추고 쓰기가 막힐 때까지 보내서 버퍼를 채운다. 수신을 재개할 때 이미 시작한 M 메시지는 끝까지 보낸 뒤 중지한다.

시험 클라이언트 버퍼를 64 KiB로 두는 이유는 루프백 측정의 커널 메모리 때문이다. 클라이언트와 중계 소켓이 같은 커널의 `tcp_mem`을 나눠 쓴다. 클라이언트 버퍼를 OS 자동 조정에 맡기면 WSL2 Ubuntu에서 상한을 넘었다. 그 결과 패킷 폐기와 재전송 백오프로 느린 흐름이 소진 기한 120초 안에 끝나지 않았다. 중계 소켓은 자동 조정을 유지한다(Q121, [운영 스펙 6절](../specs/operations.md#6-비기능호환성출처-수용-조건)). Linux에서는 측정 전후 `nstat`의 `TcpExtTCPMemoryPressures`·`TcpExtTCPRcvQDrop`를 기록할 수 있다.

로그 집계는 `waiting_registered`로 대기 집합에 넣고 `waiting_removed`·`waiting_expired`로 뺀다. `session_started`는 양쪽 연결을 대기 집합에서 빼고 세션을 활성 집합에 넣는다. `session_closed`는 활성 집합에서 뺀다. 수명 라운드마다 누적 시작·종료 수가 예상값과 같고 두 집합이 비어 있어야 한다. 중간에 쓰는 마지막 불완전 로그 줄은 집계하지 않는다.

### 3.4 결과 파일

`VNC_RELAY_PERF_OUT`을 생략하면 `CARGO_TARGET_TMPDIR/perf`에 쓴다. 각 전송은 `ops017-tcp_viewer.json`, `ops018-wss_viewer.json`과 같은 이름으로 저장한다. 같은 디렉터리에서 다시 실행하면 해당 JSON을 덮어쓴다. 원본을 보존하려면 실행마다 다른 디렉터리를 정한다.

JSON에는 OS·아키텍처·전송·L/Lw/B/M, 실제 요청 기간, 원시 자원 표본, 흐름별 송수신 바이트·오류·역압, 지연 요약, 직접 연결 기준선, 기준별 관찰값·한계·통과 여부를 넣는다. OPS-017은 라운드별 로그 집계와 자원 표본도 넣는다. 개별 서버 디렉터리에는 `relay.toml`, 시험 계정 파일, `relay.log`, `stdout.log`, `stderr.log`를 남긴다. 서로 다른 전송의 기준 실패는 마지막에 모아서 단언하므로 한 전송의 수치 실패 때문에 다음 전송을 생략하지 않는다. 시작·프로토콜·로그 정리 자체가 실패하면 해당 테스트가 즉시 실패할 수 있다.

## 4. 검증 기록

[검증 기록 규칙](../AGENTS.md#검증-기록)에 따라 실제 실행 뒤 채운다. 스모크와 정식 측정을 구분한다. 대상 커밋·미커밋 범위, 하드웨어, 환경, 명령, 종료 코드, 전송별 관찰값·실패 기준, JSON·로그 경로를 함께 남긴다.

| 대상·환경·하드웨어 | 명령·측정 기간 | 실제 결과 | 실행 증거 위치 |
| --- | --- | --- | --- |
| 스모크, 작업 트리(`1f4fe57` + 측정 도구, `TCP_NODELAY` 전). WSL2 Ubuntu 24.04, 커널 6.6.87.2-microsoft-standard-WSL2 | `measures_throughput_latency_cpu_and_rss`, 20초. 판정 불가 길이 | WS·WSS의 늘어난 p99가 34,040 µs·33,970 µs로 10 ms를 넘었다. WS 머리 묶기만 적용해도 34,040 µs·33,750 µs였다. 원인은 중계 소켓의 Nagle이었다. | `~/vnc-relay-target/perf-ubuntu-smoke-complete/`, `perf-ubuntu-vectored/` |
| 스모크, 작업 트리(`TCP_NODELAY` 뒤). WSL2 Ubuntu 같음, Windows 11 Pro 10.0.26200 | 같은 시험, 20초. 판정 불가 길이 | 늘어난 p99: Ubuntu TCP 230 µs, WS 690 µs, WSS 340 µs. Windows TCP 600 µs, WS 1,250 µs, WSS 2,500 µs. Windows WSS CPU는 1.27코어에서 0.75코어로 줄었다. 데이터 오류 0. | `~/vnc-relay-target/perf-ubuntu-nodelay/`, `target/perf-win-nodelay-vectored/` |
| 진단, WSL2 Ubuntu. `tcp_mem` = 190395 253863 380790페이지 | `D:/vr-tools/tmp/perf-diag/drain.py`(100세션 양방향 무제한 쓰기, 30초 정지 뒤 수신 재개)와 `nstat` | 클라이언트 버퍼 자동 조정: 소켓 메모리 최대 381,593페이지로 상한 초과, `TcpExtTCPRcvQDrop` 5,262, `TcpExtTCPTimeouts` 3,243, 멈춘 흐름 78개. Q120 규모(50세션)와 클라이언트 64 KiB: 최대 233,317페이지, 멈춘 흐름 0. 이 결과로 Q121을 정했다. | 대화 기록. 결과 파일 없음 |
| 정식, 커밋 `e75eb25`. Windows 11 Pro 10.0.26200, AMD Ryzen 7 5700X 8코어 16스레드, RAM 31.9 GB, Rust 1.99.1 | `cargo test --release --test performance -- --ignored --test-threads=1`, 기본 600초. 종료 코드 101(OPS-017 실패) | OPS-018 세 전송 통과: 늘어난 p99 TCP 740 µs, WS 1,590 µs, WSS 2,730 µs. CPU 0.37·0.50·0.67코어. 최대 RSS 41.5·42.4·47.2 MB. OPS-017: TCP·WSS 통과, WS 실패. WS는 15라운드 RSS − 10라운드 RSS가 7,221,248 B로 2 MiB를 넘었다. 작업 집합이 라운드마다 약 14.5 MB와 22.4 MB 두 수준을 오갔다. 나머지 기준(오버헤드, 정상 흐름 100, 소진 오류 0, 핸들 212 고정, 수명 오류 0)은 세 전송 모두 통과했다. | `target/perf-windows-formal-e75eb25/` |
| 정식, 커밋 `e75eb25`. WSL2 Ubuntu 24.04, 같은 하드웨어, `CLK_TCK=100` | WSL 3.2절 절차, 600초. OPS-017 종료 코드 101 | OPS-018 세 전송 통과: 늘어난 p99 TCP 210 µs, WS 750 µs, WSS 420 µs. CPU 0.13·0.21·0.26코어. 최대 RSS 34.4·35.3·37.3 MB. OPS-017: TCP 통과, WS·WSS 실패. 15라운드 RSS − 10라운드 RSS가 WS 3,932,160 B, WSS 2,359,296 B였다. 라운드별 증가 폭은 줄었지만 끝까지 0이 되지 않았다. 나머지 기준은 세 전송 모두 통과했다(fd 20 고정). | `~/vnc-relay-target/perf-ubuntu-formal-e75eb25/` |

## 5. 중단 조건

- 운영 서비스와 자원을 다투거나 시스템 메모리가 부족하면 중단한다.
- CPU·RSS·핸들 표본을 얻지 못하면 수치를 추측하지 않는다. 진단을 보존하고 수집 권한·도구를 확인한다.
- 기준이 실패하면 값을 낮추거나 허용 오버헤드를 늘리지 않는다. 원시 수치와 환경을 먼저 기록한다.

## 6. 롤백 또는 복구

1. 중단 뒤 남은 시험 자식 프로세스가 있으면 결과 디렉터리와 실행 경로로 대상을 확인한다.
2. 확인한 시험 프로세스만 종료한다. 이름만 보고 다른 `vnc-relay` 프로세스를 함께 종료하지 않는다.
3. 결과와 로그를 보존한다. 측정 파일은 제품 설정을 바꾸지 않으므로 제품 롤백은 필요 없다.

## 7. 해결되지 않을 때 전달할 대상

- 담당 개발자에게 실행 명령, 환경·하드웨어, 원시 JSON, stdout·stderr·운영 로그를 전달한다.
- 시험 계정 파일·개인키 파일 자체는 전달하지 않는다. 이 시험의 CA·키는 공개 시험 자료지만 실제 운영 자료와 섞지 않는다.
