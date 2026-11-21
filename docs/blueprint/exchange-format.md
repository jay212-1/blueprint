# Showtime exchange format (version 1)

One JSON file (`<name>.showtime.json`) with what the live tools of the Showtime Suite need from a
plan: channel names, the patch, the mic plan per scene and the DCAs. Blueprint writes it
(Production › "Export exchange file…"); the code is `crates/bp-core/src/exchange.rs`.

## Rules

- `format` is always `"showtime-exchange"`. Anything else is not an exchange file.
- `version` is a whole number. It grows only when a field changes its meaning or goes away.
  A reader refuses a file with a higher version than it knows.
- New fields may appear in any version. Readers ignore fields they don't know, and missing fields
  take their empty value (`""`, `0`, `false`, `null`, `[]`).
- Only plain text and numbers. There are no Blueprint ids: things refer to each other by what a
  person reads on the desk (channel numbers, transmitter channels, device names).
- UTF-8, pretty printed.

## Fields

```jsonc
{
  "format": "showtime-exchange",
  "version": 1,
  "generator": "Blueprint 0.1.0",
  "production": { "name": "", "company": "", "venue": "", "dates": "" },

  // The input patch, in channel order.
  "inputs": [{
    "channel": 1,              // input number of the patch
    "name": "Kick in",         // channel name for the desk
    "source": "Kick",
    "mic": "Beta 91",
    "phantom": true,
    "color": "red",            // red, orange, yellow, green, blue, purple or ""
    "performer": "Henrik Jansen",
    "stagebox": "SB1", "port": 1,
    "line": "MC1", "line_channel": 1,
    "desks": [{ "desk": "FOH", "channel": 1 }, { "desk": "MON", "channel": 1 }],
    "notes": "Gate"
  }],

  "outputs": [{
    "name": "IEM Ida", "desk": "MON", "bus": "Mix 7/8",
    "device": "IEM rack", "port": 1,
    "destination": "iem",      // pa, monitor, iem, recording, broadcast, other
    "performer": "Ida Lorenz", "mix_notes": "Lots of vocal"
  }],

  // Radio mics; "channel" is the desk / receiver channel as written in the plan.
  "transmitters": [{ "channel": "12", "name": "", "model": "", "frequency_mhz": 518.2, "status": "active" }],

  // The devices of the patch (Pulse takes them over as its inventory).
  "devices": [{
    "name": "SB1",
    "kind": "stagebox",        // stagebox, io-rack, console, splitter, amp, recorder, iem, other
    "location": "Stage left",  // the location's name, or ""
    "model": "DL32",
    "address": "192.168.1.31", // IP address or host name on the show network, or ""
    "notes": ""
  }],

  // The mic plan, scene by scene.
  "scenes": [{
    "number": "1.1", "title": "Harbour",
    "mics": [{ "channel": "12", "role": "Keeper", "performer": "Anna Berg", "b": false }],
    "dcas": [{ "dca": 1, "label": "Leads", "channels": ["12", "14"] }]
  }]
}
```

`status` is `active`, `spare` or `defect`. `b` marks the role's second (B) mic. A scene's `mics`
lists every transmitter in use in that scene: one entry per role, so a shared transmitter can appear
twice.

`devices` came later in version 1; older files have none (empty list). `address` is the
Address column of the device table.
