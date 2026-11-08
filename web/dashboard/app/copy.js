// Copy for protected dashboard screens. Each entry names its CPY number from dashboard.md §9.3.
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
    activeColumns: ['세션', '모드', '전송 경로(Viewer → 대상)', '시작', 'Viewer->대상 / 대상->Viewer', '상대 주소', '상세'], // CPY-187 (replaces CPY-040)
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
    addressNote: '주소는 소켓 상대 주소입니다. 신뢰 프록시를 거쳤으면 전달 주소를 따로 표시합니다.', // CPY-188 (replaces CPY-058)
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
    // SCR-004·005 settings.
    settings: '설정', // CPY-086
    settingsTitle: '설정 보기', // CPY-087
    settingsEditTitle: '동적 설정 편집', // CPY-088
    edit: '편집', // CPY-089
    reviewChanges: '변경 내용 확인', // CPY-090
    apply: '적용', // CPY-091
    configInEffect: '설정 파일과 실행 중인 설정이 같습니다.', // CPY-092
    configPending: '설정 파일에 재시작 없이 적용할 차이가 있습니다. 표시 값은 실행 중인 설정입니다.', // CPY-093
    configRestartRequired: '설정 파일에 재시작이 필요한 차이가 있어 편집할 수 없습니다. 파일을 수정하거나 서버를 재시작한 뒤 다시 불러오십시오.', // CPY-094
    configInvalid: '설정 파일의 검증에 실패해 편집할 수 없습니다. 진단을 확인하고 파일을 수정한 뒤 다시 불러오십시오.', // CPY-095
    configUnreadable: '설정 파일을 읽지 못해 편집할 수 없습니다. 파일 접근을 복구한 뒤 다시 불러오십시오.', // CPY-096
    configEditDisabled: '편집 기능이 꺼져 있습니다. 설정 파일에서 management.config_edit = true로 바꾸고 서버를 재시작하십시오.', // CPY-097
    noReloadHistory: '이 프로세스의 재로드 이력이 없습니다.', // CPY-098
    settingsApplied: '설정 {n}개를 적용했습니다.', // CPY-099
    settingsApplyTiming: '바뀐 값은 키별 다음 검사·요청부터 적용됩니다. 이미 끝난 검사·시작한 타이머·설정한 소켓 옵션은 이전 결과를 유지합니다. 기존 중계 세션은 계속 동작합니다.', // CPY-189 (replaces CPY-100)
    settingsConfirmTitle: '설정 변경 확인', // CPY-101
    settingsComparisonLabels: ['이전 값', '새 값'], // CPY-102
    addedItems: '추가한 항목', // CPY-103
    removedItems: '뺀 항목', // CPY-104
    emptyAllowWarning: '빈 허용 목록은 해당 키가 검사하는 모든 새 접속·요청을 거절합니다.', // CPY-105
    noSettingsChanges: '바뀐 설정이 없습니다.', // CPY-106
    settingsSaving: '설정을 저장하고 적용하는 중입니다.', // CPY-107
    settingsConflict: '그사이 설정 파일이 바뀌었습니다. 다시 불러오면 입력 내용을 버리고 현재 설정으로 바꿉니다.', // CPY-108
    settingsWriteFailed: '설정 파일을 쓰지 못했습니다. 파일과 실행 값은 바뀌지 않았습니다. 입력을 확인한 뒤 다시 시도하십시오.', // CPY-109
    settingsUnknown: '적용 결과를 확인할 수 없습니다. 요청을 자동으로 다시 보내지 않습니다. 설정 보기에서 현재 값을 확인하십시오.', // CPY-110
    reloadSettings: '다시 불러오기', // CPY-111
    integerRequired: '정수를 입력하십시오.', // CPY-112
    integerRange: '{min} 이상 {max} 이하로 입력하십시오.', // CPY-113
    waitingLimitError: 'limits.waiting은 limits.relay_connections 이하여야 합니다.', // CPY-114
    listWhitespaceError: '{line}줄: 항목에 공백을 넣지 마십시오.', // CPY-115
    settingsValidationFailed: '입력한 설정의 검증에 실패했습니다. 표시한 입력란을 확인하십시오.', // CPY-116
    configLineDiagnostic: '{line}줄: {diagnostic}', // CPY-117
    settingsRejected: '요청을 처리하지 못했습니다. 화면을 다시 불러오십시오. 입력 내용은 버립니다.', // CPY-118
    configListHelp: '한 줄에 항목 하나를 입력하십시오. 빈 줄은 무시합니다. 공백은 넣지 마십시오.', // CPY-119
    configDynamic: '재시작 없이 적용', // CPY-120
    configRestart: '재시작 필요', // CPY-121
    configDiffers: '파일과 다름', // CPY-122
    lastReloadTitle: '마지막 재로드', // CPY-123
    reloadResults: { applied: '적용', rejected: '거절', restart_required: '재시작 필요' }, // CPY-124
    reloadTriggers: { file_watch: '파일 변경 감지', config_edit: '설정 편집' }, // CPY-125
    emptyList: '빈 목록', // CPY-126
    noValue: '값 없음', // CPY-127
    settingsRetrieved: '조회 시각: {time}', // CPY-128
    configColumns: ['키', '값', '적용 구분'], // CPY-129
    sourceAllowLists: '출발지 IP 허용 목록', // CPY-130
    accessPolicies: '접근 정책', // CPY-131
    connectionLimits: '연결 상한', // CPY-132
    timeouts: '시간 제한', // CPY-133
    keepalive: '연결 유지', // CPY-134
    configUnits: { seconds: '초', bytes: '바이트', count: '개' }, // CPY-135
    configRangeHelp: '범위: {min}~{max}', // CPY-136
    settingsErrorSummary: '입력 오류가 {n}개 있습니다. 첫 오류로 이동합니다.', // CPY-137
    reloadLabels: ['시각', '결과', '계기', '바뀐 키'], // CPY-138
    configFileTitle: '설정 파일 상태', // CPY-139
    configDiagnostics: {
      config_missing_key: '필수 설정 값이 없습니다.', // CPY-140
      config_type_mismatch: '설정 값의 자료형이 맞지 않습니다.', // CPY-141
      config_value_out_of_range: '설정 값이 허용 범위를 벗어났습니다.', // CPY-142
      config_relation_violated: '설정 키 사이의 관계를 만족하지 않습니다.', // CPY-143
      config_ip_item_invalid: 'IP·CIDR·범위 항목의 형식이 올바르지 않습니다.', // CPY-144
      config_ip_range_reversed: 'IP 범위의 시작 주소는 끝 주소 이하여야 합니다.', // CPY-145
      config_ip_range_family_mixed: 'IP 범위의 두 주소는 모두 IPv4이거나 모두 IPv6여야 합니다.', // CPY-146
      config_origin_invalid: 'Origin 항목의 형식이 올바르지 않습니다.', // CPY-147
      config_target_item_invalid: 'Mode I 대상 항목의 형식이 올바르지 않습니다.', // CPY-148
    },
    // SCR-002 labels & link
    targetTransportLabel: 'Server/대상 전송 경로', // CPY-179
    viewerForwardedAddressLabel: 'Viewer 전달 주소', // CPY-180
    targetForwardedAddressLabel: 'Server/대상 전달 주소', // CPY-181
    viewLogs: '로그 보기', // CPY-182
    // SCR-005 warnings & help
    emptyTrustedProxiesWarning: '빈 trusted_proxies 목록은 전달 헤더를 신뢰하지 않습니다. 소켓 상대 IP로 클라이언트 주소를 검사합니다.', // CPY-183
    emptyAllowedOriginsWarning: '빈 allowed_origins 목록은 Origin이 있는 요청을 거절합니다. Origin이 없는 요청은 IP 검사만 합니다.', // CPY-184
    trustedProxiesHelp: 'trusted_proxies에는 프록시 주소만 입력하십시오. 클라이언트 범위는 넣지 마십시오. 전달 주소가 모두 신뢰 프록시에 속하면 요청을 거절합니다.', // CPY-185
    // SCR-006 logs
    logs: '로그', // CPY-149
    retainedCount: '보관 {n} / 최대 {max}', // CPY-150
    levelLabel: '수준', // CPY-151
    eventNameLabel: '사건 이름', // CPY-152
    allEvents: '모든 사건', // CPY-153
    searchLabel: '값 검색', // CPY-154
    search: '검색', // CPY-155
    searchHelp: '1~256자를 입력한 뒤 Enter를 누르거나 검색을 선택하십시오. 빈 값으로 검색하면 검색 조건을 해제합니다. 필드 값에서 ASCII 대소문자를 구분하지 않고 찾습니다.', // CPY-156
    sessionFilterChip: '세션 {session_id}', // CPY-157
    clearFilter: '필터 해제', // CPY-158
    autoRefresh: '자동 갱신', // CPY-159
    loadOlderLogs: '이전 로그 더 보기', // CPY-160
    downloadJsonLines: 'JSON Lines 다운로드', // CPY-161
    logColumns: ['시각', '수준', '사건', '요약', '자세히'], // CPY-162
    showDetails: '자세히', // CPY-163
    hideDetails: '접기', // CPY-164
    searchLengthError: '검색어는 1~256자로 입력하십시오. 검색을 해제하려면 빈 값으로 검색하십시오.', // CPY-165
    logsEmpty: '필터와 일치하는 최근 로그가 없습니다.', // CPY-166
    logsDisabled: '최근 로그를 보관하지 않습니다. 보관하려면 설정 파일의 recent_logs.max_entries를 1~10000으로 바꾸고 서버를 재시작하십시오.', // CPY-167
    logsGapNotice: '조회하기 전에 일부 로그가 보관 목록에서 제거됐습니다. 표시한 목록은 전체 이력이 아닙니다.', // CPY-168
    logsPausedNotice: '자동 갱신을 일시정지했습니다. 새로고침과 필터는 계속 사용할 수 있습니다.', // CPY-169
    autoRefreshOn: '켜짐', // CPY-170
    autoRefreshOff: '꺼짐', // CPY-171
    logFiltersTitle: '로그 필터', // CPY-172
    levelRequiredError: '수준을 하나 이상 선택하십시오.', // CPY-173
    downloadCollecting: '필터와 일치하는 로그 전체를 모으는 중입니다.', // CPY-174
    downloadFailed: '로그 다운로드에 실패했습니다. 부분 파일은 저장하지 않았습니다. 다시 시도하십시오.', // CPY-175
    logsInvalidRequest: '로그 조회 요청을 처리하지 못했습니다. 필터를 확인한 뒤 다시 조회하십시오.', // CPY-176
    logsMethodNotAllowed: '로그 조회 방식이 맞지 않습니다. 화면을 다시 불러오십시오.', // CPY-177
    recentLogsTitle: '최근 로그', // CPY-178
    sessionFilterNote: '보관 중인 로그만 검색합니다. 세션 시작 로그가 제거됐으면 연결별 로그 일부는 포함되지 않을 수 있습니다.', // CPY-186
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
    activeColumns: ['Session', 'Mode', 'Transport (Viewer → target)', 'Started', 'Viewer->target / target->Viewer', 'Peer address', 'Details'], // CPY-187 (replaces CPY-040)
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
    addressNote: 'Addresses are socket peer addresses. Forwarded addresses are shown separately when a trusted proxy is used.', // CPY-188 (replaces CPY-058)
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
    // SCR-004·005 settings.
    settings: 'Settings', // CPY-086
    settingsTitle: 'View settings', // CPY-087
    settingsEditTitle: 'Edit dynamic settings', // CPY-088
    edit: 'Edit', // CPY-089
    reviewChanges: 'Review changes', // CPY-090
    apply: 'Apply', // CPY-091
    configInEffect: 'The configuration file matches the settings in effect.', // CPY-092
    configPending: 'The configuration file has changes that can be applied without a restart. The values shown are in effect now.', // CPY-093
    configRestartRequired: 'The configuration file has changes that require a restart. Editing is blocked. Update the file or restart the server, then reload.', // CPY-094
    configInvalid: 'The configuration file failed validation. Editing is blocked. Check the diagnostics, update the file, then reload.', // CPY-095
    configUnreadable: 'The configuration file could not be read. Editing is blocked. Restore file access, then reload.', // CPY-096
    configEditDisabled: 'Editing is disabled. Set management.config_edit = true in the configuration file and restart the server.', // CPY-097
    noReloadHistory: 'This process has no reload history.', // CPY-098
    settingsApplied: 'Applied {n} settings.', // CPY-099
    settingsApplyTiming: 'Changed values apply from the next check or request for each key. Completed checks, started timers, and configured socket options keep their previous results. Existing relay sessions continue.', // CPY-189 (replaces CPY-100)
    settingsConfirmTitle: 'Confirm settings changes', // CPY-101
    settingsComparisonLabels: ['Previous value', 'New value'], // CPY-102
    addedItems: 'Added items', // CPY-103
    removedItems: 'Removed items', // CPY-104
    emptyAllowWarning: 'An empty allow list rejects all new connections or requests checked by this key.', // CPY-105
    noSettingsChanges: 'No settings have changed.', // CPY-106
    settingsSaving: 'Saving and applying settings.', // CPY-107
    settingsConflict: 'The configuration file has changed. Reloading discards your input and loads the current settings.', // CPY-108
    settingsWriteFailed: 'The configuration file could not be written. The file and settings in effect are unchanged. Review your input and try again.', // CPY-109
    settingsUnknown: 'The apply result cannot be confirmed. The request is not sent again automatically. Check the current values in View settings.', // CPY-110
    reloadSettings: 'Reload settings', // CPY-111
    integerRequired: 'Enter an integer.', // CPY-112
    integerRange: 'Enter a value from {min} to {max}.', // CPY-113
    waitingLimitError: 'limits.waiting must not exceed limits.relay_connections.', // CPY-114
    listWhitespaceError: 'Line {line}: do not include whitespace in an item.', // CPY-115
    settingsValidationFailed: 'The entered settings failed validation. Check the marked fields.', // CPY-116
    configLineDiagnostic: 'Line {line}: {diagnostic}', // CPY-117
    settingsRejected: 'The request could not be processed. Reload this screen. Your input will be discarded.', // CPY-118
    configListHelp: 'Enter one item per line. Empty lines are ignored. Do not include whitespace.', // CPY-119
    configDynamic: 'Applies without restart', // CPY-120
    configRestart: 'Restart required', // CPY-121
    configDiffers: 'Differs from file', // CPY-122
    lastReloadTitle: 'Last reload', // CPY-123
    reloadResults: { applied: 'Applied', rejected: 'Rejected', restart_required: 'Restart required' }, // CPY-124
    reloadTriggers: { file_watch: 'File watch', config_edit: 'Configuration edit' }, // CPY-125
    emptyList: 'Empty list', // CPY-126
    noValue: 'No value', // CPY-127
    settingsRetrieved: 'Retrieved at: {time}', // CPY-128
    configColumns: ['Key', 'Value', 'Application'], // CPY-129
    sourceAllowLists: 'Source IP allow lists', // CPY-130
    accessPolicies: 'Access policies', // CPY-131
    connectionLimits: 'Connection limits', // CPY-132
    timeouts: 'Timeouts', // CPY-133
    keepalive: 'Keepalive', // CPY-134
    configUnits: { seconds: 'seconds', bytes: 'bytes', count: 'count' }, // CPY-135
    configRangeHelp: 'Range: {min}–{max}', // CPY-136
    settingsErrorSummary: 'There are {n} input errors. Focus moves to the first error.', // CPY-137
    reloadLabels: ['Time', 'Result', 'Trigger', 'Changed keys'], // CPY-138
    configFileTitle: 'Configuration file status', // CPY-139
    configDiagnostics: {
      config_missing_key: 'A required setting value is missing.', // CPY-140
      config_type_mismatch: 'The setting value has an invalid type.', // CPY-141
      config_value_out_of_range: 'The setting value is outside its allowed range.', // CPY-142
      config_relation_violated: 'The relationship between setting keys is not valid.', // CPY-143
      config_ip_item_invalid: 'The IP, CIDR, or range item has an invalid format.', // CPY-144
      config_ip_range_reversed: 'The start address of an IP range must not exceed its end address.', // CPY-145
      config_ip_range_family_mixed: 'Both addresses in an IP range must be IPv4 or both must be IPv6.', // CPY-146
      config_origin_invalid: 'The Origin item has an invalid format.', // CPY-147
      config_target_item_invalid: 'The Mode I target item has an invalid format.', // CPY-148
    },
    // SCR-002 labels & link
    targetTransportLabel: 'Server/target transport', // CPY-179
    viewerForwardedAddressLabel: 'Viewer forwarded address', // CPY-180
    targetForwardedAddressLabel: 'Server/target forwarded address', // CPY-181
    viewLogs: 'View logs', // CPY-182
    // SCR-005 warnings & help
    emptyTrustedProxiesWarning: 'An empty trusted_proxies list does not trust forwarded headers. The socket peer IP is used as the client address.', // CPY-183
    emptyAllowedOriginsWarning: 'An empty allowed_origins list rejects requests with Origin. Requests without Origin use only the IP check.', // CPY-184
    trustedProxiesHelp: 'Enter only proxy addresses in trusted_proxies. Do not include client ranges. A request is rejected when all forwarded addresses belong to trusted proxies.', // CPY-185
    // SCR-006 logs
    logs: 'Logs', // CPY-149
    retainedCount: 'Retained {n} / Maximum {max}', // CPY-150
    levelLabel: 'Level', // CPY-151
    eventNameLabel: 'Event name', // CPY-152
    allEvents: 'All events', // CPY-153
    searchLabel: 'Search values', // CPY-154
    search: 'Search', // CPY-155
    searchHelp: 'Enter 1–256 characters, then press Enter or select Search. Searching with an empty value clears the search. Field values are matched without ASCII case sensitivity.', // CPY-156
    sessionFilterChip: 'Session {session_id}', // CPY-157
    clearFilter: 'Clear filter', // CPY-158
    autoRefresh: 'Auto refresh', // CPY-159
    loadOlderLogs: 'Load older logs', // CPY-160
    downloadJsonLines: 'Download JSON Lines', // CPY-161
    logColumns: ['Time', 'Level', 'Event', 'Summary', 'Details'], // CPY-162
    showDetails: 'Show details', // CPY-163
    hideDetails: 'Hide details', // CPY-164
    searchLengthError: 'Enter a search value of 1–256 characters. To clear the search, search with an empty value.', // CPY-165
    logsEmpty: 'No recent logs match the filters.', // CPY-166
    logsDisabled: 'Recent logs are not retained. To retain them, set recent_logs.max_entries to 1–10000 in the configuration file and restart the server.', // CPY-167
    logsGapNotice: 'Some logs were removed from the retained list before they could be fetched. The displayed list is not a complete history.', // CPY-168
    logsPausedNotice: 'Auto refresh is paused. Refresh and filters are still available.', // CPY-169
    autoRefreshOn: 'On', // CPY-170
    autoRefreshOff: 'Off', // CPY-171
    logFiltersTitle: 'Log filters', // CPY-172
    levelRequiredError: 'Select at least one level.', // CPY-173
    downloadCollecting: 'Collecting all logs that match the filters.', // CPY-174
    downloadFailed: 'The log download failed. No partial file was saved. Try again.', // CPY-175
    logsInvalidRequest: 'The log query could not be processed. Check the filters and query again.', // CPY-176
    logsMethodNotAllowed: 'The log query method is not supported. Reload this screen.', // CPY-177
    recentLogsTitle: 'Recent logs', // CPY-178
    sessionFilterNote: 'Only retained logs are searched. Some connection logs may be excluded if the session start log has been removed.', // CPY-186
  },
};
