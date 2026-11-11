# 도움말 작성 규칙

이 파일은 `docs/help/`에 적용한다. [문서 작성 규칙](../AGENTS.md)에 도움말 규칙을 더한다.

도움말은 vnc-relay를 설치·설정·운영하는 운영자가 읽는 안내서다. 한국어와 영어 사이트로 GitHub Pages에 게시한다(사용자 결정 2026-11-11(Q220·Q222)). 사이트의 화면과 동작은 [도움말 사이트 화면 스펙](../specs/help-site.md)이 정한다. 빌드·검사·게시 절차는 [도움말을 빌드하고 게시하는 방법](../how-to/publish-help.md)을 따른다.

## 기준 문서와의 관계

- 도움말은 기준 문서가 아니다. 동작·설정 키·형식·오류 코드의 기준은 PRD·스펙·계약·how-to에 있다. 도움말은 이 문서들을 쉽게 풀어 쓰고 링크한다.
- 도움말과 기준 문서가 다르면 기준 문서를 따르고 도움말을 고친다. 기준 문서의 동작·키·절차를 바꾸는 변경은 같은 변경에서 영향받는 도움말 페이지도 고친다.
- 키 전체 목록, 기본값 표, 오류 코드 표, 사건 필드 표처럼 기준 문서에 있는 표를 복제하지 않는다. 자주 쓰는 일부만 예로 들고 기준 문서로 링크한다.
- 도움말은 `main` 기준이다(사용자 결정 2026-11-11(Q221)). 첫 페이지(`index.md`)의 기준 안내에 최신 Release와 다른 동작을 적는다. Release를 내거나 `main`의 동작이 최신 Release와 달라지면 이 목록을 같은 변경에서 고친다.

## 파일 구조

| 위치 | 내용 |
| --- | --- |
| `ko/book.toml`, `en/book.toml` | 언어별 mdBook 설정 |
| `ko/src/`, `en/src/` | 언어별 페이지(Markdown)와 목차 `SUMMARY.md` |
| `ko/src/images/`, `en/src/images/` | 언어별 대시보드 화면 캡처(PNG) |

테마·사이트 루트 파일·빌드 도구는 코드다. 위치는 [코드 지도](../architectures/code-map.md)를 따른다.

## 두 언어 맞추기

- 두 책은 같은 파일 이름과 같은 `SUMMARY.md` 순서를 쓴다. 언어 전환은 다른 언어의 같은 파일 이름 페이지로 이동한다.
- 한쪽 언어를 고치면 같은 변경에서 다른 언어도 고친다. 한쪽에만 있는 절·절차·예시를 두지 않는다.
- 한국어 페이지는 [용어집](../../GLOSSARY.md)의 이름을 쓴다. 영어 페이지는 아래 영어 용어를 쓴다.
- 대시보드 화면의 문구는 [대시보드 화면 스펙](../specs/dashboard.md)의 문구(CPY) 원문을 그대로 인용한다. 한국어 페이지는 ko 원문을, 영어 페이지는 en 원문을 쓴다.

| 한국어(용어집) | 영어 페이지 |
| --- | --- |
| VNC Relay, `vnc-relay` | VNC Relay, `vnc-relay`(실행파일) |
| 중계 ID | relay ID |
| 진입점 | entry point |
| 리스너, 관리 리스너, 상태 리스너 | listener, management listener, status listener |
| 대기 연결, 활성 세션 | waiting connection, active session |
| 최근 종료 목록, 최근 로그, 운영 로그 | recently closed sessions, recent logs, operations log |
| 출발지 IP 허용 목록 | source IP allow list |
| 신뢰 프록시, 전달 주소 | trusted proxy, forwarded address |
| 접속 ticket, 발급 쪽, 세션 만료 시각 | access ticket, issuer, session expiry time |
| 부분 재로드, 재로드 대상, 편집 대상 | reload, reloadable key, editable key |
| 설정 보기, 동적 설정 편집 | View settings, Edit dynamic settings |
| 관리자 계정, 관리 세션 | administrator account, management session |
| 관리자 종료, 종료 원인 | termination by administrator, close reason |
| 정상 종료, 종료 유예, 시작 실패 | graceful shutdown, shutdown grace period, startup failure |

## 쓰기 규칙

- [쉬운 기술 글쓰기](../../AGENTS.md#쉬운-기술-글쓰기)를 따른다. 한국어 페이지는 "-다"로 끝낸다. 영어 페이지의 절차는 명령문으로 쓴다.
- 페이지는 제목 하나(`#`)로 시작하고, 첫 문단에 이 페이지로 할 수 있는 일을 쓴다.
- 절차는 번호 목록으로 쓰고 한 단계에 한 행동을 쓴다. 조건과 실행 위치(OS·디렉터리)를 먼저 밝힌다.
- 명령은 코드 블록에 넣고 언어를 표시한다(`shell`, `powershell`, `toml`). Windows와 Ubuntu의 명령이 다르면 둘 다 쓴다.
- 강조 상자는 인용 블록과 굵은 머리말로 쓴다. 한국어는 `> **참고:**`·`> **주의:**`, 영어는 `> **Note:**`·`> **Warning:**`다. mdBook의 `> [!NOTE]` 문법은 쓰지 않는다. 이 문법은 언어와 관계없이 영어 제목을 붙인다.
- 예시 값: 호스트는 `relay.example.test`처럼 `.test`·`.example` 도메인만 쓴다. IP는 문서용 주소(`192.0.2.0/24`, `198.51.100.0/24`, `203.0.113.0/24`, `2001:db8::/32`)와 루프백만 쓴다. 실제 조직·서비스·호스트 이름을 쓰지 않는다.
- 비밀번호, 개인키, 서명 키, 접속 ticket 원문 같은 비밀 값의 예시를 쓰지 않는다. 필요한 곳에는 `<password>` 같은 자리표시를 쓴다.
- 원시 HTML, 스크립트, 외부 이미지·글꼴 주소를 넣지 않는다. 그림은 같은 책의 `images/` 파일만 쓴다.

## 링크

- 같은 책의 페이지는 상대 링크로 연결한다(예: `configure.md#접속-허용`).
- 기준 문서와 저장소 파일은 `https://github.com/kayden-kim/vnc-relay/blob/main/` 아래 절대 주소로 연결한다(예: `https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md`).
- 외부 사이트는 공식 배포처와 공식 문서(GitHub Releases, UltraVNC, noVNC)만 연결한다. 고지 페이지(`notices.md`)는 사이트에 함께 싣는 제3자 구성요소의 공식 출처와 라이선스 원문도 연결한다.
- `cargo xtask help-site`는 이 규칙을 허용 목록으로 확인한다. 새 외부 링크가 필요하면 허용 목록과 [xtask 모듈 설계](../architectures/xtask.md)를 함께 고친다.

## 화면 캡처

- 대시보드 페이지는 실제 대시보드 화면 캡처를 쓴다(사용자 결정 2026-11-11(Q224)). 캡처는 [도움말 게시 how-to](../how-to/publish-help.md)의 캡처 절차로 다시 만든다. 손으로 고치지 않는다.
- 캡처에는 예시·시험 주소와 시험 계정 이름만 보여야 한다.
- 언어마다 그 언어의 화면을 캡처한다. 파일 이름은 두 언어가 같다.
- 모든 그림에 그림 내용을 설명하는 대체 텍스트를 쓴다.

## 검증

도움말을 바꾸면 `cargo xtask help-site`로 빌드와 검사를 실행한다. 성공 조건은 [도움말을 빌드하고 게시하는 방법](../how-to/publish-help.md)을 따른다.
