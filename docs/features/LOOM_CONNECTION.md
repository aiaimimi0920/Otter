# Shared Loom connection

## Connection and approval

The Loom row in pin settings shares its state across all pins. When disconnected, choose Connect
and enter one HTTPS service address and an optional connection code, or select a service from
the Loom menu. The address may contain an IP or hostname and a port, but no username, password,
path, query or fragment. The current native dialog has no LAN-target or history list.
The local manifest's HTTP administration endpoint is only for local discovery and registration;
it is not a Device service address accepted by this field.

After submission, Otter validates TLS and Loom identity. Initial remote registration follows
Loom's connection-code and approval policy. The state becomes Connected and the button changes
to Disconnect only after obtaining a device-signed Device session, checking the Loom ID in the
account response and successfully installing the session. Awaiting approval is not a successful connection.
Reconnecting with a saved `deviceId` only requests a signed session; revocation or failed renewal
does not automatically re-register the device.

Closing the address dialog or settings panel does not disconnect. To stop a submitted connection,
choose Disconnect or Cancel Request and Clear Local Session in the Loom menu. This stops the
local connection and attempts to revoke the server session, without deleting device identity or
pairing records. Failure to confirm offline status is reported separately, not presented as
a server-confirmed disconnect.

## Automatic connection and renewal

Explicit remote connection details are stored in one `loom-remote.json`, containing the address,
Loom ID, public certificate and optional `deviceId`, not a multi-target history. Manual connection
or cancellation intent is stored as `paused` and `origin` in `loom-autoconnect-paused.json`.
Neither file stores connection codes, private keys or session tokens.

Startup reuses the saved target and current device identity. With neither remote details nor
an explicit selection, it still tries the local Loom manifest. Manual cancellation persistently
pauses automatic connection across restarts. Selecting a service again, renewing through the menu,
or reimporting connection details clears the pause. Invalid configuration is reported as an error,
not used to guess a different target.

Automatic renewal is scheduled 60 seconds before the server-provided session expiry, with at least
one second before the next attempt. Pending approval and retryable failures use 2, 4, 8, 16 and
30-second backoff, then remain at 30 seconds. Explicit device rejection stops automatic retries;
resolve permissions in Loom before reconnecting. Canceled old tasks cannot install late results,
and no replacement session is issued until the disconnect task finishes.
This is the Device session lifecycle, not Loom's separate 30-second connection lease.

Device connections and the local Art/extension bridge are separate paths. Otter consumes Loom's
authorized capability catalog rather than providing its own marketplace. Connected does not imply
that remote execution, shared memory or every specialized capability is available.

## LAN discovery, connection codes and approval

In Loom's Otter Connection settings, enable Allow LAN Connections, select a local LAN IPv4
address and port (default `8766`), then enter the HTTPS address displayed by Loom into Otter.
Supply a connection code only if Loom requires one. Loom policy determines whether initial
registration is approved automatically.

For a private/loopback IPv4 address not found in saved details or the local manifest, submission
starts listening on UDP `49779` for up to four seconds. Only discovery packets whose source IP
and `endpoint` match the selected target are accepted. Their public certificate is then used
for actual TLS and Loom ID validation. Opening the dialog does not start continuous scanning.
Both computers must be reachable, and firewalls must separately allow the TCP service port
and UDP discovery; TCP reachability does not prove successful discovery.
Other addresses use public TLS trust roots. Manually entering a hostname does not bypass certificate validation.

Initial LAN certificate acquisition relies on a trusted network and does not defend against an
active man-in-the-middle during first pairing. Subsequent connections to the same origin reuse
saved details and revalidate TLS and Loom ID, without replacing trust from new broadcasts or
modifying system root certificates. For preconfiguration without UDP, use trusted public
connection details as described below.

The LAN endpoint is disabled by default and separate from local administration. Its restricted
routes cover device authentication, display-language reports, projection, Surface, wall, local Art HTTP and authorized
Live media WebSocket (`/v1/live/media`). This does not expose administration APIs or the local
bridge; every operation still requires its own authorization checks.

## Importing public connection details

Use this for a shared-Loom setup where one computer runs both Loom and Otter and another runs
only Otter. On the Loom computer, export from the Otter repository root to a local directory
that **does not yet exist**:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/loom-connection/Export-LoomConnection.ps1 -OutputDirectory .tmp/new-connection
```

Export reads public identity from `http://127.0.0.1:8765` by default. Use `-LocalLoom` for another
numeric-loopback HTTP endpoint. Loom must already allow LAN connections. Output contains the
address, Loom ID, public TLS certificate and import scripts; it does not read or copy
administration tokens, private keys or device identity.

Place the output directory beside the Otter package's `payload` directory on the receiver's
local `C:` drive. Exit Otter instances using that profile, then double-click `Connect-Otter.cmd`
in the output directory. After import, the script launches the adjacent `payload/otter-native.exe`;
errors return a nonzero code and leave the terminal open.
The default profile is `%LOCALAPPDATA%/OtterNative`, or `OTTER_NATIVE_PROFILE_DIR` when set.
`loom-remote.json` and `loom-autoconnect-paused.json` are backed up individually only if they exist;
the whole profile is not backed up.

The standalone import command supports `-ConnectionFile`, `-ProfileDirectory` and optional `-Executable`.
Connection files must be UTF-8 and at most 40 KiB; a Loom ID is `loom-` followed by 64 lowercase
hexadecimal characters. Import establishes connection intent only, not device approval or proof
of an actual connection. Once the initial device is approved under Loom policy, Otter connects
using the backoff and renewal rules above. Import only trusted files. If the target or certificate
changes, export again from the correct Loom; do not copy `loom.json`, `device-identity.json`
or an entire user profile.

## Focused verification

Loom can also request a display-language change for one connected Otter device. This uses the
authenticated Device receiver and does not grant general settings or filesystem access.
See [display language](DISPLAY_LANGUAGE.md) for request lifetime, revision conflicts and receipts.

- Native connection and pause state: `loom_local_connection_tests.rs`, `application_loom_connection_tests.rs`
  and adjacent tests for `application_loom_preference.rs`. Tests for `device_authorization_dialog.rs`
  cover input-dialog behavior.
- Import-script contracts: `scripts/tests/Test-LoomConnectionImport.ps1`, using a fresh test output directory.
- For real Loom service fixtures, see [joint testing](../development/JOINT_TESTING.md). Diagnose
  connections in order: selected address, discovery port, certificate and Loom ID, device approval,
  then session. Receiving policy is not a substitute for connection diagnosis.

These checks do not replace actual pin dialogs, physical-network discovery or manual user acceptance.
