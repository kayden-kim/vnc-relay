# Troubleshooting

Check logs and status by symptom, then take the appropriate recovery action. If the problem remains, collect a report without secrets.

First identify the failing stage. Process startup, administrator sign-in, relay connection rejection, and active session closure are different problems. HTTP status codes, WebSocket close codes, and CLI exit codes are also different. Check diagnostic codes and operations log events, not numbers alone.

## The server does not start

Check stderr in the server terminal for `error[<code>]` and `key`. Also check `startup_failed` in logs. Failures before configuration is read, or failures to open the log file, can be logged to standard output.

Read the exit code in the same terminal after the command returns.

Windows:

```powershell
$LASTEXITCODE
```

Ubuntu:

```shell
echo $?
```

### Accounts are missing or the account file is invalid

`accounts_file_missing` or `accounts_none` means that administrator accounts are not ready. The exit code is 5. Prepare an account with `admin add` as described in [Quick start](quick-start.md). There is no default account or password.

Unreadable or invalid account files also produce exit code 5. Check `admin.accounts_file` and read permissions. Do not repair an account file with arbitrary hashes or plain passwords. For hash minimum problems, use `admin passwd` as described in the [CLI contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md).

### A port is in use or this is a second instance

`listener_bind_failed` with exit code 4 means that binding to the indicated `listen` address failed. Check the process using the port and the configured address. Check whether the server is already running.

A second process using the same listener address fails. It does not replace the first process's waiting connections or active sessions. Do not stop the existing server unintentionally. Exactly duplicate listener addresses within one configuration can instead be rejected before binding with `config_listen_conflict`. Distinguish the diagnostic codes.

### Configuration, TLS, public key, or log files have errors

For exit code 3, inspect `config_*`, `tls_file_invalid`, `ticket_keys_invalid`, or `log_file_unavailable` diagnostics.

1. Check the setting identified by the diagnostic's `key`.
2. Resolve file paths from the configuration file directory.
3. Check read access for the OS account running the server. Log files also require an existing parent directory and write access.
4. Check TLS certificate and private key formats and that the pair matches.
5. Correct the diagnostic and start again.

Plain management HTTP on a non-loopback address prevents startup with `config_plaintext_management_not_loopback`. Configure built-in HTTPS for remote management or keep management HTTP on loopback.

For exit code 2, check command syntax and interactive input requirements. For example, omitting `--config` is a usage error. Exit code 1 also covers execution environment failures such as watcher or runtime preparation. See [CLI contract §2](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md) for complete meanings and stable diagnostic codes.

## Administrator sign-in fails

An incorrect password, a missing account, and a login limit all return `401 login_failed`. Do not distinguish the cause from the screen message alone. Check `admin_login_failed` and `admin_login_limited` in the operations log.

- During a login limit, even the correct password is rejected. Retry after the configured restriction period. A source IP limit can also affect other accounts from that source.
- If you added an account or changed a password without restarting, the server still uses the previous account set. Shut it down gracefully and start it again.
- Remote plain HTTP management is unsupported. Check that the management listener uses built-in HTTPS. Do not try to bypass the restriction with HTTPS only at a proxy.

For a forgotten password, shut down the server gracefully. Run the following command in an interactive terminal in the deployment directory. Replace `opsadmin` with the account to recover.

Windows:

```powershell
.\vnc-relay.exe admin passwd opsadmin --config .\relay.toml
```

Ubuntu:

```shell
./vnc-relay admin passwd opsadmin --config ./relay.toml
```

Enter the new hidden password twice. Start the server again, then sign in. Do not put the password on the command line. Restarting also invalidates previous management sessions.

Sign in again after idle expiry or the absolute maximum management session lifetime. Automatic polling does not extend idle expiry. See [Dashboard](dashboard.md) for session lifetime and sign-out behavior.

## Relay connections are rejected

Check `stage`, `reason`, listener, and role in `connection_rejected` events. Connections that never became active sessions do not appear in "Recently closed sessions".

### The source IP is not allowed

For `ip_not_allowed`, check the entry point's `allow` list. TCP checks the actual socket source IP. At accept time, WS/WSS checks the union of enabled role lists or trusted proxy membership. Failure here can close the connection without an HTTP response. Just before upgrade, the client address is checked against the selected role's list. Failure returns HTTP 403.

With a proxy, distinguish `peer_ip` from `forwarded_ip`. For `forwarded_for_invalid` and HTTP 400, check `trusted_proxies` and the proxy's `X-Forwarded-For` handling. Trust only proxy addresses. Requests whose forwarded addresses are all trusted proxies are also rejected. Do not allow the proxy IP as a substitute for every client. See [Configuration](configure.md).

### Origin does not match

For `origin_not_allowed` and HTTP 403, compare the browser's Origin with that role's `allowed_origins`. Normalized scheme, host, and port must match. Scheme and host ignore ASCII case, and omitted default ports match explicit default ports. Wildcards are unsupported. An empty list rejects all requests with Origin. Without Origin, only this comparison is skipped. IP checks and any enabled access ticket check still apply. See [Configuration contract §1.4](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md#14-origin-값-문법과-비교) for the exact normalization rules.

### The WS/WSS path is wrong or the role is disabled

For HTTP 404, check the URL path and the role's `enabled` setting. On `main`, Viewers use `/relay/viewer` and Servers use `/relay/server`. Do not add a trailing `/`. Disabled roles also return 404.

Releases `v2026.11.2` and earlier use `/relay` for Viewers. `v2026.11.3` and `main` do not also accept the old path. A `main` build prints the latest Release version until the next release. Check the package source or build commit, not just the version string. Migrate the server and all Viewer URLs together.

The `connection_rejected` reason `invalid_upgrade` also covers other invalid requests. Logs do not contain the request path, so this reason alone does not prove use of the old URL. Follow [Connecting VNC clients](connect.md) and migration guidance in [Operations](operate.md).

### The same relay ID and role are already occupied

`duplicate_id` means that the same role is already waiting with that relay ID, or an active session occupies it. The later TCP connection closes with FIN. WS/WSS receives close 4004. The existing connection is not replaced.

Check for duplicate client processes. If a new connection needs a different relay ID, set the same new value on both Viewer and Server. To end an existing active session, confirm the target in the dashboard before termination. A relay ID is not a password or proof of access rights.

### A Mode I target is not allowed

For `target_not_allowed` and close 4003, check `mode1.allow_targets`. The target IP and port must be allowed. Requests using a DNS name need both a name-and-port allowance and an allowance for the final connection IP and port. Adding only a name is insufficient.

`target_connect_failed` and close 4002 are different. Use operations logs to distinguish name resolution failure, target connection refusal, and connection timeout. Check connectivity to the target. Allow only required targets instead of bypassing the policy with unrestricted addresses.

### A connection or waiting limit has been reached

Check dashboard "Relay connections" and "Waiting connections". Reaching the relay connection limit at accept time can close a connection without an HTTP response. **Close 4011** (`relay_connection_limit`) specifically means that Mode I lacks a target socket slot after upgrade 101. Not every limit rejection uses 4011.

Close 4010 (`waiting_full`) means that new waiting registration reached its limit. Check duplicate clients and unnecessary connections first, then review limits against deployment capacity. Management and status connections use a separate limit, identified by `reason = admin_connection_limit` in `connection_rejected`.

### An access ticket is rejected with HTTP 401

This feature exists only on `main` and is off by default. An enabled role requires a valid access ticket. Check ticket-related `reason` values in `connection_rejected`. They distinguish missing tickets, invalid format/signature/claims, expiry, reuse, and failure of the startup-time condition. `ticket_issued_before_start` means that `iat` is at or before server startup time + 5 seconds. A ticket issued just after startup can also fail this condition.

Check issuer and server clocks, role settings, and public keys. A new ticket is needed after restart. Obtain a new value from the issuer instead of resending the same ticket. Administrator sign-in does not resolve a relay ticket failure.

For close 4012 (`ticket_relay_mismatch`) after 101, match the initial request's relay ID to the ticket's `rid` byte for byte. `0123` and `123` differ. A Viewer Mode I request that passes existing mode and initial request checks is also rejected with 4012. If Mode I is disabled, `mode_disabled` (1008) takes precedence. See [Access tickets](access-ticket.md) for all conditions. Do not paste ticket contents into logs or reports.

See [WebSocket contract §1.5 and §2](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md) for the complete HTTP rejection and close code rules.

## Active sessions end unexpectedly

Check the close reason in "Recently closed sessions" and `session_closed` events with the same `session_id`. Use "View logs" in session details to narrow the events. Recent lists and logs are retained in memory. Check file logs if the entries have already been removed.

- `peer_eof` means that both sides ended with normal EOF. Check whether the clients ended the connection.
- For `connection_error`, inspect `error_kind` and preceding connection errors. Distinguish network resets from WS protocol or message errors.
- `dead_connection` means that keepalive or ping detected an unresponsive peer. Check the network and client responses. An idle connection that responds is not closed merely because no screen data flows.
- For `admin_terminated`, check `admin_terminate` for the responsible account. WS/WSS receives close 4005.
- `session_expired` means that an access ticket's session expiry time `sx` was reached. WS/WSS receives close 4013. If both sides have expiry times, the earlier time ends the session. Check the issuer's session lifetime policy.
- `close_timeout` means that reverse-direction delivery or the closing exchange did not finish within `timeouts.close_secs` (E). Check peer responses and the network. Administrator termination can also produce this reason when cleanup times out.
- For `server_shutdown`, check server shutdown activity such as grace expiry or immediate forced cleanup. Look for `shutdown_started` and `shutdown_forced`.
- `internal_error` can indicate that a server bug ended the session owner task. Preserve surrounding logs and the deployment version for a report. Close 1011 is reserved and is not sent by the current implementation.

Do not infer the cause from a client-displayed 1006 alone. The server does not send close code 1006. Pong timeout and internal errors can close sockets without a Close frame.

If the connection was still waiting, find waiting time expiry in `waiting_expired` and other waiting removals in `waiting_removed`. `connection_aborted` records connections that ended before waiting registration or session startup. Close 4001 means that the Mode II peer did not arrive within the waiting time. Do not confuse this with an active session's close reason. See [Operations information contract §1.5 and §1.6](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md) and [Management API contract §1.10](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/management-api-v1.md) for the detailed mapping.

## Reload is rejected

`reload_restart_required` means that a restart-required value differs from the value in effect. Check `config_keys`. Revert those differences externally or restart during an approved maintenance window. Remaining differences reject entire subsequent candidates, including dynamic changes and certificate replacements.

For `reload_rejected`, correct the file using configuration, TLS, or public key diagnostics. The candidate is not partially applied. The previous valid configuration remains. `reload_watch_failed` indicates a detection failure; check file access and the watcher environment.

Check "Configuration file status" and "Last reload" under dashboard "Settings". Select "Refresh" because this screen does not poll. `pending` does not mean successful application. After correction, check `reload_applied` and the values in effect. There is no manual reload command.

## Settings cannot be edited

- For `config_edit_disabled`, set `management.config_edit = true` externally, restart, and sign in again. Editing is off by default.
- For `config_restart_pending`, revert restart-required differences or restart first. For `config_file_invalid`, correct source file read or validation problems externally.
- For `config_conflict`, another administrator or external editor changed the file. Comments and whitespace also cause conflicts. Select "Reload settings" to discard your input and load current settings, then edit again. Do not resend the old request automatically.
- For `config_write_failed`, this request changed neither the file nor the values in effect. Check file read/replacement permissions, parent directory temporary-file creation/replacement permissions, and disk space for the server OS account. On Windows, check open file handles that block replacement. On Ubuntu, also check directory write/search permissions and read-only mounts.
- Certificate, private key, public key paths, and restart-required keys have no input fields. Keys for disabled roles, listeners, or Mode I are also not editable.

Timeout or a missing response leaves the result unknown. First query current values, file status, and recent results in "View settings". Do not apply again before checking. See [Dashboard](dashboard.md) and [Operations how-to §3.5.1](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md).

## Recent logs are empty

`recent_logs.max_entries = 0` disables in-memory retention. Set it to 1–10000 in the configuration file and restart to retain logs. The default is 1000.

If retention is enabled, check level, event, search, and session filters. Restarting clears the previous retained list. The limit also removes older events. If a session start event has been removed, the session filter can omit some related connection events. Check retained operations log files when needed.

Do not treat a query failure as an empty result. Check "Stale data" and the last successful update time. Recent logs are independent of file output success. The screen alone does not prove that file logging works.

## Collect information for a report

1. Record the OS version and architecture.
2. Record the executable version.
3. Record the package source or build commit and executable hash.
4. Record the failing command, exit code, HTTP status or close code, and reproduction steps.
5. Record the time and time zone, and collect surrounding operations logs.
6. Remove secrets and unnecessary identifying information from a relevant configuration copy.

Run the version command in the deployment directory.

Windows:

```powershell
.\vnc-relay.exe --version
```

Ubuntu:

```shell
./vnc-relay --version
```

> **Warning:** Do not send passwords, hashes, private keys, signing keys, access ticket contents, management session cookies, or CSRF tokens. Do not attach account file contents or public key file contents. Do not collect raw VNC screen data or user input. Redact unnecessary addresses and administrator account names.

Send the report to the deployment's operations owner or project maintainer. See the [Operations information contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md) for event and field definitions. See [Operations](operate.md) for recovery procedures.
