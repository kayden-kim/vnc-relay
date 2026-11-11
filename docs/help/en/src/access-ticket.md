# Access tickets

Use this page to enable access ticket checks for WS and WSS roles, prepare the issuer and clients, and rotate public keys.

> **Note:** Access ticket checks exist only in `main`. They are not in the latest Release, `v2026.11.2`. Checks are off by default. See [Introduction](index.md) for the version baseline.

## What an access ticket does

An access ticket is a one-time connection permission signed by an external issuer. VNC Relay verifies its signature with a public keys file for the role. WS and WSS upgrades for an enabled role must present a valid ticket. There is no mode that checks tickets only when supplied.

Each ticket is bound to one role and one relay ID. A Viewer ticket cannot authorize the Server role. After upgrade, the handshake relay ID must also match the ticket's `rid`. A relay ID selects a pair; it is not a secret proof of access permission.

Tickets are separate from management sessions. An administrator sign-in cookie cannot replace a ticket. Source IP allow lists and Origin checks still apply. For terms, see the [glossary](https://github.com/kayden-kim/vnc-relay/blob/main/GLOSSARY.md). For the decision, see [ADR-0009](https://github.com/kayden-kim/vnc-relay/blob/main/docs/decisions/ADR-0009-optional-relay-access-ticket.md).

## Paths that tickets do not protect

TCP entry points cannot check tickets. If the TCP entry point for a ticket-enabled role is also enabled, startup and reload produce `ticket_tcp_bypass`. The warning does not block TCP connections or reject startup or reload.

Plain WS does not encrypt the ticket. Enabling tickets and WS for the same role produces `ticket_plaintext_ws`.

> **Warning:** Plain WS must be used only inside a trusted network. WSS is recommended for external access. If a proxy terminates TLS, protect the proxy-to-VNC Relay segment too. To require tickets for every connection of a role, do not leave that role's TCP entry point open.

See [Configuration](configure.md) for listeners and IP, Origin, and trusted proxy policies.

## Prepare the issuer and clients

The issuer must use different P-256 signing keys for each role. Only the issuer keeps private keys. VNC Relay receives public keys files only. When ticket checks are enabled for both roles, the files must not contain the same public key.

The issuer creates a compact JWS ticket signed with ES256. Its structure is a header, payload, and signature, each encoded as unpadded base64url and joined by dots. This help contains no sample ticket or signing key values.

- The header needs `alg = ES256` and a `kid` that selects the public key.
- The payload needs `iss`, `aud`, `rh`, `rid`, `jti`, `iat`, and `exp`.
- `iss` must match the role's `issuer`; `rh` must match its `relay_host`. `rh` is not compared with the request's `Host`.
- `aud` is `vnc-relay/viewer` for Viewer and `vnc-relay/server` for Server.
- The issuer must generate `jti` with at least 128 bits of randomness for each ticket. The use record rejects a second use of the same role and `jti` pair.
- `iat` and `exp` are the issue and connection expiration times, written as nonnegative integer Unix seconds. `iat < exp` is required. `exp − iat` must not exceed the role's `max_lifetime_secs`.
- Optional `sx` is the session expiry time, written as nonnegative integer Unix seconds. At upgrade, the current time must be less than `sx`, with no five-second tolerance. It ends the connection even during the handshake, waiting, or active state. Unlike `exp`, it remains an end time after upgrade. Without `sx`, the ticket does not set a connection end time.
- Optional `sub` can appear in operations logs. It must not contain secrets or personal information.

The issuer and VNC Relay clocks must be within five seconds. Verification requires `iat ≤ current time + 5 seconds` and `current time < exp + 5 seconds`. The server rejects tickets with `iat ≤ process startup time + 5 seconds`. After startup or restart, obtain a new ticket with `iat > process startup time + 5 seconds`. Reload does not change the startup time.

For the full format and verification order, use the [WebSocket contract, §§1.9.2–1.9.4](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md).

## Enable checks per role

Prepare the issuer and ticket-capable clients first. Existing clients that connect without tickets will be rejected after checks are enabled. The excerpt below configures Viewer. For Server, use the same types of settings under `[relay.ticket.server]`, with a public keys file for different signing keys.

1. Prepare the role's public keys file.
2. Add the role's ticket table to the configuration file.

   ```toml
   [relay.ticket.viewer]
   enabled = true
   issuer = "issuer.example.test"
   relay_host = "relay.example.test"
   public_keys_file = "keys/viewer-public.jwks.json"
   max_lifetime_secs = 60
   ```

3. If the server is running, wait for the reload result from file-change detection. Otherwise, start it using [Operations](operate.md).
4. Check for `reload_applied`, or `server_ready` at startup, in the operations log.
5. Obtain a new ticket and connect with the prepared client.

`enabled` turns on checks for the same role on both WS and WSS. `issuer`, `relay_host`, and `public_keys_file` are required when enabled. `max_lifetime_secs` limits the ticket's connection validity period. Its default is 60 seconds and its range is 10–600 seconds. Clock tolerance is fixed at five seconds and has no setting.

Relative public key paths start at the configuration file's directory. An invalid file causes startup failure with `ticket_keys_invalid`. During reload, the candidate is rejected and the previous valid settings and public keys remain in use.

For keys and application conditions, use the [configuration contract, §§1.2 and 1.8](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md).

### Public keys file structure

`public_keys_file` is a JWKS file containing a UTF-8 JSON object. It must be at most 64 KiB. Its `keys` array contains 1–16 public keys.

Each key must have `kty` set to `EC` and `crv` set to `P-256`. `kid` is a 1–128-byte string unique within the file. `x` and `y` are unpadded base64url encodings of 32-byte coordinates. The coordinates must describe a point on P-256.

If present, `use` must be `sig` and `alg` must be `ES256`. The private key member `d` must not be present. Full file rules follow the [configuration contract, §1.11](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md).

## What the client sends

The client sends two values in the upgrade request's `Sec-WebSocket-Protocol`. One is `binary`. The other is exactly one value containing lowercase `ticket.` followed by the compact JWS. With a placeholder, the header structure is `Sec-WebSocket-Protocol: binary, ticket.<JWS>`. Do not send the literal placeholder `<JWS>`.

Browser noVNC sends the values through `wsProtocols`. Do not put the ticket in a URL query, `Authorization`, or an administrator cookie. The server selects only `binary` in a successful response. It does not echo the ticket value.

Use a new ticket for every connection attempt, including retries. Do not resend the same ticket after a 401. A ticket remains consumed if sending 101 fails after the use record has been written. Client exchange rules follow the [WebSocket contract, §§1.9.1 and 1.9.7](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md).

## Rejection and session expiry

Missing, invalid, expired, pre-startup, and reused tickets are rejected with HTTP 401 before 101. The response includes `WWW-Authenticate: VncRelayTicket`. Earlier failures, such as request syntax, path, Origin, or subprotocol failures, take precedence. Sending a ticket without `binary` produces HTTP 400.

If a ticket-authorized connection sends a different relay ID during the handshake, it is rejected with Close 4012 `ticket_relay_mismatch`. Viewer Mode I is not allowed either. If Mode I is disabled, the existing mode rejection, Close 1008, takes precedence. If Mode I is enabled and existing syntax checks pass, the request gets Close 4012. No target connection is attempted.

At `sx`, the WS or WSS endpoint receives Close 4013 `session_expired`. The other side of an active session also ends. If both tickets have `sx`, the earlier time wins. After cleanup, an active session appears in recently closed sessions with close reason `session_expired`. Waiting connection expiry is recorded in the operations log.

For all response and closure rules, use the [WebSocket contract, §§1.9.3–1.9.5](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md). See [Troubleshooting](troubleshooting.md) when investigating a failure.

## Rotate public keys

The server detects changes to enabled roles' public keys files and reloads them. Both roles' file paths and contents are reloadable. The dashboard cannot edit these paths.

1. Add the new public key to the file at the same path.
2. Confirm `reload_applied` before using the new key.
3. Switch the issuer to the new signing key.
4. After switching the issuer, wait the role's `max_lifetime_secs + 10 seconds`, then remove the old public key from the file.
5. Confirm `reload_applied` again.

An invalid candidate produces `reload_rejected` and keeps the previous public keys. A pending restart-required configuration difference also rejects the entire candidate. See [Configuration](configure.md) for the conditions.

Key rotation, reload, and toggling ticket checks do not clear use records. Relay ID checks and `sx` timers for connections that have already received 101 remain unchanged. Rotation order follows the [configuration contract, §1.11](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md).

## Check the dashboard and logs

In "View settings", "Access tickets" shows both roles' keys. "Edit dynamic settings" edits each role's `enabled`, `issuer`, `relay_host`, and `max_lifetime_secs`. That role must be enabled on at least one WS or WSS listener. Public keys files are shown only as paths, never as contents.

Recently closed sessions display `session_expired` as "Session expired". See [Dashboard](dashboard.md) for screen procedures.

In operations logs, check `ticket_accepted`, ticket-related `reason` values in `connection_rejected`, `connection_aborted`, `waiting_removed`, and `session_closed`. Expiry at `sx` produces `connection_aborted` during the handshake, `waiting_removed` while waiting, or `session_closed` for an active session. Ticket contents and public keys file contents do not appear in logs, diagnostics, status, or management API responses. For fields and the full event list, use the [operations information contract, §§1.4–1.6 and 1.11](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md).
