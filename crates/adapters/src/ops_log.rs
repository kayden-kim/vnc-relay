//! Operation log: one JSON object per line on stdout or in a file
//! (ops-info-v1 §1.1-1.4). Design: docs/architectures/ops-log.md.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::SystemTime;

use serde_json::{Value, json};
use vnc_relay_app::events::{Event, LOG_CONTRACT, Stage};
use vnc_relay_app::ports::EventSink;
use vnc_relay_app::settings::{ConfigWarning, LogOutput};

use crate::clock::timestamp;

/// Longest `detail` in bytes (ops-info-v1 §1.1).
const DETAIL_MAX_BYTES: usize = 512;

/// Longest logged `origin` in bytes (ops-info-v1 §1.1). The full value stays
/// in the refusal itself.
const ORIGIN_MAX_BYTES: usize = 256;

type FailureNotice = Box<dyn Fn(&io::Error) + Send + Sync>;

/// `EventSink` that writes JSON Lines. A line is built first and written
/// with one `write_all` under a lock, so lines from many tasks never mix.
/// A failed write drops the line; only the first failure calls the notice.
pub struct JsonLinesEventSink {
    output: Mutex<Output>,
    failed: AtomicBool,
    on_first_failure: FailureNotice,
}

enum Output {
    Stdout,
    File(File),
}

impl Output {
    fn write_line(&mut self, line: &[u8]) -> io::Result<()> {
        match self {
            Self::Stdout => {
                let mut stdout = io::stdout().lock();
                stdout.write_all(line)?;
                stdout.flush()
            }
            Self::File(file) => file.write_all(line),
        }
    }
}

impl JsonLinesEventSink {
    /// Writes to stdout. The start uses it before the log output is known.
    pub fn stdout(on_first_failure: impl Fn(&io::Error) + Send + Sync + 'static) -> Self {
        Self::with_output(Output::Stdout, on_first_failure)
    }

    /// Opens the configured output. A file is appended to and created when
    /// it is missing (config-v1 §1.2 `log.output`).
    pub fn open(
        output: &LogOutput,
        on_first_failure: impl Fn(&io::Error) + Send + Sync + 'static,
    ) -> Result<Self, LogFileError> {
        let output = match output {
            LogOutput::Stdout => Output::Stdout,
            LogOutput::File(path) => Output::File(open_append(path)?),
        };
        Ok(Self::with_output(output, on_first_failure))
    }

    fn with_output(
        output: Output,
        on_first_failure: impl Fn(&io::Error) + Send + Sync + 'static,
    ) -> Self {
        Self {
            output: Mutex::new(output),
            failed: AtomicBool::new(false),
            on_first_failure: Box::new(on_first_failure),
        }
    }
}

impl EventSink for JsonLinesEventSink {
    fn record(&self, event: &Event) {
        let line = format_line(event, SystemTime::now());
        let result = self
            .output
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .write_line(line.as_bytes());
        if let Err(error) = result
            && !self.failed.swap(true, Ordering::Relaxed)
        {
            (self.on_first_failure)(&error);
        }
    }
}

fn open_append(path: &Path) -> Result<File, LogFileError> {
    OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|error| LogFileError {
            cause: match error.kind() {
                ErrorKind::NotFound => "상위 디렉터리가 없다.".to_owned(),
                ErrorKind::PermissionDenied => "권한이 없다.".to_owned(),
                ErrorKind::IsADirectory => "디렉터리다.".to_owned(),
                _ => format!("{error}."),
            },
        })
}

/// The log file cannot be opened for appending (`log_file_unavailable`,
/// config-v1 §2.1). The cause names no path: the path is a config value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogFileError {
    cause: String,
}

impl LogFileError {
    /// The stable code string of the contract.
    pub fn code(&self) -> &'static str {
        "log_file_unavailable"
    }

    /// Why the file cannot be opened, as one Korean sentence.
    pub fn cause(&self) -> &str {
        &self.cause
    }
}

impl fmt::Display for LogFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "로그 파일을 열 수 없다. {}", self.cause)
    }
}

impl std::error::Error for LogFileError {}

/// One log line, LF included: `ts`, `level`, `event`, then the event fields.
/// Absent optional fields are left out; no value is `null`.
pub fn format_line(event: &Event, at: SystemTime) -> String {
    let mut fields = vec![
        ("ts", Value::from(timestamp(at))),
        ("level", Value::from(event.level().as_str())),
        ("event", Value::from(event.name())),
    ];
    push_event_fields(&mut fields, event);
    let mut line = String::with_capacity(160);
    line.push('{');
    for (i, (key, value)) in fields.iter().enumerate() {
        if i > 0 {
            line.push(',');
        }
        // serde_json escapes quotes, line breaks, and control characters, so
        // the line ends with the one LF below.
        line.push_str(&Value::from(*key).to_string());
        line.push(':');
        line.push_str(&value.to_string());
    }
    line.push_str("}\n");
    line
}

fn push_event_fields(fields: &mut Vec<(&'static str, Value)>, event: &Event) {
    match event {
        Event::ServerStarting => fields.push(("log_contract", Value::from(LOG_CONTRACT))),
        Event::ServerReady { listeners } => {
            let listeners = listeners
                .iter()
                .map(|(name, listen)| json!({ "name": name, "listen": listen.to_string() }))
                .collect();
            fields.push(("listeners", Value::Array(listeners)));
        }
        Event::ConfigWarning { warning, phase } => match warning {
            ConfigWarning::PlaintextWsEnabled { listen } => {
                fields.push(("listener", Value::from("ws_viewer")));
                fields.push(("listen", Value::from(listen.to_string())));
            }
            ConfigWarning::AllowAllSource { entry_point, item } => {
                fields.push(("listener", Value::from(entry_point.name())));
                fields.push(("item", Value::from(item.to_string())));
                fields.push(("phase", Value::from(phase.as_str())));
            }
            ConfigWarning::AllowAllTarget { item } => {
                fields.push(("item", Value::from(item.to_string())));
                fields.push(("phase", Value::from(phase.as_str())));
            }
            ConfigWarning::StatusListenNotLoopback { listen } => {
                fields.push(("listen", Value::from(listen.to_string())));
            }
        },
        Event::StartupFailed {
            reason,
            config_key,
            listener,
            detail,
        } => {
            fields.push(("reason", Value::from(*reason)));
            if let Some(config_key) = config_key {
                fields.push(("config_key", Value::from(config_key.as_str())));
            }
            if let Some(listener) = listener {
                fields.push(("listener", Value::from(*listener)));
            }
            if let Some(detail) = detail {
                fields.push(("detail", Value::from(cut(detail, DETAIL_MAX_BYTES))));
            }
        }
        Event::ConnectionAccepted {
            conn_id,
            entry_point,
            peer_ip,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("listener", Value::from(entry_point.name())));
            fields.push(("role", Value::from(entry_point.role().as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
        }
        Event::ConnectionRejected {
            listener,
            stage,
            reason,
            peer_ip,
        } => {
            fields.push(("listener", Value::from(*listener)));
            fields.push(("stage", Value::from(stage.as_str())));
            fields.push(("reason", Value::from(reason.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            if let Some(limit) = reason.limit() {
                fields.push(("limit", Value::from(limit)));
            }
        }
        Event::UpgradeRejected {
            conn_id,
            listener,
            reason,
            peer_ip,
            origin,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("listener", Value::from(*listener)));
            fields.push(("stage", Value::from(Stage::Upgrade.as_str())));
            fields.push(("reason", Value::from(reason.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            if let Some(origin) = origin {
                fields.push((
                    "origin",
                    Value::from(cut(origin.as_str(), ORIGIN_MAX_BYTES)),
                ));
            }
        }
        Event::ConnectionAborted {
            conn_id,
            entry_point,
            stage,
            reason,
            peer_ip,
            elapsed_ms,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("listener", Value::from(entry_point.name())));
            fields.push(("stage", Value::from(stage.as_str())));
            fields.push(("reason", Value::from(reason.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            fields.push(("elapsed_ms", Value::from(*elapsed_ms)));
            if let Some(kind) = reason.error_kind() {
                fields.push(("error_kind", Value::from(kind.as_str())));
            }
        }
        Event::AdminLoginSucceeded {
            admin_account,
            peer_ip,
        } => {
            fields.push(("admin_account", Value::from(admin_account.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
        }
        Event::AdminLoginFailed {
            reason,
            scope,
            admin_account,
            peer_ip,
        } => {
            fields.push(("reason", Value::from(reason.as_str())));
            if let Some(scope) = scope {
                fields.push(("scope", Value::from(scope.as_str())));
            }
            if let Some(admin_account) = admin_account {
                fields.push(("admin_account", Value::from(admin_account.as_str())));
            }
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
        }
        Event::AdminLoginLimited {
            scope,
            lockout_secs,
            admin_account,
            peer_ip,
        } => {
            fields.push(("scope", Value::from(scope.as_str())));
            fields.push(("lockout_secs", Value::from(*lockout_secs)));
            if let Some(admin_account) = admin_account {
                fields.push(("admin_account", Value::from(admin_account.as_str())));
            }
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
        }
        Event::ServerStopped => {}
    }
}

/// The longest prefix of `text` within `max` UTF-8 bytes that ends on a
/// character boundary.
fn cut(text: &str, max: usize) -> &str {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;
    use std::time::UNIX_EPOCH;
    use vnc_relay_app::events::RejectReason;
    use vnc_relay_domain::conn_id::ConnId;
    use vnc_relay_domain::peer_ip::PeerIp;

    #[test]
    fn cuts_detail_on_a_character_boundary() {
        let detail = format!("{}가나", "a".repeat(510));
        let line = format_line(
            &Event::StartupFailed {
                reason: "runtime_unavailable",
                config_key: None,
                listener: None,
                detail: Some(detail),
            },
            UNIX_EPOCH,
        );
        // 510 + 3 bytes would pass 512, so the first Hangul syllable goes.
        let kept = format!("\"detail\":\"{}\"}}\n", "a".repeat(510));
        assert!(line.ends_with(&kept), "{line}");
        assert_eq!(cut("가나다", 512), "가나다");
        assert_eq!(cut("가나다", 5), "가");
    }

    #[test]
    fn cuts_origin_to_256_bytes_on_a_character_boundary() {
        // ops-info-v1 §1.1: `origin` is cut to 256 bytes at a character boundary.
        let line = |origin: String| {
            format_line(
                &Event::UpgradeRejected {
                    listener: "ws_viewer",
                    conn_id: ConnId::next(),
                    reason: RejectReason::OriginNotAllowed,
                    peer_ip: PeerIp::new(IpAddr::from([127, 0, 0, 1])),
                    origin: Some(origin),
                },
                UNIX_EPOCH,
            )
        };
        // 254 + 3 bytes would pass 256, so the Hangul syllable goes.
        let text = line(format!("{}가", "a".repeat(254)));
        let kept = format!("\"origin\":\"{}\"", "a".repeat(254));
        assert!(text.contains(&kept), "{text}");
        assert!(!text.contains('가'), "{text}");
        let text = line("b".repeat(300));
        let kept = format!("\"origin\":\"{}\"", "b".repeat(256));
        assert!(text.contains(&kept), "{text}");
        assert!(!text.contains(&"b".repeat(257)), "{text}");
    }
}
