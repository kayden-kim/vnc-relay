// Copy for protected dashboard screens. Each entry names its CPY number from dashboard.md §9.3.
// Values reach the page only through textContent or attributes. Tokens such as {time} are filled by fill().

export const BRAND_NAME = 'VNC Relay'; // CPY-003
export const LANGUAGE_LABEL = '언어 (Language)'; // CPY-001
// Each language is named in its own language.
export const LANGUAGE_NAMES = { ko: '한국어', en: 'English' }; // CPY-002
// Provider attributions of the IP information databases. They are the providers' license wording, so both languages
// use the same English text. CPY-278 links only its address part.
export const PROVIDER_ATTRIBUTION = {
  dbip: { text: 'IP Geolocation by DB-IP', href: 'https://db-ip.com' }, // CPY-276
  ipinfo: { text: 'IP address data powered by IPinfo', href: 'https://ipinfo.io' }, // CPY-277
  maxmind_geolite: { // CPY-278
    text: 'This product includes GeoLite Data created by MaxMind, available from https://www.maxmind.com.',
    href: 'https://www.maxmind.com',
  },
};

export const COPY = {
  ko: {
    navLabel: '주 메뉴', // CPY-078
    help: '도움말', // CPY-195
    opensInNewTab: '새 탭에서 열림', // CPY-196
    operations: '운영 현황', // CPY-004
    navOperations: '운영 현황', // CPY-294
    signOut: '로그아웃', // CPY-007
    refresh: '새로고침', // CPY-008
    details: '상세', // CPY-009
    lastSuccess: '마지막 성공 갱신: {time}', // CPY-015
    loading: '조회 중', // CPY-016
    refreshing: '갱신 중', // CPY-017
    failed: '조회 실패', // CPY-018
    stale: '오래된 데이터', // CPY-019
    staleMessage: '조회에 실패했습니다. 아래 값은 이전 데이터입니다.', // CPY-020
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
    waitingColumns: ['역할', '중계 ID', '전송 경로', '상대 주소', '접속 시각', '대기 시간'], // CPY-197 (replaces CPY-037)
    forwardedAddressLabel: '전달 주소', // CPY-199
    roleViewer: '뷰어', // CPY-327
    roleServer: '서버', // CPY-328
    waitingEmpty: '대기 없음', // CPY-038
    activeTitle: '활성 세션', // CPY-039
    activeColumns: ['세션', '모드', '전송 경로(뷰어 → 서버)', '시작', '뷰어 -> 서버 / 서버 -> 뷰어', '상대 주소', '상세'], // CPY-187 (replaces CPY-040)
    activeEmpty: '활성 없음', // CPY-041
    closedTitle: '최근 종료 목록', // CPY-042
    closedColumns: ['세션', '종료 원인', '모드', '전송 경로(뷰어 → 서버)', '시각', '뷰어 -> 서버 / 서버 -> 뷰어', '상대 주소', '로그'], // CPY-198 (replaces CPY-043)
    closedAtLabel: '종료 시각', // CPY-200
    durationLabel: '지속 시간', // CPY-201
    closedEmpty: '최근 종료 없음', // CPY-044
    closeCause: {
      peer_eof: '정상 EOF', // CPY-045
      connection_error: '연결 오류', // CPY-046
      admin_terminated: '관리자 종료', // CPY-047
      dead_connection: '끊긴 연결 감지', // CPY-048
      close_timeout: '정리 제한 시간 초과', // CPY-049
      server_shutdown: '서버 종료', // CPY-050
      internal_error: '내부 오류', // CPY-083
      session_expired: '세션 만료', // CPY-190
      ip_denied: '차단 목록으로 종료', // CPY-323
    },
    directionViewerToTarget: '뷰어 -> 서버', // CPY-325: table lines, chart names (CPY-218) and the SCR-002 transfer label.
    directionTargetToViewer: '서버 -> 뷰어', // CPY-326: table lines, chart names (CPY-218) and the SCR-002 transfer label.
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
    transportLabel: '뷰어 전송 경로', // CPY-054
    startedLabel: '시작 시각', // CPY-055
    viewerAddressLabel: '뷰어 주소', // CPY-056
    targetAddressLabel: '서버 주소', // CPY-057
    addressNote: '주소는 소켓 상대 주소입니다. 신뢰 프록시를 거쳤으면 전달 주소를 따로 표시합니다.', // CPY-188 (replaces CPY-058)
    notFound: '현재 세션을 찾을 수 없음', // CPY-059
    terminate: '세션 종료', // CPY-060
    terminateTitle: '세션 종료 확인', // CPY-061
    terminateBody: '세션 {session_id}의 연결을 종료합니다. 뷰어와 서버 또는 Mode I 대상 사이의 중계가 멈춥니다. 종료한 세션은 다시 이어지지 않습니다.', // CPY-062
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
      config_deny_overlaps_proxy: '차단 항목이 신뢰 프록시 주소와 겹칩니다. 프록시 주소는 차단할 수 없습니다.', // CPY-322
    },
    // SCR-002 labels & link
    targetTransportLabel: '서버 전송 경로', // CPY-179
    viewerForwardedAddressLabel: '뷰어 전달 주소', // CPY-180
    targetForwardedAddressLabel: '서버 전달 주소', // CPY-181
    viewLogs: '로그 보기', // CPY-182
    // SCR-005 warnings & help
    emptyTrustedProxiesWarning: '빈 trusted_proxies 목록은 전달 헤더를 신뢰하지 않습니다. 소켓 상대 IP로 클라이언트 주소를 검사합니다.', // CPY-183
    emptyAllowedOriginsWarning: '빈 allowed_origins 목록은 Origin이 있는 요청을 거절합니다. Origin이 없는 요청은 IP 검사를 하고, 그 역할에 접속 ticket 확인을 켰으면 ticket도 확인합니다.', // CPY-192 (replaces CPY-184)
    trustedProxiesHelp: 'trusted_proxies에는 프록시 주소만 입력하십시오. 클라이언트 범위는 넣지 마십시오. 전달 주소가 모두 신뢰 프록시에 속하면 요청을 거절합니다.', // CPY-185
    accessTickets: '접속 ticket', // CPY-191
    stringLengthError: '1바이트 이상 256바이트 이하로 입력하십시오.', // CPY-193
    enableTicketWarning: '접속 ticket 확인을 켜면 이 역할의 WS·WSS 업그레이드 요청 가운데 유효한 ticket이 없는 요청을 거절합니다. 기존 중계 세션은 계속 동작합니다.', // CPY-194
    denyListName: '차단 목록', // CPY-319
    denyListHelp: '허용 목록보다 우선합니다. 저장하면 이 목록에 해당하는 주소의 열린 세션과 대기 연결을 바로 끊습니다. 빈 목록은 아무것도 차단하지 않습니다.', // CPY-320
    denyChangeWarning: '예외: 차단 목록에 새로 넣거나 바꾼 항목에 해당하는 주소의 열린 세션과 대기 연결은 저장 직후 끊깁니다.', // CPY-321
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
    // SCR-007 metrics
    metrics: '메트릭', // CPY-202
    metricsRange: '추이: 최근 1시간, 10초 간격 · 점 {n}/360개', // CPY-203
    kpiTitle: '핵심 지표', // CPY-204
    throughput: '처리량', // CPY-205
    perSecond: '{value}/초', // CPY-206
    rejectedTotal: '거절한 연결(시작 뒤 누적)', // CPY-207
    cpuUsage: 'CPU 사용률', // CPY-208
    cpuNote: '직전 10초 동안 쓴 CPU 시간의 비율입니다. 100%는 코어 1개를 모두 쓴 양입니다.', // CPY-209
    memoryUsage: '메모리 사용량', // CPY-210
    memoryNote: '물리 메모리 사용량입니다. Linux는 RSS, Windows는 작업 집합입니다.', // CPY-211
    collecting: '수집 중', // CPY-212
    trendsTitle: '추이(최근 1시간)', // CPY-213
    chartSessions: '활성 세션·대기 연결', // CPY-214
    chartConnections: '연결(분당)', // CPY-215
    connectionSeries: ['수락', '거절', '중단'], // CPY-216 (accepted / rejected / aborted)
    chartThroughput: '처리량(초당)', // CPY-217
    legendValue: '현재 {current} · 최대 {max}', // CPY-219
    chartEmpty: '추이를 모으는 중입니다. 10초마다 점이 하나씩 쌓입니다.', // CPY-220
    distTitle: '시간 분포', // CPY-221
    distColumns: ['항목', '속성', '개수', '평균', 'p95 상한', '분포'], // CPY-222
    distNames: ['초기 교환', '대기', '세션', 'Mode I 대상 연결'], // CPY-223
    p95Le: '≤ {value}', // CPY-224
    p95Gt: '> {value}', // CPY-224
    distNote: 'p95 상한은 기록한 값의 95%가 들어가는 첫 구간의 경계입니다. 정확한 백분위수가 아닙니다.', // CPY-225
    causeTitle: '원인별 수(시작 뒤 누적)', // CPY-226
    causeTables: ['연결 거절', '연결 중단', '세션 종료', '관리 활동', '설정 변경', 'WebSocket'], // CPY-227
    causeConnectionColumns: ['리스너', '단계', '원인', '수'], // CPY-228
    causeClosedColumns: ['모드', '종료 원인', '수'], // CPY-229
    causeAdminColumns: ['항목', '결과·범위', '수'], // CPY-230
    adminItems: ['로그인', '로그인 제한', '관리자 종료'], // CPY-231
    causeConfigColumns: ['항목', '결과', '계기·원인', '수'], // CPY-232
    configItems: ['재로드', '편집'], // CPY-233
    causeWebSocketColumns: ['항목', '리스너', '역할', '수'], // CPY-234
    webSocketItems: ['ping 시간 초과', '접속 ticket 수락'], // CPY-235
    noRecords: '기록 없음', // CPY-236
    processTitle: '프로세스', // CPY-237
    cpuTime: 'CPU 시간(사용자 / 시스템)', // CPY-238
    virtualMemory: '가상 메모리', // CPY-239
    threadCount: '스레드 수', // CPY-240
    handleCount: '열린 핸들 수', // CPY-241
    descriptorCount: '열린 파일 기술자 수', // CPY-242
    version: '버전', // CPY-243
    externalNote: '외부 수집 도구는 상태 리스너의 /metrics에서 같은 값을 읽습니다. 켜려면 설정 파일에서 [metrics] enabled = true로 바꾸고 서버를 재시작하십시오.', // CPY-244
    metricsRejected: '메트릭 조회 요청을 처리하지 못했습니다. 새로고침으로 다시 조회하십시오.', // CPY-245
    secondsUnit: '{n}초', // CPY-246
    distBarName: '구간별 개수: {items}', // CPY-247
    distBarItem: '{bound} {count}개', // CPY-247 (one non-zero bucket)
    periodLabel: '추이 기간', // CPY-248
    periodHour: '최근 1시간', // CPY-249
    periodDay: '최근 24시간', // CPY-250
    metricsRange24h: '추이: 최근 24시간, 1분 간격 · 점 {n}/1440개', // CPY-251
    trendsTitle24h: '추이(최근 24시간)', // CPY-252
    chartEmpty24h: '추이를 모으는 중입니다. 1분마다 점이 하나씩 쌓입니다.', // CPY-253
    cpuNote24h: '직전 1분 동안 쓴 CPU 시간의 비율입니다. 100%는 코어 1개를 모두 쓴 양입니다.', // CPY-254
    sources: '접속 출처', // CPY-255
    sourcesPeriodLabel: '집계 기간', // CPY-256
    sourcesRange: '집계: {start} ~ {end} · 보관 사건 {stored}/{capacity}개', // CPY-257
    sourcesDropped: '보관 상한에 이르러 {time}까지의 사건을 버렸습니다. 이 기간의 앞부분은 수에 들어 있지 않습니다.', // CPY-258
    sourcesSummaryTitle: '요약', // CPY-259
    sourcesSummaryNames: ['연결 시도', '거절', '중단', '시작한 세션', '주소', '국가', 'ASN'], // CPY-260
    sourcesSummaryNote: '연결 시도는 중계 리스너가 받아들인 연결과 수락 단계에서 거절한 연결의 수입니다. 거절과 중단은 모든 단계에서 센 수입니다.', // CPY-261
    ipDbTitle: 'IP 정보 DB', // CPY-262
    ipDbNames: ['국가 DB', 'ASN DB'], // CPY-263
    ipDbNotConfigured: '설정 안 됨', // CPY-264
    ipDbLoaded: '사용 중', // CPY-265
    ipDbStale: '이전 DB 사용 중', // CPY-266
    ipDbUnavailable: '읽기 실패', // CPY-267
    ipDbInfoLabels: ['종류', '빌드 시각', '읽은 시각'], // CPY-268
    ipDbNotConfiguredNote: '설정 파일의 {key}에 DB 파일 경로를 넣고 서버를 재시작하면 이 정보를 찾습니다.', // CPY-269
    ipDbStaleNote: '파일을 다시 읽지 못해 이전 DB를 계속 씁니다. 원인: {reason}. 파일을 고치면 자동으로 다시 읽습니다.', // CPY-270
    ipDbUnavailableNote: 'DB 파일을 읽지 못했습니다. 원인: {reason}. 파일을 고치면 자동으로 다시 읽습니다.', // CPY-271
    ipDbFileUnreadable: '파일이 없거나 읽을 수 없음', // CPY-272
    ipDbFileTooLarge: '파일이 256 MiB보다 큼', // CPY-273
    ipDbInvalidFormat: 'MMDB 파일이 아니거나 검사에 실패함', // CPY-274
    ipDbUnsupportedData: '이 DB에 필요한 국가 또는 ASN 값이 없음', // CPY-275
    addressTitle: '주소별', // CPY-279
    addressColumns: ['주소', '국가', 'ASN', '연결 시도', '거절', '중단', '세션', '마지막 사건', '차단 상태'], // CPY-280·295
    clientAddressNote: '클라이언트 주소는 신뢰 프록시가 전달한 주소가 있으면 그 주소이고, 없으면 소켓 상대 주소입니다. 세션 열은 그 주소가 뷰어 또는 Mode II 서버로 참여한 세션 수입니다.', // CPY-281
    localAddress: '로컬', // CPY-282
    localAddressNote: '로컬 주소(루프백·사설·링크 로컬·공유 주소)는 IP 정보 DB에서 찾지 않고 조회 링크도 없습니다.', // CPY-283
    countryTitle: '국가별', // CPY-284
    countryColumns: ['국가', '주소', '연결 시도', '거절', '중단', '세션'], // CPY-285
    asnTitle: 'ASN별', // CPY-286
    asnColumns: ['ASN', '이름', '주소', '연결 시도', '거절', '중단', '세션'], // CPY-287
    unknownValue: '알 수 없음', // CPY-288
    partialCount: '전체 {total}개 중 상위 {shown}개', // CPY-289
    sourcesEmpty: '이 기간에 기록한 연결이 없습니다.', // CPY-290
    lookupName: 'ipinfo.io에서 {value} 조회', // CPY-291
    lookupNote: '조회 링크는 ipinfo.io를 새 탭에서 엽니다. 링크를 고르면 그 주소나 ASN이 ipinfo.io에 전달됩니다.', // CPY-292
    sourcesRejected: '접속 출처 조회 요청을 처리하지 못했습니다. 새로고침으로 다시 조회하십시오.', // CPY-293
    // SCR-008 address block and unblock (CPY-296~318, 324). {address} and {diagnostic} are filled by fill().
    denyBlock: '차단', // CPY-296
    denyBlockName: '{address} 차단', // CPY-297
    denyUnblock: '차단 해제', // CPY-298
    denyUnblockName: '{address} 차단 해제', // CPY-299
    denyBlocked: '차단됨', // CPY-300
    denyRangeNote: '범위 항목으로 차단되어 있습니다. 설정에서 해제하십시오.', // CPY-301
    denyEditOff: '차단하려면 설정 파일에서 management.config_edit = true로 바꾸고 서버를 재시작하십시오.', // CPY-302
    denyBlockTitle: '주소 차단 확인', // CPY-303
    denyBlockLabel: '차단할 주소', // CPY-304
    denyBlockBody: '이 주소의 새 접속을 모든 중계 진입점에서 거절하고, 열린 세션과 대기 연결을 바로 끊습니다.', // CPY-305
    denyBlocking: '차단 중…', // CPY-306
    denyUnblockTitle: '차단 해제 확인', // CPY-307
    denyUnblockLabel: '차단을 해제할 주소', // CPY-308
    denyUnblockBody: '이 주소의 새 접속을 다시 허용 목록으로 판정합니다. 끊은 연결은 되살아나지 않습니다.', // CPY-309
    denyUnblocking: '차단 해제 중…', // CPY-310
    denyBlockDone: '주소를 차단했습니다: {address}', // CPY-311
    denyUnblockDone: '차단을 해제했습니다: {address}', // CPY-312
    denyConflict: '설정 파일이 바뀌어 차단 목록을 바꾸지 못했습니다. 새로고침으로 다시 조회한 뒤 다시 시도하십시오.', // CPY-313
    denyFileState: '설정 파일 상태 때문에 차단 목록을 바꾸지 못했습니다. 설정 보기에서 파일 상태를 확인하십시오.', // CPY-314
    denyInvalid: '차단 목록을 바꾸지 못했습니다. 원인: {diagnostic}', // CPY-315
    denyRequestFailed: '요청을 처리하지 못했습니다. 화면을 다시 불러온 뒤 다시 시도하십시오.', // CPY-316
    denyUnknown: '차단 목록이 바뀌었는지 확인하지 못했습니다. 새로고침으로 다시 조회해 확인하십시오.', // CPY-317
    denyConfigReadFailed: '설정을 읽지 못해 차단 목록을 바꾸지 못했습니다. 잠시 뒤 다시 시도하십시오.', // CPY-318
    denyNothingToRemove: '차단 목록이 이미 바뀌어 지울 항목이 없습니다. 다시 조회한 결과를 확인하십시오.', // CPY-324
  },
  en: {
    navLabel: 'Main menu', // CPY-078
    help: 'Help', // CPY-195
    opensInNewTab: 'opens in a new tab', // CPY-196
    operations: 'Operations overview', // CPY-004
    navOperations: 'Overview', // CPY-294
    signOut: 'Sign out', // CPY-007
    refresh: 'Refresh', // CPY-008
    details: 'Details', // CPY-009
    lastSuccess: 'Last successful update: {time}', // CPY-015
    loading: 'Loading', // CPY-016
    refreshing: 'Updating', // CPY-017
    failed: 'Failed to load', // CPY-018
    stale: 'Stale data', // CPY-019
    staleMessage: 'Loading failed. The values below are from an earlier update.', // CPY-020
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
    waitingColumns: ['Role', 'Relay ID', 'Transport', 'Peer address', 'Connected at', 'Waiting time'], // CPY-197 (replaces CPY-037)
    forwardedAddressLabel: 'Forwarded address', // CPY-199
    roleViewer: 'Viewer', // CPY-327
    roleServer: 'Server', // CPY-328
    waitingEmpty: 'No waiting connections', // CPY-038
    activeTitle: 'Active sessions', // CPY-039
    activeColumns: ['Session', 'Mode', 'Transport (Viewer → Server)', 'Started', 'Viewer -> Server / Server -> Viewer', 'Peer address', 'Details'], // CPY-187 (replaces CPY-040)
    activeEmpty: 'No active sessions', // CPY-041
    closedTitle: 'Recently closed sessions', // CPY-042
    closedColumns: ['Session', 'Close reason', 'Mode', 'Transport (Viewer → Server)', 'Time', 'Viewer -> Server / Server -> Viewer', 'Peer address', 'Logs'], // CPY-198 (replaces CPY-043)
    closedAtLabel: 'Closed at', // CPY-200
    durationLabel: 'Duration', // CPY-201
    closedEmpty: 'No recently closed sessions', // CPY-044
    closeCause: {
      peer_eof: 'Normal EOF', // CPY-045
      connection_error: 'Connection error', // CPY-046
      admin_terminated: 'Terminated by administrator', // CPY-047
      dead_connection: 'Dead connection detected', // CPY-048
      close_timeout: 'Cleanup time limit exceeded', // CPY-049
      server_shutdown: 'Server shutdown', // CPY-050
      internal_error: 'Internal error', // CPY-083
      session_expired: 'Session expired', // CPY-190
      ip_denied: 'Closed by deny list', // CPY-323
    },
    directionViewerToTarget: 'Viewer -> Server', // CPY-325: table lines, chart names (CPY-218) and the SCR-002 transfer label.
    directionTargetToViewer: 'Server -> Viewer', // CPY-326: table lines, chart names (CPY-218) and the SCR-002 transfer label.
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
    targetAddressLabel: 'Server address', // CPY-057
    addressNote: 'Addresses are socket peer addresses. Forwarded addresses are shown separately when a trusted proxy is used.', // CPY-188 (replaces CPY-058)
    notFound: 'The current session was not found', // CPY-059
    terminate: 'Terminate session', // CPY-060
    terminateTitle: 'Confirm session termination', // CPY-061
    terminateBody: 'The connection for session {session_id} will be terminated. The relay between the Viewer and the Server or Mode I target will stop. A terminated session does not resume.', // CPY-062
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
      config_deny_overlaps_proxy: 'A deny entry overlaps a trusted proxy address. Proxy addresses cannot be blocked.', // CPY-322
    },
    // SCR-002 labels & link
    targetTransportLabel: 'Server transport', // CPY-179
    viewerForwardedAddressLabel: 'Viewer forwarded address', // CPY-180
    targetForwardedAddressLabel: 'Server forwarded address', // CPY-181
    viewLogs: 'View logs', // CPY-182
    // SCR-005 warnings & help
    emptyTrustedProxiesWarning: 'An empty trusted_proxies list does not trust forwarded headers. The socket peer IP is used as the client address.', // CPY-183
    emptyAllowedOriginsWarning: 'An empty allowed_origins list rejects requests with Origin. Requests without Origin use the IP check, and also the ticket check when access tickets are on for the role.', // CPY-192 (replaces CPY-184)
    trustedProxiesHelp: 'Enter only proxy addresses in trusted_proxies. Do not include client ranges. A request is rejected when all forwarded addresses belong to trusted proxies.', // CPY-185
    accessTickets: 'Access tickets', // CPY-191
    stringLengthError: 'Enter 1 to 256 bytes.', // CPY-193
    enableTicketWarning: "Turning on access ticket checks rejects this role's WS and WSS upgrade requests that have no valid ticket. Existing relay sessions continue.", // CPY-194
    denyListName: 'Deny list', // CPY-319
    denyListHelp: 'The deny list takes precedence over allow lists. Saving closes open sessions and waiting connections from matching addresses immediately. An empty list blocks nothing.', // CPY-320
    denyChangeWarning: 'Exception: Open sessions and waiting connections from addresses covered by new or changed deny list entries are closed immediately after saving.', // CPY-321
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
    // SCR-007 metrics
    metrics: 'Metrics', // CPY-202
    metricsRange: 'Trends: last hour, 10-second interval · Points: {n}/360', // CPY-203
    kpiTitle: 'Key indicators', // CPY-204
    throughput: 'Throughput', // CPY-205
    perSecond: '{value}/s', // CPY-206
    rejectedTotal: 'Rejected connections (since start)', // CPY-207
    cpuUsage: 'CPU usage', // CPY-208
    cpuNote: 'Share of CPU time used in the last 10 seconds. 100% equals one fully used core.', // CPY-209
    memoryUsage: 'Memory usage', // CPY-210
    memoryNote: 'Physical memory in use: RSS on Linux, working set on Windows.', // CPY-211
    collecting: 'Collecting', // CPY-212
    trendsTitle: 'Trends (last hour)', // CPY-213
    chartSessions: 'Active sessions and waiting connections', // CPY-214
    chartConnections: 'Connections (per minute)', // CPY-215
    connectionSeries: ['Accepted', 'Rejected', 'Aborted'], // CPY-216 (accepted / rejected / aborted)
    chartThroughput: 'Throughput (per second)', // CPY-217
    legendValue: 'Now {current} · Max {max}', // CPY-219
    chartEmpty: 'Collecting trend data. One point is added every 10 seconds.', // CPY-220
    distTitle: 'Duration distribution', // CPY-221
    distColumns: ['Metric', 'Attributes', 'Count', 'Mean', 'p95 upper bound', 'Distribution'], // CPY-222
    distNames: ['Initial exchange', 'Waiting', 'Session', 'Mode I target connection'], // CPY-223
    p95Le: '≤ {value}', // CPY-224
    p95Gt: '> {value}', // CPY-224
    distNote: 'The p95 upper bound is the edge of the first bucket that holds 95% of recorded values. It is not an exact percentile.', // CPY-225
    causeTitle: 'Counts by cause (since start)', // CPY-226
    causeTables: ['Rejected connections', 'Aborted connections', 'Closed sessions', 'Administration', 'Configuration changes', 'WebSocket'], // CPY-227
    causeConnectionColumns: ['Listener', 'Stage', 'Reason', 'Count'], // CPY-228
    causeClosedColumns: ['Mode', 'Close cause', 'Count'], // CPY-229
    causeAdminColumns: ['Item', 'Result or scope', 'Count'], // CPY-230
    adminItems: ['Sign-in', 'Sign-in limit', 'Administrator termination'], // CPY-231
    causeConfigColumns: ['Item', 'Result', 'Trigger or reason', 'Count'], // CPY-232
    configItems: ['Reload', 'Edit'], // CPY-233
    causeWebSocketColumns: ['Item', 'Listener', 'Role', 'Count'], // CPY-234
    webSocketItems: ['Ping timeout', 'Access ticket accepted'], // CPY-235
    noRecords: 'No records', // CPY-236
    processTitle: 'Process', // CPY-237
    cpuTime: 'CPU time (user / system)', // CPY-238
    virtualMemory: 'Virtual memory', // CPY-239
    threadCount: 'Threads', // CPY-240
    handleCount: 'Open handles', // CPY-241
    descriptorCount: 'Open file descriptors', // CPY-242
    version: 'Version', // CPY-243
    externalNote: 'External collectors read the same values from /metrics on the status listener. To turn it on, set [metrics] enabled = true in the configuration file and restart the server.', // CPY-244
    metricsRejected: 'The metrics request could not be processed. Use Refresh to query again.', // CPY-245
    secondsUnit: '{n} s', // CPY-246
    distBarName: 'Count per bucket: {items}', // CPY-247
    distBarItem: '{bound}: {count}', // CPY-247 (one non-zero bucket)
    periodLabel: 'Trend period', // CPY-248
    periodHour: 'Last hour', // CPY-249
    periodDay: 'Last 24 hours', // CPY-250
    metricsRange24h: 'Trends: last 24 hours, 1-minute interval · Points: {n}/1440', // CPY-251
    trendsTitle24h: 'Trends (last 24 hours)', // CPY-252
    chartEmpty24h: 'Collecting trend data. One point is added every minute.', // CPY-253
    cpuNote24h: 'Share of CPU time used in the last minute. 100% equals one fully used core.', // CPY-254
    sources: 'Sources', // CPY-255
    sourcesPeriodLabel: 'Period', // CPY-256
    sourcesRange: 'Counting {start} – {end} · Stored events: {stored}/{capacity}', // CPY-257
    sourcesDropped: 'The storage limit was reached, so events up to {time} were dropped. The start of this period is not counted.', // CPY-258
    sourcesSummaryTitle: 'Summary', // CPY-259
    sourcesSummaryNames: ['Connection attempts', 'Rejected', 'Aborted', 'Sessions started', 'Addresses', 'Countries', 'ASNs'], // CPY-260
    sourcesSummaryNote: 'Connection attempts are connections that relay listeners accepted or rejected at the accept stage. Rejected and aborted include all stages.', // CPY-261
    ipDbTitle: 'IP information databases', // CPY-262
    ipDbNames: ['Country database', 'ASN database'], // CPY-263
    ipDbNotConfigured: 'Not configured', // CPY-264
    ipDbLoaded: 'In use', // CPY-265
    ipDbStale: 'Using previous file', // CPY-266
    ipDbUnavailable: 'Load failed', // CPY-267
    ipDbInfoLabels: ['Type', 'Built', 'Loaded'], // CPY-268
    ipDbNotConfiguredNote: 'Set {key} in the configuration file to a database file path and restart the server to look up this information.', // CPY-269
    ipDbStaleNote: 'The file could not be read again, so the previous database is still in use. Cause: {reason}. The file is read again automatically after it is fixed.', // CPY-270
    ipDbUnavailableNote: 'The database file could not be read. Cause: {reason}. The file is read again automatically after it is fixed.', // CPY-271
    ipDbFileUnreadable: 'File missing or unreadable', // CPY-272
    ipDbFileTooLarge: 'File larger than 256 MiB', // CPY-273
    ipDbInvalidFormat: 'Not a valid MMDB file', // CPY-274
    ipDbUnsupportedData: 'No country or ASN data for this database', // CPY-275
    addressTitle: 'By address', // CPY-279
    addressColumns: ['Address', 'Country', 'ASN', 'Attempts', 'Rejected', 'Aborted', 'Sessions', 'Last seen', 'Block status'], // CPY-280·295
    clientAddressNote: 'The client address is the address forwarded by a trusted proxy, or the socket peer address when there is none. The Sessions column counts sessions that the address joined as a Viewer or Mode II Server.', // CPY-281
    localAddress: 'Local', // CPY-282
    localAddressNote: 'Local addresses (loopback, private, link-local, and shared) are not looked up in the IP information databases and have no lookup link.', // CPY-283
    countryTitle: 'By country', // CPY-284
    countryColumns: ['Country', 'Addresses', 'Attempts', 'Rejected', 'Aborted', 'Sessions'], // CPY-285
    asnTitle: 'By ASN', // CPY-286
    asnColumns: ['ASN', 'Name', 'Addresses', 'Attempts', 'Rejected', 'Aborted', 'Sessions'], // CPY-287
    unknownValue: 'Unknown', // CPY-288
    partialCount: 'Top {shown} of {total}', // CPY-289
    sourcesEmpty: 'No connections were recorded in this period.', // CPY-290
    lookupName: 'Look up {value} on ipinfo.io', // CPY-291
    lookupNote: 'Lookup links open ipinfo.io in a new tab. Selecting a link sends that address or ASN to ipinfo.io.', // CPY-292
    sourcesRejected: 'The sources request could not be processed. Use Refresh to query again.', // CPY-293
    // SCR-008 address block and unblock (CPY-296~318, 324). {address} and {diagnostic} are filled by fill().
    denyBlock: 'Block', // CPY-296
    denyBlockName: 'Block {address}', // CPY-297
    denyUnblock: 'Unblock', // CPY-298
    denyUnblockName: 'Unblock {address}', // CPY-299
    denyBlocked: 'Blocked', // CPY-300
    denyRangeNote: 'Blocked by a range entry. Remove it in Settings.', // CPY-301
    denyEditOff: 'To block addresses, set management.config_edit = true in the configuration file and restart the server.', // CPY-302
    denyBlockTitle: 'Confirm blocking address', // CPY-303
    denyBlockLabel: 'Address to block', // CPY-304
    denyBlockBody: 'New connections from this address are rejected at all relay entry points, and its open sessions and waiting connections are closed immediately.', // CPY-305
    denyBlocking: 'Blocking…', // CPY-306
    denyUnblockTitle: 'Confirm unblocking address', // CPY-307
    denyUnblockLabel: 'Address to unblock', // CPY-308
    denyUnblockBody: 'New connections from this address are checked against the allow lists again. Connections that were closed do not come back.', // CPY-309
    denyUnblocking: 'Unblocking…', // CPY-310
    denyBlockDone: 'Blocked address: {address}', // CPY-311
    denyUnblockDone: 'Unblocked address: {address}', // CPY-312
    denyConflict: 'The deny list was not changed because the configuration file changed. Use Refresh and try again.', // CPY-313
    denyFileState: 'The deny list was not changed because of the state of the configuration file. Check the file state in View settings.', // CPY-314
    denyInvalid: 'The deny list was not changed. Cause: {diagnostic}', // CPY-315
    denyRequestFailed: 'The request could not be processed. Reload this screen and try again.', // CPY-316
    denyUnknown: 'Could not confirm whether the deny list changed. Use Refresh to check.', // CPY-317
    denyConfigReadFailed: 'The settings could not be read, so the deny list was not changed. Try again later.', // CPY-318
    denyNothingToRemove: 'The deny list has already changed and has no entry to remove. Check the refreshed result.', // CPY-324
  },
};
