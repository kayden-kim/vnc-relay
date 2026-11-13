# Operations

Start and stop the server manually, and check health, logs, and configuration changes. Use this page to replace certificates, public keys, and administrator accounts, and to prepare upgrades or rollbacks.

These procedures target PowerShell on Windows 11 x64 and bash on Ubuntu 24.04 LTS x64. OS service registration and containers are out of scope. Run commands from the extracted deployment directory. First prepare settings and an administrator account as described in [Quick start](quick-start.md).

## Run in the foreground

Windows:

```powershell
.\vnc-relay.exe serve --config .\relay.toml
```

Ubuntu:

```shell
./vnc-relay serve --config ./relay.toml
```

The process occupies the terminal until it stops. Do not consider it ready before the `server_ready` event. Relative file paths within the configuration resolve from the configuration file directory, not the terminal's working directory.

## Check health

The example configuration places management at `127.0.0.1:18080` and status at `127.0.0.1:18081`. Send health requests to the **status listener**.

Run these two requests in another terminal.

Windows:

```powershell
curl.exe --fail http://127.0.0.1:18081/livez
curl.exe --fail http://127.0.0.1:18081/readyz
```

Ubuntu:

```shell
curl --fail http://127.0.0.1:18081/livez
curl --fail http://127.0.0.1:18081/readyz
```

`/livez` returns HTTP 200 and `{"status":"alive"}` when the process can respond. `/readyz` returns HTTP 200 and `{"status":"ready"}` when ready. Otherwise it returns 503. During the shutdown grace period, `/livez` remains 200 while the process can respond, but `/readyz` returns 503.

The status listener requires no authentication. It provides only liveness and readiness, not session lists. Loopback binding is recommended. See [Dashboard](dashboard.md) for management screens.

## Shut down and restart

1. Press Ctrl+C once in the server terminal.
2. Wait for the `server_stopped` event and for the command to return.
3. Check the exit code in the same terminal.

Windows:

```powershell
$LASTEXITCODE
```

Ubuntu:

```shell
echo $?
```

A completed graceful shutdown returns exit code 0. New relays stop, and active sessions receive the shutdown grace period `timeouts.shutdown_grace_secs` (G). The default is 30 seconds. With G set to 0, cleanup starts immediately. Cleaning up sessions after G expires still returns normal exit code 0.

Management sign-in, queries, termination, and configuration viewing and editing remain available during the grace period. New file-watch applications stop, but configuration edit commits remain allowed. Editing G does not change an already started grace period.

A second shutdown request skips the remaining grace period and forces immediate cleanup. Ubuntu accepts SIGINT (Ctrl+C) and SIGTERM. Windows accepts Ctrl+C and Ctrl+Break and has no SIGTERM.

> **Warning:** Closing a Windows console window forces immediate cleanup without waiting for G. The process must stop within the OS's default 5-second limit. Ctrl+C is recommended for normal operations. Windows sign-out or system shutdown, Task Manager termination, SIGKILL, and power loss do not guarantee a grace period or shutdown records.

Run the same `serve` command to restart. Previous sockets, waiting connections, active sessions, and management sessions are not restored. Administrators must sign in again.

## Operations log and file rotation

The operations log uses UTF-8 JSON Lines: one JSON object per line. `log.output = "stdout"` sends it to standard output. A file path appends to an existing file or creates it if absent. Create its parent directory in advance. Changing the output path requires a restart.

The following procedure assumes `log.output = "relay.log"`. The archive name `relay.previous.log` must not already exist.

1. Rename the active log file.

   Windows:

   ```powershell
   Rename-Item -LiteralPath .\relay.log -NewName relay.previous.log
   ```

   Ubuntu:

   ```shell
   mv -- ./relay.log ./relay.previous.log
   ```

2. Create an operations log event, for example with a test connection.
3. Confirm that new events appear at the original path, `relay.log`.

The server reopens the original path after rename, deletion, or creation notifications. It also reopens it every `reload.poll_secs`, even if a notification was missed. Events written before reopening can remain in the old file. Do not use copytruncate for this rotation procedure.

If reopening fails, the server keeps writing to the old file. It emits `log_reopen_failed` and a stderr warning when the failure begins. Restore the path and permissions, then check the next notification or periodic attempt. A failed line write emits a `log_write_failed` stderr warning only on the first failure. The server discards failed lines and continues service. Do not expect lost lines to be recovered automatically.

Rotation timing, retention, and access permissions are the operator's responsibility. Restrict logs and archives to approved operators. Monitoring disk space and stderr is recommended. [Dashboard recent logs](dashboard.md) are a separate in-memory list, not a replacement for file retention.

## Automatic reload on file changes

The server detects changes to the configuration file, TLS files in use, and public key files for roles with access ticket checks enabled. It does not watch TLS files for a WSS listener whose roles are all disabled. Periodic checks supplement OS notifications. The defaults for `reload.poll_secs` (Dp) and `reload.settle_secs` (D) are 30 and 5 seconds. Once file changes stop, even a missed notification produces an application or failure result within Dp + D. With successive changes, the result for the latest stable files is due within D after the last detected change.

Reloadable settings include source IP, Origin, trusted proxy, and Mode I target policies; limits, timeouts, and keepalive; TLS paths and contents; and access ticket settings and public key files. See application timing by key in [Configuration contract §1.8](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md).

1. Keep protected backups of the current valid files.
2. Change only the required reloadable values with an external editor and save.
3. Check the operations log for `reload_applied`.
4. Check the result with a new connection or request that uses the changed policy.

Completed checks, existing connections, started timers, and configured socket options retain their previous results. Lowering a limit below current use does not disconnect existing connections. It rejects new resource use.

Listener addresses and ports, relay role and mode switches, management HTTP/HTTPS selection, account and management session policies, and `management.config_edit` require a restart. A restart-required difference rejects the entire candidate with `reload_restart_required`. Leaving that difference in the file also blocks subsequent urgent certificate replacements.

Candidate validation failure produces `reload_rejected`. Watch failures produce `reload_watch_failed`. Failures retain the previous valid configuration and existing relays. There is no manual reload CLI or web reload API. You can also inspect file status and the last result in dashboard "View settings".

## Replace certificate and public key files

### TLS certificate and private key

Use this procedure only when TLS is enabled. The certificate and private key must form a valid pair. Do not put private key contents in logs or command arguments.

1. Prepare the new certificate chain and matching private key in protected new files.
2. Confirm that the OS account running the server can read both files.
3. Use an external editor to update the corresponding `cert_file` and `key_file` paths together and save.
4. Check for `reload_applied`.
5. Compare the public certificate's serial number or fingerprint on a new TLS connection.

Replacing file contents at the same paths is also detected. If only one side changes and the pair does not match, or a file cannot be read, the candidate is rejected and the previous TLS configuration remains. Existing TLS connections do not change certificates. TLS paths cannot be edited through the dashboard. Do not save from the dashboard while replacing files.

### Access ticket public keys

Use this procedure for a role with access tickets enabled. Keep signing keys only at the issuer. Place only public keys on VNC Relay. Public key paths are not dashboard-editable, so replace contents at the same path.

1. Prepare a new signing key pair at the issuer.
2. Add the new public key to `keys` in the role's existing public key file.
3. Confirm that `reload_applied` includes the corresponding `public_keys_file`.
4. Switch the issuer to the new signing key.
5. Wait for that role's `max_lifetime_secs` + 10 seconds.
6. Remove the old public key from the file.
7. Check for `reload_applied` again.

Invalid public key files are rejected with `ticket_keys_invalid`, retaining the previous public keys. Replacement does not clear already upgraded connections or usage records. Restricting write access to the file is recommended. Anyone able to insert a key can authorize tickets they create. See [Access tickets](access-ticket.md) for formats and prerequisites.

## Manage administrator accounts

Use the local CLI to add accounts, change passwords, and remove accounts. There is no default account or password. Commands use `admin.accounts_file` from the configuration.

1. Shut down the server gracefully.
2. Run one required command.
3. Use `admin list` to check remaining account names.
4. Start the server again.
5. Sign in with a remaining account.

Windows:

```powershell
.\vnc-relay.exe admin add opsadmin --config .\relay.toml
.\vnc-relay.exe admin passwd opsadmin --config .\relay.toml
.\vnc-relay.exe admin remove opsadmin --config .\relay.toml
.\vnc-relay.exe admin list --config .\relay.toml
```

Ubuntu:

```shell
./vnc-relay admin add opsadmin --config ./relay.toml
./vnc-relay admin passwd opsadmin --config ./relay.toml
./vnc-relay admin remove opsadmin --config ./relay.toml
./vnc-relay admin list --config ./relay.toml
```

These commands are alternatives, not a sequence to run in full. `add` adds an account, `passwd` changes its password, and `remove` deletes it. `list` shows names only. `add` and `passwd` request a hidden password twice in an interactive terminal. Do not pass passwords as arguments or through a pipe. `remove` asks for deletion confirmation.

Account file contents are not reloaded automatically. If you change only the file while running, previous credentials and existing management sessions remain valid until restart. Removing the last account prevents startup. See [CLI contract §1.6](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md) for account file permissions.

## Upgrade and roll back

Restarting ends all connections and management sessions. Work during an approved maintenance window.

1. Keep the current executable, configuration, account file, TLS files, and any access ticket public key files in a protected recovery location. Record the executable version and hash.
2. Download the target OS package from [Releases](https://github.com/kayden-kim/vnc-relay/releases) and verify its hash. Follow [Installation](install.md).
3. Prepare the new executable in a separate directory. Keep the configuration copy in the same directory as the operational configuration to preserve relative file path resolution. Migrate the copy first if the contract changed.
4. Set all enabled listeners in the copy to distinct unused loopback ports. Set log output to `stdout`. Do not modify operational account or TLS files.
5. Start the new executable with that copy. Check `server_ready` and HTTP 200 from the status listener's `/readyz`.
6. Stop the trial process gracefully with Ctrl+C.
7. Shut down the existing server gracefully during the maintenance window.
8. Install the new executable and validated configuration with **operational addresses**. Do not install the alternate-port copy.
9. Check startup, health, management sign-in, actual relay traffic, and reload.

There is no configuration-validation-only CLI. Starting on alternate ports does not validate operational ports, the firewall, or actual client relays. See the [Operations how-to](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md) for source builds and detailed deployment procedures.

> **Warning:** Upgrading from `v2026.11.2` or earlier changes the WS/WSS Viewer path from `/relay` to `/relay/viewer`. Migrate the server and all Viewer URLs together. A `main` build prints the latest Release version until the next release, so `--version` alone cannot distinguish them. Record the deployment source and commit too.

For rollback, stop the new server gracefully and restore the previous executable together with configuration compatible with it. Restore client URLs too when crossing Viewer path versions. Do not leave `[relay.ticket.*]` tables in configuration for a version without access ticket support. Restoring the protected v3 configuration is recommended when rolling back from v4 to v3. Before returning to a version without forwarded address checks, stop proxy access or switch to direct built-in WSS and actual socket IP policies. Do not allow the proxy IP as a substitute for every client.

After restoration, check startup, readiness, actual relay traffic, and management sign-in. Follow the version-specific migration and recovery steps in [Operations how-to §3.2 and §6](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md).

## Public deployment security checklist

- Remote management must use built-in HTTPS. Keep plain HTTP on loopback only. Relay proxy TLS termination rules do not apply to the management listener.
- Narrow source IP allow lists for each role. An empty list rejects all sources. Do not enable unrestricted access unintentionally.
- WSS or relay proxy TLS termination is recommended for public WebSocket access. Keep unnecessary plain WS roles disabled. Protect the proxy-to-VNC Relay segment too.
- Match browser Origin values with each role's `allowed_origins`. Origin does not replace IP checks or user authentication.
- Put only proxy addresses in `trusted_proxies`, not client ranges.
- Narrow Mode I `allow_targets` to required target IPs and ports.
- Protect account files, private keys, configuration, public key files, and logs. Restrict public key write access to prevent tampering.
- When configuration editing is enabled, check that the server OS account can read and replace the configuration file and create and replace temporary files in its parent directory. Block writes by other ordinary OS accounts.
- To control a role's access only through access tickets, disable that role's TCP entry point. Tickets do not apply to TCP.

See the [Configuration contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md), [Operations information contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md), and [PRD confirmed criteria](https://github.com/kayden-kim/vnc-relay/blob/main/docs/requirements/PRD.md) for detailed requirements. See [Troubleshooting](troubleshooting.md) for symptom-specific recovery.
