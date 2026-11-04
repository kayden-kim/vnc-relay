// Copy for the operations overview (SCR-001). Each entry names its CPY number from dashboard.md §9.3.
// Values reach the page only through textContent or attributes. Tokens such as {time} are filled by fill().

export const BRAND_NAME = 'VNC Relay'; // CPY-003
export const LANGUAGE_LABEL = '언어 (Language)'; // CPY-001
// Each language is named in its own language.
export const LANGUAGE_NAMES = { ko: '한국어', en: 'English' }; // CPY-002

export const COPY = {
  ko: {
    navLabel: '주 메뉴', // CPY-078
    operations: '운영 현황', // CPY-004
    signOut: '로그아웃', // CPY-007
    refresh: '새로고침', // CPY-008
    details: '상세', // CPY-009
    lastSuccess: '마지막 성공 갱신: {time}', // CPY-015
    loading: '조회 중', // CPY-016
    refreshing: '갱신 중', // CPY-017
    failed: '조회 실패', // CPY-018
    stale: '오래된 데이터', // CPY-019
    staleMessage: '조회에 실패했습니다. 아래 값은 마지막 성공 시각 {time}의 이전 데이터입니다.', // CPY-020
    failedMessage: '조회에 실패했습니다. 새로고침으로 다시 시도하십시오.', // CPY-021
    unavailable: '확인 불가', // CPY-035
    limit: '상한 {n}', // CPY-079
    limitUnknown: '상한 확인 불가', // CPY-080
    listCount: '{n}건', // CPY-084
    listCountOne: '1건', // CPY-085
    summaryTitle: '서버 요약', // CPY-022
    serverState: '실행 상태', // CPY-023
    stateReady: '준비됨', // CPY-024
    stateShutdownGrace: '종료 유예 중', // CPY-025
    uptime: '가동 시간', // CPY-026
    waitingCount: '대기 연결 수', // CPY-027
    activeCount: '활성 세션 수', // CPY-028
    handshaking: '초기 교환 중', // CPY-029
    targetConnecting: 'Mode I 대상 연결 중', // CPY-030
    relayTitle: '중계 연결', // CPY-031
    relayNote: '중계 측 연결 소켓의 현재 수와 상한입니다.', // CPY-032
    adminTitle: '관리 연결', // CPY-033
    adminNote: '관리·상태 리스너의 연결 소켓 현재 수와 상한입니다.', // CPY-034
    waitingTitle: '대기 연결', // CPY-036
    waitingColumns: ['역할', '중계 ID', '접속 시각', '대기 시간'], // CPY-037
    waitingEmpty: '대기 없음', // CPY-038
    activeTitle: '활성 세션', // CPY-039
    activeColumns: ['세션', '모드', '전송 경로', '시작', 'Viewer->대상 / 대상->Viewer', '상대 주소', '상세'], // CPY-040
    activeEmpty: '활성 없음', // CPY-041
    closedTitle: '최근 종료 목록', // CPY-042
    closedColumns: ['세션', '종료 원인', '종료 시각', 'Viewer->대상 / 대상->Viewer'], // CPY-043
    closedEmpty: '최근 종료 없음', // CPY-044
    closeCause: {
      peer_eof: '정상 EOF', // CPY-045
      connection_error: '연결 오류', // CPY-046
      admin_terminated: '관리자 종료', // CPY-047
      dead_connection: '끊긴 연결 감지', // CPY-048
      close_timeout: '정리 제한 시간 초과', // CPY-049
      server_shutdown: '서버 종료', // CPY-050
      internal_error: '내부 오류', // CPY-083
    },
    directionViewerToTarget: 'Viewer->대상', // CPY-040·CPY-043 table headers (spec §9.3.2).
    directionTargetToViewer: '대상->Viewer', // CPY-040·CPY-043 table headers (spec §9.3.2).
    dayUnit: '일', // DESIGN l.491 시간 길이 표시(D일 HH:MM:SS)의 단위.
    // SCR-002 session detail.
    sessionScreen: '세션 상세', // CPY-005
    sessionTitle: '세션 {session_id}', // CPY-051
    back: '목록으로', // CPY-010
    cancel: '취소', // CPY-011
    recheck: '상태 재조회', // CPY-014
    sessionInfoTitle: '세션 정보', // CPY-081
    transferTitle: '전송량', // CPY-082
    modeLabel: '모드', // CPY-053
    transportLabel: 'Viewer 전송 경로', // CPY-054
    startedLabel: '시작 시각', // CPY-055
    viewerAddressLabel: 'Viewer 주소', // CPY-056
    targetAddressLabel: 'Server/대상 주소', // CPY-057
    addressNote: '실제 소켓 상대 주소입니다.', // CPY-058
    notFound: '현재 세션을 찾을 수 없음', // CPY-059
    terminate: '세션 종료', // CPY-060
    terminateTitle: '세션 종료 확인', // CPY-061
    terminateBody: '세션 {session_id}의 연결을 끊습니다. Viewer와 Server 또는 Mode I 대상 사이의 중계가 멈춥니다. 끊긴 세션은 다시 이어지지 않습니다.', // CPY-062
    terminateConfirm: '종료 확인', // CPY-063
    terminating: '종료 요청 중…', // CPY-064
    terminateFailed: '세션 종료에 실패했습니다. 상태를 다시 확인하십시오.', // CPY-065
    terminateUnknown: '종료 결과를 확인할 수 없습니다. 세션이 끝났을 수 있습니다. 요청을 자동으로 다시 보내지 않습니다. 상태를 다시 확인하십시오.', // CPY-066
    invalidSessionId: '잘못된 세션 식별자', // CPY-067
    terminateRejected: '요청을 처리하지 못했습니다. 화면을 다시 확인한 뒤 종료를 다시 요청하십시오.', // CPY-068
  },
  en: {
    navLabel: 'Main menu', // CPY-078
    operations: 'Operations overview', // CPY-004
    signOut: 'Sign out', // CPY-007
    refresh: 'Refresh', // CPY-008
    details: 'Details', // CPY-009
    lastSuccess: 'Last successful update: {time}', // CPY-015
    loading: 'Loading', // CPY-016
    refreshing: 'Updating', // CPY-017
    failed: 'Failed to load', // CPY-018
    stale: 'Stale data', // CPY-019
    staleMessage: 'Loading failed. The values below are from the last successful update at {time}.', // CPY-020
    failedMessage: 'Loading failed. Use Refresh to try again.', // CPY-021
    unavailable: 'Unavailable', // CPY-035
    limit: 'Limit {n}', // CPY-079
    limitUnknown: 'Limit unavailable', // CPY-080
    listCount: '{n} items', // CPY-084
    listCountOne: '1 item', // CPY-085
    summaryTitle: 'Server summary', // CPY-022
    serverState: 'Server state', // CPY-023
    stateReady: 'Ready', // CPY-024
    stateShutdownGrace: 'Shutting down (grace period)', // CPY-025
    uptime: 'Uptime', // CPY-026
    waitingCount: 'Waiting connections', // CPY-027
    activeCount: 'Active sessions', // CPY-028
    handshaking: 'Initial exchange', // CPY-029
    targetConnecting: 'Connecting to Mode I target', // CPY-030
    relayTitle: 'Relay connections', // CPY-031
    relayNote: 'Current count and limit of relay-side connection sockets.', // CPY-032
    adminTitle: 'Management connections', // CPY-033
    adminNote: 'Current count and limit of management and status listener sockets.', // CPY-034
    waitingTitle: 'Waiting connections', // CPY-036
    waitingColumns: ['Role', 'Relay ID', 'Connected at', 'Waiting time'], // CPY-037
    waitingEmpty: 'No waiting connections', // CPY-038
    activeTitle: 'Active sessions', // CPY-039
    activeColumns: ['Session', 'Mode', 'Transport', 'Started', 'Viewer->target / target->Viewer', 'Peer address', 'Details'], // CPY-040
    activeEmpty: 'No active sessions', // CPY-041
    closedTitle: 'Recently closed sessions', // CPY-042
    closedColumns: ['Session', 'Close reason', 'Closed at', 'Viewer->target / target->Viewer'], // CPY-043
    closedEmpty: 'No recently closed sessions', // CPY-044
    closeCause: {
      peer_eof: 'Normal EOF', // CPY-045
      connection_error: 'Connection error', // CPY-046
      admin_terminated: 'Terminated by administrator', // CPY-047
      dead_connection: 'Dead connection detected', // CPY-048
      close_timeout: 'Cleanup time limit exceeded', // CPY-049
      server_shutdown: 'Server shutdown', // CPY-050
      internal_error: 'Internal error', // CPY-083
    },
    directionViewerToTarget: 'Viewer->target', // CPY-040·CPY-043 table headers (spec §9.3.2).
    directionTargetToViewer: 'target->Viewer', // CPY-040·CPY-043 table headers (spec §9.3.2).
    dayUnit: 'd', // DESIGN l.491 duration unit ("Dd HH:MM:SS").
    // SCR-002 session detail.
    sessionScreen: 'Session details', // CPY-005
    sessionTitle: 'Session {session_id}', // CPY-051
    back: 'Back to list', // CPY-010
    cancel: 'Cancel', // CPY-011
    recheck: 'Check status again', // CPY-014
    sessionInfoTitle: 'Session information', // CPY-081
    transferTitle: 'Transfer', // CPY-082
    modeLabel: 'Mode', // CPY-053
    transportLabel: 'Viewer transport', // CPY-054
    startedLabel: 'Started at', // CPY-055
    viewerAddressLabel: 'Viewer address', // CPY-056
    targetAddressLabel: 'Server/target address', // CPY-057
    addressNote: 'Actual socket peer address.', // CPY-058
    notFound: 'The current session was not found', // CPY-059
    terminate: 'Terminate session', // CPY-060
    terminateTitle: 'Confirm session termination', // CPY-061
    terminateBody: 'The connection for session {session_id} will close. The relay between the Viewer and the Server or Mode I target will stop. A closed session does not resume.', // CPY-062
    terminateConfirm: 'Confirm termination', // CPY-063
    terminating: 'Terminating…', // CPY-064
    terminateFailed: 'Session termination failed. Check the status again.', // CPY-065
    terminateUnknown: 'The termination result cannot be confirmed. The session may have ended. The request is not sent again automatically. Check the status again.', // CPY-066
    invalidSessionId: 'Invalid session ID', // CPY-067
    terminateRejected: 'The request was not processed. Check the screen, then request termination again.', // CPY-068
  },
};
