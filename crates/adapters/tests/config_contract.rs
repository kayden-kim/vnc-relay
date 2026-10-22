//! Contract tests of the config loader (docs/contracts/config-v1.md §5).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use vnc_relay_adapters::config_file::{load, parse};
use vnc_relay_app::settings::{
    AdminSessionSettings, AdminSettings, Argon2Settings, ConfigWarning, DashboardSettings,
    EntryPointSettings, KeepaliveSettings, LimitSettings, LogOutput, LogSettings,
    LoginLimitSettings, ManagementSettings, Mode1Settings, Mode2Settings, RecentClosedSettings,
    RelaySettings, ReloadSettings, Settings, StatusSettings, TimeoutSettings, TlsFiles,
};
use vnc_relay_domain::allow_list::{AllowList, IpItem};
use vnc_relay_domain::entry_point::EntryPoint;
use vnc_relay_domain::origin::Origin;
use vnc_relay_domain::target_policy::{TargetItem, TargetPolicy};

/// Always-required keys with plaintext loopback management.
const REQUIRED: &str = r#"
[admin]
accounts_file = "admin-accounts.toml"

[management]
listen = "127.0.0.1:18080"

[status]
listen = "127.0.0.1:18081"
"#;

fn base_dir() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\relay")
    } else {
        PathBuf::from("/etc/relay")
    }
}

fn parse_ok(text: &str) -> Settings {
    match parse(text, &base_dir()) {
        Ok(settings) => settings,
        Err(diagnostics) => panic!("unexpected diagnostics: {diagnostics:#?}"),
    }
}

/// `(code, key)` of every diagnostic, in order.
fn problems(text: &str) -> Vec<(&'static str, String)> {
    match parse(text, &base_dir()) {
        Ok(settings) => panic!("parsed unexpectedly: {settings:#?}"),
        Err(diagnostics) => diagnostics
            .into_iter()
            .map(|diagnostic| (diagnostic.error.code(), diagnostic.key))
            .collect(),
    }
}

fn problem(code: &'static str, key: &str) -> (&'static str, String) {
    (code, key.to_owned())
}

fn with_required(extra: &str) -> String {
    format!("{REQUIRED}\n{extra}")
}

fn addr(text: &str) -> SocketAddr {
    text.parse()
        .unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn allow(items: &[&str]) -> AllowList {
    let parse =
        |text: &&str| IpItem::parse(text).unwrap_or_else(|error| panic!("{text}: {error:?}"));
    AllowList::new(items.iter().map(parse).collect())
}

fn secs(value: u64) -> Duration {
    Duration::from_secs(value)
}

fn tcp_entry_point(listen: &str, items: &[&str]) -> EntryPointSettings {
    EntryPointSettings {
        enabled: true,
        listen: Some(addr(listen)),
        allow: Some(allow(items)),
        allowed_origins: Vec::new(),
        tls: None,
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vnc-relay-config-{}-{name}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .unwrap_or_else(|error| panic!("clear {}: {error}", dir.display()));
    }
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|error| panic!("create {}: {error}", dir.display()));
    dir
}

#[test]
fn accepts_minimal_example_and_fills_every_default() {
    let settings = parse_ok(
        r#"
[admin]
accounts_file = "admin-accounts.toml"

[relay.tcp_viewer]
enabled = true
listen = "0.0.0.0:15901"
allow = ["198.51.100.0/24"]

[relay.tcp_server]
enabled = true
listen = "0.0.0.0:15902"
allow = ["203.0.113.10-203.0.113.20", "2001:db8::/32"]

[mode2]
enabled = true

[management]
listen = "127.0.0.1:18080"

[status]
listen = "127.0.0.1:18081"
"#,
    );
    let expected = Settings {
        admin: AdminSettings {
            accounts_file: base_dir().join("admin-accounts.toml"),
            argon2: Argon2Settings {
                memory_kib: 19_456,
                iterations: 2,
                parallelism: 1,
            },
        },
        relay: RelaySettings {
            tcp_viewer: tcp_entry_point("0.0.0.0:15901", &["198.51.100.0/24"]),
            tcp_server: tcp_entry_point(
                "0.0.0.0:15902",
                &["203.0.113.10-203.0.113.20", "2001:db8::/32"],
            ),
            ws_viewer: EntryPointSettings::default(),
            wss_viewer: EntryPointSettings::default(),
        },
        mode1: Mode1Settings::default(),
        mode2: Mode2Settings { enabled: true },
        limits: LimitSettings {
            relay_connections: 1000,
            admin_connections: 64,
            waiting: 200,
            initial_message_bytes: 4096,
            buffer_bytes: 131_072,
            ws_message_bytes: 1_048_576,
        },
        timeouts: TimeoutSettings {
            handshake: secs(10),
            tls_upgrade: secs(10),
            wait: secs(3600),
            connect: secs(10),
            close: secs(10),
            shutdown_grace: secs(30),
        },
        keepalive: KeepaliveSettings {
            tcp_idle: secs(60),
            tcp_interval: secs(10),
            tcp_probes: 6,
            ws_ping: secs(30),
            ws_pong_timeout: secs(30),
        },
        management: ManagementSettings {
            listen: addr("127.0.0.1:18080"),
            tls: None,
            login_limit: LoginLimitSettings {
                per_account: 5,
                per_source: 20,
                window: secs(900),
                lockout: secs(900),
            },
            session: AdminSessionSettings {
                idle: secs(1800),
                max_lifetime: secs(43_200),
            },
        },
        dashboard: DashboardSettings {
            poll: secs(2),
            query_timeout: secs(5),
        },
        recent_closed: RecentClosedSettings { max_entries: 100 },
        status: StatusSettings {
            listen: addr("127.0.0.1:18081"),
        },
        log: LogSettings {
            output: LogOutput::Stdout,
        },
        reload: ReloadSettings {
            settle: secs(5),
            poll: secs(30),
        },
    };
    assert_eq!(settings, expected);
    assert_eq!(settings.warnings(), []);
}

#[test]
fn accepts_tls_example_and_resolves_relative_paths() {
    let settings = parse_ok(
        r#"
[admin]
accounts_file = "admin-accounts.toml"

[relay.wss_viewer]
enabled = true
listen = "0.0.0.0:15903"
allow = ["198.51.100.0/24"]
allowed_origins = ["https://viewer.example", "https://viewer.example:8443"]
cert_file = "certs/relay.pem"
key_file = "certs/relay.key"

[mode1]
enabled = true
allow_targets = ["192.0.2.0/24:5900-5905", "[2001:db8::/32]:5900", "pc1.example:5900"]

[mode2]
enabled = true

[management]
listen = "0.0.0.0:18443"
cert_file = "certs/admin.pem"
key_file = "certs/admin.key"

[status]
listen = "127.0.0.1:18081"

[log]
output = "stdout"
"#,
    );
    let certs = base_dir().join("certs");
    assert_eq!(
        settings.relay.wss_viewer,
        EntryPointSettings {
            enabled: true,
            listen: Some(addr("0.0.0.0:15903")),
            allow: Some(allow(&["198.51.100.0/24"])),
            allowed_origins: vec![
                Origin::parse("https://viewer.example").expect("origin"),
                Origin::parse("https://viewer.example:8443").expect("origin"),
            ],
            tls: Some(TlsFiles {
                cert_file: certs.join("relay.pem"),
                key_file: certs.join("relay.key"),
            }),
        }
    );
    let targets = [
        "192.0.2.0/24:5900-5905",
        "[2001:db8::/32]:5900",
        "pc1.example:5900",
    ]
    .map(|text| TargetItem::parse(text).expect("target"));
    assert_eq!(
        settings.mode1,
        Mode1Settings {
            enabled: true,
            allow_targets: Some(TargetPolicy::new(targets.to_vec())),
        }
    );
    assert_eq!(
        settings.management.tls,
        Some(TlsFiles {
            cert_file: certs.join("admin.pem"),
            key_file: certs.join("admin.key"),
        })
    );
    assert_eq!(settings.log.output, LogOutput::Stdout);
}

/// spec: operations.md:OPS-001 partial
#[test]
fn reports_every_problem_in_one_pass() {
    let found = problems(
        r#"
[admin]
acounts_file = "typo.toml"

[relay.tcp_viewer]
enabled = true
listen = "relay.example:5901"

[limits]
relay_connections = "1000"
buffer_bytes = 10.0
waiting = 0

[timeouts]
handshake_secs = 121

[management]
listen = "127.0.0.1"

[extra]
key = 1
"#,
    );
    assert_eq!(
        found,
        [
            problem("config_missing_key", "admin.accounts_file"),
            problem("config_unknown_key", "admin.acounts_file"),
            problem("config_listen_invalid", "relay.tcp_viewer.listen"),
            problem("config_missing_key", "relay.tcp_viewer.allow"),
            problem("config_type_mismatch", "limits.relay_connections"),
            problem("config_value_out_of_range", "limits.waiting"),
            problem("config_type_mismatch", "limits.buffer_bytes"),
            problem("config_value_out_of_range", "timeouts.handshake_secs"),
            problem("config_listen_invalid", "management.listen"),
            problem("config_missing_key", "status.listen"),
            problem("config_unknown_key", "extra"),
        ]
    );
}

/// spec: operations.md:OPS-001 partial
#[test]
fn rejects_syntax_errors_and_duplicate_keys_with_one_diagnostic() {
    for text in [
        "[admin\naccounts_file = \"a.toml\"\n",
        "[admin]\naccounts_file = \"a.toml\"\naccounts_file = \"b.toml\"\n",
        "[admin]\naccounts_file = \"a.toml\"\n[admin]\n",
        "[limits]\nrelay_connections = 99999999999999999999\n",
    ] {
        assert_eq!(
            problems(text),
            [problem("config_syntax_error", "")],
            "{text:?}"
        );
    }
}

/// spec: operations.md:OPS-001 partial
#[test]
fn rejects_missing_unreadable_and_non_utf8_files() {
    let dir = temp_dir("unreadable");
    let missing = dir.join("missing.toml");
    let non_utf8 = dir.join("non-utf8.toml");
    std::fs::write(&non_utf8, [0x5b, 0xff, 0xfe, 0x5d]).expect("write test file");

    for path in [&missing, &non_utf8, &dir] {
        let diagnostics = load(path).expect_err("unreadable file must fail");
        let found: Vec<_> = diagnostics
            .iter()
            .map(|d| (d.error.code(), d.key.as_str()))
            .collect();
        assert_eq!(
            found,
            [("config_file_unreadable", "")],
            "{}",
            path.display()
        );
    }
    std::fs::remove_dir_all(&dir).expect("remove test dir");
}

#[test]
fn load_resolves_relative_paths_against_the_config_directory() {
    let dir = temp_dir("relative");
    let config = dir.join("relay.toml");
    std::fs::write(
        &config,
        with_required("[log]\noutput = \"logs/relay.log\"\n"),
    )
    .expect("write config");

    let settings = load(&config).expect("valid config");
    assert_eq!(
        settings.admin.accounts_file,
        dir.join("admin-accounts.toml")
    );
    assert_eq!(
        settings.log.output,
        LogOutput::File(dir.join("logs/relay.log"))
    );
    std::fs::remove_dir_all(&dir).expect("remove test dir");
}

/// spec: operations.md:OPS-001 partial
#[test]
fn checks_every_numeric_key_at_its_bounds() {
    type Getter = fn(&Settings) -> u64;
    let cases: [(&str, &str, i64, i64, u64, Getter); 31] = [
        (
            "admin.argon2",
            "memory_kib",
            19_456,
            1_048_576,
            19_456,
            |s| s.admin.argon2.memory_kib.into(),
        ),
        ("admin.argon2", "iterations", 2, 10, 2, |s| {
            s.admin.argon2.iterations.into()
        }),
        ("admin.argon2", "parallelism", 1, 16, 1, |s| {
            s.admin.argon2.parallelism.into()
        }),
        ("limits", "relay_connections", 2, 65_535, 1000, |s| {
            s.limits.relay_connections.into()
        }),
        ("limits", "admin_connections", 1, 1024, 64, |s| {
            s.limits.admin_connections.into()
        }),
        ("limits", "waiting", 1, 1000, 200, |s| {
            s.limits.waiting.into()
        }),
        ("limits", "initial_message_bytes", 256, 65_536, 4096, |s| {
            s.limits.initial_message_bytes.into()
        }),
        ("limits", "buffer_bytes", 16_384, 4_194_304, 131_072, |s| {
            s.limits.buffer_bytes.into()
        }),
        (
            "limits",
            "ws_message_bytes",
            65_536,
            16_777_216,
            1_048_576,
            |s| s.limits.ws_message_bytes.into(),
        ),
        ("timeouts", "handshake_secs", 1, 120, 10, |s| {
            s.timeouts.handshake.as_secs()
        }),
        ("timeouts", "tls_upgrade_secs", 1, 120, 10, |s| {
            s.timeouts.tls_upgrade.as_secs()
        }),
        ("timeouts", "wait_secs", 10, 86_400, 3600, |s| {
            s.timeouts.wait.as_secs()
        }),
        ("timeouts", "connect_secs", 1, 120, 10, |s| {
            s.timeouts.connect.as_secs()
        }),
        ("timeouts", "close_secs", 1, 120, 10, |s| {
            s.timeouts.close.as_secs()
        }),
        ("timeouts", "shutdown_grace_secs", 0, 600, 30, |s| {
            s.timeouts.shutdown_grace.as_secs()
        }),
        ("keepalive", "tcp_idle_secs", 10, 3600, 60, |s| {
            s.keepalive.tcp_idle.as_secs()
        }),
        ("keepalive", "tcp_interval_secs", 1, 300, 10, |s| {
            s.keepalive.tcp_interval.as_secs()
        }),
        ("keepalive", "tcp_probes", 1, 20, 6, |s| {
            s.keepalive.tcp_probes.into()
        }),
        ("keepalive", "ws_ping_secs", 5, 300, 30, |s| {
            s.keepalive.ws_ping.as_secs()
        }),
        ("keepalive", "ws_pong_timeout_secs", 5, 300, 30, |s| {
            s.keepalive.ws_pong_timeout.as_secs()
        }),
        ("management.login_limit", "per_account", 1, 100, 5, |s| {
            s.management.login_limit.per_account.into()
        }),
        ("management.login_limit", "per_source", 1, 100, 20, |s| {
            s.management.login_limit.per_source.into()
        }),
        (
            "management.login_limit",
            "window_secs",
            60,
            86_400,
            900,
            |s| s.management.login_limit.window.as_secs(),
        ),
        (
            "management.login_limit",
            "lockout_secs",
            60,
            86_400,
            900,
            |s| s.management.login_limit.lockout.as_secs(),
        ),
        ("management.session", "idle_secs", 300, 28_800, 1800, |s| {
            s.management.session.idle.as_secs()
        }),
        (
            "management.session",
            "max_lifetime_secs",
            3600,
            259_200,
            43_200,
            |s| s.management.session.max_lifetime.as_secs(),
        ),
        ("dashboard", "poll_secs", 1, 60, 2, |s| {
            s.dashboard.poll.as_secs()
        }),
        ("dashboard", "query_timeout_secs", 1, 60, 5, |s| {
            s.dashboard.query_timeout.as_secs()
        }),
        ("recent_closed", "max_entries", 0, 10_000, 100, |s| {
            s.recent_closed.max_entries.into()
        }),
        ("reload", "settle_secs", 1, 60, 5, |s| {
            s.reload.settle.as_secs()
        }),
        ("reload", "poll_secs", 5, 600, 30, |s| {
            s.reload.poll.as_secs()
        }),
    ];
    let defaults = parse_ok(REQUIRED);
    for (table, key, min, max, default, get) in cases {
        let path = format!("{table}.{key}");
        // Keep Lw ≤ L when L is at its minimum.
        let extra = if key == "relay_connections" {
            "waiting = 1\n"
        } else {
            ""
        };
        let text = |value: &str| with_required(&format!("[{table}]\n{key} = {value}\n{extra}"));

        assert_eq!(get(&defaults), default, "{path} default");
        for value in [min, max] {
            let expected = u64::try_from(value).expect("bounds are not negative");
            assert_eq!(
                get(&parse_ok(&text(&value.to_string()))),
                expected,
                "{path} = {value}"
            );
        }
        for value in [min - 1, max + 1] {
            assert_eq!(
                problems(&text(&value.to_string())),
                [problem("config_value_out_of_range", &path)],
                "{path} = {value}"
            );
        }
        for value in [format!("\"{min}\""), format!("{min}.0"), "true".to_owned()] {
            assert_eq!(
                problems(&text(&value)),
                [problem("config_type_mismatch", &path)],
                "{path} = {value}"
            );
        }
    }
}

/// spec: operations.md:OPS-001 partial
#[test]
fn checks_relations_between_values() {
    let at_limit = with_required(
        "[limits]\nrelay_connections = 300\nwaiting = 300\ninitial_message_bytes = 16384\nbuffer_bytes = 16384\n\
         [management.session]\nidle_secs = 3600\nmax_lifetime_secs = 3600\n",
    );
    let settings = parse_ok(&at_limit);
    assert_eq!(settings.limits.waiting, 300);
    assert_eq!(settings.limits.initial_message_bytes, 16_384);
    assert_eq!(settings.management.session.idle, secs(3600));

    let over = with_required(
        "[limits]\nrelay_connections = 300\nwaiting = 301\ninitial_message_bytes = 32768\nbuffer_bytes = 16384\n\
         [management.session]\nidle_secs = 3601\nmax_lifetime_secs = 3600\n",
    );
    assert_eq!(
        problems(&over),
        [
            problem("config_value_out_of_range", "limits.waiting"),
            problem("config_relation_violated", "limits.initial_message_bytes"),
            problem("config_relation_violated", "management.session.idle_secs"),
        ]
    );
}

/// spec: operations.md:OPS-001 partial
#[test]
fn applies_default_ports_only_to_tcp_entry_points() {
    let settings = parse_ok(&with_required(
        "[relay.tcp_viewer]\nenabled = true\nlisten = \"203.0.113.10\"\nallow = []\n\
         [relay.tcp_server]\nenabled = true\nlisten = \"[2001:db8::1]\"\nallow = []\n",
    ));
    assert_eq!(
        settings.relay.tcp_viewer.listen,
        Some(addr("203.0.113.10:5901"))
    );
    assert_eq!(
        settings.relay.tcp_server.listen,
        Some(addr("[2001:db8::1]:5500"))
    );

    let settings = parse_ok(&with_required(
        "[relay.tcp_viewer]\nenabled = true\nlisten = \"203.0.113.10:6000\"\nallow = []\n",
    ));
    assert_eq!(
        settings.relay.tcp_viewer.listen,
        Some(addr("203.0.113.10:6000"))
    );

    let found = problems(
        r#"
[admin]
accounts_file = "a.toml"

[relay.tcp_viewer]
listen = "2001:db8::1"

[relay.tcp_server]
listen = "203.0.113.10:0"

[relay.ws_viewer]
listen = "203.0.113.10"

[relay.wss_viewer]
listen = "[2001:db8::1]"

[management]
listen = "127.0.0.1"

[status]
listen = "localhost:18081"
"#,
    );
    assert_eq!(
        found,
        [
            problem("config_listen_invalid", "relay.tcp_viewer.listen"),
            problem("config_listen_invalid", "relay.tcp_server.listen"),
            problem("config_listen_invalid", "relay.ws_viewer.listen"),
            problem("config_listen_invalid", "relay.wss_viewer.listen"),
            problem("config_listen_invalid", "management.listen"),
            problem("config_listen_invalid", "status.listen"),
        ]
    );
}

/// spec: operations.md:OPS-001 partial
#[test]
fn rejects_listen_conflicts_after_default_ports() {
    // The disabled tcp_server shares the management address and is not checked.
    let found = problems(
        r#"
[admin]
accounts_file = "a.toml"

[relay.tcp_viewer]
enabled = true
listen = "127.0.0.1"
allow = []

[relay.tcp_server]
listen = "127.0.0.1:18081"

[management]
listen = "127.0.0.1:18081"

[status]
listen = "127.0.0.1:5901"
"#,
    );
    assert_eq!(found, [problem("config_listen_conflict", "status.listen")]);

    let found = problems(&with_required(
        "[relay.tcp_server]\nenabled = true\nlisten = \"127.0.0.1:18080\"\nallow = []\n",
    ));
    assert_eq!(
        found,
        [problem("config_listen_conflict", "management.listen")]
    );
}

/// spec: operations.md:OPS-038 partial
#[test]
fn requires_allow_list_only_for_enabled_entry_points() {
    let found = problems(&with_required(
        "[relay.tcp_viewer]\nenabled = true\n\
         [relay.ws_viewer]\nenabled = true\nlisten = \"127.0.0.1:18090\"\n\
         [relay.wss_viewer]\nenabled = true\nlisten = \"127.0.0.1:18091\"\nallow = []\n",
    ));
    assert_eq!(
        found,
        [
            problem("config_missing_key", "relay.tcp_viewer.listen"),
            problem("config_missing_key", "relay.tcp_viewer.allow"),
            problem("config_missing_key", "relay.ws_viewer.allow"),
            problem("config_missing_key", "relay.wss_viewer.cert_file"),
            problem("config_missing_key", "relay.wss_viewer.key_file"),
        ]
    );

    // Disabled entry points skip the required-key check.
    let settings = parse_ok(&with_required("[relay.tcp_server]\nenabled = false\n"));
    assert_eq!(settings.relay.tcp_server, EntryPointSettings::default());

    // Disabled entry points still get format checks.
    assert_eq!(
        problems(&with_required("[relay.tcp_server]\nallow = [\"*\"]\n")),
        [problem(
            "config_ip_item_invalid",
            "relay.tcp_server.allow[0]"
        )]
    );
}

/// spec: operations.md:OPS-038 partial
#[test]
fn accepts_an_empty_allow_list() {
    let settings = parse_ok(&with_required(
        "[relay.tcp_viewer]\nenabled = true\nlisten = \"0.0.0.0:15901\"\nallow = []\n",
    ));
    assert_eq!(settings.relay.tcp_viewer.allow, Some(AllowList::default()));
    assert_eq!(settings.warnings(), []);
}

/// spec: operations.md:OPS-038 partial
#[test]
fn accepts_items_covering_a_whole_family_with_warnings() {
    let settings = parse_ok(&with_required(
        r#"
[relay.tcp_viewer]
enabled = true
listen = "0.0.0.0:15901"
allow = ["0.0.0.0/0", "198.51.100.0/24", "::/0"]

[relay.tcp_server]
enabled = true
listen = "0.0.0.0:15902"
allow = ["0.0.0.0-255.255.255.255", "::-ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff"]

[relay.ws_viewer]
allow = ["0.0.0.0/1", "128.0.0.0/1"]

[mode1]
enabled = true
allow_targets = ["0.0.0.0/0:5900", "pc1.example:5900"]
"#,
    ));
    let source = |entry_point, text: &str| ConfigWarning::AllowAllSource {
        entry_point,
        item: IpItem::parse(text).expect("item"),
    };
    assert_eq!(
        settings.warnings(),
        [
            source(EntryPoint::TcpViewer, "0.0.0.0/0"),
            source(EntryPoint::TcpViewer, "::/0"),
            source(EntryPoint::TcpServer, "0.0.0.0-255.255.255.255"),
            source(
                EntryPoint::TcpServer,
                "::-ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff"
            ),
            ConfigWarning::AllowAllTarget {
                item: TargetItem::parse("0.0.0.0/0:5900").expect("target"),
            },
        ]
    );
}

/// spec: operations.md:OPS-040 partial
#[test]
fn accepts_single_cidr_and_range_items_of_both_families() {
    let items = [
        "198.51.100.7",
        "198.51.100.0/24",
        "203.0.113.10-203.0.113.20",
        "2001:db8::1",
        "2001:db8::/32",
        "2001:db8::10-2001:db8::20",
    ];
    let list = items.map(|item| format!("\"{item}\"")).join(", ");
    let settings = parse_ok(&with_required(&format!(
        "[relay.tcp_viewer]\nenabled = true\nlisten = \"0.0.0.0:15901\"\nallow = [{list}]\n"
    )));
    assert_eq!(settings.relay.tcp_viewer.allow, Some(allow(&items)));
}

/// spec: operations.md:OPS-040 partial
#[test]
fn rejects_bad_ip_items_with_their_own_codes() {
    let found = problems(&with_required(
        r#"
[relay.tcp_viewer]
enabled = true
listen = "0.0.0.0:15901"
allow = [
    "203.0.113.20-203.0.113.10",
    "192.0.2.1-2001:db8::1",
    "192.0.2.1/24",
    "::ffff:192.0.2.1",
    "",
    "198.51.100.07",
    "198.51.100.0/24",
]
"#,
    ));
    assert_eq!(
        found,
        [
            problem("config_ip_range_reversed", "relay.tcp_viewer.allow[0]"),
            problem("config_ip_range_family_mixed", "relay.tcp_viewer.allow[1]"),
            problem("config_ip_item_invalid", "relay.tcp_viewer.allow[2]"),
            problem("config_ip_item_invalid", "relay.tcp_viewer.allow[3]"),
            problem("config_ip_item_invalid", "relay.tcp_viewer.allow[4]"),
            problem("config_ip_item_invalid", "relay.tcp_viewer.allow[5]"),
        ]
    );

    let found = problems(&with_required(
        "[relay.tcp_viewer]\nenabled = true\nlisten = \"0.0.0.0:15901\"\nallow = [\"192.0.2.1\", 7]\n",
    ));
    assert_eq!(
        found,
        [problem("config_type_mismatch", "relay.tcp_viewer.allow[1]")]
    );
}

#[test]
fn checks_origin_values_and_entry_point_specific_keys() {
    let found = problems(&with_required(
        r#"
[relay.tcp_viewer]
allowed_origins = ["https://viewer.example"]
cert_file = "a.pem"

[relay.ws_viewer]
allowed_origins = ["https://viewer.example", "null", "https://viewer.example/", "https://*.example"]
cert_file = "a.pem"
key_file = "a.key"
"#,
    ));
    assert_eq!(
        found,
        [
            problem("config_unknown_key", "relay.tcp_viewer.allowed_origins"),
            problem("config_unknown_key", "relay.tcp_viewer.cert_file"),
            problem(
                "config_origin_invalid",
                "relay.ws_viewer.allowed_origins[1]"
            ),
            problem(
                "config_origin_invalid",
                "relay.ws_viewer.allowed_origins[2]"
            ),
            problem(
                "config_origin_invalid",
                "relay.ws_viewer.allowed_origins[3]"
            ),
            problem("config_unknown_key", "relay.ws_viewer.cert_file"),
            problem("config_unknown_key", "relay.ws_viewer.key_file"),
        ]
    );
}

#[test]
fn checks_mode1_targets() {
    let found = problems(&with_required("[mode1]\nenabled = true\n"));
    assert_eq!(
        found,
        [problem("config_missing_key", "mode1.allow_targets")]
    );

    let found = problems(&with_required(
        r#"
[mode1]
allow_targets = [
    "192.0.2.20",
    "192.0.2.20:5905-5900",
    "[2001:db8::9-2001:db8::1]:5900",
    "*.example:5900",
    "192.0.2:5900",
    "pc1.example:5900",
]
"#,
    ));
    let expected: Vec<_> = (0..5)
        .map(|index| {
            problem(
                "config_target_item_invalid",
                &format!("mode1.allow_targets[{index}]"),
            )
        })
        .collect();
    assert_eq!(found, expected);

    let settings = parse_ok(&with_required(
        "[mode1]\nenabled = true\nallow_targets = []\n",
    ));
    assert_eq!(settings.mode1.allow_targets, Some(TargetPolicy::default()));
}

/// spec: operations.md:OPS-034 partial
#[test]
fn allows_plaintext_management_only_on_loopback() {
    let config = |management: &str| {
        format!(
            "[admin]\naccounts_file = \"a.toml\"\n[status]\nlisten = \"127.0.0.1:18081\"\n[management]\n{management}\n"
        )
    };
    for listen in ["127.0.0.1:18080", "127.10.0.1:18080", "[::1]:18080"] {
        let settings = parse_ok(&config(&format!("listen = \"{listen}\"")));
        assert_eq!(settings.management.listen, addr(listen));
        assert_eq!(settings.management.tls, None);
    }
    for listen in ["0.0.0.0:18080", "[::]:18080", "198.51.100.1:18080"] {
        assert_eq!(
            problems(&config(&format!("listen = \"{listen}\""))),
            [problem(
                "config_plaintext_management_not_loopback",
                "management.listen"
            )],
            "{listen}"
        );
    }
    assert_eq!(
        problems(&config(
            "listen = \"0.0.0.0:18443\"\ncert_file = \"admin.pem\""
        )),
        [problem("config_tls_pair_incomplete", "management.key_file")]
    );
    assert_eq!(
        problems(&config(
            "listen = \"0.0.0.0:18443\"\nkey_file = \"admin.key\""
        )),
        [problem(
            "config_tls_pair_incomplete",
            "management.cert_file"
        )]
    );
    let settings = parse_ok(&config(
        "listen = \"0.0.0.0:18443\"\ncert_file = \"admin.pem\"\nkey_file = \"admin.key\"",
    ));
    assert_eq!(
        settings.management.tls,
        Some(TlsFiles {
            cert_file: base_dir().join("admin.pem"),
            key_file: base_dir().join("admin.key"),
        })
    );
}

#[test]
fn warns_for_plaintext_ws_and_non_loopback_status() {
    let settings = parse_ok(
        r#"
[admin]
accounts_file = "a.toml"

[relay.ws_viewer]
enabled = true
listen = "127.0.0.1:18090"
allow = ["127.0.0.1"]

[management]
listen = "127.0.0.1:18080"

[status]
listen = "0.0.0.0:18081"
"#,
    );
    assert_eq!(
        settings.warnings(),
        [
            ConfigWarning::PlaintextWsEnabled {
                listen: addr("127.0.0.1:18090"),
            },
            ConfigWarning::StatusListenNotLoopback {
                listen: addr("0.0.0.0:18081"),
            },
        ]
    );
    let disabled = parse_ok(&with_required(
        "[relay.ws_viewer]\nlisten = \"127.0.0.1:18090\"\n",
    ));
    assert_eq!(disabled.warnings(), []);
}

#[test]
fn rejects_empty_paths_and_reads_log_output() {
    assert_eq!(
        problems(
            "[admin]\naccounts_file = \"\"\n[management]\nlisten = \"127.0.0.1:18080\"\n\
             [status]\nlisten = \"127.0.0.1:18081\"\n[log]\noutput = \"\"\n"
        ),
        [
            problem("config_value_out_of_range", "admin.accounts_file"),
            problem("config_value_out_of_range", "log.output"),
        ]
    );
    let output = |value: &str| {
        parse_ok(&with_required(&format!("[log]\noutput = '{value}'\n")))
            .log
            .output
    };
    assert_eq!(output("stdout"), LogOutput::Stdout);
    assert_eq!(
        output("./stdout"),
        LogOutput::File(base_dir().join("./stdout"))
    );
    let absolute = if cfg!(windows) {
        r"D:\logs\relay.log"
    } else {
        "/var/log/relay.log"
    };
    assert_eq!(output(absolute), LogOutput::File(PathBuf::from(absolute)));
}

#[test]
fn rejects_non_table_values_in_place_of_tables() {
    assert_eq!(
        problems(&format!("relay = 5\n{REQUIRED}")),
        [problem("config_type_mismatch", "relay")]
    );
    assert_eq!(
        problems(
            "[admin]\naccounts_file = \"a.toml\"\nargon2 = \"strong\"\n[management]\n\
             listen = \"127.0.0.1:18080\"\n[status]\nlisten = \"127.0.0.1:18081\"\n"
        ),
        [problem("config_type_mismatch", "admin.argon2")]
    );
}
