//! Public-service/real-TCP acceptance coverage for delayed stale cleanup.
//! Parks the old waiting owner's future across ID reuse without a product-only
//! test hook or a fake stream.

mod delayed_cleanup {
    use std::future::{Future, poll_fn};
    use std::sync::{Arc, Mutex, PoisonError};
    use std::task::Poll;
    use std::time::{Duration, SystemTime};

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::watch;
    use vnc_relay_adapters::clock::SystemClock;
    use vnc_relay_adapters::crypto::{RelayIdKey, SessionIdKey};
    use vnc_relay_adapters::net::{AcceptPolicy, AllowSource, TcpEntryPoint, TcpRelayStream};
    use vnc_relay_adapters::ops_log::format_line;
    use vnc_relay_adapters::target::TcpTargetConnector;
    use vnc_relay_app::events::Event;
    use vnc_relay_app::ports::{Clock, EventSink};
    use vnc_relay_app::relay::{
        ConnectionContext, PreparedConnection, RelayRuntime, RelayRuntimeConfig, RelayService,
    };
    use vnc_relay_app::reload::ports::RuntimeSettingsSource;
    use vnc_relay_app::reload::snapshot::{ReloadCell, ReloadSnapshot};
    use vnc_relay_app::reload::{ConfigCandidate, ConfigPresence};
    use vnc_relay_app::settings::KeepaliveSettings;
    use vnc_relay_domain::allow_list::{AllowList, IpItem};
    use vnc_relay_domain::budget::ConnectionBudget;
    use vnc_relay_domain::conn_id::ConnId;
    use vnc_relay_domain::entry_point::EntryPoint;
    use vnc_relay_domain::handshake::HandshakePolicy;
    use vnc_relay_domain::peer_ip::PeerIp;
    use vnc_relay_domain::target_policy::TargetPolicy;

    const GREETING: &[u8] = b"RFB 000.000\n";
    const LIMIT: Duration = Duration::from_secs(30);

    fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
        let prefix = format!("\"{name}\":\"");
        let rest = &line[line.find(&prefix)? + prefix.len()..];
        Some(&rest[..rest.find('"')?])
    }

    fn event(line: &str, name: &str) -> bool {
        field(line, "event") == Some(name)
    }

    fn request(id: &str) -> [u8; 250] {
        let text = format!("ID:{id}");
        assert!(text.len() <= 250);
        let mut bytes = [0; 250];
        bytes[..text.len()].copy_from_slice(text.as_bytes());
        bytes
    }

    const KEEPALIVE: KeepaliveSettings = KeepaliveSettings {
        tcp_idle: Duration::from_secs(60),
        tcp_interval: Duration::from_secs(10),
        tcp_probes: 6,
        ws_ping: Duration::from_secs(30),
        ws_pong_timeout: Duration::from_secs(30),
    };

    #[derive(Default)]
    struct Recorder(Mutex<Vec<String>>);

    impl EventSink for Recorder {
        fn record(&self, value: &Event) {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(format_line(value, SystemTime::now()));
        }
    }

    impl Recorder {
        fn lines(&self) -> Vec<String> {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    async fn bounded<T>(future: impl Future<Output = T>) -> T {
        tokio::time::timeout(LIMIT, future)
            .await
            .unwrap_or_else(|error| panic!(concat!("bounded real TCP operation", ": {:?}"), error))
    }

    async fn until(mut predicate: impl FnMut() -> bool) {
        bounded(async {
            while !predicate() {
                tokio::task::yield_now().await;
            }
        })
        .await;
    }

    async fn prepare_server(
        listener: &TcpListener,
        service: &RelayService,
        runtime: &RelayRuntime<TcpTargetConnector>,
        clock: &dyn Clock,
        budget: &Arc<ConnectionBudget>,
    ) -> (PreparedConnection<TcpRelayStream>, TcpStream) {
        let mut peer =
            bounded(TcpStream::connect(listener.local_addr().unwrap_or_else(
                |error| panic!(concat!("Server address", ": {:?}"), error),
            )))
            .await
            .unwrap_or_else(|error| panic!(concat!("real Server connects", ": {:?}"), error));
        let (accepted, address) = bounded(listener.accept())
            .await
            .unwrap_or_else(|error| panic!(concat!("accept actual TCP Server", ": {:?}"), error));
        let accepted_at = clock.monotonic();
        let context = ConnectionContext {
            conn_id: ConnId::next(),
            entry_point: EntryPoint::TcpServer,
            peer_ip: PeerIp::from(address),
            forwarded_ip: None,
            peer_addr: address,
            accepted_wall: clock.wall(),
            accepted_at,
            exchange_at: accepted_at,
            buffer_bytes: 16384,
            handshake: Duration::from_secs(20),
        };
        bounded(peer.write_all(&request("9201")))
            .await
            .unwrap_or_else(|error| panic!(concat!("Server request", ": {:?}"), error));
        let prepared = bounded(
            service.handle_connection(
                TcpRelayStream::with_keepalive(accepted, &KEEPALIVE).unwrap_or_else(|error| {
                    panic!(concat!("apply real TCP keepalive", ": {:?}"), error)
                }),
                context,
                budget
                    .try_acquire(8)
                    .unwrap_or_else(|| panic!("available connection budget")),
                runtime.lifecycle(),
            ),
        )
        .await
        .unwrap_or_else(|error| panic!(concat!("prepare actual Server socket", ": {:?}"), error));
        (prepared, peer)
    }

    async fn viewer(address: std::net::SocketAddr) -> TcpStream {
        let mut peer = bounded(TcpStream::connect(address))
            .await
            .unwrap_or_else(|error| panic!(concat!("real Viewer connects", ": {:?}"), error));
        let mut greeting = [0; 12];
        bounded(peer.read_exact(&mut greeting))
            .await
            .unwrap_or_else(|error| panic!(concat!("TCP entry sends greeting", ": {:?}"), error));
        assert_eq!(&greeting[..], GREETING);
        bounded(peer.write_all(&request("9201")))
            .await
            .unwrap_or_else(|error| panic!(concat!("Viewer request", ": {:?}"), error));
        peer
    }

    async fn exchange(viewer: &mut TcpStream, server: &mut TcpStream, bytes: &[u8]) {
        bounded(viewer.write_all(bytes))
            .await
            .unwrap_or_else(|error| panic!(concat!("Viewer payload", ": {:?}"), error));
        bounded(server.write_all(bytes))
            .await
            .unwrap_or_else(|error| panic!(concat!("Server payload", ": {:?}"), error));
        let mut received = vec![0; bytes.len()];
        bounded(server.read_exact(&mut received))
            .await
            .unwrap_or_else(|error| {
                panic!(concat!("Server receives exact payload", ": {:?}"), error)
            });
        assert_eq!(received, bytes);
        bounded(viewer.read_exact(&mut received))
            .await
            .unwrap_or_else(|error| {
                panic!(concat!("Viewer receives exact payload", ": {:?}"), error)
            });
        assert_eq!(received, bytes);
    }

    /// spec: relay.md:RLY-016
    #[test]
    fn retired_waiting_owner_resumes_after_same_id_reuse_without_removing_the_new_session() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap_or_else(|error| {
                panic!(concat!("real-socket scheduling runtime", ": {:?}"), error)
            })
            .block_on(async {
                let viewer_listener = TcpListener::bind("127.0.0.1:0")
                    .await
                    .unwrap_or_else(|error| panic!(concat!("Viewer bind", ": {:?}"), error));
                let viewer_address = viewer_listener
                    .local_addr()
                    .unwrap_or_else(|error| panic!(concat!("Viewer address", ": {:?}"), error));
                let server_listener = TcpListener::bind("127.0.0.1:0")
                    .await
                    .unwrap_or_else(|error| panic!(concat!("Server bind", ": {:?}"), error));
                let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
                let events = Arc::new(Recorder::default());
                let log: Arc<dyn EventSink> = events.clone();
                let mut settings = vnc_relay_adapters::config_file::parse(
                    "[admin]\naccounts_file = 'accounts.toml'\n\
                     [management]\nlisten = '127.0.0.1:18080'\n\
                     [status]\nlisten = '127.0.0.1:18081'\n",
                    &std::env::temp_dir(),
                )
                .unwrap_or_else(|error| panic!("real fixture settings: {error:?}"));
                settings.relay.tcp_viewer.enabled = true;
                settings.relay.tcp_viewer.listen = Some(viewer_address);
                settings.relay.tcp_viewer.allow = Some(AllowList::new(vec![
                    IpItem::parse("127.0.0.1")
                        .unwrap_or_else(|error| panic!(concat!("loopback allow", ": {:?}"), error)),
                ]));
                settings.mode1.allow_targets = Some(TargetPolicy::new(Vec::new()));
                settings.mode2.enabled = true;
                settings.limits.relay_connections = 8;
                settings.limits.waiting = 4;
                settings.limits.buffer_bytes = 16384;
                settings.timeouts.handshake = Duration::from_secs(20);
                settings.timeouts.wait = Duration::from_secs(60);
                settings.timeouts.connect = Duration::from_secs(1);
                settings.timeouts.close = Duration::from_secs(2);
                settings.keepalive = KEEPALIVE;
                settings.recent_closed.max_entries = 8;
                let cell = Arc::new(ReloadCell::new(ReloadSnapshot::new(
                    ConfigCandidate {
                        settings,
                        presence: ConfigPresence::default(),
                    },
                    None,
                    None,
                )));
                let settings: Arc<dyn RuntimeSettingsSource> = cell.clone();
                let budget = ConnectionBudget::new();
                let runtime = Arc::new(RelayRuntime::new(
                    TcpTargetConnector::new(Arc::clone(&settings)),
                    Arc::new(RelayIdKey::generate().unwrap_or_else(|error| {
                        panic!(concat!("real relay reference key", ": {:?}"), error)
                    })),
                    Arc::new(SessionIdKey::from_bytes(&[0x49; 32])),
                    Arc::clone(&budget),
                    ConnectionBudget::new(),
                    RelayRuntimeConfig { recent_closed: 8 },
                    Arc::clone(&settings),
                ));
                let service = Arc::new(RelayService::new(
                    HandshakePolicy {
                        mode1_enabled: false,
                        mode2_enabled: true,
                    },
                    Arc::clone(&settings),
                    Arc::clone(&clock),
                    Arc::clone(&log),
                ));
                let entry = TcpEntryPoint::new(
                    EntryPoint::TcpViewer,
                    viewer_listener,
                    AcceptPolicy {
                        allow: AllowSource::tcp_viewer(cell),
                        relay_budget: Arc::clone(&budget),
                    },
                    Arc::clone(&service),
                    Arc::clone(&runtime),
                    Arc::clone(&clock),
                );
                let (stop, stopped) = watch::channel(false);
                let entry_task = tokio::spawn(entry.run(log, stopped));
                let (prepared, mut old_server) = prepare_server(
                    &server_listener,
                    &service,
                    &runtime,
                    clock.as_ref(),
                    &budget,
                )
                .await;
                let old_conn = prepared.context().conn_id;
                let mut old_owner = Box::pin(service.dispatch(prepared, &runtime));
                poll_fn(|cx| {
                    assert!(old_owner.as_mut().poll(cx).is_pending());
                    Poll::Ready(())
                })
                .await;
                let waiting = runtime.snapshot(clock.as_ref()).waiting;
                assert_eq!(waiting.len(), 1);
                assert_eq!(waiting[0].context.conn_id, old_conn);

                // Do not poll old_owner again. The real Viewer entry owns the
                // matched pair and completes the session independently, leaving
                // the original waiting future's WaitingGuard cleanup delayed.
                let mut old_viewer = viewer(viewer_address).await;
                until(|| runtime.snapshot(clock.as_ref()).active.len() == 1).await;
                let old_id = runtime.snapshot(clock.as_ref()).active[0]
                    .session_id
                    .clone();
                exchange(&mut old_viewer, &mut old_server, b"old-session").await;
                bounded(old_viewer.shutdown())
                    .await
                    .unwrap_or_else(|error| panic!(concat!("old Viewer EOF", ": {:?}"), error));
                bounded(old_server.shutdown())
                    .await
                    .unwrap_or_else(|error| panic!(concat!("old Server EOF", ": {:?}"), error));
                until(|| {
                    let snapshot = runtime.snapshot(clock.as_ref());
                    snapshot.active.is_empty() && snapshot.recent_closed.len() == 1
                })
                .await;

                let (prepared, mut new_server) = prepare_server(
                    &server_listener,
                    &service,
                    &runtime,
                    clock.as_ref(),
                    &budget,
                )
                .await;
                let new_conn = prepared.context().conn_id;
                assert_ne!(old_conn, new_conn);
                let new_owner = {
                    let service = Arc::clone(&service);
                    let runtime = Arc::clone(&runtime);
                    tokio::spawn(async move { service.dispatch(prepared, &runtime).await })
                };
                let mut new_viewer = viewer(viewer_address).await;
                until(|| runtime.snapshot(clock.as_ref()).active.len() == 1).await;
                let new_id = runtime.snapshot(clock.as_ref()).active[0]
                    .session_id
                    .clone();
                assert_ne!(
                    old_id, new_id,
                    "same relay ID gets a new session identifier"
                );
                exchange(&mut new_viewer, &mut new_server, b"before-stale-cleanup").await;

                // This is the actual stale cleanup, resumed only AFTER the new
                // session exists. Awaiting the pinned future drops its old
                // connection-specific WaitingGuard, not the new registration.
                bounded(old_owner).await.unwrap_or_else(|error| {
                    panic!(concat!("retired waiting owner completes", ": {:?}"), error)
                });
                let snapshot = runtime.snapshot(clock.as_ref());
                assert!(snapshot.waiting.is_empty());
                assert_eq!(snapshot.active.len(), 1);
                assert_eq!(snapshot.active[0].session_id, new_id);
                assert_eq!(
                    snapshot.active[0]
                        .peer
                        .server()
                        .unwrap_or_else(|| panic!("new Server metadata"))
                        .conn_id,
                    new_conn
                );
                assert_eq!(snapshot.recent_closed.len(), 1);
                assert_eq!(snapshot.recent_closed[0].session_id, old_id);
                exchange(&mut new_viewer, &mut new_server, b"after-stale-cleanup").await;

                bounded(new_viewer.shutdown())
                    .await
                    .unwrap_or_else(|error| panic!(concat!("new Viewer EOF", ": {:?}"), error));
                bounded(new_server.shutdown())
                    .await
                    .unwrap_or_else(|error| panic!(concat!("new Server EOF", ": {:?}"), error));
                bounded(new_owner)
                    .await
                    .unwrap_or_else(|error| {
                        panic!(concat!("new waiting owner joins", ": {:?}"), error)
                    })
                    .unwrap_or_else(|error| {
                        panic!(concat!("new owner completes", ": {:?}"), error)
                    });
                until(|| {
                    let snapshot = runtime.snapshot(clock.as_ref());
                    snapshot.active.is_empty() && snapshot.recent_closed.len() == 2
                })
                .await;
                let lines = events.lines();
                for id in [&old_id, &new_id] {
                    let id = id.to_string();
                    for name in ["session_started", "session_closed"] {
                        assert_eq!(
                            lines
                                .iter()
                                .filter(|line| {
                                    event(line, name)
                                        && field(line, "session_id") == Some(id.as_str())
                                })
                                .count(),
                            1,
                            "one lifecycle event per session"
                        );
                    }
                }
                stop.send_replace(true);
                bounded(entry_task).await.unwrap_or_else(|error| {
                    panic!(concat!("real TCP entry stops", ": {:?}"), error)
                });
                let released: Vec<_> = (0..8)
                    .map(|_| {
                        budget
                            .try_acquire(8)
                            .unwrap_or_else(|| panic!("socket permit released"))
                    })
                    .collect();
                assert!(budget.try_acquire(8).is_none());
                drop(released);
            });
    }
}
