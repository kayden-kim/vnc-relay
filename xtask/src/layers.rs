//! `cargo xtask layers`: checks crate dependencies against the layer table in
//! docs/architectures/code-map.md section 1.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

use crate::Error;
use crate::cargo::{self, Package};

/// External crates adopted in the PRD "구현 기술" row (docs/requirements/PRD.md).
/// Change this list in the same commit as the PRD row.
const APPROVED: &[&str] = &[
    "argon2",
    "aws-lc-rs",
    "axum",
    "clap",
    "notify",
    "rpassword",
    "rustls",
    "rustls-pki-types",
    "serde",
    "serde_json",
    "socket2",
    "tokio",
    "tokio-rustls",
    "tokio-tungstenite",
    "toml",
    "toml_edit",
    "windows-sys",
];

enum External {
    None,
    Only(&'static [&'static str]),
    Approved,
}

struct Rule {
    package: &'static str,
    internal: &'static [&'static str],
    external: External,
}

/// One row per layer in the code map. A new workspace package needs a row here
/// and in the code map.
const RULES: &[Rule] = &[
    Rule {
        package: "vnc-relay-domain",
        internal: &[],
        external: External::None,
    },
    Rule {
        package: "vnc-relay-app",
        internal: &["vnc-relay-domain"],
        external: External::Only(&["tokio"]),
    },
    Rule {
        package: "vnc-relay-adapters",
        internal: &["vnc-relay-domain", "vnc-relay-app"],
        external: External::Approved,
    },
    Rule {
        package: "vnc-relay",
        internal: &["vnc-relay-domain", "vnc-relay-app", "vnc-relay-adapters"],
        // Browser CDP reuses the approved JSON and WebSocket crates only in
        // tests (Q112). rustls and windows-sys are also dev-dependencies.
        external: External::Only(&[
            "clap",
            "rustls",
            "serde_json",
            "tokio",
            "tokio-tungstenite",
            "windows-sys",
        ]),
    },
    Rule {
        package: "xtask",
        internal: &[],
        external: External::Approved,
    },
];

pub fn run() -> Result<bool, Error> {
    let workspace = cargo::metadata()?;
    let violations = check(&workspace.packages);
    let mut stdout = std::io::stdout().lock();
    for violation in &violations {
        writeln!(stdout, "위반: {violation}")?;
    }
    if violations.is_empty() {
        writeln!(
            stdout,
            "레이어 검사: 통과 (패키지 {}개)",
            workspace.packages.len()
        )?;
    } else {
        writeln!(stdout, "레이어 검사: 실패 (위반 {}건)", violations.len())?;
    }
    Ok(violations.is_empty())
}

fn check(packages: &[Package]) -> Vec<String> {
    let members: BTreeSet<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    let mut violations = Vec::new();
    let mut requirements: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for package in packages {
        let Some(rule) = RULES.iter().find(|rule| rule.package == package.name) else {
            violations.push(format!(
                "{}: 코드 지도에 없는 패키지다. 코드 지도의 레이어 표와 xtask/src/layers.rs의 RULES를 함께 고친다.",
                package.name
            ));
            continue;
        };
        for dependency in &package.dependencies {
            let name = dependency.name.as_str();
            let allowed = if members.contains(name) {
                rule.internal.contains(&name)
            } else {
                requirements
                    .entry(name)
                    .or_default()
                    .insert(dependency.req.as_str());
                match rule.external {
                    External::None => false,
                    External::Only(list) => list.contains(&name),
                    External::Approved => APPROVED.contains(&name),
                }
            };
            if !allowed {
                violations.push(format!(
                    "{} → {} ({}): 코드 지도가 허용하지 않는 의존이다.",
                    package.name, name, dependency.kind
                ));
            }
        }
    }
    for (name, reqs) in requirements {
        if reqs.len() > 1 {
            let reqs: Vec<&str> = reqs.into_iter().collect();
            violations.push(format!(
                "{name}: 패키지마다 버전 요구가 다르다({}). [workspace.dependencies]에 한 번만 적는다.",
                reqs.join(", ")
            ));
        }
    }
    violations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo::Dependency;
    use std::path::PathBuf;

    fn package(name: &str, dependencies: &[(&str, &str)]) -> Package {
        Package {
            name: name.to_owned(),
            version: "0.0.0".to_owned(),
            dir: PathBuf::new(),
            targets: Vec::new(),
            dependencies: dependencies
                .iter()
                .map(|(name, req)| Dependency {
                    name: (*name).to_owned(),
                    req: (*req).to_owned(),
                    kind: "normal".to_owned(),
                })
                .collect(),
        }
    }

    #[test]
    fn rejects_dependency_pointing_up_the_layers() {
        let packages = [
            package("vnc-relay-domain", &[("vnc-relay-app", "*")]),
            package("vnc-relay-app", &[]),
        ];
        let violations = check(&packages);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].starts_with("vnc-relay-domain → vnc-relay-app"));
    }

    #[test]
    fn rejects_unapproved_external_crate_and_unknown_package() {
        let packages = [
            package("vnc-relay-adapters", &[("tokio", "1"), ("anyhow", "1")]),
            package("vnc-relay-extra", &[]),
        ];
        let violations = check(&packages);
        assert_eq!(violations.len(), 2);
        assert!(violations[0].starts_with("vnc-relay-adapters → anyhow"));
        assert!(violations[1].starts_with("vnc-relay-extra:"));
    }

    #[test]
    fn rejects_different_version_requirements_for_one_crate() {
        let packages = [
            package("vnc-relay-app", &[("tokio", "1.54")]),
            package("vnc-relay-adapters", &[("tokio", "1.50")]),
        ];
        let violations = check(&packages);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].starts_with("tokio:"));
    }
}
