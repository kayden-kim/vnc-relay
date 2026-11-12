# Connecting VNC clients

Use this page to choose a connection mode for your situation and to configure UltraVNC over TCP, noVNC over WSS, WS and WSS Server roles, and Mode I target connections.

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

These examples follow `main` and leave access ticket checks disabled. To enable tickets, prepare the separate client integration in [Access tickets](access-ticket.md). The noVNC URL below does not send a ticket by itself.

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

`output` is the file that receives appended operations logs. To write them to standard output, change the value to `"stdout"`. Omitting `output` also selects standard output. See [Configuration](configure.md#management-status-and-records) for details.

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

> **Note:** Release `v2026.11.2` uses `/relay` for the Viewer path. You must change `path=relay/viewer` to `path=relay` in the URL above. Do not mix this path with the `main` path. See [Introduction](index.md) for the differences.

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
