# Reference

Use this page to find CLI commands, HTTP and WebSocket paths, configuration tables, close reasons, operations log events, and governing contracts.

## CLI commands

The table lists arguments to append to the executable. In Windows PowerShell, run `.\vnc-relay.exe`. In Ubuntu bash, run `./vnc-relay` from the executable's directory.

| Arguments | Purpose |
| --- | --- |
| `serve --config <path>` | Run the server in the foreground. |
| `admin add <account> --config <path>` | Add an administrator account. Read the password and confirmation through hidden input. |
| `admin passwd <account> --config <path>` | Change an existing account's password. Read the new password and confirmation through hidden input. |
| `admin remove <account> [--yes] --config <path>` | Remove an account. `--yes` skips deletion confirmation. |
| `admin list --config <path>` | Print administrator account names in name order, one per line. |
| `--help` or `-h` | Print usage. Do not combine with other arguments. |
| `--version` or `-V` | Print the executable version. Do not combine with other arguments. |

`--config` is required for `serve` and all `admin` commands. Passwords are not accepted through command-line arguments or environment variables. Account changes do not affect a running server. A graceful stop and restart is required.

From the extracted package's executable directory, run the following when the prepared configuration file is `relay.toml`.

Windows PowerShell:

```powershell
.\vnc-relay.exe serve --config .\relay.toml
```

Ubuntu bash:

```shell
./vnc-relay serve --config ./relay.toml
```

Exit codes distinguish success `0`, execution failure `1`, usage error `2`, configuration/TLS/ticket public key/log file preparation errors `3`, listener bind failure `4`, and account file errors `5`. For exact scopes and error names, use the [CLI contract, §§1.1, 1.7, and 2](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md). See [Operations](operate.md) for startup and shutdown procedures.

## HTTP endpoints

The management, status, and relay WS/WSS listeners are separate. One port does not serve every path.

| Listener | Path | Purpose |
| --- | --- | --- |
| Status | `GET /livez` | Return `200` with `status: alive` if the process can respond. |
| Status | `GET /readyz` | Return `200` with `status: ready` when ready. Otherwise return `503` with `status: not_ready`. |
| Management | `/` | Serve the embedded dashboard. |
| Management | Under `/api/v1/` | Administrator sign-in/out, server/connection/session/settings/log queries, termination by administrator, and dynamic configuration editing. |

Status checks need no authentication. The status listener uses plain HTTP. Management uses HTTPS as the rule; plain HTTP is allowed only on loopback. For individual management API paths, authentication, and CSRF requirements, use the [management API contract, §1.2](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/management-api-v1.md). For liveness versus readiness and full response formats, use the [operations information contract, §1.12](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md).

## WebSocket paths

| Path | Role |
| --- | --- |
| `/relay/viewer` | Viewer. Accepts Mode I and Mode II when tickets are off. |
| `/relay/server` | Server. Accepts Mode II only. |

WS and WSS use the same paths. Queries do not select a target or relay ID. Disabled roles and other paths return HTTP 404. A trailing slash is a different path. `main` also rejects the old Viewer path `/relay` with 404.

> **Note:** The latest Release, `v2026.11.2`, uses Viewer path `/relay`. This table describes `main`. See [Introduction](index.md) for version differences and [Connecting VNC clients](connect.md) for client configuration.

For upgrades and all close codes, use the [WebSocket contract, §§1.1, 1.5, and 1.7](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md). For additional rules on ticket-enabled roles, see [Access tickets](access-ticket.md).

## Configuration table map

This table helps locate settings. For the full key list, defaults, and ranges, use the [configuration contract, §1.2](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md). See [Configuration](configure.md) for authoring and reload behavior.

| Table | Purpose |
| --- | --- |
| `[admin]` | Administrator account file path |
| `[admin.argon2]` | Parameters for new password hashes |
| `[relay.tcp_viewer]` | TCP Viewer entry point |
| `[relay.tcp_server]` | TCP Server entry point |
| `[relay.ws]` | Shared plain WS listener and trusted proxies |
| `[relay.wss]` | Shared WSS listener, trusted proxies, and TLS files |
| `[relay.ws.viewer]` | WS Viewer enablement, IP, and Origin policies |
| `[relay.ws.server]` | WS Server enablement, IP, and Origin policies |
| `[relay.wss.viewer]` | WSS Viewer enablement, IP, and Origin policies |
| `[relay.wss.server]` | WSS Server enablement, IP, and Origin policies |
| `[relay.ticket.viewer]` | Access ticket checks for WS/WSS Viewer |
| `[relay.ticket.server]` | Access ticket checks for WS/WSS Server |
| `[mode1]` | Mode I enablement and target allow policy |
| `[mode2]` | Mode II enablement |
| `[limits]` | Connection, waiting connection, buffer, and WebSocket message limits |
| `[timeouts]` | Procedure timeouts and shutdown grace period |
| `[keepalive]` | TCP keepalive and WebSocket ping/pong |
| `[management]` | Management listener, TLS files, and configuration editing enablement |
| `[management.login_limit]` | Login failure limits per account and source |
| `[management.session]` | Management session idle timeout and absolute maximum lifetime |
| `[dashboard]` | Polling interval and query timeout |
| `[recent_closed]` | Retention size for recently closed sessions |
| `[recent_logs]` | In-memory recent log retention size |
| `[status]` | Status listener address |
| `[log]` | Operations log destination |
| `[reload]` | File-change result deadline and fallback polling interval |

## Close reasons

A close reason explains why an active session ended after cleanup. The labels below quote the dashboard text.

| Value | Dashboard label |
| --- | --- |
| `peer_eof` | "Normal EOF" |
| `connection_error` | "Connection error" |
| `admin_terminated` | "Terminated by administrator" |
| `session_expired` | "Session expired" |
| `dead_connection` | "Dead connection detected" |
| `close_timeout` | "Cleanup time limit exceeded" |
| `server_shutdown` | "Server shutdown" |
| `internal_error` | "Internal error" |

These values are not the full list of WebSocket close codes or connection rejection `reason` values. Waiting connections are not active sessions, so waiting expiry does not appear in recently closed sessions. Values and meanings follow the [management API contract, §1.10](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/management-api-v1.md). Log levels and additional fields follow the [operations information contract, §1.6](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md). Labels follow the [dashboard specification, §9](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/dashboard.md) and [copy.js](https://github.com/kayden-kim/vnc-relay/blob/main/web/dashboard/app/copy.js).

## Common operations log events

Operations logs use JSON Lines. The `event` field identifies the event. This is a selection of common events. For the full list and fields, use the [operations information contract, §§1.5–1.6](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md).

| Events | What to check |
| --- | --- |
| `server_ready`, `startup_failed` | Startup success or failure reason |
| `connection_rejected` | Rejection stage and `reason` |
| `waiting_registered`, `waiting_expired`, `waiting_removed` | Mode II waiting registration, wait timeout, and removal for other reasons |
| `session_started`, `session_closed` | Session start and end. The close reason is in `session_closed.close_cause`. |
| `ticket_accepted` | Successful ticket verification and use-record insertion |
| `target_connect_failed` | Mode I target resolution or connection failure |
| `reload_applied`, `reload_rejected`, `reload_restart_required`, `reload_watch_failed` | Reload success, rejection, restart requirement, or watcher failure |
| `config_edited`, `config_edit_rejected` | Dynamic configuration editing result |
| `admin_login_failed`, `admin_login_limited`, `admin_terminate` | Management sign-in failures, login limits, and termination results |
| `allow_all_source`, `trust_all_proxies`, `allow_all_target` | Broad access policy warnings |
| `ticket_tcp_bypass`, `ticket_plaintext_ws` | TCP ticket bypass and plain WS exposure warnings |
| `log_reopen_failed` | Failure to reopen a replaced log file |
| `shutdown_started`, `shutdown_forced`, `server_stopped` | Shutdown start, forced cleanup of remaining sessions, and completion |

See [Operations](operate.md) and [Troubleshooting](troubleshooting.md) for log correlation and response procedures.

## Contract versions implemented by main

These are external format contract versions, not product Release numbers. The executable's `--version` alone cannot distinguish additions in `main`. Check the Release differences in [Introduction](index.md) too.

| Contract name | Version | Governing document |
| --- | --- | --- |
| `vnc-relay-config` | 4.1.0 | [Configuration file contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md) |
| `ops-info` | 2.1.0 | [Operations information contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md) |
| `websocket-relay` | 2.1.0 | [WebSocket relay contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md) |
| `vnc-relay-cli` | 1.7.0 | [CLI and account file contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md) |
| `management-api` | 1.1.0 | [Management API contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/management-api-v1.md) |

## Find reference documents

- Requirements: [PRD](https://github.com/kayden-kim/vnc-relay/blob/main/docs/requirements/PRD.md). Distinguish confirmed criteria in §1 from undecided proposals.
- Specifications: [Relay](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/relay.md), [WS/WSS](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/websocket.md), [Operations](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/operations.md), [Management](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/management.md), [Dashboard](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/dashboard.md).
- Contracts: [Configuration](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md), [CLI and account files](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md), [WebSocket](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md), [Operations information](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md), [Management API](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/management-api-v1.md), [Repeater handshake](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/repeater-handshake-v1.md).
- Compatibility evidence: [Repeater compatibility record](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/repeater-compat-record.md).
- Procedures: [Manual operations guide](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md), [Release guide](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/release.md), [Source build and verification](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/set-up-and-verify.md), [Manual UltraVNC verification](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/verify-ultravnc-manually.md).
- Example configuration: [examples/relay.toml](https://github.com/kayden-kim/vnc-relay/blob/main/examples/relay.toml). It lists every configuration key.
- Terms: [GLOSSARY.md](https://github.com/kayden-kim/vnc-relay/blob/main/GLOSSARY.md).
- Distribution packages: [GitHub Releases](https://github.com/kayden-kim/vnc-relay/releases).
