//! The Windows service `vnc-relay`: registration and removal, the SCM control
//! handler, and status reports (cli-v1 §1.9, operations.md §3.7, ADR-0010).
//! Design: docs/architectures/windows-service.md.

#[expect(
    unsafe_code,
    reason = "Win32 account lookup and service DACL change; see docs/conventions/rust.md"
)]
mod security;

use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use tokio::sync::mpsc;
use windows_service::service::{
    Service, ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl,
    ServiceExitCode, ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{
    self, ServiceControlHandlerResult, ServiceStatusHandle,
};
use windows_service::service_dispatcher;
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

pub use security::Operator;

/// The one service name of a PC (Q257).
pub const SERVICE_NAME: &str = "vnc-relay";
const DISPLAY_NAME: &str = "VNC Relay";
/// The virtual account of the service (Q243).
const ACCOUNT: &str = r"NT SERVICE\vnc-relay";

const ERROR_ACCESS_DENIED: i32 = 5;
const ERROR_INVALID_PARAMETER: i32 = 87;
const ERROR_SERVICE_DOES_NOT_EXIST: i32 = 1060;
const ERROR_FAILED_SERVICE_CONTROLLER_CONNECT: i32 = 1063;
const ERROR_SERVICE_EXISTS: i32 = 1073;

/// Progress reports while the service starts or stops (cli-v1 §1.9 "상태 보고").
const CHECKPOINT_INTERVAL: Duration = Duration::from_secs(1);
/// How long the SCM waits for the next checkpoint. Five intervals leave room
/// for a busy machine.
const WAIT_HINT: Duration = Duration::from_secs(5);

/// A failed SCM or account call with its raw OS error number. Calls that fail
/// without one, such as a NUL in a path, carry `ERROR_INVALID_PARAMETER`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScmError(pub i32);

impl ScmError {
    pub fn is_access_denied(self) -> bool {
        self.0 == ERROR_ACCESS_DENIED
    }

    pub fn is_service_exists(self) -> bool {
        self.0 == ERROR_SERVICE_EXISTS
    }

    fn from_io(error: &io::Error) -> Self {
        Self(error.raw_os_error().unwrap_or(ERROR_INVALID_PARAMETER))
    }
}

impl From<windows_service::Error> for ScmError {
    fn from(error: windows_service::Error) -> Self {
        match &error {
            windows_service::Error::Winapi(error) => Self::from_io(error),
            _ => Self(ERROR_INVALID_PARAMETER),
        }
    }
}

/// Looks up an `--operator` name (install step 3). `Ok(None)` when Windows
/// has no user or group of that name.
pub fn lookup_operator(name: &str) -> Result<Option<Operator>, ScmError> {
    security::lookup(name).map_err(|error| ScmError::from_io(&error))
}

/// The SCM of this PC, opened for one service command.
pub struct Scm(ServiceManager);

impl Scm {
    /// Install step 1: connects with the right to create services. A terminal
    /// without administrator rights fails with access denied.
    pub fn open_for_install() -> Result<Self, ScmError> {
        let access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
        Ok(Self(ServiceManager::local_computer(None::<&str>, access)?))
    }

    /// Uninstall step 1: connects without other rights.
    pub fn open() -> Result<Self, ScmError> {
        let access = ServiceManagerAccess::CONNECT;
        Ok(Self(ServiceManager::local_computer(None::<&str>, access)?))
    }

    /// Install step 2: whether the service exists.
    pub fn service_exists(&self) -> Result<bool, ScmError> {
        match self
            .0
            .open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS)
        {
            Ok(_) => Ok(true),
            Err(error) => match ScmError::from(error) {
                ScmError(ERROR_SERVICE_DOES_NOT_EXIST) => Ok(false),
                error => Err(error),
            },
        }
    }

    /// Install step 5: registers the service with the command
    /// `<exe> service run --config <config>`. windows-service quotes a path
    /// that has spaces or quotes. No recovery action is set (Q249).
    pub fn create(&self, exe: &Path, config: &Path) -> Result<Installed, ScmError> {
        let info = ServiceInfo {
            name: SERVICE_NAME.into(),
            display_name: DISPLAY_NAME.into(),
            service_type: ServiceType::OWN_PROCESS,
            start_type: ServiceStartType::AutoStart,
            error_control: ServiceErrorControl::Normal,
            executable_path: exe.to_path_buf(),
            launch_arguments: vec![
                "service".into(),
                "run".into(),
                "--config".into(),
                OsString::from(config),
            ],
            dependencies: Vec::new(),
            account_name: Some(ACCOUNT.into()),
            account_password: None,
        };
        let access = ServiceAccess::QUERY_STATUS
            | ServiceAccess::CHANGE_CONFIG
            | ServiceAccess::READ_CONTROL
            | ServiceAccess::WRITE_DAC
            | ServiceAccess::DELETE;
        Ok(Installed(self.0.create_service(&info, access)?))
    }

    /// Uninstall steps 2 to 4: deletes the service only when it is stopped.
    pub fn uninstall(&self) -> Result<(), UninstallError> {
        let access = ServiceAccess::QUERY_STATUS | ServiceAccess::DELETE;
        let service = self.0.open_service(SERVICE_NAME, access).map_err(|error| {
            match ScmError::from(error) {
                ScmError(ERROR_SERVICE_DOES_NOT_EXIST) => UninstallError::NotFound,
                error if error.is_access_denied() => UninstallError::AccessDenied,
                error => UninstallError::Failed(error),
            }
        })?;
        let status = service
            .query_status()
            .map_err(|error| UninstallError::Failed(error.into()))?;
        if status.current_state != ServiceState::Stopped {
            return Err(UninstallError::Running);
        }
        service
            .delete()
            .map_err(|error| match ScmError::from(error) {
                error if error.is_access_denied() => UninstallError::AccessDenied,
                error => UninstallError::Failed(error),
            })
    }
}

/// Why `service uninstall` stopped (cli-v1 §1.9 "해제 순서").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UninstallError {
    NotFound,
    AccessDenied,
    Running,
    Failed(ScmError),
}

/// A service that this install registered. A later step that fails deletes it.
pub struct Installed(Service);

impl Installed {
    /// The delayed automatic start of install step 5 (Q246).
    pub fn set_delayed_start(&self) -> Result<(), ScmError> {
        Ok(self.0.set_delayed_auto_start(true)?)
    }

    /// Install step 6: adds the operator rights (Q245).
    pub fn grant(&self, operators: &[Operator]) -> Result<(), ScmError> {
        security::grant(self.0.raw_handle(), operators).map_err(|error| ScmError::from_io(&error))
    }

    /// Takes back the registration after a failed later step.
    pub fn delete(self) -> Result<(), ScmError> {
        Ok(self.0.delete()?)
    }
}

/// A control from the SCM that asks the server to stop (cli-v1 §1.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopControl {
    /// Stop: the graceful shutdown with G.
    Stop,
    /// System shutdown: closing at once, as a closed console does (Q248).
    SystemShutdown,
}

/// What the service body gets from the SCM.
pub struct ServiceSession {
    /// Stop controls in arrival order. The sender lives until the process ends.
    pub controls: mpsc::UnboundedReceiver<StopControl>,
    pub status: StatusReporter,
}

/// Why the process could not run as the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchError {
    /// The SCM did not start this process, as when a terminal runs `service run`.
    NotService,
    /// Any other failure: the dispatcher failed, or no control handler
    /// registered, so no status reached the SCM.
    Failed(ScmError),
}

type Body = Box<dyn FnOnce(ServiceSession) -> u8 + Send>;

/// The body for the SCM thread, and its outcome back for the main thread.
/// The dispatcher calls a plain function, so the two meet here.
static BODY: Mutex<Option<Body>> = Mutex::new(None);
static OUTCOME: Mutex<Option<Result<u8, ScmError>>> = Mutex::new(None);

/// Runs `body` as the service `vnc-relay` and returns its exit code. Blocks
/// until the service has stopped. The SCM sees the code as the service exit
/// code: 0 as no error, any other value as a service-specific code.
pub fn run(body: impl FnOnce(ServiceSession) -> u8 + Send + 'static) -> Result<u8, DispatchError> {
    *lock(&BODY) = Some(Box::new(body));
    match service_dispatcher::start(SERVICE_NAME, service_main) {
        Ok(()) => match lock(&OUTCOME).take() {
            Some(Ok(code)) => Ok(code),
            Some(Err(error)) => Err(DispatchError::Failed(error)),
            // The SCM returned without calling the service main function.
            None => Err(DispatchError::Failed(ScmError(ERROR_INVALID_PARAMETER))),
        },
        Err(error) => match ScmError::from(error) {
            ScmError(ERROR_FAILED_SERVICE_CONTROLLER_CONNECT) => Err(DispatchError::NotService),
            error => Err(DispatchError::Failed(error)),
        },
    }
}

/// The service main function that the SCM calls on its own thread. The
/// service arguments are not used: the registered command carries `--config`.
extern "system" fn service_main(_argc: u32, _argv: *mut *mut u16) {
    let Some(body) = lock(&BODY).take() else {
        return;
    };
    let (sender, controls) = mpsc::unbounded_channel();
    let handler = move |control| match control {
        ServiceControl::Stop => {
            // The receiver closes only after the body returned.
            let _ = sender.send(StopControl::Stop);
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Shutdown => {
            let _ = sender.send(StopControl::SystemShutdown);
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    };
    let handle = match service_control_handler::register(SERVICE_NAME, handler) {
        Ok(handle) => handle,
        // Without a handler no status can be reported, so the SCM times out the start.
        Err(error) => {
            *lock(&OUTCOME) = Some(Err(error.into()));
            return;
        }
    };
    let status = StatusReporter::start(handle);
    let code = body(ServiceSession {
        controls,
        status: status.clone(),
    });
    status.stopped(code);
    *lock(&OUTCOME) = Some(Ok(code));
}

/// Reports the service state to the SCM, with a checkpoint every second while
/// the service starts or stops (cli-v1 §1.9 "상태 보고").
#[derive(Clone)]
pub struct StatusReporter(Arc<Reporter>);

struct Reporter {
    handle: ServiceStatusHandle,
    state: Mutex<Progress>,
    tick: Condvar,
}

struct Progress {
    state: ServiceState,
    checkpoint: u32,
}

impl StatusReporter {
    /// Reports the start and starts the checkpoint thread. The thread ends
    /// when the state is stopped.
    fn start(handle: ServiceStatusHandle) -> Self {
        let reporter = Arc::new(Reporter {
            handle,
            state: Mutex::new(Progress {
                state: ServiceState::StartPending,
                checkpoint: 1,
            }),
            tick: Condvar::new(),
        });
        reporter.report(&lock(&reporter.state), ServiceExitCode::NO_ERROR);
        let ticker = Arc::clone(&reporter);
        std::thread::spawn(move || ticker.tick_until_stopped());
        Self(reporter)
    }

    /// The server is ready: it accepts stop and system shutdown controls.
    pub fn running(&self) {
        self.0.set(ServiceState::Running);
    }

    /// The server began its shutdown, by an SCM control or on its own. A
    /// system shutdown can still arrive and force the cleanup.
    pub fn stopping(&self) {
        self.0.set(ServiceState::StopPending);
    }

    fn stopped(&self, code: u8) {
        let exit = match code {
            0 => ServiceExitCode::NO_ERROR,
            code => ServiceExitCode::ServiceSpecific(u32::from(code)),
        };
        let mut progress = lock(&self.0.state);
        progress.state = ServiceState::Stopped;
        progress.checkpoint = 0;
        self.0.report(&progress, exit);
        self.0.tick.notify_all();
    }
}

impl Reporter {
    fn set(&self, state: ServiceState) {
        let mut progress = lock(&self.state);
        if progress.state == state || progress.state == ServiceState::Stopped {
            return;
        }
        progress.state = state;
        progress.checkpoint = u32::from(state != ServiceState::Running);
        self.report(&progress, ServiceExitCode::NO_ERROR);
    }

    fn tick_until_stopped(&self) {
        let mut progress = lock(&self.state);
        while progress.state != ServiceState::Stopped {
            progress = self
                .tick
                .wait_timeout(progress, CHECKPOINT_INTERVAL)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
            if matches!(
                progress.state,
                ServiceState::StartPending | ServiceState::StopPending
            ) {
                progress.checkpoint = progress.checkpoint.saturating_add(1);
                self.report(&progress, ServiceExitCode::NO_ERROR);
            }
        }
    }

    /// A failed report leaves nothing to do: the SCM shows the last state.
    fn report(&self, progress: &Progress, exit_code: ServiceExitCode) {
        let controls_accepted = match progress.state {
            ServiceState::Running => ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
            ServiceState::StopPending => ServiceControlAccept::SHUTDOWN,
            _ => ServiceControlAccept::empty(),
        };
        let pending = matches!(
            progress.state,
            ServiceState::StartPending | ServiceState::StopPending
        );
        let _ = self.handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: progress.state,
            controls_accepted,
            exit_code,
            checkpoint: progress.checkpoint,
            wait_hint: if pending { WAIT_HINT } else { Duration::ZERO },
            process_id: None,
        });
    }
}

/// A poisoned lock still holds a usable value: every writer leaves it whole.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
