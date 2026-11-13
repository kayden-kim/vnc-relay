# Installation

Use this page to download the bundle for your OS, verify its hash, and prepare the files for the first run.

## Requirements

Supported systems are Windows 11 x64 and Ubuntu 24.04 LTS x64. Windows Server and other operating systems or architectures are outside the verified scope. This guide covers manual execution. It does not cover OS service registration or containers.

You do not need Rust build tools to run a release bundle. Prepare a build environment only if you build the bundle from source.

## Download the bundle

1. Open the Release you want on [GitHub Releases](https://github.com/kayden-kim/vnc-relay/releases).
2. Download the bundle for your OS.
3. Download the file with the same name followed by `.sha256` into the same directory.

| OS | Bundle | Hash file |
| --- | --- | --- |
| Windows 11 x64 | `vnc-relay-2026.11.3-windows-x64.zip` | `vnc-relay-2026.11.3-windows-x64.zip.sha256` |
| Ubuntu 24.04 LTS x64 | `vnc-relay-2026.11.3-linux-x64.tar.gz` | `vnc-relay-2026.11.3-linux-x64.tar.gz.sha256` |

The commands below use `v2026.11.3`. If you download another version, change the version in every file name. See [Introduction](index.md) for differences between the Release and `main`.

## Verify SHA-256

Run these commands in the download directory before extracting the bundle.

### Windows PowerShell

1. Calculate the bundle's SHA-256 hash.

   ```powershell
   Get-FileHash .\vnc-relay-2026.11.3-windows-x64.zip -Algorithm SHA256
   ```

2. Display the hash file.

   ```powershell
   Get-Content .\vnc-relay-2026.11.3-windows-x64.zip.sha256
   ```

3. Compare the `Hash` from `Get-FileHash` with the first word in the hash file. The values must match, ignoring letter case.

### Ubuntu bash

1. Verify the bundle with the hash file.

   ```shell
   sha256sum -c vnc-relay-2026.11.3-linux-x64.tar.gz.sha256
   ```

2. Check that the result is `OK`.

> **Warning:** If the hashes differ, do not extract or run the bundle. Download the bundle and hash file again from the same Release.

## Extract the bundle

Choose a directory for the operational files. These commands run in the download directory and extract below the current location. To use another operational location, place the entire extracted directory there.

### Windows PowerShell

1. Extract the bundle.

   ```powershell
   Expand-Archive -Path .\vnc-relay-2026.11.3-windows-x64.zip -DestinationPath .
   ```

2. Enter the extracted directory.

   ```powershell
   Set-Location .\vnc-relay-2026.11.3-windows-x64
   ```

### Ubuntu bash

1. Extract the bundle.

   ```shell
   tar -xzf vnc-relay-2026.11.3-linux-x64.tar.gz
   ```

2. Enter the extracted directory.

   ```shell
   cd vnc-relay-2026.11.3-linux-x64
   ```

Run subsequent commands in this directory, which contains the executable and `relay.toml`.

## Bundle contents and file locations

| File or directory | Purpose |
| --- | --- |
| `vnc-relay.exe` or `vnc-relay` | Executable for the target OS |
| `relay.toml` | Loopback configuration from the repository's `examples/relay.toml` |
| `README.md` | Product introduction and quick start |
| `docs/`, `GLOSSARY.md` | Operations guide, contracts, and glossary for offline reading |
| `LICENSE` | Project MIT license |
| `THIRD-PARTY-NOTICES.txt` | Notices for third-party components included in the executable |

The bundle contains no account file, private key, password, or operations log. With the example configuration, creating an administrator account creates `admin-accounts.toml`. Running the server writes operations logs to `relay.log`.

Relative paths inside the configuration resolve from the directory containing `relay.toml`, not the terminal's current directory. The example account file and log belong in that directory. If you enable TLS, you must provide the certificate and private key separately and set their paths in the configuration.

You must restrict access to the account file, private key, and log directory. On Windows, it is recommended to restrict ACLs to the server's OS account and administrators. On Ubuntu, mode `0600` is recommended for account files and private keys. See section 3.2 of the [manual operations guide](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md).

## Alternative: build a bundle from source

On the target OS, prepare Rust and C build tools using [development setup and verification](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/set-up-and-verify.md). Distinguish a Release tag from `main` when choosing the source to package.

1. Build the bundle from the repository root.

   In Windows PowerShell, set this environment variable first if NASM is unavailable.

   ```powershell
   $env:AWS_LC_SYS_PREBUILT_NASM = "1"
   ```

   The packaging command is the same on both operating systems.

   ```shell
   cargo xtask package
   ```

2. Find the OS-specific bundle and `.sha256` file in `target/package/`. If you set `CARGO_TARGET_DIR`, use that path instead of `target`.
3. Follow the hash verification and extraction procedures on this page.

Bundle names, contents, and packaging procedures follow sections 1.2 and 3.1 of the [release guide](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/release.md) and section 3.1 of the [manual operations guide](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md).

## First-run checklist

- [ ] The bundle hash matches.
- [ ] The executable and `relay.toml` are in the operational directory.
- [ ] Required ports are available, and firewall and source IP policies have the intended scope.
- [ ] Access to the account file, private key, and log directory is restricted.
- [ ] Create an administrator account before starting the server. There is no default account or password. The server does not start without an account.

Continue with [Quick start](quick-start.md) to create the administrator account. Before public deployment, follow [Configuration](configure.md) and [Operations](operate.md).
