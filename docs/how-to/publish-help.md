# 도움말을 빌드하고 게시하는 방법

운영자 도움말 사이트(한국어·영어 mdBook 책)를 로컬에서 빌드·검사하고 미리 본 뒤, `main`에 push해 GitHub Pages에 게시한다. 성공하면 `https://kayden-kim.github.io/vnc-relay/`가 브라우저 언어에 맞는 책의 첫 페이지로 이동하고, 두 책의 모든 장이 열린다.

| 항목 | 값 |
| --- | --- |
| 대상 독자 | 도움말 작성자와 저장소 관리자 |
| 검증 상태 | Verified — mdBook 설치·미리보기·화면 대조·공개 사이트의 수동 확인은 4절에 기록한다. 빌드와 브라우저 시험의 현재 결과는 3.1·3.3절로 확인한다. 6절 롤백은 미실행이다 |
| 검증 환경·버전 | Windows 11 Pro 10.0.26200 x64(PowerShell 7.6.9), WSL2 Ubuntu 24.04.4 LTS x64, mdBook 0.5.4, Rust 1.99.1, GitHub Actions `ubuntu-24.04` |
| 최종 검증일 | 2026-11-18 |
| 관련 기준 문서 | [도움말 사이트 화면 스펙](../specs/help-site.md), [도움말 작성 규칙](../help/AGENTS.md), [xtask 모듈 설계](../architectures/xtask.md), [디자인 기준의 도움말 사이트 확장](../design/DESIGN.md#도움말-사이트-확장), [프런트엔드 컨벤션](../conventions/frontend.md) |

## 1. 대상과 영향

- 대상: 책 원본 `docs/help/ko/`·`docs/help/en/`, 테마와 사이트 루트 `web/help/`, `cargo xtask help-site`가 만드는 `target/help-site/`, 워크플로 `.github/workflows/help.yml`, 저장소 `kayden-kim/vnc-relay`의 GitHub Pages 사이트다.
- 예상 사용자 영향: 사이트는 공개 사이트다. `main`에 push하면 워크플로가 바로 다시 게시한다. 게시 중에도 이전 사이트가 계속 열린다.
- 작업 허용 시간: 제한 없다.

### 1.1 사이트 구성

| 주소 | 원본 | 화면 |
| --- | --- | --- |
| `/vnc-relay/` | `web/help/site/index.html` | HLP-002 언어 선택 |
| `/vnc-relay/ko/`, `/vnc-relay/en/`과 그 아래 장 | `docs/help/<언어>/`의 책과 테마 `web/help/theme/` | HLP-001 도움말 페이지 |
| 사이트 안의 없는 경로 | `web/help/site/404.html` | HLP-003 없는 페이지 |
| `/vnc-relay/assets/` | `web/help/site/assets/`와 빌드가 쓰는 `tokens.css`·`fonts/`·`favicon.svg` | 공통 스타일·글꼴·언어 선택 스크립트 |

빌드 결과는 저장소에 넣지 않는다. 토큰 CSS는 DESIGN.md에서, 글꼴은 `web/dashboard/fonts/`에서 빌드할 때마다 만든다.

## 2. 전제 조건

- [ ] [개발 환경 준비](set-up-and-verify.md)에 따라 Rust를 준비했다.
- [ ] 2.1절에 따라 mdBook 0.5.4를 설치했다. 다른 버전이면 `cargo xtask help-site`가 빌드 전에 멈춘다. 테마가 0.5.4 템플릿의 구조에 맞춰져 있기 때문이다.
- [ ] 브라우저 시험(3.3절)을 실행하려면 Chrome 또는 Edge가 있다. 찾는 순서는 [대시보드 검증 how-to](verify-dashboard-frontend.md)와 같고, 환경 변수 `DASHBOARD_BROWSER`로 정할 수 있다.
- [ ] 게시(3.5절)하려면 `kayden-kim/vnc-relay`에 push 권한이 있고, 처음 게시할 때는 저장소 관리자 권한이 있다.

### 2.1 mdBook 0.5.4 설치

mdBook은 공식 GitHub Release의 실행파일을 쓴다. 아래 SHA-256은 Release 자산에 GitHub가 기록한 값이다.

| OS | 자산 | SHA-256 |
| --- | --- | --- |
| Windows x64 | `mdbook-v0.5.4-x86_64-pc-windows-msvc.zip` | `8a6b2421aa522de06d746871d2b8fff9c8d71773467e310a4516d2566e7f2de4` |
| Linux x64 | `mdbook-v0.5.4-x86_64-unknown-linux-gnu.tar.gz` | `3f28de05dafca9d0f2eab99c662116b0e37b89b1d96a08f8f430b9eeae958cd7` |

Windows(PowerShell 7):

1. 자산을 내려받는다.

   ```powershell
   Invoke-WebRequest https://github.com/rust-lang/mdBook/releases/download/v0.5.4/mdbook-v0.5.4-x86_64-pc-windows-msvc.zip -OutFile mdbook-v0.5.4-x86_64-pc-windows-msvc.zip
   ```

2. 해시를 확인한다.

   ```powershell
   (Get-FileHash mdbook-v0.5.4-x86_64-pc-windows-msvc.zip -Algorithm SHA256).Hash
   ```

   기대 결과: 표의 값과 같다(대소문자 무시). 다르면 파일을 지우고 진행하지 않는다.

3. 원하는 디렉터리(예: `C:\tools\mdbook-0.5.4`)에 풀고, 환경 변수 `MDBOOK`에 `mdbook.exe`의 전체 경로를 넣는다. `PATH`에 그 디렉터리를 더해도 된다. `MDBOOK`이 있으면 `PATH`보다 먼저 쓴다.

Ubuntu(bash):

1. 자산을 내려받고 해시를 확인한다.

   ```shell
   curl -fsSLO https://github.com/rust-lang/mdBook/releases/download/v0.5.4/mdbook-v0.5.4-x86_64-unknown-linux-gnu.tar.gz
   echo "3f28de05dafca9d0f2eab99c662116b0e37b89b1d96a08f8f430b9eeae958cd7  mdbook-v0.5.4-x86_64-unknown-linux-gnu.tar.gz" | sha256sum -c -
   ```

   기대 결과: `mdbook-v0.5.4-x86_64-unknown-linux-gnu.tar.gz: OK`. 아니면 파일을 지우고 진행하지 않는다.

2. 풀고 경로를 정한다.

   ```shell
   mkdir -p "$HOME/tools/mdbook-0.5.4"
   tar -xzf mdbook-v0.5.4-x86_64-unknown-linux-gnu.tar.gz -C "$HOME/tools/mdbook-0.5.4"
   export MDBOOK="$HOME/tools/mdbook-0.5.4/mdbook"
   ```

3. 버전을 확인한다.

   ```shell
   "$MDBOOK" --version
   ```

   기대 결과: `mdbook v0.5.4`

[전체 완료 검증](set-up-and-verify.md#34-전체-완료-검증)은 도움말 시험(`tests/help_site.rs`)을 포함한다. 게이트를 실행하는 셸에도 `MDBOOK`이나 `PATH`를 정해야 한다.

## 3. 절차

- 실행 환경: Windows PowerShell 또는 Ubuntu bash다.
- 실행 위치: 저장소 루트다.
- 바꿔 넣을 값: 없다. `CARGO_TARGET_DIR`을 정했으면 아래의 `target` 대신 그 경로다.
- 최대 대기 시간: `cargo xtask help-site`는 처음 xtask를 빌드할 때 3분, 그 뒤 30초로 둔다. 브라우저 시험은 10분, GitHub Actions 한 번은 20분으로 둔다. 넘으면 로컬은 Ctrl+C로, Actions는 실행 화면의 Cancel로 중단한다.

### 3.1 빌드와 검사

1. 사이트를 만들고 검사한다.

   ```shell
   cargo xtask help-site
   ```

   기대 결과: 종료 코드 0이고 마지막 줄이 `help-site: 통과 (target/help-site)`다. 환경 변수 `CARGO_TARGET_DIR`을 정했으면 괄호 안은 그 디렉터리 아래의 `help-site`다. 이 명령은 Cargo 대상 디렉터리(기본 `target/`)의 `help-stage/`와 `help-site/`를 지우고 다시 만든다.

   검사 문제가 있으면 문제마다 `<경로>: <내용>` 한 줄을 출력하고, 마지막 줄이 `help-site: 실패 (N건)`, 종료 코드 1이다. 경로는 `target/help-site/` 기준 경로다. 목차 비교만 저장소 기준 `SUMMARY.md` 경로를 쓴다. 검사 항목은 [xtask 모듈 설계](../architectures/xtask.md)의 `help_site/checks.rs` 행이다.

   | 출력 예 | 뜻과 조치 |
   | --- | --- |
   | `docs/help/en/src/SUMMARY.md: 장 목록이 ko와 다르다 (ko: …; en: …)` | 두 책의 장 파일 이름이나 순서가 다르다. 두 `SUMMARY.md`를 맞춘다 |
   | `ko/install.html: 링크 대상이 없다: configure2.html` | 같은 사이트 안의 장이 없다. 링크를 고친다 |
   | `ko/install.html: 절이 없다: configure.html#없는-절` | 장은 있지만 그 절 제목이 없다. 링크나 제목을 고친다 |
   | `en/dashboard.html: 그림 파일이 없다: images/login.png` | 3.4절로 화면 캡처를 만든다 |
   | `ko/reference.html: 허용 목록 밖 외부 링크: https://…` | [도움말 작성 규칙의 링크](../help/AGENTS.md#링크)를 따른다 |
   | `ko/index.html: 저장소에 없는 경로: https://github.com/kayden-kim/vnc-relay/blob/main/docs/…` | `blob/main/` 링크가 가리키는 파일 이름을 고친다 |
   | `ko/dashboard.html: 빈 주소: <img>` | 링크나 그림의 주소가 비어 있다. 원본의 `[글자]()`·`![대체 텍스트]()`에 주소를 넣는다 |

### 3.2 로컬 미리보기

1. 3.1절로 사이트를 만든 뒤 미리보기 서버를 띄운다. 포트를 바꾸려면 `--port <번호>`를 붙인다. `0`이면 빈 포트를 고른다.

   ```shell
   cargo xtask help-serve
   ```

   기대 결과: `도움말: http://127.0.0.1:8090/vnc-relay/`와 `종료하려면 Ctrl+C를 누른다.`가 출력된다.

2. 출력된 주소를 브라우저로 연다. 서버는 GitHub Pages처럼 `/vnc-relay/` 아래에 사이트를 두고, 없는 경로에는 HTTP 404와 HLP-003을 준다. 서버가 도는 동안 3.1절이나 3.3절을 실행하면 사이트를 지우고 다시 만든다. 그동안에는 페이지가 열리지 않을 수 있다.
3. 확인을 마치면 Ctrl+C로 서버를 멈춘다.

### 3.3 브라우저 시험

도움말 사이트의 HELP 시나리오는 루트 통합 시험이 확인한다. 시험은 `cargo xtask help-site`로 사이트를 한 번 만들고, 시험 안의 정적 서버로 `/vnc-relay/` 아래에 제공한 뒤 헤드리스 Chromium을 조작한다.

```shell
cargo test --test help_site
```

기대 결과: 모든 시험이 통과한다. mdBook이나 Chromium이 없으면 시험은 건너뛰지 않고 실패한다.

### 3.4 화면 캡처 갱신

대시보드 화면이 바뀌면 두 책의 `src/images/`에 있는 캡처 여섯 장(`login`, `operations`, `session`, `logs`, `settings`, `settings-edit`)을 다시 만든다. 캡처 도구는 실제 VNC Relay를 쓰는 대시보드 시험의 하네스로 VNC Relay를 띄운다. VNC Relay는 루프백 주소와 시험 계정(`alice`)만 쓰고, 종료한 세션 1개·활성 세션 2개·대기 연결 1개를 만든다. 화면 언어마다 1280×800 크기로 찍는다. 운영 현황 그림만 1440×800 크기로 찍는다. 활성 세션·최근 종료 목록은 폭 1440px 이상에서 표이고 그보다 좁으면 카드이므로, 도움말이 설명하는 열이 한눈에 보이게 한다.

캡처는 Ubuntu에서만 실행한다. 설정 보기 화면이 VNC Relay 디렉터리의 절대 경로(`admin.accounts_file`)를 보여 주기 때문이다. Windows의 기본 임시 디렉터리 경로에는 사용자 이름이 들어간다. Ubuntu에서 `TMPDIR`을 정하지 않으면 그림에는 `/tmp/vnc-relay-dashboard-live-…/accounts.toml` 같은 시험 경로만 보인다.

1. Ubuntu bash에서 `TMPDIR`이 비어 있는지 확인한다.

   ```shell
   echo "${TMPDIR:-unset}"
   ```

   기대 결과: `unset`. 값이 있으면 `unset TMPDIR`을 실행한다. VNC Relay 디렉터리가 `/tmp` 아래가 아니면 캡처 도구는 그림을 쓰기 전에 실패한다.

2. 캡처를 실행한다.

   영향: 두 책의 `src/images/*.png` 열두 개를 덮어쓴다. Git으로 되돌릴 수 있다.

   ```shell
   cargo test --test dashboard_live capture_help_screenshots -- --ignored
   ```

   기대 결과: 시험 1개가 통과하고 `docs/help/ko/src/images/`와 `docs/help/en/src/images/`에 PNG 여섯 개씩이 생긴다. Windows에서는 이 시험이 컴파일되지 않으므로 `0 passed`로 끝난다.

3. 캡처 열두 장을 모두 열어 실제 조직·호스트 이름, 실제 IP, 사용자·호스트 이름이 든 로컬 경로, 비밀 값이 없는지 확인한다. 예시 값 규칙은 [도움말 작성 규칙](../help/AGENTS.md)을 따른다.
4. 3.1절을 다시 실행한다.

### 3.5 GitHub Pages 게시

1. 처음 게시할 때만 Pages의 게시 방식을 GitHub Actions로 정한다.

   영향: 저장소에 Pages 사이트가 생긴다. 6절 2단계로 끌 수 있다.

   ```shell
   gh api -X POST repos/kayden-kim/vnc-relay/pages -f build_type=workflow
   ```

   기대 결과: 응답 JSON의 `build_type`이 `workflow`이고 `html_url`이 `https://kayden-kim.github.io/vnc-relay/`다. 이미 켜져 있으면 HTTP 409가 나온다. 이때는 `gh api repos/kayden-kim/vnc-relay/pages`로 `build_type`이 `workflow`인지 확인한다.

2. 도움말을 바꾼 커밋을 `main`에 push한다. PR을 열면 워크플로 `help`가 빌드와 검사만 실행한다.

   | 작업 | 실행 조건 | 내용 |
   | --- | --- | --- |
   | `build` | PR, `main` push, 수동 실행 | mdBook 0.5.4를 내려받아 2.1절 SHA-256으로 확인한다. `rustup toolchain install` 뒤 `cargo xtask help-site`를 실행한다. `main`이면 `target/help-site/`를 Pages 산출물로 올린다 |
   | `deploy` | `main` push와 `main`의 수동 실행 | 산출물을 환경 `github-pages`에 게시한다 |

   워크플로 `help`는 ref(브랜치나 PR)마다 동시 실행 그룹 하나를 쓴다. 그래서 같은 ref의 실행은 한 번에 하나만 진행한다. 진행 중인 실행은 취소하지 않고, 기다리던 실행은 더 새 실행으로 바뀐다. `main`에는 실행이 그룹에서 기다리기 시작한 순서대로 게시된다. 거의 같은 시각에 들어온 실행의 순서는 GitHub가 보장하지 않는다([GitHub 동시 실행 문서](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency)).

3. 실행 결과를 확인한다.

   ```shell
   gh run list --workflow help --branch main --limit 1
   ```

   기대 결과: 최신 실행의 상태가 `completed`, 결론이 `success`다.

4. 게시된 사이트를 연다.

   ```shell
   curl -sS -o /dev/null -w "%{http_code}\n" https://kayden-kim.github.io/vnc-relay/ko/
   ```

   기대 결과: `200`. 브라우저로 `https://kayden-kim.github.io/vnc-relay/`를 열면 언어에 맞는 첫 페이지로 이동한다.

5. 처음 게시했으면 저장소 About의 웹사이트 주소를 정한다.

   ```shell
   gh repo edit kayden-kim/vnc-relay --homepage https://kayden-kim.github.io/vnc-relay/
   ```

   기대 결과: `gh repo view kayden-kim/vnc-relay --json homepageUrl`의 값이 같은 주소다.

## 4. 검증

[검증 기록](../AGENTS.md#검증-기록)에 따라 실제 결과와 성공 조건을 대조한다.

빌드·브라우저 시험의 현재 결과는 3.1·3.3절과 `cargo xtask scenarios`가 만드는 `scenario-report.md`로 확인한다. 아래 표에는 수동 설치·화면 확인·예시 실행의 최신 증거를 남긴다.

| 관찰 대상 | 성공 조건 | 확인 명령·방법 | 실제 결과 | 실행 증거 위치 |
| --- | --- | --- | --- | --- |
| mdBook 설치 | 2.1절 기대 결과 | 2.1절. Windows는 PowerShell 7.6.9, Linux 자산은 GitHub Actions `help` 실행, 설치본 버전은 WSL2 Ubuntu | 통과(2026-11-12). Windows 자산의 SHA-256이 표의 값과 같고 설치본이 `mdbook v0.5.4`다. Linux 자산은 Actions에서 `sha256sum -c`가 `OK`였고 WSL 설치본이 `mdbook v0.5.4`다 | 이 표 |
| 시안 대조 | [4.1절](#41-시안-대조) | 4.1절 | 통과. 고친 차이 1건, 유지한 차이 4건 | 4.1절 |
| 게시 순서 | 3.5절 2단계의 실행 순서 설명 | 커밋 `8f99434`를 push한 뒤 `main`에서 `gh workflow run help.yml --ref main`을 연달아 세 번 실행했다 | 통과(2026-11-12). 첫 실행 43142798327이 도는 동안 셋째 실행 43142807912는 `pending`이었다. 둘째 실행 43142803199는 작업을 하나도 시작하지 않은 채 셋째가 들어온 뒤 `cancelled`가 되었다(생성 02:38:23Z, 취소 02:38:28Z). 셋째의 `build`는 첫 실행이 끝난 뒤(02:38:59Z) 시작했다. 첫째와 셋째는 `success`이며, 진행 중에 취소된 실행은 없다 | GitHub Actions 실행 기록 |
| 공개 사이트 수동 확인 | 3.5절 4·5단계 기대 결과 | 커밋 `92749c8`의 Pages 사이트. Windows 11 Pro 10.0.26200, 영어 브라우저로 루트·두 책·검색·없는 경로를 열고 `gh repo view --json homepageUrl`로 About 주소를 확인했다 | 통과(2026-11-12). 루트는 200과 HLP-002이며 브라우저에서 `/vnc-relay/en/`로 이동했다. `ko/`·`en/`·장 페이지·`assets/`는 200, 없는 경로는 404와 HLP-003이었다. 한국어 책에서 "ticket 공개 키" 검색 결과는 17건이었고 콘솔 오류는 없었다. `homepageUrl`이 Pages 주소였다 | 이 표 |
| 게시된 화면 캡처 확인 | 게시한 그림이 저장소의 대상 파일과 같다 | GitHub Pages, 커밋 `8169bbf`·`a94058d`의 게시본. 두 언어의 책과 `images/operations.png`를 내려받아 HTTP 상태와 SHA-256을 저장소 파일과 대조했다 | 통과(2026-11-17). `ko/`·`en/`과 두 언어의 `images/operations.png`는 200이었다. 두 그림의 SHA-256이 저장소 파일과 같았다. 게시된 한국어 대시보드 장에 대기 연결의 전송 경로·상대 주소 설명이 있었다 | 이 표 |
| 화면 캡처 | 3.4절 2·3단계 기대 결과 | 3.4절. WSL2 Ubuntu, 대기·활성 표 통일 작업 트리(`web/dashboard/app/panels.js`·`app.css`), `TMPDIR` 없음 | 통과(2026-11-17). 시험 1개 통과, 두 책의 PNG 열두 장을 모두 열어 확인했다. 보이는 값은 시험 계정 `alice`, 루프백 주소, `/tmp` 아래 시험 경로, 시험 세션 식별자뿐이다 | `docs/help/*/src/images/`, [대시보드 검증 기록](verify-dashboard-frontend.md) |
| 추천 구성 사례의 예시 | 두 책 `recommended-setup.md`의 "ticket 발급 예시"·"ticket 확인 예시" 기대 결과 | 한국어 책의 코드 블록을 그대로 꺼내 실행했다. 영어 책의 Python 코드는 한국어 책과 문자열만 다르고(AST 대조) 나머지 코드 블록은 같다. 영어 책의 스크립트도 Windows에서 키 생성·발급·서명 확인을 실행했다. VNC Relay는 페이지의 `relay.toml`에서 리스너를 루프백 시험 포트로 바꾸고 뷰어 `allow`에 `127.0.0.1`을 더했다. 인증서는 `relay.example.test`의 자체 서명 인증서다. curl 줄에는 `--resolve`와 `--cacert`를 더했고, 인증서 확인을 끄는 `-k`는 쓰지 않았다. 그 밖에는 진행 표시를 끄는 `-sS`(Ubuntu)와 3단계 바이트를 보기 위한 출력 저장(Windows `-o`, Ubuntu `od`)만 붙였다. Windows 11 Pro 10.0.26200: Windows PowerShell, Python 3.14.6, `cryptography` 50.0.2, curl 8.22.0, `vnc-relay 2026.11.6` 디버그 빌드. WSL2 Ubuntu 24.04.4: Python 3.12.3, `python3-cryptography` 41.0.7, curl 8.5.0, `vnc-relay 2026.11.5` 디버그 빌드(이후 `crates/`·`src/` 변경은 트레이 아이콘뿐) | 통과(2026-11-18). 두 OS에서 키 파일 두 쌍이 생겼고 서명 확인이 `aud = vnc-relay/server`, `rid = 482913`인 클레임을 출력했다. 1단계는 101과 `Sec-WebSocket-Protocol: binary`, curl 종료 코드 28, `ticket_accepted`(server). 2단계는 401과 `WWW-Authenticate: VncRelayTicket`, `ticket_reused`. 3단계는 101 뒤 첫 프레임 `82 0c`와 `RFB 000.000\n`, `ticket_accepted`(viewer). Windows에서 다른 역할의 공개 키 파일, DER 서명, 같은 `kid`의 다른 키는 각각 kid 없음 메시지, 71바이트 서명 메시지, `InvalidSignature`로 끝났다 | 이 표 |
| 추천 구성 사례 페이지와 구성도 | [도움말 작성 규칙](../help/AGENTS.md#구성도)의 구성도 규칙 | Windows 11 Pro 10.0.26200, `vnc-relay 2026.11.6` 작업 트리. 3.2절 미리보기(`--port 18790`)를 헤드리스 WebKit으로 열어 두 언어의 페이지를 1280px·360px 폭에서 봤다 | 통과(2026-11-18). 구성도(760×380)는 1280px에서 692×346, 360px에서 292×146으로 줄었고 페이지 가로 스크롤은 없었다. 두 언어 모두 글씨가 상자 안에 있다. 360px에서는 글씨가 작다. 콘솔 오류는 0건이다 | 이 표. 화면 캡처는 파일로 남기지 않았다 |
| VNC 연결 구성의 방식 구성도와 상단 막대 | [도움말 작성 규칙](../help/AGENTS.md#구성도)의 구성도 규칙, [도움말 스펙](../specs/help-site.md)의 상단 막대 구성 | Windows 11 Pro 10.0.26200, 커밋 `c0fa253` 위 작업 트리. 3.2절 미리보기(`--port 18093`)를 ego lite 0.6.3.14(DOM 측정)와 헤드리스 Chrome 157.0.8103.79(CDP 폭 지정 캡처)로 열어 두 언어의 `connect.html`·`index.html`을 1280px·360px 폭에서 봤다 | 통과(2026-11-19). 구성도(760×490)는 1280px에서 695×448, 360px에서 279×181로 줄었고 페이지 가로 스크롤은 없었다. 두 언어 모두 글씨가 상자 안에 있다. 360px에서는 글씨가 작다. 상단 막대에는 목차·사이트 이름·검색·언어만 있고 `main 기준` 링크가 없다. 소개 페이지의 절은 세 개이며 기준 버전 절이 없다. 콘솔 오류는 확인하지 않았다 | 이 표. 화면 캡처는 저장소 밖 `D:/vr-tools/tmp/shots/`에 있다 |

### 4.1 시안 대조

2026-11-12에 커밋 `92749c8`의 구현과 채택 시안 [help-site.html](../design/mockups/help-site.html)의 21개 프레임을 같은 상태에서 대조했다. 구현은 `cargo xtask help-serve`로 띄운 사이트다. 데스크톱 프레임은 1080px, 좁은 폭 프레임은 400px에서 캡처했다. 모바일 목차 펼침(ko)만 390px에서 캡처했다. 기록자는 에이전트이며 사람의 검토는 하지 않았다.

| 확인 대상 | 기준 문서·범위 | 환경·명령·방법 | 실제 결과 | 실행·검토 증거 |
| --- | --- | --- | --- | --- |
| 데스크톱 페이지 틀 | `hlp001-desktop-ko`·`hlp001-desktop-en`·`hlp001-collapsed-ko`, DESIGN 페이지 틀·상단 막대·목차 행 | Windows 11, 헤드리스 Chrome(CDP), 1080px | 같음. 검색 단추 강조는 고쳤다(아래 차이 2) | 쌍 그림 2개. desktop-ko는 세션 안 화면 캡처 |
| 모바일 폭 | `hlp001-mobile-ko`·`hlp001-mobile-en`·`hlp001-mobile-open-ko` | 같음, 400px(목차 펼침은 390px) | 같음 | 쌍 그림 2개. mobile-open-ko는 세션 안 화면 캡처 |
| 검색 | `hlp001-search-idle-ko`·`search-loading-ko`·`search-results-ko`·`search-results-en`·`search-none-ko`·`search-failed-en`·`search-mobile-ko`, [검색 규칙](../specs/help-site.md#검색-규칙) | 같음, 1080px·400px | 같음. 실제 문구: 불러오는 중 "검색 자료를 불러오는 중입니다.", 실패 "Could not load search data. Close search and open it again." | 쌍 그림 6개. search-results-ko는 세션 안 화면 캡처 |
| 코드 복사 | `hlp001-copy-done-ko`·`hlp001-copy-failed-en` | 같음, 1080px | 같음. 실패 문구 "Could not copy. Select the code and copy it manually." | 쌍 그림 1개. copy-done-ko는 세션 안 화면 캡처 |
| 단축키 도움말 | `hlp001-shortcuts-ko`·`hlp001-shortcuts-en` | 같음, 1080px | 같음 | 쌍 그림 1개. shortcuts-ko는 세션 안 화면 캡처 |
| 사이트 루트 | `hlp002-default`·`hlp002-narrow` | 같음, 1080px·400px | 같음 | 쌍 그림 1개. default는 세션 안 화면 캡처 |
| 없는 페이지 | `hlp003-default`·`hlp003-narrow` | 같음, 1080px·400px | 같음 | 쌍 그림 1개. default는 세션 안 화면 캡처 |

쌍 그림은 로컬 `target/help-shots/`의 `mock-<프레임>.png`와 `<프레임>.png` 14쌍, 묶음 그림 `sheet-*.png` 7장이다. "세션 안 화면 캡처"는 대조 세션에서 보고 파일로 남기지 않은 7개 프레임이다.

| 차이·분류 | 시안과 구현의 차이 | 근거 | 처리 |
| --- | --- | --- | --- |
| 1. 검색 입력의 지우기 단추 | 구현: Chrome이 초점 있는 `type="search"` 입력에 기본 ✕ 단추를 그린다. 시안도 같은 입력이지만 프레임에 초점이 없다 | 브라우저 기본 동작 | 유지 |
| 2. 검색 단추 강조 | 초기 구현: 검색 패널이 열리면 검색 단추도 활성 바탕. 시안: 활성 바탕은 목차 단추에만 | DESIGN 상단 막대 행, 시안 | 고침. `web/help/theme/help.css`의 활성 바탕 규칙에서 검색 단추를 뺐다 |
| 3. 본문 없는 절의 검색 결과 | 구현: 제목 바로 뒤에 하위 절이 오는 절(예: "Operations › Replace certificate and public key files")은 발췌 없이 링크만 보인다. 시안에는 이런 결과가 없다 | [검색 규칙](../specs/help-site.md#검색-규칙) | 유지 |
| 4. 포인터 흔적 | 구현 캡처의 hover 바탕은 클릭한 위치에서 생긴다 | 캡처 조건 | 해당 없음 |
| 5. 내용 | 시안의 자리표시 그림 대신 실제 화면 캡처, 검색 결과 수 | 도움말의 실제 내용 | 데이터 차이, 해당 없음 |

## 5. 중단 조건

- `cargo xtask help-site`가 실패하면 push하지 않는다. 워크플로도 같은 검사에서 실패하고 게시하지 않는다.
- mdBook 자산의 SHA-256이 2.1절 값과 다르면 설치하지 않는다.
- 캡처에 실제 조직·호스트 이름, 실제 IP, 사용자·호스트 이름이 든 로컬 경로, 비밀 값이 보이면 커밋하지 않는다.

## 6. 롤백 또는 복구

1. 잘못 게시한 내용은 고친 커밋이나 되돌린 커밋을 `main`에 push해 다시 게시한다. 3.5절 3·4단계로 확인한다.
2. 사이트를 내려야 하면 Pages를 끈다.

   영향: `https://kayden-kim.github.io/vnc-relay/` 아래 모든 주소가 404가 된다. 3.5절 1단계로 다시 켤 수 있다.

   ```shell
   gh api -X DELETE repos/kayden-kim/vnc-relay/pages
   ```

3. `gh api repos/kayden-kim/vnc-relay/pages`가 HTTP 404를 돌려주는지 확인한다.

## 7. 해결되지 않을 때 전달할 대상

- 전달 대상: 프로젝트 관리자다.
- 보존할 증거: 커밋, GitHub Actions 실행 주소, 실패한 작업 이름과 로그, `cargo xtask help-site`의 출력이다.
- 비밀 값 제외 규칙: 토큰과 계정 비밀번호를 기록하지 않는다. Actions 로그는 `GITHUB_TOKEN`을 가린 상태로 보존한다.
