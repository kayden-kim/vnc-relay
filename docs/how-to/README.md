# how-to 목록

how-to는 특정 개발·운영 작업을 안전하게 완료하기 위한 절차다.

| 문서 | 대상 작업 | 검증 환경·버전 | 최종 검증일 |
| --- | --- | --- | --- |
| [의존성 라이선스를 검사하는 방법](check-dependency-licenses.md) | Rust 의존성 허용 검사, 재현 가능한 배포 고지 생성·대조, 독립 구현 출처 검토 기록 | Ubuntu 24.04.4 LTS(WSL2)·Windows 11 Pro 10.0.26200, Rust 1.99.1, cargo-deny 0.20.2 | 2026-11-04 |
| [개발 환경을 준비하고 변경을 검증하는 방법](set-up-and-verify.md) | 두 OS의 개발 환경 준비, 빌드, 변경 범위별 검증, 전체 완료 검증 | Ubuntu 24.04.4 LTS(WSL2)와 Windows 11 Pro 10.0.26200, Rust 1.99.1. rustup 설치 단계는 두 OS 모두 미실행(Draft) | 2026-10-20 |
| [대시보드 프런트엔드를 검증하는 방법](verify-dashboard-frontend.md) | 대시보드 토큰·제공 파일·브라우저 테스트와 수동 시각·접근성 검토 | 미실행(Draft). 실행 환경은 실행할 때 기록 | 없음(Draft 2026-10-22) |
| [실제 UltraVNC로 TCP 중계를 수동 확인하는 방법](verify-ultravnc-manually.md) | 사용자가 TODO 2.13의 Mode I·II 화면·입력을 확인하고 AC별 수동·자동 증거를 기록 | 미실행(Draft). 고정 UltraVNC Server·Viewer 1.8.3.0, 분리 테스트 네트워크 | 없음(Draft 2026-11-02) |
| [실행파일을 묶고 수동 운영하는 방법](operate-relay.md) | 두 OS의 release 빌드·묶음, 계정 준비·시작·중계·재로드·종료, 인증서 교체·로그 회전 | 미실행(Draft). Windows 11 x64·Ubuntu 24.04 LTS x64 | 없음(Draft 2026-11-04) |

작성 규칙은 [AGENTS.md](AGENTS.md)를 따른다.
