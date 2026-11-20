# Introduction

Use this help to install VNC Relay, connect VNC clients, and operate the relay through its dashboard.

## What VNC Relay does

`vnc-relay` is a VNC Relay compatible with UltraVNC Repeater. One executable provides TCP, WS, and WSS relay entry points and a built-in dashboard. WS means WebSocket. WSS means WebSocket encrypted with TLS.

| Feature | How it works |
| --- | --- |
| Mode I | The Viewer specifies a target address. VNC Relay connects to an allowed VNC Server. VNC Relay must be able to connect to the Server's VNC port. |
| Mode II | The Viewer and Server each connect to VNC Relay. VNC Relay pairs the two roles with the same relay ID. It works even when the Server cannot accept incoming connections from outside. |
| Dashboard | Sign in with an administrator account to view waiting connections, active sessions, recently closed sessions, metrics, connection sources, recent logs, and settings. You can also terminate active sessions and edit dynamic settings. |

To choose a mode for your situation, see [Choose a connection mode](connect.md#choose-a-connection-mode).

The project is an independent Rust implementation. It does not copy, translate, or port UltraVNC, websockify, or noVNC source code. It uses the [MIT license](https://github.com/kayden-kim/vnc-relay/blob/main/LICENSE).

noVNC is not included. It is an external Viewer client that you must provide separately. The dashboard is not a noVNC page for displaying a remote desktop.

See the [repository README](https://github.com/kayden-kim/vnc-relay/blob/main/README.md) for the product scope and independent implementation policy.

## Intended readers

This help is for operators who deploy the executable and configuration files directly. It covers Windows 11 x64 and Ubuntu 24.04 LTS x64, including Windows service and tray operations. It does not cover containers.

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
| Connect a browser Viewer and an Android Server with WSS and access tickets | [Recommended setup](recommended-setup.md) |
| Investigate startup and connection failures | [Troubleshooting](troubleshooting.md) |
| Find the CLI, configuration, and protocol contracts | [Reference](reference.md) |
| Check the license and third-party notices | [Notices](notices.md) |
