//! Test-side HTTP/TCP forwarding proxy. Ordinary upstream bytes are preserved.
//! Traffic distinguishes injected failures from actual management responses.

use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::{LIMIT, connect, must};

#[derive(Clone)]
pub enum Fault {
    Delay(Duration),
    DelayBefore(Duration),
    Hold(Arc<AtomicBool>),
    DropResponse,
    Close,
    Error500(&'static str),
}

#[derive(Clone)]
pub struct Traffic {
    pub method: String,
    pub path: String,
    pub headers: String,
    pub body: String,
    pub at: Instant,
    pub completed: Option<Instant>,
    pub forwarded: bool,
    pub injected: bool,
    pub status: Option<u16>,
    pub stage: &'static str,
    pub error: Option<(io::ErrorKind, Option<i32>)>,
    pub accepted_at: Instant,
    pub upstream_connected_at: Option<Instant>,
    pub upstream_written_at: Option<Instant>,
    pub upstream_response_at: Option<Instant>,
}

struct Rule {
    method: String,
    path: String,
    fault: Fault,
    once: bool,
}

#[derive(Default)]
struct Shared {
    rules: Mutex<Vec<Rule>>,
    traffic: Mutex<Vec<Traffic>>,
    stop: AtomicBool,
    accepted: AtomicUsize,
    request_errors: Mutex<Vec<(io::ErrorKind, Option<i32>)>>,
}

pub struct Proxy {
    pub port: u16,
    shared: Arc<Shared>,
    task: Option<JoinHandle<()>>,
}

impl Proxy {
    pub fn start(upstream: u16) -> Self {
        let listener = must(TcpListener::bind("127.0.0.1:0"));
        let port = must(listener.local_addr()).port();
        must(listener.set_nonblocking(true));
        let shared = Arc::new(Shared::default());
        let serving = Arc::clone(&shared);
        let task = thread::spawn(move || {
            let mut workers = Vec::new();
            while !serving.stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let state = Arc::clone(&serving);
                        workers.push(thread::spawn(move || exchange(stream, upstream, &state)));
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(20));
                    }
                    Err(error) => panic!("test proxy accept: {error}"),
                }
                // Expired connections never leave an unbounded thread list
                // during the five-minute expiry test.
                let mut index = 0;
                while index < workers.len() {
                    if workers[index].is_finished() {
                        let worker = workers.swap_remove(index);
                        must(worker.join());
                    } else {
                        index += 1;
                    }
                }
            }
            for worker in workers {
                must(worker.join());
            }
        });
        Self {
            port,
            shared,
            task: Some(task),
        }
    }

    pub fn set(&self, method: &str, path: &str, fault: Fault) {
        self.rule(method, path, fault, false);
    }

    pub fn once(&self, method: &str, path: &str, fault: Fault) {
        self.rule(method, path, fault, true);
    }

    fn rule(&self, method: &str, path: &str, fault: Fault, once: bool) {
        let mut rules = self
            .shared
            .rules
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        rules.retain(|rule| rule.method != method || rule.path != path);
        rules.push(Rule {
            method: method.to_owned(),
            path: path.to_owned(),
            fault,
            once,
        });
    }

    pub fn clear(&self) {
        self.shared
            .rules
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    pub fn traffic(&self) -> Vec<Traffic> {
        self.shared
            .traffic
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Accepted sockets whose workers have reached the request-read boundary.
    pub fn accepted_count(&self) -> usize {
        self.shared.accepted.load(Ordering::Acquire)
    }

    pub fn diagnostics(&self) -> serde_json::Value {
        let traffic = self.traffic();
        let origin = traffic
            .first()
            .map_or_else(Instant::now, |item| item.accepted_at);
        let records: Vec<_> = traffic.iter().skip(traffic.len().saturating_sub(80)).map(|item| serde_json::json!({
            "method": item.method,
            "path": item.path.split('?').next().unwrap_or(""),
            "at_ms": item.at.saturating_duration_since(origin).as_millis(),
            "request_read_ms": item.at.saturating_duration_since(item.accepted_at).as_millis(),
            "stage": item.stage,
            "forwarded": item.forwarded,
            "injected": item.injected,
            "status": item.status,
            "completed": item.completed.is_some(),
            "upstream_connect_ms": item.upstream_connected_at.map(|at| at.saturating_duration_since(item.at).as_millis()),
            "upstream_write_ms": item.upstream_written_at.map(|at| at.saturating_duration_since(item.at).as_millis()),
            "upstream_response_ms": item.upstream_response_at.map(|at| at.saturating_duration_since(item.at).as_millis()),
            "error": item.error.map(|(kind, os)| format!("{kind:?}/{os:?}")),
        })).collect();
        let errors: Vec<_> = self
            .shared
            .request_errors
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .map(|(kind, os)| format!("{kind:?}/{os:?}"))
            .collect();
        serde_json::json!({"traffic":records,"pre_request_errors":errors})
    }

    pub fn count(&self, method: &str, path: &str) -> usize {
        self.traffic()
            .iter()
            .filter(|item| item.method == method && item.path == path)
            .count()
    }

    pub fn injected_count(&self, method: &str, path: &str) -> usize {
        self.traffic()
            .iter()
            .filter(|item| item.method == method && item.path == path && item.injected)
            .count()
    }

    pub fn max_in_flight(&self, method: &str, path: &str) -> usize {
        let records = self.traffic();
        let mut events = Vec::new();
        for item in records
            .iter()
            .filter(|item| item.method == method && item.path == path)
        {
            events.push((item.at, 1_i32));
            if let Some(at) = item.completed {
                events.push((at, -1));
            }
        }
        events.sort_by_key(|event| *event);
        let mut current = 0_i32;
        let mut maximum = 0_i32;
        for (_, delta) in events {
            current += delta;
            maximum = maximum.max(current);
        }
        maximum as usize
    }

    pub(super) fn shutdown(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        if let Some(task) = self.task.take() {
            let result = task.join();
            if !thread::panicking() {
                must(result);
            }
        }
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn pause(duration: Duration, state: &Shared) -> bool {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        if state.stop.load(Ordering::Acquire) {
            return false;
        }
        thread::sleep(Duration::from_millis(20));
    }
    true
}

fn exchange(mut downstream: TcpStream, upstream: u16, state: &Shared) {
    // Windows accepted sockets inherit the listener's nonblocking mode.
    // Chromium may preconnect before sending its request; an immediate
    // WouldBlock must not be mistaken for a closed/failed HTTP connection.
    must(downstream.set_nonblocking(false));
    must(downstream.set_read_timeout(Some(LIMIT)));
    must(downstream.set_write_timeout(Some(LIMIT)));
    let accepted_at = Instant::now();
    state.accepted.fetch_add(1, Ordering::Release);
    let bytes = match request_bytes(&mut downstream) {
        Ok(bytes) => bytes,
        Err(error) => {
            state
                .request_errors
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push((error.kind(), error.raw_os_error()));
            return;
        }
    };
    let Some(head_end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") else {
        return;
    };
    let head = String::from_utf8_lossy(&bytes[..head_end]);
    let first = head.lines().next().unwrap_or("");
    let mut words = first.split_whitespace();
    let method = words.next().unwrap_or("").to_owned();
    let path = words.next().unwrap_or("").to_owned();
    let fault = {
        let mut rules = state.rules.lock().unwrap_or_else(PoisonError::into_inner);
        rules
            .iter()
            .position(|rule| rule.method == method && rule.path == path)
            .map(|index| {
                if rules[index].once {
                    rules.remove(index).fault
                } else {
                    rules[index].fault.clone()
                }
            })
    };
    let index = {
        let mut traffic = state.traffic.lock().unwrap_or_else(PoisonError::into_inner);
        let index = traffic.len();
        traffic.push(Traffic {
            method,
            path,
            headers: head.into_owned(),
            body: String::from_utf8_lossy(&bytes[head_end + 4..]).into_owned(),
            at: Instant::now(),
            completed: None,
            forwarded: false,
            injected: fault.is_some(),
            status: None,
            stage: "request_read",
            error: None,
            accepted_at,
            upstream_connected_at: None,
            upstream_written_at: None,
            upstream_response_at: None,
        });
        index
    };
    let result = forward(
        &mut downstream,
        upstream,
        state,
        index,
        &bytes,
        head_end,
        fault.as_ref(),
    );
    let mut traffic = state.traffic.lock().unwrap_or_else(PoisonError::into_inner);
    traffic[index].completed = Some(Instant::now());
    if let Err(error) = result {
        traffic[index].error = Some((error.kind(), error.raw_os_error()));
    } else {
        traffic[index].stage = "complete";
    }
}

fn forward(
    downstream: &mut TcpStream,
    upstream: u16,
    state: &Shared,
    index: usize,
    bytes: &[u8],
    head_end: usize,
    fault: Option<&Fault>,
) -> io::Result<()> {
    match fault {
        Some(Fault::Close) => return Ok(()),
        Some(Fault::Error500(code)) => {
            let body =
                serde_json::json!({"error":{"code":code,"message":"Injected test proxy failure"}})
                    .to_string();
            let response = format!(
                "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nX-Test-Proxy-Fault: injected\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            state.traffic.lock().unwrap_or_else(PoisonError::into_inner)[index].status = Some(500);
            return downstream.write_all(response.as_bytes());
        }
        Some(Fault::DelayBefore(delay)) if !pause(*delay, state) => return Ok(()),
        _ => {}
    }
    state.traffic.lock().unwrap_or_else(PoisonError::into_inner)[index].stage = "upstream_connect";
    let mut stream = connect(upstream);
    {
        let mut traffic = state.traffic.lock().unwrap_or_else(PoisonError::into_inner);
        traffic[index].upstream_connected_at = Some(Instant::now());
        traffic[index].stage = "upstream_write";
    }
    let head = String::from_utf8_lossy(&bytes[..head_end]);
    let mut request = String::new();
    for line in head.lines() {
        if !line
            .split_once(':')
            .is_some_and(|(name, _)| name.eq_ignore_ascii_case("connection"))
        {
            request.push_str(line);
            request.push_str("\r\n");
        }
    }
    request.push_str("Connection: close\r\n\r\n");
    stream.write_all(request.as_bytes())?;
    stream.write_all(&bytes[head_end + 4..])?;
    state.traffic.lock().unwrap_or_else(PoisonError::into_inner)[index].forwarded = true;
    {
        let mut traffic = state.traffic.lock().unwrap_or_else(PoisonError::into_inner);
        traffic[index].upstream_written_at = Some(Instant::now());
        traffic[index].stage = "upstream_read";
    }
    let response = response_bytes(&mut stream, bytes.starts_with(b"HEAD "))?;
    {
        let mut traffic = state.traffic.lock().unwrap_or_else(PoisonError::into_inner);
        traffic[index].upstream_response_at = Some(Instant::now());
        traffic[index].stage = "downstream_write";
    }
    let status = response
        .split(|byte| *byte == b' ')
        .nth(1)
        .and_then(|value| std::str::from_utf8(value).ok())
        .and_then(|value| value.parse::<u16>().ok());
    state.traffic.lock().unwrap_or_else(PoisonError::into_inner)[index].status = status;
    match fault {
        Some(Fault::DropResponse) => return Ok(()),
        Some(Fault::Delay(delay)) if !pause(*delay, state) => return Ok(()),
        Some(Fault::Hold(released)) => {
            state.traffic.lock().unwrap_or_else(PoisonError::into_inner)[index].stage =
                "response_held";
            let deadline = Instant::now() + LIMIT;
            while !released.load(Ordering::Acquire) {
                if state.stop.load(Ordering::Acquire) {
                    return Ok(());
                }
                if Instant::now() >= deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "test proxy response hold was not released",
                    ));
                }
                thread::sleep(Duration::from_millis(20));
            }
        }
        _ => {}
    }
    downstream.write_all(&response)
}

// Completion is defined by HTTP framing, not by a later TCP FIN. In
// particular, a reset after a complete response must not discard those bytes
// and turn an ordinary successful login into a browser transport failure.
fn response_bytes(stream: &mut TcpStream, head_request: bool) -> io::Result<Vec<u8>> {
    const MAX_RESPONSE: usize = 8 * 1024 * 1024;
    let deadline = Instant::now() + LIMIT;
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    let mut header_end = None;
    let mut expected = None;
    let mut chunked = false;
    let mut chunk_cursor = 0;
    loop {
        if Instant::now() >= deadline || bytes.len() > MAX_RESPONSE {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "proxy response bound",
            ));
        }
        if header_end.is_none()
            && let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n")
        {
            let body = end + 4;
            let headers = String::from_utf8_lossy(&bytes[..end]);
            let status = headers
                .split_whitespace()
                .nth(1)
                .and_then(|value| value.parse::<u16>().ok());
            let fields: Vec<_> = headers
                .lines()
                .filter_map(|line| line.split_once(':'))
                .collect();
            chunked = fields.iter().any(|(name, value)| {
                name.eq_ignore_ascii_case("transfer-encoding")
                    && value
                        .split(',')
                        .any(|value| value.trim().eq_ignore_ascii_case("chunked"))
            });
            expected = if head_request
                || status.is_some_and(|status| {
                    status == 204 || status == 304 || (100..200).contains(&status)
                }) {
                Some(body)
            } else {
                fields
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                    .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                    .and_then(|length| body.checked_add(length))
            };
            header_end = Some(body);
            chunk_cursor = body;
        }
        if let Some(end) = expected {
            if end > MAX_RESPONSE {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "proxy response too large",
                ));
            }
            if bytes.len() >= end {
                bytes.truncate(end);
                return Ok(bytes);
            }
        } else if chunked {
            while let Some(line_end) = bytes[chunk_cursor..]
                .windows(2)
                .position(|part| part == b"\r\n")
            {
                let line_end = chunk_cursor + line_end;
                let text = std::str::from_utf8(&bytes[chunk_cursor..line_end])
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                let size = usize::from_str_radix(text.split(';').next().unwrap_or(""), 16)
                    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                let data = line_end + 2;
                if size == 0 {
                    let trailers = &bytes[data..];
                    let end = if trailers.starts_with(b"\r\n") {
                        Some(data + 2)
                    } else {
                        trailers
                            .windows(4)
                            .position(|part| part == b"\r\n\r\n")
                            .map(|end| data + end + 4)
                    };
                    if let Some(end) = end {
                        bytes.truncate(end);
                        return Ok(bytes);
                    }
                    break;
                }
                let end = data
                    .checked_add(size)
                    .and_then(|end| end.checked_add(2))
                    .filter(|end| *end <= MAX_RESPONSE)
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "proxy chunk too large")
                    })?;
                if bytes.len() < end {
                    break;
                }
                if &bytes[end - 2..end] != b"\r\n" {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid proxy chunk boundary",
                    ));
                }
                chunk_cursor = end;
            }
        }
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            return if header_end.is_some() && expected.is_none() && !chunked {
                Ok(bytes)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "incomplete proxy HTTP response",
                ))
            };
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
}

fn request_bytes(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let deadline = Instant::now() + LIMIT;
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    let mut length = None;
    loop {
        if Instant::now() >= deadline || bytes.len() > 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "proxy request bound",
            ));
        }
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "proxy request ended",
            ));
        }
        bytes.extend_from_slice(&buffer[..read]);
        if length.is_none()
            && let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n")
        {
            let head = String::from_utf8_lossy(&bytes[..end]);
            let body = head
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            length = Some(end + 4 + body);
        }
        if length.is_some_and(|length| bytes.len() >= length) {
            return Ok(bytes);
        }
    }
}
