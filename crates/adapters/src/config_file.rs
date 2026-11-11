//! Config file loader (config-v4 §1 and §2.1).
//! Design: docs/architectures/config-loading.md.

mod listen;
mod reader;

use std::io::ErrorKind;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use vnc_relay_app::reload::{ConfigCandidate, ConfigPresence};
use vnc_relay_app::settings::{
    AdminSessionSettings, AdminSettings, Argon2Settings, ConfigDiagnostic, ConfigError,
    DashboardSettings, EntryPointSettings, KeepaliveSettings, LimitSettings, LogOutput,
    LogSettings, LoginLimitSettings, ManagementSettings, Mode1Settings, Mode2Settings,
    RecentClosedSettings, RecentLogsSettings, RelaySettings, ReloadSettings, Settings,
    StatusSettings, TicketRoleSettings, TicketSettings, TimeoutSettings, TlsFiles,
    WsListenerSettings, WsRoleSettings,
};
use vnc_relay_domain::account::Argon2Params;
use vnc_relay_domain::allow_list::{AllowList, IpItem, IpItemError};
use vnc_relay_domain::entry_point::Listener;
use vnc_relay_domain::origin::Origin;
use vnc_relay_domain::target_policy::{TargetItem, TargetItemError, TargetPolicy};

use crate::toml_text::error_position;
use listen::parse_listen;
use reader::{Diagnostics, Field, Section, out_of_range};

/// Reads and checks the config file at `path`. Relative paths inside the
/// file are resolved against the file's directory.
pub fn load(path: &Path) -> Result<Settings, Vec<ConfigDiagnostic>> {
    let (text, absolute) = read_text(path)?;
    parse(&text, base_dir(&absolute))
}

/// Reads settings and presence from the same file read at startup.
pub fn load_candidate(path: &Path) -> Result<ConfigCandidate, Vec<ConfigDiagnostic>> {
    let (text, absolute) = read_text(path)?;
    parse_candidate(text.as_bytes(), base_dir(&absolute))
}

/// Reads the config file for the `admin` commands: the whole TOML syntax and
/// the `[admin]` table only. Other tables are not checked (cli-v1 §1.5).
pub fn load_admin(path: &Path) -> Result<AdminSettings, Vec<ConfigDiagnostic>> {
    let (text, absolute) = read_text(path)?;
    let table = parse_table(&text)?;
    let mut d = Diagnostics::default();
    let mut root = Section::root(&table);
    let admin = read_admin(&mut root, base_dir(&absolute), &mut d);
    match admin {
        Some(admin) if d.is_empty() => Ok(admin),
        _ => Err(d.into_vec()),
    }
}

/// The file text and the absolute path of `path`.
fn read_text(path: &Path) -> Result<(String, PathBuf), Vec<ConfigDiagnostic>> {
    let unreadable = |message: String| {
        vec![ConfigDiagnostic {
            error: ConfigError::FileUnreadable,
            key: String::new(),
            message,
        }]
    };
    let absolute = std::path::absolute(path)
        .map_err(|_| unreadable(format!("설정 파일 경로를 풀 수 없다: {}", path.display())))?;
    let bytes = std::fs::read(&absolute).map_err(|error| {
        let reason = match error.kind() {
            ErrorKind::NotFound => "파일이 없다",
            ErrorKind::PermissionDenied => "읽을 권한이 없다",
            ErrorKind::IsADirectory => "파일이 아니라 디렉터리다",
            _ => "읽을 수 없다",
        };
        unreadable(format!(
            "설정 파일을 읽을 수 없다({reason}): {}",
            absolute.display()
        ))
    })?;
    let text = String::from_utf8(bytes).map_err(|_| {
        unreadable(format!(
            "설정 파일이 UTF-8 텍스트가 아니다: {}",
            absolute.display()
        ))
    })?;
    Ok((text, absolute))
}

fn base_dir(absolute: &Path) -> &Path {
    absolute.parent().unwrap_or(Path::new(""))
}

/// Checks config text. `base_dir` is the directory that relative paths are
/// resolved against. Returns every diagnostic found, or the settings.
pub fn parse(text: &str, base_dir: &Path) -> Result<Settings, Vec<ConfigDiagnostic>> {
    let table = parse_table(text)?;
    settings_from_table(&table, base_dir)
}

fn settings_from_table(
    table: &toml::Table,
    base_dir: &Path,
) -> Result<Settings, Vec<ConfigDiagnostic>> {
    let mut d = Diagnostics::default();
    let mut root = Section::root(table);
    let settings = read_settings(&mut root, base_dir, &mut d);
    root.finish(&mut d);
    match settings {
        Some(settings) if d.is_empty() => Ok(settings),
        _ => Err(d.into_vec()),
    }
}

/// Checks raw config bytes and records every explicitly written leaf key,
/// including values of disabled features (management-api-v1 §1.13).
pub fn parse_candidate(
    bytes: &[u8],
    base_dir: &Path,
) -> Result<ConfigCandidate, Vec<ConfigDiagnostic>> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        vec![ConfigDiagnostic {
            error: ConfigError::FileUnreadable,
            key: String::new(),
            message: "설정 파일이 UTF-8 텍스트가 아니다.".to_owned(),
        }]
    })?;
    let table = parse_table(text)?;
    let settings = settings_from_table(&table, base_dir)?;
    let mut presence = ConfigPresence::default();
    collect_presence(&table, "", &mut presence);
    if !settings.relay.wss.enabled()
        && settings.relay.wss.tls.is_none()
        && let Some(wss) = table
            .get("relay")
            .and_then(|relay| relay.get("wss"))
            .and_then(toml::Value::as_table)
    {
        for key in ["cert_file", "key_file"] {
            if let Some(path) = wss.get(key).and_then(toml::Value::as_str) {
                presence
                    .inactive_paths
                    .insert(format!("relay.wss.{key}"), base_dir.join(path));
            }
        }
    }
    Ok(ConfigCandidate { settings, presence })
}

fn collect_presence(table: &toml::Table, prefix: &str, presence: &mut ConfigPresence) {
    for (key, value) in table {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        if let toml::Value::Table(table) = value {
            collect_presence(table, &path, presence);
        } else {
            presence.keys.insert(path);
        }
    }
}

fn parse_table(text: &str) -> Result<toml::Table, Vec<ConfigDiagnostic>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    text.parse().map_err(|error: toml::de::Error| {
        vec![ConfigDiagnostic {
            error: ConfigError::SyntaxError,
            key: String::new(),
            message: syntax_message(text, &error),
        }]
    })
}

/// Position only; the message must not quote file content (config-v4 §2.1).
fn syntax_message(text: &str, error: &toml::de::Error) -> String {
    match error_position(text, error) {
        Some((line, column)) => format!("TOML 구문 오류 또는 중복 키다({line}행 {column}열)."),
        None => "TOML 구문 오류 또는 중복 키다.".to_owned(),
    }
}

/// Reads every table. Returns `None` when any value is invalid or missing;
/// in that case at least one diagnostic was pushed.
fn read_settings(root: &mut Section<'_>, base_dir: &Path, d: &mut Diagnostics) -> Option<Settings> {
    let admin = read_admin(root, base_dir, d);
    let relay = read_relay(root, base_dir, d);
    let mode1 = read_mode1(root, d);
    let mode2 = read_mode2(root, d);
    let limits = read_limits(root, d);
    let timeouts = read_timeouts(root, d);
    let keepalive = read_keepalive(root, d);
    let management = read_management(root, base_dir, d);
    let dashboard = read_dashboard(root, d);
    let recent_closed = read_recent_closed(root, d);
    let recent_logs = read_recent_logs(root, d);
    let status = read_status(root, d);
    let log = read_log(root, base_dir, d);
    let reload = read_reload(root, d);

    if let (Some(relay), Some(management), Some(status)) = (&relay, &management, &status) {
        check_listen_conflicts(relay, management, status, d);
    }

    Some(Settings {
        admin: admin?,
        relay: relay?,
        mode1: mode1?,
        mode2: mode2?,
        limits: limits?,
        timeouts: timeouts?,
        keepalive: keepalive?,
        management: management?,
        dashboard: dashboard?,
        recent_closed: recent_closed?,
        recent_logs: recent_logs?,
        status: status?,
        log: log?,
        reload: reload?,
    })
}

fn read_admin(
    root: &mut Section<'_>,
    base_dir: &Path,
    d: &mut Diagnostics,
) -> Option<AdminSettings> {
    let mut admin = root.section("admin", d);
    let accounts_file = read_path(&mut admin, "accounts_file", base_dir, d);
    let accounts_file = required(&admin, "accounts_file", accounts_file, "", d);
    let mut argon2 = admin.section("argon2", d);
    let memory_kib = count(
        &mut argon2,
        "memory_kib",
        19_456,
        Argon2Params::MEMORY_KIB,
        d,
    );
    let iterations = count(&mut argon2, "iterations", 2, Argon2Params::ITERATIONS, d);
    let parallelism = count(&mut argon2, "parallelism", 1, Argon2Params::PARALLELISM, d);
    argon2.finish(d);
    admin.finish(d);
    Some(AdminSettings {
        accounts_file: accounts_file?,
        argon2: Argon2Settings {
            memory_kib: memory_kib?,
            iterations: iterations?,
            parallelism: parallelism?,
        },
    })
}

fn read_relay(
    root: &mut Section<'_>,
    base_dir: &Path,
    d: &mut Diagnostics,
) -> Option<RelaySettings> {
    let mut relay = root.section("relay", d);
    let tcp_viewer = read_tcp_listener(&mut relay, Listener::TcpViewer, d);
    let tcp_server = read_tcp_listener(&mut relay, Listener::TcpServer, d);
    let ws = read_websocket_listener(&mut relay, Listener::Ws, base_dir, d);
    let wss = read_websocket_listener(&mut relay, Listener::Wss, base_dir, d);
    let ticket = read_ticket(&mut relay, base_dir, d);
    relay.finish(d);
    Some(RelaySettings {
        tcp_viewer: tcp_viewer?,
        tcp_server: tcp_server?,
        ws: ws?,
        wss: wss?,
        ticket: ticket?,
    })
}

fn read_ticket(
    relay: &mut Section<'_>,
    base_dir: &Path,
    d: &mut Diagnostics,
) -> Option<TicketSettings> {
    let mut table = relay.section("ticket", d);
    let viewer = read_ticket_role(&mut table, "viewer", base_dir, d);
    let server = read_ticket_role(&mut table, "server", base_dir, d);
    table.finish(d);
    Some(TicketSettings {
        viewer: viewer?,
        server: server?,
    })
}

fn read_ticket_role(
    ticket: &mut Section<'_>,
    role: &'static str,
    base_dir: &Path,
    d: &mut Diagnostics,
) -> Option<TicketRoleSettings> {
    let mut table = ticket.section(role, d);
    let enabled = table.bool("enabled", d).or(false);
    let issuer = read_ticket_name(&mut table, "issuer", d);
    let relay_host = read_ticket_name(&mut table, "relay_host", d);
    let public_keys_file = read_path(&mut table, "public_keys_file", base_dir, d);
    let max_lifetime = secs(&mut table, "max_lifetime_secs", 60, 10..=600, d);
    let (issuer, relay_host, public_keys_file) = if enabled == Some(true) {
        let condition = format!("{} = true", table.key_path("enabled"));
        (
            required(&table, "issuer", issuer, &condition, d).map(Some),
            required(&table, "relay_host", relay_host, &condition, d).map(Some),
            required(&table, "public_keys_file", public_keys_file, &condition, d).map(Some),
        )
    } else {
        (
            optional(issuer),
            optional(relay_host),
            optional(public_keys_file),
        )
    };
    table.finish(d);
    Some(TicketRoleSettings {
        enabled: enabled?,
        issuer: issuer?,
        relay_host: relay_host?,
        public_keys_file: public_keys_file?,
        max_lifetime: max_lifetime?,
        keys: Default::default(),
    })
}

fn read_ticket_name(
    table: &mut Section<'_>,
    key: &'static str,
    d: &mut Diagnostics,
) -> Field<String> {
    let path = table.key_path(key);
    table.string(key, d).map(|text| {
        if (1..=256).contains(&text.len()) {
            Some(text.to_owned())
        } else {
            out_of_range(d, path, 1, 256);
            None
        }
    })
}

fn read_tcp_listener(
    relay: &mut Section<'_>,
    listener: Listener,
    d: &mut Diagnostics,
) -> Option<EntryPointSettings> {
    let mut table = relay.section(listener.name(), d);
    let enabled = table.bool("enabled", d).or(false);
    let listen = read_listen(&mut table, "listen", listener.default_port(), d);
    let allow = read_allow_list(&mut table, "allow", d);
    // Disabled listeners still validate every supplied value.
    let (listen, allow) = if enabled == Some(true) {
        let condition = format!("relay.{}.enabled = true", listener.name());
        (
            required(&table, "listen", listen, &condition, d).map(Some),
            required(&table, "allow", allow, &condition, d).map(Some),
        )
    } else {
        (optional(listen), optional(allow))
    };
    table.finish(d);
    Some(EntryPointSettings {
        enabled: enabled?,
        listen: listen?,
        allow: allow?,
    })
}

fn read_websocket_listener(
    relay: &mut Section<'_>,
    listener: Listener,
    base_dir: &Path,
    d: &mut Diagnostics,
) -> Option<WsListenerSettings> {
    let mut table = relay.section(listener.name(), d);
    let listen = read_listen(&mut table, "listen", None, d);
    let trusted_proxies =
        read_allow_list(&mut table, "trusted_proxies", d).or(AllowList::new(Vec::new()));
    let (cert_file, key_file) = if listener == Listener::Wss {
        (
            read_path(&mut table, "cert_file", base_dir, d),
            read_path(&mut table, "key_file", base_dir, d),
        )
    } else {
        (Field::Absent, Field::Absent)
    };
    let (viewer_enabled, viewer) = read_websocket_role(&mut table, "viewer", d);
    let (server_enabled, server) = read_websocket_role(&mut table, "server", d);
    let enabled = viewer_enabled || server_enabled;
    let condition = format!("relay.{} 역할이 하나 이상 enabled = true", listener.name());
    let listen = if enabled {
        required(&table, "listen", listen, &condition, d).map(Some)
    } else {
        optional(listen)
    };
    let tls = if enabled && listener == Listener::Wss {
        let cert_file = required(&table, "cert_file", cert_file, &condition, d);
        let key_file = required(&table, "key_file", key_file, &condition, d);
        cert_file.zip(key_file).map(|(cert_file, key_file)| {
            Some(TlsFiles {
                cert_file,
                key_file,
            })
        })
    } else {
        match (cert_file, key_file) {
            (Field::Invalid, _) | (_, Field::Invalid) => None,
            (Field::Value(cert_file), Field::Value(key_file)) => Some(Some(TlsFiles {
                cert_file,
                key_file,
            })),
            _ => Some(None),
        }
    };
    table.finish(d);
    Some(WsListenerSettings {
        listen: listen?,
        trusted_proxies: trusted_proxies?,
        tls: tls?,
        viewer: viewer?,
        server: server?,
    })
}

fn read_websocket_role(
    listener: &mut Section<'_>,
    role: &'static str,
    d: &mut Diagnostics,
) -> (bool, Option<WsRoleSettings>) {
    let mut table = listener.section(role, d);
    let enabled = table.bool("enabled", d).or(false);
    let allow = read_allow_list(&mut table, "allow", d);
    let allowed_origins = read_origins(&mut table, d);
    let allow = if enabled == Some(true) {
        let condition = format!("{} = true", table.key_path("enabled"));
        required(&table, "allow", allow, &condition, d).map(Some)
    } else {
        optional(allow)
    };
    table.finish(d);
    let settings =
        enabled
            .zip(allow)
            .zip(allowed_origins)
            .map(|((enabled, allow), allowed_origins)| WsRoleSettings {
                enabled,
                allow,
                allowed_origins,
            });
    (enabled == Some(true), settings)
}

fn read_allow_list(
    table: &mut Section<'_>,
    key: &'static str,
    d: &mut Diagnostics,
) -> Field<AllowList> {
    let path = table.key_path(key);
    table.string_array(key, d).map(|texts| {
        let mut items = Vec::with_capacity(texts.len());
        let mut valid = true;
        for (index, text) in texts.into_iter().enumerate() {
            match IpItem::parse(text) {
                Ok(item) => items.push(item),
                Err(error) => {
                    let (code, message) = ip_item_problem(error);
                    d.push(code, format!("{path}[{index}]"), message);
                    valid = false;
                }
            }
        }
        valid.then(|| AllowList::new(items))
    })
}

fn ip_item_problem(error: IpItemError) -> (ConfigError, &'static str) {
    match error {
        IpItemError::Invalid => (
            ConfigError::IpItemInvalid,
            "IP 항목 표기가 잘못되었다. 단일 IP, CIDR, `시작-끝` 범위만 쓸 수 있다.",
        ),
        IpItemError::RangeReversed => (ConfigError::IpRangeReversed, "범위의 시작이 끝보다 크다."),
        IpItemError::RangeFamilyMixed => (
            ConfigError::IpRangeFamilyMixed,
            "범위의 두 주소가 IPv4와 IPv6로 다르다.",
        ),
    }
}

fn read_origins(table: &mut Section<'_>, d: &mut Diagnostics) -> Option<Vec<Origin>> {
    let path = table.key_path("allowed_origins");
    table
        .string_array("allowed_origins", d)
        .map(|texts| {
            let mut origins = Vec::with_capacity(texts.len());
            let mut valid = true;
            for (index, text) in texts.into_iter().enumerate() {
                match Origin::parse(text) {
                    Ok(origin) => origins.push(origin),
                    Err(_) => {
                        d.push(
                            ConfigError::OriginInvalid,
                            format!("{path}[{index}]"),
                            "Origin 값 표기가 잘못되었다. `scheme://host[:port]` 형식만 쓸 수 있다.",
                        );
                        valid = false;
                    }
                }
            }
            valid.then_some(origins)
        })
        .or(Vec::new())
}

fn read_mode1(root: &mut Section<'_>, d: &mut Diagnostics) -> Option<Mode1Settings> {
    let mut mode1 = root.section("mode1", d);
    let enabled = mode1.bool("enabled", d).or(false);
    let path = mode1.key_path("allow_targets");
    let allow_targets = mode1.string_array("allow_targets", d).map(|texts| {
        let mut items = Vec::with_capacity(texts.len());
        let mut valid = true;
        for (index, text) in texts.into_iter().enumerate() {
            match TargetItem::parse(text) {
                Ok(item) => items.push(item),
                Err(error) => {
                    d.push(
                        ConfigError::TargetItemInvalid,
                        format!("{path}[{index}]"),
                        format!("Mode I 대상 항목이 잘못되었다: {}", target_problem(error)),
                    );
                    valid = false;
                }
            }
        }
        valid.then(|| TargetPolicy::new(items))
    });
    let allow_targets = if enabled == Some(true) {
        required(
            &mode1,
            "allow_targets",
            allow_targets,
            "mode1.enabled = true",
            d,
        )
        .map(Some)
    } else {
        optional(allow_targets)
    };
    mode1.finish(d);
    Some(Mode1Settings {
        enabled: enabled?,
        allow_targets: allow_targets?,
    })
}

fn target_problem(error: TargetItemError) -> &'static str {
    match error {
        TargetItemError::Format => "`대상:포트` 또는 `[IPv6 항목]:포트` 형식이 아니다.",
        TargetItemError::Port => "포트는 1~65535의 십진수이거나 `시작-끝`이어야 한다.",
        TargetItemError::PortRangeReversed => "포트 범위의 시작이 끝보다 크다.",
        TargetItemError::Ip(IpItemError::Invalid) => "IP 항목 표기가 잘못되었다.",
        TargetItemError::Ip(IpItemError::RangeReversed) => "IP 범위의 시작이 끝보다 크다.",
        TargetItemError::Ip(IpItemError::RangeFamilyMixed) => {
            "IP 범위의 두 주소가 IPv4와 IPv6로 다르다."
        }
        TargetItemError::Name => "DNS 이름이 잘못되었다. ASCII 영문·숫자·하이픈 라벨만 쓸 수 있다.",
    }
}

fn read_mode2(root: &mut Section<'_>, d: &mut Diagnostics) -> Option<Mode2Settings> {
    let mut mode2 = root.section("mode2", d);
    let enabled = mode2.bool("enabled", d).or(false);
    mode2.finish(d);
    Some(Mode2Settings { enabled: enabled? })
}

fn read_limits(root: &mut Section<'_>, d: &mut Diagnostics) -> Option<LimitSettings> {
    let mut limits = root.section("limits", d);
    let relay_connections = count(&mut limits, "relay_connections", 1000, 2..=65_535, d);
    let admin_connections = count(&mut limits, "admin_connections", 64, 1..=1024, d);
    let waiting = count(&mut limits, "waiting", 200, 1..=65_535, d);
    let buffer_bytes = count(&mut limits, "buffer_bytes", 131_072, 16_384..=4_194_304, d);
    let ws_message_bytes = count(
        &mut limits,
        "ws_message_bytes",
        1_048_576,
        65_536..=16_777_216,
        d,
    );
    if let (Some(waiting), Some(relay_connections)) = (waiting, relay_connections)
        && waiting > relay_connections
    {
        out_of_range(
            d,
            limits.key_path("waiting"),
            1,
            i64::from(relay_connections),
        );
    }
    limits.finish(d);
    Some(LimitSettings {
        relay_connections: relay_connections?,
        admin_connections: admin_connections?,
        waiting: waiting?,
        buffer_bytes: buffer_bytes?,
        ws_message_bytes: ws_message_bytes?,
    })
}

fn read_timeouts(root: &mut Section<'_>, d: &mut Diagnostics) -> Option<TimeoutSettings> {
    let mut timeouts = root.section("timeouts", d);
    let handshake = secs(&mut timeouts, "handshake_secs", 10, 1..=120, d);
    let tls_upgrade = secs(&mut timeouts, "tls_upgrade_secs", 10, 1..=120, d);
    let wait = secs(&mut timeouts, "wait_secs", 3600, 10..=86_400, d);
    let connect = secs(&mut timeouts, "connect_secs", 10, 1..=120, d);
    let close = secs(&mut timeouts, "close_secs", 10, 1..=120, d);
    let shutdown_grace = secs(&mut timeouts, "shutdown_grace_secs", 30, 0..=600, d);
    timeouts.finish(d);
    Some(TimeoutSettings {
        handshake: handshake?,
        tls_upgrade: tls_upgrade?,
        wait: wait?,
        connect: connect?,
        close: close?,
        shutdown_grace: shutdown_grace?,
    })
}

fn read_keepalive(root: &mut Section<'_>, d: &mut Diagnostics) -> Option<KeepaliveSettings> {
    let mut keepalive = root.section("keepalive", d);
    let tcp_idle = secs(&mut keepalive, "tcp_idle_secs", 60, 10..=3600, d);
    let tcp_interval = secs(&mut keepalive, "tcp_interval_secs", 10, 1..=300, d);
    let tcp_probes = count(&mut keepalive, "tcp_probes", 6, 1..=20, d);
    let ws_ping = secs(&mut keepalive, "ws_ping_secs", 30, 5..=300, d);
    let ws_pong_timeout = secs(&mut keepalive, "ws_pong_timeout_secs", 30, 5..=300, d);
    keepalive.finish(d);
    Some(KeepaliveSettings {
        tcp_idle: tcp_idle?,
        tcp_interval: tcp_interval?,
        tcp_probes: tcp_probes?,
        ws_ping: ws_ping?,
        ws_pong_timeout: ws_pong_timeout?,
    })
}

fn read_management(
    root: &mut Section<'_>,
    base_dir: &Path,
    d: &mut Diagnostics,
) -> Option<ManagementSettings> {
    let mut management = root.section("management", d);
    let listen = read_listen(&mut management, "listen", None, d);
    let listen = required(&management, "listen", listen, "", d);
    let config_edit = management.bool("config_edit", d).or(false);
    let cert_file = read_path(&mut management, "cert_file", base_dir, d);
    let key_file = read_path(&mut management, "key_file", base_dir, d);
    match (cert_file.is_absent(), key_file.is_absent()) {
        (true, true) => {
            if let Some(listen) = listen
                && !listen.ip().is_loopback()
            {
                d.push(
                    ConfigError::PlaintextManagementNotLoopback,
                    management.key_path("listen"),
                    "평문 HTTP 관리 리스너는 루프백 주소(127.0.0.0/8, ::1)에서만 열 수 있다. 다른 주소에는 `cert_file`·`key_file`이 필요하다.",
                );
            }
        }
        (false, true) => tls_pair_incomplete(&management, "key_file", "cert_file", d),
        (true, false) => tls_pair_incomplete(&management, "cert_file", "key_file", d),
        (false, false) => {}
    }
    let tls = match (cert_file, key_file) {
        (Field::Absent, Field::Absent) => Some(None),
        (Field::Value(cert_file), Field::Value(key_file)) => Some(Some(TlsFiles {
            cert_file,
            key_file,
        })),
        _ => None,
    };

    let mut login_limit = management.section("login_limit", d);
    let per_account = count(&mut login_limit, "per_account", 5, 1..=100, d);
    let per_source = count(&mut login_limit, "per_source", 20, 1..=100, d);
    let window = secs(&mut login_limit, "window_secs", 900, 60..=86_400, d);
    let lockout = secs(&mut login_limit, "lockout_secs", 900, 60..=86_400, d);
    login_limit.finish(d);

    let mut session = management.section("session", d);
    let idle = secs(&mut session, "idle_secs", 1800, 300..=28_800, d);
    let max_lifetime = secs(&mut session, "max_lifetime_secs", 43_200, 3600..=259_200, d);
    if let (Some(idle), Some(max_lifetime)) = (idle, max_lifetime)
        && idle > max_lifetime
    {
        d.push(
            ConfigError::RelationViolated,
            session.key_path("idle_secs"),
            "`management.session.idle_secs`는 `management.session.max_lifetime_secs` 이하여야 한다.",
        );
    }
    session.finish(d);
    management.finish(d);
    Some(ManagementSettings {
        listen: listen?,
        tls: tls?,
        config_edit: config_edit?,
        login_limit: LoginLimitSettings {
            per_account: per_account?,
            per_source: per_source?,
            window: window?,
            lockout: lockout?,
        },
        session: AdminSessionSettings {
            idle: idle?,
            max_lifetime: max_lifetime?,
        },
    })
}

fn tls_pair_incomplete(
    management: &Section<'_>,
    missing: &str,
    present: &str,
    d: &mut Diagnostics,
) {
    d.push(
        ConfigError::TlsPairIncomplete,
        management.key_path(missing),
        format!("`management.{present}`가 있으면 `management.{missing}`도 써야 한다. 두 키는 함께 쓰거나 함께 생략한다."),
    );
}

fn read_dashboard(root: &mut Section<'_>, d: &mut Diagnostics) -> Option<DashboardSettings> {
    let mut dashboard = root.section("dashboard", d);
    let poll = secs(&mut dashboard, "poll_secs", 2, 1..=60, d);
    let query_timeout = secs(&mut dashboard, "query_timeout_secs", 5, 1..=60, d);
    dashboard.finish(d);
    Some(DashboardSettings {
        poll: poll?,
        query_timeout: query_timeout?,
    })
}

fn read_recent_closed(root: &mut Section<'_>, d: &mut Diagnostics) -> Option<RecentClosedSettings> {
    let mut recent_closed = root.section("recent_closed", d);
    let max_entries = count(&mut recent_closed, "max_entries", 100, 0..=10_000, d);
    recent_closed.finish(d);
    Some(RecentClosedSettings {
        max_entries: max_entries?,
    })
}

fn read_recent_logs(root: &mut Section<'_>, d: &mut Diagnostics) -> Option<RecentLogsSettings> {
    let mut recent_logs = root.section("recent_logs", d);
    let max_entries = count(&mut recent_logs, "max_entries", 1000, 0..=10_000, d);
    recent_logs.finish(d);
    Some(RecentLogsSettings {
        max_entries: max_entries?,
    })
}

fn read_status(root: &mut Section<'_>, d: &mut Diagnostics) -> Option<StatusSettings> {
    let mut status = root.section("status", d);
    let listen = read_listen(&mut status, "listen", None, d);
    let listen = required(&status, "listen", listen, "", d);
    status.finish(d);
    Some(StatusSettings { listen: listen? })
}

fn read_log(root: &mut Section<'_>, base_dir: &Path, d: &mut Diagnostics) -> Option<LogSettings> {
    let mut log = root.section("log", d);
    let output = match log.string("output", d) {
        Field::Absent => Some(LogOutput::Stdout),
        Field::Invalid => None,
        Field::Value("stdout") => Some(LogOutput::Stdout),
        Field::Value("") => {
            empty_path(d, log.key_path("output"));
            None
        }
        Field::Value(path) => Some(LogOutput::File(base_dir.join(path))),
    };
    log.finish(d);
    Some(LogSettings { output: output? })
}

fn read_reload(root: &mut Section<'_>, d: &mut Diagnostics) -> Option<ReloadSettings> {
    let mut reload = root.section("reload", d);
    let settle = secs(&mut reload, "settle_secs", 5, 1..=60, d);
    let poll = secs(&mut reload, "poll_secs", 30, 5..=600, d);
    reload.finish(d);
    Some(ReloadSettings {
        settle: settle?,
        poll: poll?,
    })
}

/// Enabled relay listeners, the management listener, and the status listener
/// must not share an `IP:port`. A conflict is reported at the later key.
fn check_listen_conflicts(
    relay: &RelaySettings,
    management: &ManagementSettings,
    status: &StatusSettings,
    d: &mut Diagnostics,
) {
    let mut listeners: Vec<(String, SocketAddr)> = Listener::ALL
        .into_iter()
        .filter_map(|listener| {
            let listen = relay
                .listen(listener)
                .filter(|_| relay.listener_enabled(listener))?;
            Some((format!("relay.{}.listen", listener.name()), listen))
        })
        .collect();
    listeners.push(("management.listen".to_owned(), management.listen));
    listeners.push(("status.listen".to_owned(), status.listen));
    for (index, (key, address)) in listeners.iter().enumerate() {
        if let Some((earlier, _)) = listeners[..index]
            .iter()
            .find(|(_, other)| other == address)
        {
            d.push(
                ConfigError::ListenConflict,
                key.clone(),
                format!("`{earlier}`와 주소·포트가 같다({address})."),
            );
        }
    }
}

fn read_listen(
    section: &mut Section<'_>,
    key: &'static str,
    default_port: Option<u16>,
    d: &mut Diagnostics,
) -> Field<SocketAddr> {
    let path = section.key_path(key);
    section.string(key, d).map(|text| {
        let address = parse_listen(text, default_port);
        if address.is_none() {
            let message = if default_port.is_some() {
                "주소 형식이 잘못되었다. `IP:포트`, `[IPv6]:포트`, `IPv4`, `[IPv6]` 중 하나로 쓴다. 호스트 이름은 받지 않는다."
            } else {
                "주소 형식이 잘못되었다. `IP:포트` 또는 `[IPv6]:포트`로 쓴다. 포트는 1~65535이며 생략할 수 없다."
            };
            d.push(ConfigError::ListenInvalid, path, message);
        }
        address
    })
}

fn read_path(
    section: &mut Section<'_>,
    key: &'static str,
    base_dir: &Path,
    d: &mut Diagnostics,
) -> Field<PathBuf> {
    let path = section.key_path(key);
    section.string(key, d).map(|text| {
        if text.is_empty() {
            empty_path(d, path);
            None
        } else {
            Some(base_dir.join(text))
        }
    })
}

fn empty_path(d: &mut Diagnostics, key: String) {
    d.push(ConfigError::ValueOutOfRange, key, "빈 경로는 쓸 수 없다.");
}

/// Reports `config_missing_key` for an absent required key. `condition` is
/// empty for keys that are always required.
fn required<T>(
    section: &Section<'_>,
    key: &str,
    field: Field<T>,
    condition: &str,
    d: &mut Diagnostics,
) -> Option<T> {
    if field.is_absent() {
        let message = if condition.is_empty() {
            "필수 키가 없다.".to_owned()
        } else {
            format!("`{condition}`이면 필수 키다.")
        };
        d.push(ConfigError::MissingKey, section.key_path(key), message);
    }
    field.value()
}

/// `Some(None)` for an absent optional key, `None` for an invalid one.
fn optional<T>(field: Field<T>) -> Option<Option<T>> {
    match field {
        Field::Absent => Some(None),
        Field::Invalid => None,
        Field::Value(value) => Some(Some(value)),
    }
}

fn count(
    section: &mut Section<'_>,
    key: &'static str,
    default: u32,
    range: std::ops::RangeInclusive<u32>,
    d: &mut Diagnostics,
) -> Option<u32> {
    let range = i64::from(*range.start())..=i64::from(*range.end());
    section
        .integer(key, range, d)
        .map(|value| u32::try_from(value).ok())
        .or(default)
}

fn secs(
    section: &mut Section<'_>,
    key: &'static str,
    default: u32,
    range: std::ops::RangeInclusive<u32>,
    d: &mut Diagnostics,
) -> Option<Duration> {
    count(section, key, default, range, d).map(|value| Duration::from_secs(u64::from(value)))
}
