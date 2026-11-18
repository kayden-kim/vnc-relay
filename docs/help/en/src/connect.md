# Connecting VNC clients

Use this page to choose a connection mode for your situation and to configure UltraVNC over TCP, noVNC over WSS, WS and WSS Server roles, and Mode I target connections. It also shows a recommended setup that uses WSS and access tickets for both roles.

## Choose a connection mode

The two modes differ in which side opens the connection between VNC Relay and the Server. In Mode II, the Server connects to VNC Relay (a reverse connection). In Mode I, VNC Relay connects to the Server.

```text
Mode II: Viewer --> VNC Relay <-- Server
Mode I:  Viewer --> VNC Relay --> Server
```

| Mode | Viewer action | Server action |
| --- | --- | --- |
| Mode II | Connects to VNC Relay's Viewer entry point with a relay ID. | Connects back to the Server entry point with the same relay ID. |
| Mode I | Sends a target address to VNC Relay's Viewer entry point. | Listens on the target VNC port. Does not connect back to VNC Relay. |

### Choose by situation

| Situation | Mode | Reason |
| --- | --- | --- |
| As in remote support, the Server PC is behind a NAT router or firewall and cannot accept incoming connections from outside. | Mode II | The Viewer and the Server both connect to VNC Relay. The Server does not need to accept incoming connections. |
| VNC Relay can connect to the Server's VNC port. For example, VNC Relay and the Server are on the same internal network, and only the Viewer is outside. | Mode I | The Server listens on its VNC port without reverse connection settings. VNC Relay connects to the target that the Viewer sends only if the target allow policy permits it. |
| You connect with noVNC in a browser. | Mode II | The verified noVNC 1.7.0 sends the relay ID from `repeaterID`. It does not send a Mode I request with a target address. |
| A Server-side client connects over WS or WSS. | Mode II | The Server role accepts only Mode II on every transport. |
| You enable access ticket checks for Viewer WS or WSS connections. | Mode II | A Viewer role with ticket checks enabled rejects Mode I requests. |

See section 5 of the [PRD](https://github.com/kayden-kim/vnc-relay/blob/main/docs/requirements/PRD.md) for the usage scenarios.

### Before you use Mode II

Place VNC Relay where both the Viewer and the Server can reach it. The Viewer and Server do not connect directly to each other. Either role can connect first. If no peer arrives, the waiting connection ends after `timeouts.wait_secs`.

Agree on the relay ID for both roles in advance. A relay ID is not a secret that proves permission to connect. You must restrict the source IP allow list of each entry point to the intended scope. You can enable [access ticket](access-ticket.md) checks for WS and WSS roles.

One relay ID can pair only one Viewer with one Server at a time. A connection that uses a relay ID already held by the same role is rejected. See [Understand duplicate ID rejection](#understand-duplicate-id-rejection) for details.

### Before you use Mode I

VNC Relay must be able to connect to the target Server's VNC port. The Server must listen on that port. If VNC Relay cannot connect to the target within `timeouts.connect_secs`, it closes the Viewer connection.

When Mode I is enabled, any client that can reach the Viewer entry point can connect to allowed targets through VNC Relay. Therefore, you must list only the required targets and ports in `mode1.allow_targets`. You must also restrict the source IP allow list of the Viewer entry point to the intended scope. For targets given by name, follow [Mode I target allow policy](configure.md#mode-i-target-allow-policy).

To use Mode I over WS or WSS, you need a WebSocket client that sends the target address in the initial request. UltraVNC Viewer does not use WebSocket.

## Mode II example configuration

These examples follow `main` and leave access ticket checks disabled. To enable tickets, prepare the separate client integration in [Access tickets](access-ticket.md). The noVNC URL below does not send a ticket by itself. For a setup with tickets on both roles, see [Recommended setup](#recommended-setup-wss-and-access-tickets-on-both-sides).

The example addresses are for documentation. For your deployment, you must replace the hosts, IPs, source IP allow lists, Origin, and certificate paths.

| Component | Example value |
| --- | --- |
| VNC Relay | `relay.example.test`, `192.0.2.20` |
| Viewer PC | `192.0.2.10` |
| Server PC | `192.0.2.30` |
| Site serving the noVNC page | `https://viewer.example.test` |
| Relay ID | `1234` |

This complete `relay.toml` uses the [README connection example](https://github.com/kayden-kim/vnc-relay/blob/main/README.md). Prepare it as a separate configuration. Do not append it unchanged to the loopback file from [Quick start](quick-start.md).

```toml
[admin]
accounts_file = "admin-accounts.toml"

[relay.tcp_viewer]
enabled = true
listen = "0.0.0.0:5901"
allow = ["192.0.2.10"]

[relay.tcp_server]
enabled = true
listen = "0.0.0.0:5500"
allow = ["192.0.2.30"]

[relay.wss]
listen = "0.0.0.0:8443"
cert_file = "tls/relay-chain.pem"
key_file = "tls/relay-key.pem"

[relay.wss.viewer]
enabled = true
allow = ["192.0.2.10"]
allowed_origins = ["https://viewer.example.test"]

[mode2]
enabled = true

[management]
listen = "127.0.0.1:18080"

[status]
listen = "127.0.0.1:18081"

[log]
output = "relay.log"
```

`allow` restricts the source IPs that can connect to each entry point. Sources outside the list are rejected. This example uses direct connections without a trusted proxy.

`cert_file` and `key_file` point to a certificate chain and private key for `relay.example.test`. You must provide a certificate that the browser trusts. Paths resolve from the directory containing `relay.toml`. Do not put the private key contents in the configuration file.

`output` is the output destination for operations logs. It takes a file path or `"stdout"`. This example appends the logs to the `relay.log` file. `"stdout"` writes them to standard output. Omitting `output` also selects standard output. See [Configuration](configure.md#management-status-and-records) for details.

1. Prepare `relay.toml` in the operational directory.
2. Provide the certificate chain and private key files specified in the configuration.
3. Create an administrator account using [Quick start](quick-start.md).
4. Start VNC Relay using [Operations](operate.md).

> **Warning:** This example binds relay listeners to all IPv4 interfaces. Before starting, you must restrict firewall rules and source IP allow lists to the intended scope. Keep management HTTP and the status listener on loopback. See [Configuration](configure.md) for public deployment preparation.

## Connect UltraVNC over TCP

The UltraVNC command forms were verified with UltraVNC 1.8.3.0. Run these PowerShell commands from the UltraVNC installation directory on each PC. VNC Relay can run on either supported OS. These commands are for Windows UltraVNC clients, not Ubuntu clients.

| Role | PC | Destination | Command to use |
| --- | --- | --- | --- |
| UltraVNC Server | `192.0.2.30` | TCP Server `192.0.2.20:5500` | Server command below |
| UltraVNC Viewer | `192.0.2.10` | TCP Viewer `192.0.2.20:5901` | Viewer command below |
| noVNC Viewer | Browser on `192.0.2.10` | WSS Viewer `relay.example.test:8443/relay/viewer` | noVNC URL in the next section |

1. Start the reverse connection on the Server PC.

   ```powershell
   .\winvnc.exe -id:1234 -connect 192.0.2.20::5500 -run
   ```

2. Connect from the Viewer PC with the same relay ID.

   ```powershell
   .\vncviewer.exe -proxy 192.0.2.20:5901 ID:1234
   ```

This order is an example. You can start the Viewer first. The `::5500` and `:5901` forms belong to their respective UltraVNC options. Do not make them identical.

See section 6 of the [compatibility verification record](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/repeater-compat-record.md) for the tested scope. Real UltraVNC was tested with Windows VNC Relay. That record marks real UltraVNC with Ubuntu VNC Relay as not run, separately from Ubuntu automated tests.

## Connect noVNC over WSS

noVNC is an external client that you must provide separately. VNC Relay does not serve noVNC files. This example assumes you serve noVNC at `https://viewer.example.test`. The verified noVNC version is 1.7.0.

1. Connect the Server with the Server command from the TCP section.
2. Open this URL in the Viewer PC's browser. Do not connect simultaneously with an UltraVNC Viewer using the same ID.

   ```text
   https://viewer.example.test/vnc.html?autoconnect=true&host=relay.example.test&port=8443&encrypt=true&path=relay/viewer&repeaterID=1234
   ```

| URL value | Meaning |
| --- | --- |
| `host=relay.example.test&port=8443` | VNC Relay's WSS listener |
| `encrypt=true` | Use WSS |
| `path=relay/viewer` | Viewer role path `/relay/viewer` |
| `repeaterID=1234` | Mode II relay ID. noVNC adds `ID:` and sends it in the initial request. |

> **Note:** Releases `v2026.11.2` and earlier use `/relay` for the Viewer path. When you connect to one of those versions, change `path=relay/viewer` to `path=relay` in the URL above. See [Introduction](index.md) for the differences.

The browser sends the origin of the site serving the noVNC page in the `Origin` header. In this example, it is `https://viewer.example.test`. Do not confuse it with the WSS destination, `relay.example.test`.

Set `allowed_origins` per role. An empty list rejects every request that has an `Origin` header. A client without that header skips only Origin comparison. IP checks and any access ticket checks enabled for that role still apply. Origin is not user authentication and does not replace the source IP allow list.

See section 1.4 of the [configuration contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md) for exact Origin syntax and comparison rules.

## Connect a WS or WSS Server role

The Server role uses `/relay/server` and accepts Mode II only. Add this table to the configuration above to enable the WSS Server role.

```toml
[relay.wss.server]
enabled = true
allow = ["192.0.2.30"]
allowed_origins = []
```

This list is for Server clients without `Origin`. For a browser client, you must list the origin of its page.

For a plaintext WS Server on a protected network, you can separately add these tables.

```toml
[relay.ws]
listen = "192.0.2.20:8081"

[relay.ws.server]
enabled = true
allow = ["192.0.2.30"]
allowed_origins = []
```

| Transport and role | Address | Required client behavior |
| --- | --- | --- |
| WSS Server | `wss://relay.example.test:8443/relay/server` | After WebSocket upgrade, send the Server's `ID:` initial request first as binary data. |
| WS Server | `ws://relay.example.test:8081/relay/server` | Use the same Server initial exchange. The plaintext connection is not encrypted. |

VNC Relay sends no greeting on the Server path. Do not implement the Viewer behavior of waiting for that greeting. You need a WebSocket client for the Server role, not a WSS URL substituted into the UltraVNC TCP command above. Byte formats follow section 1 of the [Repeater initial exchange contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/repeater-handshake-v1.md) and sections 1.4 and 1.7 of the [WebSocket contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md).

For public WebSocket entry points, WSS or TLS termination at a reverse proxy is recommended. It is recommended to enable plaintext WS only on protected networks. You must restart VNC Relay to enable a role after changing the configuration.

## Recommended setup: WSS and access tickets on both sides

This case provides remote support from a web page to Android devices across the internet. Browser noVNC acts as the Viewer, and a modified droidVNC-NG on the device acts as the Server. Both roles connect over WSS and check access tickets. It describes the structure of a setup applied in a real deployment, without service details.

This setup is recommended for these reasons:

- The Viewer and Server share one WSS listener. Only one relay port is open to the internet.
- TLS protects both segments. However, VNC Relay relays decrypted bytes internally, so this is not end-to-end encryption.
- Devices whose source IPs are not known in advance can connect. They still need a ticket that the backend issues after it checks permission.
- When the remote support session expires, VNC Relay ends its connections.
- The endpoints still perform VNC authentication. Tickets do not replace VNC authentication.

### Components

```text
            Backend API (issuer)
  +-------------------+-------------------+
  | Viewer ticket                         | Server ticket
  v                                       v
noVNC ---WSS---> VNC Relay <---WSS--- droidVNC-NG
    /relay/viewer         /relay/server
```

| Component | Runs on | Responsibility |
| --- | --- | --- |
| Remote support web app | Support staff's browser | Uses noVNC as the Viewer. Each time it opens a screen, it gets connection details and a Viewer ticket from the backend API. |
| Backend API (issuer) | Server | Authenticates support staff and devices and manages remote support sessions. Signs tickets with a different key for each role. |
| Device app | Android device | Receives remote support requests. Gets a Server ticket and tells the modified droidVNC-NG to connect. |
| Modified droidVNC-NG | Android device | VNC Server that sends the device screen. Connects back to VNC Relay over WSS. |
| VNC Relay | A host that both roles can reach | Accepts both roles on one WSS listener and checks per-role tickets. Pairs two connections with the same relay ID in Mode II. |

### Connection sequence

1. Support staff start a remote support session in the web app. The backend API picks a new relay ID and sets a session expiry time for each session.
2. The device app receives the remote support request through the existing device management channel. The request contains no ticket.
3. The device app starts the VNC server in the modified droidVNC-NG.
4. The device app requests a Server ticket from the backend API. A ticket lives only 60 seconds, so the app requests it just before connecting. The backend API authenticates the device, checks that the session is valid, and returns a ticket and the WSS address.
5. The device app passes the WSS address, relay ID, and ticket to the modified droidVNC-NG and tells it to connect.
6. The modified droidVNC-NG upgrades on `/relay/server` and sends the `ID:` initial request. VNC Relay checks the ticket and keeps the connection as a waiting connection.
7. When support staff open the remote screen, the web app requests connection details from the backend API. The backend API checks permission and session state, then returns the WSS address, relay ID, VNC password, and a new Viewer ticket.
8. noVNC connects to `/relay/viewer`. VNC Relay checks the source IP, Origin, and ticket, then pairs the Viewer with the Server that has the same relay ID.
9. noVNC and the modified droidVNC-NG complete VNC authentication and exchange the screen. VNC Relay forwards RFB data without interpreting it.
10. At the session expiry time, VNC Relay sends Close 4013 to both connections and ends the active session.

When support staff close the screen, the active session in VNC Relay ends and the Server connection closes too. VNC Relay does not pair ended connections again. To open the screen again, the device must also start a new connection with a new Server ticket. droidVNC-NG does not notify the device app when a connection drops. To detect a drop, the device app must query the connection state with `ACTION_GET_CLIENTS`. The device app in this case does not detect drops after a successful connection. The backend must therefore tell the device to connect again, for example by starting a new remote support session.

### VNC Relay configuration

This is the complete `relay.toml` for this setup. The example addresses are for documentation. For your deployment, you must replace the hosts, source IP allow lists, Origin, issuer name, and file paths.

```toml
[admin]
accounts_file = "admin-accounts.toml"

[relay.wss]
listen = "0.0.0.0:8443"
cert_file = "tls/relay-chain.pem"
key_file = "tls/relay-key.pem"

[relay.wss.viewer]
enabled = true
allow = ["203.0.113.0/24"]
allowed_origins = ["https://viewer.example.test"]

[relay.wss.server]
enabled = true
allow = ["0.0.0.0/0"]
allowed_origins = []

[relay.ticket.viewer]
enabled = true
issuer = "issuer.example.test"
relay_host = "relay.example.test"
public_keys_file = "keys/viewer-public.jwks.json"
max_lifetime_secs = 60

[relay.ticket.server]
enabled = true
issuer = "issuer.example.test"
relay_host = "relay.example.test"
public_keys_file = "keys/server-public.jwks.json"
max_lifetime_secs = 60

[mode2]
enabled = true

[timeouts]
wait_secs = 3600

[management]
listen = "127.0.0.1:18080"

[status]
listen = "127.0.0.1:18081"

[log]
output = "relay.log"
```

| Setting | Reason in this setup |
| --- | --- |
| No TCP entry point tables | TCP entry points are disabled by default. TCP cannot check tickets, so this setup does not open them. |
| Viewer `allow` | Restricted to the network that support staff use. This example uses `203.0.113.0/24`. |
| Server `allow` | Devices connect from various networks whose addresses are not known in advance. The setup allows every IPv4 source and leaves access control to the Server ticket. Startup logs an `allow_all_source` warning. |
| Viewer `allowed_origins` | Allows only the origin of the remote support web app. |
| Server `allowed_origins` | The modified droidVNC-NG does not send an `Origin` header, so the list stays empty. |
| Both ticket tables | They share `issuer` and `relay_host` but use different public keys files. The same public key in both files is rejected. |
| `relay_host` | Not compared with the request's `Host`. One fixed name works even when VNC Relay has several host names. |
| `timeouts.wait_secs` | Maximum time a waiting connection waits for its peer. In this setup, the device connects first and waits for support staff, so it is recommended to set this to at least the longest remote support session. Then a waiting connection that no Viewer joins ends at `sx`. The example's 3600 seconds is the default. |

For `cert_file`, use a certificate from a public CA that both browsers and Android trust. The modified droidVNC-NG verifies the certificate with the Android system trust store and the host name.

1. Get the two per-role public keys files from the issuer and place them in `keys/`.
2. Place the certificate chain and private key in `tls/`.
3. Synchronize the VNC Relay host clock with NTP.
4. Create an administrator account using [Quick start](quick-start.md).
5. Start VNC Relay using [Operations](operate.md).
6. In the operations log, check `server_ready` and the `allow_all_source` warning for the Server role.

### Issuer implementation

The backend API uses a different P-256 signing key for each role. In this case, the private keys stay in a cloud key management service (KMS), and the backend only requests signatures. The public keys are exported to per-role JWKS files on VNC Relay. `kid` holds the key ID from the key management service. Follow [Access tickets](access-ticket.md#public-keys-file-structure) for the file format and key rotation.

If the key management service returns ECDSA signatures in DER format, convert them to the 64-byte R‖S format before you put them in the JWS. VNC Relay rejects DER signatures.

| Claim | Value in this case |
| --- | --- |
| `rid` | Relay ID of the remote support session. This case uses six random digits. |
| `iat`, `exp` | The issue time and 60 seconds after it. |
| `sx` | Expiry time of the remote support session. |
| `sub` | Remote support session ID. It is neither a secret nor personal information. Use `ticket_sub` in the operations log to match it with the backend session. |

`iss` and `rh` hold fixed names that match `issuer` and `relay_host` for both roles. For the other claim rules, follow [Access tickets](access-ticket.md#prepare-the-issuer-and-clients).

The backend API issues a new Viewer ticket for each connection details request and a new Server ticket for each device connection attempt. It does not store tickets in its database or logs.

A ticket's `iat` comes from the backend clock. If the VNC Relay host clock is more than five seconds behind, every new ticket is rejected as `ticket_invalid`. This happened in this case too: a slow host clock rejected the tickets for both roles. You must synchronize the VNC Relay host clock with NTP.

### Viewer side: noVNC

A `vnc.html` URL cannot send a ticket. The web app creates the noVNC `RFB` object itself and passes the ticket through `wsProtocols`. This case used the noVNC 1.7.0 package.

```javascript
const rfb = new RFB(screen, connection.relayUrl, {
  repeaterID: connection.relayId,
  credentials: { password: connection.password },
  wsProtocols: ["binary", `ticket.${connection.ticket}`],
});
```

`connection` holds the connection details from the backend API. `relayUrl` is `wss://relay.example.test:8443/relay/viewer`.

- Get new connection details each time you create an `RFB` object. Do not reconnect with a used ticket.
- Do not keep tickets or VNC passwords in URLs, on the page, in browser storage, or in logs.

### Server side: modified droidVNC-NG and the device app

The original droidVNC-NG connects back to a Repeater over TCP only. This case added these features to droidVNC-NG 2.22.3:

- When it receives a `ws://` or `wss://` address, it connects over WebSocket. Without a path, it uses `/relay/server`.
- It sends `binary` in `Sec-WebSocket-Protocol`. When it receives a ticket, it also sends `ticket.<JWS>`.
- After the upgrade, it sends the 250-byte `ID:` initial request as the first binary message. It does not wait for start bytes from VNC Relay.
- After that, it exchanges RFB data as binary messages only.
- It verifies the certificate with the Android system trust store and the host name.
- A ticket connection does not reconnect automatically when it drops, because a ticket cannot be used twice.
- It refuses to connect when it receives a ticket with a TCP address. This prevents a silent switch to a TCP connection without a ticket.

Parameters arrive through the droidVNC-NG Intent interface (`ACTION_CONNECT_REPEATER`). The existing host value carries the WSS address, and a new value carries the ticket. A socket pair connects the WebSocket client in the app to the existing VNC server engine. No loopback port is opened, so other apps on the device cannot intercept the connection.

The device app is responsible for these tasks:

- Check that the installed droidVNC-NG is a modified build with WebSocket support before passing a WSS address. Do not rely on the version number alone, because a modified build can keep the original version number.
- Get a new Server ticket from the backend API for each connection attempt. If the ticket request or the connect request fails, retry with a new ticket at increasing intervals. This retry does not cover a drop after a successful connection.
- Do not fall back to TCP without a ticket when the WSS connection fails.
- Do not write tickets to logs.

> **Warning:** The device app sends the ticket request itself, so Android background network restrictions apply. In this case, on an Android 16 device, ticket requests failed while the droidVNC-NG screen sharing confirmation was in front. The connection succeeded after the device app returned to the foreground.

> **Note:** droidVNC-NG is licensed under GPL 2.0. If you distribute a modified build to others, you must meet the license terms, such as providing the source code.

### Points to watch in operation

- While you migrate older devices, you can keep a TCP Server entry point open as well. That path does not check tickets and produces the `ticket_tcp_bypass` warning. Close it after the migration.
- Tickets are checked only at connection time. If the backend ends a session early, connections that are already open stay until `sx`. You can end an active session immediately with termination by administrator in the [Dashboard](dashboard.md#session-details-and-termination).
- Restarting VNC Relay drops every connection. Tickets whose `iat` is at or before process startup time + 5 seconds are rejected. After a restart, wait briefly and reconnect with new tickets.
- Each role has only one `issuer` and one `relay_host`. If several environments, such as development and production, share one VNC Relay, each role's public keys file trusts the keys of every environment. A ticket from another environment can then connect if the relay ID matches. To isolate environments, run a separate VNC Relay for each.
- Origin only blocks browsers on other sites. It is not user authentication.

### Verified scope

VNC Relay's noVNC verification (1.7.0) used `vnc.html` without `wsProtocols`. VNC Relay's automated tests do not cover ticket sending from noVNC or the modified droidVNC-NG. In this case's deployment, an Android test device confirmed WSS and ticket connections for both roles and the screen display. Screen input and session expiry were not confirmed on a real device.

## Connect a Mode I target

In Mode I, VNC Relay connects to the Server's VNC port. The source IP allow list and target allow policy are separate. Add this table to the existing `relay.toml`.

```toml
[mode1]
enabled = true
allow_targets = ["192.0.2.30:5900"]
```

`allow_targets` is required when Mode I is enabled. An empty list rejects all targets. This example permits only `192.0.2.30:5900`.

1. Start the Server from the UltraVNC installation directory on the Server PC.

   ```powershell
   .\winvnc.exe -run
   ```

2. Restart VNC Relay to enable Mode I.
3. Specify the target and proxy from the UltraVNC installation directory on the Viewer PC.

   ```powershell
   .\vncviewer.exe -connect 192.0.2.30::5900 -proxy 192.0.2.20:5901
   ```

The Server must listen on port `5900` instead of connecting back. VNC Relay must be able to reach that port. See section 1.5 of the [configuration contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md) for target allow policy syntax.

## Understand duplicate ID rejection

If the same role already occupies a relay ID, the later connection is rejected. This applies to waiting connections and active sessions. The original connection and active session remain unchanged. The rule applies across TCP, WS, and WSS.

For example, if an UltraVNC Viewer is waiting with `1234`, a noVNC Viewer cannot also connect with `1234`. One Server with the same ID is a normal match, not a duplicate. When reconnecting, check that the previous connection has been cleaned up.

See section 2 of the [Repeater initial exchange contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/repeater-handshake-v1.md) for exact rejection behavior. If connection fails, follow [Troubleshooting](troubleshooting.md) and the recent log instructions in [Dashboard](dashboard.md).
