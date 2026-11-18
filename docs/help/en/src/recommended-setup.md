# Recommended setup

Use this page to build a remote support setup in which browser noVNC is the Viewer and the [modified droidVNC-NG](https://github.com/kayden-kim/droidVNC-NG) on an Android device is the Server. Both roles connect over WSS and check access tickets. The page covers the VNC Relay configuration, ticket issuing and verification examples, preparation of both clients, and points to watch in operation. It builds on [Connecting VNC clients](connect.md), [Configuration](configure.md), and [Access tickets](access-ticket.md).

## When to use this setup

Use it when support staff support Android devices across the internet from a web page. The support staff's browser and the device both open connections to VNC Relay. The device therefore does not need to accept incoming connections.

This setup is recommended for these reasons:

- The Viewer and Server share one WSS listener. Only one relay port is open to the internet.
- TLS protects both segments. However, VNC Relay relays decrypted bytes internally, so this is not end-to-end encryption.
- Devices whose source IPs are not known in advance can connect. They still need a ticket that the backend issues after it checks permission.
- When the remote support session expires, VNC Relay ends its connections.
- The endpoints still perform VNC authentication. Tickets do not replace VNC authentication.

## Components

![Diagram: the backend API gives a Viewer ticket to the remote support web app and a Server ticket to the device app. The device app tells the modified droidVNC-NG to connect through an Intent. noVNC connects to /relay/viewer and the modified droidVNC-NG connects to /relay/server on the WSS listener of VNC Relay. The backend API distributes the per-role public keys files to VNC Relay.](images/recommended-setup.svg)

| Component | Runs on | Responsibility |
| --- | --- | --- |
| Remote support web app | Support staff's browser | Uses noVNC as the Viewer. Each time it opens a screen, it gets connection details and a Viewer ticket from the backend API. |
| Backend API (issuer) | Server | Authenticates support staff and devices and manages remote support sessions. Signs tickets with a different key for each role. |
| Device app | Android device | Receives remote support requests. Gets a Server ticket and tells the modified droidVNC-NG to connect. |
| Modified droidVNC-NG | Android device | VNC Server that sends the device screen. Connects back to VNC Relay over WSS. |
| VNC Relay | A host that both roles can reach | Accepts both roles on one WSS listener and checks per-role tickets. Pairs two connections with the same relay ID in Mode II. Verifies signatures only with the per-role public keys files and sends no requests to the issuer. |

## Connection sequence

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

## VNC Relay configuration

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

1. Get the two per-role public keys files from the issuer and place them in `keys/`. The [ticket issuing example](#ticket-issuing-example) shows how to create them.
2. Place the certificate chain and private key in `tls/`.
3. Synchronize the VNC Relay host clock with NTP.
4. Create an administrator account using [Quick start](quick-start.md).
5. Start VNC Relay using [Operations](operate.md).
6. In the operations log, check `server_ready` and the `allow_all_source` warning for the Server role.

## Ticket issuing example

The backend API uses a different P-256 signing key for each role. In this case, the private keys stay in a cloud key management service (KMS), and the backend only requests signatures. The public keys are exported to per-role JWKS files on VNC Relay. `kid` holds the key ID from the key management service. Follow [Access tickets](access-ticket.md#public-keys-file-structure) for the file format and key rotation.

The backend API issues a new Viewer ticket for each connection details request and a new Server ticket for each device connection attempt. It does not store tickets in its database or logs.

| Claim | Value in this case |
| --- | --- |
| `rid` | Relay ID of the remote support session. This case uses six random digits. |
| `iat`, `exp` | The issue time and 60 seconds after it. |
| `sx` | Expiry time of the remote support session. |
| `sub` | Remote support session ID. It appears in the operations log as `ticket_sub`, so it must be an identifier that contains no secrets or personal information. Use it to match the operations log with the backend session. |

`iss` and `rh` hold fixed names that match `issuer` and `relay_host` for both roles. For the other claim rules, follow [Access tickets](access-ticket.md#prepare-the-issuer-and-clients).

A ticket's `iat` comes from the backend clock. If the VNC Relay host clock is more than five seconds behind, every new ticket is rejected as `ticket_invalid`. You must synchronize the backend and VNC Relay host clocks with NTP.

### Example script

The following `ticket_issue.py` creates keys and issues tickets by these rules. Use it for tests or for checks before the issuer exists. It keeps private keys in files, so for production it is recommended to keep private keys where they cannot be exported, such as a key management service. It needs Python 3 and the `cryptography` package.

```python
"""Access ticket issuing example.

python ticket_issue.py keys <role> <kid>         Create the role's signing key and public keys file.
python ticket_issue.py issue <role> <relay-id>   Issue one ticket and print it.
<role> is viewer or server.
"""
import base64
import json
import secrets
import sys
import time

from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives.asymmetric.utils import decode_dss_signature

ISSUER = "issuer.example.test"     # Same value as issuer for both roles
RELAY_HOST = "relay.example.test"  # Same value as relay_host for both roles
LIFETIME = 60                      # At most the role's max_lifetime_secs
SESSION_SECS = 1800                # Length of the example remote support session


def b64url(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode("ascii")


def b64json(value: dict) -> str:
    return b64url(json.dumps(value, separators=(",", ":")).encode("utf-8"))


def make_keys(role: str, kid: str) -> None:
    key = ec.generate_private_key(ec.SECP256R1())
    with open(f"{role}-private.pem", "wb") as f:
        f.write(key.private_bytes(serialization.Encoding.PEM,
                                  serialization.PrivateFormat.PKCS8,
                                  serialization.NoEncryption()))
    point = key.public_key().public_bytes(serialization.Encoding.X962,
                                          serialization.PublicFormat.UncompressedPoint)
    jwk = {"kty": "EC", "crv": "P-256", "kid": kid, "use": "sig", "alg": "ES256",
           "x": b64url(point[1:33]), "y": b64url(point[33:])}
    with open(f"{role}-public.jwks.json", "w", encoding="utf-8") as f:
        json.dump({"keys": [jwk]}, f)


def issue(role: str, relay_id: str, session_id: str, session_expires_at: int) -> str:
    with open(f"{role}-private.pem", "rb") as f:
        key = serialization.load_pem_private_key(f.read(), password=None)
    with open(f"{role}-public.jwks.json", encoding="utf-8") as f:
        kid = json.load(f)["keys"][0]["kid"]
    now = int(time.time())
    claims = {
        "iss": ISSUER,
        "aud": f"vnc-relay/{role}",
        "rh": RELAY_HOST,
        "rid": relay_id,
        "jti": b64url(secrets.token_bytes(16)),  # New 128-bit random value per ticket
        "iat": now,
        "exp": now + LIFETIME,
        "sx": session_expires_at,
        "sub": session_id,
    }
    signing_input = b64json({"alg": "ES256", "kid": kid}) + "." + b64json(claims)
    der = key.sign(signing_input.encode("ascii"), ec.ECDSA(hashes.SHA256()))
    r, s = decode_dss_signature(der)  # Convert the DER signature to 64-byte R‖S
    return signing_input + "." + b64url(r.to_bytes(32, "big") + s.to_bytes(32, "big"))


if __name__ == "__main__":
    command, role = sys.argv[1], sys.argv[2]
    if command == "keys":
        make_keys(role, sys.argv[3])
    else:
        # A real issuer puts the remote support session's ID and expiry time here.
        print(issue(role, sys.argv[3], "session-example", int(time.time()) + SESSION_SECS))
```

1. On the computer that acts as the issuer, install the package and create keys for both roles. Run the commands in the directory that holds `ticket_issue.py`.

   Windows:

   ```powershell
   python -m pip install cryptography
   python ticket_issue.py keys viewer viewer-2026-11
   python ticket_issue.py keys server server-2026-11
   ```

   Ubuntu:

   ```shell
   sudo apt install python3-cryptography
   python3 ticket_issue.py keys viewer viewer-2026-11
   python3 ticket_issue.py keys server server-2026-11
   ```

   Expected result: each role gets `<role>-private.pem` and `<role>-public.jwks.json`.

2. Copy the two `*-public.jwks.json` files to `keys/` on VNC Relay. Do not copy the `*-private.pem` files.

The `issue` command creates tickets. The [ticket verification example](#ticket-verification-example) shows how to use it.

### Signing with a key management service

With a key management service, you do not need `make_keys` or private key files. Get the public key from the service and convert it to JWKS. If the service returns the public key in DER format, read it with `serialization.load_der_public_key`, then derive `x` and `y` the same way as `make_keys`.

Request signatures through the service's signing API instead of `key.sign`. Follow the service documentation to decide whether to send the raw signing input or its SHA-256 digest. If the service returns ECDSA signatures in DER format, you must convert them to the 64-byte R‖S format with `decode_dss_signature`, as in the example. VNC Relay rejects DER signatures.

## Ticket verification example

### Check the signature with the public key

The following `ticket_verify.py` checks a ticket's signature with a public keys file. If the signature is valid, it prints the header and claims. It ends with an error if the public keys file has no matching `kid`, if the signature is in DER format, or if the key differs. This script checks only the signature. To check claims, times, and the use record, see [Check with VNC Relay](#check-with-vnc-relay).

```python
"""Example that checks an access ticket's signature with a public keys file.

python ticket_verify.py <public-keys-file> <ticket>
"""
import base64
import json
import sys

from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives.asymmetric.utils import encode_dss_signature


def unb64url(text: str) -> bytes:
    return base64.urlsafe_b64decode(text + "=" * (-len(text) % 4))


def verify(jwks_path: str, ticket: str) -> dict:
    header_part, claims_part, signature_part = ticket.split(".")
    header = json.loads(unb64url(header_part))
    with open(jwks_path, encoding="utf-8") as f:
        keys = [k for k in json.load(f)["keys"] if k["kid"] == header["kid"]]
    if not keys:
        sys.exit(f"kid {header['kid']!r} is not in the public keys file")
    signature = unb64url(signature_part)
    if len(signature) != 64:
        sys.exit(f"The signature is not 64-byte R‖S ({len(signature)} bytes). It may be DER")
    public_key = ec.EllipticCurvePublicKey.from_encoded_point(
        ec.SECP256R1(), b"\x04" + unb64url(keys[0]["x"]) + unb64url(keys[0]["y"]))
    der = encode_dss_signature(int.from_bytes(signature[:32], "big"),
                               int.from_bytes(signature[32:], "big"))
    # An invalid signature ends with an InvalidSignature exception.
    public_key.verify(der, f"{header_part}.{claims_part}".encode("ascii"),
                      ec.ECDSA(hashes.SHA256()))
    return {"header": header, "claims": json.loads(unb64url(claims_part))}


if __name__ == "__main__":
    print(json.dumps(verify(sys.argv[1], sys.argv[2]), indent=2, ensure_ascii=False))
```

In the directory that holds `ticket_issue.py`, issue a Server ticket and check it. `482913` is a test relay ID.

Windows:

```powershell
$t = python ticket_issue.py issue server 482913
python ticket_verify.py server-public.jwks.json $t
```

Ubuntu:

```shell
t=$(python3 ticket_issue.py issue server 482913)
python3 ticket_verify.py server-public.jwks.json "$t"
```

Expected result: the header's `alg` and `kid` and the claims are printed as JSON. `aud` is `vnc-relay/server`, and `rid` is `482913`.

### Check with VNC Relay

To check whether VNC Relay accepts a ticket, send a WebSocket upgrade request directly. Run it at least five seconds after VNC Relay starts. Tickets issued earlier are rejected as `ticket_issued_before_start`. With a real issuer, put the ticket from its API in `t` instead of using the example script.

1. Issue a Server ticket and send an upgrade request to `/relay/server`.

   Windows:

   ```powershell
   $t = python ticket_issue.py issue server 482913
   curl.exe -i --http1.1 --max-time 2 https://relay.example.test:8443/relay/server -H "Connection: Upgrade" -H "Upgrade: websocket" -H "Sec-WebSocket-Version: 13" -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" -H "Sec-WebSocket-Protocol: binary, ticket.$t"
   ```

   Ubuntu:

   ```shell
   t=$(python3 ticket_issue.py issue server 482913)
   curl -i --http1.1 --max-time 2 https://relay.example.test:8443/relay/server -H "Connection: Upgrade" -H "Upgrade: websocket" -H "Sec-WebSocket-Version: 13" -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" -H "Sec-WebSocket-Protocol: binary, ticket.$t"
   ```

   Expected result: `HTTP/1.1 101 Switching Protocols` and `Sec-WebSocket-Protocol: binary` are printed. The operations log has `ticket_accepted`. curl does not continue the WebSocket exchange, so it ends with a timeout (exit code 28) after two seconds.

2. Run the same `curl` line again.

   Expected result: HTTP 401 and `WWW-Authenticate: VncRelayTicket` are printed. `connection_rejected` in the operations log has `ticket_reused`.

3. Check the Viewer from a computer whose address is in the Viewer `allow` list. In the two lines of step 1, change the `issue` role to `viewer` and the path to `/relay/viewer`, add `-H "Origin: https://viewer.example.test"`, and run them.

   Expected result: after the 101 response, the first WebSocket frame from VNC Relay appears with `RFB 000.000`. The operations log has `ticket_accepted` for the Viewer role.

If a request is rejected, check `reason` in `connection_rejected` in the operations log. Follow [Troubleshooting](troubleshooting.md#an-access-ticket-is-rejected-with-http-401) for each reason.

## Viewer side: noVNC

A `vnc.html` URL cannot send a ticket. The web app creates the noVNC `RFB` object itself and passes the ticket through `wsProtocols`. The example follows the noVNC 1.7.0 package.

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

## Server side: modified droidVNC-NG and the device app

The original droidVNC-NG connects back to a Repeater over TCP only. The [modified droidVNC-NG](https://github.com/kayden-kim/droidVNC-NG) is based on droidVNC-NG 2.22.3 and provides these features:

- When it receives a `ws://` or `wss://` address, it connects over WebSocket. Without a path, it uses `/relay/server`.
- It sends `binary` in `Sec-WebSocket-Protocol`. When it receives a ticket, it also sends `ticket.<JWS>`.
- After the upgrade, it sends the 250-byte `ID:` initial request as the first binary message. It does not wait for start bytes from VNC Relay.
- After that, it exchanges RFB data as binary messages only.
- It verifies the certificate with the Android system trust store and the host name.
- A ticket connection does not reconnect automatically when it drops, because a ticket cannot be used twice.
- It refuses to connect when it receives a ticket with a TCP address. This prevents a silent switch to a TCP connection without a ticket.

Parameters arrive through the droidVNC-NG Intent interface (`ACTION_CONNECT_REPEATER`). The host value carries the WSS address, and a separate value carries the ticket. A socket pair connects the WebSocket client in the app to the VNC server engine. No loopback port is opened, so other apps on the device cannot intercept the connection. The [Intent interface document](https://github.com/kayden-kim/droidVNC-NG/blob/master/doc/Intent-Interface.md) of the modified build lists the value names and `adb` command examples, and [ADR 0006](https://github.com/kayden-kim/droidVNC-NG/blob/master/doc/adr/0006-bridge-websocket-repeaters-via-socket-pair.md) explains the design.

The device app is responsible for these tasks:

- Check that the installed droidVNC-NG is a modified build with WebSocket support before passing a WSS address. Do not rely on the version number alone, because a modified build can keep the original version number.
- Get a new Server ticket from the backend API for each connection attempt. If the ticket request or the connect request fails, retry with a new ticket at increasing intervals. This retry does not cover a drop after a successful connection.
- Do not fall back to TCP without a ticket when the WSS connection fails.
- Do not write tickets to logs.

> **Warning:** The device app sends the ticket request itself, so Android background network restrictions apply. On Android 16, the device app's requests can fail while the droidVNC-NG screen sharing confirmation is in front. Make the device app retry with a new ticket after it returns to the foreground.

> **Note:** droidVNC-NG is licensed under GPL 2.0. If you distribute a modified build to others, you must meet the license terms, such as providing the source code.

## Points to watch in operation

- While you migrate older devices, you can keep a TCP Server entry point open as well. That path does not check tickets and produces the `ticket_tcp_bypass` warning. Close it after the migration.
- Tickets are checked only at connection time. If the backend ends a session early, connections that are already open stay until `sx`. You can end an active session immediately with termination by administrator in the [Dashboard](dashboard.md#session-details-and-termination).
- Restarting VNC Relay drops every connection. Tickets whose `iat` is at or before process startup time + 5 seconds are rejected. After a restart, wait briefly and reconnect with new tickets.
- Each role has only one `issuer` and one `relay_host`. If several environments, such as development and production, share one VNC Relay, each role's public keys file trusts the keys of every environment. A ticket from another environment can then connect if the relay ID matches. To isolate environments, run a separate VNC Relay for each.
- Origin only blocks browsers on other sites. It is not user authentication.

## Verified scope

VNC Relay's noVNC verification (1.7.0) used `vnc.html` without `wsProtocols`. VNC Relay's automated tests do not cover ticket sending from noVNC or the modified droidVNC-NG. This setup has been confirmed on a real Android device for WSS and ticket connections of both roles and the screen display. Screen input and session expiry have not been confirmed on a real device.

The example scripts and the [Check with VNC Relay](#check-with-vnc-relay) procedure on this page have been run on Windows and Ubuntu with the expected results. The run record is in the [verification table of the help publishing how-to](https://github.com/kayden-kim/vnc-relay/blob/main/docs/how-to/publish-help.md#4-검증).
