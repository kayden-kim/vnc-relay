//! Real Windows console input, control events, and shutdown through ConPTY.
#![cfg(windows)]

#[path = "common/shutdown.rs"]
mod shutdown;

#[path = "common/distribution.rs"]
mod distribution;

#[path = "common/log_reopen.rs"]
mod log_reopen;

use std::path::Path;
use std::time::{Duration, Instant};

use shutdown::ServerProcess;

const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");
const LIMIT: Duration = Duration::from_secs(20);

// Q103 confines every Win32 call and unsafe operation to this inline module.
#[expect(
    unsafe_code,
    reason = "Q103 permits only this ConPTY harness to create and control real Windows console children"
)]
mod pseudo_console {
    use std::ffi::OsStr;
    use std::fs::File;
    use std::io::{Read, Write};
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use std::os::windows::process::CommandExt;
    use std::path::Path;
    use std::process::Command;
    use std::ptr;
    use std::sync::{Arc, Mutex, PoisonError};
    use std::thread::JoinHandle;
    use std::time::Instant;

    use windows_sys::Win32::Foundation::{
        GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{
        AttachConsole, COORD, CTRL_BREAK_EVENT, ClosePseudoConsole, CreatePseudoConsole,
        ENABLE_ECHO_INPUT, FreeConsole, GenerateConsoleCtrlEvent, GetConsoleMode, HPCON,
        SetConsoleCtrlHandler,
    };
    use windows_sys::Win32::System::Pipes::CreatePipe;
    use windows_sys::Win32::System::Threading::{
        CREATE_NEW_PROCESS_GROUP, CreateProcessW, DETACHED_PROCESS, DeleteProcThreadAttributeList,
        EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, InitializeProcThreadAttributeList,
        LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, PROCESS_INFORMATION,
        STARTF_USESTDHANDLES, STARTUPINFOEXW, TerminateProcess, UpdateProcThreadAttribute,
        WaitForSingleObject,
    };

    const HELPER_PID: &str = "VNC_RELAY_CONSOLE_HELPER_PID";
    const HELPER_ACTION: &str = "VNC_RELAY_CONSOLE_HELPER_ACTION";

    fn check(ok: i32, operation: &str) {
        assert_ne!(ok, 0, "{operation}: {}", std::io::Error::last_os_error());
    }

    fn wide(text: &OsStr) -> Vec<u16> {
        text.encode_wide().chain(std::iter::once(0)).collect()
    }

    fn pipe() -> (OwnedHandle, OwnedHandle) {
        let mut read = ptr::null_mut();
        let mut write = ptr::null_mut();
        // SAFETY: writable handle slots live through the call; null security
        // attributes make non-inheritable synchronous pipes.
        check(
            unsafe { CreatePipe(&mut read, &mut write, ptr::null(), 0) },
            "CreatePipe",
        );
        // SAFETY: CreatePipe succeeded and transfers two distinct owned handles.
        unsafe {
            (
                OwnedHandle::from_raw_handle(read),
                OwnedHandle::from_raw_handle(write),
            )
        }
    }

    struct Console(HPCON);

    impl Drop for Console {
        fn drop(&mut self) {
            // SAFETY: this guard owns the live HPCON, and the output reader
            // remains active until this call finishes, including final frames.
            unsafe { ClosePseudoConsole(self.0) };
        }
    }

    struct Attributes {
        storage: Vec<usize>,
    }

    impl Attributes {
        fn pointer(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
            self.storage.as_mut_ptr().cast()
        }

        fn new(console: HPCON) -> Self {
            let mut bytes = 0;
            // SAFETY: the documented first call queries the required buffer size.
            unsafe { InitializeProcThreadAttributeList(ptr::null_mut(), 1, 0, &mut bytes) };
            assert!(bytes > 0, "attribute list size was not returned");
            let mut storage = vec![0usize; bytes.div_ceil(size_of::<usize>())];
            // SAFETY: word-aligned storage has at least the requested byte count
            // and is not resized for the lifetime of the initialized list.
            check(
                unsafe {
                    InitializeProcThreadAttributeList(storage.as_mut_ptr().cast(), 1, 0, &mut bytes)
                },
                "InitializeProcThreadAttributeList",
            );
            let mut list = Self { storage };
            // SAFETY: the list is initialized and HPCON is the value required
            // for PSEUDOCONSOLE (not a pointer to a handle-sized slot).
            check(
                unsafe {
                    UpdateProcThreadAttribute(
                        list.pointer(),
                        0,
                        PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                        console as *const _,
                        size_of::<HPCON>(),
                        ptr::null_mut(),
                        ptr::null(),
                    )
                },
                "UpdateProcThreadAttribute",
            );
            list
        }
    }

    impl Drop for Attributes {
        fn drop(&mut self) {
            // SAFETY: the initialized list still occupies its original buffer.
            unsafe { DeleteProcThreadAttributeList(self.pointer()) };
        }
    }

    struct Process {
        handle: OwnedHandle,
        pid: u32,
    }

    impl Process {
        fn try_wait(&self) -> Option<i32> {
            // SAFETY: this guard owns a valid process handle.
            match unsafe { WaitForSingleObject(self.handle.as_raw_handle(), 0) } {
                WAIT_TIMEOUT => None,
                WAIT_OBJECT_0 => {
                    let mut code = 0;
                    // SAFETY: the process is signaled and code is writable.
                    check(
                        unsafe { GetExitCodeProcess(self.handle.as_raw_handle(), &mut code) },
                        "GetExitCodeProcess",
                    );
                    Some(i32::from_ne_bytes(code.to_ne_bytes()))
                }
                other => panic!(
                    "WaitForSingleObject returned {other}: {}",
                    std::io::Error::last_os_error()
                ),
            }
        }

        fn kill(&self) {
            // SAFETY: the handle remains owned until after cleanup. A process
            // that already exited needs no termination; that failure is benign.
            unsafe {
                TerminateProcess(self.handle.as_raw_handle(), 1);
                WaitForSingleObject(self.handle.as_raw_handle(), 5_000);
            }
        }
    }

    impl Drop for Process {
        fn drop(&mut self) {
            self.kill();
        }
    }

    pub struct ConsoleProcess {
        process: Process,
        console: Option<Console>,
        input: File,
        transcript: Arc<Mutex<Vec<u8>>>,
        reader: Option<JoinHandle<()>>,
        pub log_path: Option<std::path::PathBuf>,
    }

    impl ConsoleProcess {
        pub fn spawn(args: &[&OsStr], new_group: bool) -> Self {
            let (input_read, input_write) = pipe();
            let (output_read, output_write) = pipe();
            let mut handle = 0;
            // SAFETY: both pipe endpoints stay open through console creation;
            // the out handle is writable. No cursor inheritance is requested.
            let result = unsafe {
                CreatePseudoConsole(
                    COORD { X: 300, Y: 100 },
                    input_read.as_raw_handle(),
                    output_write.as_raw_handle(),
                    0,
                    &mut handle,
                )
            };
            assert!(result >= 0, "CreatePseudoConsole HRESULT {result:#x}");
            let console = Console(handle);
            let transcript = Arc::new(Mutex::new(Vec::new()));
            let output = Arc::clone(&transcript);
            let mut output_file = File::from(output_read);
            // Drain continuously, also during ClosePseudoConsole, to avoid a
            // full output pipe blocking shutdown or hiding a password echo.
            let reader = std::thread::spawn(move || {
                let mut buffer = [0; 8192];
                loop {
                    match output_file.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(count) => output
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .extend_from_slice(&buffer[..count]),
                    }
                }
            });
            let mut attributes = Attributes::new(handle);
            let application = wide(OsStr::new(super::BIN));
            let mut command = quote(OsStr::new(super::BIN));
            for arg in args {
                command.push(b' ' as u16);
                command.extend(quote(arg));
            }
            command.push(0);
            let mut startup = STARTUPINFOEXW::default();
            startup.StartupInfo.cb = u32::try_from(size_of::<STARTUPINFOEXW>())
                .unwrap_or_else(|e| panic!("startup size: {e}"));
            startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
            startup.StartupInfo.hStdInput = ptr::null_mut();
            startup.StartupInfo.hStdOutput = ptr::null_mut();
            startup.StartupInfo.hStdError = ptr::null_mut();
            startup.lpAttributeList = attributes.pointer();
            let mut info = PROCESS_INFORMATION::default();
            let flags = EXTENDED_STARTUPINFO_PRESENT
                | if new_group {
                    CREATE_NEW_PROCESS_GROUP
                } else {
                    0
                };
            // SAFETY: all UTF-16 buffers are terminated and live through the
            // call; STARTUPINFOEXW and its backing attribute list stay alive.
            // No ordinary handles are inherited; ConPTY supplies console I/O.
            // Ensure normal Ctrl+C handling is active so the child does not
            // inherit an ignored Ctrl+C state from an outer test runner.
            unsafe { SetConsoleCtrlHandler(None, 0) };
            check(
                unsafe {
                    CreateProcessW(
                        application.as_ptr(),
                        command.as_mut_ptr(),
                        ptr::null(),
                        ptr::null(),
                        0,
                        flags,
                        ptr::null(),
                        ptr::null(),
                        &startup.StartupInfo,
                        &mut info,
                    )
                },
                "CreateProcessW",
            );
            // SAFETY: successful CreateProcessW transfers ownership of these
            // two distinct handles. Only the process handle is retained.
            let (process, thread) = unsafe {
                (
                    OwnedHandle::from_raw_handle(info.hProcess),
                    OwnedHandle::from_raw_handle(info.hThread),
                )
            };
            drop(thread);
            drop(input_read);
            drop(output_write);
            Self {
                process: Process {
                    handle: process,
                    pid: info.dwProcessId,
                },
                console: Some(console),
                input: File::from(input_write),
                transcript,
                reader: Some(reader),
                log_path: None,
            }
        }

        pub fn serve(config: &Path, new_group: bool) -> Self {
            let log_path = config.with_file_name("stdout.log");
            File::create(&log_path).unwrap_or_else(|e| panic!("reset serve log: {e}"));
            let mut proc = Self::spawn(
                &[
                    OsStr::new("serve"),
                    OsStr::new("--config"),
                    config.as_os_str(),
                ],
                new_group,
            );
            proc.log_path = Some(log_path);
            proc
        }

        pub fn send(&mut self, bytes: &[u8]) {
            self.input
                .write_all(bytes)
                .unwrap_or_else(|e| panic!("ConPTY input: {e}"));
            self.input
                .flush()
                .unwrap_or_else(|e| panic!("ConPTY flush: {e}"));
        }

        pub fn text(&self) -> String {
            String::from_utf8_lossy(
                &self
                    .transcript
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner),
            )
            .into_owned()
        }

        pub fn wait_for(&self, marker: &str) {
            let deadline = Instant::now() + super::LIMIT;
            while !self.text().contains(marker) {
                assert!(
                    Instant::now() < deadline,
                    "ConPTY missing {marker:?}: {}",
                    self.text()
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        pub fn try_wait(&mut self) -> Option<i32> {
            self.process.try_wait()
        }

        pub fn close_console(&mut self) {
            drop(self.console.take());
        }

        pub fn finish_output(&mut self) -> String {
            self.close_console();
            if let Some(reader) = self.reader.take() {
                reader
                    .join()
                    .unwrap_or_else(|_| panic!("ConPTY reader panicked"));
            }
            self.text()
        }

        pub fn helper(&self, action: &str) {
            let executable =
                std::env::current_exe().unwrap_or_else(|e| panic!("test executable: {e}"));
            // A detached helper attaches only to this child's private console.
            // It never changes the parallel test runner's own console state.
            let output = Command::new(executable)
                .args([
                    "--exact",
                    "console_control_helper",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env(HELPER_PID, self.process.pid.to_string())
                .env(HELPER_ACTION, action)
                .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
                .output()
                .unwrap_or_else(|e| panic!("console helper: {e}"));
            assert!(
                output.status.success(),
                "console helper failed: {:?}\n{}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    impl Drop for ConsoleProcess {
        fn drop(&mut self) {
            self.process.kill();
            self.close_console();
            if let Some(reader) = self.reader.take() {
                let _ = reader.join();
            }
        }
    }

    // Standard Windows argv quoting, including trailing backslashes before
    // the closing quote. Paths are kept as UTF-16, without lossy conversion.
    fn quote(text: &OsStr) -> Vec<u16> {
        let mut result = vec![b'"' as u16];
        let mut slashes = 0;
        for unit in text.encode_wide() {
            if unit == b'\\' as u16 {
                slashes += 1;
                continue;
            }
            result.extend(std::iter::repeat_n(
                b'\\' as u16,
                if unit == b'"' as u16 {
                    slashes * 2 + 1
                } else {
                    slashes
                },
            ));
            result.push(unit);
            slashes = 0;
        }
        result.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2));
        result.push(b'"' as u16);
        result
    }

    pub fn run_helper() {
        let Ok(pid) = std::env::var(HELPER_PID) else {
            return;
        };
        let pid: u32 = pid.parse().unwrap_or_else(|e| panic!("helper pid: {e}"));
        let action = std::env::var(HELPER_ACTION).unwrap_or_else(|e| panic!("helper action: {e}"));
        // SAFETY: this is a separate, env-guarded helper process. Detaching
        // cannot affect the parent test runner or another ConPTY test.
        unsafe { FreeConsole() };
        // SAFETY: pid is a live child whose console the parent owns.
        check(unsafe { AttachConsole(pid) }, "AttachConsole");
        match action.as_str() {
            "break" => {
                // SAFETY: the target was created with CREATE_NEW_PROCESS_GROUP;
                // its PID is its group ID. The helper is outside that group,
                // but attached to the same console, so only the target receives
                // the event. No broadcast reaches the test runner.
                check(
                    unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid) },
                    "GenerateConsoleCtrlEvent",
                );
            }
            "hidden" => {
                let name = wide(OsStr::new("CONIN$"));
                // SAFETY: name is terminated and the remaining parameters are
                // valid console-device access/share flags and null optionals.
                let raw: HANDLE = unsafe {
                    CreateFileW(
                        name.as_ptr(),
                        GENERIC_READ | GENERIC_WRITE,
                        FILE_SHARE_READ | FILE_SHARE_WRITE,
                        ptr::null(),
                        OPEN_EXISTING,
                        0,
                        ptr::null_mut(),
                    )
                };
                assert_ne!(
                    raw,
                    INVALID_HANDLE_VALUE,
                    "open CONIN$: {}",
                    std::io::Error::last_os_error()
                );
                // SAFETY: CreateFileW transferred one valid owned handle.
                let input = unsafe { OwnedHandle::from_raw_handle(raw) };
                let deadline = Instant::now() + super::LIMIT;
                loop {
                    let mut mode = 0;
                    // SAFETY: input is open and mode is writable for this call.
                    check(
                        unsafe { GetConsoleMode(input.as_raw_handle(), &mut mode) },
                        "GetConsoleMode",
                    );
                    if mode & ENABLE_ECHO_INPUT == 0 {
                        break;
                    }
                    assert!(
                        Instant::now() < deadline,
                        "password input never disabled echo"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            other => panic!("unknown console helper action {other}"),
        }
        // SAFETY: only this isolated helper's console association changes.
        check(unsafe { FreeConsole() }, "FreeConsole");
    }
}

impl ServerProcess for pseudo_console::ConsoleProcess {
    fn start(config: &Path) -> Self {
        Self::serve(config, false)
    }

    fn request_stop(&mut self) {
        self.send(&[0x03]);
    }

    fn try_wait(&mut self) -> Option<i32> {
        self.try_wait()
    }
}

#[test]
fn console_control_helper() {
    pseudo_console::run_helper();
}

/// spec: operations.md:OPS-007
#[test]
fn stop_unfinished_work() {
    shutdown::stop_unfinished_work::<pseudo_console::ConsoleProcess>();
}

/// spec: operations.md:OPS-008
#[test]
fn active_session_finishes_in_grace() {
    shutdown::active_session_finishes_in_grace::<pseudo_console::ConsoleProcess>();
}

/// spec: operations.md:OPS-009
#[test]
fn grace_expiry_cleans_session() {
    shutdown::grace_expiry_cleans_session::<pseudo_console::ConsoleProcess>();
}

/// spec: operations.md:OPS-010
#[test]
fn restart_has_fresh_state() {
    shutdown::restart_has_fresh_state::<pseudo_console::ConsoleProcess>();
}

/// spec: operations.md:OPS-042
#[test]
fn second_request_forces_cleanup() {
    shutdown::second_request_forces_cleanup::<pseudo_console::ConsoleProcess>();
}

/// spec: operations.md:OPS-043
/// spec: operations.md:OPS-013 partial
#[test]
fn management_works_during_grace() {
    shutdown::management_works_during_grace::<pseudo_console::ConsoleProcess>();
}

/// spec: operations.md:OPS-044
#[test]
fn console_close_forces_cleanup() {
    shutdown::console_close_forces_cleanup::<pseudo_console::ConsoleProcess>(|process| {
        process.close_console()
    });
}

fn wait_success(process: &mut pseudo_console::ConsoleProcess) -> String {
    let deadline = Instant::now() + LIMIT;
    loop {
        if let Some(code) = process.try_wait() {
            assert_eq!(code, 0, "console child failed: {}", process.text());
            return process.finish_output();
        }
        assert!(
            Instant::now() < deadline,
            "console child did not exit: {}",
            process.text()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn assert_graceful_log(fixture: &shutdown::Fixture) {
    let log = std::fs::read_to_string(fixture.config.with_file_name("stdout.log"))
        .unwrap_or_else(|e| panic!("read serve log: {e}"));
    assert!(log.contains("\"event\":\"shutdown_started\""), "{log}");
    assert!(log.contains("\"event\":\"server_stopped\""), "{log}");
}

#[test]
fn ctrl_c_stops_gracefully_with_exit_code_zero() {
    let fixture = shutdown::Fixture::new("windows-ctrl-c", 1);
    let mut process = pseudo_console::ConsoleProcess::start(&fixture.config);
    fixture.wait_ready(&mut process);
    process.request_stop();
    wait_success(&mut process);
    assert_graceful_log(&fixture);
}

#[test]
fn ctrl_break_stops_gracefully_with_exit_code_zero() {
    let fixture = shutdown::Fixture::new("windows-ctrl-break", 1);
    // Only this child needs a new process group. Applying the flag to Ctrl+C
    // children would inherit Windows' disabled Ctrl+C handling and invalidate
    // the 0x03 input test.
    let mut process = pseudo_console::ConsoleProcess::serve(&fixture.config, true);
    fixture.wait_ready(&mut process);
    process.helper("break");
    wait_success(&mut process);
    assert_graceful_log(&fixture);
}

fn assert_unauthenticated(reply: shutdown::Reply) {
    assert_eq!(reply.status, 401, "{}", reply.body);
    assert!(
        reply.body.contains("\"code\":\"unauthenticated\""),
        "{}",
        reply.body
    );
}

fn assert_relay_listed(fixture: &shutdown::Fixture, admin: &shutdown::Admin, sid: &str) {
    let reply = fixture.query(admin, "/api/v1/sessions");
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert!(
        reply.body.contains(&format!("\"session_id\":\"{sid}\"")),
        "{}",
        reply.body
    );
}

fn assert_peer_closed(stream: &mut std::net::TcpStream) {
    use std::io::Read;

    stream
        .set_read_timeout(Some(LIMIT))
        .unwrap_or_else(|e| panic!("relay timeout: {e}"));
    let mut byte = [0];
    match stream.read(&mut byte) {
        Ok(0) => {}
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
            ) => {}
        result => panic!("terminated relay remained open: {result:?}"),
    }
}

fn terminate_relay(
    fixture: &shutdown::Fixture,
    admin: &shutdown::Admin,
    relay: &mut shutdown::RelayPair,
) {
    use std::net::Shutdown;

    let mut server = relay
        .server
        .try_clone()
        .unwrap_or_else(|e| panic!("clone server peer: {e}"));
    let mut viewer = relay
        .viewer
        .try_clone()
        .unwrap_or_else(|e| panic!("clone viewer peer: {e}"));
    // Acknowledge the relay's write half-close concurrently: the HTTP reply
    // waits for cleanup, so waiting for the reply before peer EOF deadlocks.
    let peers = std::thread::spawn(move || {
        assert_peer_closed(&mut server);
        server
            .shutdown(Shutdown::Write)
            .unwrap_or_else(|e| panic!("server close acknowledgement: {e}"));
        assert_peer_closed(&mut viewer);
        viewer
            .shutdown(Shutdown::Write)
            .unwrap_or_else(|e| panic!("viewer close acknowledgement: {e}"));
    });
    let reply = fixture.terminate(admin, &relay.sid);
    peers
        .join()
        .unwrap_or_else(|_| panic!("close acknowledgement thread failed"));
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert!(
        reply.body.contains("\"result\":\"terminated\""),
        "{}",
        reply.body
    );
    assert!(
        reply.body.contains("\"close_cause\":\"admin_terminated\""),
        "{}",
        reply.body
    );
    assert_peer_closed(&mut relay.viewer);
    assert_peer_closed(&mut relay.server);
    let reply = fixture.query(admin, "/api/v1/sessions");
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert!(!reply.body.contains(&relay.sid), "{}", reply.body);
}

/// spec: management.md:MGT-027
#[test]
fn graceful_restart_ends_the_admin_session_and_login_works_again() {
    let fixture = shutdown::Fixture::new("windows-management-restart", 1);
    let mut first = pseudo_console::ConsoleProcess::start(&fixture.config);
    fixture.wait_ready(&mut first);
    let old = fixture.login();
    assert_eq!(fixture.query(&old, "/api/v1/summary").status, 200);
    first.request_stop();
    wait_success(&mut first);
    assert_graceful_log(&fixture);
    drop(first);

    // Preserve the exact config, accounts, and addresses across graceful exit.
    let mut second = pseudo_console::ConsoleProcess::start(&fixture.config);
    fixture.wait_ready(&mut second);
    let observer = fixture.login();
    let mut relay = shutdown::RelayPair::start(&fixture, "731231");
    relay.transfer();
    assert_unauthenticated(fixture.query(&old, "/api/v1/sessions"));
    assert_unauthenticated(fixture.terminate(&old, &relay.sid));
    assert_relay_listed(&fixture, &observer, &relay.sid);
    relay.transfer();

    let again = fixture.login();
    assert_ne!(old.cookie, again.cookie);
    assert_eq!(fixture.query(&again, "/api/v1/summary").status, 200);
    assert_relay_listed(&fixture, &again, &relay.sid);
    terminate_relay(&fixture, &again, &mut relay);
    second.request_stop();
    wait_success(&mut second);
}

/// spec: operations.md:OPS-029
#[test]
fn cli_prepared_admins_have_hidden_salted_credentials_and_equal_relay_rights() {
    use std::ffi::OsStr;
    use std::fs;
    use vnc_relay_adapters::accounts_file::{self, HashFloor};

    const ALICE_PASSWORD: &str = "correct-horse-battery";
    const BOB_PASSWORD: &str = "another-long-password";
    let fixture = shutdown::Fixture::new("windows-cli-two-accounts", 1);
    let accounts_path = fixture.config.with_file_name("admin-accounts.toml");
    fs::remove_file(&accounts_path).unwrap_or_else(|e| panic!("clear seeded account: {e}"));
    // Windows ACLs are the operator's responsibility (cli-v1 §1.6,
    // user decision 2026-10-20). Unix mode bits are not an ACL assertion.
    let mut transcripts = Vec::new();
    for (account, password) in [("alice", ALICE_PASSWORD), ("bob", BOB_PASSWORD)] {
        let mut terminal = pseudo_console::ConsoleProcess::spawn(
            &[
                OsStr::new("admin"),
                OsStr::new("add"),
                OsStr::new(account),
                OsStr::new("--config"),
                fixture.config.as_os_str(),
            ],
            false,
        );
        for prompt in ["비밀번호:", "비밀번호 확인:"] {
            terminal.wait_for(prompt);
            // The product prints the prompt before rpassword switches modes.
            // Inspect the actual CONIN$ mode in an isolated same-console helper
            // instead of racing the echo flag or sleeping a guessed interval.
            terminal.helper("hidden");
            terminal.send(format!("{password}\r").as_bytes());
        }
        let transcript = wait_success(&mut terminal);
        assert!(
            transcript.contains(&format!("계정 '{account}'을(를) 추가했다.")),
            "{transcript}"
        );
        for secret in [ALICE_PASSWORD, BOB_PASSWORD, "$argon2id$"] {
            assert!(
                !transcript.contains(secret),
                "secret in ConPTY transcript: {transcript}"
            );
        }
        transcripts.push(transcript);
    }
    let stored =
        fs::read_to_string(&accounts_path).unwrap_or_else(|e| panic!("read CLI accounts: {e}"));
    for secret in [ALICE_PASSWORD, BOB_PASSWORD] {
        assert!(!stored.contains(secret), "plaintext password stored");
    }
    let accounts = accounts_file::load(&accounts_path, HashFloor::Reject)
        .unwrap_or_else(|e| panic!("load CLI accounts: {e:?}"))
        .accounts;
    assert_eq!(accounts.len(), 2);
    let mut salts = Vec::new();
    for account in &accounts {
        let hash = account.password_hash.as_str();
        assert!(
            hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
            "{hash}"
        );
        let parts: Vec<_> = hash.split('$').collect();
        assert_eq!(parts.len(), 6, "{hash}");
        assert_eq!(parts[4].len(), 22, "16-byte unpadded salt");
        assert_eq!(parts[5].len(), 43, "32-byte unpadded digest");
        salts.push(parts[4]);
        for transcript in &transcripts {
            assert!(
                !transcript.contains(hash),
                "stored hash in ConPTY transcript"
            );
            assert!(
                !transcript.contains(parts[4]),
                "stored salt in ConPTY transcript"
            );
            assert!(
                !transcript.contains(parts[5]),
                "stored digest in ConPTY transcript"
            );
        }
    }
    assert_ne!(salts[0], salts[1], "separate accounts need different salts");

    let mut process = pseudo_console::ConsoleProcess::start(&fixture.config);
    fixture.wait_ready(&mut process);
    for (account, password, relay_id) in [
        ("alice", ALICE_PASSWORD, "731232"),
        ("bob", BOB_PASSWORD, "731233"),
    ] {
        let admin = shutdown::login(fixture.ports[0], account, password);
        assert_eq!(fixture.query(&admin, "/api/v1/summary").status, 200);
        let mut relay = shutdown::RelayPair::start(&fixture, relay_id);
        relay.transfer();
        assert_relay_listed(&fixture, &admin, &relay.sid);
        terminate_relay(&fixture, &admin, &mut relay);
    }
    let wrong = shutdown::send(
        fixture.ports[0],
        "POST",
        "/api/v1/session",
        &[("Content-Type", "application/json")],
        &format!("{{\"account\":\"bob\",\"password\":\"{ALICE_PASSWORD}\"}}"),
    );
    assert_eq!(wrong.status, 401, "{}", wrong.body);
    assert!(
        wrong.body.contains("\"code\":\"login_failed\""),
        "{}",
        wrong.body
    );
    process.request_stop();
    let transcript = wait_success(&mut process);
    let log = fs::read_to_string(fixture.config.with_file_name("stdout.log"))
        .unwrap_or_else(|e| panic!("read serve log: {e}"));
    for secret in [ALICE_PASSWORD, BOB_PASSWORD, "$argon2id$"] {
        assert!(!transcript.contains(secret), "secret in serve transcript");
        assert!(!log.contains(secret), "secret in serve log");
    }
}

impl distribution::DistributionProcess for pseudo_console::ConsoleProcess {
    fn start(config: &Path) -> Self {
        Self::serve(config, false)
    }

    fn request_stop(&mut self) {
        self.send(&[0x03]);
    }

    fn try_wait(&mut self) -> Option<i32> {
        self.try_wait()
    }
}

/// spec: operations.md:OPS-019
/// spec: operations.md:OPS-031 partial
#[test]
fn example_cli_account_relay_reload_and_graceful_stop() {
    distribution::exercise_example::<pseudo_console::ConsoleProcess>(|config, password| {
        use std::ffi::OsStr;

        let mut terminal = pseudo_console::ConsoleProcess::spawn(
            &[
                OsStr::new("admin"),
                OsStr::new("add"),
                OsStr::new("opsadmin"),
                OsStr::new("--config"),
                config.as_os_str(),
            ],
            false,
        );
        for prompt in ["비밀번호:", "비밀번호 확인:"] {
            terminal.wait_for(prompt);
            terminal.helper("hidden");
            terminal.send(format!("{password}\r").as_bytes());
        }
        let transcript = wait_success(&mut terminal);
        assert!(
            !transcript.contains(password),
            "typed credential in distribution ConPTY transcript"
        );
        assert!(
            !transcript.contains("$argon2"),
            "stored hash in distribution ConPTY transcript"
        );
    });
}

impl log_reopen::LogProcess for pseudo_console::ConsoleProcess {
    fn start(config: &Path) -> Self {
        Self::serve(config, false)
    }
    fn output(&self) -> String {
        self.text()
    }
    fn request_stop(&mut self) {
        self.send(&[0x03]);
    }
    fn try_wait(&mut self) -> Option<i32> {
        self.try_wait()
    }
    fn finish_output(&mut self) -> String {
        self.finish_output()
    }
}

/// Checks actual console stderr, old-handle writes, recovery and Ctrl+C exit.
/// Notification suppression and subsequent failure streaks are adapter coverage.
/// spec: operations.md:OPS-045 partial
#[test]
fn log_reopen_failure_warns_once_keeps_old_file_and_recovers() {
    log_reopen::exercise_failure_and_recovery::<pseudo_console::ConsoleProcess>();
}
