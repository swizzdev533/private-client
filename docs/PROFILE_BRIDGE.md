# Profile Bridge

Private Client Core writes:

```json
{
  "schemaVersion": 1,
  "username": "Player",
  "uuid": "00000000-0000-0000-0000-000000000000",
  "skinModel": "classic",
  "skinPath": "skins/00000000-0000-0000-0000-000000000000.png",
  "updatedAt": "2026-07-30T12:00:00Z"
}
```

No other account/session field is allowed. In particular the document never
contains an email, password, OAuth code, access/refresh/Xbox token, cookie,
authorization header, client secret, or serialized session object.

Core validates username/UUID/model/path lengths, writes a temporary file in the
same directory, flushes it, then atomically replaces `profile.json`. The launcher
watches the directory rather than the individual file so replacement is
detected. It waits for writes to settle, caps file size, parses/validates the
whole document, and updates UI only after success. Corrupt data is quarantined
locally and the last valid in-memory profile or neutral fallback remains visible.

Skin files must remain below the dedicated cache root, be valid bounded PNGs, and
use a 64×64 layout. The profile identifies `classic` or `slim`; an invalid skin
uses the original neutral fallback.

Loopback IPC is intentionally absent from v1. The file is the sole source of
truth and materially reduces the attack surface.
