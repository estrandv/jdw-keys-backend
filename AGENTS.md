# AGENTS.md — jdw-keys-backend

## Source Structure

```
src/
  main.rs               # Entry: runs ncurses in main thread, spawns MIDI processing,
                        #   OSC read, and history daemon threads
  config.rs             # Two-layer TOML config (central ~/.config/jdw.toml → local config.toml)
  osc_model.rs          # OSC message constructors
  state.rs              # Shared application state (bpm, octave, instrument, mapping)
  event_model.rs        # Event types (NoteOn, PadHit, ControlChange)
  midi_mapping.rs       # MIDI CC -> control bus mapping definitions
  midi_read_daemon.rs   # Standalone MIDI input thread (dead code — MIDI handled via ncurses input)
  midi_translation.rs   # MIDI events -> OSC messages
  keyboard_model.rs     # Keyboard layout, MIDIEvent enum, NcursesCommand enum
  ncurses_daemon.rs     # Ncurses UI: render, input, instrument editor
  osc_client.rs         # OSC send to router
  event_history.rs      # Note history tracking with clipboard export
  util.rs               # Shared utilities
```

## Thread Model

```
┌──────────────┐  ringbuf  ┌──────────────────┐  UDP   ┌────────────┐
│ Ncurses TUI  │ ────────> │ MIDI Processor   │ ──────>│ OSC Router │
│ (main threa  │           │ (spawned thread)  │        │ :13339     │
└──────────────┘           └────────┬─────────┘        └────────────┘
       │                            │
       │                      ┌─────v──────┐
       │ KeycontrolState      │ History    │
       │ ringbuf (from OSC)   │ Daemon     │
       │                      │ (clipboard)│
       v                      └────────────┘
  Arc<Mutex<State>>
  Arc<Mutex<EventHistory>>
```

- **Main thread** runs `NcursesDaemon::begin()` — renders UI, captures keyboard input, pushes `MIDIEvent`s to ringbuf.
- **MIDI Processor thread** pops events from ringbuf, acquires `State` lock, builds OSC messages, sends to router. Also handles `NcursesCommand` state mutations.
- **OSC Read thread** listens for runtime config via OSC `/set_*` messages, writes to `State`.
- **History Daemon thread** consumes `Event` ringbuf, stringifies to Shuttle Notation, copies to clipboard.

## Config System (jdw.toml)

Two-layer TOML merge using `OnceLock<Config>` singleton:

1. **Central**: `~/.config/jdw.toml` — shared across jdw-suite tools, `[keys]` section override.
2. **Local**: `config.toml` — checked into repo, shipped defaults.

```toml
# config.toml
instrument_name = "aPad"
pack = "EMU_EDrum"
bpm = 120
quantization_resolution = 0.125
message_args = 1
bank = 1
keyboard_mode_enabled = true
recording_enabled = true
multiline_output = false
router_host = "127.0.0.1"
router_port = 13339
osc_listen_port = 17777
local_bind_port = 15459
```

`Config::get()` is a `&'static Config` reference set once at `run()` entry via `config::init(Some("config.toml"))`. `State::new()` reads `Config::get()` for defaults.

`available_instruments` defaults to a list extracted from `jdw-pycompose/scd-templating/template_synths.txt` (up to "hypersaw"), overridable at startup via `config.toml` or at runtime via OSC `/set_available_instruments`.

## Ringbuf Channels

| Channel | Type | Capacity | Producer | Consumer |
|---|---|---|---|---|
| `midi_pipe` | `MIDIEvent` | 100 | ncurses thread | MIDI processor |
| `keycontrol_pipe` | `KeyboardModeState` | 100 | OSC read thread | ncurses thread |
| `history_event` | `Event` | 100 | MIDI processor | history daemon |

## NcursesCommand (sent via MIDIEvent::Command)

| Command | Key | Effect |
|---|---|---|
| `ToggleMode` | `F2` | Keyboard ↔ Sampler |
| `ToggleRecording` | `F3` | Enable/disable history recording |
| `ToggleQuantize` | `F4` | Enable/disable quantization |
| `ToggleMultiline` | `F5` | Multi-line Shuttle Notation output |
| `CyclePadBank` | `F6` | Cycle pad sample bank (no-op) |
| `SetInstrument(String)` | `F7` → edit → `Enter` | Change instrument name (dropdown if available_instruments non-empty, freetext fallback) |
| `SetPack(String)` | `F6` → dropdown → `Enter` | Change sample pack |

## Latency Design

- **Adaptive sleep**: both ncurses and MIDI threads use idle-count-based sleep: 0µs when actively processing events, ramping up to 2ms after 10 consecutive idle polls. This keeps CPU near-zero when idle while maintaining sub-millisecond response during play.
- **No stdout I/O in hot path**: `println!` removed from note-on handler.
- **History ringbuf overflow**: drops events silently rather than panicking.
- **Typical keypress→OSC latency**: ~100–200µs during play, at most ~2ms from deep idle.

## OSC Messages Sent

- `/note_on` — `[external_id, synth_name, note, velocity, args...]`
- `/note_off` — `[external_id]`
- `/control_change` — `[bus_index, value]`
- `/pad_hit` — `[pad_id, velocity]`
- `/load_sample` — `[pack, category, name]`

## Ncurses UI Keybindings

| Key | Context | Action |
|---|---|---|
| `F2` | Normal | Toggle keyboard/sampler mode |
| `F3` | Normal | Toggle recording |
| `F4` | Normal | Toggle quantization |
| `F5` | Normal | Toggle multiline output |
| `F6` | Normal | Open pack dropdown selector |
| `F7` | Normal | Open instrument editor |
| `F8` | Normal | Octave down |
| `F9` | Normal | Octave up |
| `+` / `-` | Normal | Next/prev control bus (no shift) or octave up/down (shift held) |
| `F8` / `F9` | Normal | Octave down / up |
| `Shift+Enter` | Normal | Clear history |
| `F10` / `F1` | Normal | Quit |
| — | — | — |
| alphanumeric | Text edit mode | Append to instrument name buffer |
| `Backspace` | Text edit mode | Delete last character |
| `Enter` | Text edit / Dropdown | Confirm new instrument name |
| `Esc` / `F1` | Text edit / Dropdown | Cancel & close editor |
| `↑` / `↓` | Dropdown | Navigate instrument list |

## Runtime Config (OSC Receive)

- `/set_bpm` — `[bpm: float]`
- `/set_quantize` — `[resolution: float]`
- `/set_instrument` — `[synth_name: string]`
- `/set_sample_pack` — `[pack_name: string]`
- `/set_available_instruments` — `[name1: string, name2: string, ...]` — populates F7 instrument dropdown
- `/set_available_packs` — `[name1: string, name2: string, ...]` — populates F6 pack dropdown

## Build & Run

```bash
cargo build --release
cargo run --release
```
