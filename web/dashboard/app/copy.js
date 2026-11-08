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
    settingsApplyTiming: '새 연결부터 적용됩니다. 기존 연결·이미 시작한 타이머·이미 설정한 소켓 옵션은 이전 값을 유지합니다.', // CPY-100
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
    settingsApplyTiming: 'Changes apply from new connections. Existing connections, started timers, and configured socket options keep their previous values.', // CPY-100
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
  },
};
