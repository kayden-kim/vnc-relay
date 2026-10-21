# 운영 대시보드 화면 문구 초안

| 항목 | 값 |
| --- | --- |
| 상태 | Draft. 승인 전 초안 |
| 작성일 | 2026-10-21 |
| 확정 대상 | [운영 대시보드 화면 스펙](dashboard.md)의 DASH-TBD-003. 스펙 §3은 두 언어의 문구를 이 항목과 함께 확정한다고 정한다 |
| 화면 스펙 | [운영 대시보드 화면 스펙](dashboard.md) |
| 관리 API 계약 | [관리 API v1](../contracts/management-api-v1.md). Active, 2026-10-16 채택 |
| 관리 스펙 | [관리 조회와 세션 종료](management.md) |
| 디자인 기준 | [운영 대시보드 디자인 기준](../design/DESIGN.md). Accepted, 2026-10-20 채택 |
| 용어 | [VNC Relay 용어집](../../GLOSSARY.md) |
| 승인 | 대기. 사용자 결정 2026-10-21: 화면 문구는 승인용 초안으로 먼저 작성한다 |

## 1. 상태와 읽는 법

- 이 문서는 초안이다. 승인 전에는 문구가 확정되지 않는다.
- 이 문서는 DASH-TBD-003을 해소하지 않는다. 승인 뒤 확정 위치를 정한다(5절 9번).
- 화면 동작(상태, 요청, 폴링, 확인 대화상자)은 화면 스펙이 정한다. 이 문서는 동작을 바꾸지 않는다.
- 시안(`../design/mockups/dashboard.html`)의 문구는 예시다. 화면 스펙 머리말에 따라 문장이 다르면 스펙을 따른다.
- "근거" 열의 "스펙"은 화면 스펙을, "계약"은 관리 API 계약을, "관리 스펙"은 관리 스펙을 가리킨다.
- "제안"은 어떤 문서도 문구를 정하지 않아 이 초안이 처음 낸 문구다. 승인 대상이다.
- 문구 ID `CPY-`는 이 문서 안에서만 쓴다. 한 번 쓴 ID는 다시 쓰지 않는다.

## 2. 공통 규칙

- 한국어와 English 문구는 같은 ID 아래 짝을 이룬다. 한쪽에만 있는 문구는 없다.
- 세션 식별자, 중계 ID, 주소, Mode I·Mode II, TCP·WS·WSS 같은 값과 이름은 번역하지 않는다(스펙 §3).
- 목록에 없는 값은 받은 그대로 보여 준다(스펙 §3, 계약 server.state).
- 오류 문구는 입력값을 되풀이하지 않는다(계약, 오류 본문 규칙).
- 로그인 실패 문구는 계정 존재 여부를 드러내지 않는다. 실패 제한 중에도 같은 문구를 쓴다(스펙 §3, 관리 스펙 §3.0).
- 언어 이름은 각 언어로 쓴다: "한국어", "English"(디자인 기준).
- 두 언어의 길이가 다르다. 라벨과 버튼에 고정 폭을 두지 않는다(디자인 기준).
- 한국어 안내는 설명에 "~습니다", 행동 요청에 "~하십시오"를 쓴다(제안).

## 3. 문구 목록

### 3.1 공통

| ID | 위치 | 한국어 | English | 근거 |
| --- | --- | --- | --- | --- |
| CPY-001 | 언어 선택 이름 | 언어 (Language) | 언어 (Language) | 디자인 기준 접근성 표(두 언어 이름) |
| CPY-002 | 언어 선택 항목 | 한국어 | English | 디자인 기준 언어 선택, 언어 이름 규칙 |
| CPY-003 | 제품 이름 | VNC Relay | VNC Relay | 스펙 SCR-003 제목 |
| CPY-004 | SCR-001 화면 이름 | 운영 현황 | Operations overview | 스펙 SCR-001 이름. English는 제안 |
| CPY-005 | SCR-002 화면 이름 | 세션 상세 | Session details | 스펙 SCR-002 이름. English는 제안 |
| CPY-006 | SCR-003 화면 제목 | VNC Relay 관리자 로그인 | VNC Relay administrator sign in | 스펙 SCR-003 제목. English는 제안 |
| CPY-007 | 상단 막대 로그아웃 | 로그아웃 | Sign out | 디자인 기준 버튼 목록. English는 제안 |
| CPY-008 | 새로고침 버튼 | 새로고침 | Refresh | 스펙 §3, 디자인 기준 버튼 |
| CPY-009 | 상세 열기 버튼 | 상세 | Details | 디자인 기준 버튼. English는 제안 |
| CPY-010 | 목록으로 버튼 | 목록으로 | Back to list | 스펙 SCR-002, 디자인 기준 버튼 |
| CPY-011 | 취소·닫기 버튼 | 취소 | Cancel | 스펙 §3 대화상자, 디자인 기준 버튼 |
| CPY-012 | 로그인 버튼 | 로그인 | Sign in | 디자인 기준 버튼. English는 제안 |
| CPY-013 | 로그인 버튼, 요청 중 | 로그인 중… | Signing in… | 스펙 SCR-003 요청 중 비활성. 문구는 제안 |
| CPY-014 | 상태 재조회 버튼 | 상태 재조회 | Check status again | 스펙 §4, 디자인 기준 버튼 |

### 3.2 SCR-001 운영 현황

| ID | 위치 | 한국어 | English | 근거 |
| --- | --- | --- | --- | --- |
| CPY-015 | 마지막 성공 갱신 | 마지막 성공 갱신: {시각} | Last successful update: {time} | 스펙 §3 |
| CPY-016 | 목록 상태 배지 | 조회 중 | Loading | 스펙 §4 상태 이름. 배지 문구는 제안 |
| CPY-017 | 목록 상태 배지 | 갱신 중 | Updating | 스펙 §4 상태 이름. 배지 문구는 제안 |
| CPY-018 | 목록 상태 배지 | 조회 실패 | Failed to load | 스펙 §4 상태 이름. 배지 문구는 제안 |
| CPY-019 | 목록 상태 배지 | 오래된 데이터 | Stale data | 스펙 §3 "오래된 성공 데이터". 배지 문구는 제안 |
| CPY-020 | 조회 실패, 이전 데이터 있음 | 조회에 실패했습니다. 아래 값은 마지막 성공 시각 {시각}의 이전 데이터입니다. | Loading failed. The values below are from the last successful update at {time}. | 스펙 §4 (이전 데이터와 마지막 성공 시각). 문장은 제안 |
| CPY-021 | 조회 실패, 이전 데이터 없음 | 조회에 실패했습니다. 새로고침으로 다시 시도하십시오. | Loading failed. Use Refresh to try again. | 스펙 §4 (명시적 재조회). 문장은 제안 |
| CPY-022 | 서버 요약 제목 | 서버 요약 | Server summary | 스펙 §3 |
| CPY-023 | 실행 상태 라벨 | 실행 상태 | Server state | 스펙 §3, 계약 server.state |
| CPY-024 | 실행 상태 값 `ready` | 준비됨 | Ready | 계약 server.state 설명. English는 제안 |
| CPY-025 | 실행 상태 값 `shutdown_grace` | 종료 유예 중 | Shutting down (grace period) | 계약 server.state 설명. English는 제안 |
| CPY-026 | 가동 시간 라벨 | 가동 시간 | Uptime | 스펙 §3. English는 제안 |
| CPY-027 | 대기 연결 수 라벨 | 대기 연결 수 | Waiting connections | 계약 counts.waiting. 라벨은 제안 |
| CPY-028 | 활성 세션 수 라벨 | 활성 세션 수 | Active sessions | 계약 counts.active. 라벨은 제안 |
| CPY-029 | 초기 교환 중 연결 수 | 초기 교환 중 | Initial exchange | 스펙 §3 연결 수 |
| CPY-030 | Mode I 대상 연결 중 연결 수 | Mode I 대상 연결 중 | Connecting to Mode I target | 스펙 §3 연결 수. English는 제안 |
| CPY-031 | 중계 연결 사용량 이름 | 중계 연결 | Relay connections | 스펙 §3 (기호 L 대신 이름). English는 제안 |
| CPY-032 | 중계 연결 설명 | 중계 측 연결 소켓의 현재 수와 상한입니다. | Current count and limit of relay-side connection sockets. | 관리 스펙 §2 (L). 문장은 제안 |
| CPY-033 | 관리 연결 사용량 이름 | 관리 연결 | Management connections | 스펙 §3 (기호 Lm 대신 이름). English는 제안 |
| CPY-034 | 관리 연결 설명 | 관리·상태 리스너의 연결 소켓 현재 수와 상한입니다. | Current count and limit of management and status listener sockets. | 관리 스펙 §2 (Lm). 문장은 제안 |
| CPY-035 | 값을 읽지 못함 | 확인 불가 | Unavailable | 관리 스펙 §2 (확인 불가, 0으로 표시하지 않음) |
| CPY-036 | 대기 연결 목록 제목 | 대기 연결 | Waiting connections | 스펙 SCR-001 |
| CPY-037 | 대기 연결 표 머리글 | 역할, 중계 ID, 접속 시각, 대기 시간 | Role, Relay ID, Connected at, Waiting time | 스펙 §3 표. English는 제안 |
| CPY-038 | 대기 연결 빈 목록 | 대기 없음 | No waiting connections | 스펙 §3, §4 |
| CPY-039 | 활성 세션 목록 제목 | 활성 세션 | Active sessions | 스펙 SCR-001 |
| CPY-040 | 활성 세션 표 머리글 | 세션, 모드, 전송 경로, 시작, Viewer->대상 / 대상->Viewer, 상대 주소, 상세 | Session, Mode, Transport, Started, Viewer->target / target->Viewer, Peer address, Details | 스펙 §3 표. English는 제안 |
| CPY-041 | 활성 세션 빈 목록 | 활성 없음 | No active sessions | 스펙 §3, §4 |
| CPY-042 | 최근 종료 목록 제목 | 최근 종료 목록 | Recently closed sessions | 스펙 SCR-001, 용어집 |
| CPY-043 | 최근 종료 표 머리글 | 세션, 종료 원인, 종료 시각, Viewer->대상 / 대상->Viewer | Session, Close reason, Closed at, Viewer->target / target->Viewer | 스펙 §3 표. English는 제안 |
| CPY-044 | 최근 종료 빈 목록 | 최근 종료 없음 | No recently closed sessions | 스펙 §3, §4 |
| CPY-045 | 종료 원인 `peer_eof` | 정상 EOF | Normal EOF | 관리 스펙 §2.3, 용어집 |
| CPY-046 | 종료 원인 `connection_error` | 연결 오류 | Connection error | 관리 스펙 §2.3 |
| CPY-047 | 종료 원인 `admin_terminated` | 관리자 종료 | Terminated by administrator | 관리 스펙 §2.3, 용어집 |
| CPY-048 | 종료 원인 `dead_connection` | 끊긴 연결 감지 | Dead connection detected | 관리 스펙 §2.3 |
| CPY-049 | 종료 원인 `close_timeout` | 정리 제한 시간 초과 | Cleanup time limit exceeded | 관리 스펙 §2.3. English는 제안 |
| CPY-050 | 종료 원인 `server_shutdown` | 서버 종료 | Server shutdown | 관리 스펙 §2.3, 용어집 |

### 3.3 SCR-002 세션 상세

| ID | 위치 | 한국어 | English | 근거 |
| --- | --- | --- | --- | --- |
| CPY-051 | 화면 제목 | 세션 {식별자} | Session {session_id} | 스펙 SCR-002 |
| CPY-052 | 마지막 성공 갱신 | 마지막 성공 갱신: {시각} | Last successful update: {time} | 스펙 SCR-002 |
| CPY-053 | 모드 라벨 | 모드 | Mode | 스펙 SCR-002 |
| CPY-054 | Viewer 전송 경로 라벨 | Viewer 전송 경로 | Viewer transport | 스펙 SCR-002 |
| CPY-055 | 시작 시각 라벨 | 시작 시각 | Started at | 스펙 SCR-002. English는 제안 |
| CPY-056 | Viewer 주소 라벨 | Viewer 주소 | Viewer address | 스펙 SCR-002 |
| CPY-057 | Server·대상 주소 라벨 | Server/대상 주소 | Server/target address | 스펙 SCR-002. English는 제안 |
| CPY-058 | 주소 설명 | 실제 소켓 상대 주소입니다. | Actual socket peer address. | 스펙 SCR-002. 문장은 제안 |
| CPY-059 | 대상 없음 안내 | 현재 세션을 찾을 수 없음 | The current session was not found | 스펙 §4 (문구 그대로). English는 제안 |
| CPY-060 | 세션 종료 버튼 | 세션 종료 | Terminate session | 디자인 기준 버튼. English는 제안 |
| CPY-061 | 종료 확인 대화상자 제목 | 세션 종료 확인 | Confirm session termination | 스펙 SCR-002 대화상자. English는 제안 |
| CPY-062 | 종료 확인 대화상자 본문 | 세션 {식별자}의 연결을 끊습니다. Viewer와 Server 또는 Mode I 대상 사이의 중계가 멈춥니다. 끊긴 세션은 다시 이어지지 않습니다. | The connection for session {session_id} will close. The relay between the Viewer and the Server or Mode I target will stop. A closed session does not resume. | 스펙 §3 (끊김 영향), 관리 스펙 §4 (종료된 세션을 복원하지 않음). 문장은 제안 |
| CPY-063 | 종료 확인 버튼 | 종료 확인 | Confirm termination | 디자인 기준 버튼 |
| CPY-064 | 종료 요청 중 버튼 | 종료 요청 중… | Terminating… | 스펙 §4 요청 중. 문구는 제안 |
| CPY-065 | 종료 처리 실패 안내 | 세션 종료에 실패했습니다. 상태를 다시 확인하십시오. | Session termination failed. Check the status again. | 스펙 §4, DASH-012 (완료로 표시하지 않음). 문장은 제안 |
| CPY-066 | 결과 확인 불가 안내 | 종료 결과를 확인할 수 없습니다. 세션이 끝났을 수 있습니다. 요청을 자동으로 다시 보내지 않습니다. 상태를 다시 확인하십시오. | The termination result cannot be confirmed. The session may have ended. The request is not sent again automatically. Check the status again. | 스펙 §4·§5, DASH-013. 문장은 제안 |
| CPY-067 | 잘못된 세션 식별자 | 잘못된 세션 식별자 | Invalid session ID | 계약 화면 URL 규칙 (형식이 틀리면 요청 없이 표시). 검토 필요(5절 4번) |
| CPY-068 | 종료 요청 거절 (403) | 요청을 처리하지 못했습니다. 화면을 다시 확인한 뒤 종료를 다시 요청하십시오. | The request was not processed. Check the screen, then request termination again. | 계약 403 csrf_invalid 처리, 관리 스펙 §4. 문장은 제안. 검토 필요(5절 4번) |

### 3.4 SCR-003 관리자 로그인

| ID | 위치 | 한국어 | English | 근거 |
| --- | --- | --- | --- | --- |
| CPY-069 | 계정명 라벨 | 계정명 | Account name | 스펙 SCR-003 (필수) |
| CPY-070 | 비밀번호 라벨 | 비밀번호 | Password | 스펙 SCR-003 (필수, 가림) |
| CPY-071 | 계정명 빈 입력 | 계정명을 입력하십시오. | Enter an account name. | 스펙 §4 (빈 입력은 보내지 않음). 문장은 제안 |
| CPY-072 | 비밀번호 빈 입력 | 비밀번호를 입력하십시오. | Enter a password. | 스펙 §4 (빈 입력은 보내지 않음). 문장은 제안 |
| CPY-073 | 로그인 확인 중 안내 | 로그인을 확인하는 중입니다. | Checking sign-in. | 스펙 SCR-003 안내 영역 "로그인 중". 문장은 제안 |
| CPY-074 | 공통 인증 실패 안내 | 로그인할 수 없습니다. 계정명과 비밀번호를 확인한 뒤 다시 시도하십시오. | Sign-in failed. Check the account name and password, then try again. | 스펙 §3·§4 (공통 안내, 계정 존재 여부 비노출), 계약 login_failed. 문장은 제안. 검토 필요(5절 2번) |
| CPY-075 | 인증 필요 안내 | 로그인이 필요합니다. 다시 로그인하십시오. | Sign-in is required. Sign in again. | 계약 unauthenticated (세션 없음·만료·로그아웃·재시작을 구분하지 않음). 문장은 제안. 검토 필요(5절 3번) |
| CPY-076 | 로그아웃 결과 불명 안내 | 로그아웃이 끝났는지 확인하지 못했습니다. 이 화면의 관리 데이터는 숨겼습니다. | Could not confirm that sign-out finished. Management data is hidden in this screen. | 스펙 §5 (완료로 표시하지 않음, 데이터 숨김, 확인 필요 알림). 문장은 제안. 검토 필요(5절 5번) |
| CPY-077 | 로그인 요청 결과 불명 또는 서버 오류 | 로그인 요청을 처리하지 못했습니다. 잠시 뒤 다시 시도하십시오. | The sign-in request could not be processed. Try again later. | 스펙·계약에 이 상태가 없다. 후보만 제시. 검토 필요(5절 6번) |

## 4. 근거 문서

- 화면 문구의 상태·안내: [화면 스펙](dashboard.md) §3, §4, §5, DASH-009~013, DASH-018~019.
- 오류 코드와 화면 처리: [관리 API v1](../contracts/management-api-v1.md) 오류 코드와 소비자 처리 표, 화면 URL 규칙, server.state, counts 필드.
- 종료 원인과 세션 복원: [관리 스펙](management.md) §2.3, §3.0, §4, §5.
- 버튼 이름과 언어 이름: [디자인 기준](../design/DESIGN.md)의 버튼 목록, 언어 선택 규칙, 접근성 표.
- 용어: [VNC Relay 용어집](../../GLOSSARY.md).

## 5. 승인 요청 항목

1. 종료 완료 안내. 화면 스펙 DASH-010은 종료 완료 뒤 대상 없음과 목록 갱신만 정한다. 선택지: (a) CPY-059만 쓴다(현재 초안). (b) 완료 안내 문구와 상태를 따로 추가한다.
2. 공통 인증 실패 문구(CPY-074). 제한 중에도 같은 문구를 쓴다는 점은 스펙이 정했다. 문구를 승인하거나 고친다.
3. 인증 필요 문구(CPY-075). 계약은 세션 없음·만료·로그아웃·재시작을 구분하지 않는다. 선택지: (a) 한 문구로 쓴다(현재 초안). (b) 원인별 문구를 쓴다. 이 경우 계약 변경이 필요하며 사용자 승인이 필요하다.
4. 종료 요청 거절(CPY-068)과 잘못된 세션 식별자(CPY-067). 화면 스펙에 해당 상태 행이 없다. 선택지: (a) 종료 처리 실패 상태에 포함한다. (b) 별도 상태 행을 추가한다(스펙 §4 변경).
5. 로그아웃 결과 불명 안내(CPY-076). 표시 위치가 정해지지 않았다. 선택지: (a) SCR-003에 표시한다. (b) 직전 화면에 표시한다.
6. 로그인 요청 결과 불명·서버 오류(CPY-077). 화면 스펙에 상태가 없다. 상태를 정한 뒤 문구를 확정한다.
7. English 용어. Sign in, Sign out, Terminate session, Details, Peer address, Mode I target 같은 선택을 승인한다.
8. 한국어 존댓말. "~하십시오" 형식(2절)을 승인한다.
9. 확정 위치. 승인 뒤 선택지: (a) 이 문서를 확정 문구의 기준으로 두고, 화면 스펙 §8의 DASH-TBD-003 확정 위치를 이 문서로 바꾼다. (b) 문구 표를 화면 스펙 안으로 옮기고 이 문서는 지운다.
