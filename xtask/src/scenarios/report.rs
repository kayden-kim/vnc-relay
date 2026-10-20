//! Scenario statuses and the Markdown report.

use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use super::test_run::{Outcome, TEST_COMMAND};

/// A marker or spec problem. Reported as `오류`.
pub struct Problem {
    pub location: String,
    pub detail: String,
}

pub struct TestEntry {
    pub file: String,
    pub name: String,
    /// `None` when the test did not appear in the `cargo test` output, for
    /// example when `#[cfg]` removed it on this OS.
    pub outcome: Option<Outcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Passed,
    Failed,
    NotRun,
    Unlinked,
}

impl Status {
    pub fn of(tests: &[TestEntry]) -> Self {
        let has = |outcome| tests.iter().any(|test| test.outcome == Some(outcome));
        if tests.is_empty() {
            Self::Unlinked
        } else if has(Outcome::Failed) {
            Self::Failed
        } else if has(Outcome::Passed) {
            Self::Passed
        } else {
            Self::NotRun
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Passed => "통과",
            Self::Failed => "실패",
            Self::NotRun => "미실행",
            Self::Unlinked => "미연결",
        }
    }
}

pub struct ScenarioRow {
    pub spec: String,
    pub id: String,
    pub status: Status,
    pub tests: Vec<TestEntry>,
}

pub struct Report {
    commit: String,
    rows: Vec<ScenarioRow>,
    problems: Vec<Problem>,
    tests_succeeded: bool,
    require_all: bool,
}

impl Report {
    pub fn new(
        root: &Path,
        rows: Vec<ScenarioRow>,
        problems: Vec<Problem>,
        tests_succeeded: bool,
        require_all: bool,
    ) -> Self {
        Self {
            commit: commit(root),
            rows,
            problems,
            tests_succeeded,
            require_all,
        }
    }

    fn count(&self, status: Status) -> usize {
        self.rows.iter().filter(|row| row.status == status).count()
    }

    /// Reasons the verdict is `실패`; empty when it is `통과`.
    fn failures(&self) -> Vec<String> {
        let mut reasons = Vec::new();
        if !self.problems.is_empty() {
            reasons.push(format!("오류 {}건", self.problems.len()));
        }
        if self.count(Status::Failed) > 0 {
            reasons.push(format!("실패 시나리오 {}건", self.count(Status::Failed)));
        }
        if !self.tests_succeeded {
            reasons.push("`cargo test` 실패".to_owned());
        }
        if self.require_all {
            let open = self.count(Status::NotRun) + self.count(Status::Unlinked);
            if open > 0 {
                reasons.push(format!("--require-all: 미실행·미연결 시나리오 {open}건"));
            }
        }
        reasons
    }

    pub fn passed(&self) -> bool {
        self.failures().is_empty()
    }

    pub fn verdict_text(&self) -> String {
        let failures = self.failures();
        if failures.is_empty() {
            "통과".to_owned()
        } else {
            format!("실패 ({})", failures.join(", "))
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "시나리오 {}건: 통과 {}, 실패 {}, 미실행 {}, 미연결 {}. 오류 {}건.",
            self.rows.len(),
            self.count(Status::Passed),
            self.count(Status::Failed),
            self.count(Status::NotRun),
            self.count(Status::Unlinked),
            self.problems.len()
        )
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        let command = if self.require_all {
            "cargo xtask scenarios --require-all"
        } else {
            "cargo xtask scenarios"
        };
        let _ = writeln!(out, "# 시나리오 커버리지 보고서\n");
        let _ = writeln!(out, "| 항목 | 값 |\n| --- | --- |");
        let _ = writeln!(out, "| 생성 시각(UTC) | {} |", utc_now());
        let _ = writeln!(out, "| 대상 커밋 | {} |", self.commit);
        let _ = writeln!(
            out,
            "| 환경 | {} {}, {} |",
            std::env::consts::OS,
            std::env::consts::ARCH,
            rustc_version()
        );
        let _ = writeln!(out, "| 명령 | `{command}` (`{TEST_COMMAND}` 포함) |");
        let _ = writeln!(
            out,
            "| `cargo test` 종료 | {} |",
            if self.tests_succeeded {
                "통과"
            } else {
                "실패"
            }
        );
        let _ = writeln!(out, "| 판정 | {} |\n", self.verdict_text());

        let _ = writeln!(out, "## 요약\n");
        let _ = writeln!(
            out,
            "| 스펙 | 시나리오 | 통과 | 실패 | 미실행 | 미연결 |\n| --- | ---: | ---: | ---: | ---: | ---: |"
        );
        let mut specs: Vec<&str> = self.rows.iter().map(|row| row.spec.as_str()).collect();
        specs.dedup();
        for spec in &specs {
            let rows: Vec<&ScenarioRow> = self.rows.iter().filter(|r| r.spec == *spec).collect();
            let count = |status| rows.iter().filter(|r| r.status == status).count();
            let _ = writeln!(
                out,
                "| {spec} | {} | {} | {} | {} | {} |",
                rows.len(),
                count(Status::Passed),
                count(Status::Failed),
                count(Status::NotRun),
                count(Status::Unlinked)
            );
        }
        let _ = writeln!(
            out,
            "| 합계 | {} | {} | {} | {} | {} |\n",
            self.rows.len(),
            self.count(Status::Passed),
            self.count(Status::Failed),
            self.count(Status::NotRun),
            self.count(Status::Unlinked)
        );

        let _ = writeln!(out, "## 오류\n");
        if self.problems.is_empty() {
            let _ = writeln!(out, "없음.\n");
        } else {
            let _ = writeln!(out, "| 위치 | 내용 |\n| --- | --- |");
            for problem in &self.problems {
                let _ = writeln!(
                    out,
                    "| `{}` | {} |",
                    problem.location,
                    cell(&problem.detail)
                );
            }
            let _ = writeln!(out);
        }

        let _ = writeln!(out, "## 시나리오");
        for spec in &specs {
            let _ = writeln!(out, "\n### {spec}\n");
            let _ = writeln!(out, "| 시나리오 ID | 상태 | 테스트 |\n| --- | --- | --- |");
            for row in self.rows.iter().filter(|r| r.spec == *spec) {
                let tests: Vec<String> = row
                    .tests
                    .iter()
                    .map(|test| {
                        let outcome = match test.outcome {
                            Some(Outcome::Passed) => "통과",
                            Some(Outcome::Failed) => "실패",
                            Some(Outcome::Ignored) => "미실행(무시됨)",
                            None => "미실행(이번 실행에 없음)",
                        };
                        format!("`{}` `{}` {outcome}", test.file, test.name)
                    })
                    .collect();
                let _ = writeln!(
                    out,
                    "| {} | {} | {} |",
                    row.id,
                    row.status.label(),
                    tests.join("<br>")
                );
            }
        }
        out
    }
}

fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}

fn commit(root: &Path) -> String {
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    match (
        git(&["rev-parse", "--short", "HEAD"]),
        git(&["status", "--porcelain"]),
    ) {
        (Some(hash), Some(changes)) if changes.is_empty() => format!("`{hash}`"),
        (Some(hash), Some(_)) => format!("`{hash}` + 미커밋 변경"),
        _ => "확인 불가".to_owned(),
    }
}

fn rustc_version() -> String {
    Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map_or_else(
            || "rustc 확인 불가".to_owned(),
            |output| String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        )
}

fn utc_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let (year, month, day) = civil_from_days(seconds / 86_400);
    let time = seconds % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        time / 3600,
        time % 3600 / 60,
        time % 60
    )
}

/// Gregorian date from days since 1970-01-01 (Howard Hinnant's algorithm).
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z % 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(outcome: Option<Outcome>) -> TestEntry {
        TestEntry {
            file: "f.rs".to_owned(),
            name: "t".to_owned(),
            outcome,
        }
    }

    #[test]
    fn status_prefers_failure_then_pass_then_not_run() {
        use Outcome::{Failed, Ignored, Passed};
        assert_eq!(Status::of(&[]), Status::Unlinked);
        assert_eq!(
            Status::of(&[entry(Some(Passed)), entry(Some(Failed))]),
            Status::Failed
        );
        assert_eq!(
            Status::of(&[entry(Some(Ignored)), entry(Some(Passed))]),
            Status::Passed
        );
        assert_eq!(
            Status::of(&[entry(Some(Ignored)), entry(None)]),
            Status::NotRun
        );
    }

    #[test]
    fn converts_days_to_dates_across_leap_years() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(civil_from_days(20_746), (2026, 10, 20));
    }
}
