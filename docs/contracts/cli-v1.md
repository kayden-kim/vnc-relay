# CLI와 관리자 계정 파일 v1 계약

`vnc-relay` 실행파일의 하위 명령·옵션·종료 코드·출력 규칙과, `vnc-relay admin` 명령이 쓰고 `vnc-relay serve`가 읽는 관리자 계정 파일 형식을 정한다. 서버 실행과 관리자 계정 준비는 같은 실행파일의 서로 다른 하위 명령이다. 외부 동작은 [운영 스펙](../specs/operations.md)이 정하고, 설정 파일 키는 [설정 파일 계약](config-v2.md)이 정한다.

이 계약은 2026-10-16에 사용자가 채택했다(상태 Active). 하위 명령·옵션, 비밀번호 숨김 입력, 해시 형식, 계정 이름·비밀번호 규칙, 계정 파일 형식, 오류·경고 코드, 종료 코드가 모두 채택 범위다. 해시는 Argon2id이고 승인된 범위는 설정 계약이 정한다. 같은 날 사용자가 종료 코드를 확정했다. 설정 오류는 3, 인증서·개인키 파일 오류는 3, 리스너 바인드 실패는 4, 계정 파일 오류(파일 없음·계정 없음 포함)는 5다. 정상 종료 유예 중 두 번째 종료 요청의 처리도 확정했다. 6절에 관찰·구현에서 확인한다고 남긴 항목은 채택으로 해소되지 않았다. 지금 실행할 수 있는 명령은 `serve`(시작·종료 골격), `admin add|passwd|remove|list`, `--help`·`--version`이다.

| 항목 | 값 |
| --- | --- |
| 이름과 버전 | `vnc-relay-cli` 1.6.0. 1.1.0은 오류 코드 `runtime_unavailable`을 더했다. 1.2.0은 오류 코드 `log_file_unavailable`(종료 코드 3)과 경고 코드 `log_write_failed`를 더했다. 1.3.0은 경고 코드 `accounts_file_cleanup_failed`를 더하고 `runtime_unavailable`을 `admin` 명령에도 쓰게 했다. 1.3.0의 두 변경은 사용자가 결정했다(2026-10-20). 1.4.0은 Windows 숨김 입력의 콘솔 입력 모드 저장·복원과 입력 끝 키 설명(1.5절), 교체 실패 뒤 잠금 파일 삭제 실패 경고(1.6절)를 적었다. 이 세 가지는 사용자가 결정했다(2026-10-21). 콘솔 모드를 읽거나 되돌리지 못할 때의 `runtime_unavailable` 경우는 구현 판단이다(6절). 1.5.0은 운영체제 난수원으로 시작 키(RelayIdKey·SessionIdKey)를 만들 수 없을 때의 `serve` 실패를 `runtime_unavailable`(종료 코드 1)로 다루도록 추가했다(사용자 결정 2026-11-01(Q76)). 1.6.0은 Windows 콘솔 창 닫기를 G 없이 즉시 강제 정리하는 종료 요청으로 정하고, Windows 로그오프·시스템 종료를 강제 종료로 적었다(1.8절, 사용자 결정 2026-11-04(Q102)). 모두 호환 변경이다. |
| 상태 | Active(2026-10-16 채택) |
| 소유자 | vnc-relay 프로젝트 관리자 |
| 만드는 쪽(Producer) | `vnc-relay` 실행파일. 명령행 출력·종료 코드·계정 파일을 만든다. |
| 사용하는 쪽(Consumer) | 운영자와 스크립트(명령행 출력·종료 코드), `vnc-relay serve`(계정 파일) |
| 방향 | 운영자가 `admin` 명령을 실행하면 계정 파일이 바뀐다. `serve`는 시작 때 계정 파일을 읽는다. |
| 스키마 기준 파일 | 없음. 이 문서의 Markdown 표가 형식의 기준이다. |
| 기준 스펙 | [운영 스펙 3.1절](../specs/operations.md#31-설정-검증과-시작), [3.2절](../specs/operations.md#32-로컬-cli-계정-준비), [3.4절](../specs/operations.md#34-정상-종료와-재시작) |
| 관련 요구사항 | [PRD](../requirements/PRD.md) FR-06·FR-07, [7절 보안 및 신뢰 경계](../requirements/PRD.md#7-보안-및-신뢰-경계) |
| 호환성 범위 | Windows 11 x64·Ubuntu 24.04 LTS x64의 `vnc-relay` 1.x. Windows Server는 제외한다. 계정 파일 `version = 1`. |
| 관련 테스트 | [운영 스펙 수용 테스트](../specs/operations.md#7-수용-테스트) OPS-001~002·OPS-007~010·OPS-019·OPS-029~031·OPS-039·OPS-042·OPS-044. 계약 테스트는 5절에 연결한다. `serve` 시작·종료 골격과 계정 파일 읽기는 Ubuntu와 Windows에서 통과했다. Windows에서는 Unix 신호·권한 테스트를 실행하지 않았다. `admin` 명령은 Ubuntu에서 통과했다. Windows에서는 터미널이 없는 테스트만 자동으로 실행했다. 종료 유예의 실행파일 시험은 미실행이다(작업 5.3). |

용어는 [VNC Relay 용어집](../../GLOSSARY.md)을 따른다. 설정 파일 키는 [설정 파일 계약](config-v2.md)을 따른다. 이 문서에서 "계정"은 관리자 계정이다.

관련 문서: [설정 파일 계약](config-v2.md)(`[admin]` 테이블, 설정 오류 코드), [운영 정보 계약](ops-info-v1.md)(`serve`가 쓰는 로그, 시작 실패 사건), [관리 API 계약](management-api-v1.md)(로그인이 쓰는 계정명 규칙), [ADR-0003 단일 인스턴스·메모리 상태](../decisions/ADR-0003-single-instance-memory-state.md).

## 1. 형식과 의미

### 1.1 하위 명령과 옵션

실행파일 이름은 `vnc-relay`다. 아래 명령 외의 하위 명령·옵션은 사용법 오류다.

| 명령 | 하는 일 | 입력 | stdout |
| --- | --- | --- | --- |
| `vnc-relay serve --config <path>` | 서버를 포그라운드로 실행한다(1.8절). | 설정 파일, 계정 파일 | 운영 로그(`log.output = "stdout"`일 때만) |
| `vnc-relay admin add <account> --config <path>` | 계정을 추가한다. | 숨김 입력: 비밀번호, 비밀번호 확인 | 성공 문장 한 줄 |
| `vnc-relay admin passwd <account> --config <path>` | 기존 계정의 비밀번호를 바꾼다. | 숨김 입력: 새 비밀번호, 확인 | 성공 문장 한 줄 |
| `vnc-relay admin remove <account> [--yes] --config <path>` | 계정을 삭제한다. | 확인 입력(`--yes`로 생략할 수 있다) | 성공 문장 한 줄 |
| `vnc-relay admin list --config <path>` | 계정 이름을 나열한다. | 없음 | 이름을 줄마다 하나, 이름순 |

| 옵션 | 규칙 |
| --- | --- |
| `--config <path>` | 모든 명령에서 필수다. `--config=<path>`로도 쓸 수 있다. 한 번만 쓸 수 있다. 설정 파일 경로다. |
| `--yes` | `admin remove`에서만 쓴다. 삭제 확인을 생략한다. 스크립트용이다. |
| `--help`, `-h` | 사용법을 stdout에 쓰고 종료 코드 0으로 끝난다. 다른 인수와 같이 쓰지 않는다. |
| `--version`, `-V` | 버전을 stdout에 한 줄로 쓰고 종료 코드 0으로 끝난다. 다른 인수와 같이 쓰지 않는다. |

- 계정 이름은 위치 인수 하나다. 비밀번호를 명령행 인수·URL·환경 변수로 받지 않는다. 이 실행파일은 설정을 위해 환경 변수를 읽지 않는다.
- `admin` 명령은 설정 파일에서 `[admin]` 테이블(`accounts_file`, `argon2`)만 읽고 검사한다. 구문은 전체를 검사한다. 다른 테이블의 키·값은 검사하지 않는다. 설정 파일을 끝까지 쓰기 전에도 계정을 준비할 수 있게 하기 위해서다.
- `accounts_file`의 상대 경로는 설정 파일이 있는 디렉터리를 기준으로 푼다([설정 파일 계약 1.1절](config-v2.md#11-파일-규칙)).

### 1.2 계정 이름과 비밀번호 규칙

2026-10-16에 채택한 규칙이다.

| 대상 | 규칙 |
| --- | --- |
| 계정 이름 | 1~32자. 소문자 영문(`a-z`)·숫자(`0-9`)·점(`.`)·밑줄(`_`)·하이픈(`-`)만 쓴다. 대소문자를 구분한다. 이름은 로그(관리자 종료 사건)에 기록되므로 ASCII로 제한한다. 규칙 위반은 `account_name_invalid`다. |
| 비밀번호 길이 | 12~128자(문자 수), UTF-8로 512바이트 이하. 짧거나 길면 `password_invalid`다. |
| 비밀번호 문자 | 줄바꿈 등 제어 문자를 받지 않는다. 입력 바이트를 정규화하지 않고 그대로 해시한다. 로그인 입력도 같은 방식이다. ASCII 출력 가능 문자를 권장한다. |
| 확인 입력 | `add`·`passwd`는 비밀번호를 두 번 입력한다. 두 값이 다르면 `password_mismatch`다. |

### 1.3 계정 파일 형식

TOML이다. UTF-8 텍스트 한 파일이다. 경로는 설정 파일의 `admin.accounts_file`이다. 파일에는 해시만 있고 평문 비밀번호는 없다.

| 필드 | 자료형 | 필수 여부 | 의미·제약 |
| --- | --- | --- | --- |
| `version` | 정수 | 필수 | 계정 파일 형식 버전. 값은 `1`이다. |
| `[[account]]` | 테이블 배열 | 아님. 없으면 계정 0개 | 계정마다 테이블 하나. |
| `account.name` | 문자열 | 필수 | 계정 이름(1.2절). 파일 안에서 유일하다. |
| `account.password_hash` | 문자열 | 필수 | Argon2id 해시 문자열(1.4절). |

- 알 수 없는 키, 중복 이름, 자료형 오류, 이름 규칙 위반, 해시 문자열 오류는 파일 오류 `accounts_file_invalid`다. 파일을 일부만 읽지 않는다. UTF-8이 아닌 파일과 TOML 구문 오류도 형식 오류이므로 `accounts_file_invalid`다.
- 진단의 `key`(사용자 결정 2026-10-20): 파일 안의 오류는 계정 파일 안의 경로를 쓴다. 배열은 0부터 센 `[i]` 첨자다(예: `version`, `account[1].name`, `account[1].password_hash`). 중복 이름은 뒤에 나온 계정에 보고한다. 구문 오류·UTF-8 오류는 `key`를 생략한다. 파일 단위 오류(`accounts_file_missing`, `accounts_file_unreadable`, `accounts_none`, `accounts_file_locked`, `accounts_file_write_failed`)와 경고(`accounts_file_permissive`, `last_account_removed`, `accounts_file_cleanup_failed`)는 설정 키 `admin.accounts_file`을 쓴다.
- 저장할 때 `[[account]]`를 이름순으로 쓴다. 주석은 보존하지 않는다.
- 계정이 0개인 파일은 형식상 유효하다. `serve`는 이 파일로 시작하지 않는다.

### 1.4 해시 문자열 형식

PHC 문자열 형식이다.

```text
$argon2id$v=19$m=<memory_kib>,t=<iterations>,p=<parallelism>$<솔트>$<해시>
```

| 부분 | 규칙 |
| --- | --- |
| 알고리즘·버전 | `argon2id`, `v=19`만 받는다. |
| `m`·`t`·`p` | 이 순서의 십진수. 메모리(KiB)·반복·병렬이다. 범위는 승인값이며 [설정 파일 계약 1.2절](config-v2.md#12-키-표)의 `admin.argon2.memory_kib`·`iterations`·`parallelism`과 같다. 하한 미만은 `serve`에서 `accounts_hash_below_minimum`이다. `admin` 명령은 하한 미만 해시를 오류·경고 없이 받는다. 그 계정의 해시를 `admin passwd`로 다시 만들 수 있어야 하기 때문이다(사용자 결정 2026-10-20). 상한 초과는 승인 범위 밖의 해시 문자열이므로 `accounts_file_invalid`다. `admin`은 상한 초과가 있으면 다른 매개변수가 하한 미만이어도 `accounts_file_invalid`로 거절한다. 같은 해시를 `serve`는 하한 미만(`accounts_hash_below_minimum`)으로 보고한다. 다른 매개변수(`keyid`·`data`)가 있으면 `accounts_file_invalid`다. |
| 솔트 | 운영체제 난수원으로 만든 16바이트. 읽을 때는 16바이트 이상을 받는다. |
| 해시 | 32바이트. 솔트·해시의 인코딩은 패딩 없는 표준 Base64다. |

- 새 해시는 설정의 `admin.argon2` 값으로 만든다. 생략하면 설정 계약의 기본값을 쓴다.
- 서버는 저장된 해시의 `m`·`t`·`p`로 검증한다. 설정 값이 바뀌어도 기존 해시는 그대로 검증된다. 서버는 해시를 다시 쓰지 않는다.
- 이 문서의 예제 해시는 자리표시자다. 실제 해시를 문서에 넣지 않는다.

### 1.5 입력과 출력 규칙

| 항목 | 규칙 |
| --- | --- |
| 표준 출력(stdout) | 명령 결과 데이터만 쓴다. `admin list`의 이름, 성공 문장, `--help`·`--version` 텍스트가 해당한다. `serve`는 `log.output = "stdout"`일 때 운영 로그(JSON Lines)만 쓴다. 로그 줄이 아닌 텍스트를 stdout에 섞지 않는다. |
| 표준 오류(stderr) | 입력 안내 문구(프롬프트), 오류, 경고, 안내(`notice`)를 쓴다. |
| 오류·경고 줄 | `error[<code>] key=<key>: <message>` 또는 `warning[<code>] key=<key>: <message>`. 대상 키가 없으면 ` key=<key>`를 생략한다. `code`와 `key`는 안정적이다. `message`는 사람이 읽는 문장이며 바뀔 수 있다. 줄마다 오류 하나다. |
| 보고 범위 | `serve` 시작과 `admin` 명령은 같은 단계의 오류를 모두 모아 보고한다. 종료 코드는 처음 실패한 단계의 코드다(1.8절). |
| 비밀 | 비밀번호·해시·개인키 내용을 stdout·stderr·로그에 쓰지 않는다. `admin list`는 이름만 쓴다. |
| 숨김 입력 | 입력한 글자를 화면에 표시하지 않는다. 표준 입력이 터미널이어야 한다. 터미널이 아니면 입력을 받지 않고 `not_interactive`로 끝난다. 종료할 때(취소 포함) 터미널의 표시 설정을 되돌린다. Windows에서는 입력을 읽기 전에 콘솔 입력 모드를 저장하고, 끝날 때(취소 포함) 저장한 모드로 되돌린다(사용자 결정 2026-10-21). 저장하거나 되돌리지 못하면 `runtime_unavailable`로 끝나고 계정 파일을 바꾸지 않는다(2.2절, 6절). |
| 취소 | 비밀번호 입력 중 Ctrl+C 또는 입력 끝(EOF)은 취소다. `input_canceled`로 끝나고 계정 파일을 바꾸지 않는다. Windows 숨김 입력의 입력 끝 키는 Ctrl+D다. 빈 입력에서 Ctrl+D를 누르면 취소다(사용자 결정 2026-10-21). Ctrl+Z 뒤 Enter는 입력 끝이 아니다. 빈 줄로 받아 `password_invalid`(종료 코드 1)로 끝나고 계정 파일을 바꾸지 않는다(실행 기록). |
| 삭제 확인 | `admin remove`는 `계정 '<account>'을(를) 삭제할까요? [y/N]`을 stderr에 쓰고 한 줄을 읽는다. `y` 또는 `yes`(대소문자 무시)만 승인이다. 그 밖의 입력은 취소다. `--yes`가 있으면 묻지 않는다. 터미널이 아니고 `--yes`도 없으면 `not_interactive`다. |
| 반영 안내 | 계정을 바꾸는 명령은 성공한 뒤 stderr에 `notice`를 한 줄 쓴다: 실행 중인 서버에는 반영되지 않으며 서버를 정상 종료하고 다시 시작해야 한다(1.7절). |
| 마지막 계정 삭제 | 허용한다. 삭제 뒤 계정이 0개면 경고 `last_account_removed`를 stderr에 쓴다. 서버는 이 상태로 시작하지 않는다. |
| 계정 파일이 없을 때 | `add`는 새 파일을 만든다. 상위 디렉터리는 만들지 않는다. `list`는 stdout을 비우고 stderr에 안내 한 줄을 쓰며 종료 코드 0이다. `passwd`·`remove`는 `account_not_found`다. `serve`는 `accounts_file_missing`이다. |
| 글자 인코딩 | UTF-8. Windows 11에서 숨김 입력은 UTF-8 바이트로 저장했다. 콘솔 출력은 코드 페이지 949에서 한글 오류 줄이 바르게 보였다. 파이프·파일 출력은 UTF-8 바이트다. 읽는 쪽은 UTF-8로 해석해야 한다(PowerShell은 `[Console]::OutputEncoding`을 UTF-8로 맞춘다). 관찰 기록은 [실행 기록](#계약-테스트-실행-기록)에 있다. |

명령별 순서:

1. `add`: 이름 검사 → 설정 읽기 → 계정 파일 읽기 → 같은 이름이 있으면 `account_exists` → 비밀번호 두 번 입력과 검사 → Argon2id 해시 → 저장(1.6절) → 성공 문장.
2. `passwd`: 이름 검사 → 설정 읽기 → 계정 파일 읽기 → 이름이 없으면 `account_not_found` → 비밀번호 두 번 입력과 검사 → 해시 → 저장 → 성공 문장. 현재 비밀번호는 묻지 않는다. 계정 파일을 쓸 수 있는 운영자가 로컬 권한을 가진다.
3. `remove`: 이름 검사 → 설정 읽기 → 계정 파일 읽기 → 이름이 없으면 `account_not_found` → 삭제 확인 → 저장 → 성공 문장.
4. `list`: 설정 읽기 → 계정 파일 읽기 → 이름순 출력.

저장하기 전에 실패하면(입력 취소·검증 실패·파일 오류) 계정 파일을 읽기만 했고 바꾸지 않았다. 성공 문장은 저장이 끝난 뒤에만 쓴다.

### 1.6 저장과 파일 권한

계정 파일의 변경은 읽고 고치고 교체하는 한 번의 작업이다. 모든 단계를 지킨다.

1. 계정 파일 옆에 잠금 파일 `<계정 파일 이름>.lock`을 독점으로 만든다. 이미 있으면 기다리지 않고 `accounts_file_locked`로 끝난다. 끝나면 지운다. 명령이 강제로 끝나면 잠금 파일이 남을 수 있다. 실행 중인 `admin` 명령이 없으면 운영자가 지운다. `accounts_file_locked` 메시지가 잠금 파일 경로와 이 조건을 알린다(사용자 결정 2026-10-20). 잠금 파일을 만들 수 없으면(디렉터리 없음·권한 없음) `accounts_file_write_failed`다(사용자 결정 2026-10-20).
2. 계정 파일을 읽고 1.3절 검사를 한다. 실패하면 `accounts_file_invalid` 등으로 끝나고 아무것도 쓰지 않는다.
3. 같은 디렉터리에 임시 파일을 새로 만든다. 만든 즉시 소유자만 읽고 쓰도록 권한을 제한한 뒤 내용을 쓴다.
4. 임시 파일 내용을 디스크에 반영한다. 다시 읽어 1.3절 검사를 통과하는지 확인한다.
5. 임시 파일로 대상 파일을 한 번에 교체한다. 같은 볼륨의 원자적 교체 OS 호출을 쓴다. 대상을 먼저 지우고 쓰지 않는다.
6. Unix에서는 디렉터리 내용도 디스크에 반영한다.
7. 잠금 파일을 지운다.

4단계까지 어느 단계가 실패해도 임시 파일을 지우고 대상 파일은 그대로다. 5단계가 실패해도 대상 파일은 원래 내용이다. 이때 종료 코드는 5, 코드는 `accounts_file_write_failed`다. 교체가 끝난 뒤 실패한 단계(6·7)는 경고 `accounts_file_cleanup_failed`를 쓰고 성공으로 끝난다.

교체가 실패한 뒤 잠금 파일을 지우지 못하면 원래 오류가 결과다. 종료 코드와 코드는 그대로다. 이때 경고 `accounts_file_cleanup_failed`를 함께 쓴다(사용자 결정 2026-10-21).

파일 권한(권장):

| OS | 권장 |
| --- | --- |
| Ubuntu 24.04 LTS | 계정 파일 권한 `0600`, 소유자는 서버를 실행하는 OS 계정. 상위 디렉터리를 다른 계정이 쓰지 못하게 한다. CLI는 새 파일을 `0600`으로 만든다. 읽을 때 그룹·기타 권한이 있으면 경고 `accounts_file_permissive`를 쓴다. 경고만 하고 계속한다. |
| Windows 11 | 서버를 실행하는 OS 계정과 관리자만 읽고 쓰도록 운영자가 디렉터리의 접근 제어 목록을 제한한다. CLI는 접근 제어 목록을 바꾸지 않고 권한 경고도 쓰지 않는다. 새 파일은 디렉터리의 접근 제어 목록을 물려받는다(사용자 결정 2026-10-20). |

설정 파일의 `key_file`도 같은 원칙으로 권한을 제한하기를 권장한다. 계정 파일·개인키 파일은 버전 관리에 넣지 않는다.

### 1.7 서버 실행 중 계정 파일 변경

- `serve`는 시작 때 listener를 열기 전에 계정 파일을 한 번 읽는다. 읽은 계정 집합을 메모리에 둔다.
- 서버는 계정 파일을 감시하지 않는다. 운영 스펙의 재로드 범위에 계정이 없다([운영 스펙 3.3절](../specs/operations.md#33-파일-변경-자동-감지와-부분-재로드)). 실행 중에 계정을 바꾸는 네트워크 API도 없다([운영 스펙 3.2절](../specs/operations.md#32-로컬-cli-계정-준비)).
- 따라서 계정 추가·삭제·비밀번호 변경은 서버를 정상 종료하고 `admin` 명령을 실행한 뒤 다시 시작해야 적용된다. 실행 중 `admin` 명령은 파일을 안전하게 교체하지만 실행 중인 서버에는 반영되지 않는다.
- 삭제한 계정으로 이미 발급된 관리 세션과 바꾸기 전 비밀번호로 만든 관리 세션은 서버를 재시작하기 전까지 유효하다. 재시작하면 모든 관리 세션이 무효가 된다([관리 스펙 3.0절](../specs/management.md#30-관리자-로그인과-세션)). 즉시 막으려면 서버를 재시작해야 한다.
- 계정 파일 변경은 설정 재로드 후보 비교에 영향을 주지 않는다. 계정 파일 경로(`admin.accounts_file`) 값의 변경만 재시작 필요 변경이다.

### 1.8 수동 시작과 종료

수동 시작·종료 방식이다.

| 항목 | 규칙 |
| --- | --- |
| 시작 | 터미널에서 `vnc-relay serve --config <path>`를 실행한다. 프로세스는 종료 때까지 돌아오지 않는다. 백그라운드 전환(daemon), PID 파일, OS 서비스 등록은 하지 않는다. |
| 시작 순서 | (1) 설정 읽기·검증과 로그 출력 열기, (2) 계정 파일 읽기·검증, (3) 파일 변경 감지 준비, (4) 리스너·TLS 준비. 다음 단계는 앞 단계가 성공해야 한다. 처음 실패한 단계에서 멈추고 확보한 자원을 회수한다. 1단계는 설정 검증이 끝난 뒤 `log.output`의 로그 파일을 연다. 열지 못하면 `log_file_unavailable`(종료 코드 3)로 멈춘다. 4단계는 먼저 비동기 런타임과 종료 신호 처리기를 준비하고, 시작 키(RelayIdKey·SessionIdKey)를 운영체제 난수원으로 생성한다. 둘 중 하나라도 준비하거나 생성하지 못하면 `runtime_unavailable`(종료 코드 1)로 멈춘다(사용자 결정 2026-11-01(Q76)). 이어 인증서·개인키 파일 검사(`tls_file_invalid`, 종료 코드 3)를 리스너 바인드(`listener_bind_failed`, 종료 코드 4)보다 먼저 한다. 인증서·개인키 파일 오류가 있으면 종료 코드는 3이고, 그 오류가 없을 때의 바인드 실패만 4다. 모두 성공하면 준비됨이다([운영 스펙 2절](../specs/operations.md#2-상태와-검증-매개변수)). 각 단계의 실패는 stderr의 오류 줄과 함께 운영 로그의 `startup_failed`로도 기록한다([운영 정보 계약 1.5절](ops-info-v1.md#15-사건-목록)). |
| 종료 요청 | Ctrl+C와 SIGTERM을 정상 종료 요청으로 처리한다. Ubuntu는 SIGINT(Ctrl+C)와 SIGTERM이다. Windows 11은 Ctrl+C와 Ctrl+Break다. Windows에는 SIGTERM이 없다. Windows 콘솔 창 닫기는 종료 요청과 두 번째 종료 요청을 함께 받은 것으로 처리한다. G 없이 남은 세션을 즉시 강제 정리하고, OS가 창을 닫은 뒤 기본 5초 안에 프로세스를 끝내기 전에 종료 기록을 마친다([운영 스펙 3.4절](../specs/operations.md#34-정상-종료와-재시작), 사용자 결정 2026-11-04(Q102)). |
| 종료 절차 | 정상 종료 요청을 받으면 [운영 스펙 3.4절](../specs/operations.md#34-정상-종료와-재시작)을 따른다. 유예 시간은 `timeouts.shutdown_grace_secs`다. 유예가 끝나 남은 세션을 강제 정리하는 경우도 정상 종료 요청에 따른 종료이며 종료 코드 0이다. 이때 세션의 종료 원인은 아직 정리를 시작하지 않은 세션에만 `server_shutdown`을 기록하고, 이미 정리를 시작한 세션은 최초 정리 시작 원인을 유지한다(사용자 결정 2026-11-01(Q82)). |
| 실행 중 리스너 장애 | 복구 불가능 수락 오류는 `shutdown_started(trigger=fatal_error)`와 기존 G·정리 절차로 끝낸다. 종료 코드는 1이다. stderr에는 `error[listener_failed]`와 해당 `listen` 키를 쓴다. 바인드 실패(4)나 `startup_failed`가 아니다. [운영 정보 계약 1.12절](ops-info-v1.md#112-상태-리스너와-엔드포인트)을 따른다. 사용자 결정 2026-11-05(Q114). |
| 두 번째 종료 요청 | 사용자 승인(2026-10-16). 종료 유예 중 두 번째 종료 요청(Ctrl+C·SIGTERM 등)을 받으면 남은 유예를 건너뛰고 남은 세션을 즉시 강제 정리한다. 세션의 종료 원인은 아직 정리를 시작하지 않은 세션에만 `server_shutdown`을 기록하고, 이미 정리를 시작한 세션은 최초 정리 시작 원인을 유지한다(사용자 결정 2026-11-01(Q82)). 정상 종료 요청에 따른 종료이므로 정리가 끝나면 종료 코드 0이다. 외부 동작은 [운영 스펙 3.4절](../specs/operations.md#34-정상-종료와-재시작)이 정한다. |
| 강제 종료 | SIGKILL, 작업 관리자의 프로세스 종료, 전원 장애는 정상 유예를 보장하지 않는다. Windows 로그오프·시스템 종료도 강제 종료다. 이 신호는 서비스만 받고 대화형 콘솔 프로그램은 그 전에 OS가 끝낸다([Microsoft HandlerRoutine 문서](https://learn.microsoft.com/en-us/windows/console/handlerroutine). 이 PC에서 로그오프는 관찰하지 않았다). 사용자 결정 2026-11-04(Q102). |
| 두 번째 인스턴스 | 이미 사용 중인 리스너 주소로 시작하면 4단계에서 `listener_bind_failed`가 난다. 종료 코드 4다. 시작 중 확보한 자원을 회수하고 첫 번째 프로세스의 대기·활성 세션에 영향을 주지 않는다. 다른 주소를 쓰는 설정의 프로세스는 막지 않는다. 두 OS 모두에서 주소 공유를 허용하지 않는 방식으로 바인드해야 한다. 특히 Windows의 주소 재사용 동작은 관찰로 확인 필요다. |

두 번째 인스턴스 시작 실패의 출력 예(stderr). 종료 코드는 4다. `log.output = "stdout"`이면 stdout에는 운영 로그 줄(`server_starting`, `startup_failed`)만 있다. 로그 파일을 쓰면 stdout은 비어 있다.

```text
error[listener_bind_failed] key=relay.tcp_viewer.listen: 0.0.0.0:15901에 바인드할 수 없다. 이미 사용 중이다.
```

## 2. 오류

### 2.1 종료 코드

| 코드 | 이름 | 의미 |
| --- | --- | --- |
| `0` | 성공 | 명령이 끝났다. `serve`는 정상 종료 요청의 정리가 끝났다(유예 만료·두 번째 종료 요청으로 강제 정리한 경우 포함). `--help`·`--version`·`admin list`(계정 0개 포함)도 해당한다. |
| `1` | 실행 중 오류 | 명령을 실행하는 중에 실패했다. 입력 취소, 비밀번호 검사 실패, 계정 중복·없음, 변경 감지 준비 불가, 비동기 런타임·종료 신호 처리기 준비 불가, 서버가 계속 운영할 수 없는 실패, 정리 실패가 해당한다. |
| `2` | 사용법 오류 | 알 수 없는 하위 명령·옵션, `--config` 누락·중복, 인수 개수 오류, 계정 이름 규칙 위반, 터미널이 아닌 입력. 사용법은 `usage_error`일 때만 stderr에 쓴다(사용자 결정 2026-10-20). |
| `3` | 설정 오류 | 설정 파일의 읽기·구문·키·값 오류 전체, 인증서·개인키 파일 오류(경로·형식 오류, 인증서와 개인키가 맞지 않음), 로그 파일을 열 수 없음. 시작 실패 포함. 코드는 [설정 파일 계약 2.1절](config-v2.md#21-설정-오류와-준비-실패)의 `config_*`·`tls_file_invalid`·`log_file_unavailable`이다. |
| `4` | 리스너 준비 실패 | 리스너 바인드 실패(이미 사용 중인 주소 포함). |
| `5` | 계정 파일 오류 | 계정 파일을 읽을 수 없거나 형식·해시 하한이 맞지 않거나, 파일이 없거나(`serve`), 계정이 하나도 없거나, 잠금·저장에 실패했다. |

### 2.2 오류 코드

| 오류 코드 | 의미·형식 | 종료 코드 | 관련 스펙 시나리오 |
| --- | --- | --- | --- |
| `usage_error` | 사용법 오류(1.1절). | 2 | OPS-030 |
| `account_name_invalid` | 계정 이름이 1.2절 규칙을 어겼다. | 2 | OPS-030 |
| `not_interactive` | 표준 입력이 터미널이 아니라 숨김 입력·확인 입력을 받을 수 없다. | 2 | OPS-029 |
| `account_exists` | `add`: 같은 이름이 이미 있다. | 1 | OPS-029 |
| `account_not_found` | `passwd`·`remove`: 이름이 없다. | 1 | OPS-029 |
| `password_invalid` | 비밀번호가 1.2절 규칙을 어겼다. | 1 | OPS-030 |
| `password_mismatch` | 두 번 입력한 비밀번호가 다르다. | 1 | OPS-030 |
| `input_canceled` | 입력 취소(Ctrl+C·EOF)·삭제 확인 거절. 계정 파일은 그대로다. | 1 | OPS-030 |
| `watch_unavailable` | `serve`: 파일 변경 감지를 준비할 수 없다. 묵시적으로 끄고 시작하지 않는다. 로그 디렉터리 알림 등록만 실패한 경우는 해당하지 않는다(사용자 결정 2026-11-04(Q111)). | 1 | OPS-028 |
| `runtime_unavailable` | `serve`: 비동기 런타임이나 종료 신호 처리기를 준비할 수 없거나, 운영체제 난수원으로 시작 키(RelayIdKey·SessionIdKey)를 만들 수 없다(사용자 결정 2026-11-01(Q76)). `admin`: 입력 취소(Ctrl+C) 감지기를 준비할 수 없거나, 운영체제 난수원·해시 계산 메모리를 쓸 수 없다(사용자 결정 2026-10-20). 비밀번호·확인 입력을 읽는 스레드를 만들 수 없거나 그 스레드가 결과 없이 끝난 경우도 이 코드다. Windows에서 숨김 입력 전에 콘솔 입력 모드를 읽지 못하거나 입력 뒤에 되돌리지 못한 경우도 이 코드다(구현 판단, 6절). 이때 계정 파일은 바뀌지 않는다. 운영체제 자원(스레드·이벤트 대기 핸들) 부족 같은 실행 환경 문제다. 키는 없다. | 1 | 없음 |
| `accounts_file_missing` | `serve`: 계정 파일이 없다. | 5 | OPS-001 |
| `accounts_file_unreadable` | 계정 파일이 있으나 읽을 수 없다(권한·입출력 오류). | 5 | OPS-030 |
| `accounts_file_invalid` | 계정 파일 형식 오류(1.3절)와 해시 문자열 형식 오류(1.4절). | 5 | OPS-030 |
| `accounts_hash_below_minimum` | `serve`: 저장된 해시의 `m`·`t`·`p` 중 하나가 승인된 하한 미만이다. `admin` 명령은 이 해시를 받는다(1.4절). | 5 | OPS-029 |
| `accounts_none` | `serve`: 읽은 계정이 하나도 없다. 시작 실패다. `admin add`로 준비한 뒤 다시 시작한다. | 5 | OPS-001 |
| `accounts_file_locked` | 다른 `admin` 명령이 계정 파일을 고치는 중이다(1.6절). | 5 | OPS-030 |
| `accounts_file_write_failed` | 잠금 파일 만들기, 임시 파일 쓰기·검증, 교체에 실패했다. 기존 계정 파일은 그대로다. | 5 | OPS-030 |
| `listener_bind_failed` | 리스너 주소에 바인드하지 못했다. 이미 사용 중인 경우를 포함한다. `key`는 해당 `listen` 키다. | 4 | OPS-002, OPS-039 |
| `listener_failed` | `serve`: 실행 중 필수 리스너의 복구 불가능 수락 오류. `key`는 해당 `listen` 키다. 원시 OS 코드와 리스너 이름만 진단에 넣는다. | 1 | OPS-046 |
| `tls_file_invalid` | 인증서·개인키 경로·형식 오류. 의미는 [설정 파일 계약 2.1절](config-v2.md#21-설정-오류와-준비-실패)이 정한다. 시작 때 4단계에서 보고하지만 종료 코드는 설정 오류와 같다. | 3 | OPS-002 |
| `log_file_unavailable` | `serve`: `log.output`의 로그 파일을 열 수 없다. 의미는 [설정 파일 계약 2.1절](config-v2.md#21-설정-오류와-준비-실패)이 정한다. `key`는 `log.output`이다. | 3 | 없음 |
| `config_*` | 설정 오류. 이름은 [설정 파일 계약](config-v2.md#21-설정-오류와-준비-실패)을 따른다. | 3 | OPS-001 |

경고(종료 코드에 영향 없음): `accounts_file_permissive`, `last_account_removed`, `accounts_file_cleanup_failed`(`admin`: 계정 파일을 교체한 뒤 디렉터리 반영이나 잠금 파일 삭제에 실패했다. 교체가 실패한 뒤 잠금 파일 삭제에 실패해도 오류와 함께 나온다. 1.6절), `log_write_failed`(`serve`: 운영 로그 줄을 쓰지 못했다. 처음 한 번만 쓴다. `key`는 `log.output`이다. [운영 정보 계약 1.1절](ops-info-v1.md#11-로그-일반-규칙)). 설정 경고는 [설정 파일 계약 2.2절](config-v2.md#22-경고)을 따른다.

## 3. 호환성 규칙

- 데이터를 사용하는 쪽은 알 수 없는 하위 명령·옵션을 `usage_error`로 거절해야 한다. `serve`는 계정 파일의 알 수 없는 키·알 수 없는 `version`·허용하지 않는 해시 알고리즘을 `accounts_file_invalid`로 거절해야 한다. 스크립트는 종료 코드와 `error[<code>]`의 `code`로 판단하고 `message` 문구에 의존하지 않아야 한다.
- 호환 변경(minor 올림): 새 하위 명령, 새 선택 옵션, 새 오류·경고 코드 추가, 메시지 문구 변경. 새 오류 코드는 기존 종료 코드 안에서만 쓴다.
- 호환성을 깨는 변경(새 major 버전): 하위 명령·옵션 이름 변경이나 삭제, 종료 코드 숫자·의미 변경, `admin list` 출력 형식 변경, 계정 파일 `version`·필드·해시 형식 변경, 입력 방식(숨김 입력) 완화. 새 major는 `vnc-relay-cli` 2.0.0으로 올리고 마이그레이션(v1 계정 파일을 v2로 바꾸는 방법)과 롤백(v2 파일을 v1로 되돌리는 방법) 계획을 함께 작성한다. 지금은 v1만 있어 계획이 없다.
- 해시 알고리즘·매개변수 하한을 올리는 일은 새 major가 아니라 설정·계정 파일의 값 변경이다. 하한을 올리면 기존 해시가 거절되므로 `admin passwd`로 새 해시를 만들어야 한다는 안내를 함께 낸다.

## 4. 버전과 지원 단계

| 상태 | 조건 | 지원 종료·대체 |
| --- | --- | --- |
| Draft | 채택 전 상태. 이 계약은 2026-10-16에 채택해 Draft를 지났다. | 해당 없음 |
| Active | 현재. 사용자 채택(2026-10-16). 구현과 양 OS의 계약 테스트 통과·검증 기록은 5절 표가 관리하며, 통과하기 전에는 지원 완료로 표시하지 않는다. | v2가 Active가 된 뒤 Deprecated로 바꾼다. |
| Deprecated | 대체 계약이 Active이다. | 지원 종료 날짜는 대체 계약에서 정한다. |
| Retired | 지원 종료. | 새 구현의 근거로 쓰지 않는다. |

## 5. 예제와 계약 테스트

예제에 비밀번호·해시 값은 없다. 해시 칸은 자리표시자다. 경로·이름은 임의 값이다.

계정 추가(stderr에 프롬프트와 안내, stdout에 성공 문장). 입력한 글자는 화면에 보이지 않는다.

```text
$ vnc-relay admin add alice --config ./relay.toml
비밀번호:
비밀번호 확인:
계정 'alice'을(를) 추가했다.
notice: 실행 중인 서버에는 반영되지 않는다. 서버를 정상 종료하고 다시 시작해야 한다.
```

```text
$ vnc-relay admin list --config ./relay.toml
alice
```

계정 파일(자리표시자 해시):

```toml
version = 1

[[account]]
name = "alice"
password_hash = "$argon2id$v=19$m=19456,t=2,p=1$<솔트 Base64>$<해시 Base64>"
```

실패 예(stderr, 종료 코드 2):

```text
error[account_name_invalid]: 계정 이름은 소문자 영문·숫자·점·밑줄·하이픈 1~32자여야 한다.
```

결과와 증거는 [검증 기록](../AGENTS.md#검증-기록)을 따른다. 스펙과 같은 테스트는 해당 시나리오의 기록을 연결한다. 작업 2.2의 테스트는 `src/cli.rs` 단위 테스트, `tests/cli.rs`·`tests/serve.rs`(실행파일 시스템 테스트), `crates/adapters/tests/accounts_contract.rs`(아래 표에서 "계정 파일 계약 테스트"), `crates/domain/src/account.rs` 단위 테스트에 있다. 작업 4.1의 테스트는 `tests/admin.rs`(실행파일 시스템 테스트), `crates/app/src/accounts.rs` 단위 테스트, `crates/adapters/src/accounts_file/hasher.rs` 단위 테스트에 있다. 실행 기록은 표 아래 [계약 테스트 실행 기록](#계약-테스트-실행-기록)에 있다. 아직 기능이 없는 규칙은 미실행이다.

| 주체·테스트 위치 | 확인할 계약 규칙 | 결과 | 실행 기록 |
| --- | --- | --- | --- |
| Producer·`tests/admin.rs` OPS-029 테스트 `list_prints_account_names_in_name_order`·`list_without_an_account_file_prints_a_notice_and_exits_0`·`list_accepts_a_hash_below_the_serve_minimum`·`list_warns_when_others_can_read_the_account_file`(Ubuntu)·`remove_with_yes_deletes_one_account_and_keeps_the_others`·`removing_the_last_account_warns_and_list_shows_none`·`account_commands_without_a_terminal_exit_2_without_usage_text`·`existing_and_unknown_names_fail_before_any_password_prompt`·`tables_other_than_admin_are_not_checked`·`add_saves_two_accounts_with_argon2id_hashes_from_hidden_input`(Ubuntu)·`passwd_replaces_only_the_named_hash`(Ubuntu) | 계정 두 개 이상 준비, 숨김 입력, 솔트 포함 Argon2id 해시(1.4절), 하한 이상, 모든 계정이 같은 권한. [OPS-029](../specs/operations.md#7-수용-테스트) | 부분 통과. 계정 준비, 숨김 입력, 솔트 포함 Argon2id 해시는 통과했다. Ubuntu는 터미널 테스트를 실행했다. Windows는 터미널이 없는 테스트만 실행했다. 각 계정으로 로그인하는 검사와 모든 계정의 같은 권한 검사는 작업 4.3이 맡으며 실행하지 않았다. | [실행 기록](#계약-테스트-실행-기록) |
| Producer·`tests/admin.rs` OPS-030 테스트 `config_problems_exit_3_with_their_codes`·`broken_account_files_exit_5_and_are_not_changed`·`an_account_path_that_is_a_directory_exits_5`·`a_lock_file_stops_account_changes_and_is_left_in_place`·`a_failed_save_exits_5_and_keeps_the_account_file`·`rejected_password_entries_exit_1_and_keep_the_account_file`(Ubuntu)·`ctrl_c_at_the_password_prompt_cancels_and_restores_the_terminal`(Ubuntu)·`remove_confirmation_accepts_only_yes_and_ctrl_c_cancels`(Ubuntu)·`every_failed_preparation_keeps_the_account_file_and_prints_no_secret`(Ubuntu) | 입력 취소·검증 실패·저장 실패에서 기존 계정 파일 유지, 출력·로그에 비밀 없음(1.5·1.6절). [OPS-030](../specs/operations.md#7-수용-테스트) | 통과(Ubuntu). Windows는 `tests/admin.rs` 14건을 실행해 통과했다. 터미널 테스트는 Linux 전용이라 실행하지 않았다. Windows 시나리오 판정은 부분이다. Windows 콘솔 수동 관찰(2026-10-21, 저장·복원 구현 `f21a6bc`, 측정 `ec2e6e6`): 취소 18회와 정상 입력 1회 모두 뒤에 콘솔 입력 모드가 시작 값과 같았다(19/19). 복원 전 관찰(`ed68104`)은 Ctrl+C 20/20 취소와 입력 모드 미복원이었다. 자세한 값은 [실행 기록](#계약-테스트-실행-기록)에 있다. | [실행 기록](#계약-테스트-실행-기록) |
| Producer·`src/cli.rs` 단위 테스트, `tests/cli.rs` | `serve --config`(`=` 형식 포함)·`--help`·`--version` 파싱, `--config` 누락·중복·빈 값, 알 수 없는 하위 명령·옵션의 `usage_error`와 종료 코드 2, 사용법을 stderr에 씀, 도움말·버전은 stdout과 종료 코드 0, 진단 줄의 ` key=` 생략(1.1·1.5·2.1절) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Producer·`src/cli.rs` 단위 테스트 `parses_admin_subcommands_with_their_own_options`, `crates/domain/src/account.rs` 단위 테스트 `accepts_names_inside_the_rule_only`·`accepts_passwords_inside_the_rule_and_names_the_broken_part`, `tests/admin.rs` `invalid_account_names_exit_2_and_create_no_file`·`account_commands_without_a_terminal_exit_2_without_usage_text`, `tests/cli.rs` `usage_errors_exit_with_code_2_and_usage_on_stderr` | `admin` 하위 명령·옵션 파싱, 비밀번호 규칙, `account_name_invalid`(1.1·1.2·2절). 사용법은 `usage_error`일 때만 stderr에 쓴다. `not_interactive`와 `account_name_invalid`는 종료 코드 2이지만 사용법을 쓰지 않는다(2.1절). [OPS-030](../specs/operations.md#7-수용-테스트) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Producer·`crates/adapters/tests/accounts_contract.rs` 쓰기 테스트 `creates_a_missing_file_in_name_order_and_leaves_no_lock_or_temporary_file`·`keeps_the_file_when_the_change_fails_the_lock_exists_or_writing_fails`·`a_failed_change_returns_its_error_and_warns_when_the_lock_cannot_be_removed`·`fails_to_write_when_the_directory_does_not_exist`·`admin_reads_hashes_below_the_minimum`·`admin_refuses_an_above_maximum_parameter_even_with_a_below_minimum_one`, `crates/adapters/src/accounts_file/hasher.rs` 단위 테스트 `makes_salted_argon2id_hashes_that_verify_the_password`, `crates/app/src/accounts.rs` 단위 테스트 `adds_an_account_after_two_matching_entries_and_creates_a_missing_file`·`checks_the_name_before_asking_and_again_under_the_lock`·`stops_before_saving_on_invalid_mismatched_or_canceled_input`·`changes_only_the_hash_of_the_named_account`·`removes_only_after_y_or_yes_and_warns_when_none_is_left`·`passes_on_cleanup_warnings_after_saving`·`lists_names_in_name_order_and_tells_a_missing_file` | 저장 순서(1.6절): 잠금 파일 독점 생성, 다시 읽기, 임시 파일 쓰기와 교체, 잠금 파일 삭제. 잠금 파일이 있으면 `accounts_file_locked`다. 저장 실패 때 대상 파일과 다른 임시 파일은 그대로다. 디렉터리가 없으면 `accounts_file_write_failed`다. 교체가 실패한 뒤 잠금 파일 삭제도 실패하면 원래 오류와 경고 `accounts_file_cleanup_failed`를 함께 낸다(1.6절). 이름순 쓰기(1.3절). 솔트 포함 Argon2id 해시(1.4절). 명령별 순서(1.5절): 이름 검사 뒤 입력, 취소·검증 실패 때 저장하지 않음, 정리 경고 뒤 성공. | 통과. 2026-10-21에 추가한 시험 `a_failed_change_returns_its_error_and_warns_when_the_lock_cannot_be_removed`는 Windows 11(`ec2e6e6`)의 `accounts_contract` 12건에 들어 통과했다. Ubuntu는 `ec2e6e6`의 `vr-full.sh`가 `ALL PASSED`로 끝났고, 건수는 기록하지 않았다. | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·계정 파일 읽기 계약 테스트 7건(Ubuntu. Windows에서는 Unix 권한 테스트 1건을 뺀 6건), `crates/domain/src/account.rs` 단위 테스트(`checks_parameters_against_approved_ranges` 포함) | 계정 파일 형식(1.3절): `version`, 알 수 없는 키, 자료형, 이름 규칙(1.2절), 중복 이름, 파일 안 경로의 `key`. 해시 문자열 형식(1.4절)과 하한 미만·상한 초과 구분. 구문·UTF-8 오류, 파일 없음·읽기 불가. 메시지에 해시가 없음. Ubuntu 권한 경고(1.6절) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·`tests/serve.rs` `refuses_to_start_without_a_usable_account_with_exit_code_5` | 계정 없음·계정 파일 없음·형식 오류·해시 하한 미만은 시작 실패(종료 코드 5), 리스너를 열기 전에 멈춤, 출력에 해시 없음. [OPS-001](../specs/operations.md#7-수용-테스트), [OPS-029](../specs/operations.md#7-수용-테스트)의 하한 부분 | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·`tests/management_session.rs` `cli_prepared_admins_have_hidden_salted_credentials_and_equal_relay_rights`(Linux, PTY), `tests/windows_console.rs`의 같은 이름 시험(Windows, ConPTY) | `admin add`로 계정 두 개를 준비할 때 비밀번호를 화면에 보이지 않음, 계정 파일에 평문 비밀번호 없음, 계정마다 다른 솔트의 Argon2id 해시(`m=19456,t=2,p=1`), 두 계정 모두 로그인·조회·중계 세션 종료 가능, 다른 계정의 비밀번호는 `login_failed`(1.2~1.4절). Linux는 파일 권한 `0600`과 디렉터리의 다른 사용자 권한 없음도 확인한다. Windows 계정 파일 ACL은 1.6절에 따라 운영자 책임이다(사용자 결정 2026-10-20). [OPS-029](../specs/operations.md#7-수용-테스트) | 통과 | Linux는 PTY 시험(`tests/management_session.rs`), Windows는 ConPTY 시험(`tests/windows_console.rs`, 사용자 결정 2026-11-04(Q103))으로 두 OS에서 모두 통과했다(커밋 `de693ad`). Windows 시험은 운영자 책임인 파일 권한 검사를 제외하고 동일한 숨김 입력·해시·로그인·중계 권한을 확인한다. |
| Consumer·`tests/serve.rs` `second_instance_fails_with_exit_code_4_and_leaves_the_first_running`, `fails_before_ready_and_frees_every_port_when_a_port_is_taken_or_tls_fails` | 두 번째 인스턴스 시작 실패(종료 코드 4)와 리스너마다 `listener_bind_failed`, 앞서 바인드한 리스너 회수, 첫 인스턴스의 리스너 유지(1.8절). 내장 TLS 준비 실패는 리스너를 바인드하기 전에 종료 코드 3으로 멈춘다(사용자 결정 2026-10-22(Q26)). [OPS-039](../specs/operations.md#7-수용-테스트), [OPS-002](../specs/operations.md#7-수용-테스트) | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·`serve` 통합 테스트(미생성, 작업 2.10) | 두 번째 인스턴스 시작 때 첫 인스턴스의 대기·활성 세션 유지(1.8절). [OPS-039](../specs/operations.md#7-수용-테스트) | 미실행 | 세션 미구현 |
| Consumer·`tests/serve.rs` `stops_with_exit_code_0_on_interrupt_and_terminate`, `second_instance_fails_with_exit_code_4_and_leaves_the_first_running` | Ctrl+C(SIGINT)·SIGTERM을 받으면 리스너를 닫고 종료 코드 0(1.8절). 세션이 없는 경우만 확인한다. | 통과 | [실행 기록](#계약-테스트-실행-기록) |
| Consumer·`src/serve/listener_tests.rs` `listener_failures_retry_then_drive_fatal_grace_cleanup_and_preserve_requested_races` | 실행 중 리스너 장애가 먼저이면 G·정리 뒤 1, 정상 요청이 먼저이면 기존 0과 유예 보존(1.8·2.1절). 실제 TCP·상태 HTTP와 `serve` 감독 함수를 쓴다. [OPS-046](../specs/operations.md#7-수용-테스트). | 통과(두 OS 대상 시험) | 부모 실행 기록(2026-11-05). Q114 통합 작업 트리에서 `cargo test -p vnc-relay --bin vnc-relay serve::listener_tests:: -- --nocapture`: Windows·Ubuntu 각각 1통과·0실패, 약 2.3초. 종료 코드 반환값을 확인하는 조립 시험이다. [운영 정보 계약의 독립 스모크 기록](ops-info-v1.md#5-계약-테스트)은 실제 OS 오류와 준비 상태·중계를 별도로 확인했다. 실제 제품 바이너리 소켓 손상이나 stderr 진단의 실행 증거는 아니다. `b5dc9b5` Windows 전체 게이트는 별도 DASH-024 실패로 전체 실패이며 수정 뒤 최종 게이트 판정은 남는다. |
| Consumer·`tests/serve.rs` `fails_with_exit_code_3_when_the_log_file_cannot_be_opened`, `warns_once_when_log_writes_fail_and_keeps_running`, `writes_the_log_to_the_configured_file_from_start_to_stop` | 시작 순서 1단계의 로그 출력 열기: 열 수 없으면 `error[log_file_unavailable] key=log.output`과 종료 코드 3, 실패 줄은 stdout에 쓴다. `log.output`이 파일이면 stdout은 비어 있다. 로그 쓰기 실패는 `warning[log_write_failed]`를 한 번만 쓰고 계속 실행한다(1.5·1.8·2절). | 통과 | [운영 정보 계약 실행 기록](ops-info-v1.md#계약-테스트-실행-기록) |
| Consumer·`tests/unix_shutdown.rs`와 `tests/windows_console.rs`의 `stop_unfinished_work`·`active_session_finishes_in_grace`·`grace_expiry_cleans_session`·`restart_has_fresh_state`·`second_request_forces_cleanup`. Windows는 `ctrl_c_stops_gracefully_with_exit_code_zero`·`ctrl_break_stops_gracefully_with_exit_code_zero`·`console_close_forces_cleanup`도 확인한다. | 유예 안 자발 종료·유예 만료, 유예 중 두 번째 종료 요청의 즉시 강제 정리, Windows의 Ctrl+C·Ctrl+Break, Windows 콘솔 창 닫기의 즉시 강제 정리(1.8절). [OPS-007](../specs/operations.md#7-수용-테스트)~[OPS-010](../specs/operations.md#7-수용-테스트), [OPS-042](../specs/operations.md#7-수용-테스트), [OPS-044](../specs/operations.md#7-수용-테스트). [OPS-019](../specs/operations.md#7-수용-테스트)는 두 OS의 종료 부분만 이 행에 해당한다. OPS-019 전체는 작업 5.6에서 판정한다. | 통과 | 공통 시나리오 본문은 `tests/common/shutdown.rs`다. 콘솔 창 닫기는 `ShutdownSignal::wait`가 `ShutdownRequest::ConsoleClosed`로 돌려주고, `begin_relay_shutdown`이 G로 유예를 시작한 뒤 곧바로 `Duration::ZERO`로 강제 정리를 요청한다(사용자 결정 2026-11-04(Q102)). 커밋 `de693ad` 게이트에서 Linux(SIGTERM)와 Windows(ConPTY 신호·창 닫기) 모두 통과했다. 창 닫기 시 5초 이내 프로세스 종료를 단언한다. |
| Consumer·`serve` 통합 테스트(미생성) | 실행 중 계정 파일 변경이 서버에 반영되지 않고 재시작 뒤 반영(1.7절). [OPS-003](../specs/operations.md#7-수용-테스트), [OPS-029](../specs/operations.md#7-수용-테스트) | 미실행 | 테스트 파일 없음 |
| Producer·`tests/distribution.rs` `repository_example_loads_without_secrets`, `example_cli_account_relay_reload_and_graceful_stop`(Linux), `tests/windows_console.rs`의 같은 실행파일 시험(Windows), [수동 운영 안내](../how-to/operate-relay.md) 두 OS 실행 기록 | 배포 예시에 실제 비밀값·기본 비밀번호가 없다. 실제 터미널의 숨김 입력으로 `admin add`를 실행한 뒤 예시 복사본으로 시작·중계·재로드·정상 종료한다. 자동 시험은 debug 실행파일의 부분 증거다. release 묶음과 안내 전체는 두 OS 수동 기록으로 판정한다. [OPS-031](../specs/operations.md#7-수용-테스트) | 통과 | 커밋 `1c7c7a9`의 두 OS 게이트와 두 OS 수동 실행(2026-11-05). [실행 기록](#계약-테스트-실행-기록) |

### 계약 테스트 실행 기록

| 대상 | 환경 | 명령 | 결과 |
| --- | --- | --- | --- |
| 커밋 `4e594f4`에 작업 2.2를 더한 작업 트리(루트 `src/`·`tests/`, `crates/*`의 계정·리스너 모듈, `Cargo.toml`, `xtask/src/layers.rs`, 문서) | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행) | 통과. `src/cli.rs` 단위 5건, `tests/cli.rs` 2건, `tests/serve.rs` 7건, 계정 파일 계약 테스트 7건, domain 단위 15건(계정 5건 포함), `config_contract` 21건 모두 통과. 시나리오 보고: 통과 1(OPS-001), 부분 4(OPS-002, OPS-038, OPS-039, OPS-040), 미연결 143, 오류 0, 판정 통과 |
| 같음 | 같음 | `tests/serve.rs` 테스트 실행파일을 60번 반복 | 통과(60번 모두 통과). 테스트가 동시에 같은 빈 포트를 고르던 문제를 포트 잠금으로 고친 뒤 측정했다. |
| 같음 | 같음 | `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings` | 통과. Windows 분기(`cfg(windows)`의 종료 신호 처리기)는 타입 검사만 했고 실행하지 않았다. |
| 커밋 `f25983a`에 작업 2.3을 더한 작업 트리 | 같음 | `cargo xtask scenarios` | 통과. `tests/serve.rs` 10건(시작 실패 때 stdout의 `startup_failed` 줄 검사로 고친 기존 7건 포함), `tests/cli.rs` 2건, `src/cli.rs` 단위 5건, 계정 파일 계약 테스트 7건 모두 통과. 시나리오 보고는 [운영 정보 계약 실행 기록](ops-info-v1.md#계약-테스트-실행-기록)에 있다. |
| 같음 | 같음(교차 대상) | `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings` | 실패. 작업 2.3이 더한 aws-lc-sys의 빌드 스크립트가 Windows용 NASM·C 컴파일러 없이 끝나지 않는다. Windows 분기는 타입 검사도 하지 못했다([개발 환경 how-to](../how-to/set-up-and-verify.md)). |
| 같음 | Windows 11 | `cargo xtask scenarios` | 미실행. Visual Studio Build Tools 설치가 UAC 미승인으로 멈춰(종료 코드 1602) MSVC 링커가 없다([개발 환경 how-to](../how-to/set-up-and-verify.md)). 사용자가 Windows 검증을 뒤로 미뤘다(2026-10-20). |
| 커밋 `e1cfc4f`에 Windows 경고 수정(`crates/adapters/src/net.rs`·`tests/serve.rs`의 `cfg`)을 더한 작업 트리 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 17.14.37806.7, `AWS_LC_SYS_PREBUILT_NASM=1` | `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행) | 통과. `src/cli.rs` 단위 5건, `tests/cli.rs` 2건, `tests/serve.rs` 12건, 계정 파일 계약 테스트 6건 모두 통과. Unix 신호를 쓰는 `tests/serve.rs` 테스트와 Ubuntu 권한 경고 테스트는 `cfg`로 빠져 실행하지 않았다. 시나리오 보고는 [운영 정보 계약 실행 기록](ops-info-v1.md#계약-테스트-실행-기록)에 있다. |
| 커밋 `ed68104`에 작업 4.1을 더한 작업 트리 | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행) | 통과. `tests/admin.rs` 21건 모두 통과. 나머지 테스트: `src/cli.rs` 단위(`vnc-relay`) 6건(`accepts_both_config_spellings`·`help_and_version_stand_alone`·`rejects_missing_repeated_or_empty_config`·`rejects_unknown_commands_and_arguments`·`parses_admin_subcommands_with_their_own_options`·`leaves_out_key_when_empty`), `tests/cli.rs` 2건, `tests/serve.rs` 18건, adapters 단위 7건, `accounts_contract` 11건, `config_contract` 21건, `ops_info_contract` 14건, app 단위 9건, domain 단위 27건, xtask 12건 모두 통과. 시나리오 보고: 통과 4, 부분 14, 실패 0, 미실행 0, 미연결 130, 오류 0, 판정 통과. OPS-030은 통과, OPS-029는 부분 |
| 같음 | 같음 | `tests/admin.rs` 테스트 실행파일을 20번 반복 | 통과(20번 모두 통과, 매번 21건 통과) |
| 같은 작업 트리 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 17.14.37806.7, `AWS_LC_SYS_PREBUILT_NASM=1` | `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행) | 통과. `tests/admin.rs` 14건 통과(터미널 테스트 제외). 나머지 테스트: `src/cli.rs` 단위(`vnc-relay`) 6건, `tests/cli.rs` 2건, `tests/serve.rs` 12건, adapters 단위 6건, `accounts_contract` 10건, `config_contract` 21건, `ops_info_contract` 13건, app 단위 9건, domain 단위 27건, xtask 12건 모두 통과. 시나리오 보고: 통과 3, 부분 14, 실패 0, 미실행 1, 미연결 130, 오류 0, 판정 통과. 미실행은 OPS-007(Unix 전용). OPS-030은 부분(터미널 테스트는 Linux 전용) |
| Windows 11 Pro 10.0.26200 x64 작업 트리(커밋 `ed68104` + 작업 4.1) | Windows 11 Pro 10.0.26200 x64, 콘솔 코드 페이지 949 | 임시 Python 하네스가 새 콘솔에서 `vnc-relay`를 실행하고 `WriteConsoleInputW`로 키를 입력(하네스는 삭제함) | Ctrl+C 20/20: 모두 `error[input_canceled]`, 종료 코드 1, 계정 파일 바이트 동일, 534~592 ms에 종료. 대상은 `add`의 `비밀번호: `·`비밀번호 확인: `, `passwd`의 `새 비밀번호: `, `remove`의 `[y/N]`이며 각 5회. 숨김 입력 중 Ctrl+C 뒤 입력 모드가 되돌아오지 않음(cmd `0x1E7`→`0x007`, PowerShell `0x1E4`→`0x000`). `[y/N]`에서 Ctrl+C 뒤에는 모드가 그대로다. 한글 비밀번호는 UTF-8 바이트로 저장됨. 해시는 UTF-8 바이트로 argon2-cffi 검증이 통과했고, cp949·UTF-16LE 바이트로는 검증이 실패했다(음성 대조). Ctrl+Z 뒤 Enter는 입력 끝이 아니며 `password_invalid`, 계정 파일 변경 없음. 한글 비밀번호와 Ctrl+Z 시험은 숨김 읽기가 도우미 스레드로 옮겨지기 전에 관찰했다. 읽기 함수(`rpassword::read_password`)는 같다 |
| 커밋 `1aed781`에 admin 입력·해시 수정(상한 초과 해시 거부, 입력 실패 분류, 계약 테스트)을 더한 작업 트리 | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `cargo build`, `vnc-relay --version`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask layers`, `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행), `cargo deny check licenses` | 통과. 모든 명령 종료 코드 0. `tests/admin.rs` 21건, `accounts_contract` 12건(`admin_refuses_an_above_maximum_parameter_even_with_a_below_minimum_one` 포함), domain 단위 27건(`checks_parameters_against_approved_ranges` 포함), app 단위 9건, `config_contract` 21건, `ops_info_contract` 14건, xtask 12건 모두 통과. 시나리오 보고: 통과 4, 부분 14, 실패 0, 미실행 0, 미연결 130, 오류 0, 판정 통과. `cargo deny`: licenses ok |
| 같음 | 같음 | `tests/admin.rs` 테스트 실행파일을 20번 반복 | 통과(20번 모두 통과, 매번 21건 통과) |
| 같은 작업 트리 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`) | `cargo build`, `vnc-relay --version`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask layers`, `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행), `cargo deny check licenses` | 통과. 모든 명령 종료 코드 0. `tests/admin.rs` 14건(터미널 테스트 제외), `accounts_contract` 11건, domain 단위 27건, app 단위 9건, `config_contract` 21건, `ops_info_contract` 13건, `tests/serve.rs` 12건, `tests/cli.rs` 2건, adapters 단위 6건, xtask 12건 모두 통과. 시나리오 보고: 통과 3, 부분 14, 실패 0, 미실행 1, 미연결 130, 오류 0, 판정 통과. 미실행은 OPS-007(`closes_and_logs_every_held_connection_when_stopping`, 이번 실행에 없음). `cargo deny`: licenses ok |
| 같은 작업 트리 | Windows 11 Pro 10.0.26200 x64, `target\debug\vnc-relay.exe`(같은 작업 트리에서 빌드) | 임시 Python 하네스가 `CREATE_NEW_CONSOLE`로 `cmd.exe`를 열어 `vnc-relay`를 실행하고, `WriteConsoleInputW`로 키를 입력(하네스는 삭제함) | `admin add alice`(비밀번호 두 번 입력 후 Enter): 종료 코드 0, 계정 추가. `admin add bob`, 비밀번호 입력 중 Ctrl+Z 뒤 Enter 2회: `password_invalid`(종료 코드 1), 계정 파일 변경 없음(§6 열린 문제와 같은 관찰). `admin add carol`, Ctrl+D 뒤 Enter 2회: `input_canceled`(종료 코드 1), 변경 없음. 원인: rpassword 7.5.4 `src/lib.rs` 140–147행이 빈 입력의 Ctrl+D를 `UnexpectedEof`로 돌려주고, `Terminal::hidden`이 이를 `input_canceled`로 옮긴다(코드 읽기로 확인). `admin remove alice`, `[y/N]`에서 Ctrl+Z 뒤 Enter 3회: `input_canceled`(종료 코드 1), 변경 없음. `admin add dave`, Ctrl+C 3회: `input_canceled`(종료 코드 1), 변경 없음. Ctrl+C 뒤 콘솔 입력 모드는 이번 실행에서 측정하지 않았다(§6 열린 문제 유지) |
| 커밋 `b7b43cb`에 작업 3.1(내장 TLS, 시작 때 TLS 파일 검사, 고정 테스트 인증서)을 더한 작업 트리 | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `cargo build`, `vnc-relay --version`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask layers`, `cargo xtask scenarios`(내부에서 `cargo test --workspace --no-fail-fast` 실행), `cargo deny check licenses` | 통과. 모든 명령 종료 코드 0. `tests/serve.rs` 21건(`checks_tls_files_before_binding_a_taken_status_port` 포함), `crates/adapters/tests/tls.rs` 11건, `tests/admin.rs` 21건, `tests/cli.rs` 2건, `src/cli.rs` 단위 6건, adapters 단위 7건, `accounts_contract` 12건, `config_contract` 21건, `ops_info_contract` 14건, app 단위 9건, domain 단위 27건, xtask 단위 12건 모두 통과. 시나리오 보고: 통과 4, 부분 16, 실패 0, 미실행 0, 미연결 128, 오류 0, 판정 통과. `cargo deny`: licenses ok |
| 같은 작업 트리 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 17.14.37806.7, `AWS_LC_SYS_PREBUILT_NASM=1` | 같은 명령을 Windows에서 실행했다(`cargo build`, `vnc-relay --version`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask layers`, `cargo xtask scenarios`, `cargo deny check licenses`) | 통과. 모든 명령 종료 코드 0. `tests/serve.rs` 15건(Unix 전용 테스트 제외), `crates/adapters/tests/tls.rs` 11건, `tests/admin.rs` 14건(터미널 테스트 제외), `tests/cli.rs` 2건, `src/cli.rs` 단위 6건, adapters 단위 6건, `accounts_contract` 11건, `config_contract` 21건, `ops_info_contract` 13건, app 단위 9건, domain 단위 27건, xtask 단위 12건 모두 통과. 시나리오 보고: 통과 3, 부분 16, 실패 0, 미실행 1, 미연결 128, 오류 0, 판정 통과. 미실행은 OPS-007(`closes_and_logs_every_held_connection_when_stopping`, `cfg(unix)`). `cargo deny`: licenses ok |
| 커밋 `ec2e6e6`(코드 기준. 문서 이동 `cfddb85`와 코드가 같다). 작업 4.1 전체(콘솔 입력 모드 저장·복원 `f21a6bc` 포함) | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), Visual Studio Build Tools 2022 17.14.37806.7, `AWS_LC_SYS_PREBUILT_NASM=1` | `vr-win.ps1 -Only all`(`D:/vr-tools/vr-win.ps1`, [§3.4 명령](../how-to/set-up-and-verify.md)) | 통과. 모든 단계 종료 코드 0. `accounts_contract` 12건(`a_failed_change_returns_its_error_and_warns_when_the_lock_cannot_be_removed` 포함), `tests/admin.rs` 14건(터미널 테스트 제외) 모두 통과. 시나리오 보고: 통과 3, 부분 19, 실패 0, 미실행 1, 미연결 125, 오류 0, 판정 통과. 미실행은 OPS-007(Unix 전용). OPS-030은 부분(터미널 테스트는 Linux 전용). `vnc-relay --version`은 `vnc-relay 0.1.0`, `cargo deny`는 `licenses ok`, 레이어 검사는 `통과 (패키지 5개)`. 앞선 Windows 실행의 rustfmt 실패와 `vnc-relay-adapters → windows-sys` 레이어 위반은 이 커밋에서 고쳤다 |
| 같은 커밋 `ec2e6e6` | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target`, cargo-deny 0.20.2 | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh`(build, `--version`, fmt --check, clippy, layers, scenarios, deny) | `ALL PASSED`. 시나리오 보고: 통과 4, 부분 19, 실패 0, 미실행 0, 미연결 125, 오류 0. OPS-030 통과, OPS-007 부분. 테스트 건수는 기록하지 않았다 |
| Windows 콘솔 입력 모드 측정: 작업 트리 `f21a6bc`(저장·복원 구현)와 `b6ea846`(수정 전, 임시 worktree, 측정 뒤 삭제). 측정 커밋 `ec2e6e6`의 콘솔 코드는 `f21a6bc`와 같다 | Windows 11 Pro 10.0.26200 x64, 콘솔 코드 페이지 949, 빌드 `target\debug\vnc-relay.exe` | 임시 Python ctypes 하네스(삭제함)가 `CREATE_NEW_CONSOLE`로 `cmd.exe`·PowerShell을 열고, `AttachConsole`·`WriteConsoleInputW`로 키를 넣었다. Ctrl+C는 `GenerateConsoleCtrlEvent`로 보냈고, Ctrl+D는 `\x04`와 Enter로 보냈다. 실행 전후 `GetConsoleMode(CONIN$)` 값과 계정 파일 SHA-256을 쟀다 | 수정 빌드(`f21a6bc`): 19회 모두 실행 뒤 입력 모드가 실행 전과 같았다. 입력 모드 시작 값은 cmd `0x1E7`, PowerShell `0x1E4`였다. 취소 18회(cmd `add` Ctrl+C 5회, `add` 빈 입력 Ctrl+D 뒤 Enter 2회, `passwd` Ctrl+C 3회, `remove` `[y/N]` Ctrl+C 3회(대기 중 `0x1F7`), PowerShell `add` Ctrl+C 5회) 모두 `input_canceled`, 종료 코드 1, 계정 파일 SHA-256 불변, 0.56~0.62 s. 정상 입력 1회(cmd `add alice`)는 종료 코드 0이고 입력 대기 중 `0x001`, 끝난 뒤 `0x1E7`이었다. 수정 전 빌드(`b6ea846`): 같은 취소 실행에서 cmd `add` `0x1E7`→`0x007`(이후 실행도 `0x007`), `passwd` `0x007` 유지, `remove` `[y/N]` 실행 전 `0x007`→취소 뒤 `0x1E7`, PowerShell `add` `0x1E4`→`0x000`(이후 실행도 `0x000`). 모든 취소는 `input_canceled`, 종료 코드 1, 계정 파일 불변. 하네스의 첫 시도는 Ctrl+C를 키 이벤트로 보내 상속된 무시 플래그에 막혔다. 시험 방법의 오류라 증거로 쓰지 않는다 |
| 커밋 `e701997`(작업 3.1 검증. `89a1001`·`e701997`은 이 계약의 시험 파일을 바꾸지 않았다). 작업 트리 깨끗함 | Ubuntu 24.04.4(WSL2), Rust 1.99.1 | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | 통과(`ALL PASSED`). `tests/serve.rs`·`tests/admin.rs`·`tests/cli.rs`·`src/cli.rs`·계정 파일 계약 테스트를 포함한 전체 `cargo test --workspace`가 통과. 시나리오 보고는 [운영 정보 계약 실행 기록](ops-info-v1.md#계약-테스트-실행-기록)에 있다. |
| 같은 커밋 `e701997`, 작업 트리 깨끗함 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`), 이전 Windows 기록과 같은 환경 | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | 통과(`ALL PASSED (all)`). 전체 `cargo test --workspace`가 통과. 미실행 1건은 OPS-007(Unix 전용). 시나리오 보고는 [운영 정보 계약 실행 기록](ops-info-v1.md#계약-테스트-실행-기록)에 있다. |
| 커밋 `e9221c6`의 `target\debug\vnc-relay.exe` 복사본(콘솔 창 닫기 처리 없음) | Windows 11 Pro 10.0.26200 x64 | 임시 Python ctypes 하네스(관찰 뒤 삭제)가 `CreatePseudoConsole`로 만든 의사 콘솔에서 `serve`를 실행했다. `/readyz` 200을 확인한 뒤 `ClosePseudoConsole`로 콘솔을 닫았다. 대조로 같은 하네스에서 콘솔 입력에 Ctrl+C(`0x03`)를 썼다 | 콘솔 닫기: 10 ms 안에 종료 코드 `0xC000013A`로 끝났고, 로그에는 `server_starting`·`server_ready`만 있었다(`shutdown_started`·`server_stopped` 없음). Ctrl+C: `shutdown_started`→`server_stopped`, 종료 코드 0. `conhost.exe`로 직접 띄운 창은 기본 터미널로 넘겨져 창을 찾지 못했으므로 의사 콘솔로 관찰했다. 이 관찰을 근거로 사용자 결정 2026-11-04(Q102)를 받았다 |
| 커밋 `de693ad`(작업 5.3 정상 종료·재시작, 작업 4.3 ConPTY 시험). 작업 트리 깨끗함 | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | `ALL PASSED`. 시나리오 보고 2026-11-04T13:31:32Z(152건): 통과 107, 부분 35, 실패 0, 미실행 1(OPS-044), 미연결 9, 오류 0. `tests/unix_shutdown.rs` 6건 통과(OPS-007~010, 042, 043), `tests/management_session.rs`의 `cli_prepared_admins_have_hidden_salted_credentials_and_equal_relay_rights`(OPS-029)·`restart_ends_the_admin_session_and_login_works_again`(MGT-027) 통과. 앞선 커밋 `976550f`·`2ef5a05` 게이트 수정 반영 |
| 같은 커밋 `de693ad` | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`) | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | `ALL PASSED (all)`. 시나리오 보고 2026-11-04T13:39:13Z(152건): 통과 103, 부분 40, 실패 0, 미실행 0, 미연결 9, 오류 0. `tests/windows_console.rs` 12건 통과: Ctrl+C·Ctrl+Break 정상 종료, 창 닫기 강제 정리(OPS-044), 공통 정상 종료 시나리오 6건(OPS-007~010, 042, 043), ConPTY 숨김 입력(OPS-029), ConPTY 재시작 세션 무효(MGT-027) |
| 커밋 `1c7c7a9`(작업 5.6: 로그 다시 열기, 설정 예시, 운영 안내, 배포 시험, `tests/relay_log_scenarios.rs` 대상 포트 수정) | Ubuntu 24.04.4(WSL2), Rust 1.99.1, `CARGO_TARGET_DIR=~/vnc-relay-target` | `wsl -d Ubuntu -- bash /mnt/d/vr-tools/vr-full.sh` | `ALL PASSED`. 시나리오 보고 2026-11-04T15:43:30Z(152건): 통과 109, 부분 36, 실패 0, 미실행 1(OPS-044), 미연결 6, 오류 0. OPS-019 통과(`tests/distribution.rs` `example_cli_account_relay_reload_and_graceful_stop`), OPS-031 부분(자동 시험만), OPS-045 통과. 앞선 커밋 `a4716e9`의 WSL 게이트도 `ALL PASSED`(같은 수치)였다. `+ 미커밋 변경`은 CRLF 차이와, 게이트 출력을 저장소 루트에 잘못 저장한 추적되지 않는 로그 파일 때문이다. 이 파일은 실행 뒤 저장소 밖으로 옮겼다 |
| 같은 커밋 `1c7c7a9`, 작업 트리 깨끗함 | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`) | `powershell -NoProfile -ExecutionPolicy Bypass -File D:/vr-tools/vr-win.ps1 -Only all` | `ALL PASSED (all)`. 시나리오 보고 2026-11-04T15:35:46Z(152건): 통과 105, 부분 41, 실패 0, 미실행 0, 미연결 6, 오류 0. OPS-019 통과(`tests/windows_console.rs` `example_cli_account_relay_reload_and_graceful_stop`), OPS-031 부분, OPS-044·OPS-045 통과. 앞선 커밋 `a4716e9`의 Windows 게이트는 `tests/relay_log_scenarios.rs` `operations_log_and_status_endpoints_never_expose_secrets_or_raw_identifiers`가 1시간 넘게 멈춰 중단했다. 이 PC의 TCP 동적 포트 범위는 1024부터다(`netsh int ipv4 show dynamicport tcp`). 대상 서버가 포트 2350을 받았고, 시험 설정 `allow_targets = ["127.0.0.1:5000-65535"]` 때문에 서버가 `target_not_allowed`로 거절했다. 시험의 대상 수락은 시간 제한 없이 기다렸다. `1c7c7a9`에서 범위를 `1024-65535`로 넓히고 수락에 20초 제한을 두었다(두 OS에서 3회씩 통과) |
| 커밋 `1c7c7a9`의 release 묶음(`cargo build --release --locked` 1분 10초, `vnc-relay 0.1.0`, SHA-256 `120f319ae67bc1907a3f1e7fcfdb7d5f425e84778a554e8747c8dfbf3e667278`). 묶음: 실행파일, `relay.toml`(예시 그대로), `docs/how-to/operate-relay.md`, `THIRD-PARTY-NOTICES.txt`. `LICENSE` 없음(Q97 대기) | Windows 11 Pro 10.0.26200 x64, Rust 1.99.1(`x86_64-pc-windows-msvc`) | [운영 안내](../how-to/operate-relay.md) 3.1~3.8절 명령을 에이전트가 차례로 실행했다(2026-11-05). 숨김 입력은 도구의 의사 터미널로 입력했다. Ctrl+C는 도우미 프로세스가 서버 콘솔에 붙어 `GenerateConsoleCtrlEvent(CTRL_C_EVENT)`로 보냈다 | 통과. `cargo xtask notices --check`: `notices --check: 통과 (THIRD-PARTY-NOTICES.txt)`. `admin add opsadmin`: 프롬프트 `비밀번호:`·`비밀번호 확인:`, 입력 글자 표시 없음, 종료 코드 0. 계정 파일에 입력 문자열 없음, Argon2id `m=19456,t=2,p=1`. `admin list`: `opsadmin`. `/livez` 200 `{"status":"alive"}`, `/readyz` 200 `{"status":"ready"}`. 브라우저 로그인 뒤 운영 개요의 서버 상태 `Ready`. `check-relay.py`: `Mode II: both directions passed`. `relay.tcp_viewer.allow` 저장 뒤 0.6초 안에 `reload_change_detected`(`os_notify`)→`reload_applied`(`relay.tcp_viewer.allow`), 중계 재확인 통과. `relay.log`를 `relay.previous.log`로 바꾼 뒤 원래 경로에 새 파일이 생겼다. 회전 뒤 중계 사건 5개는 새 파일에만, 앞선 사건은 이전 파일에만 있었다. Ctrl+C: `shutdown_started`(`trigger=requested`, `grace_secs=30`, `active_sessions=0`)→`server_stopped`, 종료 코드 0. 첫 시도는 도구의 의사 터미널 안에서 서버를 띄워 래퍼가 512를 보고했다. 서버 자체의 값이 아닌 래퍼 값으로 본다 `[INFERENCE]`. 그래서 독립 콘솔에서 다시 시작해 종료 코드 0을 직접 읽었다 |
| 커밋 `1c7c7a9`의 release 묶음(`cargo build --release --locked` 58.7초, `vnc-relay 0.1.0`, SHA-256 `dc7da5485cbf722381aaa8932051c34bb0cc58b475999e313107904c21749c40`). 묶음 구성은 위와 같다. 묶음 디렉터리 `~/bundle-wsl`은 `0700` | Ubuntu 24.04.4 LTS x86_64(WSL2), Rust 1.99.1 | 위와 같은 3.1~3.8절 실행(2026-11-05). 숨김 입력은 도구의 의사 터미널로 입력했다. 브라우저 로그인은 Windows 브라우저가 WSL localhost 전달로 접속했다 | 통과. 고지 검사 통과. 첫 `admin add`는 도구가 확인 입력에 빈 줄을 보내 `error[password_mismatch]`(종료 코드 1)였고 계정 파일은 생기지 않았다. 다시 입력해 종료 코드 0. 계정 파일 `0600`, 입력 문자열 없음, Argon2id `m=19456,t=2,p=1`. `admin list`: `opsadmin`. 상태 조회 200 두 건, 브라우저 로그인 뒤 서버 상태 `Ready`, 중계 통과. 재로드는 0.6초 안에 `reload_applied`, 중계 재확인 통과. 로그 회전 결과는 Windows와 같았다. 새 로그 파일 권한은 `0644`(umask `022`). 종료의 첫 시도는 하네스 실수였다. `cd … && ./vnc-relay serve … &`로 띄워 `$!`가 서버가 아닌 subshell의 PID였다. SIGINT는 subshell이 무시했고, SIGTERM은 subshell만 끝냈다(`143`). 이때 서버 종료 기록은 없고, 그 뒤 남은 서버 프로세스도 없었다(서버가 끝난 원인은 확인하지 못함). 같은 release 실행파일을 올바른 PID로 다시 띄웠다. `/proc/<pid>/status`의 `SigCgt`가 SIGINT·SIGTERM을 잡았다(debug 실행파일도 같음). SIGINT(Ctrl+C가 앞단 작업에 보내는 신호) 뒤 `shutdown_started`→`server_stopped`, 종료 코드 0 |

## 6. 초안을 확정하기 전에 해결할 사항

- 이 계약의 이름·코드 선택(하위 명령 옵션 `--yes`·`--help`·`--version`, 오류·경고 코드, 계정 파일 TOML 형식과 필드 이름, 잠금 파일 이름)은 2026-10-16에 사용자가 채택했다. 하위 명령 이름 `serve`·`admin add|passwd|remove|list`와 `--config`는 공유 인터페이스 이름을 따른다.
- 계정 이름·비밀번호 규칙(1.2절), 솔트·해시 길이(1.4절), 성공 문장과 `notice`, 마지막 계정 삭제 허용도 채택 범위다.
- 종료 코드는 확정했다(2026-10-16). 준비된 계정이 없을 때(`accounts_none`)와 계정 파일이 없을 때(`accounts_file_missing`)는 5다. 인증서·개인키 파일 오류(`tls_file_invalid`)는 설정 오류와 같은 3이다. 리스너 바인드 실패는 4다.
- Windows에서 확인이 필요한 사항: 다른 프로그램이 주소 공유 옵션으로 같은 주소를 쓸 때의 바인드. 같은 설정의 두 번째 인스턴스는 Windows에서도 바인드에 실패했다([실행 기록](#계약-테스트-실행-기록)). 계정 파일의 Windows 접근 권한은 운영자가 정하고 CLI는 바꾸지 않는다(1.6절, 사용자 결정 2026-10-20). 콘솔 창 닫기와 로그오프는 관찰과 사용자 결정 2026-11-04(Q102)로 정했다(1.8절).
- 결정 완료(Windows, 사용자 결정 2026-10-21): 숨김 비밀번호 입력 중 Ctrl+C는 입력을 바로 끝내고 종료 코드 1을 돌려준다(20회 관찰). 그러나 취소 뒤 콘솔 입력 모드가 되돌아오지 않았다. cmd는 `0x1E7`에서 `0x007`로, PowerShell은 `0x1E4`에서 `0x000`으로 남았다. 사용자는 읽기 전에 콘솔 입력 모드를 Win32로 저장하고 끝난 뒤 되돌리는 방식으로 정했다([1.5절](#15-입력과-출력-규칙)). 되돌리기는 `f21a6bc`에 구현했고, Windows 콘솔에서 취소 18회와 정상 입력 1회 모두 입력 모드가 시작 값으로 돌아온 것을 관찰했다(2026-10-21, [실행 기록](#계약-테스트-실행-기록)). 관련 설계는 [관리자 계정 준비 CLI 모듈 설계 7절](../architectures/admin-accounts.md#7-설계-선택)이다.
- 결정 완료(Windows, 사용자 결정 2026-10-21): 숨김 비밀번호 입력에서 Ctrl+Z 뒤 Enter는 입력 끝이 아니다. 빈 줄로 받아 `password_invalid`(종료 코드 1)가 된다(실행 기록). 빈 입력의 Ctrl+D는 `input_canceled`가 된다(실행 기록). 사용자는 코드를 바꾸지 않고 1.5절에 Windows의 입력 끝 키가 빈 입력의 Ctrl+D라고 적기로 정했다.
- 계정 파일 권한 경고(Ubuntu)는 `serve`와 `admin` 명령 모두 쓴다. `admin` 명령도 잠금 파일을 쓴다(1.6절). 둘 다 2026-10-20에 사용자가 필요성을 확인했다.
- 비밀번호 입력의 정규화(1.2절)는 입력 바이트 그대로 쓰기로 채택했다. Windows 11에서 한글 비밀번호 한 개를 UTF-8 바이트로 저장한 것을 관찰했다. 다른 입력기의 바이트는 관찰하지 않았다.
- 로그 파일을 교체할 때 서버의 재오픈 여부는 관찰로 확인 필요다. 운영 로그 필드는 [운영 정보 계약](ops-info-v1.md)이 정한다.
- 구현 판단(사용자 결정 없음): Windows에서 숨김 입력 전에 콘솔 입력 모드를 읽지 못하거나 입력 뒤에 되돌리지 못하면 `runtime_unavailable`(종료 코드 1)로 끝난다. 계정 파일은 바뀌지 않는다(2.2절). 이 경우에 다른 코드를 쓸지는 사용자가 정한다.
