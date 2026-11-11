# Quick start

Use this page to run the loopback example, check readiness and dashboard sign-in, and stop the server gracefully.

## Before you start

Complete [Installation](install.md), then run commands in the extracted bundle directory. The executable and `relay.toml` must be in that directory. The bundled `relay.toml` comes from [examples/relay.toml](https://github.com/kayden-kim/vnc-relay/blob/main/examples/relay.toml) in the repository.

This example accepts connections only from the same PC. It enables TCP Mode II only. WS, WSS, and Mode I are disabled.

| Purpose | Example address |
| --- | --- |
| TCP Viewer | `127.0.0.1:5901` |
| TCP Server | `127.0.0.1:5500` |
| Dashboard and management API | `http://127.0.0.1:18080/` |
| Readiness | `http://127.0.0.1:18081/readyz` |

> **Warning:** This configuration is not for public deployment. Do not use it by changing only the bind addresses to public addresses. Before public deployment, follow [Configuration](configure.md) and [Operations](operate.md) to prepare listeners, source IP allow lists, TLS, and file permissions.

## 1. Create an administrator account

Use an interactive terminal. `opsadmin` below is an example administrator account name. There is no default account or password. The server does not start without an administrator account.

1. Add the administrator account.

   Windows PowerShell:

   ```powershell
   .\vnc-relay.exe admin add opsadmin --config .\relay.toml
   ```

   Ubuntu bash:

   ```shell
   ./vnc-relay admin add opsadmin --config ./relay.toml
   ```

2. Enter the password at the hidden-input prompt.
3. Enter the same password again to confirm it.

The password must contain 12–128 characters and use no more than 512 UTF-8 bytes. Control characters are not allowed. Do not put the password on the command line or pipe it into the command. On success, `admin-accounts.toml` appears beside `relay.toml`.

## 2. Start the server

1. Start the server in a dedicated terminal in the same directory.

   Windows PowerShell:

   ```powershell
   .\vnc-relay.exe serve --config .\relay.toml
   ```

   Ubuntu bash:

   ```shell
   ./vnc-relay serve --config ./relay.toml
   ```

The process keeps the terminal occupied. The example configuration appends JSON Lines operations logs to `relay.log`. Do not consider the server ready before the `server_ready` event is recorded.

## 3. Check readiness

1. Open another terminal on the same PC.
2. Query `/readyz` on the status listener.

   Windows PowerShell:

   ```powershell
   curl.exe --fail http://127.0.0.1:18081/readyz
   ```

   Ubuntu bash:

   ```shell
   curl --fail http://127.0.0.1:18081/readyz
   ```

When ready, the server returns HTTP 200 and `{"status":"ready"}`. The status listener requires no authentication. It is separate from the management listener that serves the dashboard.

## 4. Open the dashboard

1. Open `http://127.0.0.1:18080/` in a browser on the same PC.
2. Sign in with the administrator account you created.

Operational data is unavailable before sign-in. See [Dashboard](dashboard.md) for screen instructions. Follow [Connecting VNC clients](connect.md) to connect VNC clients. This quick start alone does not verify a remote desktop connection.

## 5. Stop gracefully

1. Press Ctrl+C in the terminal running `serve`.
2. Wait for the process to exit and the prompt to return.

On Windows, use Ctrl+C instead of closing the console window. Closing the window cleans up connections without waiting for the remaining shutdown grace period.

See sections 3.2, 3.3, and 3.8 of the [manual operations guide](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/operate-relay.md) for the full procedure. Account input and shutdown rules follow sections 1.2, 1.5, and 1.8 of the [CLI contract](https://github.com/kayden-kim/vnc-relay/blob/main/docs/contracts/cli-v1.md). If a step fails, see [Troubleshooting](troubleshooting.md).
