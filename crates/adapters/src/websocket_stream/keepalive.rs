//! Fixed-cadence empty Pings and bounded oldest-first Pong deadlines (Q84).

use std::pin::Pin;
use std::task::Context;
use std::time::Duration;

use tokio::time::{Instant, Sleep, sleep_until};
use vnc_relay_app::settings::KeepaliveSettings;

// ceil(300 / 5) + 1, from config-v3 §1.7. No per-Ping allocation.
const MAX_DEADLINES: usize = 61;

pub(super) struct Keepalive {
    interval: Duration,
    timeout: Duration,
    next_ping: Instant,
    deadlines: [Option<Instant>; MAX_DEADLINES],
    head: usize,
    len: usize,
    limit: usize,
    timer: Pin<Box<Sleep>>,
    pub(super) pending_ping: Option<u8>,
    pub(super) expired: bool,
}

impl Keepalive {
    pub(super) fn new(settings: KeepaliveSettings) -> Self {
        let next_ping = Instant::now() + settings.ws_ping;
        Self {
            interval: settings.ws_ping,
            timeout: settings.ws_pong_timeout,
            next_ping,
            deadlines: [None; MAX_DEADLINES],
            head: 0,
            len: 0,
            limit: settings
                .ws_pong_timeout
                .as_nanos()
                .div_ceil(settings.ws_ping.as_nanos()) as usize
                + 1,
            timer: Box::pin(sleep_until(next_ping)),
            pending_ping: None,
            expired: false,
        }
    }

    pub(super) fn pong(&mut self) {
        if self.len > 0 {
            self.deadlines[self.head] = None;
            self.head = (self.head + 1) % self.limit;
            self.len -= 1;
        }
    }

    pub(super) fn sent(&mut self, now: Instant) {
        // Expiry is checked before output; the configured limit leaves one slot
        // for an equal-deadline tick. Never grow or overwrite a pending deadline.
        self.deadlines[(self.head + self.len) % self.limit] = Some(now + self.timeout);
        self.len += 1;
        self.pending_ping = None;
    }

    pub(super) fn poll(&mut self, cx: &mut Context<'_>) {
        let now = Instant::now();
        if self.deadlines[self.head].is_some_and(|deadline| now >= deadline) {
            self.expired = true;
            return;
        }
        if now >= self.next_ping {
            if self.pending_ping.is_none() {
                self.pending_ping = Some(0);
            }
            // Skip missed slots without moving the cadence to traffic/Pong time.
            let remainder =
                now.duration_since(self.next_ping).as_nanos() % self.interval.as_nanos();
            self.next_ping = now + self.interval - Duration::from_nanos(remainder as u64);
        }
        let wake =
            self.deadlines[self.head].map_or(self.next_ping, |oldest| oldest.min(self.next_ping));
        if self.timer.deadline() != wake {
            self.timer.as_mut().reset(wake);
        }
        let _ = self.timer.as_mut().poll(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::task::Waker;

    fn settings(ping: u64, pong: u64) -> KeepaliveSettings {
        KeepaliveSettings {
            tcp_idle: Duration::from_secs(60),
            tcp_interval: Duration::from_secs(10),
            tcp_probes: 6,
            ws_ping: Duration::from_secs(ping),
            ws_pong_timeout: Duration::from_secs(pong),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn fixed_ticks_bound_fifo_and_pong_removes_only_oldest() {
        let mut kw = Keepalive::new(settings(5, 16));
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        assert_eq!(kw.limit, 5);
        kw.pong();
        assert_eq!(kw.len, 0, "unsolicited Pong does nothing");
        let epoch = Instant::now();
        for tick in 1..=3 {
            tokio::time::advance(Duration::from_secs(5)).await;
            kw.poll(&mut cx);
            assert_eq!(kw.pending_ping, Some(0));
            kw.sent(Instant::now());
            assert_eq!(kw.len, tick);
            assert!(kw.len <= kw.limit);
        }
        assert_eq!(kw.deadlines[kw.head], Some(epoch + Duration::from_secs(21)));
        kw.pong();
        assert_eq!(kw.len, 2);
        assert_eq!(kw.deadlines[kw.head], Some(epoch + Duration::from_secs(26)));
        tokio::time::advance(Duration::from_secs(5)).await;
        kw.poll(&mut cx);
        kw.sent(Instant::now());
        assert_eq!(kw.next_ping, epoch + Duration::from_secs(25));
        tokio::time::advance(Duration::from_secs(6)).await;
        kw.poll(&mut cx);
        assert!(
            kw.expired,
            "second Ping's original deadline is not extended"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn prompt_pongs_keep_idle_connections_alive_and_wrap_the_bounded_fifo() {
        let mut kw = Keepalive::new(settings(5, 300));
        let mut cx = Context::from_waker(Waker::noop());
        assert_eq!(kw.limit, MAX_DEADLINES);
        for _ in 0..150 {
            tokio::time::advance(Duration::from_secs(5)).await;
            kw.poll(&mut cx);
            assert!(!kw.expired);
            kw.sent(Instant::now());
            kw.pong();
            assert_eq!(kw.len, 0);
        }
        // With no Pong, expiry wins at equality with the next scheduled tick.
        kw.sent(Instant::now());
        tokio::time::advance(Duration::from_secs(300)).await;
        kw.poll(&mut cx);
        assert!(kw.expired);
    }
}

#[cfg(test)]
mod capacity_tests {
    use super::*;
    use std::task::Waker;

    #[tokio::test(start_paused = true)]
    async fn maximum_configured_fifo_expires_before_it_can_grow_past_its_bound() {
        let mut kw = Keepalive::new(KeepaliveSettings {
            tcp_idle: Duration::from_secs(60),
            tcp_interval: Duration::from_secs(10),
            tcp_probes: 6,
            ws_ping: Duration::from_secs(5),
            ws_pong_timeout: Duration::from_secs(300),
        });
        let mut cx = Context::from_waker(Waker::noop());
        for count in 1..=60 {
            tokio::time::advance(Duration::from_secs(5)).await;
            kw.poll(&mut cx);
            assert!(!kw.expired);
            kw.sent(Instant::now());
            assert_eq!(kw.len, count);
            assert!(kw.len < kw.limit);
        }
        tokio::time::advance(Duration::from_secs(5)).await;
        kw.poll(&mut cx);
        assert!(kw.expired);
        assert_eq!(kw.len, 60);
    }
}
