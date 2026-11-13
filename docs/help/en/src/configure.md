# Configuration

Use this page to understand the TOML configuration, configure listeners and access policies, and distinguish changes that do not require a restart.

## Read the configuration file

Settings come from one UTF-8 TOML file. Select it with `--config` when starting the server. There are no configuration environment variables or command-line value overrides.

Relative paths start at the configuration file's directory, not the working directory. On Windows, `/` or TOML single-quoted strings are recommended for paths. Write times as integer seconds and sizes as integer bytes. Unknown keys and incorrect types are errors.

The [distribution example](https://github.com/kayden-kim/vnc-relay/blob/main/examples/relay.toml) is a starting point. It lists every configuration key. Keys it does not use appear as `# key = value` comment lines with the default or a sample value. To set such a key, delete the leading `#` and space, then choose the value. The examples below are excerpts, not complete files. For required keys, defaults, and ranges, use the [configuration contract, §§1.1–1.2](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md).

| Table | Controls |
| --- | --- |
| `[admin]`, `[admin.argon2]` | Administrator account file and parameters for new password hashes |
| `[relay.tcp_viewer]`, `[relay.tcp_server]` | Fixed-role TCP listeners and source IP allow lists |
| `[relay.ws]`, `[relay.wss]` | Shared listener addresses, trusted proxies, and WSS TLS files |
| `[relay.ws.viewer]`, `[relay.ws.server]`, `[relay.wss.viewer]`, `[relay.wss.server]` | Role enablement, source IP allow lists, and Origin allow lists |
| `[relay.ticket.viewer]`, `[relay.ticket.server]` | Optional access ticket checks per role |
| `[mode1]`, `[mode2]` | Relay mode enablement and Mode I target allow policy |
| `[limits]` | Connection, waiting connection, buffer, and WebSocket message limits |
| `[timeouts]` | Handshake, upgrade, waiting, target connection, cleanup, and shutdown grace times |
| `[keepalive]` | TCP keepalive and WebSocket ping/pong |
| `[management]`, `[management.login_limit]`, `[management.session]` | Management listener, configuration editing, login limits, and management session lifetimes |
| `[dashboard]` | Dashboard polling interval and query timeout |
| `[recent_closed]`, `[recent_logs]` | Memory limits for recently closed sessions and recent logs |
| `[status]` | Status listener for liveness and readiness checks |
| `[log]` | Operations log output destination |
| `[reload]` | File-change result deadline and fallback polling interval |

## Listeners and roles

TCP Viewer and TCP Server each use a separate listener. Set `enabled`, `listen`, and `allow` in `[relay.tcp_viewer]` and `[relay.tcp_server]`. `listen` must use an IP address. Host names are not accepted. Enclose IPv6 addresses in brackets when adding a port.

WS and WSS each accept both roles on one listener. Set `listen` in `[relay.ws]` or `[relay.wss]`. Enable each role in its child table. The listener opens when at least one role is enabled.

- `/relay/viewer`: Viewer role. Accepts Mode I and Mode II when tickets are off.
- `/relay/server`: Server role. Accepts Mode II only.

The roles share an address and port but have separate access policies. WS and WSS `listen` values must include a port. See [Connecting VNC clients](connect.md) for client configuration.

> **Note:** In `v2026.11.2` and earlier, the Viewer path is `/relay`, and there is no access ticket check. See [Introduction](index.md) for version differences.

## Source IP allow lists

Each entry point's `allow` accepts single IPs, CIDRs, and inclusive ranges in `a-b` form. IPv4 and IPv6 are supported. Items must not contain whitespace.

```toml
[relay.tcp_viewer]
enabled = true
listen = "127.0.0.1:5901"
allow = ["127.0.0.1", "192.0.2.0/24", "198.51.100.10-198.51.100.20", "2001:db8::/32"]
```

- Omitting `allow` for an enabled entry point causes startup failure.
- `allow = []` rejects every source.
- `0.0.0.0/0` and `::/0` are accepted, but produce `allow_all_source` warnings at startup and reload.
- CIDR host bits must be zero. A range's start must not exceed its end, and both addresses must use the same IP family.

TCP checks the actual socket peer IP. At acceptance, WS and WSS check whether the peer is a trusted proxy or belongs to the union of enabled roles' `allow` lists. At upgrade, the selected role's `allow` is checked against the client address. A trusted proxy connection uses the forwarded address.

For the full syntax, use the [configuration contract, §1.3](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md).

## Origin allow lists

Each WS and WSS role's `allowed_origins` restricts the browser page's origin. Only `scheme://host[:port]` is accepted. The scheme is `http` or `https`. Paths, trailing `/`, queries, and wildcards are not allowed.

```toml
[relay.wss.viewer]
enabled = true
allow = ["192.0.2.0/24"]
allowed_origins = ["https://viewer.example.test"]
```

Scheme and host comparisons are case-insensitive. Omitting a default port is equivalent to specifying it. Otherwise, the normalized scheme, host, and port must all match. A subdomain is a different host.

`allowed_origins = []` rejects every request that has Origin. Without Origin, only this comparison is skipped. IP checks and enabled ticket checks still apply. Origin is not user authentication. For exact comparisons, use the [configuration contract, §1.4](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md).

## TLS files and trusted proxies

When any WSS role is enabled, `[relay.wss]` must contain both `cert_file` and `key_file`. Management HTTPS uses the same two keys under `[management]`.

`cert_file` is a PEM certificate chain. Put the server certificate first, followed by intermediate certificates. `key_file` is the matching PEM private key. PKCS#8 is recommended. Do not put private key contents in the configuration file. Restricting private key reads to the server's OS account is recommended. File formats follow the [configuration contract, §1.9](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md).

A proxy can terminate TLS before relay WS or WSS. The listener's `trusted_proxies` must contain only proxy IPs, not client ranges. It uses the same item syntax as `allow`. An empty list trusts no forwarded headers.

For trusted proxy connections, `X-Forwarded-For` is read from right to left. The first non-trusted address becomes the forwarded address and is checked against the role's `allow`. A missing or empty header, an invalid inspected element, or a chain containing only trusted proxies causes rejection. Logs retain the actual socket peer IP.

> **Warning:** A broad `trusted_proxies` list gives clients permission to supply forwarded addresses. An all-address item produces `trust_all_proxies`. Direct connections from a trusted proxy IP also require a valid forwarded header.

This support applies only to relay WS and WSS. The management listener ignores forwarded headers. Adding a proxy does not change the management HTTPS and loopback HTTP rules. Operators must protect the proxy-to-VNC Relay segment. Details follow the [WebSocket contract, §1.8](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md) and [ADR-0007](https://github.com/kayden-kim/vnc-relay/blob/main/docs/decisions/ADR-0007-proxy-tls-termination.md).

## Mode I target allow policy

`mode1.allow_targets` restricts the targets and ports that VNC Relay can connect to. It is separate from source IP allow lists. Enabling Mode I without this key causes startup failure. An empty array rejects every target.

```toml
[mode1]
enabled = true
allow_targets = ["192.0.2.20:5900", "server.example.test:5900"]
```

An IP target must match an IP-and-port item. A DNS target must match both a name-and-port item and an item for its final connection IP and port. In this example, the name satisfies both conditions if it resolves to `192.0.2.20`. Other resolved IPs need separate permission.

For target and port ranges and IPv6 notation, use the [configuration contract, §1.5](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md). The policy must be narrow enough to avoid becoming a public proxy to arbitrary targets.

## Limits, timeouts, and keepalive

`[limits]` restricts resource use. `relay_connections` counts accepted relay connections and Mode I target connections. `admin_connections` imposes one shared limit on management and status connections, separate from relay connections. `waiting` limits Mode II waiting connections. `buffer_bytes` and `ws_message_bytes` limit the relay buffer per connection and logical WebSocket messages respectively.

`[timeouts]` prevents unfinished procedures from retaining resources indefinitely. It controls the handshake, TLS and HTTP upgrade, Mode II wait, Mode I target connection, close exchange, and graceful shutdown period separately.

`[keepalive]` detects unresponsive connections. TCP keepalive also applies to the TCP layer of WS and WSS. WS and WSS also use ping/pong checks. An idle connection remains open if the peer responds. This is not an active-session idle timeout.

Before changing values, check the defaults, ranges, and value relationships in the [configuration contract, §1.2](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md).

## Management, status, and records

The management listener serves the dashboard and management API. HTTPS is the rule. Omitting both `management.cert_file` and `management.key_file` selects plain HTTP. Plain HTTP must bind only to loopback. Administrator sign-in remains required on loopback.

The status listener serves only `/livez` and `/readyz` over plain HTTP. It has no authentication. It does not return sessions, relay IDs, or peer addresses. Loopback is recommended. The distribution example uses management `127.0.0.1:18080` and status `127.0.0.1:18081`. These are example addresses, not defaults.

`log.output` is `"stdout"` or a file path. The default is `"stdout"`. Operations logs use JSON Lines. Files are appended to. When an operator replaces the file, the same path is reopened. Rotation and long-term retention are the operator's responsibility.

`recent_logs.max_entries` controls a separate in-memory list. `0` disables retention. Restarting clears the list. The list is independent of the output destination and output success. Recent logs are not long-term storage. `[recent_closed]` controls the list of active sessions whose cleanup has completed.

See [Operations](operate.md) for health checks and log retention procedures.

## Reload and dynamic configuration editing

The server automatically detects changes to the configuration file, TLS files in use, and public keys files for enabled ticket tables. There are 43 reloadable keys. They include IP, Origin, and trusted proxy policies, TLS files, ticket settings, Mode I target policy, limits, timeouts, and keepalive. Use the [configuration contract, §1.8](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md) for the exact list and application boundaries.

New values apply from the next check or request for each key. Completed checks, started timers, and configured socket options retain their previous results. Existing relay sessions continue.

The dashboard's "View settings" shows settings in effect and file status. Set `management.config_edit = true` and restart to enable "Edit dynamic settings". Editing is off by default. Edits are saved to the same TOML file and applied to running settings. The server account needs permission to write the file and create temporary files and replace files in that directory.

There are 37 editable keys. They exclude four TLS paths and two ticket public keys file paths from the reloadable set. These paths are view-only. Renew certificates and public keys by replacing file contents at the same path. Keys owned by roles or modes also require the activation conditions defined by the contract. See [Dashboard](dashboard.md) for screen procedures.

> **Warning:** If a valid configuration file retains even one restart-required difference, file-change detection rejects the entire candidate as `reload_restart_required`. Leaving that difference in the file also blocks later certificate and public key replacements. Dynamic configuration editing returns HTTP 409 `config_restart_pending` ([management API contract, §1.14](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/management-api-v1.md)). Revert the value or gracefully stop and restart the server.

Listener addresses, listener/role/mode enablement, management HTTP/HTTPS transitions, account settings, management login limits and session lifetimes, `management.config_edit`, dashboard settings, retention settings, log settings, and file-watch settings require a restart. Ticket table `enabled` is an exception: it is reloadable. Invalid candidates produce `reload_rejected`; the previous valid settings and connections remain in use.

## Migrate from v3 to v4

v4 rejects the old `[relay.ws_viewer]` and `[relay.wss_viewer]` tables. Listener settings and role settings must be separated.

1. Prepare a copy separate from the file used by the running server.
2. Update the copy using the mapping in the [configuration contract, §3.3](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md).
3. Gracefully stop the server.
4. Replace the configuration and executable together with their v4 versions.
5. Start the server with the new configuration.

The Viewer path must also match the executable's WebSocket contract. `main` uses `/relay/viewer`. For rollback, follow the same contract section. See [Access tickets](access-ticket.md) for ticket configuration.
