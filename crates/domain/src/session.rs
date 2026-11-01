//! Relay session state and finalized summaries, without I/O or clock access.
//! See management-api-v1 §§1.8–1.10 and ops-info-v1 §1.6.

use std::fmt;
use std::time::{Duration, SystemTime};

use crate::handshake::Mode;

const BASE32: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

/// Canonical producer ID: `ses_` and 26 lowercase, unpadded base32 characters.
/// Uniqueness and keyed block generation belong to the caller, not this value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId([u8; 30]);

impl SessionId {
    /// Encodes a supplied 128-bit block, most significant bit first.
    /// The last character has two zero padding bits. No randomness is obtained.
    pub fn from_block(block: [u8; 16]) -> Self {
        let mut text = [b'a'; 30];
        text[..4].copy_from_slice(b"ses_");
        let value = u128::from_be_bytes(block);
        for (index, character) in text[4..29].iter_mut().enumerate() {
            *character = BASE32[((value >> (123 - index * 5)) & 31) as usize];
        }
        text[29] = BASE32[((value & 7) << 2) as usize];
        Self(text)
    }

    /// Validates the producer format, including canonical zero padding.
    /// This is narrower than the consumer URL grammar in management-api-v1 §1.8.
    pub fn parse(text: &str) -> Result<Self, SessionError> {
        let bytes = text.as_bytes();
        if bytes.len() != 30 || !bytes.starts_with(b"ses_") {
            return Err(SessionError::InvalidId);
        }
        for (index, byte) in bytes[4..].iter().enumerate() {
            let value = match byte {
                b'a'..=b'z' => byte - b'a',
                b'2'..=b'7' => byte - b'2' + 26,
                _ => return Err(SessionError::InvalidId),
            };
            if index == 25 && value & 3 != 0 {
                return Err(SessionError::InvalidId);
            }
        }
        let mut value = [0; 30];
        value.copy_from_slice(bytes);
        Ok(Self(value))
    }

    /// Returns the canonical ASCII representation without allocation.
    pub fn as_str(&self) -> &str {
        match std::str::from_utf8(&self.0) {
            Ok(text) => text,
            Err(_) => unreachable!("SessionId constructors only store ASCII"),
        }
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Endpoint names used by operation events (ops-info-v1 §1.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The viewer endpoint in either mode.
    Viewer,
    /// The reverse-connected server endpoint in Mode II.
    Server,
    /// The connected target endpoint in Mode I.
    Target,
}

impl Side {
    /// Returns the contract field value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Server => "server",
            Self::Target => "target",
        }
    }
}

/// Viewer transport metadata (management-api-v1 §1.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerTransport {
    /// Plain TCP viewer connection.
    Tcp,
    /// WebSocket viewer connection.
    Ws,
    /// TLS WebSocket viewer connection.
    Wss,
}

impl ViewerTransport {
    /// Returns the contract field value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Ws => "ws",
            Self::Wss => "wss",
        }
    }

    /// Whether this is a WebSocket transport (`ws` or `wss`).
    pub fn is_websocket(self) -> bool {
        matches!(self, Self::Ws | Self::Wss)
    }
}

/// Payload direction, independent of the non-viewer endpoint's mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Viewer to Server or Mode I target.
    ViewerToTarget,
    /// Server or Mode I target to Viewer.
    TargetToViewer,
}

impl Direction {
    fn index(self) -> usize {
        match self {
            Self::ViewerToTarget => 0,
            Self::TargetToViewer => 1,
        }
    }
}

/// Session lifecycle; `Closing` still counts as active until cleanup finishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Both relay directions are available.
    Active,
    /// EOF or another close trigger has started cleanup.
    Closing,
    /// Cleanup has been confirmed and the summary has been produced.
    Finished,
}

impl SessionState {
    /// Returns the lifecycle name; finished sessions leave the active registry.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Closing => "closing",
            Self::Finished => "finished",
        }
    }
}

/// Final close reasons (management-api-v1 §1.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseCause {
    /// Both normal directions and final cleanup completed.
    PeerEof,
    /// An I/O operation failed on a still-open relay direction.
    ConnectionError,
    /// An administrator initiated cleanup.
    AdminTerminated,
    /// An external liveness check found an unresponsive endpoint.
    DeadConnection,
    /// EOF drain or administrator cleanup exceeded E.
    CloseTimeout,
    /// G expired or a second shutdown request forced cleanup.
    ServerShutdown,
}

impl CloseCause {
    /// Returns the exact contract value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PeerEof => "peer_eof",
            Self::ConnectionError => "connection_error",
            Self::AdminTerminated => "admin_terminated",
            Self::DeadConnection => "dead_connection",
            Self::CloseTimeout => "close_timeout",
            Self::ServerShutdown => "server_shutdown",
        }
    }
}

/// Invalid producer IDs or invalid session transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    /// The ID does not have the canonical producer encoding.
    InvalidId,
    /// The endpoint does not exist in this session's mode.
    InvalidSide,
    /// Normal direction completion requires its input EOF first.
    DirectionNotEof,
    /// Cleanup cannot finish before a close trigger.
    NotClosing,
    /// Provisional normal EOF requires both directions to finish.
    IncompleteDirections,
    /// The session has already produced its only closed summary.
    Finished,
    /// Successful writes cannot be added to a normally completed direction.
    DirectionFinished,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidId => "invalid canonical session ID",
            Self::InvalidSide => "endpoint does not exist in this relay mode",
            Self::DirectionNotEof => "direction has not observed EOF",
            Self::NotClosing => "session cleanup has not started",
            Self::IncompleteDirections => "normal relay directions have not both completed",
            Self::Finished => "session cleanup has already finished",
            Self::DirectionFinished => "relay direction has already completed",
        })
    }
}

impl std::error::Error for SessionError {}

/// Active relay state with supplied wall and monotonic timestamps.
/// EOF is provisional: `peer_eof` is established only by `finish` after both
/// directions have drained and shut down. Socket ownership stays in app.
#[derive(Debug)]
pub struct Session {
    id: SessionId,
    mode: Mode,
    viewer_transport: ViewerTransport,
    started_at: SystemTime,
    started_mono: Duration,
    state: SessionState,
    bytes: [u64; 2],
    eof: [bool; 2],
    completed: [bool; 2],
    first_eof: Option<(Side, Duration)>,
    close_started_mono: Option<Duration>,
    close_cause: Option<CloseCause>,
    close_side: Option<Side>,
}

impl Session {
    /// Starts a fresh active session with zero payload counters.
    pub fn new(
        id: SessionId,
        mode: Mode,
        viewer_transport: ViewerTransport,
        started_at: SystemTime,
        started_mono: Duration,
    ) -> Self {
        Self {
            id,
            mode,
            viewer_transport,
            started_at,
            started_mono,
            state: SessionState::Active,
            bytes: [0; 2],
            eof: [false; 2],
            completed: [false; 2],
            first_eof: None,
            close_started_mono: None,
            close_cause: None,
            close_side: None,
        }
    }

    /// Returns this session's identifier, never a relay ID.
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    /// Returns the mode selected by the initial exchange.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Returns the viewer's transport metadata.
    pub fn viewer_transport(&self) -> ViewerTransport {
        self.viewer_transport
    }

    /// Returns the wall-clock relay start time.
    pub fn started_at(&self) -> SystemTime {
        self.started_at
    }

    /// Returns the current lifecycle state.
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Returns the nonnegative elapsed time from supplied monotonic time.
    pub fn duration(&self, now: Duration) -> Duration {
        now.saturating_sub(self.started_mono)
    }

    /// Returns successfully written payload bytes in the named direction.
    pub fn bytes(&self, direction: Direction) -> u64 {
        self.bytes[direction.index()]
    }

    /// Returns whether input EOF, drain, and output shutdown have completed.
    pub fn direction_completed(&self, direction: Direction) -> bool {
        self.completed[direction.index()]
    }

    /// Returns the first observed EOF endpoint and monotonic time, if any.
    pub fn first_eof(&self) -> Option<(Side, Duration)> {
        self.first_eof
    }

    /// Returns the selected cause; provisional EOF has no final cause yet.
    pub fn close_cause(&self) -> Option<CloseCause> {
        self.close_cause
    }

    /// Returns the failing endpoint or, for EOF timeout, the first EOF endpoint.
    pub fn close_side(&self) -> Option<Side> {
        self.close_side
    }

    /// Adds only bytes that a payload write actually completed, with saturation.
    /// Call separately for each partial success, excluding protocol framing.
    pub fn record_write(&mut self, direction: Direction, bytes: u64) -> Result<(), SessionError> {
        self.ensure_unfinished()?;
        let index = direction.index();
        if self.completed[index] {
            return Err(SessionError::DirectionFinished);
        }
        self.bytes[index] = self.bytes[index].saturating_add(bytes);
        Ok(())
    }

    /// Records input EOF reported by read=0, not merely socket readiness.
    /// Repeated EOF observations never restart E or change the first endpoint.
    pub fn observe_eof(&mut self, side: Side, now: Duration) -> Result<(), SessionError> {
        self.ensure_unfinished()?;
        let index = self.input_direction(side)?.index();
        self.eof[index] = true;
        if self.first_eof.is_none() {
            self.first_eof = Some((side, now));
        }
        if self.state == SessionState::Active {
            self.state = SessionState::Closing;
            self.close_started_mono = Some(now);
        }
        Ok(())
    }

    /// Confirms that input EOF, all buffered writes, and output shutdown finished.
    /// This does not itself confirm final socket and permit cleanup.
    pub fn complete_direction(&mut self, direction: Direction) -> Result<(), SessionError> {
        self.ensure_unfinished()?;
        let index = direction.index();
        if !self.eof[index] {
            return Err(SessionError::DirectionNotEof);
        }
        self.completed[index] = true;
        Ok(())
    }

    /// Records a relay read/write/shutdown failure on a still-open direction.
    /// `side` is the read input or write/shutdown output endpoint. Failures from
    /// an already completed direction are cleanup-only and preserve the cause.
    pub fn io_error(&mut self, direction: Direction, side: Side) -> Result<bool, SessionError> {
        self.ensure_unfinished()?;
        self.input_direction(side)?;
        if self.completed[direction.index()] {
            return Ok(false);
        }
        Ok(self.select_cause(CloseCause::ConnectionError, Some(side)))
    }

    /// Accepts an external liveness verdict; inactivity alone is not a verdict.
    pub fn dead_connection(&mut self, side: Side) -> Result<bool, SessionError> {
        self.ensure_unfinished()?;
        self.input_direction(side)?;
        Ok(self.select_cause(CloseCause::DeadConnection, Some(side)))
    }

    /// Starts administrator cleanup only if the session is still active.
    /// The caller retains the authenticated account for the operation log.
    pub fn begin_admin_close(&mut self, now: Duration) -> Result<bool, SessionError> {
        self.ensure_unfinished()?;
        if self.state != SessionState::Active {
            return Ok(false);
        }
        self.close_started_mono = Some(now);
        Ok(self.select_cause(CloseCause::AdminTerminated, None))
    }

    /// Forces shutdown after G or a second request for any unfinished session.
    /// Sets `ServerShutdown` only when no cleanup has started; an existing first cause
    /// (EOF observed, I/O error, dead connection, admin close) is preserved.
    pub fn force_server_shutdown(&mut self) -> Result<bool, SessionError> {
        if self.state == SessionState::Finished {
            return Ok(false);
        }
        if self.close_cause.is_some() {
            return Ok(false);
        }
        if self.first_eof.is_some() {
            self.close_cause = Some(CloseCause::PeerEof);
            self.close_side = self.first_eof.map(|(side, _)| side);
            return Ok(false);
        }
        self.state = SessionState::Closing;
        self.close_cause = Some(CloseCause::ServerShutdown);
        self.close_side = None;
        Ok(true)
    }

    /// Applies the absolute E bound to provisional EOF or administrator cleanup.
    /// Equality expires; repeated EOF and later close requests do not restart E.
    /// Administrator timeout has no endpoint; EOF timeout uses the first EOF.
    pub fn expire_close(&mut self, now: Duration, timeout: Duration) -> Result<bool, SessionError> {
        self.ensure_unfinished()?;
        if !matches!(self.close_cause, None | Some(CloseCause::AdminTerminated)) {
            return Ok(false);
        }
        let Some(started) = self.close_started_mono else {
            return Ok(false);
        };
        if now < started || now.saturating_sub(started) < timeout {
            return Ok(false);
        }
        self.close_side = if self.close_cause == Some(CloseCause::AdminTerminated) {
            None
        } else {
            self.first_eof.map(|(side, _)| side)
        };
        self.close_cause = Some(CloseCause::CloseTimeout);
        Ok(true)
    }

    /// Confirms final socket/buffer/permit cleanup and produces the only summary.
    /// The caller must finish cleanup before calling. Cleanup-only errors do not
    /// enter `io_error` and cannot change a selected cause. Wall time is clamped
    /// to the start; elapsed time uses only supplied monotonic timestamps.
    pub fn finish(
        &mut self,
        ended_at: SystemTime,
        ended_mono: Duration,
    ) -> Result<ClosedSession, SessionError> {
        self.ensure_unfinished()?;
        if self.state != SessionState::Closing {
            return Err(SessionError::NotClosing);
        }
        let cause = match self.close_cause {
            Some(cause) => cause,
            None if self.completed == [true; 2] => {
                self.close_side = self.first_eof.map(|(side, _)| side);
                CloseCause::PeerEof
            }
            None => return Err(SessionError::IncompleteDirections),
        };
        self.close_cause = Some(cause);
        self.state = SessionState::Finished;
        Ok(ClosedSession {
            id: self.id.clone(),
            mode: self.mode,
            viewer_transport: self.viewer_transport,
            started_at: self.started_at,
            ended_at: ended_at.max(self.started_at),
            duration: self.duration(ended_mono),
            bytes: self.bytes,
            close_cause: cause,
            close_side: self.close_side,
        })
    }

    fn input_direction(&self, side: Side) -> Result<Direction, SessionError> {
        match (self.mode, side) {
            (_, Side::Viewer) => Ok(Direction::ViewerToTarget),
            (Mode::Mode1, Side::Target) | (Mode::Mode2, Side::Server) => {
                Ok(Direction::TargetToViewer)
            }
            _ => Err(SessionError::InvalidSide),
        }
    }

    fn ensure_unfinished(&self) -> Result<(), SessionError> {
        if self.state == SessionState::Finished {
            Err(SessionError::Finished)
        } else {
            Ok(())
        }
    }

    fn select_cause(&mut self, cause: CloseCause, side: Option<Side>) -> bool {
        if self.close_cause.is_some() {
            return false;
        }
        self.state = SessionState::Closing;
        self.close_cause = Some(cause);
        self.close_side = side;
        true
    }
}

/// Immutable final data from an activated session whose cleanup has completed.
/// Only `Session::finish` can construct this value (management-api-v1 §1.10).
#[derive(Debug, PartialEq, Eq)]
pub struct ClosedSession {
    id: SessionId,
    mode: Mode,
    viewer_transport: ViewerTransport,
    started_at: SystemTime,
    ended_at: SystemTime,
    duration: Duration,
    bytes: [u64; 2],
    close_cause: CloseCause,
    close_side: Option<Side>,
}

impl ClosedSession {
    /// Returns the finalized session identifier.
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    /// Returns the finalized relay mode.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Returns the viewer's transport metadata.
    pub fn viewer_transport(&self) -> ViewerTransport {
        self.viewer_transport
    }

    /// Returns the wall-clock relay start time.
    pub fn started_at(&self) -> SystemTime {
        self.started_at
    }

    /// Returns cleanup wall time, clamped to at least `started_at`.
    pub fn ended_at(&self) -> SystemTime {
        self.ended_at
    }

    /// Returns nonnegative elapsed monotonic time through cleanup completion.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Returns the final successfully written payload count in one direction.
    pub fn bytes(&self, direction: Direction) -> u64 {
        self.bytes[direction.index()]
    }

    /// Returns the final contract close cause.
    pub fn close_cause(&self) -> CloseCause {
        self.close_cause
    }

    /// Returns the cause endpoint, absent for administrator/server cleanup.
    pub fn close_side(&self) -> Option<Side> {
        self.close_side
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORWARD: Direction = Direction::ViewerToTarget;
    const REVERSE: Direction = Direction::TargetToViewer;

    fn session(mode: Mode) -> Session {
        Session::new(
            SessionId::from_block([1; 16]),
            mode,
            ViewerTransport::Tcp,
            SystemTime::UNIX_EPOCH + Duration::from_secs(100),
            Duration::from_secs(10),
        )
    }

    fn finish(session: &mut Session) -> ClosedSession {
        session
            .finish(
                SystemTime::UNIX_EPOCH + Duration::from_secs(120),
                Duration::from_secs(30),
            )
            .unwrap()
    }

    fn normal_complete(session: &mut Session, first: Side, second: Side) {
        session.observe_eof(first, Duration::from_secs(12)).unwrap();
        session
            .observe_eof(second, Duration::from_secs(13))
            .unwrap();
        session.complete_direction(FORWARD).unwrap();
        session.complete_direction(REVERSE).unwrap();
    }

    #[test]
    fn encodes_canonical_base32_vectors_without_allocation() {
        let vectors = [
            ([0; 16], "ses_aaaaaaaaaaaaaaaaaaaaaaaaaa"),
            ([255; 16], "ses_77777777777777777777777774"),
            (*b"abcdefghijklmnop", "ses_mfrggzdfmztwq2lknnwg23tpoa"),
        ];
        for (block, expected) in vectors {
            let id = SessionId::from_block(block);
            assert_eq!(id.as_str(), expected);
            assert_eq!(id.to_string(), expected);
            assert_eq!(SessionId::parse(expected), Ok(id.clone()));
            let mut ids = std::collections::HashSet::new();
            assert!(ids.insert(id.clone()));
            assert!(!ids.insert(id));
        }
        let mut low = [0; 16];
        low[15] = 1;
        assert_eq!(
            SessionId::from_block(low).as_str(),
            "ses_aaaaaaaaaaaaaaaaaaaaaaaaae"
        );
        let mut high = [0; 16];
        high[0] = 128;
        assert_eq!(
            SessionId::from_block(high).as_str(),
            "ses_qaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
    }

    #[test]
    fn covers_every_input_bit_and_canonical_last_character() {
        let mut ids = std::collections::HashSet::new();
        for bit in 0..128 {
            let mut block = [0; 16];
            block[bit / 8] = 1 << (7 - bit % 8);
            let id = SessionId::from_block(block);
            assert_eq!(id.as_str().len(), 30);
            assert_eq!(SessionId::parse(id.as_str()), Ok(id.clone()));
            assert!(ids.insert(id));
        }
        for (index, character) in BASE32.iter().enumerate() {
            let text = format!("ses_aaaaaaaaaaaaaaaaaaaaaaaaa{}", char::from(*character));
            assert_eq!(SessionId::parse(&text).is_ok(), index % 4 == 0);
        }
    }

    #[test]
    fn rejects_malformed_and_noncanonical_producer_ids() {
        for text in [
            "",
            "ses_",
            "aaaaaaaaaaaaaaaaaaaaaaaaaa",
            "SES_aaaaaaaaaaaaaaaaaaaaaaaaaa",
            "ses_Aaaaaaaaaaaaaaaaaaaaaaaaaa",
            "ses_aaaaaaaaaaaaaaaaaaaaaaaaa",
            "ses_aaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "ses_aaaaaaaaaaaaaaaaaaaaaaaaaa=",
            "ses_aaaaaaaaaaaaaaaaaaaaaaaaab",
            "ses_aaaaaaaaaaaaaaaaaaaaaaaaa7",
            "ses_0aaaaaaaaaaaaaaaaaaaaaaaaa",
            "ses_1aaaaaaaaaaaaaaaaaaaaaaaaa",
            "ses_8aaaaaaaaaaaaaaaaaaaaaaaaa",
            "ses_aaaaaaaaaaaaaaaaaaaaaaaaa_",
            "ses_aaaaaaaaaaaaaaaaaaaaaaaaa-",
            "ses_aaaaaaaaaaaaaaaaaaaaaaaaaé",
            "ses_aaaaaaaaaaaaaaaaaaaaaaaaaa\n",
            " ses_aaaaaaaaaaaaaaaaaaaaaaaaaa",
        ] {
            assert_eq!(
                SessionId::parse(text),
                Err(SessionError::InvalidId),
                "{text:?}"
            );
        }
    }

    #[test]
    fn exposes_exact_contract_values() {
        for (cause, expected) in [
            (CloseCause::PeerEof, "peer_eof"),
            (CloseCause::ConnectionError, "connection_error"),
            (CloseCause::AdminTerminated, "admin_terminated"),
            (CloseCause::DeadConnection, "dead_connection"),
            (CloseCause::CloseTimeout, "close_timeout"),
            (CloseCause::ServerShutdown, "server_shutdown"),
        ] {
            assert_eq!(cause.as_str(), expected);
        }
        assert_eq!(Side::Viewer.as_str(), "viewer");
        assert_eq!(Side::Server.as_str(), "server");
        assert_eq!(Side::Target.as_str(), "target");
        assert_eq!(ViewerTransport::Tcp.as_str(), "tcp");
        assert_eq!(ViewerTransport::Ws.as_str(), "ws");
        assert_eq!(ViewerTransport::Wss.as_str(), "wss");
        assert_eq!(SessionState::Active.as_str(), "active");
        assert_eq!(SessionState::Closing.as_str(), "closing");
        assert_eq!(SessionState::Finished.as_str(), "finished");
    }

    /// spec: relay.md:RLY-019 partial
    /// spec: relay.md:RLY-020 partial
    /// spec: operations.md:OPS-008 partial
    #[test]
    fn requires_both_normal_directions_and_cleanup_before_peer_eof() {
        for (mode, other) in [(Mode::Mode1, Side::Target), (Mode::Mode2, Side::Server)] {
            for (first, second) in [(Side::Viewer, other), (other, Side::Viewer)] {
                let mut session = session(mode);
                assert_eq!(session.state(), SessionState::Active);
                assert_eq!(
                    session.complete_direction(FORWARD),
                    Err(SessionError::DirectionNotEof)
                );
                assert_eq!(
                    session.finish(SystemTime::UNIX_EPOCH, Duration::ZERO),
                    Err(SessionError::NotClosing)
                );
                session.observe_eof(first, Duration::from_secs(12)).unwrap();
                assert_eq!(session.state(), SessionState::Closing);
                assert_eq!(session.close_cause(), None);
                let first_direction = if first == Side::Viewer {
                    FORWARD
                } else {
                    REVERSE
                };
                session.record_write(first_direction, 3).unwrap();
                session.complete_direction(first_direction).unwrap();
                session.complete_direction(first_direction).unwrap();
                assert_eq!(
                    session.record_write(first_direction, 1),
                    Err(SessionError::DirectionFinished)
                );
                assert_eq!(
                    session.finish(SystemTime::UNIX_EPOCH, Duration::ZERO),
                    Err(SessionError::IncompleteDirections)
                );
                let second_direction = if second == Side::Viewer {
                    FORWARD
                } else {
                    REVERSE
                };
                session.record_write(second_direction, 9).unwrap();
                session
                    .observe_eof(second, Duration::from_secs(13))
                    .unwrap();
                assert_eq!(
                    session.finish(SystemTime::UNIX_EPOCH, Duration::ZERO),
                    Err(SessionError::IncompleteDirections)
                );
                session.complete_direction(second_direction).unwrap();
                assert_eq!(session.close_cause(), None);
                let closed = finish(&mut session);
                assert_eq!(closed.close_cause(), CloseCause::PeerEof);
                assert_eq!(closed.close_side(), Some(first));
                assert_eq!(closed.bytes(first_direction), 3);
                assert_eq!(closed.bytes(second_direction), 9);
                assert_eq!(session.state(), SessionState::Finished);
            }
        }
    }

    /// spec: relay.md:RLY-019 partial
    #[test]
    fn uses_first_eof_deadline_including_drain_and_final_cleanup() {
        for both_completed in [false, true] {
            let mut session = session(Mode::Mode2);
            session
                .observe_eof(Side::Server, Duration::from_secs(12))
                .unwrap();
            session
                .observe_eof(Side::Server, Duration::from_secs(14))
                .unwrap();
            session
                .observe_eof(Side::Viewer, Duration::from_secs(15))
                .unwrap();
            assert_eq!(
                session.first_eof(),
                Some((Side::Server, Duration::from_secs(12)))
            );
            if both_completed {
                session.complete_direction(FORWARD).unwrap();
                session.complete_direction(REVERSE).unwrap();
            }
            assert!(
                !session
                    .expire_close(Duration::from_secs(11), Duration::ZERO)
                    .unwrap()
            );
            assert!(
                !session
                    .expire_close(Duration::from_secs(16), Duration::from_secs(5))
                    .unwrap()
            );
            assert!(
                session
                    .expire_close(Duration::from_secs(17), Duration::from_secs(5))
                    .unwrap()
            );
            assert!(
                !session
                    .expire_close(Duration::from_secs(18), Duration::from_secs(5))
                    .unwrap()
            );
            let closed = finish(&mut session);
            assert_eq!(closed.close_cause(), CloseCause::CloseTimeout);
            assert_eq!(closed.close_side(), Some(Side::Server));
        }
        let mut active = session(Mode::Mode1);
        assert!(!active.expire_close(Duration::MAX, Duration::ZERO).unwrap());
        active.observe_eof(Side::Viewer, Duration::MAX).unwrap();
        assert!(active.expire_close(Duration::MAX, Duration::ZERO).unwrap());
    }

    /// spec: relay.md:RLY-020 partial
    #[test]
    fn records_read_write_and_shutdown_error_endpoints_and_replaces_provisional_eof() {
        for (mode, other) in [(Mode::Mode1, Side::Target), (Mode::Mode2, Side::Server)] {
            for direction in [FORWARD, REVERSE] {
                for failing_side in [Side::Viewer, other] {
                    for eof_first in [false, true] {
                        let mut session = session(mode);
                        if eof_first {
                            session
                                .observe_eof(Side::Viewer, Duration::from_secs(12))
                                .unwrap();
                        }
                        session.record_write(direction, 4).unwrap();
                        assert!(session.io_error(direction, failing_side).unwrap());
                        assert!(!session.io_error(direction, Side::Viewer).unwrap());
                        assert!(!session.dead_connection(other).unwrap());
                        assert!(!session.expire_close(Duration::MAX, Duration::ZERO).unwrap());
                        let closed = finish(&mut session);
                        assert_eq!(closed.close_cause(), CloseCause::ConnectionError);
                        assert_eq!(closed.close_side(), Some(failing_side));
                        assert_eq!(closed.bytes(direction), 4);
                    }
                }
            }
        }
    }

    /// spec: relay.md:RLY-020 partial
    #[test]
    fn ignores_errors_from_completed_directions_but_not_still_open_directions() {
        let mut session = session(Mode::Mode2);
        session
            .observe_eof(Side::Viewer, Duration::from_secs(12))
            .unwrap();
        session.complete_direction(FORWARD).unwrap();
        assert!(!session.io_error(FORWARD, Side::Server).unwrap());
        assert_eq!(session.close_cause(), None);
        assert!(session.io_error(REVERSE, Side::Viewer).unwrap());
        assert_eq!(
            finish(&mut session).close_cause(),
            CloseCause::ConnectionError
        );

        let mut session = self::session(Mode::Mode2);
        normal_complete(&mut session, Side::Viewer, Side::Server);
        assert!(!session.io_error(FORWARD, Side::Server).unwrap());
        assert!(!session.io_error(REVERSE, Side::Viewer).unwrap());
        assert_eq!(finish(&mut session).close_cause(), CloseCause::PeerEof);
    }

    /// spec: operations.md:OPS-009 partial
    #[test]
    fn forces_server_shutdown_only_when_no_cleanup_has_started() {
        for trigger in 0..7 {
            let mut session = session(Mode::Mode1);
            let (expected_cause, expected_side) = match trigger {
                0 => (CloseCause::ServerShutdown, None),
                1 => {
                    session
                        .observe_eof(Side::Viewer, Duration::from_secs(12))
                        .unwrap();
                    (CloseCause::PeerEof, Some(Side::Viewer))
                }
                2 => {
                    normal_complete(&mut session, Side::Target, Side::Viewer);
                    (CloseCause::PeerEof, Some(Side::Target))
                }
                3 => {
                    session.io_error(FORWARD, Side::Target).unwrap();
                    (CloseCause::ConnectionError, Some(Side::Target))
                }
                4 => {
                    session.dead_connection(Side::Target).unwrap();
                    (CloseCause::DeadConnection, Some(Side::Target))
                }
                5 => {
                    session.begin_admin_close(Duration::from_secs(12)).unwrap();
                    (CloseCause::AdminTerminated, None)
                }
                6 => {
                    session
                        .observe_eof(Side::Target, Duration::from_secs(12))
                        .unwrap();
                    session
                        .expire_close(Duration::from_secs(12), Duration::ZERO)
                        .unwrap();
                    (CloseCause::CloseTimeout, Some(Side::Target))
                }
                _ => unreachable!(),
            };
            let forced = session.force_server_shutdown().unwrap();
            assert_eq!(forced, trigger == 0);
            assert!(!session.force_server_shutdown().unwrap());
            assert!(!session.io_error(FORWARD, Side::Viewer).unwrap());
            assert!(!session.begin_admin_close(Duration::from_secs(20)).unwrap());
            assert!(!session.expire_close(Duration::ZERO, Duration::MAX).unwrap());
            let closed = finish(&mut session);
            assert_eq!(closed.close_cause(), expected_cause);
            assert_eq!(closed.close_side(), expected_side);
            assert!(!session.force_server_shutdown().unwrap());
            assert_eq!(session.close_cause(), Some(expected_cause));
        }
    }

    /// spec: relay.md:RLY-020 partial
    /// spec: management.md:MGT-023 partial
    #[test]
    fn distinguishes_admin_cleanup_and_admin_timeout_without_overriding_prior_closing() {
        for expires in [false, true] {
            let mut session = session(Mode::Mode2);
            assert!(session.begin_admin_close(Duration::from_secs(12)).unwrap());
            assert!(!session.begin_admin_close(Duration::from_secs(20)).unwrap());
            session
                .observe_eof(Side::Viewer, Duration::from_secs(14))
                .unwrap();
            assert!(!session.io_error(REVERSE, Side::Server).unwrap());
            assert!(
                !session
                    .expire_close(Duration::from_secs(16), Duration::from_secs(5))
                    .unwrap()
            );
            if expires {
                assert!(
                    session
                        .expire_close(Duration::from_secs(17), Duration::from_secs(5))
                        .unwrap()
                );
            }
            let closed = finish(&mut session);
            assert_eq!(
                closed.close_cause(),
                if expires {
                    CloseCause::CloseTimeout
                } else {
                    CloseCause::AdminTerminated
                }
            );
            assert_eq!(closed.close_side(), None);
        }
        let mut session = session(Mode::Mode2);
        session
            .observe_eof(Side::Server, Duration::from_secs(12))
            .unwrap();
        assert!(!session.begin_admin_close(Duration::from_secs(13)).unwrap());
        assert_eq!(session.close_cause(), None);
    }

    #[test]
    fn accepts_external_dead_connection_and_rejects_mode_incompatible_sides() {
        for (mode, other, invalid) in [
            (Mode::Mode1, Side::Target, Side::Server),
            (Mode::Mode2, Side::Server, Side::Target),
        ] {
            for side in [Side::Viewer, other] {
                for eof_first in [false, true] {
                    let mut session = session(mode);
                    assert_eq!(
                        session.observe_eof(invalid, Duration::ZERO),
                        Err(SessionError::InvalidSide)
                    );
                    assert_eq!(
                        session.io_error(FORWARD, invalid),
                        Err(SessionError::InvalidSide)
                    );
                    assert_eq!(
                        session.dead_connection(invalid),
                        Err(SessionError::InvalidSide)
                    );
                    assert_eq!(session.state(), SessionState::Active);
                    if eof_first {
                        session.observe_eof(other, Duration::from_secs(12)).unwrap();
                    }
                    assert!(session.dead_connection(side).unwrap());
                    assert!(!session.dead_connection(other).unwrap());
                    assert!(!session.io_error(REVERSE, other).unwrap());
                    let closed = finish(&mut session);
                    assert_eq!(closed.close_cause(), CloseCause::DeadConnection);
                    assert_eq!(closed.close_side(), Some(side));
                }
            }
        }
    }

    /// spec: management.md:MGT-005 partial
    #[test]
    fn counts_each_partial_success_in_its_direction_and_saturates_independently() {
        let mut session = session(Mode::Mode1);
        for bytes in [3, 0, 4, 2] {
            session.record_write(FORWARD, bytes).unwrap();
        }
        session.record_write(REVERSE, 17).unwrap();
        assert_eq!(session.bytes(FORWARD), 9);
        assert_eq!(session.bytes(REVERSE), 17);
        session.record_write(FORWARD, u64::MAX - 10).unwrap();
        assert_eq!(session.bytes(FORWARD), u64::MAX - 1);
        session.record_write(FORWARD, 2).unwrap();
        session.record_write(FORWARD, u64::MAX).unwrap();
        assert_eq!(session.bytes(FORWARD), u64::MAX);
        assert_eq!(session.bytes(REVERSE), 17);
        session.record_write(REVERSE, u64::MAX).unwrap();
        session.io_error(REVERSE, Side::Target).unwrap();
        let closed = finish(&mut session);
        assert_eq!(closed.bytes(FORWARD), u64::MAX);
        assert_eq!(closed.bytes(REVERSE), u64::MAX);
    }

    /// spec: relay.md:RLY-016 partial
    /// spec: management.md:MGT-007 partial
    #[test]
    fn starts_fresh_state_and_clamps_wall_time_without_using_it_for_elapsed_time() {
        let mut old = session(Mode::Mode1);
        old.record_write(FORWARD, 99).unwrap();
        old.force_server_shutdown().unwrap();
        let closed = old
            .finish(SystemTime::UNIX_EPOCH, Duration::from_secs(17))
            .unwrap();
        assert_eq!(closed.ended_at(), closed.started_at());
        assert_eq!(closed.duration(), Duration::from_secs(7));
        assert_eq!(closed.id(), old.id());
        assert_eq!(closed.mode(), old.mode());
        assert_eq!(closed.viewer_transport(), old.viewer_transport());
        let mut fresh = Session::new(
            SessionId::from_block([2; 16]),
            Mode::Mode2,
            ViewerTransport::Wss,
            SystemTime::UNIX_EPOCH,
            Duration::from_secs(40),
        );
        assert_ne!(fresh.id(), old.id());
        assert_eq!(fresh.state(), SessionState::Active);
        assert_eq!(fresh.bytes(FORWARD), 0);
        assert_eq!(fresh.bytes(REVERSE), 0);
        assert_eq!(fresh.close_cause(), None);
        assert_eq!(fresh.first_eof(), None);
        assert_eq!(fresh.duration(Duration::from_secs(39)), Duration::ZERO);
        assert_eq!(fresh.started_at(), SystemTime::UNIX_EPOCH);
        fresh.force_server_shutdown().unwrap();
        let closed = fresh
            .finish(
                SystemTime::UNIX_EPOCH + Duration::from_secs(9),
                Duration::from_secs(39),
            )
            .unwrap();
        assert_eq!(closed.duration(), Duration::ZERO);
        assert_eq!(
            closed.ended_at(),
            SystemTime::UNIX_EPOCH + Duration::from_secs(9)
        );
        assert_eq!(closed.mode(), Mode::Mode2);
        assert_eq!(closed.viewer_transport(), ViewerTransport::Wss);
    }

    /// spec: relay.md:RLY-020 partial
    #[test]
    fn finalizes_exactly_once_and_rejects_all_later_mutations() {
        let mut session = session(Mode::Mode2);
        normal_complete(&mut session, Side::Viewer, Side::Server);
        let closed = finish(&mut session);
        assert_eq!(closed.close_cause(), CloseCause::PeerEof);
        assert_eq!(session.close_cause(), Some(CloseCause::PeerEof));
        assert_eq!(session.close_side(), Some(Side::Viewer));
        assert_eq!(
            session.finish(SystemTime::UNIX_EPOCH, Duration::ZERO),
            Err(SessionError::Finished)
        );
        assert_eq!(
            session.record_write(FORWARD, 1),
            Err(SessionError::Finished)
        );
        assert_eq!(
            session.observe_eof(Side::Viewer, Duration::ZERO),
            Err(SessionError::Finished)
        );
        assert_eq!(
            session.complete_direction(FORWARD),
            Err(SessionError::Finished)
        );
        assert_eq!(
            session.io_error(FORWARD, Side::Viewer),
            Err(SessionError::Finished)
        );
        assert_eq!(
            session.dead_connection(Side::Viewer),
            Err(SessionError::Finished)
        );
        assert_eq!(
            session.begin_admin_close(Duration::ZERO),
            Err(SessionError::Finished)
        );
        assert_eq!(session.force_server_shutdown(), Ok(false));
        assert_eq!(
            session.expire_close(Duration::MAX, Duration::ZERO),
            Err(SessionError::Finished)
        );
        assert!(session.direction_completed(FORWARD));
        assert!(session.direction_completed(REVERSE));
    }
}
