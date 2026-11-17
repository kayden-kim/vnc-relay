//! State and address rules from windows-tray §2..5; no outside I/O.

use crate::copy::{Language, Text};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use windows_service::service::{ServiceExitCode, ServiceState};
use windows_sys::Win32::Foundation::ERROR_SERVICE_NEVER_STARTED;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Missing,
    Stopped(u32),
    Starting,
    Running,
    NotReady,
    Stopping,
    Unknown,
}

impl State {
    pub fn from_service(state: ServiceState, exit: ServiceExitCode, ready: bool) -> Self {
        match state {
            ServiceState::Stopped => Self::Stopped(match exit {
                // The SCM reports this code for a service that has not run
                // since boot, so there is no exit code to show.
                ServiceExitCode::Win32(ERROR_SERVICE_NEVER_STARTED) => 0,
                ServiceExitCode::ServiceSpecific(n) | ServiceExitCode::Win32(n) => n,
            }),
            ServiceState::StartPending => Self::Starting,
            ServiceState::StopPending => Self::Stopping,
            ServiceState::Running if ready => Self::Running,
            ServiceState::Running => Self::NotReady,
            _ => Self::Unknown,
        }
    }

    pub fn label(self, language: Language) -> String {
        let text = match self {
            Self::Missing => Text::Missing,
            Self::Stopped(_) => Text::Stopped,
            Self::Starting => Text::Starting,
            Self::Running => Text::Running,
            Self::NotReady => Text::NotReady,
            Self::Stopping => Text::Stopping,
            Self::Unknown => Text::Unknown,
        };
        let mut result = format!("{}{}", language.text(Text::Status), language.text(text));
        if let Self::Stopped(code) = self
            && code != 0
        {
            result.push_str(&format!(" ({} {code})", language.text(Text::ExitCode)));
        }
        result
    }

    pub fn enabled(self, config_readable: bool, log_file: bool, config_path: bool) -> Enabled {
        Enabled {
            start: matches!(self, Self::Stopped(_)),
            stop: matches!(self, Self::Running | Self::NotReady),
            dashboard: self == Self::Running && config_readable,
            log: self != Self::Missing && config_readable && log_file,
            config: self != Self::Missing && config_path,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Enabled {
    pub start: bool,
    pub stop: bool,
    pub dashboard: bool,
    pub log: bool,
    pub config: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Denied,
    NotFound,
    Invalid,
    NoConfig,
    Os(i32),
}
impl Reason {
    pub fn label(self, language: Language) -> String {
        let text = match self {
            Self::Denied => Text::Denied,
            Self::NotFound => Text::NotFound,
            Self::Invalid => Text::Invalid,
            Self::NoConfig => Text::NoConfig,
            Self::Os(code) => return format!("{} {code}", language.text(Text::OsError)),
        };
        language.text(text).to_owned()
    }
    pub fn from_io(error: &std::io::Error) -> Self {
        if error.raw_os_error() == Some(5) || error.kind() == std::io::ErrorKind::PermissionDenied {
            Self::Denied
        } else {
            Self::Os(error.raw_os_error().unwrap_or(0))
        }
    }
}

pub fn connect_address(mut address: SocketAddr) -> SocketAddr {
    if address.ip().is_unspecified() {
        address.set_ip(match address.ip() {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::LOCALHOST),
        });
    }
    address
}

pub fn dashboard_url(address: SocketAddr, certificate: bool) -> String {
    format!(
        "{}://{}/",
        if certificate { "https" } else { "http" },
        connect_address(address)
    )
}

pub fn language_tag(tag: &str) -> Language {
    if tag
        .split(['-', '_'])
        .next()
        .is_some_and(|primary| primary.eq_ignore_ascii_case("ko"))
    {
        Language::Korean
    } else {
        Language::English
    }
}

pub fn run_value_matches(value: &str, executable: &str) -> bool {
    let path = value
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(value);
    path.to_lowercase() == executable.to_lowercase()
}

/// Splits a registered Windows command using CRT backslash/quote rules.
/// The first token (the executable) is not inspected for configuration options.
pub fn config_path(command: &str) -> Option<PathBuf> {
    let arguments = split_command(command);
    let mut args = arguments.iter().skip(1);
    let mut found = None;
    while let Some(arg) = args.next() {
        let value = if arg == "--config" {
            Some(args.next()?.as_str())
        } else {
            arg.strip_prefix("--config=")
        };
        if let Some(value) = value {
            if value.is_empty() || value.starts_with("--") || found.is_some() {
                return None;
            }
            found = Some(PathBuf::from(value));
        }
    }
    found
}

fn split_command(command: &str) -> Vec<String> {
    let mut chars = command.chars().peekable();
    let mut args = Vec::new();
    while chars.peek().is_some() {
        while chars.peek().is_some_and(|c| matches!(c, ' ' | '\t')) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        let mut value = String::new();
        let mut quoted = false;
        while let Some(&c) = chars.peek() {
            if !quoted && matches!(c, ' ' | '\t') {
                break;
            }
            let mut slashes = 0;
            while chars.peek() == Some(&'\\') {
                chars.next();
                slashes += 1;
            }
            if chars.peek() == Some(&'"') {
                value.extend(std::iter::repeat_n('\\', slashes / 2));
                chars.next();
                if slashes % 2 != 0 {
                    value.push('"');
                } else if quoted && chars.peek() == Some(&'"') {
                    chars.next();
                    value.push('"');
                } else {
                    quoted = !quoted;
                }
            } else {
                value.extend(std::iter::repeat_n('\\', slashes));
                if !quoted && chars.peek().is_some_and(|c| matches!(c, ' ' | '\t')) {
                    break;
                }
                if let Some(c) = chars.next() {
                    value.push(c);
                }
            }
        }
        args.push(value);
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    /// spec: windows-tray.md:TRAY-003 partial
    /// spec: windows-tray.md:TRAY-007 partial
    #[test]
    fn maps_states_and_enable_table() {
        let cases = [
            (State::Missing, false, false, false),
            (State::Stopped(0), true, false, false),
            (State::Starting, false, false, false),
            (State::Running, false, true, true),
            (State::NotReady, false, true, false),
            (State::Stopping, false, false, false),
            (State::Unknown, false, false, false),
        ];
        for (state, start, stop, dashboard) in cases {
            let enabled = state.enabled(true, true, true);
            assert_eq!(
                (enabled.start, enabled.stop, enabled.dashboard),
                (start, stop, dashboard)
            );
            assert_eq!(
                (enabled.log, enabled.config),
                (state != State::Missing, state != State::Missing)
            );
        }
        for (source, expected) in [
            (ServiceState::Stopped, State::Stopped(0)),
            (ServiceState::StartPending, State::Starting),
            (ServiceState::StopPending, State::Stopping),
            (ServiceState::Running, State::NotReady),
            (ServiceState::Paused, State::Unknown),
            (ServiceState::PausePending, State::Unknown),
            (ServiceState::ContinuePending, State::Unknown),
        ] {
            assert_eq!(
                State::from_service(source, ServiceExitCode::Win32(0), false),
                expected
            );
        }
        assert_eq!(
            State::from_service(ServiceState::Running, ServiceExitCode::Win32(0), true),
            State::Running
        );
    }

    /// spec: windows-tray.md:TRAY-003 partial
    /// spec: windows-tray.md:TRAY-007 partial
    #[test]
    fn displays_service_specific_and_win32_exit_codes() {
        for exit in [
            ServiceExitCode::ServiceSpecific(3),
            ServiceExitCode::Win32(3),
        ] {
            let state = State::from_service(ServiceState::Stopped, exit, false);
            assert_eq!(
                state.label(Language::Korean),
                "VNC Relay: 중지됨 (종료 코드 3)"
            );
            assert_eq!(
                state.label(Language::English),
                "VNC Relay: Stopped (exit code 3)"
            );
        }
        assert_eq!(
            State::Stopped(0).label(Language::Korean),
            "VNC Relay: 중지됨"
        );
        // A service installed after boot that never ran has no exit code.
        assert_eq!(
            State::from_service(
                ServiceState::Stopped,
                ServiceExitCode::Win32(ERROR_SERVICE_NEVER_STARTED),
                false
            )
            .label(Language::Korean),
            "VNC Relay: 중지됨"
        );
    }

    /// spec: windows-tray.md:TRAY-009 partial
    #[test]
    fn unreadable_config_disables_dashboard_and_log_only() {
        assert_eq!(
            State::NotReady.enabled(false, false, true),
            Enabled {
                start: false,
                stop: true,
                dashboard: false,
                log: false,
                config: true
            }
        );
        assert!(!State::Running.enabled(false, true, true).dashboard);
    }

    /// spec: windows-tray.md:TRAY-008 partial
    #[test]
    fn constructs_dashboard_urls() -> Result<(), std::net::AddrParseError> {
        for (address, tls, expected) in [
            ("127.0.0.1:80", false, "http://127.0.0.1:80/"),
            ("0.0.0.0:8443", true, "https://127.0.0.1:8443/"),
            ("[::]:8443", true, "https://[::1]:8443/"),
            ("[::1]:80", false, "http://[::1]:80/"),
        ] {
            assert_eq!(dashboard_url(address.parse()?, tls), expected);
        }
        Ok(())
    }

    /// spec: windows-tray.md:TRAY-003 partial
    #[test]
    fn maps_readyz_wildcards_and_preserves_other_addresses() -> Result<(), std::net::AddrParseError>
    {
        for (address, expected) in [
            ("0.0.0.0:9000", "127.0.0.1:9000"),
            ("[::]:9000", "[::1]:9000"),
            ("192.0.2.1:9000", "192.0.2.1:9000"),
        ] {
            assert_eq!(connect_address(address.parse()?).to_string(), expected);
        }
        Ok(())
    }

    /// spec: windows-tray.md:TRAY-009 partial
    /// spec: windows-tray.md:TRAY-010 partial
    #[test]
    fn parses_config_with_windows_quoting_and_equals() {
        for command in [
            r#""C:\Program Files\relay.exe" service run --config "C:\Relay Data\relay.toml""#,
            r#"relay.exe service run --config="C:\Relay Data\relay.toml""#,
        ] {
            assert_eq!(
                config_path(command),
                Some(PathBuf::from(r"C:\Relay Data\relay.toml"))
            );
        }
        assert_eq!(
            config_path(r"relay.exe --config C:\relay.toml"),
            Some(PathBuf::from(r"C:\relay.toml"))
        );
        assert_eq!(
            split_command(r#"x "a\\\"b" "a""b" "C:\dir\\""#),
            vec!["x", "a\\\"b", "a\"b", "C:\\dir\\"]
        );
        for command in [
            "relay.exe service run",
            "relay.exe --config",
            "relay.exe --config=",
            "relay.exe --config \"\"",
            "relay.exe --config --other",
            "relay.exe --config=a --config=b",
        ] {
            assert_eq!(config_path(command), None);
        }
    }

    /// spec: windows-tray.md:TRAY-012 partial
    #[test]
    fn maps_korean_primary_language_only() {
        for tag in ["ko", "ko-KR", "KO-kr"] {
            assert_eq!(language_tag(tag), Language::Korean);
        }
        for tag in ["en-US", "ja-JP", "kok-IN", ""] {
            assert_eq!(language_tag(tag), Language::English);
        }
    }

    /// spec: windows-tray.md:TRAY-011 partial
    #[test]
    fn matches_run_value_without_quotes_and_case() {
        let exe = r"C:\Apps\vnc-relay-tray.exe";
        assert!(run_value_matches(r#""c:\apps\VNC-RELAY-TRAY.EXE""#, exe));
        assert!(run_value_matches(exe, exe));
        assert!(!run_value_matches(r#""C:\other.exe""#, exe));
        assert!(!run_value_matches(&format!("\"{exe}\" --extra"), exe));
    }
}
