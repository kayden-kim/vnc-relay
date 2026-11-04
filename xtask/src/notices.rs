//! Distribution notices for the product's normal and build dependency closure.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde_json::Value;

use crate::{Error, cargo};

const TARGETS: [&str; 2] = ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu"];
const OUTPUT: &str = "THIRD-PARTY-NOTICES.txt";

struct Package {
    name: String,
    version: String,
    license: String,
    dir: PathBuf,
    license_file: Option<PathBuf>,
}

pub fn run(check: bool) -> Result<bool, Error> {
    let metadata: Value = serde_json::from_str(&output(
        Path::new("."),
        &[
            "metadata",
            "--offline",
            "--locked",
            "--format-version",
            "1",
            "--no-deps",
        ],
    )?)?;
    let root = Path::new(field(&metadata, "workspace_root")?);
    let text = render(root)?;
    let path = root.join(OUTPUT);
    let passed = if check {
        let stored = normalized(&std::fs::read_to_string(&path)?);
        if stored != text {
            let stored_lines: Vec<_> = stored.lines().collect();
            let expected_lines: Vec<_> = text.lines().collect();
            let line = (0..stored_lines.len().max(expected_lines.len()))
                .find(|&line| stored_lines.get(line) != expected_lines.get(line));
            if let Some(line) = line {
                writeln!(
                    std::io::stderr().lock(),
                    "{OUTPUT}:{}: stored {:?}; expected {:?}",
                    line + 1,
                    stored_lines.get(line),
                    expected_lines.get(line)
                )?;
            }
        }
        stored == text
    } else {
        std::fs::write(&path, &text)?;
        true
    };
    writeln!(
        std::io::stdout().lock(),
        "notices{}: {} ({OUTPUT})",
        if check { " --check" } else { "" },
        if passed {
            "통과"
        } else {
            "실패; cargo xtask notices로 갱신한다"
        }
    )?;
    Ok(passed)
}

fn output(root: &Path, args: &[&str]) -> Result<String, Error> {
    let output = cargo::command()
        .current_dir(root)
        .args(args)
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(Error::new(format!("cargo {} failed", args.join(" "))));
    }
    String::from_utf8(output.stdout).map_err(|error| Error::new(error.to_string()))
}

fn render(root: &Path) -> Result<String, Error> {
    let mut selected = BTreeSet::new();
    let mut values = BTreeMap::new();
    let mut keys_by_id = BTreeMap::new();
    let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut builds: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for target in TARGETS {
        // Metadata filters target edges, whereas tree also includes host build
        // dependencies. Merge both supported platforms before joining them.
        let metadata: Value = serde_json::from_str(&output(
            root,
            &[
                "metadata",
                "--offline",
                "--locked",
                "--format-version",
                "1",
                "--filter-platform",
                target,
            ],
        )?)?;
        for value in metadata["packages"]
            .as_array()
            .ok_or_else(|| Error::new("cargo metadata has no packages"))?
        {
            keys_by_id.insert(
                field(value, "id")?.to_owned(),
                (
                    field(value, "name")?.to_owned(),
                    field(value, "version")?.to_owned(),
                ),
            );
            values.insert(
                (
                    field(value, "name")?.to_owned(),
                    field(value, "version")?.to_owned(),
                ),
                value.clone(),
            );
        }
        for node in metadata["resolve"]["nodes"]
            .as_array()
            .ok_or_else(|| Error::new("cargo metadata has no resolve nodes"))?
        {
            let id = field(node, "id")?;
            for dep in node["deps"]
                .as_array()
                .ok_or_else(|| Error::new("cargo metadata node has no deps"))?
            {
                let kinds = dep["dep_kinds"]
                    .as_array()
                    .ok_or_else(|| Error::new("cargo metadata dependency has no kinds"))?;
                let pkg = field(dep, "pkg")?;
                if kinds
                    .iter()
                    .any(|kind| kind["kind"].as_str() != Some("dev"))
                {
                    graph
                        .entry(id.to_owned())
                        .or_default()
                        .insert(pkg.to_owned());
                }
                if kinds
                    .iter()
                    .any(|kind| kind["kind"].as_str() == Some("build"))
                {
                    builds
                        .entry(id.to_owned())
                        .or_default()
                        .insert(pkg.to_owned());
                }
            }
        }
        let tree = output(
            root,
            &[
                "tree",
                "--offline",
                "--locked",
                "-p",
                "vnc-relay",
                "--target",
                target,
                "--edges",
                "normal,build",
                "--prefix",
                "none",
                "--format",
                "{p}",
                "--charset",
                "ascii",
            ],
        )?;
        selected.extend(tree_packages(&tree)?);
    }
    // Host-only build edges differ in `cargo tree` on Windows and Linux.
    // Add the metadata union's build closure so either host emits the same
    // conservative distribution notice, without including dev-only roots.
    let mut pending = Vec::new();
    for (id, key) in &keys_by_id {
        if selected.contains(key) {
            pending.extend(builds.get(id).into_iter().flatten().cloned());
        }
    }
    let mut visited = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !visited.insert(id.clone()) {
            continue;
        }
        let key = keys_by_id
            .get(&id)
            .ok_or_else(|| Error::new(format!("build dependency missing from metadata: {id}")))?;
        selected.insert(key.clone());
        pending.extend(graph.get(&id).into_iter().flatten().cloned());
    }
    let mut packages = Vec::new();
    for key in selected {
        let value = values.get(&key).ok_or_else(|| {
            Error::new(format!("cargo tree package missing from metadata: {key:?}"))
        })?;
        let (name, version) = key;
        if value["source"].is_null() {
            if !matches!(
                name.as_str(),
                "vnc-relay" | "vnc-relay-domain" | "vnc-relay-app" | "vnc-relay-adapters"
            ) {
                return Err(Error::new(format!("unreviewed local dependency: {name}")));
            }
            continue;
        }
        if !field(value, "source")?.starts_with("registry+") {
            return Err(Error::new(format!(
                "unreviewed non-registry dependency: {name}"
            )));
        }
        let manifest = Path::new(field(value, "manifest_path")?);
        let dir = manifest
            .parent()
            .ok_or_else(|| Error::new("manifest has no parent"))?;
        packages.push(Package {
            name,
            version,
            license: field(value, "license")?.to_owned(),
            dir: dir.to_owned(),
            license_file: value["license_file"].as_str().map(PathBuf::from),
        });
    }

    let mut text = String::from(
        "vnc-relay: third-party dependency notices\n\
         Generated by cargo xtask notices. Do not edit by hand.\n\
         Ship this entire file with either target's executable.\n\
         Scope: union of normal + build dependencies of package vnc-relay,\n\
         default product features; excludes dev-only dependencies and xtask.\n\
         Build tools and unused nested notices may be included conservatively.\n\
         License expressions are upstream metadata, not a new license grant.\n\
         All collected copyright/license/NOTICE texts are retained below.\n\
         The project's own MIT LICENSE is separate and is not supplied here.\n\n",
    );
    writeln!(text, "Targets: {}", TARGETS.join(", ")).map_err(format_error)?;
    writeln!(text, "Third-party crates: {}\n", packages.len()).map_err(format_error)?;
    for package in &packages {
        writeln!(
            text,
            "{} {} | {}",
            package.name, package.version, package.license
        )
        .map_err(format_error)?;
    }
    for package in &packages {
        let mut files = BTreeSet::new();
        collect_texts(&package.dir, &mut files)?;
        if let Some(file) = &package.license_file {
            let file = package.dir.join(file);
            if !file.starts_with(&package.dir)
                || file
                    .components()
                    .any(|part| part == std::path::Component::ParentDir)
            {
                return Err(Error::new("license_file escapes package directory"));
            }
            files.insert(file);
        }
        if files.is_empty() {
            return Err(Error::new(format!(
                "no license text for {} {}",
                package.name, package.version
            )));
        }
        writeln!(text, "\n================================================================\n{} {}\nLicense: {}\n================================================================",
            package.name, package.version, package.license).map_err(format_error)?;
        for file in files {
            writeln!(
                text,
                "\n--- {} ---",
                cargo::display_path(&package.dir, &file)
            )
            .map_err(format_error)?;
            text.push_str(&normalized(&std::fs::read_to_string(&file)?));
            if !text.ends_with('\n') {
                text.push('\n');
            }
        }
    }
    // Keep the complete input, not an unstable std hasher or a new hash crate.
    // Any lockfile change therefore invalidates --check, even for a dev dep.
    text.push_str("\n================================================================\nCargo.lock input snapshot (not a license)\n================================================================\n");
    text.push_str(&normalized(&std::fs::read_to_string(
        root.join("Cargo.lock"),
    )?));
    Ok(text)
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a str, Error> {
    value[key]
        .as_str()
        .filter(|text| !text.is_empty())
        .ok_or_else(|| Error::new(format!("cargo metadata has no {key}")))
}

fn tree_packages(tree: &str) -> Result<BTreeSet<(String, String)>, Error> {
    tree.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts
                .next()
                .ok_or_else(|| Error::new("missing tree package"))?;
            let version = parts
                .next()
                .and_then(|part| part.strip_prefix('v'))
                .ok_or_else(|| Error::new(format!("unexpected cargo tree line: {line}")))?;
            Ok((name.to_owned(), version.to_owned()))
        })
        .collect()
}

fn is_notice_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    [
        "LICENSE",
        "LICENCE",
        "COPYING",
        "NOTICE",
        "UNLICENSE",
        "COPYRIGHT",
    ]
    .iter()
    .any(|prefix| upper.starts_with(prefix))
}

fn collect_texts(dir: &Path, files: &mut BTreeSet<PathBuf>) -> Result<(), Error> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = entry.path();
        if kind.is_symlink() {
            return Err(Error::new(format!(
                "symlink in crate package: {}",
                path.display()
            )));
        }
        if kind.is_dir() {
            collect_texts(&path, files)?;
        } else if kind.is_file() && is_notice_name(&entry.file_name().to_string_lossy()) {
            files.insert(path);
        }
    }
    Ok(())
}

fn normalized(text: &str) -> String {
    text.replace("\r\n", "\n")
}

fn format_error(error: std::fmt::Error) -> Error {
    Error::new(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_selection_deduplicates_and_preserves_versions() -> Result<(), Error> {
        let packages =
            tree_packages("root v0.1.0 (/path)\ndep v1.0.0\ndep v1.0.0 (*)\ndep v2.0.0\n")?;
        assert_eq!(packages.len(), 3);
        assert!(packages.contains(&("dep".to_owned(), "2.0.0".to_owned())));
        assert!(tree_packages("unexpected output").is_err());
        Ok(())
    }

    #[test]
    fn notice_names_include_nested_and_lowercase_conventions() {
        for name in [
            "LICENSE",
            "license-mit",
            "LICENSE-BoringSSL",
            "NOTICE.txt",
            "COPYING",
            "UNLICENSE",
            "Copyright.txt",
        ] {
            assert!(is_notice_name(name), "{name}");
        }
        assert!(!is_notice_name("Cargo.toml"));
        assert_eq!(normalized("one\r\ntwo\n"), "one\ntwo\n");
    }
}
