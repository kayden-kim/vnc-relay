# ADR-0004: 비동기 런타임은 tokio, TLS는 rustls를 쓴다

| 항목 | 내용 |
| --- | --- |
| 상태 | Accepted |
| 결정일 | 2026-10-16 |
| 결정자 | 프로젝트 소유자(사용자) |
| 관련 요구사항 | [PRD 확정 기준 "구현 기술"·"TLS 종료"](../requirements/PRD.md#확정-기준), [FR-07](../requirements/PRD.md#fr-07-설정과-프로세스-수명), [7절 보안 및 신뢰 경계](../requirements/PRD.md#7-보안-및-신뢰-경계), [8절 이식성](../requirements/PRD.md#8-비기능-요구사항) |
| 관련 스펙·계약 | [설정·운영 스펙](../specs/operations.md#4-보안과-신뢰-경계), [WS·WSS 전송 스펙](../specs/websocket.md), [설정 계약](../contracts/config-v1.md), [WebSocket 중계 계약](../contracts/websocket-relay-v1.md) |
| 관련 아키텍처 문서 | 없음(구현 전) |
| 이 ADR이 대체하는 이전 ADR | 없음 |
| 이 ADR을 대체한 새 ADR | 없음 |

## 1. 결정

- 비동기 런타임은 tokio를 쓴다.
- TLS는 rustls를 쓴다. Windows 11 x64와 Ubuntu 24.04 LTS x64에서 같은 TLS 구현을 쓴다.
- OS 인증서 저장소(Windows의 SChannel 포함)와 OpenSSL을 TLS에 쓰지 않는다. 서버 인증서와 개인키는 설정 파일이 가리키는 파일에서만 읽는다.
- rustls의 암호 모듈(crypto provider)은 구현 단계에서 Windows 11 x64와 Ubuntu 24.04 LTS x64의 빌드를 확인한 뒤 정한다([OPS-TBD-009](../specs/operations.md#8-미결정-사항)). tokio·rustls를 잇는 어댑터 crate(예: tokio-rustls)의 채택은 의존성 검토에서 정한다([OPS-021](../specs/operations.md#7-수용-테스트)). 이 ADR은 둘을 정하지 않는다.

## 2. 배경

[ADR-0001](ADR-0001-built-in-tls.md)에 따라 서버가 TLS를 직접 종료한다. 그러려면 TLS 라이브러리가 필요하다. Windows와 Ubuntu 양쪽에서 같은 동작을 보여야 한다.

- 맥락 없이 보면 의외인 점: Windows에는 OS가 제공하는 TLS(SChannel)와 인증서 저장소가 있다. Ubuntu에는 시스템 OpenSSL이 있다. 이 프로젝트는 두 OS의 기본 TLS를 쓰지 않고 Rust로 작성된 TLS 라이브러리를 가져온다. 코드를 읽는 사람은 "OS가 이미 가진 TLS를 왜 쓰지 않나"라고 물을 수 있다. Windows에서 인증서를 OS 저장소에 설치하지 않고 파일로 두는 점도 Windows 관례와 다르다.
- 되돌리기 어려운 이유: TLS 라이브러리는 인증서 형식, 재로드 방식, 오류 모델, 빌드 체인, 라이선스 고지에 스며든다. 나중에 바꾸면 설정 계약의 인증서 항목과 TLS 테스트 전체가 영향을 받는다.
- tokio: Rust 비동기 서버에서 관례적인 선택이다. 대안 비교는 두지 않고 이 ADR에서 짧게만 다룬다.

## 3. 결정 요인

1. 두 대상 OS에서 TLS 동작(허용 프로토콜 버전, 협상 실패 방식)이 같은가.
2. 인증서와 개인키를 파일에서 읽어 재시작 없이 교체할 수 있는가.
3. TLS 1.2 미만 거절을 설정 실수 없이 만족하는가([OPS-R17](../specs/operations.md#41-판정-가능한-규칙)).
4. 빌드·배포 부담. 외부 시스템 라이브러리 의존이 늘어나지 않는가.
5. 라이선스 검토 부담.
6. tokio 위에서 비동기 연결을 다루기 쉬운가.

## 4. 선택 근거

외부 사실은 아래 문서를 근거로 한다. 조회 시점의 문서이며 버전에 따라 바뀔 수 있다.

1. 같은 동작. rustls는 OS와 무관하게 같은 라이브러리다. 프로젝트 README는 rustls 자체가 플랫폼 독립이라고 설명한다([rustls README](https://github.com/rustls/rustls/tree/main)). native-tls는 Windows에서 SChannel, 그 밖의 OS(macOS 제외)에서 OpenSSL을 쓴다([native-tls 문서](https://docs.rs/native-tls/latest/native_tls/)). 이 방식에서는 두 대상 OS가 서로 다른 TLS 구현을 쓰게 된다. 구현 차이가 협상 결과에 영향을 주는지는 관찰로 확인해야 한다.
2. 파일 기반 인증서. 이 프로젝트는 인증서를 파일로 두고 파일 변경을 감지해 교체한다([운영 스펙 3.3절](../specs/operations.md#33-파일-변경-자동-감지와-부분-재로드)). OS 인증서 저장소 연동을 쓰지 않으면 인증서 위치와 교체 경로가 두 OS에서 하나가 된다. native-tls도 PEM 인증서와 키로 서버 신원을 만드는 함수를 제공한다([Identity](https://docs.rs/native-tls/latest/native_tls/struct.Identity.html)). 파일 기반이 불가능해서 제외한 것이 아니다.
3. TLS 1.2 미만 거절. rustls는 TLS 1.2와 TLS 1.3을 구현한다([rustls README](https://github.com/rustls/rustls/tree/main)). 이 프로젝트가 요구하는 하한(TLS 1.2)과 일치한다. native-tls의 프로토콜 버전 열거형에는 SSL 3.0, TLS 1.0, TLS 1.1 항목이 있다([Protocol](https://docs.rs/native-tls/latest/native_tls/enum.Protocol.html)). 하한을 설정으로 막아야 하는지 여부는 이 ADR에서 확인하지 않았다.
4. 빌드·배포. native-tls는 Ubuntu에서 OpenSSL을 쓴다([native-tls 문서](https://docs.rs/native-tls/latest/native_tls/)). rustls는 암호 연산을 암호 모듈에 맡기며 모듈을 선택해야 한다. 프로젝트 README는 aws-lc-rs 기반 모듈이 일부 플랫폼에서 빌드가 더 어려울 수 있고 ring 기반 모듈은 여러 플랫폼에서 빌드하기 쉽다고 설명한다([rustls README](https://github.com/rustls/rustls/tree/main)). 두 대상 OS에서 어느 모듈이 빌드되는지는 아직 확인하지 않았다. 이 확인을 후속 작업으로 둔다.
5. 라이선스. rustls는 Apache-2.0, MIT, ISC 중에서 고를 수 있는 조건으로 배포된다([rustls README](https://github.com/rustls/rustls/tree/main)). 이 프로젝트는 MIT이고 의존성 고지를 별도로 확인한다([OPS-021](../specs/operations.md#7-수용-테스트)). 암호 모듈의 라이선스는 모듈을 고를 때 따로 확인한다.
6. tokio 통합. tokio-rustls는 rustls를 tokio용 비동기 TLS 스트림으로 감싼다([tokio-rustls 문서](https://docs.rs/tokio-rustls/latest/tokio_rustls/)). rustls README도 tokio를 이미 쓰는 경우 tokio-rustls를 고려하라고 소개한다.

## 5. 대안

| 대안 | 두 OS 동일 동작 | 파일 기반 인증서·재로드 | TLS 1.2 하한 | 빌드·배포 부담 | 라이선스 검토 | tokio 통합 | 제외 이유 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| tokio + rustls(선택) | 같은 구현 | 가능, OS 저장소 미사용 | 구현 범위가 1.2·1.3 | 암호 모듈 선택과 두 OS 빌드 확인 필요 | 암호 모듈 포함해 검토 필요 | tokio-rustls가 있음 | 선택 |
| tokio + native-tls(Windows SChannel, Ubuntu OpenSSL) | OS마다 다른 구현 | PEM 신원 생성 함수가 있음. OS별 동작은 관찰 필요 | 구현이 낮은 버전 항목을 노출함. 하한 설정 방법은 미확인 | Ubuntu에서 OpenSSL 시스템 라이브러리에 의존 | OS 구현은 OS가 제공, OpenSSL은 별도 확인 | 미확인 | 요인 1에서 두 OS가 다른 구현을 쓰고 요인 4에서 Ubuntu가 OpenSSL에 의존함. 요인 3·6은 확인하지 못해 제외 이유로 쓰지 않음 |
| 구현 단계로 연기 | 미정 | 인증서 형식·재로드 계약을 확정할 수 없음 | 미정 | 미정 | 미정 | 미정 | 요인 2에서 설정 계약(`cert_file`·`key_file`)과 OPS-TBD-003(암호 스위트)을 구현 전에 정할 수 없음 |

native-tls 행의 "미확인" 항목은 이 ADR에서 조사하지 않았다. 제외 이유는 확인한 사실(요인 1·4)만으로 들었다.

## 6. 결과

### 긍정적 결과

- 두 대상 OS에서 같은 TLS 구현을 테스트한다. TLS 결과의 OS별 차이가 라이브러리에서 오지 않는다.
- 인증서가 파일 하나의 경로로 관리되어 설정 계약과 재로드 규칙이 단순하다.
- 시스템 OpenSSL 설치에 의존하지 않는다. 암호 모듈의 빌드 요구는 따로 있다.
- TLS 구현 범위(TLS 1.2·1.3)가 스펙의 하한과 맞는다.

### 부정적 결과와 비용

- Windows 운영자는 OS 인증서 저장소의 인증서를 쓸 수 없다. 파일로 내보내서 설정해야 한다.
- OS가 제공하는 TLS 보안 업데이트를 받지 못한다. rustls와 암호 모듈 업데이트는 이 프로젝트의 릴리스로 반영해야 한다.
- 암호 모듈을 직접 골라야 하며 선택이 빌드 가능성과 지원 기능에 영향을 준다.
- 암호 스위트 목록은 선택한 암호 모듈이 제공하는 범위 안에서만 정할 수 있다.

### 후속 작업

- 암호 모듈 선택(OPS-TBD-009): 2026-10-20에 사용자가 aws-lc-rs를 골랐다. Windows 11 x64와 Ubuntu 24.04 LTS x64에서 저장소 의존성 aws-lc-rs가 빌드되었고 라이선스 검사(`licenses ok`)가 통과했다. 근거는 [개발 환경 how-to](../how-to/set-up-and-verify.md) 4절 실행 기록 1·3과 [의존성 라이선스 검사](../how-to/check-dependency-licenses.md) 실행 기록 5다. Windows 빌드는 NASM 없이 `AWS_LC_SYS_PREBUILT_NASM=1`로 확인했다. rustls 의존성은 작업 3.1에서 `crates/adapters`에 더했다.
- 설정 계약: `cert_file`·`key_file` 형식은 [설정 계약 1.9절](../contracts/config-v1.md#19-인증서개인키-파일-형식)이 PEM으로 채택(2026-10-16)했다.
- 허용 암호 스위트는 선택한 암호 모듈의 rustls 기본 목록으로 정했고 설정 키는 없다(2026-10-16). OPS-TBD-003은 결번이다.
- tokio-rustls 등 어댑터 crate: tokio-rustls를 채택했다(2026-10-20 의존성 라이선스 검토, [PRD 확정 기준](../requirements/PRD.md#확정-기준)의 "구현 기술" 행).

## 7. 검증

| 가설·결정 요인 | 검증 방법 | 합격 기준 | 증거 |
| --- | --- | --- | --- |
| 요인 3: TLS 1.2 미만을 거절한다 | 보안 통합 테스트 | TLS 1.0·1.1 협상 실패, 1.2 이상 성립 | [OPS-036](../specs/operations.md#7-수용-테스트): 통과(`crates/adapters/tests/websocket.rs` `tls10_and_tls11_are_refused_and_tls12_and_tls13_succeed_on_wss_and_management`가 WSS 리스너와 관리 HTTPS에서 TLS 1.0·1.1 거절과 1.2·1.3 성립을 확인. Ubuntu `vr-full`, `e701997`) |
| 요인 1: 두 OS에서 같은 협상 결과를 보인다 | 배포 호환성 테스트 | 두 플랫폼의 실행 증거 확보 | [OPS-019](../specs/operations.md#7-수용-테스트): 미실행(구현·테스트 파일 없음) |
| 요인 2: 인증서 교체가 파일 변경으로 적용된다 | 통합 테스트 | 새 핸드셰이크는 새 인증서, 기존 연결 유지 | [OPS-024](../specs/operations.md#7-수용-테스트): 미실행(구현·테스트 파일 없음) |
| 요인 2: 잘못된 인증서·키 후보를 거절한다 | 통합 테스트 | 키 불일치·한쪽 교체는 후보 전체 거절, 기존 유지 | [OPS-026](../specs/operations.md#7-수용-테스트): 미실행(구현·테스트 파일 없음) |
| 요인 2: TLS 준비 실패가 정상 준비로 표시되지 않는다 | 통합 테스트 | 준비 완료 아님, 자원 회수 | [OPS-002](../specs/operations.md#7-수용-테스트): 통과(작업 3.1: `tests/serve.rs`의 `fails_with_exit_code_4_and_releases_listeners_when_a_port_is_taken`이 포트 점유 때 자원 회수를, `checks_tls_files_before_binding_a_taken_status_port`가 내장 TLS 준비 실패는 바인드 전에 멈춤을 확인. 사용자 결정 2026-10-22(Q26)로 스펙 조건을 고침. Ubuntu·Windows 모두 통과) |
| 요인 5: 의존성 라이선스를 확인한다 | 출처·라이선스 검토 | 의존성 고지 의무 확인 | [OPS-021](../specs/operations.md#7-수용-테스트): 미실행(구현·의존성 없음) |
| 요인 4: 두 OS에서 빌드·배포가 된다 | 배포 통합 테스트 | 외부 서비스·컨테이너 없이 실행 | [OPS-031](../specs/operations.md#7-수용-테스트): 미실행(구현·테스트 파일 없음) |

## 8. 재검토 조건

- rustls 또는 선택한 암호 모듈에서 이 프로젝트의 TLS 기능에 영향을 주는 보안 권고가 공개되고, 수정 버전이 나오기까지 릴리스를 낼 수 없는 기간이 발생한다.
- 두 대상 OS 중 한 곳에서 선택한 암호 모듈이 빌드되지 않거나 OPS-036이 통과하지 않는다.
- 운영자가 OS 인증서 저장소의 인증서를 직접 쓰게 해 달라는 요구를 접수하고, 그 요구가 PRD에 채택된다.
- 요구된 암호 스위트가 선택한 암호 모듈이 제공하는 범위 밖에 있다.
