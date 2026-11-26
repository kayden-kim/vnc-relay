//! Terminal progress for long commands: one status line that is rewritten in
//! place while the command prints its normal output above it
//! (docs/architectures/xtask.md, section 4). Nothing is drawn unless stderr is
//! a terminal, so redirected output and CI logs stay plain.

use std::fmt::Write as _;
use std::io::{self, IsTerminal, Write};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// The status line stays within this many columns, so the classic 80-column
/// terminal never wraps it.
const MAX_COLUMNS: usize = 79;

/// How often the elapsed time is redrawn while no output arrives.
const TICK: Duration = Duration::from_secs(1);

/// True when stderr is a terminal and progress may be drawn.
pub fn is_interactive() -> bool {
    io::stderr().is_terminal()
}

/// `mm:ss`. Minutes keep counting past 59.
pub fn format_elapsed(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs();
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

/// Counts of one `cargo test` run.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TestCount {
    /// Number of tests listed by the test executables. `None` when a listing failed.
    pub total: Option<usize>,
    pub done: usize,
    pub failed: usize,
    /// Test target that is running now, as `cargo test` names it.
    pub current: String,
}

impl TestCount {
    /// The status text: `[5/6] 시험 412/1030 (40%) · 실패 0 · 08:12 · tests/a.rs`.
    fn render(&self, label: &str, elapsed: Duration) -> String {
        let mut line = String::new();
        if !label.is_empty() {
            line.push_str(label);
            line.push(' ');
        }
        // Writing to a `String` cannot fail.
        let _ = match self.total {
            Some(total) => {
                // A listing can undercount (for example generated tests); never show over 100%.
                let total = total.max(self.done);
                let percent = (self.done * 100).checked_div(total).unwrap_or(100);
                write!(line, "시험 {}/{total} ({percent}%)", self.done)
            }
            None => write!(line, "시험 {} 완료", self.done),
        };
        let _ = write!(
            line,
            " · 실패 {} · {}",
            self.failed,
            format_elapsed(elapsed)
        );
        const SEPARATOR: &str = " · ";
        if !self.current.is_empty() {
            let room = MAX_COLUMNS.saturating_sub(display_width(&line) + display_width(SEPARATOR));
            if let Some(name) = fit(&self.current, room) {
                line.push_str(SEPARATOR);
                line.push_str(&name);
            }
        }
        line
    }
}

/// Terminal columns of `text`. Hangul and other East Asian wide characters take
/// two columns; everything else takes one.
fn display_width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}

fn char_width(c: char) -> usize {
    match u32::from(c) {
        0x1100..=0x115F
        | 0x2E80..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE6F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6 => 2,
        _ => 1,
    }
}

/// `text` cut to at most `room` columns, with `…` for the cut part. `None`
/// when less than four columns remain, which is too little to show a name.
fn fit(text: &str, room: usize) -> Option<String> {
    if display_width(text) <= room {
        return Some(text.to_owned());
    }
    if room < 4 {
        return None;
    }
    let mut kept = String::new();
    let mut width = 0;
    for c in text.chars() {
        // One column stays free for the ellipsis.
        if width + char_width(c) + 1 > room {
            break;
        }
        kept.push(c);
        width += char_width(c);
    }
    kept.push('…');
    Some(kept)
}

/// Draw state. It builds the bytes to write and owns no I/O, so tests can
/// check the sequences.
struct Screen {
    label: String,
    tests: Option<TestCount>,
    started: Option<Instant>,
    /// Columns of the status line on screen, 0 when there is none.
    drawn: usize,
    /// False after a log line that did not end with a newline.
    at_line_start: bool,
}

impl Screen {
    fn new(label: &str) -> Self {
        Self {
            label: label.to_owned(),
            tests: None,
            started: None,
            drawn: 0,
            at_line_start: true,
        }
    }

    /// Bytes for one write: erase the status line, `text`, then the status
    /// line again below it.
    fn frame(&mut self, text: &[u8], elapsed: Duration) -> Vec<u8> {
        let mut out = Vec::with_capacity(text.len() + 2 * MAX_COLUMNS + 2);
        if self.drawn > 0 {
            out.push(b'\r');
            out.resize(out.len() + self.drawn, b' ');
            out.push(b'\r');
            self.drawn = 0;
        }
        out.extend_from_slice(text);
        if let Some(&last) = text.last() {
            self.at_line_start = last == b'\n';
        }
        if self.at_line_start
            && let Some(tests) = &self.tests
        {
            let status = tests.render(&self.label, elapsed);
            self.drawn = display_width(&status);
            out.extend_from_slice(status.as_bytes());
        }
        out
    }

    fn elapsed(&self) -> Duration {
        self.started
            .map(|started| started.elapsed())
            .unwrap_or_default()
    }
}

/// Status line of a command. Drawn only when stderr is a terminal; otherwise
/// every method passes output through or does nothing.
pub struct StatusLine {
    enabled: bool,
    screen: Arc<Mutex<Screen>>,
    stop: Option<Sender<()>>,
    ticker: Option<JoinHandle<()>>,
}

impl StatusLine {
    /// `label` starts the line, for example `[5/6]`. Empty for no label.
    pub fn new(label: &str) -> Self {
        let enabled = is_interactive();
        let screen = Arc::new(Mutex::new(Screen::new(label)));
        let (stop, ticker) = if enabled {
            let (stop, signal) = mpsc::channel();
            let shared = Arc::clone(&screen);
            let ticker = std::thread::spawn(move || {
                while let Err(RecvTimeoutError::Timeout) = signal.recv_timeout(TICK) {
                    // A failed redraw is not worth stopping the command.
                    let _ = write_frame(&shared, &[]);
                }
            });
            (Some(stop), Some(ticker))
        } else {
            (None, None)
        };
        Self {
            enabled,
            screen,
            stop,
            ticker,
        }
    }

    /// Starts the test phase. `total` is the number of listed tests, if known.
    pub fn begin_tests(&self, total: Option<usize>) {
        self.update(|screen| {
            screen.started = Some(Instant::now());
            screen.tests = Some(TestCount {
                total,
                ..TestCount::default()
            });
        });
    }

    /// Names the test target that runs now. `None` when it is not a counted one.
    pub fn set_current(&self, current: Option<&str>) {
        self.update(|screen| {
            if let Some(tests) = &mut screen.tests {
                current.unwrap_or_default().clone_into(&mut tests.current);
            }
        });
    }

    /// Counts one finished test. The next `log` call draws the new numbers.
    pub fn finish_test(&self, failed: bool) {
        self.update(|screen| {
            if let Some(tests) = &mut screen.tests {
                tests.done += 1;
                tests.failed += usize::from(failed);
            }
        });
    }

    /// Ends the test phase and erases the status line.
    pub fn end_tests(&self) {
        self.update(|screen| screen.tests = None);
        // Nothing to draw anymore; only erase.
        let _ = write_frame(&self.screen, &[]);
    }

    /// Writes output (whole lines) to stderr above the status line.
    pub fn log(&self, bytes: &[u8]) -> io::Result<()> {
        if self.enabled {
            write_frame(&self.screen, bytes)
        } else {
            io::stderr().lock().write_all(bytes)
        }
    }

    fn update(&self, change: impl FnOnce(&mut Screen)) {
        if self.enabled {
            change(&mut self.screen.lock().unwrap_or_else(PoisonError::into_inner));
        }
    }
}

impl Drop for StatusLine {
    fn drop(&mut self) {
        // Closing the channel ends the ticker thread.
        drop(self.stop.take());
        if let Some(ticker) = self.ticker.take() {
            let _ = ticker.join();
        }
        if self.enabled {
            self.end_tests();
        }
    }
}

fn write_frame(screen: &Mutex<Screen>, text: &[u8]) -> io::Result<()> {
    let mut screen = screen.lock().unwrap_or_else(PoisonError::into_inner);
    let elapsed = screen.elapsed();
    let frame = screen.frame(text, elapsed);
    io::stderr().lock().write_all(&frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(total: Option<usize>, done: usize, failed: usize, current: &str) -> TestCount {
        TestCount {
            total,
            done,
            failed,
            current: current.to_owned(),
        }
    }

    #[test]
    fn elapsed_is_minutes_and_seconds() {
        assert_eq!(format_elapsed(Duration::from_secs(0)), "00:00");
        assert_eq!(format_elapsed(Duration::from_secs(492)), "08:12");
        assert_eq!(format_elapsed(Duration::from_secs(3_905)), "65:05");
    }

    #[test]
    fn status_shows_done_total_percent_failed_and_target() {
        let line = count(Some(1030), 412, 0, "tests/dashboard_browser.rs")
            .render("[5/6]", Duration::from_secs(492));
        assert_eq!(
            line,
            "[5/6] 시험 412/1030 (40%) · 실패 0 · 08:12 · tests/dashboard_browser.rs"
        );
    }

    #[test]
    fn status_without_total_or_label_shows_done_only() {
        let line = count(None, 7, 2, "").render("", Duration::from_secs(5));
        assert_eq!(line, "시험 7 완료 · 실패 2 · 00:05");
    }

    #[test]
    fn status_never_exceeds_one_hundred_percent() {
        let line = count(Some(10), 12, 0, "").render("", Duration::ZERO);
        assert_eq!(line, "시험 12/12 (100%) · 실패 0 · 00:00");
        let line = count(Some(0), 0, 0, "").render("", Duration::ZERO);
        assert_eq!(line, "시험 0/0 (100%) · 실패 0 · 00:00");
    }

    #[test]
    fn status_fits_the_column_limit_and_cuts_the_target_name() {
        let long = "crates/adapters/src/management_http/session_api_scenarios_with_a_long_name.rs";
        let line = count(Some(1030), 412, 12, long).render("[5/6]", Duration::from_secs(492));
        assert!(display_width(&line) <= MAX_COLUMNS, "{line}");
        assert!(line.ends_with('…'), "{line}");
        assert!(line.starts_with("[5/6] 시험 412/1030 (40%) · 실패 12 · 08:12 · crates/"));
    }

    #[test]
    fn status_drops_the_target_when_no_room_is_left() {
        let line = count(Some(100), 1, 0, "tests/a.rs").render(&"가".repeat(30), Duration::ZERO);
        assert!(!line.contains("tests/a.rs"), "{line}");
    }

    #[test]
    fn hangul_takes_two_columns() {
        assert_eq!(display_width("시험 7"), 6);
        assert_eq!(display_width("abc"), 3);
        assert_eq!(fit("가나다라", 5).as_deref(), Some("가나…"));
        assert_eq!(fit("abcdef", 3), None);
    }

    fn screen_with_tests() -> Screen {
        let mut screen = Screen::new("[5/6]");
        screen.tests = Some(count(Some(10), 3, 0, ""));
        screen
    }

    #[test]
    fn frame_erases_the_old_line_and_redraws_below_the_log_line() {
        let mut screen = screen_with_tests();
        let first = screen.frame(b"test a ... ok\n", Duration::from_secs(1));
        let status = "[5/6] 시험 3/10 (30%) · 실패 0 · 00:01";
        assert_eq!(first, format!("test a ... ok\n{status}").into_bytes());
        assert_eq!(screen.drawn, display_width(status));

        let second = screen.frame(b"test b ... ok\n", Duration::from_secs(2));
        let erase = format!("\r{}\r", " ".repeat(display_width(status)));
        let status = "[5/6] 시험 3/10 (30%) · 실패 0 · 00:02";
        assert_eq!(
            second,
            format!("{erase}test b ... ok\n{status}").into_bytes()
        );
    }

    #[test]
    fn frame_with_no_text_only_redraws_the_line() {
        let mut screen = screen_with_tests();
        screen.frame(b"", Duration::from_secs(1));
        let tick = screen.frame(b"", Duration::from_secs(2));
        let width = display_width("[5/6] 시험 3/10 (30%) · 실패 0 · 00:01");
        let expected = format!(
            "\r{}\r[5/6] 시험 3/10 (30%) · 실패 0 · 00:02",
            " ".repeat(width)
        );
        assert_eq!(tick, expected.into_bytes());
    }

    #[test]
    fn frame_does_not_draw_after_a_partial_line() {
        let mut screen = screen_with_tests();
        let partial = screen.frame(b"running...", Duration::ZERO);
        assert_eq!(partial, b"running...");
        assert_eq!(screen.frame(b"", Duration::ZERO), b"");
        let end = screen.frame(b" done\n", Duration::ZERO);
        assert!(end.starts_with(b" done\n[5/6] "));
    }

    #[test]
    fn frame_without_tests_only_erases() {
        let mut screen = screen_with_tests();
        screen.frame(b"", Duration::ZERO);
        screen.tests = None;
        let erased = screen.frame(b"", Duration::ZERO);
        assert!(erased.starts_with(b"\r"));
        assert!(erased.ends_with(b"\r"));
        assert!(erased[1..erased.len() - 1].iter().all(|&b| b == b' '));
        assert_eq!(screen.drawn, 0);
    }
}
