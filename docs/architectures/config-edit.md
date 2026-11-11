# 설정 보기·편집 모듈 설계

인증된 관리자가 실행 값을 보고, 같은 TOML 파일의 허용된 값만 고치도록 한다. 파일 저장과 실행 값 교체 사이에 다시 실패할 작업을 두지 않는다. 재로드의 공유 상태와 소비 시점은 [파일 감지 설계](file-watch.md)가 맡고, 이 문서는 GET/PATCH의 사용 사례·파일 포트·HTTP 경계를 맡는다.

| 항목 | 값 |
| --- | --- |
| 구조 시점 | Current — 설정 보기·편집의 app·파일·HTTP 경계 |
| 문서 상태 | Active — Q167~Q178, 2026-11-08 채택 범위 |
| 구현 대조 | Verified — 2026-11-08 작업 트리의 소스 대조([§8](#8-current-구조-대조))와 커밋 `eae442f`의 두 OS 게이트([§9](#9-확정-경계와-검증-범위), [작업 6.9 판정표](../how-to/set-up-and-verify.md#42-작업-69-판정표)). 설정 v4·신뢰 프록시 편집 범위의 구현은 커밋 `3df0693`이다. 시험은 커밋 `51bd747`의 두 OS 게이트에서 통과했다([WSS Server·프록시·최근 로그 판정표](../how-to/set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)). |
| 코드 위치 | `crates/app/src/management/config.rs`, `crates/app/src/management/config/items.rs`, `crates/adapters/src/config_edit.rs`, `crates/adapters/src/management_http/config_api.rs`, `src/serve.rs` |
| 코드 지도 | [코드 지도](code-map.md) |
| 관련 스펙 | [관리 §3.3](../specs/management.md#33-설정-보기와-동적-설정-편집) MGT-R19~R27·MGT-028~045, [운영 §3.3](../specs/operations.md#33-파일-변경-자동-감지와-부분-재로드) OPS-059~064, [화면](../specs/dashboard.md#scr-004-설정-보기) DASH-028~053 |
| 관련 계약 | [설정 §1.8](../contracts/config-v4.md#18-재로드-구분)·[§1.10](../contracts/config-v4.md#110-관리-api-편집의-파일-쓰기), [관리 API §1.13](../contracts/management-api-v1.md#113-설정-보기-get-apiv1config)·[§1.14](../contracts/management-api-v1.md#114-동적-설정-편집-patch-apiv1config)·[§2.3](../contracts/management-api-v1.md#23-consumer의-판정-규칙), [운영 정보 §1.5](../contracts/ops-info-v2.md#15-사건-목록) |
| 관련 ADR | [ADR-0006](../decisions/ADR-0006-config-edit-through-file.md) |
| 관련 문서 | [접속 ticket 모듈 설계](relay-access-ticket.md) |

현재는 ticket을 포함한 설정 v4의 재로드 43개·편집 37개 이름을 사용한다([접속 ticket 모듈 설계](relay-access-ticket.md#44-재로드와-편집)). 아래 과거 대조·실행 기록은 ticket의 실행 증거가 아니다. 이번 문서 작업에서는 빌드·시험·포매터를 실행하지 않았다. `trusted_proxies`는 해석·검증·저장·재로드까지만 반영하며 전달 주소 판정은 R2에서 연결한다. `recent_logs.max_entries`는 보기 전용 재시작 키다. 로그 버퍼와 API는 R3 범위다. 후속: R1의 구현은 커밋 `3df0693`이며, 그 범위의 시험은 커밋 `51bd747`의 두 OS 게이트에서 통과했다([WSS Server·프록시·최근 로그 판정표](../how-to/set-up-and-verify.md#43-wss-server프록시-tls-종료최근-로그-판정표)).


## 1. 책임과 경계

- app의 `ConfigEditService<A>`는 GET/PATCH 순서·권한 이후 편집 허용·오류 분류·감사 사건을 맡는다. `A`는 app이 해석하지 않는 TLS 수락기다. HTTP·JSON·TOML·SHA 구현·OS 파일 호출은 adapters에 둔다. 이는 [코드 지도 레이어 규칙](code-map.md#레이어와-의존-방향)과 [ADR-0006](../decisions/ADR-0006-config-edit-through-file.md)의 같은 검증 경로를 따른다.
- `ConfigManagement`라는 app trait로 GET/PATCH 결과를 공개하고 관리 리스너는 `Arc<dyn ConfigManagement>`를 받는다. 실행파일만 `ConfigEditService<TlsAcceptor>`의 구체 타입을 조립한다. app은 rustls·axum·toml_edit·aws-lc-rs에 의존하지 않는다([레이어 규칙](code-map.md#레이어와-의존-방향)).
- 계정 인증·CSRF·유휴 만료는 기존 `AdminAuth`가 맡는다. 파일 감지 안정화·세대 검사·감시 경로 갱신은 `ReloadWatcher`가 맡는다. 이 서비스는 수동 재로드 API가 아니다([관리 API §1.3](../contracts/management-api-v1.md#13-요청-처리-순서), [운영 §3.3](../specs/operations.md#33-파일-변경-자동-감지와-부분-재로드)).

## 2. 내부 구조와 초기화

다음은 Current 호출 경계다. 실선은 실행 중 호출, 점선은 `serve`의 포트 주입이다. 컴파일 의존은 실행파일 → adapters → app → domain이다([레이어 규칙](code-map.md#레이어와-의존-방향)).

```mermaid
flowchart LR
    S["src/serve.rs"] -.-> H["management_http/config_api.rs"]
    S -.-> E["app/management/config.rs ConfigEditService"]
    S -.-> F["adapters/config_edit.rs FsConfigFileStore / TomlConfigDocument"]
    H -->|"인증된 GET / PATCH 값"| E
    E -->|"하나의 Mutex 획득"| R["SharedReload / ReloadService"]
    E -->|"읽기·revision·편집·검증·원자 교체 포트"| F
    R -->|"준비한 후보 한 번 발행"| C["ReloadCell"]
    E -->|"편집 사건"| L["EventSink"]
    R -->|"reload_applied"| L
```

| 요소 | 소유자·수명 | 책임·근거 |
| --- | --- | --- |
| `SharedReload<A> = Arc<Mutex<ReloadService<A>>>` | `serve`에서 한 번 생성, watcher·편집 서비스가 공유 | read→revision→edit→validate→write→swap 직렬화. [API §1.14](../contracts/management-api-v1.md#114-동적-설정-편집-patch-apiv1config) |
| `ConfigEditService<A>` | `serve`, 관리 리스너 종료까지 | 설정 경로·base_dir·공유 재로드·파일/문서 포트·Clock·EventSink 보유. [API §1.13~14](../contracts/management-api-v1.md#113-설정-보기-get-apiv1config) |
| `FsConfigFileStore` | adapters, 프로세스 수명 | 원시 바이트 읽기, SHA-256, 같은 디렉터리 임시 파일과 교체. [설정 §1.10](../contracts/config-v4.md#110-관리-api-편집의-파일-쓰기) |
| `TomlConfigDocument` | adapters 포트 구현, 문서는 요청마다 생성 | `toml_edit` 0.25의 `Document<&str>` source span으로 바뀐 값만 치환하고 `config_file::parse_candidate`로 전체 후보 검증. [ADR-0006](../decisions/ADR-0006-config-edit-through-file.md) |
| `ConfigView`·`ConfigPatch`·`ConfigEditResult` | app 값, 요청 수명 | 교환 필드의 정의는 [API §1.13~14](../contracts/management-api-v1.md#113-설정-보기-get-apiv1config). JSON Value를 app에 넘기지 않음 |
| `management/config/items.rs` `config_items`, `ConfigItem`, `ConfigReloadKind` | app의 명시적 descriptor 목록 | 계약 순서의 실행값·reload 구분·정수 정적 min/max·편집 조건 생성. 참조 파일 내용은 읽지 않음 |

## 3. 데이터 모델과 GET

[API §1.13](../contracts/management-api-v1.md#113-설정-보기-get-apiv1config)을 다음 내부 자료로 만든다.

`ConfigView`는 `as_of: SystemTime`, `revision: Option<String>`, `edit_enabled: bool`, `file: FileClassification`, `last_reload: Option<LastReload>`, `items: Vec<ConfigItem>`를 담는다. `ConfigPatch`는 revision과 `Vec<ConfigChange>`이며 각 값은 `ConfigValue`다. HTTP의 changes 객체를 이 자료로 바꾸므로 app에 JSON/TOML 타입이 들어오지 않는다. `ConfigActor`는 `AccountName`·`PeerIp`, 성공 `ConfigEditResult`는 as_of·revision·applied를 담는다.

1. 공유 재로드 mutex를 잡는다. 현재 파일 바이트를 다시 읽고 revision을 계산한다. 실행 `Settings`·`last_reload`·파일 분류를 같은 잠금 구간에서 복사한 뒤 푼다. 파일 I/O 때문에 중계 상태 mutex를 잡지 않는다. GET은 어떤 재로드 결과 사건이나 `last_reload` 갱신도 만들지 않는다.
2. `ReloadService::classify_file`은 바이트 읽기 실패 또는 비UTF-8 → `unreadable`, 그 밖의 설정/TLS 후보 검증 실패 → `invalid`, 재시작 필요 차이 존재 → `restart_required`, 재로드 대상 차이만 존재 → `pending`, 차이 없음 → `in_effect` 순으로 분류한다. 비UTF-8에서도 읽은 바이트의 revision은 유지한다(작성자 판단 lead 2026-11-08). `restart_required`에는 재시작 키만, `pending`에는 동적 차이만 담는다. 진단은 `invalid`에만 담는다. 검증과 차이 비교는 쓰기 경로와 같은 함수를 쓰고 TLS 내용 변경도 준비 후보 차이에 포함한다([설정 §1.8 규칙 2~6](../contracts/config-v4.md#18-재로드-구분), [API §1.13](../contracts/management-api-v1.md#113-설정-보기-get-apiv1config)).
3. `items`는 파일 후보가 아니라 실행 중 유효 `Settings`를 [설정 키 표](../contracts/config-v4.md#12-키-표) 순서의 명시적 descriptor 목록에 투영한다. descriptor는 자료형·정수 정적 min/max·dynamic/restart·편집 조건을 함께 정의한다. Lw의 max는 L의 현재값이 아니라 65535다. 경로는 base_dir 기준으로 이미 푼 값이다. 기능 꺼짐과 무관하게 `editable`을 계산하고 `edit_enabled`는 별도로 보낸다. 꺼진 진입점의 생략 키는 `null`을 보존한다([API §1.13](../contracts/management-api-v1.md#113-설정-보기-get-apiv1config)).
4. 인증서·개인키·계정 파일은 경로만 투영한다. 내용을 직렬화하지 않는다. 후보의 미적용 값·원시 TOML·해시·비밀번호·토큰을 응답/진단/로그에 넣지 않는다. revision은 설정 파일 바이트만의 SHA-256(`sha256:`+소문자 hex 64자)이며 TLS 파일 해시와 별개다([API §1.1.1](../contracts/management-api-v1.md#111-운영-정보-노출-범위), [설정 §1.10](../contracts/config-v4.md#110-관리-api-편집의-파일-쓰기)).

`config_file::collect_presence`는 명시된 leaf 키를 `ConfigPresence.keys`에 모은다. `config_items`는 이를 실행값과 함께 읽어 꺼진 Origin의 생략/명시적 빈 배열을 구별한다. 꺼진 WSS에 인증서 또는 개인키 경로 하나만 있으면 `ConfigPresence.inactive_paths`에서 절대 기준 경로를 읽는다. 내용은 읽지 않는다. `ReloadSnapshot`은 `Arc<Settings>`·`Arc<ConfigPresence>`·TLS 수락기를 한 번에 발행한다. GET 때 현재 파일의 presence로 실행 표시를 덮지 않는다. watcher가 실행 의미는 같고 presence만 다른 유효 후보를 받으면 보기 정보만 갱신하며 사건·last_reload는 만들지 않는다([API §1.13](../contracts/management-api-v1.md#113-설정-보기-get-apiv1config)).

### v4의 편집 가능 조건

`config_items`와 PATCH 키 검사는 같은 조건을 쓴다. TCP `allow`는 해당 TCP 진입점이 켜졌을 때만 편집할 수 있다. WS·WSS의 역할 `allow`·`allowed_origins`는 해당 역할이 켜졌을 때만 편집할 수 있다. 부모 `trusted_proxies`는 해당 리스너의 역할이 하나라도 켜졌을 때만 편집할 수 있다. Mode I `allow_targets`는 Mode I이 켜졌을 때만 편집할 수 있다. 나머지 편집 대상 수치 키는 기능 켜짐과 무관하다. `management.config_edit`는 항목별 `editable`과 별도인 요청 전체 허용 조건이다.

ticket 10개 항목은 WS·WSS 역할 뒤, `mode1` 앞에 놓으며 모두 `dynamic`이다. `relay.ticket.<role>.enabled`는 `ConfigValue::Bool`이다. `issuer`·`relay_host`는 `ConfigValue::String`, `max_lifetime_secs`는 정수다. 이 8개 키는 해당 역할이 WS 또는 WSS에서 켜졌을 때만 편집할 수 있다. ticket 자체의 켜짐 여부와는 다르다. 두 `public_keys_file` 경로는 편집할 수 없다. 공개 키 내용은 응답에 넣지 않는다([접속 ticket 모듈 설계](relay-access-ticket.md#44-재로드와-편집)).

`relay.wss.cert_file`·`relay.wss.key_file`·`management.cert_file`·`management.key_file`과 ticket 공개 키 경로는 재로드 대상이지만 편집할 수 없다. TCP·WS·WSS 진입점의 `enabled`·부모 `listen`·`recent_logs.max_entries`는 재시작 키다. 꺼진 역할의 생략한 `allow`·`allowed_origins`와 꺼진 리스너의 생략한 `listen`·TLS 경로·`trusted_proxies`는 실행 presence에 따라 `null`로 표시한다. 켜진 리스너에서 생략한 `trusted_proxies`는 기본값 `[]`다. 꺼진 리스너라도 명시한 빈 배열은 `[]`로 보존한다.


## 4. PATCH 처리 순서

[API §1.14의 10단계](../contracts/management-api-v1.md#114-동적-설정-편집-patch-apiv1config)를 그대로 경계로 삼는다. 실패 때 다음 단계를 실행하지 않는다.

| 단계 | 위치·동작 | 실패 또는 결과 |
| --- | --- | --- |
| 1 | HTTP에서 인증→CSRF→경로/메서드/헤더/JSON. app에서 빈 changes·37개 외 키·위 편집 가능 조건 불충족·값 자료형 거절 | `400 invalid_request`; 매체 형식은 기존 `415`, 메서드는 `405 Allow: GET, PATCH`. 편집 사건 없음 |
| 2 | app에서 실행 중 `management.config_edit` 확인 | `403 config_edit_disabled` |
| 3 | 공유 mutex 아래 현재 파일 읽기 | `409 config_file_invalid`, 진단 `[]` |
| 4 | 현재 원시 바이트 revision과 요청 비교 | `409 config_conflict` |
| 5 | 원본 전체 parse/validate/TLS·ticket 공개 키 검증. 실행값과 비대상 비교 | `409 config_file_invalid`+진단 또는 `409 config_restart_pending`+재시작 키 |
| 6 | 원본 문서 위에 요청 중 실제 파일값이 달라지는 키만 편집 | 기타 값·주석·공백·순서·LF/CRLF 유지. ticket Bool/String/정수도 같은 부분 편집 경로 사용 |
| 7 | 편집 결과를 같은 loader와 `prepare_candidate`로 전체 검증하고 수락기/공개 키/해시/스냅샷 준비 | `400 config_invalid`+값 인용 없는 진단. 아직 원본·실행값 불변 |
| 8 | 출력 바이트와 읽은 원본 바이트 비교 | 같으면 쓰기/적용/성공 사건 없음. `200 applied: []`, 기존 revision |
| 9 | 같은 디렉터리에 create_new 임시 파일, 전부 쓰기·flush/sync·닫기, 교체 직전 파일을 다시 읽어 revision 비교, 원자 교체 | 사용자 결정 2026-11-08(Q178). 불일치/재읽기 실패면 임시 파일 삭제·`409 config_conflict`·이번 요청의 파일/실행값 변경 없음. 쓰기/교체 실패는 `500 config_write_failed` |
| 10 | 준비한 같은 후보를 `commit_prepared`로 무실패 발행. `reload_applied(config_edit)` 뒤 `config_edited` 기록 | 실행값 차이 전부를 `applied`로 응답, 새 파일 revision |

단계 3~10 전체와 단계 2의 실행값 읽기는 **동일한 공유 mutex 한 번**으로 묶는다. watcher도 최종 파일 읽기 전에 이 mutex를 잡고 세대 확인·검증·적용까지 유지한다. watcher가 안정화 전 읽은 바이트를 mutex 획득 뒤 그대로 적용하지 않는다([운영 §3.3](../specs/operations.md#33-파일-변경-자동-감지와-부분-재로드)). `prepare_candidate`는 파일 교체 전에 TLS 수락기·차이 키·경고·스냅샷을 준비한다. 단계 10에서 파일 재읽기·TLS 재검증·세대/종료 거절을 하지 않는다. 그래야 파일만 바뀌고 실행값은 거절되는 상태를 만들지 않는다([설정 §1.8 규칙 10](../contracts/config-v4.md#18-재로드-구분)).

작성자 판단(lead 2026-11-08): 단계 8은 편집 전후 **파일 텍스트/바이트**가 같은지를 판정한다. 파일에 pending 동적 변경이 있고 PATCH가 다른 키를 고치면 후보 전체를 적용한다. `reload_applied.applied`와 응답 `applied`에는 실행값이 바뀐 키 전부를 넣고, `config_edited.config_keys`에는 요청 `changes` 중 파일값이 바뀐 키만 넣는다. no-op이면 pending 변경은 watcher 몫으로 남긴다. 이 구분은 [API §1.14](../contracts/management-api-v1.md#114-동적-설정-편집-patch-apiv1config) 단계 6·8·10과 [운영 정보 config_edited](../contracts/ops-info-v2.md#관리자-로그인과-종료)의 명확화 대상이다.

## 5. 파일 포트와 OS 경계

포트는 app에 선언하고 adapters가 구현한다. 포트가 반환하는 진단은 `ConfigDiagnostic`, 값은 app 자료형이다([레이어 규칙](code-map.md#레이어와-의존-방향), [설정 §1.10](../contracts/config-v4.md#110-관리-api-편집의-파일-쓰기)).

- `ConfigFileStore::read(path) -> io::Result<Vec<u8>>`: 원시 바이트만 읽는다. 비UTF-8이면 file.state=unreadable, diagnostics=[]이고 바이트를 읽었으므로 revision은 그 바이트의 SHA-256이다. null은 바이트 읽기 실패일 때만이다. PATCH는 비UTF-8도 3단계에서 `409 config_file_invalid`, 빈 진단으로 거절한다([API §1.13~14](../contracts/management-api-v1.md#113-설정-보기-get-apiv1config), [설정 §2.1](../contracts/config-v4.md#21-설정-오류와-준비-실패), 작성자 판단 lead 2026-11-08).
- `ConfigFileStore::revision(bytes) -> String`: adapters에서 aws-lc-rs SHA-256을 사용한다. `read`는 바이트 읽기만 맡고 revision 계산은 이 별도 메서드가 맡는다. `ReloadService`의 프로세스 내부 TLS 변경 탐지용 해시를 외부 revision에 재사용하지 않는다.
- `ConfigDocument::parse(bytes, base_dir) -> Result<ConfigCandidate, Vec<ConfigDiagnostic>>`: `TomlConfigDocument`가 `config_file::parse_candidate`를 호출한다. 이 함수는 공통 `settings_from_table` 전체 검사와 presence 수집을 사용한다. `edit(bytes, changes)`는 `toml_edit::Document<&str>`로 키·값의 원본 바이트 범위를 찾고 실제 값이 다른 범위만 치환한다. `same_value`는 같은 값이면 치환하지 않는다. `edit_array`는 배열 토큰 사이의 주석·공백을 원문에서 보존한다. 누락 키/테이블은 해당 scope/끝에 삽입한다. 기존 문서 전체를 `DocumentMut`로 다시 직렬화하지 않으므로 BOM·혼합 LF/CRLF도 고치지 않은 범위에서는 그대로다([설정 §1.10](../contracts/config-v4.md#110-관리-api-편집의-파일-쓰기)).

v4의 `edit_change`는 마지막 점 앞까지를 테이블 경로로, 마지막 조각을 leaf 키로 나눈다. `[relay.ws.server]` 같은 중첩 테이블을 따라 기존 source span을 찾는다. 누락한 역할 테이블은 정확한 `[relay.ws.server]` 헤더로 추가한다. 부모 `trusted_proxies`와 역할 `allow`를 같은 scope에 쓰지 않는다. dotted/inline table도 같은 leaf를 찾아 고치며 관련 없는 원문은 그대로 둔다.

- `ConfigFileStore::replace(path, expected_revision, next) -> Result<(), ConfigReplaceError>`: `FsConfigFileStore`의 `replace_file`이 같은 디렉터리에 `create_new` 임시 파일을 만든다. write_all→원본 permission bits 복사→flush→sync_all→close→원본 재읽기·revision 비교→`std::fs::rename` 순이다. 다르거나 재읽기 실패이면 `Conflict`, 그 밖의 교체 전 쓰기/rename 실패이면 `WriteFailed`다. `TempFile::drop`은 실패 때 자기 임시 파일만 지운다. 원본 선삭제·copy+delete는 없다. Windows 공유 삭제가 막힌 핸들은 rename 실패로 남는다. rename 뒤 실패 가능한 directory sync를 추가하지 않아 교체 성공을 원본 불변 오류로 돌리지 않는다. Windows ACL과 Unix 디렉터리 권한의 운영 조건은 [운영 how-to](../how-to/operate-relay.md#351-대시보드-설정-보기편집)를 따른다([설정 §1.10](../contracts/config-v4.md#110-관리-api-편집의-파일-쓰기), 사용자 결정 2026-11-08(Q178)).

**사용자 결정 2026-11-08(Q178): 교체 직전 재확인 + 계약에 틈 명시.** 공유 mutex는 프로세스 안의 watcher/PATCH·PATCH/PATCH를 직렬화한다. 외부 편집은 단계 3 읽기와 교체 직전 재확인으로 검출한다. 마지막 재확인과 원자 교체 사이의 아주 짧은 틈은 계약의 보장 범위가 아니다. 운영 안내에는 “편집 기능을 켠 동안 외부 편집과 대시보드 편집을 동시에 하지 않는다”를 적는다([관리 MGT-033](../specs/management.md#7-수용-테스트)의 보장 범위 보완 대상). 충돌 사건은 `config_edit_rejected(reason=config_conflict)`이며 성공 사건은 없다.

고르지 않은 대안은 Linux `renameat2(RENAME_EXCHANGE)`·Windows `ReplaceFileW` 백업의 교체 뒤 원본 revision 검사/복구다. syscall/unsafe 승인·지원 파일시스템·복구 실패 처리가 필요하며 열린 구 inode/handle의 제자리 쓰기와 복구 중 외부 저장은 여전히 막지 못한다. Linux에 임의 편집기를 강제하는 일반 강제 잠금은 없다. 이 대안을 완전 보장으로 표현하거나 구현하지 않는다. 최종 선택은 Q178의 단순 재확인이다.

작성자 판단(lead 2026-11-08): 새 줄은 파일에서 처음 나오는 줄바꿈 방식으로 넣고, 기존 줄바꿈이 없으면 LF를 쓴다. 고치지 않은 바이트의 줄바꿈은 정규화하지 않는다([설정 §1.10](../contracts/config-v4.md#110-관리-api-편집의-파일-쓰기)의 명확화).

## 6. HTTP·시간 제한·종료 유예

- `session_api::answer`는 로그인 이외 상태 변경 메서드 POST/PATCH/DELETE의 CSRF를 경로 분기 전에 검사하고 `config_api::answer`로 전달한다. 인증 실패는 경로·진단·편집 enabled 상태보다 먼저다. GET은 CSRF 불필요, PATCH는 필수다. `config_api::patch_body`는 JSON을 app `ConfigPatch`·`ConfigValue`로 바꾸고, `view_body`·`edit_error`는 응답을 직렬화한다. 성공한 사용자 행동만 `AdminAuth::touch`로 연장한다. PATCH의 `X-Relay-Poll`은 값과 무관하게 거절한다([API §1.3](../contracts/management-api-v1.md#13-요청-처리-순서)·[§1.4](../contracts/management-api-v1.md#14-관리-세션)·[§1.14](../contracts/management-api-v1.md#114-동적-설정-편집-patch-apiv1config)).
- `config_api::PATCH_BODY_LIMIT`는 64 KiB(65536바이트)다. `axum::body::to_bytes`의 초과·읽기 실패는 `400 invalid_request`다. 413을 추가하지 않는다. 로그인은 기존 `session_api::LOGIN_BODY_LIMIT` 16 KiB 그대로다. 이는 설정 파일 자체의 크기 상한이 아니다([설정 §1.1](../contracts/config-v4.md#11-파일-규칙), 작성자 판단 lead 2026-11-08).
- GET/PATCH의 Consumer 대기 한도는 `dashboard.query_timeout_secs`다. 서버 전체 트랜잭션에 이 값의 취소 타이머를 씌우지 않는다. 파일 read/parse/TLS 준비/replace를 포함한 동기 작업은 `spawn_blocking`에서 끝까지 실행한다. 클라이언트 timeout·응답 연결 종료로 await가 없어져도 이미 시작한 blocking 작업을 취소하지 않는다. 특히 rename 성공 뒤에는 commit·감사 기록까지 이어 간다. Consumer는 결과 불명으로 GET을 다시 하며 자동 PATCH 재전송하지 않는다([API §2.3](../contracts/management-api-v1.md#23-consumer의-판정-규칙), MGT-R27).
- U는 해당 관리 연결 수락 때 고정한 값이다. PATCH를 위해 U를 특별히 멈추지 않는다. 기존 terminate 전용 예외를 넓히지 않는다. U로 응답 연결이 닫혀도 트랜잭션은 계속되고 결과 불명 복구를 쓴다([설정 §1.8](../contracts/config-v4.md#18-재로드-구분), [API §1.9](../contracts/management-api-v1.md#19-관리자-종료-post-apiv1sessionssession_idterminate)·[§2.3](../contracts/management-api-v1.md#23-consumer의-판정-규칙)).
- `DashboardHints`는 P·query timeout만 시작값으로 보유한다. `session_api::session_body`가 로그인/GET session 응답 때 `terminate_timeout_secs()`를 호출하며, 이 helper가 `RuntimeSettingsSource::with_current`로 현재 E를 읽어 E+5를 만든다. 기존 종료 교환 기한은 별개로 유지한다([API §1.12](../contracts/management-api-v1.md#112-폴링-주기와-조회-제한-시간의-전달)).
- `serve`는 공유 mutex/파일 포트를 만든 뒤 watcher와 관리 서비스에 같은 인스턴스를 주입한다. 정상 종료 시작 시 watcher 적용만 중지한다. 관리 서비스는 유예 동안 살아 있어 GET/PATCH를 허용한다. G는 종료 시작 때 한 번 읽고 deadline으로 고정하므로 유예 중 G 편집은 기존 deadline을 바꾸지 않는다. 편집이 lifecycle을 Running으로 되돌리거나 신규 중계 수락을 열지 않는다([운영 §3.3 규칙 7](../specs/operations.md#33-파일-변경-자동-감지와-부분-재로드), [API §1.14](../contracts/management-api-v1.md#114-동적-설정-편집-patch-apiv1config)).

## 7. 사건·오류와 남는 상태

[운영 정보 §1.5](../contracts/ops-info-v2.md#15-사건-목록)·[§1.8](../contracts/ops-info-v2.md#18-관리자-계정명-기록-규칙)의 필드만 쓴다.

| 사건 | 작성 위치와 필드 출처 | 순서·금지 |
| --- | --- | --- |
| `reload_applied` | `ReloadService::commit_prepared`, `trigger=config_edit`, 실행값/경로/내용이 달라진 43개 중 키. 같은 경로의 ticket 키 목록 교체도 `public_keys_file`에 포함 | 실행값 차이가 있으면 같은 편집의 `config_edited`보다 먼저. 값 없음 |
| `config_edited` | `ConfigEditService::patch`, `admin_account`는 AuthorizedAdmin의 계정, `peer_ip`는 실제 소켓의 정규화 PeerIp, `config_keys`는 요청 중 파일값 변경 키, `revision`은 저장한 바이트 SHA-256 | 파일 변경 있는 성공만 한 번. 헤더 전달 IP·설정값·후보 텍스트 없음 |
| `config_edit_rejected` | `ConfigEditService::reject`, 인증/CSRF 뒤의 6개 편집 업무 오류; `reason`은 API 오류 code, account/peer는 위와 같음, 선택 `config_keys`는 요청 키 | invalid_request·internal·인증·CSRF 실패에는 없음. 성공 사건 없음 |
| `allow_all_target`·`allow_all_source` | `prepare_candidate`가 실제 바뀐 정책의 경고만 준비하고 commit에서 `phase=reload`로 기록 | [설정 §1.8 규칙 5~6](../contracts/config-v4.md#18-재로드-구분), OPS-058 |

6개 업무 오류의 상태·diagnostics/config_keys는 §4 표대로 매핑한다. 조회 자체 내부 실패는 `500 internal`, 파일 읽기/검증 실패 GET은 `200`이다. 진단은 code/key만 안정된 값으로 사용하며 message는 값을 인용하지 않는다([API §2.1](../contracts/management-api-v1.md#21-오류-본문과-상태-코드)).

## 8. Current 구조 대조

| 확인한 코드 위치·심볼 | 현재 역할 | 근거 |
| --- | --- | --- |
| `crates/app/src/management/config.rs` `ConfigManagement`, `ConfigEditService::{get,patch,reject}` | object-safe 동기 포트, 동일 mutex의 GET/10단계 PATCH, 파일값 변경 키와 실행값 차이 분리, 업무 오류 감사 | API §1.13~14 |
| `crates/app/src/management/config/items.rs` `config_items` | 실행값·적용된 presence만 표시하고 TLS/계정 파일은 경로만 반환 | API §1.13 |
| `crates/app/src/reload/snapshot.rs` `ReloadSnapshot`, `ReloadCell`; `reload/service.rs` `prepare_candidate`, `classify_file`, `commit_prepared` | Settings·presence·TLS 원자 발행, 부작용 없는 준비/분류, 실패를 반환하지 않는 commit과 last_reload | 설정 §1.8, API §1.13~14 |
| `crates/adapters/src/config_file.rs` `parse_candidate`, `collect_presence`; `settings.rs` `ManagementSettings::config_edit` | 공통 전체 검증과 생략 정보, 기본 false | 설정 §1.2·§3.2 |
| `crates/adapters/src/config_edit.rs` `FsConfigFileStore`, `replace_file`, `TempFile`, `TomlConfigDocument`, `edit_change`, `edit_array` | SHA-256, 같은 디렉터리 임시 파일·최종 revision 확인·rename·실패 정리, TOML source span 기반 부분 치환 | 설정 §1.10, Q178 |
| `crates/adapters/src/management_http.rs` `ManagementServices`, `DashboardHints`; `session_api::answer`, `config_api::{answer,patch_body,view_body,edit_error}` | 서비스 주입·PATCH CSRF·64 KiB·blocking 호출·JSON 오류 매핑·응답 때 현재 E+5 | API §1.3·§1.12~14 |
| `src/serve.rs` `config_management`, `listen_until_shutdown` | `std::path::absolute`로 설정 경로와 base_dir을 맞춘 뒤 같은 SharedReload·파일/문서 포트·시계·사건 싱크를 app 서비스에 주입. helper는 `io::Result<Arc<dyn ConfigManagement>>`를 반환 | ADR-0006, 운영 §3.3 |

대조 범위는 2026-11-08 설정 편집 작업 트리의 위 소스다. 소스의 경계·타입·순서를 읽은 기록이며 실행 검증이 아니다. 빌드·시험·formatter는 실행하지 않았다. 양 OS 계약/시나리오 결과는 통합 검증 담당자의 실제 실행 기록을 따른다. 후속: 위 표의 심볼은 커밋 `eae442f`에도 있다. 실행 검증은 커밋 `eae442f`의 두 OS 게이트에서 통과했다([§9](#9-확정-경계와-검증-범위), [작업 6.9 판정표](../how-to/set-up-and-verify.md#42-작업-69-판정표)).

## 9. 확정 경계와 검증 범위

- PATCH 64 KiB·pending/edited 키 구분·비UTF-8·새 줄바꿈·재확인 읽기 실패 매핑은 작성자 판단(lead 2026-11-08)에 따른다. 외부 편집 경쟁은 Q178의 교체 직전 재확인·짧은 보장 밖 틈으로 확정했다. 외부 동작의 정확한 형식은 관련 계약을 기준으로 한다.
- `last_reload`는 watcher의 applied/rejected/restart_required와 실행값을 실제 발행한 PATCH(applied)를 기록한다. GET·동일 후보 watcher·PATCH 인증/본문/꺼짐/충돌/파일 상태/후보 검증/쓰기 실패/no-op은 갱신하지 않는다. `commit_prepared`의 applied가 비어도 갱신하지 않는다. 스키마가 허용하는 config_edit+rejected/restart_required는 현재 생성하지 않는다.
- disabled 생략 null·단독 WSS 경로·배열 내부 주석·혼합 줄바꿈·Q178 경쟁·실제 OS 교체 권한은 통합 검증 대상이었다. 아래 시험이 커밋 `eae442f`의 두 OS 게이트에서 통과했다([작업 6.9 판정표](../how-to/set-up-and-verify.md#42-작업-69-판정표)). OS 전용 시험은 해당 OS에서만 실행된다.
  - disabled 생략 null·단독 WSS 경로: `tests/management_query.rs`의 `config_view_reports_effective_values_all_file_states_and_reload_history`.
  - 배열 내부 주석·혼합 줄바꿈: `crates/adapters/tests/config_edit_document.rs`의 `array_value_edits_keep_internal_comments_spacing_and_each_newline`, `scalar_edits_preserve_bom_comments_decor_and_mixed_newlines_exactly`. 재시작 뒤 유지와 LF·CRLF 파일은 `tests/management_query.rs`의 `config_edit_preserves_document_applies_once_and_survives_restart`가 확인한다.
  - Q178 경쟁: `crates/adapters/src/config_edit.rs`의 단위 시험 `external_save_after_temp_sync_is_rechecked_and_temp_is_removed`, `tests/management_query.rs`의 `stale_config_revision_refuses_other_admin_and_external_comment_changes`.
  - 실제 OS 교체 권한: DASH-044·045 실서버 시험이 설정 디렉터리 쓰기를 막는다(Windows는 `icacls`의 거부 항목, Ubuntu는 `0o555`). Windows 전용 `crates/adapters/tests/config_edit_document.rs`의 `windows_open_handle_without_share_delete_prevents_rename_and_cleans_temp`도 있다. 시험은 시험 실행 계정으로 했고, 별도 서비스 로그온 계정에서는 확인하지 않았다.
