//! Shared accept port and fatal socket classification (ops-info-v1 §1.12).
//! Design: docs/architectures/status-listener.md.

use std::io;
use std::net::SocketAddr;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

/// A cancellation-safe accept source. Implementations return actual socket errors.
pub trait Listener: Send + 'static {
    fn accept(&mut self) -> impl Future<Output = io::Result<(TcpStream, SocketAddr)>> + Send;
    fn local_addr(&self) -> io::Result<SocketAddr>;
}

impl Listener for TcpListener {
    async fn accept(&mut self) -> io::Result<(TcpStream, SocketAddr)> {
        TcpListener::accept(self).await
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        TcpListener::local_addr(self)
    }
}

/// A required listener that can no longer accept on its existing socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListenerFailure {
    pub listener: &'static str,
    pub os_error: i32,
}

impl ListenerFailure {
    /// Contains no socket peer, payload or operating-system error text.
    pub fn detail(self) -> String {
        format!(
            "리스너 {} 수락 실패(OS 오류 {}).",
            self.listener, self.os_error
        )
    }
}

/// Reports the first unrecoverable accept error to the process supervisor.
/// The inner port remains owned until the serving loop releases it.
pub struct SupervisedListener<L> {
    inner: L,
    name: &'static str,
    failures: watch::Sender<Option<ListenerFailure>>,
}

impl<L: Listener> SupervisedListener<L> {
    pub fn new(
        inner: L,
        name: &'static str,
        failures: watch::Sender<Option<ListenerFailure>>,
    ) -> Self {
        Self {
            inner,
            name,
            failures,
        }
    }
}

impl<L: Listener> Listener for SupervisedListener<L> {
    async fn accept(&mut self) -> io::Result<(TcpStream, SocketAddr)> {
        let accepted = self.inner.accept().await;
        if let Err(error) = &accepted
            && is_fatal_accept_error(error)
            && let Some(os_error) = error.raw_os_error()
        {
            self.failures.send_if_modified(|first| {
                if first.is_some() {
                    return false;
                }
                *first = Some(ListenerFailure {
                    listener: self.name,
                    os_error,
                });
                true
            });
        }
        accepted
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }
}

/// Only an invalid listener socket or lost socket subsystem is fatal.
/// In particular resource exhaustion and Linux EOPNOTSUPP remain retryable.
pub fn is_fatal_accept_error(error: &io::Error) -> bool {
    #[cfg(target_os = "linux")]
    let fatal = matches!(error.raw_os_error(), Some(9 | 22 | 88));
    #[cfg(windows)]
    let fatal = matches!(
        error.raw_os_error(),
        Some(10009 | 10022 | 10038 | 10045 | 10093)
    );
    #[cfg(not(any(target_os = "linux", windows)))]
    let fatal = false;
    fatal
}

#[cfg(test)]
mod tests {
    use super::*;
    use socket2::{Domain, Protocol, Socket, Type};

    #[test]
    fn classifies_only_lost_listener_state_and_retries_resources_and_network_errors() {
        #[cfg(target_os = "linux")]
        let (fatal, retry): (&[i32], &[i32]) = (
            &[9, 22, 88],
            &[4, 11, 12, 14, 23, 24, 95, 100, 101, 103, 105, 110, 113],
        );
        #[cfg(windows)]
        let (fatal, retry): (&[i32], &[i32]) = (
            &[10009, 10022, 10038, 10045, 10093],
            &[
                995, 10004, 10014, 10024, 10035, 10036, 10050, 10053, 10054, 10055,
            ],
        );
        #[cfg(not(any(target_os = "linux", windows)))]
        let (fatal, retry): (&[i32], &[i32]) = (&[], &[22]);
        for code in fatal {
            assert!(
                is_fatal_accept_error(&io::Error::from_raw_os_error(*code)),
                "{code}"
            );
        }
        for code in retry {
            assert!(
                !is_fatal_accept_error(&io::Error::from_raw_os_error(*code)),
                "{code}"
            );
        }
        assert!(!is_fatal_accept_error(&io::Error::other("unclassified")));
    }

    #[tokio::test]
    async fn actual_os_rejects_a_non_listening_socket_and_supervision_preserves_the_error() {
        // No raw handle is closed behind Rust's ownership. This observes a real
        // invalid-listen-state error, not damage to an already-running binary.
        struct NotListening(std::net::TcpListener);
        impl Listener for NotListening {
            async fn accept(&mut self) -> io::Result<(TcpStream, SocketAddr)> {
                let (stream, address) = self.0.accept()?;
                stream.set_nonblocking(true)?;
                Ok((TcpStream::from_std(stream)?, address))
            }
            fn local_addr(&self) -> io::Result<SocketAddr> {
                self.0.local_addr()
            }
        }
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
            .unwrap_or_else(|error| panic!("socket: {error}"));
        socket
            .set_nonblocking(true)
            .unwrap_or_else(|error| panic!("nonblocking: {error}"));
        let (sender, mut failed) = watch::channel(None);
        let mut listener =
            SupervisedListener::new(NotListening(socket.into()), "tcp_viewer", sender);
        let error = tokio::time::timeout(std::time::Duration::from_secs(2), listener.accept())
            .await
            .unwrap_or_else(|error| panic!("accept bounded: {error}"))
            .err()
            .unwrap_or_else(|| panic!("non-listening socket cannot accept"));
        assert!(is_fatal_accept_error(&error), "{error}");
        assert_eq!(
            failed
                .borrow_and_update()
                .as_ref()
                .map(|failure| failure.os_error),
            error.raw_os_error()
        );
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn linux_shutdown_of_a_real_bound_listener_reports_its_actual_accept_error() {
        let socket = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("listener bind: {error}"));
        // Linux shutdown changes the shared socket's listening state without
        // closing a raw fd behind Rust. Windows has no equivalent guarantee.
        socket2::SockRef::from(&socket)
            .shutdown(std::net::Shutdown::Both)
            .unwrap_or_else(|error| panic!("listener shutdown: {error}"));
        let (sender, failed) = watch::channel(None);
        let mut listener = SupervisedListener::new(socket, "management", sender);
        let error = tokio::time::timeout(std::time::Duration::from_secs(2), listener.accept())
            .await
            .unwrap_or_else(|error| panic!("accept deadline: {error}"))
            .err()
            .unwrap_or_else(|| panic!("shut down listener accepted"));
        assert_eq!(error.raw_os_error(), Some(22));
        assert_eq!(
            *failed.borrow(),
            Some(ListenerFailure {
                listener: "management",
                os_error: 22
            })
        );
    }
}
