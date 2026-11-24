//! Serial outside I/O. No tray-icon or menu object crosses this thread.

use crate::copy::Text;
use crate::registry;
use crate::rules::{self, Reason, State};
use std::fs;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use vnc_relay_adapters::windows_service::SERVICE_NAME;
use vnc_relay_application::settings::LogOutput;
use windows_service::service::{ServiceAccess, ServiceState};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

const REFRESH: Duration = Duration::from_secs(2);

#[derive(Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub state: State,
    pub config_path: Option<PathBuf>,
    pub config_error: Option<Reason>,
    pub dashboard: Option<String>,
    pub log: Option<PathBuf>,
    pub sign_in: bool,
}
impl Snapshot {
    pub fn initial() -> Self {
        Self {
            state: State::Unknown,
            config_path: None,
            config_error: None,
            dashboard: None,
            log: None,
            sign_in: false,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Action {
    Start,
    Stop,
    Dashboard,
    Log,
    Config,
    SignIn,
}
impl Action {
    pub fn text(self) -> Text {
        match self {
            Self::Start => Text::Start,
            Self::Stop => Text::Stop,
            Self::Dashboard => Text::Dashboard,
            Self::Log => Text::Log,
            Self::Config => Text::Config,
            Self::SignIn => Text::SignIn,
        }
    }
}

pub enum Request {
    Refresh,
    Action(Action, u64),
}
pub enum Reply {
    Snapshot(Arc<Snapshot>),
    Failure(Action, u64, Reason),
}

pub fn spawn() -> io::Result<(Sender<Request>, Receiver<Reply>)> {
    let executable = std::env::current_exe()?;
    let (requests, inbox) = mpsc::channel();
    let (outbox, replies) = mpsc::channel();
    thread::Builder::new()
        .name("tray-worker".into())
        .spawn(move || {
            let mut snapshot = Arc::new(Snapshot::initial());
            let mut next_refresh = Instant::now();
            loop {
                let request = match inbox
                    .recv_timeout(next_refresh.saturating_duration_since(Instant::now()))
                {
                    Ok(request) => request,
                    Err(mpsc::RecvTimeoutError::Timeout) => Request::Refresh,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                };
                if let Request::Action(action, generation) = request
                    && let Err(reason) = perform(action, &snapshot, &executable)
                    && outbox
                        .send(Reply::Failure(action, generation, reason))
                        .is_err()
                {
                    break;
                }
                let started = Instant::now();
                snapshot = Arc::new(refresh(&executable));
                next_refresh = started + REFRESH;
                if outbox.send(Reply::Snapshot(Arc::clone(&snapshot))).is_err() {
                    break;
                }
            }
        })?;
    Ok((requests, replies))
}

fn manager() -> windows_service::Result<ServiceManager> {
    ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
}

fn refresh(executable: &Path) -> Snapshot {
    let mut snapshot = Snapshot::initial();
    snapshot.sign_in = registry::sign_in(executable);
    let query = (|| {
        let manager = manager()?;
        let service = manager.open_service(
            SERVICE_NAME,
            ServiceAccess::QUERY_STATUS | ServiceAccess::QUERY_CONFIG,
        )?;
        Ok::<_, windows_service::Error>((service.query_status()?, service.query_config()?))
    })();
    let (status, config) = match query {
        Ok(query) => query,
        Err(error) => {
            if service_code(&error) == Some(1060) {
                snapshot.state = State::Missing;
            }
            return snapshot;
        }
    };
    snapshot.config_path = config.executable_path.to_str().and_then(rules::config_path);
    let parsed = snapshot
        .config_path
        .as_deref()
        .ok_or(Reason::NoConfig)
        .and_then(read_config);
    let ready = match parsed {
        Ok(config) => {
            snapshot.dashboard = Some(config.dashboard);
            snapshot.log = config.log;
            status.current_state == ServiceState::Running && readyz(config.status)
        }
        Err(reason) => {
            snapshot.config_error = Some(reason);
            false
        }
    };
    snapshot.state = State::from_service(status.current_state, status.exit_code, ready);
    snapshot
}

struct Config {
    dashboard: String,
    status: SocketAddr,
    log: Option<PathBuf>,
}

fn read_config(path: &Path) -> Result<Config, Reason> {
    let bytes = fs::read(path).map_err(|error| match error.kind() {
        io::ErrorKind::PermissionDenied => Reason::Denied,
        io::ErrorKind::NotFound => Reason::NotFound,
        _ => Reason::from_io(&error),
    })?;
    let text = std::str::from_utf8(&bytes).map_err(|_| Reason::Invalid)?;
    let settings =
        vnc_relay_adapters::config_file::parse(text, path.parent().unwrap_or(Path::new("")))
            .map_err(|_| Reason::Invalid)?;
    Ok(Config {
        dashboard: rules::dashboard_url(
            settings.management.listen,
            settings.management.tls.is_some(),
        ),
        status: settings.status.listen,
        log: match settings.log.output {
            LogOutput::File(path) => Some(path),
            LogOutput::Stdout => None,
        },
    })
}

fn readyz(address: SocketAddr) -> bool {
    probe_readyz(address).unwrap_or(false)
}

fn remaining(deadline: Instant) -> io::Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "readyz deadline"))
}

fn probe_readyz(address: SocketAddr) -> io::Result<bool> {
    let deadline = Instant::now() + Duration::from_secs(1);
    let address = rules::connect_address(address);
    let mut stream = TcpStream::connect_timeout(&address, remaining(deadline)?)?;
    let request = format!("GET /readyz HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
    let mut pending = request.as_bytes();
    while !pending.is_empty() {
        stream.set_write_timeout(Some(remaining(deadline)?))?;
        let count = stream.write(pending)?;
        if count == 0 {
            return Ok(false);
        }
        pending = &pending[count..];
    }
    let mut line = [0_u8; 128];
    for index in 0..line.len() {
        stream.set_read_timeout(Some(remaining(deadline)?))?;
        if stream.read(&mut line[index..index + 1])? == 0 {
            return Ok(false);
        }
        if line[index] == b'\n' {
            let mut fields = line[..=index]
                .split(|byte| byte.is_ascii_whitespace())
                .filter(|field| !field.is_empty());
            return Ok(matches!(fields.next(), Some(b"HTTP/1.0" | b"HTTP/1.1"))
                && fields.next() == Some(b"200"));
        }
    }
    Ok(false)
}

fn perform(action: Action, snapshot: &Snapshot, executable: &Path) -> Result<(), Reason> {
    match action {
        Action::Start | Action::Stop => {
            let result = (|| {
                let manager = manager()?;
                let access = if matches!(action, Action::Start) {
                    ServiceAccess::START
                } else {
                    ServiceAccess::STOP
                };
                let service = manager.open_service(SERVICE_NAME, access)?;
                if matches!(action, Action::Start) {
                    service.start::<&str>(&[])
                } else {
                    service.stop().map(|_| ())
                }
            })();
            result.map_err(|error| {
                if service_code(&error) == Some(5) {
                    Reason::Denied
                } else {
                    Reason::Os(service_code(&error).unwrap_or(0))
                }
            })
        }
        Action::Dashboard => match &snapshot.dashboard {
            Some(url) => explorer(url),
            None => Err(Reason::NoConfig),
        },
        Action::Log => match &snapshot.log {
            Some(path) => open_location(path, true),
            None => Err(Reason::NoConfig),
        },
        Action::Config => match &snapshot.config_path {
            Some(path) => open_location(path, false),
            None => Err(Reason::NoConfig),
        },
        Action::SignIn => registry::toggle_sign_in(executable),
    }
}

fn service_code(error: &windows_service::Error) -> Option<i32> {
    match error {
        windows_service::Error::Winapi(error) => error.raw_os_error(),
        _ => None,
    }
}

fn explorer(argument: impl AsRef<std::ffi::OsStr>) -> Result<(), Reason> {
    Command::new("explorer.exe")
        .arg(argument)
        .spawn()
        .map(|_| ())
        .map_err(|error| Reason::Os(error.raw_os_error().unwrap_or(0)))
}

fn open_location(path: &Path, allow_missing_file: bool) -> Result<(), Reason> {
    if allow_missing_file {
        match fs::metadata(path) {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let directory = path.parent().unwrap_or(path);
                fs::metadata(directory)
                    .map_err(|error| Reason::Os(error.raw_os_error().unwrap_or(0)))?;
                return explorer(directory);
            }
            Err(error) => return Err(Reason::Os(error.raw_os_error().unwrap_or(0))),
        }
    } else if let Some(directory) = path.parent() {
        fs::metadata(directory).map_err(|error| Reason::Os(error.raw_os_error().unwrap_or(0)))?;
    }
    let mut argument = std::ffi::OsString::from("/select,");
    argument.push(path);
    explorer(argument)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// spec: windows-tray.md:TRAY-003 partial
    #[test]
    fn readyz_accepts_only_http_200() -> Result<(), Box<dyn std::error::Error>> {
        for (response, expected) in [
            ("HTTP/1.1 200 OK\r\n", true),
            ("HTTP/1.1 503 Unavailable\r\n", false),
            ("invalid 200\r\n", false),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0")?;
            let address = listener.local_addr()?;
            let server = thread::spawn(move || -> io::Result<()> {
                let (mut stream, _) = listener.accept()?;
                stream.set_read_timeout(Some(Duration::from_secs(2)))?;
                let mut buffer = [0; 256];
                let _ = stream.read(&mut buffer)?;
                stream.write_all(response.as_bytes())
            });
            assert_eq!(readyz(address), expected);
            server.join().map_err(|_| "server panicked")??;
        }
        Ok(())
    }
}
