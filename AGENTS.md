# AGENTS.md — jdw-keys-backend

## Source Structure

```
src/
  main.rs               # Entry: spawns MIDI read, OSC read, ncurses, history daemon threads
  osc_model.rs          # OSC message constructors
  state.rs              # Shared application state (bpm, octave, instrument, mapping)
  event_model.rs        # Event types (NoteOn, PadHit, ControlChange)
  midi_mapping.rs       # MIDI CC -> control bus mapping definitions
  midi_read_daemon.rs   # MIDI input thread
  midi_translation.rs   # MIDI events -> OSC messages
  keyboard_model.rs     # Keyboard layout and octave logic
  ncurses_daemon.rs     # Terminal UI thread
  osc_client.rs         # OSC send to router
  event_history.rs      # Note history tracking with clipboard export
  util.rs               # Shared utilities
```

## Thread Model

```
┌──────────────┐  ringbuf  ┌──────────────┐
│ MIDI Read    │ ────────> │ Main/OSC     │ ────> OSC Router
│ Daemon       │           │ Daemon       │
└──────────────┘           └──────┬───────┘
                                  │
                          ┌───────v───────┐
                          │ Ncurses TUI   │
                          │ Daemon        │
                          └───────────────┘
```

## OSC Messages Sent

- `/note_on` — `[external_id, synth_name, note, velocity, args...]`
- `/note_off` — `[external_id]`
- `/control_change` — `[bus_index, value]`
- `/pad_hit` — `[pad_id, velocity]`
- `/load_sample` — `[pack, category, name]`

## Runtime Config (OSC Receive)

- `/set_bpm` — `[bpm: float]`
- `/set_quantize` — `[resolution: float]`
- `/set_instrument` — `[synth_name: string]`
- `/set_sample_pack` — `[pack_name: string]`

## Build & Run

```bash
cargo build --release
cargo run --release
```
