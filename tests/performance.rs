//! Long-running measurements against the real executable; see the linked how-to.

use std::collections::HashSet;
use std::io::{self, Read, Write};
use std::net::TcpListener as PortReservation;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, ready};
use std::time::{Duration, Instant};

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex as AsyncMutex, watch};
use tokio::task::JoinHandle;

const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");
const B: usize = 131_072;
/// Q119·Q123: lifecycle rounds, and the round from which RSS growth is judged.
const ROUNDS: usize = 15;
const RSS_FROM_ROUND: usize = 10;
/// Q121: socket buffer size of every test client connected to the relay.
const CLIENT_SOCKET_BUFFER: u32 = 64 * 1024;
const M: usize = 1_048_576;
const CHUNK: usize = 4096;
const RATE: u64 = 125_000;
const GREETING: &[u8] = b"RFB 000.000\n";
const DEADLINE: Duration = Duration::from_secs(120);
static PORTS: AsyncMutex<()> = AsyncMutex::const_new(());

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
trait Wire: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Wire for T {}
type WireBox = Box<dyn Wire>;
type WireReader = tokio::io::ReadHalf<WireBox>;
type WireWriter = tokio::io::WriteHalf<WireBox>;

fn must<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("performance harness operation: {error:?}"))
}
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/adapters/tests/fixtures/tls")
        .join(name)
}
fn output_dir() -> PathBuf {
    let path = std::env::var_os("VNC_RELAY_PERF_OUT").map_or_else(
        || PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("perf"),
        PathBuf::from,
    );
    must(std::fs::create_dir_all(&path));
    path
}
fn save(name: &str, result: &Value) {
    must(std::fs::write(
        output_dir().join(format!("{name}.json")),
        must(serde_json::to_vec_pretty(result)),
    ));
}
fn private_file(path: &Path, text: &str) {
    must(std::fs::write(path, text));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        must(std::fs::set_permissions(
            path,
            std::fs::Permissions::from_mode(0o600),
        ));
    }
}

#[derive(Clone, Copy)]
enum Transport {
    Tcp,
    Ws,
    Wss,
}
impl Transport {
    const ALL: [Self; 3] = [Self::Tcp, Self::Ws, Self::Wss];
    fn name(self) -> &'static str {
        match self {
            Self::Tcp => "tcp_viewer",
            Self::Ws => "ws_viewer",
            Self::Wss => "wss_viewer",
        }
    }
    fn index(self) -> usize {
        match self {
            Self::Tcp => 2,
            Self::Ws => 4,
            Self::Wss => 5,
        }
    }
}

struct Relay {
    child: Child,
    dir: PathBuf,
    ports: [u16; 6],
}
impl Relay {
    async fn start(test: &str, transport: Transport) -> Self {
        let holders: Vec<_> = (0..6)
            .map(|_| must(PortReservation::bind("127.0.0.1:0")))
            .collect();
        let ports: [u16; 6] = std::array::from_fn(|i| must(holders[i].local_addr()).port());
        let dir = output_dir().join(format!(
            "{test}-{}-{}",
            transport.name(),
            std::process::id()
        ));
        must(std::fs::create_dir(&dir));
        private_file(
            &dir.join("accounts.toml"),
            "version = 1\n[[account]]\nname = 'alice'\npassword_hash = '$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA'\n",
        );
        private_file(
            &dir.join("relay.toml"),
            &format!(
                r#"[admin]
accounts_file = 'accounts.toml'
[management]
listen = '127.0.0.1:{}'
[status]
listen = '127.0.0.1:{}'
[relay.tcp_viewer]
enabled = true
listen = '127.0.0.1:{}'
allow = ['127.0.0.1']
[relay.tcp_server]
enabled = true
listen = '127.0.0.1:{}'
allow = ['127.0.0.1']
[relay.ws_viewer]
enabled = true
listen = '127.0.0.1:{}'
allow = ['127.0.0.1']
[relay.wss_viewer]
enabled = true
listen = '127.0.0.1:{}'
allow = ['127.0.0.1']
cert_file = '{}'
key_file = '{}'
[mode1]
enabled = false
[mode2]
enabled = true
[limits]
relay_connections = 1000
waiting = 200
buffer_bytes = {B}
ws_message_bytes = {M}
[timeouts]
handshake_secs = 30
wait_secs = 120
close_secs = 5
[keepalive]
ws_ping_secs = 300
ws_pong_timeout_secs = 300
[log]
output = 'relay.log'
"#,
                ports[0],
                ports[1],
                ports[2],
                ports[3],
                ports[4],
                ports[5],
                fixture("server-chain.pem").display(),
                fixture("server-key-p256.pem").display()
            ),
        );
        drop(holders);
        let child = must(
            Command::new(BIN)
                .args(["serve", "--config"])
                .arg(dir.join("relay.toml"))
                .stdin(Stdio::null())
                .stdout(Stdio::from(must(std::fs::File::create(
                    dir.join("stdout.log"),
                ))))
                .stderr(Stdio::from(must(std::fs::File::create(
                    dir.join("stderr.log"),
                ))))
                .spawn(),
        );
        let mut relay = Self { child, dir, ports };
        relay
            .wait_logs(|events| events.iter().any(|e| e["event"] == "server_ready"))
            .await;
        relay
    }
    fn events(&self) -> Vec<Value> {
        let text = match std::fs::read_to_string(self.dir.join("relay.log")) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Vec::new(),
            Err(error) => panic!("read ops log: {error}"),
        };
        text.rsplit_once('\n')
            .map_or_else(Vec::new, |(complete, _)| {
                complete
                    .lines()
                    .map(|line| must(serde_json::from_str(line)))
                    .collect()
            })
    }
    async fn wait_logs(&mut self, predicate: impl Fn(&[Value]) -> bool) -> Vec<Value> {
        let end = Instant::now() + DEADLINE;
        loop {
            let events = self.events();
            if predicate(&events) {
                return events;
            }
            assert!(
                must(self.child.try_wait()).is_none(),
                "relay exited; see {}",
                self.dir.display()
            );
            assert!(
                Instant::now() < end,
                "ops log deadline; see {}",
                self.dir.display()
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
    async fn quiescent(&mut self, expected: usize) -> Value {
        let events = self
            .wait_logs(|events| {
                let state = log_state(events);
                state["started"] == expected
                    && state["closed"] == expected
                    && state["waiting"] == 0
                    && state["active"] == 0
            })
            .await;
        log_state(&events)
    }
    async fn pair(&self, transport: Transport, id: usize) -> TestResult<(Peer, Peer)> {
        let mut server = Peer::connect(self.ports[3], Transport::Tcp, false).await?;
        server.send(&request(id)).await?;
        let mut viewer = Peer::connect(self.ports[transport.index()], transport, true).await?;
        viewer.send(&request(id)).await?;
        Ok((viewer, server))
    }
}
impl Drop for Relay {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn log_state(events: &[Value]) -> Value {
    let mut waiting = HashSet::new();
    let mut active = HashSet::new();
    let mut started = 0;
    let mut closed = 0;
    for e in events {
        match e["event"].as_str() {
            Some("waiting_registered") => {
                waiting.insert(e["conn_id"].as_str().unwrap_or_default().to_owned());
            }
            Some("waiting_removed" | "waiting_expired") => {
                waiting.remove(e["conn_id"].as_str().unwrap_or_default());
            }
            Some("session_started") => {
                started += 1;
                waiting.remove(e["server_conn_id"].as_str().unwrap_or_default());
                waiting.remove(e["viewer_conn_id"].as_str().unwrap_or_default());
                active.insert(e["session_id"].as_str().unwrap_or_default().to_owned());
            }
            Some("session_closed") => {
                closed += 1;
                active.remove(e["session_id"].as_str().unwrap_or_default());
            }
            _ => {}
        }
    }
    json!({"waiting":waiting.len(), "active":active.len(), "started":started, "closed":closed})
}
fn request(id: usize) -> [u8; 250] {
    let text = format!("ID:{id}");
    let mut bytes = [0; 250];
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    bytes
}

// A small safe async adapter avoids adding a tokio-rustls dev dependency.
// rustls owns TLS records; readiness and all socket I/O remain Tokio's.
struct TlsWire {
    socket: TcpStream,
    tls: ClientConnection,
    handshake_flushed: bool,
}
impl TlsWire {
    fn flush_tls(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        while self.tls.wants_write() {
            ready!(self.socket.poll_write_ready(cx))?;
            match self.tls.write_tls(&mut SocketWrite(&self.socket)) {
                Ok(0) => return Poll::Ready(Err(io::ErrorKind::WriteZero.into())),
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
        Poll::Ready(Ok(()))
    }
}
struct SocketRead<'a>(&'a TcpStream);
impl Read for SocketRead<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.0.try_read(bytes)
    }
}
struct SocketWrite<'a>(&'a TcpStream);
impl Write for SocketWrite<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.try_write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl AsyncRead for TlsWire {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        out: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            if !this.handshake_flushed {
                ready!(this.flush_tls(cx))?;
                this.handshake_flushed = !this.tls.is_handshaking();
            }
            // Established reads must not wait for a backpressured write.
            // The independent writer flushes application and control records.
            match this.tls.reader().read(out.initialize_unfilled()) {
                Ok(n) => {
                    out.advance(n);
                    return Poll::Ready(Ok(()));
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Poll::Ready(Err(e)),
            }
            ready!(this.socket.poll_read_ready(cx))?;
            match this.tls.read_tls(&mut SocketRead(&this.socket)) {
                Ok(0) => return Poll::Ready(Err(io::ErrorKind::UnexpectedEof.into())),
                Ok(_) => {
                    this.tls.process_new_packets().map_err(io::Error::other)?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
    }
}
impl AsyncWrite for TlsWire {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.flush_tls(cx))?;
        Poll::Ready(this.tls.writer().write(bytes))
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.get_mut().flush_tls(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        ready!(self.as_mut().poll_flush(cx))?;
        Pin::new(&mut self.get_mut().socket).poll_shutdown(cx)
    }
}
fn tls_client() -> ClientConnection {
    let mut roots = RootCertStore::empty();
    for cert in must(CertificateDer::pem_file_iter(fixture("root-ca.pem"))) {
        must(roots.add(must(cert)));
    }
    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    must(ClientConnection::new(
        Arc::new(config),
        must(ServerName::try_from("localhost")),
    ))
}

struct Peer {
    reader: Reader,
    writer: Arc<AsyncMutex<Writer>>,
}
struct Reader {
    wire: WireReader,
    ws: bool,
    remaining: usize,
    writer: Arc<AsyncMutex<Writer>>,
}
struct Writer {
    wire: WireWriter,
    ws: bool,
    mask_seq: u32,
}
impl Writer {
    async fn header(&mut self, opcode: u8, len: usize) -> io::Result<[u8; 4]> {
        self.mask_seq = self.mask_seq.wrapping_add(0x9e37_79b9);
        let mask = self.mask_seq.to_be_bytes();
        let mut header = vec![0x80 | opcode];
        if len < 126 {
            header.push(0x80 | len as u8);
        } else if len <= 65535 {
            header.push(0xfe);
            header.extend_from_slice(&(len as u16).to_be_bytes());
        } else {
            header.push(0xff);
            header.extend_from_slice(&(len as u64).to_be_bytes());
        }
        header.extend_from_slice(&mask);
        self.wire.write_all(&header).await?;
        Ok(mask)
    }
    async fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        if self.ws {
            let mask = self.header(2, bytes.len()).await?;
            let masked: Vec<_> = bytes
                .iter()
                .enumerate()
                .map(|(i, b)| b ^ mask[i % 4])
                .collect();
            self.wire.write_all(&masked).await?;
        } else {
            self.wire.write_all(bytes).await?;
        }
        self.wire.flush().await
    }
}
impl Reader {
    async fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if !self.ws {
            return self.wire.read(bytes).await;
        }
        while self.remaining == 0 {
            let mut h = [0; 2];
            self.wire.read_exact(&mut h).await?;
            if h[1] & 0x80 != 0 {
                return Err(io::Error::other("masked server frame"));
            }
            let len = match h[1] & 127 {
                126 => self.wire.read_u16().await? as usize,
                127 => usize::try_from(self.wire.read_u64().await?).map_err(io::Error::other)?,
                len => len as usize,
            };
            if len > M {
                return Err(io::Error::other("server frame larger than M"));
            }
            match h[0] & 15 {
                0 | 2 => self.remaining = len,
                9 | 10 => {
                    let mut payload = vec![0; len];
                    self.wire.read_exact(&mut payload).await?;
                    if h[0] & 15 == 9 {
                        let mut writer = self.writer.lock().await;
                        let mask = writer.header(10, len).await?;
                        for (i, byte) in payload.iter_mut().enumerate() {
                            *byte ^= mask[i % 4];
                        }
                        writer.wire.write_all(&payload).await?;
                        writer.wire.flush().await?;
                    }
                }
                8 => return Ok(0),
                _ => return Err(io::Error::other("non-binary relay frame")),
            }
        }
        let take = bytes.len().min(self.remaining);
        let n = self.wire.read(&mut bytes[..take]).await?;
        self.remaining -= n;
        Ok(n)
    }
    async fn exact(&mut self, bytes: &mut [u8]) -> io::Result<()> {
        let mut offset = 0;
        while offset < bytes.len() {
            let n = self.read(&mut bytes[offset..]).await?;
            if n == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            offset += n;
        }
        Ok(())
    }
}
impl Peer {
    async fn connect(port: u16, transport: Transport, viewer: bool) -> TestResult<Self> {
        // Q121: on loopback the test clients share the relay host's TCP memory;
        // in a deployment they sit on other hosts. Keep their buffers small.
        let socket = tokio::net::TcpSocket::new_v4()?;
        socket.set_recv_buffer_size(CLIENT_SOCKET_BUFFER)?;
        socket.set_send_buffer_size(CLIENT_SOCKET_BUFFER)?;
        let socket = socket
            .connect(std::net::SocketAddr::from((
                std::net::Ipv4Addr::LOCALHOST,
                port,
            )))
            .await?;
        socket.set_nodelay(true)?;
        let mut wire: WireBox = if matches!(transport, Transport::Wss) {
            Box::new(TlsWire {
                socket,
                tls: tls_client(),
                handshake_flushed: false,
            })
        } else {
            Box::new(socket)
        };
        let ws = !matches!(transport, Transport::Tcp);
        if ws {
            wire.write_all(b"GET /relay HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Protocol: binary\r\n\r\n").await?;
            wire.flush().await?;
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                head.push(wire.read_u8().await?);
                if head.len() > 16384 {
                    return Err("oversized upgrade response".into());
                }
            }
            let text = String::from_utf8(head)?;
            if !text.starts_with("HTTP/1.1 101 ")
                || !text
                    .to_ascii_lowercase()
                    .contains("sec-websocket-protocol: binary\r\n")
            {
                return Err(format!("upgrade rejected: {text}").into());
            }
        }
        let mut peer = Self::from_wire(wire, ws);
        if viewer {
            let mut greeting = [0; GREETING.len()];
            peer.reader.exact(&mut greeting).await?;
            if greeting != GREETING {
                return Err("invalid Viewer greeting".into());
            }
        }
        Ok(peer)
    }
    fn from_wire(wire: WireBox, ws: bool) -> Self {
        let (read, write) = tokio::io::split(wire);
        let writer = Arc::new(AsyncMutex::new(Writer {
            wire: write,
            ws,
            mask_seq: 0x1357_9bdf,
        }));
        Self {
            reader: Reader {
                wire: read,
                ws,
                remaining: 0,
                writer: writer.clone(),
            },
            writer,
        }
    }
    async fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer.lock().await.send(bytes).await
    }
}

#[derive(Clone, Copy)]
struct Sample {
    wall: f64,
    cpu: f64,
    rss: u64,
    handles: u64,
}
impl Sample {
    fn json(self) -> Value {
        json!({"wall_seconds":self.wall,"cpu_seconds":self.cpu,"rss_bytes":self.rss,"handles_or_fds":self.handles})
    }
}
async fn sample(pid: u32, epoch: Instant) -> Sample {
    must(tokio::task::spawn_blocking(move || process_sample(pid, epoch)).await)
}
#[cfg(windows)]
fn process_sample(pid: u32, epoch: Instant) -> Sample {
    let output = must(Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", &format!("$p=Get-Process -Id {pid}; [Console]::WriteLine(('{{0}},{{1}},{{2}}' -f $p.WorkingSet64,$p.TotalProcessorTime.Ticks,$p.HandleCount))")]).output());
    assert!(output.status.success(), "Get-Process: {:?}", output);
    let text = must(String::from_utf8(output.stdout));
    let fields: Vec<u64> = text.trim().split(',').map(|f| must(f.parse())).collect();
    Sample {
        wall: epoch.elapsed().as_secs_f64(),
        cpu: fields[1] as f64 / 10_000_000.0,
        rss: fields[0],
        handles: fields[2],
    }
}
#[cfg(target_os = "linux")]
fn process_sample(pid: u32, epoch: Instant) -> Sample {
    let root = PathBuf::from(format!("/proc/{pid}"));
    let status = must(std::fs::read_to_string(root.join("status")));
    let rss = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .map(|text| {
            must(
                text.split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .parse::<u64>(),
            ) * 1024
        })
        .unwrap_or_else(|| panic!("VmRSS missing"));
    let stat = must(std::fs::read_to_string(root.join("stat")));
    let fields: Vec<_> = stat
        .rsplit_once(')')
        .unwrap_or_else(|| panic!("process stat format"))
        .1
        .split_whitespace()
        .collect();
    // Linux x64 measurement assumes CLK_TCK = 100 (see the how-to).
    let cpu = (must(fields[11].parse::<u64>()) + must(fields[12].parse::<u64>())) as f64 / 100.0;
    Sample {
        wall: epoch.elapsed().as_secs_f64(),
        cpu,
        rss,
        handles: must(std::fs::read_dir(root.join("fd"))).count() as u64,
    }
}

#[derive(Default)]
struct Latency {
    buckets: Vec<u64>,
    count: u64,
    max_us: u64,
}
impl Latency {
    fn add(&mut self, us: u64) {
        if self.buckets.is_empty() {
            self.buckets.resize(100_002, 0);
        }
        self.buckets[(us.div_ceil(10) as usize).min(100_001)] += 1;
        self.count += 1;
        self.max_us = self.max_us.max(us);
    }
    fn percentile(&self, percent: u64) -> u64 {
        let target = (self.count * percent).div_ceil(100);
        let mut cumulative = 0;
        for (i, count) in self.buckets.iter().enumerate() {
            cumulative += count;
            if cumulative >= target {
                return if i == 100_001 {
                    self.max_us
                } else {
                    i as u64 * 10
                };
            }
        }
        0
    }
    fn json(&self) -> Value {
        json!({"samples":self.count,"p50_us":self.percentile(50),"p99_us":self.percentile(99),"max_us":self.max_us,"histogram_bucket_us":10})
    }
}
fn chunk(bytes: &mut [u8; CHUNK], session: usize, direction: u64, seq: u64, timestamp: u64) {
    bytes[..8].copy_from_slice(&(session as u64).to_le_bytes());
    bytes[8..16].copy_from_slice(&direction.to_le_bytes());
    bytes[16..24].copy_from_slice(&seq.to_le_bytes());
    bytes[24..32].copy_from_slice(&timestamp.to_le_bytes());
    let seed = (session as u64).wrapping_mul(0x9e37_79b9)
        ^ direction.wrapping_mul(0x85eb_ca6b)
        ^ seq.wrapping_mul(0xc2b2_ae35);
    for (i, byte) in bytes[32..].iter_mut().enumerate() {
        *byte = seed.rotate_left((i % 64) as u32) as u8 ^ i as u8;
    }
}
fn timestamp(epoch: Instant) -> u64 {
    epoch.elapsed().as_micros() as u64
}
struct Flow {
    session: usize,
    direction: u64,
    slow: bool,
    sent: AtomicU64,
    received: AtomicU64,
    errors: AtomicU64,
    done: AtomicBool,
    blocked: AtomicBool,
}
struct Traffic {
    flows: Vec<Arc<Flow>>,
    tasks: Vec<JoinHandle<io::Result<()>>>,
    // Keep both endpoints open until every direction has drained.
    retained_writers: Vec<Arc<AsyncMutex<Writer>>>,
    stop: Arc<AtomicBool>,
    resume: watch::Sender<bool>,
    latency: Arc<Mutex<Latency>>,
    measure: Arc<AtomicU64>,
    epoch: Instant,
}
impl Traffic {
    fn start(pairs: Vec<(Peer, Peer)>, slow_count: usize, epoch: Instant) -> Self {
        let (resume, _) = watch::channel(false);
        let mut traffic = Self {
            flows: Vec::new(),
            tasks: Vec::new(),
            retained_writers: Vec::new(),
            stop: Arc::new(AtomicBool::new(false)),
            resume,
            latency: Arc::new(Mutex::new(Latency::default())),
            measure: Arc::new(AtomicU64::new(u64::MAX)),
            epoch,
        };
        for (session, (viewer, server)) in pairs.into_iter().enumerate() {
            traffic
                .retained_writers
                .extend([viewer.writer.clone(), server.writer.clone()]);
            let slow = session < slow_count;
            let forward = Arc::new(Flow {
                session,
                direction: 0,
                slow,
                sent: AtomicU64::new(0),
                received: AtomicU64::new(0),
                errors: AtomicU64::new(0),
                done: AtomicBool::new(false),
                blocked: AtomicBool::new(false),
            });
            let reverse = Arc::new(Flow {
                session,
                direction: 1,
                slow,
                sent: AtomicU64::new(0),
                received: AtomicU64::new(0),
                errors: AtomicU64::new(0),
                done: AtomicBool::new(false),
                blocked: AtomicBool::new(false),
            });
            traffic.writer(viewer.writer, forward.clone());
            traffic.writer(server.writer, reverse.clone());
            traffic.reader(server.reader, forward.clone());
            traffic.reader(viewer.reader, reverse.clone());
            traffic.flows.extend([forward, reverse]);
        }
        traffic
    }
    fn writer(&mut self, writer: Arc<AsyncMutex<Writer>>, flow: Arc<Flow>) {
        let stop = self.stop.clone();
        let epoch = self.epoch;
        self.tasks.push(tokio::spawn(async move {
            let mut seq = 0;
            let start = tokio::time::Instant::now();
            let mut bytes = [0; CHUNK];
            let result = async {
                while !stop.load(Ordering::Acquire) {
                    let mut writer = writer.lock().await;
                    let chunks = if flow.slow && writer.ws { M / CHUNK } else { 1 };
                    let mask = if writer.ws {
                        Some(writer.header(2, chunks * CHUNK).await?)
                    } else {
                        None
                    };
                    for _ in 0..chunks {
                        chunk(
                            &mut bytes,
                            flow.session,
                            flow.direction,
                            seq,
                            timestamp(epoch),
                        );
                        if let Some(mask) = mask {
                            for (i, byte) in bytes.iter_mut().enumerate() {
                                *byte ^= mask[i % 4];
                            }
                        }
                        let write = async {
                            writer.wire.write_all(&bytes).await?;
                            writer.wire.flush().await
                        };
                        tokio::pin!(write);
                        tokio::select! {
                            result = &mut write => result?,
                            () = tokio::time::sleep(Duration::from_millis(250)) => {
                                flow.blocked.store(true, Ordering::Release);
                                write.await?;
                            }
                        }
                        flow.sent.fetch_add(CHUNK as u64, Ordering::Release);
                        seq += 1;
                    }
                    drop(writer);
                    if !flow.slow {
                        tokio::time::sleep_until(
                            start
                                + Duration::from_secs_f64(seq as f64 * CHUNK as f64 / RATE as f64),
                        )
                        .await;
                    }
                }
                Ok(())
            }
            .await;
            flow.done.store(true, Ordering::Release);
            result
        }));
    }
    fn reader(&mut self, mut reader: Reader, flow: Arc<Flow>) {
        let mut resume = self.resume.subscribe();
        let latency = self.latency.clone();
        let measure = self.measure.clone();
        let epoch = self.epoch;
        self.tasks.push(tokio::spawn(async move {
            if flow.slow {
                while !*resume.borrow_and_update() {
                    resume.changed().await.map_err(io::Error::other)?;
                }
            }
            let mut bytes = [0; CHUNK];
            let mut expected = [0; CHUNK];
            let mut seq = 0;
            loop {
                if flow.done.load(Ordering::Acquire)
                    && flow.received.load(Ordering::Acquire) == flow.sent.load(Ordering::Acquire)
                {
                    return Ok(());
                }
                // Do not cancel a partially read WebSocket frame header.
                let read = tokio::time::timeout(DEADLINE, reader.exact(&mut bytes)).await;
                match read {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => return Err(error),
                    Err(_)
                        if flow.done.load(Ordering::Acquire)
                            && flow.received.load(Ordering::Acquire)
                                == flow.sent.load(Ordering::Acquire) =>
                    {
                        return Ok(());
                    }
                    Err(_) => return Err(io::Error::other("drain deadline")),
                }
                let sent_at = u64::from_le_bytes(must(bytes[24..32].try_into()));
                chunk(&mut expected, flow.session, flow.direction, seq, sent_at);
                if bytes != expected {
                    flow.errors.fetch_add(1, Ordering::Relaxed);
                }
                if sent_at >= measure.load(Ordering::Acquire) {
                    latency
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .add(timestamp(epoch).saturating_sub(sent_at));
                }
                flow.received.fetch_add(CHUNK as u64, Ordering::Release);
                seq += 1;
            }
        }));
    }
    fn begin(&self) {
        self.measure.store(timestamp(self.epoch), Ordering::Release);
    }
    fn counts(&self) -> Vec<(u64, u64)> {
        self.flows
            .iter()
            .map(|f| {
                (
                    f.sent.load(Ordering::Acquire),
                    f.received.load(Ordering::Acquire),
                )
            })
            .collect()
    }
    async fn finish(mut self) -> Value {
        self.stop.store(true, Ordering::Release);
        self.resume.send_replace(true);
        let end = tokio::time::Instant::now() + DEADLINE;
        let drained = loop {
            if self.flows.iter().all(|f| {
                f.done.load(Ordering::Acquire)
                    && f.sent.load(Ordering::Acquire) == f.received.load(Ordering::Acquire)
            }) {
                break true;
            }
            if tokio::time::Instant::now() >= end {
                break false;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        };
        let mut task_errors = Vec::new();
        if !drained {
            task_errors.push("traffic drain deadline".to_owned());
        }
        for task in &mut self.tasks {
            if !task.is_finished() {
                task.abort();
            }
            match task.await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => task_errors.push(error.to_string()),
                Err(error) if error.is_cancelled() && drained => {}
                Err(error) => task_errors.push(error.to_string()),
            }
        }
        let flows: Vec<_> = self.flows.iter().map(|f| json!({"session":f.session,"direction":f.direction,"slow":f.slow,"sent":f.sent.load(Ordering::Acquire),"received":f.received.load(Ordering::Acquire),"errors":f.errors.load(Ordering::Acquire),"write_blocked":f.blocked.load(Ordering::Acquire)})).collect();
        let errors = self
            .flows
            .iter()
            .map(|f| {
                f.errors.load(Ordering::Acquire)
                    + u64::from(
                        f.sent.load(Ordering::Acquire) != f.received.load(Ordering::Acquire),
                    )
            })
            .sum::<u64>()
            + task_errors.len() as u64;
        json!({"data_errors":errors,"task_errors":task_errors,"flows":flows,"latency":self.latency.lock().unwrap_or_else(PoisonError::into_inner).json()})
    }
}
impl Drop for Traffic {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

async fn pairs(relay: &Relay, transport: Transport, count: usize) -> Vec<(Peer, Peer)> {
    let mut pairs = Vec::new();
    for id in 0..count {
        pairs.push(
            must(tokio::time::timeout(DEADLINE, relay.pair(transport, id + 1)).await)
                .unwrap_or_else(|e| panic!("Mode II pair: {e}")),
        );
    }
    pairs
}
async fn direct_pairs() -> Vec<(Peer, Peer)> {
    let listener = must(TcpListener::bind("127.0.0.1:0").await);
    let address = must(listener.local_addr());
    let mut pairs = Vec::new();
    for _ in 0..100 {
        let left = must(TcpStream::connect(address).await);
        let (right, _) = must(listener.accept().await);
        must(left.set_nodelay(true));
        must(right.set_nodelay(true));
        pairs.push((
            Peer::from_wire(Box::new(left), false),
            Peer::from_wire(Box::new(right), false),
        ));
    }
    pairs
}
fn criterion(name: &str, measured: Value, limit: Value, pass: bool) -> Value {
    json!({"name":name,"measured":measured,"limit":limit,"pass":pass})
}
fn passed(criteria: &[Value]) -> bool {
    criteria.iter().all(|c| c["pass"] == true)
}
fn metadata(test: &str, transport: Transport) -> Value {
    json!({"test":test,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"transport":transport.name(),"server_transport":"tcp_server","config":{"L":1000,"Lw":200,"B":B,"M":M,"ws_ping_secs":300,"ws_pong_timeout_secs":300},"chunk_bytes":CHUNK,"bytes_per_second_per_direction":RATE,"sessions":100,"available_parallelism":must(std::thread::available_parallelism()).get()})
}

fn window_counts(before: &[(u64, u64)], after: &[(u64, u64)], seconds: u64) -> Vec<Value> {
    let minimum = (seconds * RATE / CHUNK as u64).saturating_sub(1) * CHUNK as u64;
    before.iter().zip(after).enumerate().map(|(i,(before,after))| {
        let sent = after.0-before.0;
        json!({"session":i/2,"direction":i%2,"sent":sent,"received":after.1-before.1,"minimum_sent":minimum,"pass":sent>=minimum})
    }).collect()
}

/// spec: operations.md:OPS-018
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "long measurement; docs/how-to/measure-performance.md"]
async fn measures_throughput_latency_cpu_and_rss() {
    let _ports = PORTS.lock().await;
    let seconds: u64 = std::env::var("VNC_RELAY_PERF_SECS").map_or(600, |s| must(s.parse()));
    assert!(seconds > 0);
    let epoch = Instant::now();
    let direct = Traffic::start(direct_pairs().await, 0, epoch);
    tokio::time::sleep(Duration::from_secs(10)).await;
    direct.begin();
    let direct_before = direct.counts();
    tokio::time::sleep(Duration::from_secs(seconds)).await;
    let direct_window = window_counts(&direct_before, &direct.counts(), seconds);
    let baseline = direct.finish().await;
    let mut all_pass = true;
    for transport in Transport::ALL {
        let mut relay = Relay::start("ops018", transport).await;
        let traffic = Traffic::start(pairs(&relay, transport, 100).await, 0, epoch);
        relay.wait_logs(|e| log_state(e)["active"] == 100).await;
        tokio::time::sleep(Duration::from_secs(10)).await;
        let first = sample(relay.child.id(), epoch).await;
        traffic.begin();
        let before = traffic.counts();
        let mut samples = vec![first];
        let start = tokio::time::Instant::now();
        let end = start + Duration::from_secs(seconds);
        let mut next = start + Duration::from_secs(5);
        while next < end {
            tokio::time::sleep_until(next).await;
            samples.push(sample(relay.child.id(), epoch).await);
            next += Duration::from_secs(5);
        }
        tokio::time::sleep_until(end).await;
        // End pacing at the same duration as the direct baseline, not after
        // the external process-metrics collector returns.
        traffic.stop.store(true, Ordering::Release);
        samples.push(sample(relay.child.id(), epoch).await);
        let last = *samples.last().unwrap_or_else(|| panic!("metrics sample"));
        let window = window_counts(&before, &traffic.counts(), seconds);
        let traffic_result = traffic.finish().await;
        let max_rss = samples.iter().map(|s| s.rss).max().unwrap_or_default();
        let cpu = (last.cpu - first.cpu) / (last.wall - first.wall);
        let direct_p99 = baseline["latency"]["p99_us"].as_f64().unwrap_or_default();
        let relay_p99 = traffic_result["latency"]["p99_us"]
            .as_f64()
            .unwrap_or_default();
        let added = relay_p99 - direct_p99;
        let errors = traffic_result["data_errors"].as_u64().unwrap_or(u64::MAX)
            + baseline["data_errors"].as_u64().unwrap_or(u64::MAX);
        let criteria = vec![
            criterion(
                "judgeable_duration",
                json!(seconds),
                json!({"minimum_seconds":600}),
                seconds >= 600,
            ),
            criterion(
                "paced_directions_including_direct_baseline",
                json!(
                    window
                        .iter()
                        .chain(direct_window.iter())
                        .filter(|v| v["pass"] == true)
                        .count()
                ),
                json!(400),
                window
                    .iter()
                    .chain(direct_window.iter())
                    .all(|v| v["pass"] == true),
            ),
            criterion(
                "data_errors_including_direct_baseline",
                json!(errors),
                json!(0),
                errors == 0,
            ),
            criterion(
                "added_p99_us",
                json!(added),
                json!(10_000),
                added <= 10_000.0,
            ),
            criterion("cpu_average_cores", json!(cpu), json!(1), cpu <= 1.0),
            criterion(
                "rss_max_bytes",
                json!(max_rss),
                json!(256 * 1024 * 1024),
                max_rss <= 256 * 1024 * 1024,
            ),
            criterion(
                "rss_growth_ratio",
                json!(max_rss as f64 / first.rss as f64),
                json!(1.10),
                max_rss as f64 <= first.rss as f64 * 1.10,
            ),
        ];
        let measurements_pass = criteria
            .iter()
            .filter(|c| c["name"] != "judgeable_duration")
            .all(|c| c["pass"] == true);
        let mut result = metadata("OPS-018", transport);
        result["window_flows"] = json!(window);
        result["direct_window_flows"] = json!(direct_window);
        result["duration_seconds"] = json!(seconds);
        result["judgeable"] = json!(seconds >= 600);
        result["direct_baseline"] = baseline.clone();
        result["traffic"] = traffic_result;
        result["p99_direct_us"] = json!(direct_p99);
        result["p99_relay_us"] = json!(relay_p99);
        result["added_p99_us"] = json!(added);
        result["samples"] = json!(samples.iter().map(|s| s.json()).collect::<Vec<_>>());
        result["pass"] = json!(passed(&criteria));
        result["measurements_pass"] = json!(measurements_pass);
        result["criteria"] = json!(criteria);
        save(&format!("ops018-{}", transport.name()), &result);
        all_pass &= measurements_pass;
    }
    assert!(
        all_pass,
        "OPS-018 criteria failed; inspect JSON in {} (short runs are not judgeable)",
        output_dir().display()
    );
}

async fn lifecycle(relay: &Relay, transport: Transport, id: usize) -> TestResult {
    let (mut viewer, mut server) = relay.pair(transport, id).await?;
    let mut forward = [0; CHUNK];
    let mut reverse = [0; CHUNK];
    chunk(&mut forward, id, 0, 0, 0);
    chunk(&mut reverse, id, 1, 0, 0);
    viewer.send(&forward[..1024]).await?;
    server.send(&reverse[..1024]).await?;
    let mut received = [0; 1024];
    server.reader.exact(&mut received).await?;
    if received != forward[..1024] {
        return Err("lifecycle forward mismatch".into());
    }
    viewer.reader.exact(&mut received).await?;
    if received != reverse[..1024] {
        return Err("lifecycle reverse mismatch".into());
    }
    // Drop both endpoints only after both directions have been verified.
    Ok(())
}

/// spec: operations.md:OPS-017
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "long measurement; docs/how-to/measure-performance.md"]
async fn bounds_slow_receivers_and_lifecycle_residue() {
    let _ports = PORTS.lock().await;
    let epoch = Instant::now();
    let mut all_pass = true;
    for transport in Transport::ALL {
        let mut relay = Relay::start("ops017", transport).await;
        tokio::time::sleep(Duration::from_secs(5)).await;
        let idle = sample(relay.child.id(), epoch).await;
        let traffic = Traffic::start(pairs(&relay, transport, 100).await, 50, epoch);
        relay.wait_logs(|e| log_state(e)["active"] == 100).await;
        // Start the 60-second hold only after every slow direction has blocked.
        let blocked_deadline = Instant::now() + DEADLINE;
        while !traffic
            .flows
            .iter()
            .filter(|f| f.slow)
            .all(|f| f.blocked.load(Ordering::Acquire))
            && Instant::now() < blocked_deadline
        {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let blocked = traffic
            .flows
            .iter()
            .filter(|f| f.slow && f.blocked.load(Ordering::Acquire))
            .count();
        let before = traffic.counts();
        traffic.begin();
        let mut samples = vec![sample(relay.child.id(), epoch).await];
        let start = tokio::time::Instant::now();
        for index in 1..=12 {
            tokio::time::sleep_until(start + Duration::from_secs(index * 5)).await;
            samples.push(sample(relay.child.id(), epoch).await);
        }
        let after = traffic.counts();
        // Q120: the normal sessions keep 1 Mbit/s each way. Each direction must
        // send and deliver the paced amount of the window; one chunk at each
        // window edge is allowed for sending, one more for bytes in flight.
        let sent_minimum = (60 * RATE / CHUNK as u64).saturating_sub(1) * CHUNK as u64;
        let received_minimum = sent_minimum.saturating_sub(CHUNK as u64);
        let normal_window: Vec<_> = traffic.flows.iter().enumerate().filter(|(_,f)| !f.slow).map(|(i,f)| {
            let sent=after[i].0-before[i].0;
            let received=after[i].1-before[i].1;
            json!({"session":f.session,"direction":f.direction,"sent":sent,"received":received,"minimum_sent":sent_minimum,"minimum_received":received_minimum,"pass":sent>=sent_minimum && received>=received_minimum})
        }).collect();
        let flowing = normal_window.iter().all(|v| v["pass"] == true);
        let traffic_result = traffic.finish().await;
        relay.quiescent(100).await;
        let max_rss = samples.iter().map(|s| s.rss).max().unwrap_or_default();
        let overhead = max_rss.saturating_sub(idle.rss);
        let mut criteria = vec![
            criterion(
                "slow_directions_write_blocked",
                json!(blocked),
                json!(100),
                blocked == 100,
            ),
            criterion(
                "rss_overhead_bytes",
                json!(overhead),
                json!(200 * (B + 65536)),
                overhead <= 200 * (B as u64 + 65536),
            ),
            criterion(
                "normal_directions_flowing",
                json!(normal_window.iter().filter(|v| v["pass"] == true).count()),
                json!(100),
                flowing,
            ),
            criterion(
                "all_stream_errors_after_resume_and_drain",
                traffic_result["data_errors"].clone(),
                json!(0),
                traffic_result["data_errors"] == 0,
            ),
        ];
        let mut rounds = Vec::new();
        let mut lifecycle_errors = Vec::new();
        for round in 1..=ROUNDS {
            for batch in 0..20 {
                let mut futures = Vec::new();
                for offset in 0..50 {
                    futures.push(lifecycle(
                        &relay,
                        transport,
                        10_000 + round * 1000 + batch * 50 + offset,
                    ));
                }
                // JoinSet requires owned relay state; scoped futures keep the process owner here.
                let mut pending: Vec<_> = futures.into_iter().map(Box::pin).collect();
                must(
                    tokio::time::timeout(
                        DEADLINE,
                        std::future::poll_fn(|cx| {
                            let mut index = 0;
                            while index < pending.len() {
                                match pending[index].as_mut().poll(cx) {
                                    Poll::Pending => index += 1,
                                    Poll::Ready(result) => {
                                        if let Err(e) = result {
                                            lifecycle_errors.push(e.to_string());
                                        }
                                        drop(pending.swap_remove(index));
                                    }
                                }
                            }
                            if pending.is_empty() {
                                Poll::Ready(())
                            } else {
                                Poll::Pending
                            }
                        }),
                    )
                    .await,
                );
            }
            let state = relay.quiescent(100 + round * 1000).await;
            tokio::time::sleep(Duration::from_secs(5)).await;
            let metrics = sample(relay.child.id(), epoch).await;
            // Q122: round 1 warms reusable OS resources; later rounds must equal it.
            let reference = rounds
                .first()
                .and_then(|first: &Value| first["metrics"]["handles_or_fds"].as_u64())
                .unwrap_or(metrics.handles);
            if round > 1 {
                criteria.push(criterion(
                    &format!("round_{round}_handles_or_fds"),
                    json!(metrics.handles),
                    json!(reference),
                    metrics.handles == reference,
                ));
            }
            rounds.push(json!({"round":round,"metrics":metrics.json(),"log_state":state}));
        }
        // Q123: the allocator grows in steps before it levels off; judge the tail.
        let rss_of = |round: usize| {
            rounds[round - 1]["metrics"]["rss_bytes"]
                .as_u64()
                .unwrap_or_default()
        };
        let growth = rss_of(ROUNDS) as i64 - rss_of(RSS_FROM_ROUND) as i64;
        criteria.push(criterion(
            &format!("round_{ROUNDS}_minus_round_{RSS_FROM_ROUND}_rss_bytes"),
            json!(growth),
            json!(2 * 1024 * 1024),
            growth <= 2 * 1024 * 1024,
        ));
        criteria.push(criterion(
            "lifecycle_errors",
            json!(lifecycle_errors.len()),
            json!(0),
            lifecycle_errors.is_empty(),
        ));
        let mut result = metadata("OPS-017", transport);
        result["duration_seconds"] = json!(60);
        result["idle_baseline"] = idle.json();
        result["samples"] = json!(samples.iter().map(|s| s.json()).collect::<Vec<_>>());
        result["normal_window"] = json!(normal_window);
        result["traffic"] = traffic_result;
        result["rounds"] = json!(rounds);
        result["lifecycle_errors"] = json!(lifecycle_errors);
        result["pass"] = json!(passed(&criteria));
        result["criteria"] = json!(criteria);
        save(&format!("ops017-{}", transport.name()), &result);
        all_pass &= result["pass"] == true;
    }
    assert!(
        all_pass,
        "OPS-017 criteria failed; inspect JSON in {}",
        output_dir().display()
    );
}
