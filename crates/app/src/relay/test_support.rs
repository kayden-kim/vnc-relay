//! Shared app unit-test settings source; no transport or filesystem dependencies.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::reload::ports::RuntimeSettingsSource;
use crate::settings::*;

pub(crate) struct TestSettings(pub(crate) RwLock<Settings>);

impl RuntimeSettingsSource for TestSettings {
    fn with_current(&self, read: &mut dyn FnMut(&Settings)) {
        read(&self.0.read().expect("settings read"));
    }
}

impl TestSettings {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self(RwLock::new(settings())))
    }

    pub(crate) fn change(&self, change: impl FnOnce(&mut Settings)) {
        change(&mut self.0.write().expect("settings write"));
    }
}

pub(crate) fn settings() -> Settings {
    let listen = "127.0.0.1:8080".parse().expect("fixture address");
    Settings {
        admin: AdminSettings {
            accounts_file: "accounts.json".into(),
            argon2: Argon2Settings {
                memory_kib: 19_456,
                iterations: 2,
                parallelism: 1,
            },
        },
        relay: RelaySettings {
            tcp_viewer: EntryPointSettings::default(),
            tcp_server: EntryPointSettings::default(),
            ws: WsListenerSettings::default(),
            wss: WsListenerSettings::default(),
            ticket: TicketSettings::default(),
        },
        mode1: Mode1Settings::default(),
        mode2: Mode2Settings { enabled: true },
        limits: LimitSettings {
            relay_connections: 20,
            admin_connections: 10,
            waiting: 10,
            buffer_bytes: 16,
            ws_message_bytes: 65536,
        },
        timeouts: TimeoutSettings {
            handshake: Duration::from_secs(10),
            tls_upgrade: Duration::from_secs(10),
            wait: Duration::from_secs(5),
            connect: Duration::from_secs(4),
            close: Duration::from_secs(2),
            shutdown_grace: Duration::from_secs(10),
        },
        keepalive: KeepaliveSettings {
            tcp_idle: Duration::from_secs(60),
            tcp_interval: Duration::from_secs(10),
            tcp_probes: 3,
            ws_ping: Duration::from_secs(30),
            ws_pong_timeout: Duration::from_secs(10),
        },
        management: ManagementSettings {
            listen,
            tls: None,
            config_edit: false,
            login_limit: LoginLimitSettings {
                per_account: 5,
                per_source: 20,
                window: Duration::from_secs(60),
                lockout: Duration::from_secs(60),
            },
            session: AdminSessionSettings {
                idle: Duration::from_secs(900),
                max_lifetime: Duration::from_secs(28800),
            },
        },
        dashboard: DashboardSettings {
            poll: Duration::from_secs(2),
            query_timeout: Duration::from_secs(5),
        },
        recent_closed: RecentClosedSettings { max_entries: 2 },
        recent_logs: RecentLogsSettings { max_entries: 1000 },
        status: StatusSettings { listen },
        log: LogSettings {
            output: LogOutput::Stdout,
        },
        reload: ReloadSettings {
            settle: Duration::from_secs(2),
            poll: Duration::from_secs(5),
        },
    }
}
