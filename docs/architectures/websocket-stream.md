# WebSocket 스트림 어댑터 모듈 설계

이 문서는 WS·WSS의 바이너리 페이로드를 공통 `RelayStream`으로 옮기는 목표 구조를 정한다. 예를 들어 한 메시지에 250바이트 요청과 후속 데이터가 들어오면, 요청은 `handle_connection`이 해석한다. 후속 데이터는 같은 연결의 중계 스트림 앞부분으로 남는다.

| 항목 | 값 |
| --- | --- |
| 상태 | Draft |
| 구조 범위 | Target |
| 적용 작업 | 2.7 완료 전에 가져오는 3.3의 스트림 어댑터. 3.4의 Kw·종료 처리와 연결할 경계도 설계 |
| 기준 | [WebSocket 계약 1.4~1.6절](../contracts/websocket-relay-v1.md#14-데이터-메시지와-제어-프레임), [초기 교환 설계](initial-exchange.md), [세션 설계 3.2절](relay-session.md#32-전송-수명-인터페이스) |
| 결정 수준 | 9.1~9.3절은 사용자 결정 2026-11-01(Q77, Q78, Q79)로 해결했다. 9.4절은 사용자 결정 2026-11-01(Q83)로 확정했다. 9.5절은 3.4 범위이며 2.7을 막지 않는다. 채택된 계약을 바르게 반영함 |
| 구현 대조 | 현재 진입점·잠금 버전·공통 포트를 읽음. 스트림 어댑터는 미구현. 동작 시험은 미실행 |

별도 문서로 둔다. [기존 진입점 설계](websocket-entry-point.md)는 현재 HTTP·TLS 검사와 임시 보류를 설명한다. 이 문서는 101 뒤의 프레임·B·종료 수명만 맡는다. `Current`와 `Target`을 섞지 않기 위한 선택이다(**이 문서의 판단, 사용자 결정 없음**).

## 1. 책임과 확인한 사실

### 1.1 현재 코드

아래 표는 파일과 심볼을 읽어서 확인한 사실이다. 목표 동작을 현재 구현으로 표시하지 않는다.

| 위치 | 확인한 사실 |
| --- | --- |
| [`Cargo.lock`](../../Cargo.lock), [`Cargo.toml`](../../Cargo.toml) | 잠금 버전은 `tokio-tungstenite 0.30.0`, `tungstenite 0.30.0`이다. 직접 의존은 기본 기능을 끄고 `handshake`를 켠 `tokio-tungstenite`다. |
| [`crates/adapters/src/websocket.rs`](../../crates/adapters/src/websocket.rs)의 `upgrade` | `upgrade::read_head`와 자체 HTTP 검사를 쓴다. `switching_protocols`로 만든 101 응답을 `write_all`하고 `flush`한다. 라이브러리의 HTTP 수락 함수가 101을 보내는 구조가 아니다. |
| 같은 파일의 `WsPolicy`, `hold_upgraded` | `WsPolicy`에는 `accept`, `upgrade`, `origins`만 있다. M·B·Kw를 프레임 처리에 주입하지 않는다. `hold_upgraded`가 `WebSocketStream::from_raw_socket(wire, Role::Server, None)`을 만든다. 프레임을 읽지 않는다. 종료 요청에 1001, H 만료에 4009를 보내고 연결을 버린다. |
| [`crates/adapters/src/upgrade.rs`](../../crates/adapters/src/upgrade.rs)의 `read_head` | 1024바이트씩 읽고 HTTP head를 16 KiB로 제한한다. `\r\n\r\n` 뒤에서 `truncate(end)`하므로 함께 읽은 WS 바이트를 버린다. `HeadRead::Complete(Vec<u8>)`에는 head만 남는다. |
| 같은 파일의 `check`, `switching_protocols` | `tungstenite::handshake::derive_accept_key`를 쓴다. `binary` 서브프로토콜을 선택할 수 있다. 확장을 수락하지 않는다. |
| [`crates/app/src/relay/handshake.rs`](../../crates/app/src/relay/handshake.rs)의 `read_request` | Viewer일 때 `VIEWER_GREETING`을 한 번 `write_all`한다. 별도 `flush`는 없다. 요청을 나눠 읽고 250바이트 뒤의 데이터를 반환한다. |
| [`crates/app/src/relay.rs`](../../crates/app/src/relay.rs)의 `handle_connection`, `prepare`, `close_stream` | 성공하면 `PreparedConnection<S>`를 반환한다. 실패하면 사유별 `RelayCloseAction`을 E 안에서 진행한다. 이어 스트림·L 자리를 반환하고 사건을 기록한다. |

현재 `hold_upgraded`의 H는 그 함수 진입 때 시작한다. 목표 H는 **101 응답 전송 완료 시각**에서 시작한다. 두 시각을 같은 것으로 가정하지 않는다.

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

다음 경로와 심볼은 모두 `Target`이다. 타입 이름은 구현 때 공통 포트와 대조한다. 별도의 스트림 포트는 만들지 않는다. 아래 분리는 **이 문서의 판단, 사용자 결정 없음**이다.

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

M 초과는 프레임 헤더만으로도 알 수 있다. 이때 해당 프레임의 데이터를 더 전달하지 않고 1009 경로로 간다. 앞선 프레임·청크의 이미 전달한 바이트는 되돌리지 않는다. B 초과를 메시지 크기 오류로 바꾸거나, 프레임 분할로 M 검사를 우회하지 않는다.

## 4. 101부터 공통 서비스까지

다음 흐름은 `Target`이며 **이 문서의 판단, 사용자 결정 없음**이다. 구체적인 전체 연결 순서는 [세션 설계 8절](relay-session.md#8-현재-코드와의-차이와-연결-순서)을 따른다.

1. 현재 U·출발지 IP·HTTP·Origin 검사 순서를 유지한다. `read_head`는 HTTP head뿐 아니라 후속 원시 바이트도 보존한다.
2. U 안에서 101 응답을 모두 쓰고 `flush`한다. 완료 직후 공통 `Clock::monotonic()`을 한 번 읽는다. 이것이 `ConnectionContext.exchange_at`이다.
3. `conn_id`, 실제 상대 IP·포트, `accepted_wall`, `accepted_at`은 수락 때의 값을 보존한다. 101 완료 시각을 수락 시각으로 덮어쓰지 않는다. 현재 `net::Accepted`에는 모든 필드가 없으므로 메타데이터 전달도 연결 작업에 포함한다.
4. `WsRelayStream<Wire>`에 선행 바이트와 검증된 M·B·전송 종류를 준다. 이미 보낸 101을 라이브러리 수락 함수로 다시 보내지 않는다.
5. `handle_connection(stream, context, permit, shutdown)`을 호출한다. `shutdown`은 기존 `watch::Receiver<RelayLifecycle>`이다. `Running`, `Grace { deadline }`, `Force`를 다른 bool 수명으로 바꾸지 않는다.
6. 시작 바이트는 **공통 서비스만** 쓴다. 어댑터는 최초 12바이트를 FIN이 있는 바이너리 프레임 하나에 담는다. 부분 소켓 쓰기로 나뉘어도 프레임·메시지 수는 하나다. 진입점에서 별도로 greeting을 보내지 않는다.
7. greeting의 전송·필요한 `flush`와 요청 읽기에 같은 절대 `exchange_at + H`를 쓴다. 현재 공통 코드에는 greeting 뒤 `flush`가 없으므로, 하위 전송의 버퍼를 비워야 하는 구현이면 이 `flush`도 같은 H 안에 추가해야 한다. 청크·프레임·메시지마다 H를 다시 주지 않는다.
8. 성공한 `PreparedConnection`은 즉시 기존 `RelayService::dispatch`에 이동한다. Mode I·II의 C·W·매칭·중계는 app이 맡는다. 실패는 5절의 close action과 사건으로 끝낸다.
9. 이 실제 스트림 인계가 준비되면 `hold_upgraded`를 제거한다. Q21의 보류는 2.7 미완료 때만 유지한다. WS·WSS를 공통 서비스에 연결하고 시험하기 전에는 2.7 완료로 표시하지 않는다.

`WsPolicy`에는 M·B와 공통 서비스 인계에 필요한 값이 추가되어야 한다. Kw 설정은 6절의 진행 경계를 통해 주입한다. 이를 프레임 라이브러리의 기본 제한값으로 대신하지 않는다. WS·WSS는 같은 어댑터를 쓰며 TLS는 기존 `Wire::Tls`가 맡는다.

## 5. 기존 `RelayStream`과의 대응

### 5.1 현재 포트에 맞춘 동작

기준은 [`crates/app/src/ports.rs`](../../crates/app/src/ports.rs)의 실제 선언이다. `RelayStream: AsyncRead + AsyncWrite + Unpin + Send`이며 `transport()`와 `poll_close(Pin<&mut Self>, &mut Context<'_>, RelayCloseAction) -> Poll<io::Result<()>>`를 제공한다. `BoxRelayStream`은 `Box<dyn RelayStream>`이다. 별도 `Pin<Box<...>>` 별칭이나 WS 전용 app 스트림 인터페이스를 만들지 않는다.

| 기존 메서드 | WS·WSS 대응 |
| --- | --- |
| `transport()` | 실제 진입점에 따라 `ViewerTransport::Ws` 또는 `ViewerTransport::Wss` |
| `poll_read` | 바이너리 페이로드만 반환. 완성 메시지를 기다리지 않음. 유효한 Close를 받으면 앞선 페이로드를 모두 반환한 뒤 읽기 EOF. 빈 바이너리·Ping·Pong은 EOF 아님 |
| `poll_write` | 서버 마스크 없이 바이너리 메시지를 씀. 선언 길이는 M 이하. 헤더 쓰기 길이는 반환값에서 제외. 실제 하위 전송에 쓴 페이로드 길이만 `Ready(Ok(n))`으로 반환 |
| `poll_flush` | 진행 중 프레임을 임의로 끝내거나 별도 빈 메시지를 만들지 않음. 하위 전송을 flush |
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
| `ServerShutdown` | 1001 `server_shutdown` |
| `Reject(RelayConnectionLimit)` | 4011 `relay_connection_limit`. Mode I에서 101 성공 후 대상 연결을 위한 중계 연결 상한 L 슬롯이 없을 때(사용자 결정 2026-11-01(Q79)) |

1011 `internal_error`는 계약에 있다. 하지만 현재 action에는 내부 오류를 뜻하는 값이 없다. `Reject`나 모든 I/O 오류를 1011로 바꾸지 않는다. 실제 내부 오류 경로를 추가한다면 그 의도를 기존 action에 구별해 넣어야 한다. 이는 **이 문서의 판단, 사용자 결정 없음**이다.

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

초기 실패 close action과 형식 있는 읽기·쓰기 오류는 **현재 포트에 이미 있다**. 새로 필요한 것은 아래뿐이다. 아래 제안은 **이 문서의 판단, 사용자 결정 없음**이다. 이 문서는 `ports.rs`나 app 코드를 고치지 않는다.

| 부족한 경계 | 필요한 변경과 이유 | 적용 시점 |
| --- | --- | --- |
| app이 읽기·쓰기를 poll하지 않을 때 제어·타이머 진행 | 기존 `RelayStream`에 제어 진행 poll을 추가할 필요가 있다. 제안 형태는 `poll_maintenance(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<AbortReason>`다. 정상일 때 Pending과 wake 등록을 유지한다. 오류일 때만 `ConnectionError(kind)` 또는 `DeadConnection`을 알린다. TCP 구현은 Pending이다. 별도 스트림 trait나 소켓 소유 작업을 만들지 않는다. | 기본 제어 처리와 공통 인계를 연결할 때 검토. Kw의 완전한 동작은 3.4 |
| 미완료 WS 출력 프레임의 경계 | 기존 `RelayStream`에 남은 출력 프레임 길이를 조회할 수단이 필요하다. 제안은 `pending_write_len(&self) -> usize`다. TCP는 0이다. Close 수신 전에 헤더를 이미 쓴 프레임의 잔여만 같은 E 안에서 마저 쓴다. app이 이미 보유한 미전달 범위를 쓴다. 페이로드를 복사하는 병렬 쓰기 API는 만들지 않는다. | WS 세션 연결 전. Close 수신 때의 허용 범위는 9.4절 선택 A로 해결 |
| WS Close 뒤의 세션 반대 방향 처리 | [세션 설계 4.4절](relay-session.md#44-ws를-나중에-연결하는-경계)의 drain·TCP 쓰기 종료·Close 응답·반대 방향 폐기를 공통 엔진에 구현해야 한다. `transport()`와 읽기 EOF, 기존 action으로 구별한다. 폐기량을 `write` 성공으로 위장하지 않는다. | WS 세션 연결 전 |
| 대상 L 부족 `Reject` | 사유를 명시하는 `RelayCloseAction::Reject(RelayRejectReason::RelayConnectionLimit)`와 계약 코드 4011을 추가함(사용자 결정 2026-11-01(Q79)). | 해결(Q79) |

제어 진행 poll은 `poll_terminal`이나 out-of-band EOF 탐색이 아니다. B가 찬 데이터 페이로드를 건너뛰지 않는다. H·C·W·활성·closing의 각 소유 루프가 이 poll과 기존 절대 기한·종료 신호를 함께 진행해야 한다. `DeadConnection`을 `io_error`로 손실 없이 전달하는 데도 이 경계가 필요하다. 기존 `RelayStreamError`는 프레임 오류 종류를 그대로 맡는다.

## 6. 제어 프레임·Kw·종료 상태

다음 상태 분리는 **이 문서의 판단, 사용자 결정 없음**이다. 라이브러리의 자동 Close를 그대로 쓰면 app의 drain 순서를 보장할 수 없으므로 명시적으로 제어한다.

- 읽기는 헤더, 데이터 페이로드, 제어 페이로드, Close 수신 상태를 구별한다. 쓰기는 헤더·데이터 진행, 제어 전송, Close 전송 상태를 구별한다.
- text는 첫 text 프레임의 헤더에서 1003으로 거절한다. 잘못된 마스크·RSV·opcode·continuation·제어 길이·Close 본문은 1002다. 유효하지 않은 제어를 데이터로 돌려주지 않는다.
- Ping에는 같은 페이로드의 Pong을 보낸다. Close를 받은 뒤에는 새 Pong을 만들지 않는다. 이미 처리한 여러 Ping의 미송신 Pong은 최신 하나로 제한할 수 있다([RFC 6455 5.5.3절](https://www.rfc-editor.org/rfc/rfc6455#section-5.5.3)). 제어 입력·미송신 Pong·진행 중 출력의 슬롯 수를 고정한다.
- 제어 프레임은 데이터 **프레임 사이**에만 넣는다. 프레임 페이로드 한가운데 삽입하지 않는다. 빈 메시지·제어 폭주가 한 poll을 독점하지 않도록 처리량을 제한하고 다음 wake를 예약한다.
- Close 코드는 유효성·UTF-8을 검사한다. 코드와 수신 여부만 종료 판단에 보존한다. 상대 reason 원문은 응답·로그·TCP 데이터에 넣지 않는다.
- 상대 WS Close이면 앞선 WS 데이터 소진 → TCP 쓰기 종료 → 같은 유효 코드로 응답 → TCP EOF를 같은 E까지 기다린다. Close 뒤에 읽은 TCP 데이터는 버리고 전송량에 넣지 않는다. Close 수신 전에 헤더를 이미 쓴 출력 프레임의 잔여만 같은 E 안에서 마칠 수 있다. 새 메시지는 시작하지 않는다. 9.4절 선택 A의 판단이다.
- TCP EOF이면 앞선 데이터를 WS로 소진 → Close 1000 전송 → 상대 Close를 같은 E까지 기다린다. 그동안 들어온 WS 데이터는 TCP로 전달할 수 있다. 새 서버 데이터 메시지는 만들지 않는다.
- 양쪽 Close를 마치거나 E가 끝나면 하위 전송을 닫는다. 정상 종료 G나 강제 종료에 별도 WS 유예 시간을 덧붙이지 않는다. 어댑터 drop 뒤에도 소켓을 가진 배경 작업을 남기지 않는다.

Kw는 빈 Ping의 주기와 Pong 기한을 각각 가진다. 일반 데이터 수신은 Pong 응답이 아니다. 요청하지 않은 Pong은 생존 판정을 갱신하지 않는다. Pong 기한 초과에는 Close 없이 TCP를 닫는다. 1006은 상대의 관찰 값이지 송신 코드가 아니다([계약 1.4~1.6절](../contracts/websocket-relay-v1.md#14-데이터-메시지와-제어-프레임)). 여러 Ping의 관리 방식은 9.5절의 미결정 사항이다. 적용 작업은 3.4이며 2.7 완료를 막지 않는다.

B가 찬 데이터 프레임 뒤의 Ping·Pong·Close는 아직 읽을 수 없다. 같은 TCP 바이트열에서 그 뒤 제어만 골라 읽을 수는 없다. 배경 작업도 이 순서를 바꾸지 못한다. 제어 진행 인터페이스가 이 한계를 없앤다고 설명하지 않는다. backpressure 중 Pong 기한 정책은 9.5절에 따라 3.4에서 검증한다. 2.7의 WS 스트림 인계와 별도 범위다.

2.7 인계에는 실제 바이너리 스트리밍, M 검사, text·프로토콜 오류, 기본 Ping/Pong·Close 경계가 필요하다. Kw 전체 수용 기준은 3.4가 소유한다. 2.7 때문에 Kw를 가짜 성공 처리하거나 3.4가 진행할 수 없는 읽기 전용 래퍼를 만들지 않는다.

## 7. 테스트 계획

아래 파일·시험은 모두 `Target`이다. 실행 결과가 아니다. 바이너리 프레임 생성기는 RFC만 근거로 작성한다. 단위 시험은 메모리 duplex·부분 쓰기 스트림·제어 가능한 `Clock`·tokio 정지 시각을 쓴다. 보유 페이로드와 업그레이드 잔여 길이를 관찰해 합계 B를 검사한다.

표의 표식은 각 함수 위에 `/// spec: <표식>`으로 단다. 여러 표식은 줄을 나눈다. `partial`은 실제 WS·WSS 진입점, TCP 상대, 전체 수명 또는 실제 Viewer까지 검증하지 않은 시험이다. 기존 스펙의 수용 기준을 줄이지 않는다.

| 구체적 파일·테스트 이름 | 표식 | 확인할 내용·남은 범위 |
| --- | --- | --- |
| `crates/adapters/src/upgrade.rs::preserves_bytes_after_the_http_head` | `websocket.md:WS-018 partial` | HTTP head와 함께 들어온 프레임 바이트를 손실 없이 반환. H·중계 인계는 통합 시험에 남음 |
| `crates/adapters/src/websocket_stream/tests.rs::sends_the_greeting_as_one_binary_message` | `relay.md:RLY-001 partial` | 공통 서비스의 한 번 쓰기가 12바이트 바이너리 메시지 하나가 됨. 실제 진입점은 남음 |
| 같은 파일의 `keeps_initial_exchange_bytes_across_frames_and_messages` | `relay.md:RLY-002 partial`, `websocket.md:WS-007 partial` | 요청을 청크·프레임·메시지로 나눠도 동일. 빈 바이너리를 EOF로 보지 않음 |
| 같은 파일의 `preserves_initial_exchange_trailing_bytes_once_within_b` | `websocket.md:WS-018 partial`, `relay.md:RLY-003 partial` | 250바이트와 후속 데이터를 한 메시지·프레임에 넣음. `PreparedConnection`과 잔여 프레임에서 정확히 한 번 나옴. 최종 TCP 도착은 남음 |
| 같은 파일의 `streams_a_single_frame_larger_than_b_without_a_message_buffer` | `websocket.md:WS-011 partial`, `relay.md:RLY-018 partial` | B = 16 KiB, M = 16 MiB에서 큰 단일 프레임을 청크로 전달. 방향별 보유 합계 ≤ B. 실제 두 전송은 남음 |
| 같은 파일의 `counts_m_across_fragments_and_rejects_m_plus_one` | `websocket.md:WS-010 partial` | 단일·분할 모두 M은 허용, M+1은 1009. 이전에 전달한 prefix를 되돌리지 않음 |
| 같은 파일의 `rejects_text_and_invalid_frames_with_typed_errors` | `websocket.md:WS-008 partial`, `websocket.md:WS-019 partial` | text 1003. 마스크·RSV·opcode·continuation·제어 분할·길이·Close UTF-8 오류는 1002. `error_kind` 보존. 전체 세션 정리는 남음 |
| 같은 파일의 `answers_interleaved_ping_without_forwarding_control_payload` | `websocket.md:WS-009 partial` | 분할 사이 Ping의 Pong 일치. Pong·Ping·Close가 데이터 스트림에 없음. 양쪽 실제 전송은 남음 |
| 같은 파일의 `maps_initial_rejections_and_aborts_before_releasing_l` | `relay.md:RLY-004 partial`, `relay.md:RLY-005 partial`, `relay.md:RLY-022 partial` | 4008·4009·1008·1001, Close 후 stream·permit 반환 뒤 사건 한 번. 유효 Close 미완료와 H를 구별. 실제 등록·연결 차단은 남음 |
| 같은 파일의 `does_not_restart_h_after_101_or_partial_greeting_writes` | `operations.md:OPS-006 partial` | 제어한 101 완료 시각에서 H. greeting 쓰기·flush·분할 읽기로 기한이 늘지 않음. 실제 U·TLS는 남음 |
| 같은 파일의 `finishes_started_frames_before_replying_to_close_within_e` | `websocket.md:WS-017 partial` | 9.4절 선택 A에 따라 이미 헤더를 쓴 프레임만 완료하고 Close에 응답. 수신 뒤 TCP 데이터·새 메시지는 전달하지 않음. 전송 가능·E 만료·I/O 실패의 경합과 자원 반환 검사 |
| 같은 파일의 `bounds_outbound_messages_and_counts_only_written_payload` | `websocket.md:WS-007 partial` | 출력 메시지 ≤ M. 부분 헤더·부분 payload·오류에서 프레이밍과 미전달 바이트를 카운터에서 제외 |
| 같은 파일의 `drives_kw_without_payload_io_and_bounds_pending_pings` | `websocket.md:WS-026 partial` | 3.4에서 maintenance poll·빈 Ping·Pong 기한·요청하지 않은 Pong·backpressure를 시험. 9.5절 결정 뒤 작성 |
| `tests/websocket_stream.rs::ws_and_wss_forward_initial_trailing_data_once` | `websocket.md:WS-018` | WS·WSS 각각 101·공통 초기 교환·`dispatch`·실제 TCP 상대까지 후속 데이터 한 번 도착. 기존 `partial`의 남은 범위 해소 |
| 같은 파일의 `ws_and_wss_follow_the_asymmetric_close_contract` | `websocket.md:WS-016`, `websocket.md:WS-017` | WS Close 선행·TCP EOF 선행·TCP 오류·E 만료, 폐기량·L·활성 등록·ID 정리 확인 |
| 같은 파일의 `ws_and_wss_kw_keeps_idle_peers_and_drops_unresponsive_peers` | `websocket.md:WS-026` | 3.4에서 WS·WSS 정상 유휴 유지와 Pong 없는 연결 회수. 다른 세션 유지 |

표식 없는 단위 시험도 추가한다. `rotates_the_mask_across_every_chunk_split`은 네 마스크 위치와 16·64비트 길이 경계를 검사한다. `keeps_upgrade_suffix_and_app_payload_under_one_b`는 후속 바이트 우선 소비의 합계 불변식을 검사한다. `yields_during_empty_message_and_control_floods`는 H·E·종료 wake가 밀리지 않는지 검사한다. `close_actions_are_idempotent`는 Close 중복·제어 큐 증가·drop 뒤 작업 잔존을 검사한다.

실제 화면·입력과 모드별 WS·WSS 조합은 [WS-001~006](../specs/websocket.md#6-전송모드-검증-조합)의 별도 수용 시험이다. 이 문서의 스트림 단위 시험으로 대신하지 않는다. 외부 상호운용 도구의 소스는 읽지 않는다.

## 8. 현재와 목표의 차이

| 현재 | 목표와 선행 조건 |
| --- | --- |
| HTTP head 뒤 바이트 폐기 | `read_head` 결과에 원시 suffix 보존. 데이터가 들어온 순서대로 한 번 소비 |
| `hold_upgraded`에서 메시지 API 생성·보류 | 9.1·9.2절 결정 후 실제 스트림 어댑터로 교체. common `handle_connection` → `dispatch` |
| M·B 미주입 | 검증한 설정을 어댑터에 주입. 프레임 길이와 별도로 논리 M 검사 |
| 초기 실패 action·형식 있는 I/O 오류 포트 존재 | 그대로 사용. WS 코드만 어댑터에서 변환. 사건은 app 한 곳 |
| TCP 세션 중심의 app 흐름 | 기존 포트의 제어 진행·출력 프레임 경계를 보완. WS Close의 drain·폐기·E 처리를 세션 루프에 구현 |
| Q21의 미완료 2.7 보류 | 실제 WS·WSS 인계와 관련 시험을 끝낸 뒤 제거. TODO 3.2의 `현재:`나 Q21 문구 자체는 변경하지 않음 |

`relay-session.md`에는 3.2절의 필요한 포트 확장, 4.4절의 앞당긴 연결 범위, 7절의 WS 시험, 8절의 실제 인계 순서 연결이 필요하다. `code-map.md`에는 `websocket_stream.rs`·하위 frame 모듈과 WS → 공통 서비스 흐름을 Target으로 등록했다. `TODO.md`에는 2.7 완료 전 3.3 스트림 어댑터 선행 조건과 Q77~Q79 결정 반영이 연결된다. 9.1~9.3절 사용자 결정(Q77~Q79)으로 차단이 해소되었으며, 9.5절은 3.4 범위이며 2.7을 막지 않는다고 구분했다.

## 9. 미결정 사항

9.1~9.3절은 사용자 결정 2026-11-01(Q77, Q78, Q79)로 해결했다. 9.4절은 사용자 결정 2026-11-01(Q83)로 확정했다. 9.5절은 3.4 범위로 열어 두며 2.7 완료를 막지 않는다.

<a id="91-프레임-처리-구현의-선택"></a>

### 9.1 (해결) 프레임 처리 구현의 선택

사용자 결정 2026-11-01(Q77)로 선택 B(기존 헤더 API + 자체 스트리밍 상태 머신)를 확정했다. tungstenite 0.30.0의 `FrameHeader::parse`/`format`과 `derive_accept_key`만 재사용하고 나머지(페이로드 청킹, 인플레이스 언마스킹, 프레임 분할, 제어 프레임, Close 상태 머신)는 RFC 6455만으로 자체 구현한다. 새 crate 의존성은 추가하지 않는다([ADR-0005](../decisions/ADR-0005-in-house-ws-streaming-frame-codec.md)).

<a id="92-ws-tbd-004-m-b와-제어-진행"></a>

### 9.2 (해결) WS-TBD-004: M > B와 제어 진행

사용자 결정 2026-11-01(Q78)로 선택 B를 확정하고 WS-TBD-004를 해소했다. B와 M 사이에 크기 제약은 없다. 프레임 안에서 스트리밍하며 방향당 미전달 보관 바이트는 B 이하다. 분할 프레임을 합친 논리 메시지 크기가 M을 넘으면 1009로 닫고 추가 전달을 멈춘다. B보다 큰 유효 프레임도 수락해 스트리밍하며 전체를 저장하지 않는다.

### 9.3 (해결) 101 뒤 Mode I 대상 L 부족

사용자 결정 2026-11-01(Q79)로 선택 A를 확정했다. WebSocket 계약 1.5절과 2.2절에 새 코드 4011 `relay_connection_limit`을 추가했다. app의 `RelayCloseAction::Reject`는 이 사유를 명시하는 `RelayRejectReason::RelayConnectionLimit`을 사용한다.

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

- 대상: 커밋 `4478c9744e96e631e2224b6efe5ef7af4b4ccc37` 위의 작업 트리. 이번 변경은 이 설계와 `code-map.md`, `TODO.md`뿐이다. 공통 app·세션 작업의 미커밋 변경은 읽기만 했다. `relay-session.md`는 편집하지 않았다.
- 환경: Windows 11 x64. 소스·공식 버전 문서를 읽어 대조했다. 외부 API 자료는 조회 당시 값이다.
- 확인 방법: 잠금 버전의 `Stream`·프레임·메시지·헤더 API와 소스를 읽었다. Cargo 소스와 공개 버전 소스를 교차 확인했다. 유지 상태·크기·라이선스는 crates.io·저장소 API에서 읽었다.
- Rust 빌드·동작 시험: **미실행**. 문서 전용 변경이다. 어댑터와 표의 Target 시험은 아직 없다.
- 문서 검사: **통과**. 변경한 이 문서와 `architectures/code-map.md`, `TODO.md` 세 파일을 대상으로 Python Eval의 `check_ws_changed_docs()`를 실행했다. 로컬 링크 383개·그 안의 앵커 216개·표 22개의 오류가 모두 0개였다. 코드 구간은 제외하고 상대 경로·실제 제목·표 열 수를 검사했다. 외부 API 자료는 초안 작성 때 조회했다. 이번 변경에서는 계약과 결정 범위를 읽어 대조했다.
