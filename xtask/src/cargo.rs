//! Workspace facts read from `cargo metadata`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::Error;

pub struct Workspace {
    pub root: PathBuf,
    pub target_dir: PathBuf,
    pub packages: Vec<Package>,
}

pub struct Package {
    pub name: String,
    pub version: String,
    pub dir: PathBuf,
    pub targets: Vec<Target>,
    pub dependencies: Vec<Dependency>,
}

pub struct Target {
    pub kinds: Vec<String>,
    pub src_path: PathBuf,
}

impl Target {
    /// True when `cargo test` builds a test harness for this target.
    pub fn is_tested(&self) -> bool {
        !self
            .kinds
            .iter()
            .any(|kind| matches!(kind.as_str(), "bench" | "example" | "custom-build"))
    }

    pub fn is_lib(&self) -> bool {
        self.kinds.iter().any(|kind| kind.ends_with("lib"))
    }
}

pub struct Dependency {
    pub name: String,
    pub req: String,
    /// `normal`, `dev`, or `build`.
    pub kind: String,
}

/// The cargo that runs this tool, so the pinned toolchain is used for nested commands.
pub fn command() -> Command {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

pub fn metadata() -> Result<Workspace, Error> {
    let output = command()
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(Error::new("`cargo metadata` failed"));
    }
    let value: Value = serde_json::from_slice(&output.stdout)?;
    let packages = value["packages"]
        .as_array()
        .ok_or_else(|| missing("packages"))?
        .iter()
        .map(package)
        .collect::<Result<_, _>>()?;
    Ok(Workspace {
        root: PathBuf::from(text(&value, "workspace_root")?),
        target_dir: PathBuf::from(text(&value, "target_directory")?),
        packages,
    })
}

fn package(value: &Value) -> Result<Package, Error> {
    let manifest = Path::new(text(value, "manifest_path")?);
    let dir = manifest
        .parent()
        .ok_or_else(|| missing("manifest_path parent"))?;
    let targets = value["targets"]
        .as_array()
        .ok_or_else(|| missing("targets"))?
        .iter()
        .map(|target| {
            Ok(Target {
                kinds: strings(&target["kind"]),
                src_path: PathBuf::from(text(target, "src_path")?),
            })
        })
        .collect::<Result<_, Error>>()?;
    let dependencies = value["dependencies"]
        .as_array()
        .ok_or_else(|| missing("dependencies"))?
        .iter()
        .map(|dependency| {
            Ok(Dependency {
                name: text(dependency, "name")?.to_owned(),
                req: text(dependency, "req")?.to_owned(),
                kind: dependency["kind"].as_str().unwrap_or("normal").to_owned(),
            })
        })
        .collect::<Result<_, Error>>()?;
    Ok(Package {
        name: text(value, "name")?.to_owned(),
        version: text(value, "version")?.to_owned(),
        dir: dir.to_path_buf(),
        targets,
        dependencies,
    })
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, Error> {
    value[key].as_str().ok_or_else(|| missing(key))
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn missing(key: &str) -> Error {
    Error::new(format!("`cargo metadata` output has no `{key}`"))
}

/// Path relative to `root` with `/` separators, for reports. A path outside
/// `root`, such as a `CARGO_TARGET_DIR` elsewhere, is shown as it is.
pub fn display_path(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(relative) => relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::display_path;

    #[test]
    fn display_path_joins_inside_root_with_slashes() {
        let root = std::env::temp_dir().join("repo");
        let path = root.join("target").join("help-site");
        assert_eq!(display_path(&root, &path), "target/help-site");
    }

    #[test]
    fn display_path_keeps_paths_outside_root_as_they_are() {
        let base = std::env::temp_dir();
        let root = base.join("repo");
        let outside = base.join("vnc-relay-target").join("help-site");
        assert_eq!(display_path(&root, &outside), outside.display().to_string());
    }
}
