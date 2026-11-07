//! Release bundle for the host OS (docs/how-to/release.md).
//!
//! Builds the release executable, checks the dependency notices, copies the bundle
//! files into `<target>/package/<name>/`, and archives that directory with the OS
//! `tar`: zip on Windows, gzip tar on Linux. A SHA-256 file in `sha256sum` format
//! goes next to the archive.

use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::process::Command;

use crate::{Error, cargo, notices};

const PRODUCT: &str = "vnc-relay";

/// Files copied from the repository root as they are (docs/how-to/operate-relay.md 3.1).
const ROOT_FILES: [&str; 4] = [
    "README.md",
    "LICENSE",
    "THIRD-PARTY-NOTICES.txt",
    "GLOSSARY.md",
];

struct Platform {
    os: &'static str,
    extension: &'static str,
    executable: &'static str,
}

#[cfg(all(windows, target_arch = "x86_64"))]
const PLATFORM: Option<Platform> = Some(Platform {
    os: "windows",
    extension: "zip",
    executable: "vnc-relay.exe",
});

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const PLATFORM: Option<Platform> = Some(Platform {
    os: "linux",
    extension: "tar.gz",
    executable: "vnc-relay",
});

#[cfg(not(any(
    all(windows, target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "x86_64")
)))]
const PLATFORM: Option<Platform> = None;

pub fn run() -> Result<bool, Error> {
    let platform = PLATFORM
        .ok_or_else(|| Error::new("package: only Windows x64 and Linux x64 are supported"))?;
    let workspace = cargo::metadata()?;
    let root = &workspace.root;
    let version = &workspace
        .packages
        .iter()
        .find(|package| package.name == PRODUCT)
        .ok_or_else(|| Error::new(format!("package `{PRODUCT}` is not in the workspace")))?
        .version;
    check_version(version)?;

    // The notices check reads the sources of both targets offline, so fetch them all.
    cargo_step(root, &["fetch", "--locked"])?;
    cargo_step(
        root,
        &["build", "--release", "--locked", "--package", PRODUCT],
    )?;
    if !notices::run(true)? {
        return Ok(false);
    }

    let name = format!("{PRODUCT}-{version}-{}-x64", platform.os);
    let out = workspace.target_dir.join("package");
    let stage = out.join(&name);
    if stage.exists() {
        fs::remove_dir_all(&stage)?;
    }
    fs::create_dir_all(&stage)?;
    fs::copy(
        workspace
            .target_dir
            .join("release")
            .join(platform.executable),
        stage.join(platform.executable),
    )?;
    fs::copy(
        root.join("examples").join("relay.toml"),
        stage.join("relay.toml"),
    )?;
    for file in ROOT_FILES {
        fs::copy(root.join(file), stage.join(file))?;
    }
    copy_dir(&root.join("docs"), &stage.join("docs"))?;

    let archive_name = format!("{name}.{}", platform.extension);
    let archive = out.join(&archive_name);
    if archive.exists() {
        fs::remove_file(&archive)?;
    }
    let status = archive_command(&archive_name, &name)
        .current_dir(&out)
        .status()?;
    if !status.success() {
        return Err(Error::new(format!("tar failed to write {archive_name}")));
    }
    let digest = hex(&sha256(&fs::read(&archive)?));
    fs::write(
        out.join(format!("{archive_name}.sha256")),
        format!("{digest}  {archive_name}\n"),
    )?;
    // A CARGO_TARGET_DIR outside the repository is shown as it is.
    let shown = if archive.starts_with(root) {
        cargo::display_path(root, &archive)
    } else {
        archive.display().to_string()
    };
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "package: {shown}")?;
    writeln!(stdout, "sha256: {digest}")?;
    Ok(true)
}

fn cargo_step(root: &Path, args: &[&str]) -> Result<(), Error> {
    let status = cargo::command().current_dir(root).args(args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::new(format!("cargo {} failed", args.join(" "))))
    }
}

/// Windows uses its built-in bsdtar, which writes zip for a `.zip` name with `-a`.
/// `tar` on PATH may be another tar without zip support.
#[cfg(windows)]
fn archive_command(archive: &str, dir: &str) -> Command {
    let system_root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    let mut command = Command::new(Path::new(&system_root).join("System32").join("tar.exe"));
    command.args(["-a", "-c", "-f", archive, dir]);
    command
}

/// GNU tar on Ubuntu. Owner names of the build machine stay out of the archive.
#[cfg(not(windows))]
fn archive_command(archive: &str, dir: &str) -> Command {
    let mut command = Command::new("tar");
    command.args([
        "--owner=0",
        "--group=0",
        "--numeric-owner",
        "-czf",
        archive,
        dir,
    ]);
    command
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), Error> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// Version rule: `YEAR.MONTH.SEQUENCE` without leading zeros, for example `2026.4.0`
/// (user decisions 2026-11-08 (Q165, Q166)). SEQUENCE starts at 0 in each month.
fn check_version(version: &str) -> Result<(), Error> {
    let parts: Vec<&str> = version.split('.').collect();
    let valid = match parts.as_slice() {
        [year, month, sequence] => {
            year.len() == 4
                && is_number(year)
                && is_number(month)
                && matches!(month.parse::<u8>(), Ok(1..=12))
                && is_number(sequence)
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(Error::new(format!(
            "version `{version}` is not YEAR.MONTH.SEQUENCE without leading zeros (for example 2026.4.0)"
        )))
    }
}

fn is_number(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'))
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut text, byte| {
            text.push(char::from(b"0123456789abcdef"[usize::from(byte >> 4)]));
            text.push(char::from(b"0123456789abcdef"[usize::from(byte & 0x0f)]));
            text
        })
}

/// SHA-256 from FIPS 180-4. Written here because the tool has no hash dependency.
fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_length = u64::try_from(data.len())
        .unwrap_or(u64::MAX)
        .wrapping_mul(8);
    let mut message = Vec::with_capacity(data.len() + 72);
    message.extend_from_slice(data);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());

    let mut schedule = [0u32; 64];
    for block in message.as_chunks::<64>().0 {
        for (word, bytes) in schedule.iter_mut().zip(block.as_chunks::<4>().0) {
            *word = u32::from_be_bytes(*bytes);
        }
        for i in 16..64 {
            let s0 = schedule[i - 15].rotate_right(7)
                ^ schedule[i - 15].rotate_right(18)
                ^ (schedule[i - 15] >> 3);
            let s1 = schedule[i - 2].rotate_right(17)
                ^ schedule[i - 2].rotate_right(19)
                ^ (schedule[i - 2] >> 10);
            schedule[i] = schedule[i - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for (constant, word) in K.iter().zip(schedule) {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(*constant)
                .wrapping_add(word);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (value, add) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *value = value.wrapping_add(add);
        }
    }

    let mut digest = [0u8; 32];
    for (bytes, value) in digest.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        *bytes = value.to_be_bytes();
    }
    digest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_fips_180_examples() {
        // FIPS 180-4 examples, including a message whose padding needs a second block
        // and a long message over many blocks.
        assert_eq!(
            hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(&sha256(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        assert_eq!(
            hex(&sha256(&vec![b'a'; 1_000_000])),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn version_follows_year_month_sequence_without_leading_zeros() {
        for version in ["2026.4.0", "2026.11.0", "2026.12.15", "2027.1.3"] {
            assert!(check_version(version).is_ok(), "{version}");
        }
        for version in [
            "0.1.0",
            "2026.04.0",
            "2026.0.0",
            "2026.13.0",
            "2026.11.01",
            "2026.11",
            "2026.11.0.1",
            "2026.11.0-rc.1",
            "02026.11.0",
        ] {
            assert!(check_version(version).is_err(), "{version}");
        }
    }
}
