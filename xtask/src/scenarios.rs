//! `cargo xtask scenarios`: scenario coverage and results report
//! (docs/conventions/testing.md section 7).

mod markers;
mod report;
mod spec;
mod test_run;

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::Error;
use crate::cargo::{self, Package, Workspace};
use markers::Marker;
use report::{Problem, Report, ScenarioRow, Status, TestEntry};
use spec::Spec;
use test_run::{TestKey, TestRun};

/// The development tool's own tests carry no scenario markers.
const SKIPPED_PACKAGE: &str = "xtask";

const REPORT_FILE: &str = "scenario-report.md";

pub fn run(require_all: bool) -> Result<bool, Error> {
    let workspace = cargo::metadata()?;
    let mut problems = Vec::new();
    let specs = load_specs(&workspace.root, &mut problems)?;
    let tests = collect_marked_tests(&workspace, &mut problems)?;
    let run = test_run::run(&workspace)?;
    let rows = link(&specs, &tests, &run, &mut problems);

    let report = Report::new(&workspace.root, rows, problems, run.success, require_all);
    std::fs::create_dir_all(&workspace.target_dir)?;
    let path = workspace.target_dir.join(REPORT_FILE);
    std::fs::write(&path, report.render())?;

    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{}", report.summary_line())?;
    writeln!(stdout, "보고서: {}", path.display())?;
    writeln!(stdout, "판정: {}", report.verdict_text())?;
    Ok(report.passed())
}

fn load_specs(root: &Path, problems: &mut Vec<Problem>) -> Result<Vec<Spec>, Error> {
    let dir = root.join("docs").join("specs");
    let mut specs = Vec::new();
    for path in files_with_extension(&dir, "md")? {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name == "README.md" || name == "AGENTS.md" {
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        let (scenarios, spec_problems) = spec::parse(&text);
        let location = cargo::display_path(root, &path);
        problems.extend(spec_problems.into_iter().map(|detail| Problem {
            location: location.clone(),
            detail,
        }));
        specs.push(Spec {
            path: cargo::display_path(&dir, &path),
            scenarios,
        });
    }
    Ok(specs)
}

struct LinkedTest {
    /// `<file>` and `<module path>::<fn>` for the report.
    file: String,
    name: String,
    key: TestKey,
    markers: Vec<Marker>,
}

fn collect_marked_tests(
    workspace: &Workspace,
    problems: &mut Vec<Problem>,
) -> Result<Vec<LinkedTest>, Error> {
    let mut linked = Vec::new();
    for package in &workspace.packages {
        if package.name == SKIPPED_PACKAGE {
            continue;
        }
        for dir in ["src", "tests"] {
            for path in files_with_extension(&package.dir.join(dir), "rs")? {
                let source = std::fs::read_to_string(&path)?;
                let scan = markers::scan(&source);
                let file = cargo::display_path(&workspace.root, &path);
                problems.extend(scan.bad.into_iter().map(|bad| Problem {
                    location: format!("{file}:{}", bad.line),
                    detail: format!("`{}`: {}", bad.text, bad.reason),
                }));
                if scan.tests.is_empty() {
                    continue;
                }
                let Some((target, file_modules)) = locate(package, &path) else {
                    problems.push(Problem {
                        location: file.clone(),
                        detail: "파일이 속한 테스트 대상(lib·bin·tests/*.rs)을 정할 수 없다."
                            .to_owned(),
                    });
                    continue;
                };
                for test in scan.tests {
                    let name = file_modules
                        .iter()
                        .chain(&test.modules)
                        .chain(std::iter::once(&test.name))
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .join("::");
                    linked.push(LinkedTest {
                        file: file.clone(),
                        key: (target.clone(), name.clone()),
                        name,
                        markers: test.markers,
                    });
                }
            }
        }
    }
    Ok(linked)
}

/// Finds the test target whose module tree contains `file` and the module
/// path of `file` inside it.
fn locate(package: &Package, file: &Path) -> Option<(PathBuf, Vec<String>)> {
    let tested = || package.targets.iter().filter(|target| target.is_tested());
    if let Some(target) = tested().find(|target| target.src_path == file) {
        return Some((target.src_path.clone(), Vec::new()));
    }
    let enclosing: Vec<_> = tested()
        .filter_map(|target| {
            let dir = target.src_path.parent()?;
            file.starts_with(dir)
                .then(|| (dir.components().count(), target))
        })
        .collect();
    let deepest = enclosing.iter().map(|(depth, _)| *depth).max()?;
    let candidates: Vec<_> = enclosing
        .iter()
        .filter(|(depth, _)| *depth == deepest)
        .map(|(_, target)| *target)
        .collect();
    let target = match candidates.as_slice() {
        [only] => *only,
        many => *many.iter().find(|target| target.is_lib())?,
    };
    let relative = file.strip_prefix(target.src_path.parent()?).ok()?;
    let mut modules: Vec<String> = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let file_name = modules.pop()?;
    let stem = file_name.strip_suffix(".rs")?;
    if stem != "mod" {
        modules.push(stem.to_owned());
    }
    Some((target.src_path.clone(), modules))
}

fn link(
    specs: &[Spec],
    tests: &[LinkedTest],
    run: &TestRun,
    problems: &mut Vec<Problem>,
) -> Vec<ScenarioRow> {
    let mut linked: BTreeMap<(&str, &str), Vec<TestEntry>> = BTreeMap::new();
    for test in tests {
        for marker in &test.markers {
            let known = specs.iter().find(|spec| spec.path == marker.spec);
            match known {
                None => problems.push(Problem {
                    location: format!("{}:{}", test.file, marker.line),
                    detail: format!("스펙 `{}`가 docs/specs/에 없다.", marker.spec),
                }),
                Some(spec) if !spec.scenarios.contains(&marker.id) => problems.push(Problem {
                    location: format!("{}:{}", test.file, marker.line),
                    detail: format!(
                        "`{}`의 수용 테스트 표에 시나리오 {}가 없다.",
                        marker.spec, marker.id
                    ),
                }),
                Some(spec) => {
                    let id = spec
                        .scenarios
                        .iter()
                        .find(|id| **id == marker.id)
                        .map_or("", String::as_str);
                    linked
                        .entry((spec.path.as_str(), id))
                        .or_default()
                        .push(TestEntry {
                            file: test.file.clone(),
                            name: test.name.clone(),
                            outcome: run.outcomes.get(&test.key).copied(),
                            partial: marker.partial,
                        });
                }
            }
        }
    }
    let mut rows = Vec::new();
    for spec in specs {
        for id in &spec.scenarios {
            let tests = linked
                .remove(&(spec.path.as_str(), id.as_str()))
                .unwrap_or_default();
            rows.push(ScenarioRow {
                spec: spec.path.clone(),
                id: id.clone(),
                status: Status::of(&tests),
                tests,
            });
        }
    }
    rows
}

/// Files under `dir` with the extension, sorted. A missing `dir` has none.
fn files_with_extension(dir: &Path, extension: &str) -> Result<Vec<PathBuf>, Error> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        for entry in entries {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|e| e == extension) {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo::Target;

    fn package(dir: &str, targets: &[(&str, &str)]) -> Package {
        Package {
            name: "p".to_owned(),
            dir: PathBuf::from(dir),
            targets: targets
                .iter()
                .map(|(kind, src)| Target {
                    kinds: vec![(*kind).to_owned()],
                    src_path: PathBuf::from(src),
                })
                .collect(),
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn locates_module_path_inside_test_target() {
        let package = package(
            "/w",
            &[
                ("lib", "/w/src/lib.rs"),
                ("bin", "/w/src/main.rs"),
                ("test", "/w/tests/relay.rs"),
                ("test", "/w/tests/admin.rs"),
                ("test", "/w/tests/multi/main.rs"),
            ],
        );
        let lib = PathBuf::from("/w/src/lib.rs");
        let modules = |list: &[&str]| list.iter().map(|m| (*m).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            locate(&package, Path::new("/w/src/lib.rs")),
            Some((lib.clone(), modules(&[])))
        );
        assert_eq!(
            locate(&package, Path::new("/w/src/relay/session.rs")),
            Some((lib.clone(), modules(&["relay", "session"])))
        );
        assert_eq!(
            locate(&package, Path::new("/w/src/relay/mod.rs")),
            Some((lib, modules(&["relay"])))
        );
        assert_eq!(
            locate(&package, Path::new("/w/tests/multi/case.rs")),
            Some((PathBuf::from("/w/tests/multi/main.rs"), modules(&["case"])))
        );
        assert_eq!(locate(&package, Path::new("/w/tests/common/mod.rs")), None);
    }
}
