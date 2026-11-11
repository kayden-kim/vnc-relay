//! Operation log: one JSON object per line on stdout or in a file
//! (ops-info-v2 §1.1-1.4). Design: docs/architectures/ops-log.md.

use std::collections::{BTreeSet, VecDeque};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::SystemTime;

use serde_json::{Value, json};
use vnc_relay_app::events::{Event, LOG_CONTRACT, RejectReason, Stage};
use vnc_relay_app::ports::EventSink;
use vnc_relay_app::settings::{ConfigWarning, LogOutput};
use vnc_relay_domain::entry_point::{Listener, Role};
use vnc_relay_domain::handshake::Mode;
use vnc_relay_domain::peer_ip::PeerIp;
use vnc_relay_domain::session::CloseCause;

use crate::clock::timestamp;

/// Longest `detail` in bytes (ops-info-v2 §1.1).
const DETAIL_MAX_BYTES: usize = 512;

/// Longest logged `origin` in bytes (ops-info-v2 §1.1). The full value stays
/// in the refusal itself.
const ORIGIN_MAX_BYTES: usize = 256;

/// Longest logged target name or literal IP (ops-info-v2 §1.1).
const TARGET_HOST_MAX_BYTES: usize = 256;

type FailureNotice = Box<dyn Fn(&io::Error) + Send + Sync>;
type ReopenNotice = Box<dyn Fn(&LogFileError) + Send + Sync>;

/// `EventSink` that writes JSON Lines. A line is built first and written
/// with one `write_all` under a lock, so lines from many tasks never mix.
/// A failed write drops the line; only the first failure calls the notice.
pub struct JsonLinesEventSink {
    output: Mutex<Output>,
    failed: AtomicBool,
    on_first_failure: FailureNotice,
    path: Option<PathBuf>,
    on_reopen_failure: Option<ReopenNotice>,
    recent_logs: Option<Arc<RecentLogs>>,
}

enum Output {
    Stdout,
    File { handle: File, reopen_failed: bool },
}

impl Output {
    fn write_line(&mut self, line: &[u8]) -> io::Result<()> {
        match self {
            Self::Stdout => {
                let mut stdout = io::stdout().lock();
                stdout.write_all(line)?;
                stdout.flush()
            }
            Self::File { handle, .. } => handle.write_all(line),
        }
    }
}

impl JsonLinesEventSink {
    /// Writes to stdout. The start uses it before the log output is known.
    pub fn stdout(on_first_failure: impl Fn(&io::Error) + Send + Sync + 'static) -> Self {
        Self::with_output(Output::Stdout, None, on_first_failure, None)
    }

    /// Opens the configured output. A file is appended to and created when
    /// it is missing (config-v4 §1.2 `log.output`).
    pub fn open(
        output: &LogOutput,
        on_first_failure: impl Fn(&io::Error) + Send + Sync + 'static,
        on_reopen_failure: impl Fn(&LogFileError) + Send + Sync + 'static,
    ) -> Result<Self, LogFileError> {
        let (output, path) = match output {
            LogOutput::Stdout => (Output::Stdout, None),
            LogOutput::File(path) => (
                Output::File {
                    handle: open_append(path)?,
                    reopen_failed: false,
                },
                Some(path.clone()),
            ),
        };
        Ok(Self::with_output(
            output,
            path,
            on_first_failure,
            Some(Box::new(on_reopen_failure)),
        ))
    }

    fn with_output(
        output: Output,
        path: Option<PathBuf>,
        on_first_failure: impl Fn(&io::Error) + Send + Sync + 'static,
        on_reopen_failure: Option<ReopenNotice>,
    ) -> Self {
        Self {
            output: Mutex::new(output),
            failed: AtomicBool::new(false),
            on_first_failure: Box::new(on_first_failure),
            path,
            on_reopen_failure,
            recent_logs: None,
        }
    }

    /// Attaches an in-memory recent-logs ring buffer to receive every recorded entry.
    pub fn with_recent_logs(mut self, recent_logs: Arc<RecentLogs>) -> Self {
        self.recent_logs = Some(recent_logs);
        self
    }

    /// Original configured file path. Stdout has no reopen target.
    pub fn file_path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Reopens under the line-writing lock. A failure keeps the old handle and
    /// reports only the first failure until a successful reopen ends the streak.
    pub fn reopen(&self) {
        let Some(path) = self.file_path() else { return };
        let mut output = self.output.lock().unwrap_or_else(PoisonError::into_inner);
        let Output::File {
            handle,
            reopen_failed,
        } = &mut *output
        else {
            return;
        };
        match open_append(path) {
            Ok(file) => {
                *handle = file;
                *reopen_failed = false;
            }
            Err(error) if !*reopen_failed => {
                *reopen_failed = true;
                let (entry, line) = self.prepare(&Event::LogReopenFailed {
                    detail: error.cause().to_owned(),
                });
                if let Some((recent_logs, entry)) = entry {
                    recent_logs.record(entry);
                }
                let written = output.write_line(line.as_bytes());
                drop(output);
                self.report_write_result(written);
                if let Some(notice) = &self.on_reopen_failure {
                    notice(&error);
                }
            }
            Err(_) => {}
        }
    }

    fn prepare(&self, event: &Event) -> (Option<(&RecentLogs, Arc<LogEntry>)>, String) {
        match self
            .recent_logs
            .as_deref()
            .filter(|logs| logs.max_entries() > 0)
        {
            Some(logs) => {
                let (entry, line) = format_entry_and_line(event, SystemTime::now());
                (Some((logs, entry)), line)
            }
            None => (None, format_line(event, SystemTime::now())),
        }
    }

    fn report_write_result(&self, result: io::Result<()>) {
        if let Err(error) = result
            && !self.failed.swap(true, Ordering::Relaxed)
        {
            (self.on_first_failure)(&error);
        }
    }
}

impl EventSink for JsonLinesEventSink {
    fn record(&self, event: &Event) {
        let (entry, line) = self.prepare(event);
        let mut output = self.output.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((recent_logs, entry)) = entry {
            recent_logs.record(entry);
        }
        let result = output.write_line(line.as_bytes());
        drop(output);
        self.report_write_result(result);
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
/// config-v4 §2.1). The cause names no path: the path is a config value.
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

/// Ordered fields shared by the output line and the recent-log response.
#[derive(Debug)]
pub struct LogEntry {
    fields: Vec<(&'static str, Value)>,
}

impl LogEntry {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields
            .iter()
            .find_map(|(name, value)| (*name == key).then_some(value))
    }
}

impl std::ops::Index<&str> for LogEntry {
    type Output = Value;

    fn index(&self, key: &str) -> &Self::Output {
        self.get(key).unwrap_or(&Value::Null)
    }
}

impl fmt::Display for LogEntry {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str("{")?;
        for (index, (key, value)) in self.fields.iter().enumerate() {
            if index > 0 {
                out.write_str(",")?;
            }
            // Event field names are static ASCII identifiers, not user input.
            write!(out, "\"{key}\":{value}")?;
        }
        out.write_str("}")
    }
}

fn event_entry(event: &Event, at: SystemTime) -> LogEntry {
    let mut fields = vec![
        ("ts", Value::from(timestamp(at))),
        ("level", Value::from(event.level().as_str())),
        ("event", Value::from(event.name())),
    ];
    push_event_fields(&mut fields, event);
    LogEntry { fields }
}

/// Builds both representations from the same ordered fields without key copies.
fn format_entry_and_line(event: &Event, at: SystemTime) -> (Arc<LogEntry>, String) {
    use std::fmt::Write;
    let entry = Arc::new(event_entry(event, at));
    let mut line = String::with_capacity(160);
    let _ = writeln!(line, "{entry}");
    (entry, line)
}

/// One log line, LF included: `ts`, `level`, `event`, then the event fields.
/// Absent optional fields are left out; no value is `null`.
pub fn format_line(event: &Event, at: SystemTime) -> String {
    use std::fmt::Write;
    let entry = event_entry(event, at);
    let mut line = String::with_capacity(160);
    let _ = writeln!(line, "{entry}");
    line
}

/// An item kept in the recent-logs ring buffer.
#[derive(Debug, Clone)]
pub struct RecentLogItem {
    pub seq: u64,
    pub entry: Arc<LogEntry>,
}

/// An item in the recent-logs query response.
#[derive(Debug, Clone)]
pub struct LogItemResponse {
    pub seq: u64,
    pub entry: Arc<LogEntry>,
}

/// Filter criteria for querying recent logs.
#[derive(Debug, Clone, Default)]
pub struct LogsFilter {
    pub levels: Option<BTreeSet<String>>,
    pub events: Option<BTreeSet<String>>,
    pub q: Option<String>,
    pub session_id: Option<String>,
    pub before: Option<u64>,
    pub after: Option<u64>,
    pub limit: usize,
}

/// The response to `GET /api/v1/logs` (management-api-v1 §1.15).
#[derive(Debug, Clone)]
pub struct LogsResponse {
    pub as_of: String,
    pub max_entries: usize,
    pub oldest_seq: Option<u64>,
    pub latest_seq: Option<u64>,
    pub has_more: bool,
    pub items: Vec<LogItemResponse>,
}

impl LogsResponse {
    pub fn to_json_string(&self) -> String {
        use std::fmt::Write;
        let mut out = String::with_capacity(self.items.len() * 128 + 128);
        let oldest = match self.oldest_seq {
            Some(s) => s.to_string(),
            None => "null".to_owned(),
        };
        let latest = match self.latest_seq {
            Some(s) => s.to_string(),
            None => "null".to_owned(),
        };
        let _ = write!(
            out,
            r#"{{"as_of":"{}","max_entries":{},"oldest_seq":{},"latest_seq":{},"has_more":{},"items":["#,
            self.as_of, self.max_entries, oldest, latest, self.has_more
        );
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(out, r#"{{"seq":{},"entry":{}}}"#, item.seq, item.entry);
        }
        out.push_str("]}");
        out
    }
}

/// Bounded in-memory ring buffer of recent operation-log events (OPS-068).
#[derive(Debug)]
pub struct RecentLogs {
    max_entries: usize,
    inner: Mutex<RecentLogsInner>,
}

#[derive(Debug)]
struct RecentLogsInner {
    next_seq: u64,
    entries: VecDeque<RecentLogItem>,
}

struct QuerySnapshot {
    entries: Vec<RecentLogItem>,
    session: Option<Arc<LogEntry>>,
    oldest_seq: Option<u64>,
    latest_seq: Option<u64>,
}

impl RecentLogs {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            inner: Mutex::new(RecentLogsInner {
                next_seq: 1,
                entries: VecDeque::with_capacity(max_entries.min(10_000)),
            }),
        }
    }

    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    pub fn record(&self, entry: Arc<LogEntry>) {
        if self.max_entries == 0 {
            return;
        }
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let seq = inner.next_seq;
        inner.next_seq = inner.next_seq.saturating_add(1);
        let evicted = if inner.entries.len() >= self.max_entries {
            inner.entries.pop_front()
        } else {
            None
        };
        inner.entries.push_back(RecentLogItem { seq, entry });
        drop(inner);
        drop(evicted);
    }

    #[cfg(test)]
    fn snapshot(&self) -> (Vec<RecentLogItem>, Option<u64>, Option<u64>) {
        if self.max_entries == 0 {
            return (Vec::new(), None, None);
        }
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let oldest_seq = inner.entries.front().map(|i| i.seq);
        let latest_seq = inner.entries.back().map(|i| i.seq);
        let entries = inner.entries.iter().cloned().collect();
        (entries, oldest_seq, latest_seq)
    }

    fn query_snapshot(&self, filter: &LogsFilter) -> QuerySnapshot {
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let oldest = inner.entries.front().map(|item| item.seq);
        let latest = inner.entries.back().map(|item| item.seq);
        let start = filter.after.map_or(0, |seq| {
            inner.entries.partition_point(|item| item.seq <= seq)
        });
        let end = filter.before.map_or(inner.entries.len(), |seq| {
            inner.entries.partition_point(|item| item.seq < seq)
        });
        // Session linkage can precede the requested cursor range.
        let session = filter.session_id.as_deref().and_then(|sid| {
            inner
                .entries
                .iter()
                .rev()
                .find(|item| {
                    item.entry.get("event").and_then(Value::as_str) == Some("session_started")
                        && item.entry.get("session_id").and_then(Value::as_str) == Some(sid)
                })
                .map(|item| Arc::clone(&item.entry))
        });
        let entries = inner.entries.range(start..end).cloned().collect();
        QuerySnapshot {
            entries,
            session,
            oldest_seq: oldest,
            latest_seq: latest,
        }
    }

    pub fn query(&self, filter: &LogsFilter, at: SystemTime) -> LogsResponse {
        let QuerySnapshot {
            entries,
            session,
            oldest_seq,
            latest_seq,
        } = self.query_snapshot(filter);
        if self.max_entries == 0 || entries.is_empty() {
            return LogsResponse {
                as_of: timestamp(at),
                max_entries: self.max_entries,
                oldest_seq,
                latest_seq,
                has_more: false,
                items: Vec::new(),
            };
        }

        let mut related_conns = Vec::new();
        if let Some(entry) = session {
            for key in ["viewer_conn_id", "server_conn_id"] {
                if let Some(conn) = entry.get(key).and_then(Value::as_str) {
                    related_conns.push(conn.to_owned());
                }
            }
        }

        let needle_folded: Option<Vec<u8>> = filter
            .q
            .as_deref()
            .map(|q| q.bytes().map(|b| b.to_ascii_lowercase()).collect());

        let limit = filter.limit;
        let (selected, has_more) = if filter.after.is_some() {
            let mut matches = Vec::new();
            let mut more = false;
            for item in &entries {
                if item_matches(item, filter, &related_conns, needle_folded.as_deref()) {
                    matches.push(item);
                    if matches.len() > limit {
                        more = true;
                        break;
                    }
                }
            }
            if more {
                matches.pop();
            }
            (matches, more)
        } else {
            let mut matches = Vec::new();
            let mut more = false;
            for item in entries.iter().rev() {
                if item_matches(item, filter, &related_conns, needle_folded.as_deref()) {
                    matches.push(item);
                    if matches.len() > limit {
                        more = true;
                        break;
                    }
                }
            }
            if more {
                matches.pop();
            }
            matches.reverse();
            (matches, more)
        };

        let items = selected
            .into_iter()
            .map(|item| LogItemResponse {
                seq: item.seq,
                entry: Arc::clone(&item.entry),
            })
            .collect();

        LogsResponse {
            as_of: timestamp(at),
            max_entries: self.max_entries,
            oldest_seq,
            latest_seq,
            has_more,
            items,
        }
    }
}

fn item_matches(
    item: &RecentLogItem,
    filter: &LogsFilter,
    related_conns: &[String],
    needle_folded: Option<&[u8]>,
) -> bool {
    if let Some(levels) = &filter.levels {
        let level_match = item
            .entry
            .get("level")
            .and_then(Value::as_str)
            .is_some_and(|lvl| levels.contains(lvl));
        if !level_match {
            return false;
        }
    }
    if let Some(events) = &filter.events {
        let event_match = item
            .entry
            .get("event")
            .and_then(Value::as_str)
            .is_some_and(|ev| events.contains(ev));
        if !event_match {
            return false;
        }
    }
    if let Some(sid) = &filter.session_id {
        let sid_match = item
            .entry
            .get("session_id")
            .and_then(Value::as_str)
            .is_some_and(|s| s == sid.as_str());
        let conn_match = item
            .entry
            .get("conn_id")
            .and_then(Value::as_str)
            .is_some_and(|c| related_conns.iter().any(|rc| rc == c));
        if !sid_match && !conn_match {
            return false;
        }
    }
    if let Some(needle) = needle_folded
        && !item
            .entry
            .fields
            .iter()
            .any(|(_, value)| value_matches_q(value, needle))
    {
        return false;
    }
    true
}

fn contains_ascii_case_insensitive(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.len() < needle.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(&h, &n)| h.to_ascii_lowercase() == n)
    })
}

fn value_matches_q(value: &Value, needle: &[u8]) -> bool {
    match value {
        Value::String(s) => contains_ascii_case_insensitive(s.as_bytes(), needle),
        Value::Number(n) => {
            let mut buf = [0u8; 64];
            let mut cursor = std::io::Cursor::new(&mut buf[..]);
            use std::io::Write;
            let _ = write!(cursor, "{n}");
            let len = cursor.position() as usize;
            contains_ascii_case_insensitive(&buf[..len], needle)
        }
        Value::Bool(b) => {
            let s: &[u8] = if *b { b"true" } else { b"false" };
            contains_ascii_case_insensitive(s, needle)
        }
        Value::Null => contains_ascii_case_insensitive(b"null", needle),
        Value::Array(arr) => arr.iter().any(|v| value_matches_q(v, needle)),
        Value::Object(map) => map.values().any(|v| value_matches_q(v, needle)),
    }
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
                fields.push(("listener", Value::from("ws")));
                fields.push(("listen", Value::from(listen.to_string())));
            }
            ConfigWarning::AllowAllSource { entry_point, item } => {
                fields.push(("listener", Value::from(entry_point.listener().name())));
                if entry_point.transport().is_websocket() {
                    fields.push(("role", Value::from(entry_point.role().as_str())));
                }
                fields.push(("item", Value::from(item.to_string())));
                fields.push(("phase", Value::from(phase.as_str())));
            }
            ConfigWarning::TrustAllProxies { listener, item } => {
                fields.push(("listener", Value::from(listener.name())));
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
            ConfigWarning::TicketTcpBypass { role } => {
                fields.push(("role", Value::from(role.as_str())));
                let listener = match role {
                    Role::Viewer => Listener::TcpViewer,
                    Role::Server => Listener::TcpServer,
                };
                fields.push(("listener", Value::from(listener.name())));
                fields.push(("phase", Value::from(phase.as_str())));
            }
            ConfigWarning::TicketPlaintextWs { role } => {
                fields.push(("role", Value::from(role.as_str())));
                fields.push(("listener", Value::from(Listener::Ws.name())));
                fields.push(("phase", Value::from(phase.as_str())));
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
        Event::LogReopenFailed { detail } => {
            fields.push(("reason", Value::from("log_file_unavailable")));
            fields.push(("config_key", Value::from("log.output")));
            fields.push(("detail", Value::from(cut(detail, DETAIL_MAX_BYTES))));
        }
        Event::ConnectionAccepted {
            conn_id,
            listener,
            peer_ip,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("listener", Value::from(listener.name())));
            if let Some(role) = listener.fixed_role() {
                fields.push(("role", Value::from(role.as_str())));
            }
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
        }
        Event::TicketAccepted {
            conn_id,
            listener,
            role,
            peer_ip,
            forwarded_ip,
            ticket_jti,
            ticket_sub,
            session_expires_at,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("listener", Value::from(listener.name())));
            fields.push(("role", Value::from(role.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            if let Some(ip) = forwarded_ip {
                fields.push(("forwarded_ip", Value::from(ip.to_string())));
            }
            fields.push(("ticket_jti", Value::from(ticket_jti.as_str())));
            if let Some(sub) = ticket_sub {
                fields.push(("ticket_sub", Value::from(sub.as_str())));
            }
            if let Some(expires_at) = session_expires_at {
                fields.push(("session_expires_at", Value::from(timestamp(*expires_at))));
            }
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
            role,
            reason,
            peer_ip,
            forwarded_ip,
            origin,
            ticket_jti,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("listener", Value::from(*listener)));
            if let Some(role) = role {
                fields.push(("role", Value::from(role.as_str())));
            }
            fields.push(("stage", Value::from(Stage::Upgrade.as_str())));
            fields.push(("reason", Value::from(reason.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            if let Some(ip) = forwarded_ip {
                fields.push(("forwarded_ip", Value::from(ip.to_string())));
            }
            if let Some(origin) = origin {
                fields.push((
                    "origin",
                    Value::from(cut(origin.as_str(), ORIGIN_MAX_BYTES)),
                ));
            }
            if let Some(jti) = ticket_jti {
                fields.push(("ticket_jti", Value::from(jti.as_str())));
            }
        }
        Event::HandshakeRejected {
            conn_id,
            entry_point,
            peer_ip,
            forwarded_ip,
            reason,
            mode,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("listener", Value::from(entry_point.listener().name())));
            fields.push(("role", Value::from(entry_point.role().as_str())));
            fields.push(("stage", Value::from(Stage::InitialExchange.as_str())));
            fields.push(("reason", Value::from(reason.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            if let Some(ip) = forwarded_ip {
                fields.push(("forwarded_ip", Value::from(ip.to_string())));
            }
            if let Some(mode) = mode {
                fields.push(("mode", Value::from(mode.as_str())));
            }
        }
        Event::TicketRelayMismatch {
            conn_id,
            entry_point,
            peer_ip,
            forwarded_ip,
            mode,
            relay_id_ref,
            ticket_jti,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("listener", Value::from(entry_point.listener().name())));
            fields.push(("role", Value::from(entry_point.role().as_str())));
            fields.push(("stage", Value::from(Stage::InitialExchange.as_str())));
            fields.push((
                "reason",
                Value::from(RejectReason::TicketRelayMismatch.as_str()),
            ));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            if let Some(ip) = forwarded_ip {
                fields.push(("forwarded_ip", Value::from(ip.to_string())));
            }
            fields.push(("mode", Value::from(mode.as_str())));
            if let Some(reference) = relay_id_ref {
                fields.push(("relay_id_ref", Value::from(reference.as_str())));
            }
            fields.push(("ticket_jti", Value::from(ticket_jti.as_str())));
        }
        Event::ConnectionAborted {
            conn_id,
            listener,
            role,
            stage,
            reason,
            peer_ip,
            forwarded_ip,
            elapsed_ms,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("listener", Value::from(listener.name())));
            fields.push(("stage", Value::from(stage.as_str())));
            fields.push(("reason", Value::from(reason.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            if let Some(ip) = forwarded_ip {
                fields.push(("forwarded_ip", Value::from(ip.to_string())));
            }
            fields.push(("elapsed_ms", Value::from(*elapsed_ms)));
            if let Some(role) = role {
                fields.push(("role", Value::from(role.as_str())));
            }
            if let Some(kind) = reason.error_kind() {
                fields.push(("error_kind", Value::from(kind.as_str())));
            }
        }
        Event::TargetRejected {
            conn_id,
            entry_point,
            peer_ip,
            forwarded_ip,
            reason,
            target_host,
            target_port,
            target_ip,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("listener", Value::from(entry_point.listener().name())));
            fields.push(("role", Value::from(entry_point.role().as_str())));
            fields.push(("mode", Value::from(Mode::Mode1.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            if let Some(ip) = forwarded_ip {
                fields.push(("forwarded_ip", Value::from(ip.to_string())));
            }
            fields.push(("stage", Value::from(Stage::TargetConnect.as_str())));
            fields.push(("reason", Value::from(reason.as_str())));
            if let Some(limit) = reason.limit() {
                fields.push(("limit", Value::from(limit)));
            }
            if *reason == RejectReason::TargetNotAllowed {
                fields.push((
                    "target_host",
                    Value::from(cut(target_host, TARGET_HOST_MAX_BYTES)),
                ));
                fields.push(("target_port", Value::from(*target_port)));
                if let Some(ip) = target_ip {
                    fields.push(("target_ip", Value::from(PeerIp::new(*ip).to_string())));
                }
            }
        }
        Event::RegistrationRejected {
            conn_id,
            entry_point,
            peer_ip,
            forwarded_ip,
            reason,
            relay_id_ref,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("listener", Value::from(entry_point.listener().name())));
            fields.push(("role", Value::from(entry_point.role().as_str())));
            fields.push(("mode", Value::from(Mode::Mode2.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            if let Some(ip) = forwarded_ip {
                fields.push(("forwarded_ip", Value::from(ip.to_string())));
            }
            fields.push(("stage", Value::from(Stage::Register.as_str())));
            fields.push(("reason", Value::from(reason.as_str())));
            if let Some(limit) = reason.limit() {
                fields.push(("limit", Value::from(limit)));
            }
            if *reason == RejectReason::DuplicateId {
                fields.push(("relay_id_ref", Value::from(relay_id_ref.as_str())));
            }
        }
        Event::RelayAborted {
            conn_id,
            entry_point,
            stage,
            reason,
            peer_ip,
            forwarded_ip,
            elapsed_ms,
            mode,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("listener", Value::from(entry_point.listener().name())));
            fields.push(("stage", Value::from(stage.as_str())));
            fields.push(("reason", Value::from(reason.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            if let Some(ip) = forwarded_ip {
                fields.push(("forwarded_ip", Value::from(ip.to_string())));
            }
            fields.push(("elapsed_ms", Value::from(*elapsed_ms)));
            fields.push(("role", Value::from(entry_point.role().as_str())));
            fields.push(("mode", Value::from(mode.as_str())));
            if let Some(kind) = reason.error_kind() {
                fields.push(("error_kind", Value::from(kind.as_str())));
            }
        }
        Event::TargetConnectFailed {
            conn_id,
            entry_point,
            viewer_transport,
            peer_ip,
            forwarded_ip,
            target_host,
            target_port,
            target_ip,
            elapsed_ms,
            reason,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("listener", Value::from(entry_point.listener().name())));
            fields.push(("viewer_transport", Value::from(viewer_transport.as_str())));
            fields.push(("reason", Value::from(reason.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            if let Some(ip) = forwarded_ip {
                fields.push(("forwarded_ip", Value::from(ip.to_string())));
            }
            fields.push((
                "target_host",
                Value::from(cut(target_host, TARGET_HOST_MAX_BYTES)),
            ));
            fields.push(("target_port", Value::from(*target_port)));
            fields.push(("elapsed_ms", Value::from(*elapsed_ms)));
            if let Some(ip) = target_ip {
                fields.push(("target_ip", Value::from(PeerIp::new(*ip).to_string())));
            }
        }
        Event::WaitingRegistered {
            conn_id,
            entry_point,
            peer_ip,
            forwarded_ip,
            relay_id_ref,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("listener", Value::from(entry_point.listener().name())));
            fields.push(("role", Value::from(entry_point.role().as_str())));
            fields.push(("mode", Value::from(Mode::Mode2.as_str())));
            fields.push(("relay_id_ref", Value::from(relay_id_ref.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            if let Some(ip) = forwarded_ip {
                fields.push(("forwarded_ip", Value::from(ip.to_string())));
            }
        }
        Event::WaitingExpired {
            conn_id,
            role,
            relay_id_ref,
            waited_ms,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("role", Value::from(role.as_str())));
            fields.push(("relay_id_ref", Value::from(relay_id_ref.as_str())));
            fields.push(("waited_ms", Value::from(*waited_ms)));
        }
        Event::WaitingRemoved {
            conn_id,
            role,
            relay_id_ref,
            waited_ms,
            reason,
        } => {
            fields.push(("conn_id", Value::from(conn_id.to_string())));
            fields.push(("role", Value::from(role.as_str())));
            fields.push(("relay_id_ref", Value::from(relay_id_ref.as_str())));
            fields.push(("waited_ms", Value::from(*waited_ms)));
            fields.push(("reason", Value::from(reason.as_str())));
            if let Some(kind) = reason.error_kind() {
                fields.push(("error_kind", Value::from(kind.as_str())));
            }
        }
        Event::SessionStarted {
            session_id,
            mode,
            viewer_transport,
            server_transport,
            viewer_conn_id,
            viewer_peer_ip,
            viewer_forwarded_ip,
            server_conn_id,
            server_peer_ip,
            server_forwarded_ip,
            relay_id_ref,
            waited_ms,
            target_ip,
            target_port,
            target_host,
        } => {
            fields.push(("session_id", Value::from(session_id.as_str())));
            fields.push(("mode", Value::from(mode.as_str())));
            fields.push(("viewer_transport", Value::from(viewer_transport.as_str())));
            fields.push(("viewer_conn_id", Value::from(viewer_conn_id.to_string())));
            fields.push(("viewer_peer_ip", Value::from(viewer_peer_ip.to_string())));
            if let Some(ip) = viewer_forwarded_ip {
                fields.push(("viewer_forwarded_ip", Value::from(ip.to_string())));
            }
            match mode {
                Mode::Mode1 => {
                    if let Some(ip) = target_ip {
                        fields.push(("target_ip", Value::from(PeerIp::new(*ip).to_string())));
                    }
                    if let Some(port) = target_port {
                        fields.push(("target_port", Value::from(*port)));
                    }
                    if let Some(host) = target_host {
                        fields.push(("target_host", Value::from(cut(host, TARGET_HOST_MAX_BYTES))));
                    }
                }
                Mode::Mode2 => {
                    if let Some(transport) = server_transport {
                        fields.push(("server_transport", Value::from(transport.as_str())));
                    }
                    if let Some(id) = server_conn_id {
                        fields.push(("server_conn_id", Value::from(id.to_string())));
                    }
                    if let Some(ip) = server_peer_ip {
                        fields.push(("server_peer_ip", Value::from(ip.to_string())));
                    }
                    if let Some(ip) = server_forwarded_ip {
                        fields.push(("server_forwarded_ip", Value::from(ip.to_string())));
                    }
                    if let Some(reference) = relay_id_ref {
                        fields.push(("relay_id_ref", Value::from(reference.as_str())));
                    }
                    if let Some(waited) = waited_ms {
                        fields.push(("waited_ms", Value::from(*waited)));
                    }
                }
            }
        }
        Event::SessionClosed {
            session_id,
            mode,
            viewer_transport,
            started_at,
            duration_ms,
            bytes_viewer_to_target,
            bytes_target_to_viewer,
            close_cause,
            side,
            error_kind,
            relay_id_ref,
            admin_account,
        } => {
            fields.push(("session_id", Value::from(session_id.as_str())));
            fields.push(("mode", Value::from(mode.as_str())));
            fields.push(("viewer_transport", Value::from(viewer_transport.as_str())));
            fields.push(("started_at", Value::from(timestamp(*started_at))));
            fields.push(("duration_ms", Value::from(*duration_ms)));
            fields.push((
                "bytes_viewer_to_target",
                Value::from(*bytes_viewer_to_target),
            ));
            fields.push((
                "bytes_target_to_viewer",
                Value::from(*bytes_target_to_viewer),
            ));
            fields.push(("close_cause", Value::from(close_cause.as_str())));
            if *mode == Mode::Mode2
                && let Some(reference) = relay_id_ref
            {
                fields.push(("relay_id_ref", Value::from(reference.as_str())));
            }
            match close_cause {
                CloseCause::PeerEof
                | CloseCause::ConnectionError
                | CloseCause::DeadConnection
                | CloseCause::CloseTimeout => {
                    if let Some(side) = side {
                        fields.push(("side", Value::from(side.as_str())));
                    }
                }
                CloseCause::AdminTerminated
                | CloseCause::ServerShutdown
                | CloseCause::InternalError
                | CloseCause::SessionExpired => {}
            }
            if *close_cause == CloseCause::ConnectionError
                && let Some(kind) = error_kind
            {
                fields.push(("error_kind", Value::from(kind.as_str())));
            }
            if *close_cause == CloseCause::AdminTerminated
                && let Some(account) = admin_account
            {
                fields.push(("admin_account", Value::from(account.as_str())));
            }
        }
        Event::ShutdownStarted {
            trigger,
            grace_secs,
            active_sessions,
        } => {
            match trigger {
                vnc_relay_app::events::ShutdownTrigger::Requested => {
                    fields.push(("trigger", Value::from("requested")));
                }
                vnc_relay_app::events::ShutdownTrigger::FatalError { detail } => {
                    fields.push(("trigger", Value::from("fatal_error")));
                    fields.push(("detail", Value::from(detail.as_str())));
                }
            }
            fields.push(("grace_secs", Value::from(*grace_secs)));
            fields.push(("active_sessions", Value::from(*active_sessions)));
        }
        Event::ShutdownForced { remaining_sessions } => {
            fields.push(("remaining_sessions", Value::from(*remaining_sessions)));
        }
        Event::AdminTerminate {
            admin_account,
            peer_ip,
            session_id,
            result,
        } => {
            fields.push(("admin_account", Value::from(admin_account.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            fields.push(("result", Value::from(result.as_str())));
            if let Some(id) = session_id {
                fields.push(("session_id", Value::from(id.as_str())));
            }
        }
        Event::AdminLoginSucceeded {
            admin_account,
            peer_ip,
        } => {
            fields.push(("admin_account", Value::from(admin_account.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
        }
        Event::ConfigEdited {
            admin_account,
            peer_ip,
            config_keys,
            revision,
        } => {
            fields.push(("admin_account", Value::from(admin_account.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            fields.push(("config_keys", json!(config_keys)));
            fields.push(("revision", Value::from(revision.as_str())));
        }
        Event::ConfigEditRejected {
            admin_account,
            peer_ip,
            reason,
            config_keys,
        } => {
            fields.push(("admin_account", Value::from(admin_account.as_str())));
            fields.push(("peer_ip", Value::from(peer_ip.to_string())));
            fields.push(("reason", Value::from(reason.as_str())));
            if let Some(config_keys) = config_keys {
                fields.push(("config_keys", json!(config_keys)));
            }
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
        Event::ReloadChangeDetected { source } => {
            fields.push(("source", Value::from(source.as_str())));
        }
        Event::ReloadApplied { trigger, applied } => {
            fields.push(("trigger", Value::from(trigger.as_str())));
            let applied = applied
                .iter()
                .map(|key| Value::from(key.as_str()))
                .collect();
            fields.push(("applied", Value::Array(applied)));
        }
        Event::ReloadRejected {
            reason,
            config_key,
            detail,
        } => {
            fields.push(("reason", Value::from(reason.as_str())));
            if let Some(config_key) = config_key {
                fields.push(("config_key", Value::from(config_key.as_str())));
            }
            if let Some(detail) = detail {
                fields.push(("detail", Value::from(cut(detail, DETAIL_MAX_BYTES))));
            }
        }
        Event::ReloadRestartRequired { config_keys } => {
            let keys = config_keys
                .iter()
                .map(|key| Value::from(key.as_str()))
                .collect();
            fields.push(("config_keys", Value::Array(keys)));
        }
        Event::ReloadWatchFailed { reason, detail } => {
            fields.push(("reason", Value::from(reason.as_str())));
            if let Some(detail) = detail {
                fields.push(("detail", Value::from(cut(detail, DETAIL_MAX_BYTES))));
            }
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
    use vnc_relay_app::events::{LoginFailReason, RejectReason};
    use vnc_relay_domain::account::AccountName;
    use vnc_relay_domain::conn_id::ConnId;
    use vnc_relay_domain::login_limit::LimitScope;
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
        // ops-info-v2 §1.1: `origin` is cut to 256 bytes at a character boundary.
        let line = |origin: String| {
            format_line(
                &Event::UpgradeRejected {
                    listener: "ws",
                    role: Some(vnc_relay_domain::entry_point::Role::Viewer),
                    conn_id: ConnId::next(),
                    reason: RejectReason::OriginNotAllowed,
                    peer_ip: PeerIp::new(IpAddr::from([127, 0, 0, 1])),
                    forwarded_ip: None,
                    origin: Some(origin),
                    ticket_jti: None,
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

    #[test]
    fn admin_login_lines_carry_only_their_own_fields() {
        // ops-info-v2 §1.8: `admin_account` is set only for a real account,
        // and `scope` is set only for a refusal by a limit.
        let peer_ip = || PeerIp::new(IpAddr::from([127, 0, 0, 1]));
        let alice = AccountName::parse("alice")
            .unwrap_or_else(|error| panic!("test account name is valid: {error}"));
        let failed = format_line(
            &Event::AdminLoginFailed {
                reason: LoginFailReason::InvalidCredentials,
                scope: None,
                admin_account: Some(alice),
                peer_ip: peer_ip(),
            },
            UNIX_EPOCH,
        );
        let tail = concat!(
            "\"level\":\"warn\",\"event\":\"admin_login_failed\",",
            "\"reason\":\"invalid_credentials\",\"admin_account\":\"alice\",",
            "\"peer_ip\":\"127.0.0.1\"}\n",
        );
        assert!(failed.ends_with(tail), "{failed}");

        let limited = format_line(
            &Event::AdminLoginFailed {
                reason: LoginFailReason::Limited,
                scope: Some(LimitScope::Account),
                admin_account: None,
                peer_ip: peer_ip(),
            },
            UNIX_EPOCH,
        );
        let tail = concat!(
            "\"level\":\"warn\",\"event\":\"admin_login_failed\",",
            "\"reason\":\"limited\",\"scope\":\"account\",",
            "\"peer_ip\":\"127.0.0.1\"}\n",
        );
        assert!(limited.ends_with(tail), "{limited}");

        let lock = format_line(
            &Event::AdminLoginLimited {
                scope: LimitScope::Source,
                lockout_secs: 60,
                admin_account: None,
                peer_ip: peer_ip(),
            },
            UNIX_EPOCH,
        );
        let tail = concat!(
            "\"level\":\"warn\",\"event\":\"admin_login_limited\",",
            "\"scope\":\"source\",\"lockout_secs\":60,",
            "\"peer_ip\":\"127.0.0.1\"}\n",
        );
        assert!(lock.ends_with(tail), "{lock}");
    }

    /// spec: operations.md:OPS-068 partial
    #[test]
    fn recent_logs_record_order_and_seq() {
        let buffer = Arc::new(RecentLogs::new(10));
        let sink = JsonLinesEventSink::stdout(|_| {}).with_recent_logs(Arc::clone(&buffer));

        let conn_id = ConnId::next();
        sink.record(&Event::ServerStarting);
        sink.record(&Event::ConnectionAccepted {
            conn_id,
            listener: vnc_relay_domain::entry_point::Listener::TcpViewer,
            peer_ip: PeerIp::new(IpAddr::from([127, 0, 0, 1])),
        });
        sink.record(&Event::ServerStopped);

        let (entries, oldest, latest) = buffer.snapshot();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].seq, 1);
        assert_eq!(entries[1].seq, 2);
        assert_eq!(entries[2].seq, 3);
        assert_eq!(oldest, Some(1));
        assert_eq!(latest, Some(3));
        assert_eq!(entries[0].entry.get("event").unwrap(), "server_starting");
        assert_eq!(
            entries[1].entry.get("event").unwrap(),
            "connection_accepted"
        );
        assert_eq!(entries[2].entry.get("event").unwrap(), "server_stopped");
    }

    /// spec: operations.md:OPS-068 partial
    #[test]
    fn recent_logs_entry_content_equals_log_line_object() {
        let buffer = Arc::new(RecentLogs::new(10));
        let path = std::env::temp_dir().join(format!(
            "vnc-relay-log-entry-{}-{}.jsonl",
            std::process::id(),
            ConnId::next()
        ));
        let sink = JsonLinesEventSink::open(&LogOutput::File(path.clone()), |_| {}, |_| {})
            .unwrap()
            .with_recent_logs(Arc::clone(&buffer));
        let event = Event::UpgradeRejected {
            conn_id: ConnId::next(),
            listener: "wss",
            role: Some(vnc_relay_domain::entry_point::Role::Server),
            reason: RejectReason::IpNotAllowed,
            peer_ip: PeerIp::new("127.0.0.1".parse().unwrap()),
            forwarded_ip: Some(PeerIp::new("198.51.100.8".parse().unwrap())),
            origin: None,
            ticket_jti: None,
        };
        sink.record(&event);

        let (entries, _, _) = buffer.snapshot();
        assert_eq!(entries.len(), 1);

        drop(sink);
        let line = std::fs::read_to_string(&path).unwrap();
        let parsed: Value = serde_json::from_str(&line).unwrap();
        let retained: Value = serde_json::from_str(&entries[0].entry.to_string()).unwrap();
        assert_eq!(retained, parsed);
        std::fs::remove_file(path).unwrap();
    }

    /// spec: operations.md:OPS-068 partial
    #[test]
    fn recent_logs_eviction_at_max() {
        let buffer = Arc::new(RecentLogs::new(3));
        let sink = JsonLinesEventSink::stdout(|_| {}).with_recent_logs(Arc::clone(&buffer));

        sink.record(&Event::ServerStarting); // seq 1
        sink.record(&Event::ServerStopped); // seq 2
        sink.record(&Event::ServerStarting); // seq 3
        assert_eq!(buffer.snapshot().1, Some(1));
        assert_eq!(buffer.snapshot().2, Some(3));

        sink.record(&Event::ServerStopped); // seq 4 -> evicts 1
        let (entries, oldest, latest) = buffer.snapshot();
        assert_eq!(entries.len(), 3);
        assert_eq!(oldest, Some(2));
        assert_eq!(latest, Some(4));
        assert_eq!(
            entries.iter().map(|e| e.seq).collect::<Vec<_>>(),
            vec![2, 3, 4]
        );

        sink.record(&Event::ServerStarting); // seq 5 -> evicts 2
        let (entries, oldest, latest) = buffer.snapshot();
        assert_eq!(entries.len(), 3);
        assert_eq!(oldest, Some(3));
        assert_eq!(latest, Some(5));
        assert_eq!(
            entries.iter().map(|e| e.seq).collect::<Vec<_>>(),
            vec![3, 4, 5]
        );
    }

    /// spec: operations.md:OPS-068 partial
    #[test]
    fn recent_logs_zero_max_entries_keeps_nothing() {
        let buffer = Arc::new(RecentLogs::new(0));
        let sink = JsonLinesEventSink::stdout(|_| {}).with_recent_logs(Arc::clone(&buffer));

        sink.record(&Event::ServerStarting);
        sink.record(&Event::ServerStopped);

        let (entries, oldest, latest) = buffer.snapshot();
        assert!(entries.is_empty());
        assert_eq!(oldest, None);
        assert_eq!(latest, None);
        assert_eq!(buffer.max_entries(), 0);

        let query = buffer.query(&LogsFilter::default(), UNIX_EPOCH);
        assert_eq!(query.max_entries, 0);
        assert_eq!(query.oldest_seq, None);
        assert_eq!(query.latest_seq, None);
        assert!(!query.has_more);
        assert!(query.items.is_empty());
    }

    /// spec: operations.md:OPS-068 partial
    #[test]
    fn recent_logs_records_independently_of_output_target_and_write_failure() {
        let buffer = Arc::new(RecentLogs::new(10));
        let failed_called = Arc::new(AtomicBool::new(false));
        let failed_clone = Arc::clone(&failed_called);

        let exe = std::env::current_exe().unwrap();
        let sink = JsonLinesEventSink {
            output: Mutex::new(Output::File {
                handle: OpenOptions::new().read(true).open(exe).unwrap(),
                reopen_failed: false,
            }),
            failed: AtomicBool::new(false),
            on_first_failure: Box::new(move |_| {
                failed_clone.store(true, Ordering::Relaxed);
            }),
            path: None,
            on_reopen_failure: None,
            recent_logs: Some(Arc::clone(&buffer)),
        };

        sink.record(&Event::ServerStarting);
        assert!(failed_called.load(Ordering::Relaxed));
        let (entries, oldest, latest) = buffer.snapshot();
        assert_eq!(entries.len(), 1);
        assert_eq!(oldest, Some(1));
        assert_eq!(latest, Some(1));
        assert_eq!(entries[0].entry.get("event").unwrap(), "server_starting");
    }

    #[test]
    fn recent_logs_query_filters_alone_and_combined() {
        let buffer = Arc::new(RecentLogs::new(100));
        let sid1 = vnc_relay_domain::session::SessionId::from_block([1; 16]);
        let conn1 = ConnId::next();
        let conn2 = ConnId::next();

        for fields in [
            vec![
                ("event", json!("connection_accepted")),
                ("level", json!("info")),
                ("conn_id", json!(conn1.to_string())),
            ],
            vec![
                ("event", json!("session_started")),
                ("level", json!("info")),
                ("session_id", json!(sid1.to_string())),
                ("viewer_conn_id", json!(conn1.to_string())),
            ],
            vec![
                ("event", json!("connection_rejected")),
                ("level", json!("warn")),
                ("conn_id", json!(conn2.to_string())),
                ("reason", json!("ip_not_allowed")),
            ],
            vec![
                ("event", json!("startup_failed")),
                ("level", json!("error")),
                ("reason", json!("runtime_unavailable")),
            ],
        ] {
            buffer.record(Arc::new(LogEntry { fields }));
        }

        // level filter
        let mut filter = LogsFilter {
            limit: 10,
            ..Default::default()
        };
        filter.levels = Some(BTreeSet::from(["warn".to_owned()]));
        let res = buffer.query(&filter, UNIX_EPOCH);
        assert_eq!(res.items.len(), 1);
        assert_eq!(res.items[0].entry["event"], "connection_rejected");

        // event filter
        let mut filter = LogsFilter {
            limit: 10,
            ..Default::default()
        };
        filter.events = Some(BTreeSet::from(["connection_accepted".to_owned()]));
        let res = buffer.query(&filter, UNIX_EPOCH);
        assert_eq!(res.items.len(), 1);
        assert_eq!(res.items[0].entry["event"], "connection_accepted");

        // unknown event -> empty
        let mut filter = LogsFilter {
            limit: 10,
            ..Default::default()
        };
        filter.events = Some(BTreeSet::from(["unknown_event".to_owned()]));
        let res = buffer.query(&filter, UNIX_EPOCH);
        assert_eq!(res.items.len(), 0);
        assert_eq!(res.oldest_seq, Some(1));
        assert_eq!(res.latest_seq, Some(4));

        // q search (ASCII case-folded)
        let mut filter = LogsFilter {
            limit: 10,
            ..Default::default()
        };
        filter.q = Some("IP_NOT".to_owned());
        let res = buffer.query(&filter, UNIX_EPOCH);
        assert_eq!(res.items.len(), 1);
        assert_eq!(res.items[0].entry["reason"], "ip_not_allowed");

        // session_id with conn relation
        let mut filter = LogsFilter {
            limit: 10,
            ..Default::default()
        };
        filter.session_id = Some(sid1.to_string());
        let res = buffer.query(&filter, UNIX_EPOCH);
        assert_eq!(res.items.len(), 2);
        assert_eq!(res.items[0].entry["event"], "connection_accepted");
        assert_eq!(res.items[1].entry["event"], "session_started");
        filter.before = Some(2);
        let before = buffer.query(&filter, UNIX_EPOCH);
        assert_eq!(
            before.items.len(),
            1,
            "linkage survives a session_started outside the cursor range"
        );
        assert_eq!(before.items[0].seq, 1);
        buffer.record(Arc::new(LogEntry {
            fields: vec![
                ("event", json!("connection_aborted")),
                ("conn_id", json!(conn1.to_string())),
            ],
        }));
        filter.before = None;
        filter.after = Some(4);
        let after = buffer.query(&filter, UNIX_EPOCH);
        assert_eq!(after.items.len(), 1);
        assert_eq!(after.items[0].seq, 5);
        assert_eq!(after.oldest_seq, Some(1));
        assert_eq!(after.latest_seq, Some(5));
        filter.after = Some(5);
        let empty = buffer.query(&filter, UNIX_EPOCH);
        assert!(empty.items.is_empty());
        assert!(!empty.has_more);
        assert_eq!(empty.oldest_seq, Some(1));
        assert_eq!(empty.latest_seq, Some(5));
    }

    /// spec: operations.md:OPS-068 partial
    #[test]
    fn concurrent_recent_log_sequences_equal_file_line_order() {
        let path = std::env::temp_dir().join(format!(
            "vnc-relay-log-order-{}-{}.jsonl",
            std::process::id(),
            ConnId::next()
        ));
        let logs = Arc::new(RecentLogs::new(512));
        let sink = JsonLinesEventSink::open(&LogOutput::File(path.clone()), |_| {}, |_| {})
            .unwrap()
            .with_recent_logs(logs.clone());
        let barrier = std::sync::Barrier::new(8);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    barrier.wait();
                    for _ in 0..64 {
                        sink.record(&Event::ConnectionAccepted {
                            conn_id: ConnId::next(),
                            listener: vnc_relay_domain::entry_point::Listener::Ws,
                            peer_ip: PeerIp::new("127.0.0.1".parse().unwrap()),
                        });
                    }
                });
            }
        });
        drop(sink);
        let written = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<Value> = written
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let retained = logs.query(
            &LogsFilter {
                limit: 512,
                ..Default::default()
            },
            UNIX_EPOCH,
        );
        assert_eq!(retained.items.len(), 512);
        assert_eq!(lines.len(), 512);
        for (index, item) in retained.items.iter().enumerate() {
            assert_eq!(item.seq, (index + 1) as u64);
            assert_eq!(
                serde_json::from_str::<Value>(&item.entry.to_string()).unwrap(),
                lines[index]
            );
        }
        std::fs::remove_file(path).unwrap();
    }
}
