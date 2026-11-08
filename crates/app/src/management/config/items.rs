//! Ordered config-v3 §1.2 descriptors. Never read a referenced file here.
use super::ConfigValue;
use crate::reload::ConfigPresence;
use crate::settings::{LogOutput, Settings};
use vnc_relay_domain::entry_point::EntryPoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigReloadKind {
    Dynamic,
    Restart,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigItem {
    pub key: String,
    pub value: ConfigValue,
    pub reload: ConfigReloadKind,
    pub editable: bool,
    pub min: Option<i64>,
    pub max: Option<i64>,
}
fn item(
    key: impl Into<String>,
    value: ConfigValue,
    reload: ConfigReloadKind,
    editable: bool,
) -> ConfigItem {
    ConfigItem {
        key: key.into(),
        value,
        reload,
        editable,
        min: None,
        max: None,
    }
}
fn integer(key: &str, value: u64, min: i64, max: i64, dynamic: bool) -> ConfigItem {
    let mut out = item(
        key,
        ConfigValue::Integer(value as i64),
        if dynamic {
            ConfigReloadKind::Dynamic
        } else {
            ConfigReloadKind::Restart
        },
        dynamic,
    );
    out.min = Some(min);
    out.max = Some(max);
    out
}
fn text(value: impl ToString) -> ConfigValue {
    ConfigValue::String(value.to_string())
}
fn optional<T: ToString>(value: Option<T>) -> ConfigValue {
    value.map(text).unwrap_or(ConfigValue::Null)
}

pub fn config_items(s: &Settings, presence: &ConfigPresence) -> Vec<ConfigItem> {
    use ConfigReloadKind::{Dynamic, Restart};
    let mut out = Vec::with_capacity(67);
    out.push(item(
        "admin.accounts_file",
        text(s.admin.accounts_file.display()),
        Restart,
        false,
    ));
    out.push(integer(
        "admin.argon2.memory_kib",
        u64::from(s.admin.argon2.memory_kib),
        19456,
        1048576,
        false,
    ));
    out.push(integer(
        "admin.argon2.iterations",
        u64::from(s.admin.argon2.iterations),
        2,
        10,
        false,
    ));
    out.push(integer(
        "admin.argon2.parallelism",
        u64::from(s.admin.argon2.parallelism),
        1,
        16,
        false,
    ));
    for entry in EntryPoint::ALL {
        let e = s.relay.get(entry);
        let prefix = format!("relay.{}", entry.name());
        out.push(item(
            format!("{prefix}.enabled"),
            ConfigValue::Bool(e.enabled),
            Restart,
            false,
        ));
        out.push(item(
            format!("{prefix}.listen"),
            optional(e.listen),
            Restart,
            false,
        ));
        out.push(item(
            format!("{prefix}.allow"),
            e.allow
                .as_ref()
                .map(|allow| {
                    ConfigValue::Strings(allow.items().iter().map(ToString::to_string).collect())
                })
                .unwrap_or(ConfigValue::Null),
            Dynamic,
            e.enabled,
        ));
        if matches!(entry, EntryPoint::WsViewer | EntryPoint::WssViewer) {
            let key = format!("{prefix}.allowed_origins");
            let value = if !e.enabled && !presence.keys.contains(&key) {
                ConfigValue::Null
            } else {
                ConfigValue::Strings(e.allowed_origins.iter().map(ToString::to_string).collect())
            };
            out.push(item(key, value, Dynamic, e.enabled));
        }
        if entry == EntryPoint::WssViewer {
            for (suffix, path) in [
                ("cert_file", e.tls.as_ref().map(|tls| &tls.cert_file)),
                ("key_file", e.tls.as_ref().map(|tls| &tls.key_file)),
            ] {
                let key = format!("{prefix}.{suffix}");
                let path = path.or_else(|| {
                    (!e.enabled)
                        .then(|| presence.inactive_paths.get(&key))
                        .flatten()
                });
                out.push(item(
                    key,
                    optional(path.map(|path| path.display())),
                    Dynamic,
                    false,
                ));
            }
        }
    }
    out.push(item(
        "mode1.enabled",
        ConfigValue::Bool(s.mode1.enabled),
        Restart,
        false,
    ));
    out.push(item(
        "mode1.allow_targets",
        s.mode1
            .allow_targets
            .as_ref()
            .map(|policy| {
                ConfigValue::Strings(policy.items().iter().map(ToString::to_string).collect())
            })
            .unwrap_or(ConfigValue::Null),
        Dynamic,
        s.mode1.enabled,
    ));
    out.push(item(
        "mode2.enabled",
        ConfigValue::Bool(s.mode2.enabled),
        Restart,
        false,
    ));
    for (key, value, min, max) in [
        (
            "limits.relay_connections",
            s.limits.relay_connections,
            2,
            65535,
        ),
        (
            "limits.admin_connections",
            s.limits.admin_connections,
            1,
            1024,
        ),
        ("limits.waiting", s.limits.waiting, 1, 65535),
        ("limits.buffer_bytes", s.limits.buffer_bytes, 16384, 4194304),
        (
            "limits.ws_message_bytes",
            s.limits.ws_message_bytes,
            65536,
            16777216,
        ),
    ] {
        out.push(integer(key, u64::from(value), min, max, true));
    }
    for (key, value, min, max) in [
        ("timeouts.handshake_secs", s.timeouts.handshake, 1, 120),
        ("timeouts.tls_upgrade_secs", s.timeouts.tls_upgrade, 1, 120),
        ("timeouts.wait_secs", s.timeouts.wait, 10, 86400),
        ("timeouts.connect_secs", s.timeouts.connect, 1, 120),
        ("timeouts.close_secs", s.timeouts.close, 1, 120),
        (
            "timeouts.shutdown_grace_secs",
            s.timeouts.shutdown_grace,
            0,
            600,
        ),
        ("keepalive.tcp_idle_secs", s.keepalive.tcp_idle, 10, 3600),
        (
            "keepalive.tcp_interval_secs",
            s.keepalive.tcp_interval,
            1,
            300,
        ),
    ] {
        out.push(integer(key, value.as_secs(), min, max, true));
    }
    out.push(integer(
        "keepalive.tcp_probes",
        u64::from(s.keepalive.tcp_probes),
        1,
        20,
        true,
    ));
    out.push(integer(
        "keepalive.ws_ping_secs",
        s.keepalive.ws_ping.as_secs(),
        5,
        300,
        true,
    ));
    out.push(integer(
        "keepalive.ws_pong_timeout_secs",
        s.keepalive.ws_pong_timeout.as_secs(),
        5,
        300,
        true,
    ));
    out.push(item(
        "management.listen",
        text(s.management.listen),
        Restart,
        false,
    ));
    out.push(item(
        "management.cert_file",
        optional(s.management.tls.as_ref().map(|tls| tls.cert_file.display())),
        Dynamic,
        false,
    ));
    out.push(item(
        "management.key_file",
        optional(s.management.tls.as_ref().map(|tls| tls.key_file.display())),
        Dynamic,
        false,
    ));
    out.push(item(
        "management.config_edit",
        ConfigValue::Bool(s.management.config_edit),
        Restart,
        false,
    ));
    for (key, value, min, max) in [
        (
            "management.login_limit.per_account",
            u64::from(s.management.login_limit.per_account),
            1,
            100,
        ),
        (
            "management.login_limit.per_source",
            u64::from(s.management.login_limit.per_source),
            1,
            100,
        ),
        (
            "management.login_limit.window_secs",
            s.management.login_limit.window.as_secs(),
            60,
            86400,
        ),
        (
            "management.login_limit.lockout_secs",
            s.management.login_limit.lockout.as_secs(),
            60,
            86400,
        ),
        (
            "management.session.idle_secs",
            s.management.session.idle.as_secs(),
            300,
            28800,
        ),
        (
            "management.session.max_lifetime_secs",
            s.management.session.max_lifetime.as_secs(),
            3600,
            259200,
        ),
        ("dashboard.poll_secs", s.dashboard.poll.as_secs(), 1, 60),
        (
            "dashboard.query_timeout_secs",
            s.dashboard.query_timeout.as_secs(),
            1,
            60,
        ),
        (
            "recent_closed.max_entries",
            u64::from(s.recent_closed.max_entries),
            0,
            10000,
        ),
    ] {
        out.push(integer(key, value, min, max, false));
    }
    out.push(item("status.listen", text(s.status.listen), Restart, false));
    out.push(item(
        "log.output",
        match &s.log.output {
            LogOutput::Stdout => text("stdout"),
            LogOutput::File(path) => text(path.display()),
        },
        Restart,
        false,
    ));
    out.push(integer(
        "reload.settle_secs",
        s.reload.settle.as_secs(),
        1,
        60,
        false,
    ));
    out.push(integer(
        "reload.poll_secs",
        s.reload.poll.as_secs(),
        5,
        600,
        false,
    ));
    out
}

/// Request-shape validation does not construct the full display view or copy
/// paths and allow lists. Configuration ranges remain the loader's job.
pub(super) fn accepts_change(s: &Settings, change: &super::ConfigChange) -> bool {
    let integer = matches!(change.value, ConfigValue::Integer(_));
    let strings = matches!(change.value, ConfigValue::Strings(_));
    match change.key.as_str() {
        "relay.tcp_viewer.allow" => strings && s.relay.tcp_viewer.enabled,
        "relay.tcp_server.allow" => strings && s.relay.tcp_server.enabled,
        "relay.ws_viewer.allow" | "relay.ws_viewer.allowed_origins" => {
            strings && s.relay.ws_viewer.enabled
        }
        "relay.wss_viewer.allow" | "relay.wss_viewer.allowed_origins" => {
            strings && s.relay.wss_viewer.enabled
        }
        "mode1.allow_targets" => strings && s.mode1.enabled,
        "limits.relay_connections"
        | "limits.admin_connections"
        | "limits.waiting"
        | "limits.buffer_bytes"
        | "limits.ws_message_bytes"
        | "timeouts.handshake_secs"
        | "timeouts.tls_upgrade_secs"
        | "timeouts.wait_secs"
        | "timeouts.connect_secs"
        | "timeouts.close_secs"
        | "timeouts.shutdown_grace_secs"
        | "keepalive.tcp_idle_secs"
        | "keepalive.tcp_interval_secs"
        | "keepalive.tcp_probes"
        | "keepalive.ws_ping_secs"
        | "keepalive.ws_pong_timeout_secs" => integer,
        _ => false,
    }
}
