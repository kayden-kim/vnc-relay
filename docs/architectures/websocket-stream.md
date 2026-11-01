# WebSocket 스트림 어댑터 모듈 설계

이 문서는 WS·WSS의 바이너리 페이로드를 공통 `RelayStream`으로 옮기는 목표 구조를 정한다. 예를 들어 한 메시지에 250바이트 요청과 후속 데이터가 들어오면, 요청은 `handle_connection`이 해석한다. 후속 데이터는 같은 연결의 중계 스트림 앞부분으로 남는다.

| 항목 | 값 |
| --- | --- |
| 상태 | Draft |
| 구조 범위 | Current: §1.1·§3~§6·§8의 스트림·종료 구현. §7은 현재 회귀 시험과 계획을 구분한다. Target: §9.5 Kw(작업 3.4) |
| 적용 작업 | 2.7 완료 전에 가져오는 3.3의 스트림 어댑터. 3.4의 Kw·종료 처리와 연결할 경계도 설계 |
| 기준 | [WebSocket 계약 1.4~1.6절](../contracts/websocket-relay-v1.md#14-데이터-메시지와-제어-프레임), [초기 교환 설계](initial-exchange.md), [세션 설계 3.2절](relay-session.md#32-전송-수명-인터페이스) |
| 결정 수준 | 9.1~9.3절은 사용자 결정 2026-11-01(Q77, Q78, Q79)로 해결했다. 9.4절은 사용자 결정 2026-11-01(Q83)로 확정했다. 9.5절은 3.4 범위이며 2.7을 막지 않는다. 채택된 계약을 바르게 반영함 |
| 구현 대조 | `638cf09`에서 WS·WSS 진입점을 연결했다. 이후 결함 수정은 현재 작업 트리의 `websocket_stream.rs`, `frame.rs`, `relay/session.rs`, `serve.rs`와 대조했다. Kw(작업 3.4)는 미구현이다. 시험 범위는 §7이다. |

별도 문서로 둔다. [기존 진입점 설계](websocket-entry-point.md)는 HTTP·TLS 검사와 101까지를 설명한다. 이 문서는 101 뒤의 프레임·B·종료 수명을 맡는다. `Current`와 `Target`을 섞지 않기 위한 선택이다(**이 문서의 판단, 사용자 결정 없음**).

## 1. 책임과 확인한 사실

### 1.1 현재 코드

아래 표는 파일과 심볼을 읽어서 확인한 사실이다. 목표 동작을 현재 구현으로 표시하지 않는다.

| 위치 | 확인한 사실 |
| --- | --- |
| [`Cargo.lock`](../../Cargo.lock), [`Cargo.toml`](../../Cargo.toml) | 잠금 버전은 `tokio-tungstenite 0.30.0`, `tungstenite 0.30.0`이다. 직접 의존은 기본 기능을 끄고 `handshake`를 켠 `tokio-tungstenite`다. |
| [`crates/adapters/src/websocket.rs`](../../crates/adapters/src/websocket.rs)의 `upgrade` | `upgrade::read_head`로 head와 뒤따른 원시 바이트(`suffix`)를 받는다. `switching_protocols`의 101을 `write_all`·`flush`한 뒤 `Step::Switched { suffix, exchange_at }`를 반환한다. 라이브러리의 HTTP 수락 함수가 101을 보내는 구조가 아니다. |
| 같은 파일의 `WsPolicy`, `connection` | `WsPolicy`에는 `accept`, `upgrade`, `origins`, `message_bytes`(M), `buffer_bytes`(B)가 있다. `connection`은 `suffix`를 `WsRelayStream::new`에 넘기고 `RelayService::handle_connection`과 `dispatch`로 보낸다. `hold_upgraded`는 제거되었다. Kw는 아직 주입하지 않는다. |
| [`crates/adapters/src/upgrade.rs`](../../crates/adapters/src/upgrade.rs)의 `read_head` | 청크 단위로 읽고 HTTP head 상한을 둔다. `\r\n\r\n` 뒤 바이트는 버리지 않고 `HeadRead::Complete { head, suffix }`의 `suffix`로 돌려준다. |
| 같은 파일의 `check`, `switching_protocols` | `tungstenite::handshake::derive_accept_key`를 쓴다. `binary` 서브프로토콜을 선택할 수 있다. 확장을 수락하지 않는다. |
| [`crates/app/src/relay/handshake.rs`](../../crates/app/src/relay/handshake.rs)의 `read_request` | Viewer일 때 `VIEWER_GREETING`을 한 번 `write_all`한다. 별도 `flush`는 없다. 요청을 나눠 읽고 250바이트 뒤의 데이터를 반환한다. |
| [`crates/app/src/relay.rs`](../../crates/app/src/relay.rs)의 `handle_connection`, `prepare`, `close_stream` | 성공하면 `PreparedConnection<S>`를 반환한다. 실패하면 사유별 `RelayCloseAction`을 E 안에서 진행한다. 이어 스트림·L 자리를 반환하고 사건을 기록한다. |

WS·WSS의 H는 101 응답 전송 완료 뒤 `ConnectionContext.exchange_at`에서 시작한다(`connection`의 `ConnectionContext`, 공통 서비스 `handshake`). 머리 읽기 시작 시각을 H 시작으로 쓰지 않는다.

### 1.2 계약의 제약

- [설정 계약](../contracts/config-v2.md)은 B를 16 KiB~4 MiB로 정한다. M은 64 KiB~16 MiB다. 따라서 유효한 설정에도 M > B가 있다.
- [데이터 계약 1.4절](../contracts/websocket-relay-v1.md#14-데이터-메시지와-제어-프레임)은 프레임·메시지 경계를 중계 스트림 경계로 쓰지 않는다. 빈 바이너리 메시지는 EOF가 아니다.
- M은 **논리 메시지 전체**의 페이로드 길이다. 분할 프레임의 합에도 적용한다. M+1이면 1009다. 초과를 알기 전에 앞부분이 TCP로 전달될 수 있다.
- 서버의 첫 데이터 메시지는 `RFB 000.000\n` 12바이트를 담은 바이너리 메시지 하나다. 서버가 보내는 각 메시지는 M 이하다.
- [운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)은 초기 교환 후속 데이터도 B에 포함한다. [종료 계약 1.6절](../contracts/websocket-relay-v1.md#16-종료-의미)은 Close와 TCP half-close를 구별하고 정리 시간 E를 제한한다.
- [초기 교환 설계 9.2절](initial-exchange.md#2-해결-wswss를-27에서-연결할지)은 WS·WSS 인계를 2.7 완료 전에 요구한다. Q16·Q21과 기존 3.2의 `현재:`는 바꾸지 않는다. 단일 메시지만 처리하는 임시 파서는 만들지 않는다.

### 1.3 잠금 버전의 스트리밍 한계

`Stream<Item = Result<Message, Error>>`는 완성된 메시지를 돌려준다. `Message::Frame`은 수신 결과가 아니다. `from_partially_read`는 선행 바이트를 보존할 수 있지만 페이로드를 청크로 돌려주는 API는 아니다([버전 고정 API](https://docs.rs/tokio-tungstenite/0.30.0/tokio_tungstenite/struct.WebSocketStream.html), [Message](https://docs.rs/tungstenite/0.30.0/tungstenite/protocol/enum.Message.html)).

하위 `FrameSocket::read`도 프레임 전체를 반환한다. `FrameCodec::read_frame`은 선언한 페이로드 길이만큼 공간을 확보하고, 그 길이를 모두 읽은 뒤 프레임을 만든다. `IncompleteMessage`는 분할 페이로드를 모아 FIN에서 메시지를 완성한다([프레임 API](https://docs.rs/tungstenite/0.30.0/tungstenite/protocol/frame/struct.FrameSocket.html), [프레임 소스](https://docs.rs/tungstenite/0.30.0/src/tungstenite/protocol/frame/mod.rs.html), [메시지 소스](https://docs.rs/tungstenite/0.30.0/src/tungstenite/protocol/message.rs.html)). 같은 버전의 로컬 Cargo 소스도 읽어 확인했다.

따라서 현재 공개 API로는 **B보다 큰 유효한 단일 프레임을 B 안에서 중계할 수 없다**. `read_buffer_size`를 줄여도 프레임 전체 대기는 남는다. `max_frame_size = B`는 B < 프레임 길이 ≤ M인 유효 입력을 거절한다. 어느 쪽도 계약의 해결책이 아니다.

공개 `FrameHeader::parse`와 `format`은 헤더만 다룬다. 이 API를 재사용하고 페이로드·제어 상태를 직접 관리하는 길은 있다([헤더 API](https://docs.rs/tungstenite/0.30.0/tungstenite/protocol/frame/struct.FrameHeader.html)). 이는 완성 메시지 API를 계속 쓰는 방안과 구별한다.

## 2. 구현 후보와 의존성 경계

[ADR-0004](../decisions/ADR-0004-tokio-rustls.md)는 tokio·rustls를 고정한다. [PRD의 구현 기술](../requirements/PRD.md#확정-기준)과 [ADR-0005](../decisions/ADR-0005-in-house-ws-streaming-frame-codec.md)에 따라 자체 스트리밍 상태 머신을 사용한다(사용자 결정 2026-11-01(Q77)). 새 라이브러리 의존성은 추가하지 않고, tungstenite 0.30.0의 `FrameHeader::parse`/`format`과 `derive_accept_key`만 재사용한다.

아래 크기는 crates.io의 압축 `.crate` 크기와 `Rust code_lines`다. 실행파일 증가량이나 전이 의존성 전체 크기가 아니다. 갱신 시각은 유지보수의 관찰 자료이지 품질 보증이 아니다.

| 후보 | 페이로드 전달 단위와 제약 | 라이선스 | 공개 릴리스·유지 상태 | 크기 |
| --- | --- | --- | --- | --- |
| 잠금 버전 유지 | 완성 메시지. 하위 API도 완성 프레임. 단독으로 B 조건을 충족하지 못함 | `tokio-tungstenite`: MIT. `tungstenite`: MIT OR Apache-2.0 | 둘 다 0.30.0, 2026-07-11 릴리스 | 각각 29,413 B·952행, 71,531 B·3,941행 |
| 작은 자체 RFC 6455 코덱 | 헤더 뒤 페이로드를 호출자 버퍼로 직접 읽고 제자리에서 마스크 해제. 기존 `FrameHeader` 재사용 가능. 제어·분할·Close 상태는 프로젝트가 책임짐 | 독립 구현은 제품과 같은 MIT. 재사용 crate 고지는 유지 | 아직 구현하지 않음. 보안·호환성 유지보수는 프로젝트 책임 | 새 crate 의존성 0개인 형태 가능. 소스 행 수·배포물 증가량은 미측정. 프레임 헤더 최대 14바이트, 제어 페이로드 슬롯당 최대 125바이트 |
| `fastwebsockets 0.10.0` | `read_frame`이 완성 프레임을 반환. 자동 Close/Pong은 끌 수 있지만 큰 단일 프레임의 저장 문제는 남음 | Apache-2.0 | 2025-01-14 릴리스. 저장소 미보관, 2026-07-31 갱신 | 52,605 B·1,396행 |
| `tokio-websockets 0.13.3` | 디코더가 프레임 페이로드 전체를 기다린 뒤 `split_to(payload_length)`함. 전체 프레임보다 작은 전달 API로 대체할 수 없음 | MIT | 2026-06-25 릴리스. 저장소 미보관, 2026-07-31 갱신 | 349,370 B·2,696행 |
| `soketto 0.8.1` | `base::Codec::decode_header`는 헤더만 해석. 페이로드 청크·마스크 오프셋·논리 M·제어 처리는 직접 작성해야 함 | Apache-2.0 OR MIT | 2024-12-02 릴리스. 저장소 미보관, 2026-10-26 갱신 | 45,210 B·2,310행 |

크기·라이선스·릴리스 근거: [tokio-tungstenite](https://crates.io/api/v1/crates/tokio-tungstenite/0.30.0), [tungstenite](https://crates.io/api/v1/crates/tungstenite/0.30.0), [fastwebsockets](https://crates.io/api/v1/crates/fastwebsockets/0.10.0), [tokio-websockets](https://crates.io/api/v1/crates/tokio-websockets/0.13.3), [soketto](https://crates.io/api/v1/crates/soketto/0.8.1).

대안 API 근거: [fastwebsockets](https://docs.rs/fastwebsockets/0.10.0/fastwebsockets/struct.WebSocket.html), [tokio-websockets 디코더](https://docs.rs/tokio-websockets/0.13.3/src/tokio_websockets/proto/codec.rs.html), [soketto Codec](https://docs.rs/soketto/0.8.1/soketto/base/struct.Codec.html). 유지 상태 근거: [fastwebsockets 저장소 정보](https://api.github.com/repos/denoland/fastwebsockets), [tokio-websockets 저장소 정보](https://api.github.com/repos/Gelbpunkt/tokio-websockets), [soketto 저장소 정보](https://api.github.com/repos/paritytech/soketto).

조사한 유지 중 후보에는 요구한 청크 전달과 논리 M 검사를 그대로 제공하는 대체물이 없었다. 사용자 결정 2026-11-01(Q77)로 기존 공개 헤더 API를 재사용하는 작은 자체 코덱(선택 B)을 확정했다([ADR-0005](../decisions/ADR-0005-in-house-ws-streaming-frame-codec.md)). 별도 TLS 구현이나 다른 런타임은 들이지 않는다. RFC 6455만 보고 독립 구현하며 금지된 제품 소스는 읽거나 옮기지 않는다.

## 3. 내부 구조와 B 소유권

### 3.1 목표 모듈

다음 경로와 심볼은 현재 코드에 있다. 기존 `RelayStream`을 구현한다. 별도 스트림 포트는 없다. 아래 모듈 분리는 **이 문서의 판단, 사용자 결정 없음**이다.

| 위치 | 책임 |
| --- | --- |
| `crates/adapters/src/websocket_stream.rs`의 `WsRelayStream<S>` | `Wire` 또는 같은 tokio I/O를 소유. 기존 `RelayStream` 구현. WS·WSS 전송 종류, 선행 바이트, 읽기·쓰기·Close 상태 유지 |
| `crates/adapters/src/websocket_stream/frame.rs` | 헤더 검사, 프레임 길이·마스크 오프셋, 분할 상태, 논리 M 누적. 소켓·app 사건·세션 등록을 소유하지 않음 |
| 기존 `upgrade.rs` | HTTP head와 함께 읽은 후속 바이트를 분리해 반환. HTTP 검사와 101 형식은 유지 |
| 기존 `websocket.rs` | 101 완료 시각을 문맥에 넣고 스트림·L permit을 공통 서비스에 이동. 프레임에서 Repeater 요청을 직접 파싱하지 않음 |
| 기존 app의 `RelayService`, `PreparedConnection`, 세션 방향 버퍼 | H·C·W·E·G, 요청 판정, 등록·매칭, 전송량, 사건, permit의 최종 소유자 |

소유권은 `websocket.rs` → `handle_connection` → `PreparedConnection` → `dispatch` → 대기 항목 또는 세션 순서로 이동한다. 화살표는 호출과 자원 이동이다. 컴파일 의존은 adapters → app → domain이다. 역참조, 별도 연결 작업의 잔존 소유, 청크별 채널·박싱은 두지 않는다.

### 3.2 B는 같은 방향의 합계다

B는 WS 메시지 하나에 따로 주는 메모리가 아니다. 아래 미전달 데이터의 합을 제한한다. 이 계산 방법은 **이 문서의 판단, 사용자 결정 없음**이다.

- WS → TCP: `app 방향 버퍼의 미전달 페이로드 + 아직 소비하지 않은 업그레이드 후속 바이트 ≤ B`.
- TCP → WS: `app 방향 버퍼의 미전달 페이로드 ≤ B`. 어댑터에는 별도 데이터 메시지 큐를 두지 않는다.
- 초기 교환의 후속 데이터도 같은 app 버퍼다. `PreparedConnection`에서 대기·세션으로 이동하며 복제하지 않는다.
- 고정 길이 요청 파서와 프레임 헤더·제어 상태는 프로토콜 상태다. 제어 페이로드는 슬롯마다 125바이트 이하이고 슬롯 수가 고정이다. 임의의 제어·데이터 큐는 금지한다.
- 송신 제어 슬롯은 `decoder.pending_pong`, `active_pong`, `pending_close`다. 각 슬롯은 고정 크기다. `holding_tx_bytes()`는 미송신 Pong, 진행 중 Pong, Close와 데이터 헤더의 남은 바이트를 합친다. 페이로드를 담는 추가 B 크기 큐는 없다. 슬롯 배치는 **이 문서의 판단, 사용자 결정 없음**이다.
- 업그레이드 후속 바이트는 아직 마스크·프레임 해석 전이다. 그 안의 페이로드를 숨기지 않도록 **원시 바이트 길이 전체를 보수적으로 B에 포함**한다. HTTP head는 검사 뒤 버린다.

현재 `read_head`의 마지막 읽기는 1024바이트다. 따라서 head 뒤에 남는 원시 바이트는 최대 1023바이트다. 이 상한은 코드의 읽기 길이에서 도출한 값이다. B의 최솟값보다 작다.

어댑터는 이 후속 바이트를 **먼저 전부 소비한 뒤에만** `Wire`에서 새 데이터를 읽는다. 앱으로 페이로드를 옮길 때 후속 바이트의 소유 길이를 줄인다. 후속 바이트가 남아 있는 동안 app 데이터도 이 같은 바이트열에서만 나온다. 따라서 두 길이의 합은 처음 후속 바이트 길이를 넘지 않는다. 이후에는 후속 바이트가 0이므로 app이 주는 남은 B 크기의 `ReadBuf`가 상한을 지킨다.

이 조건에서는 새 공통 예산 타입이나 B 크기 어댑터 버퍼가 필요 없다. 나중에 읽기 선행 큐를 넣으면 이 증명은 성립하지 않는다. 그런 변경은 같은 방향의 공유 예산 설계부터 다시 검토한다.

### 3.3 페이로드 스트리밍

1. 최대 14바이트 헤더를 모은다. 클라이언트 마스크, RSV, opcode, 길이 표현, 제어 프레임 길이·FIN을 검사한다.
2. 데이터 프레임의 선언 길이를 논리 메시지 길이에 더한다. 덧셈 넘침과 M 초과를 함께 검사한다. 제어 프레임은 합에 넣지 않는다.
3. B보다 큰 유효 프레임도 허용한다. 페이로드는 호출자가 준 `ReadBuf` 길이까지만 읽는다. 프레임 전체 길이로 `reserve`하지 않는다.
4. 읽은 범위 안에서 마스크를 해제한다. 마스크 위치는 **프레임 전체 오프셋 modulo 4**다. 청크마다 0으로 되돌리지 않는다.
5. continuation과 FIN으로 논리 메시지 상태를 갱신한다. 새 바이너리 메시지는 이전 메시지가 끝났을 때만 시작한다. 빈 메시지는 다음 프레임으로 넘어간다.

RFC 근거는 [5.2절](https://www.rfc-editor.org/rfc/rfc6455#section-5.2), [5.3절](https://www.rfc-editor.org/rfc/rfc6455#section-5.3), [5.4절](https://www.rfc-editor.org/rfc/rfc6455#section-5.4)이다. 5.4절은 확장이 없으면 프레임 전체를 저장하지 않고 일부를 전달할 수 있다고 설명한다.

`FrameHeader::parse`만으로 길이 표현의 유효성을 판단하지 않는다. `try_parse_header`는 원시 길이 표식과 확장 바이트도 검사한다. 표식 126의 길이가 126보다 작으면 최소 길이 표현이 아니다. 표식 127의 길이가 65536보다 작아도 같은 오류다. 64비트 길이의 최상위 비트도 0이어야 한다. 위반은 `WsProtocolError`(`ws_protocol_error`)이며 Close 1002로 대응한다. 유효한 길이가 M을 넘는 경우의 1009와 구별한다. 확장 바이트가 덜 왔으면 헤더를 더 기다린다.

이는 [RFC 6455 5.2절](https://www.rfc-editor.org/rfc/rfc6455#section-5.2)의 검사다. 원시 검사와 `FrameHeader::parse`를 함께 쓰는 구현은 사용자 결정 2026-11-01(Q77)의 코덱 경계를 따른다. 검사 위치는 **이 문서의 판단, 사용자 결정 없음**이다.

M 초과는 프레임 헤더만으로도 알 수 있다. 이때 해당 프레임의 데이터를 더 전달하지 않고 1009 경로로 간다. 앞선 프레임·청크의 이미 전달한 바이트는 되돌리지 않는다. B 초과를 메시지 크기 오류로 바꾸거나, 프레임 분할로 M 검사를 우회하지 않는다.

## 4. 101부터 공통 서비스까지

다음은 현재 인계 흐름이다. 함수 연결 방식은 **이 문서의 판단, 사용자 결정 없음**이다. 전체 연결 순서는 [세션 설계 8절](relay-session.md#8-현재-코드와의-차이와-연결-순서)을 따른다.

1. 현재 U·출발지 IP·HTTP·Origin 검사 순서를 유지한다. `read_head`는 HTTP head뿐 아니라 후속 원시 바이트도 보존한다.
2. U 안에서 101 응답을 모두 쓰고 `flush`한다. 완료 직후 공통 `Clock::monotonic()`을 한 번 읽는다. 이것이 `ConnectionContext.exchange_at`이다.
3. `conn_id`, 실제 상대 IP·포트, `accepted_wall`, `accepted_at`은 수락 때의 값을 보존한다. 101 완료 시각을 수락 시각으로 덮어쓰지 않는다.
4. `WsRelayStream<Wire>`에 선행 바이트와 검증된 M·B·전송 종류를 준다. 이미 보낸 101을 라이브러리 수락 함수로 다시 보내지 않는다.
5. `handle_connection(stream, context, permit, shutdown)`을 호출한다. `shutdown`은 기존 `watch::Receiver<RelayLifecycle>`이다. `Running`, `Grace { deadline }`, `Force`를 다른 bool 수명으로 바꾸지 않는다.
6. 시작 바이트는 **공통 서비스만** 쓴다. 어댑터는 최초 12바이트를 FIN이 있는 바이너리 프레임 하나에 담는다. 부분 소켓 쓰기로 나뉘어도 프레임·메시지 수는 하나다. 진입점에서 별도로 greeting을 보내지 않는다.
7. greeting의 전송·필요한 `flush`와 요청 읽기에 같은 절대 `exchange_at + H`를 쓴다. 현재 공통 코드에는 greeting 뒤 `flush`가 없으므로, 하위 전송의 버퍼를 비워야 하는 구현이면 이 `flush`도 같은 H 안에 추가해야 한다. 청크·프레임·메시지마다 H를 다시 주지 않는다.
8. 성공한 `PreparedConnection`은 즉시 기존 `RelayService::dispatch`에 이동한다. Mode I·II의 C·W·매칭·중계는 app이 맡는다. 실패는 5절의 close action과 사건으로 끝낸다.
9. `hold_upgraded`는 `638cf09`에서 제거되었다. WS·WSS는 공통 서비스(`handle_connection` → `dispatch`)에 연결되어 있다. 2.7 완료 표시는 이 문서가 정하지 않는다.

`WsPolicy`의 `message_bytes`와 `buffer_bytes`가 M·B를 전달한다. Kw 설정은 아직 이 경계에 주입하지 않는다. 작업 3.4가 §9.5의 정책과 진행 경계를 함께 다룬다. WS·WSS는 같은 어댑터를 쓴다. TLS는 기존 `Wire::Tls`가 맡는다.

## 5. 기존 `RelayStream`과의 대응

### 5.1 현재 포트에 맞춘 동작

기준은 [`crates/app/src/ports.rs`](../../crates/app/src/ports.rs)의 실제 선언이다. `RelayStream: AsyncRead + AsyncWrite + Unpin + Send`이며 `transport()`와 `poll_close(Pin<&mut Self>, &mut Context<'_>, RelayCloseAction) -> Poll<io::Result<()>>`를 제공한다. `BoxRelayStream`은 `Box<dyn RelayStream>`이다. 별도 `Pin<Box<...>>` 별칭이나 WS 전용 app 스트림 인터페이스를 만들지 않는다.

| 기존 메서드 | WS·WSS 대응 |
| --- | --- |
| `transport()` | 실제 진입점에 따라 `ViewerTransport::Ws` 또는 `ViewerTransport::Wss` |
| `poll_read` | 바이너리 페이로드만 반환. 완성 메시지를 기다리지 않음. 유효한 Close를 받으면 앞선 페이로드를 모두 반환한 뒤 읽기 EOF. 빈 바이너리·Ping·Pong은 EOF 아님 |
| `poll_write` | 서버 마스크 없이 바이너리 메시지를 씀. 선언 길이는 M 이하. 헤더 쓰기 길이는 반환값에서 제외. 실제 하위 전송에 쓴 페이로드 길이만 `Ready(Ok(n))`으로 반환 |
| `poll_flush` | 입력을 소비하지 않음. 진행 중 데이터 프레임이 없을 때 대기 제어 출력을 진행. 데이터 프레임 도중에는 제어를 삽입하지 않고 하위 전송만 flush. 새 데이터 메시지를 만들지 않음 |
| `poll_shutdown` | 단독 전체 종료 의미로 쓰지 않음. 공통 엔진은 종료 의도를 `poll_close(action)`으로 전달 |
| `poll_close` | 같은 action을 반복 poll해도 Close를 중복 생성하지 않음. Close 완료가 소켓·permit 반환을 대신하지 않음. E는 호출자인 app이 제한 |

출력 헤더와 남은 프레임 길이는 어댑터 상태에 둔다. 페이로드는 app 방향 버퍼에 남긴다. 현재 세션 설계처럼 부분 쓰기가 끝날 때까지 그 미전달 범위를 유지한다. 전달 카운터는 각 부분 성공 길이만 늘린다. greeting·250바이트 요청·헤더·제어 프레임·폐기 데이터는 제외한다.

프레임 도중 취소된 출력에 Close 헤더를 끼워 넣으면 프레임이 깨진다. 초기 greeting은 고정된 12바이트이므로 어댑터가 그 진행 위치를 보존할 수 있다. 활성 세션의 미완료 출력은 app 버퍼와 협력해야 한다. 5.4절의 공통 인터페이스와 9.4절의 해결 판단을 따른다. 이 문제를 숨기려고 전송하지 않은 큐 복사량을 전송량으로 세지 않는다.

### 5.2 action과 Close 코드

아래 대응은 [계약 1.5절](../contracts/websocket-relay-v1.md#15-close-코드)의 값이다. `poll_close`는 가능한 경우 고정 reason과 함께 전송한다. 전송 불능·E 만료이면 소켓을 반환한다. Close를 보내기 위해 E를 연장하지 않는다. `1005`, `1006`, `1015`는 wire에 쓰지 않는다.

| 기존 `RelayCloseAction` | WS·WSS 동작 |
| --- | --- |
| `HandshakeInvalid` | 4008 `handshake_invalid` |
| `HandshakeTimeout` | 4009 `handshake_timeout` |
| `ModeDisabled` | 1008 `mode_disabled` |
| `RejectDuplicate` | 4004 `duplicate_id` |
| `WaitingExpired` | 4001 `wait_timeout` |
| `WaitingFull` | 4010 `waiting_full` |
| `TargetConnectFailed` | 4002 `target_connect_failed` |
| `TargetNotAllowed` | 4003 `target_not_allowed` |
| `WriteHalf` | 반대 TCP EOF 뒤의 drain을 마치면 Close 1000. WS 읽기는 상대 Close 또는 같은 E까지 유지 |
| `PeerEof` | 저장한 수신 Close 코드에 응답. 코드가 없으면 1000. 상대 reason 원문은 반사하거나 로그에 넣지 않음 |
| `StreamError(WsProtocolError)` | 1002, reason 없음 |
| `StreamError(WsTextMessage)` | 1003, reason 없음 |
| `StreamError(WsMessageTooLarge)` | 1009, reason 없음 |
| `StreamError(IoError)` | 반대 TCP 오류이면 4007 `peer_closed`를 최선 노력으로 보냄. 자기 WS 전송이 이미 깨졌으면 Close 전달을 보장하지 않음 |
| `StreamError(TlsFailed)` | 자기 WSS 전송 실패를 보존. 깨진 TLS 위에 Close를 새로 보낼 수 있다고 가정하지 않음 |
| `DeadConnection` | 반대 TCP의 끊긴 연결이면 4006 `dead_connection`. 자기 WS의 Pong 기한 만료 상태이면 Close 없이 TCP를 닫음 |
| `AdminTerminated` | 4005 `admin_terminated` |
| `ServerShutdown` | 아직 정리를 시작하지 않은 세션에 1001 `server_shutdown`. 이미 받은 Close나 대기 중 응답을 덮어쓰지 않음(사용자 결정 2026-11-01(Q82)) |
| `TargetConnectionLimit` | 4011 `relay_connection_limit`. Mode I에서 101 성공 후 대상 연결을 위한 중계 연결 상한 L 슬롯이 없을 때(사용자 결정 2026-11-01(Q79)) |

Force의 action은 [세션 설계 5.2절](relay-session.md#52-정상-종료와-g)에서 보존한 원인으로 고른다. 클라이언트 Close가 먼저면 `PeerEof`로 같은 코드에 응답한다. TCP EOF가 먼저면 `WriteHalf`로 1000을 보낸다. 뒤늦은 클라이언트 Close가 이 선택을 바꾸지 않는다. 형식 있는 오류는 `StreamError(kind)`를 유지한다. 이미 전송한 Close는 다시 보내지 않는다. 최초 원인 보존은 사용자 결정 2026-11-01(Q82)다. 수신 Close 앞에 시작한 데이터 프레임만 마치는 범위는 같은 날의 Q83이다.

`encode_close`는 최대 127바이트의 고정 배열을 쓴다. 코드 뒤 reason은 최대 123바이트다. 잘라야 하면 UTF-8 문자 경계에서 자른다. 이 저장 방식은 **이 문서의 판단, 사용자 결정 없음**이다.

1011 `internal_error`는 계약에 있다. 하지만 현재 action에는 내부 오류를 뜻하는 값이 없다. `TargetConnectionLimit`나 모든 I/O 오류를 1011로 바꾸지 않는다. 실제 내부 오류 경로를 추가한다면 그 의도를 기존 action에 구별해 넣어야 한다. 이는 **이 문서의 판단, 사용자 결정 없음**이다.

### 5.3 형식 있는 오류와 사건

잘못된 프레임·텍스트·M 초과는 기존 `RelayStreamError::new(kind).into_io_error()`로 반환한다. app은 `RelayStreamError::classify(&error)`로 같은 종류를 얻는다. 라이브러리 오류 문자열이나 원시 페이로드를 전달하지 않는다.

| 관찰 | `ConnectionErrorKind` / 사유 | 초기 교환 사건·Close |
| --- | --- | --- |
| 요청 문법 오류 | `RejectReason::HandshakeInvalid` | `HandshakeRejected` → JSON `connection_rejected`, `stage = initial_exchange`, 4008 |
| 꺼진 모드 | `RejectReason::ModeDisabled` | `HandshakeRejected` → `connection_rejected`, `mode` 포함, 1008 |
| 유효한 WS Close가 250바이트 전에 옴 | `AbortReason::PeerEof` | `ConnectionAborted`, `reason = peer_eof`. `handshake_incomplete`의 운영 대응. 받은 코드 응답, 없으면 1000 |
| 절대 H 만료 | `AbortReason::HandshakeTimeout` | `ConnectionAborted`, `handshake_timeout`, 4009 |
| 프레임 규칙 위반 | `WsProtocolError` → `ws_protocol_error` | `ConnectionAborted`, `connection_error`, 1002 |
| 첫 text 프레임 | `WsTextMessage` → `ws_text_message` | `ConnectionAborted`, `connection_error`, 1003. 텍스트를 TCP에 전달하지 않음 |
| 논리 M 초과 | `WsMessageTooLarge` → `ws_message_too_large` | `ConnectionAborted`, `connection_error`, 1009 |
| Close 없는 소켓 EOF·읽기/쓰기 오류 | `IoError` → `io_error` | `ConnectionAborted`, `connection_error`. 정상 WS 읽기 EOF로 위장하지 않음 |
| WS Pong 기한 만료 | `AbortReason::DeadConnection` | `ConnectionAborted`, `dead_connection`. Close 없이 소켓 종료. 이 사유를 전달할 진행 경계는 5.4절에서 추가 필요 |
| 서버 종료 | `AbortReason::ServerShutdown` | `ConnectionAborted`, `server_shutdown`, 1001 |

사건 필드는 [운영 정보 계약 1.5~1.7절](../contracts/ops-info-v1.md#15-사건-목록)이 기준이다. 어댑터는 사건을 직접 기록하지 않는다. app이 최초 원인 하나를 선택한다. 초기 실패는 E 안에서 action 진행 → 스트림 drop → L permit drop → 사건 한 번 순서다. Close 실패가 원래 사유를 바꾸지 않는다.

같은 오류가 대기 중이면 `waiting_removed`, 활성 뒤이면 `session_closed`와 `close_cause = connection_error`, 같은 `error_kind`로 기록한다. 죽은 연결은 `dead_connection`이다. `connection_aborted`와 `session_closed`를 한 연결 실패에 함께 만들지 않는다. 서버의 G 처리는 기존 `RelayLifecycle`과 공통 정리 소유자를 재사용한다.

### 5.4 필요한 공통 인터페이스 변경

초기 실패 Close action, 형식 있는 I/O 오류와 `pending_write_len()`은 현재 포트에 있다. 기본 제어 출력에는 기존 `poll_flush`를 쓴다. 아래 연결 방식은 **이 문서의 판단, 사용자 결정 없음**이다.

| 경계 | 현재 구현과 남은 범위 |
| --- | --- |
| app이 페이로드 읽기·쓰기를 poll하지 않을 때 대기 제어 출력 진행 | `run_session`의 활성 소유 루프가 WS Viewer의 `poll_flush(cx)`를 별도로 poll한다. `Pending`이어도 양방향 중계를 계속 poll한다. 오류는 `RelayStreamError::classify`로 분류한다. Viewer 출력 오류로 기존 정리 경로에 전달한다. |
| 미완료 WS 출력 프레임의 경계 | `pending_write_len()`이 이미 헤더를 전송하기 시작한 프레임의 잔여 페이로드 길이를 알린다. app의 기존 미전달 범위로 같은 E 안에서 마친다. 사용자 결정 2026-11-01(Q83)의 범위다. |
| WS Close 뒤의 반대 방향 처리 | `first_eof()`로 먼저 종료한 쪽을 보존한다. drain·TCP 쓰기 종료·Close 응답·반대 방향 폐기를 구분한다. [세션 설계 4.4절](relay-session.md#44-ws를-나중에-연결하는-경계)이 흐름을 설명한다. |
| Kw의 주기·기한과 새 제어 입력 | `poll_maintenance`는 아직 제안이다. H·C·W·활성·closing 전체의 Kw 진행 경계는 작업 3.4에서 검토한다. 현재 `poll_flush`가 이를 모두 구현했다고 보지 않는다. |
| 대상 L 부족 action | `RelayCloseAction::TargetConnectionLimit`로 4011을 선택한다. 사용자 결정 2026-11-01(Q79)다. |

현재 제어 진행 poll인 `poll_flush`는 입력을 소비하지 않는다. B가 찬 데이터 페이로드를 건너뛰지 않는다. 이미 관찰한 Ping의 Pong만 페이로드 방향과 독립적으로 진행한다. 정리 중에는 `poll_close`가 출력을 진행한다. 새 Ping이 B 뒤에 가려진 경우는 §9.5 Kw와 TODO 작업 3.4의 열린 범위다. `DeadConnection` 전달을 포함한 새 유지보수 포트도 그 범위와 구별해 검토한다.

## 6. 제어 프레임·Kw·종료 상태

다음 상태 분리는 **이 문서의 판단, 사용자 결정 없음**이다. 라이브러리의 자동 Close를 그대로 쓰면 app의 drain 순서를 보장할 수 없으므로 명시적으로 제어한다.

- 읽기는 헤더, 데이터 페이로드, 제어 페이로드, Close 수신 상태를 구별한다. 제어 페이로드의 남은 길이는 매 읽기마다 현재 `DecodeState::ReadingControl`에서 다시 구한다. `drain_leftover_to_control` 뒤의 값도 반영한다. 남은 길이까지만 읽는다. 다음 프레임 바이트를 읽어 버리지 않는다.
- text는 첫 text 프레임의 헤더에서 1003으로 거절한다. 잘못된 마스크·RSV·opcode·continuation·제어 길이·Close 본문은 1002다. 최소 길이 표현 위반과 64비트 길이의 최상위 비트 위반도 1002다. 유효하지 않은 제어를 데이터로 돌려주지 않는다.
- Ping에는 같은 페이로드의 Pong을 보낸다. Close를 받은 뒤에는 새 Pong을 만들지 않는다. `decoder.pending_pong`은 아직 한 바이트도 보내지 않은 Pong이다. 새 Ping은 이 슬롯만 최신 값으로 바꿀 수 있다([RFC 6455 5.5.3절](https://www.rfc-editor.org/rfc/rfc6455#section-5.5.3)).
- `active_pong`은 전송을 시작한 Pong을 보존한다. `written > 0`이면 새 Ping으로 바꾸지 않는다. 그 Pong을 끝내기 전에는 다른 Pong, 데이터 프레임이나 Close를 시작하지 않는다. `pending_close`의 일부를 보냈을 때도 새 Pong을 끼워 넣지 않는다. 미송신 Pong은 Close를 앞두고 버릴 수 있다.
- 제어 프레임은 데이터 **프레임 사이**에만 넣는다. `poll_read`, `poll_write`, `poll_flush`, `poll_close`가 같은 경계를 지킨다. 데이터 프레임의 헤더나 페이로드 중간에는 제어를 넣지 않는다. 사용자 결정 2026-11-01(Q83)에 따라 app이 이미 보유한 잔여 페이로드만 마친다.
- Pong 쓰기나 하위 전송의 flush가 `Pending`이면 상태와 쓰기 wake 등록을 유지한다. 다음 읽기 poll에서도 안전한 경계에서 다시 진행한다. 쓰기 오류를 버리지 않는다. 소유 루프의 독립 `poll_flush`가 페이로드 방향의 backpressure 중에도 이미 대기한 Pong을 진행한다. 입력은 읽지 않는다.
- Close 코드는 유효성·UTF-8을 검사한다. 코드와 수신 여부만 종료 판단에 보존한다. 상대 reason 원문은 응답·로그·TCP 데이터에 넣지 않는다. Close를 보낸 뒤 새 데이터 쓰기는 거절한다.
- `first_eof()`가 WS Viewer이면 앞선 WS 데이터를 TCP로 소진한다. TCP 쓰기를 닫은 뒤 같은 유효 코드로 응답한다. 수신 Close 전에 시작한 출력 프레임의 잔여만 같은 E 안에서 마친다. 새 메시지는 시작하지 않는다. 사용자 결정 2026-11-01(Q83)의 범위다.
- `first_eof()`가 반대 TCP이면 받은 데이터를 WS로 소진한 뒤 `WriteHalf`로 Close 1000을 보낸다. 그 전에 클라이언트 Close가 와도 echo 경로로 바꾸지 않는다. 같은 E 동안 허용된 WS 데이터는 TCP로 전달한다.
- `poll_terminal_disposal`은 입력 상태를 두 경우로 나눈다. 유효한 프로토콜의 terminal Close는 프레임을 계속 해석한다. 데이터 페이로드는 전달량에 넣지 않고 버린다. 상대 Close 또는 전송 EOF를 관찰하면 쓰기 방향을 닫고 완료한다. 상대가 TCP를 먼저 닫을 때까지 기다리지 않는다.
- 1002·1003·1009로 거절한 입력은 프레임 해석을 신뢰할 수 없다. 이때는 Close를 flush하고 쓰기를 닫는다. 이어 원시 입력을 EOF까지 제한된 횟수로 버린다. 거절한 페이로드 안의 Close 같은 바이트를 응답으로 오인하지 않는다. 유효한 terminal 정리 중 프레임 검사에 실패해도 이 원시 폐기 경로로 전환한다.
- 두 폐기 경로 모두 poll당 처리를 제한한다. E·G·Force의 절대 상한은 app이 유지한다. 별도 WS 유예 시간은 없다. 어댑터 drop 뒤 소켓을 가진 배경 작업도 남기지 않는다. 상태 슬롯, 재시도와 폐기 경로의 구현 배치는 **이 문서의 판단, 사용자 결정 없음**이다.

Kw는 빈 Ping의 주기와 Pong 기한을 각각 가진다. 일반 데이터 수신은 Pong 응답이 아니다. 요청하지 않은 Pong은 생존 판정을 갱신하지 않는다. Pong 기한 초과에는 Close 없이 TCP를 닫는다. 1006은 상대의 관찰 값이지 송신 코드가 아니다([계약 1.4~1.6절](../contracts/websocket-relay-v1.md#14-데이터-메시지와-제어-프레임)). 여러 Ping의 관리 방식은 9.5절의 미결정 사항이다. 적용 작업은 3.4이며 2.7 완료를 막지 않는다.

B가 찬 데이터 프레임 뒤의 새 Ping·Pong·Close는 아직 읽을 수 없다. 같은 TCP 바이트열에서 그 뒤 제어만 골라 읽을 수는 없다. `poll_flush`도 이 한계를 바꾸지 않는다. 이미 대기한 Pong의 출력 진행과 새 Ping의 수신을 구별한다. 새 제어 입력의 관찰 지연과 Kw 기한 정책은 §9.5 및 TODO 작업 3.4의 열린 범위다. 2.7의 WS 스트림 인계와 별도 범위다.

2.7 인계에는 실제 바이너리 스트리밍, M 검사, text·프로토콜 오류, 기본 Ping/Pong·Close 경계가 필요하다. Kw 전체 수용 기준은 3.4가 소유한다. 2.7 때문에 Kw를 가짜 성공 처리하거나 3.4가 진행할 수 없는 읽기 전용 래퍼를 만들지 않는다.

## 7. 테스트 계획

아래 표는 현재 코드에 있는 시험과 작업 3.4의 계획을 구분한다. 실행 결과 표가 아니다. 스트림 단위 시험은 메모리 duplex와 부분 쓰기 전송을 쓴다. 세션 시험은 제어 가능한 `Clock`과 tokio 정지 시각을 쓴다.

표식은 소스의 `/// spec:` 값이다. `partial`은 해당 시나리오의 일부만 확인한다는 뜻이다. 표식 없는 내부 규칙 시험도 따로 적는다. 현재 표에 없는 외부 상호운용 범위를 통과했다고 보지 않는다.

| 구체적 파일·테스트 이름 | 표식 | 확인할 내용·남은 범위 |
| --- | --- | --- |
| `crates/adapters/src/upgrade.rs::preserves_bytes_after_the_http_head` | `websocket.md:WS-018 partial` | HTTP head와 함께 들어온 프레임 바이트를 손실 없이 반환. H·중계 인계는 통합 시험에 남음 |
| `crates/adapters/src/websocket_stream/tests.rs::sends_the_greeting_as_one_binary_message` | `relay.md:RLY-001 partial` | 공통 서비스의 한 번 쓰기가 12바이트 바이너리 메시지 하나가 됨. 실제 진입점은 남음 |
| 같은 파일의 `keeps_initial_exchange_bytes_across_frames_and_messages` | `relay.md:RLY-002 partial`, `websocket.md:WS-007 partial` | 요청을 청크·프레임·메시지로 나눠도 동일. 빈 바이너리를 EOF로 보지 않음 |
| 같은 파일의 `preserves_initial_exchange_trailing_bytes_once_within_b` | `websocket.md:WS-018 partial`, `relay.md:RLY-003 partial` | 250바이트와 후속 데이터를 한 메시지·프레임에 넣음. `PreparedConnection`과 잔여 프레임에서 정확히 한 번 나옴. 최종 TCP 도착은 남음 |
| 같은 파일의 `streams_a_single_frame_larger_than_b_without_a_message_buffer` | `websocket.md:WS-011 partial`, `relay.md:RLY-018 partial` | B = 16 KiB, M = 1 MiB에서 64 KiB 단일 프레임을 청크로 전달. 실제 두 전송의 자원 상한은 별도 시험 |
| 같은 파일의 `counts_m_across_fragments_and_rejects_m_plus_one` | `websocket.md:WS-010 partial` | 단일·분할 모두 M은 허용, M+1은 1009. 이전에 전달한 prefix를 되돌리지 않음 |
| 같은 파일의 `rejects_text_and_invalid_frames_with_typed_errors` | `websocket.md:WS-008 partial`, `websocket.md:WS-019 partial` | text 1003. 마스크·RSV·opcode·continuation·제어 분할·길이·Close UTF-8 오류는 1002. `error_kind` 보존. 전체 세션 정리는 남음 |
| 같은 파일의 `answers_interleaved_ping_without_forwarding_control_payload` | `websocket.md:WS-009 partial` | 분할 사이 Ping의 Pong 일치. Pong·Ping·Close가 데이터 스트림에 없음. 양쪽 실제 전송은 남음 |
| 같은 파일의 `reads_only_the_remaining_control_payload_before_the_next_frame` | `websocket.md:WS-009 partial` | Ping의 일부가 선행 버퍼에 있어도 남은 길이만 읽음. 다음 바이너리 페이로드를 보존 |
| 같은 파일의 `emits_pong_only_after_partial_data_headers_and_payloads_finish` | `websocket.md:WS-009 partial` | 부분 데이터 헤더·페이로드 뒤에서만 Pong을 보냄. 읽기와 flush 경로를 함께 확인 |
| 같은 파일의 `preserves_an_active_pong_and_coalesces_only_unsent_pongs` | `websocket.md:WS-009 partial` | 일부를 보낸 Pong은 보존. 아직 보내지 않은 Pong만 최신 값으로 합침 |
| 같은 파일의 `retries_an_idle_pong_on_write_wake_without_application_writes` | `websocket.md:WS-009 partial` | 쓰기 wake 뒤 읽기 poll만으로 Pong을 재시도. 추가 입력은 보내지 않음 |
| 같은 파일의 `shutdown_after_received_close_preserves_the_peer_code`, `shutdown_does_not_replace_an_echo_with_a_partial_header` | 각각 `operations.md:OPS-042 partial` | 서버 종료 요청이 수신 Close나 부분 echo의 코드를 1001로 바꾸지 않음 |
| 같은 파일의 `finishes_started_frames_before_replying_to_close_within_e` | `websocket.md:WS-017 partial` | 이미 시작한 프레임을 마친 뒤 Close에 응답. 절대 E 만료는 세션 시험에서 확인 |
| 같은 파일의 `bounds_outbound_messages_and_counts_only_written_payload` | `websocket.md:WS-007 partial` | 출력 메시지 ≤ M. 부분 헤더·부분 payload·오류에서 프레이밍과 미전달 바이트를 카운터에서 제외 |
| `crates/adapters/src/websocket_stream/frame.rs::rejects_noncanonical_lengths_at_every_header_split` | `websocket.md:WS-019 partial` | 모든 헤더 분할 위치에서 비최소 길이 표현과 64비트 최상위 비트를 `WsProtocolError`로 거절 |
| 같은 파일의 `accepts_canonical_length_boundaries_and_distinguishes_m_plus_one` | `websocket.md:WS-010 partial` | 125·126·65535·65536 경계의 유효 표현과 M+1 크기 오류를 구분 |
| `crates/adapters/tests/websocket_relay.rs::invalid_initial_requests_close_4008_and_log_the_rejection` | `relay.md:RLY-004 partial` | WS·WSS 초기 요청 오류의 4008, 사건, 등록·예산 정리 |
| 같은 파일의 `incomplete_initial_exchange_closes_4009_and_logs_h_timeout` | `relay.md:RLY-005 partial`, `operations.md:OPS-006 partial` | WS·WSS에서 H 만료의 4009와 `handshake_timeout` 기록 |
| 같은 파일의 `disabled_modes_close_1008_before_registration_or_target_connect` | `relay.md:RLY-022 partial` | 모드 비활성의 1008과 등록·대상 연결 차단 |
| 같은 파일의 `ws_and_wss_forward_initial_trailing_data_once` | `websocket.md:WS-018`, `websocket.md:WS-003`~`WS-006`, `relay.md:RLY-003 partial` | WS·WSS의 Mode II 양 접속 순서에서 후속 데이터를 한 번 전달. 표식은 현재 소스 값이며 실제 외부 Viewer 화면 시험의 결과를 뜻하지 않음 |
| 같은 파일의 `client_close_drains_to_tcp_and_echoes_only_the_code` | `websocket.md:WS-016 partial` | 클라이언트 Close 선행의 drain·같은 코드 응답 |
| 같은 파일의 `tcp_eof_drains_then_forwards_client_data_after_close_1000` | `websocket.md:WS-016 partial`, `relay.md:RLY-019 partial` | TCP EOF 선행의 drain·1000과 허용된 반대 방향 전달 |
| 같은 파일의 `close_timeout_in_either_order_releases_only_its_session` | `websocket.md:WS-017 partial`, `relay.md:RLY-021 partial` | 두 종료 순서의 E 만료와 다른 세션 격리 |
| 같은 파일의 `text_frame_closes_1003_without_forwarding_text_and_logs_typed_error` | `websocket.md:WS-008` | text와 본문을 함께 보내도 1003을 받음. TCP 미전달과 `ws_text_message` 기록 |
| 같은 파일의 `single_and_fragmented_messages_accept_m_and_close_1009_at_m_plus_one` | `websocket.md:WS-010` | WS·WSS의 분할·비분할 M 수락. M+1 헤더와 거절할 본문을 한 쓰기로 보냄. 1009, 추가 미전달, `ws_message_too_large`와 자원 반환 확인 |

다음은 표식 없는 내부 회귀 시험이다. 경로는 `crates/adapters/src/websocket_stream/` 기준이다.

| 파일·테스트 | 확인할 내부 규칙 |
| --- | --- |
| `tests.rs::finishes_active_controls_before_normal_and_terminal_close` | 부분 Pong과 부분 Close의 순서를 보존. 정상·terminal Close 모두 포함 |
| `tests.rs::propagates_pong_write_errors_from_reads`, `flushes_idle_pongs_through_a_buffered_transport` | Pong 쓰기 오류 전달과 버퍼가 있는 전송의 flush |
| `tests.rs::terminal_close_completes_on_peer_close_without_transport_eof` | 유효한 Close 응답 뒤 서버가 TCP 종료를 진행. 상대 TCP EOF를 기다리지 않음 |
| `tests.rs::valid_terminal_disposal_yields_during_a_data_flood` | 정상 프레임 폐기의 poll당 처리 제한 |
| `tests.rs::rejected_frame_disposal_waits_for_transport_eof_not_apparent_close_bytes` | 거절한 본문 안의 Close 같은 바이트를 해석하지 않음 |
| `tests.rs::rejects_new_data_after_sending_close` | Close 뒤 새 데이터 프레임 차단 |
| `frame.rs::encodes_close_reason_beyond_the_old_28_byte_limit`, `encodes_maximum_close_payload_and_truncates_on_utf8_boundaries` | 29바이트 reason, 최대 Close 본문과 UTF-8 경계 |

`rotates_the_mask_across_every_chunk_split`, `keeps_upgrade_suffix_and_app_payload_under_one_b`, `yields_during_empty_message_and_control_floods`, `close_actions_are_idempotent`도 현재 표식 없는 단위 시험이다. 마스크 위치, 보유 바이트, 양보와 중복 Close 방지를 각각 확인한다.

소유 루프의 `drives_queued_and_partial_pongs_while_payload_writes_are_backpressured`와 `classifies_control_flush_errors_as_viewer_output_errors`는 `crates/app/src/relay/session.rs`에 있다. 둘 다 표식 없는 내부 시험이다. [세션 설계 7절](relay-session.md#7-설계-선택과-테스트-계획)이 Force·`first_eof`·사건 순서의 회귀 시험도 정리한다.

작업 3.4의 Kw 시험은 아직 계획이다. `drives_kw_without_payload_io_and_bounds_pending_pings`와 `ws_and_wss_kw_keeps_idle_peers_and_drops_unresponsive_peers`는 현재 시험으로 세지 않는다. §9.5 결정 뒤 빈 Ping의 주기, Pong 기한과 B 뒤의 새 제어 입력을 검증해야 한다. `WS-026` 표식도 실제 시험을 만들 때 범위에 맞춰 정한다.

실제 화면·입력과 모드별 WS·WSS 조합은 [WS-001~006](../specs/websocket.md#6-전송모드-검증-조합)의 별도 수용 시험이다. 이 문서의 스트림 단위 시험으로 대신하지 않는다. 외부 상호운용 도구의 소스는 읽지 않는다.

## 8. 현재와 목표의 차이

| 현재 | 목표와 선행 조건 |
| --- | --- |
| HTTP head 뒤 바이트 폐기 (해결, 638cf09) | `read_head`가 `suffix`를 보존해 `WsRelayStream`에 넘긴다 |
| `hold_upgraded` 보류 (해결, 638cf09) | `connection`이 `WsRelayStream`을 만들어 공통 `handle_connection` → `dispatch`로 넘긴다 |
| M·B 미주입 (해결, 638cf09) | `WsPolicy`의 `message_bytes`·`buffer_bytes`가 `WsRelayStream::new`에 주입된다 |
| 초기 실패 action·형식 있는 I/O 오류 포트 존재 | 그대로 사용. WS 코드만 어댑터에서 변환. 사건은 app 한 곳 |
| TCP 세션 중심의 app 흐름 | 현재 `run_session`은 `first_eof()`로 종료 순서를 보존한다. 독립 `poll_flush`가 이미 대기한 제어 출력을 진행한다. WS Close의 drain·폐기·E 경계도 같은 소유자가 처리한다. |
| Q21의 2.7 보류 (해결, 638cf09) | 보류는 제거되었다. Q21 문구와 TODO 3.2의 `현재:`는 바꾸지 않는다 |

`relay-session.md`는 현재 `poll_flush` 제어 진행, `first_eof`와 Force의 action 보존을 설명한다. `serve-startup.md`는 마지막 진입점 소유자가 끝난 뒤 감독 작업을 취소하고 합류하는 수명을 설명한다. §9.5 Kw는 TODO 작업 3.4의 열린 범위다. 이 문서가 그 작업의 완료 여부를 바꾸지는 않는다.

## 9. 미결정 사항

9.1~9.3절은 사용자 결정 2026-11-01(Q77, Q78, Q79)로 해결했다. 9.4절은 사용자 결정 2026-11-01(Q83)로 확정했다. 9.5절은 3.4 범위로 열어 두며 2.7 완료를 막지 않는다.

<a id="91-프레임-처리-구현의-선택"></a>

### 9.1 (해결) 프레임 처리 구현의 선택

사용자 결정 2026-11-01(Q77)로 선택 B(기존 헤더 API + 자체 스트리밍 상태 머신)를 확정했다. tungstenite 0.30.0의 `FrameHeader::parse`/`format`과 `derive_accept_key`만 재사용하고 나머지(페이로드 청킹, 인플레이스 언마스킹, 프레임 분할, 제어 프레임, Close 상태 머신)는 RFC 6455만으로 자체 구현한다. 새 crate 의존성은 추가하지 않는다([ADR-0005](../decisions/ADR-0005-in-house-ws-streaming-frame-codec.md)).

<a id="92-ws-tbd-004-m-b와-제어-진행"></a>

### 9.2 (해결) WS-TBD-004: M > B와 제어 진행

사용자 결정 2026-11-01(Q78)로 선택 B를 확정하고 WS-TBD-004를 해소했다. B와 M 사이에 크기 제약은 없다. 프레임 안에서 스트리밍하며 방향당 미전달 보관 바이트는 B 이하다. 분할 프레임을 합친 논리 메시지 크기가 M을 넘으면 1009로 닫고 추가 전달을 멈춘다. B보다 큰 유효 프레임도 수락해 스트리밍하며 전체를 저장하지 않는다.

### 9.3 (해결) 101 뒤 Mode I 대상 L 부족

사용자 결정 2026-11-01(Q79)로 선택 A를 확정했다. WebSocket 계약 1.5절과 2.2절에 새 코드 4011 `relay_connection_limit`을 추가했다. app은 사유 없는 `RelayCloseAction::Reject` 대신 `RelayCloseAction::TargetConnectionLimit`를 사용한다.

### 9.4 (해결) 수신 Close와 이미 시작한 출력 프레임

| 선택 | 장점 | 비용·충돌 |
| --- | --- | --- |
| A. 수신 Close 전에 헤더를 이미 쓴 프레임의 잔여만 같은 E 안에서 마치고 응답(선택) | 올바른 Close 프레임을 이어 쓸 수 있음. app 기존 버퍼로 가능 | 전송 중 프레임 경계 조회와 app 정리 협력 필요. 수신 뒤 TCP 데이터·새 메시지는 보내지 않음 |
| B. 잔여를 버리고 Close 헤더를 즉시 씀 | 겉보기 단순 | 진행 중 프레임의 페이로드로 해석되므로 잘못된 wire. 선택 불가 |
| C. 프레임이 진행 중이면 소켓을 바로 버림 | 추가 송신 없음, 점유 제한 쉬움 | E 안에 전송할 수 있는데도 같은 코드의 Close 응답을 생략하면 계약 1.6절 첫 행을 위반 |

사용자 결정 2026-11-01(Q83)로 선택 A를 확정했다. 대상은 Close 수신 전에 헤더를 이미 쓴 프레임뿐이다. app이 이미 보유한 잔여만 같은 E 안에서 마친 뒤 같은 코드로 Close에 응답한다. 수신 코드가 없으면 1000으로 응답한다. Close 수신 뒤에 읽은 TCP 데이터는 전달하지 않는다. 새 메시지도 시작하지 않는다.

따라서 [계약 1.6절 첫 행](../contracts/websocket-relay-v1.md#16-종료-의미)의 “Close 뒤 TCP에서 오는 데이터는 WS로 보내지 못하며 버린다”는 규칙을 유지한다. [RFC 6455 5.5.1절](https://www.rfc-editor.org/rfc/rfc6455#section-5.5.1)도 진행 중 메시지를 마친 뒤 응답할 수 있게 한다. B는 잘못된 wire를 만든다. C는 E 안에 전송할 수 있는 경우에도 계약의 같은 코드 응답을 생략하므로 선택하지 않는다. 전송 실패나 E 만료 때의 강제 자원 반환은 그대로 유지한다. 위 표는 검토한 대안의 기록이다.

### 9.5 Kw의 여러 Ping과 B backpressure

이 항목은 **3.4 범위의 미결정 사항**이다. 2.7 완료를 막지 않는다. 2.7의 기본 Ping/Pong·스트림 인계와 Kw의 생존 판정 정책을 구별한다.

| 선택 | 장점 | 비용·충돌 |
| --- | --- | --- |
| A. 주기 유지, 전송한 빈 Ping별 기한을 고정 크기 FIFO로 관리 | 각 Ping에 Pong 기한을 주는 계약 문장을 유지 | 빈 Pong은 식별자가 없어서 가장 오래된 대기를 응답 처리. B 뒤에 가려진 Pong도 기한 안에 관찰하지 못하면 종료될 수 있음 |
| B. Pong 하나를 기다리는 동안 다음 Ping을 미룸 | 한 개의 대기 상태만 필요 | 고정 주기 문장과 다름. 응답 대기로 생존 판정이 늦어짐 |
| C. backpressure 동안 Pong 시계를 멈추거나 식별자 Ping으로 바꿈 | 지연된 Pong의 오판 감소 또는 응답 구분 | 절대 응답 기한 또는 빈 Ping 계약 변경 필요. 무응답 점유 시간이 늘 수 있음 |

권고는 현재 계약을 유지하는 A다. 설정 범위상 동시 기한 슬롯은 `ceil(pong_timeout / ping_interval) + 1` 이하로 제한할 수 있다. 근거는 [데이터·제어 계약](../contracts/websocket-relay-v1.md#14-데이터-메시지와-제어-프레임)의 빈 Ping·각 Pong 기한, [설정 계약](../contracts/config-v2.md)의 범위다. FIFO는 응답의 실제 신원을 증명하지 않는다. RFC가 허용하는 Pong 합치기와도 구별한다. **이 해석과 backpressure 중 기한 정책은 3.4 전에 사용자 확인이 필요하다.** 이 문서에서 유예·식별자·새 기본값을 도입하지 않는다.

## 대조·실행 기록

- 초안 조사 대상: 커밋 `4478c9744e96e631e2224b6efe5ef7af4b4ccc37` 위의 작업 트리였다. 아래 API 조사와 문서 검사 수치는 당시 기록이다. 현재 구현 범위는 문서 앞부분과 §7에서 구분한다.
- 환경: Windows 11 x64. 소스·공식 버전 문서를 읽어 대조했다. 외부 API 자료는 조회 당시 값이다.
- 확인 방법: 잠금 버전의 `Stream`·프레임·메시지·헤더 API와 소스를 읽었다. Cargo 소스와 공개 버전 소스를 교차 확인했다. 유지 상태·크기·라이선스는 crates.io·저장소 API에서 읽었다.
- 초안 조사의 Rust 빌드·동작 시험: **미실행**이었다. 현재 스트림과 회귀 시험은 §7에 있다. 이번 문서 갱신에서는 소스와 표식을 읽었으며 시험을 다시 실행하지 않았다.
- 초안 당시 문서 검사: **통과**. 당시 `check_ws_changed_docs()`로 이 문서와 `architectures/code-map.md`, `TODO.md`를 검사했다. 로컬 링크 383개, 앵커 216개, 표 22개의 오류가 0개였다. 이 수치는 이번 개정의 검사 결과가 아니다. 이번 개정은 허용된 설계 문서 세 개만 수정했다. 코드와 시험 표식을 읽어 대조했다.
