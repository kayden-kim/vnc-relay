//! Proposed TRC-001..023, pending adoption under TRAY-TBD-001.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Korean,
    English,
}

#[derive(Clone, Copy)]
pub enum Text {
    Status,
    Missing,
    Stopped,
    Starting,
    Running,
    NotReady,
    Stopping,
    Unknown,
    ExitCode,
    Start,
    Stop,
    Dashboard,
    Log,
    Config,
    SignIn,
    Exit,
    ConfigFailure,
    ActionFailure,
    Denied,
    NotFound,
    Invalid,
    NoConfig,
    OsError,
}

impl Language {
    pub fn text(self, text: Text) -> &'static str {
        use Text::*;
        match (self, text) {
            (_, Status) => "VNC Relay: ",
            (Self::Korean, Missing) => "서비스 없음",
            (Self::English, Missing) => "Not installed",
            (Self::Korean, Stopped) => "중지됨",
            (Self::English, Stopped) => "Stopped",
            (Self::Korean, Starting) => "시작 중",
            (Self::English, Starting) => "Starting",
            (Self::Korean, Running) => "실행 중",
            (Self::English, Running) => "Running",
            (Self::Korean, NotReady) => "실행 중(준비 안 됨)",
            (Self::English, NotReady) => "Running (not ready)",
            (Self::Korean, Stopping) => "중지 중",
            (Self::English, Stopping) => "Stopping",
            (Self::Korean, Unknown) => "확인 불가",
            (Self::English, Unknown) => "Unknown",
            (Self::Korean, ExitCode) => "종료 코드",
            (Self::English, ExitCode) => "exit code",
            (Self::Korean, Start) => "시작",
            (Self::English, Start) => "Start",
            (Self::Korean, Stop) => "중지",
            (Self::English, Stop) => "Stop",
            (Self::Korean, Dashboard) => "대시보드 열기",
            (Self::English, Dashboard) => "Open dashboard",
            (Self::Korean, Log) => "로그 파일 위치 열기",
            (Self::English, Log) => "Open log file location",
            (Self::Korean, Config) => "설정 파일 위치 열기",
            (Self::English, Config) => "Open config file location",
            (Self::Korean, SignIn) => "로그인할 때 시작",
            (Self::English, SignIn) => "Start at sign-in",
            (Self::Korean, Exit) => "트레이 끝내기",
            (Self::English, Exit) => "Exit tray",
            (Self::Korean, ConfigFailure) => "설정을 읽지 못함: ",
            (Self::English, ConfigFailure) => "Cannot read config: ",
            (Self::Korean, ActionFailure) => "실패",
            (Self::English, ActionFailure) => "failed",
            (Self::Korean, Denied) => "권한 없음",
            (Self::English, Denied) => "Access denied",
            (Self::Korean, NotFound) => "파일 없음",
            (Self::English, NotFound) => "File not found",
            (Self::Korean, Invalid) => "설정 오류",
            (Self::English, Invalid) => "Invalid config",
            (Self::Korean, NoConfig) => "등록 명령에 설정 경로 없음",
            (Self::English, NoConfig) => "No config path in the service command",
            (Self::Korean, OsError) => "OS 오류",
            (Self::English, OsError) => "OS error",
        }
    }
}
