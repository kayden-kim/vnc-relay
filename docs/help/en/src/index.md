# Introduction

Use this help to install VNC Relay, connect VNC clients, and operate the relay through its dashboard.

## What VNC Relay does

`vnc-relay` is a VNC Relay compatible with UltraVNC Repeater. One executable provides TCP, WS, and WSS relay entry points and a built-in dashboard. WS means WebSocket. WSS means WebSocket encrypted with TLS.

| Feature | How it works |
| --- | --- |
| Mode I | The Viewer specifies a target address. VNC Relay connects to an allowed VNC Server. |
| Mode II | The Viewer and Server each connect to VNC Relay. VNC Relay pairs the two roles with the same relay ID. |
| Dashboard | Sign in with an administrator account to view waiting connections, active sessions, recently closed sessions, recent logs, and settings. You can also terminate active sessions and edit dynamic settings. |

The project is an independent Rust implementation. It does not copy, translate, or port UltraVNC, websockify, or noVNC source code. It uses the [MIT license](https://github.com/kayden-kim/vnc-relay/blob/main/LICENSE).

noVNC is not included. It is an external Viewer client that you must provide separately. The dashboard is not a noVNC page for displaying a remote desktop.

See the [repository README](https://github.com/kayden-kim/vnc-relay/blob/main/README.md) for the product scope and independent implementation policy.

## Intended readers

This help is for operators who deploy the executable and configuration files directly. It covers Windows 11 x64 and Ubuntu 24.04 LTS x64. It does not cover OS service registration or containers.

## Where to start

| Task | Page |
| --- | --- |
| Download a bundle and verify its hash | [Installation](install.md) |
| Run the relay on loopback for the first time | [Quick start](quick-start.md) |
| Connect UltraVNC and noVNC with Mode I or II | [Connecting VNC clients](connect.md) |
| Configure listeners, access restrictions, and TLS | [Configuration](configure.md) |
| Sign in and work with connections, sessions, logs, and settings | [Dashboard](dashboard.md) |
| Integrate connection authorization with an external issuer | [Access tickets](access-ticket.md) |
| Start, stop, reload, and upgrade the relay | [Operations](operate.md) |
| Investigate startup and connection failures | [Troubleshooting](troubleshooting.md) |
| Find the CLI, configuration, and protocol contracts | [Reference](reference.md) |
| Check the license and third-party notices | [Notices](notices.md) |

## Baseline

This help follows the repository's `main` branch. The [latest Release](https://github.com/kayden-kim/vnc-relay/releases/latest) is `v2026.11.2`, published on 2026-11-09. Download bundles from [GitHub Releases](https://github.com/kayden-kim/vnc-relay/releases).

A build from the current `main` still reports version `2026.11.2` until the next release. Do not use the version output alone to distinguish the two behaviors below.

| Difference | Behavior on `main` | For users of Release `v2026.11.2` |
| --- | --- | --- |
| WS and WSS Viewer path | Uses `/relay/viewer`. Does not accept the old `/relay` path. | You must use `/relay`. In this help's noVNC URLs, change `path=relay/viewer` to `path=relay`. Do not change TCP or `/relay/server`. |
| Opt-in access ticket checks | Enable separately for each role in `[relay.ticket.viewer]` or `[relay.ticket.server]`. Checks apply to both WS and WSS for that role, not TCP. Checks are off by default. Related operations logs, dashboard ticket configuration inputs, and display of the `session_expired` close reason are also available. | This feature is unavailable. Do not add the ticket configuration tables. Build a bundle from `main` to use it. |

The Viewer path follows sections 1.7 and 3.1 of the [WebSocket contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/websocket-relay-v2.md). Access tickets follow section 1.9 of that contract and section 1.2 of the [configuration contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/config-v4.md). Related logs follow the [operations information contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/ops-info-v2.md).

Dashboard ticket inputs and close reason display follow SCR-005 and SCR-001 in the [dashboard specification](https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/dashboard.md).
