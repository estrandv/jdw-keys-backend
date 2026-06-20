# jdw-keys-backend

MIDI keyboard controller backend for the JackDAW system. Translates keyboard input (keyboard keys, pads, knobs, buttons) into OSC messages and sends them to the JDW OSC router.

## Features

- **Note-on/note-off** — keyboard keys trigger configurable synths
- **Pad/sample playback** — drum pads trigger samples with sample pack browsing
- **Control bus modulation** — knobs/sliders map to synth parameters in real-time
- **Event history → clipboard as Shuttle Notation** — play notes, sequence copied to clipboard as valid [Shuttle Notation](https://github.com/estrandv/shuttle-notation-python) for pasting into `.bbd` billboard files
- **Ncurses terminal UI** — keyboard visualization, state display, event log, instrument editor
- **Instrument editor** — press `F7` to edit the instrument name as plaintext, `Enter` to confirm
- **OSC-based runtime config** — BPM, quantization, instrument, samples all configurable via OSC
- **Low-latency design** — adaptive sleep ramps CPU down when idle, zero sleep during active play
- **jdw.toml config** — two-layer TOML merge (central `~/.config/jdw.toml` → local `config.toml`) for all defaults and network addresses
- **MIDI mapping** — flexible mapping of MIDI CC to JDW control buses

## Composing with the Keyboard CLI

1. Play notes — the application records timing and note IDs
2. The sequence is stringified to Shuttle Notation and copied to clipboard automatically
3. Paste directly into your `.bbd` billboard file

Example output:
```
(60:1.0 62:1.0 64:0.5 65:0.5):len4,tot4
```

## Ncurses UI Keybindings

| Key | Context | Action |
|---|---|---|
| `F2` | Normal | Toggle keyboard/sampler mode |
| `F3` | Normal | Toggle recording |
| `F4` | Normal | Toggle quantization |
| `F5` | Normal | Toggle multiline output |
| `F6` | Normal | Open pack dropdown selector |
| `F7` | Normal | Open instrument editor (dropdown if instruments available, freetext fallback) |
| `F8` / `F9` | Normal | Octave down / up |
| `+` / `-` | Normal | Next/prev control bus (no shift) / octave up/down (shift held) |
| `Shift+Enter` | Normal | Clear history |
| `F10` / `F1` | Normal | Quit |
| alphanumeric | Text edit | Append to instrument name |
| `Backspace` | Text edit | Delete last character |
| `Enter` | Text edit / Dropdown | Confirm |
| `Esc` / `F1` | Text edit / Dropdown | Cancel |
| `↑` / `↓` | Dropdown | Navigate list |

## Configuration

jdw-keys-backend uses a two-layer TOML config merge:

1. **Central** — `~/.config/jdw.toml` with a `[keys]` section, shared across jdw-suite tools.
2. **Local** — `config.toml` in the project root with shipped defaults.

All configuration values (BPM, instrument, pack, quantization, network addresses, mode toggles) are read from the merged config at startup. See `config.toml` for available keys and defaults.

## Architecture

```
                       ┌──────────────────┐
  Keyboard input ─────>│  Ncurses TUI     │
  (notcurses poll)     │  (main thread)   │
                       └────────┬─────────┘
                                │ MIDIEvent ringbuf
                                v
                       ┌──────────────────┐  UDP   ┌────────────┐
                       │  MIDI Processor  │ ──────>│ OSC Router │
                       │  (spawned thread)│        │ :13339     │
                       └────────┬─────────┘        └────────────┘
                                │ Event ringbuf
                                v
                       ┌──────────────────┐
                       │  History Daemon  │ ──> Clipboard
                       │  (spawned thread)│     (Shuttle Notation)
                       └──────────────────┘
```

State is shared via `Arc<Mutex<State>>`. All three threads read from it; the MIDI processor and OSC read thread write to it.

## Dependencies

- `rosc` — OSC encoding/decoding
- `midir` — MIDI input (unused, kept for future hardware MIDI input)
- `jdw-osc-lib` — shared OSC protocol library
- `ringbuf` — lock-free inter-thread communication
- `notcurses` — terminal UI library
- `toml`, `serde` — config file parsing (jdw.toml)
- `wl-clipboard-rs` — clipboard access for Shuttle Notation export
