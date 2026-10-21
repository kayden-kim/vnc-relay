# ADR-0001: WSS·관리 HTTPS의 TLS를 서버가 직접 종료한다

| 항목 | 내용 |
| --- | --- |
| 상태 | Accepted |
| 결정일 | 2026-10-16 |
| 결정자 | 프로젝트 소유자(사용자) |
| 관련 요구사항 | [PRD 확정 기준 "TLS 종료"](../requirements/PRD.md#확정-기준), [FR-05](../requirements/PRD.md#fr-05-wswss-프록시), [FR-07](../requirements/PRD.md#fr-07-설정과-프로세스-수명), [7절 보안 및 신뢰 경계](../requirements/PRD.md#7-보안-및-신뢰-경계) |
| 관련 스펙·계약 | [WS·WSS 전송 스펙](../specs/websocket.md), [설정·운영 스펙](../specs/operations.md#4-보안과-신뢰-경계), [관리 스펙](../specs/management.md), [설정 계약](../contracts/config-v1.md), [관리 API 계약](../contracts/management-api-v1.md), [WebSocket 중계 계약](../contracts/websocket-relay-v1.md) |
| 관련 아키텍처 문서 | 없음(구현 전) |
| 이 ADR이 대체하는 이전 ADR | 없음 |
| 이 ADR을 대체한 새 ADR | 없음 |

## 1. 결정

VNC Relay 프로세스가 WSS 리스너와 관리 HTTPS 리스너에서 TLS를 직접 종료한다. 인증서와 개인키는 설정 파일이 가리키는 파일에서 읽는다. 역방향 프록시가 TLS를 종료하는 배치는 지원 대상이 아니다. 두 방식을 모두 지원하지 않는다.

TLS 라이브러리는 [ADR-0004](ADR-0004-tokio-rustls.md)가 정한다.

## 2. 배경

WSS와 관리 HTTPS는 인터넷에서 접근할 수 있다. 일반적인 웹 서비스는 앞단의 프록시가 TLS를 종료하고 애플리케이션은 평문 HTTP만 받는 구조를 쓴다. 이 프로젝트는 그 구조를 따르지 않는다.

- 맥락 없이 보면 의외인 점: 웹 서버 관례와 달리 애플리케이션이 인증서를 직접 읽고 TLS 협상을 맡는다. 코드를 읽는 사람은 "프록시에 맡기면 되는데 왜 TLS를 직접 구현에 포함했나"라고 물을 수 있다.
- 되돌리기 어려운 이유: 출발지 IP 검사 시점, 인증서 재로드, 연결 수 집계, 테스트 구성이 모두 이 선택에 맞춰 설계된다. 나중에 프록시 전제로 바꾸면 IP 판정 근거와 위 항목을 함께 다시 설계해야 한다.
- 제약: 배포 대상은 Windows 11 x64와 Ubuntu 24.04 LTS x64의 실행파일 하나다. OS 서비스 등록과 컨테이너는 배포 범위에 없다([PRD 8절](../requirements/PRD.md#8-비기능-요구사항)).

## 3. 결정 요인

1. 출발지 IP 판정 근거. 판정은 TCP 소켓의 실제 상대 주소로 하고 전달 헤더를 믿지 않는다. 판정 시점은 TLS 협상 전이다.
2. 구성 요소 수와 운영 부담. 운영자가 수동으로 설치할 수 있어야 한다.
3. 인증서 교체 책임이 제품 안에서 닫히는가. 재시작 없이 교체하는 재로드가 필요하다.
4. TLS 정책의 직접 통제. TLS 1.2 미만 협상을 이 제품이 거절해야 한다.
5. 구현·테스트 조합 수.

## 4. 선택 근거

1. 출발지 IP 판정. 중계 접근 통제는 실제 출발지 IP 허용 목록이다([ADR-0002](ADR-0002-source-ip-allowlist.md)). 프록시가 TLS를 종료하면 VNC Relay가 보는 상대 주소는 프록시의 주소다. 클라이언트 주소를 얻으려면 전달 헤더를 신뢰해야 한다. 이는 스펙이 금지한 방식이다([운영 스펙 4절](../specs/operations.md#4-보안과-신뢰-경계) "클라이언트 주소" 행). 내장 TLS에서는 [WS-023](../specs/websocket.md#7-수용-테스트)이 요구하는 "TLS 협상 전 종료"를 이 프로세스가 직접 수행한다.
2. 구성 요소 수. 프록시를 필수로 두면 운영자가 두 프로그램의 설정과 갱신을 맡는다. 배포 범위가 실행파일·설정 예시·수동 운영 안내이므로 필수 외부 구성 요소를 늘리지 않는다.
3. 인증서 교체. 인증서 파일 변경을 감지해 새 TLS 핸드셰이크부터 적용하는 동작을 제품이 소유한다([운영 스펙 3.3절](../specs/operations.md#33-파일-변경-자동-감지와-부분-재로드)). 프록시가 인증서를 가지면 이 재로드 범위가 제품 밖으로 나간다.
4. TLS 정책. TLS 1.2 미만 거절([OPS-R17](../specs/operations.md#41-판정-가능한-규칙))은 제품이 소유한 리스너에서 직접 판정하고 테스트할 수 있다.
5. 테스트 조합. 한 방식만 지원하면 IP 판정과 TLS 테스트의 조합이 늘지 않는다.

## 5. 대안

| 대안 | 출발지 IP 판정 근거 | 구성 요소 수·운영 부담 | 인증서 교체 책임 | TLS 정책 통제 | 테스트 조합 | 제외 이유 |
| --- | --- | --- | --- | --- | --- | --- |
| 서버 내장 TLS(선택) | 소켓 상대 주소를 TLS 협상 전에 사용 | 실행파일 하나 | 제품이 파일 변경을 감지해 적용 | 제품이 직접 거절 | 한 가지 | 선택 |
| 역방향 프록시가 TLS 종료 | 프록시 주소만 보임. 클라이언트 주소는 전달 헤더가 필요함 | 프록시 설치·설정·갱신이 추가됨 | 프록시가 소유. 제품 재로드 범위 밖 | 프록시 설정에 의존 | 한 가지 | 요인 1에서 전달 헤더 신뢰가 필요해 스펙 규칙과 충돌 |
| 둘 다 지원 | 배치마다 판정 근거가 다름 | 두 배치의 설정 문서가 필요함 | 배치마다 다름 | 배치마다 다름 | 배치 수만큼 늘어남 | 요인 1에서 프록시 배치의 판정 근거가 없고 요인 5의 테스트 비용만 늘어남 |

## 6. 결과

### 긍정적 결과

- 출발지 IP 검사를 TLS 협상 전에 수행할 수 있다. 허용되지 않은 출발지가 TLS 처리 자원을 쓰지 못한다.
- 관리 세션 쿠키의 Secure 속성 판단 기준이 리스너가 HTTPS인지 여부 하나다. 프록시 뒤의 HTTP를 HTTPS로 간주하는 규칙이 필요 없다.
- 배포 산출물이 실행파일·설정 예시·수동 운영 안내로 닫힌다.

### 부정적 결과와 비용

- 인증서 발급과 갱신은 운영자 책임이다. 제품은 자동 발급을 하지 않는다([운영 스펙 3.3절](../specs/operations.md#33-파일-변경-자동-감지와-부분-재로드)).
- TLS 구현의 취약점 대응이 제품 릴리스 주기에 묶인다.
- 이미 프록시를 쓰는 운영자의 배치는 지원하지 않는다.
- TLS 1.2 미만 클라이언트는 접속할 수 없다.

### 후속 작업

- 설정 계약: `cert_file`·`key_file` 키와 재로드 대상은 [설정 계약](../contracts/config-v1.md)이 채택(2026-10-16)해 정했다.
- 수동 운영 안내: 인증서 교체 절차를 쓴다(OPS-031 대상).
- 허용 암호 스위트는 선택한 rustls 암호 모듈의 rustls 기본 목록으로 정했고 설정 키는 없다(2026-10-16). 암호 모듈 선택은 OPS-TBD-009에서 관리한다.

## 7. 검증

| 가설·결정 요인 | 검증 방법 | 합격 기준 | 증거 |
| --- | --- | --- | --- |
| 요인 1: 비허용 IP는 TLS 협상 전에 끊는다 | 통합 테스트 | 비허용 출발지는 TLS 협상 없이 종료 | [WS-023](../specs/websocket.md#7-수용-테스트), [WS-014](../specs/websocket.md#7-수용-테스트): 통과(`crates/adapters/tests/websocket.rs` `disallowed_source_on_wss_is_closed_before_tls_and_holds_no_relay_place`, `disallowed_source_is_closed_before_tls_or_http_even_with_session_hints`. Ubuntu `vr-full`, `e701997`) |
| 요인 1: 전달 헤더로 정책을 우회할 수 없다 | 보안 통합 테스트 | 헤더를 보내도 주소·접근 정책이 바뀌지 않음 | [MGT-015](../specs/management.md#7-수용-테스트): 미실행(관리 연결의 전달 헤더 시험 없음. 사용자 결정(2026-10-21)으로 작업 4.4의 완료 조건에 넣었다) |
| 요인 2: 내장 TLS 없이 공개 관리 접속을 받지 않는다 | 배포 보안 테스트 | 공개 평문 HTTP 수락 없음 | [OPS-014](../specs/operations.md#7-수용-테스트): 부분(`tests/serve.rs` `public_management_never_starts_on_plaintext_or_without_tls_or_an_account`가 공개 주소의 평문·인증서 누락·계정 없음 시작 거절을 확인한다. 공개 HTTPS 401은 루트 테스트에 TLS 클라이언트가 없어 확인하지 않았다. 운영 조회(작업 4.4)와 관리자 종료(작업 4.5)는 미구현이라 미실행). [OPS-034](../specs/operations.md#7-수용-테스트): 통과(`tests/serve.rs` `refuses_public_plaintext_and_answers_login_less_queries_with_401_on_loopback`, 비부분. 부분 테스트: `crates/adapters/tests/config_contract.rs` `allows_plaintext_management_only_on_loopback`, `crates/adapters/tests/management.rs` `plaintext_api_requests_are_refused_without_a_session`) |
| 요인 3: 인증서 교체가 재시작 없이 적용된다 | 통합 테스트 | 새 핸드셰이크는 새 인증서, 기존 연결 유지 | [OPS-024](../specs/operations.md#7-수용-테스트): 미실행(구현·테스트 파일 없음) |
| 요인 4: TLS 1.2 미만을 거절한다 | 보안 통합 테스트 | TLS 1.0·1.1 협상 실패, 1.2 이상 성립 | [OPS-036](../specs/operations.md#7-수용-테스트): 통과(`crates/adapters/tests/websocket.rs` `tls10_and_tls11_are_refused_and_tls12_and_tls13_succeed_on_wss_and_management`가 WSS 리스너와 관리 HTTPS를 함께 확인. 부분 테스트: `crates/adapters/tests/tls.rs`, `crates/adapters/tests/management.rs`. Ubuntu `vr-full`, `e701997`) |
| 요인 4: WSS 인증서 오류에서 평문으로 전환하지 않는다 | 배포 통합 테스트 | 인증서 오류마다 TLS 실패 | [WS-013](../specs/websocket.md#7-수용-테스트): 통과(`crates/adapters/tests/websocket.rs` `expired_untrusted_and_mismatched_certificates_fail_the_client_on_wss`가 실제 WSS 리스너에서 만료·신뢰되지 않음·호스트 불일치를 확인. 부분 테스트: `crates/adapters/tests/tls.rs`. Ubuntu `vr-full`, `e701997`) |
| 요인 4: 관리 세션 쿠키 속성이 리스너 종류와 맞는다 | 보안 통합 테스트 | HTTPS는 Secure 포함, 평문 루프백은 Secure 제외 | [MGT-017](../specs/management.md#7-수용-테스트): 부분(`crates/adapters/tests/management.rs` `stale_session_cookie_is_cleared_without_secure_over_plain_http`, HTTPS 쿠키 삭제의 Secure는 `https_clients_on_tls12_and_tls13_get_the_unauthenticated_answer`. 로그인 쿠키 속성은 작업 4.3) |

## 8. 재검토 조건

- 운영자가 프록시 뒤 배치를 요구하는 이슈가 접수되고, 프록시가 전달하는 클라이언트 주소를 신뢰하는 방식이 PRD에서 채택된다.
- 내장 TLS 구조에서 WS-023 또는 OPS-036 테스트를 두 대상 OS 중 한 곳에서 통과시킬 수 없다는 결과가 기록된다.
- OPS-024의 인증서 교체 테스트가 두 대상 OS 중 한 곳에서 반복해서 실패한다.
