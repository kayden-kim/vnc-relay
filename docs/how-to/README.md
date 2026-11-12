# how-to 목록

how-to는 특정 개발·운영 작업을 안전하게 완료하기 위한 절차다.

| 문서 | 대상 작업 | 검증 환경·버전 | 최종 검증일 |
| --- | --- | --- | --- |
| [의존성 라이선스를 검사하는 방법](check-dependency-licenses.md) | Rust 의존성 허용 검사, 재현 가능한 배포 고지 생성·대조, 독립 구현 출처 검토 기록 | Ubuntu 24.04.4 LTS(WSL2)·Windows 11 Pro 10.0.26200, Rust 1.99.1, cargo-deny 0.20.2 | 2026-11-04 |
| [개발 환경을 준비하고 변경을 검증하는 방법](set-up-and-verify.md) | 두 OS의 개발 환경 준비, 빌드, 변경 범위별 검증, 전체 완료 검증 | Ubuntu 24.04.4 LTS(WSL2)와 Windows 11 Pro 10.0.26200, Rust 1.99.1. rustup 설치 단계는 두 OS 모두 미실행(Draft) | 2026-10-20 |
| [대시보드 프런트엔드를 검증하는 방법](verify-dashboard-frontend.md) | 대시보드 토큰·제공 파일·브라우저 테스트와 수동 시각·접근성 검토 | 미실행(Draft). 실행 환경은 실행할 때 기록 | 없음(Draft 2026-10-22) |
| [실제 UltraVNC로 TCP 중계를 수동 확인하는 방법](verify-ultravnc-manually.md) | TODO 2.13의 Mode I·II 화면·입력 확인 절차(Q116에 따라 에이전트 자동 실행, 사용자 캡처 검토)와 AC별 수동·자동 증거 기록 | Draft. Windows VNC Relay로 실행(Ubuntu VNC Relay는 분리 실행 환경이 없어 미실행). 고정 UltraVNC Server·Viewer 1.8.3.0, Hyper-V 분리 VM 3대 | 2026-11-06 |
| [실행파일을 묶고 수동 운영하는 방법](operate-relay.md) | 두 OS의 release 빌드·묶음, 계정 준비·시작·중계·재로드·종료, 인증서 교체·로그 회전 | 미실행(Draft). Windows 11 x64·Ubuntu 24.04 LTS x64 | 없음(Draft 2026-11-04) |
| [중계 성능과 잔여 자원을 측정하는 방법](measure-performance.md) | OPS-017·OPS-018과 AC-08의 루프백 부하·지연·CPU·RSS·핸들/fd 측정, 원시 JSON 보존 | Draft. Windows 11 x64·WSL2 Ubuntu 24.04 LTS x64. 스모크와 정식 측정은 본문 4절에서 구분 | 없음(Draft 2026-11-06) |
| [릴리스 묶음을 만들고 배포하는 방법](release.md) | 버전 규칙(`연.월.순서`), `cargo xtask package` 묶음, GitHub Actions 검사, 태그로 GitHub Release 내기 | Verified. Windows 11 Pro 10.0.26200·WSL2 Ubuntu 24.04.4, Rust 1.99.1, GitHub Actions `windows-2025`·`ubuntu-24.04`. 첫 Release `v2026.11.0` | 2026-11-08 |
| [도움말을 빌드하고 게시하는 방법](publish-help.md) | mdBook 0.5.4 설치, 도움말 빌드·검사, 로컬 미리보기, 브라우저 시험, 화면 캡처 갱신, GitHub Pages 게시와 복구 | Verified. Windows 11 Pro 10.0.26200·WSL2 Ubuntu 24.04.4, mdBook 0.5.4, Rust 1.99.1, GitHub Actions `ubuntu-24.04`. 6절 롤백(Pages 끄기)은 미실행 | 2026-11-12 |

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.
