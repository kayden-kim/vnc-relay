// Copy for the operations overview (SCR-001). Each entry names its CPY number from dashboard.md §9.3.
// Values reach the page only through textContent or attributes. Tokens such as {time} are filled by fill().

export const BRAND_NAME = 'VNC Relay'; // CPY-003
export const LANGUAGE_LABEL = '언어 (Language)'; // CPY-001
// Each language is named in its own language.
export const LANGUAGE_NAMES = { ko: '한국어', en: 'English' }; // CPY-002

export const COPY = {
  ko: {
    documentTitle: 'VNC Relay 운영 현황',
    navLabel: '주 메뉴', // Q3: tentative label, not yet in dashboard.md §9.3.1.
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
    limit: '상한 {n}', // Implementation choice: the limit text of a gauge.
    limitUnknown: '상한 확인 불가', // Implementation choice: shown when the limit is not loaded yet.
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
    },
    directionViewerToTarget: 'Viewer->대상',
    directionTargetToViewer: '대상->Viewer',
    peerViewer: 'Viewer',
    peerTarget: '대상',
    dayUnit: '일',
  },
  en: {
    documentTitle: 'VNC Relay Operations overview',
    navLabel: 'Main menu', // Q3: tentative label, not yet in dashboard.md §9.3.1.
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
    limit: 'Limit {n}', // Implementation choice: the limit text of a gauge.
    limitUnknown: 'Limit unavailable', // Implementation choice: shown when the limit is not loaded yet.
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
    },
    directionViewerToTarget: 'Viewer->target',
    directionTargetToViewer: 'target->Viewer',
    peerViewer: 'Viewer',
    peerTarget: 'Target',
    dayUnit: 'd',
  },
};
